//! SPEC Verification 1, tables: every grain RMS within 1 % of 0.25 unless peak-capped, peak <= 1.0,
//! |DC| < 1e-3, no NaN. SPEC-v2 1.1: generation of all 8 tables (10 mips each, plus the Shift
//! level / trim calibration) <= 150 ms in release; every mip finite, DC-free and band-limited to
//! its harmonic count.
use gts::dsp::tables::{mip_len, Fft, Tables, GRAINS, HMAX, MIPS, N};
use gts::params::Table;
use nih_plug::prelude::Enum;
use std::time::Instant;

#[test]
fn grains_are_sane_and_generation_is_fast() {
    // Per-table timing (render only), then the full generate (render + calibration).
    let mut scratch = Tables::new();
    for ti in 0..Table::variants().len() {
        let start = Instant::now();
        scratch.generate_table(ti);
        println!("table {ti} render: {:.2} ms", start.elapsed().as_secs_f64() * 1000.0);
    }
    let mut t = Tables::new();
    let start = Instant::now();
    t.generate();
    let ms = start.elapsed().as_secs_f64() * 1000.0;
    println!("graintable generation (8 tables, all mips, calibration): {ms:.2} ms");
    // ponytail: debug builds run the synthesis loop ~10x slower; the 150 ms budget is the
    // optimized plugin's, so unoptimized test builds get a 10x allowance.
    let budget = if cfg!(debug_assertions) { 1500.0 } else { 150.0 };
    assert!(ms < budget, "generation took {ms:.2} ms (budget {budget} ms)");

    for ti in 0..Table::variants().len() {
        let table = Table::from_index(ti);
        for g in 0..GRAINS {
            let grain = t.grain(table, g);
            assert_eq!(grain.len(), N);
            assert!(grain.iter().all(|x| x.is_finite()), "{table:?} grain {g} has NaN/inf");
            let peak = grain.iter().fold(0f32, |m, x| m.max(x.abs()));
            assert!(peak <= 1.0 + 1e-6, "{table:?} grain {g} peak {peak}");
            let rms = (grain.iter().map(|&x| x as f64 * x as f64).sum::<f64>() / N as f64).sqrt();
            let capped = peak > 0.999;
            assert!(capped || (rms / 0.25 - 1.0).abs() < 0.01, "{table:?} grain {g} RMS {rms}");
            assert!(rms <= 0.25 * 1.01, "{table:?} grain {g} RMS {rms} above target");
            let dc = grain.iter().map(|&x| x as f64).sum::<f64>() / N as f64;
            assert!(dc.abs() < 1e-3, "{table:?} grain {g} DC {dc}");
            for m in 0..MIPS {
                let f = t.mip(table, g, m);
                assert_eq!(f.len(), mip_len(m));
                assert!(f.iter().all(|x| x.is_finite()), "{table:?} grain {g} mip {m} has NaN/inf");
                let dc = f.iter().map(|&x| x as f64).sum::<f64>() / f.len() as f64;
                assert!(dc.abs() < 1e-3, "{table:?} grain {g} mip {m} DC {dc}");
            }
        }
    }
}

#[test]
fn mips_are_band_limited() {
    // No energy above mip m's harmonic count (HMAX >> m), relative to the mip's total.
    let mut t = Tables::new();
    t.generate();
    let fft = Fft::new(mip_len(0));
    for ti in 0..Table::variants().len() {
        for g in [0, 15, 31] {
            for m in 0..MIPS {
                let f = t.mip(Table::from_index(ti), g, m);
                let mut re: Vec<f64> = f.iter().map(|&x| x as f64).collect();
                let mut im = vec![0.0; f.len()];
                fft.run(&mut re, &mut im, false);
                let p = |h: usize| re[h] * re[h] + im[h] * im[h];
                let total: f64 = (1..f.len() / 2).map(p).sum();
                let above: f64 = ((HMAX >> m) + 1..f.len() / 2).map(p).sum();
                let db = 10.0 * (above / total).max(1e-30).log10();
                assert!(db < -120.0, "table {ti} grain {g} mip {m}: {db:.1} dB above harmonic {}", HMAX >> m);
            }
        }
    }
}

#[test]
fn generation_is_deterministic() {
    let (mut a, mut b) = (Tables::new(), Tables::new());
    a.generate();
    b.generate();
    for ti in 0..Table::variants().len() {
        for g in 0..GRAINS {
            for m in 0..MIPS {
                assert_eq!(a.mip(Table::from_index(ti), g, m), b.mip(Table::from_index(ti), g, m));
            }
        }
    }
}
