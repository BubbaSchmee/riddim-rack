//! FX 4.2 Multiband compressor, in the three-band upward/downward style: three bands, upward and downward compression.
//!
//! - Crossovers: Linkwitz-Riley 4th order (two Butterworth TPT SVF sections) at `xover_lo` and
//!   `xover_hi`. The low band runs through the 2nd-order allpass at `xover_hi` so the three bands
//!   sum to `AP_hi(AP_lo(x))`: flat magnitude. The dry side of `mix` is that same allpassed sum,
//!   so mix never combs at the crossovers.
//! - Per band: a stereo-linked mean-square detector (max of L^2, R^2) averaged over the attack
//!   time (symmetric one-pole), then held with instant rise and exponential release. Branching on
//!   the averaged level, not raw x^2, keeps a steady tone unbiased (branching on raw x^2 read a
//!   1 kHz sine 2 dB hot). Read in dB re a full-scale sine (a sine of peak -8 dBFS reads -8 dB).
//!   Downward
//!   above `down_db`, upward below `up_db`, hard knee, both with ratio `1 + 3 * depth`: depth 1.0
//!   (100 %) = 4:1, 0 = 1:1 (no gain change), 2.0 (200 %) = 7:1. Upward gain is capped at
//!   +24 dB so silence and noise floors are not raised without bound.
//! - Time 0..1 maps geometrically: attack 1 * 100^t ms (1..100), release 20 * 25^t ms (20..500);
//!   0.5 = 10 ms / 100 ms.
//! - Gain order: in -> split -> per-band compression and band gain -> sum -> out -> mix.
//! - `single_band`: the acceptance "single-band check" (SPEC 4.2). The crossovers are skipped and
//!   the whole signal runs through the mid band's compressor (mid thresholds and gain). Not a host
//!   parameter unless the integration wave wants one.
//! - No lookahead: latency 0. `on` fades the output against the raw input over 10 ms; once the
//!   fade-out ends, off is a sample-exact pass-through, and switching on again starts from cleared
//!   filters and detectors.
use std::f32::consts::{PI, SQRT_2};

/// Plain parameter values, set once per block with [`Multiband::set`]. Band index 0 = low,
/// 1 = mid, 2 = high.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MultibandParams {
    pub on: bool,
    /// 0..2 (0..200 %).
    pub depth: f32,
    /// 0..1.
    pub time: f32,
    /// 60..400 Hz.
    pub xover_lo: f32,
    /// 1..8 kHz.
    pub xover_hi: f32,
    /// Upward thresholds, -60..0 dB.
    pub up_db: [f32; 3],
    /// Downward thresholds, -60..0 dB.
    pub down_db: [f32; 3],
    /// Band output gains, -12..+12 dB.
    pub gain_db: [f32; 3],
    /// -24..+24 dB.
    pub in_db: f32,
    /// -24..+24 dB.
    pub out_db: f32,
    /// 0..1.
    pub mix: f32,
    pub single_band: bool,
}

impl Default for MultibandParams {
    fn default() -> Self {
        Self {
            on: false,
            depth: 1.0,
            time: 0.5,
            xover_lo: 120.0,
            xover_hi: 2500.0,
            up_db: [-24.0; 3],
            down_db: [-12.0; 3],
            gain_db: [0.0; 3],
            in_db: 0.0,
            out_db: 0.0,
            mix: 1.0,
            single_band: false,
        }
    }
}

const SMOOTH_S: f32 = 0.01;
const UP_MAX_DB: f32 = 24.0;

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
        (self.cur - self.tgt).abs() > 1e-6 * self.tgt.abs()
    }
}

#[inline]
fn db(x: f32) -> f32 {
    10f32.powf(x / 20.0)
}

/// Butterworth TPT SVF coefficients (k = sqrt 2) shared by every section at one frequency.
#[derive(Clone, Copy, Default)]
struct Coef {
    a1: f32,
    a2: f32,
    a3: f32,
}

impl Coef {
    fn new(sr: f32, f: f32) -> Self {
        let g = (PI * f.clamp(20.0, 0.45 * sr) / sr).tan();
        let a1 = 1.0 / (1.0 + g * (g + SQRT_2));
        Self { a1, a2: g * a1, a3: g * g * a1 }
    }
}

/// One SVF section's state; returns (lp, bp, hp).
#[derive(Clone, Copy, Default)]
struct Sec {
    ic1: f32,
    ic2: f32,
}

impl Sec {
    #[inline]
    fn run(&mut self, c: &Coef, x: f32) -> (f32, f32, f32) {
        let v3 = x - self.ic2;
        let v1 = c.a1 * self.ic1 + c.a2 * v3;
        let v2 = self.ic2 + c.a2 * self.ic1 + c.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        (v2, v1, x - SQRT_2 * v1 - v2)
    }
}

/// LR4 split: shared first section, separate second sections for the low and high outputs.
#[derive(Clone, Copy, Default)]
struct Split {
    s1: Sec,
    lp: Sec,
    hp: Sec,
}

impl Split {
    #[inline]
    fn run(&mut self, c: &Coef, x: f32) -> (f32, f32) {
        let (l, _, h) = self.s1.run(c, x);
        (self.lp.run(c, l).0, self.hp.run(c, h).2)
    }
}

pub struct Multiband {
    sr: f32,
    p: MultibandParams,
    a: f32,
    att: f32,
    rel: f32,
    lo_f: Sm,
    hi_f: Sm,
    lo_c: Coef,
    hi_c: Coef,
    split_lo: [Split; 2],
    split_hi: [Split; 2],
    ap: [Sec; 2],
    depth: Sm,
    up: [Sm; 3],
    down: [Sm; 3],
    gain: [Sm; 3],
    inp: Sm,
    out: Sm,
    mix: Sm,
    /// Detector mean square per band: attack-averaged, then release-held.
    ms: [f32; 3],
    env: [f32; 3],
    /// Last applied band gain in dB (metering / tests).
    gr: [f32; 3],
    /// Detector level per band in dB (metering).
    lvl: [f32; 3],
    /// On/off fade (see [`super::fade`]).
    en: f32,
}

impl Default for Multiband {
    fn default() -> Self {
        Self::new()
    }
}

impl Multiband {
    pub fn new() -> Self {
        let p = MultibandParams::default();
        let mut m = Self {
            sr: 48000.0,
            p,
            a: 0.0,
            att: 0.0,
            rel: 0.0,
            lo_f: Sm::new(p.xover_lo),
            hi_f: Sm::new(p.xover_hi),
            lo_c: Coef::default(),
            hi_c: Coef::default(),
            split_lo: [Split::default(); 2],
            split_hi: [Split::default(); 2],
            ap: [Sec::default(); 2],
            depth: Sm::new(p.depth),
            up: p.up_db.map(Sm::new),
            down: p.down_db.map(Sm::new),
            gain: p.gain_db.map(Sm::new),
            inp: Sm::new(1.0),
            out: Sm::new(1.0),
            mix: Sm::new(p.mix),
            ms: [0.0; 3],
            env: [0.0; 3],
            gr: [0.0; 3],
            lvl: [-120.0; 3],
            en: 0.0,
        };
        m.set_sample_rate(48000.0);
        m
    }

    pub fn set_sample_rate(&mut self, sr: f32) {
        self.sr = sr;
        self.a = (-1.0 / (SMOOTH_S * sr)).exp();
        self.set_times();
        self.reset();
    }

    pub fn reset(&mut self) {
        self.split_lo = [Split::default(); 2];
        self.split_hi = [Split::default(); 2];
        self.ap = [Sec::default(); 2];
        self.ms = [0.0; 3];
        self.env = [0.0; 3];
        self.gr = [0.0; 3];
        self.lvl = [-120.0; 3];
        self.en = if self.p.on { 1.0 } else { 0.0 };
        for s in [&mut self.lo_f, &mut self.hi_f, &mut self.depth, &mut self.inp, &mut self.out, &mut self.mix] {
            s.snap();
        }
        for s in self.up.iter_mut().chain(&mut self.down).chain(&mut self.gain) {
            s.snap();
        }
        self.lo_c = Coef::new(self.sr, self.lo_f.cur);
        self.hi_c = Coef::new(self.sr, self.hi_f.cur);
    }

    pub fn latency(&self) -> usize {
        0
    }

    /// Current per-band gain in dB (positive = upward), for meters and tests.
    pub fn band_gain_db(&self) -> [f32; 3] {
        self.gr
    }

    /// Detector level per band in dB (sine-peak referenced: a full-scale sine reads 0), for meters.
    pub fn band_level_db(&self) -> [f32; 3] {
        self.lvl
    }

    fn set_times(&mut self) {
        let t = self.p.time.clamp(0.0, 1.0);
        let ms = |x: f32| (-1.0 / (x * 0.001 * self.sr)).exp();
        self.att = ms(100f32.powf(t));
        self.rel = ms(20.0 * 25f32.powf(t));
    }

    /// True once off and faded out: `process` is then a pass-through.
    pub fn is_bypassed(&self) -> bool {
        !self.p.on && self.en == 0.0
    }

    pub fn set(&mut self, p: &MultibandParams) {
        let edge = p.on && self.is_bypassed();
        let time_changed = p.time != self.p.time;
        self.p = *p;
        if time_changed {
            self.set_times();
        }
        self.lo_f.tgt = p.xover_lo.clamp(60.0, 400.0);
        self.hi_f.tgt = p.xover_hi.clamp(1000.0, 8000.0);
        self.depth.tgt = p.depth.clamp(0.0, 2.0);
        for b in 0..3 {
            self.up[b].tgt = p.up_db[b].clamp(-60.0, 0.0);
            self.down[b].tgt = p.down_db[b].clamp(-60.0, 0.0);
            self.gain[b].tgt = p.gain_db[b].clamp(-12.0, 12.0);
        }
        self.inp.tgt = db(p.in_db.clamp(-24.0, 24.0));
        self.out.tgt = db(p.out_db.clamp(-24.0, 24.0));
        self.mix.tgt = p.mix.clamp(0.0, 1.0);
        if edge {
            // History frozen at the last switch-off would ring out through the band gains.
            self.reset();
            self.en = 0.0;
        }
    }

    /// Band gain (linear) for detector input `ms`, updating the band's envelope.
    #[inline]
    fn band(&mut self, b: usize, ms: f32, slope: f32) -> f32 {
        let a = self.a;
        let (up, down, gain) = (self.up[b].next(a), self.down[b].next(a), self.gain[b].next(a));
        let m = &mut self.ms[b];
        *m = ms + (*m - ms) * self.att;
        let m = *m;
        let e = &mut self.env[b];
        *e = if m > *e { m } else { m + (*e - m) * self.rel };
        let level = 10.0 * (2.0 * *e + 1e-20).log10();
        self.lvl[b] = level;
        let cut = (level - down).max(0.0) * slope;
        let boost = ((up - level).max(0.0) * slope).min(UP_MAX_DB);
        let g = boost - cut + gain;
        self.gr[b] = g;
        db(g)
    }

    #[inline]
    pub fn process(&mut self, l: &mut f32, r: &mut f32) {
        if self.is_bypassed() {
            return;
        }
        let en = super::fade(&mut self.en, self.p.on, self.a);
        let (dl, dr) = (*l, *r);
        self.process_on(l, r);
        if en < 1.0 {
            *l = dl + (*l - dl) * en;
            *r = dr + (*r - dr) * en;
        }
    }

    #[inline]
    fn process_on(&mut self, l: &mut f32, r: &mut f32) {
        let a = self.a;
        if self.lo_f.moving() || self.hi_f.moving() {
            self.lo_c = Coef::new(self.sr, self.lo_f.next(a));
            self.hi_c = Coef::new(self.sr, self.hi_f.next(a));
        }
        let depth = self.depth.next(a);
        let slope = 1.0 - 1.0 / (1.0 + 3.0 * depth);
        let (inp, out, mix) = (self.inp.next(a), self.out.next(a), self.mix.next(a));
        let x = [*l * inp, *r * inp];
        if self.p.single_band {
            for b in [0, 2] {
                self.up[b].next(a);
                self.down[b].next(a);
                self.gain[b].next(a);
            }
            let g = self.band(1, (x[0] * x[0]).max(x[1] * x[1]), slope);
            *l += (x[0] * g * out - *l) * mix;
            *r += (x[1] * g * out - *r) * mix;
            return;
        }
        let mut bands = [[0.0f32; 3]; 2];
        for ch in 0..2 {
            let (lo, rest) = self.split_lo[ch].run(&self.lo_c, x[ch]);
            let (mid, hi) = self.split_hi[ch].run(&self.hi_c, rest);
            let (_, bp, _) = self.ap[ch].run(&self.hi_c, lo);
            bands[ch] = [lo - 2.0 * SQRT_2 * bp, mid, hi];
        }
        let mut g = [0.0; 3];
        for (b, gb) in g.iter_mut().enumerate() {
            let ms = (bands[0][b] * bands[0][b]).max(bands[1][b] * bands[1][b]);
            *gb = self.band(b, ms, slope);
        }
        for (ch, v) in [l, r].into_iter().enumerate() {
            let [b0, b1, b2] = bands[ch];
            let dry = (b0 + b1 + b2) / inp;
            let wet = (b0 * g[0] + b1 * g[1] + b2 * g[2]) * out;
            *v = dry + (wet - dry) * mix;
        }
    }
}
