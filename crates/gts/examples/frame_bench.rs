//! CPU frame-time budget of the REAL GTS editor draw, headless (no window, no GPU).
//! Runs `editor::draw` in a bare egui context for N frames at 1000x832 and times (a) the UI pass
//! (begin_pass .. end_pass) and (b) tessellation, then prints mean / p50 / p99 / max and the mesh
//! handed to the GL painter. Between frames the real engine plays the riddim demo pattern
//! (Wobble Shot) through the plugin's own publisher (scope ring, meters, voice / ModLive record),
//! 735 samples per 60 Hz frame; engine time is NOT counted. Table generation is excluded.
//!
//!   cargo run -p gts --release --example frame_bench [-- fx] [-- ppp=1.5] [-- hover] [-- idle] [-- cpu] [-- frames=600]
//!
//!   matrix the MATRIX page (with eight routings in the slots), or `-- drag` for a patch drag in progress
//!   browser the preset browser open over a 200-preset fixture library (190 user presets, the awkward
//!          files and the factory bank), the list scrolling under the pointer all the while and
//!          a search typed and cleared now and then (SPEC v2 Track 6 gate: UI + tessellation <= 4 ms mean)
//!   idle   no notes (the silent-state cost cap, MOTION.md 11 #13)
//!   cpu    force the hero's CPU fallback (egui shapes) instead of the GL paint callback
//!
//! The hero's GL path is a paint callback (0 egui vertices); its GPU cost is measured by
//! `plugin_loop` (timer query) and `frame_gpu`.

use eframe::egui::{self, vec2, Pos2, RawInput, Rect};
use gts::dsp::tables::Tables;
use gts::editor::{draw, BusView, LiveView, Page, WINDOW};
use std::time::Instant;

#[path = "support/demo.rs"]
mod demo;

fn stats(label: &str, mut v: Vec<f64>) -> (f64, f64) {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    let p = |q: f64| v[((v.len() as f64 * q) as usize).min(v.len() - 1)];
    println!("{label:>14}: mean {mean:6.3} ms  p50 {:6.3}  p99 {:6.3}  max {:6.3}", p(0.5), p(0.99), v[v.len() - 1]);
    (mean, p(0.99))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let has = |s: &str| args.iter().any(|a| a == s);
    let fx = has("fx");
    let matrix_page = has("matrix");
    let browser = has("browser");
    let drag = has("drag");
    let ppp: f32 = args.iter().find_map(|a| a.strip_prefix("ppp=").and_then(|s| s.parse().ok())).unwrap_or(1.0);
    let n: usize = args.iter().find_map(|a| a.strip_prefix("frames=").and_then(|s| s.parse().ok())).unwrap_or(600);
    let (hover, idle, cpu) = (has("hover"), has("idle"), has("cpu"));
    let mut tables = Tables::new();
    tables.generate();
    let (key, preset) = demo::factory("Wobble Shot");
    let ctl = demo::MapCtl::with_preset(Some(preset));
    let mut patch = preset.patch;
    if fx {
        demo::fx_on(&ctl, &mut patch); // the FX page is measured with its modules running
    }
    demo::demo_routes(&ctl, &mut patch); // eight routings, so rings, live dots and the MATRIX rows are measured
    let mut feed = demo::Feed::new(patch);
    feed.attach(ctl.table.params().clone());
    let mut live = LiveView::new(feed.viz.clone(), Some(feed.ring.clone()));
    live.hero.force_cpu = cpu;
    live.browser.restore(&key);
    if browser {
        let lib = demo::fixture_library(190);
        let t = Instant::now();
        let again = gts::library::Library::open(lib.root(), gts::preset::bank::FACTORY_FILES);
        println!("browser fixture: {} presets; Library::open (factory already written, scan + parse of every file) {:.1} ms", again.entries().len(), t.elapsed().as_secs_f64() * 1e3);
        live.browser = gts::editor::Browser::with_library(lib);
        live.browser.set_filter(gts::library::Filter { category: None, ..Default::default() });
        live.browser.restore(&key);
        live.browser.set_open(true);
    }
    let mut bv = BusView {
        label: "GTS 1".into(),
        sources: vec!["GTS 1 · Mod A".into(), "VSS 1 · Step Gate".into()],
        bound: ["VSS 1 · Step Gate".into(), "GTS 2 · Mod B".into(), "GTS 3 · Mod A".into()],
        activity: [0.92, 0.35, 0.0],
        oversampling: 2,
        page: if matrix_page { Page::Matrix } else if fx { Page::Fx } else { Page::Synth },
        gr_db: 2.4,
        peak_db: -0.3,
        ..Default::default()
    };
    let ctx = egui::Context::default();
    rr_ui::theme::apply(&ctx);
    let (w, h) = (WINDOW.0 as f32, WINDOW.1 as f32);
    let (mut ui_ms, mut tess_ms, mut total_ms) = (vec![], vec![], vec![]);
    let (mut verts, mut idx, mut prims) = (vec![], 0usize, 0usize);
    for f in 0..n + 30 {
        if !idle {
            feed.render(735);
            live.peaks = feed.take_peaks();
        }
        let mut input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(w, h))),
            time: Some(f as f64 / 60.0),
            ..Default::default()
        };
        if let Some(v) = input.viewports.get_mut(&input.viewport_id) {
            v.native_pixels_per_point = Some(ppp);
        }
        if drag {
            // A patch drag in progress with the pointer on a knob (every target ringed, one hot).
            gts::editor::patching::debug_hold_drag(&ctx, gts::matrix::Source::ModB);
            input.events.push(egui::Event::PointerMoved(Pos2::new(112.0, 440.0)));
        }
        if browser {
            // The pointer rests on the list and the wheel runs it up and down; every 150 frames a
            // search is typed, then cleared (Escape), as a user filtering would.
            let t = f as f32 / 60.0;
            input.events.push(egui::Event::PointerMoved(Pos2::new(480.0, 300.0)));
            input.events.push(egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: vec2(0.0, 26.0 * (t * 0.9).cos()), modifiers: Default::default() });
            match f % 150 {
                40 => input.events.push(egui::Event::Text("wub".into())),
                100 => input.events.push(egui::Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() }),
                _ => {}
            }
        }
        if hover {
            // Pointer sweeping across the panel so hover/animation paths run.
            let x = 100.0 + (f as f32 * 7.0) % 800.0;
            input.events.push(egui::Event::PointerMoved(Pos2::new(x, 420.0)));
        }
        let t0 = Instant::now();
        ctx.set_pixels_per_point(ppp);
        let out = ctx.run(input, |ctx| {
            egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
                draw(ui, &ctl, &tables, &mut live, &mut bv);
            });
        });
        let t1 = Instant::now();
        let clipped = ctx.tessellate(out.shapes, ppp);
        let t2 = Instant::now();
        if f >= 30 {
            ui_ms.push((t1 - t0).as_secs_f64() * 1e3);
            tess_ms.push((t2 - t1).as_secs_f64() * 1e3);
            total_ms.push((t2 - t0).as_secs_f64() * 1e3);
            let mut v = 0;
            for c in &clipped {
                if let egui::epaint::Primitive::Mesh(m) = &c.primitive {
                    v += m.vertices.len();
                    if f == 30 {
                        idx += m.indices.len();
                    }
                }
            }
            if f == 30 {
                prims = clipped.len();
            }
            verts.push(v);
        }
    }
    println!(
        "GTS editor {}x{} pts, ppp {ppp}, page {}, hover {hover}, drag {drag}, notes {}, hero {}, {n} frames (after 30 warm-up)",
        w,
        h,
        if browser { "BROWSER (over SYNTH)" } else if matrix_page { "MATRIX" } else if fx { "FX" } else { "SYNTH" },
        !idle,
        if cpu { "CPU fallback" } else { "GL callback" }
    );
    stats("ui pass", ui_ms);
    stats("tessellate", tess_ms);
    stats("ui+tess", total_ms);
    let vmax = verts.iter().max().copied().unwrap_or(0);
    let vmean = verts.iter().sum::<usize>() as f64 / verts.len().max(1) as f64;
    println!("mesh per frame: {prims} clipped primitives (first frame), vertices mean {vmean:.0} max {vmax}, {idx} indices (first frame)");
}
