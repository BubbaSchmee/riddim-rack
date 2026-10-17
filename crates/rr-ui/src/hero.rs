//! Hero wavetable display (docs/design/MOTION.md 5.1): the 32 frames of a table stacked in depth,
//! the live frame lit at its fractional depth, a 2D / 3D morph, drag-to-scrub Index and A / B
//! tabs. The frame stack is drawn on the GPU through an `egui_glow` paint callback (GLSL 150, the
//! GL 3.2 core context nih_plug_egui creates); if GL is refused (shader compile / link failure,
//! or `force_cpu`), the same scene is painted with egui shapes at half the points.
//!
//! GL objects are created lazily inside the callback and kept in [`Hero`]. A reopened editor gets
//! a new context: call [`Hero::reset_gl`] from the editor's build closure. Handles are never
//! deleted explicitly (the context dies with the window). The static frame vertices are uploaded
//! only when the frames' `key` changes; per frame only the lit frame (257 points) and a handful of
//! uniforms change.

use crate::paint;
use crate::theme::{font, SRC_A, T};
use crate::widgets::KnobResponse;
use egui::epaint::{Mesh, Vertex, WHITE_UV};
use egui::{pos2, vec2, Align2, Color32, CursorIcon, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui, Vec2};
use egui_glow::glow::{self, HasContext as _};
use std::sync::{Arc, Mutex};

/// Frames in the stack and points per frame (one closed cycle: point 256 repeats point 0).
pub const FRAMES: usize = 32;
pub const POINTS: usize = 257;
const SEGS: usize = POINTS - 1;
/// The projection is specified in this design space (MOTION.md 5.1) and scaled to the rect.
const DESIGN: Vec2 = vec2(568.0, 232.0);
/// Pointer travel for the full Index range when scrubbing, in px.
pub const SCRUB_PX: f32 = 380.0;
/// Fill under each frame: hides the frames behind it.
const HIDE: Color32 = Color32::from_rgb(0x06, 0x08, 0x0a);

/// What the hero shows this frame.
pub struct HeroInput<'a> {
    /// `FRAMES * POINTS` samples, about -1..1 (the caller normalizes the table).
    pub frames: &'a [f32],
    /// Identity of `frames` (e.g. the table index): the GPU copy is rebuilt only when it changes.
    pub key: u64,
    /// Live and base Index, and the modulation range, as 0..1 of the Index range.
    pub live: f32,
    pub base: f32,
    pub range: (f32, f32),
    pub default01: f32,
    /// Amp envelope of the shown oscillator (glow level), 0..1.
    pub level: f32,
    /// Shown oscillator is on.
    pub on: bool,
    /// Tab B is shown.
    pub osc_b: bool,
    pub table: &'a str,
    pub reduced_motion: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HeroResponse {
    /// A tab was clicked: `Some(true)` = B.
    pub tab: Option<bool>,
    /// Index scrub: gesture flags and the new 0..1 value (double-click = reset to default).
    pub index: KnobResponse,
    pub hovered: bool,
}

/// Projection state (MOTION.md 5.1 / prototype-v2 `drawHero`), in design units.
#[derive(Clone, Copy, Debug, Default)]
struct Proj {
    m: f32,
    x0: f32,
    wf: f32,
    dx: f32,
    dy: f32,
    yf: f32,
    amp: f32,
    sh: f32,
    /// Design -> rect points.
    k: Vec2,
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

impl Proj {
    fn new(m: f32, sway: f32, par: Vec2, size: Vec2) -> Self {
        Self {
            m,
            x0: lerp(284.0, 205.0, m) + sway * 6.0 * m,
            wf: lerp(500.0, 330.0, m),
            dx: (190.0 + sway * 16.0 + par.x * 22.0) * m,
            dy: (104.0 + sway * 4.0 - par.y * 10.0) * m,
            yf: lerp(132.0, 176.0, m),
            amp: lerp(62.0, 30.0, m),
            sh: 0.28 * m,
            k: size / DESIGN,
        }
    }
    /// Rect-local point of wave sample `a` at `u` (0..1 across) on depth `t` (0 front, 1 back).
    fn pt(&self, u: f32, a: f32, t: f32) -> Vec2 {
        let sc = 1.0 - self.sh * t;
        vec2(self.x0 + t * self.dx + (u - 0.5) * self.wf * sc, self.yf - t * self.dy - a * self.amp * sc) * self.k
    }
}

/// Everything one frame of the scene needs, shared by the GL and CPU paths.
#[derive(Clone)]
struct Scene {
    p: Proj,
    /// Lit frame position on the depth axis (0..1) and in frames (0..31).
    th: f32,
    gh: f32,
    /// The live frame (crossfade of its two neighbours).
    lit: [f32; POINTS],
    /// Modulation range in frames and whether it is wide enough to tint.
    lo: f32,
    hi: f32,
    tint: bool,
    on: bool,
    /// Glow passes scale (amp envelope) and the core width in px.
    k: f32,
    core: f32,
    /// Wash alpha.
    wash: f32,
}

impl Scene {
    fn frame_alpha(&self, gi: usize) -> (f32, f32, [f32; 3], f32) {
        let m = self.p.m;
        let t = gi as f32 / (FRAMES - 1) as f32;
        let near = (-((gi as f32 - self.gh) / 1.5).powi(2)).exp();
        let a = m.max(0.30 * near * (1.0 - m));
        let fi = t * 127.0;
        let in_r = self.tint && fi >= self.lo - 1.5 && fi <= self.hi + 1.5;
        let kk = if in_r { 0.16 } else { 0.0 };
        let al = ((0.16 + 0.52 * (1.0 - t).powf(1.5) + 0.45 * near) * a * if in_r { 1.1 } else { 1.0 }).min(0.95);
        let col = [lerp(lerp(150.0, 120.0, t), 255.0, kk) / 255.0, lerp(lerp(190.0, 170.0, t), 180.0, kk) / 255.0, lerp(lerp(176.0, 170.0, t), 84.0, kk) / 255.0];
        (a, al, col, lerp(1.15, 0.9, t))
    }
}

fn rgb(c: Color32) -> [f32; 3] {
    [c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0]
}

/// The display. Keep one per editor (it owns the GPU objects).
pub struct Hero {
    gl: Arc<Mutex<GlShared>>,
    /// 1 = 3D, 0 = 2D, smoothed toward `mode3d`.
    mix: f32,
    pub mode3d: bool,
    par: Vec2,
    /// Paint with egui shapes even if GL works (the fallback; also for measurements).
    pub force_cpu: bool,
    /// Collect the GPU time of every hero draw (GL timer query) into [`Hero::gpu_ms`].
    pub measure_gpu: bool,
    /// Value held while scrubbing (the host round trip may lag a frame).
    drag: Option<f32>,
}

impl Default for Hero {
    fn default() -> Self {
        Self { gl: Arc::new(Mutex::new(GlShared::default())), mix: 1.0, mode3d: true, par: Vec2::ZERO, force_cpu: false, measure_gpu: false, drag: None }
    }
}

impl Hero {
    /// Forget the GPU objects (the editor window, and with it the GL context, was reopened).
    pub fn reset_gl(&mut self) {
        let mut g = self.gl.lock().unwrap_or_else(|e| e.into_inner());
        *g = GlShared { measure: g.measure, gpu_ms: std::mem::take(&mut g.gpu_ms), ..Default::default() };
    }

    /// GL state: `None` = not tried yet (no frame painted), `Some(true)` = drawing on the GPU,
    /// `Some(false)` = refused (the CPU fallback is used).
    pub fn gl_ok(&self) -> Option<bool> {
        self.gl.lock().map(|g| g.status).unwrap_or(None)
    }

    /// GPU milliseconds of each completed hero draw since the last call (needs `measure_gpu`).
    pub fn take_gpu_ms(&self) -> Vec<f32> {
        self.gl.lock().map(|mut g| std::mem::take(&mut g.gpu_ms)).unwrap_or_default()
    }

    /// GL renderer string, once the context was seen.
    pub fn gl_info(&self) -> String {
        self.gl.lock().map(|g| g.info.clone()).unwrap_or_default()
    }

    /// Draws the hero into `rect` and handles its gestures.
    pub fn show(&mut self, ui: &mut Ui, rect: Rect, inp: &HeroInput) -> HeroResponse {
        let mut out = HeroResponse::default();
        let (time, dt, shift) = ui.input(|i| (i.time, i.stable_dt.clamp(0.0, 0.05), i.modifiers.shift));
        let resp = ui.interact(rect, ui.id().with("rr-hero"), Sense::click_and_drag());
        out.hovered = resp.hovered();
        if resp.hovered() || resp.dragged() {
            ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
        }
        // Index scrub: per-event delta, Shift = 1/10, double-click = default.
        if resp.double_clicked() {
            out.index = KnobResponse { reset: true, changed: Some(inp.default01), ..Default::default() };
        } else {
            if resp.drag_started() {
                self.drag = Some(inp.base);
                out.index.drag_started = true;
            }
            if let (true, Some(v)) = (resp.dragged(), self.drag) {
                let k = if shift { crate::widgets::FINE } else { 1.0 };
                let n = (v + resp.drag_delta().x / SCRUB_PX * k).clamp(0.0, 1.0);
                if n != v {
                    self.drag = Some(n);
                    out.index.changed = Some(n);
                }
            }
            if resp.drag_stopped() {
                out.index.drag_ended = true;
                self.drag = None;
            }
        }

        // Smoothers (time-based): 2D / 3D tau 75 ms, parallax tau 140 ms; sway 0.42 rad/s.
        let still = inp.reduced_motion;
        let target = if self.mode3d { 1.0 } else { 0.0 };
        self.mix = if still { target } else { target + (self.mix - target) * (-dt / 0.075).exp() };
        if (self.mix - target).abs() < 1e-3 {
            self.mix = target;
        }
        let ptr = if still { None } else { resp.hover_pos() };
        let ptarget = ptr.map(|p| ((p - rect.center()) / (rect.size() * 0.5)).clamp(vec2(-1.0, -1.0), vec2(1.0, 1.0))).unwrap_or(Vec2::ZERO);
        self.par = ptarget + (self.par - ptarget) * (-dt / 0.14).exp();
        let sway = if still { 0.0 } else { (time * 0.42).sin() as f32 };

        let base = self.drag.unwrap_or(inp.base);
        let scene = build_scene(inp, Proj::new(self.mix, sway, self.par, rect.size()), base);
        let p = ui.painter_at(rect);
        glass(&p, rect);
        if scene.p.m < 0.98 {
            graticule(&p, rect, &scene.p);
        }
        let gl_failed = self.gl_ok() == Some(false);
        if self.force_cpu || gl_failed {
            paint_cpu(&p, rect, inp.frames, &scene);
        } else {
            let shared = self.gl.clone();
            {
                let mut g = shared.lock().unwrap_or_else(|e| e.into_inner());
                g.measure = self.measure_gpu;
                g.scene = Some(scene.clone());
                if g.key != Some(inp.key) || g.frames.len() != inp.frames.len() {
                    g.frames = inp.frames.to_vec();
                    g.key = Some(inp.key);
                    g.uploaded = false;
                }
            }
            let cb = egui_glow::CallbackFn::new(move |info, painter| {
                let mut g = shared.lock().unwrap_or_else(|e| e.into_inner());
                g.paint(&info, painter.gl());
            });
            p.add(egui::PaintCallback { rect, callback: Arc::new(cb) });
        }
        ghosts(&p, rect, inp.frames, &scene);
        overlay(ui, &p, rect, inp, &scene, base, self.mix);
        // Tabs and the 2D / 3D switch sit on top of the scrub area.
        let tab = |i: usize| Rect::from_min_size(rect.min + vec2(14.0 + 24.0 * i as f32, 14.0), vec2(24.0, 20.0));
        for (i, name) in ["A", "B"].into_iter().enumerate() {
            let r = ui.interact(tab(i), ui.id().with(("rr-hero-tab", i)), Sense::click());
            let sel = (i == 1) == inp.osc_b;
            seg(&p, tab(i), name, sel, r.hovered());
            if r.clicked() && !sel {
                out.tab = Some(i == 1);
            }
        }
        let mode = |i: usize| Rect::from_min_size(pos2(rect.left() + 14.0 + 28.0 * i as f32, rect.bottom() - 34.0), vec2(28.0, 20.0));
        for (i, name) in ["2D", "3D"].into_iter().enumerate() {
            let r = ui.interact(mode(i), ui.id().with(("rr-hero-mode", i)), Sense::click());
            seg(&p, mode(i), name, (i == 1) == self.mode3d, r.hovered());
            if r.clicked() {
                self.mode3d = i == 1;
            }
        }
        out
    }
}

/// The scene for this frame from the input (pure; shared by both paths).
fn build_scene(inp: &HeroInput, p: Proj, base: f32) -> Scene {
    let th = inp.live.clamp(0.0, 1.0);
    let gh = th * (FRAMES - 1) as f32;
    let mut lit = [0.0f32; POINTS];
    if inp.frames.len() >= FRAMES * POINTS {
        let (g0, f) = (gh.floor() as usize, gh.fract());
        let g1 = (g0 + 1).min(FRAMES - 1);
        for (i, v) in lit.iter_mut().enumerate() {
            *v = lerp(inp.frames[g0 * POINTS + i], inp.frames[g1 * POINTS + i], f);
        }
    }
    let (lo, hi) = (inp.range.0.min(base).clamp(0.0, 1.0) * 127.0, inp.range.1.max(base).clamp(0.0, 1.0) * 127.0);
    let k = if inp.on { 0.55 + 0.45 * (inp.level * 1.15).min(1.0) } else { 0.25 };
    Scene {
        p,
        th,
        gh,
        lit,
        lo,
        hi,
        tint: hi - lo > 4.0,
        on: inp.on,
        k,
        core: 2.1 + 0.4 * (1.0 - p.m),
        wash: 0.10 * (0.4 + 0.6 * (inp.level * 1.2).min(1.0)),
    }
}

// ---- egui chrome -----------------------------------------------------------------------------

fn glass(p: &egui::Painter, r: Rect) {
    paint::highlight_below(p, r, 8.0);
    p.rect_filled(r, 8.0, T.disp_bottom);
    let top = Rect::from_min_size(r.min, vec2(r.width(), r.height() * 0.46));
    paint::rounded_vgradient(p, top, 8.0, Color32::from_rgb(0x0e, 0x12, 0x15), HIDE, false);
}

fn graticule(p: &egui::Painter, rect: Rect, pr: &Proj) {
    let a = 1.0 - pr.m;
    let px = 1.0 / p.ctx().pixels_per_point();
    let (l, r) = (pr.pt(0.0, 0.0, 0.0).x, pr.pt(1.0, 0.0, 0.0).x);
    let faint = Color32::WHITE.gamma_multiply(0.05 * a);
    for i in 0..=8 {
        let x = rect.left() + l + (r - l) * i as f32 / 8.0;
        p.vline(x, (rect.top() + 50.0 * pr.k.y)..=(rect.top() + 206.0 * pr.k.y), Stroke::new(px, faint));
    }
    for v in [-1.0, -0.5, 0.5, 1.0] {
        p.hline((rect.left() + l)..=(rect.left() + r), rect.top() + pr.pt(0.0, v, 0.0).y, Stroke::new(px, faint));
    }
    p.hline((rect.left() + l)..=(rect.left() + r), rect.top() + pr.pt(0.0, 0.0, 0.0).y, Stroke::new(px, Color32::WHITE.gamma_multiply(0.12 * a)));
}

fn seg(p: &egui::Painter, r: Rect, label: &str, on: bool, hot: bool) {
    if on {
        p.rect_filled(r, 3.0, T.a(0.20));
        p.rect_stroke(r, 3.0, paint::hair(p, T.a(0.55)), StrokeKind::Inside);
    } else {
        p.rect_filled(r, 3.0, Color32::BLACK.gamma_multiply(0.45));
        p.rect_stroke(r, 3.0, paint::hair(p, Color32::WHITE.gamma_multiply(if hot { 0.16 } else { 0.08 })), StrokeKind::Inside);
    }
    let col = if on { T.accent } else if hot { T.text } else { T.muted };
    p.text(r.center(), Align2::CENTER_CENTER, label, font::label(11.0), col);
}

fn overlay(ui: &Ui, p: &egui::Painter, rect: Rect, inp: &HeroInput, s: &Scene, base: f32, m: f32) {
    let o = rect.min.to_vec2();
    let at = |u: f32, a: f32, t: f32| (s.p.pt(u, a, t) + o).to_pos2();
    // Fog so the overlay text stays legible.
    paint::vgradient(p, Rect::from_min_size(rect.min, vec2(rect.width(), 70.0)), T.disp_bottom.gamma_multiply(0.85), Color32::TRANSPARENT);
    paint::vgradient(p, Rect::from_min_max(pos2(rect.left(), rect.bottom() - 38.0), rect.max), Color32::TRANSPARENT, T.disp_bottom.gamma_multiply(0.8));

    // Depth ruler along the front-left edge (3D).
    if m > 0.02 {
        let px = 1.0 / ui.ctx().pixels_per_point();
        let rx = |t: f32| at(0.0, 0.0, t) - vec2(12.0 * (1.0 - 0.5 * t), 0.0);
        let pts: Vec<Pos2> = (0..=20).map(|i| rx(i as f32 / 20.0)).collect();
        p.add(Shape::line(pts, paint::hair(p, Color32::WHITE.gamma_multiply(0.22 * m))));
        for q in 0..=4 {
            let c = rx(q as f32 / 4.0);
            p.hline((c.x - 3.0)..=(c.x + 3.0), c.y, Stroke::new(px, Color32::WHITE.gamma_multiply(0.22 * m)));
        }
        if s.tint {
            p.line_segment([rx(s.lo / 127.0), rx(s.hi / 127.0)], paint::st(2.0, SRC_A.gamma_multiply(0.7 * m)));
        }
        let b = rx(base);
        p.add(Shape::convex_polygon(vec![b - vec2(5.0, 0.0), b + vec2(-10.0, -3.5), b + vec2(-10.0, 3.5)], T.muted.gamma_multiply(m), Stroke::NONE));
        let l = rx(s.th) + vec2(1.0, 0.0);
        p.add(Shape::convex_polygon(vec![l - vec2(1.0, 0.0), l + vec2(-8.0, -4.0), l + vec2(-8.0, 4.0)], T.accent.gamma_multiply(m), Stroke::NONE));
        p.hline(l.x..=at(0.0, 0.0, s.th).x, l.y, Stroke::new(px, T.a(0.35 * m)));
    }

    // Captions: tabs left, table name, live index right.
    let osc = if inp.osc_b { "OSC B" } else { "OSC A" };
    paint::tracked(p, rect.min + vec2(76.0, 19.0), Align2::LEFT_CENTER, &format!("{osc} \u{b7} TABLE"), font::label(11.0), T.disp_caption, 1.0);
    p.text(rect.min + vec2(76.0, 40.0), Align2::LEFT_CENTER, inp.table, font::strong(17.0), T.text);
    paint::tracked(p, pos2(rect.right() - 18.0, rect.top() + 19.0), Align2::RIGHT_CENTER, "INDEX", font::label(11.0), T.disp_caption, 1.0);
    let idx = format!("{:.1}", inp.live * 127.0);
    let ip = pos2(rect.right() - 18.0, rect.top() + 46.0);
    // Soft glow: the readout smeared 1 px around at low alpha, then the crisp text.
    for d in [vec2(-1.0, 0.0), vec2(1.0, 0.0), vec2(0.0, -1.0), vec2(0.0, 1.0)] {
        p.text(ip + d, Align2::RIGHT_CENTER, &idx, font::mono(26.0), T.a(0.12));
    }
    p.text(ip, Align2::RIGHT_CENTER, &idx, font::mono(26.0), T.accent);
    p.text(ip + vec2(0.0, 22.0), Align2::RIGHT_CENTER, format!("base {:.1}", base * 127.0), font::mono(11.0), T.disp_caption);
    let hint = if s.tint { "\u{25aa} MOD A RANGE  \u{b7}  DRAG TO SCRUB INDEX" } else { "DRAG TO SCRUB INDEX" };
    let hr = paint::tracked(p, pos2(rect.right() - 18.0, rect.bottom() - 24.0), Align2::RIGHT_CENTER, hint, font::label(11.0), T.disp_caption, 0.6);
    if s.tint {
        p.rect_filled(Rect::from_center_size(pos2(hr.left() + 3.5, hr.center().y), vec2(6.0, 6.0)), 1.0, SRC_A);
    }
    if !inp.on {
        p.rect_filled(Rect::from_min_max(rect.min + vec2(0.0, 44.0), rect.max), 0.0, Color32::from_rgba_unmultiplied(5, 7, 10, 140));
        p.text(rect.center() + vec2(0.0, 4.0), Align2::CENTER_CENTER, format!("{osc} IS OFF"), font::strong(12.0), T.muted);
    }
    // Glass edge: inner rim, black outline.
    p.rect_stroke(rect.shrink(1.0), 7.0, paint::hair(p, Color32::WHITE.gamma_multiply(0.035)), StrokeKind::Inside);
    p.rect_stroke(rect, 8.0, paint::hair(p, Color32::BLACK), StrokeKind::Inside);
}

// ---- CPU fallback ----------------------------------------------------------------------------

/// The scene with egui shapes: 129 points per frame, hairlines, hidden-line fills, three-pass lit frame.
/// 2D view: the two modulation extremes as dashed `src_a` ghosts (nearest stored frames), fading
/// out as the view opens into 3D. Drawn with egui (dashes follow the path length, so steep edges
/// dash like the rest); both paint paths use it.
fn ghosts(p: &egui::Painter, rect: Rect, frames: &[f32], s: &Scene) {
    let a = 0.34 * (1.0 - s.p.m);
    if s.p.m >= 0.98 || !s.tint || frames.len() < FRAMES * POINTS || a < 0.01 {
        return;
    }
    let o = rect.min.to_vec2();
    for f in [s.lo, s.hi] {
        let gi = ((f / 127.0 * (FRAMES - 1) as f32).round() as usize).min(FRAMES - 1);
        if (gi as f32 - s.gh).abs() < 0.5 {
            continue;
        }
        let w = &frames[gi * POINTS..(gi + 1) * POINTS];
        let pts: Vec<Pos2> = (0..POINTS).step_by(2).map(|i| (s.p.pt(i as f32 / SEGS as f32, w[i].clamp(-1.25, 1.25), 0.0) + o).to_pos2()).collect();
        p.extend(Shape::dashed_line(&pts, Stroke::new(1.2_f32, SRC_A.gamma_multiply(a)), 5.0, 3.5));
    }
}

fn paint_cpu(p: &egui::Painter, rect: Rect, frames: &[f32], s: &Scene) {
    if frames.len() < FRAMES * POINTS {
        return;
    }
    const STRIDE: usize = 2;
    let o = rect.min.to_vec2();
    let bottom = rect.bottom() + 4.0;
    let path = |w: &[f32], t: f32| -> Vec<Pos2> {
        (0..POINTS).step_by(STRIDE).map(|i| (s.p.pt(i as f32 / SEGS as f32, w[i].clamp(-1.25, 1.25), t) + o).to_pos2()).collect()
    };
    let fill = |pts: &[Pos2], col: Color32| {
        let mut mesh = Mesh::default();
        for (i, q) in pts.iter().enumerate() {
            mesh.vertices.push(Vertex { pos: *q, uv: WHITE_UV, color: col });
            mesh.vertices.push(Vertex { pos: pos2(q.x, bottom), uv: WHITE_UV, color: col });
            if i > 0 {
                let b = (i as u32 - 1) * 2;
                mesh.indices.extend_from_slice(&[b, b + 1, b + 2, b + 2, b + 1, b + 3]);
            }
        }
        p.add(Shape::mesh(mesh));
    };
    let m = s.p.m;
    let mut lit_done = false;
    let draw_lit = |p: &egui::Painter| {
        let pts = path(&s.lit, s.th);
        fill(&pts, HIDE.gamma_multiply(m));
        fill(&pts, T.a(0.12));
        for (w, a) in [(s.core * 6.0, 0.07 * s.k), (s.core * 3.0, 0.17 * s.k)] {
            p.add(Shape::line(pts.clone(), Stroke::new(w, T.a(a))));
        }
        p.add(Shape::line(pts, Stroke::new(s.core, T.accent)));
    };
    for gi in (0..FRAMES).rev() {
        let t = gi as f32 / (FRAMES - 1) as f32;
        if !lit_done && t <= s.th + 1e-6 {
            draw_lit(p);
            lit_done = true;
        }
        let (a, al, col, w) = s.frame_alpha(gi);
        if a < 0.01 || m < 0.02 {
            continue;
        }
        let pts = path(&frames[gi * POINTS..(gi + 1) * POINTS], t);
        fill(&pts, HIDE.gamma_multiply((a * 1.2).min(1.0)));
        let c = Color32::from_rgba_unmultiplied((col[0] * 255.0) as u8, (col[1] * 255.0) as u8, (col[2] * 255.0) as u8, (al * 255.0) as u8);
        p.add(Shape::line(pts, Stroke::new(w, c)));
    }
    if !lit_done {
        draw_lit(p);
    }
}

// ---- GL path ---------------------------------------------------------------------------------

#[derive(Default)]
struct GlShared {
    status: Option<bool>,
    gpu: Option<Gpu>,
    key: Option<u64>,
    frames: Vec<f32>,
    uploaded: bool,
    scene: Option<Scene>,
    measure: bool,
    gpu_ms: Vec<f32>,
    info: String,
}

struct Prog {
    p: glow::Program,
    /// Uniforms (absent ones are `None` and ignored by GL).
    u: Vec<Option<glow::UniformLocation>>,
}

struct Gpu {
    line: Prog,
    fill: Prog,
    flat: Prog,
    line_vao: glow::VertexArray,
    lit_vao: glow::VertexArray,
    lit_vbo: glow::Buffer,
    fill_vao: glow::VertexArray,
    fill_vbo: glow::Buffer,
    litfill_vao: glow::VertexArray,
    litfill_vbo: glow::Buffer,
    flat_vao: glow::VertexArray,
    flat_vbo: glow::Buffer,
    line_vbo: glow::Buffer,
    query: Option<glow::Query>,
    pending: bool,
}

// Uniform slots shared by the line and fill programs.
const U_PA: usize = 0; // x0, wf, dx, dy
const U_PB: usize = 1; // yf, amp, sh, t
const U_K: usize = 2; // design -> points
const U_SIZE: usize = 3; // rect size in points
const U_COL: usize = 4; // rgb, alpha
const U_HALFW: usize = 5; // half width in points
const U_PX: usize = 6; // points per physical px
const U_GRAD: usize = 7; // fill gradient: y top, y bottom (points), alpha at top, on
const UNIFORMS: [&str; 8] = ["u_pa", "u_pb", "u_k", "u_size", "u_col", "u_halfw", "u_px", "u_grad"];

const PROJ: &str = r#"
uniform vec4 u_pa; uniform vec4 u_pb; uniform vec2 u_k; uniform vec2 u_size;
vec2 proj(vec2 ua) {
    float t = u_pb.w; float sc = 1.0 - u_pb.z * t;
    return vec2(u_pa.x + t * u_pa.z + (ua.x - 0.5) * u_pa.y * sc, u_pb.x - t * u_pa.w - clamp(ua.y, -1.25, 1.25) * u_pb.y * sc) * u_k;
}
vec4 ndc(vec2 p) { return vec4(p.x / u_size.x * 2.0 - 1.0, 1.0 - p.y / u_size.y * 2.0, 0.0, 1.0); }
"#;

const DITHER: &str = r#"
float ign(vec2 n) { return fract(52.9829189 * fract(0.06711056 * n.x + 0.00583715 * n.y)); }
"#;

const LINE_VS: &str = r#"
in vec2 a_p0; in vec2 a_p1; in vec2 a_corner;
uniform float u_halfw; uniform float u_px;
out float v_d;
void main() {
    vec2 s0 = proj(a_p0); vec2 s1 = proj(a_p1);
    vec2 dir = s1 - s0;
    dir = dot(dir, dir) > 1e-8 ? normalize(dir) : vec2(1.0, 0.0);
    float reach = u_halfw + u_px;
    vec2 p = mix(s0, s1, a_corner.x) + vec2(-dir.y, dir.x) * a_corner.y * reach;
    v_d = a_corner.y * reach / u_px;
    gl_Position = ndc(p);
}"#;

const LINE_FS: &str = r#"
in float v_d;
uniform vec4 u_col; uniform float u_halfw; uniform float u_px;
out vec4 f;
void main() {
    float hw = u_halfw / u_px;
    float a = clamp(hw + 0.5 - abs(v_d), 0.0, 1.0) * u_col.a;
    vec3 rgb = u_col.rgb * a + (ign(gl_FragCoord.xy) - 0.5) * 0.95 / 255.0 * a;
    f = vec4(rgb, a);
}"#;

const FILL_VS: &str = r#"
in vec3 a_uak;
out float v_y;
void main() {
    vec2 p = proj(a_uak.xy);
    if (a_uak.z > 0.5) p.y = u_size.y + 4.0;
    v_y = p.y;
    gl_Position = ndc(p);
}"#;

const FILL_FS: &str = r#"
in float v_y;
uniform vec4 u_col; uniform vec4 u_grad;
out vec4 f;
void main() {
    float a = u_col.a;
    if (u_grad.w > 0.5) a *= u_grad.z * clamp((u_grad.y - v_y) / max(u_grad.y - u_grad.x, 1.0), 0.0, 1.0);
    vec3 rgb = u_col.rgb * a + (ign(gl_FragCoord.xy) - 0.5) * 0.95 / 255.0 * a;
    f = vec4(rgb, a);
}"#;

const FLAT_VS: &str = r#"
in vec2 a_pos; in vec4 a_col;
uniform vec2 u_size;
out vec4 v_col;
void main() { v_col = a_col; gl_Position = vec4(a_pos.x / u_size.x * 2.0 - 1.0, 1.0 - a_pos.y / u_size.y * 2.0, 0.0, 1.0); }"#;

const FLAT_FS: &str = r#"
in vec4 v_col;
out vec4 f;
void main() { f = v_col + vec4(vec3((ign(gl_FragCoord.xy) - 0.5) * 0.95 / 255.0 * v_col.a), 0.0); }"#;

unsafe fn compile(gl: &glow::Context, vs: &str, fs: &str, attribs: &[&str]) -> Result<Prog, String> {
    let p = gl.create_program()?;
    let mut shaders = vec![];
    for (kind, src) in [(glow::VERTEX_SHADER, vs), (glow::FRAGMENT_SHADER, fs)] {
        let s = gl.create_shader(kind)?;
        gl.shader_source(s, src);
        gl.compile_shader(s);
        if !gl.get_shader_compile_status(s) {
            return Err(gl.get_shader_info_log(s));
        }
        gl.attach_shader(p, s);
        shaders.push(s);
    }
    for (i, n) in attribs.iter().enumerate() {
        gl.bind_attrib_location(p, i as u32, n);
    }
    gl.link_program(p);
    for s in shaders {
        gl.detach_shader(p, s);
        gl.delete_shader(s);
    }
    if !gl.get_program_link_status(p) {
        return Err(gl.get_program_info_log(p));
    }
    let u = UNIFORMS.iter().map(|n| gl.get_uniform_location(p, n)).collect();
    Ok(Prog { p, u })
}

fn bytes<T>(v: &[T]) -> &[u8] {
    // SAFETY: plain f32 / u16 slices, read as bytes.
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

/// Segment quads in wave space: per vertex p0 (u, a), p1 (u, a), corner (along 0/1, side -1/1).
fn line_quads(out: &mut Vec<f32>, w: &[f32]) {
    for i in 0..SEGS {
        let (u0, u1) = (i as f32 / SEGS as f32, (i + 1) as f32 / SEGS as f32);
        for (cx, cy) in [(0.0, -1.0), (0.0, 1.0), (1.0, -1.0), (1.0, 1.0)] {
            out.extend_from_slice(&[u0, w[i], u1, w[i + 1], cx, cy]);
        }
    }
}

/// Fill strip in wave space: per point (u, a, 0) then (u, a, 1 = bottom edge).
fn fill_strip(out: &mut Vec<f32>, w: &[f32]) {
    for (i, a) in w.iter().enumerate().take(POINTS) {
        let u = i as f32 / SEGS as f32;
        out.extend_from_slice(&[u, *a, 0.0, u, *a, 1.0]);
    }
}

impl GlShared {
    unsafe fn init(&mut self, gl: &glow::Context) -> Result<(), String> {
        let hdr = "#version 150\n";
        let line = compile(gl, &format!("{hdr}{PROJ}{LINE_VS}"), &format!("{hdr}{DITHER}{LINE_FS}"), &["a_p0", "a_p1", "a_corner"])?;
        let fill = compile(gl, &format!("{hdr}{PROJ}{FILL_VS}"), &format!("{hdr}{DITHER}{FILL_FS}"), &["a_uak"])?;
        let flat = compile(gl, &format!("{hdr}{FLAT_VS}"), &format!("{hdr}{DITHER}{FLAT_FS}"), &["a_pos", "a_col"])?;
        let line_attribs = |vbo| {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            for (loc, off) in [(0u32, 0i32), (1, 8), (2, 16)] {
                gl.enable_vertex_attrib_array(loc);
                gl.vertex_attrib_pointer_f32(loc, 2, glow::FLOAT, false, 24, off);
            }
        };
        // Static index buffer for segment quads (part of each line VAO's state).
        let mut idx: Vec<u16> = Vec::with_capacity(FRAMES * SEGS * 6);
        for k in 0..(FRAMES * SEGS) as u16 {
            let b = k * 4;
            idx.extend_from_slice(&[b, b + 1, b + 2, b + 2, b + 1, b + 3]);
        }
        let ibo = gl.create_buffer()?;
        let mk_line_vao = |vbo| -> Result<glow::VertexArray, String> {
            let vao = gl.create_vertex_array()?;
            gl.bind_vertex_array(Some(vao));
            line_attribs(vbo);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ibo));
            Ok(vao)
        };
        let line_vbo = gl.create_buffer()?;
        let line_vao = mk_line_vao(line_vbo)?;
        gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, bytes(&idx), glow::STATIC_DRAW);
        let lit_vbo = gl.create_buffer()?;
        let lit_vao = mk_line_vao(lit_vbo)?;
        let mk_fill_vao = |vbo| -> Result<glow::VertexArray, String> {
            let vao = gl.create_vertex_array()?;
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 12, 0);
            Ok(vao)
        };
        let fill_vbo = gl.create_buffer()?;
        let fill_vao = mk_fill_vao(fill_vbo)?;
        let litfill_vbo = gl.create_buffer()?;
        let litfill_vao = mk_fill_vao(litfill_vbo)?;
        let flat_vbo = gl.create_buffer()?;
        let flat_vao = gl.create_vertex_array()?;
        gl.bind_vertex_array(Some(flat_vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(flat_vbo));
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 24, 0);
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_f32(1, 4, glow::FLOAT, false, 24, 8);
        gl.bind_vertex_array(None);
        gl.bind_buffer(glow::ARRAY_BUFFER, None);

        let v = gl.version();
        let has_q = gl.supported_extensions().contains("GL_ARB_timer_query") || v.major > 3 || (v.major == 3 && v.minor >= 3);
        let query = if has_q { gl.create_query().ok() } else { None };
        self.info = format!("{} | GL {} | timer_query {}", gl.get_parameter_string(glow::RENDERER), gl.get_parameter_string(glow::VERSION), query.is_some());
        self.gpu = Some(Gpu { line, fill, flat, line_vao, lit_vao, lit_vbo, fill_vao, fill_vbo, litfill_vao, litfill_vbo, flat_vao, flat_vbo, line_vbo, query, pending: false });
        self.uploaded = false;
        Ok(())
    }

    fn paint(&mut self, info: &egui::PaintCallbackInfo, gl: &glow::Context) {
        if self.status == Some(false) {
            return;
        }
        if self.gpu.is_none() {
            if let Err(e) = unsafe { self.init(gl) } {
                eprintln!("rr-ui hero: GL refused, using the CPU fallback: {e}");
                self.status = Some(false);
                return;
            }
        }
        let Some(s) = self.scene.clone() else { return };
        if self.frames.len() < FRAMES * POINTS {
            return;
        }
        let g = self.gpu.as_ref().unwrap();
        unsafe {
            if !self.uploaded {
                let mut lines = Vec::with_capacity(FRAMES * SEGS * 24);
                let mut fills = Vec::with_capacity(FRAMES * POINTS * 6);
                for f in self.frames.chunks(POINTS).take(FRAMES) {
                    line_quads(&mut lines, f);
                    fill_strip(&mut fills, f);
                }
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(g.line_vbo));
                gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes(&lines), glow::STATIC_DRAW);
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(g.fill_vbo));
                gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes(&fills), glow::STATIC_DRAW);
                self.uploaded = true;
            }
            let mut started = false;
            let mut gpu = self.gpu.take().unwrap();
            if let (true, Some(q)) = (self.measure, gpu.query) {
                if gpu.pending && gl.get_query_parameter_u32(q, glow::QUERY_RESULT_AVAILABLE) != 0 {
                    self.gpu_ms.push(gl.get_query_parameter_u32(q, glow::QUERY_RESULT) as f32 / 1e6);
                    gpu.pending = false;
                }
                if !gpu.pending {
                    gl.begin_query(glow::TIME_ELAPSED, q);
                    started = true;
                }
            }
            draw_gl(gl, &gpu, info, &s);
            if started {
                gl.end_query(glow::TIME_ELAPSED);
                gpu.pending = true;
            }
            self.gpu = Some(gpu);
        }
        self.status = Some(true);
    }
}

/// Premultiplied flat-colour triangles, rect-local points: (x, y, r, g, b, a) per vertex.
fn flat_tri(out: &mut Vec<f32>, p: [Vec2; 3], c: [[f32; 4]; 3]) {
    for (q, c) in p.iter().zip(c) {
        out.extend_from_slice(&[q.x, q.y, c[0] * c[3], c[1] * c[3], c[2] * c[3], c[3]]);
    }
}

fn flat_quad(out: &mut Vec<f32>, r: Rect, top: [f32; 4], bottom: [f32; 4]) {
    let (a, b, c, d) = (r.left_top().to_vec2(), r.right_top().to_vec2(), r.right_bottom().to_vec2(), r.left_bottom().to_vec2());
    flat_tri(out, [a, b, c], [top, top, bottom]);
    flat_tri(out, [a, c, d], [top, bottom, bottom]);
}

unsafe fn draw_gl(gl: &glow::Context, g: &Gpu, info: &egui::PaintCallbackInfo, s: &Scene) {
    let size = info.viewport.size();
    let px = 1.0 / info.pixels_per_point;
    let p = &s.p;
    let m = p.m;
    let acc = rgb(T.accent);

    // Lit frame geometry (per frame: 257 points) and its plane + wash.
    let mut lit = Vec::with_capacity(SEGS * 24);
    line_quads(&mut lit, &s.lit);
    let mut litfill = Vec::with_capacity(POINTS * 6);
    fill_strip(&mut litfill, &s.lit);
    let mut flat = Vec::with_capacity(64 * 18);
    let plane = Rect::from_min_max(p.pt(0.0, 1.25, s.th).to_pos2(), pos2(p.pt(1.0, 0.0, s.th).x, p.pt(0.0, -1.25, s.th).y));
    let mut plane_n = 0;
    if m > 0.02 {
        let mid = plane.center().y;
        let a = [acc[0], acc[1], acc[2], 0.0];
        let b = [acc[0], acc[1], acc[2], 0.07 * m];
        flat_quad(&mut flat, Rect::from_min_max(plane.min, pos2(plane.right(), mid)), a, b);
        flat_quad(&mut flat, Rect::from_min_max(pos2(plane.left(), mid), plane.max), b, a);
        let e = [acc[0], acc[1], acc[2], 0.22 * m];
        for x in [plane.left(), plane.right()] {
            let x = (x / px).round() * px;
            flat_quad(&mut flat, Rect::from_min_max(pos2(x, plane.top()), pos2(x + px, plane.bottom())), e, e);
        }
        plane_n = flat.len() / 6;
    }
    // Radial wash behind the lit frame: a 48-triangle fan, centre alpha to 0 at 190 px.
    let c = p.pt(0.5, 0.0, s.th);
    let (ci, co) = ([acc[0], acc[1], acc[2], s.wash], [acc[0], acc[1], acc[2], 0.0]);
    for i in 0..48 {
        let (a0, a1) = (i as f32 / 48.0 * std::f32::consts::TAU, (i + 1) as f32 / 48.0 * std::f32::consts::TAU);
        flat_tri(&mut flat, [c, c + vec2(a0.cos(), a0.sin()) * 190.0, c + vec2(a1.cos(), a1.sin()) * 190.0], [ci, co, co]);
    }
    let flat_n = flat.len() / 6;

    gl.bind_buffer(glow::ARRAY_BUFFER, Some(g.lit_vbo));
    gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes(&lit), glow::STREAM_DRAW);
    gl.bind_buffer(glow::ARRAY_BUFFER, Some(g.litfill_vbo));
    gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes(&litfill), glow::STREAM_DRAW);
    gl.bind_buffer(glow::ARRAY_BUFFER, Some(g.flat_vbo));
    gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes(&flat), glow::STREAM_DRAW);

    let set_common = |pr: &Prog, t: f32| {
        gl.use_program(Some(pr.p));
        gl.uniform_4_f32(pr.u[U_PA].as_ref(), p.x0, p.wf, p.dx, p.dy);
        gl.uniform_4_f32(pr.u[U_PB].as_ref(), p.yf, p.amp, p.sh, t);
        gl.uniform_2_f32(pr.u[U_K].as_ref(), p.k.x, p.k.y);
        gl.uniform_2_f32(pr.u[U_SIZE].as_ref(), size.x, size.y);
        gl.uniform_1_f32(pr.u[U_PX].as_ref(), px);
    };
    let line = |vao, first_quad: usize, t: f32, col: [f32; 3], alpha: f32, width_px: f32| {
        set_common(&g.line, t);
        gl.uniform_4_f32(g.line.u[U_COL].as_ref(), col[0], col[1], col[2], alpha);
        gl.uniform_1_f32(g.line.u[U_HALFW].as_ref(), width_px * 0.5 * px);
        gl.bind_vertex_array(Some(vao));
        gl.draw_elements(glow::TRIANGLES, (SEGS * 6) as i32, glow::UNSIGNED_SHORT, (first_quad * 6 * 2) as i32);
    };
    let fill = |vao, frame: usize, t: f32, col: [f32; 3], alpha: f32, grad: [f32; 4]| {
        set_common(&g.fill, t);
        gl.uniform_4_f32(g.fill.u[U_COL].as_ref(), col[0], col[1], col[2], alpha);
        gl.uniform_4_f32(g.fill.u[U_GRAD].as_ref(), grad[0], grad[1], grad[2], grad[3]);
        gl.bind_vertex_array(Some(vao));
        gl.draw_arrays(glow::TRIANGLE_STRIP, (frame * POINTS * 2) as i32, (POINTS * 2) as i32);
    };
    let flat_draw = |first: usize, n: usize| {
        if n == 0 {
            return;
        }
        gl.use_program(Some(g.flat.p));
        gl.uniform_2_f32(g.flat.u[U_SIZE].as_ref(), size.x, size.y);
        gl.bind_vertex_array(Some(g.flat_vao));
        gl.draw_arrays(glow::TRIANGLES, first as i32, n as i32);
    };
    let hide = rgb(HIDE);
    let draw_lit = || {
        flat_draw(0, plane_n);
        flat_draw(plane_n, flat_n - plane_n);
        fill(g.litfill_vao, 0, s.th, hide, m, [0.0; 4]);
        let (gy0, gy1) = (p.pt(0.0, 1.2, s.th).y, p.pt(0.0, -1.2, s.th).y + 30.0);
        fill(g.litfill_vao, 0, s.th, acc, 1.0, [gy0, gy1, 0.26, 1.0]);
        for (w, a) in [(s.core * 6.0, 0.07 * s.k), (s.core * 3.0, 0.17 * s.k), (s.core, 1.0)] {
            line(g.lit_vao, 0, s.th, acc, a, w);
        }
    };

    // Frames back to front, the lit frame spliced in at its depth.
    let mut lit_done = false;
    for gi in (0..FRAMES).rev() {
        let t = gi as f32 / (FRAMES - 1) as f32;
        if !lit_done && t <= s.th + 1e-6 {
            draw_lit();
            lit_done = true;
        }
        let (a, al, col, w) = s.frame_alpha(gi);
        if a < 0.01 || m < 0.02 {
            continue;
        }
        fill(g.fill_vao, gi, t, hide, (a * 1.2).min(1.0), [0.0; 4]);
        line(g.line_vao, gi * SEGS, t, col, al, w);
    }
    if !lit_done {
        draw_lit();
    }
    // X-ray: the lit frame's whole shape over the nearer frames.
    if m > 0.05 && s.on {
        line(g.lit_vao, 0, s.th, acc, 0.30 * m, 1.3);
    }
    gl.bind_vertex_array(None);
    gl.use_program(None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_matches_the_spec_numbers() {
        // 3D: front frame centre x 205, width 330, baseline 176, amplitude 30; back frame +190 x, -104 y, 28 % narrower.
        let p = Proj::new(1.0, 0.0, Vec2::ZERO, DESIGN);
        assert_eq!(p.pt(0.5, 0.0, 0.0), vec2(205.0, 176.0));
        assert_eq!(p.pt(1.0, 0.0, 0.0).x - p.pt(0.0, 0.0, 0.0).x, 330.0);
        assert_eq!(p.pt(0.5, 1.0, 0.0).y, 146.0);
        assert_eq!(p.pt(0.5, 0.0, 1.0), vec2(395.0, 72.0));
        assert!((p.pt(1.0, 0.0, 1.0).x - p.pt(0.0, 0.0, 1.0).x - 330.0 * 0.72).abs() < 1e-3);
        // 2D: the lit frame fills 500 x 124 (amplitude 62 each way).
        let q = Proj::new(0.0, 0.7, vec2(1.0, 1.0), DESIGN);
        assert_eq!(q.pt(1.0, 0.0, 0.7).x - q.pt(0.0, 0.0, 0.7).x, 500.0);
        assert_eq!(q.pt(0.0, -1.0, 0.3).y - q.pt(0.0, 1.0, 0.3).y, 124.0);
    }

    #[test]
    fn lit_frame_crossfades_its_neighbours() {
        let mut frames = vec![0.0f32; FRAMES * POINTS];
        for g in 0..FRAMES {
            frames[g * POINTS..(g + 1) * POINTS].fill(g as f32);
        }
        let inp = HeroInput { frames: &frames, key: 0, live: 10.5 / 31.0, base: 0.0, range: (0.0, 0.0), default01: 0.0, level: 1.0, on: true, osc_b: false, table: "", reduced_motion: true };
        let s = build_scene(&inp, Proj::new(1.0, 0.0, Vec2::ZERO, DESIGN), 0.0);
        assert!(s.lit.iter().all(|&v| (v - 10.5).abs() < 1e-4));
        assert!(!s.tint);
    }

    /// Scrubbing: dragging 190 px right from Index 0.25 lands on 0.75; the hero draws headless.
    #[test]
    fn drag_scrubs_index() {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx);
        let frames = vec![0.0f32; FRAMES * POINTS];
        let mut hero = Hero { force_cpu: true, ..Default::default() };
        let mut got = None;
        let mut base = 0.25;
        let mut frame = |ev: Vec<egui::Event>, t: f64, got: &mut Option<f32>, base: &mut f32| {
            let raw = egui::RawInput { time: Some(t), events: ev, screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(600.0, 260.0))), ..Default::default() };
            let _ = ctx.run(raw, |ctx| {
                egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
                    let inp = HeroInput { frames: &frames, key: 1, live: *base, base: *base, range: (0.0, 0.0), default01: 0.3, level: 0.0, on: true, osc_b: false, table: "Vowel", reduced_motion: false };
                    let r = hero.show(ui, Rect::from_min_size(pos2(10.0, 10.0), DESIGN), &inp);
                    if let Some(v) = r.index.changed {
                        *got = Some(v);
                        *base = v;
                    }
                });
            });
        };
        let at = pos2(300.0, 150.0);
        let btn = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
        frame(vec![egui::Event::PointerMoved(at)], 0.0, &mut got, &mut base);
        frame(vec![btn(at, true)], 0.02, &mut got, &mut base);
        for i in 1..=10 {
            frame(vec![egui::Event::PointerMoved(at + vec2(19.0 * i as f32, 0.0))], 0.02 + 0.016 * i as f64, &mut got, &mut base);
        }
        frame(vec![btn(at + vec2(190.0, 0.0), false)], 0.3, &mut got, &mut base);
        let v = got.expect("scrub reported a value");
        assert!((v - 0.75).abs() < 0.03, "scrub ended at {v}");
    }
}
