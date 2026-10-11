//! SPEC Verification 3: MIDI through nih-plug's `NoteEvent` path (the same `Engine::handle_event`
//! the plugin's `process` calls): note-on/off, legato rules, portamento, stealing, bend, CC1.
use gts::dsp::env::Stage;
use gts::dsp::{Engine, Patch};
use nih_plug::prelude::NoteEvent;

const SR: f32 = 48000.0;

fn engine(patch: &Patch) -> Engine {
    let mut e = Engine::new();
    e.initialize(SR);
    e.set_patch(patch);
    e
}

fn on(note: u8) -> NoteEvent<()> {
    NoteEvent::NoteOn { timing: 0, voice_id: None, channel: 0, note, velocity: 1.0 }
}

fn off(note: u8) -> NoteEvent<()> {
    NoteEvent::NoteOff { timing: 0, voice_id: None, channel: 0, note, velocity: 0.0 }
}

fn run(e: &mut Engine, secs: f32) -> Vec<f32> {
    (0..(secs * SR) as usize).map(|_| e.next_frame().0).collect()
}

#[test]
fn note_on_and_off() {
    let mut e = engine(&Patch::default());
    e.handle_event(&on(40));
    let x = run(&mut e, 0.1);
    assert_eq!(e.active_voices(), 1);
    assert!(x.iter().any(|s| s.abs() > 0.1));
    e.handle_event(&off(40));
    assert_eq!(e.voice(0).amp_env(0).stage(), Stage::Release);
    run(&mut e, 0.1); // release 60 ms
    assert_eq!(e.active_voices(), 0);
}

#[test]
fn mono_legato_glides_without_retrigger() {
    let mut e = engine(&Patch::default()); // poly 1, legato on, porta 80 ms
    e.handle_event(&on(40));
    run(&mut e, 0.2);
    assert_eq!(e.voice(0).amp_env(0).stage(), Stage::Sustain);
    e.handle_event(&on(47));
    let level = e.voice(0).amp_env(0).level();
    run(&mut e, 0.01);
    assert_eq!(e.voice(0).amp_env(0).stage(), Stage::Sustain, "legato note retriggered");
    assert!((e.voice(0).amp_env(0).level() - level).abs() < 1e-3);
    let p = e.voice(0).pitch();
    assert!(p > 40.0 && p < 47.0, "should still be gliding, at {p}");
    run(&mut e, 0.1);
    assert!((e.voice(0).pitch() - 47.0).abs() < 0.2, "glide should land within porta");

    // Releasing the top key falls back to the held one, still without retrigger.
    e.handle_event(&off(47));
    run(&mut e, 0.2);
    assert_eq!(e.voice(0).note, 40);
    assert_eq!(e.voice(0).amp_env(0).stage(), Stage::Sustain);
    assert!((e.voice(0).pitch() - 40.0).abs() < 0.05);

    // A detached note retriggers and does not glide (fingered portamento).
    e.handle_event(&off(40));
    run(&mut e, 0.02);
    e.handle_event(&on(52));
    assert_eq!(e.voice(0).amp_env(0).stage(), Stage::Attack);
    assert_eq!(e.voice(0).pitch(), 52.0);
}

#[test]
fn mono_without_legato_retriggers() {
    let mut p = Patch::default();
    p.legato = false;
    let mut e = engine(&p);
    e.handle_event(&on(40));
    run(&mut e, 0.2);
    e.handle_event(&on(43));
    assert_eq!(e.voice(0).amp_env(0).stage(), Stage::Attack);
}

#[test]
fn ninth_note_steals_the_oldest() {
    let mut p = Patch::default();
    p.poly = 8;
    let mut e = engine(&p);
    for n in 0..8 {
        e.handle_event(&on(40 + n));
        run(&mut e, 0.01);
    }
    assert_eq!(e.active_voices(), 8);
    e.handle_event(&on(60));
    run(&mut e, 0.01);
    let notes: Vec<u8> = (0..8).map(|i| e.voice(i).note).collect();
    assert!(!notes.contains(&40), "oldest note 40 should be stolen: {notes:?}");
    assert!(notes.contains(&60));
    for n in 41..48 {
        assert!(notes.contains(&n), "note {n} lost: {notes:?}");
    }
    assert_eq!(e.active_voices(), 8);
}

#[test]
fn pitch_bend_follows_range() {
    let mut e = engine(&Patch::default()); // bend range 12
    e.handle_event(&on(33)); // 55 Hz
    run(&mut e, 0.05);
    assert!((e.voice(0).freq(0) - 55.0).abs() < 0.01);
    e.handle_event(&NoteEvent::MidiPitchBend { timing: 0, channel: 0, value: 1.0 });
    run(&mut e, 0.1);
    assert!((e.voice(0).freq(0) - 110.0).abs() < 0.1, "bend up: {}", e.voice(0).freq(0));
    e.handle_event(&NoteEvent::MidiPitchBend { timing: 0, channel: 0, value: 0.25 });
    run(&mut e, 0.1);
    let want = 55.0 * 2f32.powf(-6.0 / 12.0);
    assert!((e.voice(0).freq(0) - want).abs() < 0.1, "half bend down: {}", e.voice(0).freq(0));
}

#[test]
fn mod_wheel_drives_index() {
    let mut p = Patch::default();
    p.wheel_index = 0.5;
    p.moda_index = 0.0; // isolate the wheel from the default Mod A one-shot into Index
    let mut e = engine(&p);
    e.handle_event(&on(33));
    run(&mut e, 0.1);
    let before = e.voice(0).index(0);
    assert!((before - 64.0).abs() < 2.5, "index {before}");
    e.handle_event(&NoteEvent::MidiCC { timing: 0, channel: 0, cc: 1, value: 1.0 });
    run(&mut e, 0.1);
    assert!((e.wheel() - 1.0).abs() < 1e-3);
    let after = e.voice(0).index(0);
    assert!(after > 120.0, "wheel should push index up by 63.5, got {after}");
    // Other CCs are ignored.
    e.handle_event(&NoteEvent::MidiCC { timing: 0, channel: 0, cc: 7, value: 0.0 });
    run(&mut e, 0.01);
    assert!((e.wheel() - 1.0).abs() < 1e-3);
}

#[test]
fn output_is_bounded_with_eight_loud_voices() {
    let mut p = Patch::default();
    p.poly = 8;
    p.volume_db = 6.0;
    p.osc[1].on = true;
    p.osc[0].shift = 24.0;
    p.osc[1].shift = -24.0;
    p.spread = 1.0;
    let mut e = engine(&p);
    for n in 0..8 {
        e.handle_event(&on(30 + 5 * n));
    }
    let mut peak = 0f32;
    for _ in 0..SR as usize {
        let (l, r) = e.next_frame();
        assert!(l.is_finite() && r.is_finite());
        peak = peak.max(l.abs()).max(r.abs());
    }
    // 1/sqrt(poly) headroom (SPEC Output) keeps a chord mostly out of the limiter; still loud.
    assert!(peak <= 1.0 && peak > 0.5, "peak {peak}");
}

#[test]
fn poly_stolen_voice_starts_on_pitch() {
    // Defaults: legato on, porta 80 ms. Stealing a held voice must not glide from its old note.
    let mut p = Patch::default();
    p.poly = 2;
    let mut e = engine(&p);
    e.handle_event(&on(40));
    run(&mut e, 0.01);
    e.handle_event(&on(52));
    run(&mut e, 0.01);
    e.handle_event(&on(64));
    run(&mut e, 0.001);
    let v = (0..2).map(|i| e.voice(i)).find(|v| v.note == 64).unwrap();
    assert!((v.pitch() - 64.0).abs() < 1e-3, "stolen voice glides: {}", v.pitch());
}

fn cc(cc: u8, value: f32) -> NoteEvent<()> {
    NoteEvent::MidiCC { timing: 0, channel: 0, cc, value }
}

#[test]
fn sustain_pedal_defers_note_off_and_cc123_silences() {
    for poly in [1, 4] {
        let mut p = Patch::default();
        p.poly = poly;
        let mut e = engine(&p);
        e.handle_event(&cc(64, 1.0));
        e.handle_event(&on(40));
        run(&mut e, 0.05);
        e.handle_event(&off(40));
        run(&mut e, 0.2);
        assert_eq!(e.active_voices(), 1, "poly {poly}: pedal should hold the note");
        assert_ne!(e.voice(0).amp_env(0).stage(), Stage::Release);
        e.handle_event(&cc(64, 0.0));
        assert_eq!(e.voice(0).amp_env(0).stage(), Stage::Release, "poly {poly}: pedal up releases");
        run(&mut e, 0.1);
        assert_eq!(e.active_voices(), 0);

        // CC123 releases everything, pedal or not.
        e.handle_event(&cc(64, 1.0));
        for n in [40, 44, 47] {
            e.handle_event(&on(n));
        }
        run(&mut e, 0.05);
        e.handle_event(&cc(123, 0.0));
        run(&mut e, 0.1);
        assert_eq!(e.active_voices(), 0, "poly {poly}: CC123 should silence all");
    }
}

#[test]
fn lowering_polyphony_keeps_voice_zero() {
    let mut p = Patch::default();
    p.poly = 4;
    let mut e = engine(&p);
    for n in [40, 44, 47] {
        e.handle_event(&on(n));
    }
    run(&mut e, 0.05);
    assert_eq!(e.voice(0).note, 40);
    p.poly = 1;
    e.set_patch(&p);
    run(&mut e, 0.2);
    assert_eq!(e.active_voices(), 1, "only voices >= new poly are released");
    assert!(e.voice(0).gate && e.voice(0).note == 40);
    // The kept note still releases on its own note-off in mono.
    e.handle_event(&off(40));
    run(&mut e, 0.1);
    assert_eq!(e.active_voices(), 0);
}
