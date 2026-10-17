//! Filters A and B: LP12 and BP12 SVF (trapezoidal), Comb+, Comb-, and AM types.
//!
//! SPEC v2:
//! - 1.8 SVF: Q runs from 0.707 (res 0) to self-oscillation (res >= 0.94). The damping term carries
//!   a tanh on the band state, `k*bp + kn*(bp - S*tanh(bp/S))` (cubic for small signals, a Van der Pol
//!   oscillator at negative `k`), so res 1.0 sings a clean bounded sine and a loud resonant peak
//!   compresses instead of running into the output limiter. The nonlinear damping is applied
//!   semi-implicitly: its secant gain at the previous band value sets this sample's coefficients.
//! - 1.6 Comb-: output is loudness-matched instead of v1's peak normalisation `(1 - fb)`. With no
//!   note known it is `(1 - fb) * sqrt((1 + fb) / (1 - fb))` = unity mean power gain over all
//!   frequencies. With the note known ([`MultiFilter::set_note`]) it is unity power gain over the
//!   note's harmonic series (1/h^2 weighted), which is what a played note actually feeds it. A soft
//!   clip (transparent below 0.5, never reaching 1.0) follows.
//!
//! Track 7 (G3, G5):
//! - HP12 and Notch are the same SVF (same coefficients and tanh damping as LP12): high-pass
//!   `x - k_e b - l`, notch `x - k_e b`.
//! - LP24: two linear TPT sections tuned as a Butterworth-4 (1/Q 1.848 and 0.765), global feedback
//!   `kf = LP24_FB * res` around both and `(1 + kf)` input gain, so the small-signal passband stays
//!   at 0 dB for any res. The loop is solved zero-delay for the linear part, then the input of the
//!   first section goes through `S * tanh(u / S)` (the drive in the loop): it bounds the
//!   self-oscillation (onset at res 0.96, where `kf` reaches sqrt 2, the BW4 loss at cutoff) to a
//!   clean sine of about 0.3 and saturates loud resonant input instead of letting it run away.
//! - Formant: three parallel unity-peak band-passes (linear TPT SVFs) on F1..F3 of the
//!   Peterson-Barney vowels (`tables::VOWELS`, levels and bandwidths as the Vowel table). The
//!   cutoff picks the vowel: 20 Hz = A, 20 kHz = U, log-spaced (A-E-I-O-U every 2.5 octaves),
//!   formants interpolated in log frequency. Res scales the bandwidths by `3^(1 - 2 res)`
//!   (res 0.5 = the table's 90 / 110 / 150 Hz, res 0 three times wider, res 1 three times narrower).
use super::tables::{VOWELS, VOWEL_AMP, VOWEL_BW};
use crate::params::FilterType;
use std::f64::consts::{PI, SQRT_2, TAU};

/// Delay line long enough for 20 Hz at 192 kHz.
const BUF: usize = 9600 + 4;
/// SVF damping at res 1.0 is `-SVF_NEG`: self-oscillation starts where `k` crosses 0 (res 0.94).
const SVF_NEG: f64 = 0.05;
/// Res curve exponent: `k = (sqrt 2 + SVF_NEG) * (1 - res)^SVF_P - SVF_NEG`.
const SVF_P: f64 = 1.2;
/// Nonlinear damping strength at res 1.0 (scales with res, so res 0 is the clean linear SVF).
const SVF_KN: f64 = 2.0;
/// Band level where the tanh bends: damping is `k*b + kn*(b - S*tanh(b/S))`. Small-signal
/// self-oscillation amplitude is about `2 * S * sqrt(SVF_NEG / SVF_KN)`.
const SVF_SAT: f64 = 0.5;
/// Harmonics of the note used for the Comb- loudness match.
const COMB_HARMONICS: usize = 16;
/// LP24 feedback at res 1.0 (self-oscillation from `kf = sqrt 2`) and the loop drive level.
pub const LP24_FB: f64 = 1.04 * SQRT_2;
pub const LP24_SAT: f64 = 1.0;
/// 1/Q of the two sections of a 4th-order Butterworth.
pub const BW4_K: [f64; 2] = [1.847_759_065_022_573_5, 0.765_366_864_730_179_6];
/// Formant output gain (three narrow peaks pass little of a broadband source).
pub const FORMANT_GAIN: f64 = 2.0;

/// Vowel position 0 (A) .. 4 (U) for a cutoff in Hz: log-spaced over 20 Hz .. 20 kHz.
pub fn formant_pos(hz: f64) -> f64 {
    ((hz / 20.0).log2() / 1000f64.log2()).clamp(0.0, 1.0) * 4.0
}

/// F1..F3 (Hz) at vowel position `pos` (0..4), interpolated in log frequency.
pub fn formants(pos: f64) -> [f64; 3] {
    let v0 = (pos as usize).min(3);
    let f = pos - v0 as f64;
    std::array::from_fn(|k| {
        let (a, b) = (VOWELS[v0][k] as f64, VOWELS[v0 + 1][k] as f64);
        a * (b / a).powf(f)
    })
}

/// Formant bandwidths (Hz) at `res`.
pub fn formant_bw(res: f64) -> [f64; 3] {
    let s = 3f64.powf(1.0 - 2.0 * res.clamp(0.0, 1.0));
    std::array::from_fn(|k| VOWEL_BW[k] as f64 * s)
}

/// TPT SVF coefficients `(a1, a2, a3)` for `g = tan(pi f / sr)` and damping `k`.
#[inline]
fn svf_coefs(g: f64, k: f64) -> [f64; 3] {
    let a1 = 1.0 / (1.0 + g * (g + k));
    [a1, g * a1, g * g * a1]
}

/// One linear TPT SVF step on state `[ic1, ic2]`: (band, low).
#[inline]
fn svf_tick(s: &mut [f64; 2], c: &[f64; 3], x: f64) -> (f64, f64) {
    let v3 = x - s[1];
    let v1 = c[0] * s[0] + c[1] * v3;
    let v2 = s[1] + c[1] * s[0] + c[2] * v3;
    s[0] = 2.0 * v1 - s[0];
    s[1] = 2.0 * v2 - s[1];
    (v1, v2)
}

pub struct MultiFilter {
    sr: f64,
    kind: FilterType,
    // SVF
    g: f64,
    k: f64,
    kn: f64,
    /// BP12 output gain: v1's unity-peak `k` up to Q 1, then the raw band (resonance boosts like LP12).
    bp_gain: f64,
    ic1: f64,
    ic2: f64,
    /// Last band output and the mean of the last two (sets the nonlinear damping of the next
    /// sample; the two-sample mean keeps the lagged damping from pumping at Nyquist).
    v1: f64,
    bp: f64,
    // comb / AM
    buf: Vec<f32>,
    w: usize,
    delay: f32,
    delay_t: f32,
    mix: f32,
    fb: f32,
    /// Comb- output gain.
    norm: f32,
    /// Fundamental of the note feeding the filter, Hz (0 = unknown).
    note_hz: f32,
    phase: f64,
    inc: f64,
    fresh: bool,
    // ---- Track 7: LP24 (sections 0, 1) and Formant (0..2) ----
    /// Section states `[ic1, ic2]`, coefficients `(a1, a2, a3)`, Formant band gains.
    st: [[f64; 2]; 3],
    sc: [[f64; 3]; 3],
    sa: [f64; 3],
    /// LP24 feedback and input gain.
    kf: f64,
    comp: f64,
}

/// Soft clip for the Comb- output: identity up to 0.5, tanh knee above, never reaches 1.0.
#[inline]
fn soft_clip(x: f32) -> f32 {
    let a = x.abs();
    if a <= 0.5 {
        x
    } else {
        (0.5 + 0.5 * ((a - 0.5) / 0.5).tanh()).copysign(x)
    }
}

impl MultiFilter {
    pub fn new() -> Self {
        let mut f = Self {
            sr: 44100.0,
            kind: FilterType::Lp12,
            g: 0.0,
            k: SQRT_2,
            kn: 0.0,
            bp_gain: SQRT_2,
            ic1: 0.0,
            ic2: 0.0,
            v1: 0.0,
            bp: 0.0,
            buf: vec![0.0; BUF],
            w: 0,
            delay: 100.0,
            delay_t: 100.0,
            mix: 0.0,
            fb: 0.0,
            norm: 1.0,
            note_hz: 0.0,
            phase: 0.0,
            inc: 0.0,
            fresh: true,
            st: [[0.0; 2]; 3],
            sc: [[0.0; 3]; 3],
            sa: [0.0; 3],
            kf: 0.0,
            comp: 1.0,
        };
        f.set(FilterType::Lp12, 1000.0, 0.0);
        f
    }

    pub fn set_sample_rate(&mut self, sr: f64) {
        self.sr = sr;
        self.reset();
    }

    pub fn reset(&mut self) {
        self.ic1 = 0.0;
        self.ic2 = 0.0;
        self.v1 = 0.0;
        self.bp = 0.0;
        self.buf.fill(0.0);
        self.phase = 0.0;
        self.fresh = true;
        self.st = [[0.0; 2]; 3];
    }

    pub fn kind(&self) -> FilterType {
        self.kind
    }

    /// Changes the type without touching coefficients (call `set` before processing).
    pub fn set_kind(&mut self, kind: FilterType) {
        self.kind = kind;
    }

    /// Fundamental (Hz) of the note feeding this filter; the Comb- loudness match uses its
    /// harmonic series. Call before `set`. 0 = unknown (broadband match).
    pub fn set_note(&mut self, hz: f32) {
        self.note_hz = hz;
    }

    /// Copies `other`'s full state into this filter without allocating (both buffers are `BUF` long).
    pub fn copy_from(&mut self, other: &Self) {
        self.buf.copy_from_slice(&other.buf);
        let Self { sr, kind, g, k, kn, bp_gain, ic1, ic2, v1, bp, w, delay, delay_t, mix, fb, norm, note_hz, phase, inc, fresh, .. } =
            *other;
        (self.sr, self.kind, self.g, self.k, self.kn, self.bp_gain, self.ic1, self.ic2, self.v1, self.bp) =
            (sr, kind, g, k, kn, bp_gain, ic1, ic2, v1, bp);
        (self.w, self.delay, self.delay_t, self.mix, self.fb, self.norm, self.note_hz, self.phase, self.inc, self.fresh) =
            (w, delay, delay_t, mix, fb, norm, note_hz, phase, inc, fresh);
        (self.st, self.sc, self.sa, self.kf, self.comp) = (other.st, other.sc, other.sa, other.kf, other.comp);
    }

    pub fn set(&mut self, kind: FilterType, freq_hz: f32, res: f32) {
        self.kind = kind;
        let res = res.clamp(0.0, 1.0);
        let f = (freq_hz as f64).clamp(20.0, 0.45 * self.sr);
        match kind {
            FilterType::Lp12 | FilterType::Bp12 | FilterType::Hp12 | FilterType::Notch => {
                let r = res as f64;
                self.g = (PI * f / self.sr).tan();
                self.k = (SQRT_2 + SVF_NEG) * (1.0 - r).powf(SVF_P) - SVF_NEG;
                self.kn = SVF_KN * r;
                self.bp_gain = self.k.max(1.0);
            }
            FilterType::CombPlus | FilterType::CombMinus => {
                self.delay_t = ((self.sr / f) as f32).clamp(2.0, (BUF - 2) as f32);
                if self.fresh {
                    self.delay = self.delay_t;
                    self.fresh = false;
                }
                self.mix = res;
                self.fb = res * 0.98;
                if kind == FilterType::CombMinus {
                    self.norm = self.comb_norm();
                }
            }
            FilterType::Am => {
                self.inc = TAU * f / self.sr;
                self.mix = res;
            }
            FilterType::Lp24 => {
                let g = (PI * f / self.sr).tan();
                self.sc[0] = svf_coefs(g, BW4_K[0]);
                self.sc[1] = svf_coefs(g, BW4_K[1]);
                self.kf = LP24_FB * res as f64;
                self.comp = 1.0 + self.kf;
            }
            FilterType::Formant => {
                let (fk, bw) = (formants(formant_pos(f)), formant_bw(res as f64));
                for k in 0..3 {
                    let fc = fk[k].min(0.45 * self.sr);
                    let q = fc / bw[k];
                    self.sc[k] = svf_coefs((PI * fc / self.sr).tan(), 1.0 / q);
                    self.sa[k] = FORMANT_GAIN * VOWEL_AMP[k] as f64 / q;
                }
            }
        }
    }

    /// Comb- output gain for unity power gain: over the note's harmonics if known, else broadband.
    fn comb_norm(&self) -> f32 {
        let fb = self.fb as f64;
        if self.note_hz <= 0.0 {
            return ((1.0 - fb * fb) as f32).sqrt();
        }
        let (mut num, mut den) = (0.0, 0.0);
        for h in 1..=COMB_HARMONICS {
            let fh = h as f64 * self.note_hz as f64;
            if fh > 0.45 * self.sr {
                break;
            }
            let w = 1.0 / (h * h) as f64;
            let c = (TAU * fh * self.delay_t as f64 / self.sr).cos();
            num += w / (1.0 + fb * fb - 2.0 * fb * c);
            den += w;
        }
        if num > 0.0 { (den / num).sqrt() as f32 } else { 1.0 }
    }

    /// Fractional read `d` samples behind the write head (2 <= d < BUF-1).
    fn tap(&self, d: f32) -> f32 {
        let i = d as usize;
        let fr = d - i as f32;
        let a = self.buf[(self.w + BUF - i) % BUF];
        let b = self.buf[(self.w + BUF - i - 1) % BUF];
        a + (b - a) * fr
    }

    pub fn process(&mut self, x: f32) -> f32 {
        match self.kind {
            FilterType::Lp12 | FilterType::Bp12 | FilterType::Hp12 | FilterType::Notch => {
                // Silent in, (practically) silent state: output 0 without the work. An unused
                // filter (Osc B off in Parallel) then costs nothing.
                if x == 0.0 && self.ic1.abs() < 1e-15 && self.ic2.abs() < 1e-15 {
                    (self.ic1, self.ic2, self.v1, self.bp) = (0.0, 0.0, 0.0, 0.0);
                    return 0.0;
                }
                let x = x as f64;
                let b = self.bp;
                // Secant gain of the nonlinear damping k*b + kn*(b - S*tanh(b/S)) at the last band value.
                let ke = if b.abs() > 1e-9 { self.k + self.kn * (1.0 - SVF_SAT * (b / SVF_SAT).tanh() / b) } else { self.k };
                let g = self.g;
                let a1 = 1.0 / (1.0 + g * (g + ke));
                let a2 = g * a1;
                let a3 = g * a2;
                let v3 = x - self.ic2;
                let v1 = a1 * self.ic1 + a2 * v3;
                let v2 = self.ic2 + a2 * self.ic1 + a3 * v3;
                self.ic1 = 2.0 * v1 - self.ic1;
                self.ic2 = 2.0 * v2 - self.ic2;
                self.bp = 0.5 * (v1 + self.v1);
                self.v1 = v1;
                match self.kind {
                    FilterType::Lp12 => v2 as f32,
                    FilterType::Bp12 => (self.bp_gain * v1) as f32,
                    FilterType::Hp12 => (x - ke * v1 - v2) as f32,
                    _ => (x - ke * v1) as f32,
                }
            }
            FilterType::CombPlus => {
                self.delay += (self.delay_t - self.delay) * 0.01;
                let xd = self.tap(self.delay);
                self.buf[self.w] = x;
                self.w = (self.w + 1) % BUF;
                x + self.mix * 0.5 * (xd - x)
            }
            FilterType::CombMinus => {
                self.delay += (self.delay_t - self.delay) * 0.01;
                let y = x + self.fb * self.tap(self.delay);
                self.buf[self.w] = y;
                self.w = (self.w + 1) % BUF;
                soft_clip(y * self.norm)
            }
            FilterType::Am => {
                let m = (self.phase.sin() as f32) * self.mix;
                self.phase = (self.phase + self.inc) % TAU;
                x * (1.0 - self.mix) + x * m
            }
            FilterType::Lp24 => {
                if x == 0.0 && self.st[..2].iter().flatten().all(|v| v.abs() < 1e-15) {
                    self.st = [[0.0; 2]; 3];
                    return 0.0;
                }
                let [c0, c1] = [self.sc[0], self.sc[1]];
                // Each section's low output is affine in its input: lp = a3 * in + s.
                let s0 = self.st[0][1] + c0[1] * self.st[0][0] - c0[2] * self.st[0][1];
                let s1 = self.st[1][1] + c1[1] * self.st[1][0] - c1[2] * self.st[1][1];
                let kf = self.kf;
                let u = (self.comp * x as f64 - kf * (c1[2] * s0 + s1)) / (1.0 + kf * c0[2] * c1[2]);
                let u = LP24_SAT * (u / LP24_SAT).tanh();
                let (_, y0) = svf_tick(&mut self.st[0], &c0, u);
                svf_tick(&mut self.st[1], &c1, y0).1 as f32
            }
            FilterType::Formant => {
                if x == 0.0 && self.st.iter().flatten().all(|v| v.abs() < 1e-15) {
                    self.st = [[0.0; 2]; 3];
                    return 0.0;
                }
                let x = x as f64;
                let mut y = 0.0;
                for k in 0..3 {
                    y += self.sa[k] * svf_tick(&mut self.st[k], &self.sc[k], x).0;
                }
                y as f32
            }
        }
    }
}

impl Default for MultiFilter {
    fn default() -> Self {
        Self::new()
    }
}
