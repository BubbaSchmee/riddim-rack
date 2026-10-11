//! Integration of modulators, shaper, filters and routing through the full engine (SPEC
//! "Modulators", "Filters", "Routing", Verification 1 safety). Std only.
use gts::dsp::{Engine, Patch, Transport};
use gts::params::{Curve, FilterType, Routing, ShaperMode, Table};
use nih_plug::prelude::Enum;
use std::f64::consts::TAU;

const SR: f32 = 48000.0;

fn engine(p: &Patch) -> Engine {
    let mut e = Engine::new();
    e.initialize(SR);
    e.set_patch(p);
    e
}

/// Left channel, `set_patch` every 512 samples like a host block.
fn render(e: &mut Engine, p: &Patch, secs: f32) -> Vec<f32> {
    (0..(secs * SR) as usize)
        .map(|i| {
            if i % 512 == 0 {
                e.set_patch(p);
            }
            e.next_frame().0
        })
        .collect()
}

fn note(p: &Patch, n: u8, secs: f32) -> Vec<f32> {
    let mut e = engine(p);
    e.note_on(n, 1.0);
    render(&mut e, p, secs)
}

fn hz(n: u8) -> f64 {
    440.0 * 2f64.powf((n as f64 - 69.0) / 12.0)
}

/// DFT magnitude at `f` (Hann-windowed so short windows do not leak between harmonics).
fn mag(x: &[f32], f: f64) -> f64 {
    let n = x.len() as f64;
    let (mut re, mut im) = (0.0, 0.0);
    for (i, &v) in x.iter().enumerate() {
        let w = 0.5 - 0.5 * (TAU * i as f64 / n).cos();
        let p = TAU * f * i as f64 / SR as f64;
        re += w * v as f64 * p.cos();
        im += w * v as f64 * p.sin();
    }
    (re * re + im * im).sqrt() / n
}

fn harmonics(x: &[f32], f0: f64, k: usize) -> Vec<f64> {
    (1..=k).map(|h| mag(x, f0 * h as f64)).collect()
}

fn unit(m: Vec<f64>) -> Vec<f64> {
    let n = m.iter().map(|v| v * v).sum::<f64>().sqrt().max(1e-12);
    m.into_iter().map(|v| v / n).collect()
}

fn distance(a: &[f32], b: &[f32], f0: f64) -> f64 {
    let (a, b) = (unit(harmonics(a, f0, 40)), unit(harmonics(b, f0, 40)));
    a.iter().zip(&b).map(|(x, y)| (x - y).powi(2)).sum::<f64>().sqrt()
}

fn centroid(x: &[f32], f0: f64) -> f64 {
    let m = harmonics(x, f0, 60);
    m.iter().enumerate().map(|(i, v)| (i + 1) as f64 * v).sum::<f64>() / m.iter().sum::<f64>()
}

fn rms(x: &[f32]) -> f64 {
    (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64).sqrt()
}

#[test]
fn mod_a_one_shot_into_index_changes_spectrum() {
    let mut p = Patch::riddim_v1(); // Mod A one-shot Decay 1/8 into Index +0.5
    p.routing = Routing::Bypass; // the index alone, no filter movement
    let (n, f0) = (45, hz(45));
    let d = |p: &Patch| {
        let x = note(p, n, 1.0);
        distance(&x[..4800], &x[x.len() - 4800..], f0)
    };
    let wobble = d(&p);
    p.moda_index = 0.0;
    let still = d(&p);
    println!("first vs last 100 ms spectral distance: one-shot {wobble:.3}, none {still:.3}");
    assert!(wobble > 0.2 && wobble > 3.0 * still, "one-shot {wobble}, none {still}");
}

#[test]
fn filter_env_moves_lp12_cutoff() {
    let mut p = Patch::riddim_v1();
    p.moda_index = 0.0;
    p.filt[0].ty = FilterType::Lp12;
    p.filt[0].freq = 200.0;
    p.filt[0].res = 0.0;
    p.filt[0].kbd = 0.0;
    let (n, f0) = (45, hz(45));
    let ratio = |p: &Patch| {
        let x = note(p, n, 1.0);
        centroid(&x[..2400], f0) / centroid(&x[24000..], f0)
    };
    let env = ratio(&p);
    p.filt[0].env = false;
    let flat = ratio(&p);
    println!("centroid first 50 ms / steady: env {env:.2}, env off {flat:.2}");
    assert!(env > 1.5, "brightness should decay with the filter env: {env}");
    assert!((flat - 1.0).abs() < 0.2, "no env, no movement: {flat}");
}

#[test]
fn comb_minus_peaks_follow_the_voice_harmonics() {
    // The osc two octaves down puts a dense harmonic comb under the voice; Comb- at Freq C4 with
    // Kbd 1 sits exactly on the voice pitch, so harmonics at k * f(note) stand out of the ones
    // between, for every note.
    let mut p = Patch::riddim_v1();
    p.moda_index = 0.0;
    p.osc[0].table = Table::SawStack;
    p.osc[0].index = 0.0;
    p.osc[0].oct = -2;
    p.filt[0].freq = 261.63;
    p.filt[0].kbd = 1.0;
    p.filt[0].env = false;
    for n in [45u8, 50] {
        let f = hz(n);
        let contrast = |p: &Patch| {
            let x = note(p, n, 1.0);
            let s = &x[24000..];
            let peaks: f64 = (1..=8).map(|k| mag(s, f * k as f64)).sum();
            let between: f64 =
                (1..=8).map(|k| 0.5 * (mag(s, f * k as f64 - f / 2.0) + mag(s, f * k as f64 + f / 2.0))).sum();
            peaks / between
        };
        let comb = contrast(&p);
        let mut q = p;
        q.routing = Routing::Bypass;
        let dry = contrast(&q);
        println!("note {n}: peak/between ratio comb {comb:.2}, dry {dry:.2}");
        assert!(comb > 2.5 * dry, "note {n}: comb {comb}, dry {dry}");
    }
}

#[test]
fn routings_sound_different() {
    let mut p = Patch::riddim_v1();
    p.osc[1].on = true;
    p.shaper_on = true;
    let outs: Vec<Vec<f32>> = [Routing::Parallel, Routing::Series, Routing::Bypass]
        .iter()
        .map(|&r| {
            p.routing = r;
            note(&p, 33, 0.5)
        })
        .collect();
    for (i, j) in [(0, 1), (0, 2), (1, 2)] {
        let diff: Vec<f32> = outs[i].iter().zip(&outs[j]).map(|(a, b)| a - b).collect();
        let rel = rms(&diff) / rms(&outs[i]).max(rms(&outs[j]));
        println!("routing {i} vs {j}: relative difference {rel:.3}");
        assert!(rel > 0.1, "routings {i} and {j} too similar: {rel}");
    }
}

#[test]
fn lfo_sync_follows_transport_tempo() {
    // 150 BPM, 1/8 -> 0.2 s = 9600 samples, with the transport playing (beat-locked) and stopped
    // (free-running at host tempo).
    let mut p = Patch::riddim_v1();
    p.moda.curve = Curve::Sine;
    p.moda.oneshot = false;
    p.routing = Routing::Bypass;
    for playing in [true, false] {
        let mut e = Engine::new();
        e.initialize(SR);
        e.transport = Transport { tempo: 150.0, playing, pos_beats: 0.0 };
        e.set_patch(&p);
        e.note_on(93, 1.0); // 1760 Hz: a new grain (and index sample) every 27 samples
        let mut trace = vec![];
        for i in 0..(2.0 * SR) as usize {
            if i % 512 == 0 {
                e.set_patch(&p);
            }
            e.next_frame();
            trace.push(e.voice(0).index(0));
        }
        // Upward crossings of the centre index (64) with hysteresis.
        let (mut ups, mut armed) = (vec![], false);
        for (i, &v) in trace.iter().enumerate() {
            if v < 50.0 {
                armed = true;
            } else if armed && v >= 64.0 {
                ups.push(i);
                armed = false;
            }
        }
        let gaps: Vec<f64> = ups.windows(2).map(|w| (w[1] - w[0]) as f64).collect();
        let mean = gaps.iter().sum::<f64>() / gaps.len() as f64;
        println!("playing={playing}: {} cycles, mean period {mean:.1} samples", gaps.len());
        assert!(gaps.len() >= 8);
        assert!((mean - 9600.0).abs() < 10.0, "period {mean}");
        assert!(gaps.iter().all(|g| (g - 9600.0).abs() <= 60.0), "{gaps:?}");
    }
}

#[test]
fn eight_voices_at_extremes_stay_finite_and_bounded() {
    let mut p = Patch::riddim_v1();
    p.poly = 8;
    p.volume_db = 6.0;
    p.osc[1].on = true;
    p.osc[0].shift = 24.0;
    p.osc[1].shift = -24.0;
    p.shaper_on = true;
    p.shaper_mode = ShaperMode::Quant;
    p.shaper_amt = 1.0; // 2 bit
    p.moda_pitch = 1.0;
    p.moda_index = 1.0;
    p.moda_shift = 1.0;
    p.modb_cutoff = 1.0;
    p.modb_moda = 1.0;
    p.modb.curve = Curve::Square;
    p.fenv_amt = 1.0;
    for f in &mut p.filt {
        f.res = 1.0;
        f.freq = 20000.0;
    }
    let types = FilterType::variants().len();
    for routing in [Routing::Parallel, Routing::Series, Routing::Bypass] {
        for ti in 0..types {
            p.routing = routing;
            p.filt[0].ty = FilterType::from_index(ti);
            p.filt[1].ty = FilterType::from_index((ti + 1) % types);
            let mut e = engine(&p);
            for n in 0..8 {
                e.note_on(24 + 9 * n, 1.0);
            }
            let mut peak = 0f32;
            for i in 0..(0.4 * SR) as usize {
                if i % 512 == 0 {
                    e.set_patch(&p);
                }
                let (l, r) = e.next_frame();
                assert!(l.is_finite() && r.is_finite(), "{routing:?} {:?}: NaN/inf", p.filt[0].ty);
                peak = peak.max(l.abs()).max(r.abs());
            }
            assert!(peak <= 1.0, "{routing:?} {:?}: peak {peak}", p.filt[0].ty);
        }
    }
}

/// CPU check, run in release: cargo test -p gts --release --test integration -- --ignored --nocapture
#[test]
#[ignore]
fn cpu_eight_voices() {
    let mut p = Patch::riddim_v1();
    p.poly = 8;
    p.osc[1].on = true;
    p.shaper_on = true;
    p.routing = Routing::Series;
    let mut e = engine(&p);
    for n in 0..8 {
        e.note_on(29 + 5 * n, 1.0);
    }
    let secs = 10.0f64;
    let t = std::time::Instant::now();
    let mut acc = 0f32;
    for i in 0..(secs * SR as f64) as usize {
        if i % 512 == 0 {
            e.set_patch(&p);
        }
        let (l, r) = e.next_frame();
        acc += l + r;
    }
    let cpu = t.elapsed().as_secs_f64();
    assert_eq!(e.active_voices(), 8);
    println!("8 voices, 10 s at 48 kHz in {cpu:.3} s: {:.1}x real time (checksum {acc})", secs / cpu);
    assert!(cpu < secs);
}

#[test]
fn synced_lfo_does_not_step_after_an_off_grid_note_on() {
    // Sine 1/8 at 150 BPM into pitch, note-on mid-block and off the division grid: the pitch
    // must move smoothly across the next block boundary (no re-lock jump).
    let mut p = Patch::riddim_v1();
    p.moda.curve = Curve::Sine;
    p.moda.oneshot = false;
    p.moda_pitch = 0.25;
    p.moda_index = 0.0;
    p.porta = 0.0;
    let mut e = Engine::new();
    e.initialize(SR);
    e.transport = Transport { tempo: 150.0, playing: true, pos_beats: 0.25 };
    e.set_patch(&p);
    for _ in 0..100 {
        e.next_frame();
    }
    e.note_on(45, 1.0);
    let st = |e: &Engine| 12.0 * (e.voice(0).freq(0) / 440.0).log2();
    e.next_frame();
    let mut last = st(&e);
    let mut worst = 0f32;
    for i in 101..2048 {
        if i % 512 == 0 {
            e.set_patch(&p);
        }
        e.next_frame();
        let now = st(&e);
        worst = worst.max((now - last).abs());
        last = now;
    }
    // Sine slope peaks at 6 st * 2pi / 9600 samples ~ 0.004 st per sample.
    assert!(worst < 0.02, "pitch stepped {worst} st in one sample");
}

/// Largest one-sample step of the left channel: (steady before, first 20 ms after `after`
/// replaces `before` mid-note, steady after).
fn switch_steps(before: &Patch, after: &Patch) -> (f32, f32, f32) {
    let mut e = engine(before);
    e.note_on(34, 1.0); // 0.5 s is 29.1 cycles: the switch lands off a zero crossing
    let a = render(&mut e, before, 0.5);
    let b = render(&mut e, after, 0.5);
    let step = |x: &[f32]| x.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0f32, f32::max);
    let n = (0.02 * SR) as usize;
    let mut joined = vec![a[a.len() - 1]];
    joined.extend_from_slice(&b[..n]);
    (step(&a[a.len() / 2..]), step(&joined), step(&b[b.len() / 2..]))
}

fn assert_no_click(name: &str, before: &Patch, after: &Patch) {
    let (pre, sw, post) = switch_steps(before, after);
    println!("{name}: max step steady {pre:.4} / {post:.4}, at switch {sw:.4}");
    assert!(sw <= 1.25 * pre.max(post), "{name}: switch step {sw}, steady {pre} / {post}");
}

fn click_patch() -> Patch {
    let mut p = Patch::riddim_v1();
    p.moda_index = 0.0;
    p.fenv_amt = 0.0;
    p
}

#[test]
fn shaper_toggle_does_not_click() {
    let mut p = click_patch();
    p.routing = Routing::Bypass;
    p.osc[0].table = Table::HarmonicRise;
    p.osc[0].index = 0.0;
    p.shaper_mode = ShaperMode::Saturate;
    p.shaper_amt = 1.0;
    let mut q = p;
    q.shaper_on = true;
    assert_no_click("shaper off -> on", &p, &q);
    assert_no_click("shaper on -> off", &q, &p);
}

#[test]
fn filter_type_and_routing_changes_do_not_click() {
    let mut p = click_patch();
    p.osc[1].on = true;
    p.filt[0].ty = FilterType::Lp12;
    p.filt[0].freq = 2000.0;
    p.filt[0].res = 0.5;
    for ty in [FilterType::Bp12, FilterType::CombPlus, FilterType::CombMinus, FilterType::Am] {
        let mut q = p;
        q.filt[0].ty = ty;
        assert_no_click(&format!("Lp12 -> {ty:?}"), &p, &q);
    }
    for (r0, r1) in [(Routing::Parallel, Routing::Series), (Routing::Series, Routing::Bypass), (Routing::Bypass, Routing::Parallel)] {
        let (mut a, mut b) = (p, p);
        a.routing = r0;
        b.routing = r1;
        assert_no_click(&format!("{r0:?} -> {r1:?}"), &a, &b);
    }
}

#[test]
fn switched_off_osc_does_not_hold_the_voice() {
    let mut p = Patch::riddim_v1();
    p.osc[0].rel = 0.05;
    p.osc[1].on = false;
    p.osc[1].rel = 5.0;
    let mut e = engine(&p);
    e.note_on(45, 1.0);
    render(&mut e, &p, 0.2);
    e.note_off(45);
    render(&mut e, &p, 0.5);
    assert_eq!(e.active_voices(), 0);
}

#[test]
fn synced_lfo_relock_on_play_is_smooth() {
    // Transport stopped at note-on (free-running at host tempo), then play starts off-grid: the
    // re-lock must glide, not jump.
    let mut p = Patch::riddim_v1();
    p.moda.curve = Curve::Sine;
    p.moda.oneshot = false;
    p.moda_pitch = 0.25;
    p.moda_index = 0.0;
    let mut e = Engine::new();
    e.initialize(SR);
    e.transport = Transport { tempo: 150.0, playing: false, pos_beats: 0.0 };
    e.set_patch(&p);
    e.note_on(45, 1.0);
    let st = |e: &Engine| 12.0 * (e.voice(0).freq(0) / 440.0).log2();
    let mut last = 0.0;
    let mut worst = 0f32;
    for i in 0..(0.5 * SR) as usize {
        if i == 7168 {
            // Free-run phase is 7168 / 9600 = 0.747 cycles; the grid says 0.147 (0.4 away).
            e.transport = Transport { tempo: 150.0, playing: true, pos_beats: 3.0733 };
        }
        if i % 512 == 0 {
            e.set_patch(&p);
        }
        e.next_frame();
        let now = st(&e);
        if i > 0 {
            worst = worst.max((now - last).abs());
        }
        last = now;
    }
    // A 0.4-cycle correction over ~12 ms adds ~0.02 st per sample on top of the sine's 0.004.
    assert!(worst < 0.04, "pitch stepped {worst} st in one sample");
}

#[test]
fn spread_puts_osc_a_on_the_left() {
    let mut p = Patch::riddim_v1();
    p.spread = 1.0;
    p.osc[1].on = false;
    let energy = |x: &[f32]| x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>();
    let mut e = engine(&p);
    e.note_on(45, 1.0);
    let (l, r): (Vec<f32>, Vec<f32>) = (0..24000).map(|_| e.next_frame()).unzip();
    let (el, er) = (energy(&l), energy(&r));
    assert!(el > 1.0 && er < 1e-3 * el, "engine: L {el}, R {er}");

    // Same through the plugin's channel mapping.
    let mut e = engine(&p);
    e.note_on(45, 1.0);
    let (mut ch0, mut ch1) = (vec![0f32; 24000], vec![0f32; 24000]);
    let mut buf = nih_plug::buffer::Buffer::default();
    unsafe {
        buf.set_slices(24000, |s| {
            s.push(&mut ch0);
            s.push(&mut ch1);
        })
    };
    for frame in buf.iter_samples() {
        gts::write_frame(frame, e.next_frame());
    }
    drop(buf);
    let (el, er) = (energy(&ch0), energy(&ch1));
    assert!(el > 1.0 && er < 1e-3 * el, "plugin: L {el}, R {er}");
}

#[test]
fn motion_ping_pongs_negative_and_wraps_positive() {
    let trace = |motion: f32| {
        let mut p = Patch::riddim_v1();
        p.moda_index = 0.0;
        p.osc[0].motion = motion;
        let mut e = engine(&p);
        e.note_on(93, 1.0); // a new grain every 27 samples
        (0..SR as usize)
            .map(|i| {
                if i % 512 == 0 {
                    e.set_patch(&p);
                }
                e.next_frame();
                e.voice(0).index(0)
            })
            .collect::<Vec<f32>>()
    };
    let big_jump = |t: &[f32]| t.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0f32, f32::max);
    let t = trace(-0.5);
    let (lo, hi) = t.iter().fold((f32::MAX, f32::MIN), |(a, b), &v| (a.min(v), b.max(v)));
    let peak = t.iter().position(|&v| v == hi).unwrap();
    assert!(lo >= 0.0 && hi <= 127.0, "{lo}..{hi}");
    assert!(hi > 120.0 && t[peak..].iter().any(|&v| v < 10.0), "no rise then fall: {lo}..{hi}");
    assert!(t[..peak].first().unwrap() < &hi, "should rise first");
    assert!(big_jump(&t) < 20.0, "ping-pong must not wrap: {}", big_jump(&t));
    let t = trace(0.5);
    assert!(big_jump(&t) > 100.0, "positive motion should wrap: {}", big_jump(&t));
}
