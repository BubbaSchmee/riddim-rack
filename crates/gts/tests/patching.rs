//! Drag-to-patch, ring depth editing, the knob menu and the MATRIX page, headless (SPEC v2 Track 5,
//! acceptance 8): real pointer events through the real `draw`, then the matrix is read back.
use gts::dsp::tables::Tables;
use gts::editor::patching::{chip_rects, current_notice, enable_test_hooks, knob_rects, matrix_rows, menu_rects};
use gts::editor::{draw, BusView, Ctl, LiveView, Page, ParamTable, WINDOW};
use gts::matrix::{Matrix, MatrixEditor, MatrixShared, Slot, Source};
use gts::params::GtsParams;
use nih_plug_egui::egui::{self, pos2, vec2, Event, Modifiers, PointerButton, Pos2, Rect};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

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

struct H {
    ctx: egui::Context,
    ctl: Sim,
    tables: Tables,
    live: LiveView,
    bv: BusView,
    t: f64,
    mods: Modifiers,
    pos: Pos2,
}

impl H {
    fn new() -> Self {
        let ctx = egui::Context::default();
        rr_ui::theme::apply(&ctx);
        enable_test_hooks(&ctx);
        let mut tables = Tables::new();
        tables.generate();
        let ctl = Sim { table: ParamTable::new(Arc::new(GtsParams::default())), vals: Default::default(), log: Default::default() };
        let mut h = Self { ctx, ctl, tables, live: LiveView::idle(), bv: BusView { oversampling: 2, ..Default::default() }, t: 0.0, mods: Modifiers::NONE, pos: pos2(5.0, 5.0) };
        for _ in 0..4 {
            h.frame(vec![]);
        }
        h
    }

    fn frame(&mut self, events: Vec<Event>) -> egui::Vec2 {
        self.t += 1.0 / 60.0;
        let raw = egui::RawInput {
            time: Some(self.t),
            events,
            modifiers: self.mods,
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(WINDOW.0 as f32, WINDOW.1 as f32))),
            ..Default::default()
        };
        let mut over = egui::Vec2::ZERO;
        let _ = self.ctx.run(raw, |ctx| {
            egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| over = draw(ui, &self.ctl, &self.tables, &mut self.live, &mut self.bv));
        });
        over
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

    /// Moves to `to` in `steps` frames.
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

    fn knob(&self, id: &str) -> Rect {
        knob_rects(&self.ctx).into_iter().find(|k| k.0 == id).unwrap_or_else(|| panic!("knob {id} was not drawn")).1
    }

    fn chip(&self, s: Source) -> Rect {
        chip_rects(&self.ctx).into_iter().find(|k| k.0 == s).unwrap_or_else(|| panic!("chip {s:?} was not drawn")).1
    }

    fn matrix(&self) -> Matrix {
        self.ctl.table.matrix().get()
    }

    /// Centre of a compact knob's dial (14 px label band above it).
    fn dial(&self, id: &str, d: f32) -> Pos2 {
        let r = self.knob(id);
        pos2(r.center().x, r.top() + 14.0 + d / 2.0)
    }
}

#[test]
fn chip_drag_dropped_on_a_knob_creates_a_slot_at_plus_half() {
    let mut h = H::new();
    assert!(h.matrix().slots.is_empty());
    let (chip, knob) = (h.chip(Source::ModB).center(), h.knob("shaper_amt").center());
    h.drag(chip, knob);
    let m = h.matrix();
    assert_eq!(m.slots.len(), 1, "one slot per drop: {:?}", m.slots);
    let s = &m.slots[0];
    assert_eq!((s.source, s.target.as_str()), (Source::ModB, "shaper_amt"));
    assert_eq!(s.amount, 0.5, "created at +50 %");
    assert!(s.enabled && s.bipolar, "Mod B is a bipolar source, new slots take its natural polarity");
    // The drag is over: another release elsewhere changes nothing.
    h.drag(h.chip(Source::ModA).center(), pos2(500.0, 790.0));
    assert_eq!(h.matrix().slots.len(), 1, "a drop off any knob cancels");
}

#[test]
fn a_different_source_adds_a_slot_and_the_same_source_edits_it() {
    let mut h = H::new();
    let knob = h.knob("shaper_amt").center();
    h.drag(h.chip(Source::ModB).center(), knob);
    MatrixEditor { shared: h.ctl.table.matrix(), ctl: &h.ctl }.set_amount(0, -0.3);
    // Same source again: the pair's slot is selected and keeps its depth, no second slot.
    h.drag(h.chip(Source::ModB).center(), knob);
    let m = h.matrix();
    assert_eq!((m.slots.len(), m.slots[0].amount), (1, -0.3), "{:?}", m.slots);
    // Another source on the same knob: a second slot with its own source.
    h.drag(h.chip(Source::AmpEnvA).center(), knob);
    let m = h.matrix();
    assert_eq!(m.slots.len(), 2);
    assert_eq!((m.slots[1].source, m.slots[1].target.as_str(), m.slots[1].amount), (Source::AmpEnvA, "shaper_amt", 0.5));
}

#[test]
fn a_source_dropped_on_a_built_in_route_edits_that_route_through_the_host() {
    let mut h = H::new();
    // Mod A -> Osc A Index is a fixed route (Moda Index), live while Mod A's target includes Osc A.
    h.drag(h.chip(Source::ModA).center(), h.knob("osca_index").center());
    assert!(h.matrix().slots.is_empty(), "no slot for a pair that has a built-in route");
    let log = h.ctl.log.borrow().clone();
    assert_eq!(log, ["begin:moda_index", "set:moda_index", "end:moda_index"], "one host gesture on the route param");
    let want = h.ctl.norm("moda_index", 0.5);
    assert!((h.ctl.get01("moda_index") - want).abs() < 1e-5, "route amount is +0.5");
}

#[test]
fn dragging_a_ring_changes_its_amount_and_the_dial_still_changes_the_knob() {
    let mut h = H::new();
    h.drag(h.chip(Source::ModB).center(), h.knob("shaper_amt").center());
    h.frames(3);
    let c = h.dial("shaper_amt", 32.0);
    // Ring 0 sits 3.9 px outside the 13 px track radius of a 32 px knob.
    let ring = c + vec2(0.0, -(13.0 + 3.9));
    h.log_clear();
    h.drag(ring, ring + vec2(0.0, -20.0));
    // egui starts a drag after ~6 px of travel: 20 px up is about +0.14 of the -1..1 range (100 px per 1.0).
    let a = h.matrix().slots[0].amount;
    assert!(a > 0.62 && a < 0.70, "20 px up on the ring raises the amount from 0.5, got {a}");
    assert!(h.ctl.log.borrow().iter().all(|l| !l.contains("shaper_amt")), "the knob itself was not touched: {:?}", h.ctl.log.borrow());
    // Shift = fine (1/10).
    h.mods = Modifiers::SHIFT;
    h.drag(ring, ring + vec2(0.0, 60.0));
    h.mods = Modifiers::NONE;
    let b = h.matrix().slots[0].amount;
    assert!((a - b - 0.054).abs() < 0.012, "Shift: 60 px down is about -0.054, got {}", a - b);
    // Double-click on the ring: zero (the slot stays).
    h.click(ring, PointerButton::Primary);
    h.click(ring, PointerButton::Primary);
    assert_eq!(h.matrix().slots[0].amount, 0.0);
    assert_eq!(h.matrix().slots.len(), 1);
    // A press on the dial itself moves the knob, not the ring.
    h.log_clear();
    h.drag(c, c + vec2(0.0, -30.0));
    assert!(h.ctl.log.borrow().iter().any(|l| l == "set:shaper_amt"), "{:?}", h.ctl.log.borrow());
    assert_eq!(h.matrix().slots[0].amount, 0.0);
}

impl H {
    fn log_clear(&self) {
        self.ctl.log.borrow_mut().clear();
    }
}

#[test]
fn ring_drag_on_a_built_in_route_is_one_host_gesture() {
    let mut h = H::new();
    h.ctl.vals.borrow_mut().insert("moda_index".into(), h.ctl.norm("moda_index", 0.4));
    h.frames(3);
    // Osc A Index (44 px dial) carries Mod A's range as ring 0; drag it up 20 px.
    let c = h.dial("osca_index", 44.0);
    let ring = c + vec2(0.0, -(19.0 + 3.9));
    let before = h.ctl.get01("moda_index");
    h.drag(ring, ring + vec2(0.0, -20.0));
    let log = h.ctl.log.borrow().clone();
    assert_eq!(log.first().map(String::as_str), Some("begin:moda_index"), "{log:?}");
    assert_eq!(log.last().map(String::as_str), Some("end:moda_index"), "{log:?}");
    assert!(log.iter().all(|l| l.ends_with("moda_index")), "{log:?}");
    assert!(h.ctl.get01("moda_index") > before + 0.05, "{before} -> {}", h.ctl.get01("moda_index"));
}

/// Right-click on a modulated knob, then a menu button by its recorded rect.
fn menu_click(h: &mut H, action: &str, k: usize) {
    let r = menu_rects(&h.ctx).into_iter().find(|m| m.0 == action && m.1 == k).unwrap_or_else(|| panic!("no menu button {action}/{k}")).2;
    h.click(r.center(), PointerButton::Primary);
}

#[test]
fn right_click_menu_toggles_polarity_inverts_bypasses_and_removes() {
    let mut h = H::new();
    let knob = h.knob("shaper_amt").center();
    h.drag(h.chip(Source::ModB).center(), knob);
    h.drag(h.chip(Source::AmpEnvA).center(), knob);
    h.frames(2);
    assert_eq!(h.matrix().slots.len(), 2);
    h.click(h.dial("shaper_amt", 32.0), PointerButton::Secondary);
    h.frames(1);
    menu_click(&mut h, "polarity", 0);
    assert!(!h.matrix().slots[0].bipolar, "Mod B slot went unipolar");
    menu_click(&mut h, "invert", 0);
    assert_eq!(h.matrix().slots[0].amount, -0.5);
    menu_click(&mut h, "bypass", 1);
    assert!(!h.matrix().slots[1].enabled);
    menu_click(&mut h, "bypass", 1);
    assert!(h.matrix().slots[1].enabled);
    // Remove takes this routing only.
    menu_click(&mut h, "remove", 0);
    let m = h.matrix();
    assert_eq!(m.slots.len(), 1);
    assert_eq!(m.slots[0].source, Source::AmpEnvA);
}

#[test]
fn right_click_remove_all_clears_the_knob() {
    let mut h = H::new();
    let knob = h.knob("shaper_amt").center();
    h.drag(h.chip(Source::ModB).center(), knob);
    h.drag(h.chip(Source::AmpEnvA).center(), knob);
    h.drag(h.chip(Source::ModB).center(), h.knob("filta_res").center());
    h.frames(2);
    assert_eq!(h.matrix().slots.len(), 3);
    h.click(h.dial("shaper_amt", 32.0), PointerButton::Secondary);
    h.frames(1);
    menu_click(&mut h, "all", 0);
    let m = h.matrix();
    assert_eq!(m.slots.len(), 1, "only the other knob's slot is left: {:?}", m.slots);
    assert_eq!(m.slots[0].target, "filta_res");
    // The menu is gone with its routings.
    h.frames(2);
    assert!(menu_rects(&h.ctx).is_empty());
}

#[test]
fn a_menu_removes_a_built_in_route_by_zeroing_its_knob() {
    let mut h = H::new();
    h.ctl.vals.borrow_mut().insert("moda_index".into(), h.ctl.norm("moda_index", 0.4));
    h.frames(3);
    h.click(h.dial("osca_index", 44.0), PointerButton::Secondary);
    h.frames(1);
    menu_click(&mut h, "remove", 0);
    assert_eq!(h.ctl.plain("moda_index", h.ctl.get01("moda_index")), 0.0);
}

#[test]
fn escape_cancels_a_drag_and_a_full_matrix_says_so() {
    let mut h = H::new();
    // Esc mid-drag.
    let (chip, knob) = (h.chip(Source::ModB).center(), h.knob("shaper_amt").center());
    h.mv(chip);
    h.btn(true, PointerButton::Primary);
    h.glide(chip + vec2(14.0, 0.0), 2);
    h.glide(knob, 6);
    h.frame(vec![Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }]);
    h.btn(false, PointerButton::Primary);
    h.frames(2);
    assert!(h.matrix().slots.is_empty(), "Esc cancels the drop");
    // 32 slots: a 33rd drop is refused with a line in the help bar.
    let ids: Vec<&str> = gts::matrix::target_ids().collect();
    h.ctl.table.matrix().replace(Matrix { slots: ids.iter().rev().take(32).map(|t| Slot::new(Source::Random, t, 0.1)).collect(), ..Default::default() });
    h.frames(2);
    let t = h.ctl.table.matrix().get().slots.iter().any(|s| s.target == "filta_res");
    assert!(!t, "test needs filta_res free");
    h.drag(h.chip(Source::ModB).center(), h.knob("filta_res").center());
    assert_eq!(h.matrix().slots.len(), 32);
    assert!(current_notice(&h.ctx).is_some_and(|m| m.contains("full")), "{:?}", current_notice(&h.ctx));
}

#[test]
fn resting_on_the_fx_tab_while_patching_switches_page_and_the_drag_goes_on() {
    let mut h = H::new();
    let chip = h.chip(Source::ModB).center();
    h.mv(chip);
    h.btn(true, PointerButton::Primary);
    h.glide(chip + vec2(14.0, 0.0), 2);
    // The FX segment of the page switch (header, x ~ 505, y ~ 29).
    h.glide(pos2(512.0, 29.0), 6);
    assert_eq!(h.bv.page, Page::Synth);
    h.frames(30); // 0.4 s dwell
    assert_eq!(h.bv.page, Page::Fx, "dwell on the FX tab switches page");
    h.frames(20);
    // A knob of the FX page takes the drop.
    let knob = h.knob("dist_drive").center();
    h.glide(knob, 8);
    h.btn(false, PointerButton::Primary);
    h.frames(2);
    let m = h.matrix();
    assert_eq!(m.slots.len(), 1, "{:?}", m.slots);
    assert_eq!((m.slots[0].source, m.slots[0].target.as_str()), (Source::ModB, "dist_drive"));
}

#[test]
fn the_matrix_page_lists_the_slots_as_they_are() {
    let mut h = H::new();
    h.ctl.table.matrix().replace(Matrix {
        slots: vec![
            Slot::new(Source::Velocity, "filta_freq", 0.35),
            Slot { source: Source::ModB, target: "osca_shift".into(), amount: -0.25, bipolar: false, enabled: false },
            Slot::new(Source::Macro2, "dly_mix", -0.5),
        ],
        macros: ["Wobble".into(), "Dirt".into(), "Space".into(), "Macro 4".into()],
    });
    h.bv.page = Page::Matrix;
    let mut over = egui::Vec2::ZERO;
    for _ in 0..30 {
        over = h.frame(vec![]);
    }
    assert_eq!(over, egui::Vec2::ZERO, "the MATRIX page overflows its body by {over:?}");
    let (rows, builtin) = matrix_rows(&h.ctx);
    let want: Vec<_> = h.matrix().slots.iter().map(|s| (s.source, s.target.clone(), s.amount, s.bipolar, s.enabled)).collect();
    assert_eq!(rows, want, "slot rows equal the matrix");
    assert_eq!(builtin, 21, "every built-in route has a row");
    // Editing the matrix shows on the next frame.
    h.ctl.table.matrix().edit(|m| {
        m.slots.remove(0);
    });
    h.frames(2);
    assert_eq!(matrix_rows(&h.ctx).0.len(), 2);
}

#[test]
fn removing_a_row_on_the_matrix_page_removes_the_slot() {
    let mut h = H::new();
    h.ctl.table.matrix().replace(Matrix { slots: vec![Slot::new(Source::Velocity, "filta_freq", 0.35), Slot::new(Source::Random, "osca_cent", 0.2)], ..Default::default() });
    h.bv.page = Page::Matrix;
    h.frames(30);
    // Row 1's remove button: second column of the left list, 20 px at the right end of the row.
    // Slots group: x 44, y 314; body from y 348; rows 24 px + 2 px; button x = 44 + 12 + 416.
    let (x, y) = (44.0 + 12.0 + 416.0 + 10.0, 314.0 + 34.0 + 12.0);
    h.click(pos2(x, y), PointerButton::Primary);
    h.frames(2);
    let m = h.matrix();
    assert_eq!(m.slots.len(), 1);
    assert_eq!(m.slots[0].source, Source::Random);
    // Row number: click bypasses.
    h.click(pos2(44.0 + 12.0 + 12.0, y), PointerButton::Primary);
    assert!(!h.matrix().slots[0].enabled);
}

#[test]
fn macro_labels_rename_in_place() {
    let mut h = H::new();
    let handle = h.chip(Source::Macro1);
    // The label sits right of the 20 px handle; double-click opens the editor.
    let label = pos2(handle.right() + 20.0, handle.center().y);
    h.click(label, PointerButton::Primary);
    h.click(label, PointerButton::Primary);
    h.frames(2);
    h.frame(vec![Event::Key { key: egui::Key::A, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::COMMAND }]);
    h.frame(vec![Event::Text("Wobble".into())]);
    h.frame(vec![Event::Key { key: egui::Key::Enter, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }]);
    h.frames(2);
    assert_eq!(h.matrix().macros[0], "Wobble");
}

/// The strip lives under the hero: no chip, handle or pill leaves its 568 px, and none touches the groups below.
#[test]
fn the_sources_strip_stays_inside_the_hero_width() {
    let h = H::new();
    let chips = chip_rects(&h.ctx);
    assert_eq!(chips.len(), 5 + 4, "five chips and four macro handles on the page (the rest sit behind MORE)");
    for (s, r) in chips {
        assert!(r.left() >= 44.0 && r.right() <= 612.0, "{s:?} spans {}..{}", r.left(), r.right());
        assert!(r.top() >= 296.0 && r.bottom() <= 344.0, "{s:?} spans y {}..{}", r.top(), r.bottom());
    }
}
