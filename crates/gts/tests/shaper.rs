//! Shaper: v1 behaviour plus SPEC v2 1.2 (oversampling) and 1.3 (gain staging) acceptance.
//! Measured numbers print with `cargo test -p gts --release --test shaper -- --nocapture`.
use gts::dsp::shaper::Shaper;
use gts::params::ShaperMode::{self, *};
use std::f64::consts::TAU;

const SR: f64 = 48000.0;
const C3: f64 = 130.812_782_650_299_3;

/// 64-harmonic sine-phase saw (GTS Harmonic Rise at index 127), RMS 0.25.
fn saw(f0: f64, n: usize) -> Vec<f32> {
    let raw: Vec<f64> =
        (0..n).map(|i| (1..=64).map(|h| (TAU * h as f64 * f0 * i as f64 / SR).sin() / h as f64).sum()).collect();
    let g = 0.25 / (raw.iter().map(|v| v * v).sum::<f64>() / n as f64).sqrt();
    raw.iter().map(|v| (v * g) as f32).collect()
}

fn sine(f0: f64, n: usize) -> Vec<f32> {
    (0..n).map(|i| (0.25 * 2f64.sqrt() * (TAU * f0 * i as f64 / SR).sin()) as f32).collect()
}

fn square(f0: f64, n: usize) -> Vec<f32> {
    // 64-term odd-harmonic square, RMS 0.25.
    let raw: Vec<f64> = (0..n)
        .map(|i| (0..32).map(|k| (TAU * (2 * k + 1) as f64 * f0 * i as f64 / SR).sin() / (2 * k + 1) as f64).sum())
        .collect();
    let g = 0.25 / (raw.iter().map(|v| v * v).sum::<f64>() / n as f64).sqrt();
    raw.iter().map(|v| (v * g) as f32).collect()
}

fn shape(mode: ShaperMode, amt: f32, os: usize, x: &[f32]) -> Vec<f32> {
    let mut s = Shaper::new();
    s.set_oversampling(os);
    s.set(mode, amt);
    x.iter().map(|&v| s.process(v)).collect()
}

fn rms(x: &[f32]) -> f64 {
    (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64).sqrt()
}

/// In-place radix-2 FFT.
fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -TAU / len as f64;
        for start in (0..n).step_by(len) {
            for k in 0..len / 2 {
                let (wr, wi) = ((ang * k as f64).cos(), (ang * k as f64).sin());
                let (a, b) = (start + k, start + k + len / 2);
                let (xr, xi) = (re[b] * wr - im[b] * wi, re[b] * wi + im[b] * wr);
                re[b] = re[a] - xr;
                im[b] = im[a] - xi;
                re[a] += xr;
                im[a] += xi;
            }
        }
        len <<= 1;
    }
}

/// The audit's alias metric (docs/audits/gts_audit.py `harm_ratio`): Blackman-Harris window,
/// harmonic bins = within 6 bins of k*f0, DC (< 15 Hz) ignored; non-harmonic / harmonic energy, dB.
fn alias_db(x: &[f32], f0: f64) -> f64 {
    let n = x.len().next_power_of_two() / 2;
    let x = &x[x.len() - n..];
    let (a0, a1, a2, a3) = (0.35875, 0.48829, 0.14128, 0.01168);
    let mut re: Vec<f64> = x
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let p = TAU * i as f64 / (n - 1) as f64;
            v as f64 * (a0 - a1 * p.cos() + a2 * (2.0 * p).cos() - a3 * (3.0 * p).cos())
        })
        .collect();
    let mut im = vec![0.0; n];
    fft(&mut re, &mut im);
    let df = SR / n as f64;
    let (mut h, mut nh) = (0.0, 0.0);
    for b in 0..=n / 2 {
        let f = b as f64 * df;
        if f <= 15.0 {
            continue;
        }
        let p = re[b] * re[b] + im[b] * im[b];
        let k = (f / f0).round();
        if k >= 1.0 && (f - k * f0).abs() <= 6.0 * df { h += p } else { nh += p }
    }
    10.0 * (nh / h).log10()
}

/// Hann-windowed DFT magnitude at `f`.
fn mag(x: &[f32], f: f64) -> f64 {
    let n = x.len();
    let (mut re, mut im) = (0.0, 0.0);
    for (i, &v) in x.iter().enumerate() {
        let w = 0.5 - 0.5 * (TAU * i as f64 / n as f64).cos();
        let p = TAU * f * i as f64 / SR;
        re += w * v as f64 * p.cos();
        im += w * v as f64 * p.sin();
    }
    (re * re + im * im).sqrt()
}

/// Classic THD of a sine at `f0`: harmonics 2.. below 20 kHz over the fundamental.
fn thd(y: &[f32], f0: f64) -> f64 {
    let h1 = mag(y, f0);
    let rest: f64 = (2..).map(|k| k as f64 * f0).take_while(|f| *f < 20000.0).map(|f| mag(y, f).powi(2)).sum();
    rest.sqrt() / h1
}

#[test]
fn saturate_unity_at_zero_amt() {
    // Amt 0 bypasses the curve: identity, delayed by the constant latency.
    for os in [1, 2, 4] {
        let mut s = Shaper::new();
        s.set_oversampling(os);
        s.set(Saturate, 0.0);
        let lat = s.latency();
        let y: Vec<f32> = (0..64).map(|i| s.process(if i % 2 == 0 { 1.0 } else { -1.0 })).collect();
        for i in lat..64 {
            let want = if (i - lat) % 2 == 0 { 1.0 } else { -1.0 };
            assert!((y[i] - want).abs() < 1e-6, "os {os} i {i}: {}", y[i]);
        }
    }
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
    assert_eq!(lv.len(), 4, "{lv:?}");
}

#[test]
fn no_nan_at_extremes() {
    for os in [1, 2, 4] {
        for m in [Sine, Saturate, Clip, Quant, Noise] {
            for a in [0.0, 0.5, 1.0] {
                let mut s = Shaper::new();
                s.set_oversampling(os);
                s.set(m, a);
                for x in [-10.0f32, -1.0, 0.0, 0.3, 1.0, 10.0].iter().cycle().take(200) {
                    let y = s.process(*x);
                    assert!(y.is_finite(), "{m:?} {a} {x}");
                }
            }
        }
    }
}

#[test]
fn quant_is_mid_tread() {
    // Zero in -> zero out at every bit depth: no DC pedestal, no click at note end.
    for i in 0..=20 {
        let mut s = Shaper::new();
        s.set(Quant, i as f32 / 20.0);
        assert_eq!(s.process(0.0), 0.0, "amt {}", i as f32 / 20.0);
    }
}

#[test]
fn latency_is_constant_and_matches_report() {
    // A low-level 1 kHz sine never reaches the Clip threshold, so the oversampled path is linear:
    // its output must be the input delayed by exactly `latency()`, like the bypass path.
    let x = sine(1000.0, 4800).iter().map(|v| v * 0.3).collect::<Vec<_>>();
    for os in [2, 4] {
        let mut s = Shaper::new();
        s.set_oversampling(os);
        s.set(Clip, 1e-4);
        let lat = s.latency();
        let y: Vec<f32> = x.iter().map(|&v| s.process(v)).collect();
        let err = (1000..4800).map(|i| (y[i] - x[i - lat]).abs()).fold(0.0, f32::max);
        println!("os {os}x: latency {lat} samples, max error vs delayed input {err:.2e}");
        assert!(err < 2e-3, "os {os}: {err}");
    }
    assert_eq!([1, 2, 4].map(|f| {
        let mut s = Shaper::new();
        s.set_oversampling(f);
        s.latency()
    }), [0, 21, 23]);
}

#[test]
fn oversampling_alias_floor() {
    // SPEC v2 1.2: Saturate 1.0 on a C3 saw, alias <= -60 dB at 2x and <= -70 dB at 4x.
    let x = saw(C3, 48000);
    let tail = |y: &[f32]| y[12000..].to_vec();
    for (mode, amt) in [(Saturate, 1.0), (Saturate, 0.5), (Clip, 0.5), (Clip, 1.0), (Sine, 0.5), (Sine, 1.0)] {
        let r: Vec<f64> = [1, 2, 4].iter().map(|&os| alias_db(&tail(&shape(mode, amt, os, &x)), C3)).collect();
        println!("{mode:?} {amt}: alias 1x {:6.1} dB, 2x {:6.1} dB, 4x {:6.1} dB", r[0], r[1], r[2]);
        if mode == Saturate && amt == 1.0 {
            assert!(r[1] <= -60.0, "2x {}", r[1]);
            assert!(r[2] <= -70.0, "4x {}", r[2]);
        }
    }
    println!("dry saw alias {:.1} dB", alias_db(&tail(&x), C3));
}

#[test]
fn toggling_keeps_level_within_1db() {
    // SPEC v2 1.3: amt 0.5, every mode, 0.25-RMS saw (the acceptance input). Sine and square
    // inputs must hold too (the compensation averages saw and square amplitude distributions).
    let n = 24000;
    let inputs = [("saw", saw(C3, n)), ("sine", sine(C3, n)), ("square", square(C3, n))];
    for mode in [Sine, Saturate, Clip, Quant, Noise] {
        let mut line = format!("{mode:?} 0.5:");
        for (name, x) in &inputs {
            let y = shape(mode, 0.5, 2, x);
            let db = 20.0 * (rms(&y[4800..]) / rms(&x[4800..])).log10();
            line += &format!(" {name} {db:+.2} dB");
            assert!(db.abs() <= 1.0, "{mode:?} {name}: {db:+.2} dB");
        }
        println!("{line}");
    }
}

#[test]
fn clip_half_distorts() {
    // SPEC v2 1.3: Clip 0.5 gives >= 5 % THD on a 0.25-RMS signal (v1: 0 %).
    let s = sine(C3, 24000);
    let t_sine = thd(&shape(Clip, 0.5, 2, &s)[4800..], C3);
    // Saw: distortion residual after the best linear fit to the (latency-aligned) input.
    let x = saw(C3, 24000);
    let mut sh = Shaper::new();
    sh.set(Clip, 0.5);
    let lat = sh.latency();
    let y: Vec<f32> = x.iter().map(|&v| sh.process(v)).collect();
    let (xs, ys) = (&x[4800 - lat..24000 - lat], &y[4800..]);
    let gain = xs.iter().zip(ys).map(|(a, b)| (*a * *b) as f64).sum::<f64>() / xs.iter().map(|a| (*a as f64).powi(2)).sum::<f64>();
    let res = xs.iter().zip(ys).map(|(a, b)| (*b as f64 - gain * *a as f64).powi(2)).sum::<f64>();
    let lin = xs.iter().map(|v| (gain * *v as f64).powi(2)).sum::<f64>();
    let t_saw = (res / lin).sqrt();
    println!("Clip 0.5: sine THD {:.1} %, saw distortion {:.1} %", 100.0 * t_sine, 100.0 * t_saw);
    assert!(t_sine >= 0.05 && t_saw >= 0.05);
}
