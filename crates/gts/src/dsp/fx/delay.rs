//! FX 4.6 Delay (`dly_`): free time 1 ms..2 s or a tempo-synced division 1/64..4/1 (straight,
//! dotted x1.5, triplet x2/3), feedback 0..0.95, Mono / Stereo / PingPong, a 6 dB/oct high-pass
//! and low-pass at the line input (inside the feedback loop, so every echo passes them once
//! more), mix.
//!
//! - Lines are preallocated for [`MAX_SYNC_S`] at 192 kHz; synced times longer than that are
//!   clamped (4/1 fits from 120 BPM, 2/1 from 60 BPM). Free time is clamped to 1 ms..2 s.
//! - Time changes glide: a 20 ms one-pole toward the new time whose rate is capped at [`SLEW`]
//!   samples per sample, so a glide is a tape-style pitch bend bounded to a ratio of 1 +/- SLEW
//!   (about -5 / +3.9 semitones) on the first echo and never a click. Feedback passes during a
//!   glide each take the bend again.
//! - Reads use 4-point Catmull-Rom interpolation: exact at integer delays, |H| <= 1 at every
//!   fraction, so feedback 0.95 always decays.
//! - Mono feeds the mono sum to both lines (identical, so a mode switch continues seamlessly);
//!   PingPong feeds the mono sum to the left line and cross-couples the feedback, so echoes
//!   alternate L, R, L, ...
//! - `on = false` returns the input untouched (sample-exact bypass); switching on clears the lines.
//!   Latency 0. Denormals: relies on the host wrapper's flush-to-zero (nih-plug sets it).

/// Highest sample rate the lines are allocated for.
pub const MAX_SR: f32 = 192_000.0;
/// Longest free time, seconds.
pub const MAX_FREE_S: f32 = 2.0;
/// Longest synced time (and the allocated line length), seconds.
pub const MAX_SYNC_S: f32 = 8.0;
/// Largest change of the delay time per sample while gliding (pitch ratio 1 +/- SLEW).
pub const SLEW: f64 = 0.25;
/// Parameter smoothing time constant, seconds.
const SMOOTH_S: f32 = 0.01;
/// Delay time glide time constant, seconds.
const GLIDE_S: f64 = 0.02;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelayMode {
    Mono,
    Stereo,
    PingPong,
}

/// Note value of a synced delay, in whole notes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Div {
    D1_64,
    D1_32,
    D1_16,
    D1_8,
    D1_4,
    D1_2,
    D1_1,
    D2_1,
    D4_1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feel {
    Straight,
    Dotted,
    Triplet,
}

impl Div {
    /// Length in quarter-note beats.
    pub fn beats(self, feel: Feel) -> f64 {
        let whole = match self {
            Div::D1_64 => 1.0 / 64.0,
            Div::D1_32 => 1.0 / 32.0,
            Div::D1_16 => 1.0 / 16.0,
            Div::D1_8 => 1.0 / 8.0,
            Div::D1_4 => 0.25,
            Div::D1_2 => 0.5,
            Div::D1_1 => 1.0,
            Div::D2_1 => 2.0,
            Div::D4_1 => 4.0,
        };
        let f = match feel {
            Feel::Straight => 1.0,
            Feel::Dotted => 1.5,
            Feel::Triplet => 2.0 / 3.0,
        };
        4.0 * whole * f
    }
}

/// Plain parameter values, set once per block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DelayParams {
    pub on: bool,
    pub sync: bool,
    /// Free time, ms (1..2000).
    pub time_ms: f32,
    pub div: Div,
    pub feel: Feel,
    /// Host tempo, BPM (used when `sync`).
    pub tempo: f32,
    /// 0..0.95.
    pub feedback: f32,
    pub mode: DelayMode,
    /// Hz, 20..2000.
    pub hp_hz: f32,
    /// Hz, 200..20000.
    pub lp_hz: f32,
    /// 0..1.
    pub mix: f32,
}

impl Default for DelayParams {
    fn default() -> Self {
        Self {
            on: false,
            sync: true,
            time_ms: 250.0,
            div: Div::D1_8,
            feel: Feel::Dotted,
            tempo: 140.0,
            feedback: 0.35,
            mode: DelayMode::Stereo,
            hp_hz: 120.0,
            lp_hz: 8000.0,
            mix: 0.25,
        }
    }
}

impl DelayParams {
    /// Delay time in seconds after sync and clamping.
    pub fn seconds(&self) -> f64 {
        if self.sync {
            let bpm = (self.tempo as f64).clamp(20.0, 999.0);
            (self.div.beats(self.feel) * 60.0 / bpm).clamp(0.001, MAX_SYNC_S as f64)
        } else {
            (self.time_ms as f64 / 1000.0).clamp(0.001, MAX_FREE_S as f64)
        }
    }
}

/// One-pole step toward `t`, snapping once within 1e-6 so a target of 0 is reached exactly.
#[inline]
fn glide(x: &mut f32, t: f32, k: f32) {
    *x = t + (*x - t) * k;
    if (*x - t).abs() < 1e-6 {
        *x = t;
    }
}

/// TPT one-pole coefficient `g / (1 + g)` for cutoff `hz`.
fn one_pole(hz: f32, sr: f32) -> f32 {
    let g = (std::f32::consts::PI * hz.clamp(1.0, 0.49 * sr) / sr).tan();
    g / (1.0 + g)
}

struct Line {
    buf: Vec<f32>,
    /// Active length (8 s at the current rate plus margin).
    len: usize,
    /// Next write index.
    w: usize,
    /// High-pass (its low-pass state) and low-pass one-pole states.
    hp: f32,
    lp: f32,
}

impl Line {
    fn new() -> Self {
        let n = (MAX_SYNC_S * MAX_SR) as usize + 8;
        Self { buf: vec![0.0; n], len: n, w: 0, hp: 0.0, lp: 0.0 }
    }

    fn clear(&mut self) {
        self.buf[..self.len].fill(0.0);
        (self.w, self.hp, self.lp) = (0, 0.0, 0.0);
    }

    /// `k` samples ago (1 <= k < len).
    #[inline]
    fn at(&self, k: usize) -> f32 {
        let i = if k > self.w { self.w + self.len - k } else { self.w - k };
        self.buf[i]
    }

    /// The input `d` samples ago (d >= 2), Catmull-Rom.
    #[inline]
    fn read(&self, d: f64) -> f32 {
        let i = d as usize;
        let t = (d - i as f64) as f32;
        let (p0, p1, p2, p3) = (self.at(i - 1), self.at(i), self.at(i + 1), self.at(i + 2));
        let c1 = 0.5 * (p2 - p0);
        let c2 = p0 - 2.5 * p1 + 2.0 * p2 - 0.5 * p3;
        let c3 = 0.5 * (p3 - p0) + 1.5 * (p1 - p2);
        ((c3 * t + c2) * t + c1) * t + p1
    }

    /// Filters `x` (HP then LP, coefficients `gh`, `gl`) and writes it.
    #[inline]
    fn write(&mut self, x: f32, gh: f32, gl: f32) {
        let v = (x - self.hp) * gh;
        let low = v + self.hp;
        self.hp = low + v;
        let x = x - low;
        let v = (x - self.lp) * gl;
        let y = v + self.lp;
        self.lp = y + v;
        self.buf[self.w] = y;
        self.w += 1;
        if self.w == self.len {
            self.w = 0;
        }
    }
}

pub struct Delay {
    sr: f32,
    p: DelayParams,
    lines: [Line; 2],
    /// Smoothing coefficient per sample.
    k: f32,
    /// Smoothed mix and feedback; smoothed HP / LP coefficients and their targets.
    mix: f32,
    fb: f32,
    gh: f32,
    gl: f32,
    gh_t: f32,
    gl_t: f32,
    /// Delay in samples (gliding) and its target.
    d: f64,
    d_t: f64,
    glide_k: f64,
}

impl Default for Delay {
    fn default() -> Self {
        Self::new()
    }
}

impl Delay {
    /// Allocates both lines for 8 s at 192 kHz (12.3 MB).
    pub fn new() -> Self {
        let mut s = Self {
            sr: 48000.0,
            p: DelayParams::default(),
            lines: [Line::new(), Line::new()],
            k: 0.0,
            mix: 0.0,
            fb: 0.0,
            gh: 0.0,
            gl: 0.0,
            gh_t: 0.0,
            gl_t: 0.0,
            d: 0.0,
            d_t: 0.0,
            glide_k: 0.0,
        };
        s.set_sample_rate(48000.0);
        s
    }

    pub fn set_sample_rate(&mut self, sr: f32) {
        self.sr = sr.clamp(8000.0, MAX_SR);
        self.k = (-1.0 / (SMOOTH_S * self.sr)).exp();
        self.glide_k = (-1.0 / (GLIDE_S * self.sr as f64)).exp();
        let len = (MAX_SYNC_S * self.sr) as usize + 8;
        for l in &mut self.lines {
            l.len = len;
        }
        let p = self.p;
        self.set(&p);
        self.reset();
    }

    /// Clears the lines and jumps every smoothed value to its target.
    pub fn reset(&mut self) {
        for l in &mut self.lines {
            l.clear();
        }
        self.mix = self.p.mix.clamp(0.0, 1.0);
        self.fb = self.p.feedback.clamp(0.0, 0.95);
        (self.gh, self.gl, self.d) = (self.gh_t, self.gl_t, self.d_t);
    }

    pub fn set(&mut self, p: &DelayParams) {
        let was_on = self.p.on;
        self.p = *p;
        self.gh_t = one_pole(p.hp_hz.clamp(20.0, 2000.0), self.sr);
        self.gl_t = one_pole(p.lp_hz.clamp(200.0, 20000.0), self.sr);
        self.d_t = p.seconds() * self.sr as f64;
        if p.on && !was_on {
            self.reset();
        }
    }

    pub fn latency(&self) -> usize {
        0
    }

    /// Current (gliding) delay in samples.
    pub fn delay_samples(&self) -> f64 {
        self.d
    }

    #[inline]
    pub fn process(&mut self, l: &mut f32, r: &mut f32) {
        if !self.p.on {
            return;
        }
        let k = self.k;
        glide(&mut self.mix, self.p.mix.clamp(0.0, 1.0), k);
        glide(&mut self.fb, self.p.feedback.clamp(0.0, 0.95), k);
        glide(&mut self.gh, self.gh_t, k);
        glide(&mut self.gl, self.gl_t, k);
        if self.d != self.d_t {
            self.d += ((self.d_t - self.d) * (1.0 - self.glide_k)).clamp(-SLEW, SLEW);
            if (self.d_t - self.d).abs() < 1e-6 {
                self.d = self.d_t;
            }
        }

        let (yl, yr) = (self.lines[0].read(self.d), self.lines[1].read(self.d));
        let (fb, gh, gl) = (self.fb, self.gh, self.gl);
        let m = 0.5 * (*l + *r);
        let (in_l, in_r) = match self.p.mode {
            DelayMode::Mono => (m + fb * yl, m + fb * yr),
            DelayMode::Stereo => (*l + fb * yl, *r + fb * yr),
            DelayMode::PingPong => (m + fb * yr, fb * yl),
        };
        self.lines[0].write(in_l, gh, gl);
        self.lines[1].write(in_r, gh, gl);
        *l += self.mix * (yl - *l);
        *r += self.mix * (yr - *r);
    }
}
