//! Generated textures for depth and glow (docs/design/MOTION.md 6, motion-feasibility.md 4): two
//! 9-slice soft shadows (soft and tight) that serve every rect size, and one radial glow sprite
//! that adds light when tinted with an alpha-0 premultiplied colour (egui blends premultiplied:
//! `src + dst * (1 - src.a)`). All three are generated once per egui context and cached in its
//! data; a frame draws every shadow of the panel as two meshes.

use egui::epaint::{Mesh, Vertex};
use egui::{pos2, vec2, Color32, ColorImage, Context, Id, Painter, Rect, Shape, TextureHandle, TextureOptions, Ui};

/// Cached textures, cheap to clone (handles).
#[derive(Clone)]
pub struct Fx {
    soft: TextureHandle,
    tight: TextureHandle,
    glow: TextureHandle,
}

const SOFT: (usize, usize, f32) = (96, 36, 9.0); // side, band, sigma
const TIGHT: (usize, usize, f32) = (40, 12, 2.5);

fn gauss(sigma: f32) -> Vec<f32> {
    let r = (sigma * 3.0).ceil() as i32;
    let mut k: Vec<f32> = (-r..=r).map(|i| (-(i * i) as f32 / (2.0 * sigma * sigma)).exp()).collect();
    let s: f32 = k.iter().sum();
    k.iter_mut().for_each(|v| *v /= s);
    k
}

fn blur(a: &mut [f32], n: usize, sigma: f32) {
    let k = gauss(sigma);
    let r = (k.len() / 2) as i32;
    let mut tmp = vec![0.0; a.len()];
    let at = |x: i32| x.clamp(0, n as i32 - 1) as usize;
    for y in 0..n {
        for x in 0..n {
            tmp[y * n + x] = (-r..=r).map(|i| a[y * n + at(x as i32 + i)] * k[(i + r) as usize]).sum();
        }
    }
    for y in 0..n {
        for x in 0..n {
            a[y * n + x] = (-r..=r).map(|i| tmp[at(y as i32 + i) * n + x] * k[(i + r) as usize]).sum();
        }
    }
}

/// Premultiplied-black alpha image: a rounded rect inset by `band`, blurred.
fn shadow_image((side, band, sigma): (usize, usize, f32), radius: f32) -> ColorImage {
    let mut a = vec![0.0f32; side * side];
    let half = (side - 2 * band) as f32 / 2.0 - radius;
    for y in 0..side {
        for x in 0..side {
            let (px, py) = (x as f32 + 0.5 - side as f32 / 2.0, y as f32 + 0.5 - side as f32 / 2.0);
            let (dx, dy) = (px.abs() - half, py.abs() - half);
            let d = (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt() + dx.max(dy).min(0.0) - radius;
            a[y * side + x] = (0.5 - d).clamp(0.0, 1.0);
        }
    }
    blur(&mut a, side, sigma);
    ColorImage { size: [side, side], pixels: a.iter().map(|v| Color32::from_black_alpha((v * 255.0 + 0.5) as u8)).collect() }
}

/// Premultiplied white radial falloff, exactly 0 at the rim.
fn glow_image(side: usize) -> ColorImage {
    let c = (side as f32 - 1.0) / 2.0;
    let pixels = (0..side * side)
        .map(|i| {
            let (x, y) = ((i % side) as f32 - c, (i / side) as f32 - c);
            let r = ((x * x + y * y).sqrt() / c).min(1.0);
            let v = (((1.0 - r * r).max(0.0).powi(3)) * 255.0 + 0.5) as u8;
            Color32::from_rgba_premultiplied(v, v, v, v)
        })
        .collect();
    ColorImage { size: [side, side], pixels }
}

fn fx_id() -> Id {
    Id::new("rr-fx-textures")
}

/// The textures of this context (generated on first use, about 2 ms once).
pub fn fx(ctx: &Context) -> Fx {
    if let Some(f) = ctx.data(|d| d.get_temp::<Fx>(fx_id())) {
        return f;
    }
    let f = Fx {
        soft: ctx.load_texture("rr-shadow-soft", shadow_image(SOFT, 6.0), TextureOptions::LINEAR),
        tight: ctx.load_texture("rr-shadow-tight", shadow_image(TIGHT, 6.0), TextureOptions::LINEAR),
        glow: ctx.load_texture("rr-glow", glow_image(64), TextureOptions::LINEAR),
    };
    ctx.data_mut(|d| d.insert_temp(fx_id(), f.clone()));
    f
}

/// Additive glow sprite: `radius` px, `color` at `strength` 0..1 of its alpha.
pub fn glow(p: &Painter, center: egui::Pos2, radius: f32, color: Color32, strength: f32) {
    if strength <= 0.003 {
        return;
    }
    let f = fx(p.ctx());
    p.add(Shape::mesh(glow_mesh(&f, center, vec2(radius, radius), color, strength)));
}

/// Same, stretched to `half` extents (the panel light).
pub fn glow_ellipse(p: &Painter, center: egui::Pos2, half: egui::Vec2, color: Color32, strength: f32) {
    let f = fx(p.ctx());
    p.add(Shape::mesh(glow_mesh(&f, center, half, color, strength)));
}

fn glow_mesh(f: &Fx, c: egui::Pos2, half: egui::Vec2, color: Color32, strength: f32) -> Mesh {
    let s = strength.clamp(0.0, 1.0);
    let tint = Color32::from_rgba_premultiplied((color.r() as f32 * s) as u8, (color.g() as f32 * s) as u8, (color.b() as f32 * s) as u8, 0);
    let mut m = Mesh::with_texture(f.glow.id());
    m.add_rect_with_uv(Rect::from_center_size(c, half * 2.0), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), tint);
    m
}

// ---- shadows ----------------------------------------------------------------------------------

#[derive(Clone, Default)]
struct Pending(Vec<(Rect, f32)>);

fn pending_id() -> Id {
    Id::new("rr-shadow-pending")
}

/// Reserves the paint slot every shadow of this frame lands in (call right after the panel face,
/// before any group is drawn) and clears the list.
pub fn begin_shadows(ui: &Ui) -> egui::layers::ShapeIdx {
    ui.ctx().data_mut(|d| d.insert_temp(pending_id(), Pending::default()));
    ui.painter().add(Shape::Noop)
}

/// A group, the hero or any raised box at `rect` asks for its shadow; `alpha` fades it with the box.
pub fn add_shadow(ctx: &Context, rect: Rect, alpha: f32) {
    ctx.data_mut(|d| d.get_temp_mut_or_default::<Pending>(pending_id()).0.push((rect, alpha)));
}

/// Draws all requested shadows into the reserved slot: a wide soft one (rect inset 8, offset
/// 10 down, black 60 %) and a tight contact one (offset 2 down, black 28 %).
pub fn end_shadows(ui: &Ui, slot: egui::layers::ShapeIdx) {
    let list = ui.ctx().data_mut(|d| d.remove_temp::<Pending>(pending_id())).unwrap_or_default().0;
    if list.is_empty() {
        return;
    }
    let f = fx(ui.ctx());
    let mut soft = Mesh::with_texture(f.soft.id());
    let mut tight = Mesh::with_texture(f.tight.id());
    for &(r, a) in &list {
        nine(&mut soft, r.shrink(8.0).translate(vec2(0.0, 10.0)).expand(SOFT.1 as f32), SOFT.0, 0.60 * a);
        nine(&mut tight, r.translate(vec2(0.0, 2.0)).expand(TIGHT.1 as f32), TIGHT.0, 0.28 * a);
    }
    ui.painter().set(slot, Shape::Vec(vec![Shape::mesh(soft), Shape::mesh(tight)]));
}

/// 9-slice of a `side`-texel texture over `r` (16 vertices, 9 quads).
fn nine(m: &mut Mesh, r: Rect, side: usize, opacity: f32) {
    if r.width() <= 1.0 || r.height() <= 1.0 || opacity < 0.004 {
        return;
    }
    let cut = (side as f32 / 2.0).min(r.width() / 2.0).min(r.height() / 2.0);
    let uv = cut / side as f32;
    let xs = [r.left(), r.left() + cut, r.right() - cut, r.right()];
    let ys = [r.top(), r.top() + cut, r.bottom() - cut, r.bottom()];
    let us = [0.0, uv, 1.0 - uv, 1.0];
    let tint = Color32::from_white_alpha((opacity.clamp(0.0, 1.0) * 255.0) as u8);
    let base = m.vertices.len() as u32;
    for j in 0..4 {
        for i in 0..4 {
            m.vertices.push(Vertex { pos: pos2(xs[i], ys[j]), uv: pos2(us[i], us[j]), color: tint });
        }
    }
    for j in 0..3u32 {
        for i in 0..3u32 {
            let a = base + j * 4 + i;
            m.indices.extend_from_slice(&[a, a + 1, a + 5, a, a + 5, a + 4]);
        }
    }
}
