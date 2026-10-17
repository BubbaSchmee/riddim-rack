//! GTS editor in the shared Riddim Rack look (`docs/design/DESIGN.md`, v2 layout and motion from
//! `docs/design/MOTION.md`). Fixed [`WINDOW`] 1000x832, laid out in absolute window coordinates
//! (MOTION.md 2.1): header band (nameplate, preset bar, page switch, Quality, PATCH chip, scope,
//! meter, output), then the SYNTH page (hero wavetable, sources, modulators, oscillators, filters,
//! ...) or the FX page, and the help bar.
//!
//! The UI is a plain function over the [`Ctl`] trait, so the plugin (over `ParamSetter`), the
//! screenshot / benchmark examples and the id-coverage test all render the same code. Every
//! control talks to its parameter by id; values travel normalized (0..1), readouts come from the
//! param itself. Live data (scope ring, voice and modulation record) arrives through [`LiveView`].
//!
//! Everything moving is time-based on `input.time` at the window's own cadence (~62 Hz under
//! nih_plug_egui, which requests a repaint every tick): never frame-counted.

use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, EguiState};
use rr_ui::hero::{Hero, HeroInput, FRAMES, POINTS};
use rr_ui::ring::RingReader;
use rr_ui::theme::metrics::*;
use rr_ui::theme::{font, SRC_A, SRC_B, SRC_E, SRC_M, T};
use rr_ui::widgets::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock, PoisonError, RwLock};

use crate::bus::BusLink;
use crate::dsp::lfo::Lfo;
use crate::dsp::fx::eq::EqParams;
use crate::fxview::{self, FxLive};
use crate::dsp::shaper::Shaper;
use crate::dsp::tables::{Fft, Tables, GRAINS, N};
use crate::params::*;
use crate::preset::{self, Preset};
use crate::viz::{gf, vf, Viz, VizFrame, MOD_IDS};
use egui::{pos2, vec2, Align2, Color32, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2};

/// Window size in logical px (fits 1080p at 125 % OS scaling: 864 px minus the host title bar).
pub const WINDOW: (u32, u32) = (1000, 832);

/// Hero display rect in window px (MOTION.md 2.1).
pub const HERO: (f32, f32, f32, f32) = (44.0, 80.0, 568.0, 232.0);

/// Header quality choice: shaper oversampling factors.
const QUALITY: [usize; 3] = [1, 2, 4];

/// What the editor needs from its host. All values are normalized 0..1.
pub trait Ctl {
    fn get01(&self, id: &str) -> f32;
    fn default01(&self, id: &str) -> f32;
    /// The param's own text for its current value, with unit.
    fn readout(&self, id: &str) -> String;
    /// `Some` for stepped params (int, enum, bool): knobs then quantize.
    fn steps(&self, id: &str) -> Option<usize>;
    fn begin(&self, id: &str);
    fn set01(&self, id: &str, v: f32);
    fn end(&self, id: &str);
    /// Text for any normalized value of `id` (a live, modulated value). Default: a percentage.
    fn readout_at(&self, id: &str, v: f32) -> String {
        let _ = id;
        format!("{:.0} %", v * 100.0)
    }
    /// The param's own display name ("Filter A Freq"). Default: the id.
    fn name(&self, id: &str) -> String {
        id.to_string()
    }
    /// Plain value (Hz, dB, ...) of a normalized value, and back. Default: identity.
    fn plain(&self, id: &str, v01: f32) -> f32 {
        let _ = id;
        v01
    }
    fn norm(&self, id: &str, plain: f32) -> f32 {
        let _ = id;
        plain
    }
}

/// Safe read access to every param by id. Holds the `Arc` so the raw pointers stay valid.
pub struct ParamTable {
    _keep: Arc<GtsParams>,
    map: HashMap<String, ParamPtr>,
    /// Every id a preset sets (all but the RR-BUS CV slots), sorted.
    sound_ids: Vec<String>,
}

// SAFETY: every ParamPtr points into `_keep`, which is Send + Sync and outlives them; reads are atomic.
unsafe impl Send for ParamTable {}
unsafe impl Sync for ParamTable {}

impl ParamTable {
    pub fn new(params: Arc<GtsParams>) -> Self {
        let map: HashMap<String, ParamPtr> = params.param_map().into_iter().map(|(id, ptr, _)| (id, ptr)).collect();
        let mut sound_ids: Vec<String> = map.keys().filter(|k| !k.starts_with("cv")).cloned().collect();
        sound_ids.sort();
        Self { _keep: params, map, sound_ids }
    }
    pub fn ptr(&self, id: &str) -> ParamPtr {
        *self.map.get(id).unwrap_or_else(|| panic!("unknown param id {id}"))
    }
    pub fn get01(&self, id: &str) -> f32 {
        unsafe { self.ptr(id).unmodulated_normalized_value() }
    }
    pub fn default01(&self, id: &str) -> f32 {
        unsafe { self.ptr(id).default_normalized_value() }
    }
    pub fn readout_at(&self, id: &str, v: f32) -> String {
        unsafe { self.ptr(id).normalized_value_to_string(v, true) }
    }
    pub fn name(&self, id: &str) -> String {
        unsafe { self.ptr(id).name().to_string() }
    }
    pub fn plain(&self, id: &str, v01: f32) -> f32 {
        unsafe { self.ptr(id).preview_plain(v01) }
    }
    pub fn norm(&self, id: &str, plain: f32) -> f32 {
        unsafe { self.ptr(id).preview_normalized(plain) }
    }
    pub fn steps(&self, id: &str) -> Option<usize> {
        unsafe { self.ptr(id).step_count() }
    }
    /// Normalized value of every sound param, in id order (to tell whether a loaded preset was edited).
    pub fn snapshot(&self) -> Vec<f32> {
        self.sound_ids.iter().map(|id| self.get01(id)).collect()
    }
}

/// Loads a factory preset (`None` = Init, every sound param at its default) through `c` as host
/// gestures. GUI thread only. The RR-BUS CV slots, the instance label and the Quality setting are
/// project routing, not sound: never touched.
pub fn apply_preset(table: &ParamTable, c: &dyn Ctl, preset: Option<&Preset>) {
    let json = preset.and_then(|p| serde_json::from_str::<serde_json::Value>(&p.to_json()).ok());
    let params = json.as_ref().and_then(|j| j.get("params")).and_then(|p| p.as_object());
    for id in &table.sound_ids {
        // SAFETY: the pointers live as long as `table` (it holds the params Arc).
        let normalized = params.and_then(|m| m.get(id)).and_then(|v| match v {
            serde_json::Value::Bool(b) => Some(*b as u8 as f32),
            serde_json::Value::Number(n) => n.as_f64().map(|x| unsafe { table.ptr(id).preview_normalized(x as f32) }),
            serde_json::Value::String(s) => unsafe { table.ptr(id).string_to_normalized_value(s) },
            _ => None,
        });
        one_shot(c, id, normalized.unwrap_or_else(|| c.default01(id)));
    }
}

/// What the audio thread publishes for the meters; the editor swaps them out each frame.
/// f32 bits (non-negative, so `fetch_max` keeps the larger value).
#[derive(Default)]
pub struct Meters {
    /// Deepest limiter gain reduction since the last read, dB >= 0.
    pub gr_db: AtomicU32,
    /// Largest output sample magnitude per channel (L, R) since the last read, linear.
    pub peak: [AtomicU32; 2],
}

/// Everything the audio side shares with the editor.
pub struct Feeds {
    pub meters: Arc<Meters>,
    pub viz: Arc<Viz>,
    pub scope: RingReader,
}

/// The plugin's `Ctl`: reads through the table, writes through the host's `ParamSetter`.
struct PlugCtl<'a> {
    table: &'a ParamTable,
    setter: &'a ParamSetter<'a>,
}

impl Ctl for PlugCtl<'_> {
    fn get01(&self, id: &str) -> f32 {
        self.table.get01(id)
    }
    fn default01(&self, id: &str) -> f32 {
        self.table.default01(id)
    }
    fn readout(&self, id: &str) -> String {
        self.table.readout_at(id, self.table.get01(id))
    }
    fn steps(&self, id: &str) -> Option<usize> {
        self.table.steps(id)
    }
    fn begin(&self, id: &str) {
        unsafe { self.setter.raw_context.raw_begin_set_parameter(self.table.ptr(id)) }
    }
    fn set01(&self, id: &str, v: f32) {
        unsafe { self.setter.raw_context.raw_set_parameter_normalized(self.table.ptr(id), v.clamp(0.0, 1.0)) }
    }
    fn end(&self, id: &str) {
        unsafe { self.setter.raw_context.raw_end_set_parameter(self.table.ptr(id)) }
    }
    fn readout_at(&self, id: &str, v: f32) -> String {
        self.table.readout_at(id, v)
    }
    fn name(&self, id: &str) -> String {
        self.table.name(id)
    }
    fn plain(&self, id: &str, v01: f32) -> f32 {
        self.table.plain(id, v01)
    }
    fn norm(&self, id: &str, plain: f32) -> f32 {
        self.table.norm(id, plain)
    }
}

// ---- control helpers ----------------------------------------------------------------------

fn one_shot(c: &dyn Ctl, id: &str, v: f32) {
    c.begin(id);
    c.set01(id, v);
    c.end(id);
}

/// Maps a knob response to the host's gesture calls. While a stepped param is dragged, the
/// unquantized position is kept in egui memory (the snapped value would swallow every small delta).
fn drive(ui: &Ui, c: &dyn Ctl, id: &str, r: KnobResponse, stepped: bool) {
    let key = Id::new(("gts-drag", id));
    if r.drag_started {
        c.begin(id);
        ui.data_mut(|d| d.insert_temp(key, c.get01(id)));
    }
    let dragging = ui.data(|d| d.get_temp::<f32>(key)).is_some();
    if let Some(n) = r.changed {
        if dragging {
            c.set01(id, n);
            if stepped {
                ui.data_mut(|d| d.insert_temp(key, n));
            }
        } else {
            one_shot(c, id, n); // double-click reset or a bare click
        }
    }
    if r.drag_ended {
        c.end(id);
        ui.data_mut(|d| d.remove_temp::<f32>(key));
    }
}

/// Per-frame knob decorations: live modulation (ModLive), the modulation ranges and the hovered
/// source chip.
#[derive(Clone, Default)]
struct KnobLive {
    live: [Option<(f32, Color32)>; MOD_IDS.len()],
    /// Range arcs per [`MOD_IDS`] knob, one per source (MOTION.md 5.2).
    ranges: [[Option<RingRange>; 2]; MOD_IDS.len()],
    /// Source chip under the pointer (last frame): 0 Mod A, 1 Mod B, 2 Env, 3 Macro.
    chip: Option<usize>,
    filt_env: [bool; 2],
}

fn knob_live_id() -> Id {
    Id::new("gts-knob-live")
}

/// The chip hovered this frame (read next frame, so knobs drawn before the chips can dim too).
fn chip_id() -> Id {
    Id::new("gts-chip")
}

/// What the help bar shows for the control under the pointer.
#[derive(Clone, Default)]
struct Hover {
    name: String,
    /// Param id (looks up the help line); empty for non-param controls.
    id: String,
    /// Fixed help line for controls that are not params.
    line: Option<&'static str>,
    value: String,
    /// Live (modulated) value text, its source colour and glyph.
    live: Option<(String, Color32, &'static str)>,
}

fn hover_id() -> Id {
    Id::new("gts-hover")
}

fn glyph_of(col: Color32) -> &'static str {
    if col == SRC_A {
        "A"
    } else if col == SRC_B {
        "B"
    } else if col == SRC_E {
        "E"
    } else {
        "M"
    }
}

/// Which sources move `id` (for the chip hover ring): bit 0 Mod A, 1 Mod B, 2 Env.
fn sources_of(id: &str, filt_env: [bool; 2]) -> u8 {
    match id {
        "osca_index" | "oscb_index" | "osca_shift" | "oscb_shift" => 1,
        "osca_motion" | "oscb_motion" | "osca_vol" | "oscb_vol" | "moda_pitch" | "moda_index" | "moda_shift" => 2,
        "filta_freq" => 2 | if filt_env[0] { 4 } else { 0 },
        "filtb_freq" => 2 | if filt_env[1] { 4 } else { 0 },
        "fenv_amt" => 4,
        _ => 0,
    }
}

/// When the last preset load happened (egui time); knobs glide for a while after it.
fn glide_id() -> Id {
    Id::new("gts-preset-glide")
}
const PRESET_GLIDE: f32 = 0.25;
const PRESET_GLIDE_WINDOW: f64 = 0.4;

fn kn(ui: &mut Ui, c: &dyn Ctl, id: &str, label: &str, mut style: KnobStyle) {
    let steps = c.steps(id);
    let stepped = steps.is_some();
    style.steps = steps;
    let shown = if stepped {
        ui.data(|d| d.get_temp::<f32>(Id::new(("gts-drag", id)))).unwrap_or_else(|| c.get01(id))
    } else {
        // Preset load: the audio side switches at once (its own smoothers make that click-free),
        // the dial glides over 250 ms so the whole patch visibly moves (MOTION.md 4).
        let now = ui.input(|i| i.time);
        let t0 = ui.data(|d| d.get_temp::<f64>(glide_id())).unwrap_or(-9.0);
        let secs = if (0.0..PRESET_GLIDE_WINDOW).contains(&(now - t0)) { PRESET_GLIDE } else { 0.001 };
        ui.ctx().animate_value_with_time(Id::new(("gts-glide", id)), c.get01(id), rr_ui::motion::dur(ui.ctx(), secs).max(0.001))
    };
    let mut dim_by_chip = false;
    if style.compact {
        if let Some(kl) = ui.data(|d| d.get_temp::<KnobLive>(knob_live_id())) {
            if let Some(k) = MOD_IDS.iter().position(|m| *m == id) {
                style.live = kl.live[k];
                style.ranges = kl.ranges[k];
            }
            if let Some(ch) = kl.chip {
                if sources_of(id, kl.filt_env) & (1 << ch) != 0 {
                    style.ring = Some([SRC_A, SRC_B, SRC_E, SRC_M][ch]);
                } else {
                    dim_by_chip = true;
                }
            }
        }
    }
    // Chip hover: every knob that is not a target drops to 50 % in 140 ms.
    let dk = rr_ui::motion::tween(ui.ctx(), Id::new(("gts-dim", id)), dim_by_chip, 0.14);
    let r = ui
        .scope(|ui| {
            if dk > 0.0 {
                ui.multiply_opacity(1.0 - 0.5 * dk);
            }
            knob_fn(ui, label, shown, c.default01(id), &|v| c.readout_at(id, v), style)
        })
        .inner;
    if r.hovered {
        let live = style.live.map(|(v, col)| (c.readout_at(id, v), col, glyph_of(col)));
        let value = c.readout_at(id, shown);
        ui.data_mut(|d| d.insert_temp(hover_id(), Hover { name: c.name(id).to_uppercase(), id: id.to_string(), value, live, ..Default::default() }));
    }
    drive(ui, c, id, r, stepped);
}

fn tg(ui: &mut Ui, c: &dyn Ctl, id: &str, label: &str) {
    let on = c.get01(id) > 0.5;
    let (pick, hovered) = led_toggle_ex(ui, label, on);
    if hovered {
        ui.data_mut(|d| d.insert_temp(hover_id(), Hover { name: c.name(id).to_uppercase(), id: id.to_string(), value: if on { "On" } else { "Off" }.into(), ..Default::default() }));
    }
    if let Some(on) = pick {
        one_shot(c, id, if on { 1.0 } else { 0.0 });
    }
}

fn enum_idx<E: Enum>(c: &dyn Ctl, id: &str) -> usize {
    (c.get01(id) * (E::variants().len() - 1) as f32).round() as usize
}

fn cb<E: Enum>(ui: &mut Ui, c: &dyn Ctl, id: &str, label: &str) {
    let cur = enum_idx::<E>(c, id);
    let (pick, hovered) = combo_ex(ui, label, cur, E::variants());
    if hovered {
        let value = E::variants().get(cur).copied().unwrap_or("").to_string();
        ui.data_mut(|d| d.insert_temp(hover_id(), Hover { name: c.name(id).to_uppercase(), id: id.to_string(), value, ..Default::default() }));
    }
    if let Some(i) = pick {
        one_shot(c, id, i as f32 / (E::variants().len() - 1) as f32);
    }
}

/// Opacity of a control that currently has no effect (still editable): 45 % (MOTION.md 4).
const INACTIVE: f32 = 0.45;

/// Dims everything `ui` draws from here on (a group body whose module or section is off), easing
/// over 180 ms when the state changes.
fn fade(ui: &mut Ui, inactive: bool) {
    let p = ui.next_widget_position();
    let k = rr_ui::motion::tween(ui.ctx(), Id::new(("gts-fade", p.x as i32, p.y as i32)), inactive, 0.18);
    if k > 0.0 {
        ui.multiply_opacity(1.0 - (1.0 - INACTIVE) * k);
    }
}

/// Dims one control (or a few) that does nothing in the current mode, e.g. Delay Time while synced.
fn dim(ui: &mut Ui, inactive: bool, add: impl FnOnce(&mut Ui)) {
    ui.scope(|ui| {
        fade(ui, inactive);
        add(ui);
    });
}


fn is_off(c: &dyn Ctl, id: &str) -> bool {
    c.get01(id) < 0.5
}

/// Transfer curve of the shaper as it is currently set, y in 0..1 over x in -1..1.
fn shaper_curve(c: &dyn Ctl) -> Vec<f32> {
    let mut s = Shaper::new();
    // 1x: the oversampled paths delay by their latency, which would shift the plotted curve.
    s.set_oversampling(1);
    s.set(ShaperMode::from_index(enum_idx::<ShaperMode>(c, "shaper_mode")), c.get01("shaper_amt"));
    (0..=32).map(|i| (s.process(i as f32 / 16.0 - 1.0) * 0.5 + 0.5).clamp(0.0, 1.0)).collect()
}

// ---- PATCH popover (RR-BUS) ----------------------------------------------------------------

/// Editor-session view state besides params: the editor fills the bus part from the bus and the
/// persisted state each frame and applies whatever `draw` changed in `label` / `bound`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BusView {
    pub label: String,
    /// Live sources, `"<label> · <name>"`.
    pub sources: Vec<String>,
    /// Persisted source per slot, empty = none.
    pub bound: [String; 3],
    /// |incoming value| per slot.
    pub activity: [f32; 3],
    /// Header Quality: shaper oversampling 1 / 2 / 4 (persisted host state, not a param).
    pub oversampling: usize,
    /// Header page switch: false = SYNTH, true = FX. Editor-session state, never persisted.
    pub fx_page: bool,
    /// Header preset bar: the loaded entry (0 = Init, 1.. = factory order), `None` until one is
    /// picked this session. Editor-session state, never persisted.
    pub preset: Option<usize>,
    /// The loaded preset was changed afterwards (the bar then shows a dot).
    pub preset_dirty: bool,
    /// Set by `draw` when the user picks an entry; the host editor applies it (see [`apply_preset`]).
    pub preset_pick: Option<usize>,
    /// Limiter gain reduction (dB >= 0) and output peak (dBFS) for the FX page meter.
    pub gr_db: f32,
    pub peak_db: f32,
    /// The PATCH popover is open.
    pub patch_open: bool,
    /// "Always show values" (persisted by the host editor).
    pub show_values: bool,
    /// "Reduced motion" (persisted by the host editor): instant chrome, no sway, parallax or page motion.
    pub reduced_motion: bool,
    /// The preset list is open.
    pub preset_open: bool,
}

const CV: [(&str, &str); 3] = [("cv1_target", "cv1_amount"), ("cv2_target", "cv2_amount"), ("cv3_target", "cv3_amount")];

fn patch(ui: &mut Ui, c: &dyn Ctl, bv: &mut BusView) {
    let sources = std::mem::take(&mut bv.sources);
    let idx: Vec<Option<usize>> = bv.bound.iter().map(|b| sources.iter().position(|s| s == b)).collect();
    let mut rows: Vec<PatchRow> = (0..3)
        .map(|i| PatchRow {
            sources: &sources,
            source_idx: idx[i],
            lost_name: (idx[i].is_none() && !bv.bound[i].is_empty()).then_some(bv.bound[i].as_str()),
            targets: CvTarget::variants(),
            target_idx: enum_idx::<CvTarget>(c, CV[i].0),
            amount01: c.get01(CV[i].1),
            activity01: bv.activity[i],
        })
        .collect();
    let r = patch_strip(ui, &mut bv.label, &mut rows);
    drop(rows);
    for (i, e) in r.rows.into_iter().enumerate() {
        if let Some(k) = e.source_changed {
            bv.bound[i] = k.map(|k| sources[k].clone()).unwrap_or_default();
        }
        if let Some(t) = e.target_changed {
            one_shot(c, CV[i].0, t as f32 / (CvTarget::variants().len() - 1) as f32);
        }
        drive(ui, c, CV[i].1, e.amount, false);
    }
    bv.sources = sources;
}

// ---- live data (MOTION.md section 8) ---------------------------------------------------------

/// Scope points, spectrum columns and their ranges (MOTION.md 4 / 5.5).
const SCOPE_PTS: usize = 192;
const SPEC_COLS: usize = 120;
const SPEC_DB: f32 = 66.0;
const FFT_N: usize = 2048;
/// Meter scale in dB.
const METER_DB: f32 = 48.0;

/// GUI side of the live plumbing: the last voice / modulation record, the scope and spectrum
/// buffers with their ballistics, the meter, and the hero. Lives in the editor state; every
/// buffer is allocated once here.
pub struct LiveView {
    viz: Arc<Viz>,
    ring: Option<RingReader>,
    /// The latest complete record (kept when a read races).
    pub frame: VizFrame,
    pub hero: Hero,
    /// Hero shows Osc B.
    pub hero_b: bool,
    hero_key: Option<u64>,
    hero_frames: Vec<f32>,
    /// Linear output peaks (L, R) since the last frame; the host editor fills them from [`Meters`].
    pub peaks: [f32; 2],
    meter_db: [f32; 2],
    hold_db: [f32; 2],
    hold_t: [f64; 2],
    buf: Vec<f32>,
    /// The side ring ((L - R) / 2) over the same window as `buf`.
    side_buf: Vec<f32>,
    trace: Vec<f32>,
    spec_target: Vec<f32>,
    spec: Vec<f32>,
    spec_peak: Vec<f32>,
    /// FX page displays' live state.
    pub fx: FxLive,
    fft: Fft,
    re: Vec<f64>,
    im: Vec<f64>,
    window: Vec<f64>,
    next_fft: f64,
    /// Header display: spectrum shown, and the 60 ms crossfade toward it.
    pub spectrum: bool,
    spec_mix: f32,
    last_t: Option<f64>,
    /// Reduced motion: no sway / parallax, instant chrome.
    pub reduced_motion: bool,
}

impl LiveView {
    pub fn new(viz: Arc<Viz>, ring: Option<RingReader>) -> Self {
        let window = (0..FFT_N).map(|i| 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / FFT_N as f64).cos()).collect();
        Self {
            viz,
            ring,
            frame: VizFrame::default(),
            hero: Hero::default(),
            hero_b: false,
            hero_key: None,
            hero_frames: vec![0.0; FRAMES * POINTS],
            peaks: [0.0; 2],
            meter_db: [-METER_DB; 2],
            hold_db: [-METER_DB; 2],
            hold_t: [0.0; 2],
            buf: vec![0.0; FFT_N],
            side_buf: vec![0.0; FFT_N],
            fx: FxLive::default(),
            trace: vec![0.0; SCOPE_PTS],
            spec_target: vec![0.0; SPEC_COLS],
            spec: vec![0.0; SPEC_COLS],
            spec_peak: vec![0.0; SPEC_COLS],
            fft: Fft::new(FFT_N),
            re: vec![0.0; FFT_N],
            im: vec![0.0; FFT_N],
            window,
            next_fft: 0.0,
            spectrum: false,
            spec_mix: 0.0,
            last_t: None,
            reduced_motion: false,
        }
    }

    /// Nothing connected (tests, previews): a default record, no scope.
    pub fn idle() -> Self {
        Self::new(Arc::new(Viz::default()), None)
    }

    /// Once per frame: copy the record, read the ring into the scope (and the spectrum at 30 Hz),
    /// run the meter ballistics. `now` is egui time in seconds.
    fn update(&mut self, now: f64, osc_pitch: f32) {
        let dt = self.last_t.map_or(0.0, |t| (now - t).clamp(0.0, 0.05)) as f32;
        self.last_t = Some(now);
        self.viz.read(&mut self.frame);
        let sr = self.frame.global(gf::SR).max(1000.0);

        // Scope: two cycles of the newest voice's Osc A pitch, triggered on a rising zero.
        // Mid and side are read ending at the same sample, so the Dimension display's L / R line up.
        let side = self.viz.side();
        let end = self.ring.as_ref().map(|r| match side.written() {
            0 => r.written(),
            w => r.written().min(w),
        });
        let have = matches!((&self.ring, end), (Some(r), Some(e)) if r.latest_ending(&mut self.buf, e));
        let have_side = have && end.is_some_and(|e| side.latest_ending(&mut self.side_buf, e));
        if !have {
            self.buf.fill(0.0);
        }
        if !have_side {
            self.side_buf.fill(0.0);
        }
        let span = match self.frame.newest() {
            Some(v) => {
                let note = self.frame.voice(v, vf::NOTE) + osc_pitch;
                let f0 = 440.0 * 2f32.powf((note - 69.0) / 12.0);
                ((2.0 * sr / f0.max(1.0)) as usize).clamp(64, FFT_N / 2)
            }
            None => FFT_N / 2,
        };
        let search = &self.buf[..FFT_N - span];
        let start = search.windows(2).rposition(|p| p[0] < 0.0 && p[1] >= 0.0).unwrap_or(FFT_N - span);
        let w = &self.buf[start..start + span];
        let peak = w.iter().fold(0.35f32, |m, s| m.max(s.abs()));
        self.fx.set_stereo(w, &self.side_buf[start..start + span]);
        self.fx.update(&self.frame, now, dt);
        for (i, t) in self.trace.iter_mut().enumerate() {
            let x = i as f32 / (SCOPE_PTS - 1) as f32 * (span - 1) as f32;
            let (k, f) = (x as usize, x.fract());
            let b = w[(k + 1).min(span - 1)];
            *t = (w[k] + (b - w[k]) * f) / peak;
        }

        // Spectrum: 2048-point Hann FFT at 30 Hz, 120 log columns 30 Hz .. 20 kHz over 66 dB.
        if now >= self.next_fft {
            self.next_fft = now + 1.0 / 30.0;
            for i in 0..FFT_N {
                self.re[i] = self.buf[i] as f64 * self.window[i];
                self.im[i] = 0.0;
            }
            self.fft.run(&mut self.re, &mut self.im, false);
            let norm = 2.0 / (FFT_N as f64 * 0.5);
            let bin = |f: f32| (f / sr * FFT_N as f32).clamp(1.0, (FFT_N / 2 - 1) as f32);
            for (c, out) in self.spec_target.iter_mut().enumerate() {
                let (f0, f1) = (30.0 * (20000.0f32 / 30.0).powf(c as f32 / SPEC_COLS as f32), 30.0 * (20000.0f32 / 30.0).powf((c + 1) as f32 / SPEC_COLS as f32));
                let (b0, b1) = (bin(f0) as usize, (bin(f1) as usize).max(bin(f0) as usize));
                let mag = (b0..=b1).map(|b| (self.re[b] * self.re[b] + self.im[b] * self.im[b]).sqrt() * norm).fold(0.0, f64::max);
                *out = ((20.0 * mag.max(1e-9).log10() as f32 + SPEC_DB) / SPEC_DB).clamp(0.0, 1.0);
            }
        }
        // Rise instant, fall 0.9 of the scale per s; peak hold falls 0.22 per s.
        for i in 0..SPEC_COLS {
            self.spec[i] = self.spec_target[i].max(self.spec[i] - 0.9 * dt);
            self.spec_peak[i] = self.spec[i].max(self.spec_peak[i] - 0.22 * dt);
        }
        let target = if self.spectrum { 1.0 } else { 0.0 };
        self.spec_mix = if self.reduced_motion { target } else { target + (self.spec_mix - target) * (-dt / 0.06).exp() };

        // Meter: rise instant, fall 24 dB/s; hold 1.1 s then 0.6 of the scale per s.
        for ch in 0..2 {
            let db = 20.0 * self.peaks[ch].max(1e-6).log10();
            self.meter_db[ch] = db.max(self.meter_db[ch] - 24.0 * dt).max(-METER_DB);
            if db >= self.hold_db[ch] {
                self.hold_db[ch] = db;
                self.hold_t[ch] = now;
            } else if now - self.hold_t[ch] > 1.1 {
                self.hold_db[ch] = (self.hold_db[ch] - 0.6 * METER_DB * dt).max(self.meter_db[ch]);
            }
        }
        self.peaks = [0.0; 2];
    }

    /// Hero frames of `table`: 32 frames x 257 points at a common gain that puts the table's RMS
    /// at 0.7 of the frame height (peaks then clip at 1.25 in the drawing, as in the prototype).
    fn hero_frames(&mut self, tables: &Tables, table: usize) {
        let key = table as u64;
        if self.hero_key == Some(key) || !tables.is_ready() {
            return;
        }
        self.hero_key = Some(key);
        let t = Table::from_index(table);
        for g in 0..GRAINS.min(FRAMES) {
            let grain = tables.grain(t, g);
            for i in 0..POINTS {
                self.hero_frames[g * POINTS + i] = grain[(i * (N / (POINTS - 1))) % N];
            }
        }
        let rms = (self.hero_frames.iter().map(|v| v * v).sum::<f32>() / self.hero_frames.len() as f32).sqrt().max(1e-6);
        self.hero_frames.iter_mut().for_each(|v| *v *= 0.7 / rms);
    }

    /// Ring decorations for this frame: every [`MOD_IDS`] knob whose live value moved off its base.
    fn knob_live(&self, c: &dyn Ctl) -> KnobLive {
        let mut kl = KnobLive { filt_env: [c.get01("filta_env") > 0.5, c.get01("filtb_env") > 0.5], ..Default::default() };
        for (k, id) in MOD_IDS.iter().enumerate() {
            kl.ranges[k] = ring_ranges(c, id);
        }
        if self.frame.newest().is_none() {
            return kl;
        }
        let env_on = c.get01("fenv_amt") > 0.004;
        let oscb_on = c.get01("oscb_on") > 0.5;
        for (k, id) in MOD_IDS.iter().enumerate() {
            let Some(v) = self.frame.live01(id) else { continue };
            if (v - c.get01(id)).abs() <= 0.004 || (id.starts_with("oscb") && !oscb_on) {
                continue;
            }
            let col = match *id {
                "osca_index" | "oscb_index" | "osca_shift" | "oscb_shift" => SRC_A,
                "filta_freq" if kl.filt_env[0] && env_on => SRC_E,
                "filtb_freq" if kl.filt_env[1] && env_on => SRC_E,
                "fenv_amt" => SRC_E,
                _ => SRC_B,
            };
            kl.live[k] = Some((v, col));
        }
        kl
    }
}

/// Modulation depth ranges for one ring target (MOTION.md 5.2), from the knob's base value and
/// its sources' amounts: bipolar waves swing both ways around the base, unipolar ones (Decay,
/// Attack, the envelope) run from the base one way. Spans are computed in the param's own units
/// (Index steps, semitones, Hz in octaves) and mapped back through its range.
pub fn ring_ranges(c: &dyn Ctl, id: &str) -> [Option<RingRange>; 2] {
    let hits = |tid: &str, i: usize| matches!(enum_idx::<Target>(c, tid), 2) || enum_idx::<Target>(c, tid) == i;
    let uni = |curve: &str| matches!(Curve::from_index(enum_idx::<Curve>(c, curve)), Curve::Decay | Curve::Attack);
    let p = |id: &str| c.plain(id, c.get01(id));
    let osc = usize::from(id.starts_with("oscb") || id.starts_with("filtb"));
    let span = |base: f32, delta: f32, uni: bool| if uni { (base.min(base + delta), base.max(base + delta)) } else { (base - delta.abs(), base + delta.abs()) };
    let rg = |lo: f32, hi: f32, col: Color32| Some(RingRange { lo: c.norm(id, lo.min(hi)).clamp(0.0, 1.0), hi: c.norm(id, lo.max(hi)).clamp(0.0, 1.0), color: col });
    let mut out = [None; 2];
    let osc_live = osc == 0 || c.get01("oscb_on") > 0.5;
    match id {
        "osca_index" | "oscb_index" | "osca_shift" | "oscb_shift" if osc_live && hits("moda_target", osc) => {
            let amount = if id.ends_with("index") { 127.0 * p("moda_index") } else { 24.0 * p("moda_shift") };
            if amount.abs() > 0.01 {
                let (lo, hi) = span(p(id), amount, uni("moda_curve"));
                out[0] = rg(lo, hi, SRC_A);
            }
        }
        "osca_motion" | "oscb_motion" | "osca_vol" | "oscb_vol" if osc_live && hits("modb_target", osc) => {
            let (amount, base) = if id.ends_with("motion") { (p("modb_motion"), p(id)) } else { (p(id) * p("modb_vol"), p(id)) };
            if amount.abs() > 0.004 {
                let (lo, hi) = span(base, amount, uni("modb_curve"));
                out[0] = rg(lo.max(if id.ends_with("vol") { 0.0 } else { f32::MIN }), hi, SRC_B);
            }
        }
        "filta_freq" | "filtb_freq" if enum_idx::<Routing>(c, "routing") != Routing::variants().len() - 1 || osc == 0 => {
            let base = p(id);
            let amt = p("fenv_amt");
            let env_id = if osc == 0 { "filta_env" } else { "filtb_env" };
            if c.get01(env_id) > 0.5 && amt > 0.004 {
                let oct = 8.0 * amt * if c.get01("fenv_inv") > 0.5 { -1.0 } else { 1.0 };
                out[0] = rg(base, base * 2f32.powf(oct), SRC_E);
            }
            let cut = 5.0 * p("modb_cutoff");
            if hits("modb_target", osc) && cut.abs() > 0.02 {
                let (a, b) = if uni("modb_curve") { (base, base * 2f32.powf(cut)) } else { (base * 2f32.powf(-cut.abs()), base * 2f32.powf(cut.abs())) };
                out[1] = rg(a, b, SRC_B);
            }
        }
        "moda_pitch" | "moda_index" | "moda_shift" => {
            let (base, m) = (p(id), p("modb_moda"));
            if m.abs() > 0.004 && base.abs() > 0.004 {
                let (lo, hi) = span(base, base * m, uni("modb_curve"));
                out[0] = rg(lo, hi, SRC_B);
            }
        }
        "fenv_amt" if c.get01("filta_env") > 0.5 || c.get01("filtb_env") > 0.5 => {
            let base = p(id);
            if base > 0.004 {
                out[0] = rg(0.0, base, SRC_E);
            }
        }
        _ => {}
    }
    out
}

/// "F1", "A#2" for a MIDI note.
fn note_name(n: u8) -> String {
    const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    format!("{}{}", NAMES[n as usize % 12], n as i32 / 12 - 1)
}

// ---- layout ----------------------------------------------------------------------------------

/// Where the SYNTH / FX page switch is (MOTION.md 4): the page being drawn and the transition
/// phase. Lives in egui memory so every `draw` caller (plugin, previews, tests) gets it.
#[derive(Clone, Copy)]
struct PageState {
    target: bool,
    shown: bool,
    /// Start of the running transition (egui time), negative when idle.
    t0: f64,
}

fn page_state(ctx: &egui::Context, now: f64, target: bool) -> (bool, Option<rr_ui::motion::PageTrans>) {
    use rr_ui::motion::{PageTrans, PAGE_IN, PAGE_OUT};
    let id = Id::new("gts-page-state");
    let mut s = ctx.data(|d| d.get_temp::<PageState>(id)).unwrap_or(PageState { target, shown: target, t0: -1.0 });
    if s.t0 > now {
        // egui time restarted (a reopened editor): settle.
        s = PageState { target, shown: target, t0: -1.0 };
    }
    if s.target != target {
        s.target = target;
        if rr_ui::motion::reduced_motion(ctx) {
            s.shown = target;
            s.t0 = -1.0;
        } else {
            s.t0 = now;
        }
    }
    let mut trans = None;
    if s.t0 >= 0.0 {
        let t = (now - s.t0) as f32;
        if t < PAGE_OUT {
            trans = Some(PageTrans { out: true, t });
        } else if t < PAGE_OUT + PAGE_IN {
            s.shown = s.target;
            trans = Some(PageTrans { out: false, t: t - PAGE_OUT });
        } else {
            s.shown = s.target;
            s.t0 = -1.0;
        }
    }
    ctx.data_mut(|d| d.insert_temp(id, s));
    (s.shown, trans)
}

/// Draws the whole panel into `ui` (the full [`WINDOW`]). Returns how far any group's content ran
/// past its fixed size (zero when everything fits; the layout test checks it).
pub fn draw(ui: &mut Ui, c: &dyn Ctl, tables: &Tables, live: &mut LiveView, bv: &mut BusView) -> Vec2 {
    let ctx = ui.ctx().clone();
    rr_ui::motion::set_reduced_motion(&ctx, bv.reduced_motion);
    live.reduced_motion = bv.reduced_motion;
    let full = rr_ui::panel::rack_frame(ui);
    let shadows = rr_ui::fx::begin_shadows(ui);
    let o = full.min.to_vec2();
    let at = |x: f32, y: f32, w: f32, h: f32| Rect::from_min_size(pos2(x, y) + o, vec2(w, h));
    let _ = take_group_overflow(&ctx);
    set_always_values(&ctx, bv.show_values);
    let now = ui.input(|i| i.time);
    let osc_pitch = 12.0 * (c.get01("osca_oct") * 8.0 - 4.0).round() + (c.get01("osca_semi") * 12.0).round();
    live.update(now, osc_pitch);
    let mut kl = live.knob_live(c);
    ui.data_mut(|d| {
        kl.chip = d.remove_temp::<Option<usize>>(chip_id()).flatten();
        d.insert_temp(knob_live_id(), kl);
        d.remove_temp::<Hover>(hover_id());
    });

    header(ui, c, live, bv, &at);
    let (show_fx, trans) = page_state(&ctx, now, bv.fx_page);
    rr_ui::motion::set_page_trans(&ctx, trans);
    let body = at(44.0, 80.0, 912.0, 704.0);
    let mut bui = ui.new_child(UiBuilder::new().max_rect(body).layout(egui::Layout::top_down(egui::Align::Min)));
    let mut over = Vec2::ZERO;
    if show_fx {
        fx_body(&mut bui, c, live, bv, &at);
        over = (bui.min_rect().max - body.max).max(Vec2::ZERO);
    } else {
        synth(&mut bui, c, tables, live, &at);
    }
    rr_ui::motion::set_page_trans(&ctx, None);
    rr_ui::fx::end_shadows(ui, shadows);
    help_bar(ui, at(44.0, 792.0, 912.0, 28.0), bv);
    preset_list(ui, bv, at(208.0, 16.0, 232.0, 56.0));
    patch_popover(ui, c, bv, at(508.0, 48.0, 72.0, 24.0));
    ui.allocate_rect(full, Sense::hover());
    over.max(take_group_overflow(&ctx))
}

/// A child ui at an absolute rect.
fn place<R>(ui: &mut Ui, r: Rect, add: impl FnOnce(&mut Ui) -> R) -> R {
    let mut child = ui.new_child(UiBuilder::new().max_rect(r).layout(egui::Layout::left_to_right(egui::Align::Center)));
    add(&mut child)
}

fn header(ui: &mut Ui, c: &dyn Ctl, live: &mut LiveView, bv: &mut BusView, at: &dyn Fn(f32, f32, f32, f32) -> Rect) {
    let p = ui.painter().clone();
    let note = live.frame.held_note().map(note_name);
    rr_ui::panel::nameplate_v2(&p, at(44.0, 16.0, 156.0, 56.0), "GTS", "Graintable Synth", note.as_deref());
    preset_bar(ui, at(208.0, 16.0, 232.0, 56.0), bv);
    page_switch(ui, at(448.0, 16.0, 132.0, 26.0), bv);
    place(ui, at(448.0, 48.0, 56.0, 24.0), |ui| {
        let sel = QUALITY.iter().position(|&q| q == bv.oversampling).unwrap_or(1);
        if let Some(i) = combo(ui, "", sel, &["1x", "2x", "4x"]) {
            bv.oversampling = QUALITY[i];
        }
    });
    patch_chip(ui, at(508.0, 48.0, 72.0, 24.0), bv);

    // Scope / spectrum (click flips), meter, output plate.
    let sr = at(588.0, 16.0, 184.0, 56.0);
    let resp = ui.interact(sr, Id::new("gts-scope"), Sense::click());
    if resp.clicked() {
        live.spectrum = !live.spectrum;
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        set_hover(ui, "OUTPUT", if live.spectrum { "spectrum" } else { "scope" }, None, "The output after the limiter. Click to flip between the scope and the spectrum.");
    }
    rr_ui::scope::scope_display(&p, sr, &live.trace, &live.spec, &live.spec_peak, live.spec_mix);
    let lv = |db: f32| ((db + METER_DB) / METER_DB).clamp(0.0, 1.0);
    rr_ui::scope::level_meter(&p, at(780.0, 16.0, 14.0, 56.0), [lv(live.meter_db[0]), lv(live.meter_db[1])], [lv(live.hold_db[0]), lv(live.hold_db[1])]);
    let out = at(802.0, 16.0, 154.0, 56.0);
    p.rect_filled(out.translate(vec2(0.0, 1.0)), GROUP_R, T.hi());
    p.rect_filled(out, GROUP_R, T.well);
    place(ui, out.shrink2(vec2(7.0, 4.0)), |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let s = KnobStyle::default().size(32.0).compact().cell(70.0);
        kn(ui, c, "volume", "Volume", s);
        kn(ui, c, "spread", "Spread", s);
    });
}

fn set_hover(ui: &Ui, name: &str, value: &str, live: Option<(String, Color32)>, line: &'static str) {
    let live = live.map(|(t, c)| (t, c, glyph_of(c)));
    ui.data_mut(|d| d.insert_temp(hover_id(), Hover { name: name.into(), value: value.into(), live, line: Some(line), ..Default::default() }));
}

/// Preset bar: prev / next arrows (wrap), category over the name (a dot while edited); clicking
/// the name opens the list. Loading goes through `bv.preset_pick` (the host applies it).
fn preset_bar(ui: &mut Ui, r: Rect, bv: &mut BusView) {
    let f = preset::factory();
    let n = f.len() + 1;
    let p = ui.painter().clone();
    rr_ui::paint::inset_box(&p, r, false);
    let left = Rect::from_min_size(r.min, vec2(28.0, r.height()));
    let right = Rect::from_min_size(pos2(r.right() - 28.0, r.top()), vec2(28.0, r.height()));
    let mid = Rect::from_min_max(pos2(left.right(), r.top()), pos2(right.left(), r.bottom()));
    let cur = bv.preset;
    for (rect, dir, glyph) in [(left, n - 1, "\u{2039}"), (right, 1, "\u{203a}")] {
        let resp = ui.interact(rect, Id::new(("gts-preset-arrow", dir)), Sense::click());
        if resp.hovered() {
            p.rect_filled(rect.shrink(3.0), 3.0, Color32::WHITE.gamma_multiply(0.04));
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        p.text(rect.center(), Align2::CENTER_CENTER, glyph, font::label(22.0), if resp.hovered() { T.text } else { T.muted });
        if resp.clicked() {
            bv.preset_pick = Some(cur.map_or(if dir == 1 { 0 } else { n - 1 }, |i| (i + dir) % n));
        }
    }
    let (cat, name) = match cur {
        Some(0) => ("Init".to_string(), "Init".to_string()),
        Some(i) => f.get(i - 1).map_or(("".into(), "".into()), |p| (p.category.replace('/', " / "), p.name.clone())),
        None => ("Preset".to_string(), "Custom".to_string()),
    };
    let resp = ui.interact(mid, Id::new("gts-preset-name"), Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        set_hover(ui, "PRESET", &name, None, "Click for the preset list, or use the arrows to step through it. A dot marks edits.");
    }
    rr_ui::paint::tracked(&p, pos2(mid.center().x, r.top() + 16.0), Align2::CENTER_CENTER, &cat.to_uppercase(), font::label(11.0), T.muted, 1.0);
    let nr = p.text(pos2(mid.center().x, r.top() + 36.0), Align2::CENTER_CENTER, &name, font::strong(16.0), T.text);
    let dirty = rr_ui::motion::tween(ui.ctx(), Id::new("gts-preset-dirty"), bv.preset_dirty, 0.18);
    if dirty > 0.0 {
        p.circle_filled(pos2(nr.right() + 8.0, nr.center().y), 3.0, T.accent.gamma_multiply(dirty));
    }
    if resp.clicked() {
        bv.preset_open = !bv.preset_open;
    }
    // Eased arrow-less state change: the dot fades in 180 ms (above), the list in 150 ms (below).
    let _ = n;
}

/// The preset list as a layer under the bar (MOTION.md 5.8): "Init" first, then the factory
/// presets under category headers, the loaded one marked, a hover highlight; fades in over 150 ms
/// with a 6 px drop and out over 90 ms. Esc, a pick or a click elsewhere closes it.
fn preset_list(ui: &mut Ui, bv: &mut BusView, bar: Rect) {
    let ctx = ui.ctx().clone();
    let k = rr_ui::motion::tween(&ctx, Id::new("gts-preset-list-a"), bv.preset_open, if bv.preset_open { 0.15 } else { 0.09 });
    if k <= 0.0 {
        return;
    }
    let f = preset::factory();
    let rows = 1 + f.len();
    let headers = f.iter().enumerate().filter(|(i, p)| *i == 0 || f[*i - 1].category != p.category).count();
    let size = vec2(bar.width(), 8.0 + rows as f32 * CONTROL_H + headers as f32 * 22.0);
    let pos = pos2(bar.left(), bar.bottom() + 6.0 - 6.0 * (1.0 - k));
    let cur = bv.preset;
    let open = bv.preset_open;
    let area = egui::Area::new(Id::new("gts-preset-pop")).order(egui::Order::Foreground).fixed_pos(pos).show(&ctx, |ui| {
        ui.multiply_opacity(k);
        let r = Rect::from_min_size(pos, size);
        ui.painter().rect_filled(r.translate(vec2(0.0, 10.0)).expand(4.0), 10.0, Color32::BLACK.gamma_multiply(0.35));
        ui.painter().rect_filled(r.translate(vec2(0.0, 2.0)), 6.0, Color32::BLACK.gamma_multiply(0.5));
        ui.painter().rect_filled(r, 6.0, T.inset);
        ui.painter().rect_stroke(r, 6.0, rr_ui::paint::hair(ui.painter(), T.lo()), StrokeKind::Inside);
        let mut y = r.top() + 4.0;
        let row = |ui: &mut Ui, y: &mut f32, i: usize, name: &str, cat: &str| {
            let rect = Rect::from_min_size(pos2(r.left() + 4.0, *y), vec2(r.width() - 8.0, CONTROL_H));
            *y += CONTROL_H;
            let resp = ui.interact(rect, Id::new(("gts-preset-row", i)), Sense::click());
            let hot = open && resp.hovered();
            if hot {
                ui.painter().rect_filled(rect, 3.0, T.a(0.16));
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            let on = Some(i) == cur;
            if on {
                ui.painter().circle_filled(pos2(rect.left() + 9.0, rect.center().y), 2.5, T.accent);
            }
            ui.painter().text(pos2(rect.left() + 20.0, rect.center().y), Align2::LEFT_CENTER, name, font::label(13.0), if on { T.accent } else { T.text });
            if !cat.is_empty() {
                rr_ui::paint::tracked(ui.painter(), pos2(rect.right() - 8.0, rect.center().y), Align2::RIGHT_CENTER, &cat.to_uppercase(), font::label(11.0), T.muted, 0.5);
            }
            open && resp.clicked()
        };
        let mut pick = None;
        if row(ui, &mut y, 0, "Init", "Reset") {
            pick = Some(0);
        }
        for (j, p) in f.iter().enumerate() {
            if j == 0 || f[j - 1].category != p.category {
                rr_ui::paint::tracked(ui.painter(), pos2(r.left() + 12.0, y + 14.0), Align2::LEFT_CENTER, &p.category.replace('/', " / ").to_uppercase(), font::strong(11.0), T.title, 1.2);
                y += 22.0;
            }
            if row(ui, &mut y, j + 1, &p.name, "") {
                pick = Some(j + 1);
            }
        }
        pick
    });
    if let Some(i) = area.inner {
        bv.preset_pick = Some(i);
        bv.preset_open = false;
    }
    if bv.preset_open {
        let (esc, click) = ctx.input(|i| (i.key_pressed(egui::Key::Escape), i.pointer.primary_pressed().then(|| i.pointer.interact_pos()).flatten()));
        let outside = click.is_some_and(|p| !area.response.rect.contains(p) && !bar.contains(p));
        if esc || outside {
            bv.preset_open = false;
        }
    }
}

/// SYNTH / FX segmented control with a sliding lit segment (180 ms).
fn page_switch(ui: &mut Ui, r: Rect, bv: &mut BusView) {
    let p = ui.painter().clone();
    rr_ui::paint::inset_box(&p, r, false);
    let w = (r.width() - 4.0) / 2.0;
    let x = rr_ui::motion::tween(ui.ctx(), Id::new("gts-page-lit"), bv.fx_page, 0.18);
    let lit = Rect::from_min_size(pos2(r.left() + 2.0 + x * w, r.top() + 2.0), vec2(w, r.height() - 4.0));
    p.rect_filled(lit, 3.0, T.a(0.20));
    p.rect_stroke(lit, 3.0, rr_ui::paint::hair(&p, T.a(0.55)), StrokeKind::Inside);
    for (i, name) in ["Synth", "FX"].into_iter().enumerate() {
        let seg = Rect::from_min_size(pos2(r.left() + 2.0 + i as f32 * w, r.top() + 2.0), vec2(w, r.height() - 4.0));
        let resp = ui.interact(seg, Id::new(("gts-page", i)), Sense::click());
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if resp.clicked() {
            bv.fx_page = i == 1;
        }
        let on = (i == 1) == bv.fx_page;
        let col = if on { T.accent } else if resp.hovered() { T.text } else { T.muted };
        rr_ui::paint::caps(&p, seg.center(), Align2::CENTER_CENTER, name, 11.0, col);
    }
}

/// PATCH chip: caps label and the three slot activity LEDs; click toggles the popover.
fn patch_chip(ui: &mut Ui, r: Rect, bv: &mut BusView) {
    let resp = ui.interact(r, Id::new("gts-patch-chip"), Sense::click());
    let p = ui.painter();
    rr_ui::paint::inset_box(p, r, resp.hovered() || bv.patch_open);
    if bv.patch_open {
        p.rect_stroke(r, 4.0, rr_ui::paint::hair(&p, T.a(0.55)), StrokeKind::Inside);
    }
    rr_ui::paint::caps(p, pos2(r.left() + 9.0, r.center().y), Align2::LEFT_CENTER, "Patch", 11.0, if bv.patch_open { T.accent } else { T.muted });
    for i in 0..3 {
        let c = pos2(r.right() - 22.0 + i as f32 * 6.0, r.center().y);
        let a = if bv.bound[i].is_empty() { 0.0 } else { bv.activity[i].clamp(0.0, 1.0) };
        p.rect_filled(Rect::from_center_size(c, vec2(3.0, 9.0)), 1.0, T.led_off);
        if a > 0.02 {
            p.rect_filled(Rect::from_center_size(c, vec2(3.0, 9.0)), 1.0, T.accent.gamma_multiply(0.3 + 0.7 * a));
        }
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        set_hover(ui, "PATCH", "RR-BUS inputs", None, "Opens the three RR-BUS input slots: pick a source from another Riddim Rack device and a target.");
    }
    if resp.clicked() {
        bv.patch_open = !bv.patch_open;
    }
}

/// The PATCH strip as a layer under its chip (150 ms fade + 6 px drop). Esc or a click elsewhere
/// (while no list of its own is open) closes it.
fn patch_popover(ui: &mut Ui, c: &dyn Ctl, bv: &mut BusView, chip: Rect) {
    let ctx = ui.ctx().clone();
    let k = rr_ui::motion::tween(&ctx, Id::new("gts-patch-pop-a"), bv.patch_open, if bv.patch_open { 0.15 } else { 0.09 });
    if k <= 0.0 {
        return;
    }
    let any_popup = ctx.memory(|m| m.any_popup_open());
    let pos = pos2(chip.left(), chip.bottom() + 8.0 - 6.0 * (1.0 - k));
    let size = vec2(312.0, PATCH_H);
    let area = egui::Area::new(Id::new("gts-patch-pop")).order(egui::Order::Foreground).fixed_pos(pos).show(&ctx, |ui| {
        ui.multiply_opacity(k);
        let r = Rect::from_min_size(pos, size);
        ui.painter().rect_filled(r.translate(vec2(0.0, 8.0)).expand(4.0), 10.0, Color32::BLACK.gamma_multiply(0.35));
        ui.painter().rect_filled(r.translate(vec2(0.0, 2.0)), GROUP_R, Color32::BLACK.gamma_multiply(0.5));
        ui.allocate_ui(size, |ui| {
            ui.set_width(size.x);
            if bv.patch_open {
                patch(ui, c, bv);
            } else {
                ui.disable();
                patch(ui, c, bv);
            }
        });
    });
    if bv.patch_open {
        let (esc, click) = ctx.input(|i| (i.key_pressed(egui::Key::Escape), i.pointer.primary_pressed().then(|| i.pointer.interact_pos()).flatten()));
        let outside = click.is_some_and(|p| !area.response.rect.contains(p) && !chip.contains(p));
        if esc || (outside && !any_popup) {
            bv.patch_open = false;
        }
    }
}

/// Help bar (MOTION.md 5.10): the hovered control's name, value, live value (in its source colour,
/// with the source letter) and one line of help; with nothing hovered, the gesture hint and the two
/// editor settings ("Always show values", "Reduced motion") on the right.
fn help_bar(ui: &mut Ui, r: Rect, bv: &mut BusView) {
    let hov = ui.data(|d| d.get_temp::<Hover>(hover_id()));
    let p = ui.painter().clone();
    p.rect_filled(r, 4.0, Color32::BLACK.gamma_multiply(0.5));
    let y = r.center().y;
    let mut x = r.left() + 12.0;
    match hov {
        Some(h) => {
            x = rr_ui::paint::tracked(&p, pos2(x, y), Align2::LEFT_CENTER, &h.name, font::label(11.0), T.muted, 1.0).right() + 12.0;
            x = p.text(pos2(x, y), Align2::LEFT_CENTER, &h.value, font::mono(12.0), T.accent).right() + 12.0;
            if let Some((v, col, glyph)) = h.live {
                x = p.text(pos2(x, y), Align2::LEFT_CENTER, format!("\u{203a} {glyph} {v}"), font::mono(12.0), col).right() + 14.0;
            }
            if let Some(line) = h.line.or_else(|| crate::help::help(&h.id)) {
                let clip = Rect::from_min_max(pos2(x, r.top()), pos2(r.right() - 10.0, r.bottom()));
                p.with_clip_rect(clip.intersect(p.clip_rect())).text(pos2(x, y), Align2::LEFT_CENTER, line, font::label(12.0), T.muted);
            }
        }
        None => {
            x = rr_ui::paint::tracked(&p, pos2(x, y), Align2::LEFT_CENTER, "HOVER A CONTROL", font::label(11.0), T.muted, 1.0).right() + 12.0;
            p.text(pos2(x, y), Align2::LEFT_CENTER, "Drag to set, Shift for fine, double-click to reset, wheel or arrows to nudge.", font::label(12.0), T.disp_caption);
            place(ui, Rect::from_min_max(pos2(r.right() - 236.0, r.top() + 2.0), pos2(r.right() - 2.0, r.bottom() - 2.0)), |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    if let Some(on) = led_toggle(ui, "Reduced motion", bv.reduced_motion) {
                        bv.reduced_motion = on;
                    }
                    if let Some(on) = led_toggle(ui, "Values", bv.show_values) {
                        bv.show_values = on;
                    }
                });
            });
        }
    }
}

// ---- SYNTH page --------------------------------------------------------------------------------

/// Compact (v2) knob styles: L 44, M 40, S 32 (MOTION.md 2).
fn kl(w: f32) -> KnobStyle {
    KnobStyle::default().size(KNOB_D_LG).compact().cell(w)
}
fn km(w: f32) -> KnobStyle {
    KnobStyle::default().compact().cell(w)
}
fn ks(w: f32) -> KnobStyle {
    KnobStyle::default().size(32.0).compact().cell(w)
}

/// Voices for a playhead: (voice, alpha) with the newest at full and the rest at half while active.
fn playhead_voices(f: &VizFrame) -> Vec<(usize, f32)> {
    let newest = f.newest();
    (0..crate::dsp::MAX_VOICES).filter(|&v| f.voice(v, vf::ACTIVE) > 0.5).map(|v| (v, if Some(v) == newest { 1.0 } else { 0.5 })).collect()
}

fn playhead(p: &egui::Painter, inner: Rect, at: Pos2, col: Color32, alpha: f32) {
    let px = 1.0 / p.ctx().pixels_per_point();
    let x = (at.x / px).round() * px;
    p.vline(x, inner.y_range(), Stroke::new(px, Color32::WHITE.gamma_multiply(0.5 * alpha)));
    rr_ui::fx::glow(p, at, 16.0, col, 0.5 * alpha);
    p.circle_filled(at, 2.4, Color32::WHITE.gamma_multiply(alpha));
}

/// Mod A / Mod B shape display (MOTION.md 5.3): the curve in the source colour (one cycle for a
/// one-shot, else two), one playhead per active voice from its LFO phase.
fn lfo_display(ui: &mut Ui, size: Vec2, c: &dyn Ctl, f: &VizFrame, which: usize) {
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let p = ui.painter();
    rr_ui::scope::small_glass(p, rect);
    let pre = if which == 0 { "moda" } else { "modb" };
    let col = if which == 0 { SRC_A } else { SRC_B };
    let curve = Curve::from_index(enum_idx::<Curve>(c, &format!("{pre}_curve")));
    let oneshot = c.get01(&format!("{pre}_oneshot")) > 0.5;
    let uni = matches!(curve, Curve::Decay | Curve::Attack);
    let inner = rect.shrink(8.0);
    let cycles = if oneshot { 1.0 } else { 2.0 };
    let y_of = |v: f32| if uni { inner.bottom() - v * inner.height() } else { inner.center().y - v * inner.height() / 2.0 };
    let px = 1.0 / p.ctx().pixels_per_point();
    let base = if uni { inner.bottom() } else { inner.center().y };
    p.hline(inner.x_range(), (base / px).round() * px, Stroke::new(px, Color32::WHITE.gamma_multiply(0.12)));
    let pts: Vec<Pos2> = (0..=96)
        .map(|i| {
            let x = i as f32 / 96.0;
            pos2(inner.left() + x * inner.width(), y_of(Lfo::curve_at(curve, ((x * cycles) % 1.0 + if x >= 1.0 { 1.0 } else { 0.0 }).min(1.0) as f64)))
        })
        .collect();
    rr_ui::scope::glow_line(p, pts, 1.5, col);
    for (v, a) in playhead_voices(f) {
        let ph = f.voice(v, if which == 0 { vf::PH_A } else { vf::PH_B }).max(0.0);
        let x = if oneshot { ph.min(1.0) } else { (ph % cycles) / cycles };
        let y = y_of(f.voice(v, if which == 0 { vf::OUT_A } else { vf::OUT_B }));
        playhead(p, inner, pos2(inner.left() + x * inner.width(), y), col, a);
    }
}

/// ADSR display (MOTION.md 5.3): segment widths from sqrt(time + 0.02) (normalized knob values
/// stand in for time), a notional sustain of half the width, one playhead per active voice.
fn adsr_display(ui: &mut Ui, size: Vec2, c: &dyn Ctl, pre: &str, f: &VizFrame, stage_k: usize, level_k: usize, col: Color32) {
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let p = ui.painter();
    rr_ui::scope::small_glass(p, rect);
    let inner = rect.shrink(8.0);
    let g = |id: &str| (c.get01(&format!("{pre}_{id}")) + 0.02).sqrt();
    let (wa, wd, wr, ws) = (g("att"), g("dec"), g("rel"), 0.5);
    let sum = wa + wd + wr + ws;
    let (xa, xd, xs) = (wa / sum, (wa + wd) / sum, (wa + wd + ws) / sum);
    let sus = c.get01(&format!("{pre}_sus"));
    let pt = |x: f32, y: f32| pos2(inner.left() + x * inner.width(), inner.bottom() - y * inner.height());
    let mut pts = vec![];
    for i in 0..=12 {
        let t = i as f32 / 12.0;
        pts.push(pt(xa * t, 1.0 - (1.0 - t).powi(2)));
    }
    for i in 1..=12 {
        let t = i as f32 / 12.0;
        pts.push(pt(xa + (xd - xa) * t, sus + (1.0 - sus) * (1.0 - t).powi(3)));
    }
    pts.push(pt(xs, sus));
    for i in 1..=12 {
        let t = i as f32 / 12.0;
        pts.push(pt(xs + (1.0 - xs) * t, sus * (1.0 - t).powi(3)));
    }
    rr_ui::scope::glow_line(p, pts, 1.5, col);
    for (v, a) in playhead_voices(f) {
        let (st, lv) = (f.voice(v, stage_k), f.voice(v, level_k).clamp(0.0, 1.0));
        let x = match st as i32 {
            1 => xa * (1.0 - (1.0 - lv).sqrt()),
            2 => xa + (xd - xa) * (1.0 - ((lv - sus) / (1.0 - sus).max(1e-3)).clamp(0.0, 1.0).cbrt()),
            3 => (xd + xs) / 2.0,
            4 => xs + (1.0 - xs) * (1.0 - (lv / sus.max(0.05)).clamp(0.0, 1.0).cbrt()),
            _ => continue,
        };
        playhead(p, inner, pt(x, lv), col, a);
    }
}

/// Magnitude in dB of filter type `ty` (index into [`FilterType`]: LP12, BP12, Comb+, Comb-, AM)
/// at `f` Hz for cutoff `f0` and resonance `res` (0..1), from the same coefficient laws the DSP uses
/// (`dsp::filter`): the SVF damping `k`, the comb mix / feedback, the AM dry level. Analog-style
/// magnitudes: no warping near Nyquist, good enough for a 78 px display.
pub fn filter_db(ty: usize, f: f32, f0: f32, res: f32) -> f32 {
    let x = f / f0.max(1.0);
    let k = ((std::f32::consts::SQRT_2 + 0.05) * (1.0 - res).powf(1.2) - 0.05).max(0.04);
    let den = ((1.0 - x * x).powi(2) + (k * x).powi(2)).sqrt().max(1e-4);
    let mag = match ty {
        0 => 1.0 / den,
        1 => k.max(1.0) * x / den,
        2 => {
            let m = res * 0.5;
            ((1.0 - m).powi(2) + m * m + 2.0 * (1.0 - m) * m * (std::f32::consts::TAU * x).cos()).max(0.0).sqrt()
        }
        3 => {
            let fb = res * 0.98;
            (1.0 - fb * fb).sqrt() / (1.0 + fb * fb - 2.0 * fb * (std::f32::consts::TAU * x).cos()).max(1e-4).sqrt()
        }
        _ => 1.0 - res,
    };
    20.0 * mag.max(1e-4).log10()
}

/// Filter response display (MOTION.md 5.4): log frequency 20 Hz to 20 kHz across, -30 to +18 dB up.
/// A dashed grey curve for the knob's base cutoff, the live mint curve (and fill) for the live
/// cutoff (envelope, Mod B, wheel, keyboard), a dot on it at the live cutoff (rose while the
/// filter follows the envelope, blue otherwise) and a tick under the axis at the base. Comb curves
/// show their teeth; AM shows its dry centre line with two sideband spikes.
fn filter_display(ui: &mut Ui, size: Vec2, c: &dyn Ctl, f: &VizFrame, pre: &str) {
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let p = ui.painter();
    rr_ui::scope::small_glass(p, rect);
    let inner = rect.shrink2(vec2(6.0, 7.0));
    let ty = enum_idx::<FilterType>(c, &format!("{pre}_type"));
    let res = c.get01(&format!("{pre}_res"));
    let fid = format!("{pre}_freq");
    let base_hz = c.plain(&fid, c.get01(&fid));
    let live_hz = c.plain(&fid, f.live01(&fid).unwrap_or_else(|| c.get01(&fid)));
    let env_on = c.get01(&format!("{pre}_env")) > 0.5 && c.get01("fenv_amt") > 0.004;
    let px = 1.0 / p.ctx().pixels_per_point();
    let snap = |v: f32| (v / px).round() * px;
    let x_of = |hz: f32| inner.left() + inner.width() * ((hz / 20.0).max(1.0).ln() / 1000f32.ln()).clamp(0.0, 1.0);
    let y_of = |db: f32| inner.bottom() - inner.height() * ((db + 30.0) / 48.0).clamp(0.0, 1.0);
    p.hline(inner.x_range(), snap(y_of(0.0)), Stroke::new(px, Color32::WHITE.gamma_multiply(0.12)));
    for hz in [100.0f32, 1000.0, 10000.0] {
        p.vline(snap(x_of(hz)), inner.y_range(), Stroke::new(px, Color32::WHITE.gamma_multiply(0.05)));
    }
    let n = (inner.width() as usize).max(24);
    let comb = matches!(ty, 2 | 3);
    let curve = |f0: f32| -> Vec<Pos2> {
        (0..=n)
            .map(|i| {
                let fr = |t: f32| 20.0 * 1000f32.powf(t);
                let db = if comb {
                    // Mean power across the column, so dense teeth read as a level instead of aliasing.
                    let m: f32 = (0..4).map(|s| 10f32.powf(filter_db(ty, fr((i as f32 - 0.5 + (s as f32 + 0.5) / 4.0) / n as f32), f0, res) / 10.0)).sum::<f32>() / 4.0;
                    10.0 * m.max(1e-6).log10()
                } else {
                    filter_db(ty, fr(i as f32 / n as f32), f0, res)
                };
                pos2(inner.left() + inner.width() * i as f32 / n as f32, y_of(db))
            })
            .collect()
    };
    if (live_hz / base_hz.max(1.0)).ln().abs() > 0.01 && ty != 4 {
        p.add(egui::Shape::dashed_line(&curve(base_hz), Stroke::new(px, Color32::WHITE.gamma_multiply(0.35)), 3.0, 3.0));
    }
    let live_pts = curve(live_hz);
    let mut mesh = egui::epaint::Mesh::default();
    for (i, q) in live_pts.iter().enumerate() {
        mesh.colored_vertex(*q, T.a(0.22));
        mesh.colored_vertex(pos2(q.x, inner.bottom()), Color32::TRANSPARENT);
        if i > 0 {
            let b = (i as u32 - 1) * 2;
            mesh.add_triangle(b, b + 1, b + 2);
            mesh.add_triangle(b + 2, b + 1, b + 3);
        }
    }
    p.add(egui::Shape::mesh(mesh));
    rr_ui::scope::glow_line(p, live_pts, 1.5, T.accent);
    if ty == 4 {
        // Sidebands of a ring modulator sit either side of the carrier.
        for s in [-0.2f32, 0.2] {
            let x = x_of(live_hz * 2f32.powf(s));
            let top = y_of(20.0 * (res * 0.5).max(0.03).log10());
            p.line_segment([pos2(x, inner.bottom()), pos2(x, top)], Stroke::new(1.5_f32, T.accent.gamma_multiply(0.7)));
        }
    }
    let dot_db = if ty == 4 { 20.0 * (1.0 - res).max(0.03).log10() } else { filter_db(ty, live_hz, live_hz, res) };
    let dot = pos2(x_of(live_hz), y_of(dot_db));
    let col = if env_on { SRC_E } else { SRC_B };
    rr_ui::fx::glow(p, dot, 9.0, col, 0.4);
    p.circle_filled(dot, 3.2, Color32::BLACK.gamma_multiply(0.7));
    p.circle_filled(dot, 2.2, col);
    let tx = snap(x_of(base_hz));
    p.vline(tx, (rect.bottom() - 6.0)..=(rect.bottom() - 2.0), Stroke::new(px, T.muted));
}

fn synth(ui: &mut Ui, c: &dyn Ctl, tables: &Tables, live: &mut LiveView, at: &dyn Fn(f32, f32, f32, f32) -> Rect) {
    // Sources strip first: a hovered chip rings its targets on every knob drawn after it.
    let f = live.frame;
    let src = at(44.0, 320.0, 568.0, 20.0);
    let newest = f.newest();
    let act = |k: usize| newest.map_or(0.0, |v| f.voice(v, k).abs());
    let chips = [("A", "Mod A", SRC_A, act(vf::OUT_A), 74.0), ("B", "Mod B", SRC_B, act(vf::OUT_B), 74.0), ("E", "Env", SRC_E, act(vf::FENV), 62.0), ("M", "Macro 1", SRC_M, 0.0, 86.0)];
    let chip = rr_ui::motion::with_page_fx(ui, 320.0, |ui| {
    rr_ui::paint::tracked(ui.painter(), pos2(src.left() + 8.0, src.center().y), Align2::LEFT_CENTER, "SOURCES", font::label(11.0), T.title, 1.4);
    let mut x = src.left() + 76.0;
    let mut chip = None;
    for (i, (g, name, col, a, w)) in chips.into_iter().enumerate() {
        let w = w + 30.0;
        let r = source_chip(ui, Rect::from_min_size(pos2(x, src.top()), vec2(w, 20.0)), g, name, col, a);
        if r.hovered() {
            chip = Some(i);
            let line = [
                "Mod A: sweeps pitch, Index and Shift. Hover to light the knobs it drives.",
                "Mod B: sweeps Motion, volume, cutoff and the Mod A depth. Hover to light its targets.",
                "Filter envelope: opens the Filter A and B cutoff. Hover to light its targets.",
                "Macro 1: display only until the modulation matrix arrives. It has no targets yet.",
            ][i];
            set_hover(ui, &name.to_uppercase(), ["source", "source", "source", "macro"][i], None, line);
        }
        x += w + 8.0;
    }
    chip
    });
    // Read next frame by every knob (the header ones are drawn before the chips).
    ui.data_mut(|d| d.insert_temp(chip_id(), chip));

    // Hero.
    let osc = if live.hero_b { "oscb" } else { "osca" };
    let table = enum_idx::<Table>(c, &format!("{osc}_table"));
    live.hero_frames(tables, table);
    let k = |s: &str| format!("{osc}_{s}");
    let o = live.hero_b as usize;
    // Mod A range on this osc's Index (bipolar curves swing both ways, Decay / Attack one way).
    let hits = match enum_idx::<Target>(c, "moda_target") {
        0 => o == 0,
        1 => o == 1,
        _ => true,
    };
    let amt = if hits { c.get01("moda_index") * 2.0 - 1.0 } else { 0.0 };
    let base = c.get01(&k("index"));
    let uni = matches!(Curve::from_index(enum_idx::<Curve>(c, "moda_curve")), Curve::Decay | Curve::Attack);
    let range = if uni { (base.min(base + amt), base.max(base + amt)) } else { (base - amt.abs(), base + amt.abs()) };
    let level = newest.map_or(0.0, |v| f.voice(v, if o == 0 { vf::AMP_A } else { vf::AMP_B }));
    let inp = HeroInput {
        frames: &live.hero_frames,
        key: table as u64,
        live: f.index01(o),
        base,
        range,
        default01: c.default01(&k("index")),
        level,
        on: c.get01(&k("on")) > 0.5,
        osc_b: live.hero_b,
        table: Table::variants()[table],
        reduced_motion: live.reduced_motion,
    };
    let hero_rect = at(HERO.0, HERO.1, HERO.2, HERO.3);
    let (pa, pdy) = rr_ui::motion::page_fx(ui.ctx(), HERO.1);
    rr_ui::fx::add_shadow(ui.ctx(), hero_rect.translate(vec2(0.0, pdy)), pa);
    let hr = live.hero.show(ui, hero_rect, &inp);
    if pa < 0.999 {
        // The GL display cannot fade: a veil in the panel colour stands in for its opacity.
        let veil = T.panel_top.lerp_to_gamma(T.panel_bottom, 0.15);
        ui.painter().rect_filled(hero_rect.expand(2.0), 0.0, veil.gamma_multiply(1.0 - pa));
    }
    if let Some(b) = hr.tab {
        live.hero_b = b;
    }
    if hr.hovered {
        set_hover(
            ui,
            &format!("{} INDEX", if o == 0 { "OSC A" } else { "OSC B" }),
            &c.readout(&k("index")),
            Some((format!("{:.1}", f.index01(o) * 127.0), T.accent)),
            "The wavetable of the shown oscillator. Drag sideways to scrub Index, double-click to reset it.",
        );
    }
    drive(ui, c, &k("index"), hr.index, false);

    // Mod A / Mod B: Curve in the title row; Sync, Div, 1-shot, Target; shape display + knobs.
    for (i, (title, p, y)) in [("Mod A", "moda", 80.0), ("Mod B", "modb", 214.0)].into_iter().enumerate() {
        let kk = |s: &str| format!("{p}_{s}");
        group_at(ui, at(620.0, y, 336.0, 126.0), title, |ui| cb::<Curve>(ui, c, &kk("curve"), ""), |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            strip(ui, |ui| {
                tg(ui, c, &kk("sync"), "Sync");
                dim(ui, is_off(c, &kk("sync")), |ui| cb::<Division>(ui, c, &kk("div"), ""));
                tg(ui, c, &kk("oneshot"), "1-shot");
                cb::<Target>(ui, c, &kk("target"), "");
            });
            knob_row(ui, |ui| {
                ui.vertical(|ui| {
                    ui.add_space(4.0);
                    lfo_display(ui, vec2(if i == 0 { 96.0 } else { 84.0 }, 50.0), c, &f, i);
                });
                ui.add_space(8.0);
                if i == 0 {
                    dim(ui, !is_off(c, "moda_sync"), |ui| kn(ui, c, "moda_rate", "Rate", km(52.0)));
                    kn(ui, c, "moda_pitch", "Pitch", km(52.0).bipolar());
                    kn(ui, c, "moda_index", "Index", km(52.0).bipolar());
                    kn(ui, c, "moda_shift", "Shift", km(52.0).bipolar());
                } else {
                    dim(ui, !is_off(c, "modb_sync"), |ui| kn(ui, c, "modb_rate", "Rate", km(44.0)));
                    for (id, l) in [("motion", "Motion"), ("vol", "Vol"), ("cutoff", "Cutoff"), ("moda", "Mod A")] {
                        kn(ui, c, &kk(id), l, km(44.0).bipolar());
                    }
                }
            });
        });
    }

    // Oscillators and Sub.
    for (title, p, x, stage_k, level_k) in [("Osc A", "osca", 44.0, vf::STAGE_A, vf::AMP_A), ("Osc B", "oscb", 456.0, vf::STAGE_B, vf::AMP_B)] {
        let kk = |s: &str| format!("{p}_{s}");
        group_at(ui, at(x, 348.0, 404.0, 222.0), title, |ui| {
            tg(ui, c, &kk("on"), "");
            cb::<Table>(ui, c, &kk("table"), "");
        }, |ui| {
            fade(ui, p == "oscb" && is_off(c, "oscb_on"));
            ui.spacing_mut().item_spacing.y = 3.0;
            knob_row(ui, |ui| {
                kn(ui, c, &kk("oct"), "Oct", km(52.0).bipolar());
                kn(ui, c, &kk("semi"), "Semi", km(52.0));
                kn(ui, c, &kk("cent"), "Cent", km(52.0).bipolar());
                kn(ui, c, &kk("index"), "Index", kl(60.0));
                kn(ui, c, &kk("motion"), "Motion", km(52.0).bipolar());
                kn(ui, c, &kk("shift"), "Shift", km(52.0).bipolar());
                kn(ui, c, &kk("vol"), "Vol", km(52.0));
            });
            knob_row(ui, |ui| {
                for (id, l) in [("att", "Att"), ("dec", "Dec"), ("sus", "Sus"), ("rel", "Rel")] {
                    kn(ui, c, &kk(id), l, km(52.0));
                }
                kn(ui, c, &kk("curve"), "Curve", km(52.0).bipolar());
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.add_space(3.0);
                    adsr_display(ui, vec2(112.0, 50.0), c, p, &f, stage_k, level_k, T.accent);
                });
            });
            knob_row(ui, |ui| {
                for (id, l) in [("unison", "Unison"), ("detune", "Detune"), ("det_curve", "D.Crv"), ("blend", "Blend"), ("width", "Width"), ("rand_phase", "Rand")] {
                    kn(ui, c, &kk(id), l, km(50.0));
                }
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.add_space(V2_LABEL_H + (KNOB_D - CONTROL_H) / 2.0);
                    tg(ui, c, &kk("phase_reset"), "Reset");
                });
            });
        });
    }
    group_at(ui, at(868.0, 348.0, 88.0, 222.0), "Sub", |ui| tg(ui, c, "sub_on", ""), |ui| {
        fade(ui, is_off(c, "sub_on"));
        ui.spacing_mut().item_spacing.y = 6.0;
        kn(ui, c, "sub_level", "Level", km(64.0));
        kn(ui, c, "sub_oct", "Oct", km(64.0).bipolar());
        kn(ui, c, "mono_xover", "Xover", km(64.0));
    });

    // Filters and the filter envelope.
    for (title, p, x) in [("Filter A", "filta", 44.0), ("Filter B", "filtb", 314.0)] {
        let kk = |s: &str| format!("{p}_{s}");
        group_at(ui, at(x, 578.0, 262.0, 102.0), title, |ui| {
            tg(ui, c, &kk("env"), "Env");
            cb::<FilterType>(ui, c, &kk("type"), "");
        }, |ui| {
            fade(ui, p == "filtb" && enum_idx::<Routing>(c, "routing") == Routing::variants().len() - 1);
            knob_row(ui, |ui| {
                kn(ui, c, &kk("freq"), "Freq", kl(56.0));
                kn(ui, c, &kk("res"), "Res", km(48.0));
                kn(ui, c, &kk("kbd"), "Kbd", km(48.0));
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.add_space(2.0);
                    filter_display(ui, vec2(78.0, 56.0), c, &f, p);
                });
            });
        });
    }
    group_at(ui, at(584.0, 578.0, 372.0, 102.0), "Filter Env", |ui| tg(ui, c, "fenv_inv", "Inv"), |ui| {
        knob_row(ui, |ui| {
            for (id, l) in [("att", "Att"), ("dec", "Dec"), ("sus", "Sus"), ("rel", "Rel"), ("amt", "Amt")] {
                kn(ui, c, &format!("fenv_{id}"), l, ks(44.0));
            }
            kn(ui, c, "fenv_curve", "Curve", ks(44.0).bipolar());
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.add_space(2.0);
                adsr_display(ui, vec2(78.0, 54.0), c, "fenv", &f, vf::FENV_STAGE, vf::FENV, SRC_E);
            });
        });
    });

    // Bottom row: Shaper, Voice, Wheel, Velocity.
    group_at(ui, at(44.0, 688.0, 212.0, 90.0), "Shaper", |ui| {
        tg(ui, c, "shaper_on", "");
        cb::<ShaperMode>(ui, c, "shaper_mode", "");
    }, |ui| {
        fade(ui, is_off(c, "shaper_on"));
        knob_row(ui, |ui| {
            kn(ui, c, "shaper_amt", "Amt", ks(48.0));
            ui.add_space(8.0);
            curve_preview(ui, vec2(132.0, 48.0), &shaper_curve(c));
        });
    });
    group_at(ui, at(264.0, 688.0, 224.0, 90.0), "Voice", |ui| cb::<Routing>(ui, c, "routing", ""), |ui| {
        knob_row(ui, |ui| {
            kn(ui, c, "poly", "Poly", ks(40.0));
            kn(ui, c, "porta", "Porta", ks(40.0));
            kn(ui, c, "bend_range", "Bend", ks(40.0));
            ui.add_space(4.0);
            ui.vertical(|ui| {
                ui.add_space(V2_LABEL_H + (32.0 - CONTROL_H) / 2.0);
                tg(ui, c, "legato", "Legato");
            });
        });
    });
    group_at(ui, at(496.0, 688.0, 192.0, 90.0), "Wheel", |_| {}, |ui| {
        knob_row(ui, |ui| {
            for (id, l) in [("index", "Index"), ("shift", "Shift"), ("cutoff", "Cutoff"), ("mod", "Mod")] {
                kn(ui, c, &format!("wheel_{id}"), l, ks(42.0).bipolar());
            }
        });
    });
    group_at(ui, at(696.0, 688.0, 260.0, 90.0), "Velocity", |_| {}, |ui| {
        knob_row(ui, |ui| {
            for (id, l) in [("vol_a", "Vol A"), ("vol_b", "Vol B"), ("fenv", "F.Env"), ("att", "Att"), ("shift", "Shift"), ("mod", "Mod")] {
                kn(ui, c, &format!("vel_{id}"), l, ks(39.0).bipolar());
            }
        });
    });
}

/// A toggle inside a knob row: a full knob-cell tall, centred on the knob band, 4 px of air each side.
fn tg_cell(ui: &mut Ui, c: &dyn Ctl, id: &str, label: &str) {
    ui.add_space(4.0);
    ui.vertical(|ui| {
        ui.set_min_height(knob_cell_h(KNOB_D));
        ui.add_space(KNOB_LABEL_H + (KNOB_D - CONTROL_H) / 2.0);
        tg(ui, c, id, label);
    });
    ui.add_space(4.0);
}

/// A toggle in a v2 knob-row cell for a knob of diameter `d`: the cell is as tall as a compact knob
/// cell, the toggle centred on the knob band, 4 px of air each side.
fn tgv(ui: &mut Ui, c: &dyn Ctl, id: &str, label: &str, d: f32) {
    ui.add_space(4.0);
    ui.vertical(|ui| {
        ui.set_min_height(V2_LABEL_H + d + 2.0);
        ui.add_space(V2_LABEL_H + (d - CONTROL_H) / 2.0);
        tg(ui, c, id, label);
    });
    ui.add_space(4.0);
}

/// A labelled combo in a v2 (56 px) knob-row cell.
fn cbv<E: Enum>(ui: &mut Ui, c: &dyn Ctl, id: &str, label: &str) {
    let cur = enum_idx::<E>(c, id);
    let (pick, hovered) = combo_v2(ui, label, cur, E::variants());
    if hovered {
        let value = E::variants().get(cur).copied().unwrap_or("").to_string();
        ui.data_mut(|d| d.insert_temp(hover_id(), Hover { name: c.name(id).to_uppercase(), id: id.to_string(), value, ..Default::default() }));
    }
    if let Some(i) = pick {
        one_shot(c, id, i as f32 / (E::variants().len() - 1) as f32);
    }
}

/// The plain (unit) value of a param.
fn pv(c: &dyn Ctl, id: &str) -> f32 {
    c.plain(id, c.get01(id))
}

/// FX page (MOTION.md 2.2): the eight modules in chain order at absolute positions, each with its
/// on LED in the title bar (the limiter is always on), its compact knobs and a live display.
/// Rows: y 80 (h 262), 350 (h 164), 522 (h 256), 8 px apart.
fn fx_body(ui: &mut Ui, c: &dyn Ctl, live: &LiveView, bv: &BusView, at: &dyn Fn(f32, f32, f32, f32) -> Rect) {
    let f = &live.fx;
    let on = |id: &str| !is_off(c, id);

    // ---- Row 1 -------------------------------------------------------------------------------
    group_at(ui, at(44.0, 80.0, 272.0, 262.0), "Distortion", |ui| tg(ui, c, "dist_on", ""), |ui| {
        fade(ui, !on("dist_on"));
        let o = ui.cursor().min;
        knob_row(ui, |ui| {
            kn(ui, c, "dist_mix", "Mix", km(62.0));
            kn(ui, c, "dist_drive", "Drive", km(62.0));
            kn(ui, c, "dist_bias", "Bias", km(62.0).bipolar());
            kn(ui, c, "dist_out", "Out", km(62.0));
        });
        knob_row(ui, |ui| {
            cbv::<DistType>(ui, c, "dist_type", "Type");
            tgv(ui, c, "dist_autogain", "Auto", KNOB_D);
        });
        knob_row(ui, |ui| {
            cbv::<DistFilterMode>(ui, c, "dist_filter_mode", "Filter");
            cbv::<DistFilterType>(ui, c, "dist_filter_type", "F.Type");
            kn(ui, c, "dist_filter_freq", "F.Freq", km(60.0));
            kn(ui, c, "dist_filter_q", "F.Q", km(60.0));
        });
        let kind = crate::dsp::fx::distortion::DistType::ALL[enum_idx::<DistType>(c, "dist_type").min(7)];
        let r = Rect::from_min_size(o + vec2(248.0 - 68.0, 62.0), vec2(68.0, 62.0));
        fxview::dist_display(&ui.painter().clone(), r, kind, pv(c, "dist_drive"), pv(c, "dist_bias"), &f.dist_trail, on("dist_on"));
    });

    let g = at(324.0, 80.0, 384.0, 262.0);
    group_at(ui, g, "Multiband", |ui| tg(ui, c, "mb_on", ""), |ui| {
        fade(ui, !on("mb_on"));
        let o = ui.cursor().min;
        ui.spacing_mut().item_spacing.y = 14.0;
        let w = km(60.0);
        let left: [&[(&str, &str, KnobStyle)]; 3] = [
            &[("mix", "Mix", w), ("depth", "Depth", w), ("time", "Time", w)],
            &[("in", "In", w.bipolar()), ("out", "Out", w.bipolar())],
            &[("xover_lo", "Lo X", w), ("xover_hi", "Hi X", w)],
        ];
        for (cells, (key, label, style)) in left.iter().zip([("up", "Up", w), ("down", "Down", w), ("gain", "Gain", w.bipolar())]) {
            knob_row(ui, |ui| {
                for &(id, l, s) in *cells {
                    kn(ui, c, &format!("mb_{id}"), l, s);
                }
                ui.add_space(60.0 * (3 - cells.len()) as f32); // the meter sits in the empty cell
                for (band, name) in [("lo", "Lo"), ("mid", "Mid"), ("hi", "Hi")] {
                    kn(ui, c, &format!("mb_{key}_{band}"), &format!("{name} {label}"), style);
                }
            });
        }
        let band = |k: &str| [pv(c, &format!("mb_{k}_lo")), pv(c, &format!("mb_{k}_mid")), pv(c, &format!("mb_{k}_hi"))];
        let (lvl, gain) = if on("mb_on") { (f.mb_level, f.mb_gain) } else { ([-120.0; 3], [0.0; 3]) };
        let r = Rect::from_min_size(o + vec2(122.0, 70.0), vec2(56.0, 126.0));
        fxview::mb_display(&ui.painter().clone(), r, lvl, gain, band("up"), band("down"));
        let p = ui.painter().clone();
        rr_ui::paint::tracked(&p, pos2(g.center().x, g.bottom() - 14.0), Align2::CENTER_CENTER, "THRESHOLDS PER BAND  \u{b7}  GAIN TRIM  \u{b7}  CROSSOVERS", font::label(11.0), T.muted, 0.8);
    });

    group_at(ui, at(716.0, 80.0, 240.0, 262.0), "EQ", |ui| tg(ui, c, "eq_on", ""), |ui| {
        fade(ui, !on("eq_on"));
        ui.spacing_mut().item_spacing.y = 4.0;
        let ep = EqParams {
            on: true,
            low_freq: pv(c, "eq_low_freq"),
            low_gain: pv(c, "eq_low_gain"),
            peak_freq: pv(c, "eq_peak_freq"),
            peak_q: pv(c, "eq_peak_q"),
            peak_gain: pv(c, "eq_peak_gain"),
            high_freq: pv(c, "eq_high_freq"),
            high_gain: pv(c, "eq_high_gain"),
            cut_on: on("eq_cut_on"),
            cut_freq: pv(c, "eq_cut_freq"),
        };
        let (r, _) = ui.allocate_exact_size(vec2(216.0, 56.0), Sense::hover());
        fxview::eq_display(&ui.painter().clone(), r, &ep, &live.spec, &live.spec_peak);
        let (k, kb) = (ks(72.0), ks(72.0).bipolar());
        knob_row(ui, |ui| {
            for (id, l) in [("low", "Lo Freq"), ("peak", "Pk Freq"), ("high", "Hi Freq")] {
                kn(ui, c, &format!("eq_{id}_freq"), l, k);
            }
        });
        knob_row(ui, |ui| {
            for (id, l) in [("low", "Lo Gain"), ("peak", "Pk Gain"), ("high", "Hi Gain")] {
                kn(ui, c, &format!("eq_{id}_gain"), l, kb);
            }
        });
        knob_row(ui, |ui| {
            kn(ui, c, "eq_peak_q", "Pk Q", k);
            ui.add_space(6.0); // the 60 px toggle cell centred in a 72 px column
            tgv(ui, c, "eq_cut_on", "Cut", 32.0);
            ui.add_space(6.0);
            kn(ui, c, "eq_cut_freq", "Cut Freq", k);
        });
    });

    // ---- Row 2 -------------------------------------------------------------------------------
    group_at(ui, at(44.0, 350.0, 340.0, 164.0), "Chorus", |ui| tg(ui, c, "cho_on", ""), |ui| {
        fade(ui, !on("cho_on"));
        let o = ui.cursor().min;
        knob_row(ui, |ui| {
            kn(ui, c, "cho_mix", "Mix", km(52.0));
            kn(ui, c, "cho_voices", "Voices", km(52.0));
            dim(ui, on("cho_sync"), |ui| kn(ui, c, "cho_rate", "Rate", km(52.0)));
            kn(ui, c, "cho_depth", "Depth", km(52.0));
            kn(ui, c, "cho_delay", "Delay", km(52.0));
        });
        knob_row(ui, |ui| {
            tgv(ui, c, "cho_sync", "Sync", KNOB_D);
            dim(ui, !on("cho_sync"), |ui| cbv::<Division>(ui, c, "cho_div", "Div"));
            kn(ui, c, "cho_feedback", "Feedback", km(62.0).bipolar());
            kn(ui, c, "cho_lpf", "LPF", km(60.0));
            kn(ui, c, "cho_wet_lowcut", "Low Cut", km(60.0));
        });
        let r = Rect::from_min_size(o + vec2(262.0, 0.0), vec2(54.0, 54.0));
        fxview::chorus_display(&ui.painter().clone(), r, pv(c, "cho_voices").round() as usize, f.cho_phase, pv(c, "cho_depth"));
    });
    group_at(ui, at(392.0, 350.0, 204.0, 164.0), "Dimension", |ui| tg(ui, c, "dim_on", ""), |ui| {
        fade(ui, !on("dim_on"));
        knob_row(ui, |ui| {
            kn(ui, c, "dim_mix", "Mix", km(60.0));
            kn(ui, c, "dim_size", "Size", km(60.0));
            kn(ui, c, "dim_wet_lowcut", "Low Cut", km(60.0));
        });
        let (r, _) = ui.allocate_exact_size(vec2(180.0, 58.0), Sense::hover());
        fxview::dim_display(&ui.painter().clone(), r, &f.dim_l, &f.dim_r);
    });
    group_at(ui, at(604.0, 350.0, 352.0, 164.0), "Delay", |ui| tg(ui, c, "dly_on", ""), |ui| {
        fade(ui, !on("dly_on"));
        let o = ui.cursor().min;
        knob_row(ui, |ui| {
            kn(ui, c, "dly_mix", "Mix", km(48.0));
            dim(ui, on("dly_sync"), |ui| kn(ui, c, "dly_time", "Time", km(62.0)));
            kn(ui, c, "dly_feedback", "Feedback", km(58.0));
            kn(ui, c, "dly_hp", "HP", km(56.0));
            kn(ui, c, "dly_lp", "LP", km(56.0));
        });
        knob_row(ui, |ui| {
            tgv(ui, c, "dly_sync", "Sync", KNOB_D);
            dim(ui, !on("dly_sync"), |ui| {
                cbv::<DelayDiv>(ui, c, "dly_div", "Div");
                cbv::<DelayFeel>(ui, c, "dly_feel", "Feel");
            });
            cbv::<DelayMode>(ui, c, "dly_mode", "Mode");
        });
        let r = Rect::from_min_size(o + vec2(328.0 - 52.0, 0.0), vec2(52.0, 54.0));
        let t = f.dly_time;
        let flash = |k: usize| f.dly_flash(k as f32 * t);
        fxview::delay_display(&ui.painter().clone(), r, pv(c, "dly_feedback"), pv(c, "dly_mix"), &flash, f.dly_wet_db);
    });

    // ---- Row 3 -------------------------------------------------------------------------------
    group_at(ui, at(44.0, 522.0, 312.0, 256.0), "Reverb", |ui| tg(ui, c, "rev_on", ""), |ui| {
        fade(ui, !on("rev_on"));
        knob_row(ui, |ui| {
            kn(ui, c, "rev_mix", "Mix", km(52.0));
            cbv::<ReverbType>(ui, c, "rev_type", "Type");
            kn(ui, c, "rev_size", "Size", km(52.0));
            kn(ui, c, "rev_decay", "Decay", km(52.0));
            kn(ui, c, "rev_damp", "Damp", km(52.0));
        });
        knob_row(ui, |ui| {
            kn(ui, c, "rev_predelay", "Pre-Dly", km(68.0));
            kn(ui, c, "rev_lowcut", "Low Cut", km(72.0));
            kn(ui, c, "rev_hicut", "Hi Cut", km(72.0));
            kn(ui, c, "rev_width", "Width", km(76.0));
        });
        let (r, _) = ui.allocate_exact_size(vec2(288.0, 90.0), Sense::hover());
        let lvl = (on("rev_on") && f.rev_db > -70.0).then_some(f.rev_db - f.rev_ref);
        fxview::reverb_display(&ui.painter().clone(), r, pv(c, "rev_decay"), pv(c, "rev_predelay") * 0.001, f.rev_age, lvl);
    });
    // The limiter takes the rest of the row: Gain knob, then the gain-reduction meter (0 .. -12 dB)
    // over the output spectrum.
    group_at(ui, at(364.0, 522.0, 592.0, 256.0), "Limiter", |_| {}, |ui| {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            kn(ui, c, "lim_gain", "Gain", kl(60.0));
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                let gr = if bv.gr_db < 0.05 { "0.0 dB".to_string() } else { format!("-{:.1} dB", bv.gr_db) };
                gr_meter(ui, 492.0, "Gain reduction", bv.gr_db, 12.0, &gr);
                let (r, _) = ui.allocate_exact_size(vec2(492.0, 117.0), Sense::hover());
                fxview::limiter_spectrum(&ui.painter().clone(), r, &live.spec, &live.spec_peak);
                let peak = if bv.peak_db < -90.0 { "-inf dBFS".to_string() } else { format!("{:.1} dBFS", bv.peak_db) };
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("OUTPUT PEAK").font(font::label(11.0)).color(T.muted));
                    ui.label(egui::RichText::new(peak).font(font::mono(12.0)));
                });
            });
        });
    });
}


fn rd<T: Clone>(l: &RwLock<T>) -> T {
    l.read().unwrap_or_else(PoisonError::into_inner).clone()
}

fn wr<T>(l: &RwLock<T>, v: T) {
    *l.write().unwrap_or_else(PoisonError::into_inner) = v;
}

/// Editor-side state: bus view and its refresh clock, the live displays.
pub struct EditorState {
    view: BusView,
    live: LiveView,
    next_refresh: f64,
    /// Sound params right after the last preset load; any difference marks the preset edited.
    snap: Vec<f32>,
    /// Take `snap` on the next frame (the host applies a load's gestures between frames).
    resnap: bool,
}

/// `tables` is the engine's own slot: the display reads the audio tables (no second 29 MB copy).
/// If the editor opens before `initialize`, the GUI thread generates them and the engine reuses them.
pub fn create(params: Arc<GtsParams>, feeds: Feeds, link: Arc<BusLink>, tables: Arc<OnceLock<Tables>>) -> Option<Box<dyn Editor>> {
    let table = ParamTable::new(params.clone());
    let Feeds { meters, viz, scope } = feeds;
    create_egui_editor(
        EguiState::from_size(WINDOW.0, WINDOW.1),
        EditorState { view: BusView { peak_db: -120.0, ..Default::default() }, live: LiveView::new(viz, Some(scope)), next_refresh: 0.0, snap: vec![], resnap: false },
        |ctx, st| {
            rr_ui::theme::apply(ctx);
            st.next_refresh = 0.0; // egui time restarts at 0 on every open
            st.live.hero.reset_gl(); // a reopened window is a new GL context
        },
        // nih_plug_egui requests a repaint before every update, so this runs and paints on every
        // baseview tick (~62 Hz); all motion is time-based (docs/design/motion-feasibility.md 1).
        move |ctx, setter, st| {
            // Sources list + lost-binding reconnects, ~4x/s on egui time.
            let now = ctx.input(|i| i.time);
            if now >= st.next_refresh {
                st.next_refresh = now + 0.25;
                link.refresh(&params);
                st.view.sources = link.bus().list().iter().map(|s| s.persisted()).collect();
            }
            if !ctx.wants_keyboard_input() {
                st.view.label = rd(&params.instance_label); // not while the label is being typed
            }
            st.view.bound = rd(&params.cv_sources);
            st.view.activity = std::array::from_fn(|i| link.activity(i));
            st.view.oversampling = params.oversampling.load(Ordering::Relaxed);
            st.view.show_values = params.show_values.load(Ordering::Relaxed);
            st.view.reduced_motion = params.reduced_motion.load(Ordering::Relaxed);
            // Meters: take what the audio thread gathered (the FX page readouts fall back slowly).
            let gr = f32::from_bits(meters.gr_db.swap(0, Ordering::Relaxed));
            st.view.gr_db = gr.max(st.view.gr_db - 1.0).max(0.0);
            st.live.peaks = std::array::from_fn(|i| f32::from_bits(meters.peak[i].swap(0, Ordering::Relaxed)));
            let peak = 20.0 * st.live.peaks[0].max(st.live.peaks[1]).max(1e-6).log10();
            st.view.peak_db = peak.max(st.view.peak_db - 0.5);
            if std::mem::take(&mut st.resnap) {
                st.snap = table.snapshot();
            }
            st.view.preset_dirty = st.view.preset.is_some() && table.snapshot() != st.snap;
            let before = st.view.clone();

            let ctl = PlugCtl { table: &table, setter };
            let tables = tables.get_or_init(|| {
                let mut t = Tables::new();
                t.generate();
                t
            });
            egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
                draw(ui, &ctl, tables, &mut st.live, &mut st.view);
            });

            if let Some(i) = st.view.preset_pick.take() {
                apply_preset(&table, &ctl, i.checked_sub(1).and_then(|k| preset::factory().get(k)));
                ctx.data_mut(|d| d.insert_temp(glide_id(), now));
                st.view.preset = Some(i);
                st.view.preset_dirty = false;
                st.resnap = true;
            }
            if st.view.oversampling != before.oversampling {
                params.oversampling.store(st.view.oversampling, Ordering::Relaxed);
            }
            if st.view.show_values != before.show_values {
                params.show_values.store(st.view.show_values, Ordering::Relaxed);
            }
            if st.view.reduced_motion != before.reduced_motion {
                params.reduced_motion.store(st.view.reduced_motion, Ordering::Relaxed);
            }
            if st.view.label != before.label {
                let live = link.set_label(&st.view.label);
                wr(&params.instance_label, if live.is_empty() { st.view.label.clone() } else { live });
            }
            if st.view.bound != before.bound {
                for i in 0..3 {
                    if st.view.bound[i] != before.bound[i] {
                        link.set_source(i, &st.view.bound[i]);
                    }
                }
                wr(&params.cv_sources, st.view.bound.clone());
            }
        },
    )
}
