//! SPEC v2 4.2 Multiband compressor (three-band upward/downward) acceptance, 48 kHz. Run with `--nocapture` for
//! every figure; CPU: `cargo test --release --test fx_multiband -- --ignored --nocapture`.
//!
//! Levels are dB re a full-scale sine (a sine of peak -8 dBFS is "-8 dB"), the detector's scale.

use gts::dsp::fx::multiband::{Multiband, MultibandParams};
use gts::dsp::tables::Fft;

const SR: f32 = 48000.0;

fn sine(f: f32, peak_db: f32, n: usize) -> Vec<f32> {
    let a = 10f32.powf(peak_db / 20.0);
    (0..n).map(|i| a * (std::f64::consts::TAU * f as f64 * i as f64 / SR as f64).sin() as f32).collect()
}

fn on() -> MultibandParams {
    MultibandParams { on: true, ..MultibandParams::default() }
}

fn new(p: &MultibandParams) -> Multiband {
    let mut m = Multiband::new();
    m.set_sample_rate(SR);
    m.set(p);
    m.reset();
    m
}

/// Mono in, left out, plus the band-gain trace.
fn run(m: &mut Multiband, x: &[f32]) -> (Vec<f32>, Vec<[f32; 3]>) {
    x.iter()
        .map(|&v| {
            let (mut l, mut r) = (v, v);
            m.process(&mut l, &mut r);
            (l, m.band_gain_db())
        })
        .unzip()
}

fn rms_db(x: &[f32]) -> f64 {
    10.0 * (x.iter().map(|&v| v as f64 * v as f64).sum::<f64>() / x.len() as f64).log10()
}

#[test]
fn single_band_gain_reduction_4_to_1() {
    // 1 kHz sine at -8 dB, downward threshold -20 dB (12 dB over), depth 100 % = 4:1, upward
    // threshold at the floor so only downward acts: expected GR 12 * (1 - 1/4) = 9 dB.
    let p = MultibandParams { single_band: true, up_db: [-60.0; 3], down_db: [-20.0; 3], ..on() };
    let x = sine(1000.0, -8.0, 96000);
    let (y, g) = run(&mut new(&p), &x);
    let gr = rms_db(&x[48000..]) - rms_db(&y[48000..]);
    println!("single band 4:1, 12 dB over: GR {gr:.2} dB (detector {:.2} dB)", -g[95999][1]);
    assert!((gr - 9.0).abs() <= 0.5, "GR {gr:.2}");
}

#[test]
fn upward_raises_quiet_bands() {
    for f in [100.0, 1000.0, 8000.0] {
        let x = sine(f, -40.0, 96000);
        let (y, _) = run(&mut new(&on()), &x);
        let up = rms_db(&y[48000..]) - rms_db(&x[48000..]);
        println!("depth 100 %, -40 dB tone at {f} Hz: {up:+.2} dB");
        assert!(up >= 6.0, "{f} Hz: {up:+.2} dB");
    }
}

#[test]
fn downward_cuts_loud_bands() {
    // Tones well inside one band (100 Hz would straddle the 120 Hz crossover).
    for f in [50.0, 1000.0, 8000.0] {
        let x = sine(f, -3.0, 96000);
        let (y, _) = run(&mut new(&on()), &x);
        let d = rms_db(&y[48000..]) - rms_db(&x[48000..]);
        println!("depth 100 %, -3 dB tone at {f} Hz: {d:+.2} dB");
        assert!(d <= -3.0, "{f} Hz: {d:+.2} dB");
    }
}

#[test]
fn bands_sum_flat_at_depth_0() {
    let n = 1 << 16;
    for mix in [1.0, 0.5] {
        for (lo, hi) in [(120.0, 2500.0), (60.0, 1000.0), (400.0, 8000.0)] {
            let p = MultibandParams { depth: 0.0, xover_lo: lo, xover_hi: hi, mix, ..on() };
            let mut x = vec![0.0f32; n];
            x[0] = 1.0;
            let (y, _) = run(&mut new(&p), &x);
            let mut re: Vec<f64> = y.iter().map(|&v| v as f64).collect();
            let mut im = vec![0.0; n];
            Fft::new(n).run(&mut re, &mut im, false);
            let df = SR as f64 / n as f64;
            let (mut lo_db, mut hi_db) = (f64::MAX, f64::MIN);
            for k in (20.0 / df) as usize..=(20000.0 / df) as usize {
                let m = 10.0 * (re[k] * re[k] + im[k] * im[k]).log10();
                lo_db = lo_db.min(m);
                hi_db = hi_db.max(m);
            }
            println!("depth 0, mix {mix}, xovers {lo}/{hi} Hz: 20 Hz-20 kHz {lo_db:+.4} .. {hi_db:+.4} dB");
            assert!(lo_db >= -0.2 && hi_db <= 0.2);
        }
    }
}

#[test]
fn no_pumping_at_time_50() {
    // Steady tones 6 dB over the downward threshold: after settling, the band gain must not
    // move more than 1 dB peak to peak within or across cycles.
    for f in [40.0, 60.0, 200.0, 1000.0, 5000.0] {
        let p = MultibandParams { time: 0.5, ..on() };
        let x = sine(f, -6.0, 144000);
        let (_, g) = run(&mut new(&p), &x);
        let b = if f < 120.0 { 0 } else if f < 2500.0 { 1 } else { 2 };
        let (lo, hi) = g[96000..].iter().fold((f32::MAX, f32::MIN), |(l, h), v| (l.min(v[b]), h.max(v[b])));
        println!("time 50 %, -6 dB tone {f} Hz: band {b} gain {lo:+.2} .. {hi:+.2} dB, ripple {:.2} dB", hi - lo);
        assert!(hi - lo <= 1.0, "{f} Hz ripple {:.2} dB", hi - lo);
    }
}

#[test]
fn off_is_identity_and_output_finite() {
    let x: Vec<f32> = (0..4800).map(|i| ((i * 7919) % 1000) as f32 / 500.0 - 1.0).collect();
    let (y, _) = run(&mut new(&MultibandParams::default()), &x);
    assert_eq!(x, y);
    let p = MultibandParams { depth: 2.0, time: 0.0, in_db: 24.0, out_db: 24.0, gain_db: [12.0; 3], ..on() };
    let (y, _) = run(&mut new(&p), &x);
    assert!(y.iter().all(|v| v.is_finite()));
    // Silence stays silent (upward gain is capped and applied to zero).
    let (y, _) = run(&mut new(&p), &[0.0; 4800]);
    assert!(y.iter().all(|&v| v == 0.0));
}

#[test]
#[ignore]
fn bench_cpu() {
    use std::time::Instant;
    let n = 480_000;
    let x: Vec<f32> = (0..4800).map(|i| ((i * 7919) % 1000) as f32 / 1000.0 - 0.5).collect();
    let mut m = new(&on());
    let t = Instant::now();
    let mut acc = 0.0;
    for i in 0..n {
        let (mut l, mut r) = (x[i % 4800], x[(i + 7) % 4800]);
        m.process(&mut l, &mut r);
        acc += l + r;
    }
    let ns = t.elapsed().as_nanos() as f64 / n as f64;
    println!("CPU multiband: {ns:.1} ns/sample stereo ({acc:.1})");
}
