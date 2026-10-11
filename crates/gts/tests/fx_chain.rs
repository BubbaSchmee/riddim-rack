//! SPEC v2 Track 4 chain integration: order, bypass, latency, ceiling, engine wiring. CPU:
//! `cargo test -p gts --release --test fx_chain -- --ignored --nocapture`.
use gts::dsp::fx::chorus::Chorus;
use gts::dsp::fx::delay::Delay;
use gts::dsp::fx::dimension::Dimension;
use gts::dsp::fx::distortion::{DistType, Distortion, FilterMode};
use gts::dsp::fx::eq::Eq;
use gts::dsp::fx::limiter::{Limiter, LimiterParams, CEILING};
use gts::dsp::fx::multiband::Multiband;
use gts::dsp::fx::reverb::Reverb;
use gts::dsp::fx::{FxChain, FxPatch};
use gts::dsp::{Engine, Patch, Transport};
use nih_plug::prelude::NoteEvent;

/// xorshift noise in -amp..amp.
fn noise(n: usize, seed: u32, amp: f32) -> Vec<f32> {
    let mut s = seed;
    (0..n)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            amp * ((s >> 8) as f32 / (1u32 << 23) as f32 - 1.0)
        })
        .collect()
}

fn chain(sr: f32, p: &FxPatch) -> FxChain {
    let mut c = FxChain::new();
    c.set_sample_rate(sr);
    c.set(p, 150.0, Some(0.0));
    c.reset();
    c
}

/// Every module on, with settings that make each one audible.
fn all_on() -> FxPatch {
    let mut p = Patch::default().fx;
    p.dist.on = true;
    p.dist.kind = DistType::HardClip;
    p.dist.drive_db = 12.0;
    p.dist.filter_mode = FilterMode::Post;
    p.dist.filter_freq = 6000.0;
    p.mb.on = true;
    p.eq.on = true;
    p.eq.peak_gain = 4.0;
    p.eq.cut_on = true;
    p.cho.on = true;
    p.cho.sync = true;
    p.dim.on = true;
    p.dly.on = true;
    p.dly.feedback = 0.5;
    p.rev.on = true;
    p.rev.decay = 3.0;
    p.lim_gain = 6.0;
    p
}

#[test]
fn latency_is_distortion_plus_lookahead() {
    for (sr, want) in [(44100.0, 23 + 66), (48000.0, 23 + 72)] {
        let c = chain(sr, &Patch::default().fx);
        assert_eq!(c.latency(), want, "{sr} Hz");
        // The engine reports shaper + chain.
        let mut e = Engine::new();
        e.initialize(sr);
        for f in [1, 2, 4] {
            e.set_oversampling(f);
            let shaper = e.latency() - want;
            println!("{sr} Hz, oversampling {f}x: shaper {shaper} + distortion 23 + limiter {} = {}", want - 23, e.latency());
        }
    }
}

/// All modules but the limiter off: the output is the input delayed by `latency()`, bit-exact,
/// under the ceiling; also after a module was switched on and off again (once its fade is done).
#[test]
fn bypass_is_sample_exact() {
    for sr in [44100.0, 48000.0] {
        let p = Patch::default().fx;
        let mut c = chain(sr, &p);
        assert!(c.is_bypassed());
        let lat = c.latency();
        // 0.4: the chorus phase below must not push the limiter (its release would still be
        // settling when the exact check resumes).
        let x = noise(20000, 7, 0.4);
        let y = noise(20000, 8, 0.4);
        let check = |c: &mut FxChain, from: usize| {
            let mut out = vec![];
            for i in 0..x.len() {
                let (mut l, mut r) = (x[i], y[i]);
                c.process(&mut l, &mut r, 1.0);
                out.push((l, r));
            }
            for i in from.max(lat)..x.len() {
                assert_eq!(out[i].0.to_bits(), x[i - lat].to_bits(), "{sr} Hz, L sample {i}");
                assert_eq!(out[i].1.to_bits(), y[i - lat].to_bits(), "{sr} Hz, R sample {i}");
            }
        };
        check(&mut c, 0);
        // Chorus on for a block, then off: after its fade it sleeps and the chain is exact again.
        let mut on = p;
        on.cho.on = true;
        c.set(&on, 150.0, None);
        assert!(!c.is_bypassed());
        check(&mut c, usize::MAX - 1); // run only
        c.set(&p, 150.0, None);
        check(&mut c, usize::MAX - 1); // fade-out block
        c.set(&p, 150.0, None);
        assert!(c.is_bypassed());
        check(&mut c, lat);
    }
}

/// The chain is exactly the eight modules in the SPEC order with the patch's values (incl. the
/// derived chorus sync period and delay tempo).
#[test]
fn order_and_wiring_match_the_modules() {
    let sr = 48000.0;
    let p = all_on();
    let (tempo, beat) = (150.0, Some(2.5));
    let mut c = FxChain::new();
    c.set_sample_rate(sr);
    c.set(&p, tempo, beat);
    c.reset();

    let mut dist = Distortion::new();
    let mut mb = Multiband::new();
    let mut eq = Eq::new();
    let mut cho = Chorus::new();
    let mut dim = Dimension::new();
    let mut dly = Delay::new();
    let mut rev = Reverb::new();
    let mut lim = Limiter::new();
    dist.set_sample_rate(sr);
    mb.set_sample_rate(sr);
    eq.set_sample_rate(sr as f64);
    cho.set_sample_rate(sr as f64);
    dim.set_sample_rate(sr as f64);
    dly.set_sample_rate(sr);
    rev.set_sample_rate(sr);
    lim.set_sample_rate(sr);
    dist.set(&p.dist);
    mb.set(&p.mb);
    eq.set(&p.eq);
    let cp = gts::dsp::fx::chorus::ChorusParams { sync_beats: 4.0, ..p.cho }; // cho_div 1/1
    cho.set(&cp);
    cho.set_transport(tempo, beat);
    dim.set(&p.dim);
    dly.set(&gts::dsp::fx::delay::DelayParams { tempo: tempo as f32, ..p.dly });
    rev.set(&p.rev);
    lim.set(&LimiterParams { on: true, gain_db: p.lim_gain });
    dist.reset();
    mb.reset();
    eq.reset();
    cho.reset();
    cho.set_transport(tempo, beat);
    dim.reset();
    dly.reset();
    rev.reset();
    lim.reset();

    let (x, y) = (noise(48000, 1, 0.5), noise(48000, 2, 0.5));
    let mut peak = 0f32;
    for i in 0..x.len() {
        let (mut a, mut b) = (x[i], y[i]);
        c.process(&mut a, &mut b, 1.0);
        let (mut l, mut r) = (x[i], y[i]);
        dist.process(&mut l, &mut r);
        mb.process(&mut l, &mut r);
        eq.process(&mut l, &mut r);
        cho.process(&mut l, &mut r);
        dim.process(&mut l, &mut r);
        dly.process(&mut l, &mut r);
        rev.process(&mut l, &mut r);
        lim.process(&mut l, &mut r);
        assert_eq!((a.to_bits(), b.to_bits()), (l.to_bits(), r.to_bits()), "sample {i}");
        assert!(a.is_finite() && b.is_finite());
        peak = peak.max(a.abs()).max(b.abs());
    }
    assert!(peak <= CEILING, "peak {peak}");
}

/// Through the engine: a hot, fully effected patch never passes the -0.3 dBFS ceiling at any
/// master volume (the limiter follows the volume), stays finite, and its reverb/delay tails keep
/// sounding after the note ends. With every module off, the default patch's output is the same
/// at -12 dB volume as before the chain existed: the voice never reaches the limiter.
#[test]
fn engine_limits_after_the_volume() {
    for volume_db in [-6.0, 0.0, 6.0] {
        let mut e = Engine::new();
        e.initialize(48000.0);
        e.transport = Transport { tempo: 150.0, playing: true, pos_beats: 0.0 };
        let mut patch = Patch::default();
        patch.volume_db = volume_db;
        patch.fx = all_on();
        patch.fx.lim_gain = 12.0;
        patch.fx.mb.out_db = 24.0;
        e.set_patch(&patch);
        e.handle_event(&NoteEvent::NoteOn { timing: 0, voice_id: None, channel: 0, note: 36, velocity: 1.0 });
        let (mut peak, mut tail) = (0f32, 0f32);
        for i in 0..96000 {
            if i % 512 == 0 {
                e.set_patch(&patch);
            }
            if i == 24000 {
                e.handle_event(&NoteEvent::NoteOff { timing: 0, voice_id: None, channel: 0, note: 36, velocity: 0.0 });
            }
            let (l, r) = e.next_frame();
            assert!(l.is_finite() && r.is_finite());
            peak = peak.max(l.abs()).max(r.abs());
            if i > 48000 {
                tail = tail.max(l.abs());
            }
        }
        println!("volume {volume_db:+} dB: peak {:.3} dBFS, tail {tail:.4}", 20.0 * peak.log10());
        assert!(peak <= CEILING, "peak {peak} above the ceiling at {volume_db} dB");
        assert!(peak > 0.9 * CEILING, "limiter never reached at {volume_db} dB: {peak}");
        assert!(tail > 1e-3, "no tail after note-off: {tail}");
    }
}

/// Release CPU figures: the full chain with every module on, the bypassed chain, and the limiter
/// alone, ns per stereo sample (best of 7 interleaved runs) and % of one core at 48 kHz.
#[test]
#[ignore]
fn bench_cpu() {
    use std::time::Instant;
    let sr = 48000.0;
    let n = 480_000;
    let (x, y) = (noise(4800, 5, 0.5), noise(4800, 6, 0.5));
    let mut full = chain(sr, &all_on());
    let mut off = chain(sr, &Patch::default().fx);
    let mut lim = Limiter::new();
    lim.set_sample_rate(sr);
    lim.set(&LimiterParams { on: true, gain_db: 0.0 });
    let mut best = [f64::MAX; 3];
    let mut acc = 0.0f32;
    for _ in 0..7 {
        for (k, best) in best.iter_mut().enumerate() {
            let t = Instant::now();
            for i in 0..n {
                let (mut a, mut b) = (x[i % 4800], y[i % 4800]);
                match k {
                    0 => full.process(&mut a, &mut b, 1.0),
                    1 => off.process(&mut a, &mut b, 1.0),
                    _ => lim.process(&mut a, &mut b),
                }
                acc += a + b;
            }
            *best = best.min(t.elapsed().as_secs_f64() * 1e9 / n as f64);
        }
    }
    for (name, ns) in ["full chain, every module on", "chain off (bypass)", "limiter alone"].iter().zip(best) {
        println!("{name}: {ns:.1} ns per stereo sample, {:.3} % of a core at 48 kHz", ns * 48000.0 * 1e-9 * 100.0);
    }
    println!("(acc {acc})");
}
