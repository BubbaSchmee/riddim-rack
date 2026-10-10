//! Voice management: polyphony, mono/legato, portamento, oldest-voice stealing, per-voice oscillators and amp envelopes.
//!
//! This file holds one voice; allocation (mono note stack, poly stealing) lives in `Engine`.
//! Per-sample stages of `Voice::next`, in order:
//!   1. glide (portamento in semitones),
//!   2. modulation inputs (`Voice::modulation`, written by the LFO stage before `next`),
//!   3. per osc: pitch, Shift, Index + Motion, grain oscillator, amp envelope, Vol,
//!   4. shaper / filters / routing (TODO(integration), pass-through today).

use super::env::Adsr;
use super::grain::GrainOsc;
use super::tables::{Tables, GRAINS};
use super::Patch;

/// Per-sample modulation inputs for this voice, one slot per oscillator (0 = A, 1 = B).
/// Zero means unmodulated. TODO(integration): the Mod A / Mod B LFO stage fills these (after
/// applying Target, Mod B -> Mod A scaling, wheel/velocity Mod amounts) before `Voice::next`.
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
}

#[derive(Debug, Clone, Copy, Default)]
struct Osc {
    grain: GrainOsc,
    env: Adsr,
    /// Smoothed Shift in semitones.
    shift: f32,
    /// Motion scan position in grains.
    scan: f64,
    vel_gain: f32,
    /// Last f0 in Hz (for tests / inspection).
    freq: f32,
}

#[derive(Debug, Clone, Copy, Default)]
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
}

/// Ping-pong period of the scan (0 -> 31 -> 0) and a common multiple with the 32-grain wrap.
const PINGPONG: f64 = 2.0 * (GRAINS - 1) as f64;
const SCAN_WRAP: f64 = 992.0;

impl Voice {
    pub fn is_active(&self) -> bool {
        self.osc[0].env.is_active() || self.osc[1].env.is_active()
    }

    pub fn amp_env(&self, osc: usize) -> &Adsr {
        &self.osc[osc].env
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
        (self.osc[osc].grain.grain_pos() / (GRAINS - 1) as f32 * 127.0).min(127.0)
    }

    /// Envelope times for this voice's velocity. Cheap; called per block and on trigger.
    pub fn update(&mut self, patch: &Patch, sr: f32) {
        // Velocity -> Attack: +1 makes a full-velocity attack 16x longer, -1 16x shorter.
        let att_scale = 2f32.powf(4.0 * patch.vel_att * self.velocity);
        for (o, op) in self.osc.iter_mut().zip(&patch.osc) {
            o.env.set((op.att * att_scale).clamp(0.001, 10.0), op.dec, op.sus, op.rel, sr);
        }
    }

    /// Note-on for this voice. `glide` keeps the current pitch and slides to the note;
    /// `retrigger` restarts envelopes and Motion scan (false = legato move).
    pub fn trigger(&mut self, note: u8, velocity: f32, glide: bool, retrigger: bool, age: u64, patch: &Patch, ctl: &Controls) {
        let was_active = self.is_active();
        self.note = note;
        self.gate = true;
        self.target = note as f32;
        if !glide || !was_active {
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
            let amt = vel_amts[i];
            let o = &mut self.osc[i];
            o.vel_gain = if amt >= 0.0 { 1.0 - amt * (1.0 - velocity) } else { 1.0 + amt * velocity };
            o.scan = 0.0;
            o.env.gate_on();
        }
        if !was_active {
            // Fresh voice: start grains in phase, Shift settled, nothing gliding in from old state.
            for i in 0..2 {
                let shift = self.shift_target(i, patch, ctl);
                let gp = self.grain_pos(i, patch, ctl, 0.0);
                let o = &mut self.osc[i];
                o.shift = shift;
                o.grain.reset(2f64.powf(shift as f64 / 12.0), gp);
            }
        }
    }

    pub fn release(&mut self) {
        self.gate = false;
        for o in &mut self.osc {
            o.env.gate_off();
        }
    }

    pub fn kill(&mut self) {
        self.gate = false;
        for o in &mut self.osc {
            o.env.kill();
        }
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

    /// One sample: (osc A, osc B) after envelope and Vol, before panning.
    #[inline]
    pub fn next(&mut self, tables: &Tables, patch: &Patch, ctl: &Controls) -> (f32, f32) {
        // 1. glide
        self.pitch = self.target + (self.pitch - self.target) * ctl.glide;

        // 2. modulation: `self.modulation` was written by the LFO stage (TODO(integration)).
        let m = self.modulation;

        // 3. oscillators
        let mut out = [0.0f32; 2];
        for i in 0..2 {
            let env = self.osc[i].env.next();
            let op = &patch.osc[i];
            let motion = (op.motion + m.motion[i]).abs() as f64;
            let shift = self.shift_target(i, patch, ctl);
            let scan = self.osc[i].scan;
            let gp = self.grain_pos(i, patch, ctl, scan);
            let o = &mut self.osc[i];
            // Motion: 1.0 = whole table (32 grains) per 0.25 s.
            o.scan += motion * 128.0 / ctl.sr as f64;
            if o.scan >= SCAN_WRAP {
                o.scan -= SCAN_WRAP;
            }
            o.shift = shift + (o.shift - shift) * ctl.smooth;
            let pitch = self.pitch + 12.0 * op.oct as f32 + op.semi as f32 + op.cent / 100.0 + ctl.bend + m.pitch[i];
            o.freq = (440.0 * 2f32.powf((pitch - 69.0) / 12.0)).clamp(1.0, 0.45 * ctl.sr);
            if !op.on || env <= 0.0 {
                continue;
            }
            let ratio = 2f64.powf(o.shift as f64 / 12.0);
            let s = o.grain.next(tables, op.table as usize, o.freq as f64 / ctl.sr as f64, ratio, gp);
            out[i] = s * env * op.vol * o.vel_gain * (1.0 + m.vol[i]).max(0.0);
        }

        // 4. TODO(integration): Shaper -> Filter A / Filter B per `Routing` (shared filter env,
        //    Kbd from `self.pitch`, vel_fenv / wheel_cutoff / Mod B cutoff from `patch`).
        //    Return (A path, B path); Series/Bypass can return (mix, 0.0). Pass-through for now.
        (out[0], out[1])
    }
}
