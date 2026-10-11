# GTS manual Live session checklist (this machine)

0. `export CARGO_TARGET_DIR=/d/cargo-target/riddim-rack` before any cargo command (C: is full).
   `cargo xtask bundle gts --release` produces the FOLDER `D:\cargo-target\riddim-rack\bundled\GTS.vst3`.

Logs:
- Plugin log: `%LOCALAPPDATA%\RiddimRack\riddim-rack-live.log` (note-on lines appear only when launched via the bat)
- Live's own log: `%AppData%\Ableton\Live 12.4.6\Preferences\Log.txt`

Close Live completely before running `tools/live-debug.bat`, otherwise `NIH_LOG` never reaches
the plugin. Delete the plugin log first for a clean session.

## Setup

1. Copy the whole `GTS.vst3` folder into `C:\Users\kolby\Documents\Ableton\Custom Plugins`
   Live's custom VST3 folder (Preferences > Plug-Ins). Re-copy after every rebuild.
2. Live > Preferences > Plug-Ins > Rescan.
3. Launch Live via `tools/live-debug.bat`.
4. New MIDI track, load GTS (Plug-Ins > VST3 > Riddim Rack Open Source > GTS).
5. Set project tempo to 150 BPM. Draw an 8-bar clip: 1/8-note riddim pattern on F1 / G1 / A#1
   with rests and one or two held notes.

## Checks (expected result)

6. Loads with no warning; GUI opens, fits the screen, every group visible (no clipped rows).
7. Default (init) patch on C2 and F1: a clean, round square bass with a sine sub under it and a
   short filter-envelope "pluck" (LP12 opening on each note); no wobble, no comb, no grit. Live's
   meter peaks around -9 dBFS on a single C2 note. Turn Osc A Unison to 5 and Width to 1: wide and
   thick above the low end, while the low end stays centred (mono crossover 120 Hz).
   Until presets exist, load the riddim_v1 fixture by hand to hear the old wobble patch: Osc A
   Index 64; Mod A One-Shot on (Decay, Sync 1/8) with Mod A Index -0.50; Filter A Comb- 880 Hz
   Res 0.60 Kbd 0.50; Filter B LP12 2 kHz Res 0.10; Filter Env Amt 0.50; Sub off; Mono Crossover
   20 Hz; Volume 0 dB. Expected on F1: full square-type bass with a short downward "wobble" on
   each note (the attack starts on the wide pulse) and a metallic comb tone; matches
   `demo/gts-riddim-v1.wav`.
8. Transport stopped, play keys live: same sound, legato glide when overlapping notes.
9. Set Mod B Sync on, Div 1/4, Mod B Cutoff 0.5 (Mod B group). Press Play: the filter sweep locks to the beat; change tempo to 140 and it follows.
10. Shift knob on Osc A: formant/brightness moves, pitch does NOT change (compare with a tuner).
11. Index and Motion: timbre morphs; Motion at +0.5 scans audibly.
12. Table combo: each of the 8 tables sounds distinct; Vowel table speaks A→U across Index.
13. Filter A type combo: LP12 / BP12 / Comb+ / Comb- / AM all audibly different, no clicks on switch.
14. Routing: Parallel / Series / Bypass change the sound; Shaper on with Saturate adds grit.
15. Pitch bend wheel (range 12) and mod wheel (set Wheel > Index to 1.0) respond.
16. Polyphony 4, play chords: 4 notes sound; 5th steals the oldest.
17. Save set, close, reopen: all knob values and combos restored, and the header Quality (1x/2x/4x) setting.
18. Automate Filter A Freq and Osc A Shift in Live: no zipper noise or clicks.
19. CPU: Live's CPU meter with 8 held notes stays well under 50 % on this machine.
20. Hold the sustain pedal (or send CC64 127) and release keys: notes hold until the pedal is released.
21. RR-BUS patch (two instances): load a second GTS on another MIDI track. The PATCH labels read
    `GTS 1` / `GTS 2` (names commit on Enter or when the box loses focus; rename BEFORE patching,
    bindings store the label text). On GTS 2 set row 1 to `GTS 1 · Mod A` → `Osc A Index`, amount +1.0, and play
    both tracks: GTS 2's timbre wobbles with GTS 1's Mod A and the row LED flickers. Set GTS 1 row 1
    to `GTS 2 · Gate` → `Filter A Freq`, amount -0.8: GTS 1's filter ducks while GTS 2 holds a key.
    No host automation lanes move; setting a row's source to `None` stops its modulation. Now rename GTS 1 to `Lead bass`: GTS 2's row shows the old name dimmed with a dead LED (expected,
    rebind via the source list). Save the set, close, reopen: both patches, targets, amounts and the
    labels hold. (With VSS installed also: VSS `Step Gate` → GTS 1 `Filter A Freq` chops in time.)
22. FX chain (no FX page yet: use Live's device parameter list, "Configure" or the generic view; the
    parameters are grouped Distortion, Multiband, EQ, Chorus, Dimension, Delay, Reverb, Limiter).
    On the init patch, switch each module on alone and back off, holding an F1 note:
    - Dist On: Tube adds warmth at Drive 0, grit at +12 dB with no big level jump (Autogain);
      Type Hard Clip / Fold Sine / Bit Crush each sound distinct; Filter Mode Post + LP tames fizz.
    - MB On: OTT squash, quiet tails come up, the note sounds denser; Depth 0 returns to dry.
    - EQ On: Peak Gain +12 dB at 1 kHz is an obvious honk; Low Cut On at 200 Hz thins the bass.
    - Chorus On / Dimension On: wider above the low end, the sub stays centred (check a mono
      button or Live's Utility Width 0: no level drop).
    - Delay On: dotted-1/8 echoes at 150 BPM that follow a tempo change; Mode Ping-Pong alternates
      left / right.
    - Reverb On: tail after note-off; Type Hall is longer and smoother than Plate.
    Switching any module off returns exactly to the dry sound (no level change, no click beyond
    the module's own fade). Every module off: the sound is identical to before the FX existed.
23. Limiter: set Volume +6 dB and Limiter Gain +12 dB, play F1 and chords: Live's peak meter
    holds at -0.3 dBFS and never clips (no red), the sound gets denser but not crackly. Back to
    Limiter Gain 0 dB and the default Volume: the peak returns to about -9 dBFS on C2.
24. Candidate presets with FX (no preset browser in the plugin yet): render them with
    `cargo run -p gts --release --example render_preset` and play
    `docs/audits/renders/presets-fx/*.wav` in Live next to the clip: each sounds like its
    description including the FX sentence; Screech Lead's delay answers each phrase; Sub Weight
    stays dry and mono; no file clips or pumps.
25. Resample 8 bars to an audio track, export WAV into `sessions/` named `YYYY-MM-DD-gts-150.wav`.

## After a session, send back

- `%LOCALAPPDATA%\RiddimRack\riddim-rack-live.log`
- `%AppData%\Ableton\Live 12.4.6\Preferences\Log.txt`
- the exported WAV
- which checklist item (if any) failed, and what it sounded like instead

## Reference

`demo/gts-demo.wav` is the init patch rendered offline by the engine itself (8 bars @150 BPM);
`demo/gts-riddim-v1.wav` is the same line on the riddim_v1 fixture (item 7). If Live sounds
different from the demo at the init patch, the host integration is the suspect, not the DSP.

## GUI / GPU warning

The GUI needs OpenGL 3.2. Opening the window over Remote Desktop, in a VM, or without a GPU
driver can crash Live. Audio without the window is unaffected; Live's generic parameter panel
still works. Click Live's window before pressing Space; set values by dragging.

## pluginval loop (before any GTS commit that touches the editor, lib.rs or the engine)

`tools/pluginval-loop.sh [runs=6] [bundle]` runs pluginval strictness 5 in-process with GUI tests
N times and stops on the first non-SUCCESS run (log in `$TMPDIR/pv_loop_<n>.log`). Run it on the
release bundle (6 runs) and on the debug bundle (`cargo xtask bundle gts`, 2 runs). One hang is a
failure, never "flaky": take thread stacks of the hung pluginval.exe before killing it.

Environment check first if it hangs at "Starting tests in: pluginval / Editor...": the bundle is on
D: (Samsung T7 over USB) and the external monitor hangs off the same USB-C dock. On 2026-10-10 the
two wave-2 Editor timeouts (23:16-23:19) came minutes before that dock dropped every device
(System log: Kernel-PnP 1010 "surprise removed" for the T7, Focusrite and HP display, then Ntfs 50
"Delayed Write Failed" on D: at 23:20). The same binary then passed 15/15 runs plus 2 debug runs.
Check `Get-WinEvent` Kernel-PnP / Ntfs for the hang window before blaming the plugin.
