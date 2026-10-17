//! Painter helpers shared by the widgets: gradients, tracked caps text, arcs, glows.
//! Everything is drawn with the egui painter; there are no images.

use crate::theme::T;
use egui::epaint::{Mesh, Vertex, WHITE_UV};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Shape, Stroke, StrokeKind};

/// Axis-aligned quad with a colour per corner (top-left, top-right, bottom-right, bottom-left).
fn quad(p: &Painter, r: Rect, c: [Color32; 4]) {
    if r.width() <= 0.0 || r.height() <= 0.0 {
        return;
    }
    let mut m = Mesh::default();
    for (pos, color) in [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()].into_iter().zip(c) {
        m.vertices.push(Vertex { pos, uv: WHITE_UV, color });
    }
    m.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
    p.add(Shape::mesh(m));
}

pub fn vgradient(p: &Painter, r: Rect, top: Color32, bottom: Color32) {
    quad(p, r, [top, top, bottom, bottom]);
}

pub fn hgradient(p: &Painter, r: Rect, left: Color32, right: Color32) {
    quad(p, r, [left, right, right, left]);
}

/// Rounded rect filled with a vertical gradient. The corner caps are flat fills of the end
/// colours, so the gradient runs between `r` below the top and `r` above the bottom.
/// With `round_bottom == false` the gradient runs to the bottom edge (used for partial overlays).
pub fn rounded_vgradient(p: &Painter, rect: Rect, r: f32, top: Color32, bottom: Color32, round_bottom: bool) {
    let rr = r as u8;
    p.rect_filled(rect, CornerRadius { nw: rr, ne: rr, sw: if round_bottom { rr } else { 0 }, se: if round_bottom { rr } else { 0 } }, bottom);
    let cap = Rect::from_min_size(rect.min, vec2(rect.width(), r));
    p.rect_filled(cap, CornerRadius { nw: rr, ne: rr, sw: 0, se: 0 }, top);
    let end = if round_bottom { rect.bottom() - r } else { rect.bottom() };
    vgradient(p, Rect::from_min_max(pos2(rect.left(), rect.top() + r), pos2(rect.right(), end)), top, bottom);
}

/// Width of `text` in `font` with `tracking` extra px between glyphs.
pub fn text_width(p: &Painter, font: &FontId, text: &str, tracking: f32) -> f32 {
    let n = text.chars().count();
    if n == 0 {
        return 0.0;
    }
    let adv: f32 = p.ctx().fonts(|f| text.chars().map(|c| f.glyph_width(font, c)).sum());
    adv + tracking * (n - 1) as f32
}

/// Paints `text` glyph by glyph with `tracking` px of extra advance (egui has no letter
/// spacing). Returns the painted rect. With zero tracking this is a plain `Painter::text`.
pub fn tracked(p: &Painter, pos: Pos2, anchor: Align2, text: &str, font: FontId, color: Color32, tracking: f32) -> Rect {
    if tracking == 0.0 {
        return p.text(pos, anchor, text, font, color);
    }
    let w = text_width(p, &font, text, tracking);
    let h = p.ctx().fonts(|f| f.row_height(&font));
    let rect = anchor.anchor_size(pos, vec2(w, h));
    let mut x = rect.left();
    for ch in text.chars() {
        let adv = p.ctx().fonts(|f| f.glyph_width(&font, ch));
        if ch != ' ' {
            p.text(pos2(x, rect.top()), Align2::LEFT_TOP, ch, font.clone(), color);
        }
        x += adv + tracking;
    }
    rect
}

/// Caps label: engraved look is applied by the caller where wanted.
pub fn caps(p: &Painter, pos: Pos2, anchor: Align2, text: &str, size: f32, color: Color32) -> Rect {
    tracked(p, pos, anchor, &text.to_uppercase(), crate::theme::font::label(size), color, 0.5)
}

/// Points of an arc, angles in degrees, clockwise with y down (0 = 3 o'clock). Segments are as
/// long as a 0.05 px chord error allows (a 17 px knob arc: about 9 degrees), never over 10 degrees:
/// vertex count, not shape count, is what tessellation costs.
pub fn arc_points(c: Pos2, r: f32, a0: f32, a1: f32) -> Vec<Pos2> {
    let step = (2.0 * (1.0 - 0.05 / r.max(0.1)).clamp(-1.0, 1.0).acos()).to_degrees().clamp(1.0, 10.0);
    let n = (((a1 - a0).abs() / step).ceil() as usize).max(2);
    (0..=n)
        .map(|i| {
            let a = (a0 + (a1 - a0) * i as f32 / n as f32).to_radians();
            c + vec2(a.cos(), a.sin()) * r
        })
        .collect()
}

/// Arc stroke with optional round caps (caps only for opaque colours; translucent glow
/// strokes would double-blend under the end discs).
pub fn arc_stroke(p: &Painter, c: Pos2, r: f32, a0: f32, a1: f32, width: f32, color: Color32, caps: bool) {
    let pts = arc_points(c, r, a0, a1);
    if caps {
        p.circle_filled(pts[0], width / 2.0, color);
        p.circle_filled(*pts.last().unwrap(), width / 2.0, color);
    }
    p.add(Shape::line(pts, st(width, color)));
}

/// Stacked discs approximating a soft glow: (radius, alpha) layers, widest first.
pub fn glow_discs(p: &Painter, c: Pos2, color: Color32, layers: &[(f32, f32)]) {
    for &(r, a) in layers {
        p.circle_filled(c, r, color.gamma_multiply(a));
    }
}

/// Raised-edge highlight below a rect: the same shape one px lower in the highlight colour,
/// painted before the real fill so only the lower edge shows.
pub fn highlight_below(p: &Painter, r: Rect, radius: f32) {
    p.rect_filled(r.translate(vec2(0.0, 1.0)), radius, T.hi());
}

/// Recessed control body (combo, toggle): inset fill, 1 px dark inner stroke, highlight below.
pub fn inset_box(p: &Painter, r: Rect, hot: bool) {
    highlight_below(p, r, 4.0);
    p.rect_filled(r, 4.0, T.inset);
    if hot {
        p.rect_filled(r, 4.0, Color32::WHITE.gamma_multiply(0.04));
    }
    p.rect_stroke(r, 4.0, hair(p, T.lo()), StrokeKind::Inside);
}

/// Dark display glass: gradient top 40 %, inner shadow, 1 px black edge, highlight below.
pub fn display_bg(p: &Painter, r: Rect) {
    highlight_below(p, r, crate::theme::metrics::GROUP_R);
    let rad = crate::theme::metrics::GROUP_R;
    p.rect_filled(r, rad, T.disp_bottom);
    let top = Rect::from_min_size(r.min, vec2(r.width(), r.height() * 0.4));
    rounded_vgradient(p, top, rad, T.disp_top, T.disp_bottom, false);
    // Inner shadow, strongest at the top edge.
    let shade = Rect::from_min_size(r.min + vec2(1.0, 1.0), vec2(r.width() - 2.0, 10.0));
    vgradient(p, shade, Color32::BLACK.gamma_multiply(0.5), Color32::TRANSPARENT);
    p.rect_stroke(r, rad, hair(p, Color32::BLACK), StrokeKind::Inside);
}

/// Anti-aliased single px line helper (half-px offset keeps 1 px lines crisp).
pub fn hline(p: &Painter, x0: f32, x1: f32, y: f32, color: Color32) {
    p.line_segment([pos2(x0, y), pos2(x1, y)], hair(p, color));
}

/// LED with layered glow when lit (see DESIGN.md "LED on").
pub fn led(p: &Painter, c: Pos2, on: bool) {
    if on {
        crate::fx::glow(p, c, 11.0, T.accent, 0.34);
        p.circle_stroke(c, 5.0, hair(p, T.a(0.4)));
        p.circle_filled(c, 4.0, T.accent);
    } else {
        p.circle_filled(c, 4.0, T.led_off);
        p.circle_stroke(c, 4.0, hair(p, Color32::BLACK));
    }
}

/// Level LED: the unlit LED with the lit look faded in by `level` (0..=1).
pub fn led_level(p: &Painter, c: Pos2, level: f32) {
    led(p, c, false);
    let k = level.clamp(0.0, 1.0);
    if k > 0.02 {
        crate::fx::glow(p, c, 11.0, T.accent, 0.34 * k);
        p.circle_stroke(c, 5.0, hair(p, T.a(0.4 * k)));
        p.circle_filled(c, 4.0, T.accent.gamma_multiply(k));
    }
}

/// `Stroke::new` with the width pinned to f32 (avoids float-literal inference warnings).
pub fn st(width: f32, color: Color32) -> Stroke {
    Stroke::new(width, color)
}

/// A one physical-pixel stroke: `1 / pixels_per_point` wide, so hairlines, dividers and outlines
/// resolve to one full-coverage pixel at every UI scale (a bare 1.0 is 1.25 or 1.5 px, blurred, at
/// fractional scales; motion-feasibility.md 3).
pub fn hair(p: &Painter, color: Color32) -> Stroke {
    Stroke::new(1.0 / p.ctx().pixels_per_point(), color)
}

/// A gradient quad as a shape (for placing inside a `Shape::Vec` at a reserved slot).
pub fn vgradient_shape(r: Rect, top: Color32, bottom: Color32) -> Shape {
    let mut m = Mesh::default();
    for (pos, color) in [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()].into_iter().zip([top, top, bottom, bottom]) {
        m.vertices.push(Vertex { pos, uv: WHITE_UV, color });
    }
    m.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
    Shape::mesh(m)
}
