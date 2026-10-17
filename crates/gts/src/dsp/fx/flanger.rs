//! FX Flanger (SPEC v2 Track 7, G9): one modulated short delay per side, stereo.
//!
//! - Swept delay = `delay * (1 - depth * (1 - sin(lfo)) / 2)`: it runs between `delay` (LFO top)
//!   and `delay * (1 - depth)` (LFO bottom), never below 3 samples (the cubic Hermite read needs
//!   them). The right side's LFO runs `spread` turns ahead of the left.
//! - Feedback (-0.95..0.95) writes `in + fb * tap` into the line: positive gives the classic
//!   resonant comb, negative the hollow odd-harmonic one. The loop gain is |fb| times the read's
//!   gain (<= 1), with at least 3 samples of delay: always stable.
//! - Rate is free (0.02..20 Hz) or synced through [`Flanger::set_transport`] (the phaser's
//!   [`super::phaser::SyncLfo`]).
//! - Out = dry + mix * en * (tap - dry), dry = the input itself (latency 0; mix 0.5 = deepest
//!   notches). Mix, depth and feedback glide per sample (10 ms), the delay over 25 ms so a change
//!   bends instead of clicking; `en` is the shared power fade ([`super::fade`]). Off and faded
//!   out, `process` returns at once (bit-exact pass-through); an off->on edge after the fade
//!   clears the lines first. The lines are allocated by `new` for 192 kHz.
use super::phaser::{glide, SyncLfo};
use std::f64::consts::TAU;

/// Line length per side: 10 ms + interpolation margin at 192 kHz.
const BUF: usize = 4096;
const MASK: usize = BUF - 1;
/// Shortest swept delay, samples (Hermite read).
pub const MIN_DELAY: f64 = 3.0;
const SMOOTH_S: f32 = 0.01;
const DELAY_SMOOTH_S: f32 = 0.025;
const MAX_SR: f32 = 192_000.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlangerParams {
    pub on: bool,
    /// Free rate 0.02..20 Hz.
    pub rate_hz: f32,
    pub sync: bool,
    /// LFO period in quarter notes when synced (1/16 = 0.25 .. 8 bars = 32).
    pub sync_beats: f32,
    /// Sweep depth 0..1 (fraction of `delay_ms` swept down).
    pub depth: f32,
    /// Longest (top of sweep) delay 0.1..10 ms.
    pub delay_ms: f32,
    /// -0.95..0.95.
    pub feedback: f32,
    /// Right LFO phase lead, turns 0..0.5.
    pub spread: f32,
    /// 0..1.
    pub mix: f32,
}

impl Default for FlangerParams {
    fn default() -> Self {
        Self { on: false, rate_hz: 0.2, sync: false, sync_beats: 4.0, depth: 0.8, delay_ms: 4.0, feedback: 0.5, spread: 0.25, mix: 0.5 }
    }
}

pub struct Flanger {
    sr: f32,
    p: FlangerParams,
    k: f32,
    kd: f32,
    en: f32,
    mix: f32,
    depth: f32,
    fb: f32,
    /// ms, gliding.
    delay: f32,
    buf: [Vec<f32>; 2],
    w: usize,
    lfo: SyncLfo,
    /// Swept delay the last `process` read per side, samples.
    d: [f64; 2],
}

impl Default for Flanger {
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
fn hermite(y0: f32, y1: f32, y2: f32, y3: f32, t: f32) -> f32 {
    let c1 = 0.5 * (y2 - y0);
    let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
    let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);
    ((c3 * t + c2) * t + c1) * t + y1
}

impl Flanger {
    pub fn new() -> Self {
        let mut s = Self {
            sr: 48000.0,
            p: FlangerParams::default(),
            k: 0.0,
            kd: 0.0,
            en: 0.0,
            mix: 0.0,
            depth: 0.0,
            fb: 0.0,
            delay: 4.0,
            buf: [vec![0.0; BUF], vec![0.0; BUF]],
            w: 0,
            lfo: SyncLfo::new(),
            d: [0.0; 2],
        };
        s.set_sample_rate(48000.0);
        s
    }

    pub fn set_sample_rate(&mut self, sr: f32) {
        self.sr = sr.clamp(8000.0, MAX_SR);
        self.k = (-1.0 / (SMOOTH_S * self.sr)).exp();
        self.kd = (-1.0 / (DELAY_SMOOTH_S * self.sr)).exp();
        self.lfo.set_sample_rate(self.sr as f64);
        let p = self.p;
        self.set(&p);
        self.reset();
    }

    pub fn latency(&self) -> usize {
        0
    }

    /// Clears the lines and jumps every glide (and the power fade) to its target.
    pub fn reset(&mut self) {
        let p = self.p;
        self.mix = p.mix.clamp(0.0, 1.0);
        self.depth = p.depth.clamp(0.0, 1.0);
        self.fb = p.feedback.clamp(-0.95, 0.95);
        self.delay = p.delay_ms.clamp(0.1, 10.0);
        self.en = if p.on { 1.0 } else { 0.0 };
        self.lfo.reset();
        for b in &mut self.buf {
            b.fill(0.0);
        }
    }

    /// True once off and faded out: `process` is then a bit-exact pass-through.
    pub fn is_bypassed(&self) -> bool {
        !self.p.on && self.en == 0.0
    }

    /// The wet share the last `process` mixed in (smoothed mix x power fade).
    pub fn wet_mix(&self) -> f32 {
        self.mix * self.en
    }

    pub fn lfo_phase(&self) -> f64 {
        self.lfo.phase
    }

    /// Swept delay the last `process` read per side, ms.
    pub fn delay_ms(&self) -> [f64; 2] {
        self.d.map(|d| d * 1000.0 / self.sr as f64)
    }

    pub fn set(&mut self, p: &FlangerParams) {
        let edge = p.on && self.is_bypassed();
        self.p = *p;
        self.lfo.set(p.rate_hz, p.sync, p.sync_beats);
        if edge {
            self.reset();
            self.en = 0.0;
        }
    }

    /// Host tempo and, while the transport plays, its beat position (quarter notes) at the start
    /// of this block.
    pub fn set_transport(&mut self, tempo_bpm: f64, beat: Option<f64>) {
        let idle = self.is_bypassed();
        self.lfo.set_transport(tempo_bpm, beat, idle);
    }

    #[inline]
    pub fn process(&mut self, l: &mut f32, r: &mut f32) {
        if self.is_bypassed() {
            return;
        }
        let (k, p) = (self.k, self.p);
        let en = super::fade(&mut self.en, p.on, k);
        let mix = glide(&mut self.mix, p.mix.clamp(0.0, 1.0), k) * en;
        let depth = glide(&mut self.depth, p.depth.clamp(0.0, 1.0), k) as f64;
        let fb = glide(&mut self.fb, p.feedback.clamp(-0.95, 0.95), k);
        let top = glide(&mut self.delay, p.delay_ms.clamp(0.1, 10.0), self.kd) as f64 * 1e-3 * self.sr as f64;
        let ph = self.lfo.tick(self.sr as f64);
        let spread = p.spread.clamp(0.0, 0.5) as f64;
        let x = [*l, *r];
        let mut out = [0.0f32; 2];
        for ch in 0..2 {
            let m = (TAU * (ph + spread * ch as f64)).sin();
            let d = (top * (1.0 - depth * 0.5 * (1.0 - m))).clamp(MIN_DELAY, (BUF - 4) as f64);
            self.d[ch] = d;
            let pos = (self.w + BUF) as f64 - d;
            let i = pos as usize;
            let t = (pos - i as f64) as f32;
            let b = &mut self.buf[ch];
            let y = hermite(b[(i - 1) & MASK], b[i & MASK], b[(i + 1) & MASK], b[(i + 2) & MASK], t);
            let mut wr = x[ch] + fb * y;
            if wr.abs() < 1e-20 {
                wr = 0.0;
            }
            b[self.w] = wr;
            out[ch] = x[ch] + mix * (y - x[ch]);
        }
        self.w = (self.w + 1) & MASK;
        (*l, *r) = (out[0], out[1]);
    }
}
