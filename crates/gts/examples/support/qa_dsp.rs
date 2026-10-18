//! Metric functions of the factory-bank QA tool (`examples/bank_qa.rs`): filters, spectra,
//! centroid series, autocorrelation periods, alias floor, mel distance, pitch tracking, envelope
//! times. Pure functions on `f64` slices at 48 kHz, no engine; the unit tests at the bottom feed them
//! synthetic signals with a known answer (period, centroid, correlation, pitch).
#![allow(dead_code)]
use gts::dsp::tables::Fft;
use std::f64::consts::{FRAC_1_SQRT_2, TAU};

pub const SR: f64 = 48000.0;

pub fn db10(x: f64) -> f64 {
    10.0 * x.max(1e-30).log10()
}
pub fn db20(x: f64) -> f64 {
    20.0 * x.max(1e-15).log10()
}
pub fn mean_sq(x: &[f64]) -> f64 {
    if x.is_empty() {
        0.0
    } else {
        x.iter().map(|v| v * v).sum::<f64>() / x.len() as f64
    }
}
pub fn mean(x: &[f64]) -> f64 {
    if x.is_empty() {
        0.0
    } else {
        x.iter().sum::<f64>() / x.len() as f64
    }
}

// ---- filters -----------------------------------------------------------------------------------

pub fn biquad(x: &[f64], b: [f64; 3], a: [f64; 3]) -> Vec<f64> {
    let (mut x1, mut x2, mut y1, mut y2) = (0.0, 0.0, 0.0, 0.0);
    x.iter()
        .map(|&v| {
            let y = b[0] * v + b[1] * x1 + b[2] * x2 - a[1] * y1 - a[2] * y2;
            (x2, x1, y2, y1) = (x1, v, y1, y);
            y
        })
        .collect()
}

fn butter2(fc: f64, hp: bool) -> ([f64; 3], [f64; 3]) {
    let w = TAU * fc / SR;
    let (c, al) = (w.cos(), w.sin() / (2.0 * FRAC_1_SQRT_2));
    let a0 = 1.0 + al;
    let a = [1.0, -2.0 * c / a0, (1.0 - al) / a0];
    let b = if hp {
        [(1.0 + c) / 2.0 / a0, -(1.0 + c) / a0, (1.0 + c) / 2.0 / a0]
    } else {
        [(1.0 - c) / 2.0 / a0, (1.0 - c) / a0, (1.0 - c) / 2.0 / a0]
    };
    (b, a)
}

/// 4th-order Linkwitz-Riley (two cascaded 2nd-order Butterworth sections) low- or high-pass.
pub fn lr4(x: &[f64], fc: f64, hp: bool) -> Vec<f64> {
    let (b, a) = butter2(fc, hp);
    biquad(&biquad(x, b, a), b, a)
}

pub fn bandpass(x: &[f64], lo: f64, hi: f64) -> Vec<f64> {
    lr4(&lr4(x, lo, true), hi, false)
}

// ---- stereo ------------------------------------------------------------------------------------

/// Pearson correlation. Two silent channels count as identical (1.0); one silent channel is 0.
pub fn pearson(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().min(b.len());
    if n == 0 {
        return 1.0;
    }
    let (ma, mb) = (mean(&a[..n]), mean(&b[..n]));
    let (mut sab, mut saa, mut sbb) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let (x, y) = (a[i] - ma, b[i] - mb);
        sab += x * y;
        saa += x * x;
        sbb += y * y;
    }
    let eps = 1e-18 * n as f64;
    if saa < eps && sbb < eps {
        1.0
    } else {
        sab / (saa * sbb).sqrt().max(1e-30)
    }
}

/// (mid, side) = ((L+R)/2, (L-R)/2).
pub fn mid_side(l: &[f64], r: &[f64]) -> (Vec<f64>, Vec<f64>) {
    (l.iter().zip(r).map(|(a, b)| 0.5 * (a + b)).collect(), l.iter().zip(r).map(|(a, b)| 0.5 * (a - b)).collect())
}

/// Low-band stereo figures below `fc` (LR4): (L/R correlation, side-to-mid dB, mono-sum minus stereo
/// energy dB). The last is `10 log10(P_mid / ((P_L + P_R) / 2))`: 0 for identical channels.
pub fn low_band(l: &[f64], r: &[f64], fc: f64) -> (f64, f64, f64) {
    let (ll, lr) = (lr4(l, fc, false), lr4(r, fc, false));
    let (m, s) = mid_side(&ll, &lr);
    let (pm, ps) = (mean_sq(&m), mean_sq(&s));
    let st = 0.5 * (mean_sq(&ll) + mean_sq(&lr));
    (pearson(&ll, &lr), db10(ps / pm.max(1e-30)), db10(pm / st.max(1e-30)))
}

/// L/R correlation above `fc` (LR4 high-pass): the Width figure.
pub fn width_corr(l: &[f64], r: &[f64], fc: f64) -> f64 {
    pearson(&lr4(l, fc, true), &lr4(r, fc, true))
}

// ---- spectra -----------------------------------------------------------------------------------

pub fn hann(n: usize) -> Vec<f64> {
    (0..n).map(|i| 0.5 - 0.5 * (TAU * i as f64 / n as f64).cos()).collect()
}

/// Power spectrum (nfft/2 + 1 bins) of `x * win`, zero padded to `nfft`.
pub fn frame_power(fft: &Fft, x: &[f64], win: &[f64], nfft: usize) -> Vec<f64> {
    let mut re = vec![0.0; nfft];
    for (i, (v, w)) in x.iter().zip(win).enumerate().take(nfft) {
        re[i] = v * w;
    }
    let mut im = vec![0.0; nfft];
    fft.run(&mut re, &mut im, false);
    (0..=nfft / 2).map(|k| re[k] * re[k] + im[k] * im[k]).collect()
}

/// Blackman-Harris power spectrum of a power-of-two length block, and the bin width.
pub fn bh_power(fft: &Fft, x: &[f64]) -> (Vec<f64>, f64) {
    let n = x.len();
    let w: Vec<f64> = (0..n)
        .map(|i| {
            let t = TAU * i as f64 / (n - 1) as f64;
            0.35875 - 0.48829 * t.cos() + 0.14128 * (2.0 * t).cos() - 0.01168 * (3.0 * t).cos()
        })
        .collect();
    (frame_power(fft, x, &w, n), SR / n as f64)
}

/// Amplitude-weighted (magnitude) centroid, 20 Hz to 20 kHz, mean over 4096-sample frames: the
/// evidence column next to the power-weighted Brightness.
pub fn brightness_amp(fft: &Fft, x: &[f64]) -> f64 {
    let w = hann(4096);
    let df = SR / 4096.0;
    let (mut s, mut n, mut i) = (0.0, 0usize, 0);
    while i + 4096 <= x.len() {
        let p: Vec<f64> = frame_power(fft, &x[i..i + 4096], &w, 4096).iter().map(|v| v.sqrt()).collect();
        if let Some(c) = centroid_of(&p, df, 20.0, 20000.0) {
            s += c;
            n += 1;
        }
        i += 2048;
    }
    s / n.max(1) as f64
}

/// Power-weighted centroid of the bins in `lo..=hi` Hz; `None` for (near) silence.
pub fn centroid_of(p: &[f64], df: f64, lo: f64, hi: f64) -> Option<f64> {
    let (mut num, mut den) = (0.0, 0.0);
    for (k, &v) in p.iter().enumerate().skip(1) {
        let f = k as f64 * df;
        if f >= lo && f <= hi {
            num += f * v;
            den += v;
        }
    }
    (den > 1e-24).then(|| num / den)
}

/// Centroid time series: Hann window `win_len`, zero padded to `nfft`, one value per `hop`
/// samples. Silent frames repeat the previous value.
pub fn centroid_series(fft: &Fft, x: &[f64], win_len: usize, hop: usize, nfft: usize, lo: f64, hi: f64) -> Vec<f64> {
    let w = hann(win_len);
    let df = SR / nfft as f64;
    let (mut out, mut last, mut i) = (vec![], (lo * hi).sqrt(), 0);
    while i + win_len <= x.len() {
        if let Some(c) = centroid_of(&frame_power(fft, &x[i..i + win_len], &w, nfft), df, lo, hi) {
            last = c;
        }
        out.push(last);
        i += hop;
    }
    out
}

/// The Brightness figure: mean over 4096-sample frames of the power-weighted centroid, 20 Hz to 20 kHz.
pub fn brightness(fft: &Fft, x: &[f64]) -> f64 {
    mean(&centroid_series(fft, x, 4096, 2048, 4096, 20.0, 20000.0))
}

/// The Move centroid series: 120 Hz to 8 kHz, 1024-sample window, hop 256, padded to 4096.
pub fn move_centroid(fft: &Fft, x: &[f64]) -> Vec<f64> {
    centroid_series(fft, x, 1024, 256, 4096, 120.0, 8000.0)
}
pub const MOVE_HOP: usize = 256;

pub fn percentile(v: &[f64], p: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.total_cmp(b));
    let x = p.clamp(0.0, 1.0) * (s.len() - 1) as f64;
    let (i, f) = (x.floor() as usize, x.fract());
    s[i] + (s[(i + 1).min(s.len() - 1)] - s[i]) * f
}

/// Move depth: p95 - p5 of log2 of the series, octaves.
pub fn depth_oct(c: &[f64]) -> f64 {
    let l: Vec<f64> = c.iter().map(|v| v.max(1.0).log2()).collect();
    percentile(&l, 0.95) - percentile(&l, 0.05)
}

/// Removes the least-squares line.
pub fn detrend(x: &[f64]) -> Vec<f64> {
    let n = x.len();
    if n < 2 {
        return x.to_vec();
    }
    let (mt, mx) = ((n - 1) as f64 / 2.0, mean(x));
    let (mut sxy, mut sxx) = (0.0, 0.0);
    for (i, v) in x.iter().enumerate() {
        sxy += (i as f64 - mt) * (v - mx);
        sxx += (i as f64 - mt).powi(2);
    }
    let slope = sxy / sxx.max(1e-30);
    x.iter().enumerate().map(|(i, v)| v - mx - slope * (i as f64 - mt)).collect()
}

// ---- autocorrelation / period ------------------------------------------------------------------

/// Normalised autocorrelation: `r[k]` = Pearson correlation of `x[..n-k]` and `x[k..]`. A constant
/// series gives zeros. `max_lag` is capped at `0.6 n` so every lag keeps at least 40 % of the samples.
pub fn acf(x: &[f64], max_lag: usize) -> Vec<f64> {
    let n = x.len();
    let max_lag = max_lag.min(n * 3 / 5);
    (0..=max_lag)
        .map(|k| {
            if k == 0 {
                return 1.0;
            }
            let (a, b) = (&x[..n - k], &x[k..]);
            let (ma, mb) = (mean(a), mean(b));
            let (mut sab, mut saa, mut sbb) = (0.0, 0.0, 0.0);
            for i in 0..a.len() {
                let (p, q) = (a[i] - ma, b[i] - mb);
                sab += p * q;
                saa += p * p;
                sbb += q * q;
            }
            if saa * sbb < 1e-24 {
                0.0
            } else {
                sab / (saa * sbb).sqrt()
            }
        })
        .collect()
}

#[derive(Clone, Copy, Debug)]
pub struct Peak {
    /// Lag in samples of the series (parabolic interpolation).
    pub lag: f64,
    pub val: f64,
}

/// Local maxima of `r` with a lag in `lo..=hi`.
pub fn peaks(r: &[f64], lo: usize, hi: usize) -> Vec<Peak> {
    let mut v = vec![];
    for k in lo.max(1)..=hi.min(r.len().saturating_sub(2)) {
        if r[k] > r[k - 1] && r[k] >= r[k + 1] {
            let d = r[k - 1] - 2.0 * r[k] + r[k + 1];
            let off = if d.abs() > 1e-12 { 0.5 * (r[k - 1] - r[k + 1]) / d } else { 0.0 };
            v.push(Peak { lag: k as f64 + off, val: r[k] - 0.25 * (r[k - 1] - r[k + 1]) * off });
        }
    }
    v
}

/// The autocorrelation period: the earliest peak within 10 % of the largest one in `lo..=hi`
/// (the largest alone is often a multiple of the period).
pub fn dominant_peak(r: &[f64], lo: usize, hi: usize) -> Option<Peak> {
    let p = peaks(r, lo, hi);
    let m = p.iter().map(|x| x.val).fold(f64::MIN, f64::max);
    p.into_iter().find(|x| x.val >= 0.9 * m)
}

/// The highest peak within `+-tol` (fraction) of `expected` (samples).
pub fn peak_near(r: &[f64], expected: f64, tol: f64) -> Option<Peak> {
    let lo = ((expected * (1.0 - tol)).floor() as usize).saturating_sub(1).max(1);
    let hi = (expected * (1.0 + tol)).ceil() as usize + 1;
    peaks(r, lo, hi).into_iter().filter(|p| (p.lag - expected).abs() <= expected * tol).max_by(|a, b| a.val.total_cmp(&b.val))
}

// ---- levels / envelopes ------------------------------------------------------------------------

/// dB of the mean square in `win`-sample windows every `hop` samples.
pub fn rms_env_db(x: &[f64], win: usize, hop: usize) -> Vec<f64> {
    let mut v = vec![];
    let mut i = 0;
    while i + win <= x.len() {
        v.push(db10(mean_sq(&x[i..i + win])));
        i += hop;
    }
    v
}

/// Seconds from index `from` until the envelope first sits `drop_db` below `env[from]`
/// (linear interpolation); `None` when it never does.
pub fn fall_time(env: &[f64], hop_s: f64, from: usize, drop_db: f64) -> Option<f64> {
    let target = env.get(from)? - drop_db;
    for i in from + 1..env.len() {
        if env[i] <= target {
            let (a, b) = (env[i - 1], env[i]);
            let f = if a > b { (a - target) / (a - b) } else { 1.0 };
            return Some(((i - 1 - from) as f64 + f) * hop_s);
        }
    }
    None
}

/// Seconds from the start until a (linear amplitude) envelope first reaches `frac` of its peak.
pub fn rise_time(env_amp: &[f64], hop_s: f64, frac: f64) -> Option<f64> {
    let peak = env_amp.iter().cloned().fold(0.0, f64::max);
    let i = env_amp.iter().position(|&v| v >= frac * peak)?;
    Some(i as f64 * hop_s)
}

/// Monotonic within `tol`: no step moves against the overall direction by more than `tol`.
pub fn monotonic(v: &[f64], tol: f64) -> bool {
    if v.len() < 2 {
        return true;
    }
    let dir = (v[v.len() - 1] - v[0]).signum();
    let dir = if dir == 0.0 { 1.0 } else { dir };
    v.windows(2).all(|w| (w[1] - w[0]) * dir >= -tol)
}

/// Largest first difference (per channel max) inside `a..b`.
pub fn max_step(l: &[f32], r: &[f32], a: usize, b: usize) -> f64 {
    let b = b.min(l.len());
    (a.max(1)..b).map(|i| ((l[i] - l[i - 1]).abs()).max((r[i] - r[i - 1]).abs()) as f64).fold(0.0, f64::max)
}

// ---- harmonic structure ------------------------------------------------------------------------

/// Power of the strongest bin within `+-1.5 %` of `k * f0`, plus its 3 neighbours each side.
pub fn harmonic_power(p: &[f64], df: f64, f0: f64, k: usize) -> f64 {
    let c = k as f64 * f0;
    let (a, b) = (((0.985 * c / df) as usize).max(1), ((1.015 * c / df) as usize + 1).min(p.len() - 2));
    if a >= b {
        return 0.0;
    }
    let kp = (a..=b).max_by(|&i, &j| p[i].total_cmp(&p[j])).unwrap();
    p[kp.saturating_sub(3)..=(kp + 3).min(p.len() - 1)].iter().sum()
}

/// Alias floor, dB: power outside the harmonic bands (0..20 kHz, above 15 Hz) over the power of the
/// fundamental. A harmonic band is `+-max(1 % of k f0, unison spread * k f0)` around `k f0`, no
/// narrower than 5 bins (the Blackman-Harris main lobe). This is the sheet's rule taken literally: once
/// a band is wider than f0 / 2 (k > 50 for the 1 % rule) no bin is outside it, so nothing above
/// about 50 f0 is judged (6.5 kHz for C3, 2.2 kHz for F1).
///
/// `f0` is the base of the harmonic grid (the played pitch, or a sub-octave of it when a layer sits
/// below the played pitch); the reference is the strongest of its first 8 partials.
pub fn alias_floor_db(p: &[f64], df: f64, f0: f64, spread_cents: f64) -> f64 {
    let spread = 2f64.powf(spread_cents / 1200.0) - 1.0;
    let fund = (1..=8).map(|k| harmonic_power(p, df, f0, k)).fold(0.0, f64::max);
    let mut inharm = 0.0;
    for (k, &v) in p.iter().enumerate() {
        let f = k as f64 * df;
        if !(15.0..=20000.0).contains(&f) {
            continue;
        }
        let h = (f / f0).round().max(1.0);
        let hw = (h * f0 * spread.max(0.01)).max(5.0 * df);
        if (f - h * f0).abs() > hw {
            inharm += v;
        }
    }
    db10(inharm / fund.max(1e-30))
}

/// Share of the spectrum's energy (10 Hz up) below `f_hi`.
pub fn band_fraction(p: &[f64], df: f64, f_hi: f64) -> f64 {
    let (mut lo, mut tot) = (0.0, 0.0);
    for (k, &v) in p.iter().enumerate() {
        let f = k as f64 * df;
        if f < 10.0 {
            continue;
        }
        tot += v;
        if f < f_hi {
            lo += v;
        }
    }
    lo / tot.max(1e-30)
}

/// Energy between `lo` and `hi` Hz, dB (relative).
pub fn band_db(p: &[f64], df: f64, lo: f64, hi: f64) -> f64 {
    db10(p.iter().enumerate().filter(|(k, _)| (lo..hi).contains(&(*k as f64 * df))).map(|(_, v)| v).sum())
}

/// THD, percent: harmonics 2..n below `f_max` over the fundamental (amplitude ratio).
pub fn thd_percent(p: &[f64], df: f64, f0: f64, f_max: f64) -> f64 {
    let fund = harmonic_power(p, df, f0, 1);
    let mut h = 0.0;
    let mut k = 2;
    while k as f64 * f0 < f_max {
        h += harmonic_power(p, df, f0, k);
        k += 1;
    }
    100.0 * (h / fund.max(1e-30)).sqrt()
}

// ---- mel descriptor ----------------------------------------------------------------------------

const MEL_BANDS: usize = 40;

fn mel(f: f64) -> f64 {
    2595.0 * (1.0 + f / 700.0).log10()
}
fn inv_mel(m: f64) -> f64 {
    700.0 * (10f64.powf(m / 2595.0) - 1.0)
}

/// Mean, standard deviation and mean absolute frame-to-frame change (the rate of movement) of the
/// 40-band log-mel spectrum (30 Hz to 16 kHz, 2048 frames, hop 512), dB: the descriptor the
/// distinctiveness check compares.
pub fn mel_descriptor(fft: &Fft, x: &[f64]) -> Vec<f64> {
    let (n, hop) = (2048, 512);
    let w = hann(n);
    let df = SR / n as f64;
    let (m0, m1) = (mel(30.0), mel(16000.0));
    let edges: Vec<f64> = (0..MEL_BANDS + 2).map(|i| inv_mel(m0 + (m1 - m0) * i as f64 / (MEL_BANDS + 1) as f64)).collect();
    let mut frames: Vec<Vec<f64>> = vec![];
    let mut i = 0;
    while i + n <= x.len() {
        let p = frame_power(fft, &x[i..i + n], &w, n);
        frames.push(
            (0..MEL_BANDS)
                .map(|b| {
                    let (lo, c, hi) = (edges[b], edges[b + 1], edges[b + 2]);
                    let e: f64 = p
                        .iter()
                        .enumerate()
                        .map(|(k, v)| {
                            let f = k as f64 * df;
                            let g = if f <= lo || f >= hi { 0.0 } else if f < c { (f - lo) / (c - lo) } else { (hi - f) / (hi - c) };
                            g * v
                        })
                        .sum();
                    db10(e + 1e-4)
                })
                .collect(),
        );
        i += hop;
    }
    let nf = frames.len().max(1) as f64;
    let m: Vec<f64> = (0..MEL_BANDS).map(|b| frames.iter().map(|f| f[b]).sum::<f64>() / nf).collect();
    let s: Vec<f64> = (0..MEL_BANDS).map(|b| (frames.iter().map(|f| (f[b] - m[b]).powi(2)).sum::<f64>() / nf).sqrt()).collect();
    let d: Vec<f64> = (0..MEL_BANDS).map(|b| frames.windows(2).map(|w| (w[1][b] - w[0][b]).abs()).sum::<f64>() / (frames.len().max(2) - 1) as f64).collect();
    m.into_iter().chain(s).chain(d).collect()
}

/// RMS difference of two descriptors, dB.
pub fn mel_distance(a: &[f64], b: &[f64]) -> f64 {
    (a.iter().zip(b).map(|(x, y)| (x - y).powi(2)).sum::<f64>() / a.len().max(1) as f64).sqrt()
}

// ---- pitch -------------------------------------------------------------------------------------

/// One YIN estimate (normalised difference, parabolic interpolation) of the frame at `start`:
/// `(f0, confidence 0..1)`. Searches `fmin..=fmax`; `None` when the frame does not fit.
pub fn yin(x: &[f64], start: usize, fmin: f64, fmax: f64) -> Option<(f64, f64)> {
    let (t0, t1) = ((SR / fmax).floor().max(2.0) as usize, (SR / fmin).ceil() as usize);
    let w = (2 * t1).max(480).min(4 * t1);
    if start + w + t1 + 1 >= x.len() {
        return None;
    }
    let seg = &x[start..start + w + t1 + 1];
    let mut d = vec![0.0; t1 + 2];
    for tau in 1..=t1 + 1 {
        d[tau] = (0..w).map(|j| (seg[j] - seg[j + tau]).powi(2)).sum();
    }
    let mut cm = vec![1.0; t1 + 2];
    let mut run = 0.0;
    for tau in 1..=t1 + 1 {
        run += d[tau];
        cm[tau] = if run > 0.0 { d[tau] * tau as f64 / run } else { 1.0 };
    }
    let mut best = None;
    let mut tau = t0.max(2);
    while tau <= t1 {
        if cm[tau] < 0.15 {
            while tau + 1 <= t1 && cm[tau + 1] < cm[tau] {
                tau += 1;
            }
            best = Some(tau);
            break;
        }
        tau += 1;
    }
    let tau = best.unwrap_or_else(|| (t0.max(2)..=t1).min_by(|&a, &b| cm[a].total_cmp(&cm[b])).unwrap());
    let (a, b, c) = (cm[tau - 1], cm[tau], cm[tau + 1]);
    let den = a - 2.0 * b + c;
    let off = if den.abs() > 1e-12 { (0.5 * (a - c) / den).clamp(-1.0, 1.0) } else { 0.0 };
    Some((SR / (tau as f64 + off), (1.0 - cm[tau]).clamp(0.0, 1.0)))
}

/// Pitch track every `hop` samples from `start`: `(time s, f0, confidence)`. The first frame
/// searches `fmin..fmax`; later frames follow the previous pitch (0.6x to 1.6x in period).
pub fn pitch_track(x: &[f64], start: usize, hop: usize, frames: usize, fmin: f64, fmax: f64) -> Vec<(f64, f64, f64)> {
    let mut out: Vec<(f64, f64, f64)> = vec![];
    for k in 0..frames {
        let s = start + k * hop;
        let est = match out.last() {
            None => yin(x, s, fmin, fmax),
            Some(&(_, f, _)) => yin(x, s, (f / 1.6).max(fmin * 0.5), (f * 1.6).min(fmax * 1.5)),
        };
        match est {
            Some((f, c)) => out.push((s as f64 / SR, f, c)),
            None => break,
        }
    }
    out
}

/// Linear interpolation of a track at time `t`.
pub fn track_at(tr: &[(f64, f64, f64)], t: f64) -> Option<f64> {
    let i = tr.iter().position(|p| p.0 >= t)?;
    if i == 0 {
        return Some(tr[0].1);
    }
    let (a, b) = (tr[i - 1], tr[i]);
    Some(a.1 + (b.1 - a.1) * (t - a.0) / (b.0 - a.0).max(1e-9))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(f: f64, secs: f64, amp: f64) -> Vec<f64> {
        (0..(SR * secs) as usize).map(|i| amp * (TAU * f * i as f64 / SR).sin()).collect()
    }

    #[test]
    fn pearson_known_values() {
        let a = sine(100.0, 0.2, 1.0);
        let neg: Vec<f64> = a.iter().map(|v| -v).collect();
        let half: Vec<f64> = a.iter().map(|v| 0.5 * v + 0.3).collect();
        assert!((pearson(&a, &a) - 1.0).abs() < 1e-12);
        assert!((pearson(&a, &neg) + 1.0).abs() < 1e-12);
        assert!((pearson(&a, &half) - 1.0).abs() < 1e-12, "scale and offset do not matter");
        // 90 degrees apart over whole cycles: uncorrelated.
        let q = sine(100.0, 0.2, 1.0).iter().enumerate().map(|(i, _)| (TAU * 100.0 * i as f64 / SR).cos()).collect::<Vec<_>>();
        assert!(pearson(&a, &q).abs() < 1e-6);
        assert_eq!(pearson(&[0.0; 100], &[0.0; 100]), 1.0);
        assert_eq!(pearson(&[0.0; 100], &a[..100]), 0.0);
    }

    #[test]
    fn lr4_passes_and_stops() {
        let lo = sine(50.0, 1.0, 1.0);
        let hi = sine(5000.0, 1.0, 1.0);
        let gain = |y: &[f64]| db10(mean_sq(&y[24000..]) / 0.5);
        assert!(gain(&lr4(&lo, 120.0, false)).abs() < 0.5, "50 Hz passes a 120 Hz LR4 low-pass");
        assert!(gain(&lr4(&hi, 120.0, false)) < -60.0);
        assert!(gain(&lr4(&hi, 200.0, true)).abs() < 0.1);
        assert!(gain(&lr4(&lo, 200.0, true)) < -20.0);
        // The crossover point of an LR4 is -6 dB.
        let x = sine(120.0, 1.0, 1.0);
        assert!((gain(&lr4(&x, 120.0, false)) + 6.0).abs() < 0.3);
    }

    #[test]
    fn low_band_and_width_known_correlations() {
        let l = sine(60.0, 1.0, 0.5);
        let (c, sm, md) = low_band(&l, &l, 120.0);
        assert!(c > 0.9999 && sm < -80.0 && md.abs() < 1e-6, "{c} {sm} {md}");
        // A 60 Hz tone in antiphase: correlation -1, mono sum vanishes.
        let r: Vec<f64> = l.iter().map(|v| -v).collect();
        let (c, _, md) = low_band(&l, &r, 120.0);
        assert!(c < -0.9999 && md < -60.0, "{c} {md}");
        // Width: the same 2 kHz tone on both sides is 1, with a quarter-cycle shift on one side 0.
        let a = sine(2000.0, 1.0, 0.5);
        let b: Vec<f64> = (0..a.len()).map(|i| 0.5 * (TAU * 2000.0 * i as f64 / SR).cos()).collect();
        assert!(width_corr(&a, &a, 200.0) > 0.9999);
        assert!(width_corr(&a, &b, 200.0).abs() < 0.01);
    }

    #[test]
    fn centroid_of_a_tone_and_of_a_mix() {
        let fft = Fft::new(4096);
        let x = sine(1000.0, 0.5, 0.5);
        let b = brightness(&fft, &x);
        assert!((b - 1000.0).abs() < 15.0, "{b}");
        // Equal-power 500 + 2000 Hz: the power-weighted centroid is the arithmetic mean, 1250 Hz.
        let m: Vec<f64> = sine(500.0, 0.5, 0.5).iter().zip(sine(2000.0, 0.5, 0.5)).map(|(a, b)| a + b).collect();
        let b = brightness(&fft, &m);
        assert!((b - 1250.0).abs() < 25.0, "{b}");
    }

    /// A tone that switches between 500 and 2000 Hz every `half` seconds.
    fn alternating(half: f64, secs: f64) -> Vec<f64> {
        let mut ph = 0.0;
        (0..(SR * secs) as usize)
            .map(|i| {
                let f = if ((i as f64 / SR / half) as usize) % 2 == 0 { 500.0 } else { 2000.0 };
                ph += TAU * f / SR;
                0.5 * ph.sin()
            })
            .collect()
    }

    #[test]
    fn move_depth_and_period_of_a_switching_tone() {
        let fft = Fft::new(4096);
        let x = alternating(0.1, 3.0);
        let c = move_centroid(&fft, &x);
        let d = depth_oct(&c);
        assert!((d - 2.0).abs() < 0.2, "500 -> 2000 Hz is 2 octaves, got {d}");
        let lc: Vec<f64> = c.iter().map(|v| v.log2()).collect();
        let r = acf(&detrend(&lc), (SR / MOVE_HOP as f64) as usize);
        let p = dominant_peak(&r, (0.03 * SR / MOVE_HOP as f64) as usize, r.len() - 2).unwrap();
        let secs = p.lag * MOVE_HOP as f64 / SR;
        assert!((secs - 0.2).abs() < 0.2 * 0.05, "period {secs}");
        assert!(p.val > 0.9);
        // The same series is not periodic at 0.3 s: peak_near finds the multiple only at 0.2 or 0.4.
        let near = peak_near(&r, 0.2 * SR / MOVE_HOP as f64, 0.05).unwrap();
        assert!(near.val > 0.9);
    }

    #[test]
    fn acf_period_of_a_synthetic_series() {
        // 12.5 samples per cycle (a 66.7 ms period at 187.5 Hz), 5 % tolerance needs sub-sample lags.
        let s: Vec<f64> = (0..600).map(|i| (TAU * i as f64 / 12.5).sin()).collect();
        let r = acf(&s, 300);
        let p = dominant_peak(&r, 5, 300).unwrap();
        assert!((p.lag - 12.5).abs() < 0.2, "{}", p.lag);
        // Noise has no peak above 0.4.
        let mut st = 12345u32;
        let n: Vec<f64> = (0..600)
            .map(|_| {
                st ^= st << 13;
                st ^= st >> 17;
                st ^= st << 5;
                st as f64 / u32::MAX as f64
            })
            .collect();
        let m = peaks(&acf(&n, 300), 5, 300).iter().map(|p| p.val).fold(0.0, f64::max);
        assert!(m < 0.4, "{m}");
    }

    #[test]
    fn alias_floor_sees_an_inharmonic_tone() {
        let fft = Fft::new(65536);
        let f0 = 43.65;
        let n = 65536;
        let harm: Vec<f64> = (0..n).map(|i| (1..=20).map(|k| (TAU * k as f64 * f0 * i as f64 / SR).sin() / k as f64).sum::<f64>()).collect();
        let (p, df) = bh_power(&fft, &harm);
        assert!(alias_floor_db(&p, df, f0, 0.0) < -60.0, "{}", alias_floor_db(&p, df, f0, 0.0));
        // Add a tone 40 dB below the fundamental halfway between the 22nd and 23rd harmonic (the
        // 1 % bands are narrower than f0 / 2 only below the 50th harmonic).
        let with: Vec<f64> = harm.iter().enumerate().map(|(i, v)| v + 0.01 * (TAU * 22.5 * f0 * i as f64 / SR).sin()).collect();
        let (p, df) = bh_power(&fft, &with);
        let a = alias_floor_db(&p, df, f0, 0.0);
        assert!((a + 40.0).abs() < 1.0, "{a}");
        // Above the 50th harmonic the literal 1 % rule judges nothing: the same tone at 76.5 f0 is not seen.
        let high: Vec<f64> = harm.iter().enumerate().map(|(i, v)| v + 0.01 * (TAU * 76.5 * f0 * i as f64 / SR).sin()).collect();
        let (p, df) = bh_power(&fft, &high);
        assert!(alias_floor_db(&p, df, f0, 0.0) < -60.0);
        // The 3rd harmonic detuned by 12 ct is not an alias when the unison spread is known.
        let det: Vec<f64> = (0..n).map(|i| (TAU * f0 * i as f64 / SR).sin() + 0.1 * (TAU * 3.0 * f0 * 1.0035 * i as f64 / SR).sin()).collect();
        let (p, df) = bh_power(&fft, &det);
        assert!(alias_floor_db(&p, df, f0, 12.0) < -60.0);
    }

    #[test]
    fn band_fraction_and_thd() {
        let fft = Fft::new(65536);
        let f0 = 43.65;
        let x: Vec<f64> = (0..65536).map(|i| (TAU * f0 * i as f64 / SR).sin() + 0.1 * (TAU * 2.0 * f0 * i as f64 / SR).sin()).collect();
        let (p, df) = bh_power(&fft, &x);
        assert!((thd_percent(&p, df, f0, 150.0) - 10.0).abs() < 0.5, "{}", thd_percent(&p, df, f0, 150.0));
        assert!(band_fraction(&p, df, 150.0) > 0.999);
        let mixed: Vec<f64> = x.iter().enumerate().map(|(i, v)| v + (TAU * 1000.0 * i as f64 / SR).sin()).collect();
        let (p, df) = bh_power(&fft, &mixed);
        let f = band_fraction(&p, df, 150.0);
        assert!((f - 0.5 * 1.01 / (0.505 + 0.5)).abs() < 0.02, "{f}");
    }

    #[test]
    fn mel_distance_orders_timbres() {
        let fft = Fft::new(2048);
        let a = mel_descriptor(&fft, &sine(200.0, 1.0, 0.5));
        let a2 = mel_descriptor(&fft, &sine(200.0, 1.0, 0.5));
        let b = mel_descriptor(&fft, &sine(3000.0, 1.0, 0.5));
        let c = mel_descriptor(&fft, &sine(220.0, 1.0, 0.5));
        assert_eq!(mel_distance(&a, &a2), 0.0);
        assert!(mel_distance(&a, &b) > 3.0 * mel_distance(&a, &c), "{} {}", mel_distance(&a, &b), mel_distance(&a, &c));
    }

    #[test]
    fn yin_tracks_a_dive() {
        // 400 Hz falling an octave (12 st) over 100 ms, then steady.
        let mut ph = 0.0;
        let x: Vec<f64> = (0..(SR * 0.6) as usize)
            .map(|i| {
                let t = i as f64 / SR;
                let f = 400.0 * 2f64.powf(-(t / 0.1).min(1.0));
                ph += TAU * f / SR;
                0.5 * ph.sin() + 0.2 * (2.0 * ph).sin()
            })
            .collect();
        let tr = pitch_track(&x, 0, 240, 70, 80.0, 1000.0);
        let (a, b) = (track_at(&tr, 0.012).unwrap(), track_at(&tr, 0.3).unwrap());
        assert!((b - 200.0).abs() < 4.0, "{b}");
        let st = 12.0 * (a / b).log2();
        assert!((st - 11.0).abs() < 1.5, "falls ~11-12 st between 12 ms and the end, got {st}");
    }

    #[test]
    fn envelope_times_and_monotonic() {
        // A level that falls 1 dB per hop: 20 dB takes 20 hops.
        let env: Vec<f64> = (0..100).map(|i| -(i as f64)).collect();
        assert!((fall_time(&env, 0.005, 0, 20.0).unwrap() - 0.1).abs() < 1e-9);
        assert!(fall_time(&env, 0.005, 90, 20.0).is_none());
        let amp: Vec<f64> = (0..100).map(|i| (i as f64 / 50.0).min(1.0)).collect();
        assert!((rise_time(&amp, 0.01, 0.9).unwrap() - 0.45).abs() < 0.011);
        assert!(monotonic(&[0.0, 1.0, 0.99, 2.0], 0.05));
        assert!(!monotonic(&[0.0, 1.0, 0.8, 2.0], 0.05));
        assert!(monotonic(&[2.0, 1.0, 1.02, 0.0], 0.05), "falling with a small rebound");
        let r = rms_env_db(&sine(100.0, 0.5, 1.0), 480, 480);
        assert!(r.iter().all(|v| (v + 3.01).abs() < 0.1));
    }

    #[test]
    fn percentile_and_detrend() {
        let v: Vec<f64> = (0..101).map(|i| i as f64).collect();
        assert!((percentile(&v, 0.95) - 95.0).abs() < 1e-9);
        assert!((percentile(&v, 0.05) - 5.0).abs() < 1e-9);
        let ramp: Vec<f64> = (0..50).map(|i| 3.0 * i as f64 + 7.0).collect();
        assert!(detrend(&ramp).iter().all(|v| v.abs() < 1e-9));
    }
}
