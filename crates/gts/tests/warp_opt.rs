//! Track 7 warp CPU work (2026-10): the warped oscillators read the Bend / PWM phase map from a
//! per-osc table (`MapLut`) and take the grain window from a polynomial instead of libm. Warp is Off by default, so presets render as
//! before (tests/track7_bitident.rs); these tests bound what that changes in a warped osc and pin
//! Sync, which stayed bit for bit. Figures print with `--nocapture`.
use gts::dsp::grain::{GrainOsc, MapLut, PhaseMap};
use gts::dsp::tables::{Tables, N, TOP};
use gts::dsp::{Engine, Patch};
use gts::params::{Routing, Table, WarpMode};
use std::f64::consts::TAU;
use std::sync::OnceLock;

fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(|| {
        let mut t = Tables::new();
        t.generate();
        t
    })
}

/// voice.rs: Bend radius 0.85, PWM 0.9 at amount 1.
fn maps(amt: f64) -> [(&'static str, PhaseMap); 3] {
    [("Bend+", PhaseMap::new(0.85 * amt, 0.0)), ("Bend-", PhaseMap::new(-0.85 * amt, 0.0)), ("PWM", PhaseMap::new(0.0, 0.9 * amt))]
}

fn lut(m: &PhaseMap) -> MapLut {
    let mut t = MapLut::default();
    t.build(m);
    t
}

/// Largest distance (rad, modulo a cycle) between `f` and the exact map over the positions the
/// readers see: one cycle plus the pitch-marker excursion.
fn map_error(m: &PhaseMap, f: impl Fn(f64) -> f64) -> f64 {
    let mut e: f64 = 0.0;
    for i in 0..200_000 {
        let p = -40.0 + (N as f64 + 80.0) * i as f64 / 200_000.0;
        let d = (f(p) - m.apply(p)).rem_euclid(N as f64);
        e = e.max(d.min(N as f64 - d) * TAU / N as f64);
    }
    e
}

#[test]
fn map_table_stays_within_3e7_rad_of_the_exact_map() {
    let mut worst: f64 = 0.0;
    for amt in [0.0, 0.1, 0.5, 0.9, 1.0] {
        for (name, m) in maps(amt) {
            let t = lut(&m);
            let e = map_error(&m, |p| t.apply(p));
            println!("{name} amount {amt}: {e:.2e} rad");
            worst = worst.max(e);
        }
    }
    assert!(worst <= 3e-7, "{worst:.2e} rad");
}

/// Error (dB re signal RMS) of a warped grain osc as the engine runs it (map table, polynomial
/// window) against libm and the exact map, on the 2x grid at 110 Hz with Index moving (two
/// readers in a crossfade).
fn grain_error_db(map: &PhaseMap, ratio: f64) -> f64 {
    let lut = lut(map);
    let t = tables();
    let table = Table::SawStack as usize;
    let fmul = (TOP * map.slope() / 1.3).max(1.0);
    let dp = 110.0 / 96000.0;
    let (mut a, mut b) = (GrainOsc::default(), GrainOsc::default());
    a.reset_at(ratio, 10.0, 0.3);
    b.reset_at(ratio, 10.0, 0.3);
    let (mut e, mut s) = (0.0, 0.0);
    for i in 0..96000 {
        let gp = 8.0 + 4.0 * (i % 9001) as f32 / 9001.0;
        let x = a.next_with(t, table, dp, ratio, gp, 2.0, fmul, true, |p| lut.apply(p)) as f64;
        let y = b.next_with(t, table, dp, ratio, gp, 2.0, fmul, false, |p| map.apply(p)) as f64;
        e += (x - y) * (x - y);
        s += y * y;
    }
    10.0 * (e / s).log10()
}

#[test]
fn fast_warp_kernels_stay_under_minus_90_db() {
    let mut worst = f64::MIN;
    // Shift 0, +7 (non-integer ratio: both readers apart) and +12 (pitch marker on).
    for ratio in [1.0, 1.5, 2.0] {
        for amt in [0.0, 0.5, 1.0] {
            for (name, m) in maps(amt) {
                let d = grain_error_db(&m, ratio);
                println!("{name} amount {amt} ratio {ratio}: {d:.1} dB");
                worst = worst.max(d);
            }
        }
    }
    assert!(worst <= -90.0, "{worst:.1} dB");
}

fn fnv(h: &mut u64, x: f32) {
    for b in x.to_bits().to_le_bytes() {
        *h ^= b as u64;
        *h = h.wrapping_mul(0x100_0000_01b3);
    }
}

/// 4 voices, both oscs on, unison 7 with width, Shift +7 / +12, both warped in `mode` (FM: Osc B
/// Sync), interleaved stereo.
fn warp_render(mode: WarpMode) -> Vec<f32> {
    let mut p = Patch::riddim_v1();
    p.routing = Routing::Bypass;
    p.shaper_on = false;
    p.poly = 4;
    for o in &mut p.osc {
        o.on = true;
        o.unison = 7;
        o.detune = 25.0;
        o.width = 0.8;
        o.warp_amt = 0.6;
        o.shift = 7.0;
    }
    p.osc[0].warp = mode;
    p.osc[1].warp = if mode == WarpMode::Fm { WarpMode::Sync } else { mode };
    p.osc[1].shift = 12.0;
    let mut e = Engine::new();
    e.initialize(48000.0);
    e.set_patch(&p);
    for n in [36, 43, 52, 60] {
        e.note_on(n, 0.9);
    }
    let mut v = vec![];
    for i in 0..48000 {
        if i % 512 == 0 {
            e.set_patch(&p);
        }
        let (l, r) = e.next_frame();
        v.push(l);
        v.push(r);
    }
    v
}

#[test]
fn sync_warp_renders_bit_identically_to_24efaf9() {
    // Sync only changed in where it divides and caches 2^(3 amount); hash recorded at 24efaf9.
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for x in warp_render(WarpMode::Sync) {
        fnv(&mut h, x);
    }
    assert_eq!(h, 0x2ad7_260d_3b90_6b8b, "{h:#x}");
}
