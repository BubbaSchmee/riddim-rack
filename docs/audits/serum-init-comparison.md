# GTS vs a commercial init patch: basic-shapes reference comparison

Date 2026-10-11, repo `main` at 7dbc833 plus the uncommitted `tools/compare.py` and
`crates/gts/examples/compare_render.rs`. Measured only. Nothing from the reference is copied into
the repo: the WAV stays in `docs/audits/references/` (gitignored) and only measurements are written
down here.

**Reference.** `docs/audits/references/serum-init-basic-shapes-c3.wav`: 44.1 kHz float, L = R
(max |L−R| = 0), 36.5 s. A commercial wavetable synth's init patch, one held note, 20 bars at
140 BPM, wavetable position switched every 4 bars (5 positions of its default basic-shapes table).

**Correction to the brief: the note is MIDI 60, 261.63 Hz, not 130.81 Hz.** Ableton calls MIDI 60
"C3". The spectrum has its lowest component at 261.59 Hz (2^18-point FFT, segment 2) and nothing at
130.8 Hz. With `--f0 130.81`, the autocorrelation locks to the subharmonic and every odd "harmonic"
reads −100 dBc (the comparator now prints a warning when this happens). All GTS renders below play
MIDI 60 so the pitch matches. `compare_render -- 48` renders the literal 130.81 Hz if it is wanted.

## How to reproduce

```
export PATH="$USERPROFILE/.cargo/bin:$PATH" CARGO_TARGET_DIR=/d/cargo-target/riddim-rack CARGO_INCREMENTAL=0
cargo run -p gts --release --example compare_render                 # MIDI 60 -> docs/audits/renders/compare/
cargo run -p gts --release --example compare_render -- 60 xover20   # diagnostic: crossover at its minimum
py -3.12 tools/compare.py --selfcheck
py -3.12 tools/compare.py docs/audits/references/serum-init-basic-shapes-c3.wav \
  docs/audits/renders/compare/gts-sequence.wav --f0 261.63 \
  --test-segments 0,6.857143,13.714286,20.571429,27.428571,34.285714 \
  --match-level --plots docs/audits/renders/compare/plots --json docs/audits/renders/compare/seq.json
```

The REF segments come from `--segments auto`. Auto found exactly 5, at 0.000 / 6.858 / 13.716 /
20.572 / 27.429 / 34.300 s. Nominal bar lines are at k × 6.857 s; the refined cuts land within
1.6 ms of them. The 4 → 5 cut shows the largest offset.

**GTS renders.** `compare_render.rs` uses the real `Engine` at 44.1 kHz with 512-sample blocks. It
starts from `Patch::default()` and makes it RAW: routing Bypass, shaper off, sub off, unison 1,
attack 1 ms, sustain 1, release 60 ms, Osc B off, velocity 100, oversampling at the engine default
(2×). Everything else stays at init, which includes the **120 Hz mono crossover** (see gap 3).
`gts-sequence.wav` holds one note for 20 bars, which is the same gate as the reference, and
switches (table, index) at each 4-bar line:

| ref seg | ref class (measured below) | GTS position |
|---|---|---|
| 1 | saw | Harmonic Rise index 87 |
| 2 | sine | Harmonic Rise index 0 |
| 3 | triangle | Square Morph index 0. **No triangle exists in GTS.** A search over all 8 tables × 32 grains for the closest match to odd 1/k² finds Square Morph grain 0 at 11.75 dB mean distance, then FM Morph grains 18–21 at 12.8–13.2 dB. |
| 4 | odd-harmonic blend | Square Morph index 0 |
| 5 | square | Square Morph index 0 (50 % pulse) |

Why index 87 for the saw: at C4 and 44.1 kHz the oscillator reads the mip that keeps 64 harmonics.
Harmonic Rise grain 21 is the first grain whose harmonic count (512^(21/31) = 68) covers all 64, so
index 87 renders a full 1/n saw. It is identical to Saw Stack index 0: same harmonics to 0.01 dB,
same three distances.

Single-note 4-bar files are also written: `gts-HarmonicRise-0`, `gts-HarmonicRise-87`,
`gts-SawStack-0`, `gts-SquareMorph-0`. Their steady-state figures match the corresponding sequence
segments. `renders/compare/cs4/` holds the C#4 (MIDI 61) renders used for gap 1.

**Level matching.** `--match-level` scales each GTS segment to the REF segment's steady RMS before
measuring. The gains, also printed by the tool, are **−2.15, −0.36, −2.10, −2.11 and +2.65 dB**
for segments 1–5. Before any gain, GTS measures RMS −16.65 to −16.68 dBFS at every position.

**Measurement floor.** The selfcheck synthesises an ideal float64 band-limited saw over a 3 s
window and reads it at −89 dB non-harmonic. The 5.9 s windows used here read −105 to −106 dB on
both files, so both noise floors sit at the method's floor.

## The reference, segment by segment

The amplitudes and phases below come from a least-squares fit of 84 harmonics at exactly
261.6256 Hz over 0.2 s of steady signal. The fit residual is ≤ 1.1e-3 absolute. The `rel` phase of
harmonic k is its phase minus k × the phase of h1.

| seg | class | proof |
|---|---|---|
| 1 | **textbook saw** | All harmonics present. a_k·k / a_1 = 1.0000, 0.9999, …, 0.9979 for k = 1–9, i.e. 1/k to within 0.02 dB. Deviation from −20·log10(k) is ≤ 0.05 dB up to h84. Tilt −6.03 dB/oct, odd and even fits both −6.03. rel phase steps by −π/2 per harmonic, which is a sine series. |
| 2 | **pure sine** | h2–h84 are below −100 dBc. The largest is h2, 1.5e-8 absolute (−142 dB re h1). |
| 3 | **textbook triangle** | Even harmonics below −100 dBc. Odd a_k·k² / a_1 = 0.9998, 0.9993, 0.9987, 0.9979 for k = 3, 5, 7, 9. Tilt −12.05 dB/oct. rel phase 0 on every odd harmonic. |
| 4 | **odd-only blend, not textbook** | Even harmonics below −100 dBc. Odd a_k·k / a_1 = 0.862, 0.850, 0.846, 0.844 for k = 3, 5, 7, 9: a 1/k series sitting 1.3–1.5 dB under h1. Tilt −6.21 dB/oct. rel phases are +1.65, −2.71, −0.73, +1.25 (not 0/π). It is **not** a linear mix of the other four frames: a real-weight least-squares fit leaves 44 % of the energy. |
| 5 | **textbook square** | Even harmonics below −100 dBc. Odd a_k·k / a_1 = 0.9998, 0.9993, 0.9987, 0.9979. Tilt −6.03 dB/oct. rel phase 0, π, 0, π: a cosine series. |

**Frame amplitude.** Sine, triangle, square and saw all have an underlying waveform amplitude of
0.1995 (−14.0 dBFS). The fitted h1 amplitudes are 0.1995 (sine), 0.1617 = 8/π²·A (triangle),
0.2540 = 4/π·A (square) and 0.1270 = 2/π·A (saw). The frames are therefore **peak-normalised with
no loudness compensation**. Segment LUFS spans 5.2 LU: −11.39 (square), −14.82 (sine), −15.95
(saw), −16.25 (blend), −16.57 (triangle).

**Bandwidth.** Every harmonic up to the last one below Nyquist is present: h84 = 21.98 kHz on the
saw, h83 on the square and blend. There is no roll-off before Nyquist.

**Rolloff.** Exact 1/n (saw, square) and 1/n² (triangle) to within 0.05 dB up to h84, with
textbook phases. The basic shapes are **textbook, not shaped**. Segment 4 is the one exception.

**Noise, DC, stereo and pitch.** Non-harmonic floor −105.5 to −106.4 dB, at the measurement floor.
DC is −1.0e-4 on the saw (−80 dBFS) and |DC| ≤ 2.3e-5 elsewhere. L/R correlation is 1.0000. f0 is
261.626 Hz in all five segments. No drift was measured: the h1 phase carries across the switches
to the fit's precision.

**Amp envelope.**

- **Note start (t = 0):** the first three samples are exactly 0, then 0.0005, 0.0029, 0.0068 …. The
  one-period sliding RMS is −2.2 dB at onset, −0.9 dB at +0.5 ms and 0.0 dB from +1 ms. It reaches
  90 % of steady amplitude 2.4 ms after onset on the comparator's metric, whose resolution is about
  half a period (1.9 ms).
- **Position switches (6.857, 13.714, 20.571, 27.429 s):** there is **no retrigger**. The level
  never dips; per-period RMS across the cuts runs straight from the old frame's level to the new
  one. The fundamental's phase carries across the cuts (saw → sine: h1 phase +0.449 → +0.423 rad),
  so the oscillator is free-running and its phase is not reset. The waveform leaves the old frame
  within 0.1–0.2 ms after the bar line and matches the new frame (to 1e-3 absolute, about −46 dB
  re its peak) 13.2–14.0 ms later, or 29 ms for the 4 → 5 cut. The in-between is **not a two-frame
  linear crossfade**: a per-0.5 ms least-squares mix factor swings between −1.8 and +1.5. This fits
  a smoothed position glide that passes through intermediate frames. **No clicks:** the largest sample-to-sample step within ±45 ms of a cut is 1.00× the
  larger of the two frames' steady maximum step at three cuts. At the sine → triangle cut it is
  1.38× (0.0102 vs 0.0074), where both frames have small steps.
- **Note end (34.286 s):** −0.8 dB at +0 ms, −3.5 at +2, −10.1 at +5, −22.5 at +10, −37.9 at +14,
  −56.9 at +16 and −119.7 at +18 ms. It reaches −60 dB at **14.4 ms**, and the last sample above
  1e-6 is at +16 ms. Level in dB falls faster as the release goes on (concave), so it is not an
  exponential release, which would be a straight line in dB.

## Pair tables (GTS level-matched; raw RMS/peak shown before gain)

The peaks in these tables cover the whole segment, so they include the switch region (GTS segment
2's −4.95 dBFS is the outgoing saw during the one-period Index crossfade). Steady-state crest is in
its own table further down.

### Segment 1: REF saw vs GTS Harmonic Rise 87

| h | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| REF dBc | 0.0 | -6.0 | -9.5 | -12.0 | -14.0 | -15.6 | -16.9 | -18.1 | -19.1 | -20.0 | -20.9 | -21.6 | -22.3 | -23.0 | -23.6 | -24.1 |
| GTS dBc | 0.0 | -6.0 | -9.5 | -12.0 | -14.0 | -15.6 | -16.9 | -18.1 | -19.1 | -20.0 | -20.8 | -21.6 | -22.3 | -22.9 | -23.5 | -24.1 |

| metric | REF | GTS |
|---|---|---|
| RMS / peak dBFS (GTS raw, before gain) | -18.81 / -10.95 | -16.65 / -4.69 |
| gain applied to GTS | | -2.15 dB |
| LUFS (after gain) | -15.95 | -15.96 |
| tilt dB/oct (200 Hz-10 kHz) | -6.0 | -6.0 |
| -60 dBc bandwidth | h84, 21977 Hz | h64, 16744 Hz |
| non-harmonic floor | -105.5 dB | -105.1 dB |
| DC | -1.0e-04 | -3.2e-06 |
| attack to 90 % (ms) | 2.4 | 3.3 |
| release to -60 dB (ms) | - (held) | - (held) |
| L/R correlation | 1.0000 | 1.0000 |
| **distance** h1-40 / log-mel / LUFS env | | 0.03 dB / 15.89 dB / 0.01 LU |

### Segment 2: REF sine vs GTS Harmonic Rise 0

| h | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| REF dBc | 0.0 | -100.0 | -100.0 | -100.0 | -100.0 | -100.0 | -100.0 | -100.0 | -100.0 | -100.0 | -100.0 | -100.0 | -100.0 | -100.0 | -100.0 | -100.0 |
| GTS dBc | 0.0 | -100.0 | -100.0 | -100.0 | -100.0 | -100.0 | -100.0 | -100.0 | -100.0 | -100.0 | -100.0 | -100.0 | -100.0 | -100.0 | -100.0 | -100.0 |

| metric | REF | GTS |
|---|---|---|
| RMS / peak dBFS (GTS raw, before gain) | -17.01 / -11.14 | -16.65 / -4.95 |
| gain applied to GTS | | -0.36 dB |
| LUFS (after gain) | -14.82 | -14.82 |
| tilt dB/oct (200 Hz-10 kHz) | n/a | n/a |
| -60 dBc bandwidth | h1, 262 Hz | h1, 262 Hz |
| non-harmonic floor | -106.3 dB | -106.4 dB |
| DC | -2.3e-05 | 7.7e-06 |
| attack to 90 % (ms) | - (switch) | - (switch) |
| release to -60 dB (ms) | - (held) | - (held) |
| L/R correlation | 1.0000 | 1.0000 |
| **distance** h1-40 / log-mel / LUFS env | | 0.00 dB / 0.73 dB / 0.01 LU |

### Segment 3: REF triangle vs GTS Square Morph 0 (no triangle exists)

| h | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| REF dBc | 0.0 | -100.0 | -19.1 | -100.0 | -28.0 | -100.0 | -33.8 | -100.0 | -38.2 | -100.0 | -41.7 | -100.0 | -44.6 | -100.0 | -47.1 | -100.0 |
| GTS dBc | 0.0 | -100.0 | -9.5 | -100.0 | -14.0 | -100.0 | -16.9 | -100.0 | -19.1 | -100.0 | -20.8 | -100.0 | -22.3 | -100.0 | -23.5 | -100.0 |

| metric | REF | GTS |
|---|---|---|
| RMS / peak dBFS (GTS raw, before gain) | -18.77 / -11.09 | -16.68 / -6.51 |
| gain applied to GTS | | -2.10 dB |
| LUFS (after gain) | -16.57 | -16.14 |
| tilt dB/oct (200 Hz-10 kHz) | -12.0 | -6.0 |
| -60 dBc bandwidth | h31, 8110 Hz | h63, 16482 Hz |
| non-harmonic floor | -106.3 dB | -105.8 dB |
| DC | 1.7e-06 | -6.5e-06 |
| attack to 90 % (ms) | - (switch) | - (switch) |
| release to -60 dB (ms) | - (held) | - (held) |
| L/R correlation | 1.0000 | 1.0000 |
| **distance** h1-40 / log-mel / LUFS env | | 11.77 dB / 22.30 dB / 0.43 LU |

### Segment 4: REF odd blend vs GTS Square Morph 0

| h | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| REF dBc | 0.0 | -100.0 | -10.8 | -100.0 | -15.4 | -100.0 | -18.4 | -100.0 | -20.6 | -100.0 | -22.3 | -100.0 | -23.8 | -100.0 | -25.0 | -100.0 |
| GTS dBc | 0.0 | -100.0 | -9.5 | -100.0 | -14.0 | -100.0 | -16.9 | -100.0 | -19.1 | -100.0 | -20.8 | -100.0 | -22.3 | -100.0 | -23.5 | -100.0 |

| metric | REF | GTS |
|---|---|---|
| RMS / peak dBFS (GTS raw, before gain) | -18.78 / -12.42 | -16.68 / -9.17 |
| gain applied to GTS | | -2.11 dB |
| LUFS (after gain) | -16.25 | -16.15 |
| tilt dB/oct (200 Hz-10 kHz) | -6.2 | -6.0 |
| -60 dBc bandwidth | h83, 21715 Hz | h63, 16482 Hz |
| non-harmonic floor | -106.2 dB | -105.8 dB |
| DC | -9.9e-07 | 1.7e-07 |
| attack to 90 % (ms) | - (switch) | - (switch) |
| release to -60 dB (ms) | - (held) | - (held) |
| L/R correlation | 1.0000 | 1.0000 |
| **distance** h1-40 / log-mel / LUFS env | | 0.71 dB / 16.45 dB / 0.10 LU |

### Segment 5: REF square vs GTS Square Morph 0

| h | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| REF dBc | 0.0 | -100.0 | -9.5 | -100.0 | -14.0 | -100.0 | -16.9 | -100.0 | -19.1 | -100.0 | -20.9 | -100.0 | -22.3 | -100.0 | -23.6 | -100.0 |
| GTS dBc | 0.0 | -100.0 | -9.5 | -100.0 | -14.0 | -100.0 | -16.9 | -100.0 | -19.1 | -100.0 | -20.8 | -100.0 | -22.3 | -100.0 | -23.5 | -100.0 |

| metric | REF | GTS |
|---|---|---|
| RMS / peak dBFS (GTS raw, before gain) | -14.03 / -9.54 | -16.68 / -9.17 |
| gain applied to GTS | | +2.65 dB |
| LUFS (after gain) | -11.39 | -11.39 |
| tilt dB/oct (200 Hz-10 kHz) | -6.0 | -6.0 |
| -60 dBc bandwidth | h83, 21715 Hz | h63, 16482 Hz |
| non-harmonic floor | -106.2 dB | -105.8 dB |
| DC | -7.4e-06 | -5.1e-07 |
| attack to 90 % (ms) | - (switch) | - (switch) |
| release to -60 dB (ms) | 14.4 | 50.5 |
| L/R correlation | 1.0000 | 1.0000 |
| **distance** h1-40 / log-mel / LUFS env | | 0.02 dB / 16.78 dB / 0.01 LU |

**Plots:** `docs/audits/renders/compare/plots/gts-sequence-seg{1..5}.png` (gitignored). Each one
overlays REF and GTS harmonic stems with 1/n and 1/n² guides.

### Where the log-mel distance comes from

These are 64 mel bands at 30 Hz–22 kHz. Band centres below 16 kHz are 59 of the 64.

| seg | log-mel RMS dB, all bands | bands < 16 kHz | bands ≥ 16 kHz |
|---|---|---|---|
| 1 saw | 15.89 | 0.42 | 56.84 |
| 2 sine | 0.61 | 0.44 | 1.56 |
| 3 triangle | 22.30 | 21.49 | 30.20 |
| 4 blend | 16.46 | 1.53 | 58.65 |
| 5 square | 16.77 | 0.40 | 59.98 |

Below 16 kHz, GTS matches the saw, sine and square to 0.4 dB. The large totals come from the
missing harmonics above 16.7 kHz (segments 1, 4 and 5) and from the missing triangle (segment 3).

### Steady-state crest factor (dB peak/RMS, segment minus 0.3 s each edge)

| | saw | sine | tri-class | blend-class | square |
|---|---|---|---|---|---|
| REF | 7.85 | 3.01 | 4.70 | 6.37 | 3.17 |
| GTS, init crossover 120 Hz | 11.72 | 3.01 | 7.51 | 7.51 | 7.51 |
| GTS, crossover 20 Hz (diagnostic) | 8.79 | 3.01 | 5.19 | 5.19 | 5.19 |

The GTS tables store textbook phases. Read straight from `Tables::mip`: Harmonic Rise grain 21
steps by π/2 per harmonic, and Square Morph grain 0 alternates 0/π. The GTS output does not keep
those phases.

- With the init crossover at 120 Hz, the rel phases of the output saw are −2.22, +2.71, +1.57,
  +0.51 … for k = 2, 3, 4, 5.
- With the crossover at 20 Hz they are +0.93, +1.99, +3.10 …. The step converges to 1.13 rad,
  with h1 rotated by about 0.44 rad against the line through the other harmonics.

The crossover runs the mid channel through an LR8 allpass even when the signal is mono, and that
allpass rotates the low harmonics. The harmonic amplitudes are unchanged (h1–40 within 0.03 dB),
but the waveform shape changes: the saw's peak is +3.9 dB over the reference's at equal RMS.

## Gap list, ranked by audibility

1. **Bandwidth depends on the note, and loses an octave just above C4.** Measured at C4: GTS keeps
   64 harmonics (top 16.74 kHz, −60 dBc bandwidth h63–64); the reference keeps h84 (21.98 kHz).
   Measured at C#4 (277.18 Hz): GTS keeps **32 harmonics, 8.87 kHz** (`renders/compare/cs4`, saw
   −60 dBc bandwidth h32 = 8870 Hz). The cause is mip selection by whole octaves, `HMAX >> m` under
   `TOP_HZ`, so the top harmonic lands anywhere from 8.75 to 17.5 kHz depending on the note. The
   reference was measured at one note only, so its behaviour at other notes is not known. In log-mel
   terms, the bands ≥ 16 kHz differ by 57–60 dB at C4.
   **Action:** stop band-limiting by octave. Either store more mips (half-octave mips give a
   12.4–17.5 kHz range; per-semitone harmonic counts give ≤ 6 % below the cap) or truncate the
   series at `floor(top/f0)` per note. Raise the top from 17.5 kHz toward 0.45·sr (19.8 kHz at
   44.1 kHz), still inside the SPEC 1.1 alias test. **Size M.**
2. **No triangle, and no textbook basic-shapes table.** The reference's triangle is exact odd
   1/k² (tilt −12.05 dB/oct, −60 dBc at h31). The closest GTS position (Square Morph 0) measures
   11.77 dB mean harmonic distance and 21.5 dB log-mel distance below 16 kHz, and the next nearest
   (FM Morph 18–21) is 12.8–13.2 dB away. Sine, saw and square all exist in GTS with exact series
   (h1–40 within 0.03 dB), but they are spread across Harmonic Rise, Saw Stack and Square Morph.
   **Action:** add a dedicated **Basic** table (sine → triangle → saw → square frames, exact 1/k
   and 1/k² series, textbook sine/cosine phases, band-limited by the same mips), with neighbouring
   frames kept phase-compatible so Index scans do not cancel. Append it to `Table` (saved state is
   append-only) and give it its own loudness trim. Decide whether frames are RMS-normalised (the
   GTS norm) or peak-normalised (the reference spans 5.2 LU across its frames; see gap 6).
   **Size M.**
3. **The init crossover rotates the phase of a mono signal.** Steady crest at equal RMS is +3.9 dB
   over the reference on the saw and +4.3 dB on the square. At C4 the raw GTS saw peaks at
   −4.69 dBFS against an RMS of −16.65 dBFS. In steady state the harmonic magnitudes are identical,
   so this costs headroom. It also changes what any downstream nonlinearity sees: the shaper drives
   a different waveform from the table's. **Action:** make the crossover's minimum an exact bypass
   (crossfade on enable) and default the init to Off. Alternatively, skip the allpass while the
   voice's side signal is zero (unison width 0, spread 0). Then re-measure crest against the table
   phases. **Size S.**
4. **Release length and shape.** The reference reaches −60 dB 14.4 ms after note-off, with a
   concave dB curve that is silent by about 16 ms. GTS at its init release of 60 ms reaches −60 dB
   at 50.5 ms, with −6.3 dB at +10 ms, −21.5 at +30, −42.2 at +50 and −73 at +60 ms, and the tail
   lasts to +73 ms. GTS has a curve parameter, so it can produce either shape; the difference is
   the init values. **Action:** decide the init release from bass use (the reference's 15 ms is a
   data point, not a target). If matching is wanted, set release ≈ 0.015 s and try a curve
   > 0, then confirm with `compare.py`. **Size S.**
5. **The reference's segment-4 frame has no GTS counterpart.** It is odd-only with a 1/k envelope
   1.3–1.5 dB under a stronger h1, and has dispersed phases. Square Morph 0 is 0.71 dB away in
   mean harmonic distance and 1.53 dB in log-mel below 16 kHz. Of all the pairs, this is the
   smallest difference that the measurements still resolve. What the frame is meant to be is not
   determined. **Action:** none until a use is identified. If the Basic table is built, the
   square-end frames could include an h1-weighted variant. **Size S.**
6. **Loudness across positions is a design difference, not a lack.** The reference keeps peak
   amplitude constant, so loudness moves 5.2 LU across its five positions. GTS keeps RMS constant
   (−16.65 to −16.68 dBFS raw), so switching position changes loudness by at most 0.43 LU after
   matching (the LUFS-envelope distance). **Action:** keep GTS's scheme. Note it in the Basic
   table design (gap 2). **No size.**
7. **Switch smoothing.** The reference's position change settles in 13–14 ms without dips or
   clicks. GTS latches Index per grain and crossfades a jump over one period (3.8 ms at C4); no
   click is measured on either. **Action:** none needed for clicks. A 10–15 ms Index glide is
   optional if the smoother switch matters. **Size S.**

**Not gaps.**

- **Noise floor:** both −105 to −106 dB, at the measurement floor.
- **DC:** GTS ≤ 1.4e-5 vs the reference saw's −1.0e-4.
- **Stereo:** correlation 1.0000 on both.
- **Pitch:** f0 261.626 Hz on both; the reference shows no drift and no analog-style detail
  (exact series, textbook phases), so there is nothing analog to copy.
- **Attack:** 90 % reached in 2.4 ms (reference) vs 3.3 ms (GTS, 1 ms attack), both within the
  metric's resolution of about 1.9 ms. GTS's first output sample is 0.0115 where the reference's
  is 0, inside GTS's 1 ms attack.
- **Init level:** GTS raw RMS −16.65 dBFS sits inside the reference's −14.0 to −18.8 dBFS range.
