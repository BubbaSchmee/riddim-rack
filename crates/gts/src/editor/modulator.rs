//! The MODULATOR module (`docs/design/MOTION-addendum-track7.md` 2, 3, 9): one 336 x 260 well at
//! (620, 80) with tabs for Mod A to Mod D and the mod envelope in their source colours, a toolbar
//! (library, STEP, SNAP, GRID), the shape editor and the controls row.
//!
//! The drawn shapes live in `params.lfo_shapes` (state, not parameters). Every edit goes through
//! `MatrixShared::edit_shapes`, which calls `ShapesShared::edit` and records the change like a
//! matrix edit: `matrix_rev` moves, so the host marks the Live Set modified and keeps one undo step
//! per gesture (`begin_gesture` / `end_gesture` bracket a drag). The mod envelope and the LFO
//! extensions are ordinary parameters and go through the usual host gestures.
//!
//! State that is only about the view (selected tab, SNAP, GRID, the selected point, the library
//! popover, the drag in progress) lives in egui memory, so the plugin, the previews and the
//! headless tests run the same code.

use super::*;
use crate::dsp::lfo::{beats, Lfo};
use crate::dsp::shape::{library, Mode, ShapeDef, MAX_POINTS, MAX_STEPS};
use crate::matrix::Source;
use egui::Shape;
use rr_ui::theme::SRC_MOD;

// ---- geometry ----------------------------------------------------------------------------------

/// The module in window px.
pub const RECT: (f32, f32, f32, f32) = (620.0, 80.0, 336.0, 260.0);
/// The shape editor glass, local to the module.
const GLASS: (f32, f32, f32, f32) = (12.0, 68.0, 312.0, 130.0);
/// The plot, local to the glass: x 28..304, y 26..114 (value +1 at the top, 0 at y 70).
const PLOT: (f32, f32, f32, f32) = (28.0, 26.0, 276.0, 88.0);
/// Tab strip cells: local x and width of LFO A, B, C, D and MOD ENV (2 px padding and gap).
const TAB_X: [f32; 5] = [14.0, 72.0, 130.0, 188.0, 246.0];
const TAB_W: [f32; 5] = [56.0, 56.0, 56.0, 56.0, 76.0];
const TAB_NAMES: [&str; 5] = ["LFO A", "LFO B", "LFO C", "LFO D", "MOD ENV"];
const TAB_SRC: [Source; 5] = [Source::ModA, Source::ModB, Source::ModC, Source::ModD, Source::ModEnv];
const PRE: [&str; 4] = ["moda", "modb", "modc", "modd"];
/// Toolbar row, local y and height.
const BAR_Y: f32 = 40.0;
const BAR_H: f32 = 24.0;
/// Controls row, local y.
const ROW_Y: f32 = 204.0;
/// Grid choices (fractions of the cycle, name, slots): note values at DIV 1/1 and the triplets.
pub const GRIDS: [(&str, u32); 8] = [("1/2", 2), ("1/4", 4), ("1/8", 8), ("1/16", 16), ("1/32", 32), ("1/4T", 6), ("1/8T", 12), ("1/16T", 24)];

fn origin() -> Pos2 {
    pos2(RECT.0, RECT.1)
}

/// The module rect with its top-left at window `o`.
fn module_at(o: Pos2) -> Rect {
    Rect::from_min_size(o, vec2(RECT.2, RECT.3))
}

fn local(m: Rect, r: (f32, f32, f32, f32)) -> Rect {
    Rect::from_min_size(m.min + vec2(r.0, r.1), vec2(r.2, r.3))
}

/// The editor glass in window px (origin at the window's top-left).
pub fn glass() -> Rect {
    local(module_at(origin()), GLASS)
}

/// The plot in window px.
pub fn plot() -> Rect {
    local(glass(), PLOT)
}

/// The tab cell of `tab` in window px.
pub fn tab_rect(tab: usize) -> Rect {
    Rect::from_min_size(origin() + vec2(TAB_X[tab], 8.0 + 2.0), vec2(TAB_W[tab], 22.0))
}

/// Toolbar buttons of the LFO tabs.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tool {
    Lib,
    Step,
    Snap,
    Grid,
    Steps,
    Glide,
}

/// Where a toolbar button is, in window px: `steps` = the toolbar of step mode.
pub fn tool_rect(t: Tool, steps: bool) -> Rect {
    let (x, w) = match (t, steps) {
        (Tool::Lib, false) => (12.0, 126.0),
        (Tool::Step, false) => (144.0, 52.0),
        (Tool::Snap, false) => (202.0, 52.0),
        (Tool::Grid, _) => (260.0, 64.0),
        (Tool::Lib, true) => (12.0, 74.0),
        (Tool::Step, true) => (92.0, 52.0),
        (Tool::Snap, true) => (150.0, 52.0),
        (Tool::Steps, _) => (208.0, 46.0),
        (Tool::Glide, _) => (260.0, 64.0),
    };
    Rect::from_min_size(origin() + vec2(x, BAR_Y), vec2(w, BAR_H))
}

/// The window position of a point `[x, y, curve]` of the shape editor (points mode).
pub fn point_pos(p: [f32; 3]) -> Pos2 {
    let pl = plot();
    pos2(pl.left() + p[0] * pl.width(), pl.center().y - p[1] * pl.height() / 2.0)
}

/// The window position of the middle of step `k` of `n` at level `v`.
pub fn step_pos(k: usize, n: usize, v: f32) -> Pos2 {
    let pl = plot();
    pos2(pl.left() + (k as f32 + 0.5) / n as f32 * pl.width(), pl.center().y - v * pl.height() / 2.0)
}

// ---- view state --------------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum Drag {
    /// Point `i`: its value at the press and the pointer travel since (Shift-scaled per event).
    Point { i: usize, start: [f32; 2], acc: Vec2 },
    /// Segment `seg`'s curve.
    Curve { seg: usize, start: f32, acc: f32 },
    /// The phase marker (param `modX_phase`, one host gesture).
    Phase { acc: f32, start: f32 },
    /// Painting step bars; the last bar painted.
    Steps { last: Option<usize> },
    /// The glide slider.
    Glide { start: f32, acc: f32 },
    /// An envelope handle: 0 attack, 1 decay / sustain corner, 2 release end, 3 a curve diamond.
    Env { h: u8, acc: Vec2, start: [f32; 3] },
}

#[derive(Clone)]
struct St {
    /// The tab asked for and the one drawn (they differ while the editor content cross-fades).
    tab: usize,
    shown: usize,
    t0: f64,
    snap: bool,
    grid: usize,
    /// Selected point per LFO.
    sel: [Option<usize>; 4],
    drag: Option<Drag>,
    /// The library popover is open.
    lib: bool,
    /// Context menu of a segment: where and which.
    seg_menu: Option<(Pos2, usize)>,
    /// The value pill: text, anchor and when it was last live.
    pill: Option<(String, Pos2, f64)>,
    /// A host gesture of ours is open (a drag in progress).
    gesture: bool,
    /// A library load morphing: the LFO, the shape it replaced and when (egui time).
    morph: Option<(usize, ShapeDef, f64)>,
    /// A point just added (LFO, index, when) grows in over 100 ms; a point just removed (LFO, where, when) fades out over 90 ms.
    added: Option<(usize, usize, f64)>,
    removed: Option<(usize, Pos2, f64)>,
    /// The param whose host gesture a drag has open.
    param: Option<String>,
}

impl Default for St {
    fn default() -> Self {
        Self { tab: 0, shown: 0, t0: -1.0, snap: true, grid: 2, sel: [None; 4], drag: None, lib: false, seg_menu: None, pill: None, gesture: false, param: None, morph: None, added: None, removed: None }
    }
}

fn st_id() -> Id {
    Id::new("gts-mod-state")
}

fn load(ctx: &egui::Context) -> St {
    ctx.data(|d| d.get_temp::<St>(st_id())).unwrap_or_default()
}

fn store(ctx: &egui::Context, st: St) {
    ctx.data_mut(|d| d.insert_temp(st_id(), st));
}

/// CPU time of the last frame's shape editor (input, glass and playheads, not the controls row), ms
/// (the gate of MOTION-addendum-track7.md 10.2 is 0.3 ms).
pub fn editor_cost_ms(ctx: &egui::Context) -> f32 {
    ctx.data(|d| d.get_temp::<f32>(Id::new("gts-mod-cost"))).unwrap_or(0.0)
}

fn stash_cost(ctx: &egui::Context, t0: std::time::Instant) {
    let ms = t0.elapsed().as_secs_f32() * 1e3;
    ctx.data_mut(|d| d.insert_temp(Id::new("gts-mod-cost"), ms));
}

/// Selects a tab from outside (previews and tests).
pub fn select_tab(ctx: &egui::Context, tab: usize) {
    let mut st = load(ctx);
    st.tab = tab.min(4);
    st.shown = st.tab;
    st.t0 = -1.0;
    store(ctx, st);
}

/// Opens or closes the library popover from outside (previews and tests).
pub fn open_library(ctx: &egui::Context, open: bool) {
    let mut st = load(ctx);
    st.lib = open;
    store(ctx, st);
}

/// The selected tab and whether the library is open (tests).
pub fn view(ctx: &egui::Context) -> (usize, bool) {
    let st = load(ctx);
    (st.tab, st.lib)
}

/// SNAP on / off and the GRID index (tests).
pub fn snap_grid(ctx: &egui::Context) -> (bool, usize) {
    let st = load(ctx);
    (st.snap, st.grid)
}

/// Previews and tests: SNAP and GRID without clicking.
pub fn set_snap_grid(ctx: &egui::Context, snap: bool, grid: usize) {
    let mut st = load(ctx);
    st.snap = snap;
    st.grid = grid.min(GRIDS.len() - 1);
    store(ctx, st);
}

// ---- shape evaluation (display side, the arithmetic of `dsp::shape::Shape::eval`) -----------------

fn bend(t: f32, c: f32) -> f32 {
    if c == 0.0 { t } else { t.powf(8f32.powf(c)) }
}

/// Value of a points shape at `x` (0..1), as the DSP computes it.
pub fn eval_points(pts: &[[f32; 3]], x: f32) -> f32 {
    let n = pts.len();
    if n == 0 {
        return 0.0;
    }
    let i = pts.partition_point(|p| p[0] <= x);
    let (l, r, xl, xr) = match i {
        0 => (n - 1, 0, pts[n - 1][0] - 1.0, pts[0][0]),
        _ if i == n => (n - 1, 0, pts[n - 1][0], pts[0][0] + 1.0),
        _ => (i - 1, i, pts[i - 1][0], pts[i][0]),
    };
    let t = if xr > xl { ((x - xl) / (xr - xl)).clamp(0.0, 1.0) } else { 0.0 };
    pts[l][1] + (pts[r][1] - pts[l][1]) * bend(t, pts[l][2])
}

/// Value of a step shape at `x` (0..1) with `glide` (the last `glide` of a step ramps to the next).
pub fn eval_steps(steps: &[f32], glide: f32, x: f32) -> f32 {
    let n = steps.len();
    if n == 0 {
        return 0.0;
    }
    if x >= 1.0 {
        return steps[n - 1];
    }
    let p = x.max(0.0) * n as f32;
    let k = (p as usize).min(n - 1);
    let (f, v) = (p - k as f32, steps[k]);
    if glide > 0.0 && f > 1.0 - glide {
        return v + (steps[(k + 1) % n] - v) * ((f - (1.0 - glide)) / glide).min(1.0);
    }
    v
}

/// Value of any shape at `x` (0..1).
pub fn eval_def(def: &ShapeDef, x: f32) -> f32 {
    if def.mode == Mode::Steps {
        eval_steps(&def.steps, def.glide, x)
    } else {
        eval_points(&def.points, x.min(0.99999))
    }
}

/// The polyline of a shape over 0..1 in (x, value): the points themselves, with samples inside the
/// segments that are curved, and the wrap from the last point to the first.
pub fn shape_line(def: &ShapeDef) -> Vec<(f32, f32)> {
    let mut out = vec![];
    if def.mode == Mode::Steps {
        let n = def.steps.len().max(1);
        let g = def.glide.clamp(0.0, 1.0);
        for (k, &v) in def.steps.iter().enumerate() {
            let (x0, x1) = (k as f32 / n as f32, (k + 1) as f32 / n as f32);
            let next = def.steps[(k + 1) % n];
            out.push((x0, v));
            if g > 0.0 {
                out.push((x0 + (x1 - x0) * (1.0 - g), v));
                out.push((x1, next));
            } else {
                out.push((x1, v));
                if k + 1 < n {
                    out.push((x1, next));
                }
            }
        }
        return out;
    }
    let pts = &def.points;
    if pts.is_empty() {
        return out;
    }
    if pts[0][0] > 0.0 {
        out.push((0.0, eval_points(pts, 0.0)));
    }
    for (i, p) in pts.iter().enumerate() {
        out.push((p[0], p[1]));
        if let Some(q) = pts.get(i + 1) {
            if p[2] != 0.0 && q[0] > p[0] {
                // One sample per 24 px of the 276 px plot (4 to 12): enough for a smooth bend, few vertices.
                let n = (((q[0] - p[0]) * 276.0 / 24.0).ceil() as usize).clamp(4, 12);
                for s in 1..n {
                    let t = s as f32 / n as f32;
                    out.push((p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * bend(t, p[2])));
                }
            }
        }
    }
    let last = pts[pts.len() - 1];
    if last[0] < 1.0 {
        // The wrap back to the first point: a curved last segment is drawn through the end.
        if last[2] != 0.0 {
            for s in 1..=12 {
                let x = last[0] + (1.0 - last[0]) * s as f32 / 12.0;
                out.push((x, eval_points(pts, x.min(0.99999))));
            }
        } else {
            out.push((1.0, eval_points(pts, 0.99999)));
        }
    }
    out
}

/// Thumbnails of the stock shapes, 36 samples each (the library popover).
fn thumbs() -> &'static Vec<Vec<f32>> {
    static T: OnceLock<Vec<Vec<f32>>> = OnceLock::new();
    T.get_or_init(|| library().iter().map(|(_, d)| (0..36).map(|i| d.eval(i as f64 / 35.0)).collect()).collect())
}

/// A built-in curve as a shape: sampled into at most 33 points so it can be edited. `None` for the
/// random curves and Custom, which have no fixed shape.
pub fn shape_of_curve(curve: Curve) -> Option<ShapeDef> {
    let samples: &[f32] = match curve {
        Curve::Sine => &[0.0, 1.0 / 16.0, 2.0 / 16.0, 3.0 / 16.0, 4.0 / 16.0, 5.0 / 16.0, 6.0 / 16.0, 7.0 / 16.0, 8.0 / 16.0, 9.0 / 16.0, 10.0 / 16.0, 11.0 / 16.0, 12.0 / 16.0, 13.0 / 16.0, 14.0 / 16.0, 15.0 / 16.0, 1.0],
        Curve::Triangle => &[0.0, 0.25, 0.75, 1.0],
        Curve::SawUp | Curve::SawDown => &[0.0, 1.0],
        Curve::Square => &[0.0, 0.4999, 0.5, 1.0],
        Curve::Decay | Curve::Attack => &[0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0],
        _ => return None,
    };
    let pts: Vec<[f32; 3]> = samples.iter().map(|&x| [x, Lfo::curve_at(curve, x as f64).clamp(-1.0, 1.0), 0.0]).collect();
    Some(ShapeDef::points(&pts))
}

// ---- small widgets -----------------------------------------------------------------------------

fn px(p: &egui::Painter) -> f32 {
    1.0 / p.ctx().pixels_per_point()
}

fn snap_px(p: &egui::Painter, v: f32) -> f32 {
    let q = px(p);
    (v / q).round() * q
}

/// A toggle with an LED and a caps label in exactly `rect`. Returns (clicked, hovered).
pub(super) fn toggle_at(ui: &mut Ui, rect: Rect, key: &str, label: &str, on: bool) -> (bool, bool) {
    let resp = ui.interact(rect, Id::new(("gts-mod-toggle", key)), Sense::click());
    let p = ui.painter().clone();
    rr_ui::paint::inset_box(&p, rect, resp.hovered());
    let k = rr_ui::motion::tween(ui.ctx(), resp.id.with("led"), on, 0.18);
    rr_ui::paint::led_level(&p, pos2(rect.left() + 11.0, rect.center().y), k);
    rr_ui::paint::tracked(&p, pos2(rect.left() + 21.0, rect.center().y), Align2::LEFT_CENTER, &label.to_uppercase(), font::label(11.0), if on { T.text } else { T.muted }, 0.3);
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    (resp.clicked(), resp.hovered())
}

/// One row of a [`dropdown`] list.
pub(super) enum Row<'a> {
    Item(&'a str),
    Header(&'a str),
}

/// Text style of a [`dropdown`]: 13 px, 11 px caps (toolbars), or 12 px with tight padding (narrow cells).
#[derive(Clone, Copy, PartialEq)]
pub(super) enum DdStyle {
    Normal,
    Caps,
    Compact,
}

/// An inset dropdown in exactly `rect`, showing `shown`; `rows` is the list
/// (headers are not pickable); returns the index among the `Item` rows that was picked, and whether
/// the pointer is over the closed control. Long lists scroll.
pub(super) fn dropdown(ui: &mut Ui, rect: Rect, key: &str, shown: &str, style: DdStyle, rows: &[Row], selected: Option<usize>) -> (Option<usize>, bool) {
    let resp = ui.interact(rect, Id::new(("gts-mod-dd", key)), Sense::click());
    let popup_id = resp.id.with("popup");
    if resp.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup_id));
    }
    let open = ui.memory(|m| m.is_popup_open(popup_id));
    let p = ui.painter().clone();
    rr_ui::paint::inset_box(&p, rect, resp.hovered() || open);
    let compact = style == DdStyle::Compact;
    let clip = Rect::from_min_max(rect.min, pos2(rect.right() - if compact { 17.0 } else { 20.0 }, rect.bottom()));
    let pc = p.with_clip_rect(clip.intersect(p.clip_rect()));
    let tx = rect.left() + if compact { 6.0 } else { 8.0 };
    match style {
        DdStyle::Caps => {
            rr_ui::paint::tracked(&pc, pos2(tx, rect.center().y), Align2::LEFT_CENTER, &shown.to_uppercase(), font::label(11.0), T.text, 0.5);
        }
        _ => {
            pc.text(pos2(tx, rect.center().y), Align2::LEFT_CENTER, shown, font::label(if compact { 12.0 } else { 13.0 }), T.text);
        }
    }
    let s = 1.0 - 2.0 * rr_ui::motion::tween(ui.ctx(), popup_id.with("chev"), open, 0.18);
    let (cx, cy) = (rect.right() - 11.5, rect.center().y - 1.0);
    p.add(Shape::line(vec![pos2(cx - 3.5, cy - 1.5 * s), pos2(cx, cy + 2.0 * s), pos2(cx + 3.5, cy - 1.5 * s)], rr_ui::paint::st(1.5, T.muted)));
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let mut picked = None;
    let fade = rr_ui::motion::tween(ui.ctx(), popup_id.with("fade"), open, if open { 0.13 } else { 0.09 });
    egui::popup::popup_below_widget(ui, popup_id, &resp, egui::popup::PopupCloseBehavior::CloseOnClick, |ui| {
        ui.multiply_opacity(fade);
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        ui.set_min_width((rect.width() - 8.0).max(60.0));
        let f = font::label(13.0);
        egui::ScrollArea::vertical().max_height(264.0).auto_shrink([true, true]).show(ui, |ui| {
            let mut idx = 0;
            for row in rows {
                match row {
                    Row::Header(h) => {
                        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
                        rr_ui::paint::tracked(ui.painter(), pos2(r.left() + 8.0, r.center().y + 2.0), Align2::LEFT_CENTER, &h.to_uppercase(), font::strong(10.0), T.title, 1.2);
                    }
                    Row::Item(name) => {
                        let (r, rr) = ui.allocate_exact_size(vec2(ui.available_width(), CONTROL_H), Sense::click());
                        if rr.hovered() {
                            ui.painter().rect_filled(r, 3.0, T.a(0.16));
                        }
                        let on = selected == Some(idx);
                        ui.painter().text(pos2(r.left() + 8.0, r.center().y), Align2::LEFT_CENTER, name, f.clone(), if on { T.accent } else { T.text });
                        if rr.clicked() && !on {
                            picked = Some(idx);
                        }
                        idx += 1;
                    }
                }
            }
        });
    });
    (picked, resp.hovered())
}

/// Places `add` in a child ui at exactly `r` (cells do their own spacing).
pub(super) fn put<R>(ui: &mut Ui, r: Rect, add: impl FnOnce(&mut Ui) -> R) -> R {
    let mut child = ui.new_child(UiBuilder::new().max_rect(r).layout(egui::Layout::left_to_right(egui::Align::Min)));
    child.spacing_mut().item_spacing = vec2(0.0, 0.0);
    add(&mut child)
}

/// Pill with the value being edited, above the pointer (MOTION 5.7): live while `live`, held 400 ms,
/// faded out over 120 ms.
fn pill(ui: &Ui, st: &mut St, now: f64, live: Option<(String, Pos2)>) {
    if let Some((t, at)) = live {
        st.pill = Some((t, at, now));
    }
    let Some((t, at, last)) = st.pill.clone() else { return };
    let age = now - last;
    let k = if age <= 0.4 { 1.0 } else { (1.0 - (age - 0.4) / 0.12) as f32 };
    if k <= 0.0 {
        st.pill = None;
        return;
    }
    rr_ui::widgets::value_popup(ui.ctx(), pos2(at.x, at.y - 8.0), &t, k);
}

// ---- the module --------------------------------------------------------------------------------

/// The shape the editor shows for LFO `k`: its own when the curve is Custom, else the built-in curve
/// as points (no edit target), and whether it is editable as it stands.
struct Shown {
    def: ShapeDef,
    custom: bool,
}

fn shown_shape(c: &dyn Ctl, k: usize) -> Shown {
    let curve = Curve::from_index(enum_idx::<Curve>(c, &format!("{}_curve", PRE[k])));
    let own = c.matrix().and_then(|m| m.shape(k)).unwrap_or_default();
    if curve == Curve::Custom {
        return Shown { def: own, custom: true };
    }
    // Random and sample-and-hold curves have no fixed shape: their display pattern is drawn from steps.
    let def = shape_of_curve(curve).unwrap_or_else(|| {
        let steps: Vec<f32> = (0..4).map(|i| Lfo::curve_at(curve, (i as f64 + 0.5) / 4.0)).collect();
        if curve == Curve::SampleHold {
            ShapeDef::steps(&steps, 0.0)
        } else {
            let pts: Vec<[f32; 3]> = (0..=32).map(|i| [i as f32 / 32.0, Lfo::curve_at(curve, i as f64 / 32.0), 0.0]).collect();
            ShapeDef::points(&pts)
        }
    });
    Shown { def, custom: false }
}

/// The name the LIB combo shows: the stock shape that is loaded, "Custom", or the built-in curve.
fn lib_label(c: &dyn Ctl, k: usize, sh: &Shown) -> String {
    if !sh.custom {
        return Curve::variants()[enum_idx::<Curve>(c, &format!("{}_curve", PRE[k]))].to_string();
    }
    library().into_iter().find(|(_, d)| *d == sh.def).map_or_else(|| "Custom".to_string(), |(n, _)| n.to_string())
}

/// Seconds one cycle of LFO `k` takes at the host tempo.
fn cycle_seconds(c: &dyn Ctl, k: usize, tempo: f32) -> f32 {
    let pre = PRE[k];
    // Both are read whichever is in use (the other one is the next click of SYNC away).
    let div = Division::from_index(enum_idx::<Division>(c, &format!("{pre}_div")));
    let rate = c.plain(&format!("{pre}_rate"), c.get01(&format!("{pre}_rate")));
    if c.get01(&format!("{pre}_sync")) > 0.5 {
        (beats(div) as f32 * 60.0 / tempo.max(20.0)).max(0.001)
    } else {
        1.0 / rate.max(0.01)
    }
}

pub(super) fn modulator(ui: &mut Ui, c: &dyn Ctl, f: &VizFrame, now: f64, at: &dyn Fn(f32, f32, f32, f32) -> Rect) {
    let ctx = ui.ctx().clone();
    let mrect = at(RECT.0, RECT.1, RECT.2, RECT.3);
    let mut st = load(&ctx);
    let reduced = rr_ui::motion::reduced_motion(&ctx);
    // Tab change: the content fades out in 90 ms, the new one in over 180 ms (the plot never re-lays out).
    let mut alpha = 1.0;
    if st.t0 > now {
        st.t0 = -1.0;
        st.shown = st.tab;
    }
    if st.tab != st.shown && st.t0 < 0.0 {
        if reduced {
            st.shown = st.tab;
        } else {
            st.t0 = now;
        }
    }
    if st.t0 >= 0.0 {
        let t = (now - st.t0) as f32;
        if t < 0.09 {
            alpha = 1.0 - egui::emath::easing::cubic_in(t / 0.09);
        } else {
            st.shown = st.tab;
            alpha = egui::emath::easing::cubic_out(((t - 0.09) / 0.18).min(1.0));
            if t >= 0.27 {
                st.t0 = -1.0;
            }
        }
    }
    let tab = st.shown;
    let col = SRC_MOD[tab];
    rr_ui::widgets::well_at(ui, mrect, |ui, m| {
        tabs(ui, c, f, &mut st, m);
        ui.scope(|ui| {
            if alpha < 0.999 {
                ui.multiply_opacity(alpha);
            }
            if tab < 4 {
                lfo_body(ui, c, f, &mut st, m, tab, col, now);
            } else {
                env_body(ui, c, f, &mut st, m, col, now);
            }
        });
    });
    library_popover(ui, c, &mut st, mrect);
    segment_menu(ui, c, &mut st);
    // Safety net: a gesture of ours never outlives the pointer button.
    if st.gesture && !ctx.input(|i| i.pointer.any_down()) {
        if let Some(m) = c.matrix() {
            m.end_gesture();
        }
        st.gesture = false;
        st.drag = None;
        if let Some(p) = st.param.take() {
            c.end(&p);
        }
    }
    store(&ctx, st);
}

fn begin_gesture(c: &dyn Ctl, st: &mut St) {
    if let (false, Some(m)) = (st.gesture, c.matrix()) {
        m.begin_gesture();
        st.gesture = true;
    }
}

fn end_gesture(c: &dyn Ctl, st: &mut St) {
    if st.gesture {
        if let Some(m) = c.matrix() {
            m.end_gesture();
        }
        st.gesture = false;
    }
}

/// Edits LFO `k`'s shape through `ShapesShared::edit` (see the module docs).
fn edit(c: &dyn Ctl, k: usize, f: impl FnOnce(&mut ShapeDef)) {
    if let Some(m) = c.matrix() {
        m.edit_shapes(|s| f(&mut s.0[k]));
    }
}

// ---- tabs ----------------------------------------------------------------------------------------

/// The five tabs: a segmented control whose lit segment slides (180 ms) and takes the source
/// colour. The dot is the source's live value and a drag handle that patches the source.
fn tabs(ui: &mut Ui, c: &dyn Ctl, f: &VizFrame, st: &mut St, m: Rect) {
    let p = ui.painter().clone();
    let strip = Rect::from_min_size(m.min + vec2(12.0, 8.0), vec2(312.0, 26.0));
    rr_ui::paint::inset_box(&p, strip, false);
    // The lit segment slides between the cells and cross-fades its colour.
    let t = rr_ui::motion::tween_value(ui.ctx(), Id::new("gts-mod-tab-lit"), st.tab as f32, 0.18);
    let (i, fr) = ((t.floor() as usize).min(4), t - t.floor());
    let j = (i + 1).min(4);
    let (lx, lw) = (TAB_X[i] + (TAB_X[j] - TAB_X[i]) * fr, TAB_W[i] + (TAB_W[j] - TAB_W[i]) * fr);
    let lc = SRC_MOD[i].lerp_to_gamma(SRC_MOD[j], fr);
    let lit = Rect::from_min_size(m.min + vec2(lx, 10.0), vec2(lw, 22.0));
    p.rect_filled(lit, 3.0, lc.gamma_multiply(0.18));
    p.rect_stroke(lit, 3.0, rr_ui::paint::hair(&p, lc.gamma_multiply(0.6)), StrokeKind::Inside);
    let newest = f.newest();
    let mv = super::patching::mview(ui.ctx());
    for tab in 0..5 {
        let seg = Rect::from_min_size(m.min + vec2(TAB_X[tab], 10.0), vec2(TAB_W[tab], 22.0));
        let resp = ui.interact(seg, Id::new(("gts-mod-tab", tab)), Sense::click());
        let dot = pos2(seg.left() + 11.0, seg.center().y);
        let dr = ui.interact(Rect::from_center_size(dot, vec2(16.0, 18.0)), Id::new(("gts-mod-tab-dot", tab)), Sense::drag());
        if dr.drag_started() {
            super::patching::start_drag(ui.ctx(), TAB_SRC[tab]);
        }
        if dr.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if dr.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        } else if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if resp.clicked() && !dr.dragged() {
            st.tab = tab;
            st.seg_menu = None;
        }
        let on = st.tab == tab;
        // The dot follows the source's live value (0.35 to 1).
        let live = newest
            .map(|v| match tab {
                0..=3 => f.voice(v, [vf::OUT_A, vf::OUT_B, vf::OUT_C, vf::OUT_D][tab]).abs(),
                _ => f.voice(v, vf::MENV),
            })
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
        p.circle_filled(dot, 4.5, SRC_MOD[tab].gamma_multiply(0.35 + 0.65 * live));
        let lab = if on { T.text } else if resp.hovered() { T.text } else { T.muted };
        rr_ui::paint::tracked(&p, pos2(seg.left() + 20.0, seg.center().y), Align2::LEFT_CENTER, TAB_NAMES[tab], font::label(11.0), lab, 0.9);
        if resp.hovered() || dr.hovered() {
            let n = mv.as_ref().map_or(0, |mv| mv.routes_of(TAB_SRC[tab]));
            let value = if n == 0 { "no routings".to_string() } else { format!("{n} routing{}", if n == 1 { "" } else { "s" }) };
            set_hover(ui, TAB_NAMES[tab], &value, None, crate::help::ui_help("mod:tab").unwrap_or(""));
        }
    }
    let _ = c;
}

// ---- LFO tabs ------------------------------------------------------------------------------------

fn lfo_body(ui: &mut Ui, c: &dyn Ctl, f: &VizFrame, st: &mut St, m: Rect, k: usize, col: Color32, now: f64) {
    let t_start = std::time::Instant::now();
    let sh = shown_shape(c, k);
    let steps_mode = sh.def.mode == Mode::Steps;
    toolbar(ui, c, st, m, k, col, &sh);
    let g = local(m, GLASS);
    let pl = local(g, PLOT);
    let pre = PRE[k];
    let phase = c.get01(&format!("{pre}_phase"));
    let tempo = f.global(gf::TEMPO);
    let fade_s = c.plain(&format!("{pre}_fade"), c.get01(&format!("{pre}_fade")));
    let cycle_s = cycle_seconds(c, k, if tempo > 1.0 { tempo } else { 120.0 });
    let slots = GRIDS[st.grid].1;
    let _ = now;

    // ---- interaction (before painting, so the drawn state is this frame's) -----------------------
    let id = Id::new(("gts-mod-plot", k));
    let resp = ui.interact(pl.expand2(vec2(2.0, 0.0)), id, Sense::click_and_drag());
    let marker = Rect::from_center_size(pos2(pl.left() + phase * pl.width(), g.top() + 12.0), vec2(16.0, 20.0));
    let mresp = ui.interact(marker, Id::new(("gts-mod-phase", k)), Sense::click_and_drag());
    let mut hover = Hit::None;
    let (shift, alt) = ui.input(|i| (i.modifiers.shift, i.modifiers.alt));
    let eff_snap = (st.snap ^ alt) && !shift;
    if !steps_mode {
        if let Some(pos) = resp.hover_pos().filter(|_| st.drag.is_none() && !mresp.hovered()) {
            hover = hit_test(&sh.def, pl, pos);
        }
    }
    let can_edit = sh.custom || c.matrix().is_some();
    if can_edit {
        points_or_steps_input(ui, c, st, k, &sh, pl, &resp, &mresp, g, slots, eff_snap, shift, &mut hover);
    }

    // ---- painting --------------------------------------------------------------------------------
    let p = ui.painter().clone();
    rr_ui::scope::small_glass(&p, g);
    let pxl = px(&p);
    let sn = |v: f32| snap_px(&p, v);
    let clip_p = p.with_clip_rect(g.intersect(p.clip_rect()));
    // Fade-in wedge: a band over the plot and a triangle under it.
    let fade_w = if fade_s > 0.0 { (fade_s / cycle_s * pl.width()).min(pl.right() - (pl.left() + phase * pl.width())) } else { 0.0 };
    let px0 = pl.left() + phase * pl.width();
    if fade_w > 0.5 {
        clip_p.rect_filled(Rect::from_min_max(pos2(px0, pl.top()), pos2(px0 + fade_w, pl.bottom())), 0.0, col.gamma_multiply(0.06));
    }
    // Grid: slots 6 %, beats (quarters of the cycle) 12 %; value lines +-1 and +-0.5 at 6 %, the baseline 12 %.
    for s in 0..=slots {
        let x = pl.left() + pl.width() * s as f32 / slots as f32;
        let beat = (s * 4) % slots == 0;
        clip_p.vline(sn(x), pl.y_range(), Stroke::new(pxl, Color32::WHITE.gamma_multiply(if beat { 0.12 } else { 0.06 })));
    }
    for (v, a) in [(1.0f32, 0.06), (0.5, 0.06), (0.0, 0.12), (-0.5, 0.06), (-1.0, 0.06)] {
        clip_p.hline(pl.x_range(), sn(pl.center().y - v * pl.height() / 2.0), Stroke::new(pxl, Color32::WHITE.gamma_multiply(a)));
    }
    // Ruler ticks: one per slot (3 px, 25 %), one per beat (6 px, 50 %).
    let ry = g.top() + 21.0;
    for s in 0..=slots {
        let x = sn(pl.left() + pl.width() * s as f32 / slots as f32);
        let beat = (s * 4) % slots == 0;
        clip_p.vline(x, (ry - if beat { 6.0 } else { 3.0 })..=ry, Stroke::new(pxl, Color32::WHITE.gamma_multiply(if beat { 0.5 } else { 0.25 })));
    }
    let mono = font::mono(11.0);
    for (txt, v) in [("+1", 1.0f32), ("0", 0.0), ("-1", -1.0)] {
        clip_p.text(pos2(g.left() + 23.0, pl.center().y - v * pl.height() / 2.0), Align2::RIGHT_CENTER, txt, mono.clone(), T.disp_caption);
    }
    let caption = if steps_mode { format!("STEPS {} \u{b7} GLIDE {:.2}", sh.def.steps.len(), sh.def.glide) } else { format!("GRID {} \u{b7} {}", GRIDS[st.grid].0, slots) };
    clip_p.text(pos2(g.right() - 8.0, g.top() + 10.0), Align2::RIGHT_CENTER, caption, mono.clone(), T.disp_caption);

    // Step bars under the curve.
    if steps_mode {
        let n = sh.def.steps.len().max(1);
        let w = pl.width() / n as f32;
        let hov_bar = resp.hover_pos().filter(|q| pl.contains(*q)).map(|q| (((q.x - pl.left()) / w) as usize).min(n - 1));
        let playing = playing_step(f, k, n, &sh.def, c);
        for (b, &v) in sh.def.steps.iter().enumerate() {
            let x0 = pl.left() + b as f32 * w;
            let r = Rect::from_min_max(pos2(x0 + 1.5, pl.center().y.min(pl.center().y - v * pl.height() / 2.0)), pos2(x0 + w - 1.5, pl.center().y.max(pl.center().y - v * pl.height() / 2.0)));
            let hot = Some(b) == hov_bar || playing.contains(&b);
            clip_p.rect_filled(r, 1.0, col.gamma_multiply(if hot { 0.55 } else { 0.28 }));
        }
        for b in playing {
            clip_p.rect_filled(Rect::from_min_max(pos2(pl.left() + b as f32 * w, pl.top()), pos2(pl.left() + (b + 1) as f32 * w, pl.bottom())), 0.0, Color32::WHITE.gamma_multiply(0.07));
        }
        if let Some(b) = hov_bar {
            let v = sh.def.steps[b];
            clip_p.text(pos2(pl.left() + (b as f32 + 0.5) * w, g.bottom() - 6.0), Align2::CENTER_CENTER, format!("{v:+.1}"), mono.clone(), col);
        }
    }

    // The curve: gradient fill to the baseline, then the glowing line.
    // A library load morphs from the old shape to the new one over 180 ms (the playheads keep their phase).
    let mut morphing = false;
    let mut line = shape_line(&sh.def);
    if let Some((ml, old, t0)) = st.morph.clone() {
        let t = ((now - t0) / 0.18) as f32;
        if ml != k || !(0.0..1.0).contains(&t) || rr_ui::motion::reduced_motion(ui.ctx()) {
            st.morph = None;
        } else {
            morphing = true;
            let e = egui::emath::easing::cubic_out(t);
            line = (0..=96)
                .map(|i| {
                    let x = i as f32 / 96.0;
                    (x, eval_def(&old, x) + (eval_def(&sh.def, x) - eval_def(&old, x)) * e)
                })
                .collect();
        }
    }
    let to_pos = |(x, v): (f32, f32)| pos2(pl.left() + x * pl.width(), pl.center().y - v.clamp(-1.0, 1.0) * pl.height() / 2.0);
    let pts: Vec<Pos2> = line.iter().map(|q| to_pos(*q)).collect();
    if pts.len() > 1 {
        fill_to_baseline(&clip_p, &pts, pl.center().y, col, 0.22);
        rr_ui::scope::glow_line(&clip_p, pts, 1.5, col);
    }

    // Handles: points, the hovered segment's curve diamond, playheads, the phase marker.
    if !steps_mode && !morphing {
        let sel = st.sel[k];
        let dragging_pt = match st.drag {
            Some(Drag::Point { i, .. }) => Some(i),
            _ => None,
        };
        for (i, q) in sh.def.points.iter().enumerate() {
            let pos = point_pos_in(pl, *q);
            let hot = hover == Hit::Point(i) || dragging_pt == Some(i);
            let k_hot = rr_ui::motion::tween(ui.ctx(), Id::new(("gts-mod-pt", k, i)), hot, 0.1);
            // A point just added grows in from nothing over 100 ms (instant under Reduced motion).
            let grow = match st.added {
                Some((al, ai, t0)) if al == k && ai == i && !rr_ui::motion::reduced_motion(ui.ctx()) => egui::emath::easing::cubic_out(((now - t0) / 0.1).clamp(0.0, 1.0) as f32),
                _ => 1.0,
            };
            let r = (3.5 + k_hot) * grow;
            if sel == Some(i) {
                clip_p.circle_stroke(pos, 7.0, rr_ui::paint::st(1.5, Color32::WHITE));
                clip_p.circle_filled(pos, 5.0, col);
            } else {
                // A dark rim only where the point sits on its own curve would show: one filled disc keeps
                // 32 points cheap (the rim is the line's own glow).
                clip_p.circle_filled(pos, r, col);
            }
        }
        // Segments with a bend keep a dot at the midpoint; the hovered one shows the diamond and arrowheads.
        for (s, q) in sh.def.points.windows(2).enumerate() {
            let mid = seg_mid_in(pl, q[0], q[1]);
            let hot = hover == Hit::Curve(s) || hover == Hit::Seg(s) || matches!(st.drag, Some(Drag::Curve { seg, .. }) if seg == s);
            if hot && q[1][0] > q[0][0] {
                let a = if matches!(st.drag, Some(Drag::Curve { seg, .. }) if seg == s) || hover == Hit::Curve(s) { 1.0 } else { 0.8 };
                // The hovered segment itself, 2 px at 55 % white, then the diamond.
                let seg_pts: Vec<Pos2> = (0..=12)
                    .map(|i| {
                        let t = i as f32 / 12.0;
                        pos2(pl.left() + (q[0][0] + (q[1][0] - q[0][0]) * t) * pl.width(), pl.center().y - (q[0][1] + (q[1][1] - q[0][1]) * bend(t, q[0][2])) * pl.height() / 2.0)
                    })
                    .collect();
                clip_p.add(Shape::line(seg_pts, rr_ui::paint::st(2.0, Color32::WHITE.gamma_multiply(0.55))));
                diamond(&clip_p, mid, a);
            } else if q[0][2] != 0.0 && q[1][0] > q[0][0] {
                clip_p.circle_filled(mid, 2.0, Color32::WHITE.gamma_multiply(0.5));
            }
        }
    }
    // A point just removed fades out over 90 ms.
    if let Some((rl, at, t0)) = st.removed {
        let t = ((now - t0) / 0.09) as f32;
        if rl == k && (0.0..1.0).contains(&t) && !rr_ui::motion::reduced_motion(ui.ctx()) {
            clip_p.circle_filled(at, 3.5, col.gamma_multiply(1.0 - egui::emath::easing::cubic_in(t)));
        } else {
            st.removed = None;
        }
    }
    // Playheads: one per sounding voice, at its phase (offset applied) on the curve.
    let ph_k = [vf::PH_A, vf::PH_B, vf::PH_C, vf::PH_D][k];
    let oneshot = c.get01(&format!("{pre}_oneshot")) > 0.5 || enum_idx::<Trigger>(c, &format!("{pre}_trig")) == 2;
    let curve_eval = |x: f32| -> f32 {
        if sh.def.mode == Mode::Steps {
            eval_steps(&sh.def.steps, sh.def.glide, x)
        } else {
            eval_points(&sh.def.points, x)
        }
    };
    for (v, a) in playhead_voices(f) {
        let ph = f.voice(v, ph_k).max(0.0);
        let x = if oneshot && ph >= 1.0 { if phase > 0.0 { phase } else { 1.0 } } else { (ph + phase).rem_euclid(1.0) };
        let pos = pos2(pl.left() + x * pl.width(), pl.center().y - curve_eval(x).clamp(-1.0, 1.0) * pl.height() / 2.0);
        ph_head(&clip_p, pl, pos, col, a);
    }
    // Phase marker: a hollow down-triangle on the ruler, a dotted line, the value.
    {
        let mx = sn(pl.left() + phase * pl.width());
        let mc = if mresp.hovered() || matches!(st.drag, Some(Drag::Phase { .. })) { 1.0 } else { 0.9 };
        let tri = vec![pos2(mx - 4.0, g.top() + 8.0), pos2(mx + 4.0, g.top() + 8.0), pos2(mx, g.top() + 16.0)];
        clip_p.add(Shape::convex_polygon(tri, col.gamma_multiply(0.25 * mc), rr_ui::paint::st(1.0, col)));
        dotted_v(&clip_p, mx, g.top() + 17.0, pl.bottom(), col.gamma_multiply(0.4));
        let txt = format!("{:.0}\u{b0}", phase * 360.0);
        let flip = mx + 40.0 > g.right();
        clip_p.text(pos2(if flip { mx - 7.0 } else { mx + 7.0 }, g.top() + 12.0), if flip { Align2::RIGHT_CENTER } else { Align2::LEFT_CENTER }, txt, mono.clone(), col);
    }
    // Fade ramp under the plot, y 117..127 of the glass.
    if fade_w > 0.5 {
        let y1 = g.top() + 127.0;
        let tri = vec![pos2(px0, y1), pos2(px0 + fade_w, y1), pos2(px0 + fade_w, y1 - 8.0)];
        clip_p.add(Shape::convex_polygon(tri, col.gamma_multiply(0.30), rr_ui::paint::st(1.0, col)));
        clip_p.text(pos2(px0 + fade_w + 6.0, y1 - 4.0), Align2::LEFT_CENTER, format!("FADE {fade_s:.2} s"), mono.clone(), T.disp_caption);
    }
    if !sh.custom {
        clip_p.text(pos2(pl.center().x, g.bottom() - 7.0), Align2::CENTER_CENTER, "BUILT-IN CURVE \u{b7} DOUBLE-CLICK TO EDIT", font::mono(10.0), T.disp_caption);
    }

    // Help bar and the value pill.
    if resp.hovered() || mresp.hovered() {
        let (name, line) = if mresp.hovered() {
            ("PHASE", "shape:phase")
        } else if steps_mode {
            ("STEP", "shape:step")
        } else {
            match hover {
                Hit::Point(_) => ("POINT", "shape:point"),
                Hit::Curve(_) => ("CURVE", "shape:curve"),
                _ => ("SHAPE", "shape:plot"),
            }
        };
        let value = match (hover, steps_mode) {
            (Hit::Point(i), _) => sh.def.points.get(i).map_or(String::new(), |q| format!("{:.2}  {:+.2}", q[0], q[1])),
            (Hit::Curve(s), _) => sh.def.points.get(s).map_or(String::new(), |q| format!("segment {} \u{b7} {:+.2}", s + 1, q[2])),
            _ => lib_label(c, k, &sh),
        };
        set_hover(ui, &format!("{} {}", ["LFO A", "LFO B", "LFO C", "LFO D"][k], name), &value, None, crate::help::ui_help(line).unwrap_or(""));
    }
    let _ = (eff_snap, pxl);
    stash_cost(ui.ctx(), t_start);

    // ---- controls row ----------------------------------------------------------------------------
    controls(ui, c, m, k, col);
    let now = ui.input(|i| i.time);
    pill(ui, st, now, None);
}

fn point_pos_in(pl: Rect, p: [f32; 3]) -> Pos2 {
    pos2(pl.left() + p[0] * pl.width(), pl.center().y - p[1].clamp(-1.0, 1.0) * pl.height() / 2.0)
}

/// Where the diamond of the segment between `a` and `b` sits: the middle of the segment's x range on its curve.
fn seg_mid_in(pl: Rect, a: [f32; 3], b: [f32; 3]) -> Pos2 {
    let x = (a[0] + b[0]) / 2.0;
    let y = a[1] + (b[1] - a[1]) * bend(0.5, a[2]);
    pos2(pl.left() + x * pl.width(), pl.center().y - y * pl.height() / 2.0)
}

/// A playhead of the shape editor (MOTION 5.3): a 1 px white line at 50 %, an 18 px halo in the source colour at 30 % and
/// the 2.4 px white dot on the curve. `alpha` is the voice's (the newest at full).
fn ph_head(p: &egui::Painter, plot: Rect, at: Pos2, col: Color32, alpha: f32) {
    let x = snap_px(p, at.x);
    p.vline(x, plot.y_range(), Stroke::new(px(p), Color32::WHITE.gamma_multiply(0.5 * alpha)));
    rr_ui::fx::glow(p, at, 18.0, col, 0.9 * alpha);
    p.circle_filled(at, 2.4, Color32::WHITE.gamma_multiply(alpha));
}

fn dotted_v(p: &egui::Painter, x: f32, y0: f32, y1: f32, color: Color32) {
    let w = px(p);
    let mut y = y0;
    while y < y1 {
        p.vline(x, y..=(y + 1.5).min(y1), Stroke::new(w, color));
        y += 3.0;
    }
}

fn diamond(p: &egui::Painter, c: Pos2, a: f32) {
    let r = 5.5;
    let pts = vec![pos2(c.x, c.y - r), pos2(c.x + r, c.y), pos2(c.x, c.y + r), pos2(c.x - r, c.y)];
    p.add(Shape::convex_polygon(pts, Color32::from_rgb(0x0b, 0x0f, 0x11), rr_ui::paint::st(1.5, Color32::WHITE.gamma_multiply(a))));
    for s in [-1.0f32, 1.0] {
        let y = c.y + s * 11.0;
        p.add(Shape::line(vec![pos2(c.x - 3.0, y - s * 3.0), pos2(c.x, y), pos2(c.x + 3.0, y - s * 3.0)], rr_ui::paint::st(1.2, Color32::WHITE.gamma_multiply(0.7 * a))));
    }
}

/// Translucent fill from the curve to the baseline at `y0`: `a` at the curve fading to 0 at the baseline.
fn fill_to_baseline(p: &egui::Painter, pts: &[Pos2], y0: f32, col: Color32, a: f32) {
    let mut mesh = egui::epaint::Mesh::default();
    let top = col.gamma_multiply(a);
    for (i, q) in pts.iter().enumerate() {
        mesh.colored_vertex(*q, top);
        mesh.colored_vertex(pos2(q.x, y0), Color32::TRANSPARENT);
        if i > 0 {
            let b = (i as u32 - 1) * 2;
            mesh.add_triangle(b, b + 1, b + 2);
            mesh.add_triangle(b + 2, b + 1, b + 3);
        }
    }
    p.add(Shape::mesh(mesh));
}

/// Which step of LFO `k` each sounding voice is in now.
fn playing_step(f: &VizFrame, k: usize, n: usize, _def: &ShapeDef, c: &dyn Ctl) -> Vec<usize> {
    let ph_k = [vf::PH_A, vf::PH_B, vf::PH_C, vf::PH_D][k];
    let phase = c.plain(&format!("{}_phase", PRE[k]), c.get01(&format!("{}_phase", PRE[k]))) / 360.0;
    let mut out: Vec<usize> = vec![];
    for (v, _) in playhead_voices(f) {
        let x = (f.voice(v, ph_k).max(0.0) + phase).rem_euclid(1.0);
        let b = ((x * n as f32) as usize).min(n - 1);
        if !out.contains(&b) {
            out.push(b);
        }
    }
    out
}

// ---- hit testing and input -----------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Debug)]
enum Hit {
    None,
    Point(usize),
    /// The curve diamond of segment `s` (between points `s` and `s + 1`).
    Curve(usize),
    /// The line of segment `s`.
    Seg(usize),
}

/// Point hit radius, diamond hit radius, and how near the line a pointer must be to hover a segment.
const HIT_POINT: f32 = 8.0;
const HIT_DIAMOND: f32 = 10.0;
const NEAR_LINE: f32 = 14.0;

fn hit_test(def: &ShapeDef, pl: Rect, pos: Pos2) -> Hit {
    if !pl.expand(10.0).contains(pos) {
        return Hit::None;
    }
    let mut best: Option<(usize, f32)> = None;
    for (i, q) in def.points.iter().enumerate() {
        let d = point_pos_in(pl, *q).distance(pos);
        if d <= HIT_POINT && best.is_none_or(|b| d < b.1) {
            best = Some((i, d));
        }
    }
    if let Some((i, _)) = best {
        return Hit::Point(i);
    }
    let x = (pos.x - pl.left()) / pl.width();
    for (s, q) in def.points.windows(2).enumerate() {
        if q[1][0] <= q[0][0] || x < q[0][0] || x > q[1][0] {
            continue;
        }
        if seg_mid_in(pl, q[0], q[1]).distance(pos) <= HIT_DIAMOND {
            return Hit::Curve(s);
        }
        let t = ((x - q[0][0]) / (q[1][0] - q[0][0])).clamp(0.0, 1.0);
        let y = q[0][1] + (q[1][1] - q[0][1]) * bend(t, q[0][2]);
        let sy = pl.center().y - y * pl.height() / 2.0;
        if (sy - pos.y).abs() <= NEAR_LINE {
            return Hit::Seg(s);
        }
    }
    Hit::None
}

/// Rounds a plot value to the grid: x to `slots` slots, y to eighths.
fn snap_x(x: f32, slots: u32) -> f32 {
    (x * slots as f32).round() / slots as f32
}

fn snap_y(y: f32) -> f32 {
    (y * 8.0).round() / 8.0
}

/// All pointer input of the plot: point and curve drags, add / delete, step painting, the phase marker.
#[allow(clippy::too_many_arguments)]
fn points_or_steps_input(ui: &mut Ui, c: &dyn Ctl, st: &mut St, k: usize, sh: &Shown, pl: Rect, resp: &egui::Response, mresp: &egui::Response, g: Rect, slots: u32, snap: bool, shift: bool, hover: &mut Hit) {
    let steps_mode = sh.def.mode == Mode::Steps;
    let ctx = ui.ctx().clone();
    let pre = PRE[k];
    let (down, pos, press) = ctx.input(|i| (i.pointer.primary_down(), i.pointer.latest_pos(), i.pointer.press_origin()));
    let curve_id = format!("{pre}_curve");
    let make_custom = |c: &dyn Ctl, sh: &Shown| {
        // The first edit of a built-in curve turns it into the LFO's Custom shape: its points when
        // it has a fixed shape, so there is something to edit.
        if !sh.custom {
            let def = sh.def.clone();
            edit(c, k, |s| *s = def);
            one_shot(c, &curve_id, 1.0);
        }
    };

    // ---- the phase marker (a param, one host gesture) --------------------------------------------
    let phase_id = format!("{pre}_phase");
    if mresp.drag_started() {
        c.begin(&phase_id);
        st.param = Some(phase_id.clone());
        let v = c.get01(&phase_id);
        let o = press.unwrap_or(pl.center());
        let cur = pos.unwrap_or(o);
        st.drag = Some(Drag::Phase { acc: (cur.x - o.x) / pl.width() * if shift { FINE } else { 1.0 }, start: v });
    }
    if mresp.dragged() {
        if let Some(Drag::Phase { acc, start }) = st.drag.clone() {
            let d = mresp.drag_delta().x / pl.width() * if shift { FINE } else { 1.0 };
            let acc = acc + d;
            c.set01(&phase_id, (start + acc).clamp(0.0, 1.0));
            st.drag = Some(Drag::Phase { acc, start });
        }
    }
    if mresp.drag_stopped() {
        if let Some(p) = st.param.take() {
            c.end(&p);
        }
        st.drag = None;
    }
    if mresp.double_clicked() {
        one_shot(c, &phase_id, 0.0);
    }
    if matches!(st.drag, Some(Drag::Phase { .. })) {
        return;
    }

    // ---- step mode -------------------------------------------------------------------------------
    if steps_mode {
        let n = sh.def.steps.len().max(1);
        let w = pl.width() / n as f32;
        let bar_of = |q: Pos2| (((q.x - pl.left()) / w).floor().max(0.0) as usize).min(n - 1);
        let level_of = |q: Pos2| {
            let v = ((pl.center().y - q.y) / (pl.height() / 2.0)).clamp(-1.0, 1.0);
            if snap { (v * 16.0).round() / 16.0 } else { v }
        };
        let pressed = ctx.input(|i| i.pointer.primary_pressed());
        if pressed && resp.hovered() && st.drag.is_none() && pos.is_some_and(|q| pl.contains(q)) {
            begin_gesture(c, st);
            make_custom(c, sh);
            st.drag = Some(Drag::Steps { last: None });
        }
        if let (Some(Drag::Steps { last }), true, Some(q)) = (st.drag.clone(), down, pos) {
            let q = pos2(q.x.clamp(pl.left(), pl.right() - 0.01), q.y);
            let b = bar_of(q);
            let dy = ctx.input(|i| i.pointer.delta().y);
            if last != Some(b) || dy != 0.0 {
                let v = if shift && last == Some(b) { (sh.def.steps[b] - dy / (pl.height() / 2.0) * FINE).clamp(-1.0, 1.0) } else { level_of(q) };
                if (sh.def.steps.get(b).copied().unwrap_or(0.0) - v).abs() > 1e-6 {
                    edit(c, k, |s| {
                        if let Some(l) = s.steps.get_mut(b) {
                            *l = v;
                        }
                    });
                }
                st.drag = Some(Drag::Steps { last: Some(b) });
                st.pill = Some((format!("{v:+.2}"), pos2(q.x, q.y), ctx.input(|i| i.time)));
            }
        }
        if resp.double_clicked() {
            if let Some(q) = pos.filter(|q| pl.contains(*q)) {
                let b = bar_of(q);
                make_custom(c, sh);
                edit(c, k, |s| {
                    if let Some(l) = s.steps.get_mut(b) {
                        *l = 0.0;
                    }
                });
            }
        }
        if !down && matches!(st.drag, Some(Drag::Steps { .. })) {
            st.drag = None;
            end_gesture(c, st);
        }
        return;
    }

    // ---- points mode -----------------------------------------------------------------------------
    let pts = &sh.def.points;
    let n = pts.len();
    // A press on a point selects it.
    if resp.hovered() && ctx.input(|i| i.pointer.primary_pressed()) {
        if let Some(q) = pos {
            match hit_test(&sh.def, pl, q) {
                Hit::Point(i) => st.sel[k] = Some(i),
                Hit::Curve(_) | Hit::Seg(_) => {}
                Hit::None => st.sel[k] = None,
            }
        }
        resp.request_focus();
    }
    if resp.drag_started() {
        let o = press.unwrap_or_else(|| pos.unwrap_or(pl.center()));
        let cur = pos.unwrap_or(o);
        match hit_test(&sh.def, pl, o) {
            Hit::Point(i) => {
                begin_gesture(c, st);
                make_custom(c, sh);
                st.sel[k] = Some(i);
                st.drag = Some(Drag::Point { i, start: [pts[i][0], pts[i][1]], acc: (cur - o) * if shift { FINE } else { 1.0 } });
            }
            Hit::Curve(s) => {
                begin_gesture(c, st);
                make_custom(c, sh);
                st.drag = Some(Drag::Curve { seg: s, start: pts[s][2], acc: (cur - o).y * if shift { FINE } else { 1.0 } });
            }
            _ => {}
        }
    }
    if resp.dragged() {
        let d = resp.drag_delta() * if shift { FINE } else { 1.0 };
        match st.drag.clone() {
            Some(Drag::Point { i, start, acc }) if i < n => {
                let acc = acc + d;
                let (mut x, mut y) = (start[0] + acc.x / pl.width(), start[1] - acc.y / (pl.height() / 2.0));
                if snap {
                    x = snap_x(x, slots);
                    y = snap_y(y);
                }
                // First and last points only move vertically; the others stay between their neighbours.
                let (lo, hi) = if i == 0 || i + 1 == n { (pts[i][0], pts[i][0]) } else { (pts[i - 1][0], pts[i + 1][0]) };
                let (x, y) = (x.clamp(lo, hi), y.clamp(-1.0, 1.0));
                if (pts[i][0] - x).abs() > 1e-6 || (pts[i][1] - y).abs() > 1e-6 {
                    edit(c, k, |s| {
                        if let Some(q) = s.points.get_mut(i) {
                            q[0] = x;
                            q[1] = y;
                        }
                    });
                }
                st.drag = Some(Drag::Point { i, start, acc });
                st.pill = Some((format!("{x:.2}  {y:+.2}"), pos2(pl.left() + x * pl.width(), pl.center().y - y * pl.height() / 2.0), ctx.input(|i| i.time)));
            }
            Some(Drag::Curve { seg, start, acc }) if seg + 1 < n => {
                let acc = acc + d.y;
                let sign = if pts[seg + 1][1] >= pts[seg][1] { -1.0 } else { 1.0 };
                let cv = (start + sign * -acc / 100.0).clamp(-1.0, 1.0);
                if (pts[seg][2] - cv).abs() > 1e-6 {
                    edit(c, k, |s| {
                        if let Some(q) = s.points.get_mut(seg) {
                            q[2] = cv;
                        }
                    });
                }
                st.drag = Some(Drag::Curve { seg, start, acc });
                let mid = seg_mid_in(pl, pts[seg], pts[seg + 1]);
                st.pill = Some((format!("CURVE {cv:+.2}"), mid, ctx.input(|i| i.time)));
            }
            _ => {}
        }
    }
    if resp.drag_stopped() {
        st.drag = None;
        end_gesture(c, st);
    }
    // Double-click: delete a point, straighten a curve, or add a point in the empty plot.
    if resp.double_clicked() {
        if let Some(q) = pos {
            match hit_test(&sh.def, pl, q) {
                Hit::Point(i) => {
                    if i == 0 || i + 1 == n {
                        super::patching::notice(&ctx, "The first and last points stay: they only move up and down.");
                    } else {
                        make_custom(c, sh);
                        st.removed = Some((k, point_pos_in(pl, pts[i]), ctx.input(|inp| inp.time)));
                        edit(c, k, |s| {
                            s.points.remove(i);
                        });
                        st.sel[k] = None;
                    }
                }
                Hit::Curve(s) => {
                    make_custom(c, sh);
                    edit(c, k, |d| {
                        if let Some(p) = d.points.get_mut(s) {
                            p[2] = 0.0;
                        }
                    });
                }
                Hit::Seg(_) | Hit::None => {
                    if pl.contains(q) {
                        if n >= MAX_POINTS {
                            super::patching::notice(&ctx, "A shape has at most 32 points. Delete one first.");
                        } else {
                            let mut x = ((q.x - pl.left()) / pl.width()).clamp(0.0, 1.0);
                            let mut y = ((pl.center().y - q.y) / (pl.height() / 2.0)).clamp(-1.0, 1.0);
                            if snap {
                                x = snap_x(x, slots);
                                y = snap_y(y);
                            }
                            make_custom(c, sh);
                            let at_i = sh.def.points.partition_point(|p| p[0] <= x);
                            edit(c, k, |s| s.points.insert(at_i, [x, y, 0.0]));
                            st.sel[k] = Some(at_i);
                            st.added = Some((k, at_i, ctx.input(|inp| inp.time)));
                        }
                    }
                }
            }
        }
    }
    // Right-click: delete a point, or open the segment menu.
    if resp.secondary_clicked() {
        if let Some(q) = pos {
            match hit_test(&sh.def, pl, q) {
                Hit::Point(i) if i != 0 && i + 1 != n => {
                    make_custom(c, sh);
                    st.removed = Some((k, point_pos_in(pl, pts[i]), ctx.input(|inp| inp.time)));
                    edit(c, k, |s| {
                        s.points.remove(i);
                    });
                    st.sel[k] = None;
                }
                Hit::Point(_) => super::patching::notice(&ctx, "The first and last points stay: they only move up and down."),
                Hit::Curve(s) | Hit::Seg(s) => st.seg_menu = Some((q, s)),
                Hit::None => {}
            }
        }
    }
    // Keyboard: arrows move the selected point 1/20 of the plot (Shift 1/200), Delete removes it, Tab cycles.
    if resp.has_focus() {
        // egui would use Tab and the arrows to move the focus: the plot keeps them while it has it.
        ctx.memory_mut(|m| m.set_focus_lock_filter(resp.id, egui::EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: false }));
        let (dx, dy, del, tab) = ctx.input(|i| {
            let s = if i.modifiers.shift { 0.005 } else { 0.05 };
            let k = |key| i.key_pressed(key);
            (
                s * (k(egui::Key::ArrowRight) as i32 - k(egui::Key::ArrowLeft) as i32) as f32,
                s * (k(egui::Key::ArrowUp) as i32 - k(egui::Key::ArrowDown) as i32) as f32 * 2.0,
                k(egui::Key::Delete) || k(egui::Key::Backspace),
                k(egui::Key::Tab),
            )
        });
        if let Some(i) = st.sel[k].filter(|&i| i < n) {
            if dx != 0.0 || dy != 0.0 {
                let (lo, hi) = if i == 0 || i + 1 == n { (pts[i][0], pts[i][0]) } else { (pts[i - 1][0], pts[i + 1][0]) };
                let (x, y) = ((pts[i][0] + dx).clamp(lo, hi), (pts[i][1] + dy).clamp(-1.0, 1.0));
                make_custom(c, sh);
                edit(c, k, |s| {
                    if let Some(q) = s.points.get_mut(i) {
                        q[0] = x;
                        q[1] = y;
                    }
                });
            }
            if del && i != 0 && i + 1 != n {
                make_custom(c, sh);
                st.removed = Some((k, point_pos_in(pl, pts[i]), ctx.input(|inp| inp.time)));
                edit(c, k, |s| {
                    s.points.remove(i);
                });
                st.sel[k] = None;
            }
        }
        if tab {
            st.sel[k] = Some(st.sel[k].map_or(0, |i| (i + 1) % n.max(1)));
        }
    }
    let _ = (g, hover);
}

// ---- toolbar ---------------------------------------------------------------------------------------

fn toolbar(ui: &mut Ui, c: &dyn Ctl, st: &mut St, m: Rect, k: usize, col: Color32, sh: &Shown) {
    let steps = sh.def.mode == Mode::Steps;
    let at = |t: Tool| {
        let r = tool_rect(t, steps);
        r.translate(m.min - origin())
    };
    // LIB: shows what is loaded; opens the popover.
    let lib = at(Tool::Lib);
    let resp = ui.interact(lib, Id::new("gts-mod-lib"), Sense::click());
    {
        let p = ui.painter().clone();
        rr_ui::paint::inset_box(&p, lib, resp.hovered() || st.lib);
        if st.lib {
            p.rect_stroke(lib, 4.0, rr_ui::paint::hair(&p, T.a(0.55)), StrokeKind::Inside);
        }
        let name = rr_ui::paint::elide(&p, &font::label(13.0), &lib_label(c, k, sh), lib.width() - 8.0 - 28.0);
        p.text(pos2(lib.left() + 8.0, lib.center().y), Align2::LEFT_CENTER, name, font::label(13.0), T.text);
        let s = 1.0 - 2.0 * rr_ui::motion::tween(ui.ctx(), Id::new("gts-mod-lib-chev"), st.lib, 0.18);
        let (cx, cy) = (lib.right() - 11.5, lib.center().y - 1.0);
        p.add(Shape::line(vec![pos2(cx - 3.5, cy - 1.5 * s), pos2(cx, cy + 2.0 * s), pos2(cx + 3.5, cy - 1.5 * s)], rr_ui::paint::st(1.5, T.muted)));
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        set_hover(ui, "LIBRARY", &lib_label(c, k, sh), None, crate::help::ui_help("shape:lib").unwrap_or(""));
    }
    if resp.clicked() {
        st.lib = !st.lib;
    }
    // STEP: switches the editor between points and steps (the data of both is kept).
    let (clicked, hov) = toggle_at(ui, at(Tool::Step), &format!("step{k}"), "Step", steps);
    if hov {
        set_hover(ui, "STEP", if steps { "on" } else { "off" }, None, crate::help::ui_help("shape:stepmode").unwrap_or(""));
    }
    if clicked {
        // STEP is part of the LFO's own shape; on a built-in curve it turns the LFO to Custom.
        let to = if steps { Mode::Points } else { Mode::Steps };
        edit(c, k, |s| s.mode = to);
        if !sh.custom {
            one_shot(c, &format!("{}_curve", PRE[k]), 1.0);
        }
    }
    let (clicked, hov) = toggle_at(ui, at(Tool::Snap), &format!("snap{k}"), "Snap", st.snap);
    if hov {
        set_hover(ui, "SNAP", if st.snap { "on" } else { "off" }, None, crate::help::ui_help("shape:snap").unwrap_or(""));
    }
    if clicked {
        st.snap = !st.snap;
    }
    if !steps {
        let rows: Vec<Row> = GRIDS.iter().map(|g| Row::Item(g.0)).collect();
        let (pick, hov) = dropdown(ui, at(Tool::Grid), "grid", GRIDS[st.grid].0, DdStyle::Normal, &rows, Some(st.grid));
        if hov {
            set_hover(ui, "GRID", GRIDS[st.grid].0, None, crate::help::ui_help("shape:grid").unwrap_or(""));
        }
        if let Some(i) = pick {
            st.grid = i;
        }
    } else {
        // STEPS combo and the GLIDE slider.
        let names: Vec<String> = (2..=MAX_STEPS).map(|n| n.to_string()).collect();
        let rows: Vec<Row> = names.iter().map(|n| Row::Item(n)).collect();
        let n = sh.def.steps.len();
        let (pick, hov) = dropdown(ui, at(Tool::Steps), "steps", &n.to_string(), DdStyle::Normal, &rows, Some(n.saturating_sub(2)));
        if hov {
            set_hover(ui, "STEPS", &n.to_string(), None, crate::help::ui_help("shape:steps").unwrap_or(""));
        }
        if let Some(i) = pick {
            let want = i + 2;
            edit(c, k, |s| {
                let old = s.steps.clone();
                s.steps = (0..want).map(|j| old.get(j % old.len().max(1)).copied().unwrap_or(0.0)).collect();
            });
        }
        glide_slider(ui, c, st, k, at(Tool::Glide), col, sh);
    }
}

/// The GLIDE slider of step mode: an inset bar filled in the source colour, value in it.
fn glide_slider(ui: &mut Ui, c: &dyn Ctl, st: &mut St, k: usize, r: Rect, col: Color32, sh: &Shown) {
    let resp = ui.interact(r, Id::new(("gts-mod-glide", k)), Sense::click_and_drag());
    let p = ui.painter().clone();
    rr_ui::paint::inset_box(&p, r, resp.hovered() || resp.dragged());
    let v = sh.def.glide.clamp(0.0, 1.0);
    let inner = r.shrink2(vec2(2.0, 2.0));
    if v > 0.0 {
        p.rect_filled(Rect::from_min_size(inner.min, vec2(inner.width() * v, inner.height())), 3.0, col.gamma_multiply(0.30));
    }
    p.text(pos2(r.left() + 6.0, r.center().y), Align2::LEFT_CENTER, "GLIDE", font::strong(10.0), T.muted);
    // ".05": the leading zero would not fit next to the label.
    let txt = format!("{v:.2}");
    p.text(pos2(r.right() - 5.0, r.center().y), Align2::RIGHT_CENTER, txt.strip_prefix('0').unwrap_or(&txt), font::mono(10.0), col);
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        set_hover(ui, "GLIDE", &format!("{v:.2}"), None, crate::help::ui_help("shape:glide").unwrap_or(""));
    }
    let shift = ui.input(|i| i.modifiers.shift);
    if resp.drag_started() {
        begin_gesture(c, st);
        st.drag = Some(Drag::Glide { start: v, acc: 0.0 });
    }
    if resp.dragged() {
        if let Some(Drag::Glide { start, acc }) = st.drag.clone() {
            let acc = acc + resp.drag_delta().x / inner.width() * if shift { FINE } else { 1.0 };
            let nv = (start + acc).clamp(0.0, 1.0);
            edit(c, k, |s| s.glide = nv);
            st.drag = Some(Drag::Glide { start, acc });
        }
    }
    if resp.drag_stopped() {
        st.drag = None;
        end_gesture(c, st);
    }
    if resp.double_clicked() {
        edit(c, k, |s| s.glide = 0.0);
    }
}

// ---- controls row ------------------------------------------------------------------------------------

/// One labelled cell: the 14 px label band over a 32 px knob band.
fn cell_label(ui: &Ui, cell: Rect, label: &str) {
    rr_ui::paint::caps(ui.painter(), pos2(cell.center().x, cell.top() + 7.0), Align2::CENTER_CENTER, label, 11.0, T.muted);
}

fn controls(ui: &mut Ui, c: &dyn Ctl, m: Rect, k: usize, col: Color32) {
    let pre = PRE[k];
    let id = |s: &str| format!("{pre}_{s}");
    let y = m.top() + ROW_Y;
    let cell = |x: f32, w: f32| Rect::from_min_size(pos2(m.left() + x, y), vec2(w, 48.0));
    let band = |r: Rect| Rect::from_min_size(pos2(r.left(), r.top() + 14.0 + 4.0), vec2(r.width(), 24.0));
    // SYNC + DIV (or Rate, unsynced): 76 px.
    let sync_on = c.get01(&id("sync")) > 0.5;
    let c1 = cell(12.0, 76.0);
    cell_label(ui, c1, if sync_on { "Sync \u{b7} Div" } else { "Sync \u{b7} Rate" });
    let b = band(c1);
    let (clicked, hov) = toggle_at(ui, Rect::from_min_size(b.min, vec2(24.0, 24.0)), &format!("sync{k}"), "", sync_on);
    if hov {
        set_hover(ui, &c.name(&id("sync")).to_uppercase(), if sync_on { "On" } else { "Off" }, None, crate::help::help(&id("sync")).unwrap_or(""));
    }
    if clicked {
        one_shot(c, &id("sync"), if sync_on { 0.0 } else { 1.0 });
    }
    if sync_on {
        div_combo(ui, c, &id("div"), Rect::from_min_size(b.min + vec2(28.0, 0.0), vec2(48.0, 24.0)));
    } else {
        put(ui, Rect::from_min_size(pos2(c1.left() + 26.0, c1.top()), vec2(48.0, 48.0)), |ui| kn(ui, c, &id("rate"), "Rate", ks(48.0)));
    }
    // TRIGGER: Retrig / Free / Envelope; 1-SHOT is lit in Envelope (the same switch as the old one-shot).
    let c2 = cell(90.0, 66.0);
    cell_label(ui, c2, "Trigger");
    let trig = enum_idx::<Trigger>(c, &id("trig"));
    let one = c.get01(&id("oneshot")) > 0.5;
    let shown_trig = if one { 2 } else { trig };
    let rows = [Row::Item("Retrigger"), Row::Item("Free"), Row::Item("Envelope")];
    let (pick, hov) = dropdown(ui, band(c2), &format!("trig{k}"), ["Retrig", "Free", "Env"][shown_trig], DdStyle::Caps, &rows, Some(shown_trig));
    if hov {
        set_hover(ui, &c.name(&id("trig")).to_uppercase(), Trigger::variants()[shown_trig], None, crate::help::help(&id("trig")).unwrap_or(""));
    }
    if let Some(i) = pick {
        // A pick sets the mode and clears the one-shot flag; Envelope is one-shot (saved v1 presets keep working).
        one_shot(c, &id("trig"), i as f32 / 2.0);
        if one {
            one_shot(c, &id("oneshot"), 0.0);
        }
    }
    let c3 = cell(158.0, 42.0);
    cell_label(ui, c3, "1-shot");
    let lit = one || trig == 2;
    let (clicked, hov) = toggle_at(ui, Rect::from_center_size(band(c3).center(), vec2(24.0, 24.0)), &format!("one{k}"), "", lit);
    if hov {
        set_hover(ui, &c.name(&id("oneshot")).to_uppercase(), if lit { "On" } else { "Off" }, None, crate::help::help(&id("oneshot")).unwrap_or(""));
    }
    if clicked {
        if lit {
            one_shot(c, &id("oneshot"), 0.0);
            if trig == 2 {
                one_shot(c, &id("trig"), 0.0);
            }
        } else {
            one_shot(c, &id("trig"), 1.0);
        }
    }
    for (x, w, name, label) in [(200.0, 40.0, "phase", "Phase"), (240.0, 40.0, "fade", "Fade"), (280.0, 44.0, "smooth", "Smooth")] {
        let r = cell(x, w);
        put(ui, r, |ui| kn(ui, c, &id(name), label, ks(w)));
    }
    let _ = col;
}

/// The DIV combo: Straight, Dotted and Triplet groups (the order of the enum is saved state, the
/// menu is not).
pub(super) fn div_combo(ui: &mut Ui, c: &dyn Ctl, id: &str, r: Rect) {
    use Division::*;
    const GROUPS: [(&str, &[Division]); 3] = [
        ("Straight", &[Bars4, Bars2, Bar1, Half, Quarter, Eighth, Sixteenth, ThirtySecond]),
        ("Dotted", &[HalfD, QuarterD, EighthD, SixteenthD]),
        ("Triplet", &[HalfT, QuarterT, EighthT, SixteenthT, ThirtySecondT]),
    ];
    let cur = Division::from_index(enum_idx::<Division>(c, id));
    let mut rows = vec![];
    let mut flat: Vec<Division> = vec![];
    for (name, items) in GROUPS {
        rows.push(Row::Header(name));
        for d in items {
            rows.push(Row::Item(Division::variants()[*d as usize]));
            flat.push(*d);
        }
    }
    let sel = flat.iter().position(|d| *d == cur);
    let (pick, hov) = dropdown(ui, r, &format!("div-{id}"), Division::variants()[cur as usize], DdStyle::Normal, &rows, sel);
    if hov {
        set_hover(ui, &c.name(id).to_uppercase(), Division::variants()[cur as usize], None, crate::help::help(id).unwrap_or(""));
    }
    if let Some(i) = pick {
        let d = flat[i];
        one_shot(c, id, d as usize as f32 / (Division::variants().len() - 1) as f32);
    }
}

// ---- library and segment menu --------------------------------------------------------------------

/// The library popover (336 wide, under the LIB combo): the built-in curves as a row of icons, then
/// the stock shapes in two columns with a thumbnail each. Loading copies the points into the LFO and
/// sets its Curve to Custom. Esc or a click elsewhere closes.
fn library_popover(ui: &mut Ui, c: &dyn Ctl, st: &mut St, mrect: Rect) {
    let ctx = ui.ctx().clone();
    let k = rr_ui::motion::tween(&ctx, Id::new("gts-mod-lib-a"), st.lib && st.shown < 4, if st.lib { 0.13 } else { 0.09 });
    if k <= 0.0 {
        return;
    }
    let lfo = st.shown.min(3);
    let col = SRC_MOD[lfo];
    let sh = shown_shape(c, lfo);
    let lib = library();
    let rows = (lib.len() + 1) / 2;
    let size = vec2(336.0, 26.0 + 30.0 + 6.0 + rows as f32 * 24.0 + 8.0);
    let pos = pos2(mrect.left(), mrect.top() + 68.0 - 4.0 * (1.0 - k));
    let area_rect = Rect::from_min_size(pos, size);
    let open = st.lib;
    let mut load_shape: Option<usize> = None;
    let mut pick_curve: Option<usize> = None;
    egui::Area::new(Id::new("gts-mod-lib-pop")).order(egui::Order::Foreground).fixed_pos(pos).show(&ctx, |ui| {
        ui.multiply_opacity(k);
        let r = Rect::from_min_size(pos, size);
        let p = ui.painter().clone();
        p.rect_filled(r.translate(vec2(0.0, 10.0)).expand(4.0), 10.0, Color32::BLACK.gamma_multiply(0.35));
        p.rect_filled(r.translate(vec2(0.0, 2.0)), 6.0, Color32::BLACK.gamma_multiply(0.5));
        p.rect_filled(r, 6.0, T.well);
        p.rect_filled(r, 6.0, T.inset.gamma_multiply(0.5));
        p.rect_stroke(r, 6.0, rr_ui::paint::hair(&p, T.lo()), StrokeKind::Inside);
        rr_ui::paint::tracked(&p, pos2(r.left() + 12.0, r.top() + 13.0), Align2::LEFT_CENTER, "CURVES", font::label(11.0), T.title, 1.2);
        p.text(pos2(r.right() - 12.0, r.top() + 13.0), Align2::RIGHT_CENTER, format!("Loads into {} as Custom", ["LFO A", "LFO B", "LFO C", "LFO D"][lfo]), font::label(11.0), T.disp_caption);
        // Built-in curves: icon buttons.
        let cur = enum_idx::<Curve>(c, &format!("{}_curve", PRE[lfo]));
        for i in 0..Curve::variants().len() {
            let br = Rect::from_min_size(pos2(r.left() + 12.0 + i as f32 * 31.0, r.top() + 26.0), vec2(28.0, 22.0));
            let resp = ui.interact(br, Id::new(("gts-mod-curve", i)), Sense::click());
            rr_ui::paint::inset_box(&p, br, resp.hovered());
            if cur == i {
                p.rect_filled(br, 4.0, col.gamma_multiply(0.18));
                p.rect_stroke(br, 4.0, rr_ui::paint::hair(&p, col.gamma_multiply(0.6)), StrokeKind::Inside);
            }
            let curve = Curve::from_index(i);
            let pts: Vec<Pos2> = (0..=20)
                .map(|s| {
                    let x = s as f32 / 20.0;
                    let v = if curve == Curve::Custom { eval_points(&sh.def.points, x) } else { Lfo::curve_at(curve, x as f64) };
                    pos2(br.left() + 6.0 + x * 16.0, br.center().y - v.clamp(-1.0, 1.0) * 6.0)
                })
                .collect();
            p.add(Shape::line(pts, rr_ui::paint::st(1.2, if cur == i { col } else { T.muted })));
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                set_hover(ui, &format!("CURVE {}", Curve::variants()[i].to_uppercase()), "", None, crate::help::ui_help("shape:curvepick").unwrap_or(""));
            }
            if resp.clicked() && open {
                pick_curve = Some(i);
            }
        }
        rr_ui::paint::tracked(&p, pos2(r.left() + 12.0, r.top() + 26.0 + 22.0 + 10.0), Align2::LEFT_CENTER, "STOCK SHAPES", font::label(11.0), T.title, 1.2);
        for (i, (name, _)) in lib.iter().enumerate() {
            let (cx, cy) = (i / rows, i % rows);
            let row = Rect::from_min_size(pos2(r.left() + 8.0 + cx as f32 * 160.0, r.top() + 26.0 + 22.0 + 20.0 + 4.0 + cy as f32 * 24.0), vec2(156.0, 24.0));
            let resp = ui.interact(row, Id::new(("gts-mod-lib-row", i)), Sense::click());
            let loaded = sh.custom && lib[i].1 == sh.def;
            if resp.hovered() {
                p.rect_filled(row, 3.0, T.a(0.12));
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                set_hover(ui, &format!("SHAPE {}", name.to_uppercase()), "", None, crate::help::ui_help("shape:stock").unwrap_or(""));
            }
            if loaded {
                p.circle_filled(pos2(row.left() + 7.0, row.center().y), 2.5, T.accent);
            }
            p.text(pos2(row.left() + 16.0, row.center().y), Align2::LEFT_CENTER, name, font::label(13.0), if loaded { T.accent } else { T.text });
            let th = &thumbs()[i];
            let tx = row.right() - 40.0;
            let pts: Vec<Pos2> = th.iter().enumerate().map(|(s, v)| pos2(tx + s as f32 / (th.len() - 1) as f32 * 34.0, row.center().y - v.clamp(-1.0, 1.0) * 6.0)).collect();
            p.add(Shape::line(pts, rr_ui::paint::st(1.3, col)));
            if resp.clicked() && open {
                load_shape = Some(i);
            }
        }
    });
    if let Some(i) = load_shape {
        let def = lib[i].1.clone();
        st.morph = Some((lfo, sh.def.clone(), ctx.input(|i| i.time)));
        edit(c, lfo, |s| *s = def);
        one_shot(c, &format!("{}_curve", PRE[lfo]), 1.0);
        st.lib = false;
    }
    if let Some(i) = pick_curve {
        one_shot(c, &format!("{}_curve", PRE[lfo]), i as f32 / (Curve::variants().len() - 1) as f32);
        st.lib = false;
    }
    if st.lib {
        let (esc, click) = ctx.input(|i| (i.key_pressed(egui::Key::Escape), i.pointer.primary_pressed().then(|| i.pointer.interact_pos()).flatten()));
        let lib_btn = tool_rect(Tool::Lib, sh.def.mode == Mode::Steps).translate(mrect.min - origin());
        let outside = click.is_some_and(|q| !area_rect.contains(q) && !lib_btn.contains(q));
        if esc || outside {
            st.lib = false;
        }
    }
}

/// The two-item menu of a segment: Straighten (curve 0) and Hold (the segment becomes a step).
fn segment_menu(ui: &mut Ui, c: &dyn Ctl, st: &mut St) {
    let Some((at, seg)) = st.seg_menu else { return };
    let ctx = ui.ctx().clone();
    let lfo = st.shown.min(3);
    let items = [("Straighten", "straighten"), ("Hold (a step)", "hold")];
    let size = vec2(132.0, 4.0 + items.len() as f32 * 24.0);
    let screen = ctx.screen_rect();
    let pos = pos2(at.x.min(screen.right() - size.x - 4.0), at.y.min(screen.bottom() - size.y - 4.0));
    let mut act = None;
    let area = egui::Area::new(Id::new("gts-mod-seg-menu")).order(egui::Order::Foreground).fixed_pos(pos).show(&ctx, |ui| {
        let r = Rect::from_min_size(pos, size);
        let p = ui.painter().clone();
        p.rect_filled(r.translate(vec2(0.0, 6.0)).expand(3.0), 8.0, Color32::BLACK.gamma_multiply(0.35));
        p.rect_filled(r, 5.0, T.inset);
        p.rect_stroke(r, 5.0, rr_ui::paint::hair(&p, T.lo()), StrokeKind::Inside);
        for (i, (label, key)) in items.iter().enumerate() {
            let row = Rect::from_min_size(pos2(r.left() + 2.0, r.top() + 2.0 + i as f32 * 24.0), vec2(r.width() - 4.0, 24.0));
            let resp = ui.interact(row, Id::new(("gts-mod-seg-item", i)), Sense::click());
            if resp.hovered() {
                p.rect_filled(row, 3.0, T.a(0.16));
            }
            p.text(pos2(row.left() + 8.0, row.center().y), Align2::LEFT_CENTER, label, font::label(13.0), T.text);
            if resp.clicked() {
                act = Some(*key);
            }
        }
    });
    let (esc, click) = ctx.input(|i| (i.key_pressed(egui::Key::Escape), i.pointer.primary_pressed().then(|| i.pointer.interact_pos()).flatten()));
    let mut close = esc || click.is_some_and(|q| !area.response.rect.contains(q));
    if let Some(key) = act {
        close = true;
        edit(c, lfo, |s| {
            if seg + 1 >= s.points.len() {
                return;
            }
            match key {
                "straighten" => s.points[seg][2] = 0.0,
                _ => {
                    // A step: the next point's value arrives at the next point's x from the left, so a
                    // copy of this segment's level goes just before it (equal x = a vertical edge).
                    if s.points.len() < MAX_POINTS {
                        let (y, xr) = (s.points[seg][1], s.points[seg + 1][0]);
                        s.points[seg][2] = 0.0;
                        s.points.insert(seg + 1, [xr, y, 0.0]);
                    }
                }
            }
        });
    }
    if close {
        st.seg_menu = None;
    }
}

// ---- MOD ENV tab -------------------------------------------------------------------------------------

/// Envelope geometry: widths of the three timed segments (sqrt(time + 0.02)) and the notional sustain.
struct EnvGeo {
    xa: f32,
    xd: f32,
    xs: f32,
    sus: f32,
    curve: f32,
}

fn env_geo(c: &dyn Ctl) -> EnvGeo {
    let t = |id: &str| (c.plain(id, c.get01(id)) + 0.02).sqrt();
    let (wa, wd, wr) = (t("menv_att"), t("menv_dec"), t("menv_rel"));
    let ws = 0.5 * (wa + wd + wr);
    let sum = wa + wd + wr + ws;
    EnvGeo { xa: wa / sum, xd: (wa + wd) / sum, xs: (wa + wd + ws) / sum, sus: c.get01("menv_sus"), curve: c.plain("menv_curve", c.get01("menv_curve")) }
}

/// Segment shape exponents: attack `1 - (1 - t)^q`, decay and release `(1 - t)^p` (curve +1 linear, 0
/// exponential, -1 logarithmic; the display's own approximation of the DSP's curved segments).
fn env_exps(curve: f32) -> (f32, f32) {
    let k = 3f32.powf(-curve);
    (2.0 * k, 3.0 * k)
}

fn env_points(g: &EnvGeo, pl: Rect) -> Vec<Pos2> {
    let (q, p_) = env_exps(g.curve);
    let pt = |x: f32, y: f32| pos2(pl.left() + x * pl.width(), pl.bottom() - y * pl.height());
    let mut v = vec![];
    for i in 0..=14 {
        let t = i as f32 / 14.0;
        v.push(pt(g.xa * t, 1.0 - (1.0 - t).powf(q)));
    }
    for i in 1..=14 {
        let t = i as f32 / 14.0;
        v.push(pt(g.xa + (g.xd - g.xa) * t, g.sus + (1.0 - g.sus) * (1.0 - t).powf(p_)));
    }
    v.push(pt(g.xs, g.sus));
    for i in 1..=14 {
        let t = i as f32 / 14.0;
        v.push(pt(g.xs + (1.0 - g.xs) * t, g.sus * (1.0 - t).powf(p_)));
    }
    v
}

fn env_body(ui: &mut Ui, c: &dyn Ctl, f: &VizFrame, st: &mut St, m: Rect, col: Color32, now: f64) {
    let _ = now;
    let t_start = std::time::Instant::now();
    // Toolbar: the source chip (drag to patch) and the hint.
    let bar = Rect::from_min_size(m.min + vec2(12.0, BAR_Y), vec2(312.0, BAR_H));
    let chip = Rect::from_min_size(bar.min + vec2(0.0, 2.0), vec2(chip_width(ui.painter(), "Mod Env", true), 20.0));
    let cr = source_chip(ui, chip, "M", "Mod Env", col, Some(f.newest().map_or(0.0, |v| f.voice(v, vf::MENV)).clamp(0.0, 1.0)));
    if cr.drag_started() {
        super::patching::start_drag(ui.ctx(), Source::ModEnv);
    }
    if cr.hovered() {
        set_hover(ui, "MOD ENV", "", None, crate::help::ui_help("src:Mod Env").unwrap_or(""));
    }
    ui.painter().text(pos2(chip.right() + 10.0, bar.center().y), Align2::LEFT_CENTER, "Drag onto a knob. Pitch All gives a dive.", font::label(12.0), T.disp_caption);

    let g = local(m, GLASS);
    let pl = local(g, PLOT);
    let geo = env_geo(c);
    // ---- input ----------------------------------------------------------------------------------
    let (a_pt, d_pt, s_pt, r_pt) = env_handles(&geo, pl);
    let mids = [
        pos2((a_pt.x + pl.left()) / 2.0, pl.bottom() - (1.0 - (1.0 - 0.5f32).powf(env_exps(geo.curve).0)) * pl.height()),
        pos2((a_pt.x + d_pt.x) / 2.0, pl.bottom() - (geo.sus + (1.0 - geo.sus) * 0.5f32.powf(env_exps(geo.curve).1)) * pl.height()),
        pos2((s_pt.x + r_pt.x) / 2.0, pl.bottom() - geo.sus * 0.5f32.powf(env_exps(geo.curve).1) * pl.height()),
    ];
    let resp = ui.interact(pl.expand2(vec2(10.0, 4.0)), Id::new("gts-mod-env-plot"), Sense::click_and_drag());
    let (shift, pos, press) = ui.input(|i| (i.modifiers.shift, i.pointer.latest_pos(), i.pointer.press_origin()));
    let hit = |q: Pos2| -> Option<u8> {
        for (h, hp) in [(0u8, a_pt), (1, d_pt), (2, r_pt)] {
            if hp.distance(q) <= HIT_POINT {
                return Some(h);
            }
        }
        mids.iter().position(|mp| mp.distance(q) <= HIT_DIAMOND).map(|_| 3u8)
    };
    let hover_h = resp.hover_pos().filter(|_| st.drag.is_none()).and_then(hit);
    let ids = ["menv_att", "menv_dec", "menv_sus", "menv_rel", "menv_curve"];
    if resp.drag_started() {
        let o = press.unwrap_or(pl.center());
        if let Some(h) = hit(o) {
            let cur = pos.unwrap_or(o);
            let start = [c.get01("menv_att"), c.get01("menv_dec"), c.get01("menv_sus")];
            st.drag = Some(Drag::Env { h, acc: (cur - o) * if shift { FINE } else { 1.0 }, start });
            // One host gesture per touched param.
            let touched: &[&str] = match h {
                0 => &["menv_att"],
                1 => &["menv_dec", "menv_sus"],
                2 => &["menv_rel"],
                _ => &["menv_curve"],
            };
            for t in touched {
                c.begin(t);
            }
            st.param = Some(touched.join(","));
        }
    }
    if resp.dragged() {
        if let Some(Drag::Env { h, acc, start }) = st.drag.clone() {
            let acc = acc + resp.drag_delta() * if shift { FINE } else { 1.0 };
            let o = press.unwrap_or(pl.center());
            let px_ = (o.x + acc.x).clamp(pl.left() + 1.0, pl.right() - 1.0);
            let x = (px_ - pl.left()) / pl.width();
            let t = |id: &str| (c.plain(id, c.get01(id)) + 0.02).sqrt();
            let (wa, wd, wr) = (t("menv_att"), t("menv_dec"), t("menv_rel"));
            let w2time = |id: &str, w: f32| c.norm(id, (w * w - 0.02).max(0.0)).clamp(0.0, 1.0);
            match h {
                0 => {
                    // x of the attack peak = wa / (1.5 (wa + wd + wr)): solve for wa with the others fixed.
                    let x = x.min(0.62);
                    let w = 1.5 * x * (wd + wr) / (1.0 - 1.5 * x);
                    c.set01("menv_att", w2time("menv_att", w));
                }
                1 => {
                    let xd = x.min(0.64).max(geo.xa + 0.01);
                    let w = ((1.5 * xd * (wa + wr) - wa) / (1.0 - 1.5 * xd)).max(0.1);
                    c.set01("menv_dec", w2time("menv_dec", w));
                    let y = ((pl.bottom() - (o.y + acc.y)) / pl.height()).clamp(0.0, 1.0);
                    c.set01("menv_sus", y);
                }
                2 => {
                    // The release end sits at the right edge: its drag scales the time (2x per 60 px).
                    let cur = c.get01("menv_rel");
                    let plain = c.plain("menv_rel", cur) * 2f32.powf(resp.drag_delta().x * if shift { FINE } else { 1.0 } / 60.0);
                    c.set01("menv_rel", c.norm("menv_rel", plain).clamp(0.0, 1.0));
                }
                _ => {
                    let cur = start; // (unused)
                    let _ = cur;
                    let v = (c.plain("menv_curve", c.get01("menv_curve")) - resp.drag_delta().y / 100.0 * if shift { FINE } else { 1.0 }).clamp(-1.0, 1.0);
                    c.set01("menv_curve", c.norm("menv_curve", v).clamp(0.0, 1.0));
                }
            }
            st.drag = Some(Drag::Env { h, acc, start });
            let readout = match h {
                0 => c.readout("menv_att"),
                1 => format!("{} \u{b7} {}", c.readout("menv_dec"), c.readout("menv_sus")),
                2 => c.readout("menv_rel"),
                _ => format!("CURVE {:+.2}", c.plain("menv_curve", c.get01("menv_curve"))),
            };
            st.pill = Some((readout, pos.unwrap_or(pl.center()), ui.input(|i| i.time)));
        }
    }
    if resp.drag_stopped() {
        st.drag = None;
        if let Some(p) = st.param.take() {
            for t in p.split(',') {
                c.end(t);
            }
        }
    }
    if resp.double_clicked() {
        if let Some(q) = pos {
            match hit(q) {
                Some(0) => one_shot(c, "menv_att", c.default01("menv_att")),
                Some(1) => {
                    one_shot(c, "menv_dec", c.default01("menv_dec"));
                    one_shot(c, "menv_sus", c.default01("menv_sus"));
                }
                Some(2) => one_shot(c, "menv_rel", c.default01("menv_rel")),
                Some(_) => one_shot(c, "menv_curve", c.default01("menv_curve")),
                None => {}
            }
        }
    }
    let _ = ids;

    // ---- painting ---------------------------------------------------------------------------------
    let p = ui.painter().clone();
    rr_ui::scope::small_glass(&p, g);
    let pxl = px(&p);
    let sn = |v: f32| snap_px(&p, v);
    let clip_p = p.with_clip_rect(g.intersect(p.clip_rect()));
    for i in 0..=4 {
        let y = sn(pl.bottom() - pl.height() * i as f32 / 4.0);
        clip_p.hline(pl.x_range(), y, Stroke::new(pxl, Color32::WHITE.gamma_multiply(if i == 0 { 0.12 } else { 0.06 })));
    }
    for (txt, v) in [("1", 1.0f32), (".5", 0.5), ("0", 0.0)] {
        clip_p.text(pos2(g.left() + 23.0, pl.bottom() - v * pl.height()), Align2::RIGHT_CENTER, txt, font::mono(11.0), T.disp_caption);
    }
    clip_p.text(pos2(g.right() - 8.0, g.top() + 10.0), Align2::RIGHT_CENTER, "ADSR \u{b7} CURVE", font::mono(11.0), T.disp_caption);
    // Segment boundaries and the note-off tick.
    for x in [a_pt.x, d_pt.x] {
        clip_p.vline(sn(x), pl.y_range(), Stroke::new(pxl, Color32::WHITE.gamma_multiply(0.06)));
    }
    dotted_v(&clip_p, sn(s_pt.x), pl.top(), pl.bottom(), col.gamma_multiply(0.5));
    clip_p.text(pos2(s_pt.x - 6.0, pl.top() + 8.0), Align2::RIGHT_CENTER, "note off", font::mono(11.0), T.disp_caption);
    let pts = env_points(&geo, pl);
    fill_to_baseline(&clip_p, &pts, pl.bottom(), col, 0.22);
    rr_ui::scope::glow_line(&clip_p, pts, 1.5, col);
    // Readouts under the segments.
    let ry = g.bottom() - 7.0;
    let mono = font::mono(11.0);
    for (x, txt) in [((pl.left() + a_pt.x) / 2.0, c.readout("menv_att")), ((a_pt.x + d_pt.x) / 2.0 + 14.0, c.readout("menv_dec")), ((d_pt.x + s_pt.x) / 2.0, format!("{:.0} %", c.plain("menv_sus", c.get01("menv_sus")) * 100.0)), ((s_pt.x + pl.right()) / 2.0 + 8.0, c.readout("menv_rel"))] {
        clip_p.text(pos2(x, ry), Align2::CENTER_CENTER, txt, mono.clone(), T.disp_caption);
    }
    // Handles: A peak, D / S corner, S end (a marker), R end; curve diamonds.
    let dragging = match &st.drag {
        Some(Drag::Env { h, .. }) => Some(*h),
        _ => None,
    };
    for (i, hp) in [(0u8, a_pt), (1, d_pt), (2, r_pt)] {
        let hot = hover_h == Some(i) || dragging == Some(i);
        let r = 3.5 + rr_ui::motion::tween(ui.ctx(), Id::new(("gts-env-h", i)), hot, 0.1);
        clip_p.circle_filled(hp, r, col);
        clip_p.circle_stroke(hp, r, rr_ui::paint::hair(&p, Color32::from_rgb(0x0b, 0x0f, 0x11)));
    }
    clip_p.circle_filled(s_pt, 2.5, col.gamma_multiply(0.7));
    for mp in mids {
        let hot = hover_h == Some(3) || dragging == Some(3);
        let r = 4.5;
        let pts = vec![pos2(mp.x, mp.y - r), pos2(mp.x + r, mp.y), pos2(mp.x, mp.y + r), pos2(mp.x - r, mp.y)];
        clip_p.add(Shape::convex_polygon(pts, if hot { col } else { Color32::from_rgb(0x0b, 0x0f, 0x11) }, rr_ui::paint::st(1.3, col.gamma_multiply(0.9))));
    }
    // Playheads: one per voice; the sustaining one creeps through the notional segment, the releasing one rides the release.
    let (q, pexp) = env_exps(geo.curve);
    let pt = |x: f32, y: f32| pos2(pl.left() + x * pl.width(), pl.bottom() - y * pl.height());
    for (v, a) in playhead_voices(f) {
        let (stg, lv) = (f.voice(v, vf::MENV_STAGE), f.voice(v, vf::MENV).clamp(0.0, 1.0));
        let x = match stg as i32 {
            1 => geo.xa * (1.0 - (1.0 - lv).max(0.0).powf(1.0 / q)),
            2 => geo.xa + (geo.xd - geo.xa) * (1.0 - ((lv - geo.sus) / (1.0 - geo.sus).max(1e-3)).clamp(0.0, 1.0).powf(1.0 / pexp)),
            3 => {
                let age = f.voice(v, vf::AGE);
                geo.xd + (geo.xs - geo.xd) * (age * 0.15).fract()
            }
            4 => geo.xs + (1.0 - geo.xs) * (1.0 - (lv / geo.sus.max(0.05)).clamp(0.0, 1.0).powf(1.0 / pexp)),
            _ => continue,
        };
        ph_head(&clip_p, pl, pt(x, lv), col, a);
    }
    if resp.hovered() {
        let (name, line) = match hover_h {
            Some(3) => ("ENV CURVE", "env:curve"),
            Some(_) => ("ENV HANDLE", "env:handle"),
            None => ("MOD ENV", "env:plot"),
        };
        let value = format!("A {} \u{b7} D {} \u{b7} S {:.0} % \u{b7} R {}", c.readout("menv_att"), c.readout("menv_dec"), c.plain("menv_sus", c.get01("menv_sus")) * 100.0, c.readout("menv_rel"));
        set_hover(ui, name, &value, None, crate::help::ui_help(line).unwrap_or(""));
    }
    stash_cost(ui.ctx(), t_start);
    // Controls row: Att, Dec, Sus, Rel, Curve, five 62.4 px cells.
    for (i, (id, label)) in [("menv_att", "Att"), ("menv_dec", "Dec"), ("menv_sus", "Sus"), ("menv_rel", "Rel"), ("menv_curve", "Curve")].into_iter().enumerate() {
        let r = Rect::from_min_size(pos2(m.left() + 12.0 + i as f32 * 62.4, m.top() + ROW_Y), vec2(62.4, 48.0));
        let style = if id == "menv_curve" { ks(62.4).bipolar() } else { ks(62.4) };
        put(ui, r, |ui| kn(ui, c, id, label, style));
    }
    let now = ui.input(|i| i.time);
    pill(ui, st, now, None);
}

/// Handles of the envelope: attack peak, decay / sustain corner, sustain end, release end.
fn env_handles(g: &EnvGeo, pl: Rect) -> (Pos2, Pos2, Pos2, Pos2) {
    let pt = |x: f32, y: f32| pos2(pl.left() + x * pl.width(), pl.bottom() - y * pl.height());
    (pt(g.xa, 1.0), pt(g.xd, g.sus), pt(g.xs, g.sus), pt(1.0, 0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The display's arithmetic is the DSP's.
    #[test]
    fn the_display_evaluates_shapes_like_the_dsp() {
        for (name, d) in library() {
            for i in 0..=200 {
                let x = i as f32 / 200.0;
                let want = d.eval(x as f64);
                let got = if d.mode == Mode::Steps { eval_steps(&d.steps, d.glide, x) } else { eval_points(&d.points, x) };
                assert!((want - got).abs() < 1e-4, "{name} at {x}: {want} vs {got}");
            }
        }
    }

    #[test]
    fn the_polyline_follows_the_shape() {
        for (name, d) in library() {
            let line = shape_line(&d);
            assert!(line.len() >= 2, "{name}");
            assert!(line.iter().all(|(x, v)| (0.0..=1.0).contains(x) && (-1.0001..=1.0001).contains(v)), "{name}");
            // Every vertex of the line is on the shape (or on a vertical edge of it).
            for (x, v) in &line {
                let near = [*x - 1e-3, *x, *x + 1e-3].iter().any(|&xx| {
                    let e = if d.mode == Mode::Steps { eval_steps(&d.steps, d.glide, xx.clamp(0.0, 1.0)) } else { eval_points(&d.points, xx.clamp(0.0, 1.0)) };
                    (e - v).abs() < 0.02
                });
                let edge = d.mode == Mode::Steps || d.points.iter().any(|p| (p[0] - x).abs() < 1e-4);
                assert!(near || edge, "{name}: ({x}, {v}) is off the shape");
            }
        }
    }

    #[test]
    fn built_in_curves_convert_to_editable_shapes() {
        for curve in [Curve::Sine, Curve::Triangle, Curve::SawUp, Curve::SawDown, Curve::Square, Curve::Decay, Curve::Attack] {
            let d = shape_of_curve(curve).expect("convertible");
            assert!(d.points.len() <= MAX_POINTS);
            for i in 0..=40 {
                let x = i as f32 / 40.0;
                let want = Lfo::curve_at(curve, x as f64);
                assert!((eval_points(&d.points, x.min(0.9999)) - want).abs() < 0.12, "{curve:?} at {x}");
            }
        }
        assert!(shape_of_curve(Curve::SampleHold).is_none() && shape_of_curve(Curve::Custom).is_none());
    }

    #[test]
    fn the_layout_matches_the_addendum() {
        let g = glass();
        assert_eq!((g.min.x, g.min.y, g.width(), g.height()), (632.0, 148.0, 312.0, 130.0));
        let p = plot();
        assert_eq!((p.left(), p.top(), p.right(), p.bottom()), (660.0, 174.0, 936.0, 262.0));
        assert_eq!(point_pos([0.0, 0.0, 0.0]).y, 218.0); // y 70 of the glass
        // Tabs: 4 x 56 + 76 with 2 px gaps and 2 px padding fill the 312 px strip.
        assert_eq!(TAB_X[4] + TAB_W[4] + 2.0, 12.0 + 312.0);
        // Toolbar: LIB + STEP + SNAP + GRID with 6 px gaps fill 312 px.
        assert_eq!(tool_rect(Tool::Grid, false).right() - tool_rect(Tool::Lib, false).left(), 312.0);
        assert!((tool_rect(Tool::Glide, true).right() - tool_rect(Tool::Lib, true).left() - 312.0).abs() < 1e-3);
    }

    #[test]
    fn snapping_goes_to_grid_slots_and_eighths() {
        assert_eq!(snap_x(0.30, 8), 0.25);
        assert!((snap_x(0.30, 12) - 4.0 / 12.0).abs() < 1e-6);
        assert_eq!(snap_y(0.30), 0.25);
        assert_eq!(snap_y(-0.9), -0.875);
    }

    #[test]
    fn hit_testing_prefers_points_then_the_diamond() {
        let pl = plot();
        let d = ShapeDef::points(&[[0.0, 0.0, 0.0], [0.5, 1.0, 0.0], [1.0, 0.0, 0.0]]);
        assert_eq!(hit_test(&d, pl, point_pos(d.points[1])), Hit::Point(1));
        let mid = seg_mid_in(pl, d.points[0], d.points[1]);
        assert_eq!(hit_test(&d, pl, mid), Hit::Curve(0));
        // 20 px above the line, nowhere near anything: empty.
        assert_eq!(hit_test(&d, pl, mid + vec2(0.0, -40.0)), Hit::None);
        assert!(matches!(hit_test(&d, pl, mid + vec2(-40.0, 8.0)), Hit::Seg(0) | Hit::None));
    }
}
