//! SPEC v2 Track 7 "Sound" acceptance (5)-(8) and the CPU half of (6): HP12 / Notch / LP24 /
//! Formant against their design curves, Formant peaks against the vowel table, warp alias floor
//! and extremes, noise slopes and decorrelation, sub triangle harmonics, sub mono, sub phase and
//! octave 0. Figures print with `--nocapture`.
use gts::dsp::filter::{formant_bw, formant_pos, formants, MultiFilter, BW4_K, FORMANT_GAIN, LP24_FB};
use gts::dsp::noise::{Noise, NoiseCfg};
use gts::dsp::tables::Fft;
use gts::dsp::{Engine, Patch};
use gts::params::{FilterType, NoiseType, Routing, SubShape, Table, WarpMode};
use std::f64::consts::{PI, SQRT_2, TAU};
use std::time::Instant;

const SR: f64 = 48000.0;

// ---- helpers ---------------------------------------------------------------------------------

/// Complex number as (re, im).
type C = (f64, f64);
fn mul(a: C, b: C) -> C {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}
fn div(a: C, b: C) -> C {
    let d = b.0 * b.0 + b.1 * b.1;
    ((a.0 * b.0 + a.1 * b.1) / d, (a.1 * b.0 - a.0 * b.1) / d)
}
fn abs(a: C) -> f64 {
    a.0.hypot(a.1)
}

/// `s` of the bilinear transform prewarped at `fc` (what a TPT section with g = tan(pi fc / sr)
/// computes exactly), at `f`.
fn s_at(f: f64, fc: f64) -> C {
    (0.0, (PI * f / SR).tan() / (PI * fc / SR).tan())
}

/// 2nd-order section denominators / numerators.
fn den2(s: C, k: f64) -> C {
    let s2 = mul(s, s);
    (s2.0 + k * s.0 + 1.0, s2.1 + k * s.1)
}

/// SVF damping at res (filter.rs: `(sqrt 2 + 0.05) (1 - res)^1.2 - 0.05`).
fn svf_k(res: f64) -> f64 {
    (SQRT_2 + 0.05) * (1.0 - res).powf(1.2) - 0.05
}

/// Design magnitude of `ty` at cutoff `fc`, `res`, frequency `f`.
fn design(ty: FilterType, fc: f64, res: f64, f: f64) -> f64 {
    match ty {
        FilterType::Hp12 => {
            let s = s_at(f, fc);
            abs(div(mul(s, s), den2(s, svf_k(res))))
        }
        FilterType::Notch => {
            let s = s_at(f, fc);
            let s2 = mul(s, s);
            abs(div((s2.0 + 1.0, s2.1), den2(s, svf_k(res))))
        }
        FilterType::Lp24 => {
            let s = s_at(f, fc);
            let h4 = div((1.0, 0.0), mul(den2(s, BW4_K[0]), den2(s, BW4_K[1])));
            let kf = LP24_FB * res;
            abs(div((h4.0 * (1.0 + kf), h4.1 * (1.0 + kf)), (1.0 + kf * h4.0, kf * h4.1)))
        }
        FilterType::Formant => {
            let (fk, bw) = (formants(formant_pos(fc)), formant_bw(res));
            let amp = [1.0, 0.6, 0.35];
            let mut h = (0.0, 0.0);
            for k in 0..3 {
                let q = fk[k] / bw[k];
                let s = s_at(f, fk[k]);
                let b = div(s, den2(s, 1.0 / q));
                h = (h.0 + FORMANT_GAIN * amp[k] / q * b.0, h.1 + FORMANT_GAIN * amp[k] / q * b.1);
            }
            abs(h)
        }
        _ => unreachable!(),
    }
}

fn mk() -> MultiFilter {
    let mut f = MultiFilter::new();
    f.set_sample_rate(SR);
    f
}

/// Steady-state gain of `f` for a sine at `hz` (amplitude `amp`): single-bin DFT over whole
/// cycles after a 0.5 s settle.
fn gain(f: &mut MultiFilter, hz: f64, amp: f64) -> f64 {
    let settle = 24000;
    let cycles = (hz * 0.5).ceil().max(8.0);
    let n = (cycles * SR / hz).round() as usize;
    let (mut re, mut im) = (0.0, 0.0);
    for i in 0..settle + n {
        let y = f.process((amp * (TAU * hz * i as f64 / SR).sin()) as f32) as f64;
        if i >= settle {
            let p = TAU * hz * i as f64 / SR;
            re += y * p.cos();
            im += y * p.sin();
        }
    }
    2.0 * re.hypot(im) / n as f64 / amp
}

fn db(x: f64) -> f64 {
    20.0 * x.max(1e-12).log10()
}

/// Blackman-Harris power spectrum (power-of-two length) and the bin width.
fn spectrum(x: &[f64]) -> (Vec<f64>, f64) {
    let n = x.len();
    let mut re: Vec<f64> = x
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let t = TAU * i as f64 / (n - 1) as f64;
            v * (0.35875 - 0.48829 * t.cos() + 0.14128 * (2.0 * t).cos() - 0.01168 * (3.0 * t).cos())
        })
        .collect();
    let mut im = vec![0.0; n];
    Fft::new(n).run(&mut re, &mut im, false);
    ((0..=n / 2).map(|k| re[k] * re[k] + im[k] * im[k]).collect(), SR / n as f64)
}

/// Non-harmonic over harmonic energy in dB (tests/alias.rs, SPEC v2 1.1: harmonic = within 6
/// bins of k f0, DC below 15 Hz ignored).
fn alias_db(x: &[f64], f0: f64) -> f64 {
    let (p, df) = spectrum(x);
    let (mut h, mut nh) = (0.0, 0.0);
    for (k, &v) in p.iter().enumerate() {
        let f = k as f64 * df;
        if f <= 15.0 {
            continue;
        }
        let kk = (f / f0).round();
        if kk >= 1.0 && (f - kk * f0).abs() <= 6.0 * df {
            h += v;
        } else {
            nh += v;
        }
    }
    10.0 * (nh / h).max(1e-30).log10()
}

fn hz(note: u8) -> f64 {
    440.0 * 2f64.powf((note as f64 - 69.0) / 12.0)
}

/// The alias measurement patch (tests/alias.rs): Bypass, no Mod A, shaper off, -12 dB.
fn alias_patch(table: Table) -> Patch {
    let mut p = Patch::riddim_v1();
    p.routing = Routing::Bypass;
    p.moda_index = 0.0;
    p.shaper_on = false;
    p.volume_db = -12.0;
    p.osc[0].table = table;
    p.osc[0].index = 64.0;
    p
}

fn render(p: &Patch, note: u8, secs: f64) -> (Vec<f64>, Vec<f64>) {
    let mut e = Engine::new();
    e.initialize(SR as f32);
    e.set_patch(p);
    e.note_on(note, 1.0);
    (0..(secs * SR) as usize)
        .map(|_| {
            let (l, r) = e.next_frame();
            (l as f64, r as f64)
        })
        .unzip()
}

// ---- (5) filters -----------------------------------------------------------------------------

const FREQS: [f64; 10] = [60.0, 130.0, 250.0, 420.0, 700.0, 1400.0, 2300.0, 4100.0, 7000.0, 12000.0];

#[test]
fn new_filter_types_match_their_design_curves_within_1db() {
    let mut worst: f64 = 0.0;
    let cases = [
        (FilterType::Hp12, 1000.0, 0.0),
        (FilterType::Hp12, 1000.0, 0.5),
        (FilterType::Notch, 1000.0, 0.0),
        (FilterType::Notch, 1000.0, 0.5),
        (FilterType::Lp24, 1000.0, 0.0),
        (FilterType::Lp24, 1000.0, 0.5),
        (FilterType::Lp24, 1000.0, 0.85),
        (FilterType::Formant, 20.0 * 1000f64.powf(0.25 * 0.0), 0.5),
        (FilterType::Formant, 20.0 * 1000f64.powf(0.25 * 1.5), 0.5),
        (FilterType::Formant, 20.0 * 1000f64.powf(0.25 * 3.0), 0.8),
    ];
    for (ty, fc, res) in cases {
        let mut row = vec![];
        for f in FREQS {
            let mut flt = mk();
            flt.set(ty, fc as f32, res as f32);
            let (got, want) = (db(gain(&mut flt, f, 0.01)), db(design(ty, fc, res, f)));
            // A notch / stopband deeper than -60 dB only has to stay that deep.
            let err = if want < -60.0 { (got - want).max(0.0).min((got + 60.0).max(0.0)) } else { got - want };
            row.push(format!("{f:.0}:{got:+.1}/{want:+.1}"));
            worst = worst.max(err.abs());
            assert!(err.abs() <= 1.0, "{ty:?} fc {fc:.0} res {res} at {f} Hz: {got:.2} dB vs design {want:.2} dB");
        }
        println!("{ty:?} fc {fc:.0} res {res}: {}", row.join(" "));
    }
    println!("worst deviation from the design curves: {worst:.3} dB (limit 1 dB)");
}

/// Magnitude response of `f` from its impulse response (linear filters only).
fn impulse_response(f: &mut MultiFilter, n: usize) -> (Vec<f64>, f64) {
    let mut re: Vec<f64> = (0..n).map(|i| f.process(if i == 0 { 1.0 } else { 0.0 }) as f64).collect();
    let mut im = vec![0.0; n];
    Fft::new(n).run(&mut re, &mut im, false);
    ((0..=n / 2).map(|k| re[k].hypot(im[k])).collect(), SR / n as f64)
}

#[test]
fn formant_peaks_sit_within_5_percent_of_the_vowel_table() {
    const VOWELS: [[f64; 3]; 5] =
        [[730.0, 1090.0, 2440.0], [530.0, 1840.0, 2480.0], [270.0, 2290.0, 3010.0], [570.0, 840.0, 2410.0], [300.0, 870.0, 2240.0]];
    let mut worst: f64 = 0.0;
    for res in [0.5, 1.0] {
        for (v, want) in VOWELS.iter().enumerate() {
            let fc = 20.0 * 1000f64.powf(v as f64 / 4.0);
            let mut f = mk();
            f.set(FilterType::Formant, fc as f32, res as f32);
            let (h, df) = impulse_response(&mut f, 1 << 16);
            // Local maxima between 150 Hz and 4 kHz, the three largest, in frequency order.
            let mut peaks: Vec<(f64, f64)> = (1..h.len() - 1)
                .filter(|&k| h[k] > h[k - 1] && h[k] >= h[k + 1] && (150.0..4000.0).contains(&(k as f64 * df)))
                .map(|k| (h[k], k as f64 * df))
                .collect();
            peaks.sort_by(|a, b| b.0.total_cmp(&a.0));
            let mut fs: Vec<f64> = peaks.iter().take(3).map(|p| p.1).collect();
            fs.sort_by(f64::total_cmp);
            assert_eq!(fs.len(), 3, "vowel {v} res {res}: {fs:?}");
            let errs: Vec<f64> = fs.iter().zip(want).map(|(g, w)| (g / w - 1.0).abs()).collect();
            println!("vowel {} res {res}: peaks {:?} Hz vs {want:?} (max error {:.2} %)", "AEIOU".as_bytes()[v] as char,
                fs.iter().map(|f| f.round()).collect::<Vec<_>>(), 100.0 * errs.iter().cloned().fold(0.0, f64::max));
            for e in &errs {
                worst = worst.max(*e);
                assert!(*e <= 0.05, "vowel {v} res {res}: {fs:?} vs {want:?}");
            }
        }
    }
    println!("worst formant peak error {:.2} % (limit 5 %)", 100.0 * worst);
}

#[test]
fn lp24_self_oscillation_is_a_bounded_clean_sine() {
    for fc in [220.0, 1000.0, 4000.0] {
        let mut f = mk();
        f.set(FilterType::Lp24, fc as f32, 1.0);
        let n = 2 * 48000;
        let y: Vec<f64> = (0..n).map(|i| f.process(if i == 0 { 1e-4 } else { 0.0 }) as f64).collect();
        let t = &y[n - 32768..];
        let peak = t.iter().fold(0.0f64, |a, v| a.max(v.abs()));
        let (p, df) = spectrum(t);
        let k0 = (1..p.len()).max_by(|&a, &b| p[a].total_cmp(&p[b])).unwrap();
        let band = |k: usize| p[k.saturating_sub(8)..(k + 9).min(p.len())].iter().sum::<f64>();
        let fund = band(k0);
        let harm: f64 = (2..=10).map(|h| h * k0).filter(|&k| k + 9 < p.len()).map(band).sum();
        let thd = (harm / fund).sqrt();
        println!("LP24 res 1 at {fc} Hz: oscillates at {:.0} Hz, peak {peak:.3}, THD {:.2} %", k0 as f64 * df, 100.0 * thd);
        assert!(peak > 0.05 && peak <= 1.0, "{fc}: peak {peak}");
        assert!(thd <= 0.03, "{fc}: THD {thd}");
        assert!(((k0 as f64 * df) / fc - 1.0).abs() < 0.05, "{fc}: pitch {}", k0 as f64 * df);
    }
}

#[test]
fn new_filter_types_sweep_at_res_1_stays_bounded() {
    // tests/filter.rs `svf_res1_sweep_stays_below_0dbfs` for the new types: a full-level
    // (RMS 0.25) 64-harmonic saw at C2 under a 30 Hz -> 15 kHz sweep.
    let n = 3 * 48000;
    let f0 = 65.406;
    let raw: Vec<f64> = (0..n).map(|i| (1..=64).map(|h| (TAU * h as f64 * f0 * i as f64 / SR).sin() / h as f64).sum()).collect();
    let g = 0.25 / (raw.iter().map(|v| v * v).sum::<f64>() / n as f64).sqrt();
    for ty in [FilterType::Hp12, FilterType::Notch, FilterType::Lp24, FilterType::Formant] {
        for res in [0.0, 0.9, 1.0] {
            let mut f = mk();
            let mut peak = 0.0f32;
            for (i, v) in raw.iter().enumerate() {
                if i % 16 == 0 {
                    f.set(ty, (30.0 * (15000.0f64 / 30.0).powf(i as f64 / n as f64)) as f32, res);
                }
                let y = f.process((v * g) as f32);
                assert!(y.is_finite());
                peak = peak.max(y.abs());
            }
            println!("{ty:?} res {res}: sweep peak {peak:.3} ({:+.1} dBFS)", 20.0 * peak.log10());
            // LP24 and Formant stay under 0 dBFS like LP12 (SPEC 1.8). HP12 / Notch pass the saw's
            // edge itself (0.9 at res 0), so they are held to "bounded" (+3.5 dBFS).
            let limit = if matches!(ty, FilterType::Lp24 | FilterType::Formant) { 1.0 } else { 1.5 };
            assert!(peak <= limit, "{ty:?} res {res}: {peak}");
        }
    }
}

// ---- (6) warp --------------------------------------------------------------------------------

fn warp_patch(table: Table, mode: WarpMode, amt: f32) -> Patch {
    let mut p = alias_patch(table);
    p.osc[0].warp = mode;
    p.osc[0].warp_amt = amt;
    // Osc B is the FM / sync source at the same pitch, not heard.
    p.osc[1].on = false;
    p
}

#[test]
fn warp_alias_floor_c1_to_c5_at_amount_half() {
    let mut worst = f64::MIN;
    let mut rows = vec![];
    for mode in [WarpMode::Fm, WarpMode::Sync, WarpMode::BendPlus, WarpMode::BendMinus, WarpMode::Pwm] {
        for table in [Table::SquareMorph, Table::SawStack] {
            let row: Vec<f64> = (0..5)
                .map(|o| {
                    let note = 24 + 12 * o;
                    let (x, _) = render(&warp_patch(table, mode, 0.5), note, 1.65);
                    alias_db(&x[12000..12000 + 65536], hz(note))
                })
                .collect();
            let line = format!("{mode:?} {table:?}: C1..C5 {}", row.iter().map(|v| format!("{v:6.1}")).collect::<Vec<_>>().join(" "));
            println!("{line}");
            rows.push((mode, row.iter().cloned().fold(f64::MIN, f64::max)));
            worst = worst.max(row.iter().cloned().fold(f64::MIN, f64::max));
        }
    }
    println!("worst warp alias floor: {worst:.1} dB (target <= -70 dB)");
    for (mode, w) in rows {
        assert!(w <= -70.0, "{mode:?}: alias floor {w:.1} dB");
    }
}

#[test]
fn warp_never_produces_nan_at_extremes() {
    for mode in [WarpMode::Fm, WarpMode::Sync, WarpMode::BendPlus, WarpMode::BendMinus, WarpMode::Pwm] {
        for amt in [0.0, 1.0] {
            for (note, shift, unison) in [(0u8, -24.0, 7usize), (127, 24.0, 7), (60, 0.0, 1)] {
                let mut p = warp_patch(Table::Metal, mode, amt);
                p.osc[0].shift = shift;
                p.osc[0].unison = unison;
                p.osc[0].width = 1.0;
                p.osc[1].on = true;
                p.osc[1].warp = mode;
                p.osc[1].warp_amt = amt;
                p.osc[1].oct = 4;
                let (l, r) = render(&p, note, 0.3);
                let peak = l.iter().chain(&r).fold(0.0f64, |a, v| {
                    assert!(v.is_finite(), "{mode:?} amt {amt} note {note}: {v}");
                    a.max(v.abs())
                });
                assert!(peak <= 1.0, "{mode:?} amt {amt} note {note}: peak {peak}");
            }
        }
    }
}

#[test]
fn warp_at_amount_zero_is_transparent() {
    // Bend / PWM at 0 are the identity map and Sync at 0 restarts on its own period: the warped
    // osc then equals the plain one up to the decimator (0.1 dB ripple, a few samples of delay).
    for mode in [WarpMode::Fm, WarpMode::Sync, WarpMode::BendPlus, WarpMode::Pwm] {
        let (x, _) = render(&warp_patch(Table::SawStack, mode, 0.0), 36, 1.0);
        let (y, _) = render(&warp_patch(Table::SawStack, WarpMode::Off, 0.0), 36, 1.0);
        let rms = |v: &[f64]| (v.iter().map(|a| a * a).sum::<f64>() / v.len() as f64).sqrt();
        let d = db(rms(&x[12000..]) / rms(&y[12000..]));
        println!("{mode:?} amount 0: level {d:+.2} dB re Off");
        assert!(d.abs() <= 0.5, "{mode:?}: {d:+.2} dB");
    }
}

/// Seconds to render `secs` of `p` with `notes` held (best of three).
fn cpu(p: &Patch, notes: &[u8], secs: f64) -> f64 {
    let mut best = f64::MAX;
    for _ in 0..3 {
        let mut e = Engine::new();
        e.initialize(SR as f32);
        e.set_patch(p);
        for &n in notes {
            e.note_on(n, 1.0);
        }
        let t = Instant::now();
        let mut acc = 0.0f32;
        for i in 0..(secs * SR) as usize {
            if i % 512 == 0 {
                e.set_patch(p);
            }
            acc += e.next_frame().0;
        }
        assert!(acc.is_finite());
        best = best.min(t.elapsed().as_secs_f64());
    }
    best
}

#[test]
#[ignore = "CPU budget: wall-clock timing is unreliable under parallel load; run alone with --ignored --test-threads=1 (tools/CHECKLIST-gts.md)"]
fn warp_cpu_per_voice_and_at_8_voices() {
    // Report: one voice, Osc A warped (unison 1 and 7), % of one core at 48 kHz.
    let secs = 2.0;
    let mut report = vec![];
    for unison in [1usize, 7] {
        let mut off = 0.0;
        for mode in [WarpMode::Off, WarpMode::Fm, WarpMode::Sync, WarpMode::BendPlus, WarpMode::Pwm] {
            let mut p = warp_patch(Table::SawStack, mode, 0.5);
            p.osc[0].unison = unison;
            let pct = 100.0 * cpu(&p, &[48], secs) / secs;
            if mode == WarpMode::Off {
                off = pct;
            }
            report.push(format!("unison {unison} {mode:?} {pct:.2} % ({:+.2})", pct - off));
        }
    }
    println!("CPU, one voice (% of a core, 48 kHz): {}", report.join(", "));
    // SPEC acceptance (10), warp part: 8 voices (both oscs on, Osc A warped at 0.5 in each mode)
    // against the same patch unwarped: at most +8 % of a core (release).
    let mut p = warp_patch(Table::SawStack, WarpMode::Off, 0.5);
    p.poly = 8;
    p.osc[1].on = true;
    let notes = [36, 40, 43, 47, 48, 52, 55, 59];
    let plain = cpu(&p, &notes, secs);
    let mut worst = (f64::MIN, WarpMode::Off);
    let mut line = vec![];
    for mode in [WarpMode::Fm, WarpMode::Sync, WarpMode::BendPlus, WarpMode::Pwm] {
        p.osc[0].warp = mode;
        let over = 100.0 * (cpu(&p, &notes, secs) - plain) / secs;
        line.push(format!("{mode:?} {over:+.2} %"));
        if over > worst.0 {
            worst = (over, mode);
        }
    }
    println!("8 voices, unwarped {:.1} % of a core; Osc A warped: {}", 100.0 * plain / secs, line.join(", "));
    if !cfg!(debug_assertions) {
        assert!(worst.0 <= 8.0, "{:?}: warp overhead {:.2} % of a core", worst.1, worst.0);
    }
}

// ---- (7) noise -------------------------------------------------------------------------------

/// Octave-band powers (dB) of `x` centred on 125 Hz .. 8 kHz, from an averaged periodogram.
fn octave_bands(x: &[f32]) -> Vec<f64> {
    let n = 8192;
    let mut psd = vec![0.0; n / 2 + 1];
    for seg in x.chunks_exact(n) {
        let mut re: Vec<f64> =
            seg.iter().enumerate().map(|(i, &v)| v as f64 * (0.5 - 0.5 * (TAU * i as f64 / n as f64).cos())).collect();
        let mut im = vec![0.0; n];
        Fft::new(n).run(&mut re, &mut im, false);
        for k in 0..=n / 2 {
            psd[k] += re[k] * re[k] + im[k] * im[k];
        }
    }
    let df = SR / n as f64;
    (0..7)
        .map(|o| {
            let c = 125.0 * 2f64.powi(o);
            let (lo, hi) = (c / SQRT_2, c * SQRT_2);
            let p: f64 = psd.iter().enumerate().filter(|(k, _)| (lo..hi).contains(&(*k as f64 * df))).map(|(_, v)| v).sum();
            10.0 * p.log10()
        })
        .collect()
}

/// Least-squares slope in dB per octave.
fn slope(b: &[f64]) -> f64 {
    let n = b.len() as f64;
    let mx = (n - 1.0) / 2.0;
    let my = b.iter().sum::<f64>() / n;
    let num: f64 = b.iter().enumerate().map(|(i, y)| (i as f64 - mx) * (y - my)).sum();
    let den: f64 = (0..b.len()).map(|i| (i as f64 - mx).powi(2)).sum();
    num / den
}

#[test]
fn noise_slopes_white_pink_brown() {
    for (ty, want) in [(NoiseType::White, 0.0), (NoiseType::Pink, -3.0), (NoiseType::Brown, -6.0)] {
        let cfg = NoiseCfg::new(ty, 0.0, 0.5, SR as f32);
        let mut g = Noise::new(0xDEAD_BEEF);
        let x: Vec<f32> = (0..1 << 21).map(|_| g.next(&cfg)).collect();
        let rms = (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64).sqrt();
        let b = octave_bands(&x);
        let s = slope(&b);
        // Octave bands double in width per octave: a PSD slope of p dB/oct gives band powers
        // rising p + 3 dB/oct.
        let psd_slope = s - 3.0103;
        let dev = b.iter().enumerate().map(|(i, v)| v - (b[0] + s * i as f64)).fold(0.0f64, |a, d| a.max(d.abs()));
        println!("{ty:?}: RMS {rms:.4}, spectrum slope {psd_slope:+.2} dB/oct (want {want:+.1}), max band deviation from the line {dev:.2} dB");
        assert!((psd_slope - want).abs() <= 1.0, "{ty:?}: {psd_slope:+.2} dB/oct");
        assert!(dev <= 1.0, "{ty:?}: band deviation {dev:.2} dB");
        assert!((db(rms / 0.25)).abs() <= 1.0, "{ty:?}: RMS {rms}");
    }
}

#[test]
fn noise_tone_darkens_and_brightens() {
    let band = |tone: f32| {
        let cfg = NoiseCfg::new(NoiseType::White, 0.0, tone, SR as f32);
        let mut g = Noise::new(7);
        let x: Vec<f32> = (0..1 << 18).map(|_| g.next(&cfg)).collect();
        let b = octave_bands(&x);
        (b[0], b[6])
    };
    let (flat, dark, bright) = (band(0.5), band(0.0), band(1.0));
    println!("125 Hz / 8 kHz bands: flat {flat:.1?}, tone 0 {dark:.1?}, tone 1 {bright:.1?}");
    // Tone 0 (one-pole low-pass at 200 Hz) tilts the 8 kHz band down against the 125 Hz band,
    // tone 1 (one-pole high-pass at 10 kHz) the other way, each by more than 20 dB.
    assert!((flat.0 - flat.1) - (dark.0 - dark.1) < -20.0);
    assert!((flat.1 - flat.0) - (bright.1 - bright.0) < -20.0);
}

fn noise_patch(level: f32) -> Patch {
    let mut p = Patch::default();
    p.osc[0].on = false;
    p.sub_on = false;
    p.routing = Routing::Bypass;
    p.noise_level_db = level;
    p
}

#[test]
fn noise_layer_is_stereo_decorrelated_and_follows_amp_env_a() {
    let (l, r) = render(&noise_patch(-6.0), 48, 1.0);
    let (l, r) = (&l[4800..], &r[4800..]);
    let dot = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
    let corr = dot(l, r) / (dot(l, l) * dot(r, r)).sqrt();
    println!("noise L/R correlation {corr:.4}");
    assert!(corr.abs() <= 0.1, "{corr}");
    // Off by default; after note-off it fades with Amp Env A (release 20 ms) and the voice ends.
    let mut e = Engine::new();
    e.initialize(SR as f32);
    let p = noise_patch(-6.0);
    e.set_patch(&p);
    e.note_on(48, 1.0);
    for _ in 0..4800 {
        e.next_frame();
    }
    e.note_off(48);
    for _ in 0..9600 {
        e.next_frame();
    }
    assert_eq!(e.active_voices(), 0);
    let (l, _) = render(&noise_patch(-60.0), 48, 0.2);
    assert!(l.iter().all(|v| *v == 0.0), "noise at -60 dB must be off");
}

// ---- (8) sub ---------------------------------------------------------------------------------

fn sub_patch(shape: SubShape, oct: i32) -> Patch {
    let mut p = Patch::default();
    p.osc[0].on = false;
    p.routing = Routing::Bypass;
    p.sub_on = true;
    p.sub_level_db = 0.0;
    p.sub_shape = shape;
    p.sub_oct = oct;
    p.volume_db = -6.0;
    p
}

#[test]
fn sub_triangle_odd_harmonics_fall_as_1_over_k_squared() {
    // C3 at octave 0: 130.8 Hz, where the 25 Hz low-cut costs < 0.01 dB.
    let (x, _) = render(&sub_patch(SubShape::Triangle, 0), 48, 1.8);
    let f0 = hz(48);
    let (p, df) = spectrum(&x[12000..12000 + 65536]);
    let amp = |f: f64| {
        let k = (f / df).round() as usize;
        p[k - 6..=k + 6].iter().sum::<f64>().sqrt()
    };
    let a1 = amp(f0);
    for k in [3, 5, 7, 9, 11] {
        let d = db(amp(k as f64 * f0) / a1) - db(1.0 / (k * k) as f64);
        println!("triangle harmonic {k}: {d:+.3} dB re 1/k^2");
        assert!(d.abs() <= 0.5, "harmonic {k}: {d:+.3} dB");
    }
    for k in [2, 4, 6] {
        let e = db(amp(k as f64 * f0) / a1);
        assert!(e < -60.0, "even harmonic {k}: {e:.1} dB");
    }
}

#[test]
fn sub_stays_mono_with_drive_under_a_wide_osc() {
    let mut p = sub_patch(SubShape::Triangle, -1);
    p.osc[0].on = true;
    p.osc[0].unison = 7;
    p.osc[0].width = 1.0;
    p.sub_drive = 1.0;
    let side = |p: &Patch| {
        let (l, r) = render(p, 36, 0.5);
        l.iter().zip(&r).map(|(a, b)| a - b).collect::<Vec<f64>>()
    };
    let with = side(&p);
    p.sub_on = false;
    let without = side(&p);
    let d = with.iter().zip(&without).fold(0.0f64, |a, (x, y)| a.max((x - y).abs()));
    let s = with.iter().fold(0.0f64, |a, v| a.max(v.abs()));
    println!("L-R with vs without the driven sub: max difference {d:.2e} (side peak {s:.3})");
    assert!(s > 0.01 && d <= 1e-5, "{d}");
}

#[test]
fn sub_phase_and_octave_zero() {
    // Octave 0 plays at Osc A's pitch; phase 180 inverts the sine.
    let (a, _) = render(&sub_patch(SubShape::Sine, 0), 48, 0.5);
    let mut q = sub_patch(SubShape::Sine, 0);
    q.sub_phase = 180.0;
    let (b, _) = render(&q, 48, 0.5);
    let sum = a.iter().zip(&b).fold(0.0f64, |m, (x, y)| m.max((x + y).abs()));
    let peak = a.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    println!("phase 0 + phase 180: max |sum| {sum:.2e} (peak {peak:.3})");
    assert!(sum <= 1e-3 * peak);
    let t = &a[12000..];
    let ups = t.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count() as f64;
    let f = ups / (t.len() as f64 / SR);
    println!("octave 0 sub at C3: {f:.1} Hz (osc {:.1})", hz(48));
    assert!((f / hz(48) - 1.0).abs() < 0.02, "{f}");
}

#[test]
fn mip_harmonics_table_matches_its_formula() {
    for m in 0..gts::dsp::tables::MIPS {
        let want = (512.0 * 2f64.powf(-(m as f64) / 4.0)).round() as usize;
        assert_eq!(gts::dsp::tables::harmonics(m), want, "mip {m}");
    }
}
