//! Custom LFO shapes (SPEC v2 Track 7, G1): a drawn shape of up to [`MAX_POINTS`] points with a
//! per-segment curve, or a step pattern of 2..[`MAX_STEPS`] levels with glide, one per LFO (Mod A..D).
//! An LFO plays its shape when its Curve is "Custom".
//!
//! Points: `[x 0..1, y -1..1, curve -1..1]`, sorted by x. Between two points the value runs from
//! the left point's y to the right one's along `t^(8^curve)` (0 = straight, + sags, - bulges);
//! before the first / after the last point the shape wraps (last -> first of the next cycle).
//! Steps: `n` equal steps holding their level; `glide` 0..1 is the fraction of each step spent
//! ramping (linearly) into the next level (the last glides into the first).
//!
//! Threads: [`ShapesShared`] (`params.lfo_shapes`, persisted as `"lfo_shapes"`) holds the editable
//! [`LfoShapes`] under a mutex that only non-audio threads take, and publishes a fixed
//! `[f32; SHAPES_ENC]` record through an [`rr_ui::snap::Snap`] like the matrix. The audio thread
//! polls it once per block (wait-free, no allocation) and decodes it into each voice's LFOs
//! ([`Shape::decode`], fixed size, `Copy`).

use nih_plug::params::persist::PersistentField;
use rr_ui::snap::Snap;
use serde::{Deserialize, Serialize};
use std::sync::{Mutex, MutexGuard, PoisonError};

pub const MAX_POINTS: usize = 32;
pub const MAX_STEPS: usize = 32;
/// LFOs with a shape slot: Mod A, B, C, D.
pub const LFOS: usize = 4;
/// Encoded shape: mode, points, steps, glide, then x, y, curve per point and the step levels.
pub const ENC: usize = 4 + 3 * MAX_POINTS + MAX_STEPS;
pub const SHAPES_ENC: usize = LFOS * ENC;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Points,
    Steps,
}

/// One editable shape (GUI, state and preset form).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShapeDef {
    pub mode: Mode,
    /// `[x, y, curve]`, sorted by x; `curve` bends the segment that starts at this point.
    pub points: Vec<[f32; 3]>,
    pub steps: Vec<f32>,
    pub glide: f32,
}

impl Default for ShapeDef {
    /// A triangle, and an eight-step pattern for Step mode.
    fn default() -> Self {
        Self {
            mode: Mode::Points,
            points: vec![[0.0, 0.0, 0.0], [0.25, 1.0, 0.0], [0.75, -1.0, 0.0], [1.0, 0.0, 0.0]],
            steps: vec![1.0, -1.0, 0.5, -1.0, 1.0, -0.5, 0.0, -1.0],
            glide: 0.0,
        }
    }
}

fn clamp(x: f32, lo: f32, hi: f32) -> f32 {
    if x.is_finite() { x.clamp(lo, hi) } else { 0.0f32.clamp(lo, hi) }
}

impl ShapeDef {
    pub fn points(points: &[[f32; 3]]) -> Self {
        Self { points: points.to_vec(), ..Self::default() }
    }

    pub fn steps(steps: &[f32], glide: f32) -> Self {
        Self { mode: Mode::Steps, steps: steps.to_vec(), glide, ..Self::default() }
    }

    /// Clamps every value into range, sorts the points, keeps at most 32 points / steps; an
    /// empty point list or fewer than two steps take the default ones.
    pub fn sanitize(&mut self) {
        let d = Self::default();
        for p in &mut self.points {
            *p = [clamp(p[0], 0.0, 1.0), clamp(p[1], -1.0, 1.0), clamp(p[2], -1.0, 1.0)];
        }
        self.points.sort_by(|a, b| a[0].total_cmp(&b[0]));
        self.points.truncate(MAX_POINTS);
        if self.points.is_empty() {
            self.points = d.points;
        }
        for s in &mut self.steps {
            *s = clamp(*s, -1.0, 1.0);
        }
        self.steps.truncate(MAX_STEPS);
        if self.steps.len() < 2 {
            self.steps = d.steps;
        }
        self.glide = clamp(self.glide, 0.0, 1.0);
    }

    /// Value at phase `x` (0..1), through the audio-side shape.
    pub fn eval(&self, x: f64) -> f32 {
        let mut s = self.clone();
        s.sanitize();
        let mut e = [0.0; ENC];
        s.encode_into(&mut e);
        Shape::decode(&e).eval(x)
    }

    fn encode_into(&self, e: &mut [f32]) {
        // Callers pass a sanitized shape.
        e[0] = (self.mode == Mode::Steps) as u8 as f32;
        e[1] = self.points.len() as f32;
        e[2] = self.steps.len() as f32;
        e[3] = self.glide;
        for (k, p) in self.points.iter().enumerate() {
            e[4 + k] = p[0];
            e[4 + MAX_POINTS + k] = p[1];
            e[4 + 2 * MAX_POINTS + k] = p[2];
        }
        for (k, s) in self.steps.iter().enumerate() {
            e[4 + 3 * MAX_POINTS + k] = *s;
        }
    }
}

/// The four shapes, Mod A first. Serialised as a list (missing entries take the default shape,
/// extra ones are dropped).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(from = "Vec<ShapeDef>", into = "Vec<ShapeDef>")]
pub struct LfoShapes(pub [ShapeDef; LFOS]);

impl From<Vec<ShapeDef>> for LfoShapes {
    fn from(v: Vec<ShapeDef>) -> Self {
        let mut s = Self::default();
        for (d, x) in s.0.iter_mut().zip(v) {
            *d = x;
        }
        s.sanitize();
        s
    }
}

impl From<LfoShapes> for Vec<ShapeDef> {
    fn from(s: LfoShapes) -> Self {
        s.0.into()
    }
}

impl LfoShapes {
    pub fn sanitize(&mut self) {
        self.0.iter_mut().for_each(ShapeDef::sanitize);
    }

    pub fn encode(&self) -> [f32; SHAPES_ENC] {
        let mut e = [0.0; SHAPES_ENC];
        for (s, rec) in self.0.iter().zip(e.chunks_exact_mut(ENC)) {
            let mut s = s.clone();
            s.sanitize();
            s.encode_into(rec);
        }
        e
    }
}

/// Audio-side shape: fixed size, `Copy`, evaluated per sample.
#[derive(Clone, Copy, Debug)]
pub struct Shape {
    steps: bool,
    n: usize,
    x: [f32; MAX_POINTS],
    y: [f32; MAX_POINTS],
    c: [f32; MAX_POINTS],
    ns: usize,
    s: [f32; MAX_STEPS],
    glide: f32,
}

impl Default for Shape {
    fn default() -> Self {
        let mut e = [0.0; ENC];
        ShapeDef::default().encode_into(&mut e);
        Self::decode(&e)
    }
}

/// Segment bend: `t^(8^c)`.
#[inline]
fn bend(t: f32, c: f32) -> f32 {
    if c == 0.0 { t } else { t.powf(8f32.powf(c)) }
}

impl Shape {
    /// From one [`ENC`] record (allocation-free; counts are clamped so a bad record cannot index
    /// out of bounds).
    pub fn decode(e: &[f32]) -> Self {
        let count = |v: f32, lo: usize, max: usize| (v.max(0.0) as usize).clamp(lo, max);
        let mut s = Self {
            steps: e[0] > 0.5,
            n: count(e[1], 1, MAX_POINTS),
            x: [0.0; MAX_POINTS],
            y: [0.0; MAX_POINTS],
            c: [0.0; MAX_POINTS],
            ns: count(e[2], 2, MAX_STEPS),
            s: [0.0; MAX_STEPS],
            glide: e[3].clamp(0.0, 1.0),
        };
        s.x.copy_from_slice(&e[4..4 + MAX_POINTS]);
        s.y.copy_from_slice(&e[4 + MAX_POINTS..4 + 2 * MAX_POINTS]);
        s.c.copy_from_slice(&e[4 + 2 * MAX_POINTS..4 + 3 * MAX_POINTS]);
        s.s.copy_from_slice(&e[4 + 3 * MAX_POINTS..ENC]);
        s
    }

    /// Value at phase `x` (0..1; 1 is the end of the cycle).
    #[inline]
    pub fn eval(&self, x: f64) -> f32 {
        let x = x as f32;
        if self.steps {
            let n = self.ns;
            if x >= 1.0 {
                return self.s[n - 1];
            }
            let p = x.max(0.0) * n as f32;
            let k = (p as usize).min(n - 1);
            let (f, v) = (p - k as f32, self.s[k]);
            let g = self.glide;
            if g > 0.0 && f > 1.0 - g {
                return v + (self.s[(k + 1) % n] - v) * ((f - (1.0 - g)) / g).min(1.0);
            }
            return v;
        }
        let n = self.n;
        let (xs, ys) = (&self.x[..n], &self.y[..n]);
        let i = xs.partition_point(|&px| px <= x);
        // (left point, right point, their x with the wrap applied)
        let (l, r, xl, xr) = match i {
            0 => (n - 1, 0, xs[n - 1] - 1.0, xs[0]),
            _ if i == n => (n - 1, 0, xs[n - 1], xs[0] + 1.0),
            _ => (i - 1, i, xs[i - 1], xs[i]),
        };
        let t = if xr > xl { ((x - xl) / (xr - xl)).clamp(0.0, 1.0) } else { 0.0 };
        ys[l] + (ys[r] - ys[l]) * bend(t, self.c[l])
    }
}

// ---- stock library ---------------------------------------------------------------------------

/// The stock shapes (name, shape), loadable into any LFO's Custom slot.
pub fn library() -> Vec<(&'static str, ShapeDef)> {
    let p = ShapeDef::points;
    let s = ShapeDef::steps;
    vec![
        ("Ramp and Hold", p(&[[0.0, -1.0, 0.0], [0.5, 1.0, 0.0], [0.999, 1.0, 0.0], [1.0, -1.0, 0.0]])),
        ("Stepped Triangle", s(&[-1.0, -0.5, 0.0, 0.5, 1.0, 0.5, 0.0, -0.5], 0.0)),
        ("Bounce", p(&[[0.0, 1.0, 0.6], [0.4, -1.0, -0.6], [0.6, 0.3, 0.6], [0.75, -1.0, -0.6], [0.85, 0.0, 0.6], [0.92, -1.0, -0.6], [1.0, 1.0, 0.0]])),
        ("Riddim Gate 1", s(&[1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0], 0.05)),
        ("Riddim Gate 2", s(&[1.0, 1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0], 0.05)),
        ("Riddim Triplet", s(&[1.0, -1.0, 1.0, 1.0, -1.0, -1.0], 0.05)),
        ("Trance Gate", s(&[1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0], 0.1)),
        ("Pump", p(&[[0.0, -1.0, -0.7], [0.6, 1.0, 0.0], [1.0, 1.0, 0.0]])),
        ("Wub Dip", p(&[[0.0, 1.0, 0.5], [0.3, -1.0, -0.5], [0.7, -1.0, 0.0], [1.0, 1.0, 0.0]])),
        ("Double Hump", p(&[[0.0, -1.0, -0.4], [0.25, 1.0, 0.4], [0.5, -1.0, -0.4], [0.75, 0.5, 0.4], [1.0, -1.0, 0.0]])),
        ("Stairs Down", s(&[1.0, 0.6, 0.2, -0.2, -0.6, -1.0], 0.0)),
        ("Stairs Up", s(&[-1.0, -0.6, -0.2, 0.2, 0.6, 1.0], 0.0)),
        ("Random Steps", s(&[0.3, -0.8, 0.9, -0.2, 0.6, -1.0, 0.1, 0.8, -0.5, 0.4, -0.9, 0.7, -0.1, 1.0, -0.6, 0.2], 0.0)),
        ("Exp Decay", p(&[[0.0, 1.0, 0.8], [1.0, -1.0, 0.0]])),
        ("Call and Response", p(&[[0.0, 1.0, 0.0], [0.2, 1.0, 0.0], [0.25, -1.0, 0.0], [0.5, -1.0, 0.0], [0.55, 0.5, 0.0], [0.65, -0.5, 0.0], [0.75, 0.5, 0.0], [0.85, -1.0, 0.0], [1.0, -1.0, 0.0]])),
    ]
}

// ---- shared state ----------------------------------------------------------------------------

/// `params.lfo_shapes`: the editable shapes (non-audio threads) and their audio-side snapshot.
pub struct ShapesShared {
    state: Mutex<LfoShapes>,
    snap: Snap,
}

impl Default for ShapesShared {
    fn default() -> Self {
        let s = LfoShapes::default();
        let snap = Snap::new(SHAPES_ENC);
        snap.write(&s.encode());
        Self { state: Mutex::new(s), snap }
    }
}

impl ShapesShared {
    fn lock(&self) -> MutexGuard<'_, LfoShapes> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// GUI: a copy of the shapes.
    pub fn get(&self) -> LfoShapes {
        self.lock().clone()
    }

    /// GUI / preset load / state load: replaces the shapes (sanitized) and publishes them.
    pub fn replace(&self, s: LfoShapes) {
        self.edit(|cur| *cur = s);
    }

    /// GUI: edits in place, sanitizes and publishes (one snapshot write per change).
    pub fn edit<R>(&self, f: impl FnOnce(&mut LfoShapes) -> R) -> R {
        let mut g = self.lock();
        let before = g.clone();
        let r = f(&mut g);
        g.sanitize();
        if *g != before {
            self.snap.write(&g.encode());
        }
        r
    }

    /// Audio thread, once per block: copies a newer record than `seen` into `out`. Wait-free,
    /// allocation-free; a read that raced a write returns false and is retried next block.
    pub fn poll(&self, seen: &mut u32, out: &mut [f32; SHAPES_ENC]) -> bool {
        let v = self.snap.version();
        if v == *seen || !self.snap.read(out) {
            return false;
        }
        *seen = v;
        true
    }
}

impl<'a> PersistentField<'a, LfoShapes> for ShapesShared {
    fn set(&self, new_value: LfoShapes) {
        self.replace(new_value);
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&LfoShapes) -> R,
    {
        f(&self.lock())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_has_at_least_12_valid_shapes() {
        let lib = library();
        assert!(lib.len() >= 12);
        for (name, d) in lib {
            let mut s = d.clone();
            s.sanitize();
            assert_eq!(s, d, "{name} is not sanitized");
            for k in 0..=100 {
                let v = d.eval(k as f64 / 100.0);
                assert!((-1.0..=1.0).contains(&v), "{name}: {v}");
            }
        }
    }
}
