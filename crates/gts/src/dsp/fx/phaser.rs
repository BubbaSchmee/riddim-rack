//! FX Phaser (SPEC v2 Track 7, G9): 4..12 first-order all-pass stages per side, stereo.
//!
//! - Each stage is a TPT first-order all-pass (phase 0 at DC to -180 deg at Nyquist) and every
//!   stage of a side shares one break frequency, so `n` stages cross an odd multiple of 180 deg
//!   `n / 2` times: at mix 0.5 (dry + wet in equal parts) the response has exactly `n / 2` notches,
//!   at `f = sr / pi * atan(g * tan((2k + 1) pi / (2n)))`, `g = tan(pi * break / sr)`.
//! - Break frequency = `centre * 2^(SPAN_OCT * depth * sin(lfo))`: depth 1 sweeps two octaves
//!   either side of the centre. The right side's LFO runs `spread` turns ahead of the left.
//! - Feedback (-0.95..0.95) returns the previous wet sample to the first stage. The loop is an
//!   all-pass (|gain| 1) times the feedback and one sample of delay, so |loop gain| <= 0.95:
//!   always stable.
//! - Stage count changes are click-free: the wet output glides between the taps after stages 4,
//!   6, 8, 10 and 12 (only the stages up to the taps in use run). Odd counts round down to even.
//! - Rate is free (0.02..20 Hz) or synced through [`Phaser::set_transport`] (phase locked to
//!   `beat / sync_beats`, a target jump slewed out over 50 ms, see [`SyncLfo`]).
//! - Out = dry + mix * en * (wet - dry), dry = the input itself (latency 0). Mix, depth, feedback
//!   and centre glide per sample (10 ms one-poles, the stage tap 30 ms); `en` is the shared power fade
//!   ([`super::fade`]). Off and faded out, `process` returns at once: a bit-exact pass-through. An
//!   off->on edge after the fade clears the stages first. No allocation anywhere after `new`.
use std::f64::consts::TAU;

/// Octaves the break frequency sweeps either side of the centre at depth 1.
pub const SPAN_OCT: f32 = 2.0;
pub const MIN_STAGES: u32 = 4;
pub const MAX_STAGES: u32 = 12;
const N: usize = MAX_STAGES as usize;
/// Parameter glide and power fade time constant, s (the delay's).
const SMOOTH_S: f32 = 0.01;
/// Stage-tap glide time constant, s (slower: a stage count change moves every notch).
const TAP_S: f32 = 0.03;
/// Synced phase-lock slew time constant, s (the chorus's).
const LOCK_S: f64 = 0.05;
const MAX_SR: f32 = 192_000.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhaserParams {
    pub on: bool,
    /// 4..12 (even; odd rounds down).
    pub stages: u32,
    /// Free rate 0.02..20 Hz.
    pub rate_hz: f32,
    pub sync: bool,
    /// LFO period in quarter notes when synced (1/16 = 0.25 .. 8 bars = 32).
    pub sync_beats: f32,
    /// 0..1.
    pub depth: f32,
    /// -0.95..0.95.
    pub feedback: f32,
    /// Centre break frequency 20..10000 Hz.
    pub centre_hz: f32,
    /// Right LFO phase lead, turns 0..0.5 (0 = mono sweep).
    pub spread: f32,
    /// 0..1 (0.5 = deepest notches).
    pub mix: f32,
}

impl Default for PhaserParams {
    fn default() -> Self {
        Self {
            on: false,
            stages: 8,
            rate_hz: 0.3,
            sync: false,
            sync_beats: 4.0,
            depth: 0.5,
            feedback: 0.3,
            centre_hz: 800.0,
            spread: 0.25,
            mix: 0.5,
        }
    }
}

/// Sine-LFO phase shared by the phaser and the flanger: free rate or tempo-synced, with the
/// chorus's beat lock (a jump in the locked target is slewed out, not snapped).
#[derive(Debug, Clone, Copy)]
pub(crate) struct SyncLfo {
    /// Turns, 0..1.
    pub phase: f64,
    rate: f64,
    rate_t: f64,
    free_hz: f64,
    sync: bool,
    beats: f64,
    tempo: f64,
    ph_err: f64,
    a_lock: f64,
    k: f64,
    /// Audible since the last reset (before that, a lock snaps).
    ran: bool,
}

impl SyncLfo {
    pub fn new() -> Self {
        Self { phase: 0.0, rate: 0.3, rate_t: 0.3, free_hz: 0.3, sync: false, beats: 4.0, tempo: 120.0, ph_err: 0.0, a_lock: 0.0, k: 0.0, ran: false }
    }
    pub fn set_sample_rate(&mut self, sr: f64) {
        self.a_lock = 1.0 - (-1.0 / (LOCK_S * sr)).exp();
        self.k = (-1.0 / (SMOOTH_S as f64 * sr)).exp();
    }
    fn target(&self) -> f64 {
        if self.sync {
            self.tempo / 60.0 / self.beats
        } else {
            self.free_hz
        }
    }
    pub fn set(&mut self, free_hz: f32, sync: bool, sync_beats: f32) {
        self.free_hz = free_hz.clamp(0.02, 20.0) as f64;
        self.sync = sync;
        self.beats = sync_beats.clamp(1.0 / 16.0, 32.0) as f64;
        self.rate_t = self.target();
    }
    /// Host tempo and, while playing, the beat position at the block start. `idle`: nothing
    /// audible depends on the current phase (module bypassed), so a lock may snap.
    pub fn set_transport(&mut self, tempo_bpm: f64, beat: Option<f64>, idle: bool) {
        self.tempo = tempo_bpm.max(1.0);
        self.rate_t = self.target();
        if !self.ran || idle {
            self.rate = self.rate_t;
        }
        if let (true, Some(b)) = (self.sync, beat) {
            let t = (b / self.beats).rem_euclid(1.0);
            if !self.ran || idle {
                (self.phase, self.ph_err) = (t, 0.0);
            } else {
                self.ph_err = (t - self.phase + 0.5).rem_euclid(1.0) - 0.5;
            }
        }
    }
    pub fn reset(&mut self) {
        (self.phase, self.ph_err, self.ran, self.rate) = (0.0, 0.0, false, self.rate_t);
    }
    /// Current phase, then advance one sample.
    #[inline]
    pub fn tick(&mut self, sr: f64) -> f64 {
        self.ran = true;
        let ph = self.phase;
        if self.rate != self.rate_t {
            self.rate = self.rate_t + (self.rate - self.rate_t) * self.k;
            if (self.rate - self.rate_t).abs() < 1e-9 {
                self.rate = self.rate_t;
            }
        }
        self.phase += self.rate / sr;
        if self.ph_err != 0.0 {
            let c = if self.ph_err.abs() < 1e-9 { self.ph_err } else { self.ph_err * self.a_lock };
            self.phase += c;
            self.ph_err -= c;
        }
        self.phase = self.phase.rem_euclid(1.0);
        ph
    }
}

#[inline]
pub(crate) fn glide(x: &mut f32, t: f32, k: f32) -> f32 {
    if *x != t {
        *x = t + (*x - t) * k;
        if (*x - t).abs() < 1e-6 * (1.0 + t.abs()) {
            *x = t;
        }
    }
    *x
}

pub struct Phaser {
    sr: f32,
    p: PhaserParams,
    k: f32,
    k_tap: f32,
    en: f32,
    mix: f32,
    depth: f32,
    fb: f32,
    /// log2 Hz, gliding, and its target.
    centre: f32,
    centre_t: f32,
    /// Tap position, 0 (4 stages) .. 4 (12 stages), gliding.
    tap: f32,
    /// Stage states [stage][side].
    s: [[f32; 2]; N],
    /// Stages that ran last sample.
    live: usize,
    /// Previous wet sample per side (the feedback).
    z: [f32; 2],
    lfo: SyncLfo,
    brk: [f32; 2],
    n: u32,
}

impl Default for Phaser {
    fn default() -> Self {
        Self::new()
    }
}

impl Phaser {
    pub fn new() -> Self {
        let mut s = Self {
            sr: 48000.0,
            p: PhaserParams::default(),
            k: 0.0,
            k_tap: 0.0,
            en: 0.0,
            mix: 0.0,
            depth: 0.0,
            fb: 0.0,
            centre: 0.0,
            centre_t: 0.0,
            tap: 0.0,
            s: [[0.0; 2]; N],
            live: N,
            z: [0.0; 2],
            lfo: SyncLfo::new(),
            brk: [800.0; 2],
            n: 0,
        };
        s.set_sample_rate(48000.0);
        s
    }

    pub fn set_sample_rate(&mut self, sr: f32) {
        self.sr = sr.clamp(8000.0, MAX_SR);
        self.k = (-1.0 / (SMOOTH_S * self.sr)).exp();
        self.k_tap = (-1.0 / (TAP_S * self.sr)).exp();
        self.lfo.set_sample_rate(self.sr as f64);
        let p = self.p;
        self.set(&p);
        self.reset();
    }

    pub fn latency(&self) -> usize {
        0
    }

    fn tap_t(&self) -> f32 {
        ((self.p.stages.clamp(MIN_STAGES, MAX_STAGES) - MIN_STAGES) / 2) as f32
    }

    /// Clears the stages and jumps every glide (and the power fade) to its target.
    pub fn reset(&mut self) {
        let p = self.p;
        self.mix = p.mix.clamp(0.0, 1.0);
        self.depth = p.depth.clamp(0.0, 1.0);
        self.fb = p.feedback.clamp(-0.95, 0.95);
        self.centre = self.centre_t;
        self.tap = self.tap_t();
        self.en = if p.on { 1.0 } else { 0.0 };
        self.lfo.reset();
        self.clear();
    }

    fn clear(&mut self) {
        (self.s, self.z, self.live) = ([[0.0; 2]; N], [0.0; 2], N);
    }

    /// True once off and faded out: `process` is then a bit-exact pass-through.
    pub fn is_bypassed(&self) -> bool {
        !self.p.on && self.en == 0.0
    }

    /// The wet share the last `process` mixed in (smoothed mix x power fade).
    pub fn wet_mix(&self) -> f32 {
        self.mix * self.en
    }

    /// LFO phase, turns (left side).
    pub fn lfo_phase(&self) -> f64 {
        self.lfo.phase
    }

    /// Break frequency the last `process` used per side, Hz.
    pub fn break_hz(&self) -> [f32; 2] {
        self.brk
    }

    pub fn set(&mut self, p: &PhaserParams) {
        let edge = p.on && self.is_bypassed();
        self.p = *p;
        self.centre_t = p.centre_hz.clamp(20.0, 10000.0).log2();
        self.lfo.set(p.rate_hz, p.sync, p.sync_beats);
        if edge {
            // Fresh stages, every glide at its target, then fade the wet in.
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
        let depth = glide(&mut self.depth, p.depth.clamp(0.0, 1.0), k);
        let fb = glide(&mut self.fb, p.feedback.clamp(-0.95, 0.95), k);
        let centre = glide(&mut self.centre, self.centre_t, k);
        let tap_t = self.tap_t();
        let tap = glide(&mut self.tap, tap_t, self.k_tap);
        let (lo, frac) = ((tap as usize).min(3), tap - (tap as usize).min(3) as f32);
        let ph = self.lfo.tick(self.sr as f64);
        let spread = p.spread.clamp(0.0, 0.5) as f64;
        let x = [*l, *r];
        let mut a = [0.0f32; 2];
        let mut u = [0.0f32; 2];
        for ch in 0..2 {
            let m = (TAU * (ph + spread * ch as f64)).sin() as f32;
            let hz = (centre + SPAN_OCT * depth * m).exp2().clamp(10.0, 0.45 * self.sr);
            self.brk[ch] = hz;
            let g = (std::f32::consts::PI * hz / self.sr).tan();
            a[ch] = g / (1.0 + g);
            u[ch] = x[ch] + fb * self.z[ch];
        }
        // Only the stages up to the taps in use run; a stage that comes back starts cleared (its
        // tap enters at zero weight, so the cold start is masked by the glide).
        let need = 4 + 2 * lo + if frac > 0.0 { 2 } else { 0 };
        if need > self.live {
            self.s[self.live..need].fill([0.0; 2]);
        }
        self.live = need;
        let mut taps = [[0.0f32; 2]; 5];
        // Both sides in lockstep: two independent dependency chains per stage.
        for (i, s) in self.s[..need].iter_mut().enumerate() {
            for ch in 0..2 {
                let v = (u[ch] - s[ch]) * a[ch];
                let lp = v + s[ch];
                s[ch] = lp + v;
                u[ch] = 2.0 * lp - u[ch];
            }
            if i & 1 == 1 && i >= 3 {
                taps[(i - 3) / 2] = u;
            }
        }
        let mut out = [0.0f32; 2];
        for ch in 0..2 {
            let wet = taps[lo][ch] + frac * (taps[lo + 1][ch] - taps[lo][ch]);
            self.z[ch] = wet;
            out[ch] = x[ch] + mix * (wet - x[ch]);
        }
        self.n = self.n.wrapping_add(1);
        if self.n & 63 == 0 {
            // Decaying stage states would reach f32 denormals in silence.
            for v in self.s.iter_mut().flatten().chain(&mut self.z) {
                if v.abs() < 1e-20 {
                    *v = 0.0;
                }
            }
        }
        (*l, *r) = (out[0], out[1]);
    }
}
