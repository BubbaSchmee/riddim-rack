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

/// Per-frame knob decorations: live modulation (ModLive) and the hovered source chip.
#[derive(Clone, Default)]
struct KnobLive {
    live: [Option<(f32, Color32)>; MOD_IDS.len()],
    /// Source chip under the pointer: 0 Mod A, 1 Mod B, 2 Env, 3 Macro.
    chip: Option<usize>,
    filt_env: [bool; 2],
}

fn knob_live_id() -> Id {
    Id::new("gts-knob-live")
}

/// What the help bar shows for the control under the pointer.
#[derive(Clone, Default)]
struct Hover {
    name: String,
    value: String,
    live: Option<(String, Color32)>,
}

fn hover_id() -> Id {
    Id::new("gts-hover")
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

fn kn(ui: &mut Ui, c: &dyn Ctl, id: &str, label: &str, mut style: KnobStyle) {
    let stepped = c.steps(id).is_some();
    let shown = if stepped {
        ui.data(|d| d.get_temp::<f32>(Id::new(("gts-drag", id)))).unwrap_or_else(|| c.get01(id))
    } else {
        c.get01(id)
    };
    if style.compact {
        if let Some(kl) = ui.data(|d| d.get_temp::<KnobLive>(knob_live_id())) {
            if let Some(k) = MOD_IDS.iter().position(|m| *m == id) {
                style.live = kl.live[k];
            }
            if let Some(ch) = kl.chip {
                if sources_of(id, kl.filt_env) & (1 << ch) != 0 {
                    style.ring = Some([SRC_A, SRC_B, SRC_E, SRC_M][ch]);
                }
            }
        }
    }
    let readout = c.readout(id);
    let r = knob_styled(ui, label, shown, c.default01(id), &readout, style);
    if r.hovered {
        let live = style.live.map(|(v, col)| (c.readout_at(id, v), col));
        ui.data_mut(|d| d.insert_temp(hover_id(), Hover { name: label.to_uppercase(), value: readout, live }));
    }
    drive(ui, c, id, r, stepped);
}

fn tg(ui: &mut Ui, c: &dyn Ctl, id: &str, label: &str) {
    if let Some(on) = led_toggle(ui, label, c.get01(id) > 0.5) {
        one_shot(c, id, if on { 1.0 } else { 0.0 });
    }
}

fn enum_idx<E: Enum>(c: &dyn Ctl, id: &str) -> usize {
    (c.get01(id) * (E::variants().len() - 1) as f32).round() as usize
}

fn cb<E: Enum>(ui: &mut Ui, c: &dyn Ctl, id: &str, label: &str) {
    if let Some(i) = combo(ui, label, enum_idx::<E>(c, id), E::variants()) {
        one_shot(c, id, i as f32 / (E::variants().len() - 1) as f32);
    }
}

/// Opacity of a control that currently has no effect (still editable).
const INACTIVE: f32 = 0.5;

/// Dims everything `ui` draws from here on (a group body whose module or section is off).
fn fade(ui: &mut Ui, inactive: bool) {
    if inactive {
        ui.multiply_opacity(INACTIVE);
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
    trace: Vec<f32>,
    spec_target: Vec<f32>,
    spec: Vec<f32>,
    spec_peak: Vec<f32>,
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
        let have = self.ring.as_ref().is_some_and(|r| r.latest(&mut self.buf));
        if !have {
            self.buf.fill(0.0);
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

/// "F1", "A#2" for a MIDI note.
fn note_name(n: u8) -> String {
    const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    format!("{}{}", NAMES[n as usize % 12], n as i32 / 12 - 1)
}

// ---- layout ----------------------------------------------------------------------------------

/// Draws the whole panel into `ui` (the full [`WINDOW`]). Returns how far any group's content ran
/// past its fixed size (zero when everything fits; the layout test checks it).
pub fn draw(ui: &mut Ui, c: &dyn Ctl, tables: &Tables, live: &mut LiveView, bv: &mut BusView) -> Vec2 {
    let full = rr_ui::panel::rack_frame(ui);
    let o = full.min.to_vec2();
    let at = |x: f32, y: f32, w: f32, h: f32| Rect::from_min_size(pos2(x, y) + o, vec2(w, h));
    let _ = take_group_overflow(ui.ctx());
    set_always_values(ui.ctx(), bv.show_values);
    let now = ui.input(|i| i.time);
    let osc_pitch = 12.0 * (c.get01("osca_oct") * 8.0 - 4.0).round() + (c.get01("osca_semi") * 12.0).round();
    live.update(now, osc_pitch);
    let kl = live.knob_live(c);
    ui.data_mut(|d| {
        d.insert_temp(knob_live_id(), kl);
        d.remove_temp::<Hover>(hover_id());
    });

    header(ui, c, live, bv, &at);
    let body = at(44.0, 80.0, 912.0, 704.0);
    let mut bui = ui.new_child(UiBuilder::new().max_rect(body).layout(egui::Layout::top_down(egui::Align::Min)));
    let mut over = Vec2::ZERO;
    if bv.fx_page {
        fx_body(&mut bui, c, bv);
        over = (bui.min_rect().max - body.max).max(Vec2::ZERO);
    } else {
        synth(&mut bui, c, tables, live, &at);
    }
    help_bar(ui, at(44.0, 792.0, 912.0, 28.0), bv);
    patch_popover(ui, c, bv, at(508.0, 48.0, 72.0, 24.0));
    ui.allocate_rect(full, Sense::hover());
    over.max(take_group_overflow(ui.ctx()))
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
        set_hover(ui, "OUTPUT", if live.spectrum { "spectrum" } else { "scope" }, None);
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

fn set_hover(ui: &Ui, name: &str, value: &str, live: Option<(String, Color32)>) {
    ui.data_mut(|d| d.insert_temp(hover_id(), Hover { name: name.into(), value: value.into(), live }));
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
        set_hover(ui, "PRESET", &name, None);
    }
    rr_ui::paint::tracked(&p, pos2(mid.center().x, r.top() + 16.0), Align2::CENTER_CENTER, &cat.to_uppercase(), font::label(11.0), T.muted, 1.0);
    let nr = p.text(pos2(mid.center().x, r.top() + 36.0), Align2::CENTER_CENTER, &name, font::strong(16.0), T.text);
    let dirty = ui.ctx().animate_bool_with_time(Id::new("gts-preset-dirty"), bv.preset_dirty, 0.18);
    if dirty > 0.0 {
        p.circle_filled(pos2(nr.right() + 8.0, nr.center().y), 3.0, T.accent.gamma_multiply(dirty));
    }
    let popup = Id::new("gts-preset-list");
    if resp.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup));
    }
    egui::popup::popup_below_widget(ui, popup, &resp, egui::popup::PopupCloseBehavior::CloseOnClick, |ui| {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        ui.set_min_width(mid.width() + 40.0);
        for i in 0..n {
            let (nm, ct) = if i == 0 { ("Init", "") } else { (f[i - 1].name.as_str(), f[i - 1].category.as_str()) };
            let (row, rr) = ui.allocate_exact_size(vec2(ui.available_width(), CONTROL_H), Sense::click());
            if rr.hovered() {
                ui.painter().rect_filled(row, 3.0, T.a(0.16));
            }
            ui.painter().text(pos2(row.left() + 8.0, row.center().y), Align2::LEFT_CENTER, nm, font::label(13.0), if Some(i) == cur { T.accent } else { T.text });
            rr_ui::paint::tracked(ui.painter(), pos2(row.right() - 8.0, row.center().y), Align2::RIGHT_CENTER, &ct.to_uppercase(), font::label(11.0), T.muted, 0.5);
            if rr.clicked() {
                bv.preset_pick = Some(i);
            }
        }
    });
}

/// SYNTH / FX segmented control with a sliding lit segment (180 ms).
fn page_switch(ui: &mut Ui, r: Rect, bv: &mut BusView) {
    let p = ui.painter().clone();
    rr_ui::paint::inset_box(&p, r, false);
    let w = (r.width() - 4.0) / 2.0;
    let x = ui.ctx().animate_value_with_time(Id::new("gts-page-lit"), bv.fx_page as u8 as f32, 0.18);
    let lit = Rect::from_min_size(pos2(r.left() + 2.0 + x * w, r.top() + 2.0), vec2(w, r.height() - 4.0));
    p.rect_filled(lit, 3.0, T.a(0.20));
    p.rect_stroke(lit, 3.0, rr_ui::paint::st(1.0, T.a(0.55)), StrokeKind::Inside);
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
        p.rect_stroke(r, 4.0, rr_ui::paint::st(1.0, T.a(0.55)), StrokeKind::Inside);
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
        set_hover(ui, "PATCH", "RR-BUS inputs", None);
    }
    if resp.clicked() {
        bv.patch_open = !bv.patch_open;
    }
}

/// The PATCH strip as a layer under its chip (150 ms fade + 6 px drop). Esc or a click elsewhere
/// (while no list of its own is open) closes it.
fn patch_popover(ui: &mut Ui, c: &dyn Ctl, bv: &mut BusView, chip: Rect) {
    let ctx = ui.ctx().clone();
    let k = ctx.animate_bool_with_time(Id::new("gts-patch-pop-a"), bv.patch_open, if bv.patch_open { 0.15 } else { 0.09 });
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

/// Help bar: the hovered control's name, value and live value, or the gesture hint; the
/// "Always show values" switch on the right.
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
            if let Some((v, col)) = h.live {
                p.text(pos2(x, y), Align2::LEFT_CENTER, format!("\u{203a} {v}"), font::mono(12.0), col);
            }
        }
        None => {
            x = rr_ui::paint::tracked(&p, pos2(x, y), Align2::LEFT_CENTER, "HOVER A CONTROL", font::label(11.0), T.muted, 1.0).right() + 12.0;
            let hint = if bv.fx_page { "Drag to set, Shift for fine, double-click to reset." } else { "Drag to set, Shift for fine, double-click to reset. Drag the wavetable sideways to scrub Index." };
            p.text(pos2(x, y), Align2::LEFT_CENTER, hint, font::label(12.0), T.disp_caption);
        }
    }
    place(ui, Rect::from_min_max(pos2(r.right() - 110.0, r.top() + 2.0), pos2(r.right() - 2.0, r.bottom() - 2.0)), |ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(on) = led_toggle(ui, "Values", bv.show_values) {
                bv.show_values = on;
            }
        });
    });
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
    p.circle_filled(at, 9.0, col.gamma_multiply(0.12 * alpha));
    p.circle_filled(at, 5.0, col.gamma_multiply(0.22 * alpha));
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

fn synth(ui: &mut Ui, c: &dyn Ctl, tables: &Tables, live: &mut LiveView, at: &dyn Fn(f32, f32, f32, f32) -> Rect) {
    // Sources strip first: a hovered chip rings its targets on every knob drawn after it.
    let f = live.frame;
    let src = at(44.0, 320.0, 568.0, 20.0);
    rr_ui::paint::tracked(ui.painter(), pos2(src.left() + 8.0, src.center().y), Align2::LEFT_CENTER, "SOURCES", font::label(11.0), T.title, 1.4);
    let newest = f.newest();
    let act = |k: usize| newest.map_or(0.0, |v| f.voice(v, k).abs());
    let chips = [("A", "Mod A", SRC_A, act(vf::OUT_A), 74.0), ("B", "Mod B", SRC_B, act(vf::OUT_B), 74.0), ("E", "Env", SRC_E, act(vf::FENV), 62.0), ("M", "Macro 1", SRC_M, 0.0, 86.0)];
    let mut x = src.left() + 76.0;
    let mut chip = None;
    for (i, (g, name, col, a, w)) in chips.into_iter().enumerate() {
        let w = w + 30.0;
        let r = source_chip(ui, Rect::from_min_size(pos2(x, src.top()), vec2(w, 20.0)), g, name, col, a);
        if r.hovered() {
            chip = Some(i);
            set_hover(ui, &name.to_uppercase(), if i == 3 { "macro (assignments arrive with the matrix)" } else { "source: hover shows what it moves" }, None);
        }
        x += w + 8.0;
    }
    if chip.is_some() {
        ui.data_mut(|d| d.get_temp_mut_or_default::<KnobLive>(knob_live_id()).chip = chip);
    }

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
    let hr = live.hero.show(ui, at(HERO.0, HERO.1, HERO.2, HERO.3), &inp);
    if let Some(b) = hr.tab {
        live.hero_b = b;
    }
    if hr.hovered {
        set_hover(ui, &format!("{} INDEX", if o == 0 { "OSC A" } else { "OSC B" }), &c.readout(&k("index")), Some((format!("{:.1}", f.index01(o) * 127.0), T.accent)));
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
                kn(ui, c, &kk("freq"), "Freq", kl(78.0));
                kn(ui, c, &kk("res"), "Res", km(78.0));
                kn(ui, c, &kk("kbd"), "Kbd", km(78.0));
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

/// FX page: the eight modules in chain order (Distortion -> Limiter), each with its on LED in the
/// title bar (the limiter is always on) and its Mix knob first. Knob cells are 56 px, or 60 px
/// where the readout can be eight characters wide ("120.0 Hz", "-24.0 dB"). Rows: 257 (three
/// knob rows) + 184 (two) + 191 (two, stretched) + 2 x 12 = the 656 px body.
fn fx_body(ui: &mut Ui, c: &dyn Ctl, bv: &BusView) {
    let (k, w) = (KnobStyle::default().cell(56.0), KnobStyle::default().cell(60.0));
    let (kb, wb) = (k.bipolar(), w.bipolar());
    ui.spacing_mut().item_spacing = vec2(GAP, GAP);
    let (h1, h2, h3) = (257.0, 184.0, 191.0);

    // Row 1: Distortion 272 + Multiband 384 + EQ 232 + 2 gaps = 912.
    ui.horizontal_top(|ui| {
        group_ext(ui, vec2(272.0, h1), "Distortion", |ui| tg(ui, c, "dist_on", ""), |ui| {
            fade(ui, is_off(c, "dist_on"));
            knob_row(ui, |ui| {
                kn(ui, c, "dist_mix", "Mix", k);
                kn(ui, c, "dist_drive", "Drive", w);
                kn(ui, c, "dist_bias", "Bias", kb);
                kn(ui, c, "dist_out", "Out", w);
            });
            knob_row(ui, |ui| {
                cb::<DistType>(ui, c, "dist_type", "Type");
                tg_cell(ui, c, "dist_autogain", "Auto Gain");
            });
            knob_row(ui, |ui| {
                cb::<DistFilterMode>(ui, c, "dist_filter_mode", "Filter");
                cb::<DistFilterType>(ui, c, "dist_filter_type", "F.Type");
                kn(ui, c, "dist_filter_freq", "F.Freq", w);
                kn(ui, c, "dist_filter_q", "F.Q", k);
            });
        });

        // Multiband: a 6 x 3 grid of 60 px cells. Left three columns: Mix first, then depth / time,
        // in / out and the two crossovers; right three: one column per band, rows up / down / gain.
        group_ext(ui, vec2(384.0, h1), "Multiband", |ui| tg(ui, c, "mb_on", ""), |ui| {
            fade(ui, is_off(c, "mb_on"));
            let left: [&[(&str, &str, KnobStyle)]; 3] = [
                &[("mix", "Mix", w), ("depth", "Depth", w), ("time", "Time", w)],
                &[("in", "In", wb), ("out", "Out", wb)],
                &[("xover_lo", "Lo X", w), ("xover_hi", "Hi X", w)],
            ];
            for (cells, (key, label, style)) in left.iter().zip([("up", "Up", w), ("down", "Down", w), ("gain", "Gain", wb)]) {
                knob_row(ui, |ui| {
                    for &(id, l, s) in *cells {
                        kn(ui, c, &format!("mb_{id}"), l, s);
                    }
                    ui.add_space(60.0 * (3 - cells.len()) as f32); // unused slots keep the band columns aligned
                    for (band, name) in [("lo", "Lo"), ("mid", "Mid"), ("hi", "Hi")] {
                        kn(ui, c, &format!("mb_{key}_{band}"), &format!("{name} {label}"), style);
                    }
                });
            }
        });

        // EQ: one column per band (Low / Peak / High), freq over gain; Q and the low cut underneath.
        group_ext(ui, vec2(232.0, h1), "EQ", |ui| tg(ui, c, "eq_on", ""), |ui| {
            fade(ui, is_off(c, "eq_on"));
            knob_row(ui, |ui| {
                for (id, l) in [("low", "Lo Freq"), ("peak", "Pk Freq"), ("high", "Hi Freq")] {
                    kn(ui, c, &format!("eq_{id}_freq"), l, w);
                }
            });
            knob_row(ui, |ui| {
                for (id, l) in [("low", "Lo Gain"), ("peak", "Pk Gain"), ("high", "Hi Gain")] {
                    kn(ui, c, &format!("eq_{id}_gain"), l, wb);
                }
            });
            knob_row(ui, |ui| {
                kn(ui, c, "eq_peak_q", "Pk Q", w);
                tg_cell(ui, c, "eq_cut_on", "Low Cut");
                kn(ui, c, "eq_cut_freq", "Cut Freq", w);
            });
        });
    });

    // Row 2: Chorus 336 + Dimension 200 + Delay 352 + 2 gaps = 912.
    ui.horizontal_top(|ui| {
        group_ext(ui, vec2(336.0, h2), "Chorus", |ui| tg(ui, c, "cho_on", ""), |ui| {
            fade(ui, is_off(c, "cho_on"));
            knob_row(ui, |ui| {
                kn(ui, c, "cho_mix", "Mix", k);
                kn(ui, c, "cho_voices", "Voices", k);
                dim(ui, !is_off(c, "cho_sync"), |ui| kn(ui, c, "cho_rate", "Rate", k));
                kn(ui, c, "cho_depth", "Depth", k);
                kn(ui, c, "cho_delay", "Delay", k);
            });
            knob_row(ui, |ui| {
                tg_cell(ui, c, "cho_sync", "Sync");
                dim(ui, is_off(c, "cho_sync"), |ui| cb::<Division>(ui, c, "cho_div", "Div"));
                kn(ui, c, "cho_feedback", "Feedback", kb);
                kn(ui, c, "cho_lpf", "LPF", w);
                kn(ui, c, "cho_wet_lowcut", "Low Cut", w);
            });
        });
        group_ext(ui, vec2(200.0, h2), "Dimension", |ui| tg(ui, c, "dim_on", ""), |ui| {
            fade(ui, is_off(c, "dim_on"));
            knob_row(ui, |ui| {
                kn(ui, c, "dim_mix", "Mix", k);
                kn(ui, c, "dim_size", "Size", k);
                kn(ui, c, "dim_wet_lowcut", "Low Cut", w);
            });
        });
        group_ext(ui, vec2(352.0, h2), "Delay", |ui| tg(ui, c, "dly_on", ""), |ui| {
            fade(ui, is_off(c, "dly_on"));
            knob_row(ui, |ui| {
                kn(ui, c, "dly_mix", "Mix", k);
                dim(ui, !is_off(c, "dly_sync"), |ui| kn(ui, c, "dly_time", "Time", KnobStyle::default().cell(66.0)));
                kn(ui, c, "dly_feedback", "Feedback", k);
                kn(ui, c, "dly_hp", "HP", w);
                kn(ui, c, "dly_lp", "LP", w);
            });
            knob_row(ui, |ui| {
                tg_cell(ui, c, "dly_sync", "Sync");
                dim(ui, is_off(c, "dly_sync"), |ui| {
                    cb::<DelayDiv>(ui, c, "dly_div", "Div");
                    cb::<DelayFeel>(ui, c, "dly_feel", "Feel");
                });
                cb::<DelayMode>(ui, c, "dly_mode", "Mode");
            });
        });
    });

    // Row 3: Reverb + Limiter.
    ui.horizontal_top(|ui| {
        group_ext(ui, vec2(312.0, h3), "Reverb", |ui| tg(ui, c, "rev_on", ""), |ui| {
            fade(ui, is_off(c, "rev_on"));
            knob_row(ui, |ui| {
                kn(ui, c, "rev_mix", "Mix", k);
                cb::<ReverbType>(ui, c, "rev_type", "Type");
                kn(ui, c, "rev_size", "Size", k);
                kn(ui, c, "rev_decay", "Decay", k);
                kn(ui, c, "rev_damp", "Damp", k);
            });
            knob_row(ui, |ui| {
                kn(ui, c, "rev_predelay", "Pre-Dly", KnobStyle::default().cell(68.0));
                kn(ui, c, "rev_lowcut", "Low Cut", w);
                kn(ui, c, "rev_hicut", "Hi Cut", w);
                kn(ui, c, "rev_width", "Width", k);
            });
        });
        // The limiter takes the rest of the row: Gain knob, then the gain-reduction meter (0 .. -12 dB)
        // over the output peak.
        group_ext(ui, vec2(588.0, h3), "Limiter", |_| {}, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                kn(ui, c, "lim_gain", "Gain", w);
                ui.vertical(|ui| {
                    ui.add_space(KNOB_LABEL_H);
                    let gr = if bv.gr_db < 0.05 { "0.0 dB".to_string() } else { format!("-{:.1} dB", bv.gr_db) };
                    gr_meter(ui, 488.0, "Gain reduction", bv.gr_db, 12.0, &gr);
                    ui.add_space(14.0);
                    let peak = if bv.peak_db < -90.0 { "-inf dBFS".to_string() } else { format!("{:.1} dBFS", bv.peak_db) };
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("OUTPUT PEAK").font(rr_ui::theme::font::label(11.0)).color(rr_ui::theme::T.muted));
                        ui.label(egui::RichText::new(peak).font(rr_ui::theme::font::mono(12.0)));
                    });
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
