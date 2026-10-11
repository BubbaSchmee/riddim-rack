//! SPEC v2 4.1 Distortion acceptance, 48 kHz. Run with `--nocapture` for every figure; the CPU
//! bench is `cargo test --release --test fx_distortion -- --ignored --nocapture`.
//!
//! THD of a saw is defined as the residual after the best linear fit of the (latency-aligned)
//! input: `rms(y - a*x) / rms(a*x)`, i.e. THD+N relative to the undistorted signal (a saw's own
//! harmonics are not distortion). Alias floor = the `alias.rs` metric (non-harmonic over
//! harmonic energy, Blackman-Harris, harmonic bins within 6 bins of k*f0, DC ignored).

// ponytail: `dsp::fx` is not in the crate tree until the integration wave adds `pub mod fx;` to
// dsp/mod.rs, so the module source is compiled straight into this test, with `crate::dsp::shaper`
// re-exported for its oversampler import. Switch to `gts::dsp::fx::distortion` after integration.
#[allow(dead_code)]
mod dsp {
    pub use gts::dsp::shaper;
    pub mod fx {
        #[path = "../../../src/dsp/fx/distortion.rs"]
        pub mod distortion;
    }
}

use dsp::fx::distortion::{DistType, Distortion, DistortionParams, FilterMode, FilterType};
use gts::dsp::tables::Fft;

const SR: f32 = 48000.0;
const C2: f64 = 65.406_391_325;

/// Band-limited saw (harmonics below 20 kHz), RMS 0.25.
fn saw(f0: f64, n: usize) -> Vec<f32> {
    let k = (20000.0 / f0) as usize;
    let tau = std::f64::consts::TAU;
    let y: Vec<f64> = (0..n)
        .map(|i| {
            let ph = tau * f0 * i as f64 / SR as f64;
            (1..=k).map(|h| (ph * h as f64).sin() / h as f64).sum::<f64>()
        })
        .collect();
    let rms = (y.iter().map(|v| v * v).sum::<f64>() / n as f64).sqrt();
    y.iter().map(|v| (v * 0.25 / rms) as f32).collect()
}

fn params(kind: DistType, drive_db: f32) -> DistortionParams {
    DistortionParams { on: true, kind, drive_db, ..DistortionParams::default() }
}

/// Mono-in (both channels), left out.
fn run(p: &DistortionParams, x: &[f32]) -> Vec<f32> {
    let mut d = Distortion::new();
    d.set_sample_rate(SR);
    d.set(p);
    d.reset();
    x.iter()
        .map(|&v| {
            let (mut l, mut r) = (v, v);
            d.process(&mut l, &mut r);
            l
        })
        .collect()
}

fn rms_db(x: &[f32]) -> f64 {
    10.0 * (x.iter().map(|&v| v as f64 * v as f64).sum::<f64>() / x.len() as f64).log10()
}

/// Residual after the best linear fit, `y[i + lat]` against `x[i]`, over `from..`.
fn thd(x: &[f32], y: &[f32], lat: usize, from: usize) -> f64 {
    let idx = from..x.len() - lat;
    let (mut xy, mut xx) = (0.0, 0.0);
    for i in idx.clone() {
        xy += x[i] as f64 * y[i + lat] as f64;
        xx += x[i] as f64 * x[i] as f64;
    }
    let a = xy / xx;
    let e: f64 = idx.map(|i| (y[i + lat] as f64 - a * x[i] as f64).powi(2)).sum();
    (e / (a * a * xx)).sqrt()
}

fn spectrum(x: &[f32]) -> (Vec<f64>, f64) {
    let n = x.len();
    let tau = std::f64::consts::TAU;
    let mut re: Vec<f64> = x
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let t = tau * i as f64 / (n - 1) as f64;
            v as f64 * (0.35875 - 0.48829 * t.cos() + 0.14128 * (2.0 * t).cos() - 0.01168 * (3.0 * t).cos())
        })
        .collect();
    let mut im = vec![0.0; n];
    Fft::new(n).run(&mut re, &mut im, false);
    ((0..=n / 2).map(|k| re[k] * re[k] + im[k] * im[k]).collect(), SR as f64 / n as f64)
}

fn alias_db(x: &[f32], f0: f64) -> f64 {
    let (p, df) = spectrum(x);
    let (mut h, mut nh) = (0.0, 0.0);
    for (k, &v) in p.iter().enumerate() {
        let f = k as f64 * df;
        if f <= 15.0 {
            continue;
        }
        let kk = (f / f0).round();
        if kk >= 1.0 && (f - kk * f0).abs() <= 6.0 * df {
            h += v;
        } else {
            nh += v;
        }
    }
    10.0 * (nh / h).max(1e-30).log10()
}

/// Energy above `lo` Hz relative to the total above 15 Hz, dB.
fn above_db(x: &[f32], lo: f64) -> f64 {
    let (p, df) = spectrum(x);
    let tot: f64 = p.iter().enumerate().filter(|(k, _)| *k as f64 * df > 15.0).map(|(_, v)| v).sum();
    let hi: f64 = p.iter().enumerate().filter(|(k, _)| *k as f64 * df >= lo).map(|(_, v)| v).sum();
    10.0 * (hi / tot).log10()
}

#[test]
fn bypass_and_mix0_are_the_delayed_input() {
    let x: Vec<f32> = (0..4000).map(|i| ((i * 7919) % 1000) as f32 / 500.0 - 1.0).collect();
    for p in [
        DistortionParams { on: false, ..params(DistType::HardClip, 24.0) },
        DistortionParams { mix: 0.0, ..params(DistType::FoldSine, 24.0) },
    ] {
        let y = run(&p, &x);
        assert_eq!(Distortion::new().latency(), 23);
        assert!(y[..23].iter().all(|&v| v == 0.0));
        for i in 0..x.len() - 23 {
            assert_eq!(y[i + 23], x[i], "sample {i}, {p:?}");
        }
    }
}

#[test]
fn hardclip_thd_on_c2_saw() {
    let x = saw(C2, 48000);
    let y = run(&params(DistType::HardClip, 12.0), &x);
    let t = thd(&x, &y, 23, 4800) * 100.0;
    println!("HardClip +12 dB, RMS-0.25 C2 saw: THD {t:.1} %");
    assert!((15.0..=40.0).contains(&t), "THD {t:.1} %");
}

#[test]
fn autogain_holds_rms() {
    let x = saw(C2, 48000);
    let ref_db = rms_db(&x[4800..]);
    for kind in [DistType::Tube, DistType::Overdrive] {
        for drive in [0.0, 6.0, 12.0, 18.0, 24.0] {
            let y = run(&params(kind, drive), &x);
            let d = rms_db(&y[4800..]) - ref_db;
            println!("{kind:?} drive {drive:+} dB: RMS {d:+.2} dB re input");
            assert!(d.abs() <= 1.0, "{kind:?} {drive}: {d:+.2} dB");
        }
    }
    // Autogain off: drive moves the level.
    let y = run(&DistortionParams { autogain: false, ..params(DistType::Tube, 24.0) }, &x);
    assert!(rms_db(&y[4800..]) - ref_db > 3.0);
}

#[test]
fn alias_floor_at_4x() {
    let n = 65536;
    let x = saw(C2, n + 8192);
    for kind in DistType::ALL.into_iter().filter(|k| k.oversampled()) {
        for drive in [12.0, 24.0] {
            let y = run(&params(kind, drive), &x);
            let a = alias_db(&y[8192..], C2);
            println!("{kind:?} drive +{drive} dB: alias {a:.1} dB");
            if drive == 12.0 {
                assert!(a <= -70.0, "{kind:?}: alias {a:.1} dB");
            }
        }
    }
}

#[test]
fn filter_off_pre_post() {
    let n = 32768;
    let x = saw(C2, n + 4800);
    let mut hi = Vec::new();
    for mode in [FilterMode::Off, FilterMode::Pre, FilterMode::Post] {
        let p = DistortionParams {
            filter_mode: mode,
            filter_type: FilterType::Lp,
            filter_freq: 1000.0,
            ..params(DistType::HardClip, 12.0)
        };
        let y = run(&p, &x);
        let h = above_db(&y[4800..], 4000.0);
        println!("HardClip +12, LP 1 kHz {mode:?}: energy above 4 kHz {h:.1} dB");
        hi.push(h);
    }
    // Post filters the clipper's harmonics; Pre feeds the clipper a darker signal, which then
    // regrows harmonics, so it lands between.
    assert!(hi[2] < hi[0] - 20.0, "Post {:.1} vs Off {:.1}", hi[2], hi[0]);
    assert!(hi[1] < hi[0] - 3.0 && hi[1] > hi[2] + 3.0, "Pre {:.1}", hi[1]);
    // HP Post removes the fundamental region.
    let p = DistortionParams {
        filter_mode: FilterMode::Post,
        filter_type: FilterType::Hp,
        filter_freq: 2000.0,
        ..params(DistType::HardClip, 12.0)
    };
    let y = run(&p, &x);
    let h = above_db(&y[4800..], 1000.0);
    println!("HP 2 kHz Post: energy above 1 kHz {h:.2} dB of total");
    assert!(h > -1.0);
}

#[test]
fn every_type_is_latency_aligned() {
    // Near-linear settings: the output must line up with the input at lag 23.
    let x: Vec<f32> = (0..9600).map(|i| 0.1 * (std::f32::consts::TAU * 1000.0 * i as f32 / SR).sin()).collect();
    for kind in DistType::ALL {
        let p = DistortionParams { autogain: false, ..params(kind, -12.0) };
        let y = run(&p, &x);
        let best = (0..40)
            .max_by(|&a, &b| {
                let c = |lag: usize| (2000..9000).map(|i| x[i] * y[i + lag]).sum::<f32>();
                c(a).partial_cmp(&c(b)).unwrap()
            })
            .unwrap();
        println!("{kind:?}: best lag {best}");
        // A 1 kHz sine repeats every 48 samples, so 23 must win within +-24.
        assert_eq!(best, 23, "{kind:?}");
        assert!(y.iter().all(|v| v.is_finite()));
    }
}

#[test]
fn all_types_finite_and_bounded_at_extremes() {
    let x = saw(C2, 9600);
    for kind in DistType::ALL {
        for (drive, bias) in [(36.0, 1.0), (36.0, -1.0), (-12.0, 0.5)] {
            let p = DistortionParams { bias, out_db: 12.0, ..params(kind, drive) };
            let y = run(&p, &x);
            let peak = y.iter().fold(0f32, |m, v| m.max(v.abs()));
            assert!(y.iter().all(|v| v.is_finite()) && peak < 20.0, "{kind:?} {drive} {bias}: {peak}");
        }
    }
}

#[test]
#[ignore]
fn bench_cpu() {
    use std::time::Instant;
    let n = 480_000;
    let x = saw(C2, 4800);
    for kind in DistType::ALL {
        let mut d = Distortion::new();
        d.set_sample_rate(SR);
        d.set(&DistortionParams { filter_mode: FilterMode::Post, ..params(kind, 12.0) });
        let t = Instant::now();
        let mut acc = 0.0;
        for i in 0..n {
            let (mut l, mut r) = (x[i % 4800], -x[i % 4800]);
            d.process(&mut l, &mut r);
            acc += l + r;
        }
        let ns = t.elapsed().as_nanos() as f64 / n as f64;
        println!("CPU distortion {kind:?}: {ns:.1} ns/sample stereo ({acc:.1})");
    }
}
