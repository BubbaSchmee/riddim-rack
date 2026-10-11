//! SPEC-v2 wave 2 acceptance (docs/SPEC-v2-quality.md 1.4 unison, 1.5 sub + mono crossover,
//! 1.7 voice tails + steal declick, 1.10 init level, 1.11 envelope curves, 1.12 init patch,
//! quality setting), rendered through the real Engine at 48 kHz. Run with `--nocapture` for the
//! figures.
use gts::dsp::env::Adsr;
use gts::dsp::tables::Fft;
use gts::dsp::{Engine, Patch};
use gts::params::{FilterType, GtsParams, Table};
use nih_plug::prelude::Params;
use std::f64::consts::TAU;
use std::sync::atomic::Ordering;

const SR: f32 = 48000.0;
const C2: u8 = 36;
const C3: u8 = 48;
/// MIDI velocity 100.
const VEL100: f32 = 100.0 / 127.0;

fn engine(p: &Patch) -> Engine {
    let mut e = Engine::new();
    e.initialize(SR);
    e.set_patch(p);
    e
}

/// Stereo render, `set_patch` every 512 samples like a host block.
fn render(e: &mut Engine, p: &Patch, secs: f32) -> (Vec<f32>, Vec<f32>) {
    (0..(secs * SR) as usize)
        .map(|i| {
            if i % 512 == 0 {
                e.set_patch(p);
            }
            e.next_frame()
        })
        .unzip()
}

fn note(p: &Patch, n: u8, vel: f32, secs: f32) -> (Vec<f32>, Vec<f32>) {
    let mut e = engine(p);
    e.note_on(n, vel);
    render(&mut e, p, secs)
}

fn db(x: f64) -> f64 {
    20.0 * x.log10()
}

fn power(x: &[f32]) -> f64 {
    x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64
}

fn max_step(x: &[f32]) -> f32 {
    x.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0, f32::max)
}

fn peak(x: &[f32]) -> f32 {
    x.iter().fold(0.0, |m, v| m.max(v.abs()))
}

fn sec(t: f32) -> usize {
    (t * SR) as usize
}

/// Blackman-Harris power spectrum of the first power-of-two samples of `x`; bin width in Hz.
fn spectrum(x: &[f32]) -> (Vec<f64>, f64) {
    let n = 1usize << (usize::BITS - 1 - x.len().leading_zeros());
    let fft = Fft::new(n);
    let (mut re, mut im) = (vec![0.0; n], vec![0.0; n]);
    for (i, r) in re.iter_mut().enumerate() {
        let t = TAU * i as f64 / n as f64;
        let w = 0.35875 - 0.48829 * t.cos() + 0.14128 * (2.0 * t).cos() - 0.01168 * (3.0 * t).cos();
        *r = x[i] as f64 * w;
    }
    fft.run(&mut re, &mut im, false);
    ((0..n / 2).map(|k| re[k] * re[k] + im[k] * im[k]).collect(), SR as f64 / n as f64)
}

fn band(p: &[f64], df: f64, lo: f64, hi: f64) -> f64 {
    p.iter().enumerate().filter(|(k, _)| (*k as f64 * df) >= lo && (*k as f64 * df) < hi).map(|(_, v)| v).sum()
}

/// Hann-windowed DFT magnitude at `f`.
fn mag(x: &[f32], f: f64) -> f64 {
    let n = x.len() as f64;
    let (mut re, mut im) = (0.0, 0.0);
    for (i, &v) in x.iter().enumerate() {
        let w = 0.5 - 0.5 * (TAU * i as f64 / n).cos();
        let ph = TAU * f * i as f64 / SR as f64;
        re += w * v as f64 * ph.cos();
        im += w * v as f64 * ph.sin();
    }
    (re * re + im * im).sqrt() / n
}

fn hz(n: u8) -> f64 {
    440.0 * 2f64.powf((n as f64 - 69.0) / 12.0)
}

/// The init patch with only Osc A's unison in play: sub off, crossover at its 20 Hz minimum.
fn unison_patch(n: usize, width: f32, blend: f32) -> Patch {
    let mut p = Patch::default();
    p.sub_on = false;
    p.mono_xover = 20.0;
    p.osc[0].unison = n;
    p.osc[0].width = width;
    p.osc[0].blend = blend;
    p.osc[0].rand_phase = 1.0;
    p
}

// ---- 1.4 unison ---------------------------------------------------------------------------

#[test]
fn unison_width_decorrelates_the_channels() {
    let (l, r) = note(&unison_patch(7, 1.0, 0.7), C3, 1.0, 4.0);
    let (l, r) = (&l[sec(0.5)..], &r[sec(0.5)..]);
    let lr: f64 = l.iter().zip(r).map(|(a, b)| *a as f64 * *b as f64).sum();
    let corr = lr / (power(l) * power(r)).sqrt() / l.len() as f64;
    println!("unison 7, width 1, blend 0.7, C3: inter-channel correlation {corr:.3}");
    assert!(corr <= 0.3, "correlation {corr}");
    // Width 0 is mono.
    let (l, r) = note(&unison_patch(7, 0.0, 0.7), C3, 1.0, 0.5);
    assert_eq!(l, r);
}

#[test]
fn unison_blend_1_keeps_the_level_of_one_voice() {
    let level = |n: usize| {
        let (l, r) = note(&unison_patch(n, 1.0, 1.0), C3, 1.0, 4.5);
        let s = sec(0.5);
        10.0 * (0.5 * (power(&l[s..]) + power(&r[s..]))).log10()
    };
    let one = level(1);
    for n in 2..=7 {
        let d = level(n) - one;
        println!("unison {n} at blend 1: {d:+.2} dB against 1 voice");
        assert!(d.abs() <= 1.0, "unison {n}: {d:+.2} dB");
    }
}

#[test]
fn phase_reset_repeats_the_note_start_and_free_run_does_not() {
    let first_10ms = |reset: bool| {
        let mut p = unison_patch(3, 0.5, 0.7);
        p.osc[0].rand_phase = 0.5;
        p.osc[0].phase_reset = reset;
        let mut e = engine(&p);
        let mut starts = vec![];
        for _ in 0..2 {
            e.note_on(C2, VEL100);
            let (l, r) = render(&mut e, &p, 0.3);
            starts.push((l[..480].to_vec(), r[..480].to_vec()));
            e.note_off(C2);
            let mut n = 0;
            while e.active_voices() > 0 {
                render(&mut e, &p, 0.1);
                n += 1;
                assert!(n < 100, "voice never ended");
            }
            render(&mut e, &p, 0.5);
        }
        let d = starts[0].0.iter().zip(&starts[1].0).chain(starts[0].1.iter().zip(&starts[1].1));
        d.map(|(a, b)| (a - b).abs()).fold(0.0, f32::max)
    };
    let (on, off) = (first_10ms(true), first_10ms(false));
    println!("first 10 ms of two notes, max difference: phase reset {on:.2e}, free-running {off:.2e}");
    assert!(on <= 1e-6, "phase reset: {on}");
    assert!(off > 1e-3, "free-running notes should differ: {off}");
}

// ---- 1.5 sub + mono crossover ---------------------------------------------------------------

/// Init patch with both oscillators at 7 voices, width 1.
fn wide_patch() -> Patch {
    let mut p = Patch::default();
    p.osc[1].on = true;
    for o in &mut p.osc {
        o.unison = 7;
        o.width = 1.0;
        o.rand_phase = 1.0;
    }
    p
}

#[test]
fn below_the_crossover_the_output_is_mono() {
    let (l, r) = note(&wide_patch(), C2, VEL100, 3.0);
    let s = sec(0.5);
    let side: Vec<f32> = l[s..].iter().zip(&r[s..]).map(|(a, b)| a - b).collect();
    let mid: Vec<f32> = l[s..].iter().zip(&r[s..]).map(|(a, b)| a + b).collect();
    let (ps, df) = spectrum(&side);
    let (pm, _) = spectrum(&mid);
    let low = 10.0 * (band(&ps, df, 15.0, 120.0) / band(&pm, df, 15.0, 120.0)).log10();
    let high = 10.0 * (band(&ps, df, 400.0, 20000.0) / band(&pm, df, 400.0, 20000.0)).log10();
    println!("width 1 on both oscs, C2: L-R vs L+R below 120 Hz {low:.1} dB, above 400 Hz {high:.1} dB");
    assert!(low <= -40.0, "below 120 Hz: {low:.1} dB");
    assert!(high > -15.0, "the crossover must leave the stereo image above it: {high:.1} dB");
}

#[test]
fn sweeping_the_crossover_does_not_click() {
    let p = wide_patch();
    let mut e = engine(&p);
    e.note_on(C2, VEL100);
    let (steady, _) = render(&mut e, &p, 1.0);
    let steady = max_step(&steady[sec(0.5)..]);
    let mut q = p;
    let mut x = vec![];
    for b in 0..(SR as usize / 64) {
        // 20 -> 300 -> 20 Hz in one second, a new value every 64 samples.
        let t = b as f32 * 64.0 / SR;
        q.mono_xover = 20.0 + 280.0 * (1.0 - (2.0 * t - 1.0).abs());
        e.set_patch(&q);
        for _ in 0..64 {
            x.push(e.next_frame().0);
        }
    }
    let sweep = max_step(&x);
    println!("crossover sweep 20 -> 300 -> 20 Hz: max step {sweep:.4}, steady {steady:.4}");
    assert!(sweep <= 1.25 * steady, "sweep step {sweep} vs steady {steady}");
}

#[test]
fn sub_is_a_clean_sine() {
    let mut p = Patch::default();
    p.osc[0].on = false;
    for (n, oct) in [(C2, -1), (C3, -1), (C3, -2)] {
        p.sub_oct = oct;
        let (l, r) = note(&p, n, VEL100, 2.5);
        assert_eq!(l, r, "the sub is mono");
        let x = &l[sec(0.5)..];
        let f0 = hz(n) * if oct == -1 { 0.5 } else { 0.25 };
        let h1 = mag(x, f0);
        let rest = (2..=10).map(|h| mag(x, f0 * h as f64).powi(2)).sum::<f64>().sqrt();
        let thd = 100.0 * rest / h1;
        println!("sub at {f0:.2} Hz: THD {thd:.4} %, level {:.1} dBFS RMS", 10.0 * power(x).log10());
        assert!(thd <= 0.1, "THD {thd} %");
    }
}

// ---- 1.7 voice tails, steal declick ---------------------------------------------------------

/// Plays F1 for 0.3 s with Filter A at `ty` / `res`, releases, and runs until the voice ends.
/// Returns (seconds after note-off, steady max step of the ringing tail in the 0.3 s before the
/// end, max step from 10 ms before the end to 100 ms after it, output peak 50..100 ms after it).
/// The output trails the voice by the crossover's group delay (~7 ms), so the end is judged on
/// the output around and after it, not on one sample.
fn tail(ty: FilterType, freq: f32, res: f32) -> (f32, f32, f32, f32) {
    let mut p = Patch::default();
    p.sub_on = false;
    p.filt[0].ty = ty;
    p.filt[0].freq = freq;
    p.filt[0].res = res;
    p.filt[0].env = false;
    let mut e = engine(&p);
    e.note_on(29, 1.0);
    render(&mut e, &p, 0.3);
    e.note_off(29);
    let mut x = vec![];
    let mut end = None;
    for i in 0..sec(10.0) {
        if i % 512 == 0 {
            e.set_patch(&p);
        }
        x.push(e.next_frame().0);
        if end.is_none() && e.active_voices() == 0 {
            end = Some(i);
        }
        if end.is_some_and(|n| i >= n + sec(0.1)) {
            break;
        }
    }
    let n = end.unwrap_or_else(|| panic!("{ty:?} res {res}: voice still active after 10 s"));
    let ringing = max_step(&x[n.saturating_sub(sec(0.3))..n - sec(0.01)]);
    (n as f32 / SR, ringing, max_step(&x[n - sec(0.01)..]), peak(&x[n + sec(0.05)..]))
}

#[test]
fn voices_ring_out_their_filters() {
    for (ty, freq, res) in [(FilterType::Lp12, 400.0, 1.0), (FilterType::CombMinus, 880.0, 1.0), (FilterType::Lp12, 2000.0, 0.15)] {
        let (t, ringing, at_end, after) = tail(ty, freq, res);
        let after_db = db(after as f64);
        println!("{ty:?} {freq} Hz res {res}: voice ends {t:.2} s after note-off; step at the end {at_end:.2e} (tail {ringing:.2e}); output 50 ms later {after_db:.1} dBFS");
        // A capped tail ends in a 5 ms fade: its slope adds a little to the ringing's own steps.
        assert!(at_end <= 1.25 * ringing.max(1e-5), "{ty:?} res {res}: the end steps {at_end} against a tail of {ringing}");
        assert!(after_db <= -80.0, "{ty:?} res {res}: {after} after the end");
    }
}

/// (steady max step before `event`, max step in the 20 ms after it), left channel.
fn event_step(p: &Patch, before: impl Fn(&mut Engine), event: impl Fn(&mut Engine)) -> (f32, f32) {
    let mut e = engine(p);
    before(&mut e);
    let (a, _) = render(&mut e, p, 0.6);
    event(&mut e);
    let (b, _) = render(&mut e, p, 0.02);
    let mut joined = vec![a[a.len() - 1]];
    joined.extend_from_slice(&b);
    (max_step(&a[sec(0.3)..]), max_step(&joined))
}

#[test]
fn note_off_at_1ms_release_does_not_click() {
    let mut p = Patch::default();
    p.osc[0].att = 0.001;
    p.osc[0].rel = 0.001;
    p.fenv_amt = 0.0; // static filter: the step is the envelope's alone
    let (steady, off) = event_step(&p, |e| e.note_on(C2, VEL100), |e| e.note_off(C2));
    println!("rel 1 ms note-off: step {off:.4}, steady {steady:.4} ({:.2}x)", off / steady);
    assert!(off <= 1.5 * steady, "note-off step {off} vs steady {steady}");
}

#[test]
fn voice_steal_is_declicked() {
    // Poly 2 with two held notes: a third note steals the older, still-sounding voice.
    // Filter env off: a retriggered env legitimately opens the filter (brighter edges, not a click).
    let mut p = Patch::default();
    p.poly = 2;
    p.fenv_amt = 0.0;
    let (steady, steal) = event_step(
        &p,
        |e| {
            e.note_on(C2, VEL100);
            e.note_on(C2 + 7, VEL100);
        },
        |e| e.note_on(C2 + 3, VEL100),
    );
    println!("poly steal: step {steal:.4}, steady {steady:.4} ({:.2}x)", steal / steady);
    assert!(steal <= 1.5 * steady, "steal step {steal} vs steady {steady}");
    // Mono retrigger (legato off) takes the same path.
    p.poly = 1;
    p.legato = false;
    let (steady, re) = event_step(&p, |e| e.note_on(C2, VEL100), |e| e.note_on(C2 + 5, VEL100));
    println!("mono retrigger: step {re:.4}, steady {steady:.4} ({:.2}x)", re / steady);
    assert!(re <= 1.5 * steady, "retrigger step {re} vs steady {steady}");
}

// ---- 1.11 envelope curves -------------------------------------------------------------------

/// The v1 envelope, verbatim (attack chases 1.3, decay tau d/5, release chases -0.01, tau r/5).
struct V1 {
    stage: u8,
    level: f32,
    a: f32,
    d: f32,
    r: f32,
    sus: f32,
}

impl V1 {
    fn new(att: f32, dec: f32, sus: f32, rel: f32) -> Self {
        let coef = |tau: f32| (-1.0 / (tau.max(1e-5) * SR)).exp();
        Self { stage: 0, level: 0.0, a: coef(att / -(1.0 - 1.0 / 1.3f32).ln()), d: coef(dec / 5.0), r: coef(rel / 5.0), sus }
    }
    fn next(&mut self) -> f32 {
        match self.stage {
            1 => {
                self.level = 1.3 + (self.level - 1.3) * self.a;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = 2;
                }
            }
            2 => {
                self.level = self.sus + (self.level - self.sus) * self.d;
                if self.level - self.sus < 1e-4 {
                    self.level = self.sus;
                    self.stage = 3;
                }
            }
            3 => self.level += (self.sus - self.level) * 0.002,
            4 => {
                self.level = -0.01 + (self.level + 0.01) * self.r;
                if self.level <= 0.0 {
                    self.level = 0.0;
                    self.stage = 0;
                }
            }
            _ => {}
        }
        self.level
    }
}

#[test]
fn curve_0_is_the_v1_envelope_bit_for_bit() {
    for (a, d, s, r) in [(0.001, 0.3, 1.0, 0.06), (0.1, 0.25, 0.2, 0.1), (0.005, 1.0, 0.5, 2.0)] {
        let mut v1 = V1::new(a, d, s, r);
        let mut e = Adsr::default();
        e.set_curved(a, d, s, r, 0.0, SR);
        v1.stage = 1;
        e.gate_on();
        for i in 0..sec(3.0) {
            if i == sec(1.5) {
                v1.stage = 4;
                e.gate_off();
            }
            assert_eq!(e.next().to_bits(), v1.next().to_bits(), "A {a} D {d} S {s} R {r}: sample {i}");
        }
    }
}

#[test]
fn curves_bend_the_segments_and_keep_their_times() {
    let attack_at_half = |c: f32| {
        let mut e = Adsr::default();
        e.set_curved(0.1, 0.3, 0.5, 0.2, c, SR);
        e.gate_on();
        let mut half = 0.0;
        let mut done = 0;
        for i in 1..sec(0.2) {
            let v = e.next();
            if i == sec(0.05) {
                half = v;
            }
            if v >= 1.0 && done == 0 {
                done = i;
            }
        }
        (half, done as f32 / SR)
    };
    let (log, t_log) = attack_at_half(-1.0);
    let (exp, t_exp) = attack_at_half(0.0);
    let (lin, t_lin) = attack_at_half(1.0);
    println!("attack 100 ms at 50 %: log {log:.3} / exp {exp:.3} / lin {lin:.3}; lands at {t_log:.4} / {t_exp:.4} / {t_lin:.4} s");
    assert!(log > exp && exp > lin && (lin - 0.5).abs() < 0.05, "{log} {exp} {lin}");
    for t in [t_log, t_exp, t_lin] {
        assert!((t - 0.1).abs() < 0.001, "attack lands at {t}");
    }
    // Release from 1.0 ends at ~0.92 x R for every curve.
    for c in [-1.0, -0.5, 0.0, 0.5, 1.0] {
        let mut e = Adsr::default();
        e.set_curved(0.001, 0.001, 1.0, 0.5, c, SR);
        e.gate_on();
        for _ in 0..sec(0.05) {
            e.next();
        }
        e.gate_off();
        let n = (1..).find(|_| e.next() <= 0.0).unwrap();
        let t = n as f32 / SR / 0.5;
        assert!((t - 0.923).abs() < 0.01, "curve {c}: release ends at {t} x R");
    }
}

#[test]
fn attack_1ms_note_on_does_not_click() {
    let mut p = Patch::default();
    p.osc[0].att = 0.001;
    p.fenv_amt = 0.0; // static filter: the step is the envelope's alone
    let (l, _) = note(&p, C2, VEL100, 0.6);
    let (on, steady) = (max_step(&l[..sec(0.02)]), max_step(&l[sec(0.3)..]));
    println!("attack 1 ms note-on: step {on:.4}, steady {steady:.4} ({:.2}x)", on / steady);
    assert!(on <= 2.0 * steady, "note-on step {on} vs steady {steady}");
}

// ---- 1.10 / 1.12 init patch ---------------------------------------------------------------

#[test]
fn init_patch_first_note_is_a_calibrated_clean_bass() {
    let p = Patch::default();
    assert_eq!(p.osc[0].table, Table::SquareMorph);
    let (l, r) = note(&p, C2, VEL100, 1.0);
    let pk = peak(&l).max(peak(&r));
    let (pw, df) = spectrum(&l);
    // Amplitude-weighted spectral centroid of the whole first second (DC excluded).
    let amp: Vec<f64> = pw.iter().map(|v| v.sqrt()).collect();
    let centroid = amp.iter().enumerate().skip(1).map(|(k, a)| k as f64 * df * a).sum::<f64>() / amp[1..].iter().sum::<f64>();
    let (on, steady) = (max_step(&l[..sec(0.02)]), max_step(&l[sec(0.5)..]));
    let bright = max_step(&l[sec(0.02)..sec(0.06)]);
    println!(
        "init C2 vel 100: peak {:.2} dBFS ({pk:.3}), RMS {:.1} dBFS, centroid {centroid:.0} Hz, note-on step {:.2}x steady, {:.2}x 20-60 ms",
        db(pk as f64),
        10.0 * power(&l).log10(),
        on / steady,
        on / bright
    );
    assert!((-12.0..=-6.0).contains(&db(pk as f64)), "peak {} dBFS", db(pk as f64));
    assert!((150.0..=600.0).contains(&centroid), "centroid {centroid} Hz");
    assert!(pk <= 0.95);
    // No click at the note-on: the attack's steps stay within 2x those of the bright phase right
    // after it (20-60 ms). Against the dark sustain they are ~4x because the filter envelope opens
    // the LP to ~9 kHz; the envelope alone is tested with a static filter in
    // `attack_1ms_note_on_does_not_click`.
    assert!(on <= 2.0 * bright, "note-on step {on} vs bright phase {bright} (sustain {steady})");
}

// ---- quality setting ------------------------------------------------------------------------

#[test]
fn quality_setting_persists_and_sets_the_latency() {
    let p = GtsParams::default();
    assert_eq!(p.oversampling.load(Ordering::Relaxed), 2);
    p.oversampling.store(4, Ordering::Relaxed);
    let saved = p.serialize_fields();
    let q = GtsParams::default();
    q.deserialize_fields(&saved);
    assert_eq!(q.oversampling.load(Ordering::Relaxed), 4);

    let mut e = engine(&Patch::default());
    for (f, lat) in [(1, 0), (2, 21), (4, 23)] {
        e.set_oversampling(f);
        assert_eq!((e.oversampling(), e.latency()), (f, lat));
    }
}


