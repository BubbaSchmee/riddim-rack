//! The parameter ids are a stable contract (SPEC.md "Parameters").
use gts::params::GtsParams;
use nih_plug::prelude::Params;

fn spec_ids() -> Vec<String> {
    let mut v: Vec<String> = vec![];
    for o in ["osca", "oscb"] {
        for s in ["on", "table", "oct", "semi", "cent", "index", "motion", "shift", "vol", "att", "dec", "sus", "rel",
            // v2: envelope curve, unison
            "curve", "unison", "detune", "det_curve", "blend", "width", "rand_phase", "phase_reset"] {
            v.push(format!("{o}_{s}"));
        }
    }
    for m in ["moda", "modb"] {
        for s in ["curve", "rate", "sync", "div", "oneshot", "target"] {
            v.push(format!("{m}_{s}"));
        }
    }
    for s in ["moda_pitch", "moda_index", "moda_shift", "modb_motion", "modb_vol", "modb_cutoff", "modb_moda",
        "shaper_on", "shaper_mode", "shaper_amt"] {
        v.push(s.into());
    }
    for f in ["filta", "filtb"] {
        for s in ["type", "freq", "res", "kbd", "env"] {
            v.push(format!("{f}_{s}"));
        }
    }
    for s in ["att", "dec", "sus", "rel", "amt", "inv", "curve"] {
        v.push(format!("fenv_{s}"));
    }
    for s in ["routing", "poly", "legato", "porta", "bend_range", "wheel_index", "wheel_shift", "wheel_cutoff",
        "wheel_mod", "vel_vol_a", "vel_vol_b", "vel_fenv", "vel_att", "vel_shift", "vel_mod", "volume", "spread",
        // v2: sub oscillator, mono crossover
        "sub_on", "sub_level", "sub_oct", "mono_xover"] {
        v.push(s.into());
    }
    // SPEC v2 Track 7 sound: warp, sub shape / phase / drive, noise layer
    for o in ["osca", "oscb"] {
        v.push(format!("{o}_warp_mode"));
        v.push(format!("{o}_warp_amt"));
    }
    for s in ["sub_shape", "sub_phase", "sub_drive", "noise_type", "noise_level", "noise_tone"] {
        v.push(s.into());
    }
    v.extend(fx_ids());
    // SPEC v2 Track 5: macro knobs
    v.extend((1..=4).map(|k| format!("macro{k}")));
    for n in 1..=3 {
        v.push(format!("cv{n}_target"));
        v.push(format!("cv{n}_amount"));
    }
    // SPEC v2 Track 7: Mod C / Mod D (no target), LFO extensions on all four, mod envelope, pitch.
    for m in ["modc", "modd"] {
        for s in ["curve", "rate", "sync", "div", "oneshot"] {
            v.push(format!("{m}_{s}"));
        }
    }
    for m in ["moda", "modb", "modc", "modd"] {
        for s in ["phase", "fade", "smooth", "trig"] {
            v.push(format!("{m}_{s}"));
        }
    }
    for s in ["att", "dec", "sus", "rel", "curve"] {
        v.push(format!("menv_{s}"));
    }
    v.extend(["osca_pitch", "oscb_pitch", "pitch_all"].map(String::from));
    // SPEC v2 Track 7 FX: phaser, flanger.
    for s in ["on", "stages", "rate", "sync", "div", "depth", "feedback", "centre", "spread", "mix"] {
        v.push(format!("phs_{s}"));
    }
    for s in ["on", "rate", "sync", "div", "depth", "delay", "feedback", "spread", "mix"] {
        v.push(format!("flg_{s}"));
    }
    v
}

/// SPEC v2 Track 4: the FX section, in chain order (74 ids).
pub fn fx_ids() -> Vec<String> {
    let mut v: Vec<String> = vec![];
    let modules: [(&str, &[&str]); 8] = [
        ("dist", &["on", "type", "drive", "bias", "filter_mode", "filter_type", "filter_freq", "filter_q", "out", "autogain", "mix"]),
        ("mb", &["on", "depth", "time", "xover_lo", "xover_hi", "up_lo", "up_mid", "up_hi", "down_lo", "down_mid", "down_hi",
            "gain_lo", "gain_mid", "gain_hi", "in", "out", "mix"]),
        ("eq", &["on", "low_freq", "low_gain", "peak_freq", "peak_q", "peak_gain", "high_freq", "high_gain", "cut_on", "cut_freq"]),
        ("cho", &["on", "voices", "rate", "sync", "div", "depth", "delay", "feedback", "lpf", "wet_lowcut", "mix"]),
        ("dim", &["on", "size", "wet_lowcut", "mix"]),
        ("dly", &["on", "sync", "time", "div", "feel", "feedback", "mode", "hp", "lp", "mix"]),
        ("rev", &["on", "type", "size", "decay", "damp", "predelay", "lowcut", "hicut", "width", "mix"]),
        ("lim", &["gain"]),
    ];
    for (m, ids) in modules {
        v.extend(ids.iter().map(|s| format!("{m}_{s}")));
    }
    v
}

#[test]
fn ids_match_spec() {
    let params = GtsParams::default();
    let mut got: Vec<String> = params.param_map().into_iter().map(|(id, _, _)| id).collect();
    let mut want = spec_ids();
    got.sort();
    want.sort();
    assert_eq!(got, want);
    assert_eq!((fx_ids().len(), got.len()), (74, 249)); // + 34 Track 7 modulation + 10 Track 7 sound + 19 phaser / flanger
}

/// SPEC v2 Track 4 ranges and defaults: (id, min, max, default) for every float FX param; on
/// switches default off; the module structs the chain receives equal the modules' own defaults
/// except the MB out trim (+1.5 dB, so MB On matches the dry loudness).
#[test]
fn fx_ranges_and_defaults() {
    use gts::dsp::fx::*;
    #[rustfmt::skip]
    let want: &[(&str, f32, f32, f32)] = &[
        ("dist_drive", -12.0, 36.0, 0.0), ("dist_bias", -1.0, 1.0, 0.0), ("dist_filter_freq", 20.0, 20000.0, 1000.0),
        ("dist_filter_q", 0.3, 8.0, 0.707), ("dist_out", -24.0, 12.0, 0.0), ("dist_mix", 0.0, 1.0, 1.0),
        ("mb_depth", 0.0, 2.0, 1.0), ("mb_time", 0.0, 1.0, 0.5), ("mb_xover_lo", 60.0, 400.0, 120.0),
        ("mb_xover_hi", 1000.0, 8000.0, 2500.0), ("mb_up_lo", -60.0, 0.0, -24.0), ("mb_up_mid", -60.0, 0.0, -24.0),
        ("mb_up_hi", -60.0, 0.0, -24.0), ("mb_down_lo", -60.0, 0.0, -12.0), ("mb_down_mid", -60.0, 0.0, -12.0),
        ("mb_down_hi", -60.0, 0.0, -12.0), ("mb_gain_lo", -12.0, 12.0, 0.0), ("mb_gain_mid", -12.0, 12.0, 0.0),
        ("mb_gain_hi", -12.0, 12.0, 0.0), ("mb_in", -24.0, 24.0, 0.0), ("mb_out", -24.0, 24.0, 1.5), ("mb_mix", 0.0, 1.0, 1.0),
        ("eq_low_freq", 30.0, 500.0, 100.0), ("eq_low_gain", -15.0, 15.0, 0.0), ("eq_peak_freq", 20.0, 20000.0, 1000.0),
        ("eq_peak_q", 0.2, 10.0, 1.0), ("eq_peak_gain", -15.0, 15.0, 0.0), ("eq_high_freq", 1000.0, 20000.0, 8000.0),
        ("eq_high_gain", -15.0, 15.0, 0.0), ("eq_cut_freq", 20.0, 500.0, 30.0),
        ("cho_rate", 0.1, 10.0, 0.5), ("cho_depth", 0.0, 1.0, 0.5), ("cho_delay", 1.0, 20.0, 7.0),
        ("cho_feedback", -0.95, 0.95, 0.0), ("cho_lpf", 200.0, 20000.0, 20000.0), ("cho_wet_lowcut", 20.0, 500.0, 120.0),
        ("cho_mix", 0.0, 1.0, 0.5), ("dim_size", 0.0, 1.0, 0.5), ("dim_wet_lowcut", 20.0, 500.0, 120.0), ("dim_mix", 0.0, 1.0, 0.5),
        ("dly_time", 1.0, 2000.0, 250.0), ("dly_feedback", 0.0, 0.95, 0.35), ("dly_hp", 20.0, 2000.0, 120.0),
        ("dly_lp", 200.0, 20000.0, 8000.0), ("dly_mix", 0.0, 1.0, 0.25), ("rev_size", 0.0, 1.0, 0.5),
        ("rev_decay", 0.1, 20.0, 1.5), ("rev_damp", 0.0, 1.0, 0.3), ("rev_predelay", 0.0, 300.0, 0.0),
        ("rev_lowcut", 20.0, 1000.0, 200.0), ("rev_hicut", 1000.0, 20000.0, 12000.0), ("rev_width", 0.0, 1.0, 1.0),
        ("rev_mix", 0.0, 1.0, 0.15), ("lim_gain", -24.0, 12.0, 0.0),
        // SPEC v2 Track 7 (G9)
        ("phs_stages", 4.0, 12.0, 8.0), ("phs_rate", 0.02, 20.0, 0.3), ("phs_depth", 0.0, 1.0, 0.5),
        ("phs_feedback", -0.95, 0.95, 0.3), ("phs_centre", 20.0, 10000.0, 800.0), ("phs_spread", 0.0, 0.5, 0.25),
        ("phs_mix", 0.0, 1.0, 0.5), ("flg_rate", 0.02, 20.0, 0.2), ("flg_depth", 0.0, 1.0, 0.8),
        ("flg_delay", 0.1, 10.0, 4.0), ("flg_feedback", -0.95, 0.95, 0.5), ("flg_spread", 0.0, 0.5, 0.25),
        ("flg_mix", 0.0, 1.0, 0.5),
    ];
    let params = GtsParams::default();
    let map = params.param_map();
    let close = |a: f32, b: f32| (a - b).abs() <= 1e-3 * b.abs().max(1.0);
    for &(id, lo, hi, def) in want {
        let (_, ptr, _) = map.iter().find(|(k, _, _)| k == id).unwrap_or_else(|| panic!("{id} missing"));
        // SAFETY: `params` outlives the pointers.
        let (a, b, d) = unsafe { (ptr.preview_plain(0.0), ptr.preview_plain(1.0), ptr.preview_plain(ptr.default_normalized_value())) };
        assert!(close(a, lo) && close(b, hi) && close(d, def), "{id}: {a}..{b} default {d}, want {lo}..{hi} default {def}");
    }
    let fx = gts::dsp::Patch::default().fx;
    assert!(!(fx.dist.on || fx.mb.on || fx.eq.on || fx.cho.on || fx.dim.on || fx.dly.on || fx.rev.on));
    assert_eq!(fx.dist, distortion::DistortionParams::default());
    assert_eq!(fx.mb, multiband::MultibandParams { out_db: 1.5, ..Default::default() });
    assert_eq!(fx.eq, eq::EqParams::default());
    assert_eq!(fx.cho, chorus::ChorusParams::default());
    assert_eq!(fx.dim, dimension::DimensionParams::default());
    assert_eq!(fx.dly, delay::DelayParams::default());
    assert_eq!(fx.rev, reverb::ReverbParams::default());
    assert_eq!(fx.lim_gain, 0.0);
    assert_eq!(fx.phs, phaser::PhaserParams::default());
    assert_eq!(fx.flg, flanger::FlangerParams::default());
}

/// Knob readouts: crossover floor says Off (and parses it), chorus feedback is a percentage,
/// delay time keeps one decimal, reverb pre-delay whole ms.
#[test]
fn readouts() {
    let params = GtsParams::default();
    let map = params.param_map();
    let show = |id: &str, plain: f32| {
        let (_, ptr, _) = map.iter().find(|(k, _, _)| k == id).unwrap();
        // SAFETY: `params` outlives the pointers.
        unsafe { ptr.normalized_value_to_string(ptr.preview_normalized(plain), true) }
    };
    assert_eq!(show("mono_xover", 20.0), "Off");
    assert_eq!(show("mono_xover", 120.0), "120 Hz");
    assert_eq!(show("cho_feedback", 0.35), "35%");
    assert_eq!(show("dly_time", 123.456), "123.5 ms");
    assert_eq!(show("rev_predelay", 12.4), "12 ms");
    let (_, ptr, _) = map.iter().find(|(k, _, _)| k == "mono_xover").unwrap();
    // SAFETY: as above.
    unsafe {
        assert_eq!(ptr.string_to_normalized_value("Off"), Some(0.0));
        assert!((ptr.preview_plain(ptr.string_to_normalized_value("120 Hz").unwrap()) - 120.0).abs() < 0.01);
    }
}
