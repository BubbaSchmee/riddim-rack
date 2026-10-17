//! Modulation matrix (SPEC v2 Track 5, `docs/standards/mod-matrix.md`): sources, the target
//! table, the fixed routes, persistence and the lock-free GUI -> audio handoff. The engine half
//! ([`ModRt`]) is driven by `dsp::Engine` every [`TICK`] samples.
//!
//! # Model
//! A [`Matrix`] is up to [`MAX_SLOTS`] [`Slot`]s `{source, target param id, amount -1..1, bipolar,
//! enabled}` plus four macro labels. Amounts act in the target's **normalized** space: every
//! enabled slot on a target adds `amount * source`, the sum is smoothed (5 ms one-pole, updated
//! every [`TICK`] samples) and clamped once to 0..1 on top of the knob. A negative amount inverts;
//! `enabled = false` bypasses. Polarity: every source has a natural range ([`Source::bipolar`]);
//! a unipolar slot maps a bipolar source -1..1 to 0..1, a bipolar slot maps a unipolar source
//! 0..1 to -1..1, otherwise the source passes as is. New slots take the source's natural polarity.
//!
//! Targets ([`params_info`], `target == true`): every continuous FloatParam / IntParam except the
//! macros, the RR-BUS `cv*` params and Polyphony (voice allocation, not a sound value). Enums and
//! bools are never targets. **Voice-scope** targets (`voice == true`: oscillators, unison, sub,
//! filters, filter / amp envelopes, shaper, porta, Mod A / B rates and every fixed-route amount)
//! are evaluated per voice from that voice's own sources on a per-voice `Patch` copy. **Global**
//! targets (the FX section, Volume, Spread, Mono Crossover, Bend Range) are evaluated once on the
//! engine's patch; per-voice sources feeding them come from the newest sounding voice, never a
//! sum of voices.
//!
//! Fixed routes ([`fixed_routes`]): the v1 route params (`moda_index`, `modb_cutoff`, `wheel_*`,
//! `vel_*`, `fenv_amt`, `cv{1,2,3}_amount`, ...) keep their ids, scaling and DSP stage, so saved
//! sets sound identical. A (source, target) pair that has a fixed route is edited through that
//! route param instead of becoming a slot. Their contribution is applied where it always was
//! (after the slot sum), so a target fed by both is clamped by the slot stage and then by the
//! DSP's own range (bit-identity with saved sets wins over one shared clamp).
//!
//! # GUI API
//! [`MatrixEditor`] wraps `params.matrix` ([`MatrixShared`]) and the editor's [`Ctl`]:
//! * [`MatrixEditor::add_or_edit`]`(source, target, amount)` -> [`Edit::Slot`]`(index)` (new slot,
//!   or the existing slot of that pair) or [`Edit::Fixed`]`(route param id)` (the pair is a live
//!   fixed route: its param was set through `Ctl` as one gesture);
//! * [`MatrixEditor::set_amount`], [`MatrixEditor::set_bipolar`], [`MatrixEditor::set_enabled`],
//!   [`MatrixEditor::remove`], [`MatrixEditor::remove_all_for_target`] (slots removed, live fixed
//!   routes set to 0), [`MatrixEditor::set_macro_label`];
//! * queries: [`MatrixEditor::slots_for_target`], [`MatrixEditor::targets_of_source`],
//!   [`MatrixEditor::fixed_routes_for_target`], and [`MatrixShared::get`] for the whole matrix.
//!
//! Slot indices are positions in [`Matrix::slots`]; removing a slot shifts the later ones down.
//!
//! # Threads
//! The canonical [`Matrix`] lives under a `Mutex` that only non-audio threads touch (GUI edits,
//! preset load, host state load via [`PersistentField`]). Every change is encoded into a fixed
//! `[f32; ENC_LEN]` record and published through an [`rr_ui::snap::Snap`] (the mutex keeps it
//! single-writer). The audio thread [`MatrixShared::poll`]s it once per block: wait-free, no
//! allocation, no lock.

use crate::dsp::{Patch, MAX_VOICES};
use crate::editor::Ctl;
use crate::params::{CvTarget, GtsParams};
use nih_plug::params::persist::PersistentField;
use nih_plug::prelude::{Enum, FloatRange, IntRange, ParamPtr, Params};
use rr_ui::snap::Snap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

pub const MAX_SLOTS: usize = 32;
pub const MACROS: usize = 4;
/// Samples between matrix evaluations.
pub const TICK: usize = 32;
/// Smoothing time constant of every matrix offset (seconds).
pub const SMOOTH_S: f32 = 0.005;
/// Encoded slot: source index (-1 = empty), target param index, amount, bipolar, enabled.
pub const ENC_STRIDE: usize = 5;
pub const ENC_LEN: usize = MAX_SLOTS * ENC_STRIDE;
pub const N_SOURCES: usize = 18;

// ---- sources ---------------------------------------------------------------------------------

/// Modulation sources, in record order (append only: the order is the audio-side encoding).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Source {
    ModA,
    ModB,
    FilterEnv,
    AmpEnvA,
    AmpEnvB,
    Velocity,
    /// Note relative to C4, ±60 semitones -> ±1 (follows portamento).
    Keytrack,
    /// A new value per note-on (deterministic xorshift per voice), -1..1.
    Random,
    PolyAftertouch,
    ModWheel,
    ChannelAftertouch,
    Macro1,
    Macro2,
    Macro3,
    Macro4,
    BusCv1,
    BusCv2,
    BusCv3,
}

const SOURCE_NAMES: [&str; N_SOURCES] = [
    "Mod A", "Mod B", "Filter Env", "Amp Env A", "Amp Env B", "Velocity", "Keytrack", "Random", "Poly Aftertouch",
    "Mod Wheel", "Channel Aftertouch", "Macro 1", "Macro 2", "Macro 3", "Macro 4", "Bus CV 1", "Bus CV 2", "Bus CV 3",
];

impl Source {
    pub const ALL: [Source; N_SOURCES] = [
        Source::ModA,
        Source::ModB,
        Source::FilterEnv,
        Source::AmpEnvA,
        Source::AmpEnvB,
        Source::Velocity,
        Source::Keytrack,
        Source::Random,
        Source::PolyAftertouch,
        Source::ModWheel,
        Source::ChannelAftertouch,
        Source::Macro1,
        Source::Macro2,
        Source::Macro3,
        Source::Macro4,
        Source::BusCv1,
        Source::BusCv2,
        Source::BusCv3,
    ];

    /// Display name, also the preset / state spelling.
    pub fn name(self) -> &'static str {
        SOURCE_NAMES[self as usize]
    }

    pub fn from_name(s: &str) -> Option<Self> {
        SOURCE_NAMES.iter().position(|n| *n == s).map(|i| Self::ALL[i])
    }

    pub fn from_index(i: usize) -> Option<Self> {
        Self::ALL.get(i).copied()
    }

    /// Per-voice source (one value per voice); false = one global value.
    pub fn per_voice(self) -> bool {
        self as usize <= Source::PolyAftertouch as usize
    }

    /// Natural range -1..1 (LFOs, keytrack, random, bus CV); false = 0..1.
    pub fn bipolar(self) -> bool {
        BIPOLAR[self as usize]
    }
}

impl Serialize for Source {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for Source {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Source::from_name(&s).ok_or_else(|| serde::de::Error::custom(format!("unknown source \"{s}\"")))
    }
}

// ---- data model ------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Slot {
    pub source: Source,
    /// Target param id (stable across versions).
    pub target: String,
    /// -1..1, fraction of the target's normalized range.
    pub amount: f32,
    pub bipolar: bool,
    pub enabled: bool,
}

impl Slot {
    /// A new slot at the source's natural polarity, enabled.
    pub fn new(source: Source, target: &str, amount: f32) -> Self {
        Self { source, target: target.into(), amount: amount.clamp(-1.0, 1.0), bipolar: source.bipolar(), enabled: true }
    }
}

fn default_labels() -> [String; MACROS] {
    std::array::from_fn(|k| format!("Macro {}", k + 1))
}

/// Slots and macro labels: what the project state and a preset carry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Matrix {
    #[serde(default)]
    pub slots: Vec<Slot>,
    #[serde(default = "default_labels")]
    pub macros: [String; MACROS],
}

impl Default for Matrix {
    fn default() -> Self {
        Self { slots: vec![], macros: default_labels() }
    }
}

impl Matrix {
    /// Drops slots whose target is not a matrix target, clamps amounts, keeps the first
    /// [`MAX_SLOTS`]. Returns one line per change.
    pub fn sanitize(&mut self) -> Vec<String> {
        let mut w = vec![];
        self.slots.retain(|s| {
            let ok = is_target(&s.target);
            if !ok {
                w.push(format!("matrix: {} -> {}: not a modulation target, slot dropped", s.source.name(), s.target));
            }
            ok
        });
        for s in &mut self.slots {
            let a = if s.amount.is_finite() { s.amount.clamp(-1.0, 1.0) } else { 0.0 };
            if a != s.amount {
                w.push(format!("matrix: {} -> {}: amount {} set to {a}", s.source.name(), s.target, s.amount));
                s.amount = a;
            }
        }
        if self.slots.len() > MAX_SLOTS {
            w.push(format!("matrix: {} slots, the last {} dropped", self.slots.len(), self.slots.len() - MAX_SLOTS));
            self.slots.truncate(MAX_SLOTS);
        }
        w
    }

    /// The fixed-size record the audio thread reads.
    pub fn encode(&self) -> [f32; ENC_LEN] {
        let mut e = [0.0; ENC_LEN];
        for (i, rec) in e.chunks_exact_mut(ENC_STRIDE).enumerate() {
            let slot = self.slots.get(i).and_then(|s| Some((s, param_index(&s.target).filter(|&p| params_info()[p].target)?)));
            match slot {
                Some((s, p)) => rec.copy_from_slice(&[s.source as usize as f32, p as f32, s.amount, s.bipolar as u8 as f32, s.enabled as u8 as f32]),
                None => rec[0] = -1.0,
            }
        }
        e
    }

    /// Index of the slot routing `source` to `target`.
    pub fn find(&self, source: Source, target: &str) -> Option<usize> {
        self.slots.iter().position(|s| s.source == source && s.target == target)
    }
}

// ---- params and targets ----------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub(crate) enum Range {
    F(FloatRange),
    I(IntRange),
}

impl Range {
    fn normalize(&self, plain: f32) -> f32 {
        match self {
            Range::F(r) => r.normalize(plain),
            Range::I(r) => r.normalize(plain.round() as i32),
        }
    }

    fn unnormalize(&self, v: f32) -> f32 {
        match self {
            Range::F(r) => r.unnormalize(v),
            Range::I(r) => r.unnormalize(v) as f32,
        }
    }
}

/// How a `Patch` field stores its value.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    F32,
    I32,
    Usize,
    U32,
}

/// A `Patch` field: byte offset and storage type, taken from the preset slot table (the one
/// place that names every field by param id).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Field {
    off: usize,
    kind: Kind,
}

impl Field {
    fn read(self, p: &Patch) -> f32 {
        let b = (p as *const Patch).cast::<u8>();
        // SAFETY: `off` / `kind` were measured from a live field of a `Patch` (`build_info`), so
        // the pointer is in bounds, aligned and points to a value of that type.
        unsafe {
            let at = b.add(self.off);
            match self.kind {
                Kind::F32 => *at.cast::<f32>(),
                Kind::I32 => *at.cast::<i32>() as f32,
                Kind::Usize => *at.cast::<usize>() as f32,
                Kind::U32 => *at.cast::<u32>() as f32,
            }
        }
    }

    fn write(self, p: &mut Patch, plain: f32) {
        let b = (p as *mut Patch).cast::<u8>();
        // SAFETY: as in `read`; every value written is a plain number of the field's type.
        unsafe {
            let at = b.add(self.off);
            match self.kind {
                Kind::F32 => *at.cast::<f32>() = plain,
                Kind::I32 => *at.cast::<i32>() = plain.round() as i32,
                Kind::Usize => *at.cast::<usize>() = plain.round().max(0.0) as usize,
                Kind::U32 => *at.cast::<u32>() = plain.round().max(0.0) as u32,
            }
        }
    }
}

/// Side effects a modulated target needs beyond the per-sample patch read.
pub mod flag {
    /// Voice: read by `Voice::update` (envelope times / curves, unison, shaper amount, sub, Vel Attack).
    pub const UPDATE: u8 = 1;
    /// Voice: LFO rate (`Voice::configure_mods`).
    pub const LFO: u8 = 2;
    /// Voice: velocity -> level amounts, latched at note-on.
    pub const VEL_GAIN: u8 = 4;
    /// Voice: portamento coefficient.
    pub const PORTA: u8 = 8;
    /// Global: FX chain settings (`FxChain::set`).
    pub const FX: u8 = 16;
    pub const XOVER: u8 = 32;
    pub const VOLUME: u8 = 64;
}

const FX_PREFIXES: [&str; 8] = ["dist_", "mb_", "eq_", "cho_", "dim_", "dly_", "rev_", "lim_"];

fn is_global_id(id: &str) -> bool {
    FX_PREFIXES.iter().any(|p| id.starts_with(p)) || matches!(id, "volume" | "spread" | "mono_xover" | "bend_range")
}

fn flags_of(id: &str) -> u8 {
    let osc_update = ["att", "dec", "sus", "rel", "curve", "unison", "detune", "det_curve", "blend", "width"];
    let tail = id.strip_prefix("osca_").or_else(|| id.strip_prefix("oscb_"));
    if tail.is_some_and(|t| osc_update.contains(&t))
        || matches!(id, "fenv_att" | "fenv_dec" | "fenv_sus" | "fenv_rel" | "fenv_curve" | "shaper_amt" | "sub_level" | "sub_oct" | "vel_att")
    {
        return flag::UPDATE;
    }
    match id {
        "moda_rate" | "modb_rate" => flag::LFO,
        "vel_vol_a" | "vel_vol_b" => flag::VEL_GAIN,
        "porta" => flag::PORTA,
        "mono_xover" => flag::XOVER,
        "volume" => flag::VOLUME,
        _ if FX_PREFIXES.iter().any(|p| id.starts_with(p)) => flag::FX,
        _ => 0,
    }
}

/// One parameter, in `GtsParams::param_map` order (its position is the param index ModLive and
/// the audio encoding use).
#[derive(Debug)]
pub struct ParamInfo {
    pub id: String,
    pub name: String,
    /// A matrix target.
    pub target: bool,
    /// Voice-scope target (evaluated per voice); false for global targets and non-targets.
    pub voice: bool,
    pub(crate) flags: u8,
    pub(crate) range: Option<Range>,
    pub(crate) field: Option<Field>,
}

impl ParamInfo {
    /// Normalized value of this param in `p` (0 for params without a patch field or range).
    pub fn value01(&self, p: &Patch) -> f32 {
        match (self.range, self.field) {
            (Some(r), Some(f)) => r.normalize(f.read(p)),
            _ => 0.0,
        }
    }

    /// Plain value of a normalized one.
    pub fn plain(&self, v01: f32) -> f32 {
        self.range.map_or(v01, |r| r.unnormalize(v01.clamp(0.0, 1.0)))
    }

    /// Normalized value of a plain one.
    pub fn norm(&self, plain: f32) -> f32 {
        self.range.map_or(plain, |r| r.normalize(plain))
    }
}

fn build_info() -> Vec<ParamInfo> {
    use crate::preset::Slot as P;
    let params = GtsParams::default();
    let mut patch = Patch::default();
    let base = &patch as *const Patch as usize;
    let fields: Vec<(String, Field)> = crate::preset::slots(&mut patch)
        .into_iter()
        .filter_map(|(id, s)| {
            let (at, kind) = match s {
                P::F(x) | P::IF(x) => (x as *mut f32 as usize, Kind::F32),
                P::I(x) => (x as *mut i32 as usize, Kind::I32),
                P::N(x) => (x as *mut usize as usize, Kind::Usize),
                P::U(x) => (x as *mut u32 as usize, Kind::U32),
                P::B(_) | P::E(_) => return None,
            };
            Some((id, Field { off: at - base, kind }))
        })
        .collect();
    params
        .param_map()
        .into_iter()
        .map(|(id, ptr, _)| {
            // SAFETY: `params` outlives every use of `ptr` here.
            let (range, name) = unsafe {
                let range = match ptr {
                    ParamPtr::FloatParam(p) => Some(Range::F((*p).range())),
                    ParamPtr::IntParam(p) => Some(Range::I((*p).range())),
                    _ => None,
                };
                (range, ptr.name().to_string())
            };
            let field = fields.iter().find(|f| f.0 == id).map(|f| f.1);
            let target = range.is_some() && field.is_some() && !id.starts_with("macro") && !id.starts_with("cv") && id != "poly";
            let voice = target && !is_global_id(&id);
            let flags = if target { flags_of(&id) } else { 0 };
            ParamInfo { id, name, target, voice, flags, range, field }
        })
        .collect()
}

/// Every parameter in param-map order (built once; call off the audio thread first).
pub fn params_info() -> &'static [ParamInfo] {
    static INFO: OnceLock<Vec<ParamInfo>> = OnceLock::new();
    INFO.get_or_init(build_info)
}

/// Param index (position in [`params_info`]) of `id`.
pub fn param_index(id: &str) -> Option<usize> {
    params_info().iter().position(|p| p.id == id)
}

/// `id` can be a slot target.
pub fn is_target(id: &str) -> bool {
    param_index(id).is_some_and(|i| params_info()[i].target)
}

/// Every slot target id, in param-map order.
pub fn target_ids() -> impl Iterator<Item = &'static str> {
    params_info().iter().filter(|p| p.target).map(|p| p.id.as_str())
}

// ---- fixed routes ----------------------------------------------------------------------------

/// When a fixed route actually reaches its target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gate {
    Always,
    /// Mod A / Mod B Target includes osc (or filter) 0 = A, 1 = B.
    ModATarget(usize),
    ModBTarget(usize),
    /// Filter A / B Env switch on.
    FiltEnv(usize),
    /// `cv{n}_target` (n = 1..3) is this target.
    Cv(usize, CvTarget),
}

/// A v1 route param presented as a built-in matrix row.
#[derive(Clone, Copy, Debug)]
pub struct FixedRoute {
    pub source: Source,
    pub target: &'static str,
    /// The route param whose value is this row's amount.
    pub route: &'static str,
    pub gate: Gate,
    /// How the DSP scales the route param (for the MATRIX page).
    pub scale: &'static str,
}

fn build_routes() -> Vec<FixedRoute> {
    use Gate::*;
    use Source::*;
    let r = |source, target, route, gate, scale| FixedRoute { source, target, route, gate, scale };
    let mut v = vec![
        r(ModA, "osca_index", "moda_index", ModATarget(0), "±127 Index steps"),
        r(ModA, "oscb_index", "moda_index", ModATarget(1), "±127 Index steps"),
        r(ModA, "osca_shift", "moda_shift", ModATarget(0), "±24 st"),
        r(ModA, "oscb_shift", "moda_shift", ModATarget(1), "±24 st"),
        r(ModB, "osca_motion", "modb_motion", ModBTarget(0), "±1 Motion"),
        r(ModB, "oscb_motion", "modb_motion", ModBTarget(1), "±1 Motion"),
        r(ModB, "osca_vol", "modb_vol", ModBTarget(0), "level × (1 + amount)"),
        r(ModB, "oscb_vol", "modb_vol", ModBTarget(1), "level × (1 + amount)"),
        r(ModB, "filta_freq", "modb_cutoff", ModBTarget(0), "±5 octaves"),
        r(ModB, "filtb_freq", "modb_cutoff", ModBTarget(1), "±5 octaves"),
        r(ModWheel, "osca_index", "wheel_index", Always, "±127 Index steps"),
        r(ModWheel, "oscb_index", "wheel_index", Always, "±127 Index steps"),
        r(ModWheel, "osca_shift", "wheel_shift", Always, "±24 st"),
        r(ModWheel, "oscb_shift", "wheel_shift", Always, "±24 st"),
        r(ModWheel, "filta_freq", "wheel_cutoff", Always, "±5 octaves"),
        r(ModWheel, "filtb_freq", "wheel_cutoff", Always, "±5 octaves"),
        r(Velocity, "osca_vol", "vel_vol_a", Always, "level fade by velocity"),
        r(Velocity, "oscb_vol", "vel_vol_b", Always, "level fade by velocity"),
        r(Velocity, "fenv_amt", "vel_fenv", Always, "envelope depth fade by velocity"),
        r(Velocity, "osca_att", "vel_att", Always, "attack × 16^amount"),
        r(Velocity, "oscb_att", "vel_att", Always, "attack × 16^amount"),
        r(Velocity, "osca_shift", "vel_shift", Always, "±24 st"),
        r(Velocity, "oscb_shift", "vel_shift", Always, "±24 st"),
        r(FilterEnv, "filta_freq", "fenv_amt", FiltEnv(0), "+8 octaves"),
        r(FilterEnv, "filtb_freq", "fenv_amt", FiltEnv(1), "+8 octaves"),
    ];
    // Mod B "Mod A", wheel Mod and velocity Mod scale all three Mod A amounts.
    for t in ["moda_pitch", "moda_index", "moda_shift"] {
        v.push(r(ModB, t, "modb_moda", Always, "Mod A depth × (1 + amount)"));
        v.push(r(ModWheel, t, "wheel_mod", Always, "Mod A depth × (1 + amount)"));
        v.push(r(Velocity, t, "vel_mod", Always, "Mod A depth fade by velocity"));
    }
    for (n, (src, route)) in [(BusCv1, "cv1_amount"), (BusCv2, "cv2_amount"), (BusCv3, "cv3_amount")].into_iter().enumerate() {
        for i in 1..CvTarget::variants().len() {
            let t = CvTarget::from_index(i);
            v.push(r(src, cv_target_id(t), route, Cv(n + 1, t), "±1 normalized"));
        }
    }
    v
}

/// The param a CV-in target drives (`""` for Off).
pub fn cv_target_id(t: CvTarget) -> &'static str {
    match t {
        CvTarget::Off => "",
        CvTarget::OscAIndex => "osca_index",
        CvTarget::OscAShift => "osca_shift",
        CvTarget::OscBIndex => "oscb_index",
        CvTarget::OscBShift => "oscb_shift",
        CvTarget::FilterAFreq => "filta_freq",
        CvTarget::FilterBFreq => "filtb_freq",
        // bus::apply_cv: "Mod A Amount" drives Mod A's Index amount.
        CvTarget::ModAAmount => "moda_index",
        CvTarget::Volume => "volume",
    }
}

/// Every fixed route (source, target) -> route param. Mod A -> pitch has no target knob and is not
/// listed.
pub fn fixed_routes() -> &'static [FixedRoute] {
    static R: OnceLock<Vec<FixedRoute>> = OnceLock::new();
    R.get_or_init(build_routes)
}

/// The fixed route of a (source, target) pair whatever its gate (several CV entries share a pair:
/// the one whose gate is open wins in [`live_route`]).
pub fn fixed_route(source: Source, target: &str) -> Option<&'static FixedRoute> {
    fixed_routes().iter().find(|r| r.source == source && r.target == target)
}

fn enum_index(c: &dyn Ctl, id: &str, variants: usize) -> usize {
    (c.get01(id) * (variants - 1) as f32).round() as usize
}

/// The route's gate is open under the current params.
pub fn gate_open(r: &FixedRoute, c: &dyn Ctl) -> bool {
    let hits = |id: &str, i: usize| {
        let t = enum_index(c, id, 3); // A, B, Both
        t == 2 || t == i
    };
    match r.gate {
        Gate::Always => true,
        Gate::ModATarget(i) => hits("moda_target", i),
        Gate::ModBTarget(i) => hits("modb_target", i),
        Gate::FiltEnv(j) => c.get01(["filta_env", "filtb_env"][j]) > 0.5,
        Gate::Cv(n, t) => enum_index(c, &format!("cv{n}_target"), CvTarget::variants().len()) == t.to_index(),
    }
}

/// The fixed route of this pair whose gate is open now.
pub fn live_route(source: Source, target: &str, c: &dyn Ctl) -> Option<&'static FixedRoute> {
    fixed_routes().iter().find(|r| r.source == source && r.target == target && gate_open(r, c))
}

/// Plain value of a route param (its amount) through `c`.
pub fn route_amount(r: &FixedRoute, c: &dyn Ctl) -> f32 {
    param_index(r.route).map_or(0.0, |i| params_info()[i].plain(c.get01(r.route)))
}

// ---- shared state and GUI API ----------------------------------------------------------------

/// `params.matrix`: the canonical matrix (non-audio threads) and its audio-side snapshot.
pub struct MatrixShared {
    state: Mutex<Matrix>,
    snap: Snap,
}

impl Default for MatrixShared {
    fn default() -> Self {
        let m = Matrix::default();
        let snap = Snap::new(ENC_LEN);
        snap.write(&m.encode());
        Self { state: Mutex::new(m), snap }
    }
}

impl MatrixShared {
    fn lock(&self) -> MutexGuard<'_, Matrix> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// GUI: a copy of the matrix.
    pub fn get(&self) -> Matrix {
        self.lock().clone()
    }

    /// GUI / preset / state load: replaces the matrix (sanitized) and publishes it.
    pub fn replace(&self, mut m: Matrix) {
        m.sanitize();
        self.edit(|cur| *cur = m);
    }

    /// GUI: edits the matrix in place and publishes the result (one snapshot write).
    pub fn edit<R>(&self, f: impl FnOnce(&mut Matrix) -> R) -> R {
        let mut g = self.lock();
        let r = f(&mut g);
        self.snap.write(&g.encode());
        r
    }

    /// Audio thread, once per block: when a newer matrix was published than `seen`, copies its
    /// record into `out` and returns true. Wait-free, allocation-free; a read that raced a write
    /// returns false and is retried next block.
    pub fn poll(&self, seen: &mut u32, out: &mut [f32; ENC_LEN]) -> bool {
        let v = self.snap.version();
        if v == *seen || !self.snap.read(out) {
            return false;
        }
        *seen = v;
        true
    }
}

impl<'a> PersistentField<'a, Matrix> for MatrixShared {
    fn set(&self, new_value: Matrix) {
        self.replace(new_value);
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&Matrix) -> R,
    {
        f(&self.lock())
    }
}

/// What [`MatrixEditor::add_or_edit`] did.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Edit {
    /// Created or edited the slot at this index.
    Slot(usize),
    /// The pair is a live fixed route: this route param was set (one host gesture).
    Fixed(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EditError {
    NotATarget,
    /// All [`MAX_SLOTS`] slots are in use.
    Full,
}

/// GUI-side matrix editing (see the module docs). Fixed routes go through `ctl` as host gestures.
pub struct MatrixEditor<'a> {
    pub shared: &'a MatrixShared,
    pub ctl: &'a dyn Ctl,
}

impl MatrixEditor<'_> {
    /// Drop of `source` on `target` at `amount`: edits the live fixed route of the pair, else the
    /// pair's slot, else adds a slot.
    pub fn add_or_edit(&self, source: Source, target: &str, amount: f32) -> Result<Edit, EditError> {
        if let Some(r) = live_route(source, target, self.ctl) {
            let i = param_index(r.route).expect("fixed routes name real params");
            let v = params_info()[i].norm(amount).clamp(0.0, 1.0);
            self.ctl.begin(r.route);
            self.ctl.set01(r.route, v);
            self.ctl.end(r.route);
            return Ok(Edit::Fixed(r.route));
        }
        if !is_target(target) {
            return Err(EditError::NotATarget);
        }
        let amount = amount.clamp(-1.0, 1.0);
        self.shared.edit(|m| match m.find(source, target) {
            Some(i) => {
                m.slots[i].amount = amount;
                Ok(Edit::Slot(i))
            }
            None if m.slots.len() < MAX_SLOTS => {
                m.slots.push(Slot::new(source, target, amount));
                Ok(Edit::Slot(m.slots.len() - 1))
            }
            None => Err(EditError::Full),
        })
    }

    fn with_slot(&self, i: usize, f: impl FnOnce(&mut Slot)) {
        self.shared.edit(|m| {
            if let Some(s) = m.slots.get_mut(i) {
                f(s)
            }
        })
    }

    pub fn set_amount(&self, slot: usize, amount: f32) {
        self.with_slot(slot, |s| s.amount = amount.clamp(-1.0, 1.0))
    }

    pub fn set_bipolar(&self, slot: usize, bipolar: bool) {
        self.with_slot(slot, |s| s.bipolar = bipolar)
    }

    pub fn set_enabled(&self, slot: usize, enabled: bool) {
        self.with_slot(slot, |s| s.enabled = enabled)
    }

    pub fn remove(&self, slot: usize) {
        self.shared.edit(|m| {
            if slot < m.slots.len() {
                m.slots.remove(slot);
            }
        })
    }

    /// "Remove all modulation" on a knob: its slots are removed and its live fixed routes set to
    /// 0. Returns how many rows went.
    pub fn remove_all_for_target(&self, target: &str) -> usize {
        let mut n = self.shared.edit(|m| {
            let before = m.slots.len();
            m.slots.retain(|s| s.target != target);
            before - m.slots.len()
        });
        for (r, _) in self.fixed_routes_for_target(target) {
            let i = param_index(r.route).expect("fixed routes name real params");
            self.ctl.begin(r.route);
            self.ctl.set01(r.route, params_info()[i].norm(0.0));
            self.ctl.end(r.route);
            n += 1;
        }
        n
    }

    /// Slot indices that target `target`.
    pub fn slots_for_target(&self, target: &str) -> Vec<usize> {
        self.shared.lock().slots.iter().enumerate().filter(|(_, s)| s.target == target).map(|(i, _)| i).collect()
    }

    /// Live fixed routes into `target` with a non-zero amount, and that amount (route units).
    pub fn fixed_routes_for_target(&self, target: &str) -> Vec<(&'static FixedRoute, f32)> {
        fixed_routes()
            .iter()
            .filter(|r| r.target == target && gate_open(r, self.ctl))
            .map(|r| (r, route_amount(r, self.ctl)))
            .filter(|(_, a)| *a != 0.0)
            .collect()
    }

    /// Every target `source` moves: its slots, then its live non-zero fixed routes (no duplicates).
    pub fn targets_of_source(&self, source: Source) -> Vec<String> {
        let mut v: Vec<String> = self.shared.lock().slots.iter().filter(|s| s.source == source).map(|s| s.target.clone()).collect();
        for r in fixed_routes().iter().filter(|r| r.source == source && gate_open(r, self.ctl)) {
            if route_amount(r, self.ctl) != 0.0 && !v.iter().any(|t| t == r.target) {
                v.push(r.target.to_string());
            }
        }
        v
    }

    pub fn set_macro_label(&self, k: usize, label: &str) {
        self.shared.edit(|m| {
            if let Some(l) = m.macros.get_mut(k) {
                *l = label.to_string();
            }
        })
    }
}

// ---- audio-side runtime ----------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
struct RtSlot {
    src: usize,
    /// Index into the global or voice target list.
    dst: usize,
    voice: bool,
    amount: f32,
    bipolar: bool,
}

/// The engine's matrix state: decoded slots, the distinct targets per scope and their smoothed
/// offsets (normalized). Fixed size, no allocation after `new`.
pub struct ModRt {
    info: &'static [ParamInfo],
    slots: [RtSlot; MAX_SLOTS],
    n: usize,
    /// Distinct global / voice-scope targets (param indices) and their flags.
    g: [usize; MAX_SLOTS],
    ng: usize,
    gflags: u8,
    v: [usize; MAX_SLOTS],
    nv: usize,
    vflags: u8,
    g_off: [f32; MAX_SLOTS],
    v_off: [[f32; MAX_SLOTS]; MAX_VOICES],
    /// Normalized base value of each voice-scope target, from the engine patch.
    v_base: [f32; MAX_SLOTS],
    /// Live targets are `g[..lg]` / `v[..lv]`; the rest lost their last slot and glide to 0
    /// before [`ModRt::prune`] drops them (removal is smoothed like addition).
    lg: usize,
    lv: usize,
}

/// Offset below which a fading target is dropped.
const SETTLED: f32 = 1e-5;

impl Default for ModRt {
    fn default() -> Self {
        Self::new()
    }
}

/// Source value at a slot's polarity.
#[inline]
fn polar(x: f32, natural_bipolar: bool, bipolar: bool) -> f32 {
    match (natural_bipolar, bipolar) {
        (true, false) => 0.5 * (x + 1.0),
        (false, true) => 2.0 * x - 1.0,
        _ => x,
    }
}

/// [`Source::bipolar`] by index.
const BIPOLAR: [bool; N_SOURCES] = {
    let mut b = [false; N_SOURCES];
    let mut i = 0;
    while i < N_SOURCES {
        b[i] = matches!(Source::ALL[i], Source::ModA | Source::ModB | Source::Keytrack | Source::Random | Source::BusCv1 | Source::BusCv2 | Source::BusCv3);
        i += 1;
    }
    b
};

impl ModRt {
    /// Builds the param table if needed (allocates once; not on the audio thread).
    pub fn new() -> Self {
        Self {
            info: params_info(),
            slots: [RtSlot::default(); MAX_SLOTS],
            n: 0,
            g: [0; MAX_SLOTS],
            ng: 0,
            gflags: 0,
            v: [0; MAX_SLOTS],
            nv: 0,
            vflags: 0,
            g_off: [0.0; MAX_SLOTS],
            v_off: [[0.0; MAX_SLOTS]; MAX_VOICES],
            v_base: [0.0; MAX_SLOTS],
            lg: 0,
            lv: 0,
        }
    }

    /// Any enabled slot, or a removed target still gliding out.
    pub fn is_on(&self) -> bool {
        self.ng + self.nv > 0
    }
    pub fn has_global(&self) -> bool {
        self.ng > 0
    }
    pub fn has_voice(&self) -> bool {
        self.nv > 0
    }
    pub fn gflags(&self) -> u8 {
        self.gflags
    }
    pub fn vflags(&self) -> u8 {
        self.vflags
    }

    /// Modulated targets: (param index, voice scope).
    pub fn targets(&self) -> impl Iterator<Item = (usize, bool)> + '_ {
        self.g[..self.ng].iter().map(|&p| (p, false)).chain(self.v[..self.nv].iter().map(|&p| (p, true)))
    }

    /// Takes a new encoded matrix. Offsets of targets that stay modulated carry over; new
    /// targets start at 0 and glide in, removed ones glide out. Allocation-free.
    pub fn load(&mut self, enc: &[f32; ENC_LEN]) {
        let (old_g, old_ng, old_goff) = (self.g, self.ng, self.g_off);
        let (old_v, old_nv, old_voff) = (self.v, self.nv, self.v_off);
        (self.n, self.ng, self.nv, self.gflags, self.vflags) = (0, 0, 0, 0, 0);
        for rec in enc.chunks_exact(ENC_STRIDE) {
            let (src, p) = (rec[0], rec[1]);
            if src < 0.0 || rec[4] < 0.5 || rec[2] == 0.0 || !rec[2].is_finite() {
                continue;
            }
            let (src, p) = (src as usize, p as usize);
            let Some(info) = self.info.get(p).filter(|i| i.target && src < N_SOURCES) else { continue };
            let voice = info.voice;
            let (list, n, flags) = if voice { (&mut self.v, &mut self.nv, &mut self.vflags) } else { (&mut self.g, &mut self.ng, &mut self.gflags) };
            let dst = match list[..*n].iter().position(|&q| q == p) {
                Some(d) => d,
                None => {
                    list[*n] = p;
                    *n += 1;
                    *flags |= info.flags;
                    *n - 1
                }
            };
            self.slots[self.n] = RtSlot { src, dst, voice, amount: rec[2].clamp(-1.0, 1.0), bipolar: rec[3] > 0.5 };
            self.n += 1;
        }
        for j in 0..self.ng {
            self.g_off[j] = old_g[..old_ng].iter().position(|&q| q == self.g[j]).map_or(0.0, |k| old_goff[k]);
        }
        for j in 0..self.nv {
            let k = old_v[..old_nv].iter().position(|&q| q == self.v[j]);
            for (off, old) in self.v_off.iter_mut().zip(&old_voff) {
                off[j] = k.map_or(0.0, |k| old[k]);
            }
        }
        (self.lg, self.lv) = (self.ng, self.nv);
        // Targets that lost their last slot keep their offset and glide out (raw sum 0).
        // ponytail: past MAX_SLOTS distinct targets a removed one still drops at once.
        for k in 0..old_ng {
            let t = old_g[k];
            if self.ng < MAX_SLOTS && !self.g[..self.ng].contains(&t) && old_goff[k].abs() > SETTLED {
                (self.g[self.ng], self.g_off[self.ng]) = (t, old_goff[k]);
                self.gflags |= self.info[t].flags;
                self.ng += 1;
            }
        }
        for k in 0..old_nv {
            let t = old_v[k];
            if self.nv < MAX_SLOTS && !self.v[..self.nv].contains(&t) && old_voff.iter().any(|o| o[k].abs() > SETTLED) {
                self.v[self.nv] = t;
                for (off, old) in self.v_off.iter_mut().zip(&old_voff) {
                    off[self.nv] = old[k];
                }
                self.vflags |= self.info[t].flags;
                self.nv += 1;
            }
        }
    }

    /// Drops fading targets whose offset settled (voice scope: on every active voice).
    pub(crate) fn prune(&mut self, active: impl Fn(usize) -> bool) {
        if (self.ng, self.nv) == (self.lg, self.lv) {
            return;
        }
        let mut j = self.lg;
        while j < self.ng {
            if self.g_off[j].abs() < SETTLED {
                self.g.copy_within(j + 1..self.ng, j);
                self.g_off.copy_within(j + 1..self.ng, j);
                self.ng -= 1;
            } else {
                j += 1;
            }
        }
        let mut j = self.lv;
        while j < self.nv {
            if (0..MAX_VOICES).all(|v| !active(v) || self.v_off[v][j].abs() < SETTLED) {
                self.v.copy_within(j + 1..self.nv, j);
                for o in &mut self.v_off {
                    o.copy_within(j + 1..self.nv, j);
                }
                self.nv -= 1;
            } else {
                j += 1;
            }
        }
        self.gflags = self.g[..self.ng].iter().fold(0, |f, &t| f | self.info[t].flags);
        self.vflags = self.v[..self.nv].iter().fold(0, |f, &t| f | self.info[t].flags);
    }

    /// Zeroes every smoothed offset (engine reset).
    pub fn reset(&mut self) {
        self.g_off = [0.0; MAX_SLOTS];
        self.v_off = [[0.0; MAX_SLOTS]; MAX_VOICES];
        self.prune(|_| false);
    }

    /// Sum of the slots of one scope into `raw` (per target).
    #[inline]
    fn sum(&self, voice: bool, src: &[f32; N_SOURCES]) -> [f32; MAX_SLOTS] {
        let mut raw = [0.0f32; MAX_SLOTS];
        for s in &self.slots[..self.n] {
            if s.voice == voice {
                raw[s.dst] += s.amount * polar(src[s.src], BIPOLAR[s.src], s.bipolar);
            }
        }
        raw
    }

    /// Global targets: sum the slots for `src`, smooth by `k` (0 = jump).
    pub(crate) fn eval_global(&mut self, src: &[f32; N_SOURCES], k: f32) {
        let raw = self.sum(false, src);
        for (o, r) in self.g_off[..self.ng].iter_mut().zip(raw) {
            *o = r + (*o - r) * k;
        }
    }

    /// Voice-scope targets of voice `v` from its sources.
    pub(crate) fn eval_voice(&mut self, v: usize, src: &[f32; N_SOURCES], k: f32) {
        let raw = self.sum(true, src);
        for (o, r) in self.v_off[v][..self.nv].iter_mut().zip(raw) {
            *o = r + (*o - r) * k;
        }
    }

    /// Writes the global offsets onto `p` (which holds the unmodulated values): one clamp per target.
    pub(crate) fn write_global(&self, p: &mut Patch) {
        for (&t, &o) in self.g[..self.ng].iter().zip(&self.g_off) {
            let i = &self.info[t];
            let (r, f) = (i.range.unwrap(), i.field.unwrap());
            f.write(p, r.unnormalize((r.normalize(f.read(p)) + o).clamp(0.0, 1.0)));
        }
    }

    /// Caches the normalized base value of every voice-scope target from `p` (the engine patch).
    pub(crate) fn prepare_voice_base(&mut self, p: &Patch) {
        for (b, &t) in self.v_base.iter_mut().zip(&self.v[..self.nv]) {
            let i = &self.info[t];
            *b = i.range.unwrap().normalize(i.field.unwrap().read(p));
        }
    }

    /// Writes voice `v`'s offsets onto `p` (a copy of the engine patch, base cached by
    /// [`ModRt::prepare_voice_base`]).
    pub(crate) fn write_voice(&self, v: usize, p: &mut Patch) {
        for j in 0..self.nv {
            let i = &self.info[self.v[j]];
            let r = i.range.unwrap();
            i.field.unwrap().write(p, r.unnormalize((self.v_base[j] + self.v_off[v][j]).clamp(0.0, 1.0)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every target's field accessor reads and writes the field the preset table names.
    #[test]
    fn field_offsets_match_the_preset_table() {
        let info = params_info();
        let mut p = Patch::default();
        for (k, i) in info.iter().enumerate().filter(|(_, i)| i.target) {
            let (r, f) = (i.range.unwrap(), i.field.unwrap());
            let v = r.unnormalize(0.37 + 0.001 * k as f32 % 0.5);
            f.write(&mut p, v);
            let json: serde_json::Value = serde_json::from_str(&crate::preset::Preset { patch: p, ..Default::default() }.to_json()).unwrap();
            let got = json["params"][&i.id].as_f64().unwrap() as f32;
            assert!((got - v).abs() <= 1e-4 * v.abs().max(1.0), "{}: wrote {v}, preset reads {got}", i.id);
            assert_eq!(f.read(&p), v, "{}", i.id);
        }
    }
}
