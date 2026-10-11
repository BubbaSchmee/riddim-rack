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
    }


    /// Grain position of the newest grain (what the GUI shows).
    pub fn grain_pos(&self) -> f32 {
        self.gp[self.newest]
    }

    /// One sample. `dphase` = f0 / sr (must stay below 1), `ratio` = 2^(shift/12), `gp` = grain
    /// position for the next grain that starts.
    #[inline]
    pub fn next(&mut self, tables: &Tables, table: usize, dphase: f64, ratio: f64, gp: f32) -> f32 {
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
            let top = if (ratio - 1.0).abs() < 1e-9 { TOP } else { TOP.min(0.5 - 3.0 * dphase) };
            let m = mip_for(ratio * dphase, top) as u8;
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
}
