//! Filters: v1 behaviour plus SPEC v2 1.6 (note-tied, loudness-matched combs) and 1.8 (bounded
//! SVF resonance) acceptance. Measured numbers print with `--nocapture`.
use gts::dsp::filter::MultiFilter;
use gts::dsp::{Engine, Patch};
use gts::params::{FilterType, Routing};
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

/// Hann-windowed DFT magnitude at `f` (no leakage between harmonics on short windows).
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

fn rms(x: &[f32]) -> f64 {
    (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64).sqrt()
}

fn run(f: &mut MultiFilter, input: impl Fn(usize) -> f32, n: usize) -> Vec<f32> {
    (0..n).map(|i| f.process(input(i))).collect()
}

fn sine_gain(f: &mut MultiFilter, hz: f64, amp: f64) -> f64 {
    let y = run(f, |i| (amp * (TAU * hz * i as f64 / SR).sin()) as f32, 24000);
    let t = &y[12000..];
    ((t.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / t.len() as f64) * 2.0).sqrt() / amp
}

fn mk() -> MultiFilter {
    let mut f = MultiFilter::new();
    f.set_sample_rate(SR);
    f
}

/// 64-harmonic saw, RMS 0.25 (a voice at full level).
fn saw(f0: f64, n: usize) -> Vec<f32> {
    let raw: Vec<f64> =
        (0..n).map(|i| (1..=64).map(|h| (TAU * h as f64 * f0 * i as f64 / SR).sin() / h as f64).sum()).collect();
    let g = 0.25 / (raw.iter().map(|v| v * v).sum::<f64>() / n as f64).sqrt();
    raw.iter().map(|v| (v * g) as f32).collect()
}

#[test]
fn lp12_cutoff_and_rolloff() {
    // SPEC v2 1.8: res 0 is Q 0.707 (Butterworth), -3 dB at cutoff.
    let mut f = mk();
    f.set(FilterType::Lp12, 1000.0, 0.0);
    let db = 20.0 * sine_gain(&mut f, 1000.0, 1.0).log10();
    assert!((db + 3.0).abs() < 1.0, "{db}");
    f.reset();
    let db4 = 20.0 * sine_gain(&mut f, 4000.0, 1.0).log10();
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

#[test]
fn svf_res1_sings_a_clean_sine() {
    // SPEC v2 1.8: silent input + 1e-4 excitation at res 1.0 -> a sine at cutoff, THD <= 3 %.
    for ty in [FilterType::Lp12, FilterType::Bp12] {
        for fc in [100.0, 1000.0, 5000.0] {
            let mut f = mk();
            f.set(ty, fc as f32, 1.0);
            let y = run(&mut f, |i| if i == 0 { 1e-4 } else { 0.0 }, 4 * 48000);
            let t = &y[3 * 48000..];
            // Strongest bin near the cutoff (trapezoidal warping is pre-compensated by tan()).
            let (mut f1, mut best) = (fc, 0.0);
            let mut hz = fc * 0.9;
            while hz < fc * 1.1 {
                let m = mag(t, hz);
                if m > best {
                    best = m;
                    f1 = hz;
                }
                hz += fc * 0.0005;
            }
            let rest: f64 = (2..).map(|k| k as f64 * f1).take_while(|f| *f < 20000.0).map(|f| mag(t, f).powi(2)).sum();
            let thd = rest.sqrt() / best;
            let peak = t.iter().fold(0.0f32, |a, v| a.max(v.abs()));
            println!("{ty:?} fc {fc}: sings at {f1:.1} Hz, peak {peak:.3}, THD {:.2} %", 100.0 * thd);
            assert!((f1 - fc).abs() / fc < 0.02, "{ty:?} {fc}: {f1}");
            assert!(thd <= 0.03, "{ty:?} {fc}: THD {thd}");
            assert!(peak > 0.02 && peak <= 1.0, "{ty:?} {fc}: peak {peak}");
        }
    }
}

#[test]
fn svf_res1_sweep_stays_below_0dbfs() {
    // SPEC v2 1.8: a cutoff sweep at res 1.0 over a full-level saw never exceeds 0 dBFS.
    let n = 3 * 48000;
    let x = saw(65.406, n);
    for ty in [FilterType::Lp12, FilterType::Bp12] {
        for res in [0.0, 0.9, 1.0] {
            let mut f = mk();
            let mut y = Vec::with_capacity(n);
            for (i, &v) in x.iter().enumerate() {
                if i % 16 == 0 {
                    let fc = 30.0 * (15000.0f64 / 30.0).powf(i as f64 / n as f64);
                    f.set(ty, fc as f32, res);
                }
                y.push(f.process(v));
            }
            let peak = y.iter().fold(0.0f32, |a, v| a.max(v.abs()));
            println!("{ty:?} res {res}: sweep peak {peak:.3} ({:+.1} dBFS)", 20.0 * peak.log10());
            assert!(peak <= 1.0, "{ty:?} res {res}: {peak}");
        }
    }
}

#[test]
fn comb_minus_output_never_exceeds_one() {
    // Soft clip after the loudness match: a full-scale saw on the teeth at res 1.0.
    let mut f = mk();
    f.set_note(55.0);
    f.set(FilterType::CombMinus, 55.0, 1.0);
    let x = saw(55.0, 48000);
    let peak = x.iter().map(|v| f.process(v * 4.0).abs()).fold(0.0, f32::max);
    assert!(peak < 1.0, "{peak}");
}

const F1: u8 = 29;

/// Default patch with Mod A Index off (static timbre, as in the audit), note F1, left channel.
fn render(edit: impl Fn(&mut Patch)) -> Vec<f32> {
    let mut p = Patch::riddim_v1();
    p.moda_index = 0.0;
    edit(&mut p);
    let mut e = Engine::new();
    e.initialize(SR as f32);
    e.set_patch(&p);
    e.note_on(F1, 1.0);
    (0..(1.5 * SR) as usize)
        .map(|i| {
            if i % 512 == 0 {
                e.set_patch(&p);
            }
            e.next_frame().0
        })
        .collect()
}

#[test]
fn default_class_comb_keeps_the_note() {
    // SPEC v2 1.6 at F1, sustain 0.8..1.4 s: fundamental loss <= 3 dB against Bypass; Comb- res 1.0
    // within +-3 dB of res 0; no sample above 1.0.
    let f0 = 440.0 * 2f64.powf((F1 as f64 - 69.0) / 12.0);
    let sl = (0.8 * SR) as usize..(1.4 * SR) as usize;
    let byp = render(|p| p.routing = Routing::Bypass);
    let def = render(|_| {});
    let r1 = render(|p| p.filt[0].res = 1.0);
    let r0 = render(|p| p.filt[0].res = 0.0);
    let plus1 = render(|p| {
        p.filt[0].ty = FilterType::CombPlus;
        p.filt[0].res = 1.0
    });
    let db = |a: f64, b: f64| 20.0 * (a / b).log10();
    let fund = db(mag(&def[sl.clone()], f0), mag(&byp[sl.clone()], f0));
    let fund1 = db(mag(&r1[sl.clone()], f0), mag(&byp[sl.clone()], f0));
    let level = db(rms(&r1[sl.clone()]), rms(&r0[sl.clone()]));
    let level_def = db(rms(&def[sl.clone()]), rms(&byp[sl.clone()]));
    let level_plus = db(rms(&plus1[sl.clone()]), rms(&r0[sl.clone()]));
    let peak = [&def, &r1, &r0].iter().flat_map(|v| v.iter()).fold(0.0f32, |a, v| a.max(v.abs()));
    println!(
        "F1 default (Comb- res 0.6, note-tied): fundamental {fund:+.1} dB, overall {level_def:+.1} dB vs Bypass; \
         Comb- res 1.0: fundamental {fund1:+.1} dB, overall {level:+.1} dB vs res 0; Comb+ res 1.0 {level_plus:+.1} dB; peak {peak:.3}"
    );
    assert!(fund >= -3.0, "fundamental loss {fund}");
    assert!(level.abs() <= 3.0, "res 1 vs res 0: {level}");
    assert!(peak <= 1.0, "{peak}");
}
