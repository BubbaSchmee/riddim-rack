//! SPEC v2 Track 5 acceptance 5: the audio side of a full matrix (32 slots, every source, voice
//! and global targets with every side effect) never allocates: the snapshot poll, matrix loads,
//! note / pressure / CC events, the 32-sample evaluation and the ModLive publish. Debug builds run
//! it under nih-plug's `assert_no_alloc` allocator (aborts on any allocation); release builds
//! count allocations with the allocator below. One test per binary, so no other test allocates
//! meanwhile.
use gts::dsp::{Engine, Patch};
use gts::matrix::{Matrix, MatrixShared, Slot, Source, ENC_LEN, MAX_SLOTS};
use gts::params::GtsParams;
use gts::viz::VizWriter;
use nih_plug::prelude::NoteEvent;

#[cfg(not(debug_assertions))]
mod counting {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    pub static ARMED: AtomicBool = AtomicBool::new(false);
    pub static COUNT: AtomicUsize = AtomicUsize::new(0);

    pub struct Counting;

    // SAFETY: forwards to the system allocator; only counts.
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, l: Layout) -> *mut u8 {
            if ARMED.load(Ordering::Relaxed) {
                COUNT.fetch_add(1, Ordering::Relaxed);
            }
            System.alloc(l)
        }
        unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
            if ARMED.load(Ordering::Relaxed) {
                COUNT.fetch_add(1, Ordering::Relaxed);
            }
            System.dealloc(p, l)
        }
        unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
            if ARMED.load(Ordering::Relaxed) {
                COUNT.fetch_add(1, Ordering::Relaxed);
            }
            System.realloc(p, l, n)
        }
    }

    #[global_allocator]
    static A: Counting = Counting;
}

fn no_alloc<R>(f: impl FnOnce() -> R) -> R {
    #[cfg(debug_assertions)]
    {
        assert_no_alloc::assert_no_alloc(f)
    }
    #[cfg(not(debug_assertions))]
    {
        use std::sync::atomic::Ordering;
        counting::COUNT.store(0, Ordering::Relaxed);
        counting::ARMED.store(true, Ordering::Relaxed);
        let r = f();
        counting::ARMED.store(false, Ordering::Relaxed);
        let n = counting::COUNT.load(Ordering::Relaxed);
        assert_eq!(n, 0, "{n} allocations on the audio path");
        r
    }
}

/// 32 slots: every source, voice targets with every side effect (update, LFO, velocity gain,
/// porta) and global ones (FX, crossover, volume).
fn full_matrix(shift: f32) -> Matrix {
    #[rustfmt::skip]
    let targets = [
        "osca_index", "oscb_index", "osca_shift", "filta_freq", "filtb_res", "fenv_amt", "osca_att", "fenv_dec",
        "osca_unison", "osca_detune", "shaper_amt", "sub_level", "moda_rate", "modb_rate", "vel_vol_a", "porta",
        "osca_cent", "osca_oct", "rev_mix", "dly_feedback", "dist_drive", "cho_voices", "mb_depth", "eq_peak_gain",
        "dim_size", "lim_gain", "mono_xover", "volume", "spread", "bend_range", "oscb_width", "moda_index",
    ];
    let slots = targets
        .iter()
        .enumerate()
        .map(|(k, t)| Slot { bipolar: k % 3 == 0, ..Slot::new(Source::ALL[k % Source::ALL.len()], t, shift + 0.02 * k as f32) })
        .collect::<Vec<_>>();
    assert_eq!(slots.len(), MAX_SLOTS);
    Matrix { slots, ..Default::default() }
}

#[test]
fn full_matrix_audio_path_never_allocates() {
    // The release counter counts (the debug guard is nih-plug's own).
    #[cfg(not(debug_assertions))]
    {
        use std::sync::atomic::Ordering;
        counting::ARMED.store(true, Ordering::Relaxed);
        drop(std::hint::black_box(vec![1u8; 16]));
        counting::ARMED.store(false, Ordering::Relaxed);
        assert!(counting::COUNT.swap(0, Ordering::Relaxed) > 0);
    }
    let params = GtsParams::default();
    let shared = MatrixShared::default();
    shared.replace(full_matrix(0.1));
    let mut patch = Patch::default();
    patch.poly = 8;
    patch.osc[1].on = true;
    patch.macros = [0.3, 0.6, 0.9, 0.2];
    patch.fx.rev.on = true;
    patch.fx.dly.on = true;
    let mut e = Engine::new();
    e.initialize(48000.0);
    e.set_patch(&patch);
    let (mut viz, _shared_viz, _ring) = VizWriter::new();
    let (mut seen, mut rec) = (u32::MAX, [0.0f32; ENC_LEN]);

    let ev = |note: u8, k: usize| match k % 6 {
        0 => NoteEvent::NoteOn { timing: 0, voice_id: None, channel: 0, note, velocity: 0.8 },
        1 => NoteEvent::PolyPressure { timing: 0, voice_id: None, channel: 0, note, pressure: 0.7 },
        2 => NoteEvent::MidiChannelPressure { timing: 0, channel: 0, pressure: 0.4 },
        3 => NoteEvent::MidiCC { timing: 0, channel: 0, cc: 1, value: 0.6 },
        4 => NoteEvent::MidiPitchBend { timing: 0, channel: 0, value: 0.7 },
        _ => NoteEvent::NoteOff { timing: 0, voice_id: None, channel: 0, note, velocity: 0.0 },
    };
    let mut block = |e: &mut Engine, viz: &mut VizWriter, b: usize| {
        if shared.poll(&mut seen, &mut rec) {
            e.set_matrix(&rec);
        }
        e.set_bus_cv([0.5, -0.5, 0.25]);
        e.set_patch(&patch);
        for i in 0..512 {
            if i % 64 == 0 {
                let k = b * 8 + i / 64;
                e.handle_event(&ev(36 + (k % 24) as u8, k));
            }
            if i % 32 == 0 {
                e.set_cv_patch(&patch);
            }
            let (l, r) = e.next_frame();
            viz.push(l, r);
        }
        let fx = e.fx_telemetry();
        let p = *e.patch();
        viz.publish(e, &params, &p, 48000.0, &fx);
    };
    for b in 0..40 {
        // A GUI edit between blocks (allocates on the GUI side, as it may).
        if b % 10 == 5 {
            shared.replace(full_matrix(0.05 * (b / 10) as f32));
        }
        no_alloc(|| block(&mut e, &mut viz, b));
    }
    assert!(e.matrix_on() && e.active_voices() > 0);
}
