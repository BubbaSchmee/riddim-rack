//! The MODULATOR shape editor and the FX chain bar, headless (SPEC v2 Track 7, UI wave): real pointer
//! events through the real `draw`, then `ShapesShared` / `FxOrderShared` are read back. Every edit
//! must reach the shapes through `ShapesShared::edit`, mark the Live Set modified (`matrix_rev`
//! moves) and be one undo step per gesture, exactly like a matrix edit.
use gts::dsp::fx::FxOrder;
use gts::dsp::shape::{library, Mode, ShapeDef};
use gts::dsp::tables::Tables;
use gts::editor::chain;
use gts::editor::modulator::{self, Tool};
use gts::editor::patching::enable_test_hooks;
use gts::editor::{draw, BusView, Ctl, LiveView, Page, ParamTable, WINDOW};
use gts::matrix::MatrixShared;
use gts::params::GtsParams;
use nih_plug_egui::egui::{self, pos2, vec2, Event, Key, Modifiers, PointerButton, Pos2, Rect};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Default params plus overrides; remembers every gesture the editor sent.
struct Sim {
    table: ParamTable,
    vals: RefCell<HashMap<String, f32>>,
    log: RefCell<Vec<String>>,
}

impl Ctl for Sim {
    fn get01(&self, id: &str) -> f32 {
        self.vals.borrow().get(id).copied().unwrap_or_else(|| self.table.get01(id))
    }
    fn default01(&self, id: &str) -> f32 {
        self.table.default01(id)
    }
    fn readout(&self, id: &str) -> String {
        self.table.readout_at(id, self.get01(id))
    }
    fn steps(&self, id: &str) -> Option<usize> {
        self.table.steps(id)
    }
    fn begin(&self, id: &str) {
        self.log.borrow_mut().push(format!("begin:{id}"));
    }
    fn set01(&self, id: &str, v: f32) {
        self.log.borrow_mut().push(format!("set:{id}"));
        self.vals.borrow_mut().insert(id.to_string(), v);
    }
    fn end(&self, id: &str) {
        self.log.borrow_mut().push(format!("end:{id}"));
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
    fn matrix(&self) -> Option<&MatrixShared> {
        Some(self.table.matrix())
    }
}

/// A host stand-in for the editor's context: records each `matrix_rev` gesture.
#[derive(Default)]
struct FakeHost {
    log: Mutex<Vec<String>>,
}

impl FakeHost {
    /// How many undo steps the host was given (one begin / set / end each) and the revs set.
    fn steps(&self) -> Vec<i32> {
        self.log.lock().unwrap().iter().filter_map(|l| l.parse::<i32>().ok()).collect()
    }
}

impl nih_plug::prelude::GuiContext for FakeHost {
    fn plugin_api(&self) -> nih_plug::prelude::PluginApi {
        nih_plug::prelude::PluginApi::Vst3
    }
    fn request_resize(&self) -> bool {
        false
    }
    unsafe fn raw_begin_set_parameter(&self, _: nih_plug::prelude::ParamPtr) {
        self.log.lock().unwrap().push("begin".into());
    }
    unsafe fn raw_set_parameter_normalized(&self, _: nih_plug::prelude::ParamPtr, v: f32) {
        self.log.lock().unwrap().push(format!("{}", (v * gts::matrix::REV_MAX as f32).round()));
    }
    unsafe fn raw_end_set_parameter(&self, _: nih_plug::prelude::ParamPtr) {
        self.log.lock().unwrap().push("end".into());
    }
    fn get_state(&self) -> nih_plug::prelude::PluginState {
        unimplemented!()
    }
    fn set_state(&self, _: nih_plug::prelude::PluginState) {}
}

struct H {
    ctx: egui::Context,
    ctl: Sim,
    host: Arc<FakeHost>,
    params: Arc<GtsParams>,
    tables: Tables,
    live: LiveView,
    bv: BusView,
    t: f64,
    mods: Modifiers,
    pos: Pos2,
}

impl H {
    /// An editor on LFO A with a Custom curve (the default triangle) and a host attached.
    fn new() -> Self {
        let ctx = egui::Context::default();
        rr_ui::theme::apply(&ctx);
        enable_test_hooks(&ctx);
        let mut tables = Tables::new();
        tables.generate();
        let params = Arc::new(GtsParams::default());
        let ctl = Sim { table: ParamTable::new(params.clone()), vals: Default::default(), log: Default::default() };
        let host = Arc::new(FakeHost::default());
        ctl.table.matrix().attach(Some(host.clone()));
        ctl.vals.borrow_mut().insert("moda_curve".into(), 1.0); // Custom
        let mut h = Self { ctx, ctl, host, params, tables, live: LiveView::idle(), bv: BusView { oversampling: 2, ..Default::default() }, t: 0.0, mods: Modifiers::NONE, pos: pos2(5.0, 5.0) };
        for _ in 0..4 {
            h.frame(vec![]);
        }
        h
    }

    fn frame(&mut self, events: Vec<Event>) {
        self.t += 1.0 / 60.0;
        let raw = egui::RawInput {
            time: Some(self.t),
            events,
            modifiers: self.mods,
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(WINDOW.0 as f32, WINDOW.1 as f32))),
            ..Default::default()
        };
        let _ = self.ctx.run(raw, |ctx| {
            egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
                draw(ui, &self.ctl, &self.tables, &mut self.live, &mut self.bv);
            });
        });
    }

    fn frames(&mut self, n: usize) {
        for _ in 0..n {
            self.frame(vec![]);
        }
    }

    fn mv(&mut self, p: Pos2) {
        self.pos = p;
        self.frame(vec![Event::PointerMoved(p)]);
    }

    fn btn(&mut self, pressed: bool, button: PointerButton) {
        let (pos, modifiers) = (self.pos, self.mods);
        self.frame(vec![Event::PointerButton { pos, button, pressed, modifiers }]);
    }

    fn glide(&mut self, to: Pos2, steps: usize) {
        let from = self.pos;
        for i in 1..=steps {
            self.mv(from + (to - from) * (i as f32 / steps as f32));
        }
    }

    /// Press at `from`, drag to `to`, release.
    fn drag(&mut self, from: Pos2, to: Pos2) {
        self.mv(from);
        self.btn(true, PointerButton::Primary);
        self.glide(from + (to - from).normalized() * 14.0, 2); // past egui's click distance
        self.glide(to, 8);
        self.btn(false, PointerButton::Primary);
        self.frames(2);
    }

    fn click(&mut self, p: Pos2, button: PointerButton) {
        self.mv(p);
        self.btn(true, button);
        self.btn(false, button);
        self.frame(vec![]);
    }

    fn double_click(&mut self, p: Pos2) {
        // A pause first: clicks close in time and place add up to a triple click.
        self.frames(30);
        self.click(p, PointerButton::Primary);
        self.click(p, PointerButton::Primary);
        self.frames(1);
    }

    fn key(&mut self, key: Key) {
        let modifiers = self.mods;
        self.frame(vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }]);
        self.frame(vec![Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers }]);
    }

    fn shape(&self, k: usize) -> ShapeDef {
        self.params.lfo_shapes.get().0[k].clone()
    }

    fn order(&self) -> FxOrder {
        self.params.fx_order.get()
    }

    fn steps(&self) -> usize {
        self.host.steps().len()
    }
}

fn near(a: f32, b: f32, eps: f32, what: &str) {
    assert!((a - b).abs() < eps, "{what}: {a} vs {b}");
}

#[test]
fn dragging_a_point_edits_the_shape_in_one_undo_step() {
    let mut h = H::new();
    let before = h.shape(0);
    assert_eq!(before.points.len(), 4);
    let from = modulator::point_pos(before.points[1]); // (0.25, +1)
    h.ctx.set_pixels_per_point(1.0);
    // Snap off: a free drag to x 0.5 and y 0.
    let (snap, grid) = modulator::snap_grid(&h.ctx);
    assert!(snap && grid == 2, "SNAP on at 1/8 by default");
    h.click(modulator::tool_rect(Tool::Snap, false).center(), PointerButton::Primary);
    assert!(!modulator::snap_grid(&h.ctx).0);
    assert_eq!(h.steps(), 0, "a toolbar toggle is not a shape edit");
    let to = from + vec2(60.0, 40.0);
    h.drag(from, to);
    let after = h.shape(0);
    assert_eq!(after.points.len(), 4);
    assert!(after.points[1][0] > 0.4 && after.points[1][0] < 0.5, "x moved right, clamped before the next point: {:?}", after.points[1]);
    assert!(after.points[1][1] < 0.9, "y moved down: {:?}", after.points[1]);
    assert_eq!(after.points[0], before.points[0]);
    assert_eq!(h.steps(), 1, "one drag, one host gesture: {:?}", h.host.log.lock().unwrap());
    // A second drag is a second step.
    h.drag(modulator::point_pos(after.points[2]), modulator::point_pos(after.points[2]) + vec2(0.0, -30.0));
    assert_eq!(h.steps(), 2);
    assert!(h.shape(0).points[2][1] > after.points[2][1] + 0.2);
    // Undo, as the host does it: rev back, then the background restore.
    let shared = h.ctl.table.matrix();
    let revs = h.host.steps();
    assert_eq!(shared.host_moved(revs[1]), None, "the host reports the GUI's own rev");
    assert_eq!(shared.host_moved(revs[0]), Some(revs[0]), "the host moved matrix_rev back to the first drag's rev");
    assert!(shared.restore(revs[0]));
    assert_eq!(h.shape(0), after, "undo of the second drag");
    assert_eq!(shared.host_moved(0), Some(0));
    assert!(shared.restore(0));
    assert_eq!(h.shape(0), before, "undo of the first drag restores the shapes too");
}

#[test]
fn a_drag_with_snap_lands_on_the_grid_and_the_ends_only_move_vertically() {
    let mut h = H::new();
    let p = h.shape(0).points.clone();
    // Middle point: x snaps to eighths, y to eighths.
    let from = modulator::point_pos(p[2]); // (0.75, -1)
    h.drag(from, from + vec2(-40.0, 20.0));
    let q = h.shape(0).points[2];
    near((q[0] * 8.0).round(), q[0] * 8.0, 1e-4, "x on the 1/8 grid");
    near((q[1] * 8.0).round(), q[1] * 8.0, 1e-4, "y on an eighth");
    assert!(q[0] < 0.75);
    // The last point (x = 1) only moves vertically, however far it is dragged sideways.
    let last = modulator::point_pos(p[3]);
    h.drag(last, last + vec2(-50.0, -30.0));
    let l = h.shape(0).points[3];
    assert_eq!(l[0], 1.0, "pinned in x");
    assert!(l[1] > 0.2, "moved up: {l:?}");
}

#[test]
fn double_click_adds_and_deletes_points_and_the_ends_stay() {
    let mut h = H::new();
    let pl = modulator::plot();
    // Add: empty space at x 0.5 / y +0.5 (snapped to the grid and eighths).
    let at = pos2(pl.left() + 0.5 * pl.width(), pl.center().y - 0.5 * pl.height() / 2.0 - 3.0);
    h.double_click(at);
    let d = h.shape(0);
    assert_eq!(d.points.len(), 5, "{:?}", d.points);
    let added = d.points.iter().find(|p| (p[0] - 0.5).abs() < 1e-4).expect("a point at x 0.5");
    near(added[1], 0.5, 0.13, "y near the pointer");
    assert!(d.points.windows(2).all(|w| w[0][0] <= w[1][0]), "sorted by x");
    assert_eq!(h.steps(), 1, "the add is one undo step");
    // Delete by double-click.
    h.double_click(modulator::point_pos(*added));
    assert_eq!(h.shape(0).points.len(), 4);
    assert_eq!(h.steps(), 2);
    // Delete by right-click.
    let p1 = h.shape(0).points[1];
    h.click(modulator::point_pos(p1), PointerButton::Secondary);
    assert_eq!(h.shape(0).points.len(), 3);
    assert_eq!(h.steps(), 3);
    // The first and the last point cannot be deleted.
    let (first, last) = (h.shape(0).points[0], *h.shape(0).points.last().unwrap());
    h.double_click(modulator::point_pos(first));
    h.click(modulator::point_pos(last), PointerButton::Secondary);
    assert_eq!(h.shape(0).points.len(), 3, "the pinned points stay");
    assert_eq!(h.steps(), 3, "and a refused delete is no undo step");
    // At 32 points an add is refused with a help-bar line.
    h.ctx.set_pixels_per_point(1.0);
    gts_fill_to_max(&mut h);
    let n = h.shape(0).points.len();
    let before = h.steps();
    h.double_click(pos2(pl.left() + 100.0, pl.top() + 2.0));
    assert_eq!(h.shape(0).points.len(), n, "at the limit nothing is added");
    assert_eq!(h.steps(), before);
    assert!(gts::editor::patching::current_notice(&h.ctx).is_some_and(|m| m.contains("32")));
}

/// Fills LFO A's shape to 32 points through the shared state (no gestures).
fn gts_fill_to_max(h: &mut H) {
    h.params.lfo_shapes.edit(|s| {
        s.0[0].points = (0..32).map(|i| [i as f32 / 31.0, if i % 2 == 0 { 0.5 } else { -0.5 }, 0.0]).collect();
    });
    h.frames(2);
}

#[test]
fn dragging_a_segment_diamond_bends_it_and_double_click_straightens() {
    let mut h = H::new();
    let p = h.shape(0).points.clone();
    // The middle of segment 0 (0,0) -> (0.25, 1): the diamond appears on hover.
    let mid = pos2((modulator::point_pos(p[0]).x + modulator::point_pos(p[1]).x) / 2.0, (modulator::point_pos(p[0]).y + modulator::point_pos(p[1]).y) / 2.0);
    h.mv(mid);
    h.frames(2);
    h.drag(mid, mid + vec2(0.0, -40.0));
    let c = h.shape(0).points[0][2];
    // Dragging up on a rising segment bulges it upward: a negative curve (t^(8^c) with c < 0).
    assert!(c < -0.1, "curve {c}");
    assert_eq!(h.shape(0).points[1], p[1], "the points did not move");
    assert_eq!(h.steps(), 1);
    // Double-click on the diamond: straight again.
    let q = h.shape(0).points.clone();
    let mid = {
        let (a, b) = (modulator::point_pos(q[0]), modulator::point_pos(q[1]));
        // The diamond sits on the curve at t = 0.5: y of the curved midpoint.
        let y = q[0][1] + (q[1][1] - q[0][1]) * 0.5f32.powf(8f32.powf(q[0][2]));
        pos2((a.x + b.x) / 2.0, modulator::point_pos([0.0, y, 0.0]).y)
    };
    h.mv(mid);
    h.frames(2);
    h.double_click(mid);
    near(h.shape(0).points[0][2], 0.0, 1e-6, "straightened");
}

#[test]
fn shift_drags_one_tenth_as_far() {
    let mut h = H::new();
    h.click(modulator::tool_rect(Tool::Snap, false).center(), PointerButton::Primary); // snap off
    let p = h.shape(0).points[2];
    let from = modulator::point_pos(p);
    h.drag(from, from + vec2(0.0, -40.0));
    let coarse = h.shape(0).points[2][1] - p[1];
    h.mods = Modifiers::SHIFT;
    let p2 = h.shape(0).points[1];
    let from = modulator::point_pos(p2);
    h.drag(from, from + vec2(0.0, 40.0));
    h.mods = Modifiers::NONE;
    let fine = p2[1] - h.shape(0).points[1][1];
    assert!(coarse > 0.8 && fine > 0.0 && fine < coarse * 0.25, "coarse {coarse}, fine {fine}");
}

#[test]
fn step_mode_paints_bars_and_the_steps_round_trip() {
    let mut h = H::new();
    // STEP toggle: the mode is part of the shape, so the switch is one undo step.
    h.click(modulator::tool_rect(Tool::Step, false).center(), PointerButton::Primary);
    h.frames(2);
    let d = h.shape(0);
    assert_eq!(d.mode, Mode::Steps);
    assert_eq!(h.steps(), 1);
    let n = d.steps.len();
    assert_eq!(n, 8);
    // Press on bar 2 at the top and sweep right along the bottom: bars 2.. are painted by the pointer.
    h.click(modulator::tool_rect(Tool::Snap, true).center(), PointerButton::Primary); // snap off for exact paints
    let pl = modulator::plot();
    let (top, bottom) = (pl.top() + 1.0, pl.bottom() - 1.0);
    let w = pl.width() / n as f32;
    let start = pos2(pl.left() + 2.5 * w, top);
    let before = h.steps();
    h.mv(start);
    h.btn(true, PointerButton::Primary);
    for k in 0..6 {
        h.mv(pos2(start.x + k as f32 * w * 0.5, if k < 2 { top } else { bottom }));
    }
    h.btn(false, PointerButton::Primary);
    h.frames(2);
    let s = h.shape(0).steps;
    assert!(s[2] > 0.9, "bar 2 at the top: {s:?}");
    assert!(s[3] > 0.9 || s[4] < -0.9, "the sweep painted the next bars: {s:?}");
    assert!(s[5] < -0.9, "bar 5 at the bottom: {s:?}");
    assert_eq!(s[0], d.steps[0], "bars outside the sweep are untouched");
    assert_eq!(h.steps(), before + 1, "one sweep, one undo step");
    // Glide slider: a drag across the slider sets glide; double-click sets 0.
    let g = modulator::tool_rect(Tool::Glide, true);
    h.drag(pos2(g.left() + 4.0, g.center().y), pos2(g.right() - 6.0, g.center().y));
    assert!(h.shape(0).glide > 0.8, "glide {}", h.shape(0).glide);
    h.double_click(g.center());
    assert_eq!(h.shape(0).glide, 0.0);
}

#[test]
fn the_library_loads_a_stock_shape_as_custom_in_one_undo_step() {
    let mut h = H::new();
    h.ctl.vals.borrow_mut().remove("moda_curve"); // a built-in curve now
    h.click(modulator::tool_rect(Tool::Lib, false).center(), PointerButton::Primary);
    h.frames(10);
    assert!(modulator::view(&h.ctx).1, "the popover is open");
    // Row of "Bounce" (index 2) in the first column: 8 rows of 24 px from y 148 + 26 + 22 + 20 + 4.
    let lib = library();
    let i = lib.iter().position(|(n, _)| *n == "Bounce").unwrap();
    let rows = (lib.len() + 1) / 2;
    let (col, row) = (i / rows, i % rows);
    let origin = pos2(620.0, 148.0);
    let at = origin + vec2(8.0 + col as f32 * 160.0 + 60.0, 26.0 + 22.0 + 20.0 + 4.0 + row as f32 * 24.0 + 12.0);
    h.click(at, PointerButton::Primary);
    h.frames(10);
    assert_eq!(h.shape(0), lib[i].1, "the stock points are copied into the LFO");
    assert_eq!(h.ctl.get01("moda_curve"), 1.0, "and its Curve is Custom");
    assert!(!modulator::view(&h.ctx).1, "the popover closes");
    assert_eq!(h.steps(), 1);
    // Esc closes an open popover without loading.
    h.click(modulator::tool_rect(Tool::Lib, false).center(), PointerButton::Primary);
    h.frames(10);
    assert!(modulator::view(&h.ctx).1);
    h.key(Key::Escape);
    h.frames(10);
    assert!(!modulator::view(&h.ctx).1);
    assert_eq!(h.steps(), 1);
}

#[test]
fn editing_a_built_in_curve_turns_the_lfo_custom() {
    let mut h = H::new();
    h.ctl.vals.borrow_mut().insert("moda_curve".into(), 0.0); // Sine
    h.frames(3);
    let pl = modulator::plot();
    h.double_click(pos2(pl.left() + 100.0, pl.top() + 10.0));
    assert_eq!(h.ctl.get01("moda_curve"), 1.0, "Custom now");
    let d = h.shape(0);
    assert!(d.points.len() >= 10, "the sine was sampled into points: {}", d.points.len());
}

#[test]
fn the_phase_marker_edits_the_phase_param_as_one_host_gesture() {
    let mut h = H::new();
    h.ctl.log.borrow_mut().clear();
    let pl = modulator::plot();
    let g = modulator::glass();
    let mark = pos2(pl.left(), g.top() + 12.0);
    h.drag(mark, mark + vec2(pl.width() * 0.25, 0.0));
    let log = h.ctl.log.borrow().clone();
    assert_eq!(log.first().map(String::as_str), Some("begin:moda_phase"), "{log:?}");
    assert_eq!(log.last().map(String::as_str), Some("end:moda_phase"), "{log:?}");
    assert!(log.iter().all(|l| l.ends_with("moda_phase")), "{log:?}");
    let v = h.ctl.get01("moda_phase");
    assert!(v > 0.15 && v < 0.3, "phase about a quarter turn: {v}");
    assert_eq!(h.steps(), 0, "a param, not a shape edit");
}

#[test]
fn tabs_switch_and_the_env_handles_edit_params() {
    let mut h = H::new();
    h.click(modulator::tab_rect(4).center(), PointerButton::Primary);
    h.frames(30);
    assert_eq!(modulator::view(&h.ctx).0, 4);
    // The decay / sustain corner: x = decay, y = sustain.
    h.ctl.log.borrow_mut().clear();
    let pl = modulator::plot();
    // A press on the corner handle (found by scanning the plot for the first hit is overkill: use the geometry of the defaults).
    let (att, dec, rel) = (0.001f32, 0.25, 0.1);
    let w = |t: f32| (t + 0.02).sqrt();
    let (wa, wd, wr) = (w(att), w(dec), w(rel));
    let sum = wa + wd + wr + 0.5 * (wa + wd + wr);
    let corner = pos2(pl.left() + (wa + wd) / sum * pl.width(), pl.bottom() - 0.0 * pl.height());
    h.drag(corner, corner + vec2(20.0, -30.0));
    let log = h.ctl.log.borrow().clone();
    assert!(log.iter().any(|l| l == "begin:menv_dec") && log.iter().any(|l| l == "begin:menv_sus"), "{log:?}");
    assert!(log.iter().any(|l| l == "end:menv_dec") && log.iter().any(|l| l == "end:menv_sus"), "{log:?}");
    assert!(h.ctl.get01("menv_sus") > 0.3, "sustain followed the pointer up: {}", h.ctl.get01("menv_sus"));
    assert_eq!(h.steps(), 0);
    // Switching back to a LFO tab restores its editor.
    h.click(modulator::tab_rect(1).center(), PointerButton::Primary);
    h.frames(30);
    assert_eq!(modulator::view(&h.ctx).0, 1);
}

#[test]
fn the_tab_dot_starts_a_patch_drag_like_a_chip() {
    let mut h = H::new();
    // Drag the Mod C tab's dot onto the Shaper Amt knob.
    let dot = modulator::tab_rect(2).min + vec2(11.0, 11.0);
    let knob = gts::editor::patching::knob_rects(&h.ctx).into_iter().find(|k| k.0 == "shaper_amt").map(|k| k.1.center()).expect("knob");
    h.drag(dot, knob);
    let m = h.ctl.table.matrix().get();
    assert_eq!(m.slots.len(), 1, "{:?}", m.slots);
    assert_eq!((m.slots[0].source, m.slots[0].target.as_str(), m.slots[0].amount), (gts::matrix::Source::ModC, "shaper_amt", 0.5));
    assert_eq!(modulator::view(&h.ctx).0, 0, "a drag on the dot does not switch tabs");
}

// ---- the FX chain ------------------------------------------------------------------------------

fn on_fx_page() -> H {
    let mut h = H::new();
    h.bv.page = Page::Fx;
    h.frames(30);
    h
}

fn chip(h: &H, m: usize) -> Rect {
    chain::chip_rects(&h.ctx).into_iter().find(|c| c.0 == m).unwrap_or_else(|| panic!("chip {m} was not drawn")).1
}

#[test]
fn dragging_a_chip_reorders_the_chain_in_one_undo_step() {
    let mut h = on_fx_page();
    assert_eq!(h.order(), FxOrder::default());
    // Flanger (5) onto Chorus (4): it lands before it.
    let (from, to) = (chip(&h, 5).center(), chip(&h, 4).center() + vec2(-6.0, 0.0));
    h.mv(from);
    h.btn(true, PointerButton::Primary);
    h.glide(from + vec2(-12.0, 0.0), 2);
    h.glide(to, 8);
    assert_eq!(chain::drag_state(&h.ctx).map(|d| d.0), Some(5), "the drag is live");
    assert_eq!(h.order(), FxOrder::default(), "nothing is written while the drag only previews");
    assert_eq!(h.steps(), 0);
    h.btn(false, PointerButton::Primary);
    h.frames(30);
    let names = h.order().names();
    assert_eq!(names[4], "Flanger", "{names:?}");
    assert_eq!(names[5], "Chorus", "{names:?}");
    assert_eq!(h.steps(), 1, "one drop, one undo step");
    // The chain bar shows the new order: the Flanger chip is left of the Chorus chip.
    assert!(chip(&h, 5).left() < chip(&h, 4).left());
    // Undo restores the order (history snapshots cover it).
    let shared = h.ctl.table.matrix();
    assert_eq!(shared.host_moved(1), None, "the GUI's own rev");
    assert_eq!(shared.host_moved(0), Some(0));
    assert!(shared.restore(0));
    assert_eq!(h.order(), FxOrder::default());
}

#[test]
fn esc_or_a_release_away_from_the_bar_cancels_a_chain_drag() {
    let mut h = on_fx_page();
    let from = chip(&h, 5).center();
    h.mv(from);
    h.btn(true, PointerButton::Primary);
    h.glide(chip(&h, 1).center(), 10);
    assert!(chain::drag_state(&h.ctx).is_some());
    h.key(Key::Escape);
    h.btn(false, PointerButton::Primary);
    h.frames(5);
    assert_eq!(h.order(), FxOrder::default(), "Esc cancels");
    assert_eq!(h.steps(), 0);
    // Released far below the bar: cancelled as well.
    h.mv(from);
    h.btn(true, PointerButton::Primary);
    h.glide(pos2(300.0, 400.0), 10);
    h.btn(false, PointerButton::Primary);
    h.frames(5);
    assert_eq!(h.order(), FxOrder::default());
    assert_eq!(h.steps(), 0);
}

#[test]
fn dragging_a_module_grip_onto_another_module_takes_its_place() {
    let mut h = on_fx_page();
    // Reverb (8) by its grip onto Distortion's tile: first in the chain.
    let grip = chain::grip_rect(8).center();
    let to = chain::module_rect(0).center();
    h.drag(grip, to);
    h.frames(20);
    let names = h.order().names();
    assert_eq!(names[0], "Reverb", "{names:?}");
    assert_eq!(names[1], "Distortion");
    assert_eq!(h.steps(), 1);
    // The module grid itself did not move: the Reverb tile is still bottom right.
    assert_eq!(chain::module_rect(8).min, pos2(616.0, 550.0));
}

#[test]
fn undo_and_redo_restore_shapes_and_order_together() {
    let h = on_fx_page();
    // Edit 1: the order. Edit 2: a shape. Each is a rev; every snapshot is the whole document.
    h.ctl.table.matrix().set_fx_order(FxOrder::new([1, 0, 2, 3, 4, 5, 6, 7, 8]).unwrap());
    let reordered = h.order();
    h.ctl.table.matrix().edit_shapes(|s| s.0[2].glide = 0.5);
    let revs = h.host.steps();
    assert_eq!(revs.len(), 2);
    let shared = h.ctl.table.matrix();
    // Undo the shape edit: the shape goes back, the order stays.
    assert_eq!(shared.host_moved(revs[1]), None, "the GUI's own rev");
    assert_eq!(shared.host_moved(revs[0]), Some(revs[0]));
    assert!(shared.restore(revs[0]));
    assert_eq!(h.shape(2).glide, ShapeDef::default().glide);
    assert_eq!(h.order(), reordered);
    // Undo the reorder: the default order again.
    assert_eq!(shared.host_moved(0), Some(0));
    assert!(shared.restore(0));
    assert_eq!(h.order(), FxOrder::default());
    // Redo both.
    assert_eq!(shared.host_moved(revs[1]), Some(revs[1]));
    assert!(shared.restore(revs[1]));
    assert_eq!(h.shape(2).glide, 0.5);
    assert_eq!(h.order(), reordered);
    // A preset load is one step for all three parts.
    let before = h.host.steps().len();
    h.ctl.table.matrix().replace_doc(Default::default(), Default::default(), Default::default());
    assert_eq!(h.host.steps().len(), before + 1);
    assert_eq!(h.order(), FxOrder::default());
    assert_eq!(h.shape(2), ShapeDef::default());
}

#[test]
fn the_keyboard_moves_and_deletes_the_selected_point() {
    let mut h = H::new();
    let p = h.shape(0).points.clone();
    // Click a point: it is selected and the plot takes the focus.
    h.click(modulator::point_pos(p[1]), PointerButton::Primary);
    h.frames(2);
    h.key(Key::ArrowRight);
    let q = h.shape(0).points[1];
    near(q[0], p[1][0] + 0.05, 1e-4, "an arrow moves 1/20 of the plot");
    assert_eq!(q[1], p[1][1]);
    h.mods = Modifiers::SHIFT;
    h.key(Key::ArrowRight);
    h.mods = Modifiers::NONE;
    near(h.shape(0).points[1][0], p[1][0] + 0.055, 1e-4, "Shift: 1/200");
    h.key(Key::ArrowDown);
    assert!(h.shape(0).points[1][1] < p[1][1] - 0.05, "{:?}", h.shape(0).points[1]);
    assert_eq!(h.steps(), 3, "each key press is an undo step");
    h.key(Key::Delete);
    assert_eq!(h.shape(0).points.len(), 3, "Delete removes the selected point");
    // Tab cycles to the next point.
    h.key(Key::Tab);
    h.key(Key::ArrowUp);
    assert!(h.shape(0).points.iter().zip(&p).any(|(a, b)| a != b));
}

#[test]
fn a_focused_chain_chip_moves_with_alt_arrows() {
    let mut h = on_fx_page();
    h.click(chip(&h, 5).center(), PointerButton::Primary); // Flanger takes the focus
    h.frames(2);
    h.mods = Modifiers::ALT;
    h.key(Key::ArrowRight);
    h.mods = Modifiers::NONE;
    h.frames(5);
    let names = h.order().names();
    assert_eq!((names[5], names[6]), ("Dimension", "Flanger"), "{names:?}");
    assert_eq!(h.steps(), 1);
    h.mods = Modifiers::ALT;
    h.key(Key::ArrowLeft);
    h.mods = Modifiers::NONE;
    assert_eq!(h.order(), FxOrder::default());
}
