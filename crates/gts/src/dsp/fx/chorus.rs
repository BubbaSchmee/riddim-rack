//! FX 4.4 Chorus: 2..8 modulated delay voices, alternating left/right, stereo.
//!
//! - Each side has one delay line fed by its input channel through the wet low cut (24 dB/oct, so
//!   the sub never enters the wet path and the low end stays mono) plus feedback from the average
//!   of that side's taps (|loop gain| <= |feedback| < 1: always stable).
//! - Voice `v` reads side `v & 1` at `delay * SPREAD[v] + amp * sin(lfo + OFFSET[v])` with a cubic
//!   Hermite read. The offsets put each left voice a quarter turn from its right partner. The mod
//!   amplitude is `depth * min(delay/2, 0.03 / (2 pi rate))`: up to +-50 cents of pitch wobble at
//!   any rate, never a delay that runs backwards.
//! - Rate is free (0.1..10 Hz) or synced: [`Chorus::set_transport`] takes the tempo and the
//!   host's beat position and locks the LFO phase to `beat / sync_beats`.
//! - Wet = sum of a side's voices / sqrt(voice count), then the post low-pass, plus the dry signal
//!   low-passed at the low cut (so 100 % mix keeps a mono sub). Out = dry * (1 - mix) + wet * mix;
//!   the dry path is the input itself (no delay, no pitch change).
//! - Every audible param glides per sample (two-pole, ~20 ms; delay time ~100 ms so a change bends
//!   pitch instead of clicking); voice count changes fade voices in and out. `on = false` glides
//!   the mix to 0, then the module is a bit-exact pass-through that costs nothing. No allocation
//!   in `process`.
use std::f64::consts::TAU;

/// Delay line length per side: 37.5 ms max (20 ms * 1.25 spread + 12.5 ms mod) at 192 kHz.
const BUF: usize = 8192;
const MASK: usize = BUF - 1;
pub const MAX_VOICES: usize = 8;
/// Base delay multiplier per voice (pairs share one, so left and right sit at the same depth).
const SPREAD: [f64; MAX_VOICES] = [1.0, 1.0, 0.8, 0.8, 1.25, 1.25, 0.65, 0.65];
/// LFO phase offset per voice, turns.
const OFFSET: [f64; MAX_VOICES] = [0.0, 0.25, 0.5, 0.75, 0.125, 0.375, 0.625, 0.875];
/// Largest delay slew (= pitch deviation ratio) the depth can reach.
const MAX_SLEW: f64 = 0.03;
/// Per-stage glide time constants, seconds (two stages: ~20 ms and ~100 ms to 90 %).
const SMOOTH_S: f64 = 0.005;
const DELAY_SMOOTH_S: f64 = 0.025;
/// Butterworth 4th order section dampings (k = 1/Q).
const HP_K: [f64; 2] = [1.847_759_065_022_573_5, 0.765_366_864_730_179_7];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChorusParams {
    pub on: bool,
    /// 2..8.
    pub voices: u32,
    /// Free rate 0.1..10 Hz.
    pub rate_hz: f32,
    pub sync: bool,
    /// LFO period in quarter notes when synced (1/16 = 0.25 .. 8 bars = 32).
    pub sync_beats: f32,
    /// 0..1.
    pub depth: f32,
    /// Centre delay 1..20 ms.
    pub delay_ms: f32,
    /// -0.95..0.95.
    pub feedback: f32,
    /// Post low-pass on the wet, 200 Hz..20 kHz.
    pub lpf_hz: f32,
    /// Wet low cut 20..500 Hz.
    pub wet_lowcut_hz: f32,
    /// 0..1.
    pub mix: f32,
}

impl Default for ChorusParams {
    fn default() -> Self {
        Self {
            on: false,
            voices: 4,
            rate_hz: 0.5,
            sync: false,
            sync_beats: 4.0,
            depth: 0.5,
            delay_ms: 7.0,
            feedback: 0.0,
            lpf_hz: 20000.0,
            wet_lowcut_hz: 120.0,
            mix: 0.5,
        }
    }
}

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

/// TPT state-variable filter coefficients (A. Simper).
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
    /// One sample; returns (low, band, high).
    #[inline]
    fn tick(&self, z: &mut [f64; 2], x: f64) -> (f64, f64, f64) {
        let v3 = x - z[1];
        let v1 = self.a1 * z[0] + self.a2 * v3;
        let v2 = z[1] + self.a2 * z[0] + self.a3 * v3;
        z[0] = 2.0 * v1 - z[0];
        z[1] = 2.0 * v2 - z[1];
        (v2, v1, x - self.k * v1 - v2)
    }
}

pub struct Chorus {
    sr: f64,
    a: f64,
    a_delay: f64,
    p: ChorusParams,
    /// Delay lines, left and right.
    buf: [Vec<f32>; 2],
    w: usize,
    /// LFO phase, turns.
    phase: f64,
    tempo: f64,
    /// LFO rate, Hz (glides, so a rate change never jumps the mod amplitude).
    rate: Glide,
    depth: Glide,
    delay: Glide,
    fb: Glide,
    mix: Glide,
    /// log2 Hz.
    lpf: Glide,
    lowcut: Glide,
    voice: [Glide; MAX_VOICES],
    lp: Svf,
    hp: [Svf; 2],
    /// Low-pass at the low cut that hands the dry sub to the wet side (see `process`).
    keep: Svf,
    /// Filter states [side] / [side][section].
    lp_z: [[f64; 2]; 2],
    keep_z: [[f64; 2]; 2],
    hp_z: [[[f64; 2]; 2]; 2],
    /// (sin, cos) of each voice's LFO offset.
    off: [(f64, f64); MAX_VOICES],
    /// Lines hold stale audio from before the last bypass.
    stale: bool,
}

#[inline]
fn hermite(y0: f32, y1: f32, y2: f32, y3: f32, t: f32) -> f32 {
    let c1 = 0.5 * (y2 - y0);
    let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
    let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);
    ((c3 * t + c2) * t + c1) * t + y1
}

impl Chorus {
    pub fn new() -> Self {
        let g = Glide::at;
        let mut c = Self {
            sr: 48000.0,
            a: 1.0,
            a_delay: 1.0,
            p: ChorusParams::default(),
            buf: [vec![0.0; BUF], vec![0.0; BUF]],
            w: 0,
            phase: 0.0,
            tempo: 120.0,
            rate: g(0.5),
            depth: g(0.0),
            delay: g(0.007),
            fb: g(0.0),
            mix: g(0.0),
            lpf: g(20000f64.log2()),
            lowcut: g(120f64.log2()),
            voice: [g(0.0); MAX_VOICES],
            lp: Svf::default(),
            hp: [Svf::default(); 2],
            keep: Svf::default(),
            lp_z: [[0.0; 2]; 2],
            keep_z: [[0.0; 2]; 2],
            hp_z: [[[0.0; 2]; 2]; 2],
            off: OFFSET.map(|o| (TAU * o).sin_cos()),
            stale: false,
        };
        c.set(&ChorusParams::default());
        c.set_sample_rate(48000.0);
        c
    }

    pub fn set_sample_rate(&mut self, sr: f64) {
        self.sr = sr;
        self.a = 1.0 - (-1.0 / (SMOOTH_S * sr)).exp();
        self.a_delay = 1.0 - (-1.0 / (DELAY_SMOOTH_S * sr)).exp();
        self.reset();
    }

    pub fn latency(&self) -> usize {
        0
    }

    /// True while the module is a bit-exact pass-through (off or mix 0, glides finished).
    pub fn is_bypassed(&self) -> bool {
        self.mix.v == 0.0 && self.mix.t == 0.0
    }

    /// LFO phase in turns (for a UI indicator and tests).
    pub fn lfo_phase(&self) -> f64 {
        self.phase
    }

    pub fn set(&mut self, p: &ChorusParams) {
        self.p = *p;
        self.depth.t = p.depth.clamp(0.0, 1.0) as f64;
        self.delay.t = p.delay_ms.clamp(1.0, 20.0) as f64 * 1e-3;
        self.fb.t = p.feedback.clamp(-0.95, 0.95) as f64;
        self.mix.t = if p.on { p.mix.clamp(0.0, 1.0) as f64 } else { 0.0 };
        self.lpf.t = (p.lpf_hz.clamp(200.0, 20000.0) as f64).log2();
        self.lowcut.t = (p.wet_lowcut_hz.clamp(20.0, 500.0) as f64).log2();
        self.rate.t = self.rate_hz();
        let n = p.voices.clamp(2, MAX_VOICES as u32) as usize;
        for (i, v) in self.voice.iter_mut().enumerate() {
            v.t = if i < n { 1.0 } else { 0.0 };
        }
    }

    /// Host tempo and, while the transport plays, its beat position (quarter notes) at the start of
    /// this block. Synced, the LFO phase locks to `beat / sync_beats`.
    pub fn set_transport(&mut self, tempo_bpm: f64, beat_pos: Option<f64>) {
        self.tempo = tempo_bpm.max(1.0);
        self.rate.t = self.rate_hz();
        if let (true, Some(b)) = (self.p.sync, beat_pos) {
            self.phase = (b / self.sync_beats()).rem_euclid(1.0);
        }
    }

    fn sync_beats(&self) -> f64 {
        self.p.sync_beats.clamp(1.0 / 16.0, 32.0) as f64
    }

    fn rate_hz(&self) -> f64 {
        if self.p.sync {
            self.tempo / 60.0 / self.sync_beats()
        } else {
            self.p.rate_hz.clamp(0.1, 10.0) as f64
        }
    }

    /// Clear the lines and filters and jump every glide to its target.
    pub fn reset(&mut self) {
        for g in [&mut self.rate, &mut self.depth, &mut self.delay, &mut self.fb, &mut self.mix, &mut self.lpf, &mut self.lowcut] {
            g.jump();
        }
        for v in &mut self.voice {
            v.jump();
        }
        self.clear();
        self.update_filters();
    }

    fn clear(&mut self) {
        for b in &mut self.buf {
            b.fill(0.0);
        }
        self.lp_z = [[0.0; 2]; 2];
        self.keep_z = [[0.0; 2]; 2];
        self.hp_z = [[[0.0; 2]; 2]; 2];
        self.stale = false;
    }

    fn update_filters(&mut self) {
        self.lp = Svf::new(self.lpf.v.exp2(), self.sr, std::f64::consts::SQRT_2);
        let hz = self.lowcut.v.exp2();
        self.hp = HP_K.map(|k| Svf::new(hz, self.sr, k));
        self.keep = Svf::new(hz, self.sr, std::f64::consts::SQRT_2);
    }

    #[inline]
    fn read(&self, side: usize, d: f64) -> f32 {
        let d = d.clamp(3.0, (BUF - 4) as f64);
        let pos = self.w as f64 + BUF as f64 - d;
        let i = pos as usize;
        let t = (pos - i as f64) as f32;
        let b = &self.buf[side];
        hermite(b[(i - 1) & MASK], b[i & MASK], b[(i + 1) & MASK], b[(i + 2) & MASK], t)
    }

    #[inline]
    #[allow(clippy::needless_range_loop)] // voice index picks spread, offset, gain and side
    pub fn process(&mut self, l: &mut f32, r: &mut f32) {
        if self.is_bypassed() {
            self.stale = true;
            return;
        }
        if self.stale {
            self.clear();
        }
        let (a, sr) = (self.a, self.sr);
        let depth = self.depth.tick(a);
        let delay = self.delay.tick(self.a_delay);
        let fb = self.fb.tick(a);
        let mix = self.mix.tick(a);
        if self.lpf.v != self.lpf.t || self.lowcut.v != self.lowcut.t {
            self.lpf.tick(a);
            self.lowcut.tick(a);
            self.update_filters();
        }
        let rate = self.rate.tick(a);
        let amp = depth * (0.5 * delay).min(MAX_SLEW / (TAU * rate));
        let (s, c) = (TAU * self.phase).sin_cos();
        self.phase += rate / sr;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        let mut sum = [0.0f64; 2];
        let mut gsum = [0.0f64; 2];
        for v in 0..MAX_VOICES {
            let g = self.voice[v].tick(a);
            if g == 0.0 {
                continue;
            }
            let (so, co) = self.off[v];
            let m = s * co + c * so;
            let d = (delay * SPREAD[v] + amp * m) * sr;
            let side = v & 1;
            sum[side] += g * self.read(side, d) as f64;
            gsum[side] += g;
        }

        let x = [*l as f64, *r as f64];
        let mut out = [0.0f32; 2];
        for side in 0..2 {
            // Line input: wet low cut, then feedback from this side's tap average.
            let [z0, z1] = &mut self.hp_z[side];
            let h = self.hp[1].tick(z1, self.hp[0].tick(z0, x[side]).2).2;
            let mut wr = (h + fb * sum[side] / gsum[side]) as f32;
            if wr.abs() < 1e-20 {
                wr = 0.0;
            }
            self.buf[side][self.w] = wr;
            let wet = self.lp.tick(&mut self.lp_z[side], sum[side] / gsum[side].sqrt()).0;
            // The wet side carries the dry sub (12 dB/oct below the low cut), so the low end stays
            // mono and present at any mix, 100 % included.
            let keep = self.keep.tick(&mut self.keep_z[side], x[side]).0;
            out[side] = (x[side] * (1.0 - mix) + (wet + keep) * mix) as f32;
        }
        self.w = (self.w + 1) & MASK;
        if self.w & 63 == 0 {
            // Decaying filter states would reach f64 denormals about a second into silence.
            for z in self.lp_z.iter_mut().chain(&mut self.keep_z).chain(self.hp_z.iter_mut().flatten()).flatten() {
                if z.abs() < 1e-30 {
                    *z = 0.0;
                }
            }
        }
        *l = out[0];
        *r = out[1];
    }
}

impl Default for Chorus {
    fn default() -> Self {
        Self::new()
    }
}
