//! SPEC v2 Track 7 (G9) chain integration: phaser and flanger wired in, the reorderable order
//! (persisted, preset "fx_order", click-free reorder, latency independent of order), and the new
//! continuous params as global matrix targets.
use gts::dsp::fx::{FxChain, FxOrder, FxPatch, MODULE_NAMES};
use gts::dsp::Patch;
use gts::params::GtsParams;
use gts::preset::Preset;
use nih_plug::prelude::Params;

const SR: f32 = 48000.0;

fn chain(p: &FxPatch) -> FxChain {
    let mut c = FxChain::new();
    c.set_sample_rate(SR);
    c.set(p, 140.0, None);
    c.reset();
    c
}

/// Distortion, EQ, phaser, flanger and delay on: an order change is audible.
fn patch() -> FxPatch {
    let mut p = Patch::default().fx;
    p.dist.on = true;
    p.dist.drive_db = 18.0;
    p.eq.on = true;
    p.eq.peak_gain = 9.0;
    p.phs.on = true;
    p.flg.on = true;
    p.dly.on = true;
    p.dly.sync = false;
    p.dly.time_ms = 40.0;
    p
}

fn tone(i: usize) -> (f32, f32) {
    let t = i as f32 / SR;
    let x = 0.3 * (std::f32::consts::TAU * 110.0 * t).sin() + 0.1 * (std::f32::consts::TAU * 330.0 * t).sin();
    (x, x)
}

/// Runs `n` samples from sample `from`, returning the largest sample-to-sample step.
fn run(c: &mut FxChain, from: usize, n: usize, last: &mut (f32, f32)) -> f32 {
    let mut step = 0f32;
    for i in from..from + n {
        let (mut l, mut r) = tone(i);
        c.process(&mut l, &mut r, 1.0);
        step = step.max((l - last.0).abs()).max((r - last.1).abs());
        *last = (l, r);
    }
    step
}

#[test]
fn order_names_and_validation() {
    let d = FxOrder::default();
    assert_eq!(d.names(), MODULE_NAMES);
    assert_eq!(MODULE_NAMES, ["Distortion", "Multiband", "EQ", "Phaser", "Chorus", "Flanger", "Dimension", "Delay", "Reverb"]);
    assert_eq!(FxOrder::decode(d.encode()), Some(d));
    let rev = FxOrder::new([8, 7, 6, 5, 4, 3, 2, 1, 0]).unwrap();
    assert_eq!(FxOrder::decode(rev.encode()), Some(rev));
    assert_eq!(FxOrder::from_names(&rev.names()), Some(rev));
    let mut with_lim: Vec<&str> = rev.names().to_vec();
    with_lim.push("Limiter");
    assert_eq!(FxOrder::from_names(&with_lim), Some(rev));
    assert_eq!(FxOrder::new([0, 0, 2, 3, 4, 5, 6, 7, 8]), None);
    assert_eq!(FxOrder::new([0, 1, 2, 3, 4, 5, 6, 7, 9]), None);
    assert_eq!(FxOrder::from_names(&["Distortion"]), None);
    assert!(serde_json::from_str::<FxOrder>(r#"["Reverb","Reverb"]"#).is_err());
    assert_eq!(serde_json::to_string(&d).unwrap(), serde_json::to_string(&MODULE_NAMES).unwrap());
}

#[test]
fn order_round_trips_through_presets_and_state() {
    let o = FxOrder::new([8, 0, 1, 2, 3, 4, 5, 6, 7]).unwrap();
    let p = Preset { fx_order: o, ..Preset::default() };
    let json = p.to_json();
    assert!(json.contains("\"fx_order\""));
    let (q, w) = Preset::from_json(&json).unwrap();
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(q.fx_order, o);
    // Absent = default, and the default is not written.
    assert!(!Preset::default().to_json().contains("fx_order"));
    assert_eq!(Preset::from_json(&Preset::default().to_json()).unwrap().0.fx_order, FxOrder::default());
    // A bad list loads the default with a warning.
    let bad = json.replacen("\"Reverb\"", "\"Wah\"", 1);
    let (q, w) = Preset::from_json(&bad).unwrap();
    assert_eq!(q.fx_order, FxOrder::default());
    assert!(w.iter().any(|w| w.starts_with("fx_order")), "{w:?}");

    let params = GtsParams::default();
    assert_eq!(params.fx_order.get(), FxOrder::default());
    params.fx_order.replace(o);
    let state = params.serialize_fields();
    assert_eq!(state.get("fx_order").map(String::as_str), Some(serde_json::to_string(&o).unwrap().as_str()));
    let other = GtsParams::default();
    other.deserialize_fields(&state);
    assert_eq!(other.fx_order.get(), o);
}

/// A state without "fx_order" (pre-Track 7) or with an unreadable one restores the default order.
#[test]
fn old_states_load_the_default_order() {
    use nih_plug::prelude::{Plugin, PluginState};
    for v in [None, Some("\"nope\"")] {
        let mut st = PluginState { version: String::new(), params: Default::default(), fields: Default::default() };
        if let Some(v) = v {
            st.fields.insert("fx_order".into(), v.into());
        }
        <gts::Gts as Plugin>::filter_state(&mut st);
        let f: FxOrder = serde_json::from_str(&st.fields["fx_order"]).unwrap();
        assert_eq!(f, FxOrder::default());
    }
}

/// Reordering while a tone plays: the largest sample step in the 2048 samples after the order
/// change stays within 1.5x the steady-state step (SPEC acceptance 9; steady = the largest step of
/// the old order and of 5 s, one flanger LFO cycle, of the new one), the order really changes the
/// sound, and the latency is the same in every order.
#[test]
fn reorder_is_click_free_and_latency_is_order_independent() {
    let p = patch();
    let mut c = chain(&p);
    let lat = c.latency();
    let mut last = (0.0, 0.0);
    run(&mut c, 0, 9600, &mut last); // settle
    let steady = run(&mut c, 9600, 4800, &mut last);
    let mut worst = 0f32;
    let orders = [[8, 7, 6, 5, 4, 3, 2, 1, 0], [2, 3, 0, 5, 7, 1, 4, 6, 8], [0, 1, 2, 3, 4, 5, 6, 7, 8]];
    let mut at = 14400;
    // Each order twice, from different tone phases.
    for (k, &o) in orders.iter().cycle().take(6).enumerate() {
        run(&mut c, at, 97 * k, &mut last);
        at += 97 * k;
        let o = FxOrder::new(o).unwrap();
        c.set(&p, 140.0, None);
        c.set_order(o);
        // Blocks of 256, like a host.
        let mut step = 0f32;
        for _ in 0..8 {
            c.set(&p, 140.0, None);
            c.set_order(o);
            step = step.max(run(&mut c, at, 256, &mut last));
            at += 256;
        }
        assert_eq!(c.order(), o, "the order swapped within 2048 samples");
        assert_eq!(c.latency(), lat);
        // Steady state in the new order.
        let steady_new = run(&mut c, at, 240_000, &mut last);
        at += 240_000;
        let s = steady.max(steady_new);
        println!("order {:?}: crossfade step {step:.4}, steady {s:.4} ({:.2}x)", o.names(), step / s);
        worst = worst.max(step / s);
        assert!(step <= 1.5 * s, "reorder to {:?} clicks: step {step} vs steady {s}", o.names());
    }
    println!("worst crossfade step {worst:.2}x steady");

    // Same input, two orders, different output (distortion before vs after the EQ boost).
    let render = |o: FxOrder| {
        let mut c = chain(&p);
        c.set_order(o);
        c.set(&p, 140.0, None);
        c.reset();
        let mut e = 0f64;
        let mut out = vec![];
        for i in 0..9600 {
            let (mut l, mut r) = tone(i);
            c.process(&mut l, &mut r, 1.0);
            out.push(l);
            e += (l as f64).powi(2);
        }
        (out, e)
    };
    let (a, _) = render(FxOrder::default());
    let (b, _) = render(FxOrder::new([2, 1, 0, 3, 4, 5, 6, 7, 8]).unwrap());
    let diff: f64 = a.iter().zip(&b).map(|(x, y)| ((x - y) as f64).powi(2)).sum();
    assert!(diff > 1.0, "EQ before distortion sounds different: {diff}");
}

/// Bypassed (every module off), an order change applies at once and stays sample-exact.
#[test]
fn bypassed_chain_reorders_at_once() {
    let p = Patch::default().fx;
    let mut c = chain(&p);
    let o = FxOrder::new([8, 7, 6, 5, 4, 3, 2, 1, 0]).unwrap();
    c.set_order(o);
    assert_eq!(c.order(), o);
    assert!(c.is_bypassed());
    let lat = c.latency();
    let xs: Vec<(f32, f32)> = (0..2000).map(tone).collect();
    for (i, &(x, y)) in xs.iter().enumerate() {
        let (mut l, mut r) = (x, y);
        c.process(&mut l, &mut r, 1.0);
        if i >= lat {
            assert_eq!(l.to_bits(), xs[i - lat].0.to_bits());
            assert_eq!(r.to_bits(), xs[i - lat].1.to_bits());
        }
    }
}

/// The phaser and the flanger are in the chain: switching each on changes the output.
#[test]
fn phaser_and_flanger_are_wired() {
    let base = Patch::default().fx;
    let render = |p: &FxPatch| {
        let mut c = chain(p);
        (0..9600)
            .map(|i| {
                let (mut l, mut r) = tone(i);
                c.process(&mut l, &mut r, 1.0);
                (l, r)
            })
            .collect::<Vec<_>>()
    };
    let dry = render(&base);
    for (name, p) in [
        ("phaser", FxPatch { phs: gts::dsp::fx::phaser::PhaserParams { on: true, ..base.phs }, ..base }),
        ("flanger", FxPatch { flg: gts::dsp::fx::flanger::FlangerParams { on: true, ..base.flg }, ..base }),
    ] {
        let wet = render(&p);
        let d: f32 = dry.iter().zip(&wet).map(|(a, b)| (a.0 - b.0).abs() + (a.1 - b.1).abs()).sum();
        assert!(d > 10.0, "{name} on changes nothing ({d})");
        assert!(wet.iter().all(|s| s.0.is_finite() && s.1.is_finite()));
    }
    // Params reach the patch.
    let params = GtsParams::default();
    let fx = FxPatch::from_params(&params);
    assert_eq!(fx.phs.sync_beats, 4.0);
    assert_eq!(fx.flg.sync_beats, 4.0);
}

/// Every continuous phaser / flanger param is a global matrix target with the FX flag.
#[test]
fn phaser_and_flanger_params_are_global_targets() {
    use gts::matrix::{is_target, param_index, params_info};
    let cont = [
        "phs_stages", "phs_rate", "phs_depth", "phs_feedback", "phs_centre", "phs_spread", "phs_mix", "flg_rate",
        "flg_depth", "flg_delay", "flg_feedback", "flg_spread", "flg_mix",
    ];
    for id in cont {
        assert!(is_target(id), "{id} is not a target");
        assert!(!params_info()[param_index(id).unwrap()].voice, "{id} must be global");
    }
    for id in ["phs_on", "phs_sync", "phs_div", "flg_on", "flg_sync", "flg_div"] {
        assert!(!is_target(id), "{id} must not be a target");
    }
}
