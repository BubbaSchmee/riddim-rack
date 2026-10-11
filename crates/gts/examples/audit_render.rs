//! Audit helper (docs/audits/gts-v1-audit.md): renders jobs through the real Engine to raw f32.
//! Job file lines: `<out.f32> <secs> key=val ...`; `base=raw` starts from a measurement patch
//! (the v1 default patch = `Patch::riddim_v1`, Bypass routing, Mod A index 0, shaper off, -12 dB
//! so the limiter never engages).
//! A line `bench <voices> <secs> [unison] [v1]` times the init patch (or `v1` = the riddim_v1
//! fixture) with N voices held and Osc A at `unison` voices;
//! `bench_shaper <factor> <secs>` times one voice's shaper at 1x / 2x / 4x oversampling.
//! cargo run -p gts --release --example audit_render -- jobs.txt
use gts::dsp::{Engine, Patch};
use gts::params::*;
use nih_plug::prelude::{Enum, NoteEvent};
use std::io::Write;

const SR: f32 = 48000.0;

fn patch(kv: &[(&str, &str)]) -> Patch {
    let mut p = Patch::riddim_v1();
    let f = |v: &str| v.parse::<f32>().unwrap();
    for &(k, v) in kv {
        match k {
            "base" if v == "raw" => {
                p.routing = Routing::Bypass;
                p.moda_index = 0.0;
                p.shaper_on = false;
                p.volume_db = -12.0;
            }
            "table" => p.osc[0].table = Table::from_index(v.parse().unwrap()),
            "index" => p.osc[0].index = f(v),
            "shift" => p.osc[0].shift = f(v),
            "att" => p.osc[0].att = f(v),
            "dec" => p.osc[0].dec = f(v),
            "sus" => p.osc[0].sus = f(v),
            "rel" => p.osc[0].rel = f(v),
            "motion" => p.osc[0].motion = f(v),
            "routing" => p.routing = Routing::from_index(v.parse().unwrap()),
            "shaper_on" => p.shaper_on = v == "1",
            "shaper_mode" => p.shaper_mode = ShaperMode::from_index(v.parse().unwrap()),
            "shaper_amt" => p.shaper_amt = f(v),
            "fa_type" => p.filt[0].ty = FilterType::from_index(v.parse().unwrap()),
            "fa_freq" => p.filt[0].freq = f(v),
            "fa_res" => p.filt[0].res = f(v),
            "fa_kbd" => p.filt[0].kbd = f(v),
            "fa_env" => p.filt[0].env = v == "1",
            "fb_type" => p.filt[1].ty = FilterType::from_index(v.parse().unwrap()),
            "fb_freq" => p.filt[1].freq = f(v),
            "fb_res" => p.filt[1].res = f(v),
            "fb_env" => p.filt[1].env = v == "1",
            "moda_index" => p.moda_index = f(v),
            "volume" => p.volume_db = f(v),
            "porta" => p.porta = f(v),
            "legato" => p.legato = v == "1",
            "fenv_amt" => p.fenv_amt = f(v),
            // v2 additions: Osc A tuning, Osc B, Mod A pitch/shift.
            "oct" => p.osc[0].oct = v.parse().unwrap(),
            "semi" => p.osc[0].semi = v.parse().unwrap(),
            "b_on" => p.osc[1].on = v == "1",
            "b_table" => p.osc[1].table = Table::from_index(v.parse().unwrap()),
            "b_index" => p.osc[1].index = f(v),
            "b_shift" => p.osc[1].shift = f(v),
            "b_oct" => p.osc[1].oct = v.parse().unwrap(),
            "b_cent" => p.osc[1].cent = f(v),
            "b_vol" => p.osc[1].vol = f(v),
            "moda_pitch" => p.moda_pitch = f(v),
            "moda_shift" => p.moda_shift = f(v),
            _ => {}
        }
    }
    p
}

fn on(e: &mut Engine, n: u8) {
    e.handle_event(&NoteEvent::NoteOn { timing: 0, voice_id: None, channel: 0, note: n, velocity: 1.0 });
}
fn off(e: &mut Engine, n: u8) {
    e.handle_event(&NoteEvent::NoteOff { timing: 0, voice_id: None, channel: 0, note: n, velocity: 0.0 });
}

fn main() {
    let jobs = std::fs::read_to_string(std::env::args().nth(1).expect("jobs file")).unwrap();
    for line in jobs.lines().filter(|l| !l.trim().is_empty()) {
        let mut it = line.split_whitespace();
        let out = it.next().unwrap();
        if out == "bench" {
            // bench <voices> <secs> [unison] [v1]: the v2 init patch (or the riddim_v1 fixture),
            // Osc A at `unison` voices (default 1) with width 0.5.
            let nv: usize = it.next().unwrap().parse().unwrap();
            let secs: f32 = it.next().unwrap().parse().unwrap();
            let uni: usize = it.next().map(|v| v.parse().unwrap()).unwrap_or(1);
            let mut p = if it.next() == Some("v1") { Patch::riddim_v1() } else { Patch::default() };
            p.osc[0].unison = uni;
            p.poly = nv.max(1);
            let mut e = Engine::new();
            e.initialize(SR);
            e.set_patch(&p);
            for i in 0..nv {
                on(&mut e, 36 + 3 * i as u8);
            }
            let n = (secs * SR) as usize;
            let t = std::time::Instant::now();
            let mut acc = 0f32;
            for _ in 0..n {
                acc += e.next_frame().0;
            }
            let el = t.elapsed().as_secs_f64();
            println!("bench voices={} unison={} active={} realtime_pct={:.4} ns_per_sample={:.1} ({})", nv, uni, e.active_voices(), 100.0 * el / secs as f64, el * 1e9 / n as f64, acc.is_finite());
            continue;
        }
        if out == "bench_shaper" {
            // bench_shaper <factor> <secs>: one voice's shaper (Saturate 0.7) on a C3 saw, SPEC v2 1.2.
            let factor: usize = it.next().unwrap().parse().unwrap();
            let secs: f32 = it.next().unwrap().parse().unwrap();
            let mut s = gts::dsp::shaper::Shaper::new();
            s.set_oversampling(factor);
            s.set(ShaperMode::Saturate, 0.7);
            let n = (secs * SR) as usize;
            let x: Vec<f32> = (0..n).map(|i| 0.43 * (2.0 * (i as f32 * 130.81 / SR).fract() - 1.0)).collect();
            let t = std::time::Instant::now();
            let acc: f32 = x.iter().map(|&v| s.process(v)).sum();
            let el = t.elapsed().as_secs_f64();
            println!("bench_shaper factor={} latency={} realtime_pct={:.4} ns_per_sample={:.1} ({})", s.oversampling(), s.latency(), 100.0 * el / secs as f64, el * 1e9 / n as f64, acc.is_finite());
            continue;
        }
        if out == "impulse" {
            // impulse <out> <filter type idx> <freq> <res> <secs>: raw MultiFilter impulse response.
            let o = it.next().unwrap();
            let ty = FilterType::from_index(it.next().unwrap().parse().unwrap());
            let fr: f32 = it.next().unwrap().parse().unwrap();
            let res: f32 = it.next().unwrap().parse().unwrap();
            let secs: f32 = it.next().unwrap().parse().unwrap();
            let mut f = gts::dsp::filter::MultiFilter::new();
            f.set_sample_rate(SR as f64);
            f.set(ty, fr, res);
            let buf: Vec<u8> = (0..(secs * SR) as usize)
                .flat_map(|i| f.process(if i == 0 { 1.0 } else { 0.0 }).to_le_bytes())
                .collect();
            std::fs::File::create(o).unwrap().write_all(&buf).unwrap();
            continue;
        }
        let secs: f32 = it.next().unwrap().parse().unwrap();
        let kv: Vec<(&str, &str)> = it.map(|t| t.split_once('=').unwrap()).collect();
        let get = |k: &str| kv.iter().find(|(a, _)| *a == k).map(|(_, v)| *v);
        let p = patch(&kv);
        let mut e = Engine::new();
        e.initialize(SR);
        e.set_patch(&p);
        let note: u8 = get("note").unwrap_or("36").parse().unwrap();
        let off_s: Option<f32> = get("off").map(|v| v.parse().unwrap());
        // note2=<note>@<secs>: second (overlapping, legato) note; off2 = its release.
        let note2: Option<(u8, f32)> = get("note2").map(|v| {
            let (n, t) = v.split_once('@').unwrap();
            (n.parse().unwrap(), t.parse().unwrap())
        });
        let n = (secs * SR) as usize;
        let mut buf = Vec::with_capacity(n);
        on(&mut e, note);
        for i in 0..n {
            let t = i as f32 / SR;
            if i % 512 == 0 && i > 0 {
                e.set_patch(&p);
            }
            if let Some(o) = off_s {
                if i == (o * SR) as usize {
                    off(&mut e, note);
                    if let Some((n2, _)) = note2 {
                        off(&mut e, n2);
                    }
                }
            }
            if let Some((n2, t2)) = note2 {
                if i == (t2 * SR) as usize {
                    on(&mut e, n2);
                }
            }
            let _ = t;
            buf.push(e.next_frame().0);
        }
        let mut f = std::fs::File::create(out).unwrap();
        let bytes: Vec<u8> = buf.iter().flat_map(|v| v.to_le_bytes()).collect();
        if out.ends_with(".wav") {
            // v2 addition: a `.wav` output path writes a mono 32-bit float WAV instead of raw f32.
            let mut h = Vec::with_capacity(44);
            h.extend_from_slice(b"RIFF");
            h.extend_from_slice(&(36 + bytes.len() as u32).to_le_bytes());
            h.extend_from_slice(b"WAVEfmt ");
            h.extend_from_slice(&16u32.to_le_bytes());
            h.extend_from_slice(&3u16.to_le_bytes()); // IEEE float
            h.extend_from_slice(&1u16.to_le_bytes());
            h.extend_from_slice(&(SR as u32).to_le_bytes());
            h.extend_from_slice(&(SR as u32 * 4).to_le_bytes());
            h.extend_from_slice(&4u16.to_le_bytes());
            h.extend_from_slice(&32u16.to_le_bytes());
            h.extend_from_slice(b"data");
            h.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            f.write_all(&h).unwrap();
        }
        f.write_all(&bytes).unwrap();
    }
}
