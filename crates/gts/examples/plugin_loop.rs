//! Frame cadence of the plugin's REAL window path, standalone. Opens a baseview window through
//! egui-baseview with the exact GlConfig nih_plug_egui uses (GL 3.2 core, depth 24, no MSAA,
//! vsync ON), runs for N rendered frames, and reports how often a frame is actually PAINTED
//! (timestamped inside a PaintCallback, i.e. just before swap_buffers) and how often the update
//! closure runs.
//!
//!   cargo run -p gts --release --example plugin_loop -- [gts] [notes] [fx] [gl] [vsync=0|1] [repaint=every|after33|after16]
//!                                                      [tbp] [frames=600] [scale=1.0] [shot] [cpu]
//!
//!   gts      draw the real GTS editor (else a trivial panel); its hero is the GL paint callback
//!   notes    an audio thread plays the riddim demo pattern (Wobble Shot) on the real engine in real
//!            time and publishes through the plugin's own VizWriter / Meters, so every live display moves;
//!            also reports the hero's own GPU time (timer query)
//!   page=fx  show the FX page
//!   cpu      force the hero's CPU fallback
//!   fxtex    also draw generated-texture effects (drop shadows, additive glow sprites, brushed noise) over the panel
//!   gl       also draw the 3D wavetable GL view (wavegl.rs) over the header
//!   repaint  every   = ctx.request_repaint() each update  (what nih_plug_egui does, always)
//!            after33 = only request_repaint_after(33 ms)  (what gts/src/editor.rs asks for)
//!            after16 = only request_repaint_after(16 ms)
//!   zoom=1.5 (needs --features zoom-patch and the patched egui-baseview, see docs/design/motion-feasibility.md)
//!   msaa=4   request a 4x multisampled framebuffer (nih_plug_egui hard-codes samples: None)
//!   tbp      call timeBeginPeriod(1) first (DAWs usually do; affects the WM_TIMER that drives frames)
//!   shot     save the first stable frame as docs/design/motion-proto-plugin.png (uses glReadPixels)
//!
//! Caveat: a plugin window is a CHILD of the host's window and the host pumps the message loop;
//! here baseview owns the loop (open_blocking). The scheduling code is identical.

use baseview::gl::GlConfig;
use baseview::{Size, WindowOpenOptions, WindowScalePolicy};
use eframe::egui::{self, pos2, vec2, Rect};
use egui_baseview::{EguiWindow, GraphicsConfig};
use gts::dsp::tables::Tables;
use gts::editor::{draw, BusView, LiveView, WINDOW};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[path = "../../rr-ui/examples/support/wavegl.rs"]
mod wavegl;
#[path = "../../rr-ui/examples/support/fx_tex.rs"]
mod fx_tex;
#[path = "support/demo.rs"]
mod demo;

#[cfg(windows)]
#[link(name = "winmm")]
extern "system" {
    fn timeBeginPeriod(u: u32) -> u32;
}

#[derive(Clone, Copy, PartialEq)]
enum Repaint {
    Every,
    After(u64),
}

struct State {
    gts: bool,
    gl: Option<wavegl::WaveGl>,
    ctl: demo::MapCtl,
    tables: Tables,
    bv: BusView,
    live: LiveView,
    meters: Arc<gts::editor::Meters>,
    hero_gpu: Arc<Mutex<Vec<f32>>>,
    repaint: Repaint,
    frames: usize,
    stamps: Arc<Mutex<Vec<Instant>>>,
    updates: Arc<Mutex<Vec<Instant>>>,
    update_ms: Arc<Mutex<Vec<f64>>>,
    shot: bool,
    zoom: f32,
    zoom_pending: bool,
    fx: Option<fx_tex::Fx>,
    want_fx: bool,
}

fn stats(v: &[f64]) -> String {
    if v.is_empty() {
        return "n/a".into();
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean = s.iter().sum::<f64>() / s.len() as f64;
    let over = s.iter().filter(|&&x| x > 20.0).count() as f64 / s.len() as f64 * 100.0;
    format!("mean {mean:6.2} p50 {:6.2} p99 {:6.2} max {:6.2} | >20ms: {over:4.1}% | = {:5.1} fps", s[s.len() / 2], s[(s.len() as f64 * 0.99) as usize], s[s.len() - 1], 1000.0 / mean)
}

fn deltas(t: &[Instant]) -> Vec<f64> {
    t.windows(2).skip(30).map(|w| (w[1] - w[0]).as_secs_f64() * 1e3).collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let has = |s: &str| args.iter().any(|a| a == s);
    let val = |k: &str| args.iter().find_map(|a| a.strip_prefix(&format!("{k}=")).map(String::from));
    let vsync = val("vsync").map(|v| v != "0").unwrap_or(true);
    let frames: usize = val("frames").and_then(|v| v.parse().ok()).unwrap_or(600);
    let scale: f64 = val("scale").and_then(|v| v.parse().ok()).unwrap_or(1.0);
    let repaint = match val("repaint").as_deref() {
        Some("after33") => Repaint::After(33),
        Some("after16") => Repaint::After(16),
        _ => Repaint::Every,
    };
    #[cfg(windows)]
    if has("tbp") {
        unsafe { timeBeginPeriod(1) };
    }
    let mut tables = Tables::new();
    tables.generate();
    let stamps = Arc::new(Mutex::new(Vec::<Instant>::new()));
    let updates = Arc::new(Mutex::new(Vec::<Instant>::new()));
    let update_ms = Arc::new(Mutex::new(Vec::<f64>::new()));
    let (pi, preset) = demo::factory("Wobble Shot");
    let ctl = demo::MapCtl::with_preset(Some(preset));
    let mut patch = preset.patch;
    if val("page").as_deref() == Some("fx") {
        demo::fx_on(&ctl, &mut patch); // the FX page is measured with its modules running
    }
    let feed = demo::Feed::new(patch);
    let (meters, viz, ring) = (feed.meters.clone(), feed.viz.clone(), feed.ring.clone());
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let audio = has("notes").then(|| {
        let stop = stop.clone();
        let mut feed = feed;
        // Real-time pacing: render whatever the clock says is due, in <= 256-sample blocks.
        std::thread::spawn(move || {
            let t0 = Instant::now();
            let mut done = 0usize;
            while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                let due = (t0.elapsed().as_secs_f64() * feed.sr() as f64) as usize;
                if due > done {
                    let n = (due - done).min(4096);
                    feed.render(n);
                    done += n;
                }
                std::thread::sleep(Duration::from_millis(2));
            }
        })
    });
    let mut live = LiveView::new(viz, Some(ring));
    live.hero.force_cpu = has("cpu");
    live.hero.measure_gpu = true;
    let hero_gpu = Arc::new(Mutex::new(Vec::<f32>::new()));
    let state = State {
        gts: has("gts"),
        gl: has("gl").then(|| wavegl::WaveGl::new(wavegl::demo_waves())),
        ctl,
        tables,
        bv: BusView { label: "GTS 1".into(), oversampling: 2, preset: Some(pi), peak_db: -6.0, fx_page: val("page").as_deref() == Some("fx"), ..Default::default() },
        live,
        meters,
        hero_gpu: hero_gpu.clone(),
        repaint,
        frames,
        stamps: stamps.clone(),
        updates: updates.clone(),
        update_ms: update_ms.clone(),
        shot: has("shot"),
        fx: None,
        want_fx: has("fxtex"),
        zoom: val("zoom").and_then(|v| v.parse().ok()).unwrap_or(1.0),
        zoom_pending: val("zoom").is_some(),
    };
    let desc = format!(
        "gts={} notes={} page={} hero={} gl-overlay={} vsync={vsync} repaint={} tbp={} scale={scale}",
        has("gts"),
        has("notes"),
        val("page").unwrap_or("synth".into()),
        if has("cpu") { "CPU fallback" } else { "GL callback" },
        has("gl"),
        match repaint { Repaint::Every => "every".to_string(), Repaint::After(n) => format!("after{n}") },
        has("tbp")
    );
    let report_stamps = stamps.clone();
    let gl_report = state.gl.clone();
    EguiWindow::open_blocking(
        WindowOpenOptions {
            title: "plugin_loop".into(),
            size: Size::new(WINDOW.0 as f64, WINDOW.1 as f64),
            scale: WindowScalePolicy::ScaleFactor(scale),
            gl_config: Some(GlConfig {
                version: (3, 2),
                red_bits: 8,
                blue_bits: 8,
                green_bits: 8,
                alpha_bits: 8,
                depth_bits: 24,
                stencil_bits: 8,
                samples: val("msaa").and_then(|v| v.parse().ok()),
                srgb: true,
                double_buffer: true,
                vsync,
                ..Default::default()
            }),
        },
        GraphicsConfig::default(),
        state,
        |ctx, _q, _s| rr_ui::theme::apply(ctx),
        move |ctx, queue, st: &mut State| {
            let t0 = Instant::now();
            #[cfg(feature = "zoom-patch")]
            if st.zoom_pending {
                st.zoom_pending = false;
                queue.zoom(st.zoom);
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(vec2(WINDOW.0 as f32, WINDOW.1 as f32) * st.zoom));
            }
            st.updates.lock().unwrap().push(t0);
            match st.repaint {
                Repaint::Every => ctx.request_repaint(), // nih_plug_egui does this unconditionally
                Repaint::After(ms) => ctx.request_repaint_after(Duration::from_millis(ms)),
            }
            egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
                if st.gts {
                    st.live.peaks = std::array::from_fn(|i| f32::from_bits(st.meters.peak[i].swap(0, std::sync::atomic::Ordering::Relaxed)));
                    draw(ui, &st.ctl, &st.tables, &mut st.live, &mut st.bv);
                    st.hero_gpu.lock().unwrap().extend(st.live.hero.take_gpu_ms());
                } else {
                    ui.painter().rect_filled(ui.max_rect(), 0, egui::Color32::from_rgb(30, 32, 36));
                }
                if let Some(gl) = &st.gl {
                    let t = ui.input(|i| i.time);
                    let r = Rect::from_min_size(pos2(34.0, 66.0), vec2(300.0, 150.0));
                    let mut s = gl.0.lock().unwrap();
                    s.view.yaw = (t * 0.5).sin() as f32 * 0.3;
                    s.view.focus = (t * 0.25).fract() as f32;
                    drop(s);
                    gl.show(&ui.painter().with_clip_rect(r), r);
                }
                if st.want_fx {
                    let fx = st.fx.get_or_insert_with(|| fx_tex::Fx::new(ui.ctx()));
                    let t = ui.input(|i| i.time) as f32;
                    let p = ui.painter();
                    fx.brushed(p, Rect::from_min_size(pos2(600.0, 60.0), vec2(380.0, 120.0)), 0.6);
                    for i in 0..4 {
                        let r = Rect::from_min_size(pos2(620.0 + i as f32 * 90.0, 80.0), vec2(70.0, 80.0));
                        fx.drop_shadow(p, r, vec2(0.0, 4.0), 0.7);
                        p.rect_filled(r, 4, egui::Color32::from_rgb(40, 44, 50));
                        fx.glow_at(p, r.center(), 70.0, egui::Color32::from_rgb(70, 230, 200), 0.5 + 0.5 * (t * 3.0 + i as f32).sin());
                    }
                }
                // 1x1 callback that timestamps every PAINTED frame, just before swap_buffers.
                let stamps = st.stamps.clone();
                let shot = st.shot;
                let cb = egui_glow::CallbackFn::new(move |info, p| {
                    let n = {
                        let mut s = stamps.lock().unwrap();
                        s.push(Instant::now());
                        s.len()
                    };
                    if n == 1 {
                        use egui_glow::glow::HasContext as _;
                        let gl = p.gl();
                        let (samples, buffers) = unsafe { (gl.get_parameter_i32(egui_glow::glow::SAMPLES), gl.get_parameter_i32(egui_glow::glow::SAMPLE_BUFFERS)) };
                        println!("framebuffer: GL_SAMPLES {samples}, GL_SAMPLE_BUFFERS {buffers}");
                    }
                    if shot && n == 40 {
                        use egui_glow::glow::HasContext as _;
                        let [w, h] = info.screen_size_px;
                        let mut buf = vec![0u8; (w * h * 4) as usize];
                        unsafe { p.gl().read_pixels(0, 0, w as i32, h as i32, egui_glow::glow::RGBA, egui_glow::glow::UNSIGNED_BYTE, egui_glow::glow::PixelPackData::Slice(Some(&mut buf))) };
                        let row = (w * 4) as usize;
                        let flipped: Vec<u8> = buf.chunks(row).rev().flatten().copied().collect();
                        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(if w == 1000 { "../../docs/design/motion-proto-plugin.png" } else { "../../docs/design/motion-proto-zoom.png" });
                        let mut enc = png::Encoder::new(std::io::BufWriter::new(std::fs::File::create(&path).unwrap()), w, h);
                        enc.set_color(png::ColorType::Rgba);
                        enc.set_depth(png::BitDepth::Eight);
                        enc.write_header().unwrap().write_image_data(&flipped).unwrap();
                        println!("wrote {} ({w}x{h})", path.display());
                    }
                });
                ui.painter().add(egui::PaintCallback { rect: Rect::from_min_size(pos2(0.0, 0.0), vec2(1.0, 1.0)), callback: Arc::new(cb) });
            });
            st.update_ms.lock().unwrap().push(t0.elapsed().as_secs_f64() * 1e3);
            let painted = st.stamps.lock().unwrap().len();
            if painted >= st.frames {
                queue.close_window();
            }
        },
    );
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    if let Some(a) = audio {
        a.join().unwrap();
    }
    let t = report_stamps.lock().unwrap().clone();
    println!("== {desc} ==");
    println!("painted-frame interval (ms): {}", stats(&deltas(&t)));
    println!("update() call interval  (ms): {}", stats(&deltas(&updates.lock().unwrap())));
    let um: Vec<f64> = update_ms.lock().unwrap().iter().skip(30).copied().collect();
    if let Some(g) = gl_report {
        let g = g.0.lock().unwrap();
        let f = |v: &[f32]| v.iter().skip(30).map(|&x| x as f64).collect::<Vec<_>>();
        println!("GL {}", g.gl_info);
        println!("GL callback CPU (ms): {}", stats(&f(&g.cpu_samples)).split(" | ").next().unwrap_or(""));
        println!("GL callback GPU (ms): {}", stats(&f(&g.gpu_samples)).split(" | ").next().unwrap_or(""));
    }
    println!("update() duration       (ms): {}", stats(&um).split(" | ").next().unwrap_or(""));
    let d = deltas(&t);
    println!("capture: {} painted frames over {:.2} s, intervals > 33 ms: {}, > 50 ms: {}", t.len(), d.iter().sum::<f64>() / 1e3, d.iter().filter(|&&x| x > 33.0).count(), d.iter().filter(|&&x| x > 50.0).count());
    let hg: Vec<f64> = hero_gpu.lock().unwrap().iter().skip(30).map(|&x| x as f64).collect();
    println!("hero GPU time (timer query, ms): {} ({} samples)", stats(&hg).split(" | ").next().unwrap_or(""), hg.len());
}
