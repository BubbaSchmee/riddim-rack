//! Procedural graintables: 9 tables x 32 grains, each grain mip-mapped four levels per octave.
//!
//! Every grain is one cycle defined by harmonic coefficients 1..=512 (FM is computed in the time
//! domain and transformed), then rendered by inverse FFT into `MIPS` band-limited levels: mip `m`
//! keeps harmonics `1..=harmonics(m)` (512, 431, 362, 304, 256, ..., 1: quarter-octave steps).
//! The oscillator picks the mip from the effective read rate `f0 * 2^(shift/12)` so the highest
//! kept harmonic stays at or below `TOP` x the running sample rate (SQ-1..5), so at every note the
//! top harmonic lands within a quarter octave under `TOP`. Levels are read with a cubic B-spline
//! on prefiltered samples (an exact interpolator whose images fall as `(nu / (1 - nu))^4`): at 8
//! samples per top harmonic the images sit 67 dB under that harmonic, below the 4-point Hermite
//! at 16 (63 dB) it replaced (SQ-7). Levels in octave `o` hold `4096 >> o` samples, never under
//! 128. Positions are always in frame units (0..N).
//! Each grain is DC-free and RMS-normalised to 0.25 on its 2048-sample level (`grain`), peak
//! capped at 1.0; all levels share that gain. Neighbouring grains share phase structure so
//! Index crossfades never cancel.
//! Mip selection scales with the running sample rate (`mip_for(rate, top)`), so nothing has to be
//! regenerated when the rate changes.
//!
//! Also built here, once per grain: `level`, the output-level compensation of the grain scheme
//! over Shift (SPEC-v2 1.9) times a per-table K-weighted loudness trim (SPEC-v2 1.10).

use super::grain::marker_amp;
use crate::params::Table;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

/// Frame length: samples per grain at the reference level (positions are in these units).
pub const N: usize = 2048;
/// Grains per table.
pub const GRAINS: usize = 32;
/// Number of tables (matches `params::Table`).
pub const TABLES: usize = 9;
/// Mip levels, four per octave from `HMAX` down to 1 harmonic.
pub const MIPS: usize = 37;
/// Harmonics in mip 0.
pub const HMAX: usize = 512;
/// Highest kept harmonic in cycles per sample at the running sample rate (SQ-4): 20.3 kHz at
/// 44.1 kHz, 22.1 kHz at 48 kHz. (0.45 left C4 at 44.1 kHz 0.2 % short of its 76-harmonic level,
/// at 64 harmonics = 16.7 kHz; the alias floor is the same at both.)
pub const TOP: f64 = 0.46;
/// The mip `grain()` returns: 256 harmonics in exactly `N` samples.
const GRAIN_MIP: usize = 4;
/// Fundamental at which the Hz-based tables (Vowel, Reed) are voiced. Shift moves them from here.
const REF_F0: f32 = 65.0;

/// Harmonics kept by mip `m`: `round(HMAX * 2^(-m/4))`.
pub fn harmonics(m: usize) -> usize {
    (HMAX as f64 * 2f64.powf(-(m as f64) / 4.0)).round() as usize
}

/// Samples stored for mip `m`: 8 per top harmonic of its octave, never under 128.
pub const fn mip_len(m: usize) -> usize {
    let l = 8 * (HMAX >> (m / 4));
    if l > 128 { l } else { 128 }
}

/// Start of each mip inside a grain's storage (`MIP_OFF[MIPS]` = the grain stride).
const MIP_OFF: [usize; MIPS + 1] = {
    let (mut a, mut i) = ([0; MIPS + 1], 0);
    while i < MIPS {
        a[i + 1] = a[i] + mip_len(i);
        i += 1;
    }
    a
};

/// Samples per grain over all mips (33 920, so 39 MB for 9 tables).
const STRIDE: usize = MIP_OFF[MIPS];

/// Level-compensation grid: one point per semitone of Shift, `SHIFT_LO..=-SHIFT_LO`.
/// ponytail: modulation can push Shift past +-24; the lookup holds the edge value there.
const SHIFT_LO: i32 = -24;
const SHIFTS: usize = (1 - 2 * SHIFT_LO) as usize;

/// Peterson & Barney F1..F3 (Hz) for A, E, I, O, U (same values as VSS).
const VOWELS: [[f32; 3]; 5] = [
    [730.0, 1090.0, 2440.0],
    [530.0, 1840.0, 2480.0],
    [270.0, 2290.0, 3010.0],
    [570.0, 840.0, 2410.0],
    [300.0, 870.0, 2240.0],
];

/// Harmonic coefficients, `c.0 * cos(h t) + c.1 * sin(h t)`, index = harmonic number (0 unused).
type Coefs = [(f32, f32); HMAX + 1];

pub struct Tables {
    data: Box<[f32]>,
    /// Shift level compensation x table trim, `[table][grain][semitone]`.
    level: Box<[f32]>,
    ready: bool,
}

impl Default for Tables {
    fn default() -> Self {
        Self::new()
    }
}

/// Mip for a read at `rate` table cycles per output sample (`f0 * ratio / sr`): the most
/// harmonics whose top one stays at or below `top` cycles per sample.
#[inline]
pub fn mip_for(rate: f64, top: f64) -> usize {
    let mut m = 0;
    while m < MIPS - 1 && harmonics(m) as f64 * rate > top {
        m += 1;
    }
    m
}

impl Tables {
    /// Allocates (zeroed) storage; call `generate` before reading.
    pub fn new() -> Self {
        Self {
            data: vec![0.0; TABLES * GRAINS * STRIDE].into_boxed_slice(),
            level: vec![1.0; TABLES * GRAINS * SHIFTS].into_boxed_slice(),
            ready: false,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }

    fn frame(&self, table: usize, g: usize, m: usize) -> &[f32] {
        let base = (table * GRAINS + g) * STRIDE;
        &self.data[base + MIP_OFF[m]..base + MIP_OFF[m + 1]]
    }

    /// One grain (`N` samples, harmonics 1..=256) of `table`, for display and tests. The samples
    /// are B-spline prefiltered (harmonic 256 is 0.9 dB up), close enough to draw.
    pub fn grain(&self, table: Table, g: usize) -> &[f32] {
        self.frame(table as usize, g, GRAIN_MIP)
    }

    /// Mip `m` of one grain (`mip_len(m)` samples).
    pub fn mip(&self, table: Table, g: usize, m: usize) -> &[f32] {
        self.frame(table as usize, g, m)
    }

    /// Fills the storage (allocates scratch; run off the audio thread). Deterministic.
    pub fn generate(&mut self) {
        for t in 0..TABLES {
            self.generate_table(t);
        }
        self.calibrate();
        self.ready = true;
    }

    /// Renders every grain and mip of table `t`.
    pub fn generate_table(&mut self, t: usize) {
        // Grit: per-harmonic random amplitude and phase, shared by every grain of the table.
        let mut rng = 0x9E37_79B9u32;
        let mut rnd: Coefs = [(0.0, 0.0); HMAX + 1];
        for r in rnd.iter_mut().skip(1) {
            *r = (0.5 + 0.5 * xorshift(&mut rng), TAU * xorshift(&mut rng));
        }
        let fft = Fft::new(mip_len(0));
        let mut re = vec![0f64; mip_len(0)];
        let mut im = vec![0f64; mip_len(0)];
        // Two grains per complex IFFT: X = A + iB renders grain g in the real part, g + 1 in the
        // imaginary part.
        for g in (0..GRAINS).step_by(2) {
            let mut c: [Coefs; 2] = [[(0.0, 0.0); HMAX + 1]; 2];
            for (j, c) in c.iter_mut().enumerate() {
                let x = (g + j) as f32 / (GRAINS - 1) as f32;
                match t {
                    0 => square(x, c),
                    1 => saw_stack(x, c),
                    2 => harmonic_rise(x, c),
                    3 => vowel(x, c),
                    4 => fm(x, c, &fft, &mut re[..N], &mut im[..N]),
                    5 => metal(x, c),
                    6 => reed(x, c),
                    7 => grit(x, &rnd, c),
                    _ => basic(x, c),
                }
            }
            for m in 0..MIPS {
                let l = mip_len(m);
                let (re, im) = (&mut re[..l], &mut im[..l]);
                re.fill(0.0);
                im.fill(0.0);
                // Hermitian spectra: X[h] = (cc - i ss)/2 renders cc cos + ss sin. The B-spline
                // prefilter divides harmonic h by the sampled kernel's response (4 + 2 cos w) / 6.
                for h in 1..=harmonics(m) {
                    let ((ac, as_), (bc, bs)) = (c[0][h], c[1][h]);
                    let pf = 3.0 / (4.0 + 2.0 * (std::f64::consts::TAU * h as f64 / l as f64).cos());
                    let (ac, as_, bc, bs) = (pf * ac as f64, pf * as_ as f64, pf * bc as f64, pf * bs as f64);
                    re[h] = ac + bs;
                    im[h] = bc - as_;
                    re[l - h] = ac - bs;
                    im[l - h] = bc + as_;
                }
                fft.run(re, im, true);
                for (j, part) in [&*re, &*im].into_iter().enumerate() {
                    let out = &mut self.data[(t * GRAINS + g + j) * STRIDE + MIP_OFF[m]..][..l];
                    out.iter_mut().zip(part.iter()).for_each(|(o, &r)| *o = r as f32);
                }
            }
        }
        for g in 0..GRAINS {
            let base = (t * GRAINS + g) * STRIDE;
            // One gain for every level: RMS 0.25 on the N-sample level, peak capped at 1.0.
            let ref_frame = &self.data[base + MIP_OFF[GRAIN_MIP]..][..N];
            let rms = (ref_frame.iter().map(|&v| v as f64 * v as f64).sum::<f64>() / N as f64).sqrt() as f32;
            if rms > 0.0 {
                let peak = ref_frame.iter().fold(0f32, |m, o| m.max(o.abs())) * GRAIN_RMS / rms;
                let gain = GRAIN_RMS / rms / peak.max(1.0);
                self.data[base..base + STRIDE].iter_mut().for_each(|o| *o *= gain);
            }
        }
    }

    /// Builds `level`: per grain, the gain that brings the grain scheme's output (DC removed,
    /// as the voice's DC blocker does) back to the shift-0 RMS at every semitone of Shift, times
    /// a per-table trim that equalises K-weighted loudness at C2 index 64 (BS.1770 filter).
    fn calibrate(&mut self) {
        const S: usize = 128; // samples per simulated period
        const HARM: f64 = 56.0; // output harmonics kept below S/2, so the mean square is exact
        let (n, pi) = (N as f64, std::f64::consts::PI);
        let w: Vec<f64> = (0..S).map(|k| 0.5 - 0.5 * (pi * k as f64 / S as f64).cos()).collect();
        let sn: Vec<f64> = (0..S).map(|k| (2.0 * pi * k as f64 / S as f64).sin()).collect();
        let mut level = vec![1f32; TABLES * GRAINS * SHIFTS];
        for t in 0..TABLES {
            for g in 0..GRAINS {
                let mut ref_ms = [-1f64; MIPS];
                for si in 0..SHIFTS {
                    let ratio = 2f64.powf((si as i32 + SHIFT_LO) as f64 / 12.0);
                    let mut m = 0;
                    while m < MIPS - 1 && harmonics(m) as f64 * ratio.max(1.0) > HARM {
                        m += 1;
                    }
                    let f = self.frame(t, g, m);
                    if ref_ms[m] < 0.0 {
                        ref_ms[m] = f.iter().map(|&v| v as f64 * v as f64).sum::<f64>() / f.len() as f64;
                    }
                    let (amp, scale) = (marker_amp(ratio), f.len() as f64 / N as f64);
                    let (mut s1, mut s2) = (0f64, 0f64);
                    for k in 0..S {
                        let u = k as f64 / S as f64;
                        let mk = amp * sn[k];
                        let a = bspline(f, ((u - 1.0) * n * ratio + mk).rem_euclid(n) * scale) as f64;
                        let b = bspline(f, (u * n * ratio + mk).rem_euclid(n) * scale) as f64;
                        let y = w[k] * a + (1.0 - w[k]) * b;
                        s1 += y;
                        s2 += y * y;
                    }
                    let ac = s2 / S as f64 - (s1 / S as f64).powi(2);
                    level[(t * GRAINS + g) * SHIFTS + si] = if ac > 1e-12 { (ref_ms[m] / ac).sqrt().clamp(0.25, 8.0) as f32 } else { 1.0 };
                }
            }
        }
        // Trims: K-weighted power of the index-64 mix at C2 (65.41 Hz, 48 kHz, the C2 mip).
        let f0 = 65.406;
        let m = mip_for(f0 / 48_000.0, TOP);
        let l = mip_len(m);
        let gp = 64.0 / 127.0 * (GRAINS - 1) as f64;
        let (g0, gf) = (gp as usize, gp.fract());
        let mut power = [0f64; TABLES];
        let (mut re, mut im, fft) = (vec![0f64; l], vec![0f64; l], Fft::new(l));
        for (t, p) in power.iter_mut().enumerate() {
            let (a, b) = (self.frame(t, g0, m), self.frame(t, g0 + 1, m));
            for i in 0..l {
                re[i] = a[i] as f64 * (1.0 - gf) + b[i] as f64 * gf;
                im[i] = 0.0;
            }
            fft.run(&mut re, &mut im, false);
            *p = (1..l / 2).map(|h| (re[h] * re[h] + im[h] * im[h]) * k_weight_sq(h as f64 * f0)).sum();
        }
        let mean_log = power.iter().map(|p| p.ln()).sum::<f64>() / TABLES as f64;
        for (t, p) in power.iter().enumerate() {
            let trim = ((mean_log - p.ln()).exp().sqrt()) as f32;
            level[t * GRAINS * SHIFTS..(t + 1) * GRAINS * SHIFTS].iter_mut().for_each(|v| *v *= trim);
        }
        self.level.copy_from_slice(&level);
    }

    /// Output gain for `table` at grain position `gp` and `shift` semitones (level compensation
    /// x loudness trim), interpolated over grain and semitone.
    #[inline]
    pub fn level(&self, table: usize, gp: f32, shift: f64) -> f32 {
        let s = (shift - SHIFT_LO as f64).clamp(0.0, (SHIFTS - 1) as f64);
        let s0 = (s as usize).min(SHIFTS - 2);
        let sf = (s - s0 as f64) as f32;
        let g0 = (gp as usize).min(GRAINS - 1);
        let gf = gp - g0 as f32;
        let g1 = (g0 + 1) % GRAINS;
        let at = |g: usize| {
            let row = &self.level[(table * GRAINS + g) * SHIFTS..][..SHIFTS];
            row[s0] + (row[s0 + 1] - row[s0]) * sf
        };
        let a = at(g0);
        a + (at(g1) - a) * gf
    }

    /// Cubic B-spline read of mip `mip` at frame position `p` (any real; wrapped to `[0, N)`) and
    /// grain position `gp` in `[0, GRAINS)` (the fraction crossfades to the next grain, wrapping
    /// 31 -> 0 for forward Motion scans).
    #[inline]
    pub fn read(&self, table: usize, mip: usize, gp: f32, p: f64) -> f32 {
        let g0 = (gp as usize).min(GRAINS - 1);
        let gf = gp - g0 as f32;
        let g1 = (g0 + 1) % GRAINS;
        let (fa, fb) = (self.frame(table, g0, mip), self.frame(table, g1, mip));
        // Positions >= N wrap through the index mask in `bspline`; only negatives need folding.
        let p = if p < 0.0 { p.rem_euclid(N as f64) } else { p };
        let q = p * (fa.len() as f64 / N as f64);
        let sa = bspline(fa, q);
        let sb = bspline(fb, q);
        sa + (sb - sa) * gf
    }
}

/// Uniform cubic B-spline at position `q` in `[0, len)` of a power-of-two frame of prefiltered
/// samples (see `generate_table`), which makes it an exact interpolator of the band-limited cycle.
#[inline]
fn bspline(f: &[f32], q: f64) -> f32 {
    let mask = f.len() - 1;
    let i = q as usize;
    let t = (q - i as f64) as f32;
    let (ym, y0, y1, y2) = (f[i.wrapping_sub(1) & mask], f[i & mask], f[(i + 1) & mask], f[(i + 2) & mask]);
    let c0 = (ym + 4.0 * y0 + y1) * (1.0 / 6.0);
    let c1 = 0.5 * (y1 - ym);
    let c2 = 0.5 * (ym + y1) - y0;
    let c3 = (y2 - ym) * (1.0 / 6.0) + 0.5 * (y0 - y1);
    ((c3 * t + c2) * t + c1) * t + c0
}

/// Radix-2 complex FFT with a precomputed twiddle table, for power-of-two lengths up to `max`.
pub struct Fft {
    tw: Vec<(f64, f64)>,
}

impl Fft {
    pub fn new(max: usize) -> Self {
        let tw = (0..max / 2)
            .map(|k| {
                let a = -std::f64::consts::TAU * k as f64 / max as f64;
                (a.cos(), a.sin())
            })
            .collect();
        Self { tw }
    }

    /// In place; `inverse` is unnormalised.
    pub fn run(&self, re: &mut [f64], im: &mut [f64], inverse: bool) {
        let n = re.len();
        assert!(n.is_power_of_two() && n <= 2 * self.tw.len() && im.len() == n);
        let mut j = 0;
        for i in 1..n {
            let mut bit = n >> 1;
            while j & bit != 0 {
                j ^= bit;
                bit >>= 1;
            }
            j |= bit;
            if i < j {
                re.swap(i, j);
                im.swap(i, j);
            }
        }
        let sign = if inverse { -1.0 } else { 1.0 };
        let mut len = 2;
        while len <= n {
            let (half, stride) = (len / 2, 2 * self.tw.len() / len);
            for start in (0..n).step_by(len) {
                for k in 0..half {
                    let (cr, ci) = self.tw[k * stride];
                    let ci = sign * ci;
                    let (a, b) = (start + k, start + k + half);
                    let tr = re[b] * cr - im[b] * ci;
                    let ti = re[b] * ci + im[b] * cr;
                    re[b] = re[a] - tr;
                    im[b] = im[a] - ti;
                    re[a] += tr;
                    im[a] += ti;
                }
            }
            len <<= 1;
        }
    }
}

/// |H|^2 of the BS.1770 K-weighting filter (48 kHz coefficients) at `hz`.
fn k_weight_sq(hz: f64) -> f64 {
    const B1: [f64; 3] = [1.53512485958697, -2.69169618940638, 1.19839281085285];
    const A1: [f64; 3] = [1.0, -1.69065929318241, 0.73248077421585];
    const B2: [f64; 3] = [1.0, -2.0, 1.0];
    const A2: [f64; 3] = [1.0, -1.99004745483398, 0.99007225036621];
    let w = std::f64::consts::TAU * hz / 48_000.0;
    let mag2 = |c: &[f64; 3]| {
        let (r, i) = (c[0] + c[1] * w.cos() + c[2] * (2.0 * w).cos(), -c[1] * w.sin() - c[2] * (2.0 * w).sin());
        r * r + i * i
    };
    mag2(&B1) / mag2(&A1) * mag2(&B2) / mag2(&A2)
}

fn xorshift(s: &mut u32) -> f32 {
    *s ^= *s << 13;
    *s ^= *s >> 17;
    *s ^= *s << 5;
    (*s >> 8) as f32 / (1u32 << 24) as f32
}

/// Grain RMS on its `N`-sample level (unless the peak cap kicked in).
pub const GRAIN_RMS: f32 = 0.25;

/// Harmonic `h` of a band-limited centered pulse of width `w` (cosine series).
fn pulse(w: f32, h: usize) -> f32 {
    2.0 / (PI * h as f32) * (PI * h as f32 * w).sin()
}

/// Pulse width 50 % -> 5 %.
fn square(x: f32, c: &mut Coefs) {
    let w = 0.5 - 0.45 * x;
    for h in 1..=HMAX {
        c[h].0 = pulse(w, h);
    }
}

/// 1 -> 7 saws, detune 0 -> 24 cents. A static grain cannot beat, so each saw's detune becomes
/// the phase its harmonics would have drifted to after `DRIFT` cycles; scanning Index then sweeps
/// the comb the way real detune does.
/// ponytail: frozen-phase detune, no true beating; upgrade path = free-running unison oscillators.
fn saw_stack(x: f32, c: &mut Coefs) {
    const DRIFT: f32 = 40.0;
    const SPREAD: [f32; 7] = [0.0, 1.0, -1.0, 0.667, -0.667, 0.333, -0.333];
    let voices = 1.0 + 6.0 * x;
    let cents = 24.0 * x;
    for (j, s) in SPREAD.iter().enumerate() {
        let w = (voices - j as f32).clamp(0.0, 1.0);
        let delta = 2f32.powf(s * cents / 1200.0) - 1.0;
        for h in 1..=HMAX {
            let ph = TAU * (h as f32 * delta * DRIFT).fract();
            let a = w / h as f32;
            c[h].0 += a * ph.sin();
            c[h].1 += a * ph.cos();
        }
    }
}

/// Sine -> full 1/n series, harmonics faded in one at a time; the count grows exponentially
/// (1 at index 0, about 22 at index 64, all 512 at index 127).
fn harmonic_rise(x: f32, c: &mut Coefs) {
    let count = (HMAX as f32).powf(x);
    for h in 1..=HMAX {
        c[h].1 = (count - (h - 1) as f32).clamp(0.0, 1.0) / h as f32;
    }
}

/// Narrow-pulse (flat, gently tilted) source through three formant peaks, A -> E -> I -> O -> U
/// with formants interpolated in log frequency.
fn vowel(x: f32, c: &mut Coefs) {
    const AMP: [f32; 3] = [1.0, 0.6, 0.35];
    const BW: [f32; 3] = [90.0, 110.0, 150.0];
    let u = x * 4.0;
    let v0 = (u as usize).min(3);
    let f = u - v0 as f32;
    for h in 1..=HMAX {
        let hz = h as f32 * REF_F0;
        let mut g = 0.02;
        for k in 0..3 {
            let fk = VOWELS[v0][k] * (VOWELS[v0 + 1][k] / VOWELS[v0][k]).powf(f);
            let d = (hz - fk) / BW[k];
            g += AMP[k] / (1.0 + d * d);
        }
        c[h].0 = g / (h as f32).sqrt();
    }
}

/// 2-op FM, ratio 2:1, index 0 -> 8, computed in the time domain and transformed: sidebands sit
/// at odd harmonics 1+2k and J_k(8) < 1e-4 beyond k = 16 (harmonic 33), so N samples hold it
/// exactly and the mips band-limit it like every other table.
fn fm(x: f32, c: &mut Coefs, fft: &Fft, re: &mut [f64], im: &mut [f64]) {
    let index = 8.0 * x as f64;
    for (n, (r, i)) in re.iter_mut().zip(im.iter_mut()).enumerate() {
        let t = std::f64::consts::TAU * n as f64 / N as f64;
        *r = (t + index * (2.0 * t).sin()).sin();
        *i = 0.0;
    }
    fft.run(re, im, false);
    // X[h] = N/2 (cc - i ss)
    for (h, ch) in c.iter_mut().enumerate().take(N / 2).skip(1) {
        *ch = ((2.0 * re[h] / N as f64) as f32, (-2.0 * im[h] / N as f64) as f32);
    }
}

/// Partial k lands on harmonic round(k^stretch), stretch 1.0 -> 1.6, amplitude 1/k.
/// ponytail: partials snap to integer harmonics because a grain must loop; true inharmonicity
/// would need multi-cycle grains.
fn metal(x: f32, c: &mut Coefs) {
    let stretch = 1.0 + 0.6 * x;
    for k in 1..=HMAX {
        let h = (k as f32).powf(stretch).round() as usize;
        if h > HMAX {
            break;
        }
        c[h].1 += 1.0 / k as f32;
    }
}

/// 30 % pulse plus a resonant peak sweeping 400 -> 4000 Hz (log).
fn reed(x: f32, c: &mut Coefs) {
    let fc = 400.0 * 10f32.powf(x);
    for h in 1..=HMAX {
        let d = (h as f32 * REF_F0 - fc) / (fc / 8.0);
        c[h].0 = pulse(0.3, h) * (1.0 + 8.0 / (1.0 + d * d));
    }
}

/// Seeded noise burst: random amplitude/phase per harmonic, a band around harmonic 10 widening
/// from a few harmonics to the whole low range (Gaussian, so it dies out well before 512).
fn grit(x: f32, rnd: &Coefs, c: &mut Coefs) {
    let width = 2.0 + 60.0 * x * x;
    for h in 1..=HMAX {
        let d = (h as f32 - 10.0) / width;
        let (a, ph) = rnd[h];
        let a = a * (-d * d).exp();
        c[h] = (a * ph.sin(), a * ph.cos());
    }
}

/// Basic shapes: sine (grain 0) -> triangle (10) -> saw (21) -> square (31), textbook series with
/// h1 = 1: triangle odd 1/h^2 cosines, saw 1/h sines, square odd 1/h sines (the relative phases of
/// a cosine square, 0/pi, a quarter period over so its odd harmonics match the saw's). Sine,
/// triangle, saw and square share phases except triangle -> saw: their relative phases differ by
/// pi on h = 3, 7, 11, ..., which no time alignment removes, so the triangle sits a quarter cycle
/// off on every odd harmonic and the grains in between interpolate each harmonic in amplitude and
/// phase. Nothing cancels along the scan.
fn basic(x: f32, c: &mut Coefs) {
    const KEYS: [f32; 4] = [0.0, 10.0, 21.0, 31.0];
    let g = (x * (GRAINS - 1) as f32).round();
    let k = if g < KEYS[1] { 0 } else if g < KEYS[2] { 1 } else { 2 };
    let u = (g - KEYS[k]) / (KEYS[k + 1] - KEYS[k]);
    // (amplitude, phase) of harmonic h of key shape s, as a cos(h t - phase).
    let shape = |s: usize, h: usize| {
        let (hf, odd) = (h as f32, h % 2 == 1);
        match s {
            0 => (if h == 1 { 1.0 } else { 0.0 }, 0.0),
            1 => (if odd { 1.0 / (hf * hf) } else { 0.0 }, 0.0),
            2 => (1.0 / hf, FRAC_PI_2),
            _ => (if odd { 1.0 / hf } else { 0.0 }, FRAC_PI_2),
        }
    };
    for (h, ch) in c.iter_mut().enumerate().skip(1) {
        let ((a0, p0), (a1, p1)) = (shape(k, h), shape(k + 1, h));
        let p = if a0 == 0.0 { p1 } else if a1 == 0.0 { p0 } else { p0 + u * (p1 - p0) };
        let a = a0 + u * (a1 - a0);
        *ch = (a * p.cos(), a * p.sin());
    }
}
