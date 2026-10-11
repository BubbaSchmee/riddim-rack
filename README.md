# Riddim Rack

Open-source (GPL-3.0-or-later) functional recreations of the devices riddim producers use, as Rust + nih-plug VST3/CLAP plugins for Windows, inspired by the classic Reason rack workflow.

We recreate behavior, never content: no third-party trademarks, no copied graintables, samples, graphics or preset names. All graintables are generated procedurally by our own code.

## Devices

| Plugin | Status |
|---|---|
| GTS, graintable synth | pre-alpha |

## GTS

Two-oscillator graintable synth. Each oscillator has a table, Index (picks the grain), Motion (scans the table), Shift, tuning and an ADSR. Then two LFOs, a shaper, two multimode filters (including comb and AM modes) with a shared filter envelope, and mono/legato/portamento. See `SPEC.md` for the full parameter list.

Tables (all generated procedurally): Square Morph, Saw Stack, Harmonic Rise, Vowel, FM Morph, Metal, Reed, Grit.

Notes:
- **Shift** moves the spectrum/formants while the note pitch stays fixed.
- **Mono + Legato**: a new note while one is held glides (Portamento) and does not retrigger envelopes or LFOs.
- The GUI needs **OpenGL 3.2**. It can crash or fail to open over Remote Desktop (RDP) or in a VM without GPU acceleration.
- HiDPI: the editor follows the host's scale where the host provides it.
- Tables are band-limited to 64 harmonics. It is a bass synth: above ~C5 and with Shift up you will hear aliasing. Mip-mapped tables are a planned upgrade.
- Negative **Shift** narrows grains into a pulse-like waveform by design.
- At 150 % Windows display scaling the 1000x760 window plus Live's chrome does not fit a 1080p screen; 100 % and 125 % do.
- The sustain pedal (CC64) is supported.
- Windows only (x64).

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
