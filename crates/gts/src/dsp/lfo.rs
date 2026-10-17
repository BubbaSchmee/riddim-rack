//! Mod A..D LFOs: curves (incl. Custom shapes), free or beat-synced rate, one-shot mode,
//! retrigger on note-on.
//!
//! SPEC v2 Track 7 extensions ([`Lfo::set_ext`]), all neutral at their defaults (bit-identical
//! to v1): start phase (read offset, so it also shifts a host-locked wobble against the grid),
//! linear fade-in after note-on, one-pole output smoothing (time constant `smooth` x a quarter
//! cycle) and Free trigger mode (note-on leaves phase and random state alone).
use super::shape::Shape;
use crate::params::{Curve, Division};
use std::f64::consts::TAU;
use std::sync::atomic::{AtomicU32, Ordering};

static SEED: AtomicU32 = AtomicU32::new(0x9E37_79B9);
/// Mod C / Mod D draw from their own counter, so adding them leaves the Mod A / B seeds (and
/// their random curves) exactly as before.
static SEED_CD: AtomicU32 = AtomicU32::new(0x2F6B_1C3D);

/// Cycle length in beats (quarter notes).
pub(crate) fn beats(d: Division) -> f64 {
    use Division::*;
    match d {
        Bars4 => 16.0,
        Bars2 => 8.0,
        Bar1 => 4.0,
        Half => 2.0,
        HalfT => 4.0 / 3.0,
        Quarter => 1.0,
        QuarterT => 2.0 / 3.0,
        Eighth => 0.5,
        EighthT => 1.0 / 3.0,
        Sixteenth => 0.25,
        SixteenthT => 1.0 / 6.0,
        ThirtySecond => 0.125,
        ThirtySecondT => 1.0 / 12.0,
        HalfD => 3.0,
        QuarterD => 1.5,
        EighthD => 0.75,
        SixteenthD => 0.375,
    }
}

pub struct Lfo {
    sr: f64,
    /// Unwrapped phase in cycles.
    ph: f64,
    inc: f64,
    curve: Curve,
    one_shot: bool,
    rng: u32,
    prev: f32,
    next: f32,
    /// Grid-lock phase correction still to apply, at most `slew` cycles per sample (a half-cycle
    /// correction takes 15 ms).
    pending: f64,
    slew: f64,
    /// Set by `trigger`: the next lock snaps (a fresh note starts on the grid).
    snap: bool,
    /// Start phase in cycles (read offset).
    off: f64,
    /// Fade-in progress per sample (0 = no fade) and the progress since note-on, 0..1.
    fade_inc: f32,
    fade: f32,
    /// Smoothing amount and its one-pole coefficient (0 = off), smoothed output, `y` valid.
    smooth: f32,
    smooth_a: f32,
    y: f32,
    primed: bool,
    /// Free trigger mode.
    free: bool,
    /// The shape `Curve::Custom` plays.
    custom: Shape,
}

impl Lfo {
    pub fn new() -> Self {
        // ponytail: seed from a global counter so Mod A and Mod B differ; deterministic per construction order.
        Self::seeded(SEED.fetch_add(0x6C07_8965, Ordering::Relaxed) | 1)
    }

    /// Mod C / Mod D (own seed counter, see [`SEED_CD`]).
    pub fn new_cd() -> Self {
        Self::seeded(SEED_CD.fetch_add(0x6C07_8965, Ordering::Relaxed) | 1)
    }

    fn seeded(seed: u32) -> Self {
        let mut l = Self {
            sr: 44100.0,
            ph: 0.0,
            inc: 0.0,
            curve: Curve::Sine,
            one_shot: false,
            rng: seed,
            prev: 0.0,
            next: 0.0,
            pending: 0.0,
            slew: 0.0,
            snap: true,
            off: 0.0,
            fade_inc: 0.0,
            fade: 1.0,
            smooth: 0.0,
            smooth_a: 0.0,
            y: 0.0,
            primed: false,
            free: false,
            custom: Shape::default(),
        };
        l.set_sample_rate(44100.0);
        l.advance_random();
        l.advance_random();
        l
    }

    pub fn set_sample_rate(&mut self, sr: f64) {
        self.sr = sr;
        self.slew = 0.5 / (0.015 * sr);
    }

    fn rand(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    fn advance_random(&mut self) {
        self.prev = self.next;
        self.next = self.rand();
    }

    /// Unwrapped phase in cycles (a finished one-shot sits at or above 1), for the GUI.
    pub fn phase(&self) -> f32 {
        self.ph as f32
    }

    /// Note-on: restart the fade-in; reset phase and re-arm one-shot (not in Free mode).
    pub fn trigger(&mut self) {
        self.fade = 0.0;
        if self.free {
            return;
        }
        self.ph = 0.0;
        self.pending = 0.0;
        self.snap = true;
        self.primed = false;
        self.advance_random();
    }

    /// The shape `Curve::Custom` plays (copied; call when the shapes change, not per block).
    pub fn set_shape(&mut self, s: &Shape) {
        self.custom = *s;
    }

    pub fn configure(&mut self, curve: Curve, rate_hz: f32, sync: Option<(Division, f64, Option<f64>)>, one_shot: bool) {
        self.curve = curve;
        self.one_shot = one_shot;
        let mut target = None;
        self.inc = match sync {
            Some((div, bpm, beat)) => {
                let len = beats(div);
                target = beat.map(|b| b / len);
                bpm / 60.0 / self.sr / len
            }
            None => rate_hz as f64 / self.sr,
        };
        if let (false, Some(t)) = (one_shot, target) {
            // Lock to the host grid by the shortest wrapped distance so cycle counting stays monotonic.
            let mut d = (t - self.ph).rem_euclid(1.0);
            if d > 0.5 {
                d -= 1.0;
            }
            if self.snap {
                self.ph += d;
                self.pending = 0.0;
            } else {
                // A re-lock (e.g. stop -> play) glides onto the grid instead of jumping.
                self.pending = d;
            }
        }
        self.snap = false;
        self.set_smooth_coef();
    }

    /// Track 7 extensions: start phase in degrees, fade-in seconds, smoothing 0..1, Free mode.
    /// Call after [`Lfo::configure`] (the smoothing follows the rate).
    pub fn set_ext(&mut self, phase_deg: f32, fade_s: f32, smooth: f32, free: bool) {
        self.off = (phase_deg as f64 / 360.0).rem_euclid(1.0);
        self.fade_inc = if fade_s > 0.0 { 1.0 / (fade_s * self.sr as f32) } else { 0.0 };
        self.smooth = smooth.clamp(0.0, 1.0);
        self.free = free;
        self.set_smooth_coef();
    }

    fn set_smooth_coef(&mut self) {
        self.smooth_a = if self.smooth > 0.0 { 1.0 - (-(self.inc / (0.25 * self.smooth as f64))).exp() as f32 } else { 0.0 };
    }

    fn shape(&self, x: f64) -> f32 {
        match self.curve {
            Curve::SampleHold => self.next,
            Curve::SmoothRandom => {
                let xf = x as f32;
                let s = xf * xf * (3.0 - 2.0 * xf);
                self.prev + (self.next - self.prev) * s
            }
            Curve::Custom => self.custom.eval(x),
            c => Self::curve_at(c, x),
        }
    }

    /// One cycle of `curve` at phase `x` (0..1). The random curves have no fixed shape: for
    /// display they draw a fixed four-step pattern (Sample & Hold) or its smoothed version;
    /// Custom draws the default shape (the editor draws the LFO's own from `params.lfo_shapes`).
    pub fn curve_at(curve: Curve, x: f64) -> f32 {
        let xf = x as f32;
        const STEPS: [f32; 5] = [0.6, -0.4, 0.9, -0.8, 0.6];
        match curve {
            Curve::Sine => (x * TAU).sin() as f32,
            Curve::Triangle => {
                if xf < 0.25 {
                    4.0 * xf
                } else if xf < 0.75 {
                    2.0 - 4.0 * xf
                } else {
                    4.0 * xf - 4.0
                }
            }
            Curve::SawUp => 2.0 * xf - 1.0,
            Curve::SawDown => 1.0 - 2.0 * xf,
            Curve::Square => {
                if xf < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            Curve::SampleHold => STEPS[((xf * 4.0) as usize).min(3)],
            Curve::SmoothRandom => {
                let (k, f) = (((xf * 4.0) as usize).min(3), (xf * 4.0).fract());
                STEPS[k] + (STEPS[k + 1] - STEPS[k]) * f * f * (3.0 - 2.0 * f)
            }
            Curve::Decay => Self::decay(xf),
            Curve::Attack => 1.0 - Self::decay(xf),
            Curve::Custom => Shape::default().eval(x),
        }
    }

    /// Exponential fall 1 -> 0 over one cycle.
    fn decay(x: f32) -> f32 {
        const E5: f32 = 0.006_737_947; // e^-5
        ((-5.0 * x).exp() - E5) / (1.0 - E5)
    }

    /// Shape value at the current phase (start phase applied; a finished one-shot holds its end).
    fn raw(&self) -> f32 {
        if self.one_shot && self.ph >= 1.0 {
            // One cycle from `off` ends back at `off` (at 1.0 for no offset).
            self.shape(if self.off > 0.0 { self.off } else { 1.0 })
        } else {
            self.shape((self.ph + self.off).rem_euclid(1.0))
        }
    }

    /// The value the next [`Lfo::next`] returns, without advancing.
    pub fn peek(&self) -> f32 {
        let raw = self.raw();
        let out = if self.smooth_a > 0.0 && self.primed { self.y + (raw - self.y) * self.smooth_a } else { raw };
        if self.fade_inc > 0.0 { out * self.fade } else { out }
    }

    pub fn next(&mut self) -> f32 {
        let raw = self.raw();
        let mut out = raw;
        if self.smooth_a > 0.0 {
            self.y = if self.primed { self.y + (raw - self.y) * self.smooth_a } else { raw };
            self.primed = true;
            out = self.y;
        }
        if self.fade_inc > 0.0 {
            out *= self.fade;
            self.fade = (self.fade + self.fade_inc).min(1.0);
        } else {
            self.fade = 1.0;
        }
        if self.one_shot && self.ph >= 1.0 {
            return out;
        }
        let before = self.ph.floor();
        let c = self.pending.clamp(-self.slew, self.slew);
        self.pending -= c;
        self.ph += self.inc + c;
        if !self.one_shot && self.ph.floor() > before {
            self.advance_random();
        }
        out
    }
}

impl Default for Lfo {
    fn default() -> Self {
        Self::new()
    }
}
