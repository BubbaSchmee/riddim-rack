//! SPEC v2 Track 7 acceptance 9 (flanger part): the delay sweep matches the set range, the line
//! delays by the set time (comb notch where designed), feedback stability at the extremes, synced
//! timing from the tempo, bit-exact bypass, no allocation, CPU. Numbers print with `--nocapture`.
use gts::dsp::fx::flanger::*;
use std::f64::consts::TAU;
use std::time::Instant;

// Debug builds run `no_alloc` under nih-plug's `assert_no_alloc` allocator (aborts on any
// allocation). Release builds (where nih-plug disables it) count this thread's allocations, so
// the other tests running in parallel do not count.
#[cfg(not(debug_assertions))]
mod counting {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;
    use std::sync::atomic::{AtomicUsize, Ordering};

    thread_local! { pub static ARMED: Cell<bool> = const { Cell::new(false) }; }
    pub static COUNT: AtomicUsize = AtomicUsize::new(0);

    fn note() {
        if ARMED.try_with(|a| a.get()).unwrap_or(false) {
            COUNT.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub struct Counting;

    // SAFETY: forwards to the system allocator; only counts.
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, l: Layout) -> *mut u8 {
            note();
            System.alloc(l)
        }
        unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
            note();
            System.dealloc(p, l)
        }
        unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
            note();
            System.realloc(p, l, n)
        }
    }

    #[global_allocator]
    static A: Counting = Counting;
}

fn no_alloc(f: impl FnOnce()) {
    #[cfg(debug_assertions)]
    assert_no_alloc::assert_no_alloc(f);
    #[cfg(not(debug_assertions))]
    {
        use std::sync::atomic::Ordering;
        counting::ARMED.with(|a| a.set(true));
        f();
        counting::ARMED.with(|a| a.set(false));
        assert_eq!(counting::COUNT.load(Ordering::Relaxed), 0, "process allocated");
    }
}

const SR: f32 = 48000.0;

fn params() -> FlangerParams {
    FlangerParams { on: true, ..FlangerParams::default() }
}

fn make(p: &FlangerParams) -> Flanger {
    let mut f = Flanger::new();
    f.set_sample_rate(SR);
    f.set(p);
    f.reset();
    f
}

fn noise(s: &mut u32) -> f32 {
    *s ^= *s << 13;
    *s ^= *s >> 17;
    *s ^= *s << 5;
    (*s as f32 / u32::MAX as f32) - 0.5
}

#[test]
fn sweep_matches_the_set_range() {
    let min_ms = MIN_DELAY * 1000.0 / SR as f64;
    for (delay, depth) in [(5.0f32, 0.8f32), (10.0, 0.5), (2.0, 1.0), (0.5, 0.3)] {
        let mut f = make(&FlangerParams { delay_ms: delay, depth, rate_hz: 1.0, ..params() });
        let (mut lo, mut hi) = (f64::MAX, 0.0f64);
        for _ in 0..48000 {
            let (mut l, mut r) = (0.1, 0.1);
            f.process(&mut l, &mut r);
            let d = f.delay_ms()[0];
            (lo, hi) = (lo.min(d), hi.max(d));
        }
        let (elo, ehi) = ((delay * (1.0 - depth)) as f64, delay as f64);
        let elo = elo.max(min_ms);
        println!("flanger sweep delay {delay} ms depth {depth}: {lo:.4}..{hi:.4} ms (design {elo:.4}..{ehi:.4})");
        assert!((lo - elo).abs() < 1e-3 && (hi - ehi).abs() < 1e-3);
    }
}

#[test]
fn line_delays_by_the_set_time() {
    // Depth 0, mix 1: the output is the input delayed by `delay_ms` (2 ms = 96 samples).
    let mut f = make(&FlangerParams { delay_ms: 2.0, depth: 0.0, feedback: 0.0, mix: 1.0, ..params() });
    let out: Vec<f32> = (0..200)
        .map(|i| {
            let x = if i == 0 { 1.0 } else { 0.0 };
            let (mut l, mut r) = (x, x);
            f.process(&mut l, &mut r);
            l
        })
        .collect();
    let peak = (0..out.len()).max_by(|&a, &b| out[a].abs().total_cmp(&out[b].abs())).unwrap();
    assert_eq!(peak, 96);
    assert!((out[96] - 1.0).abs() < 1e-6);
    // Mix 0.5, 1 ms: dry + echo cancels at 500 Hz (first comb notch) and doubles at 1 kHz.
    let rms = |hz: f64| {
        let mut f = make(&FlangerParams { delay_ms: 1.0, depth: 0.0, feedback: 0.0, mix: 0.5, ..params() });
        let mut e = 0.0f64;
        for i in 0..48000 {
            let x = (TAU * hz * i as f64 / SR as f64).sin() as f32;
            let (mut l, mut r) = (x, x);
            f.process(&mut l, &mut r);
            if i >= 4800 {
                e += (l as f64).powi(2);
            }
        }
        (e / 43200.0).sqrt() * std::f64::consts::SQRT_2
    };
    let (n, p) = (rms(500.0), rms(1000.0));
    println!("flanger 1 ms comb: gain at 500 Hz {:.1} dB, at 1 kHz {:.2} dB", 20.0 * n.log10(), 20.0 * p.log10());
    assert!(n < 1e-3 && (p - 1.0).abs() < 1e-3);
}

#[test]
fn feedback_extremes_stay_stable() {
    for fb in [-0.95f32, 0.95] {
        for (delay, depth) in [(0.1f32, 1.0f32), (10.0, 1.0), (10.0, 0.0)] {
            let mut f = make(&FlangerParams { feedback: fb, delay_ms: delay, depth, rate_hz: 20.0, mix: 1.0, ..params() });
            let mut s = 1u32;
            let (mut peak, mut tail) = (0.0f32, 0.0f32);
            for i in 0..(10.0 * SR) as usize {
                let x = if i < SR as usize { noise(&mut s) } else { 0.0 };
                let (mut l, mut r) = (x, x);
                f.process(&mut l, &mut r);
                assert!(l.is_finite() && r.is_finite(), "fb {fb} delay {delay}: sample {i}");
                peak = peak.max(l.abs()).max(r.abs());
                if i >= 9 * SR as usize {
                    tail = tail.max(l.abs()).max(r.abs());
                }
            }
            println!("flanger fb {fb} delay {delay} ms depth {depth}: peak {peak:.2}, peak 8 s after input {tail:.1e}");
            assert!(peak < 50.0 && tail < 1e-3, "fb {fb} delay {delay}: peak {peak}, tail {tail}");
        }
    }
}

#[test]
fn sync_follows_the_tempo() {
    // 150 BPM: a 1-beat period is 0.4 s, 1/8 (half a beat) 0.2 s.
    for (beats, secs) in [(1.0f32, 0.4f64), (0.5, 0.2), (4.0, 1.6)] {
        let mut f = make(&FlangerParams { sync: true, sync_beats: beats, ..params() });
        f.set_transport(150.0, Some(0.0));
        for _ in 0..(secs * SR as f64).round() as usize {
            let (mut l, mut r) = (0.1, 0.1);
            f.process(&mut l, &mut r);
        }
        let ph = f.lfo_phase();
        let err = ph.min(1.0 - ph) * secs * SR as f64;
        println!("flanger sync {beats} beats at 150 BPM: phase after {secs} s = {ph:.6} ({err:.3} samples off)");
        assert!(err < 1.0);
    }
    let mut f = make(&FlangerParams { sync: true, sync_beats: 2.0, ..params() });
    f.set_transport(90.0, Some(5.0));
    assert!((f.lfo_phase() - 0.5).abs() < 1e-12);
}

#[test]
fn off_is_bit_exact() {
    let mut f = make(&FlangerParams { on: false, ..params() });
    let (mut l, mut r) = (0.25f32, -0.5f32);
    f.process(&mut l, &mut r);
    assert_eq!((l, r), (0.25, -0.5));
    let mut f = make(&FlangerParams { feedback: 0.9, ..params() });
    let mut s = 7u32;
    let mut exact_from = None;
    for i in 0..48000usize {
        if i == 9600 {
            f.set(&FlangerParams { on: false, feedback: 0.9, ..params() });
        }
        let x = noise(&mut s);
        let (mut l, mut r) = (x, -x);
        f.process(&mut l, &mut r);
        if (l, r) == (x, -x) {
            exact_from.get_or_insert(i);
        } else {
            exact_from = None;
        }
    }
    let from = exact_from.expect("never bit-exact") - 9600;
    println!("flanger: bit-exact pass-through {:.1} ms after on = false", from as f64 / 48.0);
    assert!(from < 9600 && f.is_bypassed());
}

#[test]
fn process_does_not_allocate() {
    let mut f = Flanger::new();
    let p = FlangerParams { sync: true, feedback: -0.9, ..params() };
    no_alloc(|| {
        f.set_sample_rate(192000.0);
        f.set(&p);
        f.reset();
        for i in 0..20000 {
            if i % 512 == 0 {
                f.set(&FlangerParams { delay_ms: 0.1 + (i % 7) as f32, on: (i / 4096) % 2 == 0, ..p });
                f.set_transport(128.0, Some(i as f64 / 512.0));
            }
            let (mut l, mut r) = (0.1, -0.1);
            f.process(&mut l, &mut r);
        }
    });
}

#[test]
fn cpu_bench() {
    let n = if cfg!(debug_assertions) { 48000 } else { 4_800_000 };
    let mut f = make(&FlangerParams { feedback: 0.7, ..params() });
    let mut s = 3u32;
    let mut acc = 0.0f32;
    let t = Instant::now();
    for _ in 0..n {
        let x = noise(&mut s);
        let (mut l, mut r) = (x, x);
        f.process(&mut l, &mut r);
        acc += l + r;
    }
    let ns = t.elapsed().as_nanos() as f64 / n as f64;
    println!("flanger CPU: {ns:.1} ns/sample stereo, {:.3} % of a core at 48 kHz ({acc:.1})", ns * 48000.0 / 1e7);
}
