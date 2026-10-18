//! Shared by the editor examples (frame_bench, plugin_loop, frame_gpu, editor_preview): a `Ctl`
//! that holds its own values, and [`Feed`], which plays the riddim demo pattern on the REAL engine
//! and publishes the scope ring, meters and voice / modulation record exactly as `Gts::process`
//! does (same `VizWriter`, same `Meters`).
#![allow(dead_code)]

use gts::dsp::{Engine, Patch};
use gts::editor::{apply_preset, Ctl, Meters, ParamTable};
use gts::matrix::{MatrixShared, ENC_LEN};
use gts::params::GtsParams;
use gts::preset::{self, Preset};
use gts::viz::{Viz, VizWriter};
use rr_ui::ring::RingReader;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// Default params plus an override map: edits (and preset loads) stay inside the example.
pub struct MapCtl {
    pub table: ParamTable,
    pub vals: RefCell<HashMap<String, f32>>,
    /// The Patch the sound started from (what Save writes; edits in `vals` are not folded back).
    pub base: std::cell::Cell<Patch>,
}

impl MapCtl {
    pub fn new() -> Self {
        Self { table: ParamTable::new(Arc::new(GtsParams::default())), vals: RefCell::new(HashMap::new()), base: Default::default() }
    }
    /// Loads a factory preset the way the editor does (through `Ctl` gestures).
    pub fn with_preset(p: Option<&Preset>) -> Self {
        let c = Self::new();
        apply_preset(&c.table, &c, p);
        c.base.set(p.map_or_else(Patch::default, |p| p.patch));
        c
    }
}

impl Ctl for MapCtl {
    fn get01(&self, id: &str) -> f32 {
        self.vals.borrow().get(id).copied().unwrap_or_else(|| self.table.get01(id))
    }
    fn default01(&self, id: &str) -> f32 {
        self.table.default01(id)
    }
    fn readout(&self, id: &str) -> String {
        self.table.readout_at(id, self.get01(id))
    }
    fn steps(&self, id: &str) -> Option<usize> {
        self.table.steps(id)
    }
    fn begin(&self, _: &str) {}
    fn set01(&self, id: &str, v: f32) {
        let v = match self.table.steps(id) {
            Some(n) if n > 0 => (v * n as f32).round() / n as f32,
            _ => v,
        };
        self.vals.borrow_mut().insert(id.to_string(), v);
    }
    fn end(&self, _: &str) {}
    fn readout_at(&self, id: &str, v: f32) -> String {
        self.table.readout_at(id, v)
    }
    fn name(&self, id: &str) -> String {
        self.table.name(id)
    }
    fn plain(&self, id: &str, v01: f32) -> f32 {
        self.table.plain(id, v01)
    }
    fn norm(&self, id: &str, plain: f32) -> f32 {
        self.table.norm(id, plain)
    }
    fn matrix(&self) -> Option<&MatrixShared> {
        Some(self.table.matrix())
    }
    fn current_preset(&self) -> Option<Preset> {
        let m = self.table.matrix().get();
        let p = self.table.params();
        Some(Preset { patch: self.base.get(), matrix: m.slots.clone(), macros: m.macros.to_vec(), lfo_shapes: p.lfo_shapes.get(), fx_order: p.fx_order.get(), ..Preset::default() })
    }
}

/// A library over `user` generated user presets (plus the awkward files) and the embedded factory
/// bank, in a scratch folder under the target dir, rebuilt each run. A few favourites and a
/// remembered author make the browser shots look lived in.
pub fn fixture_library(user: usize) -> gts::library::Library {
    let base = std::env::var_os("CARGO_TARGET_DIR").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir).join("tmp").join("gts-fixture");
    let _ = std::fs::remove_dir_all(base.join("Presets").join("User"));
    for f in ["favorites.json", "settings.json"] {
        let _ = std::fs::remove_file(base.join(f));
    }
    gts::library::fixture::write_user(&base, user);
    let mut lib = gts::library::Library::open(&base, preset::bank::FACTORY_FILES);
    let keys: Vec<String> = lib.entries().iter().map(|e| e.key.clone()).collect();
    for k in keys.iter().step_by(7).take(6) {
        let _ = lib.set_favorite(k, true);
    }
    let _ = lib.update_settings(|s| s.author = "Mara Voss".into());
    lib
}

/// The FX page shots and benchmarks show the chain working: Distortion, Multiband, EQ, Phaser,
/// Flanger, Dimension, Delay and Reverb on, in the real engine's patch and on the toggles.
pub fn fx_on(ctl: &MapCtl, patch: &mut Patch) {
    let f = &mut patch.fx;
    for b in [&mut f.dist.on, &mut f.mb.on, &mut f.eq.on, &mut f.phs.on, &mut f.flg.on, &mut f.dim.on, &mut f.dly.on, &mut f.rev.on] {
        *b = true;
    }
    for id in ["dist_on", "mb_on", "eq_on", "phs_on", "flg_on", "dim_on", "dly_on", "rev_on"] {
        ctl.vals.borrow_mut().insert(id.to_string(), 1.0);
    }
}

/// A plain value on the knob (the editor's side only; the engine reads the `Patch`).
pub fn set(ctl: &MapCtl, id: &str, plain: f32) {
    let v = ctl.table.norm(id, plain);
    ctl.vals.borrow_mut().insert(id.to_string(), v);
}

/// Track 7 on show: all four LFOs on custom shapes (stock shapes, a step pattern on Mod C), the mod
/// envelope shaped and patched to Pitch All for a dive, Mod C and Mod D patched too, the noise layer
/// and the sub triangle on, Osc A on FM warp: the editor shots and the frame gates show the new
/// modulators, displays and rings working. The shapes reach the engine through `Feed::attach`.
pub fn track7_demo(ctl: &MapCtl, patch: &mut Patch) {
    use gts::dsp::shape::library;
    use gts::matrix::Source::*;
    use gts::params::{Curve, SubShape, WarpMode};
    let lib = library();
    let pick = |n: &str| lib.iter().find(|(m, _)| *m == n).map(|(_, d)| d.clone()).expect("stock shape");
    ctl.table.matrix().edit_shapes(|s| {
        s.0[0] = pick("Call and Response");
        s.0[1] = pick("Pump");
        s.0[2] = pick("Riddim Gate 2");
        s.0[3] = pick("Bounce");
    });
    for (i, id) in ["moda_curve", "modb_curve", "modc_curve", "modd_curve"].into_iter().enumerate() {
        ctl.vals.borrow_mut().insert(id.to_string(), 1.0);
        let m = match i {
            0 => &mut patch.moda,
            1 => &mut patch.modb,
            2 => &mut patch.modc,
            _ => &mut patch.modd,
        };
        m.curve = Curve::Custom;
    }
    patch.modc.sync = true;
    patch.modd.sync = true;
    for id in ["modc_sync", "modd_sync"] {
        ctl.vals.borrow_mut().insert(id.to_string(), 1.0);
    }
    (patch.menv_att, patch.menv_dec, patch.menv_sus, patch.menv_rel) = (0.012, 0.26, 0.55, 0.38);
    for (id, v) in [("menv_att", 0.012), ("menv_dec", 0.26), ("menv_sus", 0.55), ("menv_rel", 0.38)] {
        set(ctl, id, v);
    }
    patch.noise_level_db = -24.0;
    patch.noise_type = gts::params::NoiseType::Pink;
    set(ctl, "noise_level", -24.0);
    ctl.vals.borrow_mut().insert("noise_type".into(), 0.5);
    patch.sub_shape = SubShape::Triangle;
    ctl.vals.borrow_mut().insert("sub_shape".into(), 1.0);
    patch.osc[0].warp = WarpMode::Fm;
    patch.osc[0].warp_amt = 0.35;
    ctl.vals.borrow_mut().insert("osca_warp_mode".into(), 1.0 / 5.0);
    set(ctl, "osca_warp_amt", 0.35);
    ctl.table.matrix().edit(|m| {
        for (s, t, a) in [(ModEnv, "pitch_all", 0.15), (ModEnv, "osca_pitch", 0.1), (ModC, "filta_freq", 0.3), (ModD, "dist_drive", 0.25), (ModA, "oscb_vol", 0.2)] {
            if m.find(s, t).is_none() && m.slots.len() < gts::matrix::MAX_SLOTS {
                m.slots.push(gts::matrix::Slot::new(s, t, a));
            }
        }
    });
}

/// Eight routings on top of whatever the preset has, on varied sources and targets, and two macros
/// turned up: the editor shots and benchmarks show rings, live dots and the MATRIX rows working.
pub fn demo_routes(ctl: &MapCtl, patch: &mut Patch) {
    use gts::matrix::Source::*;
    ctl.table.matrix().edit(|m| {
        for (s, t, a) in [
            (Velocity, "filta_freq", 0.35),
            (Macro1, "dist_drive", 0.5),
            (ModB, "osca_shift", 0.25),
            (Random, "osca_cent", 0.2),
            (Keytrack, "filta_freq", 0.3),
            (ModWheel, "rev_mix", 0.4),
            (AmpEnvA, "shaper_amt", 0.3),
            (Macro2, "dly_mix", -0.5),
        ] {
            if m.find(s, t).is_none() && m.slots.len() < gts::matrix::MAX_SLOTS {
                m.slots.push(gts::matrix::Slot::new(s, t, a));
            }
        }
    });
    patch.macros = [0.6, 0.3, 0.0, 0.0];
    for (id, v) in [("macro1", 0.6), ("macro2", 0.3)] {
        ctl.vals.borrow_mut().insert(id.to_string(), v);
    }
}

/// Factory preset by name, with its library key ("Factory/Bass/Growl Hound").
pub fn factory(name: &str) -> (String, &'static Preset) {
    let f = preset::factory();
    let i = f.iter().position(|p| p.name == name).unwrap_or(0);
    (format!("Factory/{}", preset::bank::FACTORY_FILES[i].0.trim_end_matches(".json")), &f[i])
}

/// 150 BPM, 1/8 steps, 16-step riddim pattern (F1, G1, A#1; 0 = rest), gate 70 % of the step.
const PATTERN: [u8; 16] = [29, 0, 29, 31, 0, 29, 34, 0, 29, 0, 29, 31, 0, 34, 31, 29];
const STEP_S: f64 = 0.2;

pub struct Feed {
    engine: Engine,
    params: GtsParams,
    patch: Patch,
    writer: VizWriter,
    pub meters: Arc<Meters>,
    pub viz: Arc<Viz>,
    pub ring: RingReader,
    sr: f32,
    clock: u64,
    held: Option<u8>,
    /// The params whose matrix, custom shapes and FX order the engine follows (see [`Feed::attach`]),
    /// and the versions seen.
    matrix: Option<Arc<GtsParams>>,
    seen: u32,
    rec: [f32; ENC_LEN],
    shapes_seen: u32,
    shapes_rec: [f32; gts::dsp::shape::SHAPES_ENC],
}

impl Feed {
    pub fn new(patch: Patch) -> Self {
        let mut engine = Engine::new();
        engine.initialize(44100.0);
        engine.set_patch(&patch);
        let (writer, viz, ring) = VizWriter::new();
        Self { engine, params: GtsParams::default(), patch, writer, meters: Arc::new(Meters::default()), viz, ring, sr: 44100.0, clock: 0, held: None, matrix: None, seen: u32::MAX, rec: [0.0; ENC_LEN], shapes_seen: u32::MAX, shapes_rec: [0.0; gts::dsp::shape::SHAPES_ENC] }
    }

    /// Plays through the matrix of `params` (the editor's `ParamTable::params`): a preset load or a
    /// patch made in the editor reaches the engine at the next block, as `Gts::process` does.
    pub fn attach(&mut self, params: Arc<GtsParams>) {
        self.matrix = Some(params);
    }

    /// Renders `n` samples in blocks of up to 256 (note events on the pattern grid), publishing after each block.
    pub fn render(&mut self, mut n: usize) {
        if let Some(p) = &self.matrix {
            if p.matrix.poll(&mut self.seen, &mut self.rec) {
                self.engine.set_matrix(&self.rec);
            }
            if p.lfo_shapes.poll(&mut self.shapes_seen, &mut self.shapes_rec) {
                self.engine.set_lfo_shapes(&self.shapes_rec);
            }
            self.engine.set_fx_order(p.fx_order.get());
        }
        let step_len = (STEP_S * self.sr as f64) as u64;
        let gate_len = step_len * 7 / 10;
        while n > 0 {
            let pos = self.clock % step_len;
            let block = (256u64.min(n as u64)).min(if pos < gate_len { gate_len - pos } else { step_len - pos }) as usize;
            let step = (self.clock / step_len) as usize % PATTERN.len();
            if pos == 0 && PATTERN[step] != 0 {
                self.engine.note_on(PATTERN[step], 0.9);
                self.held = Some(PATTERN[step]);
            }
            if pos == gate_len {
                if let Some(h) = self.held.take() {
                    self.engine.note_off(h);
                }
            }
            let mut peak = [0.0f32; 2];
            for _ in 0..block {
                let (l, r) = self.engine.next_frame();
                peak = [peak[0].max(l.abs()), peak[1].max(r.abs())];
                self.writer.push(l, r);
            }
            for (m, p) in self.meters.peak.iter().zip(peak) {
                m.fetch_max(p.to_bits(), Ordering::Relaxed);
            }
            self.meters.gr_db.fetch_max(self.engine.limiter_gr_db().to_bits(), Ordering::Relaxed);
            let fx = self.engine.fx_telemetry();
            self.writer.publish(&self.engine, &self.params, &self.patch, self.sr, &fx);
            self.clock += block as u64;
            n -= block;
        }
    }

    /// Holds a chord of `notes` (velocity 0.8) for the rest of the run: with the riddim pattern on top,
    /// the frame gates measure eight voices sounding.
    pub fn hold_chord(&mut self, notes: &[u8]) {
        for &n in notes {
            self.engine.note_on(n, 0.8);
        }
    }

    /// Sample rate of the feed.
    pub fn sr(&self) -> f32 {
        self.sr
    }

    /// Takes the meter peaks gathered since the last call (what the editor closure does).
    pub fn take_peaks(&self) -> [f32; 2] {
        std::array::from_fn(|i| f32::from_bits(self.meters.peak[i].swap(0, Ordering::Relaxed)))
    }
}
