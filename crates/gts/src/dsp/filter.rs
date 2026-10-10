//! Filters A and B: LP12 and BP12 SVF (trapezoidal), Comb+, Comb-, and AM types.
use crate::params::FilterType;
use std::f64::consts::{PI, TAU};

/// Delay line long enough for 20 Hz at 192 kHz.
const BUF: usize = 9600 + 4;

pub struct MultiFilter {
    sr: f64,
    kind: FilterType,
    // SVF
    a1: f64,
    a2: f64,
    a3: f64,
    k: f64,
    ic1: f64,
    ic2: f64,
    // comb / AM
    buf: Vec<f32>,
    w: usize,
    delay: f32,
    delay_t: f32,
    mix: f32,
    fb: f32,
    phase: f64,
    inc: f64,
    fresh: bool,
}

impl MultiFilter {
    pub fn new() -> Self {
        let mut f = Self {
            sr: 44100.0,
            kind: FilterType::Lp12,
            a1: 0.0,
            a2: 0.0,
            a3: 0.0,
            k: 1.0,
            ic1: 0.0,
            ic2: 0.0,
            buf: vec![0.0; BUF],
            w: 0,
            delay: 100.0,
            delay_t: 100.0,
            mix: 0.0,
            fb: 0.0,
            phase: 0.0,
            inc: 0.0,
            fresh: true,
        };
        f.set(FilterType::Lp12, 1000.0, 0.0);
        f
    }

    pub fn set_sample_rate(&mut self, sr: f64) {
        self.sr = sr;
        self.reset();
    }

    pub fn reset(&mut self) {
        self.ic1 = 0.0;
        self.ic2 = 0.0;
        self.buf.fill(0.0);
        self.phase = 0.0;
        self.fresh = true;
    }

    pub fn set(&mut self, kind: FilterType, freq_hz: f32, res: f32) {
        self.kind = kind;
        let res = res.clamp(0.0, 1.0);
        let f = (freq_hz as f64).clamp(20.0, 0.45 * self.sr);
        match kind {
            FilterType::Lp12 | FilterType::Bp12 => {
                let q = 0.5 * 40f64.powf(res as f64); // 0.5 .. 20
                let g = (PI * f / self.sr).tan();
                self.k = 1.0 / q;
                self.a1 = 1.0 / (1.0 + g * (g + self.k));
                self.a2 = g * self.a1;
                self.a3 = g * self.a2;
            }
            FilterType::CombPlus | FilterType::CombMinus => {
                self.delay_t = ((self.sr / f) as f32).clamp(2.0, (BUF - 2) as f32);
                if self.fresh {
                    self.delay = self.delay_t;
                    self.fresh = false;
                }
                self.mix = res;
                self.fb = res * 0.98;
            }
            FilterType::Am => {
                self.inc = TAU * f / self.sr;
                self.mix = res;
            }
        }
    }

    /// Fractional read `d` samples behind the write head (2 <= d < BUF-1).
    fn tap(&self, d: f32) -> f32 {
        let i = d as usize;
        let fr = d - i as f32;
        let a = self.buf[(self.w + BUF - i) % BUF];
        let b = self.buf[(self.w + BUF - i - 1) % BUF];
        a + (b - a) * fr
    }

    pub fn process(&mut self, x: f32) -> f32 {
        match self.kind {
            FilterType::Lp12 | FilterType::Bp12 => {
                let x = x as f64;
                let v3 = x - self.ic2;
                let v1 = self.a1 * self.ic1 + self.a2 * v3;
                let v2 = self.ic2 + self.a2 * self.ic1 + self.a3 * v3;
                self.ic1 = 2.0 * v1 - self.ic1;
                self.ic2 = 2.0 * v2 - self.ic2;
                if self.kind == FilterType::Lp12 {
                    v2 as f32
                } else {
                    (self.k * v1) as f32
                }
            }
            FilterType::CombPlus => {
                self.delay += (self.delay_t - self.delay) * 0.01;
                let xd = self.tap(self.delay);
                self.buf[self.w] = x;
                self.w = (self.w + 1) % BUF;
                x + self.mix * 0.5 * (xd - x)
            }
            FilterType::CombMinus => {
                self.delay += (self.delay_t - self.delay) * 0.01;
                let y = x + self.fb * self.tap(self.delay);
                self.buf[self.w] = y;
                self.w = (self.w + 1) % BUF;
                y * (1.0 - self.fb)
            }
            FilterType::Am => {
                let m = (self.phase.sin() as f32) * self.mix;
                self.phase = (self.phase + self.inc) % TAU;
                x * (1.0 - self.mix) + x * m
            }
        }
    }
}

impl Default for MultiFilter {
    fn default() -> Self {
        Self::new()
    }
}
