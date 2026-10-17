//! SPEC v2 Track 7 acceptance 9 (phaser part): notch count = stages / 2 at the design
//! frequencies, sweep range, feedback stability at the extremes, synced timing from the tempo,
//! bit-exact bypass, click-free changes, no allocation, CPU. Numbers print with `--nocapture`.
use gts::dsp::fx::phaser::*;
use std::f64::consts::{PI, TAU};
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

fn params() -> PhaserParams {
    PhaserParams { on: true, ..PhaserParams::default() }
}

fn make(p: &PhaserParams) -> Phaser {
    let mut f = Phaser::new();
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

/// |H(f)| of an impulse response.
fn mag(ir: &[f64], f: f64) -> f64 {
    let w = TAU * f / SR as f64;
    let (mut re, mut im) = (0.0, 0.0);
    for (n, &h) in ir.iter().enumerate() {
        let (s, c) = (w * n as f64).sin_cos();
        re += h * c;
        im -= h * s;
    }
    re.hypot(im)
}

#[test]
fn notch_count_is_half_the_stages() {
    let fc = 1000.0f64;
    let g = (PI * fc / SR as f64).tan();
    for stages in [4u32, 6, 8, 10, 12] {
        let mut f = make(&PhaserParams { stages, depth: 0.0, feedback: 0.0, centre_hz: fc as f32, mix: 0.5, ..params() });
        let ir: Vec<f64> = (0..16384)
            .map(|i| {
                let x = if i == 0 { 1.0 } else { 0.0 };
                let (mut l, mut r) = (x, x);
                f.process(&mut l, &mut r);
                l as f64
            })
            .collect();
        // Log grid 20 Hz..20 kHz, 4000 points: count the deep local minima.
        let fs: Vec<f64> = (0..4000).map(|i| 20.0 * 1000f64.powf(i as f64 / 3999.0)).collect();
        let m: Vec<f64> = fs.iter().map(|&f| mag(&ir, f)).collect();
        let mins: Vec<f64> = (1..m.len() - 1).filter(|&i| m[i] < m[i - 1] && m[i] < m[i + 1] && m[i] < 0.1).map(|i| fs[i]).collect();
        let n = stages as usize;
        let design: Vec<f64> =
            (0..n / 2).map(|k| SR as f64 / PI * (g * ((2 * k + 1) as f64 * PI / (2 * n) as f64).tan()).atan()).collect();
        println!("phaser {stages} stages: notches {mins:.0?}, design {design:.0?}");
        assert_eq!(mins.len(), n / 2, "{stages} stages: {mins:?}");
        for (a, b) in mins.iter().zip(&design) {
            assert!((a / b - 1.0).abs() < 0.005, "{stages} stages: notch {a:.1} vs design {b:.1}");
            assert!(mag(&ir, *b) < 1e-3, "{stages} stages: depth at {b:.1} Hz");
        }
    }
}

#[test]
fn sweep_covers_the_set_range() {
    for (depth, centre) in [(0.5f32, 1000.0f32), (1.0, 500.0), (0.25, 4000.0)] {
        let mut f = make(&PhaserParams { depth, centre_hz: centre, rate_hz: 2.0, ..params() });
        let (mut lo, mut hi) = (f32::MAX, 0.0f32);
        for _ in 0..48000 {
            let (mut l, mut r) = (0.1, 0.1);
            f.process(&mut l, &mut r);
            let b = f.break_hz()[0];
            (lo, hi) = (lo.min(b), hi.max(b));
        }
        let (elo, ehi) = (centre * 2f32.powf(-SPAN_OCT * depth), centre * 2f32.powf(SPAN_OCT * depth));
        println!("phaser sweep depth {depth} centre {centre}: {lo:.1}..{hi:.1} Hz (design {elo:.1}..{ehi:.1})");
        assert!((lo / elo - 1.0).abs() < 0.002 && (hi / ehi - 1.0).abs() < 0.002);
    }
}

#[test]
fn feedback_extremes_stay_stable() {
    for fb in [-0.95f32, 0.95] {
        for centre in [20.0f32, 1000.0, 10000.0] {
            for stages in [4, 12] {
                let mut f = make(&PhaserParams { stages, feedback: fb, depth: 1.0, rate_hz: 20.0, centre_hz: centre, mix: 1.0, ..params() });
                let mut s = 1u32;
                let (mut peak, mut early, mut late) = (0.0f32, 0.0f64, 0.0f64);
                for i in 0..(10.0 * SR) as usize {
                    let x = if i < SR as usize { noise(&mut s) } else { 0.0 };
                    let (mut l, mut r) = (x, x);
                    f.process(&mut l, &mut r);
                    assert!(l.is_finite() && r.is_finite(), "fb {fb} centre {centre}: sample {i}");
                    peak = peak.max(l.abs()).max(r.abs());
                    let e = (l as f64).powi(2) + (r as f64).powi(2);
                    if (SR as usize..2 * SR as usize).contains(&i) {
                        early += e;
                    } else if i >= 9 * SR as usize {
                        late += e;
                    }
                }
                let decay_db = 10.0 * (late.max(1e-30) / early).log10();
                println!("phaser fb {fb} centre {centre} {stages} st: peak {peak:.2}, last s vs first s after input {decay_db:.0} dB");
                assert!(peak < 50.0);
                // Decaying, not growing. A 20 Hz centre rings longest: twelve stages delay the low
                // end by ~0.2 s per trip round the 0.95 loop.
                assert!(decay_db < -30.0, "fb {fb} centre {centre}: {decay_db:.1} dB");
            }
        }
    }
}

#[test]
fn sync_follows_the_tempo() {
    // 150 BPM: a 1-beat period is 0.4 s, 1/8 (half a beat) 0.2 s.
    for (beats, secs) in [(1.0f32, 0.4f64), (0.5, 0.2), (1.5, 0.6)] {
        let mut f = make(&PhaserParams { sync: true, sync_beats: beats, ..params() });
        f.set_transport(150.0, Some(0.0));
        let n = (secs * SR as f64).round() as usize;
        for _ in 0..n {
            let (mut l, mut r) = (0.1, 0.1);
            f.process(&mut l, &mut r);
        }
        let ph = f.lfo_phase();
        let err = ph.min(1.0 - ph) * secs * SR as f64;
        println!("phaser sync {beats} beats at 150 BPM: phase after {secs} s = {ph:.6} ({err:.3} samples off)");
        assert!(err < 1.0);
    }
    // The phase locks to the beat position.
    let mut f = make(&PhaserParams { sync: true, sync_beats: 4.0, ..params() });
    f.set_transport(140.0, Some(3.0));
    assert!((f.lfo_phase() - 0.75).abs() < 1e-12);
}

#[test]
fn off_is_bit_exact() {
    // Off from the start: exact from sample 0.
    let mut f = make(&PhaserParams { on: false, ..params() });
    let (mut l, mut r) = (0.25f32, -0.5f32);
    f.process(&mut l, &mut r);
    assert_eq!((l, r), (0.25, -0.5));
    // On, then off: exact once the fade has finished.
    let mut f = make(&PhaserParams { feedback: 0.9, ..params() });
    let mut s = 7u32;
    let mut exact_from = None;
    for i in 0..48000usize {
        if i == 9600 {
            f.set(&PhaserParams { on: false, feedback: 0.9, ..params() });
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
    println!("phaser: bit-exact pass-through {:.1} ms after on = false", from as f64 / 48.0);
    assert!(from < 9600 && f.is_bypassed());
}

#[test]
fn changes_do_not_click() {
    // A 300 Hz sine through a slow sweep leaves nothing above 10 kHz; a click does.
    let a = PhaserParams { stages: 4, depth: 0.2, feedback: 0.0, centre_hz: 400.0, mix: 0.3, rate_hz: 0.2, ..params() };
    let b = PhaserParams { stages: 12, depth: 0.6, feedback: 0.7, centre_hz: 2000.0, mix: 1.0, rate_hz: 0.2, ..params() };
    let off = PhaserParams { on: false, ..b };
    for (name, x, y) in [("steady", a, a), ("everything", a, b), ("on/off", b, off)] {
        let mut f = make(&x);
        let mut hp = hp10k();
        let mut peak: f64 = 0.0;
        for i in 0..96000usize {
            if i % 12000 == 6000 {
                f.set(if (i / 12000) % 2 == 0 { &y } else { &x });
            }
            let s = (0.5 * (TAU * 300.0 * i as f64 / SR as f64).sin()) as f32;
            let (mut l, mut r) = (s, s);
            f.process(&mut l, &mut r);
            let h = hp(l as f64);
            if i > 4800 {
                peak = peak.max(h.abs());
            }
        }
        let db = 20.0 * peak.log10();
        println!("phaser clicks, {name}: peak above 10 kHz {db:.1} dBFS");
        assert!(db < -70.0, "{name}: {db:.1} dBFS");
    }
}

/// 4th-order Butterworth-ish high-pass at 10 kHz (two RBJ sections).
fn hp10k() -> impl FnMut(f64) -> f64 {
    let sec = || {
        let w = TAU * 10000.0 / SR as f64;
        let alpha = w.sin() / (2.0 * std::f64::consts::FRAC_1_SQRT_2);
        let a0 = 1.0 + alpha;
        let (b0, b1) = ((1.0 + w.cos()) / 2.0 / a0, -(1.0 + w.cos()) / a0);
        let (a1, a2) = (-2.0 * w.cos() / a0, (1.0 - alpha) / a0);
        let (mut x1, mut x2, mut y1, mut y2) = (0.0, 0.0, 0.0, 0.0);
        move |x: f64| {
            let y = b0 * x + b1 * x1 + b0 * x2 - a1 * y1 - a2 * y2;
            (x2, x1, y2, y1) = (x1, x, y1, y);
            y
        }
    };
    let (mut a, mut b) = (sec(), sec());
    move |x| b(a(x))
}

#[test]
fn process_does_not_allocate() {
    let mut f = Phaser::new();
    let p = PhaserParams { stages: 12, sync: true, feedback: -0.8, ..params() };
    no_alloc(|| {
        f.set_sample_rate(96000.0);
        f.set(&p);
        f.reset();
        for i in 0..20000 {
            if i % 512 == 0 {
                f.set(&PhaserParams { stages: 4 + 2 * ((i / 512) % 5) as u32, on: (i / 4096) % 2 == 0, ..p });
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
    for stages in [4, 12] {
        let mut f = make(&PhaserParams { stages, feedback: 0.5, ..params() });
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
        println!("phaser CPU {stages} stages:{ns:.1} ns/sample stereo, {:.3} % of a core at 48 kHz ({acc:.1})", ns * 48000.0 / 1e7);
    }
}
