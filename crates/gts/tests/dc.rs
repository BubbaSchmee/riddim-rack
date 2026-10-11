//! SPEC Verification 1, dc: for every table at shift -12 and -24, |mean| of the oscillator output
//! is under 2 % of its RMS (8 Hz per-osc DC blocker). Bypass routing, shaper off: the output is
//! osc A times a constant (sustain 1.0, steady second).
use gts::dsp::{Engine, Patch};
use gts::params::{Routing, Table};
use nih_plug::prelude::Enum;

const SR: f32 = 48000.0;

#[test]
fn negative_shift_has_no_dc() {
    for ti in 0..Table::variants().len() {
        let table = Table::from_index(ti);
        for shift in [-12.0, -24.0] {
            let mut e = Engine::new();
            e.initialize(SR);
            let mut p = Patch::default();
            p.routing = Routing::Bypass;
            p.osc[0].table = table;
            p.osc[0].shift = shift;
            e.set_patch(&p);
            e.note_on(33, 1.0);
            let x: Vec<f64> = (0..2 * SR as usize).map(|_| e.next_frame().0 as f64).skip(SR as usize).collect();
            let mean = x.iter().sum::<f64>() / x.len() as f64;
            let rms = (x.iter().map(|v| v * v).sum::<f64>() / x.len() as f64).sqrt();
            assert!(mean.abs() < 0.02 * rms, "{table:?} shift {shift}: mean {mean} rms {rms}");
        }
    }
}
