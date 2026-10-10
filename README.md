# Riddim Rack

Open-source (GPL-3.0-or-later) functional recreations of the Reason devices riddim producers use, as Rust + nih-plug VST3/CLAP plugins for Windows, inspired by the classic Reason rack workflow.

We recreate behavior, never content: no Reason/Propellerhead trademarks beyond the sentence above, no copied graintables, samples, graphics or preset names. All graintables are generated procedurally by our own code.

## Devices

| Plugin | Status |
|---|---|
| GTS, graintable synth | pre-alpha |

## Build and install

Requires Rust stable (MSVC). The VST3 bundle is a folder, not a single file.

```
cargo xtask bundle gts --release
```

Copy the folder `target/bundled/GTS.vst3` (or `$CARGO_TARGET_DIR/bundled/GTS.vst3`) into `C:\Program Files\Common Files\VST3\`, then rescan plugins in your host.

## License

GPL-3.0-or-later, see `LICENSE` and `THIRD-PARTY-NOTICES.md`.

VST is a trademark of Steinberg Media Technologies GmbH.
