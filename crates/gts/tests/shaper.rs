use gts::dsp::shaper::Shaper;
use gts::params::ShaperMode::*;

#[test]
fn saturate_unity_at_zero_amt() {
    let mut s = Shaper::new();
    s.set(Saturate, 0.0);
    assert!((s.process(1.0) - 1.0).abs() < 1e-6);
    assert!((s.process(-1.0) + 1.0).abs() < 1e-6);
}

#[test]
fn quant_2bit_levels() {
    let mut s = Shaper::new();
    s.set(Quant, 1.0);
    let mut lv: Vec<i32> = vec![];
    for i in -2000..=2000 {
        let y = s.process(i as f32 / 1000.0);
        let k = (y * 1000.0).round() as i32;
        if !lv.contains(&k) {
            lv.push(k);
        }
    }
    assert!(lv.len() <= 4, "{lv:?}");
}

#[test]
fn no_nan_at_extremes() {
    for m in [Sine, Saturate, Clip, Quant, Noise] {
        for a in [0.0, 0.5, 1.0] {
            let mut s = Shaper::new();
            s.set(m, a);
            for x in [-10.0f32, -1.0, 0.0, 0.3, 1.0, 10.0] {
                let y = s.process(x);
                assert!(y.is_finite(), "{m:?} {a} {x}");
            }
        }
    }
}
