//! SPEC Verification 1 "demo": 8 bars at 150 BPM, a riddim call-and-response mono line on
//! F1 / G1 / A#1 in 1/8 notes, rendered through the full engine (MIDI events, transport playing)
//! to `demo/gts-demo.wav` on the v2 init patch (clean bass) and to `demo/gts-riddim-v1.wav` on the
//! v1 wobble patch (`Patch::riddim_v1`), both 16-bit, 48 kHz, stereo.
//!
//! cargo run -p gts --release --example demo
use gts::dsp::{Engine, Patch, Transport};
use nih_plug::prelude::NoteEvent;
use std::io::Write;

pub const SR: u32 = 48000;
pub const BPM: f64 = 150.0;
pub const BARS: usize = 8;
/// Samples per 1/8 note (0.2 s at 150 BPM).
pub const EIGHTH: usize = (SR as f64 * 30.0 / BPM) as usize;
const BLOCK: usize = 512;

const F1: u8 = 29;
const G1: u8 = 31;
const AS1: u8 = 34;

/// (start, length) in 1/8 notes, note. A length past the next start overlaps it: a legato slide
/// (no retrigger, so no fresh wobble, 80 ms glide).
type Phrase = &'static [(f32, f32, u8)];
// Two bars each: a call on F1 answered on A#1 / G1.
#[rustfmt::skip]
const A: Phrase = &[
    (0.0, 0.6, F1), (1.0, 0.6, F1), (3.0, 1.7, F1), (6.0, 0.6, G1), (7.0, 0.5, F1),
    (8.0, 0.6, AS1), (9.0, 2.25, AS1), (11.0, 0.9, G1), (14.0, 0.6, F1), (15.0, 0.5, F1),
];
#[rustfmt::skip]
const B: Phrase = &[
    (0.0, 0.6, F1), (1.0, 0.6, F1), (2.0, 0.6, G1), (3.0, 2.25, F1), (5.0, 0.8, G1), (7.0, 0.5, F1),
    (8.0, 0.6, AS1), (9.0, 0.6, AS1), (11.0, 1.25, G1), (12.0, 1.7, F1), (15.0, 0.5, G1),
];
#[rustfmt::skip]
const END: Phrase = &[
    (0.0, 0.6, F1), (1.0, 0.6, F1), (2.0, 0.6, G1), (3.0, 2.25, F1), (5.0, 0.8, G1), (7.0, 0.5, F1),
    (8.0, 0.6, AS1), (9.0, 1.25, G1), (10.0, 0.6, F1), (12.0, 3.5, F1),
];

/// Note events as (sample, note, on), sorted.
fn events() -> Vec<(usize, u8, bool)> {
    let mut ev = vec![];
    for (i, phrase) in [A, B, A, END].iter().enumerate() {
        let base = i as f32 * 16.0;
        for &(start, len, n) in phrase.iter() {
            let at = |t: f32| ((base + t) * EIGHTH as f32) as usize;
            ev.push((at(start), n, true));
            ev.push((at(start + len), n, false));
        }
    }
    // Offs before ons at the same sample, so a repeated note retriggers.
    ev.sort_by_key(|&(t, _, on)| (t, on));
    ev
}

/// The init patch: interleaved stereo frames, 8 bars plus a short release tail.
pub fn render() -> Vec<[f32; 2]> {
    render_patch(&Patch::default())
}

pub fn render_patch(patch: &Patch) -> Vec<[f32; 2]> {
    let patch = *patch;
    let mut e = Engine::new();
    e.initialize(SR as f32);
    e.transport = Transport { tempo: BPM, playing: true, pos_beats: 0.0 };
    let ev = events();
    let mut next = 0;
    let len = BARS * 8 * EIGHTH + SR as usize / 2;
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        if i % BLOCK == 0 {
            e.set_patch(&patch);
        }
        while next < ev.len() && ev[next].0 <= i {
            let (_, note, on) = ev[next];
            let event = if on {
                NoteEvent::NoteOn { timing: 0, voice_id: None, channel: 0, note, velocity: 1.0 }
            } else {
                NoteEvent::NoteOff { timing: 0, voice_id: None, channel: 0, note, velocity: 0.0 }
            };
            e.handle_event(&event);
            next += 1;
        }
        let (l, r) = e.next_frame();
        out.push([l, r]);
    }
    out
}

/// Minimal 16-bit PCM stereo WAV.
pub fn write_wav(path: &std::path::Path, frames: &[[f32; 2]]) -> std::io::Result<()> {
    let data = (frames.len() * 4) as u32;
    let mut b = Vec::with_capacity(44 + data as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes()); // PCM
    b.extend_from_slice(&2u16.to_le_bytes()); // channels
    b.extend_from_slice(&SR.to_le_bytes());
    b.extend_from_slice(&(SR * 4).to_le_bytes()); // byte rate
    b.extend_from_slice(&4u16.to_le_bytes()); // block align
    b.extend_from_slice(&16u16.to_le_bytes()); // bits
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data.to_le_bytes());
    for s in frames.iter().flatten() {
        b.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes());
    }
    std::fs::File::create(path)?.write_all(&b)
}

#[allow(dead_code)] // tests/demo.rs includes this file for `render` only
fn main() -> std::io::Result<()> {
    for (name, patch) in [("gts-demo.wav", Patch::default()), ("gts-riddim-v1.wav", Patch::riddim_v1())] {
        let frames = render_patch(&patch);
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../demo").join(name);
        write_wav(&path, &frames)?;
        let flat: Vec<f32> = frames.iter().flatten().copied().collect();
        let rms = (flat.iter().map(|v| v * v).sum::<f32>() / flat.len() as f32).sqrt();
        let peak = flat.iter().fold(0f32, |m, v| m.max(v.abs()));
        println!("wrote {} ({:.2} s): RMS {:.1} dBFS, peak {:.3}", path.display(), frames.len() as f32 / SR as f32, 20.0 * rms.log10(), peak);
    }
    Ok(())
}
