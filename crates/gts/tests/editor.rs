//! The editor must bind every parameter, and only real ones.
use gts::dsp::tables::Tables;
use gts::editor::{draw, BusView, Ctl, ParamTable, WINDOW};
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
}

#[test]
fn editor_binds_every_param_exactly() {
    let params = Arc::new(GtsParams::default());
    let want: BTreeSet<String> = params.param_map().into_iter().map(|(id, _, _)| id).collect();
    assert_eq!(want.len(), 108); // 81 + six RR-BUS CV-in params + 21 v2 (unison, curves, sub, crossover)

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
    // Two passes: the first one only loads the fonts.
    let mut bv = BusView { sources: vec!["GTS 1 · Mod A".into()], oversampling: 2, ..Default::default() };
    let mut overflow = egui::Vec2::ZERO;
    for _ in 0..2 {
        let _ = ctx.run(input.clone(), |ctx| {
            egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| overflow = draw(ui, &ctl, &tables, 64, &mut bv));
        });
    }
    // Layout: every row fits the window's body area (no clipped groups).
    assert_eq!(overflow, egui::Vec2::ZERO, "body content overflows the {WINDOW:?} window by {overflow:?}");
    // (an unknown id would already have panicked inside ParamTable)
    assert_eq!(*ctl.seen.borrow(), want, "editor ids differ from the param map");
}
