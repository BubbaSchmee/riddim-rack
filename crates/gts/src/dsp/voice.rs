//! Voice management: polyphony, mono/legato, portamento, oldest-voice stealing, per-voice oscillators and amp envelopes.
//!
//! This file holds one voice; allocation (mono note stack, poly stealing) lives in `Engine`.
//! Per-sample stages of `Voice::next`, in order:
//!   1. glide (portamento in semitones),
//!   2. modulators: Mod A / Mod B LFOs -> `Voice::modulation` (Target, Mod B -> Mod A depth,
//!      wheel / velocity Mod amounts),
//!   3. per osc: pitch, Shift, Index + Motion, grain oscillator, amp envelope, Vol,
//!   4. shared filter envelope, cutoffs (every `FILTER_TICK` samples), Shaper -> Filter A / B per
//!      `Routing`, then Spread.

use super::env::Adsr;
use super::filter::MultiFilter;
use super::grain::GrainOsc;
use super::lfo::Lfo;
use super::shaper::Shaper;
use super::tables::{Tables, GRAINS};
use super::{Patch, Transport};
use crate::params::{Routing, Target};

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
    /// Mod A, Mod B.
    lfo: [Lfo; 2],
    /// Filter A, Filter B.
    filt: [MultiFilter; 2],
    /// Snapshot of `filt` taken at a Type / Routing change, faded out over `XFADE` with the old
    /// routing (`old_routing`) while `filt` fades in. `xfade` runs 1 -> 0.
    old: [MultiFilter; 2],
    routing: Option<Routing>,
    old_routing: Option<Routing>,
    xfade: f32,
    shaper: Shaper,
    /// Shaper wet amount, slewed toward Shaper On (0/1) so toggling does not click.
    shaper_mix: f32,
    /// Osc A / B On, latched at `update`.
    on: [bool; 2],
    fenv: Adsr,
    /// Samples until the next cutoff update.
    tick: u32,
    /// Set on a fresh note-on: grains start on the first `next`, once modulation is known.
    fresh: bool,
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

/// Bipolar velocity amount -> gain: +amt fades low velocities down, -amt fades high ones down.
#[inline]
fn vel_scale(amt: f32, vel: f32) -> f32 {
    if amt >= 0.0 { 1.0 - amt * (1.0 - vel) } else { 1.0 + amt * vel }
}

#[inline]
fn hits(t: Target, i: usize) -> bool {
    matches!((t, i), (Target::Both, _) | (Target::A, 0) | (Target::B, 1))
}

impl Voice {
    pub fn is_active(&self) -> bool {
        // Only enabled oscillators keep the voice alive.
        (self.on[0] && self.osc[0].env.is_active()) || (self.on[1] && self.osc[1].env.is_active())
    }

    pub fn amp_env(&self, osc: usize) -> &Adsr {
        &self.osc[osc].env
    }

    pub fn filter_env(&self) -> &Adsr {
        &self.fenv
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

    /// Not real-time: resets the filters.
    pub fn set_sample_rate(&mut self, sr: f32) {
        for l in &mut self.lfo {
            l.set_sample_rate(sr as f64);
        }
        for f in self.filt.iter_mut().chain(&mut self.old) {
            f.set_sample_rate(sr as f64);
        }
        self.xfade = 0.0;
    }

    /// Envelope times for this voice's velocity, shaper settings. Cheap; called per block and on trigger.
    pub fn update(&mut self, patch: &Patch, sr: f32) {
        // Velocity -> Attack: +1 makes a full-velocity attack 16x longer, -1 16x shorter.
        let att_scale = 2f32.powf(4.0 * patch.vel_att * self.velocity);
        for (o, op) in self.osc.iter_mut().zip(&patch.osc) {
            o.env.set((op.att * att_scale).clamp(0.001, 10.0), op.dec, op.sus, op.rel, sr);
        }
        self.fenv.set(patch.fenv_att, patch.fenv_dec, patch.fenv_sus, patch.fenv_rel, sr);
        self.shaper.set(patch.shaper_mode, patch.shaper_amt);
        self.on = [patch.osc[0].on, patch.osc[1].on];
    }

    /// LFO rate / sync / curve, per block. Synced LFOs lock to the host beat only while it plays.
    pub fn configure_mods(&mut self, patch: &Patch, transport: &Transport) {
        let beat = transport.playing.then_some(transport.pos_beats);
        for (l, m) in self.lfo.iter_mut().zip([&patch.moda, &patch.modb]) {
            let sync = m.sync.then_some((m.div, transport.tempo, beat));
            l.configure(m.curve, m.rate, sync, m.oneshot);
        }
    }

    /// Note-on for this voice. `glide` keeps the current pitch and slides to the note;
    /// `retrigger` restarts envelopes, LFOs and Motion scan (false = legato move).
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
            let o = &mut self.osc[i];
            o.vel_gain = vel_scale(vel_amts[i], velocity);
            o.scan = 0.0;
            o.env.gate_on();
        }
        for l in &mut self.lfo {
            l.trigger();
        }
        self.fenv.gate_on();
        self.tick = 0;
        if !was_active {
            // Fresh voice: no ringing from the last note (fills the comb lines, no allocation).
            for (f, fp) in self.filt.iter_mut().zip(&patch.filt) {
                f.reset();
                f.set_kind(fp.ty);
            }
            self.routing = Some(patch.routing);
            self.xfade = 0.0;
            self.shaper_mix = if patch.shaper_on { 1.0 } else { 0.0 };
            self.fresh = true;
        }
    }

    pub fn release(&mut self) {
        self.gate = false;
        for o in &mut self.osc {
            o.env.gate_off();
        }
        self.fenv.gate_off();
    }

    pub fn kill(&mut self) {
        self.gate = false;
        for o in &mut self.osc {
            o.env.kill();
        }
        self.fenv.kill();
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

    /// Mod A / Mod B -> `self.modulation`.
    fn modulate(&mut self, patch: &Patch, ctl: &Controls) {
        let (a, b) = (self.lfo[0].next(), self.lfo[1].next());
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
        for j in 0..2 {
            let fp = &patch.filt[j];
            let mut oct = fp.kbd * (self.pitch - KBD_REF) / 12.0 + self.modulation.cutoff[j] + wheel_oct;
            if fp.env {
                oct += env_oct;
            }
            let hz = (fp.freq * 2f32.powf(oct)).clamp(20.0, 0.45 * ctl.sr);
            self.filt[j].set(fp.ty, hz, fp.res);
        }
    }

    /// One sample, (left, right) after routing and Spread.
    #[inline]
    pub fn next(&mut self, tables: &Tables, patch: &Patch, ctl: &Controls) -> (f32, f32) {
        // 1. glide
        self.pitch = self.target + (self.pitch - self.target) * ctl.glide;

        // 2. modulators
        self.modulate(patch, ctl);
        let m = self.modulation;
        if self.fresh {
            // Start grains in phase with Shift settled and the first grain at the modulated
            // Index (a one-shot into Index opens on its peak, not one period late).
            self.fresh = false;
            for i in 0..2 {
                let shift = self.shift_target(i, patch, ctl);
                let gp = self.grain_pos(i, patch, ctl, 0.0);
                let o = &mut self.osc[i];
                o.shift = shift;
                o.grain.reset(2f64.powf(shift as f64 / 12.0), gp);
            }
        }

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

        // 4. filter env, shaper, filters, routing
        let fenv = self.fenv.next();
        // A Type or Routing change crossfades from a snapshot of the old path (reset the changed
        // filter, ~5 ms fade) instead of switching with stale state.
        let ty_changed = (0..2).any(|j| self.filt[j].kind() != patch.filt[j].ty);
        if ty_changed || self.routing != Some(patch.routing) {
            for j in 0..2 {
                self.old[j].copy_from(&self.filt[j]);
                if self.filt[j].kind() != patch.filt[j].ty {
                    self.filt[j].reset();
                    self.filt[j].set_kind(patch.filt[j].ty);
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
        let (mix, sh) = (self.shaper_mix, &mut self.shaper);
        let (l, r) = route(patch.routing, &mut self.filt, sh, mix, spread, a, b);
        if self.xfade <= 0.0 {
            return (l, r);
        }
        let (ol, or) = route(self.old_routing.unwrap_or(patch.routing), &mut self.old, sh, mix, spread, a, b);
        let g = self.xfade;
        self.xfade -= 1.0 / (XFADE * ctl.sr);
        (l + (ol - l) * g, r + (or - r) * g)
    }
}

/// Shaper -> Filter A / B per `routing`, then Spread. `mix` crossfades dry -> shaped.
#[inline]
fn route(routing: Routing, f: &mut [MultiFilter; 2], s: &mut Shaper, mix: f32, spread: f32, a: f32, b: f32) -> (f32, f32) {
    let mut shape = |x: f32| if mix > 0.0 { x + (s.process(x) - x) * mix } else { x };
    match routing {
        Routing::Parallel => {
            // Shaper on A only, as in the original lineage. Spread pans A left, B right.
            let fa = f[0].process(shape(a));
            let fb = f[1].process(b);
            (fa + fb * (1.0 - spread), fa * (1.0 - spread) + fb)
        }
        Routing::Series => {
            let y = f[0].process(shape(a + b));
            let y = f[1].process(y);
            (y, y)
        }
        Routing::Bypass => {
            let y = shape(a + b);
            (y, y)
        }
    }
}
