//! Renders the REAL GTS editor (default params) in an eframe window, saves a screenshot to
//! `docs/design/preview-gts-editor.png` (SYNTH page; `fx` renders the FX page into `preview-gts-editor-fx.png`) on the first stable frame and exits. `live` keeps it open.
//!
//!   cargo run -p gts --example editor_preview [fx] [live]

use eframe::egui::{self, vec2, Context, ViewportCommand};
use gts::dsp::tables::Tables;
use gts::editor::{draw, BusView, Ctl, ParamTable, WINDOW};
use gts::params::GtsParams;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

/// Default params plus an override map so dragging works in `live` mode.
struct PreviewCtl {
    table: ParamTable,
    vals: RefCell<HashMap<String, f32>>,
}

impl Ctl for PreviewCtl {
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
    fn begin(&self, _: &str) {}
    fn set01(&self, id: &str, v: f32) {
        let v = match self.table.steps(id) {
            Some(n) if n > 0 => (v * n as f32).round() / n as f32,
            _ => v,
        };
        self.vals.borrow_mut().insert(id.to_string(), v);
    }
    fn end(&self, _: &str) {}
}

struct App {
    ctl: PreviewCtl,
    tables: Tables,
    live: bool,
    frame: u32,
    asked: bool,
    bus: BusView,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let (w, h) = (WINDOW.0 as f32, WINDOW.1 as f32);
        if self.frame == 0 {
            // Render at 1 px per point regardless of OS scaling so the PNG is 1:1.
            let native = ctx.native_pixels_per_point().unwrap_or(1.0);
            ctx.set_pixels_per_point(1.0);
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(vec2(w, h) / native));
        }
        self.frame += 1;
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
            draw(ui, &self.ctl, &self.tables, 64, &mut self.bus);
        });

        let size = ctx.screen_rect().size();
        let ready = (size.x - w).abs() < 1.0 && (size.y - h).abs() < 1.0 && self.frame >= 3;
        if self.live {
            return;
        }
        if (ready || self.frame > 120) && !self.asked {
            self.asked = true;
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(Default::default()));
        }
        let shot = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = shot {
            let name = if self.bus.fx_page { "preview-gts-editor-fx.png" } else { "preview-gts-editor.png" };
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/design").join(name);
            let file = std::io::BufWriter::new(std::fs::File::create(&path).expect("create png"));
            let mut enc = png::Encoder::new(file, img.size[0] as u32, img.size[1] as u32);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header().unwrap().write_image_data(img.as_raw()).unwrap();
            println!("wrote {} ({}x{})", path.display(), img.size[0], img.size[1]);
            ctx.send_viewport_cmd(ViewportCommand::Close);
        } else {
            ctx.request_repaint();
        }
    }
}

fn main() -> eframe::Result {
    let live = std::env::args().any(|a| a == "live");
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([WINDOW.0 as f32, WINDOW.1 as f32]).with_resizable(false).with_title("GTS editor preview"),
        ..Default::default()
    };
    eframe::run_native("GTS editor preview", opts, Box::new(move |cc| {
        gts_theme(&cc.egui_ctx);
        let mut tables = Tables::new();
        tables.generate();
        // Demo patching: row 1 and 2 follow live sources, row 3 a lost one.
        let vals = [("cv1_target", 5.0 / 8.0), ("cv1_amount", 0.78), ("cv2_target", 1.0 / 8.0), ("cv2_amount", 0.35), ("cv3_target", 7.0 / 8.0), ("cv3_amount", 0.5)];
        let vals = vals.iter().map(|&(k, v)| (k.to_string(), v)).collect();
        let ctl = PreviewCtl { table: ParamTable::new(Arc::new(GtsParams::default())), vals: RefCell::new(vals) };
        let s = |v: &str| v.to_string();
        let bus = BusView {
            label: s("GTS 1"),
            sources: vec![s("GTS 1 · Mod A"), s("VSS 1 · Step Gate"), s("GTS 2 · Mod B")],
            bound: [s("VSS 1 · Step Gate"), s("GTS 2 · Mod B"), s("GTS 3 · Mod A")],
            activity: [0.92, 0.35, 0.0],
            oversampling: 2,
            fx_page: std::env::args().any(|a| a == "fx"),
        };
        Ok(Box::new(App { ctl, tables, live, frame: 0, asked: false, bus }))
    }))
}

fn gts_theme(ctx: &Context) {
    rr_ui::theme::apply(ctx);
}
