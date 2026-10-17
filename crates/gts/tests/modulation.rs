//! SPEC v2 Track 7 modulation acceptance (G1, G2, G6, G7): (1) the extensions are neutral at
//! their defaults, (2) custom / step shapes and their persistence, (3) dotted timing, start
//! phase, fade-in, (4) Mod Env -> pitch_all, (10) CPU of the extra modulators. The golden-hash
//! render with every new default lives in tests/matrix.rs.
use gts::dsp::env::Adsr;
use gts::dsp::lfo::Lfo;
use gts::dsp::shape::{library, LfoShapes, Mode, Shape, ShapeDef, ShapesShared, ENC, SHAPES_ENC};
use gts::dsp::{Engine, Patch};
use gts::matrix::{Matrix, Slot, Source};
use gts::params::{Curve, Division, GtsParams, Table, Trigger};
use gts::preset::Preset;
use nih_plug::params::persist::PersistentField;
use nih_plug::prelude::{NoteEvent, Params};
use std::time::Instant;

const SR: f64 = 48000.0;

fn lfo() -> Lfo {
    let mut l = Lfo::new();
    l.set_sample_rate(SR);
    l
}

/// The audio-side form of `d` (as the engine decodes it from the published record).
fn shape(d: &ShapeDef) -> Shape {
    let mut s = LfoShapes::default();
    s.0[0] = d.clone();
    Shape::decode(&s.encode()[..ENC])
}

// ---- (1) neutral defaults --------------------------------------------------------------------

/// `set_ext(0°, 0 s, 0, Retrigger)` changes nothing, sample for sample, for every deterministic
/// curve in free, synced (with and without a host grid) and one-shot mode.
#[test]
fn default_extensions_are_bit_identical() {
    let curves = [Curve::Sine, Curve::Triangle, Curve::SawUp, Curve::SawDown, Curve::Square, Curve::Decay, Curve::Attack, Curve::Custom];
    for curve in curves {
        for mode in 0..4 {
            let (mut a, mut b) = (lfo(), lfo());
            let mut beat = Some(0.37);
            for blk in 0..20 {
                let sync = match mode {
                    1 => Some((Division::Sixteenth, 140.0, None)),
                    2 => Some((Division::EighthT, 140.0, beat)),
                    _ => None,
                };
                for (k, l) in [&mut a, &mut b].into_iter().enumerate() {
                    l.configure(curve, 7.3, sync, mode == 3);
                    if k == 1 {
                        l.set_ext(0.0, 0.0, 0.0, false);
                    }
                    if blk == 5 {
                        l.trigger();
                    }
                }
                for _ in 0..512 {
                    assert_eq!(a.next().to_bits(), b.next().to_bits(), "{curve:?} mode {mode}");
                }
                beat = beat.map(|x| x + 512.0 / SR * 140.0 / 60.0);
            }
        }
    }
}

/// Every new parameter's default is the neutral value.
#[test]
fn new_param_defaults_are_neutral() {
    let p = Patch::default();
    for e in p.lfo_ext {
        assert_eq!((e.phase, e.fade, e.smooth, e.trig), (0.0, 0.0, 0.0, Trigger::Retrigger));
    }
    assert_eq!((p.osca_pitch, p.oscb_pitch, p.pitch_all), (0.0, 0.0, 0.0));
    assert!(!p.modc.oneshot && !p.modd.oneshot);
    assert_eq!(p.moda.rate, 4.0);
    // The 0.03..100 Hz rate keeps the 4 Hz default where the 0.03..30 Hz range had it.
    let params = GtsParams::default();
    let map = params.param_map();
    let (_, rate, _) = map.iter().find(|(id, _, _)| id == "moda_rate").unwrap();
    // SAFETY: `params` outlives the pointer.
    let (n, top) = unsafe { (rate.default_normalized_value(), rate.preview_plain(1.0)) };
    assert!((n - ((4.0f32 - 0.03) / 29.97).powf(0.25)).abs() < 1e-6, "{n}");
    assert!((top - 100.0).abs() < 1e-3);
}

// ---- (2) custom and step shapes --------------------------------------------------------------

/// A drawn shape (12 points, curved segments) is reproduced at its points within 1e-4, through
/// the published record and the LFO.
#[test]
fn drawn_shape_reproduces_its_points() {
    let pts: Vec<[f32; 3]> = (0..12)
        .map(|k| {
            let x = (4 * k) as f32 / 48.0;
            [x, ((k * 7919) % 200) as f32 / 100.0 - 1.0, ((k * 37) % 21) as f32 / 10.0 - 1.0]
        })
        .collect();
    let def = ShapeDef::points(&pts);
    let s = shape(&def);
    for p in &pts {
        assert!((s.eval(p[0] as f64) - p[1]).abs() <= 1e-4, "{p:?}: {}", s.eval(p[0] as f64));
    }
    // 10 Hz at 48 kHz: point k (x = k/12) is reached at sample 400 k.
    let mut l = lfo();
    l.set_shape(&s);
    l.configure(Curve::Custom, 10.0, None, false);
    l.trigger();
    for n in 0..4800 {
        let v = l.next();
        if n % 400 == 0 {
            let want = pts[n / 400][1];
            assert!((v - want).abs() <= 1e-4, "sample {n}: {v} vs {want}");
        }
    }
}

/// Step mode holds each level, glides over the set fraction of the step into the next one, and
/// the last step glides into the first.
#[test]
fn step_mode_holds_and_glides() {
    let levels = [1.0, -1.0, 0.5, 0.0];
    let s = shape(&ShapeDef::steps(&levels, 0.25));
    for k in 0..4 {
        let at = |f: f64| s.eval((k as f64 + f) / 4.0);
        for f in [0.0, 0.3, 0.6, 0.749] {
            assert_eq!(at(f), levels[k], "step {k} at {f}");
        }
        let next = levels[(k + 1) % 4];
        let half = levels[k] + 0.5 * (next - levels[k]);
        assert!((at(0.875) - half).abs() < 1e-5, "step {k} glide midpoint {}", at(0.875));
    }
    let hard = shape(&ShapeDef::steps(&levels, 0.0));
    assert_eq!(hard.eval(0.2499), 1.0);
    assert_eq!(hard.eval(0.25), -1.0);
}

/// Shapes round-trip through presets ("lfo_shapes", absent = defaults) and the host state.
#[test]
fn shapes_round_trip_through_presets_and_state() {
    let lib = library();
    let mut shapes = LfoShapes::default();
    for (k, s) in shapes.0.iter_mut().enumerate() {
        *s = lib[k * 3].1.clone();
    }
    assert_eq!(shapes.0[1].mode, Mode::Steps);
    let p = Preset { lfo_shapes: shapes.clone(), ..Preset::default() };
    let json = p.to_json();
    assert!(json.contains("\"lfo_shapes\""));
    let (q, w) = Preset::from_json(&json).unwrap();
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(q.lfo_shapes, shapes);
    assert!(!Preset::default().to_json().contains("lfo_shapes"));
    assert_eq!(Preset::from_json(&Preset::default().to_json()).unwrap().0.lfo_shapes, LfoShapes::default());

    let params = GtsParams::default();
    params.lfo_shapes.replace(shapes.clone());
    let state = params.serialize_fields();
    let other = GtsParams::default();
    other.deserialize_fields(&state);
    assert_eq!(other.lfo_shapes.get(), shapes);
    other.lfo_shapes.set(LfoShapes::default());
    assert_eq!(other.lfo_shapes.get(), LfoShapes::default());
}

/// The audio side sees a published edit once, through the fixed record.
#[test]
fn shapes_reach_the_engine_through_the_snapshot() {
    let shared = ShapesShared::default();
    let (mut seen, mut rec) = (u32::MAX, [0.0f32; SHAPES_ENC]);
    assert!(shared.poll(&mut seen, &mut rec));
    assert!(!shared.poll(&mut seen, &mut rec));
    let step = ShapeDef::steps(&[1.0, -1.0], 0.0);
    shared.edit(|s| s.0[2] = step.clone());
    assert!(shared.poll(&mut seen, &mut rec));
    assert_eq!(Shape::decode(&rec[2 * ENC..3 * ENC]).eval(0.75), -1.0);
}

// ---- (3) timing, phase, fade -----------------------------------------------------------------

/// Dotted 1/8 at 150 BPM is 0.3 s per cycle (14400 samples) to the sample.
#[test]
fn dotted_eighth_at_150_is_0_3_s() {
    let mut l = lfo();
    l.trigger();
    let (mut wraps, mut last) = (vec![], f32::MIN);
    for n in 0..4 * 48000 {
        if n % 512 == 0 {
            l.configure(Curve::SawUp, 1.0, Some((Division::EighthD, 150.0, None)), false);
        }
        let x = l.next();
        if x < last - 1.0 {
            wraps.push(n);
        }
        last = x;
    }
    assert!(wraps.len() >= 10, "{wraps:?}");
    for w in wraps.windows(2) {
        assert!((w[1] as i64 - w[0] as i64 - 14400).abs() <= 1, "{wraps:?}");
    }
}

#[test]
fn phase_180_inverts_a_sine() {
    let (mut a, mut b) = (lfo(), lfo());
    a.configure(Curve::Sine, 3.0, None, false);
    b.configure(Curve::Sine, 3.0, None, false);
    b.set_ext(180.0, 0.0, 0.0, false);
    a.trigger();
    b.trigger();
    for _ in 0..48000 {
        let (x, y) = (a.next(), b.next());
        assert!((x + y).abs() < 1e-5, "{x} {y}");
    }
}

/// Fade-in: 0 at note-on, about half way at half the time, at least 99 % at the set time, and it
/// restarts on the next note.
#[test]
fn fade_in_reaches_99_percent_at_the_set_time() {
    let mut l = lfo();
    // Square at 0.03 Hz is +1 for the first 16 s: the output is the fade gain.
    l.configure(Curve::Square, 0.03, None, false);
    l.set_ext(0.0, 0.5, 0.0, false);
    for _ in 0..2 {
        l.trigger();
        let v: Vec<f32> = (0..48000).map(|_| l.next()).collect();
        assert_eq!(v[0], 0.0);
        assert!((v[12000] - 0.5).abs() < 0.01, "{}", v[12000]);
        assert!(v[24000] >= 0.99, "{}", v[24000]);
        assert!(v.windows(2).all(|w| w[1] >= w[0]));
    }
}

/// Free keeps running through a note-on, Retrigger restarts; smoothing rounds the square's step.
#[test]
fn free_trigger_and_smoothing() {
    let (mut free, mut re) = (lfo(), lfo());
    for (l, f) in [(&mut free, true), (&mut re, false)] {
        l.configure(Curve::SawUp, 1.0, None, false);
        l.set_ext(0.0, 0.0, 0.0, f);
        l.trigger();
        for _ in 0..12000 {
            l.next();
        }
        l.trigger();
    }
    assert!((free.next() - (-0.5)).abs() < 1e-3);
    assert_eq!(re.next(), -1.0);

    let mut sq = lfo();
    sq.configure(Curve::Square, 2.0, None, false);
    sq.set_ext(0.0, 0.0, 0.5, false); // time constant 1/8 cycle: the half cycle still settles
    sq.trigger();
    let v: Vec<f32> = (0..48000).map(|_| sq.next()).collect();
    let max_step = v.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0, f32::max);
    assert!(max_step < 0.01, "{max_step}");
    assert!(v.iter().any(|x| *x > 0.9) && v.iter().any(|x| *x < -0.9));
}

// ---- (4) Mod Env -> pitch_all ----------------------------------------------------------------

/// Fundamental period in samples by normalized autocorrelation (parabolic peak refinement).
fn period(x: &[f32], min_lag: usize, max_lag: usize) -> f64 {
    let n = x.len() - max_lag - 1;
    let r = |lag: usize| -> f64 {
        let (mut num, mut e0, mut e1) = (0.0, 0.0, 0.0);
        for i in 0..n {
            let (a, b) = (x[i] as f64, x[i + lag] as f64);
            num += a * b;
            e0 += a * a;
            e1 += b * b;
        }
        num / (e0 * e1).sqrt().max(1e-12)
    };
    let rs: Vec<f64> = (min_lag - 1..=max_lag + 1).map(r).collect();
    let k = (1..rs.len() - 1).max_by(|&a, &b| rs[a].total_cmp(&rs[b])).unwrap();
    let (a, b, c) = (rs[k - 1], rs[k], rs[k + 1]);
    (min_lag - 1 + k) as f64 + 0.5 * (a - c) / (a - 2.0 * b + c)
}

/// Mod Env (sustain 1) -> pitch_all +24 st holds the note two octaves up while the key is down
/// and dives back to the note on release (the amp envelope still sounding).
#[test]
fn mod_env_to_pitch_all_dives_two_octaves() {
    let mut p = Patch::default();
    p.osc[0].table = Table::Basic;
    p.osc[0].index = 0.0;
    p.osc[0].rel = 3.0;
    p.sub_on = false;
    (p.menv_att, p.menv_sus, p.menv_rel) = (0.001, 1.0, 0.02);
    let mut e = Engine::new();
    e.initialize(SR as f32);
    e.set_patch(&p);
    // pitch_all spans 96 st: +24 st is a quarter of the normalized range.
    e.set_matrix(&Matrix { slots: vec![Slot::new(Source::ModEnv, "pitch_all", 0.25)], ..Default::default() }.encode());
    e.handle_event(&NoteEvent::NoteOn { timing: 0, voice_id: None, channel: 0, note: 48, velocity: 1.0 });
    let mut out = vec![];
    for n in 0..48000 {
        if n % 256 == 0 {
            e.set_patch(&p);
        }
        if n == 19200 {
            e.handle_event(&NoteEvent::NoteOff { timing: 0, voice_id: None, channel: 0, note: 48, velocity: 0.0 });
        }
        out.push(e.next_frame().0);
    }
    // C3 = 130.81 Hz: 366.9 samples; two octaves up 91.7.
    let up = period(&out[9600..19200], 60, 140);
    let down = period(&out[28800..43200], 300, 450);
    let semis = 12.0 * (down / up).log2();
    println!("periods {up:.2} / {down:.2} samples: {semis:.3} st");
    assert!((down - SR / 130.8128).abs() < 0.5, "{down}");
    assert!((semis - 24.0).abs() < 0.1, "{semis}");
}

// ---- (10) CPU ----------------------------------------------------------------------------------

/// Fraction of one core to render `secs` of audio, best of three.
fn core_share(mut f: impl FnMut(), secs: f64) -> f64 {
    (0..3)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed().as_secs_f64() / secs
        })
        .fold(f64::MAX, f64::min)
}

/// What the modulation wave adds per sample at 8 voices: two more LFOs (Custom shapes, smoothing,
/// fade) and the mod envelope per voice. Reported with the whole engine at 8 voices with all four
/// LFOs Custom and matrix-routed, against the init patch.
#[test]
#[ignore = "CPU budget: wall-clock timing is unreliable under parallel load; run alone with --ignored --test-threads=1 (tools/CHECKLIST-gts.md)"]
fn cpu_of_the_extra_modulators_at_8_voices() {
    if cfg!(debug_assertions) {
        return; // timing only means something optimised
    }
    let lib = library();
    let n = 48000;
    let mut lfos: Vec<Lfo> = (0..16).map(|_| lfo()).collect();
    for (k, l) in lfos.iter_mut().enumerate() {
        l.set_shape(&shape(&lib[k % lib.len()].1));
        l.configure(Curve::Custom, 3.0 + k as f32, None, false);
        l.set_ext(45.0, 0.5, 0.3, false);
        l.trigger();
    }
    let mut envs = [Adsr::default(); 8];
    for e in &mut envs {
        e.set(0.01, 0.3, 0.5, 0.2, SR as f32);
        e.gate_on();
    }
    let mut sink = 0.0f32;
    let extra = core_share(
        || {
            for _ in 0..n {
                for l in lfos.iter_mut() {
                    sink += l.next();
                }
                for e in envs.iter_mut() {
                    sink += e.next();
                }
            }
        },
        1.0,
    );
    std::hint::black_box(sink);

    let render = |p: &Patch, slots: &[Slot], shapes: bool| {
        let mut e = Engine::new();
        e.initialize(SR as f32);
        if shapes {
            let mut s = LfoShapes::default();
            for (k, d) in s.0.iter_mut().enumerate() {
                *d = lib[k].1.clone();
            }
            e.set_lfo_shapes(&s.encode());
        }
        e.set_patch(p);
        e.set_matrix(&Matrix { slots: slots.to_vec(), ..Default::default() }.encode());
        for k in 0..8 {
            e.handle_event(&NoteEvent::NoteOn { timing: 0, voice_id: None, channel: 0, note: 36 + 3 * k, velocity: 0.8 });
        }
        core_share(
            || {
                for i in 0..n {
                    if i % 256 == 0 {
                        e.set_patch(p);
                    }
                    std::hint::black_box(e.next_frame());
                }
            },
            1.0,
        )
    };
    let mut base = Patch::default();
    base.poly = 8;
    let mut full = base;
    for m in [&mut full.moda, &mut full.modb, &mut full.modc, &mut full.modd] {
        m.curve = Curve::Custom;
    }
    for e in &mut full.lfo_ext {
        (e.phase, e.fade, e.smooth) = (90.0, 0.2, 0.2);
    }
    full.menv_sus = 0.5;
    let slots = [
        Slot::new(Source::ModC, "osca_index", 0.3),
        Slot::new(Source::ModD, "filta_freq", 0.2),
        Slot::new(Source::ModEnv, "pitch_all", 0.05),
        Slot::new(Source::ModA, "osca_shift", 0.2),
    ];
    let (b, f) = (render(&base, &[], false), render(&full, &slots, true));
    println!(
        "extra modulators (16 LFOs + 8 envs): {:.2} % of a core; engine 8 voices init {:.1} %, 4 custom LFOs + matrix {:.1} %",
        100.0 * extra,
        100.0 * b,
        100.0 * f
    );
    assert!(extra < 0.08, "{extra}");
}
