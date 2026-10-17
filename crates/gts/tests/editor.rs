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
    assert_eq!(all.len(), 220); // 81 + six RR-BUS CV-in params + 21 v2 (unison, curves, sub, crossover) + 74 FX + 4 macros + 34 Track 7 modulation
    // SPEC v2 Track 7 modulation params the editor does not draw yet: UI wave binds it.
    let pending = |id: &str| {
        ["modc_", "modd_", "menv_"].iter().any(|p| id.starts_with(p))
            || ["osca_pitch", "oscb_pitch", "pitch_all"].contains(&id)
            || ["moda_", "modb_"].iter().any(|p| id.strip_prefix(p).is_some_and(|t| ["phase", "fade", "smooth", "trig"].contains(&t)))
    };
    assert_eq!(all.iter().filter(|id| pending(id)).count(), 34);
    let all: BTreeSet<String> = all.into_iter().filter(|id| !pending(id)).collect();

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
    // (an unknown id would already have panicked inside ParamTable)
    assert_eq!(*ctl.seen.borrow(), all, "editor ids differ from the param map");
}

/// MOTION.md 11 #8: the hero stays a large display although the strip took two rows under it.
#[test]
fn hero_is_at_least_35_percent_of_the_width_and_25_percent_of_the_height() {
    let (_, _, w, h) = gts::editor::HERO;
    assert!(w >= 0.35 * WINDOW.0 as f32, "{w}");
    assert!(h >= 0.25 * WINDOW.1 as f32, "{h}");
}
