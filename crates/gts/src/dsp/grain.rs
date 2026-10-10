//! Pitch-synchronous grain playback: two overlapping Hann-windowed readers per oscillator; Index, Motion, Shift.
//!
//! A master phase runs at f0. Each wrap starts a new grain on the idle reader; every grain lasts
//! two periods under a Hann window, so the two readers always overlap 50 % and their windows sum
//! to exactly 1. Each reader moves through the table at `ratio * N` samples per period
//! (`ratio = 2^(shift/12)`), so Shift moves the spectrum while the grain rate (the pitch) stays f0.
//! For Shift > 0 the window follows the table instead (two table cycles, PSOLA-style), so grains
//! shorten to 2/ratio periods and the grain rate stays audible as the fundamental.
//! ponytail: for Shift < 0 PSOLA would stretch grains past two periods and need more readers;
//! the window is capped at two periods (two readers), which keeps pitch exact.
//! At ratio 1 both readers sit on the same table position and the output is plain single-cycle
//! playback, periodic at f0. The grain position (Index) is latched per grain, so any jump in Index
//! (scan wrap, scan reset, stepped modulation) is crossfaded over one period by the windows.

use super::tables::{Tables, N};

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
}

impl GrainOsc {
    /// Restart at phase 0 (fresh note on an idle voice).
    pub fn reset(&mut self, ratio: f64, gp: f32) {
        self.phase = 0.0;
        self.newest = 0;
        // Newest reader at age 0, the other at age 1 (grain centre, table position 0).
        self.pos = [(-(N as f64) * ratio).rem_euclid(N as f64), 0.0];
        self.gp = [gp; 2];
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
            // The new grain began `phase` periods ago. Grains are centred on table position 0
            // (age 1 = window peak), where the tables put their main event (pulse edge, saw
            // jump), so a downward Shift stretches that event instead of windowing it out.
            self.pos[self.newest] = ((self.phase - 1.0) * n * ratio).rem_euclid(n);
            self.gp[self.newest] = gp;
        }
        let (a, b) = (self.newest, self.newest ^ 1);
        if ratio <= 1.0 {
            // Output-frame Hann over two periods: the pair always sums to exactly 1.
            let w = (0.5 - 0.5 * (std::f64::consts::PI * self.phase).cos()) as f32;
            w * tables.read(table, self.gp[a], self.pos[a]) + (1.0 - w) * tables.read(table, self.gp[b], self.pos[b])
        } else {
            // Source-frame Hann (PSOLA): a grain spans two table cycles, so it lasts 2/ratio
            // periods. Without this, integer ratios would put both readers on the same table
            // position and the output would simply be the table at ratio * f0 (+12 = octave up).
            let len = 2.0 / ratio;
            let win = |age: f64| if age < len { 0.5 - 0.5 * (std::f64::consts::TAU * age / len).cos() } else { 0.0 };
            let (wa, wb) = (win(self.phase) as f32, win(self.phase + 1.0) as f32);
            // sqrt(ratio) keeps loudness roughly level as the windows thin out.
            let s = wa * tables.read(table, self.gp[a], self.pos[a]) + wb * tables.read(table, self.gp[b], self.pos[b]);
            s * (ratio as f32).sqrt()
        }
    }
}
