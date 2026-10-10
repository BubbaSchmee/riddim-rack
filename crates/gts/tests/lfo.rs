use gts::dsp::lfo::Lfo;
use gts::params::{Curve, Division};

const SR: f64 = 48000.0;

#[test]
fn sync_eighth_at_150_is_0_2s() {
    for beat in [None, Some(0.0)] {
        let mut l = Lfo::new();
        l.set_sample_rate(SR);
        l.trigger();
        let mut w = vec![];
        let mut idx = 0usize;
        let mut b = beat;
        let mut last = f32::MIN;
        // 4 blocks of 24000 samples so per-block grid lock is exercised.
        for _ in 0..4 {
            l.configure(Curve::SawUp, 1.0, Some((Division::Eighth, 150.0, b)), false);
            for _ in 0..24000 {
                let x = l.next();
                if x < last - 1.0 {
                    w.push(idx);
                }
                last = x;
                idx += 1;
            }
            b = b.map(|v| v + 24000.0 / SR * 150.0 / 60.0);
        }
        assert!(w.len() >= 8, "{w:?}");
        for p in w.windows(2) {
            assert!((p[1] as i64 - p[0] as i64 - 9600).abs() <= 1, "{w:?}");
        }
    }
}

#[test]
fn one_shot_holds() {
    for curve in [Curve::Sine, Curve::SawUp, Curve::Decay, Curve::Attack, Curve::SmoothRandom, Curve::SampleHold] {
        let mut l = Lfo::new();
        l.set_sample_rate(SR);
        l.configure(curve, 100.0, None, true);
        l.trigger();
        for _ in 0..485 {
            l.next();
        }
        let h = l.next();
        for _ in 0..5000 {
            assert_eq!(l.next(), h);
        }
    }
}

#[test]
fn curves_in_range_and_free_rate() {
    for curve in [
        Curve::Sine, Curve::Triangle, Curve::SawUp, Curve::SawDown, Curve::Square,
        Curve::SampleHold, Curve::SmoothRandom, Curve::Decay, Curve::Attack,
    ] {
        let mut l = Lfo::new();
        l.set_sample_rate(SR);
        l.configure(curve, 30.0, None, false);
        l.trigger();
        for _ in 0..48000 {
            let x = l.next();
            assert!((-1.0..=1.0).contains(&x), "{curve:?} {x}");
        }
    }
    let mut l = Lfo::new();
    l.set_sample_rate(SR);
    l.configure(Curve::SawUp, 10.0, None, false);
    let mut last = l.next();
    let mut wraps = 0;
    for _ in 1..48000 {
        let x = l.next();
        if x < last - 1.0 {
            wraps += 1;
        }
        last = x;
    }
    assert_eq!(wraps, 9); // 10 cycles, first starts at 0
}
