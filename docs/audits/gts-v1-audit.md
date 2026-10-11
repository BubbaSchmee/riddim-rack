# GTS v1 measurement audit (2026-10-10)

Every number below comes from rendering through the real `gts::dsp::Engine` (or, for filter
impulse responses, the real `MultiFilter`) at 48 kHz:

- `crates/gts/examples/audit_render.rs` runs a job file and writes raw f32 renders, impulse responses and CPU benches
- `docs/audits/gts_audit.py` (py -3.12, numpy/scipy) writes the jobs, measures, prints every figure here, and writes `renders/*.wav`

To reproduce: `cargo build -p gts --release --example audit_render`, then `py -3.12 docs/audits/gts_audit.py`.
Nothing in the synth was changed.

The **measurement patch** (`base=raw`) is the default patch with Routing Bypass, Mod A Index 0, the
shaper off and Volume -12 dB, so the safety limiter never engages. Steady-state analysis uses a
Blackman-Harris window over 0.25 s to 1.0 s (2.0 s for the aliasing map).

**Alias metric.** I tried the brief's "±1 % of k·f0" harmonic band first and dropped it. At C5 the
aliases land 0.27·f0 from a harmonic, which falls inside a 1 % band once k > 27, so C5 came out
"clean" when it is not. GTS pitch is exact (the grain phase is f64), so harmonic bins are taken as
±6 FFT bins (the Blackman-Harris main lobe is 4 bins). The result is non-harmonic energy over
harmonic energy, in dB. Flag: above -40 dB.

**Fact-check note.** Web search was unavailable in this run (session limit). Industry references
below are marked *(not re-verified this run)*. Each finding stands on its measurement plus physical
limits (the 20 kHz audible band, Nyquist at 24 kHz). It does not rest on a claim about another product.

## Ranked findings (by audible impact)

### 1. Bandwidth wall: low notes carry nothing above 64·f0 (sounds dull, muffled, "cheap") — size M
- The tables stop at harmonic 64. The highest frequency GTS can produce is 2.09 kHz at C1, 2.79 kHz at
  F1 (the demo's riddim root), 4.19 kHz at C2 and 8.37 kHz at C3. Filling the band to 20 kHz needs
  611 harmonics at C1 and 305 at C2.
- Measured at C2 (RMS-normalised):

  | Source | Energy above 4.2 kHz | Energy 10–20 kHz |
  |---|---|---|
  | GTS 50 % square | -83.5 dB | -86.2 dB |
  | Full-band square | -23.0 dB | -28.7 dB |
  | PolyBLEP square | -23.9 dB | -31.1 dB |

  GTS is missing about 60 dB of top octave compared with even a cheap PolyBLEP oscillator.
- Default patch at F1:

  | Signal | Above 2 kHz | Above 3 kHz | Centroid |
  |---|---|---|---|
  | GTS default patch | -35.5 dB | -90.7 dB | 71 Hz |
  | Full-band square at F1 | -21.0 dB | -22.9 dB | — |

  Other default-patch figures: RMS -15.4 dBFS, peak 0.34 (-9.4 dBFS), K-weighted -19.9 LUFS.
- Q9 asked how much "digital HF garbage" GTS adds. The answer is none: the default patch's
  non-harmonic energy is -63 to -69 dB. The problem is the opposite. There is no top end at all, so
  the patch sounds like a filtered sub, which works against a riddim bass's grit and presence.
- **Fix:** build per-octave mip tables. Each octave holds every harmonic below about 20 kHz for that
  octave's top note, and a 2048-sample frame can hold up to 1023 harmonics. Pick the mip from
  `f0·2^(shift/12)`, the effective table rate. SPEC already names this as the upgrade path. For
  context, the common commercial wavetable frame is 2048 samples, as in Serum and Vital *(not
  re-verified this run)*. This fix also removes finding 2.

### 2. Aliasing above about F#4 (sounds harsh and inharmonic, "out of tune fizz") — size M (same work as 1)
- Flagged cells: 96 of 288.
- Shift 0, by note:

  | Notes | Median | Worst | Cells flagged |
  |---|---|---|---|
  | C1–C3 | -80.8 dB | -66.3 dB | 0 / 48 |
  | C4 | -77.6 dB | -8.5 dB | 17 / 48 (all at shift +12) |
  | C5 | -21.3 dB | +0.6 dB | 37 / 48 |
  | C6 | -13.5 dB | +5.6 dB | 42 / 48 |

  The +5.6 dB worst case is Grit at index 127 with shift +12: more alias energy than harmonic energy.
- The edge is 24 kHz / 64 = 375 Hz (F#4). Shift +12 doubles the table rate, so C4 aliases at +12.
  Mod A pitch, Octave and Semi all push notes over the edge too.
- Worst rows at C5, shift 0: Reed index 127 -11.0 dB, Square Morph index 127 -17.9 dB, Saw Stack
  index 127 -20.1 dB.
- Two tables are clean everywhere at shift 0: Harmonic Rise index 0 and FM Morph index 0 (both pure
  sines). FM Morph index 64 is also clean up to C6.
- C1–C3 sit at about -80 dB, which is linear interpolation of a 2048-point table. Inaudible.
- **Fix:** the mip tables from finding 1.

### 3. Comb filters do not line up with the note; Comb- res 1.0 loses 28 dB (sounds hollow, phasey, then disappears) — size S–M
- Static response at 880 Hz:

  | Type and res | Peaks | Between teeth |
  |---|---|---|
  | Comb- res 0.6 | 0 dB | -11.7 dB |
  | Comb- res 1.0 | -0.7 dB, and very narrow (DC is already -6.1 dB at 5 Hz) | -39.9 dB |
  | Comb+ res 0.6 | — | -7.9 dB |
  | Comb+ res 1.0 | — | -54.7 dB |

- In the default patch the comb sits at 880 Hz × 2^(kbd 0.5 · (note-60)/12) plus the filter envelope.
  That is not a multiple of f0, so the teeth fall at arbitrary points across the harmonics.
- Measured at F1, filter A in versus Bypass, sustain portion:

  | Setting | Fundamental | Harmonics 2–39 | Overall level |
  |---|---|---|---|
  | Default (Comb- res 0.6) | -2.2 dB | -11.7 to -0.3 dB | -3.4 dB |
  | Comb- res 1.0 | -26.7 dB | — | -28.1 dB (peak 0.085) |
  | Comb+ res 1.0 | -0.2 dB | down to -29.2 dB | -0.8 dB |

  At Comb- res 1.0 the patch nearly vanishes. Producers turn Res up expecting more resonance and get silence.
- The filter envelope (+4 octaves at the attack peak) also sweeps the comb delay on every note.
  That is a built-in flanger swoop, which reads as "phasey".
- **Fix:**
  1. Express comb Freq relative to the played note, or snap the delay to an integer multiple of the
     note period. Default Kbd to 1.0 for the comb types.
  2. Replace Comb-'s `(1-fb)` output normalisation with a loudness-matched one. The mean power gain
     of a feedback comb is (1-fb)/(1+fb), so compensate partway, e.g. ×sqrt((1+fb)/(1-fb)) followed by a soft clip.

### 4. Voice is cut while the filter is still ringing (click at note end) — size S
- `Voice::is_active` looks only at the amp envelopes, so a voice goes idle mid-ring.
- Cut levels at the moment the voice dies (F1, default release 60 ms):

  | Filter | Last sample | RMS of last 10 ms |
  |---|---|---|
  | LP12 400 Hz res 1.0 | -25.2 dBFS | -26.8 dBFS |
  | Comb- res 1.0 | -53.2 dBFS | — |
  | Default patch, att/rel 1 ms | -23.4 dBFS jump | — |

  With att/rel 1 ms the note-off step is 0.0678 against a steady-state maximum step of 0.0201.
- **Fix:** keep the voice alive until both filters' output stays below about -90 dBFS for one
  block (or until a 5 ms fade completes). This is a tail check in `is_active`.

### 5. Shaper changes loudness more than tone; half the Clip knob is only a volume knob — size S
- Toggling the default shaper (Saturate 0.3) on Square Morph C2: +8.6 dB.
- On the C3 64-harmonic saw (dry -24.1 dBFS):

  | Mode and amt | Level change | Sine THD |
  |---|---|---|
  | Saturate 0.5 | +10.0 dB | -14.7 dB (18 %) |
  | Clip 0.5 | +5.2 dB | 0 % (input peak 0.35 never reaches the 0.55 threshold) |
  | Clip 1.0 | — | -9.5 dB (34 %) |
  | Quant 1.0 (2 bit) | — | -6.3 dB (48 %) |

- A louder signal always wins an A/B, so the shaper cannot be judged by ear, and the default patch
  jumps when it is toggled.
- **Fix:** add output gain compensation per mode, normalised to the expected input RMS of 0.25.
  Remap the Clip threshold to the real signal level (about 0.5 down to 0.05).

### 6. Shaper aliases (no oversampling) (sounds fizzy and gritty in a bad way) — size M
- Alias energy on the C3 saw:

  | Mode | amt 0.5 | amt 1.0 |
  |---|---|---|
  | Saturate | -45.8 dB | -38.7 dB |
  | Clip | — | -37.0 dB |
  | Quant | -49.2 dB | -26.8 dB |

  Quant's aliasing is partly the intended bitcrush. On a sine input, Saturate stays clean (-95 dB)
  because tanh of a low-level sine has a fast-falling harmonic series. At C5 the oscillator's own
  -24 dB aliasing (finding 2) dominates.
- **Fix:** run the Sine, Saturate and Clip modes at 2× or 4× oversampling with a polyphase halfband
  filter, or use first-order antiderivative anti-aliasing (ADAA) for tanh and clip. CPU is
  available (finding 14). Oversampling around nonlinear stages is standard practice in commercial
  synth distortion *(not re-verified this run)*.

### 7. Shift > 0 adds deep per-period amplitude modulation; Shift < 0 loses level (sounds buzzy / goes quiet) — size M
- The grain scheme is spectrally clean. At ±5, ±7, ±12, +19 and ±24 the f0/2 sub-harmonic is ≤ -107
  dBc and non-harmonic energy is ≤ -81 dB. No sidebands and no sub-octave.
- On a pure sine (Harmonic Rise index 0, C2), Hilbert-envelope AM depth:

  | Shift | -24 | -12 | -7 | -5 | 0 | +5 | +7 | +12 | +19 | +24 |
  |---|---|---|---|---|---|---|---|---|---|---|
  | AM depth | 7.3 % | 0.3 % | 3.2 % | 3.7 % | 0.3 % | 61 % | 97 % | 98 % | 99.3 % | 99.6 % |

- For Shift > 0 the PSOLA window (2/ratio periods) leaves the two Hann windows summing well below 1.
  Above +12 they leave gaps. The output becomes a pulsed grain train at f0, which is the "buzz".
- Level:

  | Signal | Shift 0 | Shift -12 | Shift -24 | Shift > 0 |
  |---|---|---|---|---|
  | Square Morph index 64 | -24.1 dBFS | -23.7 dBFS | -35.4 dBFS (-11.3 dB) | within 2 dB |
  | Sine | -24.1 dBFS | -30.1 dBFS | -38.3 dBFS (-14.2 dB) | within 2 dB (sqrt(ratio) compensation) |

- **Fix:**
  1. For 1 < ratio < 2, divide by the window sum `wa+wb`, which is non-zero there. For ratio ≥ 2,
     use a flatter (Tukey) window.
  2. Add level compensation for ratio < 1, mirroring the sqrt(ratio) used above 1.
  3. Or keep the buzz as character, but document it in the UI.

### 8. LP12 at Res 1.0: +26 dB peak, no self-oscillation, runs into the limiter — size S
- Q = 0.5·40^res, so the range is 0.5 to 20.
- Res 1.0: peak +26.0 dB at the cutoff, rings for 44 ms to -60 dB. It is a linear SVF, so it never self-oscillates.
- Res 0: Q 0.5, so the cutoff point is -6 dB, softer than a Butterworth's -3 dB.
- In the engine, with the cutoff on the 4th harmonic of C2, Res 1.0 raises the level 6.9 dB over
  Res 0 and peaks at 1.000. The tanh "safety" limiter becomes the distortion.
- **Fix:** add a soft nonlinearity (tanh) on the SVF band state, the usual analog-modelled SVF
  treatment, which bounds the peak and allows a controlled self-oscillation near the top of the
  range. Remap Q to start at 0.707.

### 9. Note-on transient at fast attack — size S (low impact)
- Grains start at table position 0, which is the pulse centre, i.e. the waveform peak.
- Measured max sample step at note-on against steady state (raw oscillator, C2, Square Morph index 64):

  | Attack | Note-on step | Steady state |
  |---|---|---|
  | 1 ms | 0.052 | 0.0176 (3×) |
  | 5 ms (default) | 0.0305 | 0.0176 (1.7×) |

- This gives consistent "punch" and some producers want it. It turns into a click when Attack is
  set to 1 ms deliberately.
- **Fix:** add an optional start phase / retrigger control.

### 10. Loudness consistency across tables — size S (minor)
- C2, index 64: RMS spread 2.7 dB; K-weighted (BS.1770) spread 3.2 dB. Vowel is quietest (-27.3
  LUFS) and Grit loudest (-24.1 LUFS).
- Index 0→127 sweep: Square Morph 1.7 dB RMS / 1.3 dB K-weighted; Saw Stack 0.6 dB / 1.5 dB.
- The 2026-10-10 RMS normalisation worked. The remaining spread is perceptual.
- **Fix:** per-table K-weighted trim, about +1 to +3 dB on Vowel.

### 11. Envelope shape — no fix needed (documentation)
- The curves are exponential, analog style. Measured with A 100 ms / D 300 ms / S 0.5 / R 300 ms on a C4 sine.
- Attack is convex: 0.41 at 25 % of A, 0.68 at 50 %, 0.87 at 75 % (linear would be 0.25 / 0.50 / 0.75).
- Release: -10.5 dB at 30 ms, -15 dB at 60 ms, -30 dB at 150 ms.
- Release ends cleanly. The final non-zero sample is -155 dBFS.
- Release length depends on the sustain level. It ends at 0.79·R from sustain 0.5 (measured
  236 ms for R = 300 ms) and at 0.92·R from 1.0. The doc comment says about 0.92·R regardless.

### 12. Mono legato portamento — no fix needed
- The glide is exponential in semitones (RC style).
- C2→C3 at porta 80 ms: 10 % at 2.3 ms, 50 % at 13.9 ms, 90 % at 46.3 ms, 98 % at 78.8 ms. This matches SPEC.
- There is no constant-time (linear) glide option. Many commercial synths offer one as a curve control *(not re-verified this run)*.

### 13. AM filter mode aliasing — clean
- Carrier 5 kHz on the C3 saw: energy outside the expected k·f0 and k·f0 ± fc lines is -84.5 dB.
- Carrier 15 kHz: -88.3 dB.
- No fix.

### 14. CPU per voice — fine
- Release build, default patch: 135 ns per sample per voice, i.e. 0.65 % of one core at 48 kHz.
- 8 voices: 947 ns per sample (4.5 %).
- There is plenty of headroom for mip tables and a 2× oversampled shaper.

## Listening renders (`docs/audits/renders/`, 16-bit 48 kHz mono, 4.7 MB, not committed)

| File | Demonstrates |
|---|---|
| 01_default_patch_F1_one_note | the default patch as shipped (dull: finding 1) |
| 02_bandwidth_GTS_square_vs_fullband_square_C1_C2 | GTS 50 % square, then a full-band square, at C1 and C2, level-matched |
| 03_aliasing_worst_cells_C5_C6 | the four worst aliasing cells |
| 04_aliasing_saw_stack_C6_shift12_vs_polyblep | Saw Stack C6 at +12 against a PolyBLEP saw an octave up |
| 05_click_default_patch_att1ms_rel1ms_vs_5_60ms | note on/off clicks at 1 ms, then the 5/60 ms defaults |
| 06_voice_end_cuts_ringing_filter_LP12res1_then_CombMinus_res1 | ringing filter cut off at voice end |
| 07_shift_sweep_steps_sine_minus24_to_plus24 | AM buzz above 0 st and level loss below on a pure sine (shifts -24 -12 -7 -5 0 5 7 12 19 24) |
| 08_shift_steps_square_minus24_to_plus24 | the same steps on Square Morph |
| 09_shaper_dry_sat_clip_quant_amt1_C5 | shaper aliasing at C5 (level-matched) |
| 10_comb_minus_vs_plus_res1_F1 | Comb- res 1 nearly silent against Comb+ res 1 |
| 11_lp12_res1_vs_res0 | LP12 res 1 into the limiter, then res 0 |
| 12_porta_C2_to_C3_80ms | default glide |
| 13_table_tour_C2_idx64 | all 8 tables, level-matched |
| 14_am_15k_carrier_C3_saw | AM at a 15 kHz carrier (clean) |
