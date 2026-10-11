//! The complete GTS parameter contract (SPEC.md "Parameters"). Ids are stable; never rename.
//!
//! Units: all envelope/portamento times are stored in SECONDS (f32). Frequencies in Hz, tuning in
//! semitones/cents, `volume` in dB, everything else normalized as noted per field.
//! Nested structs use `id_prefix`, so `osca.index` has the id `osca_index`, `filta.freq` has
//! `filta_freq`, and so on. Mod A has two nested structs sharing the prefix `moda`.

use nih_plug::prelude::*;

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

use std::sync::atomic::AtomicUsize;
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
        }
    }
}

/// Shared by Mod A and Mod B. Ids: `mod{a,b}_{curve,rate,sync,div,oneshot,target}`.
#[derive(Params)]
pub struct ModParams {
    #[id = "curve"]
    pub curve: EnumParam<Curve>,
    /// Free rate in Hz, 0.03..30 (log).
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
            rate: FloatParam::new(
                n("Rate"),
                4.0,
                FloatRange::Skewed { min: 0.03, max: 30.0, factor: FloatRange::skew_factor(-2.0) },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(2))
            .with_unit(" Hz"),
            sync: BoolParam::new(n("Sync"), sync),
            div: EnumParam::new(n("Division"), Division::Eighth),
            oneshot: BoolParam::new(n("One-Shot"), oneshot),
            target: EnumParam::new(n("Target"), Target::Both),
        }
    }
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
    /// Octaves below Osc A, -2..-1.
    #[id = "oct"]
    pub oct: IntParam,
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
                oct: IntParam::new("Sub Octave", -1, IntRange::Linear { min: -2, max: -1 }),
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
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_rounded(0)),

            cv1_target: EnumParam::new("CV 1 Target", CvTarget::Off),
            cv1_amount: bipolar("CV 1 Amount".into(), 0.0),
            cv2_target: EnumParam::new("CV 2 Target", CvTarget::Off),
            cv2_amount: bipolar("CV 2 Amount".into(), 0.0),
            cv3_target: EnumParam::new("CV 3 Target", CvTarget::Off),
            cv3_amount: bipolar("CV 3 Amount".into(), 0.0),
            cv_sources: RwLock::new(Default::default()),
            instance_label: RwLock::new(String::new()),
            oversampling: AtomicUsize::new(2),
        }

    }
}
