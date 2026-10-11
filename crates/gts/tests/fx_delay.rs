//! FX 4.6 Delay acceptance (SPEC v2 Track 4; fx-section.md test 7). The module is included by
//! path until the integration wave declares `dsp::fx` in `dsp/mod.rs`. Numbers print with
//! `--nocapture`; run `cargo test --release --test fx_delay -- --nocapture` for the CPU figure.
#[allow(dead_code)]
#[path = "../src/dsp/fx/delay.rs"]
mod delay;
use delay::*;

fn params() -> DelayParams {
    DelayParams {
        on: true,
        sync: false,
        time_ms: 250.0,
        div: Div::D1_8,
        feel: Feel::Straight,
        tempo: 140.0,
        feedback: 0.0,
        mode: DelayMode::Stereo,
        hp_hz: 20.0,
        lp_hz: 20000.0,
        mix: 1.0,
    }
}

fn make(sr: f32, p: &DelayParams) -> Delay {
    let mut d = Delay::new();
    d.set_sample_rate(sr);
    d.set(p);
    d.reset();
    d
}

/// Runs `n` samples of the input generator, returns (L, R).
fn run(d: &mut Delay, n: usize, mut input: impl FnMut(usize) -> (f32, f32)) -> (Vec<f32>, Vec<f32>) {
    let (mut ol, mut or) = (Vec::with_capacity(n), Vec::with_capacity(n));
    for i in 0..n {
        let (mut l, mut r) = input(i);
        d.process(&mut l, &mut r);
        ol.push(l);
        or.push(r);
    }
    (ol, or)
}

fn impulse(i: usize) -> (f32, f32) {
    if i == 0 {
        (1.0, 1.0)
    } else {
        (0.0, 0.0)
    }
}

fn argmax(x: &[f32]) -> usize {
    (0..x.len()).max_by(|&a, &b| x[a].abs().total_cmp(&x[b].abs())).unwrap()
}

#[test]
fn echo_lands_within_one_sample() {
    for sr in [48000.0f32, 44100.0] {
        let cases: [(&str, DelayParams); 4] = [
            ("250 ms", params()),
            ("1/8 @140", DelayParams { sync: true, ..params() }),
            ("1/8. @140", DelayParams { sync: true, feel: Feel::Dotted, ..params() }),
            ("1/8T @140", DelayParams { sync: true, feel: Feel::Triplet, ..params() }),
        ];
        for (name, p) in cases {
            let mut d = make(sr, &p);
            let (l, r) = run(&mut d, (0.6 * sr) as usize, impulse);
            let want = p.seconds() * sr as f64;
            let (gl, gr) = (argmax(&l), argmax(&r));
            println!("{sr} Hz {name}: want {want:.2}, echo L {gl} R {gr}");
            assert!((gl as f64 - want).abs() <= 1.0 && (gr as f64 - want).abs() <= 1.0, "{sr} {name}");
        }
    }
    // 1/8 at 140 BPM = 214.29 ms; dotted 1.5x; triplet 2/3.
    let p = DelayParams { sync: true, ..params() };
    assert!((p.seconds() - 0.214_285_7).abs() < 1e-6);
    assert!((DelayParams { feel: Feel::Dotted, ..p }.seconds() / p.seconds() - 1.5).abs() < 1e-12);
    assert!((DelayParams { feel: Feel::Triplet, ..p }.seconds() / p.seconds() - 2.0 / 3.0).abs() < 1e-12);
    // Range ends: 1/64 at 140 = 26.8 ms; 4/1 = 16 beats fits the 8 s line from 120 BPM, clamps below.
    assert!((DelayParams { div: Div::D1_64, ..p }.seconds() - 60.0 / 140.0 / 16.0).abs() < 1e-9);
    assert!((DelayParams { div: Div::D4_1, ..p }.seconds() - 16.0 * 60.0 / 140.0).abs() < 1e-9);
    assert_eq!(DelayParams { div: Div::D4_1, tempo: 120.0, ..p }.seconds(), 8.0);
    assert_eq!(DelayParams { div: Div::D4_1, tempo: 100.0, ..p }.seconds(), MAX_SYNC_S as f64);
    assert_eq!(DelayParams { sync: false, time_ms: 5000.0, ..p }.seconds(), 2.0);
}

/// Peak of each echo window (both channels) for an impulse at `period` samples.
fn echo_peaks(l: &[f32], r: &[f32], period: usize) -> Vec<f32> {
    (1..l.len() / period)
        .map(|k| {
            let w = k * period - period / 2..k * period + period / 2;
            l[w.clone()].iter().chain(&r[w]).fold(0.0f32, |m, v| m.max(v.abs()))
        })
        .collect()
}

#[test]
fn feedback_095_decays_in_every_mode() {
    for mode in [DelayMode::Mono, DelayMode::Stereo, DelayMode::PingPong] {
        // 1/64 at 137 BPM: a fractional delay, so the interpolator is in the loop.
        let p = DelayParams { sync: true, div: Div::D1_64, tempo: 137.0, feedback: 0.95, mode, ..params() };
        let sr = 48000.0;
        let mut d = make(sr, &p);
        let period = (p.seconds() * sr as f64).round() as usize;
        let (l, r) = run(&mut d, period * 500, impulse);
        let peaks = echo_peaks(&l, &r, period);
        let grow = peaks.windows(2).filter(|w| w[1] > w[0] * 1.000_001).count();
        println!(
            "{mode:?}: echo 1 {:.4}, echo 100 {:.2e}, echo {} {:.2e}",
            peaks[0],
            peaks[99],
            peaks.len(),
            peaks[peaks.len() - 1]
        );
        assert_eq!(grow, 0, "{mode:?}: an echo grew");
        assert!(peaks[peaks.len() - 1] < 1e-6 * peaks[0], "{mode:?}");
        assert!(l.iter().chain(&r).all(|v| v.is_finite()));
    }
    // Sustained loud noise at feedback 0.95 stays bounded (<= 1/(1-0.95) = 20x the input).
    let mut d = make(48000.0, &DelayParams { time_ms: 13.7, feedback: 0.95, ..params() });
    let mut seed = 1u32;
    let (l, _) = run(&mut d, 480_000, |_| {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let v = (seed >> 8) as f32 / (1 << 24) as f32 * 2.0 - 1.0;
        (v, v)
    });
    let peak = l.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    println!("noise at fb 0.95: peak {peak:.2}");
    assert!(peak < 20.0);
}

/// RMS of `x` over `w`.
fn rms(x: &[f32], w: std::ops::Range<usize>) -> f64 {
    (x[w.clone()].iter().map(|&v| v as f64 * v as f64).sum::<f64>() / w.len() as f64).sqrt()
}

#[test]
fn ping_pong_alternates_and_feedback_ratio_holds() {
    let sr = 48000.0;
    let period = 2400; // 50 ms
                       // 5 ms Hann-windowed 1 kHz burst.
    let burst = |i: usize| {
        let n = 240;
        if i >= n {
            return (0.0, 0.0);
        }
        let t = i as f32 / sr;
        let w = 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / n as f32).cos();
        let v = w * (std::f32::consts::TAU * 1000.0 * t).sin();
        (v, v)
    };
    let p = DelayParams { time_ms: 50.0, feedback: 0.5, mode: DelayMode::PingPong, ..params() };
    let mut d = make(sr, &p);
    let (l, r) = run(&mut d, period * 7, burst);
    for k in 1..=6 {
        let w = k * period..k * period + 240;
        let (el, er) = (rms(&l, w.clone()), rms(&r, w));
        println!("ping-pong echo {k}: L {el:.2e} R {er:.2e}");
        let (loud, quiet) = if k % 2 == 1 { (el, er) } else { (er, el) };
        // The other side only holds the 20 Hz high-pass tail of the previous echo.
        assert!(quiet < 1e-3 * loud, "echo {k} on the wrong side");
    }
    for mode in [DelayMode::Stereo, DelayMode::PingPong] {
        let mut d = make(sr, &DelayParams { mode, ..p });
        let (l, r) = run(&mut d, period * 4, burst);
        let e = |k: usize| rms(&l, k * period..k * period + 240).max(rms(&r, k * period..k * period + 240));
        let (r1, r2) = (e(2) / e(1), e(3) / e(2));
        println!("{mode:?} feedback 0.5: echo ratios {r1:.4} {r2:.4}");
        assert!((r1 - 0.5).abs() <= 0.02 && (r2 - 0.5).abs() <= 0.02);
    }
}

#[test]
fn bypass_and_mix_zero_are_exact() {
    let input = |i: usize| ((i as f32 * 0.37).sin() * 0.8, (i as f32 * 0.11).cos() * 0.5);
    let mut d = make(48000.0, &DelayParams { on: false, feedback: 0.9, ..params() });
    let (l, r) = run(&mut d, 20000, input);
    assert!((0..20000).all(|i| (l[i], r[i]) == input(i)));
    let mut d = make(48000.0, &DelayParams { mix: 0.0, feedback: 0.9, ..params() });
    let (l, r) = run(&mut d, 20000, input);
    assert!((0..20000).all(|i| (l[i], r[i]) == input(i)));
    // A running delay switched off passes the very next sample untouched.
    let p = DelayParams { feedback: 0.9, time_ms: 10.0, ..params() };
    let mut d = make(48000.0, &p);
    run(&mut d, 5000, input);
    d.set(&DelayParams { on: false, ..p });
    let (mut a, mut b) = input(5000);
    d.process(&mut a, &mut b);
    assert_eq!((a, b), input(5000));
}

#[test]
fn time_change_glides_within_the_slew() {
    let sr = 48000.0;
    let f0 = 1000.0f64;
    let p = DelayParams { time_ms: 100.0, ..params() };
    let mut d = make(sr, &p);
    let sine = |i: usize| {
        let v = (std::f64::consts::TAU * f0 * i as f64 / sr as f64).sin() as f32;
        (v, v)
    };
    let mut out = Vec::new();
    let mut max_step = 0.0f64;
    for (i, t, n) in [(0usize, 100.0f32, 24000usize), (24000, 300.0, 72000), (96000, 20.0, 96000)] {
        d.set(&DelayParams { time_ms: t, ..p });
        let mut prev = d.delay_samples();
        for j in 0..n {
            let (mut l, mut r) = sine(i + j);
            d.process(&mut l, &mut r);
            max_step = max_step.max((d.delay_samples() - prev).abs());
            prev = d.delay_samples();
            out.push(l);
        }
        assert!((d.delay_samples() - t as f64 * 0.001 * sr as f64).abs() < 1e-6, "glide reaches {t} ms");
    }
    // Instantaneous frequency from zero-crossing intervals, after the first echo arrives.
    let mut crossings = Vec::new();
    for i in 4801..out.len() {
        if out[i - 1] <= 0.0 && out[i] > 0.0 {
            crossings.push(i as f64 - 1.0 + out[i - 1] as f64 / (out[i - 1] - out[i]) as f64);
        }
    }
    let freqs: Vec<f64> = crossings.windows(2).map(|w| sr as f64 / (w[1] - w[0])).collect();
    let (lo, hi) = freqs.iter().fold((f64::MAX, 0.0f64), |(a, b), &f| (a.min(f), b.max(f)));
    let max_jump = out.windows(2).skip(4801).fold(0.0f32, |m, w| m.max((w[1] - w[0]).abs()));
    let bound = (std::f64::consts::TAU * f0 * (1.0 + SLEW) / sr as f64) as f32;
    println!("glide: max step {max_step:.4} samples/sample, freq {lo:.1}..{hi:.1} Hz, max |dy| {max_jump:.4} (bound {bound:.4})");
    assert!(max_step <= SLEW + 1e-9);
    assert!(lo >= f0 * (1.0 - SLEW) * 0.99 && hi <= f0 * (1.0 + SLEW) * 1.01);
    assert!(max_jump <= bound * 1.01, "a click");
}

#[test]
fn cpu_bench() {
    let sr = 48000.0;
    for mode in [DelayMode::Stereo, DelayMode::PingPong] {
        let mut d = make(
            sr,
            &DelayParams {
                sync: true,
                div: Div::D1_8,
                feel: Feel::Dotted,
                feedback: 0.6,
                mode,
                hp_hz: 150.0,
                lp_hz: 6000.0,
                mix: 0.3,
                ..params()
            },
        );
        let n = 480_000;
        let mut acc = 0.0f32;
        let t = std::time::Instant::now();
        for i in 0..n {
            let (mut l, mut r) = ((i as f32 * 0.01).sin(), (i as f32 * 0.013).sin());
            d.process(&mut l, &mut r);
            acc += l + r;
        }
        let ns = t.elapsed().as_nanos() as f64 / n as f64;
        println!(
            "CPU delay {mode:?}: {ns:.1} ns/sample stereo ({}) [{acc:.1}]",
            if cfg!(debug_assertions) { "debug" } else { "release" }
        );
    }
}
