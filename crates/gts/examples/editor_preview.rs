//! Renders the REAL GTS editor in an eframe window (glow backend, so the hero's GL paint
//! callback runs as in the plugin), saves a screenshot to `docs/design/preview-gts-editor.png`
//! (SYNTH page; `fx` renders the FX page into `preview-gts-editor-fx.png`) and exits. `live`
//! keeps it open. The real engine plays the riddim demo pattern on Growl Hound through the
//! plugin's own publisher, so the scope, meter, hero and playheads show live data.
//!
//!   cargo run -p gts --example editor_preview [fx] [live] [dirty] [cpu] [values]
//!
//!   cpu     paint the hero with its CPU fallback instead of GL (writes preview-gts-editor-cpu.png)
//!   values  "Always show values" on
//!   2d      hero in 2D (writes preview-gts-editor-2d.png)

use eframe::egui::{self, vec2, Context, ViewportCommand};
use gts::dsp::tables::Tables;
use gts::editor::{draw, BusView, LiveView, WINDOW};

#[path = "support/demo.rs"]
mod demo;

struct App {
    ctl: demo::MapCtl,
    tables: Tables,
    feed: demo::Feed,
    live: LiveView,
    keep_open: bool,
    frame: u32,
    asked: bool,
    bus: BusView,
    name: &'static str,
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
        // Live mode follows real time; the screenshot run advances a fixed 1/60 s per frame.
        let dt = if self.keep_open { ctx.input(|i| i.stable_dt).clamp(0.0, 0.05) } else { 1.0 / 60.0 };
        self.feed.render((dt * self.feed.sr()) as usize);
        self.live.peaks = self.feed.take_peaks();
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
            draw(ui, &self.ctl, &self.tables, &mut self.live, &mut self.bus);
        });
        ctx.request_repaint();
        let size = ctx.screen_rect().size();
        let ready = (size.x - w).abs() < 1.0 && (size.y - h).abs() < 1.0 && self.frame >= 20;
        if self.keep_open {
            return;
        }
        if (ready || self.frame > 140) && !self.asked {
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
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/design").join(self.name);
            let file = std::io::BufWriter::new(std::fs::File::create(&path).expect("create png"));
            let mut enc = png::Encoder::new(file, img.size[0] as u32, img.size[1] as u32);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header().unwrap().write_image_data(img.as_raw()).unwrap();
            println!("wrote {} ({}x{}); hero GL: {:?} {}", path.display(), img.size[0], img.size[1], self.live.hero.gl_ok(), self.live.hero.gl_info());
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }
}

fn main() -> eframe::Result {
    let has = |s: &str| std::env::args().any(|a| a == s);
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([WINDOW.0 as f32, WINDOW.1 as f32]).with_resizable(false).with_title("GTS editor preview"),
        ..Default::default()
    };
    eframe::run_native("GTS editor preview", opts, Box::new(move |cc| {
        rr_ui::theme::apply(&cc.egui_ctx);
        let mut tables = Tables::new();
        tables.generate();
        let (pi, preset) = demo::factory("Growl Hound");
        let ctl = demo::MapCtl::with_preset(Some(preset));
        // Demo patching: rows 1 and 2 follow live sources, row 3 a lost one.
        for (k, v) in [("cv1_target", 5.0 / 8.0), ("cv1_amount", 0.78), ("cv2_target", 1.0 / 8.0), ("cv2_amount", 0.35), ("cv3_target", 7.0 / 8.0), ("cv3_amount", 0.5)] {
            ctl.vals.borrow_mut().insert(k.to_string(), v);
        }
        let mut patch = preset.patch;
        if has("fx") {
            demo::fx_on(&ctl, &mut patch); // the FX shot shows the modules working
        }
        let mut feed = demo::Feed::new(patch);
        // Pre-roll into the pattern: a note was just struck when the shot is taken.
        feed.render((0.92 * feed.sr()) as usize);
        let mut live = LiveView::new(feed.viz.clone(), Some(feed.ring.clone()));
        live.hero.force_cpu = has("cpu");
        live.hero.mode3d = !has("2d");
        let s = |v: &str| v.to_string();
        let bus = BusView {
            label: s("GTS 1"),
            sources: vec![s("GTS 1 · Mod A"), s("VSS 1 · Step Gate"), s("GTS 2 · Mod B")],
            bound: [s("VSS 1 · Step Gate"), s("GTS 2 · Mod B"), s("GTS 3 · Mod A")],
            activity: [0.92, 0.35, 0.0],
            oversampling: 2,
            fx_page: has("fx"),
            preset: Some(pi),
            preset_dirty: has("dirty"),
            gr_db: 2.4,
            peak_db: -0.3,
            show_values: has("values"),
            ..Default::default()
        };
        let name = if has("fx") { "preview-gts-editor-fx.png" } else if has("cpu") { "preview-gts-editor-cpu.png" } else if has("2d") { "preview-gts-editor-2d.png" } else { "preview-gts-editor.png" };
        Ok(Box::new(App { ctl, tables, feed, live, keep_open: has("live"), frame: 0, asked: false, bus, name }))
    }))
}
