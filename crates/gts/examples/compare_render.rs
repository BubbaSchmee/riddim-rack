//! Raw-tone renders for `tools/compare.py` against the owner's reference (a commercial synth's
//! init patch, one held MIDI 60 note, wavetable position switched at each 4-bar line, 140 BPM,
//! 44.1 kHz). The init patch made RAW: routing Bypass (no filters), shaper off, sub off, unison 1,
//! amp env attack 1 ms / sustain 1 / init release (20 ms), velocity 100, init crossover (off),
//! oversampling default (2x).
//! Writes 32-bit float 44.1 kHz stereo WAVs to `docs/audits/renders/compare/` (gitignored):
//! - `gts-sequence.wav`: one note held 20 bars, (table, index) switched at each 4-bar line in the
//!   reference's order (saw, sine, triangle, odd blend, square) on the Basic table, like the
//!   reference's gate (the blend has no counterpart: it plays the square);
//! - one 4-bar note per Basic key frame (`gts-Basic-<index>.wav`).
//!
//! Also prints, for every table and grain, the distance of its harmonics 1..40 (dB re h1, clipped
//! at -100) to a textbook triangle (odd 1/k^2), to find the closest triangle-like position.
//!
//! cargo run -p gts --release --example compare_render [-- <midi note> [xover120]]
use gts::dsp::tables::{mip_for, mip_len, Fft, Tables, GRAINS, TOP};
use gts::dsp::{Engine, Patch, Transport};
use gts::params::{Routing, Table};
use nih_plug::prelude::{Enum, NoteEvent};
use std::io::Write;
use std::path::Path;

const SR: u32 = 44_100;
const BPM: f64 = 140.0;
const BLOCK: usize = 512;
/// Basic table key frames: grain 10 (triangle), 21 (saw), 31 (square) as Index values.
const TRI: f32 = 10.0 * 127.0 / 31.0;
const SAW: f32 = 21.0 * 127.0 / 31.0;
const SQUARE: f32 = 127.0;

fn diag() -> bool {
    std::env::args().nth(2).as_deref() == Some("xover120")
}

fn raw() -> Patch {
    let mut p = Patch::default();
    // Diagnostic only (`-- 60 xover120`): the v2 init's 120 Hz mono crossover, whose LR8 allpass
    // rotates the phases of the mid.
    if diag() {
        p.mono_xover = 120.0;
    }
    p.routing = Routing::Bypass;
    p.shaper_on = false;
    p.sub_on = false;
    let o = &mut p.osc[0];
    o.unison = 1;
    o.att = 0.001;
    o.sus = 1.0;
    p.osc[1].on = false;
    p
}

/// One note held over `steps` (table, index) settings of 4 bars each, plus a 0.5 s tail.
fn render(note: u8, steps: &[(Table, f32)]) -> Vec<[f32; 2]> {
    let bar4 = (SR as f64 * 60.0 / BPM * 16.0).round() as usize;
    let len = bar4 * steps.len() + SR as usize / 2;
    let mut e = Engine::new();
    e.initialize(SR as f32);
    e.transport = Transport { tempo: BPM, playing: true, pos_beats: 0.0 };
    let mut p = raw();
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        if i % BLOCK == 0 || i % bar4 == 0 {
            if let Some(&(t, idx)) = steps.get(i / bar4) {
                (p.osc[0].table, p.osc[0].index) = (t, idx);
            }
            e.set_patch(&p);
        }
        if i == 0 {
            e.handle_event(&NoteEvent::NoteOn { timing: 0, voice_id: None, channel: 0, note, velocity: 100.0 / 127.0 });
        }
        if i == bar4 * steps.len() {
            e.handle_event(&NoteEvent::NoteOff { timing: 0, voice_id: None, channel: 0, note, velocity: 0.0 });
        }
        let (l, r) = e.next_frame();
        out.push([l, r]);
    }
    out
}

fn write_f32_wav(path: &Path, frames: &[[f32; 2]]) -> std::io::Result<()> {
    let data = (frames.len() * 8) as u32;
    let mut b = Vec::with_capacity(44 + data as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&3u16.to_le_bytes()); // IEEE float
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&SR.to_le_bytes());
    b.extend_from_slice(&(SR * 8).to_le_bytes());
    b.extend_from_slice(&8u16.to_le_bytes());
    b.extend_from_slice(&32u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data.to_le_bytes());
    for s in frames.iter().flatten() {
        b.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::File::create(path)?.write_all(&b)
}

/// Spectrum of mip `m` of one grain with the B-spline prefilter undone: (re, im) per bin.
fn mip_spectrum(t: &Tables, table: Table, g: usize, m: usize) -> (Vec<f64>, Vec<f64>) {
    let f = t.mip(table, g, m);
    let n = f.len();
    let (mut re, mut im): (Vec<f64>, Vec<f64>) = (f.iter().map(|&v| v as f64).collect(), vec![0.0; n]);
    Fft::new(mip_len(0)).run(&mut re, &mut im, false);
    for h in 0..n {
        let b = (4.0 + 2.0 * (std::f64::consts::TAU * h as f64 / n as f64).cos()) / 6.0;
        (re[h], im[h]) = (re[h] * b, im[h] * b);
    }
    (re, im)
}

/// Harmonics 1..40 of one grain at the mip the oscillator would read for `f0`, dB re h1.
fn grain_harm(t: &Tables, table: Table, g: usize, f0: f64) -> Vec<f64> {
    let m = mip_for(f0 / SR as f64, TOP);
    let (re, im) = mip_spectrum(t, table, g, m);
    let n = re.len();
    let p: Vec<f64> = (1..=40).map(|h| if h < n / 2 { re[h] * re[h] + im[h] * im[h] } else { 0.0 }).collect();
    p.iter().map(|&v| (10.0 * (v / p[0].max(1e-30)).max(1e-30).log10()).max(-100.0)).collect()
}

fn main() -> std::io::Result<()> {
    let note: u8 = std::env::args().nth(1).map(|s| s.parse().expect("midi note")).unwrap_or(60);
    let f0 = 440.0 * 2f64.powf((note as f64 - 69.0) / 12.0);
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/audits/renders/compare");
    std::fs::create_dir_all(&dir)?;

    // Closest triangle-like grain.
    let mut tables = Tables::new();
    tables.generate();
    let tri: Vec<f64> = (1..=40).map(|k| if k % 2 == 1 { (-40.0 * (k as f64).log10()).max(-100.0) } else { -100.0 }).collect();
    let mut best: Vec<(f64, Table, usize)> = vec![];
    for tb in (0..Table::variants().len()).map(Table::from_index) {
        for g in 0..GRAINS {
            let h = grain_harm(&tables, tb, g, f0);
            let d = h.iter().zip(&tri).map(|(a, b)| (a - b).abs()).sum::<f64>() / 40.0;
            best.push((d, tb, g));
        }
    }
    best.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    println!("closest grains to a textbook triangle (mean |dB| h1-40, index = grain*127/31):");
    for (d, tb, g) in best.iter().take(6) {
        println!("  {tb:?} grain {g} (index {:.1}): {d:.2} dB", *g as f64 * 127.0 / 31.0);
    }
    // Stored phases of the candidate grains (harmonic k's phase minus k x h1's, as compare.py's
    // notes): a textbook sine-series saw steps by -pi/2, a cosine-series square by pi.
    for g in [10, 21, 31] {
        let (re, im) = mip_spectrum(&tables, Table::Basic, g, mip_for(f0 / SR as f64, TOP));
        let tb = Table::Basic;
        let p1 = im[1].atan2(re[1]);
        let rel: Vec<String> = (1..=8)
            .map(|k| {
                let d = im[k].atan2(re[k]) - k as f64 * p1;
                format!("{:+.2}", d.sin().atan2(d.cos()))
            })
            .collect();
        println!("table phases {tb:?} grain {g} (k=1..8): {}", rel.join(" "));
    }
    let m = mip_for(f0 / SR as f64, TOP);
    println!("f0 {f0:.3} Hz: mip {m} keeps {} harmonics (top {:.0} Hz)", gts::dsp::tables::harmonics(m), gts::dsp::tables::harmonics(m) as f64 * f0);

    let b = Table::Basic;
    let seq = [(b, SAW), (b, 0.0), (b, TRI), (b, SQUARE), (b, SQUARE)];
    if diag() {
        write_f32_wav(&dir.join("gts-sequence-xover120.wav"), &render(note, &seq))?;
        println!("wrote gts-sequence-xover120.wav (diagnostic)");
        return Ok(());
    }
    write_f32_wav(&dir.join("gts-sequence.wav"), &render(note, &seq))?;
    println!("wrote gts-sequence.wav: {seq:?}");
    for idx in [0.0, TRI, SAW, SQUARE] {
        let name = format!("gts-Basic-{idx:.0}.wav");
        write_f32_wav(&dir.join(&name), &render(note, &[(b, idx)]))?;
        println!("wrote {name}");
    }
    Ok(())
}
