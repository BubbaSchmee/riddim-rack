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

/// Smooth monotonic phase warp (Bend+, Bend-, PWM): the Blaschke factor
/// `z -> (z - a) / (1 - conj(a) z)` on the unit circle, rotated so phase 0 stays at 0. It is
/// analytic, so a warped band-limited table stays band-limited around its peak instantaneous
/// frequency `slope()` x the plain one (no corners, nothing for the oversampling to miss).
/// Real `a` squeezes the table's start (a > 0, the grain's main event) or its middle (a < 0);
/// imaginary `a` squeezes the first half of the cycle into less time: pulse-width warp.
#[derive(Debug, Clone, Copy)]
pub struct PhaseMap {
    a: (f64, f64),
    /// `arg B(1)`, subtracted so the map keeps phase 0.
    off: f64,
}

impl PhaseMap {
    pub fn new(re: f64, im: f64) -> Self {
        Self { a: (re, im), off: 2.0 * (-im).atan2(1.0 - re) }
    }

    /// Largest phase slope, `(1 + |a|) / (1 - |a|)`.
    pub fn slope(&self) -> f64 {
        let r = self.a.0.hypot(self.a.1);
        (1.0 + r) / (1.0 - r)
    }

    /// Table position (frame samples, any real) -> warped position. `arg B(z) = 2 arg(z - a) - arg z`.
    /// The warp term is computed in f32 (1e-7 rad, far under the table's own error) on the
    /// position folded to one cycle; the cycle count and the identity part stay f64.
    #[inline]
    pub fn apply(&self, p: f64) -> f64 {
        let n = N as f64;
        let q = p.rem_euclid(n);
        let th = (std::f32::consts::TAU / N as f32) * q as f32;
        let (s, c) = th.sin_cos();
        let d = 2.0 * (s - self.a.1 as f32).atan2(c - self.a.0 as f32) - 2.0 * th - self.off as f32;
        // `d` is the map minus the identity, wrapped to (-pi, pi].
        let d = d - std::f32::consts::TAU * (d / std::f32::consts::TAU).round();
        p + d as f64 * (n / TAU)
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
    #[inline]
    pub fn next(&mut self, tables: &Tables, table: usize, dphase: f64, ratio: f64, gp: f32) -> f32 {
        self.next_with(tables, table, dphase, ratio, gp, 1.0, 1.0, |p| p)
    }

    /// [`GrainOsc::next`] on a grid `os` times the base rate (`dphase` per grid sample), the mip
    /// picked for `fmul` times the base-rate read rate, both reader positions through `warp`.
    /// `os = fmul = 1` with the identity is `next` exactly.
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
        let w = if same { 1.0 } else { (0.5 - 0.5 * (PI * self.phase).cos()) as f32 };
        let mix = |m: u8| {
            let sa = tables.read(table, m as usize, ga, pa);
            if same { sa } else { w * sa + (1.0 - w) * tables.read(table, m as usize, gb, pb) }
        };
        let mut s = mix(self.mip);
        if self.fade < 1.0 {
            self.fade = (self.fade + 1.0 / BLOCK as f32).min(1.0);
            s = s * self.fade + mix(self.prev) * (1.0 - self.fade);
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
        let read = |m: u8, ph: f64| tables.read(table, m as usize, g, top * ph);
        let mut s = read(self.mip, self.phase);
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
        let d = (1.0 - self.phase) / dphase;
        if d < 1.0 {
            // The next sample wraps: the step is the table at 0 against the table at rho cycles.
            let h = read(self.mip, 0.0) - read(self.mip, 1.0);
            let x = (1.0 - d) as f32;
            s += 0.5 * h * x * x;
            self.step = h;
        }
        s * self.level
    }
}
