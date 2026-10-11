//! FX 4.3 EQ: low shelf, peak, high shelf and a switchable 24 dB/oct low cut, stereo, no Mix.
//!
//! Each band is the RBJ-cookbook analog prototype turned into a biquad by magnitude matching
//! (M. Vicanek, "Matched Second Order Digital Filters", 2016): poles by impulse invariance, zeros
//! fitted to the analog magnitude at DC, Nyquist and the corner. Unlike the bilinear transform this
//! does not cramp near Nyquist: a +15 dB bell at 10 kHz at 48 kHz keeps its analog shape instead of
//! being 4 dB off at 16 kHz.
//!
//! Each section runs as a TPT state-variable filter carrying those matched coefficients
//! ([`to_svf`]). Smoothing: a new value redesigns the target coefficients once, then the SVF
//! parameters `[g, k, m0, m1, m2]` glide per sample (two cascaded one-poles, 5 ms each, so a
//! target jump does not step the slope either). The SVF is stable for any positive g, k, so every
//! point on the glide is stable, and its states stay meaningful while the coefficients move, so
//! jumps do not click (a direct form ramped per block clicked at -42 dBFS above 10 kHz on a
//! full-range high-shelf jump; this measures below -100 dBFS).
//! A band at 0 dB (or `on = false`) glides its mix to pass-through and then sleeps, so all gains 0
//! is bit-exact pass-through at no cost. No allocation anywhere.
use std::f64::consts::PI;

/// Time constant of each of the two glide stages, seconds (90 % in about 20 ms).
const SMOOTH_S: f64 = 0.005;
/// Butterworth 4th order = two 2nd-order sections with these Qs.
const CUT_Q: [f64; 2] = [0.541_196_100_146_197, 1.306_562_964_876_376_7];
/// Shelf Q (RBJ slope S = 1).
const SHELF_Q: f64 = std::f64::consts::FRAC_1_SQRT_2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EqParams {
    pub on: bool,
    /// Low shelf corner 30..500 Hz, gain -15..15 dB.
    pub low_freq: f32,
    pub low_gain: f32,
    /// Peak 20 Hz..20 kHz, Q 0.2..10, gain -15..15 dB.
    pub peak_freq: f32,
    pub peak_q: f32,
    pub peak_gain: f32,
    /// High shelf corner 1..20 kHz, gain -15..15 dB.
    pub high_freq: f32,
    pub high_gain: f32,
    /// Low cut (24 dB/oct Butterworth high-pass) 20..500 Hz.
    pub cut_on: bool,
    pub cut_freq: f32,
}

impl Default for EqParams {
    fn default() -> Self {
        Self {
            on: false,
            low_freq: 100.0,
            low_gain: 0.0,
            peak_freq: 1000.0,
            peak_q: 1.0,
            peak_gain: 0.0,
            high_freq: 8000.0,
            high_gain: 0.0,
            cut_on: false,
            cut_freq: 30.0,
        }
    }
}

/// Analog prototype `(n2 s^2 + n1 s + n0) / (d2 s^2 + d1 s + d0)` with `s = j f / f0`.
pub type Proto = [f64; 6];

/// RBJ shapes; `a = 10^(dB/40)`.
pub fn proto_low_shelf(db: f64) -> Proto {
    let a = 10f64.powf(db / 40.0);
    let k = a.sqrt() / SHELF_Q;
    [a, a * k, a * a, a, k, 1.0]
}
pub fn proto_high_shelf(db: f64) -> Proto {
    let a = 10f64.powf(db / 40.0);
    let k = a.sqrt() / SHELF_Q;
    [a * a, a * k, a, 1.0, k, a]
}
pub fn proto_peak(db: f64, q: f64) -> Proto {
    let a = 10f64.powf(db / 40.0);
    [1.0, a / q, 1.0, 1.0, 1.0 / (a * q), 1.0]
}
pub fn proto_highpass(q: f64) -> Proto {
    [1.0, 0.0, 0.0, 1.0, 1.0 / q, 1.0]
}
/// The low cut's two sections.
pub fn proto_low_cut() -> [Proto; 2] {
    [proto_highpass(CUT_Q[0]), proto_highpass(CUT_Q[1])]
}

/// |H(j x)|^2 of a prototype, `x = f / f0`.
pub fn proto_mag2(p: &Proto, x: f64) -> f64 {
    let [n2, n1, n0, d2, d1, d0] = *p;
    let (nr, ni) = (n0 - n2 * x * x, n1 * x);
    let (dr, di) = (d0 - d2 * x * x, d1 * x);
    (nr * nr + ni * ni) / (dr * dr + di * di)
}

/// Magnitude-matched biquad `[b0, b1, b2, a1, a2]` for a prototype with corner `f0` Hz.
pub fn matched(p: &Proto, f0: f64, sr: f64) -> [f64; 5] {
    let [_, _, _, d2, d1, d0] = *p;
    // Poles: impulse invariance of s^2 + 2 zeta wp s + wp^2.
    let wp = 2.0 * PI * f0 / sr * (d0 / d2).sqrt();
    let zeta = d1 / (2.0 * (d0 * d2).sqrt());
    let r = (-zeta * wp).exp();
    let a1 = if zeta <= 1.0 {
        -2.0 * r * (wp * (1.0 - zeta * zeta).sqrt()).cos()
    } else {
        -2.0 * r * (wp * (zeta * zeta - 1.0).sqrt()).cosh()
    };
    let a2 = r * r;
    // |A|^2 = A0 phi0 + A1 phi1 + A2 phi2 with phi1 = sin^2(w/2), phi0 = 1 - phi1, phi2 = 4 phi0 phi1.
    let (ca0, ca1, ca2) = ((1.0 + a1 + a2).powi(2), (1.0 - a1 + a2).powi(2), -4.0 * a2);
    let cb0 = proto_mag2(p, 0.0) * ca0;
    let cb1 = proto_mag2(p, sr / (2.0 * f0)) * ca1;
    // Third match point: the corner (kept off Nyquist, where phi2 -> 0).
    let fm = f0.min(0.45 * sr);
    let ph1 = (PI * fm / sr).sin().powi(2);
    let ph0 = 1.0 - ph1;
    let ph2 = 4.0 * ph0 * ph1;
    let cb2 = (proto_mag2(p, fm / f0) * (ca0 * ph0 + ca1 * ph1 + ca2 * ph2) - cb0 * ph0 - cb1 * ph1) / ph2;
    let (s0, s1) = (cb0.sqrt(), cb1.sqrt());
    if p[1] == 0.0 && p[2] == 0.0 {
        // High-pass: keep the analog double zero at DC, match Nyquist only.
        return [s1 / 4.0, -s1 / 2.0, s1 / 4.0, a1, a2];
    }
    let w = 0.5 * (s0 + s1);
    // Unreachable third point: take the nearest realisable numerator (DC and Nyquist stay exact).
    let cb2 = cb2.max(-w * w);
    let b0 = 0.5 * (w + (w * w + cb2).sqrt());
    let b1 = 0.5 * (s0 - s1);
    let b2 = -cb2 / (4.0 * b0);
    [b0, b1, b2, a1, a2]
}

/// 1 / H. The matched numerator is minimum phase (b0 > |b2|, |b1| < b0 + b2), so this is stable.
fn invert(c: [f64; 5]) -> [f64; 5] {
    let [b0, b1, b2, a1, a2] = c;
    [1.0 / b0, a1 / b0, a2 / b0, b1 / b0, b2 / b0]
}

// The RBJ shapes satisfy H(-dB) = 1 / H(+dB). Matching fits one side of each pair much better
// near Nyquist (measured: a +15 dB shelf at 20 kHz, 48 kHz, is 14 dB off designed directly and
// 1.2 dB off as an inverted cut), so shelf boosts are inverted cuts and bell cuts inverted boosts.

/// Low shelf biquad, `db` in -15..15.
pub fn low_shelf(db: f64, f0: f64, sr: f64) -> [f64; 5] {
    if db > 0.0 { invert(matched(&proto_low_shelf(-db), f0, sr)) } else { matched(&proto_low_shelf(db), f0, sr) }
}
pub fn high_shelf(db: f64, f0: f64, sr: f64) -> [f64; 5] {
    if db > 0.0 { invert(matched(&proto_high_shelf(-db), f0, sr)) } else { matched(&proto_high_shelf(db), f0, sr) }
}
pub fn peak(db: f64, q: f64, f0: f64, sr: f64) -> [f64; 5] {
    if db < 0.0 { invert(matched(&proto_peak(-db, q), f0, sr)) } else { matched(&proto_peak(db, q), f0, sr) }
}
pub fn low_cut(f0: f64, sr: f64) -> [[f64; 5]; 2] {
    proto_low_cut().map(|p| matched(&p, f0, sr))
}

/// Re-express a digital biquad `[b0, b1, b2, a1, a2]` as a TPT state-variable filter
/// `[g, k, m0, m1, m2]`, `y = m0 x + m1 band + m2 low` (A. Simper's form). The bilinear map is a
/// bijection on stable second-order sections, so this is exact; the point is the topology: the SVF
/// is stable for every g, k > 0 and its states (integrator outputs) stay meaningful while the
/// coefficients move, so modulation does not click the way a direct form does.
pub fn to_svf(c: [f64; 5]) -> [f64; 5] {
    let [b0, b1, b2, a1, a2] = c;
    let q = 1.0 - a1 + a2;
    let g = ((1.0 + a1 + a2) / q).sqrt();
    let k = 2.0 * (1.0 - a2) / (q * g);
    let (n0, n1, n2) = ((b0 + b1 + b2) / (g * g * q), 2.0 * (b0 - b2) / (g * q), (b0 - b1 + b2) / q);
    [g, k, n2, n1 - n2 * k, n0 - n2]
}

/// Pass-through mix `[m0, m1, m2]`.
const THROUGH: [f64; 3] = [1.0, 0.0, 0.0];

/// TPT SVF section, f64, stereo state; its parameters `[g, k, m0, m1, m2]` glide per sample.
#[derive(Clone, Copy)]
struct Section {
    p: [f64; 5],
    /// First stage of the two-pole glide (p follows q follows t: no slope step when t jumps).
    q: [f64; 5],
    t: [f64; 5],
    /// Integrator gains from g, k.
    a: [f64; 3],
    /// Integrator states per channel.
    z: [[f64; 2]; 2],
    /// Awake (asleep = exact pass-through, state zero).
    on: bool,
    settled: bool,
}

impl Section {
    const fn new() -> Self {
        Self { p: [1.0, 1.0, 1.0, 0.0, 0.0], q: [1.0, 1.0, 1.0, 0.0, 0.0], t: [1.0, 1.0, 1.0, 0.0, 0.0], a: [0.0; 3], z: [[0.0; 2]; 2], on: false, settled: true }
    }

    fn gains(&mut self) {
        let [g, k, ..] = self.p;
        let a1 = 1.0 / (1.0 + g * (g + k));
        self.a = [a1, g * a1, g * g * a1];
    }

    /// New target: the biquad `c`, or pass-through (`None`, keeping the poles).
    fn target(&mut self, c: Option<[f64; 5]>) {
        match c {
            Some(c) => {
                self.t = to_svf(c);
                if !self.on {
                    // Wake from pass-through: target poles, mix fading in from identity.
                    *self = Self { t: self.t, on: true, ..Self::new() };
                    self.p = [self.t[0], self.t[1], 1.0, 0.0, 0.0];
                    self.q = self.p;
                    self.gains();
                }
            }
            None => {
                let [g, k, ..] = self.t;
                self.t = [g, k, THROUGH[0], THROUGH[1], THROUGH[2]];
            }
        }
        self.settled = self.p == self.t;
    }

    /// Jump to the target.
    fn land(&mut self) {
        self.p = self.t;
        self.q = self.t;
        self.z = [[0.0; 2]; 2];
        self.on = self.t[2..] != THROUGH;
        self.settled = true;
        self.gains();
    }

    #[inline]
    fn tick(&mut self, ch: usize, x: f64) -> f64 {
        let [a1, a2, a3] = self.a;
        let [_, _, m0, m1, m2] = self.p;
        let z = &mut self.z[ch];
        let v3 = x - z[1];
        let v1 = a1 * z[0] + a2 * v3;
        let v2 = z[1] + a2 * z[0] + a3 * v3;
        z[0] = 2.0 * v1 - z[0];
        z[1] = 2.0 * v2 - z[1];
        m0 * x + m1 * v1 + m2 * v2
    }

    /// One sample of glide; arrival is exact, and arriving at pass-through puts the section to sleep.
    #[inline]
    fn glide(&mut self, a: f64) {
        if self.settled {
            return;
        }
        let mut done = true;
        for ((p, q), t) in self.p.iter_mut().zip(&mut self.q).zip(self.t) {
            *q += (t - *q) * a;
            *p += (*q - *p) * a;
            if (t - *p).abs() + (t - *q).abs() > 1e-7 * (1.0 + t.abs()) {
                done = false;
            }
        }
        if done {
            self.p = self.t;
            self.q = self.t;
            self.settled = true;
            if self.t[2..] == THROUGH {
                *self = Self::new();
                return;
            }
        }
        self.gains();
    }

    /// Decaying states would reach f64 denormals about a second into silence.
    fn flush(&mut self) {
        for v in self.z.iter_mut().flatten() {
            if v.abs() < 1e-30 {
                *v = 0.0;
            }
        }
    }
}

pub struct Eq {
    sr: f64,
    /// Per-sample glide coefficient.
    a: f64,
    /// Params the targets were designed from (`None` = redesign on the next `set`).
    last: Option<EqParams>,
    /// Sections: low shelf, peak, high shelf, low cut x2.
    s: [Section; 5],
    /// Sample counter for the denormal flush.
    n: u32,
}

impl Eq {
    pub fn new() -> Self {
        let mut e = Self { sr: 48000.0, a: 1.0, last: None, s: [Section::new(); 5], n: 0 };
        e.set_sample_rate(48000.0);
        e
    }

    pub fn set_sample_rate(&mut self, sr: f64) {
        self.sr = sr;
        self.a = 1.0 - (-1.0 / (SMOOTH_S * sr)).exp();
        if let Some(p) = self.last.take() {
            self.set(&p);
        }
        self.reset();
    }

    pub fn latency(&self) -> usize {
        0
    }

    /// True while the EQ is a bit-exact pass-through (off or flat, glides finished).
    pub fn is_bypassed(&self) -> bool {
        !self.s.iter().any(|s| s.on)
    }

    /// Set the targets (once per block; values are clamped to the SPEC ranges). Coefficients are
    /// only redesigned when a value changed.
    pub fn set(&mut self, p: &EqParams) {
        if self.last == Some(*p) {
            return;
        }
        self.last = Some(*p);
        let (sr, top) = (self.sr, 0.49 * self.sr);
        let gain = |g: f32| if p.on && g != 0.0 { Some(g.clamp(-15.0, 15.0) as f64) } else { None };
        let [low, pk, high, cut0, cut1] = &mut self.s;
        low.target(gain(p.low_gain).map(|g| low_shelf(g, p.low_freq.clamp(30.0, 500.0) as f64, sr)));
        let (pf, pq) = ((p.peak_freq.clamp(20.0, 20000.0) as f64).min(top), p.peak_q.clamp(0.2, 10.0) as f64);
        pk.target(gain(p.peak_gain).map(|g| peak(g, pq, pf, sr)));
        let hf = (p.high_freq.clamp(1000.0, 20000.0) as f64).min(top);
        high.target(gain(p.high_gain).map(|g| high_shelf(g, hf, sr)));
        let cut = low_cut(p.cut_freq.clamp(20.0, 500.0) as f64, sr);
        for (s, c) in [cut0, cut1].into_iter().zip(cut) {
            s.target((p.on && p.cut_on).then_some(c));
        }
    }

    /// Clear the filter state and jump to the targets.
    pub fn reset(&mut self) {
        for s in &mut self.s {
            s.land();
        }
    }

    #[inline]
    pub fn process(&mut self, l: &mut f32, r: &mut f32) {
        let mut x = [*l as f64, *r as f64];
        let mut any = false;
        for s in &mut self.s {
            if s.on {
                any = true;
                x = [s.tick(0, x[0]), s.tick(1, x[1])];
                s.glide(self.a);
            }
        }
        if any {
            *l = x[0] as f32;
            *r = x[1] as f32;
            self.n = self.n.wrapping_add(1);
            if self.n & 63 == 0 {
                self.s.iter_mut().for_each(Section::flush);
            }
        }
    }
}

impl Default for Eq {
    fn default() -> Self {
        Self::new()
    }
}
