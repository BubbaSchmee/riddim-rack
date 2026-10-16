//! Built-in effects (SPEC v2 Track 4): the eight modules, one per file, and [`FxChain`], which runs
//! them in the fixed order Distortion -> Multiband -> EQ -> Chorus -> Dimension -> Delay -> Reverb
//! -> Limiter on the summed stereo output (after the voice sum and the mono crossover).
//!
//! - The master volume is applied between the Reverb and the Limiter (`process`'s `volume`):
//!   modules 1..7 see the voice level whatever the volume (the distortion's drive and autogain and
//!   the compressor's thresholds are calibrated to it), and the limiter guards the real output, so
//!   no sample leaves the plugin above -0.3 dBFS at any volume. It replaces the old tanh safety
//!   limiter, which is gone. (A limiter before the volume would clamp a raw voice that a -12 dB
//!   volume used to keep clean: the oscillator alias floor measured -42 dB instead of -80.)
//! - Parameters arrive once per block through [`FxChain::set`]; every module smooths its own values
//!   per sample. Tempo and beat position (from `Engine::transport`) go to the chorus (synced rate,
//!   phase lock) and the delay (synced time).
//! - Latency is constant: the distortion's 4x oversampler (23 samples, on or off, every type) plus
//!   the limiter's 1.5 ms lookahead (66 samples at 44.1 kHz, 72 at 48 kHz).
//! - Every module's power button fades (about 10 ms) instead of switching: the chorus, dimension
//!   and EQ through their mix, the distortion, multiband, delay and reverb through [`fade`]. An
//!   off->on edge after the fade-out has finished clears that module's history first.
//! - Non-finite input samples are replaced by 0 at the chain input.
//! - Bypass: when every module except the limiter is off (and has finished fading out), modules 2..7 are not called at all and the distortion only runs its
//!   compensation delay. The chain is then sample-exact: the output is the input delayed by
//!   [`FxChain::latency`] samples (times the volume), bit for bit, as long as it stays under the
//!   limiter's -0.3 dBFS ceiling at `lim_gain` 0 dB (the limiter is always on, so its delay and
//!   its ceiling still apply). Keeping the distortion's delay in the bypass is what keeps the
//!   reported latency true whether the chain is on or off.
//! - No allocation in `process`: every buffer is allocated by [`FxChain::new`] for 192 kHz.

pub mod chorus;
pub mod delay;
pub mod dimension;
pub mod distortion;
pub mod eq;
pub mod limiter;
pub mod multiband;
pub mod reverb;

use crate::params::{Division, GtsParams};
use chorus::{Chorus, ChorusParams};
use delay::{Delay, DelayParams};
use dimension::{Dimension, DimensionParams};
use distortion::{Distortion, DistortionParams};
use eq::{Eq, EqParams};
use limiter::{Limiter, LimiterParams};
use multiband::{Multiband, MultibandParams};
use reverb::{Reverb, ReverbParams};

/// Plain-value snapshot of the FX parameters, part of [`crate::dsp::Patch`] (so presets carry it).
/// Two module fields are not parameters and are filled by [`FxChain::set`]: `cho.sync_beats`
/// (from `cho_div`) and `dly.tempo` (from the transport).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FxPatch {
    pub dist: DistortionParams,
    pub mb: MultibandParams,
    pub eq: EqParams,
    pub cho: ChorusParams,
    /// Chorus LFO period when synced.
    pub cho_div: Division,
    pub dim: DimensionParams,
    pub dly: DelayParams,
    pub rev: ReverbParams,
    /// Limiter input gain, dB.
    pub lim_gain: f32,
}

impl FxPatch {
    pub fn from_params(p: &GtsParams) -> Self {
        let (d, m, e, c, di, dl, r) = (&p.dist, &p.mb, &p.eq, &p.cho, &p.dim, &p.dly, &p.rev);
        Self {
            dist: DistortionParams {
                on: d.on.value(),
                kind: d.ty.value(),
                drive_db: d.drive.value(),
                bias: d.bias.value(),
                filter_mode: d.filter_mode.value(),
                filter_type: d.filter_type.value(),
                filter_freq: d.filter_freq.value(),
                filter_q: d.filter_q.value(),
                out_db: d.out.value(),
                autogain: d.autogain.value(),
                mix: d.mix.value(),
            },
            mb: MultibandParams {
                on: m.on.value(),
                depth: m.depth.value(),
                time: m.time.value(),
                xover_lo: m.xover_lo.value(),
                xover_hi: m.xover_hi.value(),
                up_db: [m.up_lo.value(), m.up_mid.value(), m.up_hi.value()],
                down_db: [m.down_lo.value(), m.down_mid.value(), m.down_hi.value()],
                gain_db: [m.gain_lo.value(), m.gain_mid.value(), m.gain_hi.value()],
                in_db: m.input.value(),
                out_db: m.out.value(),
                mix: m.mix.value(),
                single_band: false,
            },
            eq: EqParams {
                on: e.on.value(),
                low_freq: e.low_freq.value(),
                low_gain: e.low_gain.value(),
                peak_freq: e.peak_freq.value(),
                peak_q: e.peak_q.value(),
                peak_gain: e.peak_gain.value(),
                high_freq: e.high_freq.value(),
                high_gain: e.high_gain.value(),
                cut_on: e.cut_on.value(),
                cut_freq: e.cut_freq.value(),
            },
            cho: ChorusParams {
                on: c.on.value(),
                voices: c.voices.value().clamp(2, chorus::MAX_VOICES as i32) as u32,
                rate_hz: c.rate.value(),
                sync: c.sync.value(),
                sync_beats: crate::dsp::lfo::beats(c.div.value()) as f32,
                depth: c.depth.value(),
                delay_ms: c.delay.value(),
                feedback: c.feedback.value(),
                lpf_hz: c.lpf.value(),
                wet_lowcut_hz: c.wet_lowcut.value(),
                mix: c.mix.value(),
            },
            cho_div: c.div.value(),
            dim: DimensionParams {
                on: di.on.value(),
                size: di.size.value(),
                wet_lowcut_hz: di.wet_lowcut.value(),
                mix: di.mix.value(),
            },
            dly: DelayParams {
                on: dl.on.value(),
                sync: dl.sync.value(),
                time_ms: dl.time.value(),
                div: dl.div.value(),
                feel: dl.feel.value(),
                tempo: DelayParams::default().tempo,
                feedback: dl.feedback.value(),
                mode: dl.mode.value(),
                hp_hz: dl.hp.value(),
                lp_hz: dl.lp.value(),
                mix: dl.mix.value(),
            },
            rev: ReverbParams {
                on: r.on.value(),
                ty: r.ty.value(),
                size: r.size.value(),
                decay: r.decay.value(),
                damp: r.damp.value(),
                predelay_ms: r.predelay.value(),
                lowcut_hz: r.lowcut.value(),
                hicut_hz: r.hicut.value(),
                width: r.width.value(),
                mix: r.mix.value(),
            },
            lim_gain: p.lim.gain.value(),
        }
    }
}

pub struct FxChain {
    dist: Distortion,
    mb: Multiband,
    eq: Eq,
    cho: Chorus,
    dim: Dimension,
    dly: Delay,
    rev: Reverb,
    lim: Limiter,
    /// Modules 2..7 have work this block (decided in `set`, after their own fade-outs).
    active: bool,
}

/// Power-button fade shared by the distortion, multiband, delay and reverb: glides `en` toward 1
/// (on) or 0 (off) with one-pole coefficient `k` (`x = t + (x - t) * k`), snapping within 1e-4 of
/// the end so an off module reaches exactly 0 and drops out of the chain.
#[inline]
pub(crate) fn fade(en: &mut f32, on: bool, k: f32) -> f32 {
    let t = if on { 1.0 } else { 0.0 };
    *en = t + (*en - t) * k;
    if (*en - t).abs() < 1e-4 {
        *en = t;
    }
    *en
}

impl Default for FxChain {
    fn default() -> Self {
        Self::new()
    }
}

impl FxChain {
    /// Allocates every module's buffers (about 20 MB, mostly the delay's 8 s lines at 192 kHz).
    pub fn new() -> Self {
        Self {
            dist: Distortion::new(),
            mb: Multiband::new(),
            eq: Eq::new(),
            cho: Chorus::new(),
            dim: Dimension::new(),
            dly: Delay::new(),
            rev: Reverb::new(),
            lim: Limiter::new(),
            active: false,
        }
    }

    pub fn set_sample_rate(&mut self, sr: f32) {
        self.dist.set_sample_rate(sr);
        self.mb.set_sample_rate(sr);
        self.eq.set_sample_rate(sr as f64);
        self.cho.set_sample_rate(sr as f64);
        self.dim.set_sample_rate(sr as f64);
        self.dly.set_sample_rate(sr);
        self.rev.set_sample_rate(sr);
        self.lim.set_sample_rate(sr);
    }

    /// Clears every line, tail and envelope.
    pub fn reset(&mut self) {
        self.dist.reset();
        self.mb.reset();
        self.eq.reset();
        self.cho.reset();
        self.dim.reset();
        self.dly.reset();
        self.rev.reset();
        self.lim.reset();
    }

    /// Constant delay in samples: distortion oversampler + limiter lookahead.
    pub fn latency(&self) -> usize {
        self.dist.latency() + self.lim.latency()
    }

    /// True while the chain is the sample-exact bypass (only the delays and the limiter run).
    pub fn is_bypassed(&self) -> bool {
        !self.active && self.dist.is_bypassed()
    }

    /// Once per block: parameters, host tempo and (while playing) the beat position at the block
    /// start.
    pub fn set(&mut self, p: &FxPatch, tempo: f64, beat: Option<f64>) {
        self.dist.set(&p.dist);
        self.mb.set(&p.mb);
        self.eq.set(&p.eq);
        self.cho.set(&ChorusParams { sync_beats: crate::dsp::lfo::beats(p.cho_div) as f32, ..p.cho });
        self.cho.set_transport(tempo, beat);
        self.dim.set(&p.dim);
        self.dly.set(&DelayParams { tempo: tempo as f32, ..p.dly });
        self.rev.set(&p.rev);
        self.lim.set(&LimiterParams { on: true, gain_db: p.lim_gain });
        self.active = !self.mb.is_bypassed()
            || !self.eq.is_bypassed()
            || !self.cho.is_bypassed()
            || !self.dim.is_bypassed()
            || !self.dly.is_bypassed()
            || !self.rev.is_bypassed();
    }

    /// One stereo sample; `volume` (linear, the smoothed master volume) is applied before the
    /// limiter.
    #[inline]
    pub fn process(&mut self, l: &mut f32, r: &mut f32, volume: f32) {
        // One NaN/Inf from the voices would latch every module with history (filters, lines,
        // tanks) into NaN, and the limiter would then mute the session until a host reset.
        if !(l.is_finite() && r.is_finite()) {
            (*l, *r) = (if l.is_finite() { *l } else { 0.0 }, if r.is_finite() { *r } else { 0.0 });
        }
        // Off, the distortion is its 23-sample compensation delay.
        self.dist.process(l, r);
        if self.active {
            self.mb.process(l, r);
            self.eq.process(l, r);
            self.cho.process(l, r);
            self.dim.process(l, r);
            self.dly.process(l, r);
            self.rev.process(l, r);
        }
        (*l, *r) = (*l * volume, *r * volume);
        self.lim.process(l, r);
    }
}
