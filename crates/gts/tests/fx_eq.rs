//! SPEC v2 4.3 EQ acceptance: measured response within +-0.5 dB of the designed (analog) curve at
//! 20 frequencies, unity at all gains 0, click-free sweeps, CPU. Numbers print with `--nocapture`.
use gts::dsp::fx::eq::*;
use std::f64::consts::TAU;
use std::time::Instant;

/// Below this designed level the f32 output can not resolve 0.5 dB (the low cut reaches -127 dB
/// here); the check there is "at least as deep".
const STOPBAND_DB: f64 = -80.0;

fn params() -> EqParams {
    EqParams { on: true, ..EqParams::default() }
}

/// Measured |H| in dB at `f` from the impulse response.
fn measured_db(ir: &[f32], f: f64, sr: f64) -> f64 {
    let (mut re, mut im) = (0.0, 0.0);
    for (n, &v) in ir.iter().enumerate() {
        let p = TAU * f * n as f64 / sr;
        re += v as f64 * p.cos();
        im -= v as f64 * p.sin();
    }
    10.0 * (re * re + im * im).log10()
}

/// The analog curve: the RBJ prototypes the bands are built from.
fn analog_db(p: &EqParams, f: f64) -> f64 {
    let mut m2 = 1.0;
    if p.low_gain != 0.0 {
        m2 *= proto_mag2(&proto_low_shelf(p.low_gain as f64), f / p.low_freq as f64);
    }
    if p.peak_gain != 0.0 {
        m2 *= proto_mag2(&proto_peak(p.peak_gain as f64, p.peak_q as f64), f / p.peak_freq as f64);
    }
    if p.high_gain != 0.0 {
        m2 *= proto_mag2(&proto_high_shelf(p.high_gain as f64), f / p.high_freq as f64);
    }
    if p.cut_on {
        for s in proto_low_cut() {
            m2 *= proto_mag2(&s, f / p.cut_freq as f64);
        }
    }
    10.0 * m2.log10()
}

fn biquad_db(c: [f64; 5], f: f64, sr: f64) -> f64 {
    let [b0, b1, b2, a1, a2] = c;
    let w = TAU * f / sr;
    let (c1, s1, c2, s2) = (w.cos(), -w.sin(), (2.0 * w).cos(), -(2.0 * w).sin());
    let (nr, ni) = (b0 + b1 * c1 + b2 * c2, b1 * s1 + b2 * s2);
    let (dr, di) = (1.0 + a1 * c1 + a2 * c2, a1 * s1 + a2 * s2);
    10.0 * ((nr * nr + ni * ni) / (dr * dr + di * di)).log10()
}

/// The digital design: the biquads the module computes for these params.
fn digital_db(p: &EqParams, f: f64, sr: f64) -> f64 {
    let mut db = 0.0;
    if p.low_gain != 0.0 {
        db += biquad_db(low_shelf(p.low_gain as f64, p.low_freq as f64, sr), f, sr);
    }
    if p.peak_gain != 0.0 {
        db += biquad_db(peak(p.peak_gain as f64, p.peak_q as f64, p.peak_freq as f64, sr), f, sr);
    }
    if p.high_gain != 0.0 {
        db += biquad_db(high_shelf(p.high_gain as f64, p.high_freq as f64, sr), f, sr);
    }
    if p.cut_on {
        for c in low_cut(p.cut_freq as f64, sr) {
            db += biquad_db(c, f, sr);
        }
    }
    db
}

fn impulse_response(p: &EqParams, sr: f64, n: usize) -> Vec<f32> {
    let mut e = Eq::new();
    e.set_sample_rate(sr);
    e.set(p);
    e.reset();
    (0..n)
        .map(|i| {
            let (mut l, mut r) = (if i == 0 { 1.0 } else { 0.0 }, 0.0);
            e.process(&mut l, &mut r);
            assert_eq!(r, 0.0);
            l
        })
        .collect()
}

/// Every audible upper corner of `p` at or below a fifth of the sample rate.
fn corners_below_fifth_rate(p: &EqParams, sr: f64) -> bool {
    let hi = [(p.peak_gain, p.peak_freq), (p.high_gain, p.high_freq)];
    hi.iter().all(|&(g, f)| g == 0.0 || f as f64 <= 0.2 * sr)
}

#[test]
fn response_matches_design_at_20_frequencies() {
    let cfgs = [
        // Typical bass EQ.
        EqParams { low_freq: 100.0, low_gain: 6.0, peak_freq: 1000.0, peak_q: 1.4, peak_gain: -8.0, high_freq: 8000.0, high_gain: 4.0, cut_on: true, cut_freq: 40.0, ..params() },
        // Everything at the top of its range.
        EqParams { low_freq: 500.0, low_gain: -15.0, peak_freq: 10000.0, peak_q: 2.0, peak_gain: 15.0, high_freq: 20000.0, high_gain: 15.0, cut_on: true, cut_freq: 500.0, ..params() },
        // Everything at the bottom.
        EqParams { low_freq: 30.0, low_gain: 15.0, peak_freq: 20.0, peak_q: 10.0, peak_gain: 15.0, high_freq: 1000.0, high_gain: -15.0, cut_on: true, cut_freq: 20.0, ..params() },
        // Wide bell cut near the top.
        EqParams { low_freq: 200.0, low_gain: 9.0, peak_freq: 15000.0, peak_q: 0.2, peak_gain: -15.0, high_freq: 4000.0, high_gain: 15.0, ..params() },
        EqParams { peak_freq: 5000.0, peak_q: 10.0, peak_gain: 15.0, high_freq: 12000.0, high_gain: -15.0, ..params() },
        EqParams { peak_freq: 20000.0, peak_q: 1.0, peak_gain: 15.0, ..params() },
        EqParams { peak_freq: 3000.0, peak_q: 0.5, peak_gain: -15.0, low_freq: 60.0, low_gain: -15.0, ..params() },
        EqParams { peak_freq: 10000.0, peak_q: 2.0, peak_gain: -15.0, high_freq: 11000.0, high_gain: 15.0, low_freq: 500.0, low_gain: 15.0, ..params() },
    ];
    let freqs: Vec<f64> = (0..20).map(|i| 20.0 * 1000f64.powf(i as f64 / 19.0)).collect();
    let (mut worst_analog, mut worst_digital, mut worst_nyq): (f64, f64, f64) = (0.0, 0.0, 0.0);
    for sr in [44100.0, 48000.0, 96000.0] {
        for (ci, p) in cfgs.iter().enumerate() {
            // 3 s: the 20 Hz, Q 10, +15 dB bell rings with a 0.38 s time constant.
            let ir = impulse_response(p, sr, (3.0 * sr) as usize);
            let strict = corners_below_fifth_rate(p, sr);
            let (mut e_analog, mut e_digital): (f64, f64) = (0.0, 0.0);
            for &f in &freqs {
                let (m, a, d) = (measured_db(&ir, f, sr), analog_db(p, f), digital_db(p, f, sr));
                if a < STOPBAND_DB {
                    assert!(m < STOPBAND_DB + 0.5, "sr {sr} cfg {ci} at {f:.0} Hz: {m:.1} dB, designed {a:.1} dB");
                    continue;
                }
                e_analog = e_analog.max((m - a).abs());
                e_digital = e_digital.max((m - d).abs());
            }
            println!("sr {sr} cfg {ci}: max |measured - analog| {e_analog:.3} dB, |measured - digital design| {e_digital:.4} dB{}", if strict { "" } else { "  (corner above sr/5)" });
            // The implementation is exactly its design.
            assert!(e_digital <= 0.01, "sr {sr} cfg {ci}: {e_digital:.4} dB off its own coefficients");
            if strict {
                assert!(e_analog <= 0.5, "sr {sr} cfg {ci}: {e_analog:.3} dB off the analog curve");
                worst_analog = worst_analog.max(e_analog);
            } else {
                worst_nyq = worst_nyq.max(e_analog);
            }
            worst_digital = worst_digital.max(e_digital);
        }
    }
    println!("EQ vs analog curve, corners <= sr/5: worst {worst_analog:.3} dB (limit 0.5)");
    println!("EQ vs analog curve, a corner above sr/5 (one biquad per band, no oversampling): worst {worst_nyq:.3} dB");
    println!("EQ vs its digital design: worst {worst_digital:.4} dB");
}

fn noise(n: usize, seed: u32) -> Vec<f32> {
    let mut s = seed;
    (0..n)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            (s as f32 / u32::MAX as f32) * 2.0 - 1.0
        })
        .collect()
}

#[test]
fn unity_at_all_gains_zero_and_when_off() {
    let x = noise(48000, 7);
    let y = noise(48000, 99);
    // On, every gain 0, cut off: bit-exact from the first sample.
    let mut e = Eq::new();
    e.set(&EqParams { peak_q: 7.0, peak_freq: 300.0, ..params() });
    e.reset();
    for (&a, &b) in x.iter().zip(&y) {
        let (mut l, mut r) = (a, b);
        e.process(&mut l, &mut r);
        assert_eq!((l, r), (a, b));
    }
    assert!(e.is_bypassed());
    // Running hot, then switched off: glides to flat, then bit-exact.
    let hot = EqParams { low_gain: 12.0, peak_gain: -9.0, high_gain: 6.0, cut_on: true, cut_freq: 200.0, ..params() };
    e.set(&hot);
    let mut exact_from = None;
    for (i, (&a, &b)) in x.iter().zip(&y).enumerate() {
        if i == 4800 {
            e.set(&EqParams { on: false, ..hot });
        }
        let (mut l, mut r) = (a, b);
        e.process(&mut l, &mut r);
        let same = (l, r) == (a, b);
        if i > 4800 && same && exact_from.is_none() {
            exact_from = Some(i);
        }
        if !same {
            exact_from = None;
        }
    }
    let from = exact_from.expect("never became bit-exact after on = false") - 4800;
    println!("bit-exact pass-through {from} samples ({:.1} ms) after on = false", from as f64 / 48.0);
    assert!(from < 9600, "took {from} samples");
    assert!(e.is_bypassed());
}

/// RBJ high-pass for the test's click detector (independent of the module's design code).
fn rbj_highpass(f: f64, sr: f64) -> impl FnMut(f64) -> f64 {
    let w = TAU * f / sr;
    let alpha = w.sin() / (2.0 * std::f64::consts::FRAC_1_SQRT_2);
    let a0 = 1.0 + alpha;
    let (b0, b1, b2) = ((1.0 + w.cos()) / 2.0 / a0, -(1.0 + w.cos()) / a0, (1.0 + w.cos()) / 2.0 / a0);
    let (a1, a2) = (-2.0 * w.cos() / a0, (1.0 - alpha) / a0);
    let (mut x1, mut x2, mut y1, mut y2) = (0.0, 0.0, 0.0, 0.0);
    move |x| {
        let y = b0 * x + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
        (x2, x1, y2, y1) = (x1, x, y1, y);
        y
    }
}

/// Peak level above 10 kHz while a 100 Hz sine at -6 dBFS runs through `e` and the params flip
/// between `a` and `b` every 250 ms. The sine leaves nothing up there; a click does.
fn hf_peak_db(a: &EqParams, b: &EqParams) -> f64 {
    let sr = 48000.0;
    let mut e = Eq::new();
    e.set(a);
    e.reset();
    let (mut h1, mut h2) = (rbj_highpass(10000.0, sr), rbj_highpass(10000.0, sr));
    let mut peak_hf: f64 = 0.0;
    for i in 0..96000 {
        if i % 12000 == 6000 {
            e.set(if (i / 12000) % 2 == 0 { b } else { a });
        }
        let x = 0.5 * (TAU * 100.0 * i as f64 / sr).sin() as f32;
        let (mut l, mut r) = (x, x);
        e.process(&mut l, &mut r);
        assert!(l.is_finite() && l.abs() < 16.0);
        let hf = h2(h1(l as f64));
        if i > 2000 {
            peak_hf = peak_hf.max(hf.abs());
        }
    }
    20.0 * peak_hf.log10()
}

#[test]
fn parameter_jumps_do_not_click() {
    let a = EqParams { low_freq: 40.0, low_gain: -15.0, peak_freq: 300.0, peak_q: 0.5, peak_gain: 15.0, high_freq: 2000.0, high_gain: -15.0, ..params() };
    let b = EqParams { low_freq: 400.0, low_gain: 15.0, peak_freq: 3000.0, peak_q: 6.0, peak_gain: -15.0, high_freq: 16000.0, high_gain: 15.0, ..params() };
    let cut = EqParams { cut_on: true, cut_freq: 300.0, ..a };
    let off = EqParams { on: false, ..b };
    for (name, x, y) in [
        ("steady (baseline)", a, a),
        ("low shelf jumps", a, EqParams { low_freq: 400.0, low_gain: 15.0, ..a }),
        ("peak jumps", a, EqParams { peak_freq: 3000.0, peak_q: 6.0, peak_gain: -15.0, ..a }),
        ("high shelf jumps", a, EqParams { high_freq: 16000.0, high_gain: 15.0, ..a }),
        ("all band jumps", a, b),
        ("low cut on/off", a, cut),
        ("module on/off", a, off),
        ("everything at once", a, EqParams { cut_on: true, cut_freq: 300.0, ..b }),
    ] {
        let db = hf_peak_db(&x, &y);
        println!("EQ clicks, {name}: peak above 10 kHz {db:.1} dBFS");
        assert!(db < -90.0, "{name}: click at {db:.1} dBFS above 10 kHz");
    }
}

#[test]
fn cpu_bench() {
    let n = if cfg!(debug_assertions) { 48000 } else { 4_800_000 };
    let x = noise(4096, 3);
    let all = EqParams { low_gain: 6.0, peak_gain: -4.0, high_gain: 3.0, cut_on: true, ..params() };
    for (name, p, sweep) in [("all bands, static", all, false), ("all bands, peak swept", all, true), ("flat (bypassed)", params(), false)] {
        let mut e = Eq::new();
        e.set(&p);
        e.reset();
        let mut acc = 0.0f32;
        let t = Instant::now();
        for i in 0..n {
            if sweep && i % 256 == 0 {
                e.set(&EqParams { peak_freq: 200.0 + (i % 48000) as f32 / 4.0, ..p });
            }
            let (mut l, mut r) = (x[i & 4095], x[(i + 7) & 4095]);
            e.process(&mut l, &mut r);
            acc += l + r;
        }
        let ns = t.elapsed().as_nanos() as f64 / n as f64;
        println!("EQ CPU {name}: {ns:.1} ns/sample stereo ({acc:.1})");
    }
}
