//! Uncapped frame cost of the REAL GTS editor on this machine's GPU (eframe + glow, vsync OFF,
//! continuous repaint). Brackets the whole egui frame with a GL timer query (GPU time of all
//! egui draw calls) and reports the frame-to-frame interval (CPU build + tessellation + driver +
//! swap) and the UI build time. The editor's own hero is a GL PaintCallback (its GPU time is inside the
//! total); `gl` adds the old prototype wavetable overlay on top. The real engine plays the demo pattern
//! (735 samples per frame) through the plugin's publisher so every live display moves.
//!
//!   cargo run -p gts --release --example frame_gpu [-- gl] [-- fx] [-- ppp=1.5] [-- cpu]

use eframe::egui::{self, pos2, vec2, Context, Rect, ViewportCommand};
use egui_glow::glow::{self, HasContext as _};
use gts::dsp::tables::Tables;
use gts::editor::{draw, BusView, LiveView, WINDOW};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[path = "../../rr-ui/examples/support/wavegl.rs"]
mod wavegl;
#[path = "support/demo.rs"]
mod demo;

#[derive(Default)]
struct Q {
    q: Option<glow::Query>,
    pending: bool,
    started: bool,
    ms: Vec<f64>,
    ok: bool,
}

struct App {
    ctl: demo::MapCtl,
    tables: Tables,
    bv: BusView,
    feed: demo::Feed,
    live: LiveView,
    gl: Option<wavegl::WaveGl>,
    q: Arc<Mutex<Q>>,
    ppp: f32,
    frame: u32,
    last: Option<Instant>,
    interval: Vec<f64>,
    upd: Vec<f64>,
}

fn stats(label: &str, v: &[f64]) {
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    if s.is_empty() {
        println!("{label}: n/a");
        return;
    }
    println!("{label:<34} mean {:6.3}  p50 {:6.3}  p99 {:6.3}  max {:6.3}", s.iter().sum::<f64>() / s.len() as f64, s[s.len() / 2], s[(s.len() as f64 * 0.99) as usize], s[s.len() - 1]);
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _f: &mut eframe::Frame) {
        // Engine time is not UI time: render the demo audio before the clock starts.
        self.feed.render(735);
        self.live.peaks = self.feed.take_peaks();
        let t0 = Instant::now();
        if self.frame == 0 {
            let native = ctx.native_pixels_per_point().unwrap_or(1.0);
            ctx.set_pixels_per_point(self.ppp);
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(vec2(WINDOW.0 as f32, WINDOW.1 as f32) * self.ppp / native));
        }
        if let Some(l) = self.last {
            if self.frame > 60 {
                self.interval.push((t0 - l).as_secs_f64() * 1e3);
            }
        }
        self.last = Some(t0);
        let q = self.q.clone();
        let begin = egui_glow::CallbackFn::new(move |_, p| {
            let gl = p.gl();
            let mut s = q.lock().unwrap();
            unsafe {
                if s.q.is_none() {
                    s.q = gl.create_query().ok();
                    s.ok = s.q.is_some();
                }
                if let Some(qq) = s.q {
                    if s.pending && gl.get_query_parameter_u32(qq, glow::QUERY_RESULT_AVAILABLE) != 0 {
                        let ns = gl.get_query_parameter_u32(qq, glow::QUERY_RESULT);
                        s.ms.push(ns as f64 / 1e6);
                        s.pending = false;
                    }
                    s.started = !s.pending;
                    if s.started {
                        gl.begin_query(glow::TIME_ELAPSED, qq);
                    }
                }
            }
        });
        let q = self.q.clone();
        let end = egui_glow::CallbackFn::new(move |_, p| {
            let mut s = q.lock().unwrap();
            if s.started {
                unsafe { p.gl().end_query(glow::TIME_ELAPSED) };
                s.pending = true;
                s.started = false;
            }
        });
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
            let tiny = Rect::from_min_size(pos2(0.0, 0.0), vec2(1.0, 1.0));
            ui.painter().add(egui::PaintCallback { rect: tiny, callback: Arc::new(begin) });
            draw(ui, &self.ctl, &self.tables, &mut self.live, &mut self.bv);
            if let Some(gl) = &self.gl {
                let t = ui.input(|i| i.time);
                let r = Rect::from_min_size(pos2(34.0, 66.0), vec2(300.0, 150.0));
                {
                    let mut s = gl.0.lock().unwrap();
                    s.view.yaw = (t * 0.5).sin() as f32 * 0.3;
                    s.view.focus = (t * 0.25).fract() as f32;
                }
                gl.show(&ui.painter().with_clip_rect(r), r);
            }
            ui.painter().add(egui::PaintCallback { rect: tiny, callback: Arc::new(end) });
        });
        if self.frame > 60 {
            self.upd.push(t0.elapsed().as_secs_f64() * 1e3);
        }
        self.frame += 1;
        if self.frame >= 660 {
            println!("GTS editor, vsync OFF, ppp {}, {} frames, page {}, hero {} ({:?}), gl overlay {}", self.ppp, self.frame - 61, format!("{:?}", self.bv.page).to_uppercase(), if self.live.hero.force_cpu { "CPU" } else { "GL" }, self.live.hero.gl_ok(), self.gl.is_some());
            stats("UI build (update) ms", &self.upd);
            stats("frame interval ms (uncapped)", &self.interval);
            let g = self.q.lock().unwrap();
            stats(&format!("GPU time of all egui draws ms (query {})", g.ok), &g.ms[g.ms.len().min(30)..]);
            drop(g);
            ctx.send_viewport_cmd(ViewportCommand::Close);
        } else {
            ctx.request_repaint();
        }
    }
}

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    let ppp: f32 = args.iter().find_map(|a| a.strip_prefix("ppp=").and_then(|s| s.parse().ok())).unwrap_or(1.0);
    let (gl, fx, cpu) = (args.iter().any(|a| a == "gl"), args.iter().any(|a| a == "fx"), args.iter().any(|a| a == "cpu"));
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([WINDOW.0 as f32, WINDOW.1 as f32]).with_resizable(false),
        vsync: false,
        depth_buffer: 24,
        stencil_buffer: 8,
        ..Default::default()
    };
    eframe::run_native("frame_gpu", opts, Box::new(move |cc| {
        rr_ui::theme::apply(&cc.egui_ctx);
        let mut tables = Tables::new();
        tables.generate();
        let (key, preset) = demo::factory("Wobble Shot");
        let bv = BusView { label: "GTS 1".into(), oversampling: 2, page: if fx { gts::editor::Page::Fx } else { gts::editor::Page::Synth }, peak_db: -6.0, ..Default::default() };
        let ctl = demo::MapCtl::with_preset(Some(preset));
        let mut feed = demo::Feed::new(preset.patch);
        feed.attach(ctl.table.params().clone());
        let mut live = LiveView::new(feed.viz.clone(), Some(feed.ring.clone()));
        live.hero.force_cpu = cpu;
        live.browser.restore(&key);
        Ok(Box::new(App {
            ctl,
            tables,
            bv,
            feed,
            live,
            gl: gl.then(|| wavegl::WaveGl::new(wavegl::demo_waves())),
            q: Arc::new(Mutex::new(Q::default())),
            ppp,
            frame: 0,
            last: None,
            interval: vec![],
            upd: vec![],
        }))
    }))
}
