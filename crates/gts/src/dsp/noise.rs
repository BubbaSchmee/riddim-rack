//! Noise layer (SPEC v2 Track 7, G8): White / Pink / Brown with a one-pole tone control, one
//! generator per channel with its own seed (stereo decorrelated). The voice mixes it into Osc A's
//! path ahead of the shaper and filters, on Amp Env A.
//!
//! Levels: every type is scaled to RMS 0.25 (a full-level oscillator) at 0 dB, tone flat.
//! - White: uniform xorshift32, flat.
//! - Pink: Paul Kellet's refined 7-pole filter (-3 dB/oct within about 0.05 dB from 10 Hz up at
//!   44.1 kHz; the poles are fixed, so at other rates the band moves with the rate).
//! - Brown: white through a leaky integrator with a 10 Hz corner (-6 dB/oct above it).
//!
//! Tone 0..1: 0.5 is flat (bypassed); below it a one-pole low-pass glides from 20 kHz down to
//! 200 Hz at 0, above it a one-pole high-pass from 20 Hz up to 10 kHz at 1.
use crate::params::NoiseType;
use std::f32::consts::TAU;

/// Uniform white in [-1, 1) has RMS 1/sqrt(3).
const WHITE_GAIN: f32 = 0.25 * 1.732_050_8;
/// Kellet pink of that white measures RMS 3.06 (tests/track7_sound.rs `noise_slopes...`).
const PINK_GAIN: f32 = 0.25 * 1.732_050_8 / 3.06;
/// Brown corner, Hz.
const BROWN_HZ: f32 = 10.0;
/// Below this level the layer is off (the knob's bottom reads "Off").
pub const NOISE_OFF_DB: f32 = -60.0;

/// Per-block settings, from the patch.
#[derive(Debug, Clone, Copy)]
pub struct NoiseCfg {
    pub ty: NoiseType,
    /// Linear gain (0 = off).
    pub amp: f32,
    /// One-pole coefficient of the tone filter and its mode (-1 low-pass, 0 flat, 1 high-pass).
    tone_c: f32,
    tone: i8,
    /// Leaky integrator pole and its RMS normalisation.
    brown_a: f32,
    brown_g: f32,
}

impl NoiseCfg {
    pub fn new(ty: NoiseType, level_db: f32, tone: f32, sr: f32) -> Self {
        if level_db <= NOISE_OFF_DB {
            // Off (the default): no transcendental work in `Voice::update`.
            return Self { ty, amp: 0.0, tone_c: 0.0, tone: 0, brown_a: 0.0, brown_g: 0.0 };
        }
        let amp = 10f32.powf(level_db / 20.0);
        let t = tone.clamp(0.0, 1.0);
        let (fc, mode) = if t < 0.5 {
            (20000.0 * 0.01f32.powf((0.5 - t) / 0.5), -1)
        } else if t > 0.5 {
            (20.0 * 500f32.powf((t - 0.5) / 0.5), 1)
        } else {
            (0.0, 0)
        };
        let brown_a = 1.0 - TAU * BROWN_HZ / sr;
        Self {
            ty,
            amp,
            tone_c: 1.0 - (-TAU * fc.min(0.45 * sr) / sr).exp(),
            tone: mode,
            brown_a,
            brown_g: WHITE_GAIN * (1.0 - brown_a * brown_a).sqrt(),
        }
    }

    pub fn on(&self) -> bool {
        self.amp > 0.0
    }
}

impl Default for NoiseCfg {
    fn default() -> Self {
        Self::new(NoiseType::White, NOISE_OFF_DB, 0.5, 48000.0)
    }
}

/// One channel's generator.
#[derive(Debug, Clone, Copy)]
pub struct Noise {
    rng: u32,
    pink: [f32; 7],
    brown: f32,
    /// Tone filter state.
    z: f32,
}

impl Noise {
    pub fn new(seed: u32) -> Self {
        Self { rng: seed.max(1), pink: [0.0; 7], brown: 0.0, z: 0.0 }
    }

    /// One sample at RMS 0.25 x `cfg.amp` (before the tone filter).
    #[inline]
    pub fn next(&mut self, cfg: &NoiseCfg) -> f32 {
        let r = &mut self.rng;
        *r ^= *r << 13;
        *r ^= *r >> 17;
        *r ^= *r << 5;
        let w = (*r as i32) as f32 * (1.0 / 2_147_483_648.0);
        let x = match cfg.ty {
            NoiseType::White => w * WHITE_GAIN,
            NoiseType::Pink => {
                let b = &mut self.pink;
                b[0] = 0.99886 * b[0] + w * 0.055_517_9;
                b[1] = 0.99332 * b[1] + w * 0.075_075_9;
                b[2] = 0.96900 * b[2] + w * 0.153_852;
                b[3] = 0.86650 * b[3] + w * 0.310_485_6;
                b[4] = 0.55000 * b[4] + w * 0.532_952_2;
                b[5] = -0.7616 * b[5] - w * 0.016_898;
                let p = b[0] + b[1] + b[2] + b[3] + b[4] + b[5] + b[6] + w * 0.5362;
                b[6] = w * 0.115_926;
                p * PINK_GAIN
            }
            NoiseType::Brown => {
                self.brown = cfg.brown_a * self.brown + w;
                self.brown * cfg.brown_g
            }
        };
        let y = match cfg.tone {
            0 => x,
            t => {
                self.z += cfg.tone_c * (x - self.z);
                if t < 0 { self.z } else { x - self.z }
            }
        };
        y * cfg.amp
    }
}

impl Default for Noise {
    fn default() -> Self {
        Self::new(0x1234_5678)
    }
}
