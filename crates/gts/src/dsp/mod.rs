//! GTS DSP: the voice engine (allocation, MIDI, output stage) over graintables and grain oscillators.
//! Everything `process` touches is preallocated here for 8 voices.

pub mod env;
pub mod filter;
pub mod grain;
pub mod lfo;
pub mod shaper;
pub mod tables;
pub mod voice;

use crate::params::{GtsParams, OscParams, Table};
use nih_plug::prelude::NoteEvent;
use std::time::{Duration, Instant};
use tables::Tables;
use voice::{Controls, Voice};

pub const MAX_VOICES: usize = 8;
/// Mono note stack depth.
const MAX_HELD: usize = 16;

/// One oscillator's parameter values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OscPatch {
    pub on: bool,
    pub table: Table,
    pub oct: i32,
    pub semi: i32,
    pub cent: f32,
    pub index: f32,
    pub motion: f32,
    pub shift: f32,
    pub vol: f32,
    /// Seconds.
    pub att: f32,
    pub dec: f32,
    pub sus: f32,
    pub rel: f32,
}

impl OscPatch {
    fn from_params(p: &OscParams) -> Self {
        Self {
            on: p.on.value(),
            table: p.table.value(),
            oct: p.oct.value(),
            semi: p.semi.value(),
            cent: p.cent.value(),
            index: p.index.value(),
            motion: p.motion.value(),
            shift: p.shift.value(),
            vol: p.vol.value(),
            att: p.att.value(),
            dec: p.dec.value(),
            sus: p.sus.value(),
            rel: p.rel.value(),
        }
    }
}

/// Plain-value snapshot of the parameters, taken once per block (with sample-accurate automation
/// the host splits blocks at parameter changes). Tests build one with `Patch::default()`.
/// TODO(integration): add the Mod A/B, Shaper, Filter, Filter Env and Routing values here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Patch {
    pub osc: [OscPatch; 2],
    pub poly: usize,
    pub legato: bool,
    /// Seconds.
    pub porta: f32,
    /// Semitones.
    pub bend_range: f32,
    pub wheel_index: f32,
    pub wheel_shift: f32,
    /// Not applied yet; for the filter stage.
    pub wheel_cutoff: f32,
    /// Not applied yet; for the LFO stage.
    pub wheel_mod: f32,
    pub vel_vol_a: f32,
    pub vel_vol_b: f32,
    /// Not applied yet; for the filter envelope.
    pub vel_fenv: f32,
    pub vel_att: f32,
    pub vel_shift: f32,
    /// Not applied yet; for the LFO stage.
    pub vel_mod: f32,
    pub volume_db: f32,
    pub spread: f32,
}

impl Patch {
    pub fn from_params(p: &GtsParams) -> Self {
        Self {
            osc: [OscPatch::from_params(&p.osca), OscPatch::from_params(&p.oscb)],
            poly: p.poly.value().clamp(1, MAX_VOICES as i32) as usize,
            legato: p.legato.value(),
            porta: p.porta.value(),
            bend_range: p.bend_range.value() as f32,
            wheel_index: p.wheel.index.value(),
            wheel_shift: p.wheel.shift.value(),
            wheel_cutoff: p.wheel.cutoff.value(),
            wheel_mod: p.wheel.mod_amt.value(),
            vel_vol_a: p.vel.vol_a.value(),
            vel_vol_b: p.vel.vol_b.value(),
            vel_fenv: p.vel.fenv.value(),
            vel_att: p.vel.att.value(),
            vel_shift: p.vel.shift.value(),
            vel_mod: p.vel.mod_amt.value(),
            volume_db: p.volume.value(),
            spread: p.spread.value(),
        }
    }
}

impl Default for Patch {
    fn default() -> Self {
        Self::from_params(&GtsParams::default())
    }
}

/// Host transport at the start of the current block. TODO(integration): LFO sync advances
/// `pos_beats` per sample from `tempo` inside the block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transport {
    pub tempo: f64,
    pub playing: bool,
    pub pos_beats: f64,
}

impl Default for Transport {
    fn default() -> Self {
        Self { tempo: 120.0, playing: false, pos_beats: 0.0 }
    }
}

/// Safety limiter: identity up to 0.95, tanh knee above, never exceeds 1.0.
#[inline]
pub fn limit(x: f32) -> f32 {
    let a = x.abs();
    if a <= 0.95 {
        x
    } else {
        (0.95 + 0.05 * ((a - 0.95) / 0.05).tanh()).copysign(x)
    }
}

pub struct Engine {
    tables: Tables,
    voices: [Voice; MAX_VOICES],
    patch: Patch,
    ctl: Controls,
    /// Mono note stack in press order, last = sounding.
    held: [(u8, f32); MAX_HELD],
    n_held: usize,
    age: u64,
    last_voice: usize,
    /// Raw controller values: bend -1..1, wheel 0..1.
    bend_raw: f32,
    wheel_raw: f32,
    gain: f32,
    gain_target: f32,
    pub transport: Transport,
    pub log_notes: bool,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    /// Allocates everything (tables are filled by `initialize`).
    pub fn new() -> Self {
        let mut e = Self {
            tables: Tables::new(),
            voices: [Voice::default(); MAX_VOICES],
            patch: Patch::default(),
            ctl: Controls { sr: 44100.0, bend: 0.0, wheel: 0.0, glide: 0.0, smooth: 0.0 },
            held: [(0, 0.0); MAX_HELD],
            n_held: 0,
            age: 0,
            last_voice: 0,
            bend_raw: 0.0,
            wheel_raw: 0.0,
            gain: 0.0,
            gain_target: 0.0,
            transport: Transport::default(),
            log_notes: false,
        };
        e.set_sample_rate(44100.0);
        e
    }

    /// Sets the sample rate and generates the tables the first time. Returns the generation time
    /// when it ran.
    pub fn initialize(&mut self, sr: f32) -> Option<Duration> {
        self.set_sample_rate(sr);
        if self.tables.is_ready() {
            return None;
        }
        let t = Instant::now();
        self.tables.generate();
        Some(t.elapsed())
    }

    fn set_sample_rate(&mut self, sr: f32) {
        self.ctl.sr = sr;
        self.ctl.smooth = (-1.0 / (0.005 * sr)).exp();
        let p = self.patch;
        self.set_patch(&p);
        self.gain = self.gain_target;
    }

    pub fn tables(&self) -> &Tables {
        &self.tables
    }

    pub fn reset(&mut self) {
        for v in &mut self.voices {
            v.kill();
        }
        self.n_held = 0;
        self.bend_raw = 0.0;
        self.ctl.bend = 0.0;
        self.gain = self.gain_target;
    }

    /// Call once per block, before rendering it.
    pub fn set_patch(&mut self, p: &Patch) {
        if p.poly != self.patch.poly {
            for v in &mut self.voices {
                v.release();
            }
            self.n_held = 0;
        }
        self.patch = *p;
        let sr = self.ctl.sr;
        // Portamento: one-pole in semitones, ~98 % of the way after `porta` seconds.
        self.ctl.glide = if p.porta > 0.0 { (-4.0 / (p.porta * sr)).exp() } else { 0.0 };
        self.gain_target = 10f32.powf(p.volume_db / 20.0);
        for v in &mut self.voices {
            v.update(p, sr);
        }
    }

    pub fn handle_event(&mut self, event: &NoteEvent<()>) {
        match *event {
            NoteEvent::NoteOn { note, velocity, .. } => self.note_on(note, velocity),
            NoteEvent::NoteOff { note, .. } => self.note_off(note),
            NoteEvent::Choke { note, .. } => self.choke(note),
            NoteEvent::MidiPitchBend { value, .. } => self.pitch_bend(value),
            NoteEvent::MidiCC { cc: 1, value, .. } => self.mod_wheel(value),
            _ => {}
        }
    }

    /// Pitch bend as delivered by nih-plug: 0..1, 0.5 = centre.
    pub fn pitch_bend(&mut self, value: f32) {
        self.bend_raw = (value * 2.0 - 1.0).clamp(-1.0, 1.0);
    }

    /// Mod wheel (CC1), 0..1.
    pub fn mod_wheel(&mut self, value: f32) {
        self.wheel_raw = value.clamp(0.0, 1.0);
    }

    /// Smoothed mod wheel, 0..1 (what the voices use; the filter/LFO stages read it too).
    pub fn wheel(&self) -> f32 {
        self.ctl.wheel
    }

    pub fn voice(&self, i: usize) -> &Voice {
        &self.voices[i]
    }

    /// Mutable voice access for the modulation stage (writes `Voice::modulation`).
    pub fn voice_mut(&mut self, i: usize) -> &mut Voice {
        &mut self.voices[i]
    }

    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.is_active()).count()
    }

    /// Osc A grain index (0..127) of the most recently triggered voice, for the GUI.
    pub fn display_index(&self) -> u32 {
        self.voices[self.last_voice].index(0).round() as u32
    }

    fn glide_allowed(&self, v: &Voice) -> bool {
        // Fingered portamento with legato on (glide only between overlapping notes), always
        // glide from a sounding voice with legato off.
        v.is_active() && (!self.patch.legato || v.gate)
    }

    pub fn note_on(&mut self, note: u8, velocity: f32) {
        self.age += 1;
        let (patch, ctl, age) = (self.patch, self.ctl, self.age);
        if patch.poly <= 1 {
            self.remove_held(note);
            if self.n_held == MAX_HELD {
                self.held.copy_within(1.., 0);
                self.n_held -= 1;
            }
            self.held[self.n_held] = (note, velocity);
            self.n_held += 1;
            let glide = self.glide_allowed(&self.voices[0]);
            let v = &mut self.voices[0];
            // Legato: a new key while one is held moves the pitch without retriggering.
            let retrigger = !(patch.legato && v.gate);
            v.trigger(note, velocity, glide, retrigger, age, &patch, &ctl);
            self.last_voice = 0;
            return;
        }
        let n = patch.poly.min(MAX_VOICES);
        let vs = &self.voices[..n];
        let oldest = |pred: &dyn Fn(&Voice) -> bool| {
            vs.iter().enumerate().filter(|(_, v)| pred(v)).min_by_key(|(_, v)| v.age).map(|(i, _)| i)
        };
        let i = oldest(&|v| v.gate && v.note == note)
            .or_else(|| oldest(&|v| !v.is_active()))
            .or_else(|| oldest(&|v| !v.gate))
            .or_else(|| oldest(&|_| true))
            .unwrap_or(0);
        let glide = self.glide_allowed(&self.voices[i]);
        self.voices[i].trigger(note, velocity, glide, true, age, &patch, &ctl);
        self.last_voice = i;
    }

    pub fn note_off(&mut self, note: u8) {
        let (patch, ctl) = (self.patch, self.ctl);
        if patch.poly <= 1 {
            if !self.remove_held(note) {
                return;
            }
            let v = &mut self.voices[0];
            if !(v.gate && v.note == note) {
                return;
            }
            if self.n_held == 0 {
                v.release();
            } else {
                // Fall back to the most recent still-held key.
                let (n, vel) = self.held[self.n_held - 1];
                self.age += 1;
                v.trigger(n, vel, true, !patch.legato, self.age, &patch, &ctl);
            }
            return;
        }
        for v in &mut self.voices {
            if v.gate && v.note == note {
                v.release();
            }
        }
    }

    pub fn choke(&mut self, note: u8) {
        self.remove_held(note);
        for v in &mut self.voices {
            if v.note == note {
                v.kill();
            }
        }
    }

    fn remove_held(&mut self, note: u8) -> bool {
        match self.held[..self.n_held].iter().position(|h| h.0 == note) {
            Some(i) => {
                self.held.copy_within(i + 1..self.n_held, i);
                self.n_held -= 1;
                true
            }
            None => false,
        }
    }

    /// One stereo frame.
    #[inline]
    pub fn next_frame(&mut self) -> (f32, f32) {
        let k = self.ctl.smooth;
        let bend = self.bend_raw * self.patch.bend_range;
        self.ctl.bend = bend + (self.ctl.bend - bend) * k;
        self.ctl.wheel = self.wheel_raw + (self.ctl.wheel - self.wheel_raw) * k;
        self.gain = self.gain_target + (self.gain - self.gain_target) * k;

        // TODO(integration): per-voice LFO stage here, writing `Voice::modulation` (it can read
        // `self.transport`, `self.ctl.wheel`, each voice's velocity and gate).
        let spread = self.patch.spread.clamp(0.0, 1.0);
        let (mut l, mut r) = (0.0, 0.0);
        for v in &mut self.voices {
            if !v.is_active() {
                continue;
            }
            // Spread pans A left and B right; 0 = both centred.
            let (a, b) = v.next(&self.tables, &self.patch, &self.ctl);
            l += a + b * (1.0 - spread);
            r += a * (1.0 - spread) + b;
        }
        (limit(l * self.gain), limit(r * self.gain))
    }
}
