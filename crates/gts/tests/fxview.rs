//! FX page telemetry and display math (MOTION.md 2.2): what the audio thread publishes about the
//! modules, the pure curves the displays draw, and the GUI-side histories.
use gts::dsp::fx::distortion::{transfer, DistType};
use gts::dsp::fx::eq::{response_db, EqParams};
use gts::dsp::{Engine, Patch};
use gts::fxview::FxLive;
use gts::params::GtsParams;
use gts::viz::{xf, VizFrame, VizWriter};

const SR: f32 = 44100.0;

/// The real engine and publisher, FX on, a note held then released: every module reports.
#[test]
fn modules_publish_what_they_do() {
    let params = GtsParams::default();
    let mut patch = Patch::from_params(&params);
    patch.fx.dist.on = true;
    patch.fx.dist.drive_db = 18.0;
    patch.fx.mb.on = true;
    patch.fx.mb.down_db = [-40.0; 3];
    patch.fx.cho.on = true;
    patch.fx.cho.mix = 0.5;
    patch.fx.dim.on = true;
    patch.fx.dim.mix = 1.0;
    patch.fx.dly.on = true;
    patch.fx.dly.sync = false;
    patch.fx.dly.time_ms = 120.0;
    patch.fx.dly.mix = 0.5;
    patch.fx.rev.on = true;
    patch.fx.rev.mix = 0.5;
    let mut e = Engine::new();
    e.initialize(SR);
    e.set_patch(&patch);
    let (mut w, viz, _ring) = VizWriter::new();
    let mut f = VizFrame::default();
    let (mut max_in, mut max_lvl, mut max_cho, mut max_dly, mut max_rev, mut min_gain) = (0.0f32, -200.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32);
    let mut tail = (0.0f32, 0.0f32);
    for block in 0..400 {
        if block == 0 {
            e.note_on(41, 0.9);
        }
        if block == 200 {
            e.note_off(41);
        }
        for _ in 0..256 {
            let (l, r) = e.next_frame();
            w.push(l, r);
        }
        let fx = e.fx_telemetry();
        w.publish(&e, &params, &patch, SR, &fx);
        assert!(viz.read(&mut f));
        max_in = max_in.max(f.fx(xf::DIST_IN).abs());
        max_lvl = max_lvl.max(f.fx(xf::MB_LEVEL + 1));
        min_gain = min_gain.min(f.fx(xf::MB_GAIN + 1));
        max_cho = max_cho.max(f.fx(xf::CHO_PHASE));
        max_dly = max_dly.max(f.fx(xf::DLY_WET));
        max_rev = max_rev.max(f.fx(xf::REV_WET));
        if block == 399 {
            tail = (f.fx(xf::DLY_IN), f.fx(xf::REV_WET));
        }
    }
    assert!(max_in > 0.05, "distortion input {max_in}");
    assert!(max_lvl > -40.0, "multiband mid level {max_lvl} dB");
    assert!(min_gain < -0.5, "multiband mid band was not reduced ({min_gain} dB)");
    assert!(max_cho > 0.0, "chorus phase never moved");
    assert!((f.fx(xf::DLY_TIME) - 0.12).abs() < 0.002, "delay time {}", f.fx(xf::DLY_TIME));
    assert!(max_dly > 0.01, "delay wet {max_dly}");
    assert!(max_rev > 0.005, "reverb wet {max_rev}");
    // 2 s after the note the reverb still rings (a tail), while the delay input is silent.
    assert!(tail.0 < 1e-3, "delay input after release {}", tail.0);
    assert!(tail.1 > 1e-4, "reverb tail gone after 1.2 s: {}", tail.1);
    // The side ring runs in step with the scope ring.
    assert_eq!(viz.side().written(), 400 * 256);
}

/// Off modules publish nothing: no wet signal, no shaper input.
#[test]
fn bypassed_modules_publish_silence() {
    let params = GtsParams::default();
    let mut patch = Patch::from_params(&params);
    patch.fx.dist.on = false;
    patch.fx.dly.on = false;
    patch.fx.rev.on = false;
    let mut e = Engine::new();
    e.initialize(SR);
    e.set_patch(&patch);
    e.note_on(41, 0.9);
    for _ in 0..8192 {
        e.next_frame();
    }
    let t = e.fx_telemetry();
    assert_eq!((t.dist_in, t.dly_wet, t.rev_wet), (0.0, 0.0, 0.0));
}

#[test]
fn transfer_curve_matches_the_shapes() {
    assert_eq!(transfer(DistType::HardClip, 0.0, 0.0, 2.0), 1.0);
    assert_eq!(transfer(DistType::HardClip, 0.0, 0.0, -0.4), -0.4);
    assert!(transfer(DistType::Tube, 12.0, 0.0, 0.0).abs() < 1e-6);
    // Drive steepens the curve: the same input lands higher.
    assert!(transfer(DistType::Tube, 12.0, 0.0, 0.2) > transfer(DistType::Tube, 0.0, 0.0, 0.2));
    // Every type is finite across the plotted range and the drive range.
    for k in DistType::ALL {
        for d in [-12.0, 0.0, 36.0] {
            for i in 0..=20 {
                assert!(transfer(k, d, 0.3, i as f32 / 10.0 - 1.0).is_finite());
            }
        }
    }
}

#[test]
fn eq_response_follows_the_bands() {
    let flat = EqParams { on: true, ..Default::default() };
    for f in [30.0, 300.0, 3000.0, 18000.0] {
        assert!(response_db(&flat, f).abs() < 1e-6);
    }
    let peak = EqParams { on: true, peak_gain: 6.0, peak_freq: 1000.0, peak_q: 1.0, ..Default::default() };
    assert!((response_db(&peak, 1000.0) - 6.0).abs() < 0.01);
    assert!(response_db(&peak, 30.0).abs() < 0.5 && response_db(&peak, 18000.0).abs() < 0.5);
    let shelf = EqParams { on: true, low_gain: -9.0, low_freq: 200.0, ..Default::default() };
    assert!((response_db(&shelf, 30.0) + 9.0).abs() < 0.8, "{}", response_db(&shelf, 30.0));
    let cut = EqParams { on: true, cut_on: true, cut_freq: 200.0, ..Default::default() };
    assert!(response_db(&cut, 50.0) < -20.0 && response_db(&cut, 2000.0).abs() < 0.1);
    assert_eq!(response_db(&EqParams { on: false, peak_gain: 6.0, ..Default::default() }, 1000.0), 0.0);
}

/// The delay history answers "what entered k delays ago", the reverb age counts from the last loud input.
#[test]
fn live_state_histories() {
    let mut live = FxLive::default();
    let mut f = VizFrame::default();
    let (mut now, dt) = (0.0f64, 1.0 / 60.0);
    for frame in 0..180 {
        // A burst between 0.5 s and 0.6 s, silence otherwise.
        let loud = (0.5..0.6).contains(&now);
        f.0[gts::viz::FX_BASE + xf::DLY_IN] = if loud { 0.5 } else { 0.0 };
        f.0[gts::viz::FX_BASE + xf::REV_IN] = if loud { 0.5 } else { 0.0 };
        f.0[gts::viz::FX_BASE + xf::REV_WET] = 0.1;
        live.update(&f, now, dt as f32);
        now += dt;
        if frame == 179 {
            // 3 s in: the burst was 2.4 s ago (+/- the burst length).
            assert!(live.dly_flash(2.45) > 0.5, "echo of the burst should land at lag 2.45 s");
            assert!(live.dly_flash(1.0) < 0.05, "nothing entered 1 s ago");
            assert!(live.dly_flash(20.0) == 0.0, "beyond the history");
            assert!(live.rev_age > 2.2 && live.rev_age < 2.6, "age {}", live.rev_age);
        }
    }
    // Side and mid -> L and R.
    live.set_stereo(&[0.5; 64], &[0.25; 64]);
    assert!(live.dim_l.iter().all(|v| (*v - 1.0).abs() < 1e-6) && live.dim_r.iter().all(|v| (*v - 1.0 / 3.0).abs() < 1e-5));
}
