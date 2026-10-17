//! SPEC v2 Track 7 acceptance (10), sound half: the new audio paths never allocate. Every warp
//! mode on both oscillators (unison, stereo), the noise layer, the driven triangle sub at
//! octave 0, every new filter type, and a matrix on the new continuous targets, under nih-plug's
//! `assert_no_alloc` in debug builds and an allocation counter in release builds (one test per
//! binary, as tests/matrix_alloc.rs).
use gts::dsp::{Engine, Patch};
use gts::matrix::{Matrix, Slot, Source};
use gts::params::{FilterType, NoiseType, Routing, SubShape, WarpMode};
use nih_plug::prelude::Enum;

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

#[test]
fn track7_sound_paths_never_allocate() {
    let mut p = Patch::default();
    p.poly = 8;
    p.osc[1].on = true;
    p.osc[0].unison = 3;
    p.osc[0].width = 0.8;
    (p.sub_shape, p.sub_oct, p.sub_drive, p.sub_phase) = (SubShape::Triangle, 0, 0.7, 90.0);
    (p.noise_type, p.noise_level_db, p.noise_tone) = (NoiseType::Pink, -12.0, 0.3);
    p.routing = Routing::Series;
    let matrix = Matrix {
        slots: ["osca_warp_amt", "oscb_warp_amt", "noise_level", "noise_tone", "sub_drive", "sub_phase", "filta_freq"]
            .iter()
            .enumerate()
            .map(|(k, t)| Slot::new(Source::ALL[k], t, 0.3))
            .collect(),
        ..Default::default()
    };
    let enc = matrix.encode();
    let mut e = Engine::new();
    e.initialize(48000.0);
    e.set_patch(&p);
    e.set_matrix(&enc);
    let modes = [WarpMode::Fm, WarpMode::Sync, WarpMode::BendPlus, WarpMode::BendMinus, WarpMode::Pwm, WarpMode::Off];
    let types = [FilterType::Hp12, FilterType::Notch, FilterType::Lp24, FilterType::Formant];
    for b in 0..48 {
        // Each block a new combination (mode / type switches mid-note included).
        p.osc[0].warp = modes[b % modes.len()];
        p.osc[1].warp = modes[(b + 2) % modes.len()];
        (p.osc[0].warp_amt, p.osc[1].warp_amt) = (0.2 + 0.1 * (b % 8) as f32, 0.5);
        p.filt[0].ty = types[b % types.len()];
        p.filt[1].ty = types[(b + 1) % types.len()];
        p.noise_type = NoiseType::from_index(b % 3);
        no_alloc(|| {
            e.set_patch(&p);
            for i in 0..512 {
                if i % 64 == 0 {
                    let n = 36 + ((b * 8 + i / 64) % 24) as u8;
                    if i % 128 == 0 { e.note_on(n, 0.8) } else { e.note_off(n.saturating_sub(5)) }
                }
                let (l, r) = e.next_frame();
                assert!(l.is_finite() && r.is_finite());
            }
        });
    }
    assert!(e.active_voices() > 0);
}
