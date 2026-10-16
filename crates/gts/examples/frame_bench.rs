//! CPU frame-time budget of the REAL GTS editor draw, headless (no window, no GPU).
//! Runs `editor::draw` in a bare egui context for 600 frames at 1000x832 and times
//! (a) the UI pass (begin_pass .. end_pass) and (b) tessellation, then prints mean / p50 / p99 / max
//! and the mesh size handed to the GL painter. Table generation is excluded.
//!
//!   cargo run -p gts --release --example frame_bench [-- fx] [-- ppp=1.5]
//!
//! The GPU side (glow paint + swap) is NOT measured here; see rr-ui example `motion_gl`.

use eframe::egui::{self, vec2, Pos2, RawInput, Rect};
use gts::dsp::tables::Tables;
use gts::editor::{draw, BusView, Ctl, ParamTable, WINDOW};
use gts::params::GtsParams;
use std::sync::Arc;
use std::time::Instant;

struct DefCtl(ParamTable);
impl Ctl for DefCtl {
    fn get01(&self, id: &str) -> f32 { self.0.get01(id) }
    fn default01(&self, id: &str) -> f32 { self.0.default01(id) }
    fn readout(&self, id: &str) -> String { self.0.readout_at(id, self.0.get01(id)) }
    fn steps(&self, id: &str) -> Option<usize> { self.0.steps(id) }
    fn begin(&self, _: &str) {}
    fn set01(&self, _: &str, _: f32) {}
    fn end(&self, _: &str) {}
}

fn stats(label: &str, mut v: Vec<f64>) {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    let p = |q: f64| v[((v.len() as f64 * q) as usize).min(v.len() - 1)];
    println!("{label:>14}: mean {mean:6.3} ms  p50 {:6.3}  p99 {:6.3}  max {:6.3}", p(0.5), p(0.99), v[v.len() - 1]);
}

fn main() {
    let fx = std::env::args().any(|a| a == "fx");
    let ppp: f32 = std::env::args().find_map(|a| a.strip_prefix("ppp=").and_then(|s| s.parse().ok())).unwrap_or(1.0);
    let hover = std::env::args().any(|a| a == "hover");
    let mut tables = Tables::new();
    tables.generate();
    let ctl = DefCtl(ParamTable::new(Arc::new(GtsParams::default())));
    let mut bv = BusView {
        label: "GTS 1".into(),
        sources: vec!["GTS 1 · Mod A".into(), "VSS 1 · Step Gate".into()],
        bound: ["VSS 1 · Step Gate".into(), "GTS 2 · Mod B".into(), "GTS 3 · Mod A".into()],
        activity: [0.92, 0.35, 0.0],
        oversampling: 2,
        fx_page: fx,
        preset: Some(2),
        gr_db: 2.4,
        peak_db: -0.3,
        ..Default::default()
    };
    let ctx = egui::Context::default();
    rr_ui::theme::apply(&ctx);
    let (w, h) = (WINDOW.0 as f32, WINDOW.1 as f32);
    let (mut ui_ms, mut tess_ms, mut total_ms) = (vec![], vec![], vec![]);
    let (mut verts, mut idx, mut prims) = (0usize, 0usize, 0usize);
    let n = 600;
    for f in 0..n + 30 {
        let mut input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(w, h))),
            time: Some(f as f64 / 60.0),
            ..Default::default()
        };
        input.viewports.get_mut(&input.viewport_id).map(|v| v.native_pixels_per_point = Some(ppp));
        if hover {
            // Pointer sweeping across the panel so hover/animation paths run.
            let x = 100.0 + (f as f32 * 7.0) % 800.0;
            input.events.push(egui::Event::PointerMoved(Pos2::new(x, 420.0)));
        }
        let t0 = Instant::now();
        ctx.set_pixels_per_point(ppp);
        let out = ctx.run(input, |ctx| {
            egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
                draw(ui, &ctl, &tables, 40, &mut bv);
            });
        });
        let t1 = Instant::now();
        let clipped = ctx.tessellate(out.shapes, ppp);
        let t2 = Instant::now();
        if f >= 30 {
            ui_ms.push((t1 - t0).as_secs_f64() * 1e3);
            tess_ms.push((t2 - t1).as_secs_f64() * 1e3);
            total_ms.push((t2 - t0).as_secs_f64() * 1e3);
            if f == 30 {
                for c in &clipped {
                    if let egui::epaint::Primitive::Mesh(m) = &c.primitive {
                        verts += m.vertices.len();
                        idx += m.indices.len();
                    }
                }
                prims = clipped.len();
            }
        }
    }
    println!("GTS editor {}x{} pts, ppp {ppp}, page {}, hover {hover}, {n} frames (after 30 warm-up)", w, h, if fx { "FX" } else { "SYNTH" });
    stats("ui pass", ui_ms);
    stats("tessellate", tess_ms);
    stats("ui+tess", total_ms);
    println!("mesh per frame: {prims} clipped primitives, {verts} vertices, {idx} indices");
}
