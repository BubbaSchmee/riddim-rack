//! FX 4.8 Limiter: always-on output safety limiter, ceiling -0.3 dBFS, input gain -24..+12 dB.
//!
//! Lookahead brickwall (sample peak), stereo linked:
//! 1. required gain per sample `req = min(1, ceiling / max|l|,|r|)`;
//! 2. minimum of `req` over the last `L + 1 + H` samples (lookahead `L` = 1.5 ms, hold `H` =
//!    20 ms) with a monotonic deque, O(1) per sample;
//! 3. instant attack, exponential release (80 ms) towards that minimum;
//! 4. an `L`-sample moving average of the result (the attack ramp), applied to the input delayed
//!    by `L`. Every average term is <= `req` of the delayed sample, so the gain never undershoots
//!    what is needed; a final clamp to the ceiling covers float rounding.
//!
//! The hold keeps the gain flat between the peaks of anything above ~25 Hz, so a steady
//! over-ceiling tone is turned down without gain ripple (THD stays low); release only starts
//! after the material drops. Latency = `L` samples (72 at 48 kHz), constant; `on = false` outputs
//! the input delayed by `L`, bit-exact. Non-finite input is replaced by silence.

/// Plain parameter values, set once per block with [`Limiter::set`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LimiterParams {
    /// Always on in the chain (SPEC 4.8); `false` is a delayed pass-through for tests and hosts.
    pub on: bool,
    /// Input gain, -24..+12 dB.
    pub gain_db: f32,
}

impl Default for LimiterParams {
    fn default() -> Self {
        Self { on: true, gain_db: 0.0 }
    }
}

/// -0.3 dBFS.
pub const CEILING: f32 = 0.965_936_3;
const LOOKAHEAD_S: f32 = 0.0015;
const HOLD_S: f32 = 0.020;
const RELEASE_S: f32 = 0.080;
const SMOOTH_S: f32 = 0.01;
const MAX_SR: f32 = 192000.0;
/// Delay / average length at 192 kHz.
const MAX_L: usize = (LOOKAHEAD_S * MAX_SR) as usize + 1;
const MAX_W: usize = ((LOOKAHEAD_S + HOLD_S) * MAX_SR) as usize + 2;

pub struct Limiter {
    p: LimiterParams,
    l: usize,
    win: usize,
    a: f32,
    rel: f32,
    gain_cur: f32,
    gain_tgt: f32,
    /// Audio delay lines, `l` long.
    dly: [Vec<f32>; 2],
    /// Moving-average ring of the released gain, `l` long, and its running sum.
    avg: Vec<f32>,
    sum: f64,
    pos: usize,
    /// Monotonic deque of (sample index, req), ring of capacity `MAX_W`.
    dq: Vec<(u64, f32)>,
    head: usize,
    len: usize,
    n: u64,
    env: f32,
}

impl Default for Limiter {
    fn default() -> Self {
        Self::new()
    }
}

impl Limiter {
    pub fn new() -> Self {
        let mut s = Self {
            p: LimiterParams::default(),
            l: 1,
            win: 1,
            a: 0.0,
            rel: 0.0,
            gain_cur: 1.0,
            gain_tgt: 1.0,
            dly: [vec![0.0; MAX_L], vec![0.0; MAX_L]],
            avg: vec![1.0; MAX_L],
            sum: 0.0,
            pos: 0,
            dq: vec![(0, 1.0); MAX_W],
            head: 0,
            len: 0,
            n: 0,
            env: 1.0,
        };
        s.set_sample_rate(48000.0);
        s
    }

    pub fn set_sample_rate(&mut self, sr: f32) {
        let sr = sr.min(MAX_SR);
        self.l = ((LOOKAHEAD_S * sr) as usize).clamp(1, MAX_L);
        self.win = (((LOOKAHEAD_S + HOLD_S) * sr) as usize + 1).clamp(self.l + 1, MAX_W);
        self.a = (-1.0 / (SMOOTH_S * sr)).exp();
        self.rel = (-1.0 / (RELEASE_S * sr)).exp();
        self.reset();
    }

    pub fn reset(&mut self) {
        for d in &mut self.dly {
            d.fill(0.0);
        }
        self.avg.fill(1.0);
        self.sum = self.l as f64;
        self.pos = 0;
        self.head = 0;
        self.len = 0;
        self.n = 0;
        self.env = 1.0;
        self.gain_cur = self.gain_tgt;
    }

    /// Lookahead in samples (constant per sample rate).
    pub fn latency(&self) -> usize {
        self.l
    }

    pub fn set(&mut self, p: &LimiterParams) {
        self.p = *p;
        self.gain_tgt = 10f32.powf(p.gain_db.clamp(-24.0, 12.0) / 20.0);
    }

    #[inline]
    pub fn process(&mut self, l: &mut f32, r: &mut f32) {
        let pos = self.pos;
        self.pos = (pos + 1) % self.l;
        let clean = |v: f32| if v.is_finite() { v } else { 0.0 };
        if !self.p.on {
            // Keep the delay line current so switching on never replays stale audio.
            let (dl, dr) = (self.dly[0][pos], self.dly[1][pos]);
            (self.dly[0][pos], self.dly[1][pos]) = (clean(*l), clean(*r));
            (*l, *r) = (dl, dr);
            return;
        }
        self.gain_cur = self.gain_tgt + (self.gain_cur - self.gain_tgt) * self.a;
        let (xl, xr) = (clean(*l) * self.gain_cur, clean(*r) * self.gain_cur);
        let peak = xl.abs().max(xr.abs());
        let req = if peak > CEILING { CEILING / peak } else { 1.0 };

        // Sliding minimum of req over the last `win` samples.
        let cap = MAX_W;
        while self.len > 0 && self.dq[(self.head + self.len - 1) % cap].1 >= req {
            self.len -= 1;
        }
        self.dq[(self.head + self.len) % cap] = (self.n, req);
        self.len += 1;
        if self.dq[self.head].0 + self.win as u64 <= self.n {
            self.head = (self.head + 1) % cap;
            self.len -= 1;
        }
        self.n += 1;
        let held = self.dq[self.head].1;

        self.env = if held < self.env { held } else { held + (self.env - held) * self.rel };
        self.sum += (self.env - self.avg[pos]) as f64;
        self.avg[pos] = self.env;
        let g = (self.sum / self.l as f64) as f32;

        let (dl, dr) = (self.dly[0][pos], self.dly[1][pos]);
        (self.dly[0][pos], self.dly[1][pos]) = (xl, xr);
        *l = (dl * g).clamp(-CEILING, CEILING);
        *r = (dr * g).clamp(-CEILING, CEILING);
    }
}
