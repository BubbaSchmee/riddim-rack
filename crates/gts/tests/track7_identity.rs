//! SPEC v2 Track 7 acceptance 1: with every new parameter at its default, the engine renders
//! bit-identically to the engine before Track 7 (c178995). The golden hashes were recorded by
//! these scenarios at that commit: every factory preset (patch + matrix), and a busy patch run
//! through every pre-Track-7 filter type and both sub octaves.
use gts::dsp::{Engine, Patch};
use gts::params::{Curve, FilterType, Routing};
use gts::preset::factory;

const SR: f32 = 48000.0;

fn fnv(h: &mut u64, x: f32) {
    for b in x.to_bits().to_le_bytes() {
        *h ^= b as u64;
        *h = h.wrapping_mul(0x100_0000_01b3);
    }
}

fn render(p: &Patch, enc: Option<&[f32; gts::matrix::ENC_LEN]>, n: usize) -> u64 {
    let mut e = Engine::new();
    e.initialize(SR);
    e.set_patch(p);
    if let Some(enc) = enc {
        e.set_matrix(enc);
    }
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for i in 0..n {
        if i % 256 == 0 {
            e.set_patch(p);
        }
        match i {
            64 => e.note_on(36, 0.9),
            4000 => e.note_on(43, 0.5),
            9000 => e.mod_wheel(0.6),
            14000 => e.note_off(36),
            20000 => e.note_on(55, 1.0),
            30000 => e.note_off(43),
            34000 => e.note_off(55),
            _ => {}
        }
        let (l, r) = e.next_frame();
        fnv(&mut h, l);
        fnv(&mut h, r);
    }
    h
}

fn busy() -> Patch {
    let mut p = Patch::riddim_v1();
    p.poly = 3;
    p.osc[0].unison = 3;
    p.osc[0].width = 0.6;
    p.osc[1].on = true;
    p.osc[1].shift = 7.0;
    p.modb.curve = Curve::Sine;
    p.modb.rate = 3.0;
    p.modb_cutoff = 0.3;
    p.shaper_on = true;
    p.sub_on = true;
    p
}

#[test]
fn new_defaults_render_bit_identically_to_the_pre_track7_engine() {
    let mut hashes = vec![];
    for pr in factory() {
        let enc = pr.matrix_state().encode();
        hashes.push((pr.name.clone(), render(&pr.patch, Some(&enc), 40000)));
    }
    let types = [FilterType::Lp12, FilterType::Bp12, FilterType::CombPlus, FilterType::CombMinus, FilterType::Am];
    for (k, ty) in types.into_iter().enumerate() {
        let mut p = busy();
        p.filt[0].ty = ty;
        p.filt[1].ty = types[(k + 2) % types.len()];
        p.filt[0].res = 0.8;
        p.routing = if k % 2 == 0 { Routing::Parallel } else { Routing::Series };
        p.sub_oct = if k % 2 == 0 { -1 } else { -2 };
        hashes.push((format!("busy {ty:?}"), render(&p, None, 30000)));
    }
    let mut all = 0xcbf2_9ce4_8422_2325u64;
    for (name, h) in &hashes {
        println!("{name}: {h:#018x}");
        for b in h.to_le_bytes() {
            all ^= b as u64;
            all = all.wrapping_mul(0x100_0000_01b3);
        }
    }
    println!("combined {all:#018x} over {} renders", hashes.len());
    assert_eq!(all, GOLDEN);
}

/// Recorded at c178995 (before Track 7).
const GOLDEN: u64 = 0xb64c_840a_c460_a514;
