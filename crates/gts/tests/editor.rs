//! The editor must bind every parameter, and only real ones.
use gts::dsp::tables::Tables;
use gts::editor::{draw, BusView, Ctl, LiveView, Page, ParamTable, WINDOW};
use gts::matrix::MatrixShared;
use gts::params::GtsParams;
use nih_plug::prelude::Params;
use nih_plug_egui::egui;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::sync::Arc;

/// Reads through to the default params and records every id the editor asks about.
struct Recorder {
    table: ParamTable,
    seen: RefCell<BTreeSet<String>>,
}

impl Recorder {
    fn t(&self, id: &str) {
        self.seen.borrow_mut().insert(id.to_string());
    }
}

impl Ctl for Recorder {
    fn get01(&self, id: &str) -> f32 {
        self.t(id);
        self.table.get01(id)
    }
    fn default01(&self, id: &str) -> f32 {
        self.t(id);
        self.table.default01(id)
    }
    fn readout(&self, id: &str) -> String {
        self.t(id);
        self.table.readout_at(id, self.table.get01(id))
    }
    fn steps(&self, id: &str) -> Option<usize> {
        self.t(id);
        self.table.steps(id)
    }
    fn begin(&self, id: &str) {
        self.t(id);
    }
    fn set01(&self, id: &str, _: f32) {
        self.t(id);
    }
    fn end(&self, id: &str) {
        self.t(id);
    }
    fn matrix(&self) -> Option<&MatrixShared> {
        Some(self.table.matrix())
    }
}

#[test]
fn editor_binds_every_param_exactly() {
    let params = Arc::new(GtsParams::default());
    let all: BTreeSet<String> = params.param_map().into_iter().map(|(id, _, _)| id).collect();
    assert_eq!(all.len(), 249); // 81 + six RR-BUS CV-in params + 21 v2 (unison, curves, sub, crossover) + 74 FX + 4 macros + 34 Track 7 modulation + 10 Track 7 sound + 19 phaser / flanger

    let ctl = Recorder { table: ParamTable::new(params), seen: RefCell::new(BTreeSet::new()) };
    let mut tables = Tables::new();
    tables.generate();

    // Headless egui pass: fonts must be installed first, as the plugin does in its build closure.
    let ctx = egui::Context::default();
    rr_ui::theme::apply(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(WINDOW.0 as f32, WINDOW.1 as f32))),
        ..Default::default()
    };
    // All three pages; 30 passes each (the first loads the fonts, the page transition runs 260 ms). The PATCH popover is open
    // (its CV-in params live there).
    let mut bv = BusView { sources: vec!["GTS 1 · Mod A".into()], oversampling: 2, patch_open: true, ..Default::default() };
    let mut live = LiveView::idle();
    for page in [Page::Synth, Page::Fx, Page::Matrix] {
        bv.page = page;
        let mut overflow = egui::Vec2::ZERO;
        for _ in 0..30 {
            let _ = ctx.run(input.clone(), |ctx| {
                egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| overflow = draw(ui, &ctl, &tables, &mut live, &mut bv));
            });
        }
        // Layout: every group's content fits its fixed rect, and the FX rows fit the body area.
        assert_eq!(overflow, egui::Vec2::ZERO, "page {page:?} overflows the {WINDOW:?} window by {overflow:?}");
    }
    // The MODULATOR draws one tab at a time and SUB | NOISE one of its two: bind the rest too.
    bv.page = Page::Synth;
    let mut overflow = egui::Vec2::ZERO;
    for tab in 0..5 {
        gts::editor::modulator::select_tab(&ctx, tab);
        gts::editor::sub::select_tab(&ctx, tab % 2);
        for _ in 0..30 {
            let _ = ctx.run(input.clone(), |ctx| {
                egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| overflow = draw(ui, &ctl, &tables, &mut live, &mut bv));
            });
        }
        assert_eq!(overflow, egui::Vec2::ZERO, "tab {tab} overflows");
    }
    // (an unknown id would already have panicked inside ParamTable)
    // Strict: all 249 params, no pending list (Track 7 UI wave).
    assert_eq!(*ctl.seen.borrow(), all, "editor ids differ from the param map");
}

/// MOTION-addendum-track7.md 10.6: the MODULATOR is 336 x 260 at (620, 80) with a 312 x 130 glass, the oscillator
/// rows keep 44 px cells at least, SUB | NOISE is 88 x 222, and the FX page fits its grid.
#[test]
fn layout_matches_the_track7_addendum() {
    use gts::editor::{chain, modulator, patching, sub};
    assert_eq!(modulator::RECT, (620.0, 80.0, 336.0, 260.0));
    let g = modulator::glass();
    assert_eq!((g.min.x, g.min.y, g.width(), g.height()), (632.0, 148.0, 312.0, 130.0));
    assert_eq!(sub::RECT, (868.0, 348.0, 88.0, 222.0));
    assert_eq!(chain::BAR, (44.0, 80.0, 912.0, 20.0));
    // The oscillator rows: every knob cell at least 44 px wide.
    let params = Arc::new(GtsParams::default());
    let ctl = Recorder { table: ParamTable::new(params), seen: RefCell::new(BTreeSet::new()) };
    let mut tables = Tables::new();
    tables.generate();
    let ctx = egui::Context::default();
    rr_ui::theme::apply(&ctx);
    patching::enable_test_hooks(&ctx);
    let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(WINDOW.0 as f32, WINDOW.1 as f32))), ..Default::default() };
    let (mut bv, mut live) = (BusView { oversampling: 2, ..Default::default() }, LiveView::idle());
    for _ in 0..30 {
        let _ = ctx.run(input.clone(), |ctx| {
            egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
                draw(ui, &ctl, &tables, &mut live, &mut bv);
            });
        });
    }
    let rects = patching::knob_rects(&ctx);
    for osc in ["osca", "oscb"] {
        for id in ["oct", "semi", "cent", "pitch", "index", "motion", "shift", "vol", "att", "dec", "sus", "rel", "curve", "unison", "detune", "det_curve", "blend", "width", "rand_phase", "warp_amt"] {
            let r = rects.iter().find(|k| k.0 == format!("{osc}_{id}")).unwrap_or_else(|| panic!("{osc}_{id} was not drawn")).1;
            assert!(r.width() >= 44.0, "{osc}_{id} is {} px wide", r.width());
        }
    }
    // Row 1 is eight cells of 47.5 px: Pitch sits between Cent and Index.
    let x = |id: &str| rects.iter().find(|k| k.0 == id).unwrap().1.left();
    assert!(x("osca_cent") < x("osca_pitch") && x("osca_pitch") < x("osca_index"));
    // The seven Mod A / Mod B route knobs left the SYNTH page (they are MATRIX rows now).
    for id in ["moda_pitch", "moda_index", "moda_shift", "modb_motion", "modb_vol", "modb_cutoff", "modb_moda"] {
        assert!(!rects.iter().any(|k| k.0 == id), "{id} is still on the SYNTH page");
    }
}

/// MOTION.md 11 #8: the hero stays a large display although the strip took two rows under it.
#[test]
fn hero_is_at_least_35_percent_of_the_width_and_25_percent_of_the_height() {
    let (_, _, w, h) = gts::editor::HERO;
    assert!(w >= 0.35 * WINDOW.0 as f32, "{w}");
    assert!(h >= 0.25 * WINDOW.1 as f32, "{h}");
}
