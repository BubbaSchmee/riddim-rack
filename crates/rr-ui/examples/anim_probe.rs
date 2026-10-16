//! Headless probes for the animation / scaling parts of docs/design/motion-feasibility.md.
//!
//!   cargo run -p rr-ui --release --example anim_probe
//!
//! 1. egui's built-in tweens: progress over time, easing, cost for 500 animated widgets per frame
//! 2. a frame-rate independent exponential smoother kept in `ctx.data` (what a knob "glide" needs)
//! 3. reading a 2048-sample scope window from the lock-free ring while a writer thread hammers it
//! 4. what `set_zoom_factor` / `set_pixels_per_point` do when the host window (egui-baseview) owns
//!    `screen_rect` and the tessellation scale

use eframe::egui::{self, emath::easing, pos2, vec2, Context, Id, Pos2, RawInput, Rect};
use std::time::Instant;

fn frame(ctx: &Context, t: f64, ppp: Option<f32>, mut f: impl FnMut(&Context)) -> egui::FullOutput {
    let mut input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 832.0))), time: Some(t), ..Default::default() };
    if let Some(p) = ppp {
        input.viewports.get_mut(&input.viewport_id).unwrap().native_pixels_per_point = Some(p);
    }
    ctx.run(input, |c| f(c))
}

/// dt-independent exponential approach: reaches 63 % of the gap in `1/rate` seconds.
fn smooth(ctx: &Context, id: Id, target: f32, rate: f32) -> f32 {
    let dt = ctx.input(|i| i.unstable_dt).min(0.1);
    let v = ctx.data_mut(|d| {
        let v = d.get_temp_mut_or_insert_with(id, || target);
        *v += (target - *v) * (1.0 - (-rate * dt).exp());
        *v
    });
    if (v - target).abs() > 1e-3 {
        ctx.request_repaint();
    }
    v
}

fn ms(f: impl FnMut(), n: usize) -> f64 {
    let mut f = f;
    let t = Instant::now();
    for _ in 0..n {
        f();
    }
    t.elapsed().as_secs_f64() * 1e3 / n as f64
}

fn main() {
    let ctx = Context::default();
    println!("animation_time (style default): {:.4} s", ctx.style().animation_time);

    // 1. progress: flip a bool at t=1.0, sample on the 60 Hz grid
    let id = Id::new("a");
    let mut line = String::new();
    for k in 0..14 {
        let t = 1.0 + k as f64 / 60.0;
        let mut v = (0.0, 0.0);
        frame(&ctx, t, None, |c| {
            let on = t >= 1.0 + 1.0 / 60.0;
            v = (c.animate_bool_with_time(id, on, 0.2), c.animate_bool_with_time_and_easing(Id::new("b"), on, 0.2, easing::cubic_out));
        });
        line += &format!("{:.2}/{:.2} ", v.0, v.1);
    }
    println!("animate_bool_with_time(0.2 s) linear/cubic_out per 60 Hz frame: {line}");

    // cost of 500 tweens per frame
    let mut t = 10.0;
    let c1 = ms(|| {
        t += 1.0 / 60.0;
        let on = (t * 2.0) as i64 % 2 == 0;
        frame(&ctx, t, None, |c| {
            for i in 0..500 {
                std::hint::black_box(c.animate_bool_with_time_and_easing(Id::new(("w", i)), on, 0.15, easing::cubic_out));
            }
        });
    }, 600);
    let c0 = ms(|| {
        t += 1.0 / 60.0;
        frame(&ctx, t, None, |_| {});
    }, 600);
    println!("500 x animate_bool_with_time_and_easing per frame: {:.3} ms (empty pass {:.3} ms) => {:.2} us per widget", c1, c0, (c1 - c0) * 1000.0 / 500.0);

    // 2. exponential smoother
    let mut vals = vec![];
    for k in 0..12 {
        let t = 100.0 + k as f64 / 60.0;
        frame(&ctx, t, None, |c| vals.push(smooth(c, Id::new("s"), if k == 0 { 0.0 } else { 1.0 }, 25.0)));
    }
    println!("smoother rate 25/s toward 1.0 over 12 frames @60 Hz: {:?}", vals.iter().map(|v| (v * 100.0).round() / 100.0).collect::<Vec<_>>());
    let c2 = ms(|| {
        t += 1.0 / 60.0;
        let tg = if (t * 2.0) as i64 % 2 == 0 { 1.0 } else { 0.0 };
        frame(&ctx, t, None, |c| {
            for i in 0..500 {
                std::hint::black_box(smooth(c, Id::new(("k", i)), tg, 25.0));
            }
        });
    }, 600);
    println!("500 x ctx.data smoother per frame: {:.3} ms => {:.2} us per widget", c2, (c2 - c0) * 1000.0 / 500.0);

    // 3. ring read
    let (mut w, r) = rr_ui::ring::ring(8192);
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let s2 = stop.clone();
    let th = std::thread::spawn(move || {
        let blk = [0.25f32; 128];
        while !s2.load(std::sync::atomic::Ordering::Relaxed) {
            w.push(&blk);
            std::hint::spin_loop();
        }
    });
    let mut out = [0.0f32; 2048];
    std::thread::sleep(std::time::Duration::from_millis(50));
    let mut ok = 0;
    let rd = ms(|| ok += r.latest(&mut out) as usize, 20000);
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    th.join().unwrap();
    println!("ring.latest(2048) with a saturating writer thread: {:.2} us per read ({} of 20000 reads succeeded)", rd * 1000.0, ok);


    // 5. what one rr-ui widget costs (UI build + tessellation) and how many vertices it emits
    let ctx = Context::default();
    rr_ui::theme::apply(&ctx);
    let wave: Vec<f32> = (0..513).map(|i| (i as f32 / 513.0 * std::f32::consts::TAU).sin()).collect();
    let probe = |name: &str, n: usize, add: &dyn Fn(&mut egui::Ui)| {
        let mut verts = 0;
        let mut total = vec![];
        for f in 0..260 {
            let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 832.0))), time: Some(f as f64 / 60.0), ..Default::default() };
            let t0 = Instant::now();
            let out = ctx.run(input, |c| {
                egui::CentralPanel::default().frame(egui::Frame::NONE).show(c, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for _ in 0..n {
                            add(ui);
                        }
                    });
                });
            });
            let prims = ctx.tessellate(out.shapes, 1.0);
            if f >= 60 {
                total.push(t0.elapsed().as_secs_f64() * 1e3);
            }
            if f == 100 {
                verts = prims.iter().map(|c| if let egui::epaint::Primitive::Mesh(m) = &c.primitive { m.vertices.len() } else { 0 }).sum::<usize>();
            }
        }
        total.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let med = total[total.len() / 2];
        println!("  {name:<28} x{n:<3} median {med:6.3} ms for the frame => {:5.1} us and {:5} vertices per widget", (med - c0) * 1000.0 / n as f64, verts / n);
    };
    println!("widget cost (frame median minus an empty pass of {c0:.3} ms):");
    probe("knob (value arc + readout)", 100, &|ui| {
        rr_ui::widgets::knob(ui, "FREQ", 0.6, 0.5, "2.0 kHz");
    });
    probe("led_toggle", 50, &|ui| {
        rr_ui::widgets::led_toggle(ui, "SYNC", true);
    });
    probe("waveform_display_ex 300x140", 1, &|ui| rr_ui::widgets::waveform_display_ex(ui, &wave, &[&wave, &wave], 0.5, "OSC A", "INDEX 40"));

    // 4. zoom behaviour when the backend (egui-baseview) fixes screen_rect and tessellation ppp
    let ctx = Context::default();
    ctx.set_zoom_factor(1.25);
    let mut rect_pts = Rect::NOTHING;
    let mut screen = Rect::NOTHING;
    frame(&ctx, 0.9, Some(1.0), |_| {}); // first pass of a fresh Context runs a special sizing pass
    let out = frame(&ctx, 1.0, Some(1.0), |c| {
        screen = c.screen_rect();
        egui::CentralPanel::default().show(c, |ui| rect_pts = ui.button("probe").rect);
    });
    println!("set_zoom_factor(1.25), native ppp 1.0: ctx.pixels_per_point() = {}, screen_rect during the pass = {:?} (the backend's value, untouched by zoom), button rect (points) = {:?}", ctx.pixels_per_point(), screen, rect_pts);
    let verts_x_max = ctx.tessellate(out.shapes, 1.0 /* what egui-baseview passes: its own native ppp */).iter().filter_map(|c| if let egui::epaint::Primitive::Mesh(m) = &c.primitive { Some(m.vertices.iter().map(|v| v.pos.x).fold(0.0f32, f32::max)) } else { None }).fold(0.0f32, f32::max);
    println!("  -> mesh vertex positions are in points and the painter multiplies by the baseview ppp (1.0): rightmost vertex {verts_x_max:.1} pt, i.e. the UI is NOT scaled on screen; only the layout/font ppp changed");
    let _ = pos2(0.0, 0.0);
}
