//! SPEC v2 4.8 Limiter acceptance, 48 kHz. Run with `--nocapture` for every figure; CPU:
//! `cargo test --release --test fx_limiter -- --ignored --nocapture`.

use gts::dsp::fx::limiter::{Limiter, LimiterParams, CEILING};

const SR: f32 = 48000.0;

fn new(gain_db: f32, sr: f32) -> Limiter {
    let mut m = Limiter::new();
    m.set_sample_rate(sr);
    m.set(&LimiterParams { on: true, gain_db });
    m.reset();
    m
}

fn run(m: &mut Limiter, l: &[f32], r: &[f32]) -> (Vec<f32>, Vec<f32>) {
    l.iter()
        .zip(r)
        .map(|(&a, &b)| {
            let (mut a, mut b) = (a, b);
            m.process(&mut a, &mut b);
            (a, b)
        })
        .unzip()
}

/// xorshift white noise in -1..1.
fn noise(n: usize, seed: u32) -> Vec<f32> {
    let mut s = seed;
    (0..n)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            s as f32 / u32::MAX as f32 * 2.0 - 1.0
        })
        .collect()
}

#[test]
fn never_exceeds_ceiling() {
    let n = 96000;
    let mut cases: Vec<(&str, Vec<f32>, Vec<f32>)> = vec![
        ("noise", noise(n, 1), noise(n, 2)),
        ("square 0 dBFS", (0..n).map(|i| if (i / 50) % 2 == 0 { 1.0 } else { -1.0 }).collect(), vec![0.0; n]),
        ("clicks", (0..n).map(|i| if i % 997 == 0 { 8.0 } else { 0.0 }).collect(), (0..n).map(|i| if i % 1009 == 0 { -8.0 } else { 0.0 }).collect()),
    ];
    // Bursts: loud noise gated on and off, plus a sweep from 20 Hz to 20 kHz.
    let bursts: Vec<f32> = noise(n, 3).iter().enumerate().map(|(i, v)| if (i / 4000) % 2 == 0 { 4.0 * v } else { 0.05 * v }).collect();
    let sweep: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f64 / SR as f64;
            let ph = std::f64::consts::TAU * 20.0 * 2.0 / (1000f64.ln()) * (1000f64.powf(t / 2.0) - 1.0);
            (1.5 * ph.sin()) as f32
        })
        .collect();
    cases.push(("bursts", bursts.clone(), bursts));
    cases.push(("sweep", sweep.clone(), sweep));
    for sr in [44100.0, 48000.0, 96000.0, 192000.0] {
        for (name, l, r) in &cases {
            for gain in [0.0, 12.0] {
                let (yl, yr) = run(&mut new(gain, sr), l, r);
                let peak = yl.iter().chain(&yr).fold(0f32, |m, v| m.max(v.abs()));
                if sr == SR {
                    println!("{name} gain +{gain} dB: peak {:.3} dBFS", 20.0 * peak.log10());
                }
                assert!(peak <= CEILING, "{name} @ {sr}: {peak}");
            }
        }
    }
    // Non-finite input is silenced.
    let (yl, _) = run(&mut new(0.0, SR), &[f32::NAN, f32::INFINITY, 0.5], &[0.0; 3]);
    assert!(yl.iter().all(|v| v.is_finite()));
}

/// THD of the steady part via DFT at exact-bin harmonics.
fn thd(y: &[f32], f: f64) -> f64 {
    let n = y.len();
    let amp = |h: f64| {
        let w = std::f64::consts::TAU * f * h / SR as f64;
        let (mut c, mut s) = (0.0, 0.0);
        for (i, &v) in y.iter().enumerate() {
            c += v as f64 * (w * i as f64).cos();
            s += v as f64 * (w * i as f64).sin();
        }
        (c * c + s * s).sqrt() * 2.0 / n as f64
    };
    let a1 = amp(1.0);
    let hs: f64 = (2..=(20000.0 / f) as usize).map(|h| amp(h as f64).powi(2)).sum();
    hs.sqrt() / a1
}

#[test]
fn thd_at_3_db_over() {
    // n = 48000 (1 s): integer-Hz tones land on exact bins.
    let a = CEILING * 10f32.powf(3.0 / 20.0);
    for f in [40.0, 65.0, 100.0, 1000.0, 5000.0] {
        let x: Vec<f32> = (0..96000).map(|i| a * (std::f64::consts::TAU * f * i as f64 / SR as f64).sin() as f32).collect();
        let (y, _) = run(&mut new(0.0, SR), &x, &x);
        let t = thd(&y[48000..], f) * 100.0;
        let peak = y[48000..].iter().fold(0f32, |m, v| m.max(v.abs()));
        println!("sine {f} Hz at +3 dB over: THD {t:.3} %, peak {:.2} dBFS", 20.0 * peak.log10());
        assert!(t <= 1.0, "{f} Hz THD {t:.3} %");
    }
}

#[test]
fn latency_and_bypass() {
    let mut m = new(0.0, SR);
    assert_eq!(m.latency(), 72);
    let x: Vec<f32> = (0..2000).map(|i| 0.5 * ((i * 7919) % 1000) as f32 / 1000.0 - 0.25).collect();
    let (y, _) = run(&mut m, &x, &x);
    // Below the ceiling at gain 0: the input, delayed by the lookahead, unchanged.
    for i in 0..x.len() - 72 {
        assert!((y[i + 72] - x[i]).abs() < 1e-6, "{i}");
    }
    let mut b = new(12.0, SR);
    b.set(&LimiterParams { on: false, gain_db: 12.0 });
    let big: Vec<f32> = x.iter().map(|v| v * 10.0).collect();
    let (y, _) = run(&mut b, &big, &big);
    for i in 0..x.len() - 72 {
        assert_eq!(y[i + 72], big[i]);
    }
    assert_eq!(new(0.0, 44100.0).latency(), 66);
}

#[test]
#[ignore]
fn bench_cpu() {
    use std::time::Instant;
    let n = 480_000;
    let (l, r) = (noise(4800, 5), noise(4800, 6));
    let mut m = new(6.0, SR);
    let t = Instant::now();
    let mut acc = 0.0;
    for i in 0..n {
        let (mut a, mut b) = (l[i % 4800], r[i % 4800]);
        m.process(&mut a, &mut b);
        acc += a + b;
    }
    let ns = t.elapsed().as_nanos() as f64 / n as f64;
    println!("CPU limiter: {ns:.1} ns/sample stereo ({acc:.1})");
}
