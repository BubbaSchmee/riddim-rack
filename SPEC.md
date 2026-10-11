# Riddim Rack — functional recreations of the Reason devices riddim producers use

Open-source (GPL-3.0-or-later), Rust + nih-plug, VST3 (+CLAP), Windows first. One plugin per
device, built in this order: **GTS** (graintable synth, this spec), then SDU (damage unit,
Scream-style), then PBG (pattern band gate, Alligator-style). Each is a crate under `crates/`.

Legal boundary (non-negotiable): we recreate *behavior*, never content. Third-party trademarks
never appear in product names, code identifiers, UI strings, plugin metadata, release notes or
README marketing copy; the README carries exactly one "inspired by the classic Reason rack
workflow" sentence. Nominative references (naming the original device when describing the
behavior being recreated) are permitted ONLY in this SPEC and in `docs/` research notes. No
copied graintables, samples, graphics or preset names. All graintables are generated
procedurally by our own code.

This file is the single source of truth. Every agent reads it first. The orchestrator owns it.

## Shared stack (copied from the VSS project, already proven)
- Rust stable MSVC at `%USERPROFILE%\.cargo\bin`; **always** `export CARGO_TARGET_DIR=/d/cargo-target/riddim-rack`
  (C: is full). Bundle lands in `D:\cargo-target\riddim-rack\bundled\`.
- `nih_plug` (git, feature `assert_process_allocs`), `nih_plug_egui`, `nih_plug_xtask` via `cargo xtask bundle gts --release`.
- pluginval 1.0.4 at `%LOCALAPPDATA%\pluginval\pluginval.exe`, strictness 5, on release AND debug bundles.
- CI: windows-latest, test + bundle + pluginval (`--skip-gui-tests` in CI only) + artifact; tags `v*` → Release zip with LICENSE + THIRD-PARTY-NOTICES.md.
- Vendor "Riddim Rack Open Source", url/email `https://github.com/BubbaSchmee/riddim-rack` (/issues).
- Workspace layout:
```
Cargo.toml            [workspace] members = ["crates/gts", "xtask"], resolver 2, shared [profile.release]
bundler.toml          [gts] name = "GTS"
crates/gts/           package "gts", cdylib+lib, src/{lib.rs,params.rs,editor.rs,dsp/...}, tests/, examples/demo.rs
tools/live-debug.bat  NIH_LOG → %LOCALAPPDATA%\RiddimRack\riddim-rack-live.log, same Live-running guard as VSS
tools/CHECKLIST-gts.md
demo/gts-demo.wav
```

# GTS — Graintable Synth (v1)

A two-oscillator graintable synthesizer in the Malström lineage: each oscillator plays a table of
short single-cycle grains; **Index** picks the grain, **Motion** scans the table automatically,
**Shift** resamples each grain to move the spectrum/formant while the note pitch stays fixed.
Two LFO modulators with tempo sync and one-shot mode, a shaper, two multimode filters including
comb and AM modes, shared filter envelope, mono/legato/portamento. Instrument: MIDI in (`MidiConfig::MidiCCs`, required for
pitch bend and CC1 to be delivered), stereo out, no audio in.

## Graintables (procedural, generated at `initialize`, no allocation in `process`)
Representation: a table = `G` grains × `N = 2048` f32 samples, each grain exactly one cycle at a
reference pitch, band-limited additive synthesis with harmonics up to 64 (`// ponytail:` aliasing
ceiling above ~C5; upgrade path = per-octave mip tables). Index 0..127 maps linearly across the
grains with linear crossfade between neighbours. `G = 32` for all tables.

| # | Name | Index morphs… |
|---|---|---|
| 0 | Square Morph | pulse width 50% → 5% (the square-type riddim basis) |
| 1 | Saw Stack | 1 → 7 detuned saws (detune 0 → 24 cents) |
| 2 | Harmonic Rise | sine → 1/n harmonic series, harmonics added progressively |
| 3 | Vowel | A→E→I→O→U formant-filtered pulse (Peterson & Barney F1–F3, same table as VSS) |
| 4 | FM Morph | 2-op FM, ratio 2:1, index = mod index 0 → 8 |
| 5 | Metal | inharmonic partials (stretch factor 1.0 → 1.6) |
| 6 | Reed | pulse + resonant peak sweeping 400 → 4000 Hz |
| 7 | Grit | band-limited noise bursts, index = bandwidth narrow → wide (seeded, deterministic) |

Each grain DC-removed and normalized to RMS 0.25, then scaled down further only if its peak
exceeds 1.0 (revised 2026-10-10: peak normalization gave a 12.7 dB loudness spread between tables
and 12 dB across Square Morph's index range).

## Oscillator playback (per voice, per osc)
- Pitch: note + octave (-4..+4) + semi (0..12) + cent (-50..+50) + pitch bend + Mod A pitch.
- Grain scheduling: pitch-synchronous. A new grain starts every `1/f0` seconds; each grain is read
  at rate `f0 * 2^(shift/12) * N / sr` samples/sample with a Hann window; two alternating grain
  readers overlap 50 % so Shift ≠ 0 never clicks. At Shift = 0 this is plain single-cycle playback
  (test: output is periodic at f0 for every table/index). Shift range -24..+24 semitones.
- Index: param 0..127 + Motion scan + Mod A index + wheel index + velocity. Motion -1..+1: 0 = static;
  >0 scans forward (wraps), <0 scans forward-backward (ping-pong); magnitude = scan speed, 1.0 =
  full table per 0.25 s. Scan position resets on note-on.
- Per-osc ADSR amp envelope (A 1 ms..10 s log, D, S, R), per-osc Vol, On/Off.
- Per-osc one-pole DC blocker (8 Hz) after the grain oscillator: negative Shift turns grains into a
  unipolar pulse train (measured DC up to 98 % of RMS at shift -24 without it).

## Modulators (two LFOs, per voice, retrigger on note-on)
Curves: Sine, Triangle, Saw Up, Saw Down, Square, Sample&Hold, Smooth Random, Decay (one cycle
exponential fall), Attack (exponential rise). Rate 0.03..30 Hz log, or Sync on with divisions
{4/1, 2/1, 1/1, 1/2, 1/2T, 1/4, 1/4T, 1/8, 1/8T, 1/16, 1/16T, 1/32, 1/32T} locked to host beat
position when playing (free-running at host tempo when stopped). One-Shot: run a single cycle on
note-on and hold the final value (this is the riddim "wobble on each note"). Target A/B/Both.
- Mod A amounts (bipolar -1..+1): Pitch (±24 st), Index (±127), Shift (±24 st).
- Mod B amounts: Motion (±1), Vol (±1), Filter cutoff (±5 octaves), Mod A amount (±1 scales all Mod A amounts).

## Shaper (optional, before Filter A)
Modes: Sine (`sin(x·k)`), Saturate (`tanh`), Clip (hard), Quant (bit depth 16 → 2, MID-TREAD so
zero maps to zero: no DC pedestal, no click at note end), Noise (input-scaled white noise added). Amt 0..1 drives the amount/gain. On/Off.

## Filters A and B (identical; per voice)
Types: LP12 (SVF), BP12 (SVF), Comb+ (feedforward comb tuned to Freq, Res = mix), Comb- (feedback
comb tuned to Freq, Res = feedback 0..0.98), AM (ring-mod by internal sine at Freq, Res = dry/wet).
Freq 20 Hz..20 kHz log; Res 0..1; Kbd tracking 0..1 (1 = full key follow); Env On/Off.
Shared filter ADSR with Amt (0..1 = up to +8 octaves) and Inv. Mod B cutoff and wheel cutoff add in
the log domain. Clamp cutoff to [20 Hz, 0.45 sr].

## Routing (enum)
- Parallel: A → FA, B → FB, summed (default).
- Series: (A + B) → Shaper → FA → FB.
- Bypass: (A + B) → Shaper → out (filters off).
Shaper sits before FA in Parallel too (applies to A only, as in the original lineage).

## Voice management
Polyphony 1..8 (default 1). Legato on/off (default on): in mono, a new note while one is held
does not retrigger envelopes/LFOs. Portamento 0..2 s (default 0.08 s in mono). Pitch bend range
0..24 st (default 12). Voice stealing: oldest. Changing Polyphony releases only the voices
with index ≥ the new count (a held note on voice 0 keeps sounding). Sustain pedal (CC64 ≥ 0.5)
defers note-offs and flushes them on release; CC120/123 (all sound/notes off) release everything. Mod wheel (CC1) amounts: Index, Shift, Filter
cutoff, Mod amount. Velocity (bipolar amounts): Vol A, Vol B, Filter env, Attack, Shift, Mod amount.

## Output
Volume -24..+6 dB (default 0; raised with osc vol 1.0 on 2026-10-10 after RMS grain normalization left the default patch at -25 dBFS), Spread 0..1 (A left / B right stereo offset, 0 = mono). The voice
sum is scaled by 1/sqrt(poly) so chords do not live in the limiter (mono unchanged). Soft limiter
(tanh above 0.95) as safety only.

## Parameters (~80; ids stable; display names in quotes)
Osc A/B each: `osc{a,b}_on` "Osc A On", `_table` (enum above), `_oct` (-4..4), `_semi` (0..12),
`_cent` (-50..50), `_index` (0..127, default 64), `_motion` (-1..1, 0), `_shift` (-24..24, 0),
`_vol` (0..1, 1.0), `_att` (1 ms..10 s, 5 ms), `_dec` (1 ms..10 s, 300 ms), `_sus` (0..1, 1.0),
`_rel` (1 ms..10 s, 60 ms). Osc B default off, table Saw Stack.
Mod A/B each: `mod{a,b}_curve`, `_rate` (0.03..30 Hz, 4 Hz), `_sync` (bool, A: on), `_div`
(enum, 1/8), `_oneshot` (bool, A: on), `_target` (A/B/Both). Mod A: `moda_pitch`, `moda_index`
(default -0.5: the one-shot Decay then starts each note on the wide, loud pulse and narrows it,
giving a punchy attack; +0.5 started notes 8 dB quiet), `moda_shift`. Mod B: `modb_motion`, `modb_vol`, `modb_cutoff`, `modb_moda`.
Shaper: `shaper_on`, `shaper_mode`, `shaper_amt` (0.3).
Filter A/B each: `filt{a,b}_type` (A: Comb-, B: LP12), `_freq` (A: 880 Hz — was 220 Hz, which put the comb below the bass fundamental and cut it ~10 dB; B: 2 kHz), `_res`
(A: 0.6), `_kbd` (0.5), `_env` (bool, on).
Filter env: `fenv_att/dec/sus/rel` (5 ms, 250 ms, 0.2, 100 ms), `fenv_amt` (0.5), `fenv_inv`.
`routing`, `poly` (1), `legato` (on), `porta` (0.08), `bend_range` (12).
Wheel: `wheel_index`, `wheel_shift`, `wheel_cutoff`, `wheel_mod`. Velocity: `vel_vol_a`,
`vel_vol_b`, `vel_fenv`, `vel_att`, `vel_shift`, `vel_mod`. Output: `volume`, `spread`.
Defaults not listed above (fixed by the scaffold): `moda_curve` Decay, `modb_curve` Sine,
`modb_sync` off, `modb_oneshot` off, both `_target` Both, both `_div` 1/8, `shaper_on` off,
`shaper_mode` Saturate, `filtb_res` 0.1, all `wheel_*`, `vel_*` and `modb_*` amounts 0 (all
bipolar -1..1), `spread` 0. Envelope times and portamento are stored in seconds.
Defaults are chosen so an init patch on a low note already sounds like a riddim bass:
Square Morph, mono legato, Mod A one-shot 1/8 sync into Index, Comb- filter A at 880 Hz. Kbd tracking pivots at C4 (note 60); wheel cutoff range is ±5 octaves like Mod B.

## GUI (nih_plug_egui, functional, dark)
Fixed window 1000×760 logical px (fits 1080p at 100 % and 125 % OS scaling; at 150 % Live's
chrome pushes it off screen, documented in README). The functional egui layout is v1; the
designed look is specified in `docs/design/DESIGN.md` (Variant B mint) and replaces it. Sections as labelled groups: Osc A | Osc B (top row),
Mod A | Mod B | Shaper (second row), Filter A | Filter B | Filter Env (third row), Voice | Wheel |
Velocity | Output (bottom row). ParamSlider at width 80 with `with_width`, ComboBox for enums
at width 110, bool as checkbox. Compute every row's width in a comment and keep rows under
960 px (the VSS GUI overflowed; do not repeat that). Header "GTS — Graintable Synth" + version.
A small live readout of the current grain index of Osc A (shared AtomicU32) is nice-to-have.

## Logging (nih_log!, gated: note-on lines only when NIH_LOG is set, checked once in initialize)
initialize: sr, max buffer, table generation time. Transport play/stop, tempo change. Note-on:
note, velocity, voice count. Panics land in the same file via nih-plug's hook.

## Verification (all must pass before handoff)
1. `cargo test`, std-only tests, in `crates/gts/tests/`:
   - pitch: for every table at index 0/64/127 and shift ∈ {-12, 0, +12}, a 2 s render of A1
     (55 Hz, 48 kHz, filters bypassed) has its fundamental at 55 Hz ± 1 % (autocorrelation or
     Goertzel comb over harmonics), i.e. Shift must not change pitch.
   - shift: spectral centroid at shift +12 > shift 0 > shift -12 for Square Morph index 64.
   - index: spectra at index 0 vs 127 differ (normalized spectral distance > 0.2) for every table.
   - tables: every grain RMS within 1 % of 0.25 unless peak-capped at 1.0, peak ≤ 1.0, |DC| < 1e-3,
     no NaN; generation under 50 ms total (release).
   - dc: for every table at shift -12 and -24, |mean| of the oscillator output < 2 % of its RMS.
   - sustain: CC64 held defers note-off; releasing the pedal releases the voice; CC123 silences all.
   - poly: lowering Polyphony while a note sounds on voice 0 keeps it sounding.
   - envelope: amp ADSR with A=100 ms reaches 0.99 within 100 ms ± 5 %; release to < 1e-3 within R.
   - lfo: sync 1/8 at 150 BPM gives period 0.2 s ± 1 sample; one-shot holds final value after 1 cycle.
   - filters: LP12 at 1 kHz is -3 dB ± 1 dB at 1 kHz and < -20 dB at 4 kHz; Comb- at 220 Hz has
     peaks at 220·k within 2 %; AM of a 1 kHz sine at Freq 100 Hz yields 900 and 1100 Hz components.
   - safety: 1 s of 8 voices at extreme params (shift ±24, res 1, shaper quant 2 bit, cutoff
     20 kHz) produces no NaN/inf and peak ≤ 1.0 after the limiter.
   - demo: `examples/demo.rs` renders `demo/gts-demo.wav` (16-bit 48 kHz, std WAV writer): 8 bars
     at 150 BPM, a riddim-style mono bass line on the default patch (notes F1/G1/A#1 in a 1/8
     call-and-response pattern with one-shot wobbles), RMS between -30 and -8 dBFS, peak < 0.99.
2. `cargo xtask bundle gts --release` and debug bundle both pass pluginval strictness 5
   (`--validate-in-process`; locally WITH GUI tests). The debug bundle proves no allocation in
   `process` (assert_process_allocs); voice/grain state must be preallocated for 8 voices.
3. MIDI handling verified in a test using nih-plug's `NoteEvent` path: note-on/off, pitch bend,
   CC1, legato retrigger rules, voice stealing at 9 notes.

## Non-goals for v1
Preset browser, factory preset bank, graintable import, oscillator direct outs, CV, custom
graphics, macOS/Linux, audio input, mip-mapped tables, per-osc filter routing beyond the three
routings above.
