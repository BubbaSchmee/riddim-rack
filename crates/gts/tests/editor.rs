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

/// FX ids (SPEC v2 Track 4) the editor does not bind yet: the FX page wave (SPEC 4.9) builds the
/// page and deletes this list, after which every param is bound again. Until then the test checks
/// the editor binds every other id, none of these, and that each listed id is a real param.
#[rustfmt::skip]
const PENDING_GUI: [&str; 74] = [
    "dist_on", "dist_type", "dist_drive", "dist_bias", "dist_filter_mode", "dist_filter_type", "dist_filter_freq",
    "dist_filter_q", "dist_out", "dist_autogain", "dist_mix",
    "mb_on", "mb_depth", "mb_time", "mb_xover_lo", "mb_xover_hi", "mb_up_lo", "mb_up_mid", "mb_up_hi", "mb_down_lo",
    "mb_down_mid", "mb_down_hi", "mb_gain_lo", "mb_gain_mid", "mb_gain_hi", "mb_in", "mb_out", "mb_mix",
    "eq_on", "eq_low_freq", "eq_low_gain", "eq_peak_freq", "eq_peak_q", "eq_peak_gain", "eq_high_freq", "eq_high_gain",
    "eq_cut_on", "eq_cut_freq",
    "cho_on", "cho_voices", "cho_rate", "cho_sync", "cho_div", "cho_depth", "cho_delay", "cho_feedback", "cho_lpf",
    "cho_wet_lowcut", "cho_mix",
    "dim_on", "dim_size", "dim_wet_lowcut", "dim_mix",
    "dly_on", "dly_sync", "dly_time", "dly_div", "dly_feel", "dly_feedback", "dly_mode", "dly_hp", "dly_lp", "dly_mix",
    "rev_on", "rev_type", "rev_size", "rev_decay", "rev_damp", "rev_predelay", "rev_lowcut", "rev_hicut", "rev_width",
    "rev_mix",
    "lim_gain",
];

#[test]
fn editor_binds_every_param_exactly() {
    let params = Arc::new(GtsParams::default());
    let all: BTreeSet<String> = params.param_map().into_iter().map(|(id, _, _)| id).collect();
    assert_eq!(all.len(), 182); // 81 + six RR-BUS CV-in params + 21 v2 (unison, curves, sub, crossover) + 74 FX
    let pending: BTreeSet<String> = PENDING_GUI.iter().map(|s| s.to_string()).collect();
    assert_eq!(pending.len(), PENDING_GUI.len(), "duplicate id in PENDING_GUI");
    let unknown: Vec<&String> = pending.difference(&all).collect();
    assert!(unknown.is_empty(), "PENDING_GUI lists ids that are not params: {unknown:?}");
    let want: BTreeSet<String> = all.difference(&pending).cloned().collect();
    assert_eq!(want.len(), 108);

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
    // Strict: exactly the non-pending ids. A bound FX id fails too: remove it from PENDING_GUI.
    assert_eq!(*ctl.seen.borrow(), want, "editor ids differ from the param map minus PENDING_GUI");
}
