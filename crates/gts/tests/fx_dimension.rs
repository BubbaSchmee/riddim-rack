//! SPEC v2 4.5 Dimension acceptance: width grows monotonically with size (correlation falls),
//! mono sum within 1 dB of dry, sub band >= 0.95 correlated; plus the FX-standard S/M test,
//! bypass, clicks and CPU. Numbers print with `--nocapture`.
use gts::dsp::fx::dimension::*;
use std::f64::consts::TAU;
use std::time::Instant;

const SR: f64 = 48000.0;

fn params() -> DimensionParams {
    DimensionParams { on: true, ..DimensionParams::default() }
}

/// RBJ biquad, high- or low-pass, Butterworth Q.
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

fn energy(x: &[f64]) -> f64 {
    x.iter().map(|v| v * v).sum()
}

struct Noise(u32);
impl Noise {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0 as f64 / u32::MAX as f64 - 0.5
    }
}

fn saw(f: f64, i: usize) -> f64 {
    0.5 * (2.0 * (f * i as f64 / SR).fract() - 1.0)
}

/// Run `secs` of stereo input through the module; returns (in L, in R, out L, out R) after 0.5 s.
fn run(p: &DimensionParams, secs: f64, mut input: impl FnMut(usize) -> (f64, f64)) -> [Vec<f64>; 4] {
    let mut d = Dimension::new();
    d.set_sample_rate(SR);
    d.set(p);
    d.reset();
    let warm = (0.5 * SR) as usize;
    let mut out: [Vec<f64>; 4] = Default::default();
    for i in 0..warm + (secs * SR) as usize {
        let (a, b) = input(i);
        let (a, b) = (a as f32, b as f32);
        let (mut l, mut r) = (a, b);
        d.process(&mut l, &mut r);
        assert!(l.is_finite() && r.is_finite());
        if i >= warm {
            for (v, x) in out.iter_mut().zip([a, b, l, r]) {
                v.push(x as f64);
            }
        }
    }
    out
}

#[test]
fn width_grows_with_size() {
    for (name, mix) in [("noise", 1.0f32), ("noise", 0.5), ("saw", 1.0), ("saw", 0.5)] {
        let mut prev = 1.0;
        let mut line = String::new();
        for size in [0.0f32, 0.25, 0.5, 0.75, 1.0] {
            let mut n = Noise(9);
            let [_, _, l, r] = run(&DimensionParams { size, mix, ..params() }, 4.0, |i| {
                let x = if name == "noise" { n.next() } else { saw(110.0, i) };
                (x, x)
            });
            let c = corr(&l, &r);
            line += &format!(" {size:.2}:{c:.3}");
            assert!(c < prev, "{name} mix {mix}: correlation {c:.3} at size {size} not below {prev:.3}");
            prev = c;
        }
        println!("dimension {name} mix {mix}: L/R correlation by size{line}");
    }
}

#[test]
fn mono_sum_equals_dry() {
    let mut n = Noise(3);
    let mut m = Noise(77);
    for size in [0.0f32, 0.5, 1.0] {
        let [a, b, l, r] = run(&DimensionParams { size, mix: 1.0, ..params() }, 3.0, |i| {
            let x = saw(55.0, i) + 0.5 * n.next();
            (x, x * 0.7 + 0.3 * m.next())
        });
        let dry: Vec<f64> = a.iter().zip(&b).map(|(x, y)| x + y).collect();
        let wet: Vec<f64> = l.iter().zip(&r).map(|(x, y)| x + y).collect();
        let worst = dry.iter().zip(&wet).map(|(x, y)| (x - y).abs()).fold(0.0, f64::max);
        let mut band_db: f64 = 0.0;
        for f in [100.0, 300.0, 1000.0, 3000.0, 10000.0] {
            let (d, w) = (filt(&filt(&dry, true, f / 1.4), false, f * 1.4), filt(&filt(&wet, true, f / 1.4), false, f * 1.4));
            band_db = band_db.max((10.0 * (energy(&w) / energy(&d)).log10()).abs());
        }
        println!("dimension size {size}: mono sum max |wet - dry| {worst:.1e}, worst band level change {band_db:.4} dB");
        assert!(worst < 1e-6);
        assert!(band_db < 1.0);
    }
}

#[test]
fn sub_band_stays_correlated() {
    for (size, mix) in [(1.0f32, 1.0f32), (0.5, 0.5), (0.0, 1.0)] {
        let [_, _, l, r] = run(&DimensionParams { size, mix, ..params() }, 4.0, |i| {
            let x = 0.5 * (TAU * 50.0 * i as f64 / SR).sin() + saw(220.0, i);
            (x, x)
        });
        let c = corr(&filt(&l, false, 100.0), &filt(&r, false, 100.0));
        println!("dimension size {size} mix {mix}: L/R correlation below 100 Hz = {c:.4}");
        assert!(c >= 0.95, "sub correlation {c:.4}");
    }
}

#[test]
fn side_to_mid_rises_above_300_hz_only() {
    // FX standard test 6, on a slightly wide input (S/M = -20 dB): mix 50 %, default size.
    let mut n = Noise(5);
    let mut m = Noise(6);
    let [a, b, l, r] = run(&DimensionParams { mix: 0.5, ..params() }, 4.0, |_| {
        let (x, s) = (n.next(), 0.1 * m.next());
        (x + s, x - s)
    });
    let sm = |l: &[f64], r: &[f64], hp: bool, f: f64| {
        let mid: Vec<f64> = l.iter().zip(r).map(|(x, y)| 0.5 * (x + y)).collect();
        let side: Vec<f64> = l.iter().zip(r).map(|(x, y)| 0.5 * (x - y)).collect();
        // 48 dB/oct band edges: the octave just above 100 Hz is legitimately widened (cut at 120).
        let band = |x: &[f64]| filt(&filt(x, hp, f), hp, f);
        10.0 * (energy(&band(&side)) / energy(&band(&mid))).log10()
    };
    let hi = sm(&l, &r, true, 300.0) - sm(&a, &b, true, 300.0);
    let lo = sm(&l, &r, false, 100.0) - sm(&a, &b, false, 100.0);
    println!("dimension S/M change: above 300 Hz {hi:+.2} dB, below 100 Hz {lo:+.2} dB");
    assert!(hi >= 6.0);
    assert!(lo < 1.0);
}

#[test]
fn off_is_bit_exact_after_the_fade() {
    let mut d = Dimension::new();
    d.set(&DimensionParams { size: 1.0, mix: 1.0, ..params() });
    d.reset();
    let mut exact_from = None;
    for i in 0..48000usize {
        if i == 9600 {
            d.set(&DimensionParams { on: false, size: 1.0, mix: 1.0, ..params() });
        }
        let x = saw(110.0, i) as f32;
        let (mut l, mut r) = (x, 0.5 * x);
        d.process(&mut l, &mut r);
        if (l, r) == (x, 0.5 * x) {
            exact_from.get_or_insert(i);
        } else {
            exact_from = None;
        }
    }
    let from = exact_from.expect("never bit-exact") - 9600;
    println!("dimension: bit-exact pass-through {:.1} ms after on = false", from as f64 / 48.0);
    assert!(from < 9600);
    let mut d = Dimension::new();
    d.set(&DimensionParams { on: false, ..params() });
    d.reset();
    let (mut l, mut r) = (0.25f32, -0.5f32);
    d.process(&mut l, &mut r);
    assert_eq!((l, r), (0.25, -0.5));
}

#[test]
fn param_jumps_do_not_click() {
    let a = DimensionParams { size: 0.0, mix: 0.2, wet_lowcut_hz: 40.0, ..params() };
    let b = DimensionParams { size: 1.0, mix: 1.0, wet_lowcut_hz: 400.0, ..params() };
    for (name, x, y) in [("steady", b, b), ("size/mix/low cut", a, b), ("on/off", b, DimensionParams { on: false, ..b })] {
        let mut d = Dimension::new();
        d.set(&x);
        d.reset();
        let (mut h1, mut h2) = (rbj(true, 10000.0), rbj(true, 10000.0));
        let mut peak: f64 = 0.0;
        for i in 0..96000usize {
            if i % 12000 == 6000 {
                d.set(if (i / 12000) % 2 == 0 { &y } else { &x });
            }
            let s = (0.5 * (TAU * 300.0 * i as f64 / SR).sin()) as f32;
            let (mut l, mut r) = (s, s);
            d.process(&mut l, &mut r);
            let hf = h2(h1(l as f64));
            if i > 4800 {
                peak = peak.max(hf.abs());
            }
        }
        let db = 20.0 * peak.log10();
        println!("dimension clicks, {name}: peak above 10 kHz {db:.1} dBFS");
        assert!(db < -80.0, "{name}: {db:.1} dBFS");
    }
}

#[test]
fn cpu_bench() {
    let n = if cfg!(debug_assertions) { 48000 } else { 4_800_000 };
    let mut d = Dimension::new();
    d.set(&params());
    d.reset();
    let mut acc = 0.0f32;
    let t = Instant::now();
    for i in 0..n {
        let x = saw(110.0, i & 65535) as f32;
        let (mut l, mut r) = (x, x);
        d.process(&mut l, &mut r);
        acc += l - r;
    }
    let ns = t.elapsed().as_nanos() as f64 / n as f64;
    println!("dimension CPU: {ns:.1} ns/sample stereo ({acc:.1})");
}
