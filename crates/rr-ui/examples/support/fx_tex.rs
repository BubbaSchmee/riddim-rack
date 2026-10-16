//! Prototype: generated textures for a more physical look, all through plain
//! `ctx.load_texture` (no GL code, works in nih_plug_egui as-is).
//!
//!  * soft drop shadow: a pre-blurred rounded rect, drawn as a 9-slice so ONE texture serves any
//!    rect size (9 quads, 16 vertices)
//!  * radial glow sprite: drawn tinted; with an alpha-0 premultiplied tint it blends ADDITIVELY
//!    (egui blends premultiplied: out = src + dst * (1 - src.a))
//!  * brushed-metal noise tile: repeat-wrapped, drawn over a panel as ONE quad
//!
//! Colour space: egui textures are sRGB bytes, blending happens in gamma space.

use eframe::egui::epaint::{Mesh, Vertex};
use eframe::egui::{self, pos2, Color32, ColorImage, Context, Painter, Rect, TextureFilter, TextureHandle, TextureOptions, TextureWrapMode, Vec2};

pub struct Fx {
    pub shadow: TextureHandle,
    pub glow: TextureHandle,
    pub noise: TextureHandle,
    /// Shadow texture side and the width of its corner/edge band, in texels (1 texel = 1 pt at ppp 1).
    pub shadow_side: usize,
    pub shadow_band: usize,
    pub noise_side: usize,
    pub gen_ms: [f64; 3],
}

fn gauss(sigma: f32) -> Vec<f32> {
    let r = (sigma * 3.0).ceil() as i32;
    let mut k: Vec<f32> = (-r..=r).map(|i| (-(i * i) as f32 / (2.0 * sigma * sigma)).exp()).collect();
    let s: f32 = k.iter().sum();
    k.iter_mut().for_each(|v| *v /= s);
    k
}

/// Separable gaussian blur of a w*h f32 image, clamped edges.
fn blur(a: &mut [f32], w: usize, h: usize, sigma: f32) {
    let k = gauss(sigma);
    let r = (k.len() / 2) as i32;
    let mut tmp = vec![0.0; a.len()];
    for y in 0..h {
        for x in 0..w {
            tmp[y * w + x] = (-r..=r).map(|i| a[y * w + (x as i32 + i).clamp(0, w as i32 - 1) as usize] * k[(i + r) as usize]).sum();
        }
    }
    for y in 0..h {
        for x in 0..w {
            a[y * w + x] = (-r..=r).map(|i| tmp[(y as i32 + i).clamp(0, h as i32 - 1) as usize * w + x] * k[(i + r) as usize]).sum();
        }
    }
}

/// Black shadow: a rounded rect of `radius` inset by `band` texels, blurred with `sigma`.
/// Alpha-only (premultiplied black), so tint with `Color32::from_black_alpha` or any alpha.
pub fn shadow_image(side: usize, band: usize, radius: f32, sigma: f32) -> ColorImage {
    let mut a = vec![0.0f32; side * side];
    let (lo, hi) = (band as f32, (side - band) as f32);
    for y in 0..side {
        for x in 0..side {
            // signed distance to the rounded rect, 1 px coverage ramp
            let (px, py) = (x as f32 + 0.5 - side as f32 / 2.0, y as f32 + 0.5 - side as f32 / 2.0);
            let half = (hi - lo) / 2.0 - radius;
            let (dx, dy) = (px.abs() - half, py.abs() - half);
            let d = (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt() + dx.max(dy).min(0.0) - radius;
            a[y * side + x] = (0.5 - d).clamp(0.0, 1.0);
        }
    }
    blur(&mut a, side, side, sigma);
    ColorImage { size: [side, side], pixels: a.iter().map(|v| Color32::from_black_alpha((v * 255.0 + 0.5) as u8)).collect() }
}

/// Radial falloff, premultiplied white: smooth gaussian-ish core, exactly 0 at the edge.
pub fn glow_image(side: usize) -> ColorImage {
    let c = (side as f32 - 1.0) / 2.0;
    let pixels = (0..side * side)
        .map(|i| {
            let (x, y) = ((i % side) as f32 - c, (i / side) as f32 - c);
            let r = ((x * x + y * y).sqrt() / c).min(1.0);
            let f = (1.0 - r * r).max(0.0).powi(3); // smooth, zero slope at the rim
            let v = (f * 255.0 + 0.5) as u8;
            Color32::from_rgba_premultiplied(v, v, v, v)
        })
        .collect();
    ColorImage { size: [side, side], pixels }
}

/// Tileable brushed-metal streaks: per-row random brightness blurred horizontally (wrapping),
/// plus fine grain. Premultiplied white with low alpha, so it lightens the panel underneath.
pub fn noise_image(side: usize, seed: u32) -> ColorImage {
    let mut s = seed.wrapping_mul(2654435761).max(1);
    let mut rnd = move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        (s >> 8) as f32 / (1u32 << 24) as f32
    };
    let mut v = vec![0.0f32; side * side];
    for y in 0..side {
        let row: Vec<f32> = (0..side).map(|_| rnd()).collect();
        for x in 0..side {
            // horizontal wrap-around box blur over 32 texels: long streaks
            let mut acc = 0.0;
            for k in 0..32 {
                acc += row[(x + k) % side];
            }
            v[y * side + x] = acc / 32.0 * 0.8 + rnd() * 0.2;
        }
    }
    let pixels = v
        .iter()
        .map(|&n| {
            let a = ((n - 0.5).abs() * 2.0 * 28.0 + 3.0) as u8; // alpha 3..31 of 255
            if n > 0.5 { Color32::from_rgba_premultiplied(a, a, a, a) } else { Color32::from_black_alpha(a) }
        })
        .collect();
    ColorImage { size: [side, side], pixels }
}

impl Fx {
    pub fn new(ctx: &Context) -> Self {
        let t = std::time::Instant::now();
        let (shadow_side, shadow_band) = (72, 28);
        let shadow = ctx.load_texture("rr-shadow", shadow_image(shadow_side, shadow_band, 6.0, 7.0), TextureOptions::LINEAR);
        let t1 = t.elapsed().as_secs_f64() * 1e3;
        let glow = ctx.load_texture("rr-glow", glow_image(128), TextureOptions::LINEAR);
        let t2 = t.elapsed().as_secs_f64() * 1e3 - t1;
        let noise_side = 256;
        let noise = ctx.load_texture(
            "rr-noise",
            noise_image(noise_side, 7),
            TextureOptions { magnification: TextureFilter::Linear, minification: TextureFilter::Linear, wrap_mode: TextureWrapMode::Repeat, mipmap_mode: None },
        );
        let t3 = t.elapsed().as_secs_f64() * 1e3 - t1 - t2;
        Self { shadow, glow, noise, shadow_side, shadow_band, noise_side, gen_ms: [t1, t2, t3] }
    }

    /// Soft shadow under `rect`: the blurred rounded rect fits `rect` expanded by `band`.
    pub fn drop_shadow(&self, p: &Painter, rect: Rect, offset: Vec2, opacity: f32) {
        let r = rect.translate(offset).expand(self.shadow_band as f32);
        let m = (self.shadow_side as f32 / 2.0).min(r.width() / 2.0).min(r.height() / 2.0);
        let uvm = m / self.shadow_side as f32;
        let xs = [r.left(), r.left() + m, r.right() - m, r.right()];
        let ys = [r.top(), r.top() + m, r.bottom() - m, r.bottom()];
        let us = [0.0, uvm, 1.0 - uvm, 1.0];
        let tint = Color32::from_white_alpha((opacity * 255.0) as u8);
        let mut mesh = Mesh::with_texture(self.shadow.id());
        for j in 0..4 {
            for i in 0..4 {
                mesh.vertices.push(Vertex { pos: pos2(xs[i], ys[j]), uv: pos2(us[i], us[j]), color: tint });
            }
        }
        for j in 0..3u32 {
            for i in 0..3u32 {
                let a = j * 4 + i;
                mesh.indices.extend_from_slice(&[a, a + 1, a + 5, a, a + 5, a + 4]);
            }
        }
        p.add(egui::Shape::mesh(mesh));
    }

    /// Additive glow sprite centred on `c`, diameter `d`, colour `rgb` scaled by `k`.
    pub fn glow_at(&self, p: &Painter, c: egui::Pos2, d: f32, rgb: Color32, k: f32) {
        let tint = Color32::from_rgba_premultiplied((rgb.r() as f32 * k) as u8, (rgb.g() as f32 * k) as u8, (rgb.b() as f32 * k) as u8, 0);
        p.image(self.glow.id(), Rect::from_center_size(c, Vec2::splat(d)), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), tint);
    }

    /// Brushed texture tiled over `rect` as a single quad.
    pub fn brushed(&self, p: &Painter, rect: Rect, k: f32) {
        let s = self.noise_side as f32;
        p.image(self.noise.id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(rect.width() / s, rect.height() / s)), Color32::from_white_alpha((k * 255.0) as u8));
    }
}
