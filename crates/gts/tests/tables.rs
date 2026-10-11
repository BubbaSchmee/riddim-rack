//! SPEC Verification 1, tables: every grain RMS within 1 % of 0.25 unless peak-capped, peak <= 1.0, |DC| < 1e-3, no NaN; generation < 50 ms.
use gts::dsp::tables::{Tables, GRAINS, N};
use gts::params::Table;
use nih_plug::prelude::Enum;
use std::time::Instant;

#[test]
fn grains_are_sane_and_generation_is_fast() {
    let mut t = Tables::new();
    let start = Instant::now();
    t.generate();
    let ms = start.elapsed().as_secs_f64() * 1000.0;
    println!("graintable generation: {ms:.2} ms");
    // ponytail: debug builds run the synthesis loop ~10x slower; the 50 ms budget is the
    // optimized plugin's, so unoptimized test builds get a 10x allowance.
    let budget = if cfg!(debug_assertions) { 500.0 } else { 50.0 };
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
            assert_eq!(a.grain(Table::from_index(ti), g), b.grain(Table::from_index(ti), g));
        }
    }
}
