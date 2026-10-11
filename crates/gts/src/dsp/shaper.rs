//! Waveshaper before Filter A: Sine, Saturate, Clip, Quant, Noise.
//!
//! SPEC v2 1.2 / 1.3:
//! - Sine, Saturate and Clip run oversampled ([`Shaper::set_oversampling`], 1x / 2x / 4x, default
//!   2x); Quant and Noise stay at 1x (their aliasing is the effect). Amt 0 bypasses the curve.
//! - Every path is delayed to the same constant latency, [`Shaper::latency`] samples, so switching
//!   mode, amount or bypass never jumps in time.
//! - Output is level-compensated per mode, normalised to RMS-0.25 input (the mean output power of
//!   a saw and a square at that level), so toggling or turning the shaper changes tone, not level.
//! - Clip threshold runs 0.5 -> 0.05 (0 to 20 dB of drive) across Amt, matching the real signal
//!   level instead of v1's 1.0 -> 0.1.
// ponytail: declared here, not in dsp/mod.rs, so this wave's parallel agents never touch the same
// file; a later wave can move it to `dsp::oversample`.
#[path = "oversample.rs"]
pub mod oversample;

use crate::params::ShaperMode;
use oversample::{latency, Oversampler};

/// Reference input RMS for the gain compensation (a voice at full level, SPEC v2 1.3).
const REF_RMS: f32 = 0.25;
/// Crossfade from the 1x curve into the oversampled path after it (re)starts from silence: the
/// filters need `WARM` samples to fill, then `FADE` samples of linear crossfade.
const WARM: u32 = 48;
const FADE: u32 = 32;

/// The memoryless part of the oversampled modes, `Copy` so the oversampler closure can own it.
#[derive(Clone, Copy)]
struct Curve {
    mode: ShaperMode,
    /// Saturate / Sine pre-gain, or the Clip threshold.
    k: f32,
    /// Output compensation.
    comp: f32,
}

impl Curve {
    #[inline]
    fn raw(&self, x: f32) -> f32 {
        match self.mode {
            ShaperMode::Sine => (x * self.k).sin(),
            ShaperMode::Saturate => (x * self.k).tanh(),
            ShaperMode::Clip => x.clamp(-self.k, self.k),
            _ => x,
        }
    }

    #[inline]
    fn apply(&self, x: f32) -> f32 {
        self.raw(x) * self.comp
    }
}

#[derive(Clone)]
pub struct Shaper {
    mode: ShaperMode,
    amt: f32,
    curve: Curve,
    /// Quant levels per unit, 2^(bits-1).
    levels: f32,
    rng: u32,
    factor: usize,
    os: Oversampler,
    /// Input history for the constant-latency delay of the 1x paths (and the warm-up fallback).
    dly: [f32; 32],
    w: usize,
    /// Samples since the oversampled path (re)started; 0 = not running.
    warm: u32,
}

impl Shaper {
    pub fn new() -> Self {
        let mut s = Self {
            mode: ShaperMode::Saturate,
            amt: -1.0,
            curve: Curve { mode: ShaperMode::Saturate, k: 1.0, comp: 1.0 },
            levels: 1.0,
            rng: 0x1234_5679,
            factor: 2,
            os: Oversampler::default(),
            dly: [0.0; 32],
            w: 0,
            warm: 0,
        };
        s.set(ShaperMode::Saturate, 0.0);
        s
    }

    /// Oversampling for Sine / Saturate / Clip: 1, 2 or 4 (other values round to the nearest).
    /// A global quality setting (SPEC v2 1.2), not a preset parameter. Not real-time safe to call
    /// mid-note without a click: it clears the filter state.
    pub fn set_oversampling(&mut self, factor: usize) {
        let f = match factor {
            0 | 1 => 1,
            2 | 3 => 2,
            _ => 4,
        };
        if f != self.factor {
            self.factor = f;
            self.reset();
        }
    }

    pub fn oversampling(&self) -> usize {
        self.factor
    }

    /// Constant processing delay in samples at the current oversampling (0 / 21 / 23).
    pub fn latency(&self) -> usize {
        latency(self.factor)
    }

    pub fn reset(&mut self) {
        self.os.reset();
        self.dly = [0.0; 32];
        self.warm = 0;
    }

    pub fn set(&mut self, mode: ShaperMode, amt: f32) {
        let amt = amt.clamp(0.0, 1.0);
        if mode == self.mode && amt == self.amt {
            return;
        }
        self.mode = mode;
        self.amt = amt;
        let k = match mode {
            ShaperMode::Sine => 1.0 + amt * 8.0,
            ShaperMode::Saturate => 1.0 + amt * 10.0,
            ShaperMode::Clip => 0.5 * 10f32.powf(-amt),
            _ => 1.0,
        };
        self.levels = 2f32.powf(15.0 - amt * 14.0);
        self.curve = Curve { mode, k, comp: 1.0 };
        self.curve.comp = match mode {
            // Noise adds variance amt^2/3 * x^2 (uniform noise scaled by |x|).
            ShaperMode::Noise => 1.0 / (1.0 + amt * amt / 3.0).sqrt(),
            _ => {
                // Output power for RMS-0.25 inputs, averaged over a saw (uniform amplitude, peak
                // 0.25 * sqrt 3, midpoint rule) and a square (|x| = 0.25); a sine falls in between.
                const M: usize = 64;
                let peak = REF_RMS * 3f32.sqrt();
                let f = |x: f32| if mode == ShaperMode::Quant { self.quant(x) } else { self.curve.raw(x) };
                let saw = (0..M).map(|i| f((i as f32 + 0.5) / M as f32 * peak).powi(2)).sum::<f32>() / M as f32;
                let ms = 0.5 * (saw + f(REF_RMS).powi(2));
                if ms > 0.0 { REF_RMS / ms.sqrt() } else { 1.0 }
            }
        };
    }

    #[inline]
    fn quant(&self, x: f32) -> f32 {
        // Mid-tread (zero -> zero): 2^bits levels, bits 16 -> 2 with amt.
        let l = self.levels;
        (x * l).round().clamp(-l, l - 1.0) / l
    }

    /// Call instead of `process` while the shaper is switched out: keeps the latency delay line
    /// current (one store) so switching back in starts from the real signal, not stale samples.
    #[inline]
    pub fn idle(&mut self, x: f32) {
        self.dly[self.w] = x;
        self.w = (self.w + 1) % 32;
        self.warm = 0;
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        self.dly[self.w] = x;
        let lat = self.latency();
        let xd = self.dly[(self.w + 32 - lat) % 32];
        self.w = (self.w + 1) % 32;
        let oversampled = matches!(self.mode, ShaperMode::Sine | ShaperMode::Saturate | ShaperMode::Clip);
        if self.amt <= 0.0 && oversampled {
            self.warm = 0;
            return xd;
        }
        if !oversampled || self.factor == 1 {
            self.warm = 0;
            return self.shape_1x(xd);
        }
        if self.warm == 0 {
            self.os.reset();
        }
        let c = self.curve;
        let y = self.os.process(x, self.factor, |v| c.apply(v));
        if self.warm >= WARM + FADE {
            return y;
        }
        // Filters still filling from zero: fade in from the 1x curve on the delayed input.
        self.warm += 1;
        let g = (self.warm.saturating_sub(WARM) as f32 / FADE as f32).min(1.0);
        let fallback = c.apply(xd);
        fallback + (y - fallback) * g
    }

    #[inline]
    fn shape_1x(&mut self, x: f32) -> f32 {
        match self.mode {
            ShaperMode::Quant => self.quant(x) * self.curve.comp,
            ShaperMode::Noise => {
                let mut r = self.rng;
                r ^= r << 13;
                r ^= r >> 17;
                r ^= r << 5;
                self.rng = r;
                (x + (r as f32 / u32::MAX as f32 * 2.0 - 1.0) * x.abs() * self.amt) * self.curve.comp
            }
            _ => self.curve.apply(x),
        }
    }
}

impl Default for Shaper {
    fn default() -> Self {
        Self::new()
    }
}
