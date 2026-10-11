//! FX 4.7 Reverb (`rev_`): Plate (Dattorro 1997 figure-of-eight tank) and Hall (16-line feedback
//! delay network, Hadamard matrix), mono in / stereo out.
//!
//! Signal path: mono sum -> pre-delay (0..300 ms) -> low cut (Butterworth 24 dB/oct, 20..1000 Hz,
//! default 200) -> high cut (Butterworth 12 dB/oct, 1..20 kHz) -> input diffusers -> tank ->
//! width (mid/side, 0 = mono) -> dry/wet mix.
//!
//! - Decay is the RT60 in seconds (0.1..20). Every delay in a loop carries the gain
//!   `10^(-3 L / (T sr))` for its length `L` (allpass lengths are lumped with the following delay,
//!   since an allpass of length L averages L samples of group delay), so the loop loses 60 dB in
//!   `T` seconds whatever the size. Damping is a one-pole low-pass in the loop with unity gain at
//!   DC (damp 0 = no filter), so it shortens only the high-frequency decay. Decays shorter than
//!   ~1.9 s (plate) / 0.54 s (hall) also cap the size, see `SHORT_PASS`. Measured: Schroeder T20
//!   within 10 % of `decay` over 0.1..20 s, both types, sizes 0..1 (`tests/fx_reverb.rs`).
//! - Loop reads are whole samples (linear interpolation inside a loop loses high end every pass
//!   and measured 10-14 % short); the modulated lines (two plate allpasses, four hall lines) use
//!   first-order allpass interpolation, which is lossless.
//! - Size scales every tank delay (plate 0.3..1.3 x Dattorro's lengths, hall 0.5..1.6 x 25..85 ms)
//!   and glides (20 ms one-pole); the rounded lengths then step one sample at a time, so automating
//!   size is click-free in practice but not sample-smooth. Pre-delay glides through a linear read
//!   and settles on a whole sample.
//! - Buffers are preallocated for 192 kHz at the largest size. `on = false` is a sample-exact
//!   bypass; switching on, or changing type, clears the buffers (a type change cuts the old tail).
//!   Latency 0. Denormals: relies on the host wrapper's flush-to-zero (nih-plug sets it).

use std::f32::consts::{PI, TAU};

/// Highest sample rate the buffers are allocated for.
pub const MAX_SR: f32 = 192_000.0;
/// Longest pre-delay, seconds.
pub const MAX_PREDELAY_S: f32 = 0.3;
/// Parameter smoothing (mix, width, damping, gains, filter cutoffs), seconds.
const SMOOTH_S: f32 = 0.01;
/// Size and pre-delay glide, seconds.
const GLIDE_S: f32 = 0.02;

/// Dattorro's design rate; his delay lengths are in samples at this rate.
const DAT_SR: f32 = 29761.0;
const PLATE_SCALE: (f32, f32) = (0.3, 1.3);
/// Half of the tank loop at scale 1, seconds.
const PLATE_HALF_S: f32 = 0.5 * (672.0 + 4453.0 + 1800.0 + 3720.0 + 908.0 + 4217.0 + 2656.0 + 3163.0) / DAT_SR;
/// Short decays: a pass through the loop (plate half-loop, longest hall line) is held to at most
/// `decay / SHORT_PASS` by shrinking the size, and each input diffuser's own ring to
/// `DIFF_T * decay` by lowering its coefficient. Without this the tank's first pass and the
/// diffusers' rings set a floor (~0.4 s plate, ~0.2 s hall) and short decays measure long. It only
/// acts below decay 4 * 0.36 * 1.3 = 1.9 s (plate, largest size) / 0.54 s (hall).
const SHORT_PASS: f32 = 4.0;
const DIFF_T: f32 = 0.5;
/// Plate decay calibration: the lumped-allpass loop model measures 3.5 % long (Schroeder T20,
/// sizes 0..1, decays 0.5..20 s), so the loop gains target a 3.5 % shorter time.
const PLATE_RT_CAL: f32 = 1.0 / 1.035;
/// Input diffusers (length, nominal coefficient).
const PLATE_DIFF: [(f32, f32); 4] = [(142.0, 0.75), (107.0, 0.75), (379.0, 0.625), (277.0, 0.625)];
/// Tank per half: modulated allpass, delay, allpass, delay.
const PLATE_L: [f32; 4] = [672.0, 4453.0, 1800.0, 3720.0];
const PLATE_R: [f32; 4] = [908.0, 4217.0, 2656.0, 3163.0];
/// Modulation depth (peak, Dattorro samples) and rates (Hz) of the two tank allpasses.
const PLATE_EXC: f32 = 8.0;
const PLATE_RATE: [f32; 2] = [0.5, 0.63];
/// Output taps (Dattorro table 2): (line, position, sign). Lines: 0 left delay 1, 1 left
/// allpass 2, 2 left delay 2, 3 right delay 1, 4 right allpass 2, 5 right delay 2.
const PLATE_TAPS_L: [(usize, f32, f32); 7] = [
    (3, 266.0, 1.0),
    (3, 2974.0, 1.0),
    (4, 1913.0, -1.0),
    (5, 1996.0, 1.0),
    (0, 1990.0, -1.0),
    (1, 187.0, -1.0),
    (2, 1066.0, -1.0),
];
const PLATE_TAPS_R: [(usize, f32, f32); 7] = [
    (0, 353.0, 1.0),
    (0, 3627.0, 1.0),
    (1, 1228.0, -1.0),
    (2, 2673.0, 1.0),
    (3, 2111.0, -1.0),
    (4, 335.0, -1.0),
    (5, 121.0, -1.0),
];

const HALL_N: usize = 16;
const HALL_SCALE: (f32, f32) = (0.5, 1.6);
/// Line lengths at scale 1, ms: geometric 25..85 ms, nudged off simple ratios.
const HALL_MS: [f32; HALL_N] =
    [25.0, 27.3, 29.9, 32.4, 35.6, 38.3, 42.1, 45.7, 49.6, 53.9, 58.3, 63.4, 68.6, 74.5, 80.6, 85.0];
/// Input injection signs per line (spreads the input over the matrix's sign patterns).
const HALL_IN: [f32; HALL_N] = [1., -1., -1., -1., -1., 1., 1., 1., 1., -1., -1., -1., -1., 1., 1., 1.];
/// Input diffusers (ms, nominal coefficient).
const HALL_DIFF: [(f32, f32); 4] = [(4.77, 0.7), (3.59, 0.7), (12.73, 0.6), (9.31, 0.6)];
/// Slow modulation of every fourth line (peak ms) against modal ringing.
const HALL_MOD_MS: f32 = 0.25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReverbType {
    Plate,
    Hall,
}

/// Plain parameter values, set once per block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReverbParams {
    pub on: bool,
    pub ty: ReverbType,
    /// 0..1.
    pub size: f32,
    /// RT60, seconds (0.1..20).
    pub decay: f32,
    /// 0..1.
    pub damp: f32,
    /// ms, 0..300.
    pub predelay_ms: f32,
    /// Hz, 20..1000.
    pub lowcut_hz: f32,
    /// Hz, 1000..20000.
    pub hicut_hz: f32,
    /// 0 (mono) .. 1.
    pub width: f32,
    /// 0..1.
    pub mix: f32,
}

impl Default for ReverbParams {
    fn default() -> Self {
        Self {
            on: false,
            ty: ReverbType::Plate,
            size: 0.5,
            decay: 1.5,
            damp: 0.3,
            predelay_ms: 0.0,
            lowcut_hz: 200.0,
            hicut_hz: 12000.0,
            width: 1.0,
            mix: 0.15,
        }
    }
}

#[inline]
fn glide(x: &mut f32, t: f32, k: f32) {
    *x = t + (*x - t) * k;
    if (*x - t).abs() < 1e-6 {
        *x = t;
    }
}

/// Power-of-two circular buffer; `w` indexes the newest sample.
struct Line {
    buf: Vec<f32>,
    mask: usize,
    w: usize,
}

impl Line {
    fn new(max_len: f32) -> Self {
        let n = (max_len as usize + 4).next_power_of_two();
        Self { buf: vec![0.0; n], mask: n - 1, w: 0 }
    }

    fn clear(&mut self) {
        self.buf.fill(0.0);
    }

    #[inline]
    fn push(&mut self, x: f32) {
        self.w = (self.w + 1) & self.mask;
        self.buf[self.w] = x;
    }

    /// `d` samples behind the newest (d >= 0), linear interpolation (pre-delay only: linear
    /// interpolation loses high end, which inside a loop would shorten the decay).
    #[inline]
    fn tap(&self, d: f32) -> f32 {
        let i = d as usize;
        let f = d - i as f32;
        let a = self.buf[self.w.wrapping_sub(i) & self.mask];
        let b = self.buf[self.w.wrapping_sub(i + 1) & self.mask];
        a + (b - a) * f
    }

    /// `k` samples behind the newest.
    #[inline]
    fn at(&self, k: usize) -> f32 {
        self.buf[self.w.wrapping_sub(k) & self.mask]
    }

    /// Before this sample's push: the input `round(d)` samples ago (d >= 1).
    #[inline]
    fn read(&self, d: f32) -> f32 {
        self.at((d - 0.5) as usize)
    }

    /// Before this sample's push: the input `d` samples ago (d >= 2) through a first-order allpass
    /// (Thiran) interpolator, lossless at every fraction. The fraction is kept in 0.5..1.5 so
    /// the coefficient stays within -0.2..0.33; `y1` is the interpolator state.
    #[inline]
    fn read_ap(&self, d: f32, y1: &mut f32) -> f32 {
        let i = (d - 0.5) as usize;
        let f = d - i as f32;
        let eta = (1.0 - f) / (1.0 + f);
        *y1 = eta * self.at(i - 1) + self.at(i) - eta * *y1;
        *y1
    }

    /// Schroeder allpass of length `round(d)` around this line: `w = x + g z`, `y = z - g w`.
    #[inline]
    fn allpass(&mut self, x: f32, d: f32, g: f32) -> f32 {
        let z = self.read(d);
        let w = x + g * z;
        self.push(w);
        z - g * w
    }

    /// The same with a fractional, modulated length (allpass-interpolated read).
    #[inline]
    fn allpass_mod(&mut self, x: f32, d: f32, g: f32, y1: &mut f32) -> f32 {
        let z = self.read_ap(d, y1);
        let w = x + g * z;
        self.push(w);
        z - g * w
    }
}

/// TPT state-variable section, returns (low, high).
#[inline]
fn svf(s: &mut [f32; 2], x: f32, g: f32, k: f32) -> (f32, f32) {
    let hp = (x - (k + g) * s[0] - s[1]) / (1.0 + k * g + g * g);
    let bp = g * hp + s[0];
    s[0] = g * hp + bp;
    let lp = g * bp + s[1];
    s[1] = g * bp + lp;
    (lp, hp)
}

/// 1/Q of the two Butterworth-4 sections, and of Butterworth-2.
const BW4_K: [f32; 2] = [1.847_759, 0.765_367];
const BW2_K: f32 = std::f32::consts::SQRT_2;

/// One-pole damping coefficient: 0 at damp 0, cutoff falling from Nyquist to 1 kHz at damp 1.
fn damp_coef(damp: f32, sr: f32) -> f32 {
    let damp = damp.clamp(0.0, 1.0);
    let ny = 0.5 * sr;
    let fc = ny * (1000.0 / ny).powf(damp);
    let a0 = (-PI).exp();
    ((-TAU * fc / sr).exp() - a0) / (1.0 - a0)
}

/// Quadrature sine oscillator (rotating phasor).
#[derive(Clone, Copy)]
struct Osc {
    c: f32,
    s: f32,
    rc: f32,
    rs: f32,
}

impl Osc {
    fn new(hz: f32, sr: f32, phase: f32) -> Self {
        let w = TAU * hz / sr;
        Self { c: phase.cos(), s: phase.sin(), rc: w.cos(), rs: w.sin() }
    }

    #[inline]
    fn next(&mut self) -> f32 {
        let (c, s) = (self.c * self.rc - self.s * self.rs, self.c * self.rs + self.s * self.rc);
        // Keep the phasor on the unit circle.
        let n = 1.5 - 0.5 * (c * c + s * s);
        (self.c, self.s) = (c * n, s * n);
        self.s
    }
}

struct Plate {
    diff: [Line; 4],
    /// Per half (left, right): modulated allpass, delay 1, allpass 2, delay 2.
    tank: [[Line; 4]; 2],
    damp: [f32; 2],
    lfo: [Osc; 2],
    /// Allpass-interpolator states of the modulated tank allpasses.
    ap: [f32; 2],
}

impl Plate {
    fn new() -> Self {
        let k = MAX_SR / DAT_SR;
        let max = PLATE_SCALE.1 * k;
        let half = |len: [f32; 4]| std::array::from_fn(|i| Line::new(len[i] * max + PLATE_EXC * max + 2.0));
        Self {
            diff: std::array::from_fn(|i| Line::new(PLATE_DIFF[i].0 * k + 2.0)),
            tank: [half(PLATE_L), half(PLATE_R)],
            damp: [0.0; 2],
            lfo: [Osc::new(1.0, 48000.0, 0.0); 2],
            ap: [0.0; 2],
        }
    }

    fn clear(&mut self) {
        self.diff.iter_mut().chain(self.tank.iter_mut().flatten()).for_each(Line::clear);
        (self.damp, self.ap) = ([0.0; 2], [0.0; 2]);
    }

    /// `s` = Dattorro samples -> samples, `ga`/`gb` = loop gains after delay 1 / delay 2 per half,
    /// `dr` = diffuser length scale (sr only), `dd` = damping coefficient, `dd2` = decay diffusion 2.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn process(
        &mut self,
        x: f32,
        s: f32,
        dr: f32,
        dg: &[f32; 4],
        ga: [f32; 2],
        gb: [f32; 2],
        dd: f32,
        dd2: f32,
    ) -> (f32, f32) {
        let mut x = x;
        for ((line, &(len, _)), &g) in self.diff.iter_mut().zip(&PLATE_DIFF).zip(dg) {
            x = line.allpass(x, len * dr, g);
        }
        let ends = [self.tank[0][3].read(PLATE_L[3] * s) * gb[0], self.tank[1][3].read(PLATE_R[3] * s) * gb[1]];
        let lens = [PLATE_L, PLATE_R];
        for h in 0..2 {
            let len = lens[h];
            let m = self.lfo[h].next() * PLATE_EXC * s;
            let t = &mut self.tank[h];
            let a = t[0].allpass_mod(x + ends[1 - h], len[0] * s + m, -0.7, &mut self.ap[h]);
            let b = t[1].read(len[1] * s);
            t[1].push(a);
            self.damp[h] = b + (self.damp[h] - b) * dd;
            let c = t[2].allpass(self.damp[h] * ga[h], len[2] * s, dd2);
            t[3].push(c);
        }
        let lines = [
            &self.tank[0][1],
            &self.tank[0][2],
            &self.tank[0][3],
            &self.tank[1][1],
            &self.tank[1][2],
            &self.tank[1][3],
        ];
        let tap = |taps: &[(usize, f32, f32); 7]| {
            taps.iter().map(|&(i, p, g)| g * lines[i].at((p * s + 0.5) as usize)).sum::<f32>()
        };
        (0.6 * tap(&PLATE_TAPS_L), 0.6 * tap(&PLATE_TAPS_R))
    }
}

/// In-place 16-point fast Walsh-Hadamard transform, scaled to be orthonormal.
#[inline]
fn hadamard(y: &mut [f32; HALL_N]) {
    let mut h = 1;
    while h < HALL_N {
        for i in (0..HALL_N).step_by(2 * h) {
            for j in i..i + h {
                let (a, b) = (y[j], y[j + h]);
                (y[j], y[j + h]) = (a + b, a - b);
            }
        }
        h *= 2;
    }
    for v in y.iter_mut() {
        *v *= 0.25;
    }
}

struct Hall {
    diff: [Line; 4],
    lines: [Line; HALL_N],
    damp: [f32; HALL_N],
    lfo: [Osc; 4],
    /// Allpass-interpolator states of the modulated lines.
    ap: [f32; 4],
}

impl Hall {
    fn new() -> Self {
        let ms = MAX_SR / 1000.0;
        Self {
            diff: std::array::from_fn(|i| Line::new(HALL_DIFF[i].0 * ms + 2.0)),
            lines: std::array::from_fn(|i| Line::new((HALL_MS[i] * HALL_SCALE.1 + HALL_MOD_MS) * ms + 2.0)),
            damp: [0.0; HALL_N],
            lfo: [Osc::new(1.0, 48000.0, 0.0); 4],
            ap: [0.0; 4],
        }
    }

    fn clear(&mut self) {
        self.diff.iter_mut().chain(self.lines.iter_mut()).for_each(Line::clear);
        (self.damp, self.ap) = ([0.0; HALL_N], [0.0; 4]);
    }

    /// `s` = ms -> samples at the current size, `ms` = ms -> samples, `g` = per-line loop gains.
    #[inline]
    fn process(&mut self, x: f32, s: f32, ms: f32, dg: &[f32; 4], g: &[f32; HALL_N], dd: f32) -> (f32, f32) {
        let mut x = x;
        for ((line, &(len, _)), &c) in self.diff.iter_mut().zip(&HALL_DIFF).zip(dg) {
            x = line.allpass(x, len * ms, c);
        }
        let m: [f32; 4] = std::array::from_fn(|i| self.lfo[i].next() * HALL_MOD_MS * ms);
        let mut y = [0.0; HALL_N];
        let (mut l, mut r) = (0.0, 0.0);
        for i in 0..HALL_N {
            let v = if i % 4 == 0 {
                self.lines[i].read_ap(HALL_MS[i] * s + m[i / 4], &mut self.ap[i / 4])
            } else {
                self.lines[i].read(HALL_MS[i] * s)
            };
            self.damp[i] = v + (self.damp[i] - v) * dd;
            y[i] = self.damp[i] * g[i];
            let sign = if i & 2 == 0 { 1.0 } else { -1.0 };
            if i % 2 == 0 {
                l += sign * v;
            } else {
                r += sign * v;
            }
        }
        hadamard(&mut y);
        for ((line, v), sign) in self.lines.iter_mut().zip(y).zip(HALL_IN) {
            line.push(v + 0.25 * sign * x);
        }
        // Level-matched to the plate (wet RMS for steady noise within 0.5 dB at decay 2 s).
        (1.05 * l, 1.05 * r)
    }
}

pub struct Reverb {
    sr: f32,
    p: ReverbParams,
    pre: Line,
    plate: Plate,
    hall: Hall,
    /// Low cut (two sections) and high cut SVF states.
    lc: [[f32; 2]; 2],
    hc: [f32; 2],
    k: f32,
    glide_k: f32,
    /// Smoothed values (`*_t` = targets set per block).
    mix: f32,
    width: f32,
    dd: f32,
    dd_t: f32,
    lc_g: f32,
    lc_g_t: f32,
    hc_g: f32,
    hc_g_t: f32,
    pre_d: f32,
    pre_d_t: f32,
    /// Size: plate in samples per Dattorro sample, hall in samples per ms.
    s: f32,
    s_t: f32,
    /// Loop gains: plate `[ga_l, ga_r, gb_l, gb_r]`, or the hall's 16 per-line gains.
    g: [f32; HALL_N],
    g_t: [f32; HALL_N],
    /// Input diffuser coefficients.
    dg: [f32; 4],
    dg_t: [f32; 4],
}

impl Default for Reverb {
    fn default() -> Self {
        Self::new()
    }
}

impl Reverb {
    /// Allocates every buffer for 192 kHz at the largest size.
    pub fn new() -> Self {
        let mut r = Self {
            sr: 48000.0,
            p: ReverbParams::default(),
            pre: Line::new(MAX_PREDELAY_S * MAX_SR + 2.0),
            plate: Plate::new(),
            hall: Hall::new(),
            lc: [[0.0; 2]; 2],
            hc: [0.0; 2],
            k: 0.0,
            glide_k: 0.0,
            mix: 0.0,
            width: 0.0,
            dd: 0.0,
            dd_t: 0.0,
            lc_g: 0.0,
            lc_g_t: 0.0,
            hc_g: 0.0,
            hc_g_t: 0.0,
            pre_d: 0.0,
            pre_d_t: 0.0,
            s: 0.0,
            s_t: 0.0,
            g: [0.0; HALL_N],
            g_t: [0.0; HALL_N],
            dg: [0.0; 4],
            dg_t: [0.0; 4],
        };
        r.set_sample_rate(48000.0);
        r
    }

    pub fn set_sample_rate(&mut self, sr: f32) {
        self.sr = sr.clamp(8000.0, MAX_SR);
        self.k = (-1.0 / (SMOOTH_S * self.sr)).exp();
        self.glide_k = (-1.0 / (GLIDE_S * self.sr)).exp();
        self.plate.lfo = std::array::from_fn(|i| Osc::new(PLATE_RATE[i], self.sr, 0.5 * PI * i as f32));
        self.hall.lfo = std::array::from_fn(|i| Osc::new(0.31 + 0.13 * i as f32, self.sr, 1.7 * i as f32));
        let p = self.p;
        self.set(&p);
        self.reset();
    }

    /// Clears every buffer and jumps smoothed values to their targets.
    pub fn reset(&mut self) {
        self.pre.clear();
        self.plate.clear();
        self.hall.clear();
        (self.lc, self.hc) = ([[0.0; 2]; 2], [0.0; 2]);
        self.mix = self.p.mix.clamp(0.0, 1.0);
        self.width = self.p.width.clamp(0.0, 1.0);
        (self.dd, self.lc_g, self.hc_g, self.pre_d, self.s, self.g) =
            (self.dd_t, self.lc_g_t, self.hc_g_t, self.pre_d_t, self.s_t, self.g_t);
        self.dg = self.dg_t;
    }

    pub fn set(&mut self, p: &ReverbParams) {
        let prev = self.p;
        self.p = *p;
        let sr = self.sr;
        let size = p.size.clamp(0.0, 1.0);
        let t60 = p.decay.clamp(0.1, 20.0);
        let cal = if p.ty == ReverbType::Plate { PLATE_RT_CAL } else { 1.0 };
        // Loop gain for `len` samples of delay.
        let gain = |len: f32| 10f32.powf(-3.0 * len / (cal * t60 * sr));
        // Short decays shrink the tank and soften the diffusers (see SHORT_PASS).
        let diff_g = |len: f32, g: f32| g.min(10f32.powf(-3.0 * len / (DIFF_T * t60 * sr)));
        match p.ty {
            ReverbType::Plate => {
                let scale =
                    (PLATE_SCALE.0 + (PLATE_SCALE.1 - PLATE_SCALE.0) * size).min(t60 / (SHORT_PASS * PLATE_HALF_S));
                self.s_t = sr / DAT_SR * scale;
                self.dg_t = std::array::from_fn(|i| diff_g(PLATE_DIFF[i].0 * sr / DAT_SR, PLATE_DIFF[i].1));
                let s = self.s_t;
                self.g_t[0] = gain((PLATE_L[0] + PLATE_L[1]) * s);
                self.g_t[1] = gain((PLATE_R[0] + PLATE_R[1]) * s);
                self.g_t[2] = gain((PLATE_L[2] + PLATE_L[3]) * s);
                self.g_t[3] = gain((PLATE_R[2] + PLATE_R[3]) * s);
            }
            ReverbType::Hall => {
                let longest = HALL_MS[HALL_N - 1] * 0.001;
                let scale = (HALL_SCALE.0 + (HALL_SCALE.1 - HALL_SCALE.0) * size).min(t60 / (SHORT_PASS * longest));
                self.s_t = sr / 1000.0 * scale;
                self.dg_t = std::array::from_fn(|i| diff_g(HALL_DIFF[i].0 * sr / 1000.0, HALL_DIFF[i].1));
                self.g_t = HALL_MS.map(|ms| gain(ms * self.s_t));
            }
        }
        self.dd_t = damp_coef(p.damp, sr);
        let tan = |hz: f32| (PI * hz.min(0.45 * sr) / sr).tan();
        self.lc_g_t = tan(p.lowcut_hz.clamp(20.0, 1000.0));
        self.hc_g_t = tan(p.hicut_hz.clamp(1000.0, 20000.0));
        // Whole samples, so the linear read is exact once the glide settles.
        self.pre_d_t = (p.predelay_ms.clamp(0.0, MAX_PREDELAY_S * 1000.0) * 0.001 * sr).round();
        if p.on && (!prev.on || p.ty != prev.ty) {
            self.reset();
        }
    }

    pub fn latency(&self) -> usize {
        0
    }

    #[inline]
    pub fn process(&mut self, l: &mut f32, r: &mut f32) {
        if !self.p.on {
            return;
        }
        let (k, gk) = (self.k, self.glide_k);
        glide(&mut self.mix, self.p.mix.clamp(0.0, 1.0), k);
        glide(&mut self.width, self.p.width.clamp(0.0, 1.0), k);
        glide(&mut self.dd, self.dd_t, k);
        glide(&mut self.lc_g, self.lc_g_t, k);
        glide(&mut self.hc_g, self.hc_g_t, k);
        glide(&mut self.pre_d, self.pre_d_t, gk);
        glide(&mut self.s, self.s_t, gk);
        let n = if self.p.ty == ReverbType::Plate { 4 } else { HALL_N };
        for i in 0..n {
            glide(&mut self.g[i], self.g_t[i], k);
        }
        for i in 0..4 {
            glide(&mut self.dg[i], self.dg_t[i], k);
        }

        self.pre.push(0.5 * (*l + *r));
        let mut x = self.pre.tap(self.pre_d);
        for (st, kk) in self.lc.iter_mut().zip(BW4_K) {
            x = svf(st, x, self.lc_g, kk).1;
        }
        x = svf(&mut self.hc, x, self.hc_g, BW2_K).0;

        let (wl, wr) = match self.p.ty {
            ReverbType::Plate => {
                let g = &self.g;
                let dr = self.sr / DAT_SR;
                self.plate.process(
                    x,
                    self.s,
                    dr,
                    &self.dg,
                    [g[0], g[1]],
                    [g[2], g[3]],
                    self.dd,
                    plate_dd2(self.p.decay),
                )
            }
            ReverbType::Hall => self.hall.process(x, self.s, self.sr / 1000.0, &self.dg, &self.g, self.dd),
        };
        let mid = 0.5 * (wl + wr);
        let side = 0.5 * (wl - wr) * self.width;
        *l += self.mix * (mid + side - *l);
        *r += self.mix * (mid - side - *r);
    }
}

/// Dattorro's decay diffusion 2: decay-dependent, 0.25..0.5.
#[inline]
fn plate_dd2(decay: f32) -> f32 {
    // His `decay` is a per-pass gain ~0.5 for a medium plate; map the RT60 onto it.
    (0.15 + (decay / 4.0).min(1.0) * 0.5).clamp(0.25, 0.5)
}
