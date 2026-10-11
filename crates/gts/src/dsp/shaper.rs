//! Waveshaper before Filter A: Sine, Saturate, Clip, Quant, Noise.
use crate::params::ShaperMode;

pub struct Shaper {
    mode: ShaperMode,
    amt: f32,
    rng: u32,
}

impl Shaper {
    pub fn new() -> Self {
        Self { mode: ShaperMode::Saturate, amt: 0.0, rng: 0x1234_5679 }
    }

    pub fn set(&mut self, mode: ShaperMode, amt: f32) {
        self.mode = mode;
        self.amt = amt.clamp(0.0, 1.0);
    }

    pub fn process(&mut self, x: f32) -> f32 {
        let a = self.amt;
        match self.mode {
            ShaperMode::Sine => (x * (1.0 + a * 8.0)).sin(),
            ShaperMode::Saturate => {
                let g = 1.0 + a * 10.0;
                (x * g).tanh() / g.tanh()
            }
            ShaperMode::Clip => {
                let t = 1.0 - a * 0.9;
                x.clamp(-t, t) / t
            }
            ShaperMode::Quant => {
                // Mid-tread (zero -> zero): 2^bits levels, bits 16 -> 2 with amt.
                let l = 2f32.powf(15.0 - a * 14.0); // 2^(bits-1)
                (x * l).round().clamp(-l, l - 1.0) / l
            }
            ShaperMode::Noise => {
                let mut r = self.rng;
                r ^= r << 13;
                r ^= r >> 17;
                r ^= r << 5;
                self.rng = r;
                x + (r as f32 / u32::MAX as f32 * 2.0 - 1.0) * x.abs() * a
            }
        }
    }
}

impl Default for Shaper {
    fn default() -> Self {
        Self::new()
    }
}
