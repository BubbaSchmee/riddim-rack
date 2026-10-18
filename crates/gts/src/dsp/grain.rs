//! Pitch-synchronous grain playback: two overlapping Hann-windowed readers per oscillator; Index, Motion, Shift.
//!
//! A master phase runs at f0. Each wrap starts a new grain on the idle reader; every grain lasts
//! two periods under an output-frame Hann window, so the two readers always overlap 50 % and their
//! windows sum to exactly 1 at every Shift (no gaps, no window ripple: SPEC-v2 1.9). Each reader
//! moves through the table at `ratio * N` samples per period (`ratio = 2^(shift/12)`), so Shift
//! moves the spectrum while the grain rate (the pitch) stays f0. Grains are centred on table
//! position 0 (age 1 = window peak), where the tables put their main event (pulse edge, saw jump).
//!
//! At integer ratios >= 2 both readers sit on the same table phase and the output would be the
//! table at `ratio * f0` (+12 = an octave up). A small f0-rate phase wobble (`marker_amp`,
//! 0.12 rad on the table fundamental, fading out 0.15 away from the integer) keeps f0 present
//! without amplitude modulation. The level that the readers' phase mismatch costs (up to -14 dB
//! for a sine at -24) is restored by `Tables::level`, which also carries the per-table trim.
//! ponytail: a sine at a non-integer ratio (e.g. +7, 1.5 x f0) still comes out as two equal
//! neighbouring harmonics, i.e. full beating: any real window read at a constant rate gives
//! |W(-f)| = |W(f)|. Only a variable-rate (warped) read avoids it, and that quantises the
//! formant to integer ratios, so it was measured and rejected.
//!
//! Every `BLOCK` samples the mip is chosen from the effective read rate `f0 * ratio` (and the
//! pitch-marker depth refreshed); a mip change crossfades over the next block. At ratio 1 both readers read the same table position and the
//! output is plain single-cycle playback, periodic at f0. The grain position (Index) is latched
//! per grain, so any jump in Index (scan wrap, scan reset, stepped modulation) is crossfaded over
//! one period by the windows.
//!
//! Warp (SPEC v2 Track 7, G4; the voice runs a warped osc on a 2x or 4x grid and decimates):
//! - [`GrainOsc::next_with`] remaps both readers' table positions (FM: a phase offset; Bend / PWM:
//!   [`PhaseMap`]) and picks the mip for a multiple of the read rate, so the warped spectrum's
//!   peak instantaneous frequency stays where the oversampled grid does not fold it back.
//! - [`GrainOsc::next_sync`] is hard sync: one rectangular reader restarting at table position 0
//!   every period, `rho` table cycles per period, the restart step band-limited by a polyBLEP
//!   (the phase is deterministic, so the sample before the step is corrected too).

use super::tables::{mip_for, Tables, N, TOP};
use std::f64::consts::{PI, TAU};

/// Samples between mip / level updates, and the length of a mip crossfade.
const BLOCK: u8 = 32;
/// Pitch-marker depth in radians of the table fundamental, and the ratio distance it fades over.
const MARKER: f64 = 0.12;
const MARKER_WIDTH: f64 = 0.15;

/// Amplitude (frame samples) of the pitch marker's table-position wobble `amp * sin(2pi phase)`.
#[inline]
pub(crate) fn marker_amp(ratio: f64) -> f64 {
    let k = ratio.round();
    if k < 2.0 {
        return 0.0;
    }
    MARKER * (1.0 - (ratio - k).abs() / MARKER_WIDTH).max(0.0) / TAU * N as f64
}

/// Track 7 warp only (the unwarped path keeps libm, bit for bit): the grain window
/// `0.5 - 0.5 cos(pi phase)` for phase in [0, 1] as `0.5 + 0.5 sin(pi (phase - 0.5))`, sin(pi y)
/// on |y| <= 0.5 by a near-minimax polynomial (|err| < 1.4e-11; no libm call, no branch).
#[inline(always)]
fn hann(phase: f64) -> f64 {
    let y = phase - 0.5;
    let (u, y) = (y * y, 0.5 * y);
    let u2 = u * u;
    let s = (3.141_592_653_244_535_5 - 5.167_712_741_271_129 * u)
        + u2 * ((2.550_162_795_692_813 - 0.599_247_413_040_686_3 * u) + u2 * (0.082_031_260_698_699_13 - 0.007_000_542_739_186_552_5 * u));
    0.5 + y * s
}

/// Smooth monotonic phase warp (Bend+, Bend-, PWM): the Blaschke factor
/// `z -> (z - a) / (1 - conj(a) z)` on the unit circle, rotated so phase 0 stays at 0. It is
/// analytic, so a warped band-limited table stays band-limited around its peak instantaneous
/// frequency `slope()` x the plain one (no corners, nothing for the oversampling to miss).
/// Real `a` squeezes the table's start (a > 0, the grain's main event) or its middle (a < 0);
/// imaginary `a` squeezes the first half of the cycle into less time: pulse-width warp.
/// The warped oscs read it through a [`MapLut`]; `apply` is the exact map (reference, fallback).
#[derive(Debug, Clone, Copy, Default)]
pub struct PhaseMap {
    a: (f64, f64),
}

impl PhaseMap {
    pub fn new(re: f64, im: f64) -> Self {
        Self { a: (re, im) }
    }

    /// Largest phase slope, `(1 + |a|) / (1 - |a|)`.
    pub fn slope(&self) -> f64 {
        let r = self.a.0.hypot(self.a.1);
        (1.0 + r) / (1.0 - r)
    }

    /// Table position (frame samples, any real) -> warped position. `arg B(z) = 2 arg(z - a) -
    /// arg z`, less `arg B(1)`, wrapped to within half a cycle of `p`.
    pub fn apply(&self, p: f64) -> f64 {
        let th = TAU / N as f64 * p;
        let (s, c) = th.sin_cos();
        let off = 2.0 * (-self.a.1).atan2(1.0 - self.a.0);
        let d = 2.0 * (s - self.a.1).atan2(c - self.a.0) - 2.0 * th - off;
        let d = d - TAU * (d / TAU).round();
        p + d * (N as f64 / TAU)
    }
}

/// Nodes of a [`MapLut`] per cycle.
pub const LUT: usize = 1024;

/// A [`PhaseMap`] tabulated for the warped oscs' hot loop (SPEC v2 Track 7 CPU): the warp term
/// and its slope at [`LUT`] nodes, cubic Hermite in between, within 3e-7 rad of the exact map at
/// the steepest one (PWM at amount 1; tests/warp_opt.rs). ~4 ns per read against ~25 ns for the
/// libm map; [`MapLut::build`] costs ~50 us, so the engine keeps one per oscillator and rebuilds
/// it only when the warp mode or amount changes.
pub struct MapLut {
    /// `[d, h d']` per node (the map minus the identity in rad, its slope per node step), node
    /// `LUT` = node 0 so the last interval needs no wrap.
    tab: Box<[[f64; 2]]>,
}

impl Default for MapLut {
    /// The identity map. Allocates: build it off the audio thread.
    fn default() -> Self {
        Self { tab: vec![[0.0; 2]; LUT + 1].into_boxed_slice() }
    }
}

impl MapLut {
    /// Tabulates `m` (allocation-free). With `a = r e^(i alpha)`, `k = (1 + r) / (1 - r)` and
    /// `phi = theta - alpha`, the map minus the identity is `2 atan(k tan(phi / 2)) - phi =
    /// 2 atan2((k - 1) sin phi, (k + 1) - (k - 1) cos phi)` less its value at theta = 0: the
    /// denominator stays >= 2, so it is continuous over the cycle; its slope is the Poisson kernel
    /// of `a` minus 1.
    pub fn build(&mut self, m: &PhaseMap) {
        let (r, alpha) = (m.a.0.hypot(m.a.1), m.a.1.atan2(m.a.0));
        let k = (1.0 + r) / (1.0 - r);
        let g = |phi: f64| 2.0 * ((k - 1.0) * phi.sin()).atan2((k + 1.0) - (k - 1.0) * phi.cos());
        let off = g(-alpha);
        let h = TAU / LUT as f64;
        for (i, e) in self.tab.iter_mut().enumerate() {
            let phi = (i % LUT) as f64 * h - alpha;
            *e = [g(phi) - off, h * ((1.0 - r * r) / (1.0 - 2.0 * r * phi.cos() + r * r) - 1.0)];
        }
    }

    /// Table position (frame samples, > -4 N) -> warped position.
    #[inline(always)]
    pub fn apply(&self, p: f64) -> f64 {
        // Offset by 4 cycles so truncation is floor for the slightly negative marker positions.
        let x = p * (LUT as f64 / N as f64) + 4.0 * LUT as f64;
        let i = x as i64;
        let t = x - i as f64;
        let k = i as usize & (LUT - 1);
        let ([d0, m0], [d1, m1]) = (self.tab[k], self.tab[k + 1]);
        let c2 = 3.0 * (d1 - d0) - 2.0 * m0 - m1;
        let c3 = 2.0 * (d0 - d1) + m0 + m1;
        p + (d0 + t * (m0 + t * (c2 + t * c3))) * (N as f64 / TAU)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GrainOsc {
    /// Master phase in periods, 0..1.
    phase: f64,
    /// Table read position per reader, 0..N.
    pos: [f64; 2],
    /// Grain position (0..32) latched when each reader's grain started.
    gp: [f32; 2],
    /// Reader that started most recently.
    newest: usize,
    /// Current mip, the one being faded out, and the fade (1 = done).
    mip: u8,
    prev: u8,
    fade: f32,
    /// Samples into the current block.
    blk: u8,
    /// Output gain (`Tables::level`), smoothed toward `level_target` once per block.
    level: f32,
    level_target: f32,
    /// False until the first sample after `reset` set mip and level without fades.
    primed: bool,
    /// `marker_amp(ratio)`, refreshed per block.
    marker: f64,
    /// Sync: the restart step announced by the sample before it (0 = none pending).
    step: f32,
}

impl GrainOsc {
    /// Restart at phase 0 (fresh note on an idle voice).
    pub fn reset(&mut self, ratio: f64, gp: f32) {
        self.reset_at(ratio, gp, 0.0);
    }

    /// Restart at master phase `phase` (0..1 of a period; unison random phase).
    pub fn reset_at(&mut self, ratio: f64, gp: f32, phase: f64) {
        let n = N as f64;
        self.phase = phase;
        self.newest = 0;
        // Newest reader at age `phase`, the other at age 1 + `phase`; age 1 = grain centre =
        // table position 0. Phase 0 gives (-N * ratio, 0).
        self.pos = [((phase - 1.0) * n * ratio).rem_euclid(n), (phase * n * ratio).rem_euclid(n)];
        self.gp = [gp; 2];
        self.blk = 0;
        self.primed = false;
        self.step = 0.0;
    }


    /// Grain position of the newest grain (what the GUI shows).
    pub fn grain_pos(&self) -> f32 {
        self.gp[self.newest]
    }

    /// One sample. `dphase` = f0 / sr (must stay below 1), `ratio` = 2^(shift/12), `gp` = grain
    /// position for the next grain that starts.
    #[inline(never)]
    pub fn next(&mut self, tables: &Tables, table: usize, dphase: f64, ratio: f64, gp: f32) -> f32 {
        self.next_with(tables, table, dphase, ratio, gp, 1.0, 1.0, false, |p| p)
    }

    /// [`GrainOsc::next`] on a grid `os` times the base rate (`dphase` per grid sample), the mip
    /// picked for `fmul` times the base-rate read rate, both reader positions through `warp`.
    /// `fast` (the warped oscs) takes the grain window from [`hann`] instead of libm (1.4e-11
    /// off). `os = fmul = 1`, `fast = false` with the identity is `next` exactly.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub fn next_with(
        &mut self,
        tables: &Tables,
        table: usize,
        dphase: f64,
        ratio: f64,
        gp: f32,
        os: f64,
        fmul: f64,
        fast: bool,
        warp: impl Fn(f64) -> f64,
    ) -> f32 {
        let n = N as f64;
        let step = n * ratio * dphase;
        self.phase += dphase;
        for p in &mut self.pos {
            *p += step;
            if *p >= n {
                *p = p.rem_euclid(n);
            }
        }
        if self.phase >= 1.0 {
            self.phase -= 1.0;
            self.newest ^= 1;
            // The new grain began `phase` periods ago; its window peak (age 1) is at position 0.
            self.pos[self.newest] = ((self.phase - 1.0) * n * ratio).rem_euclid(n);
            self.gp[self.newest] = gp;
        }
        if self.blk == 0 {
            self.marker = marker_amp(ratio);
            // Off Shift 0 the two readers' windows and the pitch marker spread every harmonic over
            // about +-3 f0, so the top harmonic stays that far under Nyquist (SPEC-v2 1.1).
            let db = dphase * os;
            let top = if (ratio - 1.0).abs() < 1e-9 { TOP } else { TOP.min(0.5 - 3.0 * db) };
            let m = mip_for(ratio * db * fmul, top) as u8;
            self.level_target = tables.level(table, self.gp[self.newest], 12.0 * ratio.log2());
            if !self.primed {
                self.primed = true;
                (self.mip, self.prev, self.fade, self.level) = (m, m, 1.0, self.level_target);
            } else if m != self.mip {
                (self.prev, self.mip, self.fade) = (self.mip, m, 0.0);
            }
        }
        self.blk = (self.blk + 1) % BLOCK;
        self.level += (self.level_target - self.level) * (1.0 / BLOCK as f32);

        let (a, b) = (self.newest, self.newest ^ 1);
        let mk = if self.marker > 0.0 { self.marker * (TAU * self.phase).sin() } else { 0.0 };
        let (pa, pb, ga, gb) = (self.pos[a] + mk, self.pos[b] + mk, self.gp[a], self.gp[b]);
        // Both readers on the same grain and table position (Shift 0, steady Index): the windows
        // sum to 1, so one read is the whole sample (half the work; the unison hot path).
        let same = ga == gb && (pa - pb).abs() < 1e-6;
        // Track 7 warp: the map once when both readers share the position.
        let (pa, pb) = if same {
            let w = warp(pa);
            (w, w)
        } else {
            (warp(pa), warp(pb))
        };
        let w = if same {
            1.0
        } else {
            (if fast { hann(self.phase) } else { 0.5 - 0.5 * (PI * self.phase).cos() }) as f32
        };
        // The mip fade-out reads are rare: out of line, so the hot reads can inline.
        let read = |m: u8, g: f32, p: f64, cold: bool| {
            if cold { tables.read_cold(table, m as usize, g, p) } else { tables.read(table, m as usize, g, p) }
        };
        let mix = |m: u8, cold: bool| {
            let sa = read(m, ga, pa, cold);
            if same { sa } else { w * sa + (1.0 - w) * read(m, gb, pb, cold) }
        };
        let mut s = mix(self.mip, false);
        if self.fade < 1.0 {
            self.fade = (self.fade + 1.0 / BLOCK as f32).min(1.0);
            s = s * self.fade + mix(self.prev, true) * (1.0 - self.fade);
        }
        s * self.level
    }

    /// Hard sync (Track 7 G4) on a grid `os` times the base rate: master phase at `dphase` per
    /// grid sample, one reader at `rho` table cycles per master period restarting at position 0
    /// on every wrap, mip for the read rate `rho * dphase * os` per base sample, level of Shift 0.
    /// The restart step gets a 2-point polyBLEP on this grid: announced one sample early (the
    /// phase is deterministic), so both neighbours of the step are corrected.
    #[inline]
    pub fn next_sync(&mut self, tables: &Tables, table: usize, dphase: f64, rho: f64, gp: f32, os: f64) -> f32 {
        let n = N as f64;
        self.phase += dphase;
        let mut after = None;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
            self.gp[0] = gp;
            after = Some(self.phase / dphase);
        }
        if self.blk == 0 {
            let m = mip_for(rho * dphase * os, TOP) as u8;
            self.level_target = tables.level(table, self.gp[0], 0.0);
            if !self.primed {
                self.primed = true;
                (self.mip, self.prev, self.fade, self.level) = (m, m, 1.0, self.level_target);
            } else if m != self.mip {
                (self.prev, self.mip, self.fade) = (self.mip, m, 0.0);
            }
        }
        self.blk = (self.blk + 1) % BLOCK;
        self.level += (self.level_target - self.level) * (1.0 / BLOCK as f32);
        let (g, top) = (self.gp[0], rho * n);
        // Hot: this read. Out of line: the fade-out and the step reads (once per period).
        let mut s = tables.read(table, self.mip as usize, g, top * self.phase);
        let read = |m: u8, ph: f64| tables.read_cold(table, m as usize, g, top * ph);
        if self.fade < 1.0 {
            self.fade = (self.fade + 1.0 / BLOCK as f32).min(1.0);
            s = s * self.fade + read(self.prev, self.phase) * (1.0 - self.fade);
        }
        if let Some(t) = after {
            // Step h = (start of the new period) - (end of the old one), announced last sample.
            let h = if self.step != 0.0 { self.step } else { read(self.mip, 0.0) - read(self.mip, 1.0) };
            let x = (1.0 - t) as f32;
            s -= 0.5 * h * x * x;
            self.step = 0.0;
        }
        // `d < 1` implies `1 - phase < dphase`, so the division only runs next to a wrap.
        if 1.0 - self.phase < dphase {
            let d = (1.0 - self.phase) / dphase;
            if d < 1.0 {
                // The next sample wraps: the step is the table at 0 against the table at rho cycles.
                let h = read(self.mip, 0.0) - read(self.mip, 1.0);
                let x = (1.0 - d) as f32;
                s += 0.5 * h * x * x;
                self.step = h;
            }
        }
        s * self.level
    }
}
