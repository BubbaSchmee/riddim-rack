//! The complete GTS parameter contract (SPEC.md "Parameters"). Ids are stable; never rename.
//!
//! Units: all envelope/portamento times are stored in SECONDS (f32). Frequencies in Hz, tuning in
//! semitones/cents, `volume` in dB, everything else normalized as noted per field.
//! Nested structs use `id_prefix`, so `osca.index` has the id `osca_index`, `filta.freq` has
//! `filta_freq`, and so on. Mod A has two nested structs sharing the prefix `moda`.

use nih_plug::prelude::*;

pub use crate::dsp::fx::delay::{DelayMode, Div as DelayDiv, Feel as DelayFeel};
pub use crate::dsp::fx::distortion::{DistType, FilterMode as DistFilterMode, FilterType as DistFilterType};
pub use crate::dsp::fx::reverb::ReverbType;

#[derive(Enum, Debug, PartialEq, Eq, Clone, Copy)]
pub enum Table {
    #[name = "Square Morph"]
    SquareMorph,
    #[name = "Saw Stack"]
    SawStack,
    #[name = "Harmonic Rise"]
    HarmonicRise,
    #[name = "Vowel"]
    Vowel,
    #[name = "FM Morph"]
    FmMorph,
    #[name = "Metal"]
    Metal,
    #[name = "Reed"]
    Reed,
    #[name = "Grit"]
    Grit,
    /// Appended (saved state is append-only): sine -> triangle -> saw -> square.
    #[name = "Basic"]
    Basic,
}

#[derive(Enum, Debug, PartialEq, Eq, Clone, Copy)]
pub enum Curve {
    #[name = "Sine"]
    Sine,
    #[name = "Triangle"]
    Triangle,
    #[name = "Saw Up"]
    SawUp,
    #[name = "Saw Down"]
    SawDown,
    #[name = "Square"]
    Square,
    #[name = "Sample & Hold"]
    SampleHold,
    #[name = "Smooth Random"]
    SmoothRandom,
    #[name = "Decay"]
    Decay,
    #[name = "Attack"]
    Attack,
    /// Appended (SPEC v2 Track 7, G1): the LFO's drawn / step shape (`dsp::shape`).
    #[name = "Custom"]
    Custom,
}

#[derive(Enum, Debug, PartialEq, Eq, Clone, Copy)]
pub enum Division {
    #[name = "4/1"]
    Bars4,
    #[name = "2/1"]
    Bars2,
    #[name = "1/1"]
    Bar1,
    #[name = "1/2"]
    Half,
    #[name = "1/2T"]
    HalfT,
    #[name = "1/4"]
    Quarter,
    #[name = "1/4T"]
    QuarterT,
    #[name = "1/8"]
    Eighth,
    #[name = "1/8T"]
    EighthT,
    #[name = "1/16"]
    Sixteenth,
    #[name = "1/16T"]
    SixteenthT,
    #[name = "1/32"]
    ThirtySecond,
    #[name = "1/32T"]
    ThirtySecondT,
    // Appended (SPEC v2 Track 7, G6): dotted divisions.
    #[name = "1/2D"]
    HalfD,
    #[name = "1/4D"]
    QuarterD,
    #[name = "1/8D"]
    EighthD,
    #[name = "1/16D"]
    SixteenthD,
}

/// LFO trigger mode (SPEC v2 Track 7, G6). Synced LFOs lock to the host grid while it plays in
/// Retrigger and Free mode.
#[derive(Enum, Debug, PartialEq, Eq, Clone, Copy)]
pub enum Trigger {
    /// Phase restarts at every note-on (the v1 behaviour).
    #[name = "Retrigger"]
    Retrigger,
    /// Phase runs on across notes.
    #[name = "Free"]
    Free,
    /// One cycle per note, then holds the end (= One-Shot).
    #[name = "Envelope"]
    Envelope,
}

#[derive(Enum, Debug, PartialEq, Eq, Clone, Copy)]
pub enum Target {
    #[name = "A"]
    A,
    #[name = "B"]
    B,
    #[name = "Both"]
    Both,
}

#[derive(Enum, Debug, PartialEq, Eq, Clone, Copy)]
pub enum ShaperMode {
    #[name = "Sine"]
    Sine,
    #[name = "Saturate"]
    Saturate,
    #[name = "Clip"]
    Clip,
    #[name = "Quant"]
    Quant,
    #[name = "Noise"]
    Noise,
}

#[derive(Enum, Debug, PartialEq, Eq, Clone, Copy)]
pub enum FilterType {
    #[name = "LP12"]
    Lp12,
    #[name = "BP12"]
    Bp12,
    #[name = "Comb+"]
    CombPlus,
    #[name = "Comb-"]
    CombMinus,
    #[name = "AM"]
    Am,
    // ---- Track 7 (G3, G5), appended: saved state is append-only ----
    #[name = "HP12"]
    Hp12,
    #[name = "Notch"]
    Notch,
    #[name = "LP24"]
    Lp24,
    #[name = "Formant"]
    Formant,
}

/// Oscillator warp (SPEC v2 Track 7, G4). Order is saved state: append only.
#[derive(Enum, Debug, PartialEq, Eq, Clone, Copy)]
pub enum WarpMode {
    #[name = "Off"]
    Off,
    /// Through-zero phase modulation from the other oscillator's output.
    #[name = "FM"]
    Fm,
    /// Hard sync to the other oscillator's period.
    #[name = "Sync"]
    Sync,
    #[name = "Bend+"]
    BendPlus,
    #[name = "Bend-"]
    BendMinus,
    #[name = "PWM"]
    Pwm,
}

/// Sub oscillator shape (Track 7, G10).
#[derive(Enum, Debug, PartialEq, Eq, Clone, Copy)]
pub enum SubShape {
    #[name = "Sine"]
    Sine,
    #[name = "Triangle"]
    Triangle,
}

/// Noise layer colour (Track 7, G8).
#[derive(Enum, Debug, PartialEq, Eq, Clone, Copy)]
pub enum NoiseType {
    #[name = "White"]
    White,
    #[name = "Pink"]
    Pink,
    #[name = "Brown"]
    Brown,
}

#[derive(Enum, Debug, PartialEq, Eq, Clone, Copy)]
pub enum Routing {
    #[name = "Parallel"]
    Parallel,
    #[name = "Series"]
    Series,
    #[name = "Bypass"]
    Bypass,
}

/// RR-BUS CV-in targets (SPEC "RR-BUS — inter-device modulation"). Order is part of the saved
/// state: append only.
#[derive(Enum, Debug, PartialEq, Eq, Clone, Copy)]
pub enum CvTarget {
    #[name = "Off"]
    Off,
    #[name = "Osc A Index"]
    OscAIndex,
    #[name = "Osc A Shift"]
    OscAShift,
    #[name = "Osc B Index"]
    OscBIndex,
    #[name = "Osc B Shift"]
    OscBShift,
    #[name = "Filter A Freq"]
    FilterAFreq,
    #[name = "Filter B Freq"]
    FilterBFreq,
    #[name = "Mod A Amount"]
    ModAAmount,
    #[name = "Volume"]
    Volume,
}

// ---- helpers -------------------------------------------------------------------------------

fn unit(name: String, default: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min: 0.0, max: 1.0 })
        .with_value_to_string(formatters::v2s_f32_rounded(2))
}

fn bipolar(name: String, default: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min: -1.0, max: 1.0 })
        .with_value_to_string(formatters::v2s_f32_rounded(2))
}

fn semitones(name: String, default: f32, range: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min: -range, max: range })
        .with_unit(" st")
        .with_value_to_string(formatters::v2s_f32_rounded(2))
}

/// 1 ms .. 10 s, log-ish skew, value in seconds.
fn time(name: String, default_s: f32) -> FloatParam {
    FloatParam::new(
        name,
        default_s,
        FloatRange::Skewed { min: 0.001, max: 10.0, factor: FloatRange::skew_factor(-2.5) },
    )
    .with_value_to_string(Arc::new(|v| {
        if v < 1.0 {
            format!("{:.0} ms", v * 1000.0)
        } else {
            format!("{:.2} s", v)
        }
    }))
    .with_string_to_value(Arc::new(|s| {
        let s = s.trim().to_lowercase();
        if let Some(ms) = s.strip_suffix("ms") {
            ms.trim().parse::<f32>().ok().map(|v| v / 1000.0)
        } else {
            s.strip_suffix('s').unwrap_or(&s).trim().parse::<f32>().ok()
        }
    }))
}

use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::{Arc, RwLock};

// ---- nested groups -------------------------------------------------------------------------

/// One oscillator. Ids: `osc{a,b}_{on,table,oct,semi,cent,index,motion,shift,vol,att,dec,sus,rel}`,
/// v2: `_curve` (amp envelope curve) and the unison set `_{unison,detune,det_curve,blend,width,
/// rand_phase,phase_reset}`.
#[derive(Params)]
pub struct OscParams {
    #[id = "on"]
    pub on: BoolParam,
    #[id = "table"]
    pub table: EnumParam<Table>,
    /// Octaves, -4..+4.
    #[id = "oct"]
    pub oct: IntParam,
    /// Semitones, 0..12.
    #[id = "semi"]
    pub semi: IntParam,
    /// Cents, -50..+50.
    #[id = "cent"]
    pub cent: FloatParam,
    /// Grain index, 0..127.
    #[id = "index"]
    pub index: FloatParam,
    /// -1..+1; 0 static, >0 forward scan, <0 ping-pong, magnitude = speed (1.0 = table per 0.25 s).
    #[id = "motion"]
    pub motion: FloatParam,
    /// Grain resample shift in semitones, -24..+24 (moves spectrum, not pitch).
    #[id = "shift"]
    pub shift: FloatParam,
    #[id = "vol"]
    pub vol: FloatParam,
    #[id = "att"]
    pub att: FloatParam,
    #[id = "dec"]
    pub dec: FloatParam,
    #[id = "sus"]
    pub sus: FloatParam,
    #[id = "rel"]
    pub rel: FloatParam,
    /// Amp envelope curve, -1 log .. 0 exponential (the v1 shape) .. +1 linear.
    #[id = "curve"]
    pub curve: FloatParam,
    /// Unison voices, 1..7.
    #[id = "unison"]
    pub unison: IntParam,
    /// Unison detune of the outermost voices in cents, 0..50.
    #[id = "detune"]
    pub detune: FloatParam,
    /// Detune spacing, 0 linear .. 1 power (inner voices cluster, outer ones spread).
    #[id = "det_curve"]
    pub det_curve: FloatParam,
    /// Side-voice level against the centre voice, 0 (centre only) .. 1 (all equal).
    #[id = "blend"]
    pub blend: FloatParam,
    /// Stereo width of the unison voices, 0..1.
    #[id = "width"]
    pub width: FloatParam,
    /// Random start phase per unison voice, 0..1 of a cycle.
    #[id = "rand_phase"]
    pub rand_phase: FloatParam,
    /// On: every note starts the oscillator at the same (seeded) phases. Off: free-running.
    #[id = "phase_reset"]
    pub phase_reset: BoolParam,
    // ---- Track 7 (G4) ----
    /// Warp mode (default Off) and amount 0..1.
    #[id = "warp_mode"]
    pub warp_mode: EnumParam<WarpMode>,
    #[id = "warp_amt"]
    pub warp_amt: FloatParam,
}

impl OscParams {
    fn new(letter: &str, on: bool, table: Table, index: f32) -> Self {
        let n = |s: &str| format!("Osc {letter} {s}");
        Self {
            on: BoolParam::new(n("On"), on),
            table: EnumParam::new(n("Table"), table),
            oct: IntParam::new(n("Octave"), 0, IntRange::Linear { min: -4, max: 4 }),
            semi: IntParam::new(n("Semi"), 0, IntRange::Linear { min: 0, max: 12 }),
            cent: FloatParam::new(n("Cent"), 0.0, FloatRange::Linear { min: -50.0, max: 50.0 })
                .with_unit(" ct")
                .with_value_to_string(formatters::v2s_f32_rounded(1)),
            index: FloatParam::new(n("Index"), index, FloatRange::Linear { min: 0.0, max: 127.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(1)),
            motion: bipolar(n("Motion"), 0.0),
            shift: semitones(n("Shift"), 0.0, 24.0),
            vol: unit(n("Vol"), 1.0),
            att: time(n("Attack"), 0.005),
            dec: time(n("Decay"), 0.3),
            sus: unit(n("Sustain"), 1.0),
            rel: time(n("Release"), 0.02),
            curve: bipolar(n("Env Curve"), 0.0),
            unison: IntParam::new(n("Unison"), 1, IntRange::Linear { min: 1, max: 7 }),
            detune: FloatParam::new(n("Detune"), 12.0, FloatRange::Linear { min: 0.0, max: 50.0 })
                .with_unit(" ct")
                .with_value_to_string(formatters::v2s_f32_rounded(1)),
            det_curve: unit(n("Detune Curve"), 0.0),
            blend: unit(n("Blend"), 0.7),
            width: unit(n("Width"), 0.5),
            rand_phase: unit(n("Rand Phase"), 0.0),
            phase_reset: BoolParam::new(n("Phase Reset"), true),
            warp_mode: EnumParam::new(n("Warp"), WarpMode::Off),
            warp_amt: unit(n("Warp Amt"), 0.0),
        }
    }
}

/// Shared by Mod A and Mod B. Ids: `mod{a,b}_{curve,rate,sync,div,oneshot,target}`.
#[derive(Params)]
pub struct ModParams {
    #[id = "curve"]
    pub curve: EnumParam<Curve>,
    /// Free rate in Hz, 0.03..100 (log; SPEC v2 Track 7 raised the ceiling from 30).
    #[id = "rate"]
    pub rate: FloatParam,
    #[id = "sync"]
    pub sync: BoolParam,
    #[id = "div"]
    pub div: EnumParam<Division>,
    #[id = "oneshot"]
    pub oneshot: BoolParam,
    #[id = "target"]
    pub target: EnumParam<Target>,
}

impl ModParams {
    fn new(letter: &str, curve: Curve, sync: bool, oneshot: bool) -> Self {
        let n = |s: &str| format!("Mod {letter} {s}");
        Self {
            curve: EnumParam::new(n("Curve"), curve),
            rate: lfo_rate(n("Rate")),
            sync: BoolParam::new(n("Sync"), sync),
            div: EnumParam::new(n("Division"), Division::Eighth),
            oneshot: BoolParam::new(n("One-Shot"), oneshot),
            target: EnumParam::new(n("Target"), Target::Both),
        }
    }
}

/// LFO free rate, 0.03..100 Hz. The skew keeps the 4 Hz default at the normalized position it
/// had on the v1 0.03..30 Hz range (saved states and presets store plain values, so they load
/// unchanged).
fn lfo_rate(name: String) -> FloatParam {
    // v1: ((4 - 0.03) / 29.97)^0.25; solve ((4 - 0.03) / 99.97)^k for the same value.
    let k = ((4.0f32 - 0.03) / 29.97).powf(0.25).ln() / ((4.0f32 - 0.03) / 99.97).ln();
    FloatParam::new(name, 4.0, FloatRange::Skewed { min: 0.03, max: 100.0, factor: k })
        .with_value_to_string(formatters::v2s_f32_rounded(2))
        .with_unit(" Hz")
}

/// Mod A amounts, all bipolar -1..+1. Ids: `moda_{pitch,index,shift}`.
#[derive(Params)]
pub struct ModAAmounts {
    /// x ±24 st.
    #[id = "pitch"]
    pub pitch: FloatParam,
    /// x ±127.
    #[id = "index"]
    pub index: FloatParam,
    /// x ±24 st.
    #[id = "shift"]
    pub shift: FloatParam,
}

/// Mod B amounts, all bipolar -1..+1. Ids: `modb_{motion,vol,cutoff,moda}`.
#[derive(Params)]
pub struct ModBAmounts {
    /// x ±1.
    #[id = "motion"]
    pub motion: FloatParam,
    /// x ±1.
    #[id = "vol"]
    pub vol: FloatParam,
    /// x ±5 octaves.
    #[id = "cutoff"]
    pub cutoff: FloatParam,
    /// Scales all Mod A amounts, x ±1.
    #[id = "moda"]
    pub moda: FloatParam,
}

#[derive(Params)]
pub struct ShaperParams {
    #[id = "on"]
    pub on: BoolParam,
    #[id = "mode"]
    pub mode: EnumParam<ShaperMode>,
    #[id = "amt"]
    pub amt: FloatParam,
}

/// Filter A or B. Ids: `filt{a,b}_{type,freq,res,kbd,env}`.
#[derive(Params)]
pub struct FilterParams {
    #[id = "type"]
    pub ty: EnumParam<FilterType>,
    /// Hz, 20..20000 (log).
    #[id = "freq"]
    pub freq: FloatParam,
    #[id = "res"]
    pub res: FloatParam,
    /// Keyboard tracking, 0..1.
    #[id = "kbd"]
    pub kbd: FloatParam,
    /// Filter envelope on/off.
    #[id = "env"]
    pub env: BoolParam,
}

impl FilterParams {
    fn new(letter: &str, ty: FilterType, freq: f32, res: f32) -> Self {
        let n = |s: &str| format!("Filter {letter} {s}");
        Self {
            ty: EnumParam::new(n("Type"), ty),
            freq: FloatParam::new(
                n("Freq"),
                freq,
                FloatRange::Skewed { min: 20.0, max: 20000.0, factor: FloatRange::skew_factor(-2.5) },
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(1))
            .with_string_to_value(formatters::s2v_f32_hz_then_khz()),
            res: unit(n("Res"), res),
            kbd: unit(n("Kbd"), 0.5),
            env: BoolParam::new(n("Env"), true),
        }
    }
}

/// Shared filter envelope. Ids: `fenv_{att,dec,sus,rel,amt,inv,curve}`.
#[derive(Params)]
pub struct FilterEnvParams {
    #[id = "att"]
    pub att: FloatParam,
    #[id = "dec"]
    pub dec: FloatParam,
    #[id = "sus"]
    pub sus: FloatParam,
    #[id = "rel"]
    pub rel: FloatParam,
    /// 0..1 = up to +8 octaves.
    #[id = "amt"]
    pub amt: FloatParam,
    #[id = "inv"]
    pub inv: BoolParam,
    /// -1 log .. 0 exponential (the v1 shape) .. +1 linear.
    #[id = "curve"]
    pub curve: FloatParam,
}

/// Sine sub oscillator (SPEC v2 1.5). Ids: `sub_{on,level,oct}`.
#[derive(Params)]
pub struct SubParams {
    #[id = "on"]
    pub on: BoolParam,
    /// dB, -60..0 (0 dB = the RMS of a full-level oscillator).
    #[id = "level"]
    pub level: FloatParam,
    /// Octaves below Osc A, -2..0 (Track 7 added 0; stored plain values keep their meaning).
    #[id = "oct"]
    pub oct: IntParam,
    // ---- Track 7 (G10) ----
    #[id = "shape"]
    pub shape: EnumParam<SubShape>,
    /// Start phase at note-on, degrees 0..360.
    #[id = "phase"]
    pub phase: FloatParam,
    /// Soft saturation 0..1 (the sub stays mono).
    #[id = "drive"]
    pub drive: FloatParam,
}

/// Noise layer (Track 7, G8). Ids `noise_{type,level,tone}`.
#[derive(Params)]
pub struct NoiseParams {
    #[id = "type"]
    pub ty: EnumParam<NoiseType>,
    /// dB, -60 (off) .. 0 (0 dB = the RMS of a full-level oscillator).
    #[id = "level"]
    pub level: FloatParam,
    /// 0 dark (low-pass) .. 0.5 flat .. 1 bright (high-pass).
    #[id = "tone"]
    pub tone: FloatParam,
}

/// Mod wheel (CC1) amounts, bipolar. Ids: `wheel_{index,shift,cutoff,mod}`.
#[derive(Params)]
pub struct WheelParams {
    #[id = "index"]
    pub index: FloatParam,
    #[id = "shift"]
    pub shift: FloatParam,
    #[id = "cutoff"]
    pub cutoff: FloatParam,
    #[id = "mod"]
    pub mod_amt: FloatParam,
}

/// Velocity amounts, bipolar. Ids: `vel_{vol_a,vol_b,fenv,att,shift,mod}`.
#[derive(Params)]
pub struct VelocityParams {
    #[id = "vol_a"]
    pub vol_a: FloatParam,
    #[id = "vol_b"]
    pub vol_b: FloatParam,
    #[id = "fenv"]
    pub fenv: FloatParam,
    #[id = "att"]
    pub att: FloatParam,
    #[id = "shift"]
    pub shift: FloatParam,
    #[id = "mod"]
    pub mod_amt: FloatParam,
}

// ---- FX section (SPEC v2 Track 4) ----------------------------------------------------------
// Default chain order Distortion -> Multiband -> EQ -> Phaser -> Chorus -> Flanger -> Dimension -> Delay ->
// Reverb, reorderable (`fx_order`, Track 7), then the Limiter, always last.
// Every module is off by default except the always-on limiter (no `on` param, `lim_gain` only).
// Plain values are the units the DSP modules take (`dsp::fx::*Params`), so the preset slots point
// straight at them: ms for chorus / delay / pre-delay times, seconds for the reverb decay, 0..2
// for multiband depth and 0..1 for its time and every mix (shown as %).

fn hz(name: String, default: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Skewed { min, max, factor: FloatRange::skew_factor(-2.0) })
        .with_value_to_string(formatters::v2s_f32_hz_then_khz(1))
        .with_string_to_value(formatters::s2v_f32_hz_then_khz())
}

fn decibels(name: String, default: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min, max })
        .with_unit(" dB")
        .with_value_to_string(formatters::v2s_f32_rounded(1))
}

fn percent(name: String, default: f32, max: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min: 0.0, max })
        .with_unit("%")
        .with_value_to_string(formatters::v2s_f32_percentage(0))
        .with_string_to_value(formatters::s2v_f32_percentage())
}

fn skewed(name: String, default: f32, min: f32, max: f32, unit: &'static str) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Skewed { min, max, factor: FloatRange::skew_factor(-1.5) })
        .with_unit(unit)
        .with_value_to_string(formatters::v2s_f32_rounded(2))
}

fn linear(name: String, default: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min, max }).with_value_to_string(formatters::v2s_f32_rounded(2))
}

/// FX 4.1. Ids `dist_{on,type,drive,bias,filter_mode,filter_type,filter_freq,filter_q,out,autogain,mix}`.
#[derive(Params)]
pub struct DistParams {
    #[id = "on"]
    pub on: BoolParam,
    #[id = "type"]
    pub ty: EnumParam<DistType>,
    /// dB, -12..+36.
    #[id = "drive"]
    pub drive: FloatParam,
    #[id = "bias"]
    pub bias: FloatParam,
    #[id = "filter_mode"]
    pub filter_mode: EnumParam<DistFilterMode>,
    #[id = "filter_type"]
    pub filter_type: EnumParam<DistFilterType>,
    #[id = "filter_freq"]
    pub filter_freq: FloatParam,
    #[id = "filter_q"]
    pub filter_q: FloatParam,
    /// dB, -24..+12, wet path.
    #[id = "out"]
    pub out: FloatParam,
    #[id = "autogain"]
    pub autogain: BoolParam,
    #[id = "mix"]
    pub mix: FloatParam,
}

/// FX 4.2, three-band upward/downward compressor. Ids `mb_{on,depth,time,xover_lo,xover_hi,{up,down,gain}_{lo,mid,hi},in,out,mix}`.
#[derive(Params)]
pub struct MultibandFxParams {
    #[id = "on"]
    pub on: BoolParam,
    /// 0..2 (0..200 %).
    #[id = "depth"]
    pub depth: FloatParam,
    /// 0..1: attack 1..100 ms, release 20..500 ms.
    #[id = "time"]
    pub time: FloatParam,
    #[id = "xover_lo"]
    pub xover_lo: FloatParam,
    #[id = "xover_hi"]
    pub xover_hi: FloatParam,
    #[id = "up_lo"]
    pub up_lo: FloatParam,
    #[id = "up_mid"]
    pub up_mid: FloatParam,
    #[id = "up_hi"]
    pub up_hi: FloatParam,
    #[id = "down_lo"]
    pub down_lo: FloatParam,
    #[id = "down_mid"]
    pub down_mid: FloatParam,
    #[id = "down_hi"]
    pub down_hi: FloatParam,
    #[id = "gain_lo"]
    pub gain_lo: FloatParam,
    #[id = "gain_mid"]
    pub gain_mid: FloatParam,
    #[id = "gain_hi"]
    pub gain_hi: FloatParam,
    #[id = "in"]
    pub input: FloatParam,
    #[id = "out"]
    pub out: FloatParam,
    #[id = "mix"]
    pub mix: FloatParam,
}

/// FX 4.3 (no mix). Ids `eq_{on,low_freq,low_gain,peak_freq,peak_q,peak_gain,high_freq,high_gain,
/// cut_on,cut_freq}`.
#[derive(Params)]
pub struct EqFxParams {
    #[id = "on"]
    pub on: BoolParam,
    #[id = "low_freq"]
    pub low_freq: FloatParam,
    #[id = "low_gain"]
    pub low_gain: FloatParam,
    #[id = "peak_freq"]
    pub peak_freq: FloatParam,
    #[id = "peak_q"]
    pub peak_q: FloatParam,
    #[id = "peak_gain"]
    pub peak_gain: FloatParam,
    #[id = "high_freq"]
    pub high_freq: FloatParam,
    #[id = "high_gain"]
    pub high_gain: FloatParam,
    #[id = "cut_on"]
    pub cut_on: BoolParam,
    #[id = "cut_freq"]
    pub cut_freq: FloatParam,
}

/// FX 4.4. Ids `cho_{on,voices,rate,sync,div,depth,delay,feedback,lpf,wet_lowcut,mix}`.
#[derive(Params)]
pub struct ChorusFxParams {
    #[id = "on"]
    pub on: BoolParam,
    #[id = "voices"]
    pub voices: IntParam,
    /// Free rate, Hz.
    #[id = "rate"]
    pub rate: FloatParam,
    #[id = "sync"]
    pub sync: BoolParam,
    /// LFO period when synced (the Mod A/B divisions).
    #[id = "div"]
    pub div: EnumParam<Division>,
    #[id = "depth"]
    pub depth: FloatParam,
    /// Centre delay, ms.
    #[id = "delay"]
    pub delay: FloatParam,
    #[id = "feedback"]
    pub feedback: FloatParam,
    #[id = "lpf"]
    pub lpf: FloatParam,
    #[id = "wet_lowcut"]
    pub wet_lowcut: FloatParam,
    #[id = "mix"]
    pub mix: FloatParam,
}

/// FX 4.5. Ids `dim_{on,size,wet_lowcut,mix}`.
#[derive(Params)]
pub struct DimensionFxParams {
    #[id = "on"]
    pub on: BoolParam,
    #[id = "size"]
    pub size: FloatParam,
    #[id = "wet_lowcut"]
    pub wet_lowcut: FloatParam,
    #[id = "mix"]
    pub mix: FloatParam,
}

/// FX 4.6. Ids `dly_{on,sync,time,div,feel,feedback,mode,hp,lp,mix}`.
#[derive(Params)]
pub struct DelayFxParams {
    #[id = "on"]
    pub on: BoolParam,
    #[id = "sync"]
    pub sync: BoolParam,
    /// Free time, ms.
    #[id = "time"]
    pub time: FloatParam,
    /// Synced note value and its feel (straight / dotted / triplet).
    #[id = "div"]
    pub div: EnumParam<DelayDiv>,
    #[id = "feel"]
    pub feel: EnumParam<DelayFeel>,
    #[id = "feedback"]
    pub feedback: FloatParam,
    #[id = "mode"]
    pub mode: EnumParam<DelayMode>,
    #[id = "hp"]
    pub hp: FloatParam,
    #[id = "lp"]
    pub lp: FloatParam,
    #[id = "mix"]
    pub mix: FloatParam,
}

/// FX 4.7. Ids `rev_{on,type,size,decay,damp,predelay,lowcut,hicut,width,mix}`.
#[derive(Params)]
pub struct ReverbFxParams {
    #[id = "on"]
    pub on: BoolParam,
    #[id = "type"]
    pub ty: EnumParam<ReverbType>,
    #[id = "size"]
    pub size: FloatParam,
    /// RT60, seconds.
    #[id = "decay"]
    pub decay: FloatParam,
    #[id = "damp"]
    pub damp: FloatParam,
    /// ms.
    #[id = "predelay"]
    pub predelay: FloatParam,
    #[id = "lowcut"]
    pub lowcut: FloatParam,
    #[id = "hicut"]
    pub hicut: FloatParam,
    #[id = "width"]
    pub width: FloatParam,
    #[id = "mix"]
    pub mix: FloatParam,
}

/// The four macro knobs (SPEC v2 Track 5), matrix sources and host-automatable. Ids `macro1..macro4`.
#[derive(Params)]
pub struct MacroParams {
    #[id = "macro1"]
    pub m1: FloatParam,
    #[id = "macro2"]
    pub m2: FloatParam,
    #[id = "macro3"]
    pub m3: FloatParam,
    #[id = "macro4"]
    pub m4: FloatParam,
}

impl MacroParams {
    pub fn values(&self) -> [f32; 4] {
        [self.m1.value(), self.m2.value(), self.m3.value(), self.m4.value()]
    }
}

impl Default for MacroParams {
    fn default() -> Self {
        let m = |k: u32| unit(format!("Macro {k}"), 0.0);
        Self { m1: m(1), m2: m(2), m3: m(3), m4: m(4) }
    }
}

/// FX 4.8: always on, ceiling fixed at -0.3 dBFS. Id `lim_gain`.
#[derive(Params)]
pub struct LimiterFxParams {
    /// Input gain, dB, -24..+12.
    #[id = "gain"]
    pub gain: FloatParam,
}

impl Default for DistParams {
    fn default() -> Self {
        let n = |s: &str| format!("Dist {s}");
        Self {
            on: BoolParam::new(n("On"), false),
            ty: EnumParam::new(n("Type"), DistType::Tube),
            drive: decibels(n("Drive"), 0.0, -12.0, 36.0),
            bias: bipolar(n("Bias"), 0.0),
            filter_mode: EnumParam::new(n("Filter Mode"), DistFilterMode::Off),
            filter_type: EnumParam::new(n("Filter Type"), DistFilterType::Lp),
            filter_freq: hz(n("Filter Freq"), 1000.0, 20.0, 20000.0),
            filter_q: skewed(n("Filter Q"), 0.707, 0.3, 8.0, ""),
            out: decibels(n("Out"), 0.0, -24.0, 12.0),
            autogain: BoolParam::new(n("Autogain"), true),
            mix: percent(n("Mix"), 1.0, 1.0),
        }
    }
}

impl Default for MultibandFxParams {
    fn default() -> Self {
        let n = |s: &str| format!("MB {s}");
        Self {
            on: BoolParam::new(n("On"), false),
            depth: percent(n("Depth"), 1.0, 2.0),
            time: percent(n("Time"), 0.5, 1.0),
            xover_lo: hz(n("Low Xover"), 120.0, 60.0, 400.0),
            xover_hi: hz(n("High Xover"), 2500.0, 1000.0, 8000.0),
            up_lo: decibels(n("Low Up Thresh"), -24.0, -60.0, 0.0),
            up_mid: decibels(n("Mid Up Thresh"), -24.0, -60.0, 0.0),
            up_hi: decibels(n("High Up Thresh"), -24.0, -60.0, 0.0),
            down_lo: decibels(n("Low Down Thresh"), -12.0, -60.0, 0.0),
            down_mid: decibels(n("Mid Down Thresh"), -12.0, -60.0, 0.0),
            down_hi: decibels(n("High Down Thresh"), -12.0, -60.0, 0.0),
            gain_lo: decibels(n("Low Gain"), 0.0, -12.0, 12.0),
            gain_mid: decibels(n("Mid Gain"), 0.0, -12.0, 12.0),
            gain_hi: decibels(n("High Gain"), 0.0, -12.0, 12.0),
            input: decibels(n("In"), 0.0, -24.0, 24.0),
            out: decibels(n("Out"), 1.5, -24.0, 24.0),
            mix: percent(n("Mix"), 1.0, 1.0),
        }
    }
}

impl Default for EqFxParams {
    fn default() -> Self {
        let n = |s: &str| format!("EQ {s}");
        Self {
            on: BoolParam::new(n("On"), false),
            low_freq: hz(n("Low Freq"), 100.0, 30.0, 500.0),
            low_gain: decibels(n("Low Gain"), 0.0, -15.0, 15.0),
            peak_freq: hz(n("Peak Freq"), 1000.0, 20.0, 20000.0),
            peak_q: skewed(n("Peak Q"), 1.0, 0.2, 10.0, ""),
            peak_gain: decibels(n("Peak Gain"), 0.0, -15.0, 15.0),
            high_freq: hz(n("High Freq"), 8000.0, 1000.0, 20000.0),
            high_gain: decibels(n("High Gain"), 0.0, -15.0, 15.0),
            cut_on: BoolParam::new(n("Low Cut On"), false),
            cut_freq: hz(n("Low Cut Freq"), 30.0, 20.0, 500.0),
        }
    }
}

impl Default for ChorusFxParams {
    fn default() -> Self {
        let n = |s: &str| format!("Chorus {s}");
        Self {
            on: BoolParam::new(n("On"), false),
            voices: IntParam::new(n("Voices"), 4, IntRange::Linear { min: 2, max: 8 }),
            rate: skewed(n("Rate"), 0.5, 0.1, 10.0, " Hz"),
            sync: BoolParam::new(n("Sync"), false),
            div: EnumParam::new(n("Division"), Division::Bar1),
            depth: unit(n("Depth"), 0.5),
            delay: skewed(n("Delay"), 7.0, 1.0, 20.0, " ms"),
            feedback: linear(n("Feedback"), 0.0, -0.95, 0.95)
                .with_unit("%")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            lpf: hz(n("LPF"), 20000.0, 200.0, 20000.0),
            wet_lowcut: hz(n("Wet Low Cut"), 120.0, 20.0, 500.0),
            mix: percent(n("Mix"), 0.5, 1.0),
        }
    }
}

impl Default for DimensionFxParams {
    fn default() -> Self {
        let n = |s: &str| format!("Dimension {s}");
        Self {
            on: BoolParam::new(n("On"), false),
            size: unit(n("Size"), 0.5),
            wet_lowcut: hz(n("Wet Low Cut"), 120.0, 20.0, 500.0),
            mix: percent(n("Mix"), 0.5, 1.0),
        }
    }
}

impl Default for DelayFxParams {
    fn default() -> Self {
        let n = |s: &str| format!("Delay {s}");
        Self {
            on: BoolParam::new(n("On"), false),
            sync: BoolParam::new(n("Sync"), true),
            time: skewed(n("Time"), 250.0, 1.0, 2000.0, " ms").with_value_to_string(formatters::v2s_f32_rounded(1)),
            div: EnumParam::new(n("Division"), DelayDiv::D1_8),
            feel: EnumParam::new(n("Feel"), DelayFeel::Dotted),
            feedback: linear(n("Feedback"), 0.35, 0.0, 0.95),
            mode: EnumParam::new(n("Mode"), DelayMode::Stereo),
            hp: hz(n("HP"), 120.0, 20.0, 2000.0),
            lp: hz(n("LP"), 8000.0, 200.0, 20000.0),
            mix: percent(n("Mix"), 0.25, 1.0),
        }
    }
}

impl Default for ReverbFxParams {
    fn default() -> Self {
        let n = |s: &str| format!("Reverb {s}");
        Self {
            on: BoolParam::new(n("On"), false),
            ty: EnumParam::new(n("Type"), ReverbType::Plate),
            size: unit(n("Size"), 0.5),
            decay: skewed(n("Decay"), 1.5, 0.1, 20.0, " s"),
            damp: unit(n("Damp"), 0.3),
            predelay: linear(n("Pre-Delay"), 0.0, 0.0, 300.0).with_unit(" ms").with_value_to_string(formatters::v2s_f32_rounded(0)),
            lowcut: hz(n("Low Cut"), 200.0, 20.0, 1000.0),
            hicut: hz(n("High Cut"), 12000.0, 1000.0, 20000.0),
            width: unit(n("Width"), 1.0),
            mix: percent(n("Mix"), 0.15, 1.0),
        }
    }
}

impl Default for LimiterFxParams {
    fn default() -> Self {
        Self { gain: decibels("Limiter Gain".into(), 0.0, -24.0, 12.0) }
    }
}

// ---- SPEC v2 Track 7 FX (G9): phaser and flanger, off by default ---------------------------

/// Right-side LFO lead in turns (0..0.5), shown in degrees.
fn spread_turns(name: String) -> FloatParam {
    FloatParam::new(name, 0.25, FloatRange::Linear { min: 0.0, max: 0.5 })
        .with_value_to_string(Arc::new(|v| format!("{:.0}°", v * 360.0)))
        .with_string_to_value(Arc::new(|s| s.trim().trim_end_matches('°').trim().parse::<f32>().ok().map(|d| d / 360.0)))
}

fn feedback_pct(name: String, default: f32) -> FloatParam {
    linear(name, default, -0.95, 0.95)
        .with_unit("%")
        .with_value_to_string(formatters::v2s_f32_percentage(0))
        .with_string_to_value(formatters::s2v_f32_percentage())
}

/// Ids `phs_{on,stages,rate,sync,div,depth,feedback,centre,spread,mix}` (`dsp::fx::phaser`).
#[derive(Params)]
pub struct PhaserFxParams {
    #[id = "on"]
    pub on: BoolParam,
    /// All-pass stages per side, 4..12 (odd counts round down).
    #[id = "stages"]
    pub stages: IntParam,
    /// Free rate, Hz.
    #[id = "rate"]
    pub rate: FloatParam,
    #[id = "sync"]
    pub sync: BoolParam,
    /// LFO period when synced.
    #[id = "div"]
    pub div: EnumParam<Division>,
    #[id = "depth"]
    pub depth: FloatParam,
    #[id = "feedback"]
    pub feedback: FloatParam,
    /// Centre break frequency, Hz.
    #[id = "centre"]
    pub centre: FloatParam,
    #[id = "spread"]
    pub spread: FloatParam,
    #[id = "mix"]
    pub mix: FloatParam,
}

impl Default for PhaserFxParams {
    fn default() -> Self {
        let n = |s: &str| format!("Phaser {s}");
        Self {
            on: BoolParam::new(n("On"), false),
            stages: IntParam::new(n("Stages"), 8, IntRange::Linear { min: 4, max: 12 }),
            rate: skewed(n("Rate"), 0.3, 0.02, 20.0, " Hz"),
            sync: BoolParam::new(n("Sync"), false),
            div: EnumParam::new(n("Division"), Division::Bar1),
            depth: unit(n("Depth"), 0.5),
            feedback: feedback_pct(n("Feedback"), 0.3),
            centre: hz(n("Centre"), 800.0, 20.0, 10000.0),
            spread: spread_turns(n("Spread")),
            mix: percent(n("Mix"), 0.5, 1.0),
        }
    }
}

/// Ids `flg_{on,rate,sync,div,depth,delay,feedback,spread,mix}` (`dsp::fx::flanger`).
#[derive(Params)]
pub struct FlangerFxParams {
    #[id = "on"]
    pub on: BoolParam,
    #[id = "rate"]
    pub rate: FloatParam,
    #[id = "sync"]
    pub sync: BoolParam,
    #[id = "div"]
    pub div: EnumParam<Division>,
    /// Fraction of the delay swept down, 0..1.
    #[id = "depth"]
    pub depth: FloatParam,
    /// Top-of-sweep delay, ms.
    #[id = "delay"]
    pub delay: FloatParam,
    #[id = "feedback"]
    pub feedback: FloatParam,
    #[id = "spread"]
    pub spread: FloatParam,
    #[id = "mix"]
    pub mix: FloatParam,
}

impl Default for FlangerFxParams {
    fn default() -> Self {
        let n = |s: &str| format!("Flanger {s}");
        Self {
            on: BoolParam::new(n("On"), false),
            rate: skewed(n("Rate"), 0.2, 0.02, 20.0, " Hz"),
            sync: BoolParam::new(n("Sync"), false),
            div: EnumParam::new(n("Division"), Division::Bar1),
            depth: unit(n("Depth"), 0.8),
            delay: skewed(n("Delay"), 4.0, 0.1, 10.0, " ms"),
            feedback: feedback_pct(n("Feedback"), 0.5),
            spread: spread_turns(n("Spread")),
            mix: percent(n("Mix"), 0.5, 1.0),
        }
    }
}

// ---- SPEC v2 Track 7 modulation (G1, G2, G6, G7) -------------------------------------------
// Appended block. Every default is neutral: Mod C / Mod D and Mod Env are matrix-only sources
// (no fixed routes), the extensions default to the v1 behaviour, the pitch offsets to 0.

/// LFO extensions shared by Mod A..D. Ids `mod{a,b,c,d}_{phase,fade,smooth,trig}`.
#[derive(Params)]
pub struct LfoExtParams {
    /// Start phase, 0..360 degrees.
    #[id = "phase"]
    pub phase: FloatParam,
    /// Fade-in after note-on, 0..4 s (linear, 0 = off).
    #[id = "fade"]
    pub fade: FloatParam,
    /// Output smoothing, 0..1 (one-pole, time constant up to a quarter cycle).
    #[id = "smooth"]
    pub smooth: FloatParam,
    #[id = "trig"]
    pub trig: EnumParam<Trigger>,
}

impl LfoExtParams {
    fn new(letter: &str) -> Self {
        let n = |s: &str| format!("Mod {letter} {s}");
        Self {
            phase: FloatParam::new(n("Phase"), 0.0, FloatRange::Linear { min: 0.0, max: 360.0 })
                .with_unit("°")
                .with_value_to_string(formatters::v2s_f32_rounded(0)),
            fade: FloatParam::new(n("Fade"), 0.0, FloatRange::Skewed { min: 0.0, max: 4.0, factor: FloatRange::skew_factor(-1.0) })
                .with_unit(" s")
                .with_value_to_string(formatters::v2s_f32_rounded(2)),
            smooth: unit(n("Smooth"), 0.0),
            trig: EnumParam::new(n("Trigger"), Trigger::Retrigger),
        }
    }
}

/// Mod C / Mod D: the Mod A / B LFO set without Target (they have no fixed routes; matrix
/// sources only). Ids `mod{c,d}_{curve,rate,sync,div,oneshot}`.
#[derive(Params)]
pub struct ModCdParams {
    #[id = "curve"]
    pub curve: EnumParam<Curve>,
    #[id = "rate"]
    pub rate: FloatParam,
    #[id = "sync"]
    pub sync: BoolParam,
    #[id = "div"]
    pub div: EnumParam<Division>,
    #[id = "oneshot"]
    pub oneshot: BoolParam,
}

impl ModCdParams {
    fn new(letter: &str) -> Self {
        let n = |s: &str| format!("Mod {letter} {s}");
        Self {
            curve: EnumParam::new(n("Curve"), Curve::Sine),
            rate: lfo_rate(n("Rate")),
            sync: BoolParam::new(n("Sync"), false),
            div: EnumParam::new(n("Division"), Division::Eighth),
            oneshot: BoolParam::new(n("One-Shot"), false),
        }
    }
}

/// Mod envelope (G7): ADSR with curve, matrix source "Mod Env". Ids `menv_{att,dec,sus,rel,curve}`.
#[derive(Params)]
pub struct ModEnvParams {
    #[id = "att"]
    pub att: FloatParam,
    #[id = "dec"]
    pub dec: FloatParam,
    #[id = "sus"]
    pub sus: FloatParam,
    #[id = "rel"]
    pub rel: FloatParam,
    /// -1 log .. 0 exponential .. +1 linear.
    #[id = "curve"]
    pub curve: FloatParam,
}

impl Default for ModEnvParams {
    fn default() -> Self {
        Self {
            att: time("Mod Env Attack".into(), 0.001),
            dec: time("Mod Env Decay".into(), 0.25),
            sus: unit("Mod Env Sustain".into(), 0.0),
            rel: time("Mod Env Release".into(), 0.1),
            curve: bipolar("Mod Env Curve".into(), 0.0),
        }
    }
}

/// Continuous pitch offsets (G2), -48..+48 st, voice-scope matrix targets. `pitch_all` moves
/// Osc A, Osc B and the sub. Ids `osca_pitch`, `oscb_pitch`, `pitch_all`.
#[derive(Params)]
pub struct PitchParams {
    #[id = "osca_pitch"]
    pub osca: FloatParam,
    #[id = "oscb_pitch"]
    pub oscb: FloatParam,
    #[id = "pitch_all"]
    pub all: FloatParam,
}

impl Default for PitchParams {
    fn default() -> Self {
        Self {
            // One decimal ("+0.0 st"): the editor's Pitch knob has a centre detent at 0 (addendum 4).
            osca: semitones("Osc A Pitch".into(), 0.0, 48.0).with_value_to_string(Arc::new(|v| format!("{v:+.1} st"))).with_unit(""),
            oscb: semitones("Osc B Pitch".into(), 0.0, 48.0).with_value_to_string(Arc::new(|v| format!("{v:+.1} st"))).with_unit(""),
            all: semitones("Pitch All".into(), 0.0, 48.0).with_value_to_string(Arc::new(|v| format!("{v:+.1} st"))).with_unit(""),
        }
    }
}

#[derive(Params)]
pub struct GtsParams {
    #[nested(id_prefix = "osca", group = "Osc A")]
    pub osca: OscParams,
    #[nested(id_prefix = "oscb", group = "Osc B")]
    pub oscb: OscParams,

    #[nested(id_prefix = "moda", group = "Mod A")]
    pub moda: ModParams,
    #[nested(id_prefix = "moda", group = "Mod A")]
    pub moda_amt: ModAAmounts,
    #[nested(id_prefix = "modb", group = "Mod B")]
    pub modb: ModParams,
    #[nested(id_prefix = "modb", group = "Mod B")]
    pub modb_amt: ModBAmounts,

    #[nested(id_prefix = "shaper", group = "Shaper")]
    pub shaper: ShaperParams,

    #[nested(id_prefix = "filta", group = "Filter A")]
    pub filta: FilterParams,
    #[nested(id_prefix = "filtb", group = "Filter B")]
    pub filtb: FilterParams,
    #[nested(id_prefix = "fenv", group = "Filter Env")]
    pub fenv: FilterEnvParams,
    #[nested(id_prefix = "sub", group = "Sub")]
    pub sub: SubParams,
    /// Track 7 (G8).
    #[nested(id_prefix = "noise", group = "Noise")]
    pub noise: NoiseParams,

    #[id = "routing"]
    pub routing: EnumParam<Routing>,
    /// Polyphony 1..8.
    #[id = "poly"]
    pub poly: IntParam,
    #[id = "legato"]
    pub legato: BoolParam,
    /// Portamento time in seconds, 0..2.
    #[id = "porta"]
    pub porta: FloatParam,
    /// Pitch bend range in semitones, 0..24.
    #[id = "bend_range"]
    pub bend_range: IntParam,

    #[nested(id_prefix = "wheel", group = "Wheel")]
    pub wheel: WheelParams,
    #[nested(id_prefix = "vel", group = "Velocity")]
    pub vel: VelocityParams,

    /// Output volume in dB, -24..+6.
    #[id = "volume"]
    pub volume: FloatParam,
    /// Stereo spread, 0..1 (A left / B right, 0 = mono).
    #[id = "spread"]
    pub spread: FloatParam,
    /// Mono crossover in Hz, 20..300: the output is mono below it (SPEC v2 1.5); 20 = off (bypass).
    #[id = "mono_xover"]
    pub mono_xover: FloatParam,

    #[nested(id_prefix = "dist", group = "Distortion")]
    pub dist: DistParams,
    #[nested(id_prefix = "mb", group = "Multiband")]
    pub mb: MultibandFxParams,
    #[nested(id_prefix = "eq", group = "EQ")]
    pub eq: EqFxParams,
    #[nested(id_prefix = "cho", group = "Chorus")]
    pub cho: ChorusFxParams,
    #[nested(id_prefix = "dim", group = "Dimension")]
    pub dim: DimensionFxParams,
    #[nested(id_prefix = "dly", group = "Delay")]
    pub dly: DelayFxParams,
    #[nested(id_prefix = "rev", group = "Reverb")]
    pub rev: ReverbFxParams,
    #[nested(id_prefix = "lim", group = "Limiter")]
    pub lim: LimiterFxParams,

    /// Macro 1..4 (SPEC v2 Track 5). Ids `macro1..macro4`.
    #[nested(group = "Macros")]
    pub macros: MacroParams,

    /// RR-BUS CV-in slots: target and bipolar amount. Ids `cv{1,2,3}_{target,amount}`.
    #[id = "cv1_target"]
    pub cv1_target: EnumParam<CvTarget>,
    #[id = "cv1_amount"]
    pub cv1_amount: FloatParam,
    #[id = "cv2_target"]
    pub cv2_target: EnumParam<CvTarget>,
    #[id = "cv2_amount"]
    pub cv2_amount: FloatParam,
    #[id = "cv3_target"]
    pub cv3_target: EnumParam<CvTarget>,
    #[id = "cv3_amount"]
    pub cv3_amount: FloatParam,

    // ---- SPEC v2 Track 7 modulation (appended) ----
    #[nested(id_prefix = "moda", group = "Mod A")]
    pub moda_ext: LfoExtParams,
    #[nested(id_prefix = "modb", group = "Mod B")]
    pub modb_ext: LfoExtParams,
    #[nested(id_prefix = "modc", group = "Mod C")]
    pub modc: ModCdParams,
    #[nested(id_prefix = "modc", group = "Mod C")]
    pub modc_ext: LfoExtParams,
    #[nested(id_prefix = "modd", group = "Mod D")]
    pub modd: ModCdParams,
    #[nested(id_prefix = "modd", group = "Mod D")]
    pub modd_ext: LfoExtParams,
    #[nested(id_prefix = "menv", group = "Mod Env")]
    pub menv: ModEnvParams,
    #[nested(group = "Pitch")]
    pub pitch: PitchParams,

    // ---- SPEC v2 Track 7 FX (appended) ----
    #[nested(id_prefix = "phs", group = "Phaser")]
    pub phs: PhaserFxParams,
    #[nested(id_prefix = "flg", group = "Flanger")]
    pub flg: FlangerFxParams,
    /// FX chain order (SPEC v2 Track 7, G9): project and preset state, read lock-free by the
    /// audio thread (see `crate::dsp::fx::FxOrderShared`).
    #[persist = "fx_order"]
    pub fx_order: crate::dsp::fx::FxOrderShared,

    /// Persisted CV-in sources, `"<label> · <name>"`, empty = none (GUI thread only).
    #[persist = "cv_sources"]
    pub cv_sources: RwLock<[String; 3]>,
    /// This instance's bus label (GUI thread only).
    #[persist = "instance_label"]
    pub instance_label: RwLock<String>,
    /// Shaper oversampling 1 / 2 / 4 (SPEC v2 1.2): a host-side quality setting, never a preset
    /// value. Written by the editor, applied by `process` at a block start.
    #[persist = "oversampling"]
    pub oversampling: AtomicUsize,
    /// Editor setting "Always show values" (MOTION.md 1): knobs print their value instead of
    /// their label. Host state, not a param.
    #[persist = "show_values"]
    pub show_values: AtomicBool,
    /// Editor setting "Reduced motion" (MOTION.md 4): no sway, parallax, page or popover motion,
    /// instant chrome transitions. Host state, not a param; off by default (no OS query).
    #[persist = "reduced_motion"]
    pub reduced_motion: AtomicBool,
    /// Which preset this instance has loaded, by library key (`"Factory/Bass/Growl Hound"`,
    /// `"Init"`; empty = none yet). Host state (SPEC v2 2.6), written by the editor, never a param.
    #[persist = "preset_key"]
    pub preset_key: RwLock<String>,
    /// The loaded preset was edited afterwards (the header dot). Host state like `preset_key`.
    #[persist = "preset_dirty"]
    pub preset_dirty: AtomicBool,
    /// Modulation matrix slots and macro labels (SPEC v2 Track 5): project state, edited by the
    /// GUI, read lock-free by the audio thread (see `crate::matrix`).
    #[persist = "matrix"]
    pub matrix: crate::matrix::MatrixShared,
    /// Custom LFO shapes of Mod A..D (SPEC v2 Track 7, G1): project and preset state, read
    /// lock-free by the audio thread (see `crate::dsp::shape`).
    #[persist = "lfo_shapes"]
    pub lfo_shapes: crate::dsp::shape::ShapesShared,
}

impl GtsParams {
    /// (target, amount) of the three CV-in slots.
    pub fn cv_slots(&self) -> [(CvTarget, f32); 3] {
        [
            (self.cv1_target.value(), self.cv1_amount.value()),
            (self.cv2_target.value(), self.cv2_amount.value()),
            (self.cv3_target.value(), self.cv3_amount.value()),
        ]
    }
}

impl Default for GtsParams {
    fn default() -> Self {
        Self {
            // SPEC v2 1.12: the init patch is a clean bass. The v1 riddim patch lives on as the
            // `dsp::Patch::riddim_v1` fixture.
            osca: OscParams::new("A", true, Table::SquareMorph, 40.0),
            oscb: OscParams::new("B", false, Table::SawStack, 64.0),

            moda: ModParams::new("A", Curve::Decay, true, false),
            moda_amt: ModAAmounts {
                pitch: bipolar("Mod A Pitch".into(), 0.0),
                index: bipolar("Mod A Index".into(), 0.0),
                shift: bipolar("Mod A Shift".into(), 0.0),
            },
            modb: ModParams::new("B", Curve::Sine, false, false),
            modb_amt: ModBAmounts {
                motion: bipolar("Mod B Motion".into(), 0.0),
                vol: bipolar("Mod B Vol".into(), 0.0),
                cutoff: bipolar("Mod B Cutoff".into(), 0.0),
                moda: bipolar("Mod B Mod A".into(), 0.0),
            },

            shaper: ShaperParams {
                on: BoolParam::new("Shaper On", false),
                mode: EnumParam::new("Shaper Mode", ShaperMode::Saturate),
                amt: unit("Shaper Amt".into(), 0.3),
            },

            filta: FilterParams::new("A", FilterType::Lp12, 2000.0, 0.15),
            filtb: FilterParams::new("B", FilterType::Lp12, 20000.0, 0.0),
            fenv: FilterEnvParams {
                att: time("Filter Env Attack".into(), 0.005),
                dec: time("Filter Env Decay".into(), 0.25),
                sus: unit("Filter Env Sustain".into(), 0.2),
                rel: time("Filter Env Release".into(), 0.1),
                amt: unit("Filter Env Amt".into(), 0.4),
                inv: BoolParam::new("Filter Env Invert", false),
                curve: bipolar("Filter Env Curve".into(), 0.0),
            },
            sub: SubParams {
                on: BoolParam::new("Sub On", true),
                level: FloatParam::new("Sub Level", -6.0, FloatRange::Linear { min: -60.0, max: 0.0 })
                    .with_unit(" dB")
                    .with_value_to_string(formatters::v2s_f32_rounded(1)),
                oct: IntParam::new("Sub Octave", -1, IntRange::Linear { min: -2, max: 0 }),
                shape: EnumParam::new("Sub Shape", SubShape::Sine),
                phase: FloatParam::new("Sub Phase", 0.0, FloatRange::Linear { min: 0.0, max: 360.0 })
                    .with_unit("°")
                    .with_value_to_string(formatters::v2s_f32_rounded(0)),
                drive: unit("Sub Drive".into(), 0.0),
            },
            noise: NoiseParams {
                ty: EnumParam::new("Noise Type", NoiseType::White),
                level: FloatParam::new("Noise Level", -60.0, FloatRange::Linear { min: -60.0, max: 0.0 })
                    .with_value_to_string(Arc::new(|v| if v <= -59.95 { "Off".into() } else { format!("{v:.1} dB") }))
                    .with_string_to_value(Arc::new(|s| {
                        let s = s.trim().trim_end_matches("dB").trim();
                        if s.eq_ignore_ascii_case("off") { Some(-60.0) } else { s.parse().ok() }
                    })),
                tone: unit("Noise Tone".into(), 0.5),
            },

            routing: EnumParam::new("Routing", Routing::Parallel),
            poly: IntParam::new("Polyphony", 1, IntRange::Linear { min: 1, max: 8 }),
            legato: BoolParam::new("Legato", true),
            porta: FloatParam::new("Portamento", 0.08, FloatRange::Linear { min: 0.0, max: 2.0 })
                .with_value_to_string(Arc::new(|v| format!("{:.0} ms", v * 1000.0))),
            bend_range: IntParam::new("Bend Range", 12, IntRange::Linear { min: 0, max: 24 })
                .with_unit(" st"),

            wheel: WheelParams {
                index: bipolar("Wheel Index".into(), 0.0),
                shift: bipolar("Wheel Shift".into(), 0.0),
                cutoff: bipolar("Wheel Cutoff".into(), 0.0),
                mod_amt: bipolar("Wheel Mod".into(), 0.0),
            },
            vel: VelocityParams {
                vol_a: bipolar("Vel Vol A".into(), 0.0),
                vol_b: bipolar("Vel Vol B".into(), 0.0),
                fenv: bipolar("Vel Filter Env".into(), 0.0),
                att: bipolar("Vel Attack".into(), 0.0),
                shift: bipolar("Vel Shift".into(), 0.0),
                mod_amt: bipolar("Vel Mod".into(), 0.0),
            },

            // SPEC v2 1.10: the init note (C2, velocity 100) peaks at about -9 dBFS.
            volume: FloatParam::new("Volume", -5.0, FloatRange::Linear { min: -24.0, max: 6.0 })
                .with_smoother(SmoothingStyle::Linear(10.0))
                .with_unit(" dB")
                .with_value_to_string(formatters::v2s_f32_rounded(1)),
            spread: unit("Spread".into(), 0.0),
            mono_xover: FloatParam::new(
                "Mono Crossover",
                20.0,
                FloatRange::Skewed { min: 20.0, max: 300.0, factor: FloatRange::skew_factor(-1.0) },
            )
            // Bottom of the range = crossover off: say so (the unit lives in the formatter).
            .with_value_to_string(Arc::new(|v| if v <= 20.5 { "Off".into() } else { format!("{v:.0} Hz") }))
            .with_string_to_value(Arc::new(|s| {
                let s = s.trim().trim_end_matches("Hz").trim_end_matches("hz").trim();
                if s.eq_ignore_ascii_case("off") { Some(20.0) } else { s.parse().ok() }
            })),

            dist: DistParams::default(),
            mb: MultibandFxParams::default(),
            eq: EqFxParams::default(),
            cho: ChorusFxParams::default(),
            dim: DimensionFxParams::default(),
            dly: DelayFxParams::default(),
            rev: ReverbFxParams::default(),
            lim: LimiterFxParams::default(),
            macros: MacroParams::default(),

            cv1_target: EnumParam::new("CV 1 Target", CvTarget::Off),
            cv1_amount: bipolar("CV 1 Amount".into(), 0.0),
            cv2_target: EnumParam::new("CV 2 Target", CvTarget::Off),
            cv2_amount: bipolar("CV 2 Amount".into(), 0.0),
            cv3_target: EnumParam::new("CV 3 Target", CvTarget::Off),
            cv3_amount: bipolar("CV 3 Amount".into(), 0.0),
            cv_sources: RwLock::new(Default::default()),
            instance_label: RwLock::new(String::new()),
            oversampling: AtomicUsize::new(2),
            show_values: AtomicBool::new(false),
            reduced_motion: AtomicBool::new(false),
            preset_key: RwLock::new(String::new()),
            preset_dirty: AtomicBool::new(false),
            matrix: Default::default(),

            // SPEC v2 Track 7 (appended)
            moda_ext: LfoExtParams::new("A"),
            modb_ext: LfoExtParams::new("B"),
            modc: ModCdParams::new("C"),
            modc_ext: LfoExtParams::new("C"),
            modd: ModCdParams::new("D"),
            modd_ext: LfoExtParams::new("D"),
            menv: ModEnvParams::default(),
            pitch: PitchParams::default(),
            phs: PhaserFxParams::default(),
            flg: FlangerFxParams::default(),
            fx_order: Default::default(),
            lfo_shapes: Default::default(),
        }

    }
}
