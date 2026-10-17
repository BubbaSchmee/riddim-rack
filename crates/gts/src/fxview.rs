//! FX page displays (docs/design/MOTION.md 2.2 and 4): one glass per module, drawn from the
//! module's own parameters plus the live record the audio thread publishes (`viz::xf`, the side
//! ring). Painters take plain values; [`FxLive`] holds the GUI-side ballistics and histories and is
//! updated once per frame from the latest record. Nothing here runs DSP: the transfer curve and
//! the EQ response call the same pure functions the DSP is built from.

use crate::dsp::fx::chorus::OFFSET;
use crate::dsp::fx::distortion::{transfer, DistType};
use crate::dsp::fx::eq::{response_db, EqParams};
use crate::viz::{xf, VizFrame};
use nih_plug_egui::egui::epaint::{Mesh, Vertex, WHITE_UV};
use nih_plug_egui::egui::{pos2, vec2, Align2, Color32, Painter, Pos2, Rect, Shape, Stroke};
use rr_ui::paint;
use rr_ui::scope::{glow_line, small_glass, spectrum};
use rr_ui::theme::{font, T};

/// Delay-input history: one value per 10 ms, 10 s deep (covers the longest synced delay).
const HIST_N: usize = 1024;
const HIST_STEP: f64 = 0.01;
/// Distortion dot trail, frames.
pub const TRAIL: usize = 6;

fn db(x: f32) -> f32 {
    20.0 * x.max(1e-6).log10()
}

/// GUI-side state of the FX displays: smoothed meters and the histories the delay and reverb read.
pub struct FxLive {
    /// Distortion shaper input, signed, newest first.
    pub dist_trail: [f32; TRAIL],
    /// Multiband detector level (dB, fall 40 dB/s) and gain (dB) per band.
    pub mb_level: [f32; 3],
    pub mb_gain: [f32; 3],
    /// Chorus LFO phase, turns.
    pub cho_phase: f32,
    /// Output left / right over the scope window (192 points, -1.2..1.2), from the mid and side rings.
    pub dim_l: Vec<f32>,
    pub dim_r: Vec<f32>,
    dly_hist: Vec<f32>,
    dly_n: usize,
    dly_next: f64,
    /// Peak input since the last history step and the latest value (held when no frame came in between).
    dly_acc: f32,
    dly_last: f32,
    dly_fresh: bool,
    /// Delay time (s) and wet level (dB, fall 60 dB/s).
    pub dly_time: f32,
    pub dly_wet_db: f32,
    /// Reverb: seconds since the input went quiet, wet level (dB, fall 60 dB/s) and the loudest
    /// wet level of this tail (the reference the curve is drawn from).
    pub rev_age: f32,
    pub rev_db: f32,
    pub rev_ref: f32,
    rev_loud: bool,
}

impl Default for FxLive {
    fn default() -> Self {
        Self {
            dist_trail: [0.0; TRAIL],
            mb_level: [-120.0; 3],
            mb_gain: [0.0; 3],
            cho_phase: 0.0,
            dim_l: vec![0.0; 192],
            dim_r: vec![0.0; 192],
            dly_hist: vec![0.0; HIST_N],
            dly_n: 0,
            dly_next: 0.0,
            dly_acc: 0.0,
            dly_last: 0.0,
            dly_fresh: false,
            dly_time: 0.25,
            dly_wet_db: -120.0,
            rev_age: 0.0,
            rev_db: -120.0,
            rev_ref: -80.0,
            rev_loud: false,
        }
    }
}

impl FxLive {
    /// Once per frame from the newest record; `dt` is the clamped frame time, `now` egui time.
    pub fn update(&mut self, f: &VizFrame, now: f64, dt: f32) {
        self.dist_trail.rotate_right(1);
        self.dist_trail[0] = f.fx(xf::DIST_IN);
        for b in 0..3 {
            self.mb_level[b] = f.fx(xf::MB_LEVEL + b).max(self.mb_level[b] - 40.0 * dt).max(-120.0);
            self.mb_gain[b] = f.fx(xf::MB_GAIN + b);
        }
        self.cho_phase = f.fx(xf::CHO_PHASE);

        // Delay: the input level goes into a 100 Hz history so tap k can ask what entered k delays ago.
        self.dly_last = f.fx(xf::DLY_IN);
        self.dly_acc = self.dly_acc.max(self.dly_last);
        self.dly_fresh = true;
        if now < self.dly_next - 1.0 || now > self.dly_next + 1.0 {
            self.dly_next = now; // egui time restarted, or the editor was hidden for a while
        }
        while now >= self.dly_next {
            self.dly_hist[self.dly_n % HIST_N] = if std::mem::take(&mut self.dly_fresh) { std::mem::take(&mut self.dly_acc) } else { self.dly_last };
            self.dly_n += 1;
            self.dly_next += HIST_STEP;
        }
        let t = f.fx(xf::DLY_TIME);
        if t > 0.001 {
            self.dly_time = t;
        }
        self.dly_wet_db = db(f.fx(xf::DLY_WET)).max(self.dly_wet_db - 60.0 * dt).max(-120.0);

        // Reverb: age of the tail since the input last was loud, wet level against the loudest one.
        let loud = db(f.fx(xf::REV_IN)) > -36.0;
        if loud && !self.rev_loud {
            self.rev_ref = -80.0;
        }
        self.rev_loud = loud;
        self.rev_age = if loud { 0.0 } else { self.rev_age + dt };
        self.rev_db = db(f.fx(xf::REV_WET)).max(self.rev_db - 60.0 * dt).max(-120.0);
        self.rev_ref = self.rev_ref.max(self.rev_db);
    }

    /// Output L / R over the scope window: `mid` and `side` are the same samples of the two rings.
    pub fn set_stereo(&mut self, mid: &[f32], side: &[f32]) {
        let span = mid.len().min(side.len());
        if span < 2 {
            return;
        }
        // One scale for both channels, so a difference between them stays a difference.
        let peak = (0..span).fold(0.05f32, |m, i| m.max((mid[i] + side[i]).abs()).max((mid[i] - side[i]).abs()));
        for i in 0..self.dim_l.len() {
            let x = i as f32 / (self.dim_l.len() - 1) as f32 * (span - 1) as f32;
            let (k, fr) = (x as usize, x.fract());
            let k1 = (k + 1).min(span - 1);
            let m = mid[k] + (mid[k1] - mid[k]) * fr;
            let s = side[k] + (side[k1] - side[k]) * fr;
            self.dim_l[i] = ((m + s) / peak).clamp(-1.2, 1.2);
            self.dim_r[i] = ((m - s) / peak).clamp(-1.2, 1.2);
        }
    }

    /// 0..1 brightness of what entered the delay `lag_s` seconds ago (-50 dB .. 0 dB), the flash of
    /// a tap that lands now.
    pub fn dly_flash(&self, lag_s: f32) -> f32 {
        let lag = (lag_s / HIST_STEP as f32).round() as usize;
        if lag >= self.dly_n.min(HIST_N) {
            return 0.0;
        }
        ((db(self.dly_hist[(self.dly_n - 1 - lag) % HIST_N]) + 50.0) / 50.0).clamp(0.0, 1.0)
    }
}

fn px(p: &Painter) -> f32 {
    1.0 / p.ctx().pixels_per_point()
}

fn snap(p: &Painter, v: f32) -> f32 {
    let q = px(p);
    (v / q).round() * q
}

/// Vertical gradient fill between a polyline and the horizontal line `base`.
fn fill_to(p: &Painter, pts: &[Pos2], base: f32, at_curve: Color32, at_base: Color32) {
    let mut mesh = Mesh::default();
    for (i, q) in pts.iter().enumerate() {
        mesh.vertices.push(Vertex { pos: *q, uv: WHITE_UV, color: at_curve });
        mesh.vertices.push(Vertex { pos: pos2(q.x, base), uv: WHITE_UV, color: at_base });
        if i > 0 {
            let b = (i as u32 - 1) * 2;
            mesh.indices.extend_from_slice(&[b, b + 1, b + 2, b + 2, b + 1, b + 3]);
        }
    }
    p.add(Shape::mesh(mesh));
}

fn faint(p: &Painter, a: f32) -> Stroke {
    Stroke::new(px(p), Color32::WHITE.gamma_multiply(a))
}

/// Live dot: glow sprite, dark outline, white core (the playhead recipe of MOTION.md 5.3).
fn dot(p: &Painter, at: Pos2, col: Color32, alpha: f32) {
    rr_ui::fx::glow(p, at, 9.0, col, 0.45 * alpha);
    p.circle_filled(at, 3.2, Color32::BLACK.gamma_multiply(0.7 * alpha));
    p.circle_filled(at, 2.3, Color32::WHITE.gamma_multiply(alpha));
}

/// Distortion transfer curve (input -1..1 across, output -1.25..1.25 up), with the live input as a
/// dot riding the curve and a short fading trail of where it just was.
pub fn dist_display(p: &Painter, r: Rect, kind: DistType, drive_db: f32, bias: f32, trail: &[f32; TRAIL], live: bool) {
    small_glass(p, r);
    let inner = r.shrink(6.0);
    let at = |x: f32, y: f32| pos2(inner.center().x + x * inner.width() / 2.0, inner.center().y - y / 1.25 * inner.height() / 2.0);
    p.hline(inner.x_range(), snap(p, inner.center().y), faint(p, 0.12));
    p.vline(snap(p, inner.center().x), inner.y_range(), faint(p, 0.12));
    p.add(Shape::dashed_line(&[at(-1.0, -1.0), at(1.0, 1.0)], faint(p, 0.14), 3.0, 3.0));
    let n = (inner.width() as usize).max(32);
    let pts: Vec<Pos2> = (0..=n)
        .map(|i| {
            let x = i as f32 / n as f32 * 2.0 - 1.0;
            at(x, transfer(kind, drive_db, bias, x).clamp(-1.25, 1.25))
        })
        .collect();
    glow_line(p, pts, 1.5, T.accent);
    if live {
        for (i, &x) in trail.iter().enumerate().rev() {
            if x.abs() < 1e-4 {
                continue;
            }
            let x = x.clamp(-1.0, 1.0);
            let q = at(x, transfer(kind, drive_db, bias, x).clamp(-1.25, 1.25));
            if i == 0 {
                dot(p, q, T.accent, 1.0);
            } else {
                p.circle_filled(q, 2.0, T.accent.gamma_multiply(0.5 * (1.0 - i as f32 / TRAIL as f32).powi(2)));
            }
        }
    }
}

/// Multiband band meters: per band a bar of the detector level (-60 .. 0 dB), a bright cap where the
/// level ends up after the band gain, the gain itself as a lighter (cut) or mint (boost) segment,
/// and the two thresholds as marks (down: solid, up: dashed).
pub fn mb_display(p: &Painter, r: Rect, level: [f32; 3], gain: [f32; 3], up: [f32; 3], down: [f32; 3]) {
    small_glass(p, r);
    let inner = Rect::from_min_max(pos2(r.left() + 5.0, r.top() + 6.0), pos2(r.right() - 5.0, r.bottom() - 17.0));
    let gap = 4.0;
    let bw = (inner.width() - 2.0 * gap) / 3.0;
    let y_of = |db: f32| inner.bottom() - inner.height() * ((db + 60.0) / 60.0).clamp(0.0, 1.0);
    for b in 0..3 {
        let x0 = inner.left() + b as f32 * (bw + gap);
        let (xa, xb) = (x0, x0 + bw);
        p.rect_filled(Rect::from_min_max(pos2(xa, inner.top()), pos2(xb, inner.bottom())), 1.0, Color32::from_rgb(0x0a, 0x0e, 0x10));
        let top = y_of(level[b]);
        if top < inner.bottom() - 0.5 {
            paint::vgradient(p, Rect::from_min_max(pos2(xa, top), pos2(xb, inner.bottom())), T.accent.gamma_multiply(0.85), T.accent.gamma_multiply(0.25));
        }
        // Gain overlay: the level moves by `gain` dB.
        let out = y_of(level[b] + gain[b]);
        if gain[b].abs() > 0.15 && level[b] > -60.0 {
            let col = if gain[b] < 0.0 { Color32::WHITE.gamma_multiply(0.20) } else { T.accent.gamma_multiply(0.35) };
            p.rect_filled(Rect::from_min_max(pos2(xa, top.min(out)), pos2(xb, top.max(out))), 0.0, col);
        }
        if level[b] > -60.0 {
            p.rect_filled(Rect::from_min_max(pos2(xa, snap(p, out) - 0.75), pos2(xb, snap(p, out) + 0.75)), 0.0, Color32::WHITE.gamma_multiply(0.9));
        }
        let dy = snap(p, y_of(down[b]));
        p.hline((xa - 2.0)..=(xb + 2.0), dy, Stroke::new(px(p), T.text.gamma_multiply(0.8)));
        let uy = snap(p, y_of(up[b]));
        p.add(Shape::dashed_line(&[pos2(xa - 2.0, uy), pos2(xb + 2.0, uy)], Stroke::new(px(p), T.text.gamma_multiply(0.55)), 2.0, 2.0));
        p.text(pos2((xa + xb) / 2.0, r.bottom() - 8.0), Align2::CENTER_CENTER, ["L", "M", "H"][b], font::label(11.0), T.disp_caption);
    }
}

/// EQ response (30 Hz .. 20 kHz across, +-15 dB up) over a faint live spectrum, with a dot per band
/// at its frequency and gain (display only) and a dashed mark at the low cut.
pub fn eq_display(p: &Painter, r: Rect, ep: &EqParams, spec: &[f32], peak: &[f32]) {
    small_glass(p, r);
    let inner = r.shrink2(vec2(6.0, 6.0));
    let x_of = |hz: f32| inner.left() + inner.width() * ((hz / 30.0).max(1.0).ln() / (20000f32 / 30.0).ln()).clamp(0.0, 1.0);
    let y_of = |d: f32| inner.center().y - inner.height() / 2.0 * (d / 15.0).clamp(-1.0, 1.0);
    for hz in [100.0f32, 1000.0, 10000.0] {
        p.vline(snap(p, x_of(hz)), inner.y_range(), faint(p, 0.05));
    }
    for d in [-10.0f32, 10.0] {
        p.hline(inner.x_range(), snap(p, y_of(d)), faint(p, 0.04));
    }
    if spec.len() > 1 {
        spectrum(p, inner, spec, peak, 0.35);
    }
    let y0 = snap(p, y_of(0.0));
    p.hline(inner.x_range(), y0, faint(p, 0.14));
    if ep.cut_on {
        let x = snap(p, x_of(ep.cut_freq));
        p.add(Shape::dashed_line(&[pos2(x, inner.top()), pos2(x, inner.bottom())], faint(p, 0.22), 3.0, 3.0));
    }
    let n = (inner.width() as usize).max(32);
    let pts: Vec<Pos2> = (0..=n)
        .map(|i| {
            let hz = 30.0 * (20000f32 / 30.0).powf(i as f32 / n as f32);
            pos2(inner.left() + inner.width() * i as f32 / n as f32, y_of(response_db(ep, hz as f64) as f32))
        })
        .collect();
    fill_to(p, &pts, y0, T.a(0.20), T.a(0.0));
    glow_line(p, pts, 1.5, T.accent);
    for (hz, g) in [(ep.low_freq, ep.low_gain), (ep.peak_freq, ep.peak_gain), (ep.high_freq, ep.high_gain)] {
        let q = pos2(x_of(hz), y_of(g.clamp(-15.0, 15.0)));
        p.circle_filled(q, 3.4, Color32::BLACK.gamma_multiply(0.7));
        p.circle_filled(q, 2.4, T.text);
    }
}

/// Chorus voices: one lane per voice, a dot on it moving with that voice's LFO (the DSP's own phase
/// and per-voice offsets); the swing grows with Depth.
pub fn chorus_display(p: &Painter, r: Rect, voices: usize, phase: f32, depth: f32) {
    small_glass(p, r);
    let inner = r.shrink2(vec2(8.0, 7.0));
    let n = voices.clamp(2, OFFSET.len());
    let amp = (0.25 + 0.75 * depth.clamp(0.0, 1.0)) * inner.height() / 2.0;
    p.hline(inner.x_range(), snap(p, inner.center().y), faint(p, 0.10));
    for v in 0..n {
        let x = inner.left() + inner.width() * (v as f32 + 0.5) / n as f32;
        p.vline(snap(p, x), inner.y_range(), faint(p, 0.06));
        let s = (std::f32::consts::TAU * (phase + OFFSET[v] as f32)).sin();
        let q = pos2(x, inner.center().y - s * amp);
        let col = if v % 2 == 0 { T.accent } else { T.text };
        rr_ui::fx::glow(p, q, 8.0, col, 0.35);
        p.circle_filled(q, 2.4, col);
    }
}

/// Dimension: the output's left (mint, upper) and right (white, lower) channel over the same two
/// cycles; what Dimension's side signal does to them shows as the difference between the traces.
pub fn dim_display(p: &Painter, r: Rect, l: &[f32], rr: &[f32]) {
    small_glass(p, r);
    let inner = r.shrink2(vec2(8.0, 5.0));
    for (k, (tr, col)) in [(l, T.accent), (rr, T.text)].into_iter().enumerate() {
        let mid = inner.top() + inner.height() * (0.25 + 0.5 * k as f32);
        let a = inner.height() * 0.21;
        p.hline(inner.x_range(), snap(p, mid), faint(p, 0.10));
        if tr.len() > 1 {
            let n = tr.len() - 1;
            let pts = tr.iter().enumerate().map(|(i, s)| pos2(inner.left() + inner.width() * i as f32 / n as f32, mid - s * a)).collect();
            glow_line(p, pts, 1.5, col);
        }
    }
}

/// Delay taps: a dry stub then one bar per echo, heights falling by the feedback each tap. A bar
/// flashes when the sound that entered the delay k delay times ago lands (`flash(k)`, 0..1); the
/// thin bar at the right edge is the echo level.
pub fn delay_display(p: &Painter, r: Rect, feedback: f32, mix: f32, flash: &dyn Fn(usize) -> f32, wet_db: f32) {
    small_glass(p, r);
    let inner = Rect::from_min_max(pos2(r.left() + 7.0, r.top() + 6.0), pos2(r.right() - 12.0, r.bottom() - 6.0));
    let base = snap(p, inner.bottom());
    p.hline(inner.x_range(), base, faint(p, 0.14));
    let fb = feedback.clamp(0.0, 0.95);
    let mut taps = vec![];
    let mut h = 0.3 + 0.7 * mix.clamp(0.0, 1.0);
    for k in 1..=8 {
        if h < 0.05 && k > 1 {
            break;
        }
        taps.push(h);
        h *= fb;
    }
    let slots = taps.len() as f32 + 1.0;
    let dx = inner.width() / slots;
    let bar = |x: f32, h: f32, col: Color32, w: f32| {
        p.rect_filled(Rect::from_min_max(pos2(x - w / 2.0, inner.bottom() - h * inner.height()), pos2(x + w / 2.0, inner.bottom())), 1.0, col);
    };
    bar(inner.left() + dx * 0.5, 0.7, Color32::WHITE.gamma_multiply(0.35), 3.0);
    for (i, &h) in taps.iter().enumerate() {
        let x = inner.left() + dx * (i as f32 + 1.5);
        let f = flash(i + 1);
        bar(x, h, T.accent.gamma_multiply(0.45 + 0.55 * f), 3.0);
        if f > 0.05 {
            rr_ui::fx::glow(p, pos2(x, inner.bottom() - h * inner.height()), 9.0, T.accent, 0.5 * f);
        }
    }
    let lv = ((wet_db + 60.0) / 60.0).clamp(0.0, 1.0);
    let track = Rect::from_min_max(pos2(r.right() - 8.0, inner.top()), pos2(r.right() - 5.0, inner.bottom()));
    p.rect_filled(track, 0.0, Color32::from_rgb(0x0a, 0x0e, 0x10));
    if lv > 0.01 {
        p.rect_filled(Rect::from_min_max(pos2(track.left(), track.bottom() - track.height() * lv), track.max), 0.0, T.accent);
    }
}

/// Reverb tail: the decay for the set Decay and Pre-delay on a dB scale (0 to -60 dB up the left
/// axis, seconds along the bottom), a straight line down to -60 dB at the decay time since the tail
/// decays exponentially; a playhead at the age of the tail with a dot at the measured wet level
/// relative to the tail's peak.
pub fn reverb_display(p: &Painter, r: Rect, decay_s: f32, predelay_s: f32, age: f32, level_db: Option<f32>) {
    small_glass(p, r);
    let inner = Rect::from_min_max(pos2(r.left() + 32.0, r.top() + 10.0), pos2(r.right() - 12.0, r.bottom() - 20.0));
    let decay = decay_s.max(0.05);
    let span = (decay * 1.15).max(predelay_s + 0.4);
    let x_of = |t: f32| inner.left() + inner.width() * (t / span).clamp(0.0, 1.0);
    let y_of = |d: f32| inner.top() + inner.height() * (-d / 60.0).clamp(0.0, 1.0);
    let label = |pos: Pos2, a: Align2, s: &str| p.text(pos, a, s, font::mono(10.0), T.disp_caption);
    // dB grid and labels.
    for d in [0.0f32, -20.0, -40.0, -60.0] {
        p.hline(inner.x_range(), snap(p, y_of(d)), faint(p, if d == -60.0 { 0.14 } else { 0.05 }));
        label(pos2(inner.left() - 6.0, y_of(d)), Align2::RIGHT_CENTER, &format!("{d:.0}"));
    }
    // Time grid and labels: a step that gives three to six ticks.
    let step = [0.1f32, 0.2, 0.5, 1.0, 2.0, 5.0].into_iter().find(|s| span / s <= 6.0).unwrap_or(5.0);
    let mut t = 0.0;
    while t <= span + 1e-3 {
        let x = snap(p, x_of(t));
        p.vline(x, inner.y_range(), faint(p, 0.05));
        let s = if step < 1.0 { format!("{t:.1}") } else { format!("{t:.0}") };
        label(pos2(x, inner.bottom() + 9.0), if t == 0.0 { Align2::LEFT_CENTER } else { Align2::CENTER_CENTER }, &s);
        t += step;
    }
    label(pos2(r.right() - 6.0, r.bottom() - 7.0), Align2::RIGHT_CENTER, "s");
    label(pos2(r.left() + 6.0, r.top() + 8.0), Align2::LEFT_CENTER, "dB");
    // Curve: flat at 0 dB through the pre-delay, then down to -60 dB at the decay time (and the floor).
    let t0 = predelay_s.min(span * 0.9);
    let t_end = (t0 + decay).min(span);
    let mut line = vec![pos2(x_of(0.0), y_of(0.0)), pos2(x_of(t0), y_of(0.0)), pos2(x_of(t_end), y_of(-60.0 * (t_end - t0) / decay))];
    if t_end < span {
        line.push(pos2(x_of(span), y_of(-60.0)));
    }
    fill_to(p, &line, snap(p, inner.bottom()), T.a(0.18), T.a(0.0));
    glow_line(p, line, 1.5, T.accent);
    if let Some(d) = level_db {
        if age < span {
            let x = snap(p, x_of(age));
            p.vline(x, inner.y_range(), Stroke::new(px(p), Color32::WHITE.gamma_multiply(0.45)));
            dot(p, pos2(x, y_of(d.clamp(-60.0, 0.0))), T.accent, 1.0);
        }
    }
}

/// Limiter output spectrum glass with decade ticks and a caption.
pub fn limiter_spectrum(p: &Painter, r: Rect, spec: &[f32], peak: &[f32]) {
    small_glass(p, r);
    let inner = Rect::from_min_max(pos2(r.left() + 10.0, r.top() + 20.0), pos2(r.right() - 10.0, r.bottom() - 16.0));
    for d in [0.25f32, 0.5, 0.75] {
        p.hline(inner.x_range(), snap(p, inner.bottom() - inner.height() * d), faint(p, 0.04));
    }
    if spec.len() > 1 {
        spectrum(p, inner, spec, peak, 1.0);
    }
    paint::tracked(p, pos2(r.left() + 10.0, r.top() + 10.0), Align2::LEFT_CENTER, "OUTPUT SPECTRUM", font::label(11.0), T.disp_caption, 0.8);
    for (hz, s) in [(100.0f32, "100"), (1000.0, "1k"), (10000.0, "10k")] {
        let x = inner.left() + inner.width() * (hz / 30.0).ln() / (20000f32 / 30.0).ln();
        p.text(pos2(x, r.bottom() - 8.0), Align2::CENTER_CENTER, s, font::mono(10.0), T.disp_caption);
    }
}
