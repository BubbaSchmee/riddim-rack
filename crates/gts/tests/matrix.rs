//! SPEC v2 Track 5 acceptance 1: with an empty matrix the engine renders bit-identically to the
//! pre-matrix engine (788edb8). The golden hash was recorded by this scenario at that commit.
use gts::dsp::{Engine, Patch};
use gts::params::{Curve, Routing};
use nih_plug::prelude::NoteEvent;

const SR: f32 = 48000.0;

/// FNV-1a over the bits of every output sample.
fn fnv(h: &mut u64, x: f32) {
    for b in x.to_bits().to_le_bytes() {
        *h ^= b as u64;
        *h = h.wrapping_mul(0x100_0000_01b3);
    }
}

fn busy_patch() -> Patch {
    let mut p = Patch::riddim_v1();
    p.poly = 4;
    p.osc[0].unison = 3;
    p.osc[0].width = 0.6;
    p.osc[1].on = true;
    p.modb.curve = Curve::Sine;
    p.modb.rate = 3.0;
    p.modb_cutoff = 0.3;
    p.modb_motion = 0.2;
    p.wheel_index = 0.4;
    p.vel_shift = 0.2;
    p.shaper_on = true;
    p.routing = Routing::Series;
    p.spread = 0.3;
    p.fx.cho.on = true;
    p.fx.dly.on = true;
    p.fx.rev.on = true;
    p
}

/// Host-like driving: set_patch per 256-sample block, a CV-style index sweep through
/// set_cv_patch every 32 samples, notes / bend / wheel / pressure events.
pub fn render_scenario(e: &mut Engine, p: &Patch, secs: f32) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    let n = (secs * SR) as usize;
    for i in 0..n {
        if i % 256 == 0 {
            e.set_patch(p);
        }
        if i % 32 == 0 {
            let mut q = *p;
            q.osc[0].index = 64.0 + 40.0 * (i as f32 / 9000.0).sin();
            e.set_cv_patch(&q);
        }
        match i {
            100 => e.handle_event(&NoteEvent::NoteOn { timing: 0, voice_id: None, channel: 0, note: 36, velocity: 0.9 }),
            5000 => e.handle_event(&NoteEvent::NoteOn { timing: 0, voice_id: None, channel: 0, note: 43, velocity: 0.4 }),
            9000 => e.handle_event(&NoteEvent::MidiCC { timing: 0, channel: 0, cc: 1, value: 0.7 }),
            12000 => e.handle_event(&NoteEvent::PolyPressure { timing: 0, voice_id: None, channel: 0, note: 36, pressure: 0.8 }),
            14000 => e.handle_event(&NoteEvent::MidiChannelPressure { timing: 0, channel: 0, pressure: 0.5 }),
            16000 => e.handle_event(&NoteEvent::MidiPitchBend { timing: 0, channel: 0, value: 0.8 }),
            30000 => e.handle_event(&NoteEvent::NoteOff { timing: 0, voice_id: None, channel: 0, note: 36, velocity: 0.0 }),
            40000 => e.handle_event(&NoteEvent::NoteOn { timing: 0, voice_id: None, channel: 0, note: 48, velocity: 1.0 }),
            60000 => e.handle_event(&NoteEvent::NoteOff { timing: 0, voice_id: None, channel: 0, note: 43, velocity: 0.0 }),
            _ => {}
        }
        let (l, r) = e.next_frame();
        fnv(&mut h, l);
        fnv(&mut h, r);
    }
    h
}

#[test]
fn empty_matrix_is_bit_identical_to_the_pre_matrix_engine() {
    let p = busy_patch();
    let mut e = Engine::new();
    e.initialize(SR);
    e.set_patch(&p);
    let h = render_scenario(&mut e, &p, 2.0);
    println!("scenario hash {h:#018x}");
    assert_eq!(h, GOLDEN);
}

/// Recorded at 788edb8 (before the matrix existed).
const GOLDEN: u64 = 0xdd4d_2e6a_1c5e_4fe1;

// ---- helpers ---------------------------------------------------------------------------------

use gts::editor::{apply_preset, Ctl, ParamTable};
use gts::matrix::{
    fixed_routes, param_index, params_info, Edit, EditError, Matrix, MatrixEditor, MatrixShared, ParamInfo, Slot, Source,
    ENC_LEN, MAX_SLOTS,
};
use gts::params::GtsParams;
use gts::preset::Preset;
use nih_plug::prelude::Params;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

fn info(id: &str) -> &'static ParamInfo {
    &params_info()[param_index(id).unwrap_or_else(|| panic!("{id}"))]
}

fn enc(slots: &[Slot]) -> [f32; ENC_LEN] {
    Matrix { slots: slots.to_vec(), ..Default::default() }.encode()
}

fn engine_with(p: &Patch, slots: &[Slot]) -> Engine {
    let mut e = Engine::new();
    e.initialize(SR);
    e.set_patch(p);
    e.set_matrix(&enc(slots));
    e
}

/// `n` samples with `set_patch` every 256 like a host block.
fn run(e: &mut Engine, p: &Patch, n: usize) {
    for i in 0..n {
        if i % 256 == 0 {
            e.set_patch(p);
        }
        e.next_frame();
    }
}

fn voice_of(e: &Engine, note: u8) -> usize {
    (0..8).find(|&i| e.voice(i).note == note && e.voice(i).gate).expect("voice for note")
}

/// Expected plain value of `id` at base `plain` plus a normalized offset (clamped once).
fn moved(id: &str, plain: f32, off: f32) -> f32 {
    let i = info(id);
    i.plain((i.norm(plain) + off).clamp(0.0, 1.0))
}

fn near(a: f32, b: f32, tol: f32, what: &str) {
    assert!((a - b).abs() <= tol, "{what}: {a} vs {b}");
}

/// A host stand-in: reads defaults through the param table, records what is set.
struct FakeCtl {
    table: ParamTable,
    vals: RefCell<BTreeMap<String, f32>>,
}

impl FakeCtl {
    fn new() -> Self {
        Self { table: ParamTable::new(Arc::new(GtsParams::default())), vals: Default::default() }
    }
    fn set(&self, id: &str, v: f32) {
        self.vals.borrow_mut().insert(id.into(), v);
    }
}

impl Ctl for FakeCtl {
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
        self.set(id, v);
    }
    fn end(&self, _: &str) {}
}

// ---- (1) equivalence -------------------------------------------------------------------------

/// Bypassed and zero-amount slots, and a matrix set then cleared, leave the render bit-identical.
#[test]
fn inactive_matrices_are_bit_identical_too() {
    let p = busy_patch();
    let mut off: Vec<Slot> =
        (0..MAX_SLOTS).map(|k| Slot { enabled: false, ..Slot::new(Source::Macro1, "filta_freq", 0.5 + k as f32 / 100.0) }).collect();
    off[3] = Slot::new(Source::ModA, "rev_mix", 0.0);
    for slots in [off.clone(), vec![]] {
        let mut e = engine_with(&p, &[Slot::new(Source::Velocity, "osca_index", 0.7), Slot::new(Source::Macro1, "volume", 0.5)]);
        e.set_matrix(&enc(&slots));
        assert!(!e.matrix_on());
        assert_eq!(render_scenario(&mut e, &p, 2.0), GOLDEN);
    }
}

// ---- (2) per voice vs newest voice -----------------------------------------------------------

#[test]
fn velocity_slot_moves_each_voice_by_its_own_velocity() {
    let mut p = Patch::default();
    p.poly = 2;
    p.filt[0].kbd = 0.0;
    p.filt[0].env = false;
    p.filt[0].freq = 1000.0;
    let mut e = engine_with(&p, &[Slot::new(Source::Velocity, "filta_freq", 0.5)]);
    e.note_on(48, 0.2);
    e.note_on(55, 1.0);
    run(&mut e, &p, 2048);
    let (a, b) = (voice_of(&e, 48), voice_of(&e, 55));
    let (ca, cb) = (e.voice(a).cutoff_hz[0], e.voice(b).cutoff_hz[0]);
    near(ca, moved("filta_freq", 1000.0, 0.1), 0.005 * ca, "vel 0.2 cutoff");
    near(cb, moved("filta_freq", 1000.0, 0.5), 0.005 * cb, "vel 1.0 cutoff");
    assert!(cb > 2.0 * ca, "{ca} Hz vs {cb} Hz");
    // The knob itself (the engine patch) is untouched.
    assert_eq!(e.patch().filt[0].freq, 1000.0);
}

#[test]
fn global_target_follows_the_newest_voice() {
    let mut p = Patch::default();
    p.poly = 2;
    let mut e = engine_with(&p, &[Slot::new(Source::Velocity, "volume", 0.5)]);
    e.note_on(48, 0.2);
    run(&mut e, &p, 4800);
    near(e.patch().volume_db, moved("volume", -5.0, 0.1), 0.01, "after the first note");
    e.note_on(55, 1.0);
    run(&mut e, &p, 4800);
    near(e.patch().volume_db, moved("volume", -5.0, 0.5), 0.01, "newest = vel 1.0 (never the sum 0.6)");
    e.handle_event(&NoteEvent::Choke { timing: 0, voice_id: None, channel: 0, note: 55 });
    run(&mut e, &p, 4800);
    near(e.patch().volume_db, moved("volume", -5.0, 0.1), 0.01, "back to the remaining voice");
}

// ---- (3) sum and clamp -----------------------------------------------------------------------

#[test]
fn slots_on_one_target_add_and_clamp_once() {
    let base = 40.0; // the init Osc A Index
    for (macros, want) in [([0.5, 0.5], moved("osca_index", base, 0.15 + 0.2)), ([1.0, 1.0], 127.0), ([0.0, 0.0], base)] {
        let mut p = Patch::default();
        p.macros = [macros[0], macros[1], 0.0, 0.0];
        let mut e = engine_with(&p, &[Slot::new(Source::Macro1, "osca_index", 0.3), Slot::new(Source::Macro2, "osca_index", 0.4)]);
        e.note_on(48, 1.0);
        run(&mut e, &p, 512);
        near(e.voice_patch(voice_of(&e, 48)).osc[0].index, want, 1e-3, &format!("index at macros {macros:?}"));
    }
    // Global: the -24 dB floor.
    let mut p = Patch::default();
    p.macros = [1.0, 1.0, 0.0, 0.0];
    let mut e = engine_with(&p, &[Slot::new(Source::Macro1, "volume", -0.6), Slot::new(Source::Macro2, "volume", -0.6)]);
    run(&mut e, &p, 4800);
    near(e.patch().volume_db, -24.0, 1e-3, "volume clamps at its minimum");
}

// ---- (4) polarity, invert, bypass ------------------------------------------------------------

/// Osc A Index of the note-60 voice for one slot.
fn index_with(slot: Slot, macro1: f32) -> f32 {
    let mut p = Patch::default();
    p.macros[0] = macro1;
    let mut e = engine_with(&p, &[slot]);
    e.note_on(60, 1.0);
    run(&mut e, &p, 512);
    e.voice_patch(voice_of(&e, 60)).osc[0].index
}

#[test]
fn unipolar_bipolar_invert_and_bypass() {
    let base = 40.0;
    // Keytrack is bipolar by nature: C4 = 0. Unipolar maps it to 0.5.
    near(index_with(Slot::new(Source::Keytrack, "osca_index", 0.5), 0.0), base, 1e-3, "keytrack bipolar at C4");
    let uni = Slot { bipolar: false, ..Slot::new(Source::Keytrack, "osca_index", 0.5) };
    near(index_with(uni, 0.0), moved("osca_index", base, 0.25), 1e-3, "keytrack unipolar at C4");
    // A macro is unipolar by nature; bipolar maps 0..1 to -1..1.
    near(index_with(Slot::new(Source::Macro1, "osca_index", 0.5), 0.5), moved("osca_index", base, 0.25), 1e-3, "macro unipolar");
    let bi = Slot { bipolar: true, ..Slot::new(Source::Macro1, "osca_index", 0.2) };
    near(index_with(bi, 0.0), moved("osca_index", base, -0.2), 1e-3, "macro bipolar at 0");
    // A negative amount inverts; bypass keeps the slot but stops it.
    near(index_with(Slot::new(Source::Macro1, "osca_index", -0.3), 1.0), moved("osca_index", base, -0.3), 1e-3, "inverted");
    let off = Slot { enabled: false, ..Slot::new(Source::Macro1, "osca_index", 0.9) };
    near(index_with(off, 1.0), base, 0.0, "bypassed");
}

// ---- sources ---------------------------------------------------------------------------------

#[test]
fn random_is_per_note_and_deterministic() {
    let once = || {
        let mut p = Patch::default();
        p.poly = 4;
        let mut e = engine_with(&p, &[Slot::new(Source::Random, "osca_index", 0.25)]);
        let mut got = vec![];
        for n in [40u8, 47, 52] {
            e.note_on(n, 1.0);
            run(&mut e, &p, 256);
            got.push(e.voice_patch(voice_of(&e, n)).osc[0].index);
        }
        got
    };
    let (a, b) = (once(), once());
    assert_eq!(a, b, "same notes, same random values");
    assert!(a[0] != a[1] && a[1] != a[2], "{a:?}");
    for x in &a {
        assert!((moved("osca_index", 40.0, -0.25)..=moved("osca_index", 40.0, 0.25)).contains(x), "{x}");
    }
}

#[test]
fn poly_and_channel_aftertouch_reach_their_voices() {
    let mut p = Patch::default();
    p.poly = 2;
    let slots = [Slot::new(Source::PolyAftertouch, "osca_shift", 0.5), Slot::new(Source::ChannelAftertouch, "oscb_shift", 0.5)];
    let mut e = engine_with(&p, &slots);
    e.note_on(48, 1.0);
    e.note_on(55, 1.0);
    e.handle_event(&NoteEvent::PolyPressure { timing: 0, voice_id: None, channel: 0, note: 55, pressure: 1.0 });
    e.handle_event(&NoteEvent::MidiChannelPressure { timing: 0, channel: 0, pressure: 0.5 });
    run(&mut e, &p, 4800);
    let (a, b) = (voice_of(&e, 48), voice_of(&e, 55));
    near(e.voice_patch(a).osc[0].shift, 0.0, 1e-3, "no pressure on 48");
    near(e.voice_patch(b).osc[0].shift, 24.0, 1e-3, "full pressure on 55: +0.5 of the ±24 st range");
    for v in [a, b] {
        near(e.voice_patch(v).osc[1].shift, 12.0, 1e-3, "channel pressure 0.5 on every voice");
    }
}

/// 5 ms one-pole, stepped every 32 samples: a macro step is ~63 % of the way after 5 ms.
#[test]
fn offsets_glide_over_five_ms() {
    let mut p = Patch::default();
    let mut e = engine_with(&p, &[Slot::new(Source::Macro1, "rev_mix", 0.5)]);
    run(&mut e, &p, 1024);
    p.macros[0] = 1.0;
    let base = info("rev_mix").norm(p.fx.rev.mix);
    let mut at = |n: usize| {
        run(&mut e, &p, n);
        (info("rev_mix").norm(e.patch().fx.rev.mix) - base) / 0.5
    };
    let (t5, t50) = (at(256), at(2048));
    assert!((0.55..0.75).contains(&t5), "after 256 samples {t5}");
    assert!(t50 > 0.999, "{t50}");
}

// ---- ModLive ---------------------------------------------------------------------------------

#[test]
fn modlive_publishes_slot_targets() {
    use gts::viz::{VizFrame, VizWriter, MOD_IDS};
    let params = GtsParams::default();
    let mut p = Patch::default();
    p.macros = [1.0, 0.5, 0.0, 0.0];
    let slots = [Slot::new(Source::Macro1, "rev_mix", 0.4), Slot::new(Source::Macro2, "filta_res", 0.5), Slot::new(Source::Macro1, "osca_index", 0.2)];
    let mut e = engine_with(&p, &slots);
    let (mut w, viz, _) = VizWriter::new();
    e.note_on(48, 1.0);
    for _ in 0..16 {
        run(&mut e, &p, 512);
        let fx = e.fx_telemetry();
        let gp = *e.patch();
        w.publish(&e, &params, &gp, SR, &fx);
    }
    let mut f = VizFrame::default();
    assert!(viz.read(&mut f));
    assert_eq!(f.live_len(), MOD_IDS.len() + 2, "osca_index is already a fixed-route entry");
    near(f.live01("rev_mix").unwrap(), info("rev_mix").norm(0.15) + 0.4, 1e-3, "global slot target");
    near(f.live01("filta_res").unwrap(), 0.15 + 0.25, 1e-3, "voice slot target");
    near(f.live01("osca_index").unwrap(), 40.0 / 127.0 + 0.2, 1e-3, "the fixed-route entry includes the slot");
    assert_eq!(f.live01("filtb_res"), None);
}

// ---- (6) CPU ---------------------------------------------------------------------------------

/// 32 distinct voice-scope targets, including ones that re-run envelope / unison / LFO setup.
fn heavy_slots() -> Vec<Slot> {
    let targets = [
        "osca_index", "oscb_index", "osca_shift", "oscb_shift", "osca_motion", "oscb_motion", "osca_vol", "oscb_vol",
        "osca_cent", "oscb_cent", "filta_freq", "filtb_freq", "filta_res", "filtb_res", "filta_kbd", "filtb_kbd",
        "fenv_amt", "fenv_dec", "osca_dec", "oscb_dec", "osca_detune", "oscb_detune", "osca_blend", "oscb_width",
        "shaper_amt", "sub_level", "porta", "moda_index", "modb_cutoff", "moda_rate", "modb_rate", "wheel_index",
    ];
    let sources = [Source::ModA, Source::ModB, Source::FilterEnv, Source::AmpEnvA, Source::Velocity, Source::Keytrack, Source::Random, Source::Macro1];
    targets.iter().enumerate().map(|(k, t)| Slot::new(sources[k % sources.len()], t, 0.05 + 0.01 * k as f32)).collect()
}

fn cpu_render(slots: &[Slot], secs: f64) -> f64 {
    let mut p = Patch::riddim_v1();
    p.poly = 8;
    p.osc[1].on = true;
    p.osc[0].unison = 3;
    p.shaper_on = true;
    p.routing = Routing::Series;
    p.macros[0] = 0.5;
    let mut e = engine_with(&p, slots);
    for n in 0..8 {
        e.note_on(29 + 5 * n, 0.5 + 0.06 * n as f32);
    }
    let t = std::time::Instant::now();
    let mut acc = 0f32;
    for i in 0..(secs * SR as f64) as usize {
        if i % 512 == 0 {
            e.set_patch(&p);
        }
        let (l, r) = e.next_frame();
        acc += l + r;
    }
    let dt = t.elapsed().as_secs_f64();
    assert_eq!(e.active_voices(), 8);
    assert!(acc.is_finite());
    dt
}

/// SPEC v2 Track 5 acceptance 6: 8 voices with 32 voice-scope slots cost at most 5 % of a core
/// over the matrix-empty engine at 48 kHz (best of three each; asserted in release builds).
#[test]
fn cpu_full_matrix_costs_under_five_percent_of_a_core() {
    let secs = 4.0;
    let slots = heavy_slots();
    assert_eq!(slots.len(), 32);
    assert!(slots.iter().all(|s| info(&s.target).voice));
    let (mut empty, mut full) = (f64::MAX, f64::MAX);
    for _ in 0..3 {
        empty = empty.min(cpu_render(&[], secs));
        full = full.min(cpu_render(&slots, secs));
    }
    let overhead = (full - empty) / secs;
    println!(
        "8 voices, {secs} s at 48 kHz: empty {:.1} % of a core, 32 voice slots {:.1} %, matrix overhead {:+.2} % of a core",
        100.0 * empty / secs,
        100.0 * full / secs,
        100.0 * overhead
    );
    if !cfg!(debug_assertions) {
        assert!(overhead <= 0.05, "matrix overhead {:.2} % of a core", 100.0 * overhead);
    }
}

// ---- (7) persistence -------------------------------------------------------------------------

fn sample_matrix() -> Vec<Slot> {
    vec![
        Slot::new(Source::Macro1, "filta_freq", 0.5),
        Slot { bipolar: true, ..Slot::new(Source::Macro2, "rev_mix", -0.25) },
        Slot { enabled: false, ..Slot::new(Source::BusCv3, "osca_detune", 0.125) },
        Slot::new(Source::PolyAftertouch, "shaper_amt", 1.0),
    ]
}

#[test]
fn preset_round_trips_matrix_and_macros() {
    let mut patch = Patch::default();
    patch.macros = [0.25, 0.5, 0.75, 1.0];
    let p = Preset { name: "M".into(), macros: vec!["Growl".into(), "Space".into()], matrix: sample_matrix(), patch, ..Preset::default() };
    let json = p.to_json();
    for s in [r#""source": "Macro 1""#, r#""target": "filta_freq""#, r#""label": "Growl""#, r#""macro3": 0.75"#] {
        assert!(json.contains(s), "{s} missing:\n{json}");
    }
    let (q, w) = Preset::from_json(&json).unwrap();
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(q, p);
    let m = q.matrix_state();
    assert_eq!(m.macros, ["Growl", "Space", "Macro 3", "Macro 4"]);
    assert_eq!(m.slots, sample_matrix());
}

#[test]
fn old_presets_load_with_an_empty_matrix_and_bad_slots_warn() {
    let (p, w) = Preset::from_json(r#"{"format_version": 1, "product": "GTS", "params": {"osca_index": 50}}"#).unwrap();
    assert!(w.is_empty() && p.matrix.is_empty() && p.patch.macros == [0.0; 4], "{w:?}");
    let json = r#"{"format_version": 1, "product": "GTS", "matrix": [
        {"source": "Macro 1", "target": "osca_index", "amount": 3.0, "bipolar": false, "enabled": true},
        {"source": "Nope", "target": "osca_index", "amount": 0.5, "bipolar": false, "enabled": true},
        {"source": "Macro 2", "target": "poly", "amount": 0.5, "bipolar": false, "enabled": true},
        {"source": "Macro 3", "target": "osca_shift"}]}"#;
    let (p, w) = Preset::from_json(json).unwrap();
    assert_eq!(p.matrix, vec![Slot::new(Source::Macro1, "osca_index", 1.0)]);
    assert_eq!(w.len(), 4, "{w:?}");
}

#[test]
fn state_round_trips_and_reaches_the_audio_snapshot() {
    let a = GtsParams::default();
    let m = Matrix { slots: sample_matrix(), macros: ["A".into(), "B".into(), "C".into(), "D".into()] };
    a.matrix.replace(m.clone());
    let state = a.serialize_fields();
    assert!(state.contains_key("matrix"));
    let b = GtsParams::default();
    let (mut seen, mut rec) = (u32::MAX, [0.0; ENC_LEN]);
    assert!(b.matrix.poll(&mut seen, &mut rec));
    assert_eq!(rec, Matrix::default().encode());
    assert!(!b.matrix.poll(&mut seen, &mut rec), "no change, no new record");
    b.deserialize_fields(&state);
    assert_eq!(b.matrix.get(), m);
    assert!(b.matrix.poll(&mut seen, &mut rec), "a state load publishes");
    assert_eq!(rec, m.encode());
}

/// A pre-matrix state (or an unparsable matrix) clears the instance's matrix and macros.
#[test]
fn old_state_clears_a_loaded_matrix_and_macros() {
    use nih_plug::prelude::{Plugin, PluginState};
    use nih_plug::wrapper::state::ParamValue;
    for field in [None, Some("{\"slots\":[{\"source\":\"Future\"}]}")] {
        let mut st = PluginState { version: String::new(), params: Default::default(), fields: Default::default() };
        if let Some(f) = field {
            st.fields.insert("matrix".into(), f.into());
        }
        st.params.insert("macro2".into(), ParamValue::F32(0.7));
        <gts::Gts as Plugin>::filter_state(&mut st);
        let b = GtsParams::default();
        b.matrix.replace(Matrix { slots: sample_matrix(), ..Default::default() });
        b.deserialize_fields(&st.fields);
        assert_eq!(b.matrix.get(), Matrix::default());
        assert!(matches!(st.params["macro1"], ParamValue::F32(v) if v == 0.0));
        assert!(matches!(st.params["macro2"], ParamValue::F32(v) if v == 0.7), "present params stay");
    }
}

#[test]
fn preset_load_sets_matrix_labels_and_macro_values() {
    let c = FakeCtl::new();
    let params = Arc::new(GtsParams::default());
    let table = ParamTable::new(params.clone());
    let mut patch = Patch::default();
    patch.macros = [0.5, 0.0, 0.0, 0.25];
    let p = Preset { macros: vec!["Growl".into()], matrix: sample_matrix(), patch, ..Preset::default() };
    apply_preset(&table, &c, Some(&p));
    assert_eq!(params.matrix.get(), p.matrix_state());
    near(c.get01("macro1"), 0.5, 1e-6, "macro1 value");
    near(c.get01("macro4"), 0.25, 1e-6, "macro4 value");
    apply_preset(&table, &c, None);
    assert_eq!(params.matrix.get(), Matrix::default());
}

// ---- target table, fixed routes, GUI API -----------------------------------------------------

#[test]
fn target_table_follows_the_spec_scopes() {
    #[rustfmt::skip]
    let voice = ["osca_index", "oscb_unison", "osca_cent", "sub_level", "filta_freq", "filtb_res", "filta_kbd", "fenv_amt",
        "fenv_att", "osca_att", "osca_detune", "shaper_amt", "porta", "moda_rate", "modb_cutoff", "vel_fenv"];
    for id in voice {
        assert!(info(id).target && info(id).voice, "{id} should be a voice-scope target");
    }
    #[rustfmt::skip]
    let global = ["dist_drive", "mb_depth", "eq_peak_gain", "cho_rate", "cho_voices", "dim_size", "dly_feedback", "rev_mix",
        "lim_gain", "volume", "spread", "mono_xover", "bend_range"];
    for id in global {
        assert!(info(id).target && !info(id).voice, "{id} should be a global target");
    }
    for id in ["macro1", "macro4", "cv1_amount", "cv2_target", "poly", "osca_table", "osca_on", "routing", "fenv_inv", "dly_mode"] {
        assert!(!info(id).target, "{id} must not be a target");
    }
    // Every FloatParam / IntParam (138) minus the 4 macros, 3 CV amounts and Polyphony.
    let n = params_info().iter().filter(|p| p.target).count();
    println!("{n} targets ({} voice scope)", params_info().iter().filter(|p| p.voice).count());
    assert_eq!(n, 130);
}

#[test]
fn fixed_routes_name_real_params() {
    for r in fixed_routes() {
        assert!(info(r.target).target, "{r:?}");
        assert!(param_index(r.route).is_some(), "{r:?}");
    }
    assert!(gts::matrix::fixed_route(Source::ModA, "osca_index").is_some_and(|r| r.route == "moda_index"));
    assert!(gts::matrix::fixed_route(Source::FilterEnv, "filtb_freq").is_some_and(|r| r.route == "fenv_amt"));
    assert!(gts::matrix::fixed_route(Source::Macro1, "osca_index").is_none());
}

#[test]
fn editor_api_adds_edits_and_removes() {
    let (shared, c) = (MatrixShared::default(), FakeCtl::new());
    let ed = MatrixEditor { shared: &shared, ctl: &c };
    assert_eq!(ed.add_or_edit(Source::Macro1, "filta_freq", 0.5), Ok(Edit::Slot(0)));
    ed.set_amount(0, 0.2);
    assert_eq!(ed.add_or_edit(Source::Macro1, "filta_freq", 0.5), Ok(Edit::Slot(0)), "same pair returns its slot");
    assert_eq!(ed.add_or_edit(Source::Macro2, "filta_freq", 0.3), Ok(Edit::Slot(1)), "another source adds");
    assert_eq!(shared.get().slots[0].amount, 0.2, "a re-drop keeps the amount");
    // A bypassed slot: the re-drop enables it and keeps its amount.
    ed.set_enabled(0, false);
    assert_eq!(ed.add_or_edit(Source::Macro1, "filta_freq", 0.5), Ok(Edit::Slot(0)));
    let s0 = shared.get().slots[0].clone();
    assert!(s0.enabled && s0.amount == 0.2, "{s0:?}");
    assert_eq!(ed.slots_for_target("filta_freq"), vec![0, 1]);
    assert_eq!(ed.targets_of_source(Source::Macro1), vec!["filta_freq".to_string()]);
    for bad in ["poly", "osca_table", "macro1", "cv1_amount", "nope"] {
        assert_eq!(ed.add_or_edit(Source::Macro1, bad, 0.5), Err(EditError::NotATarget), "{bad}");
    }

    // Fixed routes edit their param through Ctl: Mod A -> Osc A Index is Mod A Index (Target Both).
    assert_eq!(ed.add_or_edit(Source::ModA, "osca_index", 0.4), Ok(Edit::Fixed("moda_index")));
    near(c.get01("moda_index"), 0.7, 1e-6, "moda_index normalized");
    assert_eq!(ed.add_or_edit(Source::ModA, "osca_index", -0.9), Ok(Edit::Fixed("moda_index")), "a live route returns its handle");
    near(c.get01("moda_index"), 0.7, 1e-6, "a live non-zero route keeps its amount");
    assert_eq!(ed.fixed_routes_for_target("osca_index").len(), 1);
    assert!(ed.targets_of_source(Source::ModA).contains(&"osca_index".to_string()));
    // Mod A Target = B: the route does not reach Osc A Index, so the drop makes a slot.
    c.set("moda_target", 0.5);
    assert_eq!(ed.add_or_edit(Source::ModA, "osca_index", 0.4), Ok(Edit::Slot(2)));
    // Bus CV 1 is a fixed route only to its selected target.
    assert_eq!(ed.add_or_edit(Source::BusCv1, "oscb_shift", 0.5), Ok(Edit::Slot(3)));
    c.set("cv1_target", 2.0 / 8.0); // Osc A Shift
    assert_eq!(ed.add_or_edit(Source::BusCv1, "osca_shift", -0.5), Ok(Edit::Fixed("cv1_amount")));
    near(c.get01("cv1_amount"), 0.25, 1e-6, "cv1_amount normalized");

    ed.set_bipolar(0, true);
    ed.set_enabled(1, false);
    ed.set_amount(0, 7.0);
    let m = shared.get();
    assert!(m.slots[0].bipolar && !m.slots[1].enabled && m.slots[0].amount == 1.0);
    ed.set_macro_label(2, "Space");
    assert_eq!(shared.get().macros[2], "Space");

    // Remove all on Osc A Index: its slot goes and its live Mod A route is zeroed.
    c.set("moda_target", 1.0);
    assert_eq!(ed.remove_all_for_target("osca_index"), 2, "one slot and the Mod A route");
    near(c.get01("moda_index"), 0.5, 1e-6, "route zeroed");
    assert!(ed.slots_for_target("osca_index").is_empty());
    ed.remove(0);
    assert_eq!(shared.get().slots.len(), 2);

    // Full.
    for t in gts::matrix::target_ids().take(40) {
        let _ = ed.add_or_edit(Source::Macro3, t, 0.1);
    }
    assert_eq!(shared.get().slots.len(), MAX_SLOTS);
    assert_eq!(ed.add_or_edit(Source::Macro4, "osca_index", 0.1), Err(EditError::Full));
}

// ---- review fixes ----------------------------------------------------------------------------

/// Removing the last slot on a target glides its offset out over ~5 ms (global and voice scope),
/// then the target is dropped and the engine is back on the plain patch.
#[test]
fn removed_targets_glide_out() {
    let mut p = Patch::default();
    p.macros[0] = 1.0;
    p.filt[0].kbd = 0.0;
    p.filt[0].env = false;
    p.filt[0].freq = 1000.0;
    let mut e = engine_with(&p, &[Slot::new(Source::Macro1, "rev_mix", 0.5), Slot::new(Source::Macro1, "filta_freq", 0.5)]);
    e.note_on(48, 1.0);
    run(&mut e, &p, 4096);
    let i = voice_of(&e, 48);
    let (rev0, freq0) = (info("rev_mix").norm(p.fx.rev.mix), info("filta_freq").norm(1000.0));
    let off = |e: &Engine| (info("rev_mix").norm(e.patch().fx.rev.mix) - rev0, info("filta_freq").norm(e.voice_patch(i).filt[0].freq) - freq0);
    near(off(&e).0, 0.5, 1e-3, "rev on");
    e.set_matrix(&enc(&[]));
    assert!(e.matrix_on(), "still gliding out");
    run(&mut e, &p, 64);
    let (g, v) = off(&e);
    assert!(g > 0.3 && v > 0.3, "no jump to 0 on removal: {g} {v}");
    run(&mut e, &p, 4096);
    assert!(!e.matrix_on(), "settled targets are dropped");
    assert_eq!(e.patch().fx.rev.mix, p.fx.rev.mix);
    assert_eq!(e.voice_patch(i).filt[0].freq, 1000.0);
}

/// A global target fed by a per-voice source holds the last sounding voice's value while only
/// tails ring, instead of falling back to 0.
#[test]
fn global_target_holds_through_the_tail() {
    let p = Patch::default();
    let mut e = engine_with(&p, &[Slot::new(Source::Velocity, "rev_mix", 0.5)]);
    e.note_on(48, 1.0);
    run(&mut e, &p, 4800);
    let on = e.patch().fx.rev.mix;
    e.handle_event(&NoteEvent::Choke { timing: 0, voice_id: None, channel: 0, note: 48 });
    run(&mut e, &p, 4800);
    assert!(e.newest().is_none());
    assert_eq!(e.patch().fx.rev.mix, on);
}

/// A new note's matrix snap reads the LFO at its reset phase (one-shot Decay opens at +1).
#[test]
fn new_note_snap_reads_the_reset_lfo() {
    let mut p = Patch::default();
    p.moda.curve = Curve::Decay;
    p.moda.oneshot = true;
    p.moda.sync = false;
    p.moda.rate = 1.0;
    p.filt[0].kbd = 0.0;
    p.filt[0].env = false;
    p.filt[0].freq = 1000.0;
    let mut e = engine_with(&p, &[Slot::new(Source::ModA, "filta_freq", 0.5)]);
    e.note_on(48, 1.0);
    let i = voice_of(&e, 48);
    near(e.voice_patch(i).filt[0].freq, moved("filta_freq", 1000.0, 0.5), 1.0, "opens on the peak");
}

// ---- host dirty flag and undo (matrix_rev) ---------------------------------------------------

/// A host stand-in for the editor's context: records the gestures and each rev set.
#[derive(Default)]
struct FakeHost {
    log: Mutex<Vec<String>>,
}

impl FakeHost {
    fn log(&self) -> Vec<String> {
        self.log.lock().unwrap().clone()
    }
}

impl nih_plug::prelude::GuiContext for FakeHost {
    fn plugin_api(&self) -> nih_plug::prelude::PluginApi {
        nih_plug::prelude::PluginApi::Vst3
    }
    fn request_resize(&self) -> bool {
        false
    }
    unsafe fn raw_begin_set_parameter(&self, _: nih_plug::prelude::ParamPtr) {
        self.log.lock().unwrap().push("begin".into());
    }
    unsafe fn raw_set_parameter_normalized(&self, _: nih_plug::prelude::ParamPtr, v: f32) {
        self.log.lock().unwrap().push(format!("{}", (v * gts::matrix::REV_MAX as f32).round()));
    }
    unsafe fn raw_end_set_parameter(&self, _: nih_plug::prelude::ParamPtr) {
        self.log.lock().unwrap().push("end".into());
    }
    fn get_state(&self) -> nih_plug::prelude::PluginState {
        unimplemented!()
    }
    fn set_state(&self, _: nih_plug::prelude::PluginState) {}
}

fn attached() -> (MatrixShared, Arc<FakeHost>) {
    let (shared, host) = (MatrixShared::default(), Arc::new(FakeHost::default()));
    shared.attach(Some(host.clone()));
    (shared, host)
}

/// The host sees matrix_rev, not automatable and not read-only; GtsParams (editor, help and
/// preset tables) does not.
#[test]
fn matrix_rev_is_a_host_param_outside_the_sound_params() {
    use nih_plug::prelude::{ParamFlags, Plugin};
    let plugin = gts::Gts::default();
    let params = plugin.params();
    let map = params.param_map();
    let (_, ptr, _) = map.iter().find(|(id, _, _)| id == "matrix_rev").expect("matrix_rev reaches the host");
    // SAFETY: `params` outlives `ptr`.
    let flags = unsafe { ptr.flags() };
    assert!(flags.contains(ParamFlags::NON_AUTOMATABLE) && !flags.contains(ParamFlags::HIDDEN), "{flags:?}");
    let sound = GtsParams::default().param_map().into_iter().map(|(id, _, _)| id).collect::<Vec<_>>();
    assert_eq!(map.len(), sound.len() + 1);
    assert!(!sound.iter().any(|id| id == "matrix_rev"));
}

/// Every GUI edit moves matrix_rev as one gesture; a host undo (rev back) restores the matrix
/// from the background task; redo returns; an unknown rev changes nothing.
#[test]
fn gui_edits_bump_matrix_rev_and_host_undo_restores() {
    let (shared, host) = attached();
    let c = FakeCtl::new();
    let ed = MatrixEditor { shared: &shared, ctl: &c };
    let empty = shared.get();
    ed.add_or_edit(Source::Macro1, "filta_freq", 0.5).unwrap();
    assert_eq!(host.log(), ["begin", "1", "end"]);
    let one = shared.get();
    ed.set_macro_label(0, "Growl");
    let two = shared.get();
    assert_eq!(host.log()[3..], ["begin", "2", "end"]);
    // Process before the host reports the GUI's value, then after: nothing to restore.
    assert_eq!(shared.host_moved(1), None, "host not caught up yet");
    assert_eq!(shared.host_moved(2), None, "the GUI's own rev");
    // Host undo: rev 1, then rev 0.
    assert_eq!(shared.host_moved(1), Some(1));
    assert!(shared.restore(1));
    assert_eq!(shared.get(), one);
    assert_eq!(shared.host_moved(0), Some(0));
    assert!(shared.restore(0));
    assert_eq!(shared.get(), empty);
    // Redo.
    assert_eq!(shared.host_moved(2), Some(2));
    assert!(shared.restore(2));
    assert_eq!(shared.get(), two);
    // Unknown rev: unchanged.
    assert_eq!(shared.host_moved(999), Some(999));
    assert!(!shared.restore(999));
    assert_eq!(shared.get(), two);
    // An edit after an undo moves from the host's rev to a fresh one; its undo is the undone state.
    assert_eq!(shared.host_moved(1), Some(1));
    assert!(shared.restore(1));
    ed.remove(0);
    assert_eq!(host.log()[host.log().len() - 3..], ["begin", "3", "end"]);
    assert_eq!(shared.host_moved(1), None, "host not caught up");
    assert_eq!(shared.host_moved(3), None);
    assert!(shared.get().slots.is_empty());
    assert_eq!(shared.host_moved(1), Some(1));
    assert!(shared.restore(1));
    assert_eq!(shared.get(), one, "undo of the remove");
}

/// A ring drag (amount edits of one slot in quick succession) is one host step; no change is no
/// step; the history keeps the last HISTORY revs; without an editor, edits make no step.
#[test]
fn drags_coalesce_and_history_is_bounded() {
    let (shared, host) = attached();
    let c = FakeCtl::new();
    let ed = MatrixEditor { shared: &shared, ctl: &c };
    ed.add_or_edit(Source::Macro1, "filta_freq", 0.5).unwrap();
    for k in 1..=20 {
        ed.set_amount(0, 0.5 + k as f32 * 0.01);
    }
    let end = shared.get().slots[0].amount;
    ed.set_amount(0, end); // unchanged
    assert_eq!(host.log(), ["begin", "1", "end", "begin", "2", "end"], "add + one drag step");
    assert_eq!(shared.host_moved(2), None);
    assert_eq!(shared.host_moved(1), Some(1));
    assert!(shared.restore(1));
    assert_eq!(shared.get().slots[0].amount, 0.5, "undo returns to before the drag");
    assert_eq!(shared.host_moved(2), Some(2));
    assert!(shared.restore(2));
    assert_eq!(shared.get().slots[0].amount, end, "redo returns to the drag's end");

    // Bounded: after HISTORY + 10 more edits the oldest revs are gone, the newest restore.
    let n = gts::matrix::HISTORY + 10;
    for k in 0..n {
        ed.set_macro_label(1, &format!("L{k}"));
    }
    assert!(!shared.restore(1), "rev 1 fell out of the history");
    let last = shared.get();
    let newest = 2 + n as i32;
    assert_eq!(host.log()[host.log().len() - 2], newest.to_string());
    assert!(shared.restore(newest));
    assert_eq!(shared.get(), last);

    // No editor attached: edits apply, no gesture.
    shared.attach(None);
    let before = host.log().len();
    ed.set_macro_label(1, "Off");
    assert_eq!(shared.get().macros[1], "Off");
    assert_eq!(host.log().len(), before);
}

/// A host state load drops the session history and adopts the loaded rev: no restore follows,
/// and the next edit's undo returns to the loaded matrix.
#[test]
fn state_load_adopts_the_loaded_rev() {
    let (shared, _host) = attached();
    let c = FakeCtl::new();
    let ed = MatrixEditor { shared: &shared, ctl: &c };
    ed.add_or_edit(Source::Macro1, "filta_freq", 0.5).unwrap();
    assert_eq!(shared.host_moved(1), None);
    let loaded = Matrix { slots: sample_matrix(), ..Default::default() };
    nih_plug::params::persist::PersistentField::set(&shared, loaded.clone());
    assert_eq!(shared.get(), loaded);
    // The loaded state carried rev 0 (the param keeps its value here): adopted, not restored.
    assert_eq!(shared.host_moved(0), None);
    assert!(!shared.restore(1), "history dropped");
    ed.remove(0);
    assert_eq!(shared.host_moved(0), None, "host not caught up");
    assert_eq!(shared.host_moved(2), None);
    assert_eq!(shared.host_moved(0), Some(0));
    assert!(shared.restore(0));
    assert_eq!(shared.get(), loaded);
}

/// A state whose matrix has one slot from a newer version keeps the other slots and the labels.
#[test]
fn state_with_an_unknown_source_keeps_the_rest() {
    use nih_plug::prelude::{Plugin, PluginState};
    let m = Matrix { slots: sample_matrix(), macros: ["A".into(), "B".into(), "C".into(), "D".into()] };
    let mut v = serde_json::to_value(&m).unwrap();
    let future = serde_json::json!({"source": "Future LFO", "target": "filta_freq", "amount": 0.3, "bipolar": true, "enabled": true});
    v["slots"].as_array_mut().unwrap().insert(1, future);
    let mut st = PluginState { version: String::new(), params: Default::default(), fields: Default::default() };
    st.fields.insert("matrix".into(), v.to_string());
    <gts::Gts as Plugin>::filter_state(&mut st);
    let b = GtsParams::default();
    b.deserialize_fields(&st.fields);
    assert_eq!(b.matrix.get(), m);
}
