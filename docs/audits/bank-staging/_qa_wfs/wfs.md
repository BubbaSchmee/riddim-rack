# Bank QA report

Tool `bank_qa` (GTS), contract `docs/design/bank-recipes.md` section A. Renders at 150 and 140 BPM, 48 kHz, velocity 100 (127 for the peak check). Tags that steer the tool are listed in the source header of `crates/gts/examples/bank_qa.rs`.

Inputs: `docs/audits/bank-staging/wobble-formant-subs/`

## Summary

| Preset | Family | Checks | Fail | Info/Skip | Result |
|---|---|---|---|---|---|
| Dirty Sub Layer | sub (tags) | 41 | 0 | 4 | PASS |
| Drift Random Wobble | wobble (tags) | 43 | 0 | 4 | PASS |
| Eighth Triangle Wub | wobble (tags) | 46 | 0 | 3 | PASS |
| Pure Sine Sub | sub (tags) | 42 | 0 | 3 | PASS |
| Sixteenth Gate Wub | wobble (tags) | 46 | 0 | 3 | PASS |
| Slow Sine Wub | wobble (tags) | 46 | 0 | 3 | PASS |
| Talking Chatter | formant (tags) | 42 | 0 | 5 | PASS |
| Triplet Saw Chop | wobble (tags) | 46 | 0 | 3 | PASS |
| Vowel Pluck Bark | formant (qa_family tag) | 38 | 0 | 7 | PASS |
| Warm Tube Sub | sub (tags) | 41 | 0 | 4 | PASS |
| Yoi Sweep Slow | formant (tags) | 42 | 0 | 5 | PASS |

## Dirty Sub Layer (sub, Bass/sub)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 716.0 chars |  | +636.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -11.71 | -11.70 | +8.704 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.87 | -17.86 | +0.635 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -18.38 | -18.38 | +1.118 | pass |
| Crest (peak - RMS) | 3.00..7.00 dB | 5.79 | 5.78 | +1.215 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.135 | 0.135 | +1.865 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.995 | 1.00 | 1.00 | +0.005 | pass |
| Low-band side/mid, dB | - dB | -300.0 | -300.0 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 40.00..160.0 Hz | 129.2 | 129.2 | +30.817 | pass |
| Width: L/R correlation above 200 Hz | 0.950..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Static: Move depth | <= 0.250 oct | 0.209 | 0.210 | +0.040 | pass |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | -0.243 | -0.252 | +2.748 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -11.71 |  | +8.705 | pass |
| Alias floor (frozen modulation) | <= -60.00 dB | -86.69 |  | +26.687 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -87.17 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.000 |  | +1.000 | pass |
| Energy below 150 Hz (share) | >= 90.00 % | 92.44 |  | +2.442 | pass |
| THD below 150 Hz | - % | 51.50 |  | - | info |
| Macro 1 "Dirt" sweep | >= 0.300 | 2.20 C oct |  | +1.898 | pass |
| Macro 2 "Layer Mix" sweep | >= 1.00 | 2.96 L LU |  | +1.962 | pass |
| Macro 3 "Sub Level" sweep | >= 1.00 | 4.03 L LU |  | +3.027 | pass |
| Macro 4 "Cutoff" sweep | >= 0.300 | 3.35 C oct |  | +3.052 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.000 |  | +2.000 | pass |
| Log-mel distance to the nearest sub preset | >= 3.30 | 6.17 dB |  | +2.867 | pass |

## Drift Random Wobble (wobble, Bass/wobble)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 719.0 chars |  | +639.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -7.28 | -7.43 | +4.284 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.72 | -17.72 | +0.778 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.75 | -16.75 | +2.747 | pass |
| Crest (peak - RMS) | 5.00..11.00 dB | 10.12 | 10.29 | +0.706 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.281 | 0.278 | +1.719 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.000 | 1.000 | +0.010 | pass |
| Low-band side/mid, dB | - dB | -63.93 | -64.42 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.000 | -0.000 | +1.000 | pass |
| Brightness (power centroid) | 180.0..1600.0 Hz | 950.3 | 953.4 | +646.580 | pass |
| Width: L/R correlation above 200 Hz | 0.700..1.00 | 0.996 | 0.996 | +0.004 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.800 oct | 1.40 | 1.33 | +0.532 | pass |
| Not periodic: largest ACF peak beyond 30 ms (Mod B SmoothRandom 100 Hz) | <= 0.400 | 0.235 | 0.201 | +0.165 | pass |
| Sub (<90 Hz) RMS variation over the cycle | <= 1.50 dB | 1.18 | 1.22 | +0.277 | pass |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | -0.541 | -0.545 | +2.455 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -7.51 |  | +4.514 | pass |
| Alias floor (frozen modulation) | <= -55.00 dB | -86.45 |  | +31.453 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -86.72 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.209 |  | +0.791 | pass |
| Macro 1 "Drift Depth" sweep | >= 0.300 | 1.79 D oct |  | +1.493 | pass |
| Macro 2 "Centre" sweep | >= 0.300 | 0.904 C oct |  | +0.604 | pass |
| Macro 3 "Slow Sweep" sweep | >= 0.300 | 0.643 C oct |  | +0.343 | pass |
| Macro 4 "Width" sweep | >= 0.100 | 0.322 W corr |  | +0.222 | pass |
| Repeatability (two renders, -80 dB) | - | n/a  |  | - | skip |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.146 |  | +1.854 | pass |
| Log-mel distance to the nearest wobble preset | >= 3.30 | 6.15 dB |  | +2.854 | pass |

Details of failed or skipped checks:

- repeat: random LFO or free random phase: the engine seeds those from a global counter

## Eighth Triangle Wub (wobble, Bass/wobble)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 801.0 chars |  | +721.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -7.95 | -7.96 | +4.950 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.81 | -17.81 | +0.689 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.65 | -16.65 | +2.852 | pass |
| Crest (peak - RMS) | 5.00..11.00 dB | 9.86 | 9.85 | +1.139 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.189 | 0.189 | +1.811 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | - dB | -300.0 | -300.0 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 180.0..1600.0 Hz | 643.5 | 632.2 | +452.183 | pass |
| Width: L/R correlation above 200 Hz | 0.700..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.800 oct | 1.72 | 1.71 | +0.912 | pass |
| Move period vs Mod B Triangle 1/8 = 0.2000 s (error) | -5.00..5.00 % | 0.895 | 1.47 | +3.532 | pass |
| Move period ACF peak height | >= 0.500 | 0.959 | 0.972 | +0.459 | pass |
| Sub (<90 Hz) RMS variation over the cycle | <= 1.50 dB | 0.870 | 0.910 | +0.590 | pass |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | -0.500 | -0.506 | +2.494 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -7.95 |  | +4.950 | pass |
| Alias floor (frozen modulation) | <= -55.00 dB | -71.51 |  | +16.513 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -71.40 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.131 |  | +0.869 | pass |
| Macro 1 "Wobble Depth" sweep | >= 0.300 | 1.64 D oct |  | +1.341 | pass |
| Macro 2 "Cutoff" sweep | >= 0.300 | 1.57 C oct |  | +1.270 | pass |
| Macro 3 "Grit" sweep | >= 0.300 | 1.83 C oct |  | +1.531 | pass |
| Macro 4 "Phase Shift" sweep | >= 0.300 | 0.505 C oct |  | +0.205 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.054 |  | +1.946 | pass |
| Log-mel distance to the nearest wobble preset | >= 3.30 | 4.17 dB |  | +0.870 | pass |

## Pure Sine Sub (sub, Bass/sub)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 715.0 chars |  | +635.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -13.42 | -13.42 | +10.421 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.46 | -17.45 | +1.045 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -19.04 | -19.04 | +0.455 | pass |
| Crest (peak - RMS) | 3.00..7.00 dB | 3.26 | 3.26 | +0.263 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.545 | 0.546 | +1.454 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.995 | 1.00 | 1.00 | +0.005 | pass |
| Low-band side/mid, dB | - dB | -300.0 | -300.0 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 40.00..160.0 Hz | 44.60 | 44.60 | +4.600 | pass |
| Width: L/R correlation above 200 Hz | 0.990..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Static: Move depth | <= 0.100 oct | 0.035 | 0.035 | +0.065 | pass |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | -0.321 | -0.323 | +2.677 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -13.42 |  | +10.422 | pass |
| Alias floor (frozen modulation) | <= -70.00 dB | -87.58 |  | +17.579 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -87.66 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.000 |  | +1.000 | pass |
| Energy below 150 Hz (share) | >= 99.00 % | 100.00 |  | +1.000 | pass |
| THD below 150 Hz | <= 0.300 % | 0.000 |  | +0.300 | pass |
| Macro 1 "Level" sweep | >= 1.00 | 6.00 L LU |  | +5.000 | pass |
| Macro 2 "Soft Attack" sweep | >= 10.00 | 22.00 T ms |  | +12.000 | pass |
| Macro 3 "Octave Up" sweep | >= 1.00 | 1.63 L LU |  | +0.632 | pass |
| Macro 4 "Release" sweep | >= 50.00 | 406.1 T ms |  | +356.060 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.410 |  | +1.590 | pass |
| Log-mel distance to the nearest sub preset | >= 3.30 | 4.02 dB |  | +0.724 | pass |

## Sixteenth Gate Wub (wobble, Bass/wobble)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 662.0 chars |  | +582.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -7.63 | -7.62 | +4.623 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -18.08 | -18.07 | +0.421 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.93 | -16.93 | +2.568 | pass |
| Crest (peak - RMS) | 5.00..11.00 dB | 10.45 | 10.45 | +0.549 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -0.079 | -0.073 | +1.921 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | - dB | -300.0 | -300.0 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 180.0..1600.0 Hz | 596.9 | 583.2 | +403.203 | pass |
| Width: L/R correlation above 200 Hz | 0.700..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.800 oct | 1.82 | 1.82 | +1.017 | pass |
| Move period vs Mod B Square 1/16 = 0.1000 s (error) | -5.00..5.00 % | 0.389 | -0.135 | +4.611 | pass |
| Move period ACF peak height | >= 0.500 | 0.985 | 0.978 | +0.478 | pass |
| Sub (<90 Hz) RMS variation over the cycle | <= 2.00 dB | 0.632 | 0.786 | +1.214 | pass |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | -0.671 | -0.686 | +2.314 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -7.63 |  | +4.627 | pass |
| Alias floor (frozen modulation) | <= -55.00 dB | -84.21 |  | +29.205 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -84.28 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.652 |  | +0.348 | pass |
| Macro 1 "Gate Depth" sweep | >= 0.300 | 1.66 C oct |  | +1.363 | pass |
| Macro 2 "Cutoff" sweep | >= 0.300 | 1.30 C oct |  | +1.002 | pass |
| Macro 3 "Smooth" sweep | >= 0.300 | 0.425 C oct |  | +0.125 | pass |
| Macro 4 "Pump" sweep | >= 1.00 | 2.59 L LU |  | +1.594 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.214 |  | +1.786 | pass |
| Log-mel distance to the nearest wobble preset | >= 3.30 | 4.17 dB |  | +0.870 | pass |

## Slow Sine Wub (wobble, Bass/wobble)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 618.0 chars |  | +538.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -7.40 | -7.41 | +4.401 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -18.10 | -18.09 | +0.404 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.84 | -16.84 | +2.657 | pass |
| Crest (peak - RMS) | 5.00..11.00 dB | 10.68 | 10.67 | +0.322 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -0.096 | -0.088 | +1.904 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | - dB | -300.0 | -300.0 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 180.0..1600.0 Hz | 435.6 | 433.3 | +253.268 | pass |
| Width: L/R correlation above 200 Hz | 0.700..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.800 oct | 1.39 | 1.38 | +0.576 | pass |
| Move period vs Mod B Sine 1/4 = 0.4000 s (error) | -5.00..5.00 % | -0.074 | -0.185 | +4.815 | pass |
| Move period ACF peak height | >= 0.500 | 0.997 | 0.992 | +0.492 | pass |
| Sub (<90 Hz) RMS variation over the cycle | <= 1.50 dB | 1.17 | 1.17 | +0.328 | pass |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | -0.166 | -0.166 | +2.834 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -7.40 |  | +4.401 | pass |
| Alias floor (frozen modulation) | <= -55.00 dB | -68.03 |  | +13.031 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -68.35 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.877 |  | +0.123 | pass |
| Macro 1 "Wobble Depth" sweep | >= 0.300 | 1.14 D oct |  | +0.844 | pass |
| Macro 2 "Cutoff" sweep | >= 0.300 | 2.09 C oct |  | +1.787 | pass |
| Macro 3 "Resonance" sweep | >= 0.300 | 0.505 D oct |  | +0.205 | pass |
| Macro 4 "Warmth" sweep | >= 0.300 | 1.76 C oct |  | +1.464 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.231 |  | +1.769 | pass |
| Log-mel distance to the nearest wobble preset | >= 3.30 | 4.38 dB |  | +1.079 | pass |

## Talking Chatter (formant, Bass/formant)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 785.0 chars |  | +705.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -6.86 | -6.21 | +3.210 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.50 | -17.50 | +0.996 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.01 | -16.00 | +2.501 | pass |
| Crest (peak - RMS) | 6.00..12.00 dB | 10.64 | 11.29 | +0.706 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.496 | 0.496 | +1.504 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | - dB | -300.0 | -300.0 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 350.0..2200.0 Hz | 1026.7 | 1026.6 | +676.614 | pass |
| Width: L/R correlation above 200 Hz | 0.850..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 1.00 oct | 1.27 | 1.26 | +0.256 | pass |
| Move period vs Mod B Custom 1/2 = 0.8000 s (error) | -5.00..5.00 % | -0.509 | 0.152 | +4.491 | pass |
| Move period ACF peak height | >= 0.500 | 0.980 | 0.990 | +0.480 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.253 | 0.215 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -6.86 |  | +3.863 | pass |
| Alias floor (frozen modulation) | <= -55.00 dB | -86.23 |  | +31.231 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -86.29 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.132 |  | +0.868 | pass |
| Macro 1 "Syllables" sweep | >= 0.300 | 0.476 C oct |  | +0.176 | pass |
| Macro 2 "Mouth" sweep | >= 1.00 | 2.41 L LU |  | +1.413 | pass |
| Macro 3 "Glide" sweep | >= 1.00 | 1.04 L LU |  | +0.042 | pass |
| Macro 4 "Chatter" sweep | >= 0.300 | 0.343 D oct |  | +0.043 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.362 |  | +1.638 | pass |
| Log-mel distance to the nearest formant preset | >= 3.30 | 4.04 dB |  | +0.739 | pass |

## Triplet Saw Chop (wobble, Bass/wobble)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 674.0 chars |  | +594.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -7.59 | -7.55 | +4.546 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -18.21 | -18.23 | +0.275 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.86 | -16.88 | +2.622 | pass |
| Crest (peak - RMS) | 5.00..11.00 dB | 10.62 | 10.55 | +0.377 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -0.210 | -0.225 | +1.775 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | - dB | -300.0 | -300.0 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 180.0..1600.0 Hz | 748.9 | 738.1 | +558.084 | pass |
| Width: L/R correlation above 200 Hz | 0.700..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.800 oct | 1.93 | 1.92 | +1.119 | pass |
| Move period vs Mod B SawDown 1/8T = 0.1333 s (error) | -5.00..5.00 % | 0.127 | 0.130 | +4.870 | pass |
| Move period ACF peak height | >= 0.500 | 0.984 | 0.982 | +0.482 | pass |
| Sub (<90 Hz) RMS variation over the cycle | <= 1.50 dB | 0.858 | 0.799 | +0.642 | pass |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | -0.681 | -0.638 | +2.319 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -7.59 |  | +4.587 | pass |
| Alias floor (frozen modulation) | <= -55.00 dB | -65.26 |  | +10.264 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -65.89 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.234 |  | +0.766 | pass |
| Macro 1 "Chop Depth" sweep | >= 0.300 | 0.877 D oct |  | +0.577 | pass |
| Macro 2 "Brightness" sweep | >= 0.300 | 1.14 C oct |  | +0.839 | pass |
| Macro 3 "Release" sweep | >= 0.300 | 0.498 D oct |  | +0.198 | pass |
| Macro 4 "Dirt" sweep | >= 0.300 | 1.77 C oct |  | +1.471 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.345 |  | +1.655 | pass |
| Log-mel distance to the nearest wobble preset | >= 3.30 | 5.69 dB |  | +2.388 | pass |

## Vowel Pluck Bark (formant, Bass/formant)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 768.0 chars |  | +688.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -7.43 | -7.47 | +4.431 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -18.16 | -18.16 | +0.336 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.63 | -16.63 | +2.868 | pass |
| Crest (peak - RMS) | 6.00..12.00 dB | 10.73 | 10.69 | +1.268 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -0.164 | -0.162 | +1.836 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | - dB | -300.0 | -300.0 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 350.0..2200.0 Hz | 998.4 | 1000.8 | +648.434 | pass |
| Width: L/R correlation above 200 Hz | 0.850..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.030 oct | 0.064 | 0.056 | +0.026 | pass |
| Move period | - | skip | skip | - | skip |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.096 | 0.097 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -7.43 |  | +4.431 | pass |
| Alias floor (frozen modulation) | <= -55.00 dB | -87.04 |  | +32.040 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -87.09 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.086 |  | +0.914 | pass |
| Macro 1 "Bark" sweep | >= 1.00 | 3.08 L LU |  | +2.082 | pass |
| Macro 2 "Vowel" sweep | >= 0.300 | 0.463 C oct |  | +0.163 | pass |
| Macro 3 "Decay" sweep | >= 0.300 | 1.05 D oct |  | +0.746 | pass |
| Macro 4 "Grit" sweep | >= 1.00 | 4.40 L LU |  | +3.403 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.299 |  | +1.701 | pass |
| Log-mel distance to the nearest formant preset | >= 3.30 | 4.04 dB |  | +0.739 | pass |

Details of failed or skipped checks:

- period_err@150: qa_period=none
- period_err@140: qa_period=none

## Warm Tube Sub (sub, Bass/sub)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 702.0 chars |  | +622.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -12.88 | -12.88 | +9.875 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.03 | -17.03 | +1.467 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -18.62 | -18.62 | +0.883 | pass |
| Crest (peak - RMS) | 3.00..7.00 dB | 3.53 | 3.53 | +0.530 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.967 | 0.968 | +1.032 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.995 | 1.00 | 1.00 | +0.005 | pass |
| Low-band side/mid, dB | - dB | -300.0 | -300.0 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 40.00..140.0 Hz | 51.43 | 51.43 | +11.432 | pass |
| Width: L/R correlation above 200 Hz | 0.950..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Static: Move depth | <= 0.250 oct | 0.217 | 0.217 | +0.033 | pass |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | -0.268 | -0.268 | +2.732 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -12.88 |  | +9.877 | pass |
| Alias floor (frozen modulation) | <= -60.00 dB | -87.60 |  | +27.602 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -87.67 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.000 |  | +1.000 | pass |
| Energy below 150 Hz (share) | >= 90.00 % | 99.99 |  | +9.994 | pass |
| THD below 150 Hz | - % | 4.74 |  | - | info |
| Macro 1 "Warmth" sweep | >= 0.300 | 0.906 C oct |  | +0.606 | pass |
| Macro 2 "Small Speaker" sweep | >= 1.00 | 1.49 L LU |  | +0.491 | pass |
| Macro 3 "Level" sweep | >= 1.00 | 6.00 L LU |  | +5.000 | pass |
| Macro 4 "Release" sweep | >= 50.00 | 516.3 T ms |  | +466.253 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.832 |  | +1.168 | pass |
| Log-mel distance to the nearest sub preset | >= 3.30 | 4.02 dB |  | +0.724 | pass |

## Yoi Sweep Slow (formant, Bass/formant)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 746.0 chars |  | +666.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -6.84 | -6.96 | +3.844 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.93 | -17.93 | +0.566 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.13 | -16.11 | +2.611 | pass |
| Crest (peak - RMS) | 6.00..12.00 dB | 11.09 | 10.97 | +0.910 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.066 | 0.068 | +1.932 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | - dB | -300.0 | -300.0 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 350.0..2200.0 Hz | 1008.5 | 1009.9 | +658.507 | pass |
| Width: L/R correlation above 200 Hz | 0.850..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 1.00 oct | 1.66 | 1.65 | +0.648 | pass |
| Move period vs Mod A Triangle 1/2 = 0.8000 s (error) | -5.00..5.00 % | 0.382 | 0.151 | +4.618 | pass |
| Move period ACF peak height | >= 0.500 | 0.995 | 0.998 | +0.495 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 1.00 | 1.04 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -6.84 |  | +3.844 | pass |
| Alias floor (frozen modulation) | <= -55.00 dB | -84.09 |  | +29.088 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -84.49 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.950 |  | +0.050 | pass |
| Macro 1 "Vowel Sweep" sweep | >= 1.00 | 1.63 L LU |  | +0.625 | pass |
| Macro 2 "Vowel Position" sweep | >= 1.00 | 1.58 L LU |  | +0.584 | pass |
| Macro 3 "Sharpness" sweep | >= 1.00 | 2.00 L LU |  | +0.998 | pass |
| Macro 4 "Grit" sweep | >= 0.300 | 1.13 C oct |  | +0.833 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.069 |  | +1.931 | pass |
| Log-mel distance to the nearest formant preset | >= 3.30 | 4.46 dB |  | +1.162 | pass |

## Distinctiveness (log-mel distance, dB; threshold 3.3 dB)

Same-family pairs are judged; other pairs are listed for calibration.

| A | B | Same family | Distance |
|---|---|---|---|
| Pure Sine Sub | Warm Tube Sub | yes | 4.02 |
| Talking Chatter | Vowel Pluck Bark | yes | 4.04 |
| Drift Random Wobble | Yoi Sweep Slow | no | 4.06 |
| Eighth Triangle Wub | Sixteenth Gate Wub | yes | 4.17 |
| Sixteenth Gate Wub | Slow Sine Wub | yes | 4.38 |
| Talking Chatter | Yoi Sweep Slow | yes | 4.46 |
| Drift Random Wobble | Talking Chatter | no | 4.64 |
| Drift Random Wobble | Vowel Pluck Bark | no | 4.68 |
| Eighth Triangle Wub | Slow Sine Wub | yes | 4.94 |
| Vowel Pluck Bark | Yoi Sweep Slow | yes | 5.16 |
| Triplet Saw Chop | Yoi Sweep Slow | no | 5.48 |
| Sixteenth Gate Wub | Triplet Saw Chop | yes | 5.69 |
| Eighth Triangle Wub | Triplet Saw Chop | yes | 5.77 |
| Drift Random Wobble | Triplet Saw Chop | yes | 6.15 |
| Dirty Sub Layer | Warm Tube Sub | yes | 6.17 |
| Slow Sine Wub | Triplet Saw Chop | yes | 6.66 |
| Triplet Saw Chop | Vowel Pluck Bark | no | 6.86 |
| Talking Chatter | Triplet Saw Chop | no | 7.01 |
| Eighth Triangle Wub | Yoi Sweep Slow | no | 7.22 |
| Sixteenth Gate Wub | Yoi Sweep Slow | no | 8.64 |
| Eighth Triangle Wub | Talking Chatter | no | 9.03 |
| Eighth Triangle Wub | Vowel Pluck Bark | no | 9.05 |
| Slow Sine Wub | Yoi Sweep Slow | no | 9.19 |
| Drift Random Wobble | Eighth Triangle Wub | yes | 9.49 |
| Dirty Sub Layer | Pure Sine Sub | yes | 9.83 |
| Sixteenth Gate Wub | Talking Chatter | no | 10.04 |
| Sixteenth Gate Wub | Vowel Pluck Bark | no | 10.09 |
| Drift Random Wobble | Sixteenth Gate Wub | yes | 10.42 |
| Slow Sine Wub | Talking Chatter | no | 10.55 |
| Slow Sine Wub | Vowel Pluck Bark | no | 10.73 |
| Drift Random Wobble | Slow Sine Wub | yes | 11.51 |
| Dirty Sub Layer | Slow Sine Wub | no | 12.46 |
| Dirty Sub Layer | Sixteenth Gate Wub | no | 15.05 |
| Dirty Sub Layer | Eighth Triangle Wub | no | 15.25 |
| Dirty Sub Layer | Triplet Saw Chop | no | 16.58 |
| Slow Sine Wub | Warm Tube Sub | no | 16.64 |
| Dirty Sub Layer | Vowel Pluck Bark | no | 18.04 |
| Dirty Sub Layer | Yoi Sweep Slow | no | 18.46 |
| Dirty Sub Layer | Talking Chatter | no | 18.51 |
| Pure Sine Sub | Slow Sine Wub | no | 19.04 |
| Sixteenth Gate Wub | Warm Tube Sub | no | 19.11 |
| Eighth Triangle Wub | Warm Tube Sub | no | 19.69 |
| Dirty Sub Layer | Drift Random Wobble | no | 20.02 |
| Triplet Saw Chop | Warm Tube Sub | no | 20.74 |
| Pure Sine Sub | Sixteenth Gate Wub | no | 21.27 |
| Vowel Pluck Bark | Warm Tube Sub | no | 21.86 |
| Eighth Triangle Wub | Pure Sine Sub | no | 22.00 |
| Talking Chatter | Warm Tube Sub | no | 22.38 |
| Warm Tube Sub | Yoi Sweep Slow | no | 22.43 |
| Pure Sine Sub | Triplet Saw Chop | no | 22.90 |
| Pure Sine Sub | Vowel Pluck Bark | no | 23.65 |
| Drift Random Wobble | Warm Tube Sub | no | 23.96 |
| Pure Sine Sub | Talking Chatter | no | 24.23 |
| Pure Sine Sub | Yoi Sweep Slow | no | 24.37 |
| Drift Random Wobble | Pure Sine Sub | no | 25.84 |

## Bank summary

11 preset(s) analysed; 11 use the FX chain; 4 use a reordered chain (the sheet asks 20 of 41 and 6 of 41 for the whole bank). Failed checks: 0.
