//! Output displays (docs/design/MOTION.md 5.5): the header scope / spectrum glass and the stereo
//! level meter. Painters only: the caller owns the data and its ballistics.

use crate::paint;
use crate::theme::{font, T};
use egui::epaint::{Mesh, Vertex, WHITE_UV};
use egui::{pos2, vec2, Align2, Color32, Painter, Pos2, Rect, Shape, Stroke, StrokeKind};

/// Glow recipe for live curves: 6x at 7 %, 3x at 17 %, then the 1x core.
pub fn glow_line(p: &Painter, pts: Vec<Pos2>, core: f32, color: Color32) {
    p.add(Shape::line(pts.clone(), Stroke::new(core * 6.0, color.gamma_multiply(0.07))));
    p.add(Shape::line(pts.clone(), Stroke::new(core * 3.0, color.gamma_multiply(0.17))));
    p.add(Shape::line(pts, Stroke::new(core, color)));
}

/// Small glass (radius 4): `disp_bottom` with a `disp_top` gradient over the top 40 %, 1 px edges.
pub fn small_glass(p: &Painter, r: Rect) {
    paint::highlight_below(p, r, 4.0);
    p.rect_filled(r, 4.0, T.disp_bottom);
    paint::rounded_vgradient(p, Rect::from_min_size(r.min, vec2(r.width(), r.height() * 0.4)), 4.0, T.disp_top, T.disp_bottom, false);
    p.rect_stroke(r, 4.0, paint::hair(p, Color32::BLACK), StrokeKind::Inside);
}

/// Scope / spectrum, crossfaded by `mix` (0 = scope, 1 = spectrum). `trace` is -1..1 oldest
/// first; `spec` and `peak` are 0..1 heights of log-spaced columns (30 Hz .. 20 kHz).
pub fn scope_display(p: &Painter, r: Rect, trace: &[f32], spec: &[f32], peak: &[f32], mix: f32) {
    small_glass(p, r);
    let px = 1.0 / p.ctx().pixels_per_point();
    let (x0, x1, y0, y1) = (r.left() + 10.0, r.right() - 10.0, r.top() + 18.0, r.bottom() - 6.0);
    let mid = (y0 + y1) / 2.0;
    let snap = |v: f32| (v / px).round() * px;
    if mix < 0.99 && trace.len() > 1 {
        let a = 1.0 - mix;
        for i in 0..=8 {
            p.vline(snap(x0 + (x1 - x0) * i as f32 / 8.0), y0..=y1, Stroke::new(px, Color32::WHITE.gamma_multiply(0.06 * a)));
        }
        p.hline(x0..=x1, snap(mid), Stroke::new(px, Color32::WHITE.gamma_multiply(0.12 * a)));
        let n = trace.len() - 1;
        let pts = trace.iter().enumerate().map(|(i, s)| pos2(x0 + (x1 - x0) * i as f32 / n as f32, mid - s.clamp(-1.2, 1.2) * (y1 - y0) / 2.0 * 0.9)).collect();
        glow_line(p, pts, 1.5, T.accent.gamma_multiply(a));
    }
    if mix > 0.01 && spec.len() > 1 {
        spectrum(p, Rect::from_min_max(pos2(x0, y0), pos2(x1, y1)), spec, peak, mix);
    }
    // Tabs: the active one in accent.
    let (sc, sp) = (if mix < 0.5 { T.accent } else { T.disp_caption }, if mix >= 0.5 { T.accent } else { T.disp_caption });
    let rr = paint::tracked(p, pos2(r.right() - 8.0, r.top() + 9.0), Align2::RIGHT_CENTER, "SPEC", font::label(11.0), sp, 0.8);
    paint::tracked(p, pos2(rr.left() - 8.0, r.top() + 9.0), Align2::RIGHT_CENTER, "SCOPE", font::label(11.0), sc, 0.8);
}

/// Spectrum curve in `r` (columns log-spaced 30 Hz .. 20 kHz, heights 0..1): decade lines, an accent
/// under-fill (30 % at the top fading to 0), the glowing curve and the thin peak-hold line, all
/// scaled by `alpha`. Shared by the header, the EQ and the Limiter displays.
pub fn spectrum(p: &Painter, r: Rect, spec: &[f32], peak: &[f32], alpha: f32) {
    let px = 1.0 / p.ctx().pixels_per_point();
    let (x0, x1, y0, y1) = (r.left(), r.right(), r.top(), r.bottom());
    for f in [100.0f32, 1000.0, 10000.0] {
        let x = x0 + (x1 - x0) * (f / 30.0).ln() / (20000.0f32 / 30.0).ln();
        p.vline((x / px).round() * px, y0..=y1, Stroke::new(px, Color32::WHITE.gamma_multiply(0.06 * alpha)));
    }
    let n = spec.len() - 1;
    let pts: Vec<Pos2> = spec.iter().enumerate().map(|(i, v)| pos2(x0 + (x1 - x0) * i as f32 / n as f32, y1 - v.clamp(0.0, 1.0) * (y1 - y0))).collect();
    let mut mesh = Mesh::default();
    for (i, q) in pts.iter().enumerate() {
        let k = ((y1 - q.y) / (y1 - y0)).clamp(0.0, 1.0);
        mesh.vertices.push(Vertex { pos: *q, uv: WHITE_UV, color: T.a(0.30 * k * alpha) });
        mesh.vertices.push(Vertex { pos: pos2(q.x, y1), uv: WHITE_UV, color: Color32::TRANSPARENT });
        if i > 0 {
            let b = (i as u32 - 1) * 2;
            mesh.indices.extend_from_slice(&[b, b + 1, b + 2, b + 2, b + 1, b + 3]);
        }
    }
    p.add(Shape::mesh(mesh));
    glow_line(p, pts, 1.5, T.accent.gamma_multiply(alpha));
    if peak.len() == spec.len() {
        let pk = peak.iter().enumerate().map(|(i, v)| pos2(x0 + (x1 - x0) * i as f32 / n as f32, y1 - v.clamp(0.0, 1.0) * (y1 - y0))).collect();
        p.add(Shape::line(pk, paint::hair(p, T.text.gamma_multiply(0.28 * alpha))));
    }
}

/// Stereo level meter: two 5 px bars (mint gradient), `level` and `hold` as 0..1 of the scale;
/// the hold line turns rose above 0.93.
pub fn level_meter(p: &Painter, r: Rect, level: [f32; 2], hold: [f32; 2]) {
    let (bw, gap) = (5.0, 2.0);
    let x0 = r.center().x - (2.0 * bw + gap) / 2.0;
    let px = 1.0 / p.ctx().pixels_per_point();
    for i in 0..2 {
        let x = x0 + i as f32 * (bw + gap);
        let track = Rect::from_min_max(pos2(x, r.top() + 3.0), pos2(x + bw, r.bottom() - 3.0));
        p.rect_filled(track, 0.0, Color32::from_rgb(0x0a, 0x0e, 0x10));
        let h = track.height() * level[i].clamp(0.0, 1.0);
        if h > 0.5 {
            // Gradient #2fae7d -> accent -> #d7f7e8 over the full scale, clipped at the level.
            let full = track;
            let lo = Color32::from_rgb(0x2f, 0xae, 0x7d);
            let top = Color32::from_rgb(0xd7, 0xf7, 0xe8);
            let y70 = full.bottom() - full.height() * 0.7;
            let bar = Rect::from_min_max(pos2(x, full.bottom() - h), full.max);
            let lower = Rect::from_min_max(pos2(x, bar.top().max(y70)), full.max);
            let k = ((full.bottom() - lower.top()) / (full.height() * 0.7)).clamp(0.0, 1.0);
            paint::vgradient(p, lower, lo.lerp_to_gamma(T.accent, k), lo);
            if bar.top() < y70 {
                let k2 = ((y70 - bar.top()) / (full.height() * 0.3)).clamp(0.0, 1.0);
                paint::vgradient(p, Rect::from_min_max(bar.min, pos2(bar.right(), y70)), T.accent.lerp_to_gamma(top, k2), T.accent);
            }
        }
        let hy = ((track.bottom() - track.height() * hold[i].clamp(0.0, 1.0)) / px).round() * px;
        let col = if hold[i] > 0.93 { Color32::from_rgb(0xff, 0x7a, 0x90) } else { T.text };
        p.rect_filled(Rect::from_min_max(pos2(x, hy - 1.0), pos2(x + bw, hy + 0.5)), 0.0, col);
    }
}
