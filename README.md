# Riddim Rack

Open-source (GPL-3.0-or-later) functional recreations of the devices riddim producers use, as Rust + nih-plug VST3/CLAP plugins for Windows, inspired by the classic Reason rack workflow.

We recreate behavior, never content: no third-party trademarks, no copied grain tables, samples, graphics or preset names. All grain tables are generated procedurally by our own code.

## Devices

| Plugin | Status |
|---|---|
| GTS, grain table synth | pre-alpha |

## GTS

Two-oscillator grain table synth. Each oscillator has a table, Index (picks the grain), Motion (scans the table), Shift, tuning and an ADSR. Then two LFOs, a shaper, two multimode filters (including comb and AM modes) with a shared filter envelope, unison (up to 7 voices), a sub oscillator with a mono crossover, and mono/legato/portamento. See `SPEC.md` for the full parameter list.

Tables (all generated procedurally): Square Morph, Saw Stack, Harmonic Rise, Vowel, FM Morph, Metal, Reed, Grit, Basic.

**FX chain.** The summed output runs through eight built-in effects in a fixed order: Distortion (eight types, Pre/Post filter, autogain), a three-band upward/downward compressor, a three-band EQ with a low cut, Chorus and Dimension (both keep the sub mono with a wet low cut), a tempo-syncable Delay (Mono, Stereo, Ping-Pong) and a Plate/Hall Reverb, then the master volume and an always-on lookahead limiter that holds every output sample at or below -0.3 dBFS. Each module is off until you switch it on; with all of them off the chain is a sample-exact pass-through apart from its constant latency (FX 95 samples at 48 kHz, 89 at 44.1 kHz, plus 0/21/23 for Quality 1x/2x/4x: 116 at 48 kHz by default; reported to the host, so Live's delay compensation covers it). FX settings are part of every preset. The editor has a SYNTH | FX page switch; the FX page holds all effect parameters. Presets are JSON files (`crates/gts/presets/candidates`).

**Editor.** The window is a fixed 1000 x 832 panel with a SYNTH | FX page switch, a header (preset bar, Quality, PATCH chip, scope/spectrum, level meter, Volume, Spread) and a help bar along the bottom.

- **Hero display.** Shows the table of the oscillator picked with its A / B tab as a stack of 32 frames in depth, with the frame that is playing now lit and the Mod A range marked. Drag it sideways to scrub Index (Shift for fine, double-click to reset); 2D / 3D flips between the stack and the single live frame. It is drawn on the GPU through OpenGL; if GL is refused it falls back to a simpler CPU drawing, and the plugin log says which one is in use (`GTS hero: GPU (...)` or `GTS hero: CPU fallback (...)`).
- **Rings.** A coloured ring on a knob is a modulation source moving it: the arc is the range, the dot is where it is now (amber Mod A, blue Mod B, rose filter envelope). They show only while a source is actually moving that knob.
- **Value popup and help bar.** A knob's label turns into its value while you hover or drag it, and a small popup shows the value while you drag (the Values toggle keeps every value printed). The help bar names the control under the pointer, its value and a one-line description. The Values and Reduced motion toggles sit at its right edge at all times. Reduced motion turns off sway, parallax and the animated transitions.
- **Source chips** (Mod A, Mod B, Env) are display-only: hovering one lights the knobs it drives. Dragging a chip onto a knob to patch it, and Macro chips, arrive with the modulation matrix; nothing on the panel patches by dragging yet.
- **Preset list.** The preset bar has prev / next arrows and a list (Init plus the factory presets); a dot beside the name marks edits. Loading a preset never touches the PATCH routing, the instance label or Quality.
- **PATCH chip.** Opens the RR-BUS popover (see below); its three small lights show the activity of the three input slots.
- The FX page has a live display for each module.

Notes:
- **Shift** moves the spectrum/formants while the note pitch stays fixed.
- **Mono + Legato**: a new note while one is held glides (Portamento) and does not retrigger envelopes or LFOs.
- The GUI needs **OpenGL 3.2**. It can crash or fail to open over Remote Desktop (RDP) or in a VM without GPU acceleration.
- HiDPI: the editor follows the host's scale where the host provides it.
- Tables are band-limited (512 harmonics, 37 quarter-octave mips, B-spline read); extreme Shift settings can still alias slightly.
- Negative **Shift** narrows grains into a pulse-like waveform by design.
- **Window size.** The panel is fixed at 1000 x 832 and there is no UI scale step yet. At 125 % Windows display scaling it needs about 1072 px of vertical space with the host's title bar, which a 1080p screen only has if you auto-hide the taskbar; otherwise use 100 % scaling (on the laptop, for now) until scale steps ship. At 150 % it does not fit a 1080p screen at all.
- If the plugin window goes black after a dock or display event (monitor unplugged, dock reconnected), close it and open it again.
- Arrow-key nudging of a focused knob is best-effort inside Ableton Live: Live can take the keys for its own shortcuts before the plugin sees them. Drag, wheel and double-click always work.
- The sustain pedal (CC64) is supported.
- Windows only (x64).

## Modulation bus (RR-BUS)

Instances of GTS and VSS in the same host process can patch to each other, no host automation lanes needed. GTS publishes Mod A, Mod B, Gate, Filter Env and Amp Env; each instance has three PATCH rows (source | target | amount | activity LED) to pick any live source from any instance and a target to modulate.

- The bus is control-rate: values update once per audio block, it is not audio-rate modulation.
- Instance labels (default `GTS 1`, `GTS 2`) are editable in the PATCH header and appear in source names (`Lead · Mod A`). Names commit on Enter or when the box loses focus; empty names are rejected.
- Bindings store the label text, so name your instances before patching. Renaming a source instance afterwards leaves other instances showing the old name dimmed until you re-pick it.
- If an instance disappears (closed or crashed), its signals are reclaimed after 2 s and rows bound to it show their name dimmed; they reconnect when a source with that name returns.

## Build and install

Requires Rust stable (MSVC).

```
git clone https://github.com/BubbaSchmee/riddim-rack
cd riddim-rack
cargo xtask bundle gts --release
```

The VST3 bundle is a folder, not a single file (`target/bundled/GTS.vst3`, or `$CARGO_TARGET_DIR/bundled/GTS.vst3`). Install it one of two ways, keeping the folder intact, then rescan plugins in your host:

1. Copy the whole `GTS.vst3` folder into `C:\Program Files\Common Files\VST3\`.
2. Or copy it into any custom plugin folder your host scans.

## License

GPL-3.0-or-later, see `LICENSE` and `THIRD-PARTY-NOTICES.md`.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.

VST is a trademark of Steinberg Media Technologies GmbH.
