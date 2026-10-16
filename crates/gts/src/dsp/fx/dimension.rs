//! FX 4.5 Dimension: a mid/side Haas widener that cannot hurt the mono sum.
//!
//! A classic dimension design (four delay lines summed out of phase, slowly amplitude-modulated): the
//! mid signal `(l + r) / 2` goes through the wet low cut (48 dB/oct) into one delay line read by
//! four taps; their alternating-sign sum `D` is added to the left and subtracted from the right:
//! `L = l + w D`, `R = r - w D`. So `L + R = l + r` exactly: the mono sum is the dry mono sum at
//! every setting, and the sub (cut out of `D`) stays centred.
//! - Size moves the taps from 3..6 ms to 7..17 ms and sets the amount with mix:
//!   `w = mix * (0.2 + 0.8 * size)`. Bigger size = more side energy = lower L/R correlation.
//! - Each tap pair's gain wobbles +-25 % at its own slow rate (0.13, 0.21 Hz) so the image breathes.
//! - The taps' signs sum to zero and their delays balance, so `D` has a zero at DC (a double one
//!   while the two wobbles agree) on top of the low cut: the sub stays mono.
//! - Size, amount and the low cut glide per sample (two-pole). `on = false` glides the amount to 0,
//!   then the module is a bit-exact pass-through that costs nothing; waking refills the line before
//!   the side fades back in. No allocation in `process`.
use std::f64::consts::TAU;

const BUF: usize = 8192;
const MASK: usize = BUF - 1;
/// Tap delays at size 0 and size 1, seconds. Signs alternate + - + -, and d0 + d2 = d1 + d3 at
/// every size, so with equal pair gains `D` has a double zero at DC.
const TAP_MIN: [f64; 4] = [0.0030, 0.0041, 0.0063, 0.0052];
const TAP_MAX: [f64; 4] = [0.0071, 0.0103, 0.0169, 0.0137];
const TAP_SIGN: [f64; 4] = [0.5, -0.5, 0.5, -0.5];
/// Gain wobble per tap pair (0+1, 2+3; a pair shares it so the DC zero survives): Hz and depth.
const WOBBLE_HZ: [f64; 2] = [0.13, 0.21];
const WOBBLE: f64 = 0.25;
/// Per-stage glide time constants, seconds.
const SMOOTH_S: f64 = 0.005;
const SIZE_SMOOTH_S: f64 = 0.025;
/// Butterworth 8th order section dampings (k = 1/Q): 48 dB/oct, so even the octave just under
/// the low cut adds next to no side energy.
const HP_K: [f64; 4] = [1.961_570_560_806_461, 1.662_939_224_605_090_5, 1.111_140_466_039_204_4, 0.390_180_644_032_256_6];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DimensionParams {
    pub on: bool,
    /// 0..1.
    pub size: f32,
    /// 20..500 Hz.
    pub wet_lowcut_hz: f32,
    /// 0..1.
    pub mix: f32,
}

impl Default for DimensionParams {
    fn default() -> Self {
        Self { on: false, size: 0.5, wet_lowcut_hz: 120.0, mix: 0.5 }
    }
}

// ponytail: Glide, Svf and hermite repeat the chorus copies so each FX file stands alone; the
// integration wave can hoist them into fx/mod.rs.

/// Two cascaded one-poles (no slope step when the target jumps, so no click), exact arrival.
#[derive(Clone, Copy)]
struct Glide {
    v: f64,
    q: f64,
    t: f64,
}

impl Glide {
    const fn at(v: f64) -> Self {
        Self { v, q: v, t: v }
    }
    fn jump(&mut self) {
        self.v = self.t;
        self.q = self.t;
    }
    #[inline]
    fn tick(&mut self, a: f64) -> f64 {
        if self.v != self.t {
            self.q += (self.t - self.q) * a;
            self.v += (self.q - self.v) * a;
            if (self.t - self.v).abs() + (self.t - self.q).abs() < 1e-6 * (1.0 + self.t.abs()) {
                self.jump();
            }
        }
        self.v
    }
}

/// TPT state-variable high-pass (A. Simper).
#[derive(Clone, Copy, Default)]
struct Svf {
    k: f64,
    a1: f64,
    a2: f64,
    a3: f64,
}

impl Svf {
    fn new(hz: f64, sr: f64, k: f64) -> Self {
        let g = (std::f64::consts::PI * hz.min(0.49 * sr) / sr).tan();
        let a1 = 1.0 / (1.0 + g * (g + k));
        Self { k, a1, a2: g * a1, a3: g * g * a1 }
    }
    #[inline]
    fn highpass(&self, z: &mut [f64; 2], x: f64) -> f64 {
        let v3 = x - z[1];
        let v1 = self.a1 * z[0] + self.a2 * v3;
        let v2 = z[1] + self.a2 * z[0] + self.a3 * v3;
        z[0] = 2.0 * v1 - z[0];
        z[1] = 2.0 * v2 - z[1];
        x - self.k * v1 - v2
    }
}

#[inline]
fn hermite(y0: f32, y1: f32, y2: f32, y3: f32, t: f32) -> f32 {
    let c1 = 0.5 * (y2 - y0);
    let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
    let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);
    ((c3 * t + c2) * t + c1) * t + y1
}

pub struct Dimension {
    sr: f64,
    a: f64,
    a_size: f64,
    buf: Vec<f32>,
    w: usize,
    size: Glide,
    /// Side amount `w`.
    amount: Glide,
    /// log2 Hz.
    lowcut: Glide,
    hp: [Svf; 4],
    hp_z: [[f64; 2]; 4],
    /// Wobble phasors (cos, sin) and their per-sample rotation.
    rot: [(f64, f64); 2],
    osc: [(f64, f64); 2],
    n: u32,
    /// The line holds audio from before the last bypass.
    stale: bool,
    /// Samples left to fill the line after waking.
    prime: usize,
}

impl Dimension {
    pub fn new() -> Self {
        let mut d = Self {
            sr: 48000.0,
            a: 1.0,
            a_size: 1.0,
            buf: vec![0.0; BUF],
            w: 0,
            size: Glide::at(0.5),
            amount: Glide::at(0.0),
            lowcut: Glide::at(120f64.log2()),
            hp: [Svf::default(); 4],
            hp_z: [[0.0; 2]; 4],
            rot: [(1.0, 0.0); 2],
            osc: [(1.0, 0.0); 2],
            n: 0,
            stale: false,
            prime: 0,
        };
        d.set(&DimensionParams::default());
        d.set_sample_rate(48000.0);
        d
    }

    pub fn set_sample_rate(&mut self, sr: f64) {
        self.sr = sr;
        self.a = 1.0 - (-1.0 / (SMOOTH_S * sr)).exp();
        self.a_size = 1.0 - (-1.0 / (SIZE_SMOOTH_S * sr)).exp();
        self.rot = WOBBLE_HZ.map(|f| ((TAU * f / sr).cos(), (TAU * f / sr).sin()));
        self.reset();
    }

    pub fn latency(&self) -> usize {
        0
    }

    /// True while the module is a bit-exact pass-through (off or mix 0, glides finished).
    pub fn is_bypassed(&self) -> bool {
        self.amount.v == 0.0 && self.amount.t == 0.0
    }

    pub fn set(&mut self, p: &DimensionParams) {
        let size = p.size.clamp(0.0, 1.0) as f64;
        self.size.t = size;
        self.amount.t = if p.on { p.mix.clamp(0.0, 1.0) as f64 * (0.2 + 0.8 * size) } else { 0.0 };
        self.lowcut.t = (p.wet_lowcut_hz.clamp(20.0, 500.0) as f64).log2();
    }

    /// Clear the line and filters, restart the wobble and jump every glide to its target.
    pub fn reset(&mut self) {
        self.size.jump();
        self.amount.jump();
        self.lowcut.jump();
        self.clear();
        self.update_filters();
    }

    fn clear(&mut self) {
        self.buf.fill(0.0);
        self.hp_z = [[0.0; 2]; 4];
        // Start the wobbles a quarter turn apart.
        self.osc = [(1.0, 0.0), (0.0, 1.0)];
        self.stale = false;
        self.prime = (TAP_MAX[2] * self.sr) as usize + 4;
    }

    fn update_filters(&mut self) {
        let hz = self.lowcut.v.exp2();
        self.hp = HP_K.map(|k| Svf::new(hz, self.sr, k));
    }

    #[inline]
    fn read(&self, d: f64) -> f32 {
        let d = d.clamp(3.0, (BUF - 4) as f64);
        let pos = self.w as f64 + BUF as f64 - d;
        let i = pos as usize;
        let t = (pos - i as f64) as f32;
        let b = &self.buf;
        hermite(b[(i - 1) & MASK], b[i & MASK], b[(i + 1) & MASK], b[(i + 2) & MASK], t)
    }

    #[inline]
    pub fn process(&mut self, l: &mut f32, r: &mut f32) {
        if self.is_bypassed() {
            self.stale = true;
            return;
        }
        if self.stale {
            self.clear();
        }
        let mut h = 0.5 * (*l as f64 + *r as f64);
        for (f, z) in self.hp.iter().zip(&mut self.hp_z) {
            h = f.highpass(z, h);
        }
        self.buf[self.w] = h as f32;
        if self.prime > 0 {
            // Waking from bypass: fill the line before the side fades in, so the line's start-up
            // edge has passed every tap by then (it clicked at -55 dBFS otherwise).
            self.prime -= 1;
            self.w = (self.w + 1) & MASK;
            return;
        }
        let size = self.size.tick(self.a_size);
        let amount = self.amount.tick(self.a);
        if self.lowcut.v != self.lowcut.t {
            self.lowcut.tick(self.a);
            self.update_filters();
        }
        let mut d = 0.0;
        for k in 0..4 {
            let delay = (TAP_MIN[k] + size * (TAP_MAX[k] - TAP_MIN[k])) * self.sr;
            d += TAP_SIGN[k] * (1.0 + WOBBLE * self.osc[k / 2].1) * self.read(delay) as f64;
        }
        for (o, r) in self.osc.iter_mut().zip(self.rot) {
            *o = (o.0 * r.0 - o.1 * r.1, o.1 * r.0 + o.0 * r.1);
        }
        self.w = (self.w + 1) & MASK;
        let side = (amount * d) as f32;
        *l += side;
        *r -= side;

        self.n = self.n.wrapping_add(1);
        if self.n & 63 == 0 {
            // Keep the wobble phasors on the unit circle and the filter states out of denormals.
            for (c, s) in &mut self.osc {
                let g = 1.0 / (*c * *c + *s * *s).sqrt();
                *c *= g;
                *s *= g;
            }
            for z in self.hp_z.iter_mut().flatten() {
                if z.abs() < 1e-30 {
                    *z = 0.0;
                }
            }
        }
    }
}

impl Default for Dimension {
    fn default() -> Self {
        Self::new()
    }
}
