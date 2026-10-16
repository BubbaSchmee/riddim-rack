//! FX 4.1 Distortion: eight types, drive, bias, Off/Pre/Post filter, out trim, autogain, mix.
//!
//! - Tube, HardClip, Overdrive, Diode, FoldLin and FoldSine run inside the existing 4x
//!   [`Oversampler`] (`shaper::oversample`). BitCrush and Downsample run at 1x with no
//!   anti-imaging: their aliasing / zero-order-hold images are the effect (as in Vital's Bit crush
//!   and Down sample and Serum's Downsample); the Post low-pass is the user's tool to tame them.
//! - The six shaping curves also run first-order antiderivative anti-aliasing (ADAA, f64) at the
//!   4x rate: 4x alone left HardClip at -70 dB and FoldLin at -62 dB alias on the +12 dB C2-saw
//!   check. ADAA adds half a 4x sample (1/8 base sample) of delay to the wet path only.
//! - Latency is constant, [`Distortion::latency`] = 23 samples (the 4x oversampler's), for every
//!   type and for bypass: the 1x types and the dry path are delayed to match, so mix, type and
//!   on/off never shift in time. `on = false` outputs the input delayed by exactly 23 samples.
//! - Drive: gain into the curve for the six shaping types; for BitCrush it sets the bit depth
//!   (16 bits at -12 dB down to 2 bits at +36 dB), for Downsample the hold factor (1x .. 64x).
//! - Bias shifts the operating point, `f(g*x + b) - f(b)`: even harmonics without a static DC
//!   step; a 2 Hz DC blocker on the wet path removes the signal-dependent DC.
//! - Autogain: static level compensation per (type, drive, bias) normalised to an RMS-0.25 input
//!   (mean output power of a saw and a square at that level, as `shaper.rs` does), so drive moves
//!   tone, not loudness. Out trim and the filter act on the wet path only, so mix 0 = dry.
//! - Type changes are discrete (no crossfade): switching between a 4x and a 1x type mid-note can
//!   click once while the oversampler (or the 1x history) refills from silence.
//! - `on` fades the wet in and out over 10 ms; an off->on edge after the fade-out clears the wet
//!   history first.
use crate::dsp::shaper::oversample::{latency as os_latency, Oversampler};
use nih_plug::prelude::Enum;
use std::cell::Cell;
use std::f32::consts::PI;

/// Host enum names are part of the saved state and the preset format: append only.
#[derive(Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum DistType {
    Tube,
    #[name = "Hard Clip"]
    HardClip,
    Overdrive,
    Diode,
    #[name = "Fold Lin"]
    FoldLin,
    #[name = "Fold Sine"]
    FoldSine,
    #[name = "Bit Crush"]
    BitCrush,
    Downsample,
}

impl DistType {
    pub const ALL: [DistType; 8] = [
        DistType::Tube,
        DistType::HardClip,
        DistType::Overdrive,
        DistType::Diode,
        DistType::FoldLin,
        DistType::FoldSine,
        DistType::BitCrush,
        DistType::Downsample,
    ];

    /// Runs inside the 4x oversampler.
    pub fn oversampled(self) -> bool {
        !matches!(self, DistType::BitCrush | DistType::Downsample)
    }
}

#[derive(Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterMode {
    Off,
    Pre,
    Post,
}

#[derive(Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterType {
    #[name = "LP"]
    Lp,
    #[name = "BP"]
    Bp,
    #[name = "HP"]
    Hp,
}

/// Plain parameter values, set once per block with [`Distortion::set`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DistortionParams {
    pub on: bool,
    pub kind: DistType,
    /// -12..+36 dB.
    pub drive_db: f32,
    /// -1..1.
    pub bias: f32,
    pub filter_mode: FilterMode,
    pub filter_type: FilterType,
    /// 20 Hz..20 kHz.
    pub filter_freq: f32,
    /// 0.3..8.
    pub filter_q: f32,
    /// -24..+12 dB, wet path.
    pub out_db: f32,
    pub autogain: bool,
    /// 0..1.
    pub mix: f32,
}

impl Default for DistortionParams {
    fn default() -> Self {
        Self {
            on: false,
            kind: DistType::Tube,
            drive_db: 0.0,
            bias: 0.0,
            filter_mode: FilterMode::Off,
            filter_type: FilterType::Lp,
            filter_freq: 1000.0,
            filter_q: 0.707,
            out_db: 0.0,
            autogain: true,
            mix: 1.0,
        }
    }
}

const OS: usize = 4;
const LAT: usize = os_latency(OS);
const DLY: usize = 32;
/// Reference input RMS for autogain (a voice at full level, SPEC v2 1.3).
const REF_RMS: f32 = 0.25;
/// Parameter smoothing time constant.
const SMOOTH_S: f32 = 0.01;
const DC_HZ: f32 = 2.0;

/// Per-sample one-pole smoother.
#[derive(Clone, Copy)]
struct Sm {
    cur: f32,
    tgt: f32,
}

impl Sm {
    fn new(v: f32) -> Self {
        Self { cur: v, tgt: v }
    }
    #[inline]
    fn next(&mut self, a: f32) -> f32 {
        self.cur = self.tgt + (self.cur - self.tgt) * a;
        self.cur
    }
    fn snap(&mut self) {
        self.cur = self.tgt;
    }
    fn moving(&self) -> bool {
        (self.cur - self.tgt).abs() > 1e-6 * self.tgt.abs().max(1e-3)
    }
}

/// The curve for one base-rate sample (drive and bias fixed across its 4x sub-samples).
#[derive(Clone, Copy)]
struct Shape {
    kind: DistType,
    /// Drive gain (shaping types) or quantiser levels per unit (BitCrush).
    g: f32,
    b: f32,
    /// `f(b)`, subtracted so bias adds no static DC step.
    fb: f32,
}

impl Shape {
    fn new(kind: DistType, drive_db: f32, bias: f32) -> Self {
        let g = match kind {
            // 2^(bits - 1) levels, bits 16 -> 2 over the drive range.
            DistType::BitCrush => 2f32.powf(15.0 - 14.0 * ((drive_db + 12.0) / 48.0).clamp(0.0, 1.0)),
            _ => db(drive_db),
        };
        let mut s = Self { kind, g, b: bias, fb: 0.0 };
        s.fb = s.curve(bias);
        s
    }

    #[inline]
    fn curve(&self, x: f32) -> f32 {
        match self.kind {
            DistType::Tube => {
                if x >= 0.0 {
                    1.0 - (-x).exp()
                } else {
                    ((0.6 * x).exp() - 1.0) / 0.6
                }
            }
            DistType::HardClip => x.clamp(-1.0, 1.0),
            DistType::Overdrive => x / (1.0 + x * x).sqrt(),
            DistType::Diode => {
                if x >= 0.0 {
                    x.tanh()
                } else {
                    0.5 * (2.0 * x).tanh()
                }
            }
            DistType::FoldLin => {
                let t = 0.25 * (x - 1.0);
                1.0 - 4.0 * (t - (t + 0.5).floor()).abs()
            }
            DistType::FoldSine => (0.5 * PI * x).sin(),
            DistType::BitCrush => (x * self.g).round() / self.g,
            DistType::Downsample => x,
        }
    }

    #[inline]
    fn eval(&self, x: f32) -> f32 {
        let x = if self.kind == DistType::BitCrush { x } else { x * self.g };
        self.curve(x + self.b) - self.fb
    }

    /// One 4x-rate sample with ADAA1; `st` holds the previous curve input and its antiderivative.
    #[inline]
    fn eval_adaa(&self, st: &Cell<(f64, f64)>, u: f32) -> f32 {
        let v = (u * self.g + self.b) as f64;
        let (pv, pf) = st.get();
        let f = anti(self.kind, v);
        st.set((v, f));
        let d = v - pv;
        let y = if d.abs() > 1e-6 { ((f - pf) / d) as f32 } else { self.curve((0.5 * (v + pv)) as f32) };
        y - self.fb
    }

    /// Autogain: output gain that maps an RMS-0.25 saw / square to RMS 0.25.
    fn comp(&self) -> f32 {
        const M: usize = 64;
        let peak = REF_RMS * 3f32.sqrt();
        let saw = (0..M).map(|i| self.eval(((i as f32 + 0.5) / M as f32 * 2.0 - 1.0) * peak).powi(2)).sum::<f32>()
            / M as f32;
        let sq = 0.5 * (self.eval(REF_RMS).powi(2) + self.eval(-REF_RMS).powi(2));
        let ms = 0.5 * (saw + sq);
        if ms > 1e-12 {
            REF_RMS / ms.sqrt()
        } else {
            1.0
        }
    }
}

/// Antiderivative of each shaping curve (continuous everywhere, so ADAA differences are exact).
#[inline]
fn anti(kind: DistType, v: f64) -> f64 {
    let ln_cosh = |x: f64| x.abs() + (-2.0 * x.abs()).exp().ln_1p() - std::f64::consts::LN_2;
    match kind {
        DistType::Tube => {
            if v >= 0.0 {
                v + (-v).exp() - 1.0
            } else {
                ((0.6 * v).exp() / 0.6 - v) / 0.6 - 1.0 / 0.36
            }
        }
        DistType::HardClip => {
            if v.abs() <= 1.0 {
                0.5 * v * v
            } else {
                v.abs() - 0.5
            }
        }
        DistType::Overdrive => (1.0 + v * v).sqrt(),
        DistType::Diode => {
            if v >= 0.0 {
                ln_cosh(v)
            } else {
                0.25 * ln_cosh(2.0 * v)
            }
        }
        DistType::FoldLin => {
            let u = v - 1.0 - 4.0 * (0.25 * (v - 1.0) + 0.5).floor();
            u - 0.5 * u * u.abs()
        }
        DistType::FoldSine => -(std::f64::consts::FRAC_PI_2 * v).cos() / std::f64::consts::FRAC_PI_2,
        DistType::BitCrush | DistType::Downsample => 0.5 * v * v,
    }
}

#[inline]
fn db(x: f32) -> f32 {
    10f32.powf(x / 20.0)
}

/// Stereo TPT state-variable filter (Simper), BP normalised to unity peak.
#[derive(Clone, Copy)]
struct Svf {
    a1: f32,
    a2: f32,
    a3: f32,
    k: f32,
    ic: [[f32; 2]; 2],
}

impl Svf {
    fn tune(&mut self, sr: f32, f: f32, q: f32) {
        let g = (PI * f.clamp(20.0, 0.45 * sr) / sr).tan();
        self.k = 1.0 / q.clamp(0.3, 8.0);
        self.a1 = 1.0 / (1.0 + g * (g + self.k));
        self.a2 = g * self.a1;
        self.a3 = g * self.a2;
    }

    #[inline]
    fn process(&mut self, ch: usize, x: f32, t: FilterType) -> f32 {
        let [ic1, ic2] = &mut self.ic[ch];
        let v3 = x - *ic2;
        let v1 = self.a1 * *ic1 + self.a2 * v3;
        let v2 = *ic2 + self.a2 * *ic1 + self.a3 * v3;
        *ic1 = 2.0 * v1 - *ic1;
        *ic2 = 2.0 * v2 - *ic2;
        match t {
            FilterType::Lp => v2,
            FilterType::Bp => self.k * v1,
            FilterType::Hp => x - self.k * v1 - v2,
        }
    }
}

pub struct Distortion {
    sr: f32,
    p: DistortionParams,
    a: f32,
    drive: Sm,
    bias: Sm,
    comp: Sm,
    out: Sm,
    mix: Sm,
    freq: Sm,
    q: Sm,
    svf: Svf,
    os: [Oversampler; 2],
    /// ADAA state per channel: previous curve input and its antiderivative.
    adaa: [(f64, f64); 2],
    /// Input history (dry path and bypass) and the 1x wet history, both delayed by `LAT`.
    dry: [[f32; DLY]; 2],
    wet: [[f32; DLY]; 2],
    w: usize,
    /// Downsample: hold phase and held samples.
    hold_ph: f32,
    held: [f32; 2],
    /// DC blocker pole and state (x1, y1) per channel.
    dc_r: f32,
    dc: [[f32; 2]; 2],
    /// On/off fade (see [`super::fade`]): the wet/dry mix is scaled by it.
    en: f32,
}

impl Default for Distortion {
    fn default() -> Self {
        Self::new()
    }
}

impl Distortion {
    pub fn new() -> Self {
        let p = DistortionParams::default();
        let mut d = Self {
            sr: 48000.0,
            p,
            a: 0.0,
            drive: Sm::new(p.drive_db),
            bias: Sm::new(p.bias),
            comp: Sm::new(1.0),
            out: Sm::new(1.0),
            mix: Sm::new(p.mix),
            freq: Sm::new(p.filter_freq),
            q: Sm::new(p.filter_q),
            svf: Svf { a1: 0.0, a2: 0.0, a3: 0.0, k: 1.0, ic: [[0.0; 2]; 2] },
            os: [Oversampler::default(); 2],
            adaa: [(0.0, 0.0); 2],
            dry: [[0.0; DLY]; 2],
            wet: [[0.0; DLY]; 2],
            w: 0,
            hold_ph: 0.0,
            held: [0.0; 2],
            dc_r: 0.0,
            dc: [[0.0; 2]; 2],
            en: 0.0,
        };
        d.set_sample_rate(48000.0);
        d.apply(&p, true);
        d
    }

    pub fn set_sample_rate(&mut self, sr: f32) {
        self.sr = sr;
        self.a = (-1.0 / (SMOOTH_S * sr)).exp();
        self.dc_r = 1.0 - 2.0 * PI * DC_HZ / sr;
        self.svf.tune(sr, self.freq.cur, self.q.cur);
        self.reset();
    }

    pub fn reset(&mut self) {
        self.dry = [[0.0; DLY]; 2];
        self.clear_wet();
        self.en = if self.p.on { 1.0 } else { 0.0 };
    }

    /// Clears the wet path's history and snaps its smoothers (the dry history stays).
    fn clear_wet(&mut self) {
        self.os = [Oversampler::default(); 2];
        self.adaa = [(0.0, anti(self.p.kind, 0.0)); 2];
        self.wet = [[0.0; DLY]; 2];
        self.svf.ic = [[0.0; 2]; 2];
        self.hold_ph = 0.0;
        self.held = [0.0; 2];
        self.dc = [[0.0; 2]; 2];
        for s in [&mut self.drive, &mut self.bias, &mut self.comp, &mut self.out, &mut self.mix, &mut self.freq, &mut self.q] {
            s.snap();
        }
        self.svf.tune(self.sr, self.freq.cur, self.q.cur);
    }

    /// Constant processing delay in samples (bypass included).
    pub fn latency(&self) -> usize {
        LAT
    }

    pub fn set(&mut self, p: &DistortionParams) {
        self.apply(p, false);
    }

    /// True once off and faded out: `process` is then only the compensation delay.
    pub fn is_bypassed(&self) -> bool {
        !self.p.on && self.en == 0.0
    }

    fn apply(&mut self, p: &DistortionParams, force: bool) {
        let edge = p.on && self.is_bypassed();
        let drive = p.drive_db.clamp(-12.0, 36.0);
        let bias = p.bias.clamp(-1.0, 1.0);
        if force || p.kind != self.p.kind || drive != self.p.drive_db || bias != self.p.bias || p.autogain != self.p.autogain {
            self.comp.tgt = if p.autogain { Shape::new(p.kind, drive, bias).comp() } else { 1.0 };
        }
        if p.kind != self.p.kind {
            if p.kind.oversampled() && !self.p.kind.oversampled() {
                self.os = [Oversampler::default(); 2];
            } else if !p.kind.oversampled() && self.p.kind.oversampled() {
                // ponytail: refill from silence (as 1x->4x does) instead of replaying the 1x
                // history last written whenever a 1x type ran; a crossfade would remove the step.
                self.wet = [[0.0; DLY]; 2];
            }
            for st in &mut self.adaa {
                st.1 = anti(p.kind, st.0);
            }
        }
        self.drive.tgt = drive;
        self.bias.tgt = bias;
        self.out.tgt = db(p.out_db.clamp(-24.0, 12.0));
        self.mix.tgt = p.mix.clamp(0.0, 1.0);
        self.freq.tgt = p.filter_freq.clamp(20.0, 20000.0);
        self.q.tgt = p.filter_q.clamp(0.3, 8.0);
        self.p = DistortionParams { drive_db: drive, bias, ..*p };
        if edge && !force {
            // Wet history frozen at the last switch-off would replay; fade the wet in from clean.
            self.clear_wet();
            self.en = 0.0;
        }
    }

    /// Off, this is only the 23-sample delay (inlined, so a bypassed chain pays next to nothing).
    #[inline(always)]
    pub fn process(&mut self, l: &mut f32, r: &mut f32) {
        let w = self.w;
        self.w = (w + 1) % DLY;
        let rd = (w + DLY - LAT) % DLY;
        let x = [*l, *r];
        (self.dry[0][w], self.dry[1][w]) = (x[0], x[1]);
        let dry = [self.dry[0][rd], self.dry[1][rd]];
        if self.is_bypassed() {
            (*l, *r) = (dry[0], dry[1]);
            return;
        }
        let en = super::fade(&mut self.en, self.p.on, self.a);
        self.process_wet(l, r, x, dry, w, rd, en);
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn process_wet(&mut self, l: &mut f32, r: &mut f32, x: [f32; 2], dry: [f32; 2], w: usize, rd: usize, en: f32) {
        let a = self.a;
        let (drive, bias, comp, out, mix) =
            (self.drive.next(a), self.bias.next(a), self.comp.next(a), self.out.next(a), self.mix.next(a) * en);
        let fm = self.p.filter_mode;
        if fm != FilterMode::Off && (self.freq.moving() || self.q.moving()) {
            let (f, q) = (self.freq.next(a), self.q.next(a));
            self.svf.tune(self.sr, f, q);
        }
        let kind = self.p.kind;
        let ft = self.p.filter_type;
        let sh = Shape::new(kind, drive, bias);
        let hold = kind == DistType::Downsample && {
            // Hold factor 1..64 over the drive range; a new sample is taken when the phase wraps.
            let n = 2f32.powf(6.0 * ((drive + 12.0) / 48.0).clamp(0.0, 1.0));
            self.hold_ph += 1.0;
            let take = self.hold_ph >= n;
            if take {
                self.hold_ph -= n;
            }
            !take
        };
        let mut y = [0.0; 2];
        for ch in 0..2 {
            let mut v = x[ch];
            if fm == FilterMode::Pre {
                v = self.svf.process(ch, v, ft);
            }
            let mut s = if kind.oversampled() {
                let st = Cell::new(self.adaa[ch]);
                let y = self.os[ch].process(v, OS, |u| sh.eval_adaa(&st, u));
                self.adaa[ch] = st.get();
                y
            } else {
                let v1 = if kind == DistType::Downsample {
                    if !hold {
                        self.held[ch] = v;
                    }
                    self.held[ch]
                } else {
                    sh.eval(v)
                };
                self.wet[ch][w] = v1;
                self.wet[ch][rd]
            };
            s *= comp;
            let [x1, y1] = &mut self.dc[ch];
            let o = s - *x1 + self.dc_r * *y1;
            (*x1, *y1) = (s, o);
            s = o;
            if fm == FilterMode::Post {
                s = self.svf.process(ch, s, ft);
            }
            y[ch] = dry[ch] + (s * out - dry[ch]) * mix;
        }
        (*l, *r) = (y[0], y[1]);
    }
}
