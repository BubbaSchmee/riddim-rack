//! FX 4.7 Reverb acceptance (SPEC v2 Track 4; fx-section.md test 8). The module is included by
//! path until the integration wave declares `dsp::fx` in `dsp/mod.rs`. Numbers print with
//! `--nocapture`; run `cargo test --release --test fx_reverb -- --nocapture` for the CPU figure.
#[allow(dead_code)]
#[path = "../src/dsp/fx/reverb.rs"]
mod reverb;
use reverb::*;

const SR: f32 = 48000.0;

fn params(ty: ReverbType) -> ReverbParams {
    ReverbParams {
        on: true,
        ty,
        size: 0.5,
        decay: 2.0,
        damp: 0.0,
        predelay_ms: 0.0,
        lowcut_hz: 20.0,
        hicut_hz: 20000.0,
        width: 1.0,
        mix: 1.0,
    }
}

fn make(sr: f32, p: &ReverbParams) -> Reverb {
    let mut r = Reverb::new();
    r.set_sample_rate(sr);
    r.set(p);
    r.reset();
    r
}

fn run(r: &mut Reverb, n: usize, mut input: impl FnMut(usize) -> (f32, f32)) -> (Vec<f32>, Vec<f32>) {
    let (mut ol, mut or) = (Vec::with_capacity(n), Vec::with_capacity(n));
    for i in 0..n {
        let (mut l, mut rr) = input(i);
        r.process(&mut l, &mut rr);
        ol.push(l);
        or.push(rr);
    }
    (ol, or)
}

fn impulse(i: usize) -> (f32, f32) {
    if i == 0 {
        (1.0, 1.0)
    } else {
        (0.0, 0.0)
    }
}

fn noise(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*seed >> 8) as f32 / (1 << 24) as f32 * 2.0 - 1.0
}

/// Schroeder-integrated T20 (-5..-25 dB least-squares fit) extrapolated to 60 dB, seconds.
fn rt60(l: &[f32], r: &[f32], sr: f32) -> f64 {
    let e: Vec<f64> = l.iter().zip(r).map(|(&a, &b)| a as f64 * a as f64 + b as f64 * b as f64).collect();
    let mut edc = vec![0.0; e.len()];
    let mut acc = 0.0;
    for i in (0..e.len()).rev() {
        acc += e[i];
        edc[i] = acc;
    }
    let pts: Vec<(f64, f64)> = edc
        .iter()
        .enumerate()
        .map(|(i, &v)| (i as f64 / sr as f64, 10.0 * (v / edc[0]).log10()))
        .filter(|&(_, db)| (-25.0..=-5.0).contains(&db))
        .collect();
    let n = pts.len() as f64;
    let (sx, sy) = pts.iter().fold((0.0, 0.0), |(a, b), p| (a + p.0, b + p.1));
    let (mx, my) = (sx / n, sy / n);
    let (sxy, sxx) =
        pts.iter().fold((0.0, 0.0), |(a, b), p| (a + (p.0 - mx) * (p.1 - my), b + (p.0 - mx) * (p.0 - mx)));
    -60.0 / (sxy / sxx)
}

#[test]
fn rt60_matches_decay() {
    let mut worst: f64 = 0.0;
    for ty in [ReverbType::Plate, ReverbType::Hall] {
        for (size, decay) in [
            (0.5, 2.0),
            (0.0, 2.0),
            (1.0, 2.0),
            (0.0, 0.5),
            (0.5, 0.5),
            (1.0, 0.5),
            (0.5, 1.0),
            (1.0, 1.0),
            (0.5, 8.0),
            (1.0, 20.0),
            (0.0, 0.1),
            (0.5, 0.1),
            (0.0, 0.25),
            (0.5, 0.25),
        ] {
            let p = ReverbParams { size, decay, ..params(ty) };
            let mut rv = make(SR, &p);
            // Long enough that the -25 dB point sits well inside the record.
            let n = ((decay * 0.75 + 1.0) * SR) as usize;
            let (l, r) = run(&mut rv, n, impulse);
            let t = rt60(&l, &r, SR);
            let err = t / decay as f64 - 1.0;
            println!("{ty:?} size {size} decay {decay}: RT60 {t:.3} s ({:+.1} %)", 100.0 * err);
            if decay == 2.0 {
                assert!(err.abs() <= 0.15, "{ty:?} size {size}: RT60 {t:.3} s vs 2.0 s");
            }
            worst = worst.max(err.abs());
        }
    }
    println!("worst calibration error {:.1} %", 100.0 * worst);
    assert!(worst <= 0.15, "decay calibration off by more than 15 %");
}

/// In-place radix-2 FFT (len a power of two).
fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -std::f64::consts::TAU / len as f64;
        let (cr, ci) = (ang.cos(), ang.sin());
        let (mut wr, mut wi) = (1.0f64, 0.0f64);
        for k in 0..len / 2 {
            for start in (0..n).step_by(len) {
                let (a, b) = (start + k, start + k + len / 2);
                let (xr, xi) = (re[b] * wr - im[b] * wi, re[b] * wi + im[b] * wr);
                (re[b], im[b]) = (re[a] - xr, im[a] - xi);
                re[a] += xr;
                im[a] += xi;
            }
            (wr, wi) = (wr * cr - wi * ci, wr * ci + wi * cr);
        }
        len <<= 1;
    }
}

/// Power spectrum of `x` zero-padded to `n`.
fn power(x: &[f32], n: usize) -> Vec<f64> {
    let mut re: Vec<f64> = x.iter().map(|&v| v as f64).chain(std::iter::repeat(0.0)).take(n).collect();
    let mut im = vec![0.0; n];
    fft(&mut re, &mut im);
    (0..n / 2).map(|k| re[k] * re[k] + im[k] * im[k]).collect()
}

#[test]
fn low_cut_removes_the_sub_from_the_wet() {
    for ty in [ReverbType::Plate, ReverbType::Hall] {
        let n = 1 << 17;
        let below = |lowcut: f32| {
            let mut rv = make(SR, &ReverbParams { lowcut_hz: lowcut, ..params(ty) });
            let (l, r) = run(&mut rv, n, impulse);
            let hz = SR as f64 / n as f64;
            let (pl, pr) = (power(&l, n), power(&r, n));
            (1..(100.0 / hz) as usize).map(|k| pl[k] + pr[k]).sum::<f64>()
        };
        let db = 10.0 * (below(200.0) / below(20.0)).log10();
        println!("{ty:?}: energy below 100 Hz with low cut 200 Hz vs 20 Hz: {db:.1} dB");
        assert!(db <= -20.0);
    }
    assert_eq!(ReverbParams::default().lowcut_hz, 200.0);
}

#[test]
fn no_modal_ringing_in_the_tail() {
    let mut fails = 0;
    for ty in [ReverbType::Plate, ReverbType::Hall] {
        for size in [0.0, 0.5, 1.0] {
            let mut rv = make(SR, &ReverbParams { size, ..params(ty) });
            // Welch PSD of the tail (from 50 ms after the burst, 2 s long): 4096-point Hann
            // segments, 50 % overlap, averaged over 12 bursts. One 100 ms burst's own spectrum
            // is exponentially distributed per ~10 Hz, which alone puts ~8 dB chance peaks among
            // 670 bins; the ensemble removes that so what remains is the reverb's own structure.
            let seg = 4096;
            let mut psd = vec![0.0; seg / 2];
            let burst = (0.1 * SR) as usize;
            for run_i in 0..12u32 {
                rv.reset();
                let mut seed = 7 + 1000 * run_i;
                let (l, r) = run(&mut rv, (2.15 * SR) as usize, |i| {
                    if i < burst {
                        let v = noise(&mut seed);
                        (v, v)
                    } else {
                        (0.0, 0.0)
                    }
                });
                let mut s = burst + (0.05 * SR) as usize;
                while s + seg <= l.len() {
                    for ch in [&l, &r] {
                        let w: Vec<f32> = (0..seg)
                            .map(|i| ch[s + i] * (0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / seg as f32).cos()))
                            .collect();
                        for (a, b) in psd.iter_mut().zip(power(&w, seg)) {
                            *a += b;
                        }
                    }
                    s += seg / 2;
                }
            }
            // Each bin against the median of its +/- 1/6 octave neighbourhood, 200 Hz..8 kHz.
            let hz = SR as f64 / seg as f64;
            let mut worst = (0.0f64, 0.0f64);
            for k in (200.0 / hz) as usize..=(8000.0 / hz) as usize {
                let (a, b) =
                    ((k as f64 * 2f64.powf(-1.0 / 6.0)) as usize, (k as f64 * 2f64.powf(1.0 / 6.0)).ceil() as usize);
                let mut nb: Vec<f64> = psd[a..=b].to_vec();
                nb.sort_by(f64::total_cmp);
                let db = 10.0 * (psd[k] / nb[nb.len() / 2]).log10();
                if db > worst.0 {
                    worst = (db, k as f64 * hz);
                }
            }
            println!("{ty:?} size {size}: largest peak over the local median {:.2} dB at {:.0} Hz", worst.0, worst.1);
            fails += (worst.0 > 6.0) as usize;
        }
    }
    assert_eq!(fails, 0, "modal peaks above 6 dB");
}

fn correlation(l: &[f32], r: &[f32]) -> f64 {
    let (mut lr, mut ll, mut rr) = (0.0, 0.0, 0.0);
    for (&a, &b) in l.iter().zip(r) {
        lr += a as f64 * b as f64;
        ll += a as f64 * a as f64;
        rr += b as f64 * b as f64;
    }
    lr / (ll * rr).sqrt()
}

#[test]
fn width_goes_to_mono_and_mix_zero_nulls() {
    for ty in [ReverbType::Plate, ReverbType::Hall] {
        let mut prev = -2.0;
        for width in [1.0, 0.75, 0.5, 0.25, 0.0] {
            let mut rv = make(SR, &ReverbParams { width, ..params(ty) });
            let mut seed = 3u32;
            let (l, r) = run(&mut rv, 48000, |_| {
                let v = noise(&mut seed);
                (v, v)
            });
            let c = correlation(&l, &r);
            let rms = (l.iter().chain(&r).map(|&v| v as f64 * v as f64).sum::<f64>() / 96000.0).sqrt();
            // Input RMS is 1/sqrt(3) (uniform noise).
            println!(
                "{ty:?} width {width}: L/R correlation {c:.3}, wet {:+.1} dB re input",
                20.0 * (rms * 3f64.sqrt()).log10()
            );
            assert!(c > prev, "correlation must rise as width falls");
            prev = c;
            if width == 0.0 {
                assert!(l == r, "width 0 is mono");
            }
        }
        // Mix 0 and bypass return the input bit-exactly.
        let input = |i: usize| ((i as f32 * 0.37).sin() * 0.8, (i as f32 * 0.11).cos() * 0.5);
        for p in [ReverbParams { mix: 0.0, ..params(ty) }, ReverbParams { on: false, ..params(ty) }] {
            let mut rv = make(SR, &p);
            let (l, r) = run(&mut rv, 20000, input);
            assert!((0..20000).all(|i| (l[i], r[i]) == input(i)));
        }
    }
}

#[test]
fn predelay_and_192k() {
    for ty in [ReverbType::Plate, ReverbType::Hall] {
        for sr in [44100.0f32, 192000.0] {
            let p = ReverbParams { predelay_ms: 300.0, size: 1.0, decay: 20.0, ..params(ty) };
            let mut rv = make(sr, &p);
            let (l, r) = run(&mut rv, (1.5 * sr) as usize, impulse);
            let first = l.iter().zip(&r).position(|(a, b)| a.abs().max(b.abs()) > 1e-9).unwrap();
            println!("{ty:?} {sr} Hz: first wet sample at {:.1} ms", first as f32 / sr * 1000.0);
            assert!(first as f32 >= 0.3 * sr - 1.0);
            assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() < 4.0));
        }
    }
}

#[test]
fn cpu_bench() {
    for ty in [ReverbType::Plate, ReverbType::Hall] {
        let mut rv = make(
            SR,
            &ReverbParams { damp: 0.4, predelay_ms: 20.0, lowcut_hz: 200.0, hicut_hz: 10000.0, mix: 0.2, ..params(ty) },
        );
        let n = 480_000;
        let mut seed = 9u32;
        let mut acc = 0.0f32;
        let t = std::time::Instant::now();
        for _ in 0..n {
            let (mut l, mut r) = (0.3 * noise(&mut seed), 0.3 * noise(&mut seed));
            rv.process(&mut l, &mut r);
            acc += l + r;
        }
        let ns = t.elapsed().as_nanos() as f64 / n as f64;
        println!(
            "CPU reverb {ty:?}: {ns:.1} ns/sample stereo ({}) [{acc:.1}]",
            if cfg!(debug_assertions) { "debug" } else { "release" }
        );
    }
}
