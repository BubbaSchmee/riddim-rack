//! Voice management: polyphony, mono/legato, portamento, oldest-voice stealing, per-voice oscillators and amp envelopes.
//!
//! This file holds one voice; allocation (mono note stack, poly stealing) lives in `Engine`.
//! Per-sample stages of `Voice::next`, in order:
//!   1. glide (portamento in semitones),
//!   2. modulators: Mod A / Mod B LFOs -> `Voice::modulation` (Target, Mod B -> Mod A depth,
//!      wheel / velocity Mod amounts); Mod C / Mod D and the mod envelope run alongside (matrix
//!      sources only, SPEC v2 Track 7),
//!   3. per osc: pitch, Shift, Index + Motion, 1..7 unison grain oscillators, amp envelope, Vol,
//!      then the sine sub on Osc A's pitch and envelope,
//!   4. shared filter envelope, cutoffs (every `FILTER_TICK` samples), Shaper -> Filter A / B per
//!      `Routing`, then Spread; the sub joins after the filters (mono, undistorted, SQ-22),
//!   5. tail: once the amp envelopes are idle the voice keeps running until its output stays below
//!      -90 dBFS for a whole `TAIL_BLOCK`, capped at `TAIL_CAP` seconds with a 5 ms fade (SPEC v2 1.7).
//!
//! Stereo (SPEC v2 1.4): with unison width the oscillators are stereo, and the shaper and both
//! filters run a second (right) channel. A voice switches that channel on at the first block that
//! needs it (copying the left state, so nothing jumps) and back off only when it next starts idle.
//! A mono voice processes one channel and duplicates it, bit-identical to v1.
//!
//! Retriggering a sounding voice (poly steal, mono non-legato) resets the oscillator phases; the
//! step that leaves at each oscillator and the sub is faded out over `DECLICK` seconds (SQ-31).
//!
//! Track 7 (sound): a warped oscillator renders its unison sum on a 2x grid (FM, Bend, PWM) or a
//! 4x grid (Sync) and decimates it through an elliptic low-pass (`Dec`, about 2 / 2.4 samples of
//! group delay at low frequencies, so it stays aligned with an unwarped osc). FM's modulator is a
//! shadow copy of the other osc's centre grain voice (copied from it at note-on and whenever FM
//! starts, so it stays in phase) generated on the same grid: an image of the modulator is phase
//! modulation too, multiplied by the carrier's harmonic number, and a base-rate modulator brought
//! up to the grid (linearly, or through the shaper's 23-tap up stage) measured -25 .. -52 dB.
//! The sub gains Triangle (polyBLAMP corners), octave 0, a start phase and a soft drive; the
//! noise layer joins Osc A's path on Amp Env A, stereo-decorrelated.

use super::env::Adsr;
use super::filter::MultiFilter;
use super::grain::{GrainOsc, MapLut, PhaseMap};
use super::noise::{Noise, NoiseCfg, NOISE_OFF_DB};
use super::lfo::Lfo;
use super::shaper::Shaper;
use super::tables::{Tables, GRAINS, N, TOP};
use super::{OscPatch, Patch, Transport};
use super::shape::Shape;
use crate::params::{NoiseType, Routing, SubShape, Target, Trigger, WarpMode};
use std::f64::consts::TAU;

/// Most unison voices per oscillator.
pub const MAX_UNISON: usize = 7;

/// Per-sample modulation inputs for this voice, one slot per oscillator / filter (0 = A, 1 = B).
/// Zero means unmodulated. Written by the modulator stage at the top of `Voice::next`.
#[derive(Debug, Clone, Copy, Default)]
pub struct VoiceMod {
    /// Semitones added to the pitch.
    pub pitch: [f32; 2],
    /// Index units (0..127 scale) added to the index.
    pub index: [f32; 2],
    /// Semitones added to Shift.
    pub shift: [f32; 2],
    /// Added to Motion.
    pub motion: [f32; 2],
    /// Gain offset: the osc is scaled by `max(0, 1 + vol)`.
    pub vol: [f32; 2],
    /// Octaves added to Filter A / B cutoff (Mod B).
    pub cutoff: [f32; 2],
}

/// Engine-wide per-sample values every voice reads.
#[derive(Debug, Clone, Copy)]
pub struct Controls {
    pub sr: f32,
    /// Smoothed pitch bend in semitones.
    pub bend: f32,
    /// Smoothed mod wheel, 0..1.
    pub wheel: f32,
    /// One-pole coefficient for portamento.
    pub glide: f32,
    /// One-pole coefficient for Shift de-zippering (~5 ms).
    pub smooth: f32,
    /// One-pole coefficient of the 8 Hz per-osc DC blocker, `1 - exp(-2π·8/sr)`.
    pub dc: f32,
    /// `tan(π·25/sr)`: the sub's 25 Hz low-cut.
    pub sub_g: f32,
}

/// Unison layout of one oscillator, rebuilt per block from its patch.
#[derive(Debug, Clone, Copy)]
struct Unison {
    n: usize,
    /// Frequency ratio (detune) per voice.
    ratio: [f64; MAX_UNISON],
    /// Left / right gain per voice: blend, constant-power pan, 1/sqrt(sum of squares).
    gl: [f32; MAX_UNISON],
    gr: [f32; MAX_UNISON],
    stereo: bool,
}

impl Default for Unison {
    fn default() -> Self {
        Self { n: 1, ratio: [1.0; MAX_UNISON], gl: [1.0; MAX_UNISON], gr: [1.0; MAX_UNISON], stereo: false }
    }
}

impl Unison {
    /// Detune symmetric around the centre (`u` = -1..1 across the voices, cents = detune *
    /// sign(u) |u|^(1 + 2 det_curve)); centre voice(s) at 1, the others at `blend`; the voices
    /// pair off mirror-wise to opposite sides (k and n-1-k), alternating with k, so each side gets
    /// low and high detunes; pan = ±width, centre voice of an odd count in the middle.
    fn new(op: &OscPatch) -> Self {
        let n = op.unison.clamp(1, MAX_UNISON);
        let mut u = Self { n, ..Self::default() };
        if n == 1 {
            return u;
        }
        let c = (n - 1) as f32 / 2.0;
        let p = 1.0 + 2.0 * op.det_curve.clamp(0.0, 1.0);
        let width = op.width.clamp(0.0, 1.0);
        let blend = op.blend.clamp(0.0, 1.0);
        let mut amp = [0f32; MAX_UNISON];
        let mut pan = [0f32; MAX_UNISON];
        for k in 0..n {
            let x = (k as f32 - c) / c;
            let cents = op.detune * x.signum() * x.abs().powf(p);
            u.ratio[k] = 2f64.powf(cents as f64 / 1200.0);
            amp[k] = if (k as f32 - c).abs() < 0.75 { 1.0 } else { blend };
            if k < n / 2 {
                let side = if k % 2 == 0 { -1.0 } else { 1.0 };
                pan[k] = side * width;
                pan[n - 1 - k] = -side * width;
            }
        }
        let norm = 1.0 / amp[..n].iter().map(|a| a * a).sum::<f32>().sqrt();
        for k in 0..n {
            let th = (pan[k] + 1.0) * std::f32::consts::FRAC_PI_4;
            let g = std::f32::consts::SQRT_2 * amp[k] * norm;
            (u.gl[k], u.gr[k]) = (g * th.cos(), g * th.sin());
        }
        u.stereo = width > 0.0;
        u
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Osc {
    grain: [GrainOsc; MAX_UNISON],
    uni: Unison,
    env: Adsr,
    /// Smoothed Shift in semitones.
    shift: f32,
    /// Motion scan position in grains.
    scan: f64,
    vel_gain: f32,
    /// Last f0 in Hz (for tests / inspection).
    freq: f32,
    /// DC blocker state per channel: running mean of the grain oscillator output.
    dc: [f32; 2],
    /// The grains have been started at least once (a free-running osc starts at phase 0).
    started: bool,
    // ---- Track 7 (G4) ----
    /// Warp decimators (left, right); FM: the modulator shadow, its last value, the rate-deviation
    /// envelope and the value the mip follows, the shadow is in step.
    dec: Dec,
    fm_mod: GrainOsc,
    fm_last: f32,
    fm_dev: f64,
    fm_hold: f64,
    fm_live: bool,
    /// Grid factor rendered last sample (0 = unwarped); a change restarts the decimators.
    os: u8,
    /// `440 * 2^((pitch - 69) / 12)` and `2^(shift / 12)` for the pitch / Shift bits they were
    /// computed from.
    pitch_key: Option<u32>,
    hz: f32,
    shift_key: Option<u32>,
    ratio: f64,
    /// Unison count, detune, curve, blend, width `uni` was built from.
    uni_key: Option<[u32; 5]>,
    /// Warp mode and amount `map`, `map_fmul` and `sync_mul` were built for.
    warp_key: Option<(WarpMode, u32)>,
    map: PhaseMap,
    map_fmul: f64,
    sync_mul: f64,
}

/// What a warped osc needs of the other one: f0 (Sync) and the FM modulator's settings, plus its
/// centre grain voice when the FM shadow has to be (re)started from it.
struct Other {
    hz: f32,
    table: usize,
    ratio: f64,
    gp: f32,
    grain: Option<GrainOsc>,
}

/// Grid factor per warp mode: Sync 4x (its restart step needs the room), FM, Bend and PWM 2x (a
/// smooth map or a band-limited modulator needs only the room for its spread; half the cost).
fn warp_os(mode: WarpMode) -> usize {
    if mode == WarpMode::Sync { 4 } else { 2 }
}

/// Bend / PWM: highest instantaneous read rate (cycles per base sample) the 2x grid carries
/// without folding into the base band (`2 - 0.5`), less a margin for the map's spectral tails.
const MAP_HEADROOM: f64 = 1.3;
/// FM: table cycles of phase offset per unit of modulator at amount 1, and the per-sample decay of
/// the measured rate-deviation peak (~0.4 s at 48 kHz). The mip follows a held copy that only
/// moves when the peak rises above it or falls 30 % under it: a mip that toggled inside every
/// modulator period (blocks are not period-synchronous) would modulate the tone.
const FM_DEPTH: f64 = 2.0;
const FM_DEV_DECAY: f64 = 0.99995;
const FM_HOLD_DROP: f64 = 0.7;
/// FM's carrier headroom on its 2x grid (cycles per base sample; 1.3 measured -53 dB at C5, 1.0
/// -79 dB): the mip keeps the carrier's peak instantaneous frequency under it.
const FM_HEADROOM: f64 = 1.0;
/// Sync: the slave reads up to 2^SYNC_OCT table cycles per master period at amount 1.
const SYNC_OCT: f64 = 3.0;
/// Phase-map radius at amount 1: Bend (max slope 12) and PWM (max slope 19, ~5 % pulse).
const BEND_R: f64 = 0.85;
const PWM_R: f64 = 0.9;

/// Elliptic decimation low-passes, transposed direct form II sections `[b0, b1, b2, a1, a2]`,
/// 0.1 dB ripple to 0.417 x the base rate and -90 dB from 0.5 x (scipy `ellip(n, 0.1, 90, 20 kHz)`
/// at 96 / 192 kHz for a 48 kHz base; the edges scale with the sample rate). 2x: order 10.
#[allow(clippy::excessive_precision)]
const DEC2: [[f64; 5]; 5] = [
    [2.47497769852760745e-03, 4.53536748331540875e-03, 2.47497769852760788e-03, -1.14000400443441441e+00, 3.68539798235029470e-01],
    [1.0, 1.00016312224687787e+00, 1.0, -9.36816707540395588e-01, 5.28478004811736679e-01],
    [1.0, 3.58765141799458898e-01, 9.99999999999999889e-01, -7.04060073466292735e-01, 7.17536134276178017e-01],
    [1.0, 3.86701200794793698e-02, 1.0, -5.48668675816770812e-01, 8.59648437872647553e-01],
    [1.0, -8.69607230776464624e-02, 1.00000000000000022e+00, -4.83771943644881686e-01, 9.57833633587214184e-01],
];
/// 4x: order 11.
#[allow(clippy::excessive_precision)]
const DEC4: [[f64; 5]; 6] = [
    [1.93071631038950324e-04, 2.08695316013203195e-05, 1.93071631038950324e-04, -7.98730254601680789e-01, 0.0],
    [1.0, 1.0, 0.0, -1.58955157270586267e+00, 6.81999670256675516e-01],
    [1.0, -9.61499510015715875e-01, 9.99999999999999778e-01, -1.57292339079112953e+00, 7.78522565943983724e-01],
    [1.0, -1.29499170069459391e+00, 1.0, -1.55942598130098586e+00, 8.70964301342513236e-01],
    [1.0, -1.41561012511920570e+00, 1.0, -1.55567970384414256e+00, 9.36688784438000965e-01],
    [1.0, -1.45782546493645992e+00, 1.0, -1.56537860221310199e+00, 9.81228066608045402e-01],
];

/// Left and right decimator, one lane each (`z[section][state][lane]`: the two lanes vectorise).
#[derive(Debug, Clone, Copy, Default)]
struct Dec {
    z: [[[f64; 2]; 2]; 6],
}

impl Dec {
    #[inline(always)]
    fn process(&mut self, x: [f64; 2], sections: &[[f64; 5]]) -> [f64; 2] {
        let mut y = x;
        for (z, c) in self.z.iter_mut().zip(sections) {
            let mut o = [0.0; 2];
            for ch in 0..2 {
                o[ch] = c[0] * y[ch] + z[0][ch];
                z[0][ch] = c[1] * y[ch] - c[3] * o[ch] + z[1][ch];
                z[1][ch] = c[2] * y[ch] - c[4] * o[ch];
            }
            y = o;
        }
        y
    }

    /// The left lane only (a mono osc).
    #[inline(always)]
    fn process_left(&mut self, x: f64, sections: &[[f64; 5]]) -> f64 {
        let mut y = x;
        for (z, c) in self.z.iter_mut().zip(sections) {
            let o = c[0] * y + z[0][0];
            z[0][0] = c[1] * y - c[3] * o + z[1][0];
            z[1][0] = c[2] * y - c[4] * o;
            y = o;
        }
        y
    }
}

impl Osc {
    /// One base sample on the `OS`x grid: every unison voice through `f` (voice, its detune
    /// ratio, grid step) for all `OS` steps in turn, the steps' sums (in voice order, as one step
    /// at a time adds them) through the decimators. The last decimated step is the output.
    #[inline(always)]
    fn grid<const OS: usize>(&mut self, sections: &[[f64; 5]], mut f: impl FnMut(&mut GrainOsc, f64, usize) -> f32) -> (f32, f32) {
        let u = self.uni;
        // ponytail: `f` (a whole grain step) stays out of line here; inlining it into both loops
        // measured 40 % slower (register pressure, code size).
        if u.n == 1 {
            let mut l = 0.0;
            for s in 0..OS {
                l = self.dec.process_left(f(&mut self.grain[0], u.ratio[0], s) as f64, sections);
            }
            return (l as f32, l as f32);
        }
        let (mut a, mut b) = ([0.0f32; OS], [0.0f32; OS]);
        for k in 0..u.n {
            let g = &mut self.grain[k];
            for s in 0..OS {
                let v = f(g, u.ratio[k], s);
                a[s] += v * u.gl[k];
                b[s] += v * u.gr[k];
            }
        }
        let mut y = [0.0; 2];
        for s in 0..OS {
            y = self.dec.process([a[s] as f64, b[s] as f64], sections);
        }
        (y[0] as f32, y[1] as f32)
    }

    /// One base-rate sample of a warped osc (Track 7 G4): its unison sum (left, right) on its
    /// grid through the decimators, before DC blocking and gain.
    #[allow(clippy::too_many_arguments)]
    fn warped(
        &mut self,
        tables: &Tables,
        table: usize,
        op: &OscPatch,
        ratio: f64,
        gp: f32,
        sr: f32,
        other: Other,
        lut: Option<&MapLut>,
    ) -> (f32, f32) {
        let os = warp_os(op.warp);
        if self.os as usize != os {
            self.dec = Dec::default();
            self.os = os as u8;
        }
        let sections: &[[f64; 5]] = if os == 4 { &DEC4 } else { &DEC2 };
        let amt = op.warp_amt.clamp(0.0, 1.0) as f64;
        if self.warp_key != Some((op.warp, op.warp_amt.to_bits())) {
            // Per mode / amount change, not per sample: the map, its mip factor, Sync's rate factor.
            self.warp_key = Some((op.warp, op.warp_amt.to_bits()));
            self.map = warp_map(op.warp, op.warp_amt);
            self.map_fmul = (TOP * self.map.slope() / MAP_HEADROOM).max(1.0);
            self.sync_mul = 2f64.powf(SYNC_OCT * amt);
        }
        let osf = os as f64;
        let own_hz = self.freq;
        let dphase = own_hz as f64 / sr as f64;
        let ds = dphase / osf;
        match op.warp {
            WarpMode::Sync => {
                let master = other.hz as f64 / sr as f64 / osf;
                let rho = ratio * (own_hz / other.hz) as f64 * self.sync_mul;
                self.grid::<4>(sections, |g, k, _| g.next_sync(tables, table, master * k, rho, gp, osf))
            }
            WarpMode::Fm => {
                let rate = ratio * dphase;
                // Deviation peak in table cycles per base sample (from the last sample's grid steps).
                if self.fm_dev > self.fm_hold || self.fm_dev < FM_HOLD_DROP * self.fm_hold {
                    self.fm_hold = self.fm_dev;
                }
                let fmul = (TOP * (rate + self.fm_hold) / (FM_HEADROOM * rate)).max(1.0);
                if let Some(g) = other.grain {
                    (self.fm_mod, self.fm_live) = (g, true);
                }
                let md = other.hz as f64 / sr as f64 / osf;
                // The modulator first: its phase offsets for both grid steps.
                let mut off = [0.0; 2];
                for o in &mut off {
                    let mv = self.fm_mod.next_with(tables, other.table, md, other.ratio, other.gp, osf, 1.0, true, |p| p);
                    let dev = (mv - self.fm_last).abs() as f64 * amt * FM_DEPTH * osf;
                    self.fm_dev = dev.max(self.fm_dev * FM_DEV_DECAY);
                    self.fm_last = mv;
                    *o = amt * FM_DEPTH * mv as f64 * N as f64;
                }
                self.grid::<2>(sections, |g, k, s| g.next_with(tables, table, ds * k, ratio, gp, osf, fmul, true, |p| p + off[s]))
            }
            _ => {
                let (map, fmul) = (self.map, self.map_fmul);
                match lut {
                    Some(t) => self.grid::<2>(sections, |g, k, _| g.next_with(tables, table, ds * k, ratio, gp, osf, fmul, true, |p| t.apply(p))),
                    None => self.grid::<2>(sections, |g, k, _| g.next_with(tables, table, ds * k, ratio, gp, osf, fmul, true, |p| map.apply(p))),
                }
            }
        }
    }
}

/// Bend / PWM map for a warp mode and amount.
fn warp_map(mode: WarpMode, amt: f32) -> PhaseMap {
    let amt = amt.clamp(0.0, 1.0) as f64;
    match mode {
        WarpMode::BendPlus => PhaseMap::new(BEND_R * amt, 0.0),
        WarpMode::BendMinus => PhaseMap::new(-BEND_R * amt, 0.0),
        _ => PhaseMap::new(0.0, PWM_R * amt),
    }
}

/// Engine-wide Bend / PWM map tables, one per oscillator, for the engine patch's warp mode and
/// amount (Warp is not a matrix target, so every voice shares them; a voice whose patch differs
/// falls back to [`PhaseMap::apply`]).
#[derive(Default)]
pub struct WarpLuts {
    key: [Option<(WarpMode, u32)>; 2],
    lut: [MapLut; 2],
}

impl WarpLuts {
    /// Per sample (two compares): rebuilds a table when its osc's Bend / PWM setting changed.
    pub fn update(&mut self, patch: &Patch) {
        for (i, o) in patch.osc.iter().enumerate() {
            let key = Some((o.warp, o.warp_amt.to_bits()));
            if matches!(o.warp, WarpMode::BendPlus | WarpMode::BendMinus | WarpMode::Pwm) && self.key[i] != key {
                self.lut[i].build(&warp_map(o.warp, o.warp_amt));
                self.key[i] = key;
            }
        }
    }

    fn get(&self, i: usize, op: &OscPatch) -> Option<&MapLut> {
        (self.key[i] == Some((op.warp, op.warp_amt.to_bits()))).then(|| &self.lut[i])
    }
}

/// 2-pole Butterworth high-pass (TPT state-variable form).
#[derive(Debug, Clone, Copy, Default)]
struct Hp2 {
    s1: f32,
    s2: f32,
}

impl Hp2 {
    #[inline]
    fn process(&mut self, x: f32, g: f32) -> f32 {
        const K: f32 = std::f32::consts::SQRT_2;
        let hp = (x - (K + g) * self.s1 - self.s2) / (1.0 + K * g + g * g);
        let bp = g * hp + self.s1;
        self.s1 = g * hp + bp;
        let lp = g * bp + self.s2;
        self.s2 = g * bp + lp;
        hp
    }
}

/// Mod C / Mod D, constructed from their own seed counter (`Lfo::new_cd`) so the Mod A / B seeds
/// stay as they were.
struct LfoCd([Lfo; 2]);

impl Default for LfoCd {
    fn default() -> Self {
        Self([Lfo::new_cd(), Lfo::new_cd()])
    }
}

#[derive(Default)]
pub struct Voice {
    pub note: u8,
    pub velocity: f32,
    /// Key held.
    pub gate: bool,
    /// Trigger order; lower = older.
    pub age: u64,
    /// Current (gliding) pitch and its target, in MIDI semitones.
    pitch: f32,
    target: f32,
    osc: [Osc; 2],
    pub modulation: VoiceMod,
    /// Raw Mod A..D outputs (-1..1) of the last sample (Mod A / B are published on the bus).
    pub lfo_out: [f32; 4],
    /// Mod A, Mod B.
    lfo: [Lfo; 2],
    /// SPEC v2 Track 7: Mod C, Mod D and the mod envelope (matrix sources).
    lfo_cd: LfoCd,
    menv: Adsr,
    /// Filter A / B cutoff in Hz at the last cutoff update (for the GUI).
    pub cutoff_hz: [f32; 2],
    /// `[channel][filter]`: Filter A, Filter B for the left (and, in stereo, right) channel.
    filt: [[MultiFilter; 2]; 2],
    /// Snapshot of `filt` taken at a Type / Routing change, faded out over `XFADE` with the old
    /// routing (`old_routing`) while `filt` fades in. `xfade` runs 1 -> 0.
    old: [[MultiFilter; 2]; 2],
    routing: Option<Routing>,
    old_routing: Option<Routing>,
    xfade: f32,
    /// Left / right shaper.
    shaper: [Shaper; 2],
    /// Shaper wet amount, slewed toward Shaper On (0/1) so toggling does not click.
    shaper_mix: f32,
    /// Osc A / B On and Sub On, latched at `update`.
    on: [bool; 2],
    sub_on: bool,
    /// Second (right) channel of shaper and filters running.
    stereo: bool,
    fenv: Adsr,
    /// Samples until the next cutoff update.
    tick: u32,
    /// Set on a note-on: grains (re)start on the first `next`, once modulation is known.
    fresh: bool,
    /// The note-on retriggered a sounding voice: fade the phase-reset steps out.
    declick: bool,
    /// Last sample of [A L, A R, B L, B R, sub], the steps being faded, samples left.
    last: [f32; 5],
    click: [f32; 5],
    click_n: u32,
    /// Sub: phase in cycles, sine amplitude for the level, frequency multiplier, low-cut state.
    sub_phase: f64,
    sub_amp: f32,
    sub_mul: f32,
    sub_hp: Hp2,
    /// Unison random-phase generator (xorshift32, never 0).
    rng: u32,
    /// Still ringing (envelopes may be idle); samples since the envelopes went idle; block peak.
    tail: bool,
    tail_n: u32,
    tail_blk: u32,
    tail_peak: f32,
    // ---- Track 7 sound (G8) ----
    /// Noise layer generators (left, right), settings, on (latched at `update`).
    noise: [Noise; 2],
    noise_cfg: NoiseCfg,
    noise_key: Option<(NoiseType, [u32; 3])>,
    noise_on: bool,
}

/// Ping-pong period of the scan (0 -> 31 -> 0) and a common multiple with the 32-grain wrap.
const PINGPONG: f64 = 2.0 * (GRAINS - 1) as f64;
const SCAN_WRAP: f64 = 992.0;
/// Cutoff update interval in samples. The SVF takes stepped coefficients cleanly at this rate and
/// the comb smooths its delay internally.
const FILTER_TICK: u32 = 16;
/// Filter Type / Routing crossfade length in seconds.
const XFADE: f32 = 0.005;
/// Kbd tracking pivot: Filter Freq is exact at this note (C4) for any Kbd amount.
pub const KBD_REF: f32 = 60.0;
/// Tail end: a whole block of this many samples below -90 dBFS.
const TAIL_BLOCK: u32 = 64;
const TAIL_FLOOR: f32 = 3.162e-5;
/// Hard cap on a tail (seconds after the envelopes went idle) and the fade at the cap. Comb- at
/// res 1.0 on a low note rings ~8 s.
const TAIL_CAP: f32 = 8.0;
const TAIL_FADE: f32 = 0.005;
/// Retrigger declick length in seconds.
const DECLICK: f32 = 0.003;
/// Sub level reference: 0 dB = RMS 0.25, the grain level.
const SUB_REF: f32 = 0.25 * std::f32::consts::SQRT_2;
/// Phase-reset seed: every note of a phase-reset osc draws the same random phases.
const SEED: u32 = 0x2545_F491;

/// Bipolar velocity amount -> gain: +amt fades low velocities down, -amt fades high ones down.
#[inline]
pub(crate) fn vel_scale(amt: f32, vel: f32) -> f32 {
    if amt >= 0.0 { 1.0 - amt * (1.0 - vel) } else { 1.0 + amt * vel }
}

#[inline]
fn hits(t: Target, i: usize) -> bool {
    matches!((t, i), (Target::Both, _) | (Target::A, 0) | (Target::B, 1))
}

/// Uniform 0..1 from a xorshift32 state.
#[inline]
fn rand01(r: &mut u32) -> f64 {
    *r ^= *r << 13;
    *r ^= *r >> 17;
    *r ^= *r << 5;
    (*r >> 8) as f64 / (1u32 << 24) as f64
}

/// Any enabled oscillator with unison width needs the right channel (and the stereo noise layer).
fn needs_stereo(patch: &Patch) -> bool {
    patch.osc.iter().any(|o| o.on && o.unison > 1 && o.width > 0.0) || patch.noise_level_db > NOISE_OFF_DB
}

/// Band-limited triangle, 0 at phase 0 and +1 at 0.25: naive plus a 2-point polyBLAMP
/// `(1 - |t|)^3 / 6` per corner (slope change -8 / +8 per cycle at 0.25 / 0.75).
#[inline]
fn triangle(ph: f64, dt: f64) -> f32 {
    let mut y = if ph < 0.25 {
        4.0 * ph
    } else if ph < 0.75 {
        2.0 - 4.0 * ph
    } else {
        4.0 * ph - 4.0
    };
    for (c, ds) in [(0.25, -8.0), (0.75, 8.0)] {
        let t = ((ph - c) / dt).abs();
        if t < 1.0 {
            let x = 1.0 - t;
            y += ds * dt * x * x * x / 6.0;
        }
    }
    y as f32
}

/// Sub drive: `tanh(g w) / tanh(g)`, g = 1 + 4 drive (peak kept at 1).
#[inline]
fn sub_drive(w: f32, drive: f32) -> f32 {
    let g = 1.0 + 4.0 * drive.min(1.0);
    (g * w).tanh() / g.tanh()
}

impl Voice {
    /// An amp envelope that makes sound is running (oscillators, or the sub on Osc A's envelope).
    pub fn sounding(&self) -> bool {
        ((self.on[0] || self.sub_on || self.noise_on) && self.osc[0].env.is_active()) || (self.on[1] && self.osc[1].env.is_active())
    }

    /// Sounding, or its filters still ringing out.
    pub fn is_active(&self) -> bool {
        self.sounding() || self.tail
    }

    pub fn amp_env(&self, osc: usize) -> &Adsr {
        &self.osc[osc].env
    }

    /// Loudest enabled oscillator envelope, 0..1.
    pub fn amp_level(&self) -> f32 {
        (0..2).filter(|&i| self.on[i]).map(|i| self.osc[i].env.level()).fold(0.0, f32::max)
    }

    pub fn filter_env(&self) -> &Adsr {
        &self.fenv
    }

    /// Mod A..D phase in cycles (unwrapped; a finished one-shot is at or above 1).
    pub fn lfo_phase(&self, i: usize) -> f32 {
        if i < 2 { self.lfo[i].phase() } else { self.lfo_cd.0[i - 2].phase() }
    }

    /// The mod envelope (matrix source Mod Env).
    pub fn mod_env(&self) -> &Adsr {
        &self.menv
    }

    /// Mod A..D, in order.
    fn lfos(&mut self) -> impl Iterator<Item = &mut Lfo> {
        self.lfo.iter_mut().chain(self.lfo_cd.0.iter_mut())
    }

    /// Custom shape of LFO `k` (0 = Mod A .. 3 = Mod D).
    pub fn set_lfo_shape(&mut self, k: usize, s: &Shape) {
        if let Some(l) = self.lfos().nth(k) {
            l.set_shape(s);
        }
    }

    /// Current oscillator f0 in Hz (after glide, bend, octave/semi/cent and pitch modulation).
    pub fn freq(&self, osc: usize) -> f32 {
        self.osc[osc].freq
    }

    /// Current gliding note pitch in semitones.
    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// Index (0..127) of the newest grain.
    pub fn index(&self, osc: usize) -> f32 {
        (self.osc[osc].grain[0].grain_pos() / (GRAINS - 1) as f32 * 127.0).min(127.0)
    }

    /// Not real-time: resets the filters.
    pub fn set_sample_rate(&mut self, sr: f32) {
        for l in self.lfos() {
            l.set_sample_rate(sr as f64);
        }
        for f in self.filt.iter_mut().chain(&mut self.old).flatten() {
            f.set_sample_rate(sr as f64);
        }
        self.xfade = 0.0;
    }

    /// Shaper oversampling (1 / 2 / 4). Clears the shaper state when it changes.
    pub fn set_oversampling(&mut self, factor: usize) {
        for s in &mut self.shaper {
            s.set_oversampling(factor);
        }
    }

    pub fn shaper(&self) -> &Shaper {
        &self.shaper[0]
    }

    /// Right shaper / filters running (unison width somewhere in this note).
    pub fn is_stereo(&self) -> bool {
        self.stereo
    }

    /// Envelope times for this voice's velocity, shaper settings, unison layout, sub. Cheap;
    /// called per block and on trigger.
    pub fn update(&mut self, patch: &Patch, sr: f32) {
        // Velocity -> Attack: +1 makes a full-velocity attack 16x longer, -1 16x shorter.
        let att_scale = 2f32.powf(4.0 * patch.vel_att * self.velocity);
        for (o, op) in self.osc.iter_mut().zip(&patch.osc) {
            o.env.set_curved((op.att * att_scale).clamp(0.001, 10.0), op.dec, op.sus, op.rel, op.curve, sr);
            // The layout only when its settings moved (a matrix tick re-runs `update` every 32
            // samples; 7 powf + 7 sin / cos per osc otherwise).
            let key = [op.unison as u32, op.detune.to_bits(), op.det_curve.to_bits(), op.blend.to_bits(), op.width.to_bits()];
            if o.uni_key == Some(key) {
                continue;
            }
            o.uni_key = Some(key);
            let uni = Unison::new(op);
            // Voices joining mid-note start in phase with voice 0 instead of from stale state.
            for k in o.uni.n..uni.n {
                o.grain[k] = o.grain[0];
            }
            if o.uni.n == 1 && uni.n > 1 {
                o.dc[1] = o.dc[0];
            }
            o.uni = uni;
        }
        self.fenv.set_curved(patch.fenv_att, patch.fenv_dec, patch.fenv_sus, patch.fenv_rel, patch.fenv_curve, sr);
        self.menv.set_curved(patch.menv_att, patch.menv_dec, patch.menv_sus, patch.menv_rel, patch.menv_curve, sr);
        // The right shaper takes the left one's curve (the same computation, done once).
        let [sl, sr_] = &mut self.shaper;
        sl.set(patch.shaper_mode, patch.shaper_amt);
        sr_.set_like(sl);
        self.on = [patch.osc[0].on, patch.osc[1].on];
        self.sub_on = patch.sub_on;
        self.sub_amp = if patch.sub_on { SUB_REF * 10f32.powf(patch.sub_level_db / 20.0) } else { 0.0 };
        self.sub_mul = match patch.sub_oct {
            o if o <= -2 => 0.25,
            -1 => 0.5,
            _ => 1.0,
        };
        let key = (patch.noise_type, [patch.noise_level_db.to_bits(), patch.noise_tone.to_bits(), sr.to_bits()]);
        if self.noise_key != Some(key) {
            self.noise_key = Some(key);
            self.noise_cfg = NoiseCfg::new(patch.noise_type, patch.noise_level_db, patch.noise_tone, sr);
        }
        self.noise_on = self.noise_cfg.on();
        if needs_stereo(patch) && !self.stereo {
            if self.is_active() {
                let ([fl, fr], [ol, or]) = (&mut self.filt, &mut self.old);
                for j in 0..2 {
                    fr[j].copy_from(&fl[j]);
                    or[j].copy_from(&ol[j]);
                }
                self.shaper[1] = self.shaper[0].clone();
            }
            self.stereo = true;
        }
    }

    /// LFO rate / sync / curve and the Track 7 extensions, per block. Synced LFOs lock to the host
    /// beat only while it plays. Trigger Envelope = One-Shot; One-Shot on wins over Free.
    pub fn configure_mods(&mut self, patch: &Patch, transport: &Transport) {
        let beat = transport.playing.then_some(transport.pos_beats);
        let mods = [&patch.moda, &patch.modb, &patch.modc, &patch.modd];
        for ((l, m), e) in self.lfos().zip(mods).zip(&patch.lfo_ext) {
            let sync = m.sync.then_some((m.div, transport.tempo, beat));
            let one_shot = m.oneshot || e.trig == Trigger::Envelope;
            l.configure(m.curve, m.rate, sync, one_shot);
            l.set_ext(e.phase, e.fade, e.smooth, e.trig == Trigger::Free && !one_shot);
        }
    }

    /// Note-on for this voice. `glide` keeps the current pitch and slides to the note;
    /// `retrigger` restarts envelopes, LFOs, Motion scan and (per Phase Reset) the oscillator
    /// phases (false = legato move).
    pub fn trigger(&mut self, note: u8, velocity: f32, glide: bool, retrigger: bool, age: u64, patch: &Patch, ctl: &Controls) {
        let sounding = self.sounding();
        let active = self.is_active();
        self.note = note;
        self.gate = true;
        self.target = note as f32;
        if !glide || !sounding {
            self.pitch = self.target;
        }
        if !retrigger {
            return;
        }
        self.age = age;
        self.velocity = velocity;
        self.update(patch, ctl.sr);
        let vel_amts = [patch.vel_vol_a, patch.vel_vol_b];
        for i in 0..2 {
            let o = &mut self.osc[i];
            o.vel_gain = vel_scale(vel_amts[i], velocity);
            o.scan = 0.0;
            o.env.gate_on();
        }
        for l in self.lfos() {
            l.trigger();
        }
        self.menv.gate_on();
        if !active {
            // An idle voice froze its filter envelope wherever its tail ended.
            self.fenv.kill();
        }
        self.fenv.gate_on();
        self.tick = 0;
        self.tail = true;
        if !active {
            // Fresh voice: no ringing from the last note (fills the comb lines, no allocation).
            self.stereo = needs_stereo(patch);
            for ch in self.filt.iter_mut() {
                for (f, fp) in ch.iter_mut().zip(&patch.filt) {
                    f.reset();
                    f.set_kind(fp.ty);
                }
            }
            self.routing = Some(patch.routing);
            self.xfade = 0.0;
            self.shaper_mix = if patch.shaper_on { 1.0 } else { 0.0 };
        }
        if !active {
            // Track 7: decorrelate the noise of voices started together.
            let k = (age as u32).wrapping_mul(0x9E37_79B9);
            self.noise = [Noise::new(k ^ 0xA5A5_1234), Noise::new(k ^ 0x5A5A_4321)];
        }
        self.fresh = true;
        // Silent oscillators restart from zero; a sounding one leaves a step to fade out.
        self.declick = sounding;
        if !sounding {
            self.sub_hp = Hp2::default();
        }
    }

    /// Velocity -> level gains from `patch` (matrix: Vel Vol A / B modulated mid-note).
    pub fn set_vel_gain(&mut self, patch: &Patch) {
        for (o, amt) in self.osc.iter_mut().zip([patch.vel_vol_a, patch.vel_vol_b]) {
            o.vel_gain = vel_scale(amt, self.velocity);
        }
    }

    pub fn release(&mut self) {
        self.gate = false;
        for o in &mut self.osc {
            o.env.gate_off();
        }
        self.fenv.gate_off();
        self.menv.gate_off();
    }

    /// Hard stop (choke / reset / CC123 is a release, not this): no tail.
    pub fn kill(&mut self) {
        self.gate = false;
        for o in &mut self.osc {
            o.env.kill();
        }
        self.fenv.kill();
        self.menv.kill();
        self.tail = false;
        self.click_n = 0;
    }

    fn shift_target(&self, i: usize, patch: &Patch, ctl: &Controls) -> f32 {
        patch.osc[i].shift
            + 24.0 * (patch.wheel_shift * ctl.wheel + patch.vel_shift * self.velocity)
            + self.modulation.shift[i]
    }

    /// Grain position (0..32) from Index, wheel, modulation and the Motion scan offset.
    fn grain_pos(&self, i: usize, patch: &Patch, ctl: &Controls, scan: f64) -> f32 {
        let op = &patch.osc[i];
        let index = (op.index + 127.0 * patch.wheel_index * ctl.wheel + self.modulation.index[i]).clamp(0.0, 127.0);
        let base = (index / 127.0 * (GRAINS - 1) as f32) as f64;
        let motion = op.motion + self.modulation.motion[i];
        let gp = if motion > 0.0 {
            (base + scan).rem_euclid(GRAINS as f64)
        } else if motion < 0.0 {
            let x = (base + scan).rem_euclid(PINGPONG);
            if x > PINGPONG / 2.0 { PINGPONG - x } else { x }
        } else {
            base
        };
        gp as f32
    }

    /// `lfo_out` = the LFOs' values at their current phase (a new note's first sample).
    pub fn peek_lfos(&mut self) {
        let [c, d] = &self.lfo_cd.0;
        self.lfo_out = [self.lfo[0].peek(), self.lfo[1].peek(), c.peek(), d.peek()];
    }

    /// Mod A / Mod B -> `self.modulation`.
    fn modulate(&mut self, patch: &Patch, ctl: &Controls) {
        let (a, b) = (self.lfo[0].next(), self.lfo[1].next());
        let [lc, ld] = &mut self.lfo_cd.0;
        self.lfo_out = [a, b, lc.next(), ld.next()];
        self.menv.next();
        // Mod B "Mod A" amount, wheel Mod and velocity Mod all scale the Mod A depth.
        let depth = a
            * (1.0 + patch.modb_moda * b)
            * (1.0 + patch.wheel_mod * ctl.wheel)
            * vel_scale(patch.vel_mod, self.velocity);
        let mut m = VoiceMod::default();
        for i in 0..2 {
            if hits(patch.moda.target, i) {
                m.pitch[i] = 24.0 * patch.moda_pitch * depth;
                m.index[i] = 127.0 * patch.moda_index * depth;
                m.shift[i] = 24.0 * patch.moda_shift * depth;
            }
            // Mod B Target picks the oscillator for Motion / Vol and the filter for cutoff.
            if hits(patch.modb.target, i) {
                m.motion[i] = patch.modb_motion * b;
                m.vol[i] = patch.modb_vol * b;
                m.cutoff[i] = 5.0 * patch.modb_cutoff * b;
            }
        }
        self.modulation = m;
    }

    /// Filter A / B cutoffs from Freq, Kbd, filter env, Mod B and wheel.
    fn set_filters(&mut self, patch: &Patch, ctl: &Controls, fenv: f32) {
        let sign = if patch.fenv_inv { -1.0 } else { 1.0 };
        let env_oct = sign * 8.0 * patch.fenv_amt * fenv * vel_scale(patch.vel_fenv, self.velocity);
        // ponytail: wheel cutoff spans ±5 octaves like Mod B cutoff (SPEC gives no range).
        let wheel_oct = 5.0 * patch.wheel_cutoff * ctl.wheel;
        let chans = 1 + self.stereo as usize;
        for j in 0..2 {
            let fp = &patch.filt[j];
            // SPEC v2 1.6: combs follow the note fully at the default Kbd 0.5 (Kbd 0 = fixed);
            // the Comb- loudness match reads the feeding osc's fundamental.
            let comb = matches!(fp.ty, crate::params::FilterType::CombPlus | crate::params::FilterType::CombMinus);
            let kbd = if comb { (2.0 * fp.kbd).min(1.0) } else { fp.kbd };
            let mut oct = kbd * (self.pitch - KBD_REF) / 12.0 + self.modulation.cutoff[j] + wheel_oct;
            if fp.env {
                oct += env_oct;
            }
            let hz = (fp.freq * 2f32.powf(oct)).clamp(20.0, 0.45 * ctl.sr);
            self.cutoff_hz[j] = hz;
            for ch in &mut self.filt[..chans] {
                ch[j].set_note(self.osc[j].freq);
                ch[j].set(fp.ty, hz, fp.res);
            }
        }
    }

    /// (Re)starts the grains of every osc at a note-on: Phase Reset draws the seeded phases (the
    /// same every note), otherwise Rand Phase draws fresh ones, otherwise the osc runs free.
    fn start_grains(&mut self, patch: &Patch, ctl: &Controls, was_sounding: bool) {
        if self.rng == 0 {
            self.rng = SEED;
        }
        for i in 0..2 {
            let shift = self.shift_target(i, patch, ctl);
            let gp = self.grain_pos(i, patch, ctl, 0.0);
            let op = &patch.osc[i];
            let o = &mut self.osc[i];
            o.shift = shift;
            if op.phase_reset || op.rand_phase > 0.0 || !o.started {
                let ratio = 2f64.powf(shift as f64 / 12.0);
                let mut rng = if op.phase_reset { SEED ^ (i as u32 + 1).wrapping_mul(0x9E37_79B9) } else { self.rng };
                for g in &mut o.grain[..o.uni.n] {
                    let ph = if op.rand_phase > 0.0 { op.rand_phase.min(1.0) as f64 * rand01(&mut rng) } else { 0.0 };
                    g.reset_at(ratio, gp, ph);
                }
                if !op.phase_reset {
                    self.rng = rng;
                }
                o.started = true;
            }
            if !was_sounding {
                o.dc = [0.0; 2];
            }
            o.fm_live = false;
        }
        self.sub_phase = (patch.sub_phase as f64 / 360.0).rem_euclid(1.0);
    }

    /// One sample, (left, right) after routing, Spread and the sub.
    #[inline]
    pub fn next(&mut self, tables: &Tables, patch: &Patch, ctl: &Controls, luts: &WarpLuts) -> (f32, f32) {
        // 1. glide
        self.pitch = self.target + (self.pitch - self.target) * ctl.glide;

        // 2. modulators
        self.modulate(patch, ctl);
        let m = self.modulation;
        let declick = std::mem::take(&mut self.declick);
        if self.fresh {
            // Start grains in phase with Shift settled and the first grain at the modulated
            // Index (a one-shot into Index opens on its peak, not one period late).
            self.fresh = false;
            self.start_grains(patch, ctl, declick);
        }

        // 3. oscillators, [osc][channel]: both f0s first (Sync reads the other osc's), then render.
        let mut out = [[0.0f32; 2]; 2];
        let (mut envs, mut gps) = ([0.0f32; 2], [0.0f32; 2]);
        for i in 0..2 {
            envs[i] = self.osc[i].env.next();
            let op = &patch.osc[i];
            let motion = (op.motion + m.motion[i]).abs() as f64;
            let shift = self.shift_target(i, patch, ctl);
            let scan = self.osc[i].scan;
            gps[i] = self.grain_pos(i, patch, ctl, scan);
            let o = &mut self.osc[i];
            // Motion: 1.0 = whole table (32 grains) per 0.25 s.
            o.scan += motion * 128.0 / ctl.sr as f64;
            if o.scan >= SCAN_WRAP {
                o.scan -= SCAN_WRAP;
            }
            o.shift = shift + (o.shift - shift) * ctl.smooth;
            // Track 7 pitch offsets come last: adding 0 leaves the v1 pitch bit for bit.
            let pitch = self.pitch + 12.0 * op.oct as f32 + op.semi as f32 + op.cent / 100.0 + ctl.bend + m.pitch[i]
                + ([patch.osca_pitch, patch.oscb_pitch][i] + patch.pitch_all);
            // The two exponentials only when their input moved (both settle on held notes).
            if o.pitch_key != Some(pitch.to_bits()) {
                (o.pitch_key, o.hz) = (Some(pitch.to_bits()), 440.0 * 2f32.powf((pitch - 69.0) / 12.0));
            }
            if o.shift_key != Some(o.shift.to_bits()) {
                (o.shift_key, o.ratio) = (Some(o.shift.to_bits()), 2f64.powf(o.shift as f64 / 12.0));
            }
            o.freq = o.hz.clamp(1.0, 0.45 * ctl.sr);
        }
        let env_a = envs[0];
        for i in 0..2 {
            let (env, gp) = (envs[i], gps[i]);
            let op = &patch.osc[i];
            if !op.on || env <= 0.0 {
                continue;
            }
            // Track 7: the other osc as Sync master / FM modulator.
            let other = (op.warp != WarpMode::Off).then(|| {
                let (j, oo) = (1 - i, &self.osc[1 - i]);
                let fresh = op.warp == WarpMode::Fm && !self.osc[i].fm_live;
                Other {
                    hz: oo.freq,
                    table: patch.osc[j].table as usize,
                    ratio: if op.warp == WarpMode::Fm { oo.ratio } else { 1.0 },
                    gp: gps[j],
                    grain: fresh.then_some(oo.grain[0]),
                }
            });
            let o = &mut self.osc[i];
            let ratio = o.ratio;
            let dphase = o.freq as f64 / ctl.sr as f64;
            let table = op.table as usize;
            let gain = (1.0 + m.vol[i]).max(0.0);
            if let Some(other) = other {
                // Track 7 (G4).
                if op.warp != WarpMode::Fm {
                    o.fm_live = false;
                }
                let (l, r) = o.warped(tables, table, op, ratio, gp, ctl.sr, other, luts.get(i, op));
                o.dc[0] += ctl.dc * (l - o.dc[0]);
                let l = l - o.dc[0];
                let r = if o.uni.n == 1 {
                    l
                } else {
                    o.dc[1] += ctl.dc * (r - o.dc[1]);
                    r - o.dc[1]
                };
                let g = env * op.vol * o.vel_gain * gain;
                out[i] = [l * g, r * g];
                continue;
            }
            (o.os, o.fm_live) = (0, false);
            if o.uni.n == 1 {
                let s = o.grain[0].next(tables, table, dphase, ratio, gp);
                // DC blocker: negative Shift makes the grains a unipolar pulse train.
                o.dc[0] += ctl.dc * (s - o.dc[0]);
                let s = s - o.dc[0];
                let v = s * env * op.vol * o.vel_gain * gain;
                out[i] = [v, v];
            } else {
                let u = o.uni;
                let (mut l, mut r) = (0.0, 0.0);
                for k in 0..u.n {
                    let s = o.grain[k].next(tables, table, dphase * u.ratio[k], ratio, gp);
                    l += s * u.gl[k];
                    r += s * u.gr[k];
                }
                o.dc[0] += ctl.dc * (l - o.dc[0]);
                o.dc[1] += ctl.dc * (r - o.dc[1]);
                let g = env * op.vol * o.vel_gain * gain;
                out[i] = [(l - o.dc[0]) * g, (r - o.dc[1]) * g];
            }
        }

        // Sub: sine / triangle on Osc A's pitch (so it follows Pitch All and Osc A Pitch) and
        // envelope, start phase at note-on, optional soft drive, 25 Hz low-cut; mono.
        let mut sub = 0.0;
        if self.sub_on && env_a > 0.0 {
            let dp = (self.osc[0].freq * self.sub_mul) as f64 / ctl.sr as f64;
            self.sub_phase += dp;
            if self.sub_phase >= 1.0 {
                self.sub_phase -= 1.0;
            }
            let w = match patch.sub_shape {
                SubShape::Sine => (TAU * self.sub_phase).sin() as f32,
                SubShape::Triangle => triangle(self.sub_phase, dp),
            };
            let w = if patch.sub_drive > 0.0 { sub_drive(w, patch.sub_drive) } else { w };
            sub = w * self.sub_amp * env_a * self.osc[0].vel_gain;
        }
        let mut sub = self.sub_hp.process(sub, ctl.sub_g);

        // Retrigger declick: the reset left a step against the last sample; fade it out.
        let mut now = [out[0][0], out[0][1], out[1][0], out[1][1], sub];
        if declick {
            for j in 0..5 {
                self.click[j] = self.last[j] - now[j];
            }
            self.click_n = (DECLICK * ctl.sr) as u32;
        }
        if self.click_n > 0 {
            let w = self.click_n as f32 / (DECLICK * ctl.sr);
            for (v, c) in now.iter_mut().zip(self.click) {
                *v += c * w;
            }
            self.click_n -= 1;
            out = [[now[0], now[1]], [now[2], now[3]]];
            sub = now[4];
        }
        self.last = now;

        // Track 7 (G8): noise on Osc A's path and Amp Env A.
        if self.noise_on && env_a > 0.0 {
            let g = env_a * self.osc[0].vel_gain;
            let nl = self.noise[0].next(&self.noise_cfg) * g;
            let nr = if self.stereo { self.noise[1].next(&self.noise_cfg) * g } else { nl };
            out[0][0] += nl;
            out[0][1] += nr;
        }

        // 4. filter env, shaper, filters, routing
        let fenv = self.fenv.next();
        // A Type or Routing change crossfades from a snapshot of the old path (reset the changed
        // filter, ~5 ms fade) instead of switching with stale state.
        let chans = 1 + self.stereo as usize;
        let ty_changed = (0..2).any(|j| self.filt[0][j].kind() != patch.filt[j].ty);
        if ty_changed || self.routing != Some(patch.routing) {
            for c in 0..chans {
                for j in 0..2 {
                    self.old[c][j].copy_from(&self.filt[c][j]);
                    if self.filt[c][j].kind() != patch.filt[j].ty {
                        self.filt[c][j].reset();
                        self.filt[c][j].set_kind(patch.filt[j].ty);
                    }
                }
            }
            self.old_routing = self.routing;
            self.routing = Some(patch.routing);
            self.xfade = 1.0;
            self.tick = 0;
        }
        if self.tick == 0 {
            self.set_filters(patch, ctl, fenv);
            self.tick = FILTER_TICK;
        }
        self.tick -= 1;
        let on = if patch.shaper_on { 1.0 } else { 0.0 };
        self.shaper_mix = on + (self.shaper_mix - on) * ctl.smooth;
        if (self.shaper_mix - on).abs() < 1e-4 {
            self.shaper_mix = on;
        }
        let [a, b] = out;
        let spread = patch.spread.clamp(0.0, 1.0);
        let (mix, st) = (self.shaper_mix, self.stereo);
        let (mut l, mut r) = route(patch.routing, &mut self.filt, &mut self.shaper, mix, spread, st, a, b);
        if self.xfade > 0.0 {
            let (ol, or) = route(self.old_routing.unwrap_or(patch.routing), &mut self.old, &mut self.shaper, mix, spread, st, a, b);
            let g = self.xfade;
            self.xfade -= 1.0 / (XFADE * ctl.sr);
            (l, r) = (l + (ol - l) * g, r + (or - r) * g);
        }
        l += sub;
        r += sub;

        // 5. tail
        if self.sounding() {
            self.tail_n = 0;
            self.tail_blk = 0;
            self.tail_peak = 0.0;
            return (l, r);
        }
        self.tail_n += 1;
        let cap = (TAIL_CAP * ctl.sr) as u32;
        if self.tail_n > cap {
            let g = 1.0 - (self.tail_n - cap) as f32 / (TAIL_FADE * ctl.sr);
            if g <= 0.0 {
                self.tail = false;
                return (0.0, 0.0);
            }
            (l, r) = (l * g, r * g);
        }
        self.tail_peak = self.tail_peak.max(l.abs()).max(r.abs());
        self.tail_blk += 1;
        if self.tail_blk >= TAIL_BLOCK {
            if self.tail_peak < TAIL_FLOOR {
                self.tail = false;
            }
            self.tail_blk = 0;
            self.tail_peak = 0.0;
        }
        (l, r)
    }
}

/// One channel of Shaper -> Filter A / B per `routing`: (A path, B path). Series and Bypass carry
/// everything on the A path. `mix` crossfades dry -> shaped.
#[inline]
fn chan(routing: Routing, f: &mut [MultiFilter; 2], s: &mut Shaper, mix: f32, a: f32, b: f32) -> (f32, f32) {
    let mut shape = |x: f32| if mix > 0.0 { x + (s.process(x) - x) * mix } else { s.idle(x); x };
    match routing {
        // Shaper on A only, as in the original lineage.
        Routing::Parallel => {
            let fa = f[0].process(shape(a));
            (fa, f[1].process(b))
        }
        Routing::Series => {
            let y = f[0].process(shape(a + b));
            (f[1].process(y), 0.0)
        }
        Routing::Bypass => (shape(a + b), 0.0),
    }
}

/// Both channels (the right one only in stereo; mono duplicates the left), then Spread: in
/// Parallel it pans A left and B right.
#[inline]
#[allow(clippy::too_many_arguments)]
fn route(
    routing: Routing,
    f: &mut [[MultiFilter; 2]; 2],
    s: &mut [Shaper; 2],
    mix: f32,
    spread: f32,
    stereo: bool,
    a: [f32; 2],
    b: [f32; 2],
) -> (f32, f32) {
    let [fl, fr] = f;
    let [sl, sr] = s;
    let (al, bl) = chan(routing, fl, sl, mix, a[0], b[0]);
    let (ar, br) = if stereo { chan(routing, fr, sr, mix, a[1], b[1]) } else { (al, bl) };
    match routing {
        Routing::Parallel => (al + bl * (1.0 - spread), ar * (1.0 - spread) + br),
        _ => (al, ar),
    }
}
