//! Motion / GPU feasibility prototype (see docs/design/motion-feasibility.md).
//!
//!   cargo run -p rr-ui --release --example motion_gl -- bench      uncapped (vsync off) frame time per scene, prints a table
//!   cargo run -p rr-ui --release --example motion_gl -- headless   CPU build + tessellation cost per scene, no window
//!   cargo run -p rr-ui --release --example motion_gl -- shot       gallery screenshot -> docs/design/motion-proto.png
//!   cargo run -p rr-ui --release --example motion_gl -- aa [ppp=1.25]   line / arc coverage numbers at a fractional scale
//!   cargo run -p rr-ui --release --example motion_gl -- live        interactive gallery; keys 1..8 select a scene
//!
//! The GL callback path is the same `egui_glow::CallbackFn` the plugin's egui-baseview renderer
//! paints; only the window / context creation differs (eframe vs baseview).

use eframe::egui::{self, pos2, vec2, Color32, Context, Pos2, Rect, Shape, Stroke, ViewportCommand};
use std::time::Instant;

#[path = "support/fx_tex.rs"]
mod fx_tex;
#[path = "support/wavegl.rs"]
mod wavegl;
use fx_tex::Fx;
use wavegl::WaveGl;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Scene {
    Base,
    CpuLines,
    CpuFill,
    GlStatic,
    GlReupload,
    GlNoGlow,
    FxTex,
    FxNative,
    Gallery,
}

const BENCH: [Scene; 8] = [Scene::Base, Scene::CpuLines, Scene::CpuFill, Scene::GlStatic, Scene::GlReupload, Scene::GlNoGlow, Scene::FxTex, Scene::FxNative];
const LIVE_KEYS: [(egui::Key, Scene); 9] = [
    (egui::Key::Num1, Scene::Base),
    (egui::Key::Num2, Scene::CpuLines),
    (egui::Key::Num3, Scene::CpuFill),
    (egui::Key::Num4, Scene::GlStatic),
    (egui::Key::Num5, Scene::GlReupload),
    (egui::Key::Num6, Scene::GlNoGlow),
    (egui::Key::Num7, Scene::FxTex),
    (egui::Key::Num8, Scene::FxNative),
    (egui::Key::Num9, Scene::Gallery),
];

struct Sc {
    gl: WaveGl,
    fx: Fx,
    waves: Vec<f32>,
}

const W: f32 = 1000.0;
const H: f32 = 832.0;
/// 3D view size used by single-scene runs (the GTS header display is smaller, this is generous).
const VIEW: Rect = Rect { min: pos2(20.0, 20.0), max: pos2(660.0, 420.0) };

fn draw_fx_panel(p: &egui::Painter, sc: &Sc, area: Rect, t: f64, native: bool) {
    let panel = Color32::from_rgb(34, 36, 40);
    p.rect_filled(area, 0, Color32::from_rgb(26, 28, 31));
    sc.fx.brushed(p, area, 0.9);
    let (cols, rows) = (6, 4);
    let cell = vec2(area.width() / cols as f32, area.height() / rows as f32);
    for j in 0..rows {
        for i in 0..cols {
            let r = Rect::from_min_size(area.min + vec2(i as f32 * cell.x, j as f32 * cell.y) + vec2(14.0, 14.0), cell - vec2(28.0, 28.0));
            if native {
                p.add(egui::Shadow { offset: [0, 4], blur: 14, spread: 0, color: Color32::BLACK.gamma_multiply(0.6) }.as_shape(r, 4));
            } else {
                sc.fx.drop_shadow(p, r, vec2(0.0, 4.0), 0.6);
            }
            p.rect_filled(r, 4, panel);
            p.rect_stroke(r, 4, Stroke::new(1.0_f32, Color32::from_gray(58)), egui::StrokeKind::Inside);
            // an LED-like glow under a small lamp
            let c = r.center() + vec2(0.0, (t as f32 * 2.0 + (i + j) as f32).sin() * 4.0);
            if !native {
                sc.fx.glow_at(p, c, 64.0, Color32::from_rgb(70, 220, 255), 0.55 + 0.45 * (t as f32 * 3.0 + i as f32).sin());
            } else {
                p.circle_filled(c, 20.0, Color32::from_rgb(70, 220, 255).gamma_multiply(0.15));
            }
            p.circle_filled(c, 5.0, Color32::from_rgb(160, 240, 255));
        }
    }
}

fn draw(ctx: &Context, sc: &mut Sc, scene: Scene, t: f64) {
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(Color32::from_rgb(20, 21, 24))).show(ctx, |ui| {
        let p = ui.painter().clone();
        let mut view = wavegl::View { yaw: (t * 0.5).sin() as f32 * 0.3, focus: (t * 0.25).fract() as f32, ..Default::default() };
        let bgc = Color32::from_rgb((view.bg[0] * 255.0) as u8, (view.bg[1] * 255.0) as u8, (view.bg[2] * 255.0) as u8);
        match scene {
            Scene::Base => {
                p.rect_filled(VIEW, 4, bgc);
            }
            Scene::CpuLines | Scene::CpuFill => {
                p.rect_filled(VIEW, 4, bgc);
                let clipped = p.with_clip_rect(VIEW);
                wavegl::paint_cpu(&clipped, VIEW, &sc.waves, &view, scene == Scene::CpuFill);
            }
            Scene::GlStatic | Scene::GlReupload | Scene::GlNoGlow => {
                p.rect_filled(VIEW, 4, bgc);
                view.reupload = scene == Scene::GlReupload;
                if scene == Scene::GlNoGlow {
                    view.glow = 0.0;
                }
                sc.gl.0.lock().unwrap().view = view;
                sc.gl.show(&p.with_clip_rect(VIEW), VIEW);
            }
            Scene::FxTex => draw_fx_panel(&p, sc, Rect::from_min_size(pos2(20.0, 20.0), vec2(960.0, 790.0)), t, false),
            Scene::FxNative => draw_fx_panel(&p, sc, Rect::from_min_size(pos2(20.0, 20.0), vec2(960.0, 790.0)), t, true),
            Scene::Gallery => {
                let a = Rect::from_min_size(pos2(20.0, 20.0), vec2(470.0, 300.0));
                let b = Rect::from_min_size(pos2(510.0, 20.0), vec2(470.0, 300.0));
                p.rect_filled(a, 4, bgc);
                p.rect_filled(b, 4, bgc);
                sc.gl.0.lock().unwrap().view = view;
                sc.gl.show(&p.with_clip_rect(a), a);
                wavegl::paint_cpu(&p.with_clip_rect(b), b, &sc.waves, &view, true);
                p.text(a.left_top() + vec2(8.0, 6.0), egui::Align2::LEFT_TOP, "GL PaintCallback (shader ribbons + glow)", egui::FontId::proportional(12.0), Color32::from_gray(200));
                p.text(b.left_top() + vec2(8.0, 6.0), egui::Align2::LEFT_TOP, "CPU egui shapes (Shape::line + mesh fill)", egui::FontId::proportional(12.0), Color32::from_gray(200));
                draw_fx_panel(&p, sc, Rect::from_min_size(pos2(20.0, 340.0), vec2(960.0, 470.0)), t, false);
            }
        }
        ui.allocate_rect(Rect::from_min_size(Pos2::ZERO, vec2(W, H)), egui::Sense::hover());
    });
}

fn stats(v: &[f64]) -> (f64, f64, f64) {
    if v.is_empty() {
        return (f64::NAN, f64::NAN, f64::NAN);
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (s.iter().sum::<f64>() / s.len() as f64, s[s.len() / 2], s[((s.len() as f64 * 0.99) as usize).min(s.len() - 1)])
}

// ---- headless: build + tessellate ---------------------------------------------------------

fn headless() {
    let ctx = Context::default();
    rr_ui::theme::apply(&ctx);
    let mut sc = Sc { gl: WaveGl::new(wavegl::demo_waves()), fx: Fx::new(&ctx), waves: wavegl::demo_waves() };
    println!("texture generation (ms): shadow {:.2}, glow {:.2}, noise {:.2}", sc.fx.gen_ms[0], sc.fx.gen_ms[1], sc.fx.gen_ms[2]);
    println!("{:<10} {:>9} {:>9} {:>9} {:>9} {:>9}  {:>9} {:>9} {:>9}", "scene", "build mean", "tess mean", "total mean", "p99", "max", "verts", "indices", "meshes");
    for scene in [Scene::Base, Scene::CpuLines, Scene::CpuFill, Scene::GlStatic, Scene::FxTex, Scene::FxNative] {
        let (mut tot, mut ui_t, mut te_t) = (vec![], vec![], vec![]);
        let (mut vs, mut is, mut ms) = (0, 0, 0);
        for f in 0..630 {
            let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(W, H))), time: Some(f as f64 / 60.0), ..Default::default() };
            let t0 = Instant::now();
            let out = ctx.run(input, |ctx| draw(ctx, &mut sc, scene, f as f64 / 60.0));
            let t1 = Instant::now();
            let prims = ctx.tessellate(out.shapes, 1.0);
            let t2 = Instant::now();
            if f >= 30 {
                ui_t.push((t1 - t0).as_secs_f64() * 1e3);
                te_t.push((t2 - t1).as_secs_f64() * 1e3);
                tot.push((t2 - t0).as_secs_f64() * 1e3);
            }
            if f == 100 {
                (vs, is, ms) = (0, 0, 0);
                for c in &prims {
                    if let egui::epaint::Primitive::Mesh(m) = &c.primitive {
                        vs += m.vertices.len();
                        is += m.indices.len();
                        ms += 1;
                    }
                }
            }
        }
        let (m_ui, ..) = stats(&ui_t);
        let (m_te, ..) = stats(&te_t);
        let (m_t, _, p99) = stats(&tot);
        let mx = tot.iter().cloned().fold(0.0, f64::max);
        println!("{:<10} {:>9.3} {:>9.3} {:>9.3} {:>9.3} {:>9.3}  {:>9} {:>9} {:>9}", format!("{scene:?}"), m_ui, m_te, m_t, p99, mx, vs, is, ms);
    }
}

// ---- eframe app: bench / shot / live / aa --------------------------------------------------

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Bench,
    Shot,
    Live,

}

struct App {
    mode: Mode,
    sc: Sc,
    scene: Scene,
    frame: u32,
    scene_idx: usize,
    last: Option<Instant>,
    upd: Vec<f64>,
    interval: Vec<f64>,
    rows: Vec<String>,
    asked: bool,
    ppp: f32,
    gl_info_printed: bool,
}

const WARM: u32 = 90;
const MEASURE: u32 = 480;

fn pr(label: &str, v: &[f64]) -> String {
    let (m, p50, p99) = stats(v);
    format!("{label} mean {m:6.3} p50 {p50:6.3} p99 {p99:6.3}")
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _f: &mut eframe::Frame) {
        let start = Instant::now();
        if self.frame == 0 && self.scene_idx == 0 {
            let native = ctx.native_pixels_per_point().unwrap_or(1.0);
            ctx.set_pixels_per_point(self.ppp);
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(vec2(W, H) * self.ppp / native));
        }
        if self.mode == Mode::Bench {
            if self.scene_idx >= BENCH.len() {
                return;
            }
            self.scene = BENCH[self.scene_idx];
        }
        if let Some(l) = self.last {
            if self.frame >= WARM {
                self.interval.push((start - l).as_secs_f64() * 1e3);
            }
        }
        self.last = Some(start);
        if self.mode == Mode::Live {
            for (k, s) in LIVE_KEYS {
                if ctx.input(|i| i.key_pressed(k)) {
                    self.scene = s;
                }
            }
        }
        let t = if self.mode == Mode::Shot { 1.7 } else { ctx.input(|i| i.time) };
        draw(ctx, &mut self.sc, self.scene, t);
        if self.mode == Mode::Live {
            let fps = 1.0 / ctx.input(|i| i.stable_dt).max(1e-4);
            egui::Area::new("hud".into()).fixed_pos(pos2(700.0, 24.0)).show(ctx, |ui| {
                ui.colored_label(Color32::WHITE, format!("{:?}  {:.0} fps  (keys 1-9)", self.scene, fps));
            });
        }
        if self.frame >= WARM {
            self.upd.push(start.elapsed().as_secs_f64() * 1e3);
        }
        if !self.gl_info_printed {
            let info = self.sc.gl.0.lock().unwrap().gl_info.clone();
            if !info.is_empty() {
                println!("GL: {info}");
                self.gl_info_printed = true;
            }
        }
        self.frame += 1;
        match self.mode {
            Mode::Bench if self.frame >= WARM + MEASURE => {
                let (gpu, cpu) = {
                    let s = self.sc.gl.0.lock().unwrap();
                    (s.gpu_samples.iter().map(|&x| x as f64).collect::<Vec<_>>(), s.cpu_samples.iter().skip(WARM as usize).map(|&x| x as f64).collect::<Vec<_>>())
                };
                let mut row = format!("{:<10} update(UI build) [{}]  frame interval [{}]", format!("{:?}", self.scene), pr("ms", &self.upd), pr("ms", &self.interval));
                if !cpu.is_empty() {
                    row += &format!("  | GL callback CPU [{}]", pr("ms", &cpu));
                }
                if !gpu.is_empty() {
                    row += &format!("  GPU [{}]", pr("ms", &gpu));
                }
                self.rows.push(row);
                self.upd.clear();
                self.interval.clear();
                {
                    let mut s = self.sc.gl.0.lock().unwrap();
                    s.gpu_samples.clear();
                    s.cpu_samples.clear();
                }
                self.scene_idx += 1;
                self.frame = 0;
                self.last = None;
                if self.scene_idx >= BENCH.len() {
                    println!("---- vsync OFF, continuous repaint, 1000x832 px, {} frames per scene ----", MEASURE);
                    for r in &self.rows {
                        println!("{r}");
                    }
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                    return;
                }
                ctx.request_repaint();
            }
            Mode::Shot => {
                let ready = self.frame >= 8;
                if ready && !self.asked {
                    self.asked = true;
                    ctx.send_viewport_cmd(ViewportCommand::Screenshot(Default::default()));
                }
                let shot = ctx.input(|i| i.events.iter().find_map(|e| if let egui::Event::Screenshot { image, .. } = e { Some(image.clone()) } else { None }));
                if let Some(img) = shot {
                    if self.mode == Mode::Shot {
                        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/design/motion-proto.png");
                        save_png(&path, &img);
                        println!("wrote {} ({}x{})", path.display(), img.size[0], img.size[1]);
                    }
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                } else {
                    ctx.request_repaint();
                }
            }
            _ => ctx.request_repaint(),
        }
    }
}

fn save_png(path: &std::path::Path, img: &egui::ColorImage) {
    let file = std::io::BufWriter::new(std::fs::File::create(path).expect("create png"));
    let mut enc = png::Encoder::new(file, img.size[0] as u32, img.size[1] as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(img.as_raw()).unwrap();
}

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    let has = |s: &str| args.iter().any(|a| a == s);
    if has("headless") {
        headless();
        return Ok(());
    }
    if has("aa") {
        return aa::run(args.iter().find_map(|a| a.strip_prefix("ppp=").and_then(|s| s.parse().ok())).unwrap_or(1.0));
    }
    let mode = if has("bench") {
        Mode::Bench
    } else if has("shot") {
        Mode::Shot
    } else {
        Mode::Live
    };
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([W, H]).with_resizable(false).with_title("motion_gl"),
        vsync: mode != Mode::Bench, // bench measures uncapped throughput
        depth_buffer: 24,           // same as the plugin's GlConfig
        stencil_buffer: 8,
        ..Default::default()
    };
    eframe::run_native("motion_gl", opts, Box::new(move |cc| {
        rr_ui::theme::apply(&cc.egui_ctx);
        let fx = Fx::new(&cc.egui_ctx);
        println!("texture generation (ms): shadow {:.2}, glow {:.2}, noise {:.2}", fx.gen_ms[0], fx.gen_ms[1], fx.gen_ms[2]);
        let sc = Sc { gl: WaveGl::new(wavegl::demo_waves()), fx, waves: wavegl::demo_waves() };
        Ok(Box::new(App {
            mode,
            sc,
            scene: if mode == Mode::Shot { Scene::Gallery } else { Scene::Gallery },
            frame: 0,
            scene_idx: 0,
            last: None,
            upd: vec![],
            interval: vec![],
            rows: vec![],
            asked: false,
            ppp: 1.0,
            gl_info_printed: false,
        }))
    }))
}

// ---- anti-aliasing numbers -----------------------------------------------------------------

mod aa {
    use super::*;

    struct AaApp {
        ppp: f32,
        frame: u32,
        asked: bool,
    }

    /// (label, y in points, width in points)
    const LINES: [(&str, f32, f32); 8] = [
        ("w1.0 y=20.0 (on grid at ppp1)", 20.0, 1.0),
        ("w1.0 y=40.5", 40.5, 1.0),
        ("w1.0 y=60.3", 60.3, 1.0),
        ("w0.5 y=80.5", 80.5, 0.5),
        ("w1.5 y=100.5", 100.5, 1.5),
        ("w2.0 y=120.0", 120.0, 2.0),
        ("w1.0 y=140.0", 140.0, 1.0),
        ("w1.0 y=160.5", 160.5, 1.0),
    ];

    impl eframe::App for AaApp {
        fn update(&mut self, ctx: &Context, _f: &mut eframe::Frame) {
            if self.frame == 0 {
                let native = ctx.native_pixels_per_point().unwrap_or(1.0);
                ctx.set_pixels_per_point(self.ppp);
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(vec2(320.0, 240.0) * self.ppp / native));
            }
            self.frame += 1;
            egui::CentralPanel::default().frame(egui::Frame::NONE.fill(Color32::BLACK)).show(ctx, |ui| {
                let p = ui.painter();
                for (_, y, w) in LINES {
                    p.line_segment([pos2(10.0, y), pos2(110.0, y)], Stroke::new(w, Color32::WHITE));
                }
                // widths given in PHYSICAL pixels (1/ppp, 2/ppp) at an arbitrary y
                for (_, y, k) in self.px_lines() {
                    p.line_segment([pos2(10.0, y), pos2(110.0, y)], Stroke::new(k / self.ppp, Color32::WHITE));
                }
                p.line_segment([pos2(130.0, 20.0), pos2(210.0, 90.0)], Stroke::new(1.0_f32, Color32::WHITE));
                // thin knob arc: 270 degrees, 1.5 pt, drawn as a polyline like rr-ui does
                let c = pos2(260.0, 60.0);
                let pts: Vec<Pos2> = (0..=64).map(|i| {
                    let a = (135.0 + 270.0 * i as f32 / 64.0).to_radians();
                    c + vec2(a.cos(), a.sin()) * 24.0
                }).collect();
                p.add(Shape::line(pts, Stroke::new(1.5_f32, Color32::WHITE)));
                p.circle_stroke(pos2(260.0, 150.0), 24.0, Stroke::new(1.0_f32, Color32::WHITE));
                p.rect_stroke(Rect::from_min_size(pos2(130.0, 120.0), vec2(80.0, 40.0)), 0, Stroke::new(1.0_f32, Color32::WHITE), egui::StrokeKind::Inside);
                p.rect_stroke(Rect::from_min_size(pos2(130.5, 170.5), vec2(80.0, 40.0)), 0, Stroke::new(1.0_f32, Color32::WHITE), egui::StrokeKind::Inside);
            });
            if self.frame > 8 && !self.asked {
                self.asked = true;
                ctx.send_viewport_cmd(ViewportCommand::Screenshot(Default::default()));
            }
            let shot = ctx.input(|i| i.events.iter().find_map(|e| if let egui::Event::Screenshot { image, .. } = e { Some(image.clone()) } else { None }));
            if let Some(img) = shot {
                self.report(&img);
                ctx.send_viewport_cmd(ViewportCommand::Close);
            } else {
                ctx.request_repaint();
            }
        }
    }

    impl AaApp {
        /// (label, y in points, width in physical px)
        fn px_lines(&self) -> [(&'static str, f32, f32); 3] {
            [("w=1 phys px, y=180.2", 180.2, 1.0), ("w=2 phys px, y=200.0", 200.0, 2.0), ("w=1 phys px, y=220.7", 220.7, 1.0)]
        }
        fn report(&self, img: &egui::ColorImage) {
            let (w, h) = (img.size[0], img.size[1]);
            println!("screenshot {w}x{h} px at ppp {} (expect {}x{})", self.ppp, (320.0 * self.ppp).round(), (240.0 * self.ppp).round());
            let px = |x: usize, y: usize| img.pixels[y * w + x].r();
            let x = (60.0 * self.ppp) as usize;
            println!("horizontal 1-D coverage profile down the column x=60pt (pixel rows around each line, 0..255):");
            for (label, y, _) in LINES {
                let yp = (y * self.ppp).floor() as i32;
                let prof: Vec<u8> = (yp - 2..=yp + 3).filter(|&r| r >= 0 && (r as usize) < h).map(|r| px(x, r as usize)).collect();
                let sum: u32 = prof.iter().map(|&v| v as u32).sum();
                println!("  {label:<34} rows {:>3}..{:<3}: {:?}  sum/255 = {:.2} px of ink", yp - 2, yp + 3, prof, sum as f32 / 255.0);
            }
            for (label, y, k) in self.px_lines() {
                let yp = (y * self.ppp).floor() as i32;
                let prof: Vec<u8> = (yp - 2..=yp + 3).filter(|&r| r >= 0 && (r as usize) < h).map(|r| px(x, r as usize)).collect();
                let sum: u32 = prof.iter().map(|&v| v as u32).sum();
                println!("  {label:<34} rows {:>3}..{:<3}: {:?}  sum/255 = {:.2} px of ink (wanted {k})", yp - 2, yp + 3, prof, sum as f32 / 255.0);
            }
            // vertical border of an outlined rect (integer vs half-point origin)
            for (label, x0) in [("rect stroke x=130.0", 130.0f32), ("rect stroke x=130.5", 130.5)] {
                let yy = if x0 == 130.0 { 140.0 } else { 190.5 };
                let xp = (x0 * self.ppp).floor() as i32;
                let prof: Vec<u8> = (xp - 2..=xp + 3).map(|c| px(c as usize, (yy * self.ppp) as usize)).collect();
                println!("  {label:<34} cols {:>3}..{:<3}: {:?}", xp - 2, xp + 3, prof);
            }
            // arc roughness: count distinct grey levels along the knob arc and max neighbour jump
            let mut grey = std::collections::BTreeSet::new();
            let (cx, cy, r) = (260.0 * self.ppp, 60.0 * self.ppp, 24.0 * self.ppp);
            for k in 0..720 {
                let a = (135.0f32 + 270.0 * k as f32 / 720.0).to_radians();
                for dr in -2..=2 {
                    let rr = r + dr as f32;
                    let (x, y) = ((cx + a.cos() * rr).round() as usize, (cy + a.sin() * rr).round() as usize);
                    grey.insert(px(x, y));
                }
            }
            println!("  knob arc 1.5pt: {} distinct grey levels sampled across the stroke (smooth AA = many)", grey.len());
            let path = std::env::temp_dir().join(format!("motion-aa-{}.png", self.ppp));
            save_png(&path, img);
            println!("  wrote {}", path.display());
        }
    }

    pub fn run(ppp: f32) -> eframe::Result {
        let opts = eframe::NativeOptions { viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]).with_resizable(false), ..Default::default() };
        eframe::run_native("aa", opts, Box::new(move |_| Ok(Box::new(AaApp { ppp, frame: 0, asked: false }))))
    }
}
