//! RR-BUS device tests (SPEC "RR-BUS — inter-device modulation", Verification: Device tests).
//! Every test opens its own uniquely named bus, so nothing touches a running host's bus.
use gts::bus::{apply_cv, DeviceBus};
use gts::dsp::{Engine, Patch};
use gts::params::{CvTarget, GtsParams};
use nih_plug::prelude::*;
use rr_bus::{Bus, Kind};

const SR: f32 = 48000.0;
const BLOCK: usize = 64;

fn bus_name(test: &str) -> String {
    format!("Local\\RiddimRackBus.gts-test.{}.{test}", std::process::id())
}

/// Default params with CV 1 -> `target` at `amount`, bound to `source`.
fn params(target: CvTarget, amount: f32, source: &str) -> GtsParams {
    let mut p = GtsParams::default();
    p.cv1_target = EnumParam::new("CV 1 Target", target);
    p.cv1_amount = FloatParam::new("CV 1 Amount", amount, FloatRange::Linear { min: -1.0, max: 1.0 });
    p.cv_sources.write().unwrap()[0] = source.into();
    p
}

fn freq_n(p: &GtsParams, patch: &Patch) -> f32 {
    p.filta.freq.preview_normalized(patch.filt[0].freq)
}

#[test]
fn cv_in_tracks_a_published_ramp_within_one_block_plus_smoothing() {
    let name = bus_name("ramp");
    let bus = Bus::open_named(&name);
    let publisher = bus.register("TEST");
    let ramp = publisher.publish("Ramp", Kind::Unipolar);
    let p = params(CvTarget::FilterAFreq, 1.0, &format!("{} · Ramp", publisher.label()));
    let mut dev = DeviceBus::default();
    dev.connect(&p, &name);

    let base_n = p.filta.freq.modulated_normalized_value();
    let host_value = p.filta.freq.value();

    // A step: the target moves within the first block and settles within 5 time constants.
    ramp.set(0.25);
    let mut patch = Patch::from_params(&p);
    apply_cv(&mut patch, &p, dev.process_block([0.0; 5], BLOCK, SR));
    let first = freq_n(&p, &patch) - base_n;
    assert!(first > 0.01 && first < 0.25, "after one block: {first}");
    for _ in 0..(0.025 * SR) as usize / BLOCK {
        patch = Patch::from_params(&p);
        apply_cv(&mut patch, &p, dev.process_block([0.0; 5], BLOCK, SR));
    }
    let settled = freq_n(&p, &patch) - base_n;
    assert!((settled - 0.25).abs() < 0.003, "settled: {settled}");

    // A ramp (0.25 -> 0.6 over 0.35 s): lags by at most one block + 5 ms once running.
    let slope = 1.0; // per second
    let lag = slope * (BLOCK as f32 / SR + 0.005);
    for b in 0..(0.35 * SR) as usize / BLOCK {
        let v = 0.25 + slope * (b * BLOCK) as f32 / SR;
        ramp.set(v);
        let got = dev.process_block([0.0; 5], BLOCK, SR)[0];
        if b > 20 {
            assert!(got <= v + 1e-3 && v - got <= lag + 1e-3, "block {b}: published {v}, applied {got}");
        }
    }
    // Modulation, not automation: the host value never moved.
    assert_eq!(p.filta.freq.value(), host_value);
}

#[test]
fn unbound_and_lost_sources_apply_zero() {
    let name = bus_name("lost");
    let bus = Bus::open_named(&name);

    // Unbound: nothing persisted, target set, amount full.
    let p = params(CvTarget::Volume, 1.0, "");
    let mut dev = DeviceBus::default();
    dev.connect(&p, &name);
    let cv = dev.process_block([0.0; 5], BLOCK, SR);
    assert_eq!(cv, [0.0; 3]);
    let mut patch = Patch::from_params(&p);
    apply_cv(&mut patch, &p, cv);
    assert_eq!(patch, Patch::from_params(&p));

    // Bound to a name nobody publishes.
    let p = params(CvTarget::Volume, 1.0, "NOBODY 9 · Mod A");
    let mut dev = DeviceBus::default();
    dev.connect(&p, &name);
    assert_eq!(dev.process_block([0.0; 5], BLOCK, SR), [0.0; 3]);

    // Bound, then the publisher goes away: the offset decays to zero.
    let publisher = bus.register("TEST");
    let src = publisher.publish("Gate", Kind::Gate);
    let p = params(CvTarget::FilterAFreq, -1.0, &format!("{} · Gate", publisher.label()));
    let mut dev = DeviceBus::default();
    dev.connect(&p, &name);
    src.set(1.0);
    for _ in 0..40 {
        dev.process_block([0.0; 5], BLOCK, SR);
    }
    assert!(dev.process_block([0.0; 5], BLOCK, SR)[0] > 0.99);
    drop(src);
    drop(publisher);
    let mut cv = [1.0; 3];
    for _ in 0..40 {
        cv = dev.process_block([0.0; 5], BLOCK, SR);
    }
    assert!(cv[0].abs() < 1e-3, "lost source still applies {}", cv[0]);
    assert_eq!(dev.link.activity(0), 0.0);
}

#[test]
fn gui_source_swap_reaches_the_audio_side() {
    let name = bus_name("swap");
    let bus = Bus::open_named(&name);
    let a = bus.register("TEST");
    let (sa, sb) = (a.publish("A", Kind::Bipolar), a.publish("B", Kind::Bipolar));
    sa.set(-0.5);
    sb.set(0.75);
    let p = params(CvTarget::OscAIndex, 1.0, "");
    let mut dev = DeviceBus::default();
    dev.connect(&p, &name);
    let run = |dev: &mut DeviceBus| (0..40).map(|_| dev.process_block([0.0; 5], BLOCK, SR)[1]).last().unwrap();
    dev.link.set_source(1, &format!("{} · A", a.label()));
    assert!((run(&mut dev) + 0.5).abs() < 1e-3);
    assert!((dev.link.activity(1) - 0.5).abs() < 1e-6);
    dev.link.set_source(1, &format!("{} · B", a.label()));
    assert!((run(&mut dev) - 0.75).abs() < 1e-3);
    dev.link.set_source(1, "");
    assert!(run(&mut dev).abs() < 1e-3);
}

#[test]
fn device_registers_and_publishes_its_signals() {
    let name = bus_name("publish");
    let bus = Bus::open_named(&name);
    let p = GtsParams::default();
    let mut dev = DeviceBus::default();
    dev.connect(&p, &name);
    let label = p.instance_label.read().unwrap().clone();
    assert!(label.starts_with("GTS "), "default label persisted: {label:?}");
    let listed: Vec<String> = bus.list().iter().map(|s| s.persisted()).collect();
    for n in ["Mod A", "Mod B", "Gate", "Filter Env", "Amp Env"] {
        assert!(listed.contains(&format!("{label} · {n}")), "{n} missing from {listed:?}");
    }
    assert!(!dev.needs_register());

    // Self-patching: read our own Gate.
    let p2 = params(CvTarget::Volume, 1.0, &format!("{label} · Gate"));
    *p2.instance_label.write().unwrap() = label.clone();
    dev.connect(&p2, &name);
    dev.process_block([0.0, 0.0, 1.0, 0.0, 0.0], BLOCK, SR);
    assert!(dev.process_block([0.0, 0.0, 1.0, 0.0, 0.0], BLOCK, SR)[0] > 0.0);

    // A persisted label is applied on load.
    let p3 = GtsParams::default();
    *p3.instance_label.write().unwrap() = "Lead bass".into();
    let mut dev3 = DeviceBus::default();
    dev3.connect(&p3, &name);
    assert!(bus.list().iter().any(|s| s.persisted() == "Lead bass · Mod A"));
}

#[test]
fn engine_signals_follow_the_newest_voice() {
    let mut e = Engine::new();
    e.initialize(SR);
    e.set_patch(&Patch::default());
    assert_eq!(e.bus_signals(), [0.0; 5]);
    e.note_on(40, 1.0);
    for _ in 0..2400 {
        e.next_frame();
    }
    let s = e.bus_signals();
    assert_eq!(s[2], 1.0, "gate");
    assert!(s[4] > 0.5 && s[3] > 0.0, "envelopes: {s:?}");
    assert!((-1.0..=1.0).contains(&s[0]) && (-1.0..=1.0).contains(&s[1]));
    e.note_off(40);
    for _ in 0..(SR as usize) {
        e.next_frame();
    }
    assert_eq!(e.bus_signals(), [0.0; 5]);
}

#[test]
fn persisted_fields_round_trip() {
    let p = GtsParams::default();
    *p.cv_sources.write().unwrap() = ["GTS 2 · Mod A".into(), String::new(), "VSS 1 · Step Gate".into()];
    *p.instance_label.write().unwrap() = "Lead bass".into();
    let saved = p.serialize_fields();
    let q = GtsParams::default();
    q.deserialize_fields(&saved);
    assert_eq!(*q.cv_sources.read().unwrap(), *p.cv_sources.read().unwrap());
    assert_eq!(*q.instance_label.read().unwrap(), "Lead bass");
}
