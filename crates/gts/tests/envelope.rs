//! SPEC Verification 1, envelope: A = 100 ms reaches 0.99 within 100 ms +- 5 %; release falls
//! below 1e-3 within R; retrigger never jumps.
use gts::dsp::env::{Adsr, Stage};

const SR: f32 = 48000.0;

#[test]
fn attack_reaches_099_on_time() {
    let mut e = Adsr::default();
    e.set(0.1, 0.3, 1.0, 0.06, SR);
    e.gate_on();
    let n = (1..).find(|_| e.next() >= 0.99).unwrap();
    let ms = n as f32 / SR * 1000.0;
    assert!((95.0..=105.0).contains(&ms), "0.99 at {ms} ms");
}

#[test]
fn release_falls_below_threshold_within_r() {
    for r in [0.001, 0.06, 0.5, 3.0] {
        let mut e = Adsr::default();
        e.set(0.005, 0.3, 1.0, r, SR);
        e.gate_on();
        for _ in 0..(0.1 * SR) as usize {
            e.next();
        }
        assert_eq!(e.stage(), Stage::Sustain);
        e.gate_off();
        let n = (r * SR).ceil() as usize;
        let mut last = 1.0;
        for _ in 0..n {
            last = e.next();
        }
        assert!(last < 1e-3, "R {r}: level {last} after R");
        for _ in 0..n {
            e.next();
        }
        assert_eq!(e.stage(), Stage::Idle);
    }
}

#[test]
fn decay_settles_on_sustain_and_retrigger_is_continuous() {
    let mut e = Adsr::default();
    e.set(0.005, 0.1, 0.4, 0.2, SR);
    e.gate_on();
    for _ in 0..(0.2 * SR) as usize {
        e.next();
    }
    assert!((e.level() - 0.4).abs() < 1e-3);
    e.gate_off();
    for _ in 0..(0.05 * SR) as usize {
        e.next();
    }
    let before = e.level();
    e.gate_on();
    let after = e.next();
    assert!(after >= before && after - before < 0.01, "retrigger jumped {before} -> {after}");
}
