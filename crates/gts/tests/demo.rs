//! SPEC Verification 1 "demo": the examples/demo.rs render is loud enough, never clips, and every
//! bar plays.
#[path = "../examples/demo.rs"]
#[allow(dead_code)]
mod demo;

fn rms_db(x: &[[f32; 2]]) -> f64 {
    let s: f64 = x.iter().flatten().map(|v| (*v as f64).powi(2)).sum();
    10.0 * (s / (2 * x.len()) as f64).log10()
}

#[test]
fn demo_render_level_and_bars() {
    let frames = demo::render();
    let rms = rms_db(&frames);
    let peak = frames.iter().flatten().fold(0f32, |m, v| m.max(v.abs()));
    println!("demo: RMS {rms:.1} dBFS, peak {peak:.3}");
    assert!((-30.0..=-8.0).contains(&rms), "RMS {rms} dBFS");
    assert!(peak < 0.99, "peak {peak}");
    let bar = 8 * demo::EIGHTH;
    for b in 0..demo::BARS {
        let r = rms_db(&frames[b * bar..(b + 1) * bar]);
        println!("bar {}: {r:.1} dBFS", b + 1);
        assert!(r > -40.0, "bar {} is silent: {r} dBFS", b + 1);
    }
}
