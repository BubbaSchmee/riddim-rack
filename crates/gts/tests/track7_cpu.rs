//! SPEC v2 Track 7 acceptance 10, CPU part (a report, not a gate: timings depend on the machine):
//! 8 voices, 32 voice-scope matrix slots, the Track 4 FX chain all on, 4 s at 48 kHz, best of 5.
//! `cargo test -p gts --release --test track7_cpu -- --ignored --nocapture --test-threads=1`.
//! The same "base" scenario at c178995 (2 LFOs, no warp / noise, 8 FX modules) is the reference.
use gts::dsp::fx::distortion::{DistType, FilterMode};
use gts::dsp::fx::FxPatch;
use gts::dsp::shape::{library, LfoShapes};
use gts::dsp::{Engine, Patch};
use gts::matrix::{Matrix, Slot, Source};
use gts::params::{Curve, Routing, WarpMode};

const SR: f32 = 48000.0;

fn fx_all_on(mut p: FxPatch) -> FxPatch {
    p.dist.on = true;
    p.dist.kind = DistType::HardClip;
    p.dist.drive_db = 12.0;
    p.dist.filter_mode = FilterMode::Post;
    p.mb.on = true;
    p.eq.on = true;
    p.eq.peak_gain = 4.0;
    p.cho.on = true;
    p.dim.on = true;
    p.dly.on = true;
    p.dly.feedback = 0.5;
    p.rev.on = true;
    p.rev.decay = 3.0;
    p
}

fn slots(sources: &[Source]) -> Vec<Slot> {
    let targets = [
        "osca_index", "oscb_index", "osca_shift", "oscb_shift", "osca_motion", "oscb_motion", "osca_vol", "oscb_vol",
        "osca_cent", "oscb_cent", "filta_freq", "filtb_freq", "filta_res", "filtb_res", "filta_kbd", "filtb_kbd",
        "fenv_amt", "fenv_dec", "osca_dec", "oscb_dec", "osca_detune", "oscb_detune", "osca_blend", "oscb_width",
        "shaper_amt", "sub_level", "porta", "moda_index", "modb_cutoff", "moda_rate", "modb_rate", "wheel_index",
    ];
    targets.iter().enumerate().map(|(k, t)| Slot::new(sources[k % sources.len()], t, 0.05 + 0.01 * k as f32)).collect()
}

fn base() -> Patch {
    let mut p = Patch::riddim_v1();
    p.poly = 8;
    p.osc[1].on = true;
    p.osc[0].unison = 3;
    p.shaper_on = true;
    p.routing = Routing::Series;
    p.macros[0] = 0.5;
    p.modb.curve = Curve::Sine;
    p.modb.rate = 3.0;
    p.fx = fx_all_on(p.fx);
    p
}

fn render(p: &Patch, slots: &[Slot], shapes: Option<&LfoShapes>, secs: f64) -> f64 {
    let mut best = f64::MAX;
    for _ in 0..5 {
        let mut e = Engine::new();
        e.initialize(SR);
        e.set_patch(p);
        e.set_matrix(&Matrix { slots: slots.to_vec(), ..Default::default() }.encode());
        if let Some(s) = shapes {
            e.set_lfo_shapes(&s.encode());
        }
        for n in 0..8 {
            e.note_on(29 + 5 * n, 0.5 + 0.06 * n as f32);
        }
        let t = std::time::Instant::now();
        let mut acc = 0f32;
        for i in 0..(secs * SR as f64) as usize {
            if i % 512 == 0 {
                e.set_patch(p);
            }
            let (l, r) = e.next_frame();
            acc += l + r;
        }
        best = best.min(t.elapsed().as_secs_f64());
        assert_eq!(e.active_voices(), 8);
        assert!(acc.is_finite());
    }
    100.0 * best / secs
}

#[test]
#[ignore]
fn track7_cpu_report() {
    use Source::*;
    let secs = 4.0;
    let old_src = [ModA, ModB, FilterEnv, AmpEnvA, Velocity, Keytrack, Random, Macro1];
    let p0 = base();
    let b = render(&p0, &slots(&old_src), None, secs);

    // Track 7 modulation + sound: four running LFOs with custom shapes, Mod Env, FM warp on both
    // oscillators, the noise layer on; the matrix uses Mod A..D and Mod Env.
    let mut p = p0;
    for m in [&mut p.moda, &mut p.modb, &mut p.modc, &mut p.modd] {
        m.curve = Curve::Custom;
        m.oneshot = false;
        m.rate = 2.0;
    }
    p.menv_dec = 0.4;
    for o in &mut p.osc {
        o.warp = WarpMode::Fm;
        o.warp_amt = 0.5;
    }
    p.noise_level_db = -12.0;
    let lib = library();
    let shapes = LfoShapes([lib[0].1.clone(), lib[3].1.clone(), lib[5].1.clone(), lib[10].1.clone()]);
    let new_src = [ModA, ModB, ModC, ModD, ModEnv, FilterEnv, Velocity, Random];
    let c = render(&p, &slots(&new_src), Some(&shapes), secs);

    // Breakdown, one feature at a time on the base scenario.
    let mut lfo = p0;
    for m in [&mut lfo.moda, &mut lfo.modb, &mut lfo.modc, &mut lfo.modd] {
        m.curve = Curve::Custom;
        m.oneshot = false;
        m.rate = 2.0;
    }
    let l = render(&lfo, &slots(&new_src), Some(&shapes), secs);
    let mut fm = p0;
    for o in &mut fm.osc {
        o.warp = WarpMode::Fm;
        o.warp_amt = 0.5;
    }
    let f = render(&fm, &slots(&old_src), None, secs);
    let mut fm1 = p0;
    fm1.osc[1].warp = WarpMode::Fm;
    fm1.osc[1].warp_amt = 0.5;
    let f1 = render(&fm1, &slots(&old_src), None, secs);
    let mut nz = p0;
    nz.noise_level_db = -12.0;
    let z = render(&nz, &slots(&old_src), None, secs);
    println!(
        "breakdown vs base: 4 custom LFOs + Mod C/D/Env slots {:+.2}, FM warp both oscs {:+.2} (Osc B only, unison 1: {:+.2}), noise {:+.2}",
        l - b,
        f - b,
        f1 - b,
        z - b
    );

    let mut q = p;
    q.fx.phs.on = true;
    q.fx.flg.on = true;
    let d = render(&q, &slots(&new_src), Some(&shapes), secs);

    println!("base scenario (Track 7 defaults): {b:.2} % of a core");
    println!("+ 4 custom LFOs, Mod Env, FM warp on both oscs, noise: {c:.2} % ({:+.2})", c - b);
    println!("+ phaser + flanger on: {d:.2} % ({:+.2})", d - c);
}

/// The factory-bank worst case: 8 voices, both oscs unison 7 (width), Bend+ on A and Sync on B,
/// four custom LFOs, Mod Env, 32 voice-scope slots, noise, every FX module on (phaser, flanger).
fn heavy() -> Patch {
    let mut p = base();
    for o in &mut p.osc {
        o.on = true;
        o.unison = 7;
        o.detune = 25.0;
        o.width = 0.8;
        o.warp_amt = 0.5;
    }
    p.osc[0].warp = WarpMode::BendPlus;
    p.osc[1].warp = WarpMode::Sync;
    for m in [&mut p.moda, &mut p.modb, &mut p.modc, &mut p.modd] {
        m.curve = Curve::Custom;
        m.oneshot = false;
        m.rate = 2.0;
    }
    p.menv_dec = 0.4;
    p.noise_level_db = -12.0;
    p.fx.phs.on = true;
    p.fx.flg.on = true;
    p
}

/// One `secs` render of 8 held notes, % of a core.
fn render1(p: &Patch, slots: &[Slot], shapes: Option<&LfoShapes>, secs: f64) -> f64 {
    let mut e = Engine::new();
    e.initialize(SR);
    e.set_patch(p);
    e.set_matrix(&Matrix { slots: slots.to_vec(), ..Default::default() }.encode());
    if let Some(s) = shapes {
        e.set_lfo_shapes(&s.encode());
    }
    for n in 0..8 {
        e.note_on(29 + 5 * n, 0.5 + 0.06 * n as f32);
    }
    let t = std::time::Instant::now();
    let mut acc = 0f32;
    for i in 0..(secs * SR as f64) as usize {
        if i % 512 == 0 {
            e.set_patch(p);
        }
        let (l, r) = e.next_frame();
        acc += l + r;
    }
    assert!(acc.is_finite());
    100.0 * t.elapsed().as_secs_f64() / secs
}

/// Median of three renders, % of a core.
fn median3(p: &Patch, slots: &[Slot], shapes: Option<&LfoShapes>, secs: f64) -> f64 {
    let mut v: Vec<f64> = (0..3).map(|_| render1(p, slots, shapes, secs)).collect();
    v.sort_by(f64::total_cmp);
    v[1]
}

/// Profile by ablation (no sampling profiler on this machine): the heavy scenario, then with one
/// feature removed at a time; the difference is that feature's share. A report, like the rest of
/// this file: `cargo test -p gts --release --test track7_cpu heavy -- --ignored --nocapture
/// --test-threads=1`.
#[test]
#[ignore]
fn heavy_scenario_profile() {
    use Source::*;
    let secs = 2.0;
    let lib = library();
    let shapes = LfoShapes([lib[0].1.clone(), lib[3].1.clone(), lib[5].1.clone(), lib[10].1.clone()]);
    let src = [ModA, ModB, ModC, ModD, ModEnv, FilterEnv, Velocity, Random];
    let full = slots(&src);
    let h = heavy();
    let total = median3(&h, &full, Some(&shapes), secs);
    println!("heavy scenario: {total:.2} % of a core");
    let mut rows: Vec<(&str, f64)> = vec![];
    let mut ab = |name: &'static str, p: Patch, sl: &[Slot], sh: Option<&LfoShapes>| {
        rows.push((name, total - median3(&p, sl, sh, secs)));
    };
    let mut p = h;
    p.osc[0].warp = WarpMode::Off;
    ab("Osc A Bend+ warp (vs Off)", p, &full, Some(&shapes));
    let mut p = h;
    p.osc[1].warp = WarpMode::Off;
    ab("Osc B Sync warp (vs Off)", p, &full, Some(&shapes));
    let mut p = h;
    for o in &mut p.osc {
        o.warp = WarpMode::Off;
        o.unison = 1;
    }
    ab("osc work over unison 1 unwarped", p, &full, Some(&shapes));
    ab("32-slot matrix (vs none)", h, &[], Some(&shapes));
    let mut p = h;
    for m in [&mut p.moda, &mut p.modb, &mut p.modc, &mut p.modd] {
        m.curve = Curve::Sine;
    }
    ab("custom LFO shapes (vs sine)", p, &full, None);
    let mut p = h;
    p.fx = Patch::riddim_v1().fx;
    ab("FX chain (vs factory default)", p, &full, Some(&shapes));
    let mut p = h;
    p.noise_level_db = -200.0;
    ab("noise", p, &full, Some(&shapes));
    let mut p = h;
    p.shaper_on = false;
    ab("shaper", p, &full, Some(&shapes));
    rows.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (n, d) in rows {
        println!("  {n:36} {d:+6.2} % of a core ({:4.1} % of the scenario)", 100.0 * d / total);
    }
}
