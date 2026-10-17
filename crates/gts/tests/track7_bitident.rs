//! SPEC v2 Track 7 acceptance 1, per render: every factory preset (patch + matrix), the
//! riddim_v1 fixture (with the FX chain off and with every module on, so the default chain order
//! is covered) and the all-defaults patch from `GtsParams` render bit-identically to c178995 with
//! every Track 7 feature at its default. The hashes in `REF` were printed by this file at c178995.
use gts::dsp::{Engine, Patch};
use gts::params::GtsParams;
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
            6000 => e.note_on(43, 0.5),
            11000 => e.mod_wheel(0.6),
            15000 => e.pitch_bend(0.7),
            20000 => e.note_off(36),
            26000 => e.note_on(55, 1.0),
            36000 => e.note_off(43),
            40000 => e.note_off(55),
            _ => {}
        }
        let (l, r) = e.next_frame();
        fnv(&mut h, l);
        fnv(&mut h, r);
    }
    h
}

fn renders() -> Vec<(String, u64)> {
    let n = 48000;
    let mut out = vec![];
    for pr in factory() {
        let enc = pr.matrix_state().encode();
        out.push((pr.name.clone(), render(&pr.patch, Some(&enc), n)));
    }
    out.push(("riddim_v1".into(), render(&Patch::riddim_v1(), None, n)));
    let mut p = Patch::riddim_v1();
    p.fx.dist.on = true;
    p.fx.mb.on = true;
    p.fx.eq.on = true;
    p.fx.eq.peak_gain = 6.0;
    p.fx.cho.on = true;
    p.fx.dim.on = true;
    p.fx.dly.on = true;
    p.fx.rev.on = true;
    out.push(("riddim_v1 full fx".into(), render(&p, None, n)));
    out.push(("param defaults".into(), render(&Patch::from_params(&GtsParams::default()), None, n)));
    out
}

#[test]
fn every_factory_preset_and_fixture_renders_as_before_track7() {
    let got = renders();
    for (name, h) in &got {
        println!("    (\"{name}\", {h:#018x}),");
    }
    assert_eq!(got.len(), REF.len(), "render count");
    for ((name, h), (rn, rh)) in got.iter().zip(REF) {
        assert_eq!(name, rn);
        assert_eq!(h, rh, "{name} differs from c178995");
    }
}

/// Printed by this test at c178995 (before Track 7).
const REF: &[(&str, u64)] = &[
    ("Growl Hound", 0x86c1917cbe114a1f),
    ("Reese Drift", 0xbbfd39f3ff0c5274),
    ("Sub Weight", 0xba6c34ea40a67831),
    ("Talk Box", 0xe6a02fa61f63dbb1),
    ("Wobble Shot", 0xa9ef1de720ca0108),
    ("Screech Lead", 0x7946760786ca3ad9),
    ("riddim_v1", 0xde663322bfc964a5),
    ("riddim_v1 full fx", 0x5f5fc098ad0e679d),
    ("param defaults", 0x91cd0ae5a5ad9085),
];
