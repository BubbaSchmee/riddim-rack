//! Preset render + QA (SPEC v2 2.5 subset, SQ-37/40): loads preset files, plays a 4-bar riddim
//! pattern at 150 BPM through the real Engine with the transport playing, writes a 16-bit 48 kHz
//! stereo WAV to `docs/audits/renders/presets-fx/<name>.wav` (gitignored) and prints level and
//! listen-proxy figures:
//! - peak / RMS dBFS (both channels, the 4 bars), crest factor, BS.1770 gated integrated LUFS;
//! - on the held note: fundamental (strongest peak within ±6 % of the played pitch, parabolic
//!   interpolation) and the alias floor (non-harmonic / harmonic energy, harmonic = within 6 bins
//!   of k·f0, as `tests/alias.rs`; unison detune and LFO sidebands count as non-harmonic too);
//! - centroid movement: octaves between the lowest and highest spectral centroid of 2048-sample
//!   frames inside the first 300 ms of the held note;
//! - share of energy below 150 Hz and L/R correlation above 200 Hz over the 4 bars.
//! SQ-37 pass/fail per preset (peak ≤ −3 dBFS, RMS −24..−14 dBFS, no NaN) and across the bank
//! (every LUFS within ±1 dB of the bank mean: the calibration target, inside SQ-37's ±2 dB).
//!
//! Basses play 1/8 call-and-response on F1 / G1 / A#1 with rests, bar 4 one held F1; presets whose
//! category starts with "Lead" play a phrase around F3 / A#3, bar 4 one held F3. Velocity 100.
//!
//! cargo run -p gts --release --example render_preset [-- [--canon] <preset.json>...]
//! No paths = every file in crates/gts/presets/candidates. `--canon` rewrites each file in the
//! canonical complete form (every parameter, sorted) after loading it.
use gts::dsp::tables::Fft;
use gts::dsp::{Engine, Patch, Transport};
use gts::preset::Preset;
use nih_plug::prelude::NoteEvent;
use std::path::{Path, PathBuf};

#[path = "demo.rs"]
#[allow(dead_code)]
mod demo;

pub const SR: u32 = 48000;
pub const BPM: f64 = 150.0;
pub const EIGHTH: usize = (SR as f64 * 30.0 / BPM) as usize;
const BARS_LEN: usize = 32 * EIGHTH;
const BLOCK: usize = 512;
const VELOCITY: f32 = 100.0 / 127.0;
/// Held-note analysis window: 65536 samples from 0.2 s after its note-on.
const HELD_SKIP: usize = SR as usize / 5;
const HELD_WIN: usize = 65536;

type Phrase = &'static [(f32, f32, u8)];
const F1: u8 = 29;
const G1: u8 = 31;
const AS1: u8 = 34;
const F3: u8 = 53;
const GS3: u8 = 56;
const AS3: u8 = 58;
const C4: u8 = 60;
/// (start, length) in 1/8 notes, note. The last entry is the held note.
#[rustfmt::skip]
const BASS: Phrase = &[
    (0.0, 0.6, F1), (1.0, 0.6, F1), (3.0, 1.7, F1), (6.0, 0.6, G1), (7.0, 0.5, F1),
    (8.0, 0.6, AS1), (9.0, 0.6, AS1), (11.0, 0.9, G1), (14.0, 0.6, F1), (15.0, 0.5, F1),
    (16.0, 0.6, F1), (17.0, 0.6, F1), (18.0, 0.6, G1), (19.0, 1.7, F1), (22.0, 0.6, G1), (23.0, 0.5, F1),
    (24.0, 7.9, F1),
];
#[rustfmt::skip]
const LEAD: Phrase = &[
    (0.0, 0.9, F3), (1.0, 0.9, GS3), (2.0, 1.8, AS3), (4.0, 0.9, C4), (5.0, 0.9, AS3), (6.0, 1.8, GS3),
    (8.0, 0.9, F3), (10.0, 0.9, F3), (11.0, 0.9, GS3), (12.0, 3.8, AS3),
    (16.0, 0.9, F3), (17.0, 0.9, GS3), (18.0, 1.8, AS3), (20.0, 0.9, C4), (21.0, 0.9, AS3), (22.0, 1.8, GS3),
    (24.0, 7.9, F3),
];

pub fn hz(note: u8) -> f64 {
    440.0 * 2f64.powf((note as f64 - 69.0) / 12.0)
}

pub fn is_lead(p: &Preset) -> bool {
    p.category.starts_with("Lead")
}

pub struct Render {
    /// The 4 bars plus a 0.5 s tail.
    pub frames: Vec<[f32; 2]>,
    pub held_on: usize,
    pub held_note: u8,
    /// Widest unison detune (cents) of a sounding oscillator, 0 without unison.
    pub detune: f32,
}

pub fn render(patch: &Patch, lead: bool) -> Render {
    let phrase = if lead { LEAD } else { BASS };
    let mut ev: Vec<(usize, u8, bool)> = vec![];
    for &(start, len, n) in phrase {
        ev.push(((start * EIGHTH as f32) as usize, n, true));
        ev.push((((start + len) * EIGHTH as f32) as usize, n, false));
    }
    ev.sort_by_key(|&(t, _, on)| (t, on));
    let &(held_start, _, held_note) = phrase.last().unwrap();
    let mut e = Engine::new();
    e.initialize(SR as f32);
    e.transport = Transport { tempo: BPM, playing: true, pos_beats: 0.0 };
    let len = BARS_LEN + SR as usize / 2;
    let mut frames = Vec::with_capacity(len);
    let mut next = 0;
    for i in 0..len {
        if i % BLOCK == 0 {
            e.set_patch(patch);
        }
        while next < ev.len() && ev[next].0 <= i {
            let (_, note, on) = ev[next];
            e.handle_event(&if on {
                NoteEvent::NoteOn { timing: 0, voice_id: None, channel: 0, note, velocity: VELOCITY }
            } else {
                NoteEvent::NoteOff { timing: 0, voice_id: None, channel: 0, note, velocity: 0.0 }
            });
            next += 1;
        }
        let (l, r) = e.next_frame();
        frames.push([l, r]);
    }
    let detune = patch.osc.iter().filter(|o| o.on && o.unison > 1).map(|o| o.detune).fold(0.0, f32::max);
    Render { frames, held_on: (held_start * EIGHTH as f32) as usize, held_note, detune }
}

#[derive(Debug, Clone, Copy)]
pub struct Qa {
    pub peak_db: f64,
    pub rms_db: f64,
    pub crest_db: f64,
    pub lufs: f64,
    pub alias_db: f64,
    /// The same with each harmonic's band widened by the unison detune spread (detuned copies of
    /// a harmonic are not aliases).
    pub alias_uni_db: f64,
    pub f0_hz: f64,
    pub f0_err_pct: f64,
    /// Fundamental peak level against the strongest spectral peak, dB.
    pub f0_rel_db: f64,
    pub centroid_oct: f64,
    pub low_share: f64,
    pub corr_hi: f64,
    pub finite: bool,
}

fn db(x: f64) -> f64 {
    10.0 * x.max(1e-30).log10()
}

fn biquad(x: &[f64], b: [f64; 3], a: [f64; 3]) -> Vec<f64> {
    let (mut x1, mut x2, mut y1, mut y2) = (0.0, 0.0, 0.0, 0.0);
    x.iter()
        .map(|&v| {
            let y = b[0] * v + b[1] * x1 + b[2] * x2 - a[1] * y1 - a[2] * y2;
            (x2, x1, y2, y1) = (x1, v, y1, y);
            y
        })
        .collect()
}

/// BS.1770 K-weighting at 48 kHz.
fn k_weight(x: &[f64]) -> Vec<f64> {
    let y = biquad(x, [1.53512485958697, -2.69169618940638, 1.19839281085285], [1.0, -1.69065929318241, 0.73248077421585]);
    biquad(&y, [1.0, -2.0, 1.0], [1.0, -1.99004745483398, 0.99007225036621])
}

/// BS.1770-4 integrated loudness of a stereo signal: 400 ms blocks, 75 % overlap, -70 LUFS
/// absolute and -10 LU relative gates.
pub fn lufs(l: &[f64], r: &[f64]) -> f64 {
    let (kl, kr) = (k_weight(l), k_weight(r));
    let (blk, hop) = (SR as usize * 2 / 5, SR as usize / 10);
    let z: Vec<f64> = (0..=(kl.len().saturating_sub(blk)) / hop)
        .map(|j| {
            let s = j * hop;
            (kl[s..s + blk].iter().map(|v| v * v).sum::<f64>() + kr[s..s + blk].iter().map(|v| v * v).sum::<f64>()) / blk as f64
        })
        .collect();
    let loud = |z: f64| -0.691 + db(z);
    let gated = |thr: f64| -> Vec<f64> { z.iter().copied().filter(|&v| loud(v) > thr).collect() };
    let abs = gated(-70.0);
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len().max(1) as f64;
    let rel = loud(mean(&abs)) - 10.0;
    loud(mean(&gated(rel.max(-70.0))))
}

/// Blackman-Harris power spectrum (power-of-two length) and bin width.
fn spectrum(x: &[f64]) -> (Vec<f64>, f64) {
    let n = x.len();
    let tau = std::f64::consts::TAU;
    let mut re: Vec<f64> = x
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let t = tau * i as f64 / (n - 1) as f64;
            v * (0.35875 - 0.48829 * t.cos() + 0.14128 * (2.0 * t).cos() - 0.01168 * (3.0 * t).cos())
        })
        .collect();
    let mut im = vec![0.0; n];
    Fft::new(n).run(&mut re, &mut im, false);
    ((0..=n / 2).map(|k| re[k] * re[k] + im[k] * im[k]).collect(), SR as f64 / n as f64)
}

/// Spectral centroid (magnitude-weighted, 20 Hz..20 kHz) of a Hann-windowed frame.
fn centroid(x: &[f64], fft: &Fft) -> f64 {
    let n = x.len();
    let mut re: Vec<f64> =
        x.iter().enumerate().map(|(i, v)| v * (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / n as f64).cos())).collect();
    let mut im = vec![0.0; n];
    fft.run(&mut re, &mut im, false);
    let df = SR as f64 / n as f64;
    let (mut num, mut den) = (0.0, 0.0);
    for k in 1..n / 2 {
        let f = k as f64 * df;
        if (20.0..=20000.0).contains(&f) {
            let m = (re[k] * re[k] + im[k] * im[k]).sqrt();
            num += f * m;
            den += m;
        }
    }
    num / den.max(1e-30)
}

/// 4th-order Butterworth high-pass at `fc` (two RBJ sections).
fn highpass4(x: &[f64], fc: f64) -> Vec<f64> {
    let mut y = x.to_vec();
    for q in [0.541_196_1, 1.306_563] {
        let w = std::f64::consts::TAU * fc / SR as f64;
        let (c, alpha) = (w.cos(), w.sin() / (2.0 * q));
        let a0 = 1.0 + alpha;
        let b = [(1.0 + c) / 2.0 / a0, -(1.0 + c) / a0, (1.0 + c) / 2.0 / a0];
        y = biquad(&y, b, [1.0, -2.0 * c / a0, (1.0 - alpha) / a0]);
    }
    y
}

pub fn measure(r: &Render) -> Qa {
    let bars = &r.frames[..BARS_LEN];
    let finite = r.frames.iter().flatten().all(|v| v.is_finite());
    let l: Vec<f64> = bars.iter().map(|f| f[0] as f64).collect();
    let rr: Vec<f64> = bars.iter().map(|f| f[1] as f64).collect();
    let peak = r.frames.iter().flatten().fold(0f64, |m, v| m.max(v.abs() as f64));
    let ms = (l.iter().map(|v| v * v).sum::<f64>() + rr.iter().map(|v| v * v).sum::<f64>()) / (2 * bars.len()) as f64;
    let (peak_db, rms_db) = (20.0 * peak.max(1e-15).log10(), db(ms));

    // Held note: fundamental and alias floor.
    let f0 = hz(r.held_note);
    let s = r.held_on + HELD_SKIP;
    let mono: Vec<f64> = r.frames[s..s + HELD_WIN].iter().map(|f| 0.5 * (f[0] + f[1]) as f64).collect();
    let (p, df) = spectrum(&mono);
    let k0 = ((0.94 * f0 / df) as usize).max(2);
    let k1 = (1.06 * f0 / df) as usize + 1;
    let kp = (k0..=k1).max_by(|&a, &b| p[a].total_cmp(&p[b])).unwrap();
    let (a, b, c) = (db(p[kp - 1]), db(p[kp]), db(p[kp + 1]));
    let f0_hz = (kp as f64 + 0.5 * (a - c) / (a - 2.0 * b + c)) * df;
    let pmax = p.iter().skip(1).copied().fold(0.0, f64::max);
    let spread = 2f64.powf(r.detune as f64 / 1200.0) - 1.0;
    let floor = |spread: f64| {
        let (mut h, mut nh) = (0.0, 0.0);
        for (k, &v) in p.iter().enumerate() {
            let f = k as f64 * df;
            if f <= 15.0 {
                continue;
            }
            let kk = (f / f0).round();
            if kk >= 1.0 && (f - kk * f0).abs() <= 6.0 * df + spread * kk * f0 {
                h += v;
            } else {
                nh += v;
            }
        }
        db(nh / h.max(1e-30))
    };

    // Centroid movement in the first 300 ms of the held note.
    let n = 2048;
    let fft = Fft::new(n);
    let cs: Vec<f64> = (r.held_on..r.held_on + SR as usize * 3 / 10 - n)
        .step_by(256)
        .map(|i| {
            let x: Vec<f64> = r.frames[i..i + n].iter().map(|f| 0.5 * (f[0] + f[1]) as f64).collect();
            centroid(&x, &fft)
        })
        .collect();
    let (cmin, cmax) = cs.iter().fold((f64::MAX, 0f64), |(lo, hi), &c| (lo.min(c), hi.max(c)));

    // Energy below 150 Hz over the 4 bars (zero-padded FFT, Parseval).
    let len = bars.len().next_power_of_two();
    let mut re: Vec<f64> = bars.iter().map(|f| 0.5 * (f[0] + f[1]) as f64).collect();
    re.resize(len, 0.0);
    let mut im = vec![0.0; len];
    Fft::new(len).run(&mut re, &mut im, false);
    let dfl = SR as f64 / len as f64;
    let (mut lo, mut tot) = (0.0, 0.0);
    for k in 1..len / 2 {
        let e = re[k] * re[k] + im[k] * im[k];
        tot += e;
        if (k as f64 * dfl) < 150.0 {
            lo += e;
        }
    }

    // L/R correlation above 200 Hz.
    let (hl, hr) = (highpass4(&l, 200.0), highpass4(&rr, 200.0));
    let dot = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
    let corr_hi = dot(&hl, &hr) / (dot(&hl, &hl) * dot(&hr, &hr)).sqrt().max(1e-30);

    Qa {
        peak_db,
        rms_db,
        crest_db: peak_db - rms_db,
        lufs: lufs(&l, &rr),
        alias_db: floor(0.0),
        alias_uni_db: floor(spread),
        f0_hz,
        f0_err_pct: 100.0 * (f0_hz / f0 - 1.0),
        f0_rel_db: db(p[kp] / pmax),
        centroid_oct: (cmax / cmin).log2(),
        low_share: lo / tot.max(1e-30),
        corr_hi,
        finite,
    }
}

/// SQ-37 per-preset violations (empty = pass).
pub fn sq37(q: &Qa) -> Vec<String> {
    let mut v = vec![];
    if !q.finite {
        v.push("NaN/inf in output".into());
    }
    if q.peak_db > -3.0 {
        v.push(format!("peak {:.2} dBFS > -3", q.peak_db));
    }
    if !(-24.0..=-14.0).contains(&q.rms_db) {
        v.push(format!("RMS {:.2} dBFS outside -24..-14", q.rms_db));
    }
    v
}

/// Bank calibration: each LUFS within ±1 dB of the bank mean (SQ-37 allows ±2).
pub fn bank_violations(names: &[String], lufs: &[f64]) -> Vec<String> {
    let mean = lufs.iter().sum::<f64>() / lufs.len() as f64;
    names
        .iter()
        .zip(lufs)
        .filter(|(_, &l)| (l - mean).abs() > 1.0)
        .map(|(n, l)| format!("{n}: {l:.2} LUFS is {:+.2} dB from the bank mean {mean:.2}", l - mean))
        .collect()
}

pub fn candidates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("presets/candidates")
}

pub fn candidates() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(candidates_dir())
        .expect("presets/candidates")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    v.sort();
    v
}

pub fn print(name: &str, q: &Qa, held_note: u8) {
    println!(
        "{name}: peak {:.2} dBFS, RMS {:.2} dBFS, {:.2} LUFS, crest {:.2} dB, alias floor {:.1} dB (unison-aware {:.1}), \
         f0 {:.2} Hz (played {:.2}, {:+.2} %, {:.1} dB re max peak), centroid move {:.2} oct, \
         <150 Hz {:.1} %, corr >200 Hz {:.3}",
        q.peak_db, q.rms_db, q.lufs, q.crest_db, q.alias_db, q.alias_uni_db, q.f0_hz, hz(held_note), q.f0_err_pct, q.f0_rel_db,
        q.centroid_oct, 100.0 * q.low_share, q.corr_hi
    );
}

#[allow(dead_code)] // tests/preset_qa.rs includes this file for its functions
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let canon = args.iter().any(|a| a == "--canon");
    let mut paths: Vec<PathBuf> = args.iter().filter(|a| !a.starts_with("--")).map(PathBuf::from).collect();
    if paths.is_empty() {
        paths = candidates();
    }
    let out_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/audits/renders/presets-fx");
    std::fs::create_dir_all(&out_dir).unwrap();
    let (mut names, mut louds, mut fails) = (vec![], vec![], 0);
    for path in &paths {
        let (preset, warnings) = Preset::load(path).unwrap_or_else(|e| panic!("{e}"));
        for w in &warnings {
            println!("  warning: {w}");
        }
        if canon {
            preset.save(path).unwrap();
        }
        let r = render(&preset.patch, is_lead(&preset));
        let stem = path.file_stem().unwrap().to_string_lossy().to_string();
        demo::write_wav(&out_dir.join(format!("{stem}.wav")), &r.frames).unwrap();
        let q = measure(&r);
        print(&preset.name, &q, r.held_note);
        let v = sq37(&q);
        println!("  SQ-37: {}", if v.is_empty() { "PASS".to_string() } else { format!("FAIL ({})", v.join("; ")) });
        fails += v.len();
        names.push(preset.name.clone());
        louds.push(q.lufs);
    }
    if names.len() > 1 {
        let bank = bank_violations(&names, &louds);
        let (lo, hi) = louds.iter().fold((f64::MAX, f64::MIN), |(a, b), &l| (a.min(l), b.max(l)));
        println!("bank: LUFS {lo:.2} .. {hi:.2} (spread {:.2} dB): {}", hi - lo, if bank.is_empty() { "PASS".into() } else { bank.join("; ") });
        fails += bank.len();
    }
    println!("wrote {}", out_dir.display());
    if fails > 0 {
        std::process::exit(1);
    }
}
