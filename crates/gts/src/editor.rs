//! egui editor per SPEC.md "GUI": fixed 1000x700 dark window, four rows of labelled groups.
//! Each group lays its params in a grid of at most 5 columns (here 2-5). Cell width =
//! max(label, widget): label ~7 px per char at 12.5 px, sliders 80, combos 110, checkboxes 20.
//! Item spacing 8 px; a group adds 16 px of frame. `Group::width` and `row_width` implement that
//! arithmetic; the unit test below (plus a debug assertion in `create`) keeps every row < 960 px.
//!
//! Row widths (widest grid line per group + 16 frame, groups 8 apart):
//!   Row 1  Osc A | Osc B, 5 cols: 5*80 + 4*8 + 16 = 448 each            -> 448*2 + 8 = 904
//!   Row 2  Mod A / Mod B, 3 cols: widest line combo+slider+check or
//!          combo+check(One-shot, 56)+combo = 110+56+110+16+16 = 308 each;
//!          Shaper, 2 cols: 20+110+8+16 = 154 vs title -> ~154       -> 308*2+154+16 = 786
//!   Row 3  Filter A / B, 3 cols: 110+80+80+16+16 = 302 each; Filter Env, 3 cols:
//!          3*80+16+16 = 272                                         -> 302*2+272+16 = 892
//!   Row 4  Voice 3 cols: 110+80+20+16+16 = 242; Wheel 2 cols: 168+16 = 184; Velocity 3 cols:
//!          3*80+16+16 = 272; Output 2 cols: 184                     -> 882 + 24 = 906

use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, widgets::ParamSlider, EguiState};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use crate::params::*;

const SPACING: f32 = 8.0;
const FRAME: f32 = 16.0;
const MAX_ROW: f32 = 960.0;
const CHAR_W: f32 = 7.0;

struct Cell<'a> {
    label: &'static str,
    w: f32,
    draw: Box<dyn Fn(&mut egui::Ui, &ParamSetter) + 'a>,
}

impl Cell<'_> {
    fn width(&self) -> f32 {
        (self.label.chars().count() as f32 * CHAR_W).max(self.w)
    }
}

struct Group<'a> {
    title: &'static str,
    cols: usize,
    cells: Vec<Cell<'a>>,
}

impl Group<'_> {
    fn width(&self) -> f32 {
        let grid = self
            .cells
            .chunks(self.cols)
            .map(|r| r.iter().map(Cell::width).sum::<f32>() + SPACING * (r.len() - 1) as f32)
            .fold(0.0, f32::max);
        let title = self.title.chars().count() as f32 * CHAR_W;
        grid.max(title) + FRAME
    }
    #[cfg(test)]
    fn height(&self) -> f32 {
        // title 18 + per grid line (label 16 + widget 22 + gaps 4) + frame
        18.0 + 42.0 * self.cells.chunks(self.cols).count() as f32 + FRAME
    }
}

fn row_width(r: &[Group]) -> f32 {
    r.iter().map(Group::width).sum::<f32>() + SPACING * (r.len() - 1) as f32
}

fn slider<'a, P: Param + 'a>(label: &'static str, p: &'a P) -> Cell<'a> {
    Cell {
        label,
        w: 80.0,
        draw: Box::new(move |ui, s| {
            ui.add(ParamSlider::for_param(p, s).with_width(80.0));
        }),
    }
}

fn check<'a>(label: &'static str, p: &'a BoolParam) -> Cell<'a> {
    Cell {
        label,
        w: 20.0,
        draw: Box::new(move |ui, s| {
            let mut v = p.value();
            if ui.checkbox(&mut v, "").changed() {
                s.begin_set_parameter(p);
                s.set_parameter(p, v);
                s.end_set_parameter(p);
            }
        }),
    }
}

fn combo<'a, T: Enum + PartialEq + Copy + 'static>(label: &'static str, p: &'a EnumParam<T>) -> Cell<'a> {
    Cell {
        label,
        w: 110.0,
        draw: Box::new(move |ui, s| {
            let cur = p.value();
            let names = T::variants();
            egui::ComboBox::from_id_salt((label, p as *const _ as usize))
                .width(110.0)
                .selected_text(names[cur.to_index()])
                .show_ui(ui, |ui| {
                    for (i, name) in names.iter().enumerate() {
                        let v = T::from_index(i);
                        if ui.selectable_label(cur == v, *name).clicked() {
                            s.begin_set_parameter(p);
                            s.set_parameter(p, v);
                            s.end_set_parameter(p);
                        }
                    }
                });
        }),
    }
}

fn osc<'a>(title: &'static str, o: &'a OscParams) -> Group<'a> {
    Group {
        title,
        cols: 5,
        cells: vec![
            check("On", &o.on),
            combo("Table", &o.table),
            slider("Oct", &o.oct),
            slider("Semi", &o.semi),
            slider("Cent", &o.cent),
            slider("Index", &o.index),
            slider("Motion", &o.motion),
            slider("Shift", &o.shift),
            slider("Vol", &o.vol),
            slider("Attack", &o.att),
            slider("Decay", &o.dec),
            slider("Sustain", &o.sus),
            slider("Release", &o.rel),
        ],
    }
}

fn modg<'a>(title: &'static str, m: &'a ModParams, mut amts: Vec<Cell<'a>>) -> Group<'a> {
    let mut cells = vec![
        combo("Curve", &m.curve),
        slider("Rate", &m.rate),
        check("Sync", &m.sync),
        combo("Div", &m.div),
        check("One-shot", &m.oneshot),
        combo("Target", &m.target),
    ];
    cells.append(&mut amts);
    Group { title, cols: 3, cells }
}

fn filt<'a>(title: &'static str, f: &'a FilterParams) -> Group<'a> {
    Group {
        title,
        cols: 3,
        cells: vec![
            combo("Type", &f.ty),
            slider("Freq", &f.freq),
            slider("Res", &f.res),
            slider("Kbd", &f.kbd),
            check("Env", &f.env),
        ],
    }
}

fn layout(p: &GtsParams) -> Vec<Vec<Group<'_>>> {
    vec![
        vec![osc("Osc A", &p.osca), osc("Osc B", &p.oscb)],
        vec![
            modg(
                "Mod A",
                &p.moda,
                vec![
                    slider("Pitch", &p.moda_amt.pitch),
                    slider("Index", &p.moda_amt.index),
                    slider("Shift", &p.moda_amt.shift),
                ],
            ),
            modg(
                "Mod B",
                &p.modb,
                vec![
                    slider("Motion", &p.modb_amt.motion),
                    slider("Vol", &p.modb_amt.vol),
                    slider("Cutoff", &p.modb_amt.cutoff),
                    slider("Mod A", &p.modb_amt.moda),
                ],
            ),
            Group {
                title: "Shaper",
                cols: 2,
                cells: vec![
                    check("On", &p.shaper.on),
                    combo("Mode", &p.shaper.mode),
                    slider("Amount", &p.shaper.amt),
                ],
            },
        ],
        vec![
            filt("Filter A", &p.filta),
            filt("Filter B", &p.filtb),
            Group {
                title: "Filter Env",
                cols: 3,
                cells: vec![
                    slider("Attack", &p.fenv.att),
                    slider("Decay", &p.fenv.dec),
                    slider("Sustain", &p.fenv.sus),
                    slider("Release", &p.fenv.rel),
                    slider("Amount", &p.fenv.amt),
                    check("Invert", &p.fenv.inv),
                ],
            },
        ],
        vec![
            Group {
                title: "Voice",
                cols: 3,
                cells: vec![
                    combo("Routing", &p.routing),
                    slider("Poly", &p.poly),
                    check("Legato", &p.legato),
                    slider("Porta", &p.porta),
                    slider("Bend", &p.bend_range),
                ],
            },
            Group {
                title: "Wheel",
                cols: 2,
                cells: vec![
                    slider("Index", &p.wheel.index),
                    slider("Shift", &p.wheel.shift),
                    slider("Cutoff", &p.wheel.cutoff),
                    slider("Mod", &p.wheel.mod_amt),
                ],
            },
            Group {
                title: "Velocity",
                cols: 3,
                cells: vec![
                    slider("Vol A", &p.vel.vol_a),
                    slider("Vol B", &p.vel.vol_b),
                    slider("Filt Env", &p.vel.fenv),
                    slider("Attack", &p.vel.att),
                    slider("Shift", &p.vel.shift),
                    slider("Mod", &p.vel.mod_amt),
                ],
            },
            Group {
                title: "Output",
                cols: 2,
                cells: vec![slider("Volume", &p.volume), slider("Spread", &p.spread)],
            },
        ],
    ]
}

pub fn create(params: Arc<GtsParams>, current_index: Arc<AtomicU32>) -> Option<Box<dyn Editor>> {
    for r in &layout(&params) {
        debug_assert!(row_width(r) < MAX_ROW, "editor row {} px >= {MAX_ROW}", row_width(r));
    }
    create_egui_editor(
        EguiState::from_size(1000, 700),
        (),
        |ctx, _| ctx.set_visuals(egui::Visuals::dark()),
        move |ctx, setter, _| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(format!("GTS — Graintable Synth  v{}", env!("CARGO_PKG_VERSION")));
                    ui.label(format!("Osc A grain: {}", current_index.load(Ordering::Relaxed)));
                });
                for row in layout(&params) {
                    ui.horizontal_top(|ui| {
                        for g in &row {
                            ui.group(|ui| {
                                ui.vertical(|ui| {
                                    ui.strong(g.title);
                                    for line in g.cells.chunks(g.cols) {
                                        ui.horizontal(|ui| {
                                            for c in line {
                                                ui.vertical(|ui| {
                                                    ui.label(c.label);
                                                    (c.draw)(ui, setter);
                                                });
                                            }
                                        });
                                    }
                                });
                            });
                        }
                    });
                }
            });
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_fit() {
        let p = GtsParams::default();
        let rows = layout(&p);
        assert_eq!(rows.len(), 4);
        let mut h = 40.0;
        for r in &rows {
            let w = row_width(r);
            assert!(w < MAX_ROW, "row width {w}");
            assert!(r.iter().all(|g| g.cols <= 5));
            h += r.iter().map(Group::height).fold(0.0, f32::max) + SPACING;
        }
        assert!(h < 700.0, "height {h}");
    }
}
