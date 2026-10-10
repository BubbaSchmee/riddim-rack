//! Procedural graintable generation (8 tables x 32 grains x 2048 samples), run once at initialize.
//!
//! Every grain is one cycle built additively from harmonics 1..=64 (or, for FM, computed directly
//! in the time domain where its sidebands already die out far below harmonic 64), then DC-removed
//! and peak-normalized. Neighbouring grains share phase structure so Index crossfades never cancel.
//! ponytail: harmonics stop at 64, so tables alias above ~C5 (64 * 523 Hz > 24 kHz at 48 kHz);
//! upgrade path = per-octave mip tables.

use crate::params::Table;
use std::f32::consts::{PI, TAU};

/// Samples per grain.
pub const N: usize = 2048;
/// Grains per table.
pub const GRAINS: usize = 32;
/// Number of tables (matches `params::Table`).
pub const TABLES: usize = 8;
/// Highest harmonic in any grain.
pub const HARMONICS: usize = 64;
/// Fundamental at which the Hz-based tables (Vowel, Reed) are voiced. Shift moves them from here.
const REF_F0: f32 = 65.0;

/// Peterson & Barney F1..F3 (Hz) for A, E, I, O, U (same values as VSS).
const VOWELS: [[f32; 3]; 5] = [
    [730.0, 1090.0, 2440.0],
    [530.0, 1840.0, 2480.0],
    [270.0, 2290.0, 3010.0],
    [570.0, 840.0, 2410.0],
    [300.0, 870.0, 2240.0],
];

/// Harmonic coefficients, `c.0 * cos(h t) + c.1 * sin(h t)`, index = harmonic number (0 unused).
type Coefs = [(f32, f32); HARMONICS + 1];

pub struct Tables {
    data: Box<[f32]>,
    ready: bool,
}

impl Default for Tables {
    fn default() -> Self {
        Self::new()
    }
}

impl Tables {
    /// Allocates (zeroed) storage; call `generate` before reading.
    pub fn new() -> Self {
        Self { data: vec![0.0; TABLES * GRAINS * N].into_boxed_slice(), ready: false }
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// One grain (`N` samples) of `table`.
    pub fn grain(&self, table: Table, g: usize) -> &[f32] {
        let start = (table as usize * GRAINS + g) * N;
        &self.data[start..start + N]
    }

    /// Fills the preallocated storage. Deterministic; no allocation.
    pub fn generate(&mut self) {
        let mut sin = [0f32; N];
        for (i, s) in sin.iter_mut().enumerate() {
            *s = (TAU * i as f32 / N as f32).sin();
        }
        // Grit: per-harmonic random amplitude and phase, shared by every grain of the table.
        let mut rng = 0x9E37_79B9u32;
        let mut rnd: Coefs = [(0.0, 0.0); HARMONICS + 1];
        for r in rnd.iter_mut().skip(1) {
            *r = (0.5 + 0.5 * xorshift(&mut rng), TAU * xorshift(&mut rng));
        }
        for t in 0..TABLES {
            for g in 0..GRAINS {
                let x = g as f32 / (GRAINS - 1) as f32;
                let start = (t * GRAINS + g) * N;
                let out = &mut self.data[start..start + N];
                out.fill(0.0);
                let mut c: Coefs = [(0.0, 0.0); HARMONICS + 1];
                match t {
                    0 => square(x, &mut c),
                    1 => saw_stack(x, &mut c),
                    2 => harmonic_rise(x, &mut c),
                    3 => vowel(x, &mut c),
                    4 => fm(x, out),
                    5 => metal(x, &mut c),
                    6 => reed(x, &mut c),
                    _ => grit(x, &rnd, &mut c),
                }
                synth(out, &c, &sin);
                finish(out);
            }
        }
        self.ready = true;
    }

    /// Linear read at sample position `p` in `[0, N)` and grain position `gp` in `[0, GRAINS)`
    /// (the fraction crossfades to the next grain, wrapping 31 -> 0 for forward Motion scans).
    #[inline]
    pub fn read(&self, table: usize, gp: f32, p: f64) -> f32 {
        let g0 = (gp as usize).min(GRAINS - 1);
        let gf = gp - g0 as f32;
        let g1 = (g0 + 1) % GRAINS;
        let i0 = (p as usize) & (N - 1);
        let f = (p - p.floor()) as f32;
        let i1 = (i0 + 1) & (N - 1);
        let a = &self.data[(table * GRAINS + g0) * N..][..N];
        let b = &self.data[(table * GRAINS + g1) * N..][..N];
        let sa = a[i0] + (a[i1] - a[i0]) * f;
        let sb = b[i0] + (b[i1] - b[i0]) * f;
        sa + (sb - sa) * gf
    }
}

fn xorshift(s: &mut u32) -> f32 {
    *s ^= *s << 13;
    *s ^= *s >> 17;
    *s ^= *s << 5;
    (*s >> 8) as f32 / (1u32 << 24) as f32
}

/// Adds the coefficients to `out` with an integer-phase sine table (phase quantized to 1/N cycle).
fn synth(out: &mut [f32], c: &Coefs, sin: &[f32; N]) {
    for (h, &(cc, ss)) in c.iter().enumerate().skip(1) {
        let a = cc.hypot(ss);
        if a < 1e-7 {
            continue;
        }
        // cc cos + ss sin = a sin(h t + phi), phi = atan2(cc, ss)
        let mut idx = ((cc.atan2(ss) / TAU * N as f32).round() as i64).rem_euclid(N as i64) as usize;
        for o in out.iter_mut() {
            *o += a * sin[idx];
            idx = (idx + h) & (N - 1);
        }
    }
}

/// DC removal + peak normalization to 1.0.
fn finish(out: &mut [f32]) {
    let mean = out.iter().sum::<f32>() / N as f32;
    let mut peak = 0f32;
    for o in out.iter_mut() {
        *o -= mean;
        peak = peak.max(o.abs());
    }
    if peak > 0.0 {
        for o in out.iter_mut() {
            *o /= peak;
        }
    }
}

/// Harmonic `h` of a band-limited centered pulse of width `w` (cosine series).
fn pulse(w: f32, h: usize) -> f32 {
    2.0 / (PI * h as f32) * (PI * h as f32 * w).sin()
}

/// Pulse width 50 % -> 5 %.
fn square(x: f32, c: &mut Coefs) {
    let w = 0.5 - 0.45 * x;
    for h in 1..=HARMONICS {
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
        for h in 1..=HARMONICS {
            let ph = TAU * h as f32 * delta * DRIFT;
            let a = w / h as f32;
            c[h].0 += a * ph.sin();
            c[h].1 += a * ph.cos();
        }
    }
}

/// Sine -> 1/n series, harmonics faded in one at a time.
fn harmonic_rise(x: f32, c: &mut Coefs) {
    let count = 1.0 + (HARMONICS - 1) as f32 * x;
    for h in 1..=HARMONICS {
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
    for h in 1..=HARMONICS {
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

/// 2-op FM, ratio 2:1, index 0 -> 8, computed directly: sidebands sit at odd harmonics 1+2k and
/// J_k(8) < 1e-4 beyond k = 16 (harmonic 33), so the grain is band-limited well under 64.
fn fm(x: f32, out: &mut [f32]) {
    let index = 8.0 * x;
    for (n, o) in out.iter_mut().enumerate() {
        let t = TAU * n as f32 / N as f32;
        *o = (t + index * (2.0 * t).sin()).sin();
    }
}

/// Partial k lands on harmonic round(k^stretch), stretch 1.0 -> 1.6, amplitude 1/k.
/// ponytail: partials snap to integer harmonics because a grain must loop; true inharmonicity
/// would need multi-cycle grains.
fn metal(x: f32, c: &mut Coefs) {
    let stretch = 1.0 + 0.6 * x;
    for k in 1..=HARMONICS {
        let h = (k as f32).powf(stretch).round() as usize;
        if h > HARMONICS {
            break;
        }
        c[h].1 += 1.0 / k as f32;
    }
}

/// 30 % pulse plus a resonant peak sweeping 400 -> 4000 Hz (log).
fn reed(x: f32, c: &mut Coefs) {
    let fc = 400.0 * 10f32.powf(x);
    for h in 1..=HARMONICS {
        let d = (h as f32 * REF_F0 - fc) / (fc / 8.0);
        c[h].0 = pulse(0.3, h) * (1.0 + 8.0 / (1.0 + d * d));
    }
}

/// Seeded noise burst: random amplitude/phase per harmonic, a band around harmonic 10 widening
/// from a few harmonics to the whole 1..64 range.
fn grit(x: f32, rnd: &Coefs, c: &mut Coefs) {
    let width = 2.0 + 60.0 * x * x;
    for h in 1..=HARMONICS {
        let d = (h as f32 - 10.0) / width;
        let (a, ph) = rnd[h];
        let a = a * (-d * d).exp();
        c[h] = (a * ph.sin(), a * ph.cos());
    }
}
