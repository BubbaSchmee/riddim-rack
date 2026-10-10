//! GTS DSP. The engine is a stub (sine per held note, mono, last-note priority) until the real
//! voice/grain modules below are filled in.

pub mod env;
pub mod filter;
pub mod grain;
pub mod lfo;
pub mod shaper;
pub mod tables;
pub mod voice;

const MAX_HELD: usize = 16;

pub struct Engine {
    sample_rate: f64,
    phase: f64,
    /// Held notes in press order, last = active. Fixed size so `process` never allocates.
    held: [(u8, f32); MAX_HELD],
    n_held: usize,
    pub log_notes: bool,
}

impl Engine {
    pub fn new() -> Self {
        Self { sample_rate: 44100.0, phase: 0.0, held: [(0, 0.0); MAX_HELD], n_held: 0, log_notes: false }
    }

    pub fn set_sample_rate(&mut self, sr: f64) {
        self.sample_rate = sr;
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.n_held = 0;
    }

    pub fn note_on(&mut self, note: u8, velocity: f32) {
        self.note_off(note);
        if self.n_held == MAX_HELD {
            self.held.copy_within(1.., 0);
            self.n_held -= 1;
        }
        self.held[self.n_held] = (note, velocity);
        self.n_held += 1;
    }

    pub fn note_off(&mut self, note: u8) {
        if let Some(i) = self.held[..self.n_held].iter().position(|h| h.0 == note) {
            self.held.copy_within(i + 1..self.n_held, i);
            self.n_held -= 1;
        }
    }

    pub fn held_count(&self) -> usize {
        self.n_held
    }

    /// One mono sample (stub: plain sine of the most recent held note, hard gate).
    pub fn next_sample(&mut self) -> f32 {
        if self.n_held == 0 {
            return 0.0;
        }
        let (note, vel) = self.held[self.n_held - 1];
        let freq = 440.0 * 2f64.powf((note as f64 - 69.0) / 12.0);
        self.phase = (self.phase + freq / self.sample_rate).fract();
        (self.phase * std::f64::consts::TAU).sin() as f32 * 0.5 * vel
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}
