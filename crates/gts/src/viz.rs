//! Audio -> GUI plumbing (docs/design/MOTION.md section 8): what the audio thread publishes so the
//! editor can draw live data without re-running any DSP.
//!
//! * the output scope ring: `rr_ui::ring`, mono sum after the limiter, pushed every 64 samples;
//! * the output side ring: (L - R) / 2 in the same chunks as the scope ring (mid = scope), for the
//!   Dimension display;
//! * one [`Snap`] record per block: globals (sample rate, live grain position and table per osc),
//!   `VoiceViz` (per voice: note, gate, velocity, age since note-on, amp env A / B level and stage,
//!   filter env level and stage, Mod A / B phase and output) and `ModLive` (post-modulation
//!   normalized value of every ring target in [`MOD_IDS`], from the newest sounding voice) and the
//!   FX display values ([`xf`], from `dsp::fx::FxTelemetry`).
//!
//! Everything here is allocation-free on the audio side ([`VizWriter`] owns fixed buffers) and
//! lock-free on both sides; the GUI copies a record with [`Viz::read`] and keeps the last good one.

use crate::dsp::env::Stage;
use crate::dsp::fx::FxTelemetry;
use crate::dsp::voice::vel_scale;
use crate::dsp::{Engine, Patch, MAX_VOICES};
use crate::params::GtsParams;
use nih_plug::prelude::Param;
use rr_ui::ring::{ring, RingReader, RingWriter};
use rr_ui::snap::Snap;
use std::sync::Arc;

/// Global fields of a record.
pub mod gf {
    /// Sample rate in Hz.
    pub const SR: usize = 0;
    /// Index of the newest sounding voice, -1 when none.
    pub const NEWEST: usize = 1;
    /// Live grain position of Osc A / Osc B in Index units (0..127): the DSP's own frame,
    /// Index + wheel + Mod A + Motion scan, or the knob (plus wheel) while nothing sounds.
    pub const INDEX_A: usize = 2;
    pub const INDEX_B: usize = 3;
    /// Table enum index per osc (the block's patch).
    pub const TABLE_A: usize = 4;
    pub const TABLE_B: usize = 5;
    /// Smoothed mod wheel 0..1.
    pub const WHEEL: usize = 6;
    pub const N: usize = 7;
}

/// Per-voice fields (`VoiceViz`).
pub mod vf {
    pub const NOTE: usize = 0;
    /// 1 while the key is held.
    pub const GATE: usize = 1;
    pub const VEL: usize = 2;
    /// Seconds since the voice's last note-on (block resolution).
    pub const AGE: usize = 3;
    pub const AMP_A: usize = 4;
    pub const AMP_B: usize = 5;
    /// Envelope stage: 0 idle, 1 attack, 2 decay, 3 sustain, 4 release.
    pub const STAGE_A: usize = 6;
    pub const STAGE_B: usize = 7;
    pub const FENV: usize = 8;
    pub const FENV_STAGE: usize = 9;
    /// Mod A / Mod B phase in cycles (unwrapped; a finished one-shot is >= 1).
    pub const PH_A: usize = 10;
    pub const PH_B: usize = 11;
    /// Mod A / Mod B output, -1..1.
    pub const OUT_A: usize = 12;
    pub const OUT_B: usize = 13;
    /// 1 while the voice renders (sounding or ringing out).
    pub const ACTIVE: usize = 14;
    pub const N: usize = 15;
}

/// `ModLive`: the param ids whose post-modulation value is published, in record order.
pub const MOD_IDS: [&str; 14] = [
    "osca_index", "oscb_index", "osca_shift", "oscb_shift", "osca_motion", "oscb_motion", "osca_vol", "oscb_vol",
    "filta_freq", "filtb_freq", "moda_pitch", "moda_index", "moda_shift", "fenv_amt",
];

/// FX display fields (`dsp::fx::FxTelemetry`), after the ModLive block.
pub mod xf {
    /// Distortion shaper input with the largest magnitude, signed linear.
    pub const DIST_IN: usize = 0;
    /// Multiband band gain, dB (positive = upward), three bands.
    pub const MB_GAIN: usize = 1;
    /// Multiband detector level per band, dB.
    pub const MB_LEVEL: usize = 4;
    /// Chorus LFO phase, turns.
    pub const CHO_PHASE: usize = 7;
    /// Delay peak input, peak wet (echo) signal (linear) and delay time (s).
    pub const DLY_IN: usize = 8;
    pub const DLY_WET: usize = 9;
    pub const DLY_TIME: usize = 10;
    /// Reverb peak input and peak wet (tail) signal, linear.
    pub const REV_IN: usize = 11;
    pub const REV_WET: usize = 12;
    pub const N: usize = 13;
}

pub const VOICE_BASE: usize = gf::N;
pub const MOD_BASE: usize = VOICE_BASE + MAX_VOICES * vf::N;
pub const FX_BASE: usize = MOD_BASE + MOD_IDS.len();
/// Values per record.
pub const LEN: usize = FX_BASE + xf::N;
/// Scope ring capacity (the GUI reads up to half of it).
pub const SCOPE_RING: usize = 8192;
/// Samples gathered before one ring push.
const CHUNK: usize = 64;

/// The shared record. Writer: [`VizWriter`] (audio thread). Reader: the editor.
pub struct Viz {
    snap: Snap,
    side: RingReader,
}

impl Default for Viz {
    fn default() -> Self {
        Self { snap: Snap::new(LEN), side: ring(2).1 }
    }
}

impl Viz {
    /// The side ring, (L - R) / 2 of the output in step with the scope ring (empty until audio runs).
    pub fn side(&self) -> &RingReader {
        &self.side
    }

    /// Copies the latest complete record into `out`; `false` keeps the caller's previous copy.
    pub fn read(&self, out: &mut VizFrame) -> bool {
        let mut tmp = [0.0f32; LEN];
        let ok = self.snap.read(&mut tmp);
        if ok {
            out.0 = tmp;
        }
        ok
    }

    /// Completed writes so far (0 = nothing published yet).
    pub fn version(&self) -> u32 {
        self.snap.version()
    }

    /// Writes a raw record (tests and demo drivers).
    pub fn write(&self, rec: &[f32; LEN]) {
        self.snap.write(rec)
    }
}

/// One copied record with typed accessors.
#[derive(Clone, Copy, Debug)]
pub struct VizFrame(pub [f32; LEN]);

impl Default for VizFrame {
    fn default() -> Self {
        let mut f = Self([0.0; LEN]);
        f.0[gf::SR] = 44100.0;
        f.0[gf::NEWEST] = -1.0;
        f
    }
}

impl VizFrame {
    pub fn global(&self, k: usize) -> f32 {
        self.0[k]
    }
    pub fn voice(&self, v: usize, k: usize) -> f32 {
        self.0[VOICE_BASE + v * vf::N + k]
    }
    /// Newest sounding voice.
    pub fn newest(&self) -> Option<usize> {
        let n = self.0[gf::NEWEST];
        (n >= 0.0).then_some(n as usize).filter(|&i| i < MAX_VOICES)
    }
    /// Live grain position of osc `i` (0 = A) as 0..1 of the Index range.
    pub fn index01(&self, osc: usize) -> f32 {
        (self.0[gf::INDEX_A + osc] / 127.0).clamp(0.0, 1.0)
    }
    /// Post-modulation normalized value of `id`, if it is a [`MOD_IDS`] target.
    pub fn live01(&self, id: &str) -> Option<f32> {
        MOD_IDS.iter().position(|m| *m == id).map(|k| self.0[MOD_BASE + k])
    }
    /// The held note of the newest voice with its gate open.
    /// FX display value `k` (see [`xf`]).
    pub fn fx(&self, k: usize) -> f32 {
        self.0[FX_BASE + k]
    }
    pub fn held_note(&self) -> Option<u8> {
        (0..MAX_VOICES)
            .filter(|&v| self.voice(v, vf::GATE) > 0.5)
            .max_by(|&a, &b| self.voice(b, vf::AGE).total_cmp(&self.voice(a, vf::AGE)))
            .map(|v| self.voice(v, vf::NOTE) as u8)
    }
}

/// Audio-side half: owns the scope ring writer, the note-on clocks and the record buffer.
pub struct VizWriter {
    viz: Arc<Viz>,
    scope: RingWriter,
    side: RingWriter,
    chunk: [f32; CHUNK],
    side_chunk: [f32; CHUNK],
    n: usize,
    /// Samples rendered so far, and per voice the sample of its last note-on (seen by its age stamp).
    clock: u64,
    /// Clock at the previous publish: a note-on seen now happened at or after it.
    block_start: u64,
    on_at: [u64; MAX_VOICES],
    seen: [u64; MAX_VOICES],
    rec: [f32; LEN],
}

impl VizWriter {
    /// Allocates the record and the ring (call off the audio thread).
    pub fn new() -> (Self, Arc<Viz>, RingReader) {
        let (scope, reader) = ring(SCOPE_RING);
        let (side, side_reader) = ring(SCOPE_RING);
        let viz = Arc::new(Viz { snap: Snap::new(LEN), side: side_reader });
        let w = Self { viz: viz.clone(), scope, side, chunk: [0.0; CHUNK], side_chunk: [0.0; CHUNK], n: 0, clock: 0, block_start: 0, on_at: [0; MAX_VOICES], seen: [0; MAX_VOICES], rec: VizFrame::default().0 };
        (w, viz, reader)
    }

    /// One output frame (after the limiter) into the scope ring.
    #[inline]
    pub fn push(&mut self, l: f32, r: f32) {
        self.chunk[self.n] = 0.5 * (l + r);
        self.side_chunk[self.n] = 0.5 * (l - r);
        self.n += 1;
        self.clock += 1;
        if self.n == CHUNK {
            self.scope.push(&self.chunk);
            self.side.push(&self.side_chunk);
            self.n = 0;
        }
    }

    /// End of a block: flushes the scope and publishes one record. `patch` is the block's patch
    /// (what the voices read), `sr` the sample rate, `fx` the chain's display record.
    pub fn publish(&mut self, engine: &Engine, params: &GtsParams, patch: &Patch, sr: f32, fx: &FxTelemetry) {
        if self.n > 0 {
            self.scope.push(&self.chunk[..self.n]);
            self.side.push(&self.side_chunk[..self.n]);
            self.n = 0;
        }
        let rec = &mut self.rec;
        let stage = |s: Stage| match s {
            Stage::Idle => 0.0,
            Stage::Attack => 1.0,
            Stage::Decay => 2.0,
            Stage::Sustain => 3.0,
            Stage::Release => 4.0,
        };
        let mut newest: Option<usize> = None;
        for i in 0..MAX_VOICES {
            let v = engine.voice(i);
            if v.age != self.seen[i] {
                self.seen[i] = v.age;
                self.on_at[i] = self.block_start;
            }
            if v.sounding() && newest.is_none_or(|n| engine.voice(n).age < v.age) {
                newest = Some(i);
            }
            let f = &mut rec[VOICE_BASE + i * vf::N..VOICE_BASE + (i + 1) * vf::N];
            f[vf::NOTE] = v.note as f32;
            f[vf::GATE] = v.gate as u8 as f32;
            f[vf::VEL] = v.velocity;
            f[vf::AGE] = (self.clock - self.on_at[i]) as f32 / sr;
            f[vf::AMP_A] = v.amp_env(0).level();
            f[vf::AMP_B] = v.amp_env(1).level();
            f[vf::STAGE_A] = stage(v.amp_env(0).stage());
            f[vf::STAGE_B] = stage(v.amp_env(1).stage());
            f[vf::FENV] = v.filter_env().level();
            f[vf::FENV_STAGE] = stage(v.filter_env().stage());
            f[vf::PH_A] = v.lfo_phase(0);
            f[vf::PH_B] = v.lfo_phase(1);
            f[vf::OUT_A] = v.lfo_out[0];
            f[vf::OUT_B] = v.lfo_out[1];
            f[vf::ACTIVE] = v.is_active() as u8 as f32;
        }
        let wheel = engine.wheel();
        rec[gf::SR] = sr;
        rec[gf::NEWEST] = newest.map_or(-1.0, |n| n as f32);
        rec[gf::WHEEL] = wheel;
        for o in 0..2 {
            rec[gf::TABLE_A + o] = patch.osc[o].table as usize as f32;
            rec[gf::INDEX_A + o] = match newest {
                Some(n) => engine.voice(n).index(o),
                None => (patch.osc[o].index + 127.0 * patch.wheel_index * wheel).clamp(0.0, 127.0),
            };
        }

        // ModLive, in MOD_IDS order. Unmodulated values while nothing sounds.
        let (m, vel, fenv, cut, b) = match newest.map(|n| engine.voice(n)) {
            Some(v) => (v.modulation, v.velocity, v.filter_env().level(), v.cutoff_hz, v.lfo_out[1]),
            None => (Default::default(), 1.0, 0.0, [patch.filt[0].freq, patch.filt[1].freq], 0.0),
        };
        // The Mod A depth scale, as `Voice::modulate` applies it.
        let depth = (1.0 + patch.modb_moda * b) * (1.0 + patch.wheel_mod * wheel) * vel_scale(patch.vel_mod, vel);
        let p = params;
        let live = [
            p.osca.index.preview_normalized((patch.osc[0].index + 127.0 * patch.wheel_index * wheel + m.index[0]).clamp(0.0, 127.0)),
            p.oscb.index.preview_normalized((patch.osc[1].index + 127.0 * patch.wheel_index * wheel + m.index[1]).clamp(0.0, 127.0)),
            p.osca.shift.preview_normalized(patch.osc[0].shift + 24.0 * (patch.wheel_shift * wheel + patch.vel_shift * vel) + m.shift[0]),
            p.oscb.shift.preview_normalized(patch.osc[1].shift + 24.0 * (patch.wheel_shift * wheel + patch.vel_shift * vel) + m.shift[1]),
            p.osca.motion.preview_normalized(patch.osc[0].motion + m.motion[0]),
            p.oscb.motion.preview_normalized(patch.osc[1].motion + m.motion[1]),
            p.osca.vol.preview_normalized(patch.osc[0].vol * (1.0 + m.vol[0]).max(0.0)),
            p.oscb.vol.preview_normalized(patch.osc[1].vol * (1.0 + m.vol[1]).max(0.0)),
            p.filta.freq.preview_normalized(cut[0]),
            p.filtb.freq.preview_normalized(cut[1]),
            // Amount knobs: the depth Mod B ("Mod A"), wheel Mod and velocity Mod give them now.
            p.moda_amt.pitch.preview_normalized(patch.moda_pitch * depth),
            p.moda_amt.index.preview_normalized(patch.moda_index * depth),
            p.moda_amt.shift.preview_normalized(patch.moda_shift * depth),
            p.fenv.amt.preview_normalized(patch.fenv_amt * fenv * vel_scale(patch.vel_fenv, vel)),
        ];
        for (k, x) in live.into_iter().enumerate() {
            rec[MOD_BASE + k] = x.clamp(0.0, 1.0);
        }
        let f = &mut rec[FX_BASE..];
        f[xf::DIST_IN] = fx.dist_in;
        f[xf::MB_GAIN..xf::MB_GAIN + 3].copy_from_slice(&fx.mb_gain_db);
        f[xf::MB_LEVEL..xf::MB_LEVEL + 3].copy_from_slice(&fx.mb_level_db);
        f[xf::CHO_PHASE] = fx.cho_phase;
        f[xf::DLY_IN] = fx.dly_in;
        f[xf::DLY_WET] = fx.dly_wet;
        f[xf::DLY_TIME] = fx.dly_time_s;
        f[xf::REV_IN] = fx.rev_in;
        f[xf::REV_WET] = fx.rev_wet;
        self.viz.write(rec);
        self.block_start = self.clock;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_is_dense_and_ids_are_real() {
        assert_eq!(LEN, gf::N + MAX_VOICES * vf::N + MOD_IDS.len() + xf::N);
        let params = GtsParams::default();
        let ids: Vec<String> = nih_plug::prelude::Params::param_map(&params).into_iter().map(|(id, _, _)| id).collect();
        for id in MOD_IDS {
            assert!(ids.iter().any(|i| i == id), "{id} is not a param");
        }
    }

    /// The real publisher on a playing engine: one consistent record per block, the ring filled.
    #[test]
    fn publishes_a_playing_voice() {
        let params = GtsParams::default();
        let mut e = Engine::new();
        e.initialize(44100.0);
        let patch = Patch::from_params(&params);
        e.set_patch(&patch);
        let (mut w, viz, ring) = VizWriter::new();
        e.note_on(41, 0.9);
        for _ in 0..8 {
            for _ in 0..256 {
                let (l, r) = e.next_frame();
                w.push(l, r);
            }
            let fx = e.fx_telemetry();
            w.publish(&e, &params, &patch, 44100.0, &fx);
        }
        let mut f = VizFrame::default();
        assert!(viz.read(&mut f));
        let v = f.newest().expect("a sounding voice");
        assert_eq!(f.voice(v, vf::NOTE), 41.0);
        assert_eq!(f.voice(v, vf::GATE), 1.0);
        assert!((f.voice(v, vf::AGE) - 2048.0 / 44100.0).abs() < 1e-3, "age {}", f.voice(v, vf::AGE));
        assert!(f.voice(v, vf::AMP_A) > 0.5);
        assert_eq!(f.held_note(), Some(41));
        assert!((f.index01(0) - params.osca.index.value() / 127.0).abs() < 0.02);
        assert_eq!(ring.written(), 2048);
        assert_eq!(viz.side().written(), 2048);
        let mut s = [0.0f32; 1024];
        assert!(ring.latest(&mut s));
        assert!(s.iter().any(|x| x.abs() > 0.01), "scope is silent");
    }
}
