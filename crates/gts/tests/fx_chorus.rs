//! SPEC v2 4.4 Chorus acceptance: L/R correlation 0.2..0.8 above 400 Hz at depth 0.5, >= 0.95
//! below the wet low cut, no pitch error of the dry path; plus bypass, sync, clicks, feedback
//! stability and CPU. Numbers print with `--nocapture`.
use gts::dsp::fx::chorus::*;
use std::f64::consts::TAU;
use std::time::Instant;

const SR: f64 = 48000.0;

fn params() -> ChorusParams {
    ChorusParams { on: true, ..ChorusParams::default() }
}

/// RBJ biquad, `hp` or low-pass, Butterworth Q.
fn rbj(hp: bool, f: f64) -> impl FnMut(f64) -> f64 {
    let w = TAU * f / SR;
    let alpha = w.sin() / (2.0 * std::f64::consts::FRAC_1_SQRT_2);
    let a0 = 1.0 + alpha;
    let (b0, b1) = if hp { ((1.0 + w.cos()) / 2.0, -(1.0 + w.cos())) } else { ((1.0 - w.cos()) / 2.0, 1.0 - w.cos()) };
    let (b0, b1, b2, a1, a2) = (b0 / a0, b1 / a0, b0 / a0, -2.0 * w.cos() / a0, (1.0 - alpha) / a0);
    let (mut x1, mut x2, mut y1, mut y2) = (0.0, 0.0, 0.0, 0.0);
    move |x| {
        let y = b0 * x + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
        (x2, x1, y2, y1) = (x1, x, y1, y);
        y
    }
}

/// 4th-order (two stacked RBJ sections) filter over a whole signal.
fn filt(x: &[f64], hp: bool, f: f64) -> Vec<f64> {
    let (mut a, mut b) = (rbj(hp, f), rbj(hp, f));
    x.iter().map(|&v| b(a(v))).collect()
}

fn corr(l: &[f64], r: &[f64]) -> f64 {
    let (mut lr, mut ll, mut rr) = (0.0, 0.0, 0.0);
    for (&a, &b) in l.iter().zip(r) {
        lr += a * b;
        ll += a * a;
        rr += b * b;
    }
    lr / (ll * rr).sqrt()
}

fn saw(f: f64, i: usize) -> f64 {
    let ph = (f * i as f64 / SR).fract();
    0.5 * (2.0 * ph - 1.0)
}

/// Run `secs` of mono `input` through the chorus; returns (L, R) after a 0.5 s warm-up.
fn run(p: &ChorusParams, secs: f64, input: impl Fn(usize) -> f64) -> (Vec<f64>, Vec<f64>) {
    let mut c = Chorus::new();
    c.set_sample_rate(SR);
    c.set(p);
    c.reset();
    let warm = (0.5 * SR) as usize;
    let n = warm + (secs * SR) as usize;
    let (mut lo, mut ro) = (Vec::new(), Vec::new());
    for i in 0..n {
        let x = input(i) as f32;
        let (mut l, mut r) = (x, x);
        c.process(&mut l, &mut r);
        assert!(l.is_finite() && r.is_finite());
        if i >= warm {
            lo.push(l as f64);
            ro.push(r as f64);
        }
    }
    (lo, ro)
}

#[test]
fn width_above_400_hz_is_in_band() {
    // FX standard test 5: mono 220 Hz saw, mix 50 %, rate 0.5 Hz, depth 50 %, over 5 s.
    for voices in [2, 4, 6, 8] {
        let p = ChorusParams { voices, rate_hz: 0.5, depth: 0.5, mix: 0.5, ..params() };
        let (l, r) = run(&p, 5.0, |i| saw(220.0, i));
        let c = corr(&filt(&l, true, 400.0), &filt(&r, true, 400.0));
        println!("chorus {voices} voices: L/R correlation above 400 Hz = {c:.3}");
        assert!((0.2..=0.8).contains(&c), "{voices} voices: correlation {c:.3}");
    }
}

#[test]
fn sub_band_stays_mono() {
    // 55 Hz sine under a 220 Hz saw; correlation of everything below 100 Hz (default low cut 120).
    for (mix, fb) in [(0.5, 0.0), (1.0, 0.0), (1.0, 0.9), (1.0, -0.9)] {
        let p = ChorusParams { mix, feedback: fb, depth: 1.0, voices: 8, ..params() };
        let (l, r) = run(&p, 5.0, |i| 0.5 * (TAU * 55.0 * i as f64 / SR).sin() + saw(220.0, i));
        let c = corr(&filt(&l, false, 100.0), &filt(&r, false, 100.0));
        println!("chorus mix {mix} feedback {fb}: L/R correlation below 100 Hz = {c:.4}");
        assert!(c >= 0.95, "mix {mix} fb {fb}: sub correlation {c:.4}");
    }
}

#[test]
fn dry_path_is_the_input() {
    let x = |i: usize| 0.4 * (TAU * 1000.0 * i as f64 / SR).sin() + 0.3 * saw(97.0, i);
    // Mix 0: bit-exact, correlation exactly 1.
    let (l, r) = run(&ChorusParams { mix: 0.0, ..params() }, 1.0, x);
    for (i, (&a, &b)) in l.iter().zip(&r).enumerate() {
        let want = x(i + (0.5 * SR) as usize) as f32 as f64;
        assert_eq!((a, b), (want, want));
    }
    assert_eq!(corr(&l, &r), 1.0);
    // Any mix: out = (1 - mix) * input + mix * wet exactly, so the dry part carries no delay and
    // no pitch change (the wet path does not depend on mix).
    let hot = ChorusParams { depth: 1.0, feedback: 0.6, voices: 8, rate_hz: 3.0, ..params() };
    let (wl, wr) = run(&ChorusParams { mix: 1.0, ..hot }, 1.0, x);
    let (hl, hr) = run(&ChorusParams { mix: 0.5, ..hot }, 1.0, x);
    let mut worst: f64 = 0.0;
    for i in 0..hl.len() {
        let dry = x(i + (0.5 * SR) as usize) as f32 as f64;
        worst = worst.max((hl[i] - (0.5 * dry + 0.5 * wl[i])).abs());
        worst = worst.max((hr[i] - (0.5 * dry + 0.5 * wr[i])).abs());
    }
    println!("chorus: |out - (dry/2 + wet/2)| <= {worst:.2e}");
    assert!(worst < 1e-6);
}

#[test]
fn off_is_bit_exact_after_the_fade() {
    let mut c = Chorus::new();
    c.set(&ChorusParams { mix: 0.8, feedback: 0.7, ..params() });
    c.reset();
    let mut exact_from = None;
    for i in 0..48000usize {
        if i == 9600 {
            c.set(&ChorusParams { on: false, mix: 0.8, feedback: 0.7, ..params() });
        }
        let x = saw(110.0, i) as f32;
        let (mut l, mut r) = (x, -x);
        c.process(&mut l, &mut r);
        if (l, r) == (x, -x) {
            exact_from.get_or_insert(i);
        } else {
            exact_from = None;
        }
    }
    let from = exact_from.expect("never bit-exact") - 9600;
    println!("chorus: bit-exact pass-through {:.1} ms after on = false", from as f64 / 48.0);
    assert!(from < 9600);
    assert!(c.is_bypassed());
    // Off from the start: exact from sample 0.
    let mut c = Chorus::new();
    c.set(&ChorusParams { on: false, ..params() });
    c.reset();
    let (mut l, mut r) = (0.25f32, -0.5f32);
    c.process(&mut l, &mut r);
    assert_eq!((l, r), (0.25, -0.5));
}

#[test]
fn sync_locks_the_lfo_to_the_beat() {
    let mut c = Chorus::new();
    c.set(&ChorusParams { sync: true, sync_beats: 1.0, ..params() });
    c.reset();
    c.set_transport(120.0, Some(2.5));
    assert!((c.lfo_phase() - 0.5).abs() < 1e-12);
    // 120 BPM, one cycle per beat = 2 Hz: a quarter second is half a cycle.
    for _ in 0..12000 {
        let (mut l, mut r) = (0.1, 0.1);
        c.process(&mut l, &mut r);
    }
    let ph = c.lfo_phase();
    assert!(ph < 1e-6 || ph > 1.0 - 1e-6, "phase {ph}");
    // Free-running ignores the transport.
    c.set(&ChorusParams { sync: false, ..params() });
    c.set_transport(120.0, Some(0.25));
    assert!(c.lfo_phase() < 1e-6 || c.lfo_phase() > 1.0 - 1e-6);
}

#[test]
fn param_jumps_do_not_click() {
    // A 300 Hz sine leaves nothing above 10 kHz; a click does.
    let a = ChorusParams { voices: 2, depth: 0.2, delay_ms: 3.0, feedback: 0.0, mix: 0.3, lpf_hz: 20000.0, wet_lowcut_hz: 40.0, ..params() };
    let b = ChorusParams { voices: 8, depth: 1.0, delay_ms: 18.0, feedback: 0.8, mix: 1.0, lpf_hz: 2000.0, wet_lowcut_hz: 400.0, rate_hz: 4.0, ..params() };
    let off = ChorusParams { on: false, ..b };
    for (name, x, y) in [("steady", a, a), ("everything", a, b), ("on/off", b, off)] {
        let mut c = Chorus::new();
        c.set(&x);
        c.reset();
        let (mut h1, mut h2) = (rbj(true, 10000.0), rbj(true, 10000.0));
        let mut peak: f64 = 0.0;
        for i in 0..96000usize {
            if i % 12000 == 6000 {
                c.set(if (i / 12000) % 2 == 0 { &y } else { &x });
            }
            let s = (0.5 * (TAU * 300.0 * i as f64 / SR).sin()) as f32;
            let (mut l, mut r) = (s, s);
            c.process(&mut l, &mut r);
            let hf = h2(h1(l as f64));
            if i > 4800 {
                peak = peak.max(hf.abs());
            }
        }
        let db = 20.0 * peak.log10();
        println!("chorus clicks, {name}: peak above 10 kHz {db:.1} dBFS");
        assert!(db < -80.0, "{name}: {db:.1} dBFS");
    }
}

#[test]
fn full_feedback_decays() {
    let mut c = Chorus::new();
    c.set(&ChorusParams { feedback: 0.95, depth: 1.0, voices: 8, mix: 1.0, delay_ms: 1.0, lpf_hz: 20000.0, ..params() });
    c.reset();
    let mut s = 1u32;
    let mut tail: f32 = 0.0;
    for i in 0..(10.0 * SR) as usize {
        let x = if i < 48000 {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            (s as f32 / u32::MAX as f32) - 0.5
        } else {
            0.0
        };
        let (mut l, mut r) = (x, x);
        c.process(&mut l, &mut r);
        assert!(l.is_finite() && l.abs() < 8.0, "sample {i}: {l}");
        if i > (9.0 * SR) as usize {
            tail = tail.max(l.abs().max(r.abs()));
        }
    }
    println!("chorus feedback 0.95: peak 8 s after the input stops {tail:.2e}");
    assert!(tail < 1e-3);
}

#[test]
fn cpu_bench() {
    let n = if cfg!(debug_assertions) { 48000 } else { 4_800_000 };
    for voices in [2, 4, 8] {
        let mut c = Chorus::new();
        c.set(&ChorusParams { voices, feedback: 0.3, ..params() });
        c.reset();
        let mut acc = 0.0f32;
        let t = Instant::now();
        for i in 0..n {
            let x = saw(110.0, i & 65535) as f32;
            let (mut l, mut r) = (x, x);
            c.process(&mut l, &mut r);
            acc += l + r;
        }
        let ns = t.elapsed().as_nanos() as f64 / n as f64;
        println!("chorus CPU {voices} voices: {ns:.1} ns/sample stereo ({acc:.1})");
    }
}
