//! GTS DSP: the voice engine (allocation, MIDI, output stage) over graintables and grain oscillators.
//! Everything `process` touches is preallocated here for 8 voices.

pub mod env;
pub mod filter;
pub mod grain;
pub mod lfo;
pub mod shaper;
pub mod tables;
pub mod voice;

use crate::params::{
    Curve, Division, FilterParams, FilterType, GtsParams, ModParams, OscParams, Routing, ShaperMode, Table, Target,
};
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

/// One LFO's settings (Mod A or Mod B).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModPatch {
    pub curve: Curve,
    /// Hz, used when `sync` is off.
    pub rate: f32,
    pub sync: bool,
    pub div: Division,
    pub oneshot: bool,
    pub target: Target,
}

impl ModPatch {
    fn from_params(p: &ModParams) -> Self {
        Self {
            curve: p.curve.value(),
            rate: p.rate.value(),
            sync: p.sync.value(),
            div: p.div.value(),
            oneshot: p.oneshot.value(),
            target: p.target.value(),
        }
    }
}

/// Filter A or B settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilterPatch {
    pub ty: FilterType,
    /// Hz.
    pub freq: f32,
    pub res: f32,
    pub kbd: f32,
    pub env: bool,
}

impl FilterPatch {
    fn from_params(p: &FilterParams) -> Self {
        Self { ty: p.ty.value(), freq: p.freq.value(), res: p.res.value(), kbd: p.kbd.value(), env: p.env.value() }
    }
}

/// Plain-value snapshot of the parameters, taken once per block (with sample-accurate automation
/// the host splits blocks at parameter changes). Tests build one with `Patch::default()`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Patch {
    pub osc: [OscPatch; 2],
    /// Mod A, Mod B.
    pub moda: ModPatch,
    pub modb: ModPatch,
    /// Mod A amounts, bipolar: x24 st, x127, x24 st.
    pub moda_pitch: f32,
    pub moda_index: f32,
    pub moda_shift: f32,
    /// Mod B amounts, bipolar: x1, x1, x5 octaves, x1 (scales Mod A depth).
    pub modb_motion: f32,
    pub modb_vol: f32,
    pub modb_cutoff: f32,
    pub modb_moda: f32,
    pub shaper_on: bool,
    pub shaper_mode: ShaperMode,
    pub shaper_amt: f32,
    /// Filter A, Filter B.
    pub filt: [FilterPatch; 2],
    /// Seconds.
    pub fenv_att: f32,
    pub fenv_dec: f32,
    pub fenv_sus: f32,
    pub fenv_rel: f32,
    /// 0..1 = up to +8 octaves.
    pub fenv_amt: f32,
    pub fenv_inv: bool,
    pub routing: Routing,
    pub poly: usize,
    pub legato: bool,
    /// Seconds.
    pub porta: f32,
    /// Semitones.
    pub bend_range: f32,
    pub wheel_index: f32,
    pub wheel_shift: f32,
    /// x5 octaves on both filters.
    pub wheel_cutoff: f32,
    /// Scales Mod A depth by `1 + wheel_mod * wheel`.
    pub wheel_mod: f32,
    pub vel_vol_a: f32,
    pub vel_vol_b: f32,
    pub vel_fenv: f32,
    pub vel_att: f32,
    pub vel_shift: f32,
    pub vel_mod: f32,
    pub volume_db: f32,
    pub spread: f32,
}

impl Patch {
    pub fn from_params(p: &GtsParams) -> Self {
        Self {
            osc: [OscPatch::from_params(&p.osca), OscPatch::from_params(&p.oscb)],
            moda: ModPatch::from_params(&p.moda),
            modb: ModPatch::from_params(&p.modb),
            moda_pitch: p.moda_amt.pitch.value(),
            moda_index: p.moda_amt.index.value(),
            moda_shift: p.moda_amt.shift.value(),
            modb_motion: p.modb_amt.motion.value(),
            modb_vol: p.modb_amt.vol.value(),
            modb_cutoff: p.modb_amt.cutoff.value(),
            modb_moda: p.modb_amt.moda.value(),
            shaper_on: p.shaper.on.value(),
            shaper_mode: p.shaper.mode.value(),
            shaper_amt: p.shaper.amt.value(),
            filt: [FilterPatch::from_params(&p.filta), FilterPatch::from_params(&p.filtb)],
            fenv_att: p.fenv.att.value(),
            fenv_dec: p.fenv.dec.value(),
            fenv_sus: p.fenv.sus.value(),
            fenv_rel: p.fenv.rel.value(),
            fenv_amt: p.fenv.amt.value(),
            fenv_inv: p.fenv.inv.value(),
            routing: p.routing.value(),
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

/// Host transport. The plugin sets it at the start of each block; `Engine::next_frame` advances
/// `pos_beats` from `tempo` while playing, so offline renders only set it once.
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
    /// Sustain pedal (CC64) down, and the notes whose note-off it is holding back.
    sustain: bool,
    pending: [bool; 128],
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
            voices: std::array::from_fn(|_| Voice::default()),
            patch: Patch::default(),
            ctl: Controls { sr: 44100.0, bend: 0.0, wheel: 0.0, glide: 0.0, smooth: 0.0, dc: 0.0 },
            held: [(0, 0.0); MAX_HELD],
            n_held: 0,
            age: 0,
            last_voice: 0,
            bend_raw: 0.0,
            wheel_raw: 0.0,
            gain: 0.0,
            gain_target: 0.0,
            sustain: false,
            pending: [false; 128],
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
        self.ctl.dc = 1.0 - (-std::f32::consts::TAU * 8.0 / sr).exp();
        for v in &mut self.voices {
            v.set_sample_rate(sr);
        }
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
        self.sustain = false;
        self.pending = [false; 128];
        self.bend_raw = 0.0;
        self.ctl.bend = 0.0;
        self.gain = self.gain_target;
    }

    /// Call once per block, before rendering it.
    pub fn set_patch(&mut self, p: &Patch) {
        if p.poly != self.patch.poly {
            // Only voices the new count no longer reaches are released.
            for v in &mut self.voices[p.poly..] {
                v.release();
            }
            self.n_held = 0;
            // Into mono: voice 0's held key becomes the note stack, so its note-off still lands.
            let v = &self.voices[0];
            if p.poly <= 1 && v.gate {
                self.held[0] = (v.note, v.velocity);
                self.n_held = 1;
            }
        }
        self.patch = *p;
        let sr = self.ctl.sr;
        // Portamento: one-pole in semitones, ~98 % of the way after `porta` seconds.
        self.ctl.glide = if p.porta > 0.0 { (-4.0 / (p.porta * sr)).exp() } else { 0.0 };
        // 1/sqrt(poly) keeps chords out of the limiter (mono unchanged).
        self.gain_target = 10f32.powf(p.volume_db / 20.0) / (p.poly as f32).sqrt();
        for v in &mut self.voices {
            v.update(p, sr);
            v.configure_mods(p, &self.transport);
        }
    }

    pub fn handle_event(&mut self, event: &NoteEvent<()>) {
        match *event {
            NoteEvent::NoteOn { note, velocity, .. } => self.note_on(note, velocity),
            NoteEvent::NoteOff { note, .. } => self.note_off(note),
            NoteEvent::Choke { note, .. } => self.choke(note),
            NoteEvent::MidiPitchBend { value, .. } => self.pitch_bend(value),
            NoteEvent::MidiCC { cc: 1, value, .. } => self.mod_wheel(value),
            NoteEvent::MidiCC { cc: 64, value, .. } => self.sustain_pedal(value >= 0.5),
            NoteEvent::MidiCC { cc: 120 | 123, .. } => self.all_notes_off(),
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

    /// Sustain pedal: while down, note-offs are deferred; lifting it flushes them.
    pub fn sustain_pedal(&mut self, down: bool) {
        self.sustain = down;
        if !down {
            for n in 0..128u8 {
                if std::mem::take(&mut self.pending[n as usize]) {
                    self.note_off(n);
                }
            }
        }
    }

    /// CC120 / CC123: release every voice and forget held and sustained keys.
    pub fn all_notes_off(&mut self) {
        for v in &mut self.voices {
            v.release();
        }
        self.n_held = 0;
        self.pending = [false; 128];
    }

    /// Smoothed mod wheel, 0..1 (what the voices use; the filter/LFO stages read it too).
    pub fn wheel(&self) -> f32 {
        self.ctl.wheel
    }

    pub fn voice(&self, i: usize) -> &Voice {
        &self.voices[i]
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
        // glide from a sounding voice with legato off. Mono only: in poly a reused voice's pitch
        // belongs to an unrelated note.
        self.patch.poly <= 1 && v.is_active() && (!self.patch.legato || v.gate)
    }

    pub fn note_on(&mut self, note: u8, velocity: f32) {
        // A re-pressed key is held again; its deferred note-off no longer applies.
        self.pending[note as usize & 127] = false;
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
            v.configure_mods(&patch, &self.transport);
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
        // Lock synced LFOs to the grid now, not at the next block (that would step mid-note).
        self.voices[i].configure_mods(&patch, &self.transport);
        self.last_voice = i;
    }

    pub fn note_off(&mut self, note: u8) {
        if self.sustain {
            self.pending[note as usize & 127] = true;
            return;
        }
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
                v.configure_mods(&patch, &self.transport);
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

        if self.transport.playing {
            self.transport.pos_beats += self.transport.tempo / 60.0 / self.ctl.sr as f64;
        }
        let (mut l, mut r) = (0.0, 0.0);
        for v in &mut self.voices {
            if !v.is_active() {
                continue;
            }
            let (a, b) = v.next(&self.tables, &self.patch, &self.ctl);
            l += a;
            r += b;
        }
        (limit(l * self.gain), limit(r * self.gain))
    }
}
