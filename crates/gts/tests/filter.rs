use gts::dsp::filter::MultiFilter;
use gts::params::FilterType;
use std::f64::consts::TAU;

const SR: f64 = 48000.0;

fn goertzel(x: &[f32], f: f64) -> f64 {
    let (mut re, mut im) = (0.0, 0.0);
    for (n, &v) in x.iter().enumerate() {
        let p = TAU * f * n as f64 / SR;
        re += v as f64 * p.cos();
        im += v as f64 * p.sin();
    }
    (re * re + im * im).sqrt() / x.len() as f64 * 2.0
}

fn run(f: &mut MultiFilter, input: impl Fn(usize) -> f32, n: usize) -> Vec<f32> {
    (0..n).map(|i| f.process(input(i))).collect()
}

fn sine_gain(f: &mut MultiFilter, hz: f64) -> f64 {
    let y = run(f, |i| (TAU * hz * i as f64 / SR).sin() as f32, 24000);
    let t = &y[12000..];
    ((t.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / t.len() as f64) * 2.0).sqrt()
}

fn mk() -> MultiFilter {
    let mut f = MultiFilter::new();
    f.set_sample_rate(SR);
    f
}

#[test]
fn lp12_cutoff_and_rolloff() {
    // res 0.094 -> Q = 0.707 (Butterworth), -3 dB at cutoff.
    let mut f = mk();
    f.set(FilterType::Lp12, 1000.0, 0.094);
    let db = 20.0 * sine_gain(&mut f, 1000.0).log10();
    assert!((db + 3.0).abs() < 1.0, "{db}");
    f.reset();
    let db4 = 20.0 * sine_gain(&mut f, 4000.0).log10();
    assert!(db4 < -20.0, "{db4}");
}

#[test]
fn comb_minus_peaks() {
    let mut f = mk();
    f.set(FilterType::CombMinus, 220.0, 0.9);
    let ir = run(&mut f, |i| if i == 0 { 1.0 } else { 0.0 }, 48000);
    for k in 1..=5 {
        let target = 220.0 * k as f64;
        let (mut best, mut bf) = (0.0, 0.0);
        let mut hz = target * 0.9;
        while hz < target * 1.1 {
            let m = goertzel(&ir, hz);
            if m > best {
                best = m;
                bf = hz;
            }
            hz += 0.5;
        }
        assert!((bf - target).abs() / target < 0.02, "k={k} peak {bf}");
    }
}

#[test]
fn am_sidebands() {
    let mut f = mk();
    f.set(FilterType::Am, 100.0, 1.0);
    let y = run(&mut f, |i| (TAU * 1000.0 * i as f64 / SR).sin() as f32, 48000);
    let (lo, hi, mid) = (goertzel(&y, 900.0), goertzel(&y, 1100.0), goertzel(&y, 1000.0));
    assert!(lo > 0.4 && hi > 0.4 && mid < 0.01, "{lo} {hi} {mid}");
}

#[test]
fn all_types_finite_at_extremes() {
    for k in [FilterType::Lp12, FilterType::Bp12, FilterType::CombPlus, FilterType::CombMinus, FilterType::Am] {
        let mut f = mk();
        for (hz, r) in [(20.0, 1.0), (20000.0, 1.0), (1000.0, 0.0)] {
            f.set(k, hz, r);
            for i in 0..5000 {
                assert!(f.process(if i % 50 == 0 { 1.0 } else { -0.3 }).is_finite());
            }
        }
    }
}
