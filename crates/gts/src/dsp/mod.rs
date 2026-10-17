//! GTS DSP: the voice engine (allocation, MIDI, output stage) over grain tables and grain oscillators.
//! Everything `process` touches is preallocated here for 8 voices.

pub mod env;
pub mod filter;
pub mod fx;
pub mod grain;
pub mod lfo;
pub mod shape;
pub mod noise;
pub mod shaper;
pub mod tables;
pub mod voice;

use crate::matrix::{self, flag, ModRt, Source, ENC_LEN, N_SOURCES, TICK};
use crate::params::{
    Curve, Division, FilterParams, FilterType, GtsParams, LfoExtParams, ModCdParams, ModParams, OscParams, Routing,
    ShaperMode, Table, Target, Trigger,
};
use crate::params::{NoiseType, SubShape, WarpMode};
use fx::{FxChain, FxPatch};
use nih_plug::prelude::NoteEvent;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tables::Tables;
use voice::{Controls, Voice};

pub const MAX_VOICES: usize = 8;
/// Mono note stack depth.
const MAX_HELD: usize = 16;

/// One oscillator's parameter values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OscPatch {
    pub on: bool,
    pub table: Table,
    pub oct: i32,
    pub semi: i32,
    pub cent: f32,
    pub index: f32,
    pub motion: f32,
    pub shift: f32,
    pub vol: f32,
    /// Seconds.
    pub att: f32,
    pub dec: f32,
    pub sus: f32,
    pub rel: f32,
    /// Amp envelope curve, -1..1 (0 = the v1 exponential shape).
    pub curve: f32,
    /// Unison voices 1..7, detune in cents, detune curve 0..1, blend 0..1, width 0..1, random
    /// phase 0..1, phase reset.
    pub unison: usize,
    pub detune: f32,
    pub det_curve: f32,
    pub blend: f32,
    pub width: f32,
    pub rand_phase: f32,
    pub phase_reset: bool,
    /// Track 7 (G4): warp mode and amount 0..1.
    pub warp: WarpMode,
    pub warp_amt: f32,
}

impl OscPatch {
    fn from_params(p: &OscParams) -> Self {
        Self {
            on: p.on.value(),
            table: p.table.value(),
            oct: p.oct.value(),
            semi: p.semi.value(),
            cent: p.cent.value(),
            index: p.index.value(),
            motion: p.motion.value(),
            shift: p.shift.value(),
            vol: p.vol.value(),
            att: p.att.value(),
            dec: p.dec.value(),
            sus: p.sus.value(),
            rel: p.rel.value(),
            curve: p.curve.value(),
            unison: p.unison.value().clamp(1, voice::MAX_UNISON as i32) as usize,
            detune: p.detune.value(),
            det_curve: p.det_curve.value(),
            blend: p.blend.value(),
            width: p.width.value(),
            rand_phase: p.rand_phase.value(),
            phase_reset: p.phase_reset.value(),
            warp: p.warp_mode.value(),
            warp_amt: p.warp_amt.value(),
        }
    }
}

/// One LFO's settings (Mod A or Mod B).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModPatch {
    pub curve: Curve,
    /// Hz, used when `sync` is off.
    pub rate: f32,
    pub sync: bool,
    pub div: Division,
    pub oneshot: bool,
    pub target: Target,
}

impl ModPatch {
    fn from_params(p: &ModParams) -> Self {
        Self {
            curve: p.curve.value(),
            rate: p.rate.value(),
            sync: p.sync.value(),
            div: p.div.value(),
            oneshot: p.oneshot.value(),
            target: p.target.value(),
        }
    }
}

impl ModPatch {
    /// Mod C / Mod D (no Target param: `target` is `Both` and unused, they have no fixed routes).
    fn from_cd(p: &ModCdParams) -> Self {
        Self {
            curve: p.curve.value(),
            rate: p.rate.value(),
            sync: p.sync.value(),
            div: p.div.value(),
            oneshot: p.oneshot.value(),
            target: Target::Both,
        }
    }
}

/// SPEC v2 Track 7 LFO extensions of one LFO (Mod A..D). Defaults are the v1 behaviour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LfoExt {
    /// Start phase, degrees 0..360.
    pub phase: f32,
    /// Fade-in, seconds (0 = off).
    pub fade: f32,
    /// Smoothing 0..1.
    pub smooth: f32,
    pub trig: Trigger,
}

impl LfoExt {
    fn from_params(p: &LfoExtParams) -> Self {
        Self { phase: p.phase.value(), fade: p.fade.value(), smooth: p.smooth.value(), trig: p.trig.value() }
    }
}

/// Filter A or B settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilterPatch {
    pub ty: FilterType,
    /// Hz.
    pub freq: f32,
    pub res: f32,
    pub kbd: f32,
    pub env: bool,
}

impl FilterPatch {
    fn from_params(p: &FilterParams) -> Self {
        Self { ty: p.ty.value(), freq: p.freq.value(), res: p.res.value(), kbd: p.kbd.value(), env: p.env.value() }
    }
}

/// Plain-value snapshot of the parameters, taken once per block (with sample-accurate automation
/// the host splits blocks at parameter changes). Tests build one with `Patch::default()`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Patch {
    pub osc: [OscPatch; 2],
    /// Mod A, Mod B.
    pub moda: ModPatch,
    pub modb: ModPatch,
    /// Mod A amounts, bipolar: x24 st, x127, x24 st.
    pub moda_pitch: f32,
    pub moda_index: f32,
    pub moda_shift: f32,
    /// Mod B amounts, bipolar: x1, x1, x5 octaves, x1 (scales Mod A depth).
    pub modb_motion: f32,
    pub modb_vol: f32,
    pub modb_cutoff: f32,
    pub modb_moda: f32,
    pub shaper_on: bool,
    pub shaper_mode: ShaperMode,
    pub shaper_amt: f32,
    /// Filter A, Filter B.
    pub filt: [FilterPatch; 2],
    /// Seconds.
    pub fenv_att: f32,
    pub fenv_dec: f32,
    pub fenv_sus: f32,
    pub fenv_rel: f32,
    /// 0..1 = up to +8 octaves.
    pub fenv_amt: f32,
    pub fenv_inv: bool,
    /// Filter envelope curve, -1..1.
    pub fenv_curve: f32,
    /// Sub: on, level in dB, octave (-1 / -2).
    pub sub_on: bool,
    pub sub_level_db: f32,
    pub sub_oct: i32,
    /// Mono crossover, Hz.
    pub mono_xover: f32,
    pub routing: Routing,
    pub poly: usize,
    pub legato: bool,
    /// Seconds.
    pub porta: f32,
    /// Semitones.
    pub bend_range: f32,
    pub wheel_index: f32,
    pub wheel_shift: f32,
    /// x5 octaves on both filters.
    pub wheel_cutoff: f32,
    /// Scales Mod A depth by `1 + wheel_mod * wheel`.
    pub wheel_mod: f32,
    pub vel_vol_a: f32,
    pub vel_vol_b: f32,
    pub vel_fenv: f32,
    pub vel_att: f32,
    pub vel_shift: f32,
    pub vel_mod: f32,
    pub volume_db: f32,
    pub spread: f32,
    /// FX section (SPEC v2 Track 4).
    pub fx: FxPatch,
    /// Macro 1..4, 0..1 (matrix sources, SPEC v2 Track 5).
    pub macros: [f32; 4],

    // ---- SPEC v2 Track 7 modulation (appended block) ----
    /// Mod C, Mod D: matrix-only sources.
    pub modc: ModPatch,
    pub modd: ModPatch,
    /// Mod A..D extensions.
    pub lfo_ext: [LfoExt; 4],
    /// Mod envelope: seconds, sustain 0..1, curve -1..1.
    pub menv_att: f32,
    pub menv_dec: f32,
    pub menv_sus: f32,
    pub menv_rel: f32,
    pub menv_curve: f32,
    /// Pitch offsets in semitones: Osc A, Osc B, and all (Osc A, Osc B and the sub).
    pub osca_pitch: f32,
    pub oscb_pitch: f32,
    pub pitch_all: f32,
    // ---- Track 7 sound (G8, G10) ----
    /// Sub shape, start phase in degrees, soft drive 0..1.
    pub sub_shape: SubShape,
    pub sub_phase: f32,
    pub sub_drive: f32,
    /// Noise layer: type, level dB (-60 = off), tone 0..1 (0.5 flat).
    pub noise_type: NoiseType,
    pub noise_level_db: f32,
    pub noise_tone: f32,
}

impl Patch {
    pub fn from_params(p: &GtsParams) -> Self {
        Self {
            osc: [OscPatch::from_params(&p.osca), OscPatch::from_params(&p.oscb)],
            moda: ModPatch::from_params(&p.moda),
            modb: ModPatch::from_params(&p.modb),
            moda_pitch: p.moda_amt.pitch.value(),
            moda_index: p.moda_amt.index.value(),
            moda_shift: p.moda_amt.shift.value(),
            modb_motion: p.modb_amt.motion.value(),
            modb_vol: p.modb_amt.vol.value(),
            modb_cutoff: p.modb_amt.cutoff.value(),
            modb_moda: p.modb_amt.moda.value(),
            shaper_on: p.shaper.on.value(),
            shaper_mode: p.shaper.mode.value(),
            shaper_amt: p.shaper.amt.value(),
            filt: [FilterPatch::from_params(&p.filta), FilterPatch::from_params(&p.filtb)],
            fenv_att: p.fenv.att.value(),
            fenv_dec: p.fenv.dec.value(),
            fenv_sus: p.fenv.sus.value(),
            fenv_rel: p.fenv.rel.value(),
            fenv_amt: p.fenv.amt.value(),
            fenv_inv: p.fenv.inv.value(),
            fenv_curve: p.fenv.curve.value(),
            sub_on: p.sub.on.value(),
            sub_level_db: p.sub.level.value(),
            sub_oct: p.sub.oct.value(),
            mono_xover: p.mono_xover.value(),
            routing: p.routing.value(),
            poly: p.poly.value().clamp(1, MAX_VOICES as i32) as usize,
            legato: p.legato.value(),
            porta: p.porta.value(),
            bend_range: p.bend_range.value() as f32,
            wheel_index: p.wheel.index.value(),
            wheel_shift: p.wheel.shift.value(),
            wheel_cutoff: p.wheel.cutoff.value(),
            wheel_mod: p.wheel.mod_amt.value(),
            vel_vol_a: p.vel.vol_a.value(),
            vel_vol_b: p.vel.vol_b.value(),
            vel_fenv: p.vel.fenv.value(),
            vel_att: p.vel.att.value(),
            vel_shift: p.vel.shift.value(),
            vel_mod: p.vel.mod_amt.value(),
            volume_db: p.volume.value(),
            spread: p.spread.value(),
            fx: FxPatch::from_params(p),
            macros: p.macros.values(),

            modc: ModPatch::from_cd(&p.modc),
            modd: ModPatch::from_cd(&p.modd),
            lfo_ext: [&p.moda_ext, &p.modb_ext, &p.modc_ext, &p.modd_ext].map(LfoExt::from_params),
            menv_att: p.menv.att.value(),
            menv_dec: p.menv.dec.value(),
            menv_sus: p.menv.sus.value(),
            menv_rel: p.menv.rel.value(),
            menv_curve: p.menv.curve.value(),
            osca_pitch: p.pitch.osca.value(),
            oscb_pitch: p.pitch.oscb.value(),
            pitch_all: p.pitch.all.value(),
            // Track 7 sound
            sub_shape: p.sub.shape.value(),
            sub_phase: p.sub.phase.value(),
            sub_drive: p.sub.drive.value(),
            noise_type: p.noise.ty.value(),
            noise_level_db: p.noise.level.value(),
            noise_tone: p.noise.tone.value(),
        }
    }
}

impl Default for Patch {
    fn default() -> Self {
        Self::from_params(&GtsParams::default())
    }
}

impl Patch {
    /// The v1 default patch, the riddim wobble (SPEC v2 1.12 moved it out of the init): Osc A
    /// Square Morph index 64, Mod A one-shot Decay 1/8 into Index -0.5, Filter A Comb- 880 Hz res
    /// 0.6, Filter B LP12 2 kHz res 0.1, filter env 0.5, no sub, crossover off, 0 dB. Test fixture and demo
    /// patch until presets exist.
    pub fn riddim_v1() -> Self {
        let mut p = Self::default();
        p.osc[0].index = 64.0;
        p.moda.oneshot = true;
        p.moda_index = -0.5;
        p.filt[0] = FilterPatch { ty: FilterType::CombMinus, freq: 880.0, res: 0.6, kbd: 0.5, env: true };
        p.filt[1] = FilterPatch { ty: FilterType::Lp12, freq: 2000.0, res: 0.1, kbd: 0.5, env: true };
        p.fenv_amt = 0.5;
        p.sub_on = false;
        p.mono_xover = 20.0; // v1 had no crossover; 20 Hz is off
        p.volume_db = 0.0;
        p
    }
}

/// Mono crossover (SPEC v2 1.5), a Linkwitz-Riley 8 (24 dB/oct x 2) split done in mid/side:
/// the side is high-passed (LR8 high-pass), the mid goes through the LR8 allpass (low-pass +
/// high-pass of the same split, which equals the cascade of the two Butterworth-4 sections'
/// allpasses). LR high- and low-pass are in phase with that allpass at every frequency, so this is
/// `L' = LP(M) + HP(L)`, `R' = LP(M) + HP(R)`: mono below the crossover, the stereo image intact
/// above it, flat magnitude everywhere. The cutoff glides per sample; the TPT sections take moving
/// coefficients without clicks. At the 20 Hz minimum it is bypassed (the allpass would still
/// rotate the phases of a mono signal: +3.9 dB of saw crest); the filters keep running and
/// `mix` crossfades between dry and split, so switching it on or off does not click.
struct MonoXover {
    /// Smoothed `tan(π fc / sr)` and its target.
    g: f64,
    g_target: f64,
    /// Side high-pass sections (Butterworth-4 twice), mid allpass sections.
    side: [[f64; 2]; 4],
    mid: [[f64; 2]; 2],
    /// 1 = split, 0 = bypass (at `XOVER_OFF`), and the smoothed mix toward it.
    on: f64,
    mix: f64,
}

/// Mono crossover setting that means off.
const XOVER_OFF: f32 = 20.0;

/// 1/Q of the two sections of a 4th-order Butterworth.
const BW4_K: [f64; 2] = [1.847_759, 0.765_367];

/// One TPT state-variable section: (low, band, high).
#[inline]
fn svf(s: &mut [f64; 2], x: f64, g: f64, k: f64) -> (f64, f64, f64) {
    let hp = (x - (k + g) * s[0] - s[1]) / (1.0 + k * g + g * g);
    let bp = g * hp + s[0];
    s[0] = g * hp + bp;
    let lp = g * bp + s[1];
    s[1] = g * bp + lp;
    (lp, bp, hp)
}

impl MonoXover {
    fn set(&mut self, hz: f32, sr: f32) {
        self.g_target = (std::f64::consts::PI * (hz as f64).clamp(10.0, 0.45 * sr as f64) / sr as f64).tan();
        self.on = if hz > XOVER_OFF { 1.0 } else { 0.0 };
    }

    /// Jump to the targets (nothing sounding).
    fn settle(&mut self) {
        (self.g, self.mix) = (self.g_target, self.on);
    }

    fn reset(&mut self) {
        (self.side, self.mid) = ([[0.0; 2]; 4], [[0.0; 2]; 2]);
        self.settle();
    }

    /// (mid, side) -> (mid through the allpass, side through the high-pass), crossfaded with the
    /// dry pair by `mix` (exactly dry at 0).
    #[inline]
    fn process(&mut self, m0: f64, s0: f64, smooth: f64) -> (f64, f64) {
        self.g = self.g_target + (self.g - self.g_target) * smooth;
        self.mix = self.on + (self.mix - self.on) * smooth;
        if (self.mix - self.on).abs() < 1e-6 {
            self.mix = self.on;
        }
        let g = self.g;
        let (mut m, mut s) = (m0, s0);
        for (i, st) in self.side.iter_mut().enumerate() {
            s = svf(st, s, g, BW4_K[i % 2]).2;
        }
        for (st, k) in self.mid.iter_mut().zip(BW4_K) {
            m -= 2.0 * k * svf(st, m, g, k).1;
        }
        if self.mix == 0.0 {
            return (m0, s0);
        }
        (m0 + (m - m0) * self.mix, s0 + (s - s0) * self.mix)
    }
}

/// Host transport. The plugin sets it at the start of each block; `Engine::next_frame` advances
/// `pos_beats` from `tempo` while playing, so offline renders only set it once.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transport {
    pub tempo: f64,
    pub playing: bool,
    pub pos_beats: f64,
}

impl Default for Transport {
    fn default() -> Self {
        Self { tempo: 120.0, playing: false, pos_beats: 0.0 }
    }
}

pub struct Engine {
    /// Generated once (by `initialize`, or by the editor if it opens first) and shared with it.
    tables: Arc<OnceLock<Tables>>,
    xover: MonoXover,
    /// Shaper oversampling factor (1 / 2 / 4) set on every voice.
    oversampling: usize,
    voices: [Voice; MAX_VOICES],
    patch: Patch,
    ctl: Controls,
    /// Mono note stack in press order, last = sounding.
    held: [(u8, f32); MAX_HELD],
    n_held: usize,
    age: u64,
    /// Raw controller values: bend -1..1, wheel 0..1.
    bend_raw: f32,
    wheel_raw: f32,
    /// Master volume (linear) and the 1/sqrt(poly) voice-sum normalisation, smoothed.
    gain: f32,
    gain_target: f32,
    norm: f32,
    norm_target: f32,
    fx: FxChain,
    /// Sustain pedal (CC64) down, and the notes whose note-off it is holding back.
    sustain: bool,
    pending: [bool; 128],
    pub transport: Transport,
    pub log_notes: bool,
    /// Modulation matrix (SPEC v2 Track 5). `src_patch` is what the host set (`set_patch` /
    /// `set_cv_patch`); `patch` adds the global targets, `vpatch[i]` voice i's targets on top.
    mm: ModRt,
    src_patch: Patch,
    vpatch: [Patch; MAX_VOICES],
    vsrc: [VoiceSrc; MAX_VOICES],
    /// Per-voice portamento coefficient (only while Portamento is a modulated target).
    vglide: [f32; MAX_VOICES],
    chan_at: f32,
    bus_cv: [f32; 3],
    /// Samples to the next matrix evaluation, and the per-evaluation smoothing coefficient.
    mm_tick: usize,
    mm_k: f32,
    /// Per-voice sources of the newest sounding voice at the last tick that had one: global
    /// targets hold them while only tails ring (no snap to 0 at the end of a phrase).
    held_src: [f32; N_SOURCES],
}

/// Matrix sources the engine keeps per voice.
#[derive(Clone, Copy, Debug)]
struct VoiceSrc {
    /// Voice age at the last note-on seen (a change = new note).
    age: u64,
    random: f32,
    rng: u32,
    poly_at: f32,
}

/// Portamento one-pole coefficient: ~98 % of the way after `porta` seconds.
fn glide_coef(porta: f32, sr: f32) -> f32 {
    if porta > 0.0 { (-4.0 / (porta * sr)).exp() } else { 0.0 }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    /// Allocates everything (tables are filled by `initialize`).
    pub fn new() -> Self {
        let mut e = Self {
            tables: Arc::new(OnceLock::new()),
            xover: MonoXover { g: 0.0, g_target: 0.0, side: [[0.0; 2]; 4], mid: [[0.0; 2]; 2], on: 0.0, mix: 0.0 },
            oversampling: 2,
            voices: std::array::from_fn(|_| Voice::default()),
            patch: Patch::default(),
            ctl: Controls { sr: 44100.0, bend: 0.0, wheel: 0.0, glide: 0.0, smooth: 0.0, dc: 0.0, sub_g: 0.0 },
            held: [(0, 0.0); MAX_HELD],
            n_held: 0,
            age: 0,
            bend_raw: 0.0,
            wheel_raw: 0.0,
            gain: 0.0,
            gain_target: 0.0,
            norm: 1.0,
            norm_target: 1.0,
            fx: FxChain::new(),
            sustain: false,
            pending: [false; 128],
            transport: Transport::default(),
            log_notes: false,
            mm: ModRt::new(),
            src_patch: Patch::default(),
            vpatch: [Patch::default(); MAX_VOICES],
            vsrc: std::array::from_fn(|i| VoiceSrc {
                age: 0,
                random: 0.0,
                rng: 0x9E37_79B9 ^ (i as u32 + 1).wrapping_mul(0x85EB_CA6B),
                poly_at: 0.0,
            }),
            vglide: [0.0; MAX_VOICES],
            chan_at: 0.0,
            bus_cv: [0.0; 3],
            mm_tick: 0,
            mm_k: 0.0,
            held_src: [0.0; N_SOURCES],
        };
        e.set_sample_rate(44100.0);
        e
    }

    /// Sets the sample rate and generates the tables the first time. Returns the generation time
    /// when it ran.
    pub fn initialize(&mut self, sr: f32) -> Option<Duration> {
        self.set_sample_rate(sr);
        if self.tables.get().is_some() {
            return None;
        }
        let t = Instant::now();
        self.tables.get_or_init(generate_tables);
        Some(t.elapsed())
    }

    /// The shared table slot, for the editor's grain display.
    pub fn tables_handle(&self) -> Arc<OnceLock<Tables>> {
        self.tables.clone()
    }

    /// Shaper oversampling 1 / 2 / 4 on every voice (SPEC v2 1.2). Call at a block start; a change
    /// restarts the shapers with their warm-up crossfade.
    pub fn set_oversampling(&mut self, factor: usize) {
        for v in &mut self.voices {
            v.set_oversampling(factor);
        }
        self.oversampling = self.voices[0].shaper().oversampling();
    }

    pub fn oversampling(&self) -> usize {
        self.oversampling
    }

    /// Processing latency in samples: the shaper's constant delay at the current oversampling plus
    /// the FX chain's (distortion oversampler + limiter lookahead).
    pub fn latency(&self) -> usize {
        self.voices[0].shaper().latency() + self.fx.latency()
    }

    fn set_sample_rate(&mut self, sr: f32) {
        self.ctl.sr = sr;
        self.ctl.smooth = (-1.0 / (0.005 * sr)).exp();
        self.ctl.dc = 1.0 - (-std::f32::consts::TAU * 8.0 / sr).exp();
        self.ctl.sub_g = (std::f32::consts::PI * 25.0 / sr).tan();
        self.mm_k = (-(TICK as f32) / (matrix::SMOOTH_S * sr)).exp();
        for v in &mut self.voices {
            v.set_sample_rate(sr);
        }
        self.fx.set_sample_rate(sr);
        let p = self.src_patch;
        self.set_patch(&p);
        (self.gain, self.norm) = (self.gain_target, self.norm_target);
        self.xover.reset();
    }

    /// The grain tables (panics before `initialize`).
    pub fn tables(&self) -> &Tables {
        self.tables.get().expect("Engine::initialize generates the tables")
    }

    pub fn reset(&mut self) {
        for v in &mut self.voices {
            v.kill();
        }
        self.n_held = 0;
        self.sustain = false;
        self.pending = [false; 128];
        self.bend_raw = 0.0;
        self.ctl.bend = 0.0;
        (self.gain, self.norm) = (self.gain_target, self.norm_target);
        self.xover.reset();
        self.fx.reset();
        self.mm.reset();
        self.held_src = [0.0; N_SOURCES];
        self.chan_at = 0.0;
        for s in &mut self.vsrc {
            s.poly_at = 0.0;
        }
    }

    /// Call once per block, before rendering it.
    /// Mid-block update for the RR-BUS CV targets (osc index/shift, filter freq, Mod A index,
    /// volume), which voices read from the patch per sample. Unlike [`Engine::set_patch`] it leaves
    /// envelopes, LFO configuration and voice count alone; `p` must otherwise equal the block's patch.
    pub fn set_cv_patch(&mut self, p: &Patch) {
        self.src_patch = *p;
        self.patch = *p;
        self.gain_target = 10f32.powf(p.volume_db / 20.0);
        // Re-apply the current matrix offsets on the new base (no smoothing step).
        if self.mm.has_global() {
            self.apply_global();
        }
        if self.mm.has_voice() {
            self.apply_voices(false);
        }
    }

    pub fn set_patch(&mut self, p: &Patch) {
        if p.poly != self.patch.poly {
            // Only voices the new count no longer reaches are released.
            for v in &mut self.voices[p.poly..] {
                v.release();
            }
            self.n_held = 0;
            // Into mono: voice 0's held key becomes the note stack, so its note-off still lands.
            let v = &self.voices[0];
            if p.poly <= 1 && v.gate {
                self.held[0] = (v.note, v.velocity);
                self.n_held = 1;
            }
        }
        self.src_patch = *p;
        self.patch = *p;
        let sr = self.ctl.sr;
        self.xover.set(p.mono_xover, sr);
        if !self.voices.iter().any(|v| v.is_active()) {
            // Nothing sounding: take a new crossover at once instead of gliding into the next note.
            self.xover.settle();
        }
        // Portamento: one-pole in semitones, ~98 % of the way after `porta` seconds.
        self.ctl.glide = glide_coef(p.porta, sr);
        // 1/sqrt(poly), applied before the FX chain, keeps chords out of the limiter and the FX at
        // voice level (mono unchanged).
        self.gain_target = 10f32.powf(p.volume_db / 20.0);
        self.norm_target = 1.0 / (p.poly as f32).sqrt();
        let t = self.transport;
        self.fx.set(&p.fx, t.tempo, t.playing.then_some(t.pos_beats));
        if self.mm.has_global() {
            self.apply_global();
        }
        if self.mm.has_voice() {
            // Each voice updates from its own patch only: an update from the base patch first
            // would flip modulated unison counts back and forth every block.
            self.apply_voices(true);
        } else {
            for v in &mut self.voices {
                v.update(p, sr);
                v.configure_mods(p, &self.transport);
            }
        }
    }

    // ---- modulation matrix -------------------------------------------------------------------

    /// Takes a new encoded matrix (`matrix::Matrix::encode`) at a block start and re-applies the
    /// patch with it. Allocation-free.
    pub fn set_matrix(&mut self, enc: &[f32; ENC_LEN]) {
        self.mm.load(enc);
        let p = self.src_patch;
        self.set_patch(&p);
    }

    /// The matrix has an enabled slot (the plugin then re-applies CV / matrix every 32 samples).
    pub fn matrix_on(&self) -> bool {
        self.mm.is_on()
    }

    pub fn matrix(&self) -> &ModRt {
        &self.mm
    }

    /// Smoothed RR-BUS CV-in values (matrix sources Bus CV 1..3).
    pub fn set_bus_cv(&mut self, cv: [f32; 3]) {
        self.bus_cv = cv;
    }

    /// Takes new custom LFO shapes (`dsp::shape::ShapesShared::poll`) at a block start: every
    /// voice's Mod A..D gets its shape. Allocation-free.
    pub fn set_lfo_shapes(&mut self, enc: &[f32; shape::SHAPES_ENC]) {
        for (k, rec) in enc.chunks_exact(shape::ENC).enumerate() {
            let s = shape::Shape::decode(rec);
            for v in &mut self.voices {
                v.set_lfo_shape(k, &s);
            }
        }
    }

    /// The patch voice `i` renders with (its voice-scope targets applied).
    pub fn voice_patch(&self, i: usize) -> &Patch {
        if self.mm.has_voice() { &self.vpatch[i] } else { &self.patch }
    }

    /// Index of the newest sounding voice.
    pub fn newest(&self) -> Option<usize> {
        (0..MAX_VOICES).filter(|&i| self.voices[i].sounding()).max_by_key(|&i| self.voices[i].age)
    }

    /// Matrix source values for voice `v` (per-voice sources 0 when `None`), in `Source` order.
    pub fn sources(&self, v: Option<usize>) -> [f32; N_SOURCES] {
        let mut s = [0.0; N_SOURCES];
        if let Some(i) = v {
            let (v, x) = (&self.voices[i], &self.vsrc[i]);
            s[Source::ModA as usize] = v.lfo_out[0];
            s[Source::ModB as usize] = v.lfo_out[1];
            s[Source::FilterEnv as usize] = v.filter_env().level();
            s[Source::AmpEnvA as usize] = v.amp_env(0).level();
            s[Source::AmpEnvB as usize] = v.amp_env(1).level();
            s[Source::Velocity as usize] = v.velocity;
            s[Source::Keytrack as usize] = ((v.pitch() - voice::KBD_REF) / 60.0).clamp(-1.0, 1.0);
            s[Source::Random as usize] = x.random;
            s[Source::PolyAftertouch as usize] = x.poly_at;
            s[Source::ModC as usize] = v.lfo_out[2];
            s[Source::ModD as usize] = v.lfo_out[3];
            s[Source::ModEnv as usize] = v.mod_env().level();
        }
        s[Source::ModWheel as usize] = self.ctl.wheel;
        s[Source::ChannelAftertouch as usize] = self.chan_at;
        s[Source::Macro1 as usize..Source::Macro1 as usize + 4].copy_from_slice(&self.src_patch.macros);
        s[Source::BusCv1 as usize..Source::BusCv1 as usize + 3].copy_from_slice(&self.bus_cv);
        s
    }

    /// `patch` = `src_patch` + the global offsets, and the settings that do not read the patch
    /// per sample.
    fn apply_global(&mut self) {
        self.patch = self.src_patch;
        self.mm.write_global(&mut self.patch);
        let (f, t) = (self.mm.gflags(), self.transport);
        let p = &self.patch;
        if f & flag::FX != 0 {
            self.fx.set(&p.fx, t.tempo, t.playing.then_some(t.pos_beats));
        }
        if f & flag::XOVER != 0 {
            self.xover.set(p.mono_xover, self.ctl.sr);
        }
        if f & flag::VOLUME != 0 {
            self.gain_target = 10f32.powf(p.volume_db / 20.0);
        }
    }

    /// Every voice's patch from `patch` and its current offsets. `full`: also update every voice
    /// (block start), else only active voices and only the settings the targets need.
    fn apply_voices(&mut self, full: bool) {
        self.mm.prepare_voice_base(&self.patch);
        for i in 0..MAX_VOICES {
            if full || self.voices[i].is_active() {
                self.apply_voice(i, full);
            }
        }
    }

    /// Voice `i`'s patch (base cached by `prepare_voice_base`).
    fn apply_voice(&mut self, i: usize, full: bool) {
        let mut vp = self.patch;
        self.mm.write_voice(i, &mut vp);
        self.vpatch[i] = vp;
        let (f, sr) = (self.mm.vflags(), self.ctl.sr);
        let v = &mut self.voices[i];
        if full || f & flag::UPDATE != 0 {
            v.update(&vp, sr);
        }
        if full || f & flag::LFO != 0 {
            v.configure_mods(&vp, &self.transport);
        }
        if f & flag::VEL_GAIN != 0 {
            v.set_vel_gain(&vp);
        }
        if f & flag::PORTA != 0 {
            self.vglide[i] = glide_coef(vp.porta, sr);
        }
    }

    /// One matrix evaluation (every `TICK` samples): sources -> smoothed offsets -> patches.
    fn mod_tick(&mut self) {
        let k = self.mm_k;
        if self.mm.has_global() {
            let newest = self.newest();
            let mut src = self.sources(newest);
            for k in (0..N_SOURCES).filter(|&k| Source::ALL[k].per_voice()) {
                if newest.is_some() {
                    self.held_src[k] = src[k];
                } else {
                    src[k] = self.held_src[k];
                }
            }
            self.mm.eval_global(&src, k);
            self.apply_global();
        }
        if self.mm.has_voice() {
            self.mm.prepare_voice_base(&self.patch);
            for i in 0..MAX_VOICES {
                if self.voices[i].is_active() {
                    let src = self.sources(Some(i));
                    self.mm.eval_voice(i, &src, k);
                    self.apply_voice(i, false);
                }
            }
        }
        let voices = &self.voices;
        self.mm.prune(|i| voices[i].is_active());
    }

    /// After note events: a voice with a new note-on draws its Random value, clears its
    /// pressure and (matrix on) jumps to its own modulation before its first sample.
    fn mod_new_notes(&mut self) {
        for i in 0..MAX_VOICES {
            let age = self.voices[i].age;
            let s = &mut self.vsrc[i];
            if age == s.age {
                continue;
            }
            s.age = age;
            s.rng ^= s.rng << 13;
            s.rng ^= s.rng >> 17;
            s.rng ^= s.rng << 5;
            s.random = (s.rng >> 8) as f32 / (1u32 << 23) as f32 - 1.0;
            s.poly_at = 0.0;
            if self.mm.has_voice() {
                self.mm.prepare_voice_base(&self.patch);
                self.voices[i].peek_lfos();
                let src = self.sources(Some(i));
                self.mm.eval_voice(i, &src, 0.0);
                self.apply_voice(i, true);
            }
        }
    }

    pub fn handle_event(&mut self, event: &NoteEvent<()>) {
        match *event {
            NoteEvent::NoteOn { note, velocity, .. } => self.note_on(note, velocity),
            NoteEvent::NoteOff { note, .. } => self.note_off(note),
            NoteEvent::Choke { note, .. } => self.choke(note),
            NoteEvent::MidiPitchBend { value, .. } => self.pitch_bend(value),
            NoteEvent::MidiCC { cc: 1, value, .. } => self.mod_wheel(value),
            NoteEvent::MidiCC { cc: 64, value, .. } => self.sustain_pedal(value >= 0.5),
            NoteEvent::MidiCC { cc: 120 | 123, .. } => self.all_notes_off(),
            NoteEvent::PolyPressure { note, pressure, .. } => self.poly_pressure(note, pressure),
            NoteEvent::MidiChannelPressure { pressure, .. } => self.chan_at = pressure.clamp(0.0, 1.0),
            _ => {}
        }
    }

    /// Poly aftertouch for the held voices of `note`, 0..1 (matrix source).
    pub fn poly_pressure(&mut self, note: u8, pressure: f32) {
        for (v, s) in self.voices.iter().zip(&mut self.vsrc) {
            if v.gate && v.note == note {
                s.poly_at = pressure.clamp(0.0, 1.0);
            }
        }
    }

    /// Pitch bend as delivered by nih-plug: 0..1, 0.5 = centre.
    pub fn pitch_bend(&mut self, value: f32) {
        self.bend_raw = (value * 2.0 - 1.0).clamp(-1.0, 1.0);
    }

    /// Mod wheel (CC1), 0..1.
    pub fn mod_wheel(&mut self, value: f32) {
        self.wheel_raw = value.clamp(0.0, 1.0);
    }

    /// Sustain pedal: while down, note-offs are deferred; lifting it flushes them.
    pub fn sustain_pedal(&mut self, down: bool) {
        self.sustain = down;
        if !down {
            for n in 0..128u8 {
                if std::mem::take(&mut self.pending[n as usize]) {
                    self.note_off(n);
                }
            }
        }
    }

    /// CC120 / CC123: release every voice and forget held and sustained keys.
    pub fn all_notes_off(&mut self) {
        for v in &mut self.voices {
            v.release();
        }
        self.n_held = 0;
        self.pending = [false; 128];
    }

    /// Smoothed mod wheel, 0..1 (what the voices use; the filter/LFO stages read it too).
    pub fn wheel(&self) -> f32 {
        self.ctl.wheel
    }

    /// The patch the voices currently read (the block's, CV included).
    pub fn patch(&self) -> &Patch {
        &self.patch
    }

    pub fn voice(&self, i: usize) -> &Voice {
        &self.voices[i]
    }

    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.is_active()).count()
    }

    /// RR-BUS signals `[Mod A, Mod B, Gate, Filter Env, Amp Env]`: Gate is 1 while any key is
    /// held, the rest come from the most recently triggered active voice (0 when none).
    pub fn bus_signals(&self) -> [f32; 5] {
        let gate = if self.voices.iter().any(|v| v.gate) { 1.0 } else { 0.0 };
        match self.voices.iter().filter(|v| v.sounding()).max_by_key(|v| v.age) {
            Some(v) => [v.lfo_out[0], v.lfo_out[1], gate, v.filter_env().level(), v.amp_level()],
            None => [0.0, 0.0, gate, 0.0, 0.0],
        }
    }

    /// FX display record (peaks since the last call). Audio thread, once per block.
    pub fn fx_telemetry(&mut self) -> fx::FxTelemetry {
        self.fx.telemetry()
    }

    /// Limiter gain reduction in dB (>= 0), deepest since the last call. Audio thread, once per block.
    pub fn limiter_gr_db(&mut self) -> f32 {
        self.fx.limiter_gr_db()
    }

    fn glide_allowed(&self, v: &Voice) -> bool {
        // Fingered portamento with legato on (glide only between overlapping notes), always
        // glide from a sounding voice with legato off. Mono only: in poly a reused voice's pitch
        // belongs to an unrelated note.
        self.patch.poly <= 1 && v.sounding() && (!self.patch.legato || v.gate)
    }

    pub fn note_on(&mut self, note: u8, velocity: f32) {
        self.note_on_inner(note, velocity);
        self.mod_new_notes();
    }

    fn note_on_inner(&mut self, note: u8, velocity: f32) {
        // A re-pressed key is held again; its deferred note-off no longer applies.
        self.pending[note as usize & 127] = false;
        self.age += 1;
        let (patch, ctl, age) = (self.patch, self.ctl, self.age);
        if patch.poly <= 1 {
            self.remove_held(note);
            if self.n_held == MAX_HELD {
                self.held.copy_within(1.., 0);
                self.n_held -= 1;
            }
            self.held[self.n_held] = (note, velocity);
            self.n_held += 1;
            let glide = self.glide_allowed(&self.voices[0]);
            // The voice's own (matrix) patch: its modulated unison count etc. survive the retrigger.
            let vp = *self.voice_patch(0);
            let v = &mut self.voices[0];
            // Legato: a new key while one is held moves the pitch without retriggering.
            let retrigger = !(patch.legato && v.gate);
            v.trigger(note, velocity, glide, retrigger, age, &vp, &ctl);
            v.configure_mods(&vp, &self.transport);
            return;
        }
        let n = patch.poly.min(MAX_VOICES);
        let vs = &self.voices[..n];
        let oldest = |pred: &dyn Fn(&Voice) -> bool| {
            vs.iter().enumerate().filter(|(_, v)| pred(v)).min_by_key(|(_, v)| v.age).map(|(i, _)| i)
        };
        let i = oldest(&|v| v.gate && v.note == note)
            .or_else(|| oldest(&|v| !v.is_active()))
            .or_else(|| oldest(&|v| !v.gate))
            .or_else(|| oldest(&|_| true))
            .unwrap_or(0);
        let glide = self.glide_allowed(&self.voices[i]);
        let vp = *self.voice_patch(i);
        self.voices[i].trigger(note, velocity, glide, true, age, &vp, &ctl);
        // Lock synced LFOs to the grid now, not at the next block (that would step mid-note).
        self.voices[i].configure_mods(&vp, &self.transport);
    }

    pub fn note_off(&mut self, note: u8) {
        self.note_off_inner(note);
        // Mono: falling back to a held key retriggers.
        self.mod_new_notes();
    }

    fn note_off_inner(&mut self, note: u8) {
        if self.sustain {
            self.pending[note as usize & 127] = true;
            return;
        }
        let (patch, ctl) = (self.patch, self.ctl);
        if patch.poly <= 1 {
            if !self.remove_held(note) {
                return;
            }
            let vp = *self.voice_patch(0);
            let v = &mut self.voices[0];
            if !(v.gate && v.note == note) {
                return;
            }
            if self.n_held == 0 {
                v.release();
            } else {
                // Fall back to the most recent still-held key.
                let (n, vel) = self.held[self.n_held - 1];
                self.age += 1;
                v.trigger(n, vel, true, !patch.legato, self.age, &vp, &ctl);
                v.configure_mods(&vp, &self.transport);
            }
            return;
        }
        for v in &mut self.voices {
            if v.gate && v.note == note {
                v.release();
            }
        }
    }

    pub fn choke(&mut self, note: u8) {
        self.remove_held(note);
        for v in &mut self.voices {
            if v.note == note {
                v.kill();
            }
        }
    }

    fn remove_held(&mut self, note: u8) -> bool {
        match self.held[..self.n_held].iter().position(|h| h.0 == note) {
            Some(i) => {
                self.held.copy_within(i + 1..self.n_held, i);
                self.n_held -= 1;
                true
            }
            None => false,
        }
    }

    /// One stereo frame.
    #[inline]
    pub fn next_frame(&mut self) -> (f32, f32) {
        let k = self.ctl.smooth;
        let bend = self.bend_raw * self.patch.bend_range;
        self.ctl.bend = bend + (self.ctl.bend - bend) * k;
        self.ctl.wheel = self.wheel_raw + (self.ctl.wheel - self.wheel_raw) * k;
        self.gain = self.gain_target + (self.gain - self.gain_target) * k;
        self.norm = self.norm_target + (self.norm - self.norm_target) * k;
        if self.mm.is_on() {
            if self.mm_tick == 0 {
                self.mod_tick();
                self.mm_tick = TICK;
            }
            self.mm_tick -= 1;
        }

        if self.transport.playing {
            self.transport.pos_beats += self.transport.tempo / 60.0 / self.ctl.sr as f64;
        }
        let Some(tables) = self.tables.get() else {
            return (0.0, 0.0);
        };
        let (mut l, mut r) = (0.0, 0.0);
        let vm = self.mm.has_voice();
        let porta = vm && self.mm.vflags() & flag::PORTA != 0;
        for (i, v) in self.voices.iter_mut().enumerate() {
            if !v.is_active() {
                continue;
            }
            let p = if vm { &self.vpatch[i] } else { &self.patch };
            let glided;
            let ctl = if porta {
                glided = Controls { glide: self.vglide[i], ..self.ctl };
                &glided
            } else {
                &self.ctl
            };
            let (a, b) = v.next(tables, p, ctl);
            l += a;
            r += b;
        }
        // Mono below the crossover.
        let (m, s) = self.xover.process(0.5 * (l as f64 + r as f64), 0.5 * (l as f64 - r as f64), self.ctl.smooth as f64);
        let (mut l, mut r) = ((m + s) as f32 * self.norm, (m - s) as f32 * self.norm);
        // FX 1..7, master volume, then the -0.3 dBFS limiter (the output stage).
        self.fx.process(&mut l, &mut r, self.gain);
        (l, r)
    }
}

fn generate_tables() -> Tables {
    let mut t = Tables::new();
    t.generate();
    t
}

