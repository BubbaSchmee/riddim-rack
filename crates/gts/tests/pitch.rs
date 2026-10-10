//! SPEC Verification 1: pitch (Shift never moves pitch), shift (centroid ordering), index (spectra
//! differ). Renders A1 = 55 Hz at 48 kHz through the engine (filters are not in the path yet).
use gts::dsp::{Engine, Patch};
use gts::params::Table;
use nih_plug::prelude::Enum;

const SR: f32 = 48000.0;
const F0: f64 = 55.0;
const A1: u8 = 33;

fn render(table: Table, index: f32, shift: f32, secs: f32) -> Vec<f32> {
    let mut e = Engine::new();
    e.initialize(SR);
    let mut p = Patch::default();
    p.osc[0].table = table;
    p.osc[0].index = index;
    p.osc[0].shift = shift;
    e.set_patch(&p);
    e.note_on(A1, 1.0);
    (0..(secs * SR) as usize).map(|_| e.next_frame().0).collect()
}

/// Fundamental via the normalized square difference function: the shortest lag at which the
/// signal repeats (NSDF within 0.005 of the best lag). A Shift that moved the pitch up would
/// repeat at a shorter lag; one that broke periodicity would fail the `rmax` check.
fn pitch(x: &[f32]) -> f64 {
    let w = 8192;
    let (lo, hi) = (96, 1600); // 500 Hz .. 30 Hz
    let r: Vec<f64> = (0..=hi + 1)
        .map(|lag| {
            if lag < lo - 1 {
                return 0.0;
            }
            let (mut ac, mut m) = (0.0f64, 0.0f64);
            for n in 0..w {
                let (a, b) = (x[n] as f64, x[n + lag] as f64);
                ac += a * b;
                m += a * a + b * b;
            }
            2.0 * ac / m.max(1e-12)
        })
        .collect();
    let rmax = r[lo..=hi].iter().cloned().fold(f64::MIN, f64::max);
    assert!(rmax > 0.95, "not periodic: best NSDF {rmax}");
    let lag = (lo..=hi)
        .find(|&l| r[l] >= rmax - 0.005 && r[l] >= r[l - 1] && r[l] >= r[l + 1])
        .unwrap();
    let (a, b, c) = (r[lag - 1], r[lag], r[lag + 1]);
    let d = 0.5 * (a - c) / (a - 2.0 * b + c);
    SR as f64 / (lag as f64 + if d.is_finite() { d } else { 0.0 })
}

/// Magnitudes at harmonics 1..=k of 55 Hz (Goertzel over a whole number of cycles).
fn harmonics(x: &[f32], k: usize) -> Vec<f64> {
    (1..=k)
        .map(|h| {
            let c = 2.0 * (std::f64::consts::TAU * h as f64 * F0 / SR as f64).cos();
            let (mut s1, mut s2) = (0.0f64, 0.0f64);
            for &v in x {
                let s = v as f64 + c * s1 - s2;
                s2 = s1;
                s1 = s;
            }
            (s1 * s1 + s2 * s2 - c * s1 * s2).max(0.0).sqrt()
        })
        .collect()
}

/// The steady second of a 2 s render.
fn steady(x: &[f32]) -> &[f32] {
    &x[SR as usize..]
}

#[test]
fn shift_never_changes_pitch() {
    for ti in 0..Table::variants().len() {
        let table = Table::from_index(ti);
        for index in [0.0, 64.0, 127.0] {
            for shift in [-12.0, 0.0, 12.0] {
                let x = render(table, index, shift, 2.0);
                let f = pitch(steady(&x));
                assert!((f / F0 - 1.0).abs() < 0.01, "{table:?} index {index} shift {shift}: {f:.2} Hz");
            }
        }
    }
}

#[test]
fn shift_zero_is_exactly_periodic() {
    // Single-cycle playback: one period later the waveform repeats (fractional period, so compare
    // against linear interpolation).
    let x = render(Table::Vowel, 40.0, 0.0, 1.0);
    let period = SR as f64 / F0;
    let s = &x[24000..];
    let mut err = 0f64;
    for n in 0..4000 {
        let t = n as f64 + period;
        let (i, f) = (t as usize, t.fract());
        let y = s[i] as f64 * (1.0 - f) + s[i + 1] as f64 * f;
        err = err.max((y - s[n] as f64).abs());
    }
    assert!(err < 0.02, "max deviation one period later: {err}");
}

fn centroid(m: &[f64]) -> f64 {
    m.iter().enumerate().map(|(i, v)| (i + 1) as f64 * v).sum::<f64>() / m.iter().sum::<f64>()
}

#[test]
fn shift_moves_spectral_centroid() {
    let c = |shift| centroid(&harmonics(steady(&render(Table::SquareMorph, 64.0, shift, 2.0)), 200));
    let (down, zero, up) = (c(-12.0), c(0.0), c(12.0));
    println!("centroid (harmonic number): -12 {down:.2}, 0 {zero:.2}, +12 {up:.2}");
    assert!(up > zero && zero > down, "{down} {zero} {up}");
}

#[test]
fn index_changes_spectrum_for_every_table() {
    let unit = |m: Vec<f64>| {
        let n = m.iter().map(|v| v * v).sum::<f64>().sqrt();
        m.into_iter().map(|v| v / n).collect::<Vec<_>>()
    };
    for ti in 0..Table::variants().len() {
        let table = Table::from_index(ti);
        let a = unit(harmonics(steady(&render(table, 0.0, 0.0, 2.0)), 64));
        let b = unit(harmonics(steady(&render(table, 127.0, 0.0, 2.0)), 64));
        let d = a.iter().zip(&b).map(|(x, y)| (x - y).powi(2)).sum::<f64>().sqrt();
        println!("{table:?}: spectral distance index 0 vs 127 = {d:.3}");
        assert!(d > 0.2, "{table:?}: distance {d}");
    }
}


#[test]
fn shift_sweep_does_not_click() {
    // Largest sample-to-sample step while Shift sweeps -24 -> +24 must stay within what the
    // static extremes already produce: no discontinuities from the sweep itself.
    let max_step = |x: &[f32]| x.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0f32, f32::max);
    let stat = [-24.0, 0.0, 24.0]
        .iter()
        .map(|&s| max_step(steady(&render(Table::SquareMorph, 64.0, s, 2.0))))
        .fold(0f32, f32::max);
    let mut e = Engine::new();
    e.initialize(SR);
    let mut p = Patch::default();
    e.set_patch(&p);
    e.note_on(A1, 1.0);
    let mut x = vec![];
    for block in 0..1500 {
        p.osc[0].shift = -24.0 + 48.0 * block as f32 / 1500.0;
        e.set_patch(&p);
        x.extend((0..64).map(|_| e.next_frame().0));
    }
    let sweep = max_step(&x[4800..]);
    println!("max step: static {stat:.4}, sweep {sweep:.4}");
    assert!(sweep <= stat * 1.2, "sweep step {sweep} vs static {stat}");
}
