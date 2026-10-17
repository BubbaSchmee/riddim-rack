//! Built-in effects (SPEC v2 Tracks 4 and 7): the ten modules, one per file, and [`FxChain`], which
//! runs nine of them in a reorderable order ([`FxOrder`], default Distortion -> Multiband -> EQ ->
//! Phaser -> Chorus -> Flanger -> Dimension -> Delay -> Reverb), then the Limiter, always last, on
//! the summed stereo output (after the voice sum and the mono crossover).
//!
//! - Reordering ([`FxChain::set_order`], once per block, lock-free: the order is a fixed `[u8; 9]`
//!   snapshot of `params.fx_order`) is click-free: the old order's output crossfades (raised
//!   cosine, 5 ms) to the dry chain input (aligned by the distortion's latency), the order swaps
//!   while only dry is heard, dry holds 10 ms while the modules' short memories settle on their
//!   new inputs, and the dry crossfades into the new order (5 ms): 10 ms of crossfade, 20 ms in
//!   all. (A crossfade straight from the old order's output to the new one would need a second
//!   copy of every module's state; the stateful modules can only run once per sample.) Delay and
//!   reverb tails carry over into the new order. The latency is the same in every order (the
//!   distortion's compensation delay runs wherever it sits).
//! - The master volume is applied between the last reorderable module and the Limiter (`process`'s
//!   `volume`): modules 1..9 see the voice level whatever the volume (the distortion's drive and autogain and
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
pub mod flanger;
pub mod limiter;
pub mod multiband;
pub mod reverb;
pub mod phaser;

use crate::params::{Division, GtsParams};
use chorus::{Chorus, ChorusParams};
use delay::{Delay, DelayParams};
use dimension::{Dimension, DimensionParams};
use distortion::{Distortion, DistortionParams};
use eq::{Eq, EqParams};
use flanger::{Flanger, FlangerParams};
use phaser::{Phaser, PhaserParams};
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
    /// SPEC v2 Track 7 (G9). `phs.sync_beats` / `flg.sync_beats` come from the divisions.
    pub phs: PhaserParams,
    pub phs_div: Division,
    pub flg: FlangerParams,
    pub flg_div: Division,
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
            phs: PhaserParams {
                on: p.phs.on.value(),
                stages: p.phs.stages.value().clamp(phaser::MIN_STAGES as i32, phaser::MAX_STAGES as i32) as u32,
                rate_hz: p.phs.rate.value(),
                sync: p.phs.sync.value(),
                sync_beats: crate::dsp::lfo::beats(p.phs.div.value()) as f32,
                depth: p.phs.depth.value(),
                feedback: p.phs.feedback.value(),
                centre_hz: p.phs.centre.value(),
                spread: p.phs.spread.value(),
                mix: p.phs.mix.value(),
            },
            phs_div: p.phs.div.value(),
            flg: FlangerParams {
                on: p.flg.on.value(),
                rate_hz: p.flg.rate.value(),
                sync: p.flg.sync.value(),
                sync_beats: crate::dsp::lfo::beats(p.flg.div.value()) as f32,
                depth: p.flg.depth.value(),
                delay_ms: p.flg.delay.value(),
                feedback: p.flg.feedback.value(),
                spread: p.flg.spread.value(),
                mix: p.flg.mix.value(),
            },
            flg_div: p.flg.div.value(),
        }
    }
}

// ---- chain order (SPEC v2 Track 7, G9) ------------------------------------------------------

/// Reorderable modules (the Limiter is fixed last).
pub const MODULES: usize = 9;
/// Module names in [`FxOrder`] index order (the default order); the preset / state spelling.
pub const MODULE_NAMES: [&str; MODULES] =
    ["Distortion", "Multiband", "EQ", "Phaser", "Chorus", "Flanger", "Dimension", "Delay", "Reverb"];
const DIST: u8 = 0;
const MB: u8 = 1;
const EQ: u8 = 2;
const PHS: u8 = 3;
const CHO: u8 = 4;
const FLG: u8 = 5;
const DIM: u8 = 6;
const DLY: u8 = 7;

/// Processing order: module indices ([`MODULE_NAMES`]) first to last, always a permutation of
/// 0..9. Serialised as the list of names; a list that is not a permutation of the nine names
/// (a trailing "Limiter" is accepted) does not parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FxOrder(pub [u8; MODULES]);

impl Default for FxOrder {
    fn default() -> Self {
        Self(std::array::from_fn(|i| i as u8))
    }
}

impl FxOrder {
    /// `Some` when `v` is a permutation of 0..9.
    pub fn new(v: [u8; MODULES]) -> Option<Self> {
        let mut seen = 0u16;
        for &m in &v {
            if m as usize >= MODULES || seen & (1 << m) != 0 {
                return None;
            }
            seen |= 1 << m;
        }
        Some(Self(v))
    }

    pub fn from_names<S: AsRef<str>>(names: &[S]) -> Option<Self> {
        let names: Vec<&str> = names.iter().map(|s| s.as_ref()).filter(|s| *s != "Limiter").collect();
        if names.len() != MODULES {
            return None;
        }
        let mut v = [0u8; MODULES];
        for (o, n) in v.iter_mut().zip(&names) {
            *o = MODULE_NAMES.iter().position(|m| m == n)? as u8;
        }
        Self::new(v)
    }

    pub fn names(&self) -> [&'static str; MODULES] {
        self.0.map(|m| MODULE_NAMES[m as usize])
    }

    /// Four bits per position (lock-free storage in [`FxOrderShared`]).
    pub fn encode(&self) -> u64 {
        self.0.iter().enumerate().fold(0, |a, (i, &m)| a | (m as u64) << (4 * i))
    }

    pub fn decode(x: u64) -> Option<Self> {
        Self::new(std::array::from_fn(|i| ((x >> (4 * i)) & 15) as u8))
    }
}

impl serde::Serialize for FxOrder {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.names().serialize(s)
    }
}

impl<'de> serde::Deserialize<'de> for FxOrder {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Vec::<String>::deserialize(d)?;
        Self::from_names(&v).ok_or_else(|| {
            serde::de::Error::custom(format!("fx_order must list each of {MODULE_NAMES:?} once, got {v:?}"))
        })
    }
}

/// `params.fx_order` (persisted as `"fx_order"`): written by the GUI, a preset load or a state
/// load, read by the audio thread once per block with one atomic load.
#[derive(Debug)]
pub struct FxOrderShared(std::sync::atomic::AtomicU64);

impl Default for FxOrderShared {
    fn default() -> Self {
        Self(std::sync::atomic::AtomicU64::new(FxOrder::default().encode()))
    }
}

impl FxOrderShared {
    pub fn get(&self) -> FxOrder {
        FxOrder::decode(self.0.load(std::sync::atomic::Ordering::Relaxed)).unwrap_or_default()
    }

    pub fn replace(&self, o: FxOrder) {
        self.0.store(o.encode(), std::sync::atomic::Ordering::Relaxed);
    }
}

impl<'a> nih_plug::params::persist::PersistentField<'a, FxOrder> for FxOrderShared {
    fn set(&self, new_value: FxOrder) {
        self.replace(new_value);
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&FxOrder) -> R,
    {
        f(&self.get())
    }
}

/// Reorder crossfade: each half (old order -> dry, dry -> new order), seconds.
const REORDER_HALF_S: f32 = 0.005;
/// Dry hold between the halves, seconds.
const REORDER_HOLD_S: f32 = 0.01;
/// Dry ring for the reorder crossfade (>= the distortion latency).
const DRY_N: usize = 64;

/// What the editor's FX displays read (docs/design/MOTION.md 5.8): peaks since the last
/// [`FxChain::telemetry`] call plus a few instantaneous values. Plain `Copy` data, no allocation.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FxTelemetry {
    /// Distortion shaper input with the largest magnitude (signed, linear).
    pub dist_in: f32,
    /// Multiband band gain (dB, positive = upward) and detector level (dB) per band.
    pub mb_gain_db: [f32; 3],
    pub mb_level_db: [f32; 3],
    /// Chorus LFO phase, turns.
    pub cho_phase: f32,
    /// Delay: peak input, peak wet (echo) signal (linear) and the gliding delay time in seconds.
    pub dly_in: f32,
    pub dly_wet: f32,
    pub dly_time_s: f32,
    /// Reverb: peak input and peak wet (tail) signal, linear.
    pub rev_in: f32,
    pub rev_wet: f32,
}

pub struct FxChain {
    sr: f32,
    /// Peaks gathered by `process` (see [`FxTelemetry`]).
    tel: FxTelemetry,
    /// Samples gathered into `tel` and the previous full window's peaks: the GUI reads only the
    /// newest record (~60 Hz), so a published peak covers at least one 1/60 s window, not one block.
    tel_n: u32,
    tel_prev: FxTelemetry,
    dist: Distortion,
    mb: Multiband,
    eq: Eq,
    cho: Chorus,
    dim: Dimension,
    dly: Delay,
    rev: Reverb,
    phs: Phaser,
    flg: Flanger,
    lim: Limiter,
    /// Modules other than the distortion have work this block (decided in `set`, after their
    /// own fade-outs).
    active: bool,
    /// Order in use, the order asked for, and the reorder crossfade position (1 = the order's
    /// output only, 0 = dry only) with its per-sample step.
    order: FxOrder,
    want: FxOrder,
    xf: f32,
    xf_step: f32,
    /// Dry-only samples left after a swap, and their number per swap.
    hold: u32,
    hold_n: u32,
    /// Chain input history for the crossfade's dry path.
    dry: [[f32; 2]; DRY_N],
    dry_w: usize,
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
            sr: 44100.0,
            tel: FxTelemetry::default(),
            tel_n: 0,
            tel_prev: FxTelemetry::default(),
            dist: Distortion::new(),
            mb: Multiband::new(),
            eq: Eq::new(),
            cho: Chorus::new(),
            dim: Dimension::new(),
            dly: Delay::new(),
            rev: Reverb::new(),
            phs: Phaser::new(),
            flg: Flanger::new(),
            lim: Limiter::new(),
            active: false,
            order: FxOrder::default(),
            want: FxOrder::default(),
            xf: 1.0,
            xf_step: 1.0 / (REORDER_HALF_S * 44100.0),
            hold: 0,
            hold_n: (REORDER_HOLD_S * 44100.0) as u32,
            dry: [[0.0; 2]; DRY_N],
            dry_w: 0,
        }
    }

    pub fn set_sample_rate(&mut self, sr: f32) {
        self.sr = sr;
        self.dist.set_sample_rate(sr);
        self.mb.set_sample_rate(sr);
        self.eq.set_sample_rate(sr as f64);
        self.cho.set_sample_rate(sr as f64);
        self.dim.set_sample_rate(sr as f64);
        self.dly.set_sample_rate(sr);
        self.rev.set_sample_rate(sr);
        self.phs.set_sample_rate(sr);
        self.flg.set_sample_rate(sr);
        self.lim.set_sample_rate(sr);
        self.xf_step = 1.0 / (REORDER_HALF_S * sr);
        self.hold_n = (REORDER_HOLD_S * sr) as u32;
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
        self.phs.reset();
        self.flg.reset();
        self.lim.reset();
        (self.order, self.xf, self.hold, self.dry, self.dry_w) = (self.want, 1.0, 0, [[0.0; 2]; DRY_N], 0);
        (self.tel, self.tel_prev, self.tel_n) = (FxTelemetry::default(), FxTelemetry::default(), 0);
    }

    /// Constant delay in samples, the same in every order: the sum of the modules' latencies
    /// (distortion oversampler + limiter lookahead; the phaser and flanger have none).
    pub fn latency(&self) -> usize {
        self.dist.latency() + self.phs.latency() + self.flg.latency() + self.lim.latency()
    }

    /// Once per block: the module order. A change crossfades over 10 ms (see the module docs);
    /// while the chain is bypassed it applies at once (every order sounds the same then).
    pub fn set_order(&mut self, o: FxOrder) {
        self.want = o;
        if !self.active {
            (self.order, self.xf, self.hold) = (o, 1.0, 0);
        }
    }

    /// The order in use (a crossfade swaps to the wanted one halfway through).
    pub fn order(&self) -> FxOrder {
        self.order
    }

    /// Phaser break frequency per side (Hz) and flanger swept delay per side (ms), for displays.
    pub fn phaser_break_hz(&self) -> [f32; 2] {
        self.phs.break_hz()
    }

    pub fn flanger_delay_ms(&self) -> [f64; 2] {
        self.flg.delay_ms()
    }

    /// Limiter gain reduction in dB (>= 0), deepest since the last call (the GUI meter).
    pub fn limiter_gr_db(&mut self) -> f32 {
        self.lim.take_gr_db()
    }

    /// The displays' record: peaks over the current and the previous 1/60 s window (so a 60 Hz
    /// reader sees every peak whatever the block size) and the live values.
    pub fn telemetry(&mut self) -> FxTelemetry {
        let d = self.dist.take_input_peak();
        if d.abs() > self.tel.dist_in.abs() {
            self.tel.dist_in = d;
        }
        let (c, p) = (self.tel, self.tel_prev);
        let t = FxTelemetry {
            dist_in: if p.dist_in.abs() > c.dist_in.abs() { p.dist_in } else { c.dist_in },
            mb_gain_db: self.mb.band_gain_db(),
            mb_level_db: self.mb.band_level_db(),
            cho_phase: self.cho.lfo_phase() as f32,
            dly_in: c.dly_in.max(p.dly_in),
            dly_wet: c.dly_wet.max(p.dly_wet),
            dly_time_s: (self.dly.delay_samples() / self.sr as f64) as f32,
            rev_in: c.rev_in.max(p.rev_in),
            rev_wet: c.rev_wet.max(p.rev_wet),
        };
        if self.tel_n as f32 >= self.sr / 60.0 {
            self.tel_prev = c;
            self.tel = FxTelemetry::default();
            self.tel_n = 0;
        }
        t
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
        self.phs.set(&PhaserParams { sync_beats: crate::dsp::lfo::beats(p.phs_div) as f32, ..p.phs });
        self.phs.set_transport(tempo, beat);
        self.flg.set(&FlangerParams { sync_beats: crate::dsp::lfo::beats(p.flg_div) as f32, ..p.flg });
        self.flg.set_transport(tempo, beat);
        self.active = !self.mb.is_bypassed()
            || !self.eq.is_bypassed()
            || !self.cho.is_bypassed()
            || !self.dim.is_bypassed()
            || !self.dly.is_bypassed()
            || !self.rev.is_bypassed()
            || !self.phs.is_bypassed()
            || !self.flg.is_bypassed();
        if !self.active {
            (self.order, self.xf, self.hold) = (self.want, 1.0, 0);
        }
    }

    /// Module `m` ([`MODULE_NAMES`] index) on one sample, gathering the delay / reverb telemetry.
    #[inline(always)]
    fn run(&mut self, m: u8, l: &mut f32, r: &mut f32) {
        match m {
            DIST => self.dist.process(l, r),
            MB => self.mb.process(l, r),
            EQ => self.eq.process(l, r),
            PHS => self.phs.process(l, r),
            CHO => self.cho.process(l, r),
            FLG => self.flg.process(l, r),
            DIM => self.dim.process(l, r),
            DLY => {
                let (il, ir) = (*l, *r);
                self.dly.process(l, r);
                let t = &mut self.tel;
                t.dly_in = t.dly_in.max(il.abs()).max(ir.abs());
                // out - in = mix * (wet - in), with the mix the module actually used this sample.
                let m = self.dly.wet_mix();
                if m >= 1e-3 {
                    let k = 1.0 / m;
                    t.dly_wet = t.dly_wet.max(((*l - il) * k + il).abs()).max(((*r - ir) * k + ir).abs());
                }
            }
            _ => {
                let (il, ir) = (*l, *r);
                self.rev.process(l, r);
                let t = &mut self.tel;
                t.rev_in = t.rev_in.max(il.abs()).max(ir.abs());
                let m = self.rev.wet_mix();
                if m >= 1e-3 {
                    let k = 1.0 / m;
                    t.rev_wet = t.rev_wet.max(((*l - il) * k + il).abs()).max(((*r - ir) * k + ir).abs());
                }
            }
        }
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
        self.tel_n = self.tel_n.saturating_add(1);
        if !self.active {
            // Off, the distortion is its 23-sample compensation delay (the same in every order).
            self.dist.process(l, r);
        } else {
            // The dry input for a reorder crossfade, kept current so a crossfade starts clean.
            let w = self.dry_w;
            self.dry[w] = [*l, *r];
            self.dry_w = (w + 1) % DRY_N;
            if self.xf >= 1.0 && self.order == self.want {
                // Steady: the order as it is.
                for k in 0..MODULES {
                    self.run(self.order.0[k], l, r);
                }
            } else {
                // Reorder crossfade: the order's output against the dry input, delayed like the
                // chain; the swap happens at dry only, and dry holds while the modules' short
                // memories (filters, phaser, flanger comb) settle on their new inputs.
                for k in 0..MODULES {
                    self.run(self.order.0[k], l, r);
                }
                let g = 0.5 - 0.5 * (std::f32::consts::PI * self.xf).cos();
                let d = self.dry[(w + DRY_N - self.dist.latency().min(DRY_N - 1)) % DRY_N];
                (*l, *r) = (d[0] + g * (*l - d[0]), d[1] + g * (*r - d[1]));
                if self.order != self.want {
                    self.xf = (self.xf - self.xf_step).max(0.0);
                    if self.xf == 0.0 {
                        (self.order, self.hold) = (self.want, self.hold_n);
                    }
                } else if self.hold > 0 {
                    self.hold -= 1;
                } else {
                    self.xf = (self.xf + self.xf_step).min(1.0);
                }
            }
        }
        (*l, *r) = (*l * volume, *r * volume);
        self.lim.process(l, r);
    }
}
