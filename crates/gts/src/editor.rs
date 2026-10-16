//! GTS editor in the shared Riddim Rack look (`docs/design/DESIGN.md`, widgets from `rr-ui`).
//! Fixed [`WINDOW`] (1000x832: v2 added a unison row to each oscillator), laid out like
//! `crates/rr-ui/examples/preview.rs`.
//!
//! The UI is a plain function over the [`Ctl`] trait, so the plugin (over `ParamSetter`), the
//! screenshot example and the id-coverage test all render the same code. Every control talks to
//! its parameter by id; values travel normalized (0..1), readouts come from the param itself.

use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, EguiState};
use rr_ui::theme::metrics::*;
use rr_ui::widgets::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock, PoisonError, RwLock};

use crate::bus::BusLink;
use crate::dsp::shaper::Shaper;
use crate::dsp::tables::{Tables, GRAINS, N};
use crate::params::*;
use egui::{vec2, Id, Ui, Vec2};

/// Window size in logical px (fits 1080p at 125 % OS scaling: 864 px minus the host title bar).
pub const WINDOW: (u32, u32) = (1000, 832);

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
}

/// Safe read access to every param by id. Holds the `Arc` so the raw pointers stay valid.
pub struct ParamTable {
    _keep: Arc<GtsParams>,
    map: HashMap<String, ParamPtr>,
}

// SAFETY: every ParamPtr points into `_keep`, which is Send + Sync and outlives them; reads are atomic.
unsafe impl Send for ParamTable {}
unsafe impl Sync for ParamTable {}

impl ParamTable {
    pub fn new(params: Arc<GtsParams>) -> Self {
        let map = params.param_map().into_iter().map(|(id, ptr, _)| (id, ptr)).collect();
        Self { _keep: params, map }
    }
    fn ptr(&self, id: &str) -> ParamPtr {
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

fn kn(ui: &mut Ui, c: &dyn Ctl, id: &str, label: &str, style: KnobStyle) {
    let stepped = c.steps(id).is_some();
    let shown = if stepped {
        ui.data(|d| d.get_temp::<f32>(Id::new(("gts-drag", id)))).unwrap_or_else(|| c.get01(id))
    } else {
        c.get01(id)
    };
    let r = knob_styled(ui, label, shown, c.default01(id), &c.readout(id), style);
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

// ---- display ------------------------------------------------------------------------------

/// One grain as a closed, peak-normalized polyline of 513 points.
fn wave(tables: &Tables, table: Table, g: usize) -> Vec<f32> {
    let grain = tables.grain(table, g.min(GRAINS - 1));
    let peak = grain.iter().fold(1e-6f32, |m, s| m.max(s.abs()));
    let mut v: Vec<f32> = grain.iter().step_by(N / 512).map(|s| s / peak).collect();
    v.push(v[0]);
    v
}

/// Transfer curve of the shaper as it is currently set, y in 0..1 over x in -1..1.
fn shaper_curve(c: &dyn Ctl) -> Vec<f32> {
    let mut s = Shaper::new();
    // 1x: the oversampled paths delay by their latency, which would shift the plotted curve.
    s.set_oversampling(1);
    s.set(ShaperMode::from_index(enum_idx::<ShaperMode>(c, "shaper_mode")), c.get01("shaper_amt"));
    (0..=32).map(|i| (s.process(i as f32 / 16.0 - 1.0) * 0.5 + 0.5).clamp(0.0, 1.0)).collect()
}

// ---- PATCH strip (RR-BUS) ----------------------------------------------------------------

/// What the PATCH strip shows and edits besides params: the editor fills it from the bus and the
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
    /// Header settings row: shaper oversampling 1 / 2 / 4 (persisted host state, not a param).
    pub oversampling: usize,
    /// Header page switch: false = SYNTH, true = FX. Editor-session state, never persisted.
    pub fx_page: bool,
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

// ---- layout (mirrors rr-ui/examples/preview.rs) -------------------------------------------

/// Draws the whole panel into `ui` (the full [`WINDOW`]). Returns how far the body's content
/// reached past the space it was given (zero when everything fits; the layout test checks it).
pub fn draw(ui: &mut Ui, c: &dyn Ctl, tables: &Tables, current_index: u32, bv: &mut BusView) -> Vec2 {
    let status = format!("Two-osc graintable \u{b7} v{}", env!("CARGO_PKG_VERSION"));
    rr_ui::panel::rack_panel(ui, "GTS", "Graintable Synth", &status, vec2(232.0, 124.0), |hui, bui| {
        header(hui, c, tables, current_index, bv);
        let room = bui.max_rect();
        if bv.fx_page {
            fx_body(bui, c);
        } else {
            body(bui, c);
        }
        (bui.min_rect().max - room.max).max(Vec2::ZERO)
    })
}

fn header(ui: &mut Ui, c: &dyn Ctl, tables: &Tables, current_index: u32, bv: &mut BusView) {
    let t_idx = enum_idx::<Table>(c, "osca_table");
    let table = Table::from_index(t_idx);
    let idx = current_index.min(127);
    let g = ((idx as f32 / 127.0 * (GRAINS - 1) as f32).round() as usize).min(GRAINS - 1);
    let main = wave(tables, table, g);
    let prev = wave(tables, table, g.saturating_sub(1));
    let next = wave(tables, table, (g + 1).min(GRAINS - 1));
    // Grain display | settings column 104 px (page switch over Quality) | PATCH strip 300 px.
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        let (pw, qw) = (300.0, 104.0);
        ui.allocate_ui(vec2(ui.available_width() - pw - qw - 2.0 * GAP, DISPLAY_H), |ui| {
            waveform_display_ex(
                ui,
                &main,
                &[&prev, &next],
                idx as f32 / 127.0,
                &format!("Osc A  /  {}", Table::variants()[t_idx]),
                &format!("Index {idx}"),
            );
        });
        ui.allocate_ui(vec2(qw, DISPLAY_H), |ui| {
            ui.vertical(|ui| {
                ui.set_min_width(qw); // the column keeps its width whatever its content (PATCH stays flush right)
                if let Some(i) = segmented(ui, &["Synth", "FX"], bv.fx_page as usize) {
                    bv.fx_page = i == 1;
                }
                ui.add_space(12.0);
                let sel = QUALITY.iter().position(|&q| q == bv.oversampling).unwrap_or(1);
                if let Some(i) = combo_stacked(ui, "Quality", sel, &["1x", "2x", "4x"]) {
                    bv.oversampling = QUALITY[i];
                }
            });
        });
        ui.allocate_ui(vec2(pw, PATCH_H), |ui| patch(ui, c, bv));
    });
}

fn body(ui: &mut Ui, c: &dyn Ctl) {
    let sig = KnobStyle::default();
    let bip = KnobStyle::default().bipolar();
    ui.spacing_mut().item_spacing = vec2(GAP, GAP);
    // Row 1 is three knob rows tall: 44 chrome + 3 x 67 + 2 x 6.
    let (h1, h2, h3, h4) = (257.0, 141.0, 111.0, 111.0);

    // Row 1: oscillators (2 x 400: 7 x 52 = 364 of 376 inner) + Sub (88) = 912 with gaps.
    ui.horizontal_top(|ui| {
        let d = KnobStyle::default().cell(52.0);
        let u = KnobStyle::default().cell(50.0);
        for (title, p) in [("Osc A", "osca"), ("Osc B", "oscb")] {
            let k = |s: &str| format!("{p}_{s}");
            group_ext(ui, vec2(400.0, h1), title, |ui| {
                tg(ui, c, &k("on"), "");
                cb::<Table>(ui, c, &k("table"), "");
            }, |ui| {
                knob_row(ui, |ui| {
                    kn(ui, c, &k("oct"), "Oct", d.bipolar());
                    kn(ui, c, &k("semi"), "Semi", d);
                    kn(ui, c, &k("cent"), "Cent", d.bipolar());
                    kn(ui, c, &k("index"), "Index", d);
                    kn(ui, c, &k("motion"), "Motion", d.bipolar());
                    kn(ui, c, &k("shift"), "Shift", d.bipolar());
                    kn(ui, c, &k("vol"), "Vol", d);
                });
                knob_row(ui, |ui| {
                    kn(ui, c, &k("att"), "Att", d);
                    kn(ui, c, &k("dec"), "Dec", d);
                    kn(ui, c, &k("sus"), "Sus", d);
                    kn(ui, c, &k("rel"), "Rel", d);
                    kn(ui, c, &k("curve"), "Curve", d.bipolar());
                });
                // Unison: 6 x 50 + 8 + Reset LED (~66) = 374 of 376.
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    knob_row(ui, |ui| {
                        kn(ui, c, &k("unison"), "Unison", u);
                        kn(ui, c, &k("detune"), "Detune", u);
                        kn(ui, c, &k("det_curve"), "D.Crv", u);
                        kn(ui, c, &k("blend"), "Blend", u);
                        kn(ui, c, &k("width"), "Width", u);
                        kn(ui, c, &k("rand_phase"), "Rand", u);
                    });
                    ui.vertical(|ui| {
                        ui.add_space(KNOB_LABEL_H + (KNOB_D - CONTROL_H) / 2.0);
                        tg(ui, c, &k("phase_reset"), "Reset");
                    });
                });
            });
        }
        // Sub: one knob column, Level / Oct / Xover (the mono crossover lives with the sub).
        group_ext(ui, vec2(88.0, h1), "Sub", |ui| tg(ui, c, "sub_on", ""), |ui| {
            kn(ui, c, "sub_level", "Level", sig);
            kn(ui, c, "sub_oct", "Oct", bip);
            kn(ui, c, "mono_xover", "Xover", sig);
        });
    });

    // Row 2: modulators + shaper
    ui.horizontal_top(|ui| {
        let dense = KnobStyle::default().cell(52.0);
        for (title, p, w) in [("Mod A", "moda", 316.0), ("Mod B", "modb", 356.0)] {
            let k = |s: &str| format!("{p}_{s}");
            group_ext(ui, vec2(w, h2), title, |ui| cb::<Curve>(ui, c, &k("curve"), ""), |ui| {
                strip(ui, |ui| {
                    tg(ui, c, &k("sync"), "Sync");
                    cb::<Division>(ui, c, &k("div"), "");
                    tg(ui, c, &k("oneshot"), "1-shot");
                    cb::<Target>(ui, c, &k("target"), "");
                });
                knob_row(ui, |ui| {
                    if p == "moda" {
                        kn(ui, c, "moda_rate", "Rate", sig);
                        kn(ui, c, "moda_pitch", "Pitch", bip);
                        kn(ui, c, "moda_index", "Index", bip);
                        kn(ui, c, "moda_shift", "Shift", bip);
                    } else {
                        kn(ui, c, "modb_rate", "Rate", dense);
                        for (id, l) in [("motion", "Motion"), ("vol", "Vol"), ("cutoff", "Cutoff"), ("moda", "Mod A")] {
                            kn(ui, c, &k(id), l, dense.bipolar());
                        }
                    }
                });
            });
        }
        group_ext(ui, vec2(216.0, h2), "Shaper", |ui| tg(ui, c, "shaper_on", ""), |ui| {
            strip(ui, |ui| cb::<ShaperMode>(ui, c, "shaper_mode", ""));
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                kn(ui, c, "shaper_amt", "Amt", sig);
                ui.vertical(|ui| {
                    ui.add_space(KNOB_LABEL_H + (KNOB_D - 48.0) / 2.0);
                    curve_preview(ui, vec2(64.0, 48.0), &shaper_curve(c));
                });
            });
        });
    });

    // Row 3: filters (2 x 276: type cell 80 + 3 x 56 + 24) + Filter Env (6 x 52 + 24 = 336).
    ui.horizontal_top(|ui| {
        for (title, p) in [("Filter A", "filta"), ("Filter B", "filtb")] {
            let k = |s: &str| format!("{p}_{s}");
            group_ext(ui, vec2(276.0, h3), title, |ui| tg(ui, c, &k("env"), "Env"), |ui| {
                knob_row(ui, |ui| {
                    cb::<FilterType>(ui, c, &k("type"), "Type");
                    kn(ui, c, &k("freq"), "Freq", sig);
                    kn(ui, c, &k("res"), "Res", sig);
                    kn(ui, c, &k("kbd"), "Kbd", sig);
                });
            });
        }
        group_ext(ui, vec2(336.0, h3), "Filter Env", |ui| tg(ui, c, "fenv_inv", "Inv"), |ui| {
            knob_row(ui, |ui| {
                let d = KnobStyle::default().cell(52.0);
                for (id, l) in [("att", "Att"), ("dec", "Dec"), ("sus", "Sus"), ("rel", "Rel"), ("amt", "Amt")] {
                    kn(ui, c, &format!("fenv_{id}"), l, d);
                }
                kn(ui, c, "fenv_curve", "Curve", d.bipolar());
            });
        });
    });

    // Row 4: voice, wheel, velocity, output
    ui.horizontal_top(|ui| {
        let d = KnobStyle::default().cell(44.0);
        let dv = KnobStyle::default().cell(50.0);
        group_ext(ui, vec2(258.0, h4), "Voice", |ui| tg(ui, c, "legato", "Legato"), |ui| {
            knob_row(ui, |ui| {
                cb::<Routing>(ui, c, "routing", "Routing");
                kn(ui, c, "poly", "Poly", dv);
                kn(ui, c, "porta", "Porta", dv);
                kn(ui, c, "bend_range", "Bend", dv);
            });
        });
        group_ext(ui, vec2(200.0, h4), "Wheel", |_| {}, |ui| {
            knob_row(ui, |ui| {
                for (id, l) in [("index", "Index"), ("shift", "Shift"), ("cutoff", "Cutoff"), ("mod", "Mod")] {
                    kn(ui, c, &format!("wheel_{id}"), l, d.bipolar());
                }
            });
        });
        group_ext(ui, vec2(290.0, h4), "Velocity", |_| {}, |ui| {
            knob_row(ui, |ui| {
                for (id, l) in [("vol_a", "Vol A"), ("vol_b", "Vol B"), ("fenv", "F.Env"), ("att", "Att"), ("shift", "Shift"), ("mod", "Mod")] {
                    kn(ui, c, &format!("vel_{id}"), l, d.bipolar());
                }
            });
        });
        group_ext(ui, vec2(128.0, h4), "Output", |_| {}, |ui| {
            knob_row(ui, |ui| {
                let o = KnobStyle::default().cell(52.0);
                kn(ui, c, "volume", "Volume", o);
                kn(ui, c, "spread", "Spread", o);
            });
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
fn fx_body(ui: &mut Ui, c: &dyn Ctl) {
    let (k, w) = (KnobStyle::default().cell(56.0), KnobStyle::default().cell(60.0));
    let (kb, wb) = (k.bipolar(), w.bipolar());
    ui.spacing_mut().item_spacing = vec2(GAP, GAP);
    let (h1, h2, h3) = (257.0, 184.0, 191.0);

    // Row 1: Distortion 272 + Multiband 384 + EQ 232 + 2 gaps = 912.
    ui.horizontal_top(|ui| {
        group_ext(ui, vec2(272.0, h1), "Distortion", |ui| tg(ui, c, "dist_on", ""), |ui| {
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
            knob_row(ui, |ui| {
                kn(ui, c, "cho_mix", "Mix", k);
                kn(ui, c, "cho_voices", "Voices", k);
                kn(ui, c, "cho_rate", "Rate", k);
                kn(ui, c, "cho_depth", "Depth", k);
                kn(ui, c, "cho_delay", "Delay", k);
            });
            knob_row(ui, |ui| {
                tg_cell(ui, c, "cho_sync", "Sync");
                cb::<Division>(ui, c, "cho_div", "Div");
                kn(ui, c, "cho_feedback", "Feedback", kb);
                kn(ui, c, "cho_lpf", "LPF", w);
                kn(ui, c, "cho_wet_lowcut", "Low Cut", w);
            });
        });
        group_ext(ui, vec2(200.0, h2), "Dimension", |ui| tg(ui, c, "dim_on", ""), |ui| {
            knob_row(ui, |ui| {
                kn(ui, c, "dim_mix", "Mix", k);
                kn(ui, c, "dim_size", "Size", k);
                kn(ui, c, "dim_wet_lowcut", "Low Cut", w);
            });
        });
        group_ext(ui, vec2(352.0, h2), "Delay", |ui| tg(ui, c, "dly_on", ""), |ui| {
            knob_row(ui, |ui| {
                kn(ui, c, "dly_mix", "Mix", k);
                kn(ui, c, "dly_time", "Time", KnobStyle::default().cell(66.0));
                kn(ui, c, "dly_feedback", "Feedback", k);
                kn(ui, c, "dly_hp", "HP", w);
                kn(ui, c, "dly_lp", "LP", w);
            });
            knob_row(ui, |ui| {
                tg_cell(ui, c, "dly_sync", "Sync");
                cb::<DelayDiv>(ui, c, "dly_div", "Div");
                cb::<DelayFeel>(ui, c, "dly_feel", "Feel");
                cb::<DelayMode>(ui, c, "dly_mode", "Mode");
            });
        });
    });

    // Row 3: Reverb + Limiter.
    ui.horizontal_top(|ui| {
        group_ext(ui, vec2(312.0, h3), "Reverb", |ui| tg(ui, c, "rev_on", ""), |ui| {
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
        group_ext(ui, vec2(88.0, h3), "Limiter", |_| {}, |ui| {
            kn(ui, c, "lim_gain", "Gain", w);
        });
    });
}

fn rd<T: Clone>(l: &RwLock<T>) -> T {
    l.read().unwrap_or_else(PoisonError::into_inner).clone()
}

fn wr<T>(l: &RwLock<T>, v: T) {
    *l.write().unwrap_or_else(PoisonError::into_inner) = v;
}

/// Editor-side state: bus view and its refresh clock.
pub struct EditorState {
    view: BusView,
    next_refresh: f64,
}

/// `tables` is the engine's own slot: the display reads the audio tables (no second 29 MB copy).
/// If the editor opens before `initialize`, the GUI thread generates them and the engine reuses them.
pub fn create(
    params: Arc<GtsParams>,
    current_index: Arc<AtomicU32>,
    link: Arc<BusLink>,
    tables: Arc<OnceLock<Tables>>,
) -> Option<Box<dyn Editor>> {
    let table = ParamTable::new(params.clone());
    create_egui_editor(
        EguiState::from_size(WINDOW.0, WINDOW.1),
        EditorState { view: BusView::default(), next_refresh: 0.0 },
        |ctx, st| {
            rr_ui::theme::apply(ctx);
            st.next_refresh = 0.0; // egui time restarts at 0 on every open
        },
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
            let before = st.view.clone();

            let ctl = PlugCtl { table: &table, setter };
            let tables = tables.get_or_init(|| {
                let mut t = Tables::new();
                t.generate();
                t
            });
            egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
                draw(ui, &ctl, tables, current_index.load(Ordering::Relaxed), &mut st.view);
            });

            if st.view.oversampling != before.oversampling {
                params.oversampling.store(st.view.oversampling, Ordering::Relaxed);
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
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
        },
    )
}
