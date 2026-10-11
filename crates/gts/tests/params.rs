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
    for n in 1..=3 {
        v.push(format!("cv{n}_target"));
        v.push(format!("cv{n}_amount"));
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
}
