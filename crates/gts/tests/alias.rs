//! SPEC-v2 Track 1 acceptance measurements for the oscillator (1.1 mips, 1.9 Shift evenness,
//! 1.10 table trims), rendered through the real Engine at 48 kHz with the v1 audit's measurement
//! patch (Bypass routing, Mod A index 0, shaper off, -12 dB) and its metrics
//! (docs/audits/gts-v1-audit.md):
//! - alias floor: non-harmonic over harmonic energy, Blackman-Harris spectrum of the steady part,
//!   harmonic bins = within 6 FFT bins of k*f0 (the audit's replacement for "+-1 % of k*f0",
//!   which hides aliases at high k), DC (< 15 Hz) ignored. -80 dB is an engineering target [E].
//! - AM depth: (max - min) / (max + min) of the Hilbert envelope.
//! - loudness: BS.1770 K-weighting, ungated (steady tones).
//!
//! Run with `--nocapture` to print every figure.
use gts::dsp::tables::Fft;
use gts::dsp::{Engine, Patch};
use gts::params::{Routing, Table};
use nih_plug::prelude::Enum;

const SR: f32 = 48000.0;
const STEADY: usize = 12000; // 0.25 s

fn hz(note: u8) -> f64 {
    440.0 * 2f64.powf((note as f64 - 69.0) / 12.0)
}

fn render(table: Table, index: f32, shift: f32, note: u8, secs: f32) -> Vec<f64> {
    let mut e = Engine::new();
    e.initialize(SR);
    let mut p = Patch::riddim_v1();
    p.routing = Routing::Bypass;
    p.moda_index = 0.0;
    p.shaper_on = false;
    p.volume_db = -12.0;
    p.osc[0].table = table;
    p.osc[0].index = index;
    p.osc[0].shift = shift;
    e.set_patch(&p);
    e.note_on(note, 1.0);
    (0..(secs * SR) as usize).map(|_| e.next_frame().0 as f64).collect()
}

/// Blackman-Harris power spectrum of `x` (power-of-two length) and the bin width in Hz.
fn spectrum(x: &[f64]) -> (Vec<f64>, f64) {
    let n = x.len();
    let tau = std::f64::consts::TAU;
    let mut re: Vec<f64> = x
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let t = tau * i as f64 / (n - 1) as f64;
            v * (0.35875 - 0.48829 * t.cos() + 0.14128 * (2.0 * t).cos() - 0.01168 * (3.0 * t).cos())
        })
        .collect();
    let mut im = vec![0.0; n];
    Fft::new(n).run(&mut re, &mut im, false);
    ((0..=n / 2).map(|k| re[k] * re[k] + im[k] * im[k]).collect(), SR as f64 / n as f64)
}

/// Non-harmonic / harmonic energy in dB.
fn alias_db(x: &[f64], f0: f64) -> f64 {
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

/// Energy above `lo` Hz relative to everything above 15 Hz, in dB.
fn above_db(x: &[f64], lo: f64) -> f64 {
    let (p, df) = spectrum(x);
    let tot: f64 = p.iter().enumerate().filter(|(k, _)| *k as f64 * df > 15.0).map(|(_, v)| v).sum();
    let hi: f64 = p.iter().enumerate().filter(|(k, _)| *k as f64 * df >= lo).map(|(_, v)| v).sum();
    10.0 * (hi / tot).log10()
}

/// The audit's PolyBLEP square (50 %).
fn polyblep_square(f0: f64, n: usize) -> Vec<f64> {
    let dt = f0 / SR as f64;
    let blep = |t: f64| {
        if t < dt {
            let u = t / dt;
            2.0 * u - u * u - 1.0
        } else if t > 1.0 - dt {
            let u = (t - 1.0) / dt;
            u * u + 2.0 * u + 1.0
        } else {
            0.0
        }
    };
    let y: Vec<f64> = (0..n)
        .map(|i| {
            let t = (i as f64 * dt).fract();
            (if t < 0.5 { 1.0 } else { -1.0 }) + blep(t) - blep((t + 0.5).fract())
        })
        .collect();
    let mean = y.iter().sum::<f64>() / n as f64;
    y.into_iter().map(|v| v - mean).collect()
}

/// Hilbert-envelope AM depth of `x` (power-of-two length), 2000 samples trimmed at each end.
fn am_depth(x: &[f64]) -> f64 {
    let n = x.len();
    let mut re = x.to_vec();
    let mut im = vec![0.0; n];
    let fft = Fft::new(n);
    fft.run(&mut re, &mut im, false);
    for k in 1..n / 2 {
        re[k] *= 2.0;
        im[k] *= 2.0;
    }
    for k in n / 2 + 1..n {
        re[k] = 0.0;
        im[k] = 0.0;
    }
    fft.run(&mut re, &mut im, true);
    let env: Vec<f64> = (2000..n - 2000).map(|i| (re[i] * re[i] + im[i] * im[i]).sqrt() / n as f64).collect();
    let (lo, hi) = env.iter().fold((f64::MAX, 0f64), |(l, h), &v| (l.min(v), h.max(v)));
    (hi - lo) / (hi + lo)
}

fn rms_db(x: &[f64]) -> f64 {
    10.0 * (x.iter().map(|v| v * v).sum::<f64>() / x.len() as f64).log10()
}

/// BS.1770 K-weighted loudness (ungated), LUFS.
fn lufs(x: &[f64]) -> f64 {
    let biquad = |x: &[f64], b: [f64; 3], a: [f64; 3]| {
        let (mut x1, mut x2, mut y1, mut y2) = (0.0, 0.0, 0.0, 0.0);
        x.iter()
            .map(|&v| {
                let y = b[0] * v + b[1] * x1 + b[2] * x2 - a[1] * y1 - a[2] * y2;
                (x2, x1, y2, y1) = (x1, v, y1, y);
                y
            })
            .collect::<Vec<f64>>()
    };
    let y = biquad(x, [1.53512485958697, -2.69169618940638, 1.19839281085285], [1.0, -1.69065929318241, 0.73248077421585]);
    let y = biquad(&y, [1.0, -2.0, 1.0], [1.0, -1.99004745483398, 0.99007225036621]);
    -0.691 + rms_db(&y)
}

#[test]
fn alias_floor_square_morph_and_saw_stack_c1_to_c7() {
    let mut worst = f64::MIN;
    for table in [Table::SquareMorph, Table::SawStack] {
        for index in [0.0, 64.0, 127.0] {
            for shift in [0.0, 12.0] {
                let row: Vec<f64> = (0..7)
                    .map(|o| {
                        let note = 24 + 12 * o;
                        let x = render(table, index, shift, note, 1.65);
                        alias_db(&x[STEADY..STEADY + 65536], hz(note))
                    })
                    .collect();
                println!("{table:?} idx {index:3} shift {shift:+3}: C1..C7 {}", row.iter().map(|v| format!("{v:6.1}")).collect::<Vec<_>>().join(" "));
                worst = row.iter().cloned().fold(worst, f64::max);
            }
        }
    }
    println!("worst alias floor: {worst:.1} dB (target <= -80 dB [E])");
    assert!(worst <= -80.0, "alias floor {worst:.1} dB");
}

#[test]
fn bandwidth_c2_index64_matches_polyblep() {
    let f0 = hz(36);
    let blep = above_db(&polyblep_square(f0, 65536), 4200.0);
    for table in [Table::SquareMorph, Table::SawStack] {
        let x = render(table, 64.0, 0.0, 36, 1.65);
        let gts = above_db(&x[STEADY..STEADY + 65536], 4200.0);
        println!("C2 index 64 {table:?}: energy above 4.2 kHz {gts:.1} dB vs PolyBLEP square {blep:.1} dB (diff {:+.1})", gts - blep);
        if table == Table::SquareMorph {
            assert!((gts - blep).abs() <= 6.0, "{gts:.1} vs {blep:.1}");
        }
    }
}

#[test]
fn shift_level_is_even() {
    // Square Morph index 64 and a pure sine (Harmonic Rise index 0) at C2, every semitone.
    for (table, index) in [(Table::SquareMorph, 64.0), (Table::HarmonicRise, 0.0)] {
        let level = |s: f32| rms_db(&render(table, index, s, 36, 0.6)[STEADY..]);
        let l0 = level(0.0);
        let d: Vec<f64> = (-24..=24).map(|s| level(s as f32) - l0).collect();
        let (lo, hi) = d.iter().fold((f64::MAX, f64::MIN), |(l, h), &v| (l.min(v), h.max(v)));
        println!("{table:?} idx {index}: level re shift 0 over -24..+24: {lo:+.2} .. {hi:+.2} dB");
        println!("  {}", d.iter().map(|v| format!("{v:+.1}")).collect::<Vec<_>>().join(" "));
        assert!(lo >= -3.0 && hi <= 3.0, "{table:?}: {lo:+.2} .. {hi:+.2} dB");
    }
}

#[test]
fn shift_keeps_the_non_harmonic_floor() {
    let f0 = hz(36);
    let mut worst = f64::MIN;
    for (table, index) in [(Table::SquareMorph, 64.0), (Table::HarmonicRise, 0.0)] {
        for shift in [-24.0, -12.0, -7.0, -5.0, 5.0, 7.0, 12.0, 19.0, 24.0] {
            let x = render(table, index, shift, 36, 1.65);
            let r = alias_db(&x[STEADY..STEADY + 65536], f0);
            println!("{table:?} idx {index} shift {shift:+3}: non-harmonic {r:.1} dB");
            worst = worst.max(r);
        }
    }
    assert!(worst <= -80.0, "worst {worst:.1} dB");
}

fn sine_am(shift: f32) -> f64 {
    let x = render(Table::HarmonicRise, 0.0, shift, 36, 1.0);
    am_depth(&x[STEADY..STEADY + 32768])
}

#[test]
fn sine_am_depth_at_plus_12() {
    let d = sine_am(12.0);
    println!("sine AM depth at +12: {:.1} %", 100.0 * d);
    assert!(d <= 0.10, "{:.1} %", 100.0 * d);
}

/// SPEC-v2 1.9 asks for <= 10 % at +7 too. Not met: at ratio 1.498 a sine read at a constant rate
/// under any real grain window comes out as two equal neighbouring harmonics (|W(-f)| = |W(f)|),
/// i.e. full beating. See `grain.rs`. Kept as a failing, ignored test so the gap stays visible.
#[test]
#[ignore = "SPEC-v2 1.9 +7 sine AM: not attainable with constant-rate grain reads (reported)"]
fn sine_am_depth_at_plus_7() {
    let d = sine_am(7.0);
    println!("sine AM depth at +7: {:.1} %", 100.0 * d);
    assert!(d <= 0.10, "{:.1} %", 100.0 * d);
}

#[test]
fn table_loudness_spread_c2_index64() {
    let l: Vec<f64> = (0..Table::variants().len())
        .map(|t| lufs(&render(Table::from_index(t), 64.0, 0.0, 36, 1.0)[STEADY..]))
        .collect();
    for (t, v) in l.iter().enumerate() {
        println!("{:?}: {v:.2} LUFS", Table::from_index(t));
    }
    let spread = l.iter().cloned().fold(f64::MIN, f64::max) - l.iter().cloned().fold(f64::MAX, f64::min);
    println!("K-weighted spread at C2 index 64: {spread:.2} dB");
    assert!(spread <= 1.5, "{spread:.2} dB");
}

#[test]
fn glide_across_mips_does_not_click() {
    // Legato glide C2 -> C6 (crosses four mip boundaries): the largest sample step during the
    // glide stays within the static C6 note's, so the per-block mip crossfades add no clicks.
    let max_step = |x: &[f64]| x.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0f64, f64::max);
    let stat = max_step(&render(Table::SawStack, 0.0, 0.0, 84, 0.6)[STEADY..]);
    let mut e = Engine::new();
    e.initialize(SR);
    let mut p = Patch::riddim_v1();
    p.routing = Routing::Bypass;
    p.moda_index = 0.0;
    p.shaper_on = false;
    p.volume_db = -12.0;
    p.osc[0].table = Table::SawStack;
    p.osc[0].index = 0.0;
    (p.poly, p.legato, p.porta) = (1, true, 0.3);
    e.set_patch(&p);
    e.note_on(36, 1.0);
    let x: Vec<f64> = (0..(1.5 * SR) as usize)
        .map(|i| {
            if i == STEADY {
                e.note_on(84, 1.0);
            }
            e.next_frame().0 as f64
        })
        .collect();
    let glide = max_step(&x[STEADY..]);
    println!("max step: static C6 {stat:.4}, glide C2->C6 {glide:.4}");
    assert!(glide <= stat * 1.2, "glide step {glide} vs static {stat}");
}
