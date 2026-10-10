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
   (Live's custom VST3 folder, already configured for VSS). Re-copy after every rebuild.
2. Live > Preferences > Plug-Ins > Rescan.
3. Launch Live via `tools/live-debug.bat`.
4. New MIDI track, load GTS (Plug-Ins > VST3 > Riddim Rack Open Source > GTS).
5. Set project tempo to 150 BPM. Draw an 8-bar clip: 1/8-note riddim pattern on F1 / G1 / A#1
   with rests and one or two held notes.

## Checks (expected result)

6. Loads with no warning; GUI opens, fits the screen, every group visible (no clipped rows).
7. Default patch on F1: punchy square-type bass with a short downward "wobble" on each note
   (Mod A one-shot into Index) and a metallic comb tone. Not thin, not clipping (Live meter < 0 dB).
8. Transport stopped, play keys live: same sound, legato glide when overlapping notes.
9. Press Play: Mod B with Sync on and one-shot off locks to the grid; changing tempo to 140 follows.
10. Shift knob on Osc A: formant/brightness moves, pitch does NOT change (compare with a tuner).
11. Index and Motion: timbre morphs; Motion at +0.5 scans audibly.
12. Table combo: each of the 8 tables sounds distinct; Vowel table speaks A→U across Index.
13. Filter A type combo: LP12 / BP12 / Comb+ / Comb- / AM all audibly different, no clicks on switch.
14. Routing: Parallel / Series / Bypass change the sound; Shaper on with Saturate adds grit.
15. Pitch bend wheel (range 12) and mod wheel (set Wheel > Index to 1.0) respond.
16. Polyphony 4, play chords: 4 notes sound; 5th steals the oldest.
17. Save set, close, reopen: all knob values and combos restored.
18. Automate Filter A Freq and Osc A Shift in Live: no zipper noise or clicks.
19. CPU: Live's CPU meter with 8 held notes stays well under 50 % on this machine.
20. Resample 8 bars to an audio track, export WAV into `sessions/` named `YYYY-MM-DD-gts-150.wav`.

## After a session, send back

- `%LOCALAPPDATA%\RiddimRack\riddim-rack-live.log`
- `%AppData%\Ableton\Live 12.4.6\Preferences\Log.txt`
- the exported WAV
- which checklist item (if any) failed, and what it sounded like instead

## Reference

`demo/gts-demo.wav` is the default patch rendered offline by the engine itself (8 bars @150 BPM).
If Live sounds different from the demo at the default patch, the host integration is the suspect,
not the DSP.

## GUI / GPU warning

The GUI needs OpenGL 3.2. Opening the window over Remote Desktop, in a VM, or without a GPU
driver can crash Live. Audio without the window is unaffected; Live's generic parameter panel
still works. Click Live's window before pressing Space; set values by dragging.
