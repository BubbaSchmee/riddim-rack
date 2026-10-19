# Bank QA report

Tool `bank_qa` (GTS), contract `docs/design/bank-recipes.md` section A. Renders at 150 and 140 BPM, 48 kHz, velocity 100 (127 for the peak check). Tags that steer the tool are listed in the source header of `crates/gts/examples/bank_qa.rs`.

Inputs: `docs/audits/bank-staging/riddim-seq-init`

## Summary

| Preset | Family | Checks | Fail | Info/Skip | Result |
|---|---|---|---|---|---|
| Call and Response Square | riddim (tags) | 46 | 2 | 3 | FAIL |
| Dotted Metal Stab | riddim (tags) | 46 | 5 | 3 | FAIL |
| Hollow Square Reference | riddim (tags) | 46 | 3 | 3 | FAIL |
| Init Square Bass | init (category) | 21 | 0 | 7 | PASS |
| Pulse Width Pocket | riddim (tags) | 46 | 2 | 3 | FAIL |
| Robot Bark Drop | riddim (tags) | 43 | 3 | 5 | FAIL |
| Robotic Comb Chatter | riddim (tags) | 46 | 3 | 3 | FAIL |
| Triplet Bark AM | riddim (tags) | 46 | 7 | 3 | FAIL |

## Call and Response Square (riddim, Bass/riddim)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 435.0 chars |  | +355.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -5.46 | -5.34 | +2.336 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -18.16 | -18.15 | +0.341 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.17 | -16.13 | +2.626 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 12.60 | 12.81 | +0.189 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -0.159 | -0.147 | +1.841 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | <= -30.00 dB | -300.0 | -300.0 | +270.000 | pass |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 250.0..1500.0 Hz | 2805.0 X | 2813.3 X | -1313.324 | FAIL |
| Width: L/R correlation above 200 Hz | 0.600..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.600 oct | 1.43 | 1.42 | +0.820 | pass |
| Move period vs Mod C Custom 1/1 = 1.6000 s (error) | -5.00..5.00 % | -0.154 | 0.028 | +4.846 | pass |
| Move period ACF peak height | >= 0.500 | 0.924 | 0.959 | +0.424 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 2.78 | 2.76 | - | info |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | -0.081 | -0.081 | +2.919 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -5.46 |  | +2.465 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -81.94 |  | +31.945 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -85.66 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.029 |  | +0.971 | pass |
| Macro 1 "Call Depth" sweep | >= 0.300 | 0.686 D oct |  | +0.386 | pass |
| Macro 2 "Response Gap" sweep | >= 0.300 | 1.73 D oct |  | +1.434 | pass |
| Macro 3 "Bite" sweep | >= 1.00 | 1.60 L LU |  | +0.600 | pass |
| Macro 4 "Grit" sweep | >= 1.00 | 3.33 L LU |  | +2.332 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.000 |  | +2.000 | pass |
| Log-mel distance to the nearest riddim preset | >= 3.30 | 3.91 dB |  | +0.607 | pass |

Details of failed or skipped checks:

- bright@150: F1 2604.3, G1 2729.7, A#1 2805.0; power-weighted centroid for comparison: F1 576.0, G1 618.8, A#1 651.7
- bright@140: F1 2615.4, G1 2728.3, A#1 2813.3; power-weighted centroid for comparison: F1 581.4, G1 624.8, A#1 661.2

## Dotted Metal Stab (riddim, Bass/riddim)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 511.0 chars |  | +431.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -7.99 | -7.31 | +4.309 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -19.18 X | -19.18 X | -0.683 | FAIL |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -17.00 | -17.00 | +2.501 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 11.19 | 11.87 | +1.128 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -1.18 | -1.18 | +0.817 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 0.999 | 0.999 | +0.009 | pass |
| Low-band side/mid, dB | <= -30.00 dB | -34.79 | -35.10 | +4.786 | pass |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.001 | -0.001 | +0.999 | pass |
| Brightness (power centroid) | 250.0..1500.0 Hz | 1773.0 X | 1818.5 X | -318.459 | FAIL |
| Width: L/R correlation above 200 Hz | 0.600..1.00 | 0.973 | 0.970 | +0.027 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.600 oct | 0.719 | 0.738 | +0.119 | pass |
| Move period vs Mod C SawDown 1/8D = 0.3000 s (error) | -5.00..5.00 % | -0.316 | -0.249 | +4.684 | pass |
| Move period ACF peak height | >= 0.500 | 0.942 | 0.947 | +0.442 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 1.22 | 1.08 | - | info |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | 0.149 | 0.127 | +2.851 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -7.99 |  | +4.992 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -61.61 |  | +11.611 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -84.30 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.486 |  | +0.514 | pass |
| Macro 1 "Stab Depth" sweep | >= 0.300 | 1.39 D oct |  | +1.091 | pass |
| Macro 2 "FM Mix" sweep | >= 0.300 | 1.03 C oct |  | +0.734 | pass |
| Macro 3 "Notch Sweep" sweep | >= 0.300 | 1.69 D oct |  | +1.385 | pass |
| Macro 4 "Width" sweep | >= 0.100 | 0.280 W corr |  | +0.180 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -1.02 |  | +0.976 | pass |
| Log-mel distance to the nearest riddim preset | >= 3.30 | 2.92 dB |  | -0.381 | FAIL |

Details of failed or skipped checks:

- rms@150: per note: F1 -19.21, G1 -19.01, A#1 -19.33
- rms@140: per note: F1 -19.22, G1 -19.01, A#1 -19.32
- bright@150: F1 1599.7, G1 1773.0, A#1 1770.2; power-weighted centroid for comparison: F1 439.4, G1 499.8, A#1 570.3
- bright@140: F1 1610.6, G1 1818.5, A#1 1775.5; power-weighted centroid for comparison: F1 440.2, G1 502.6, A#1 569.8
- pair_dist: nearest: Hollow Square Reference

## Hollow Square Reference (riddim, Bass/riddim)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 399.0 chars |  | +319.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -7.05 | -7.07 | +4.049 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -18.03 | -18.03 | +0.467 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.26 | -16.26 | +2.758 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 10.98 | 10.96 | +2.016 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -0.033 | -0.030 | +1.967 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | <= -30.00 dB | -300.0 | -300.0 | +270.000 | pass |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 250.0..1500.0 Hz | 1935.7 X | 1924.4 X | -435.705 | FAIL |
| Width: L/R correlation above 200 Hz | 0.600..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.600 oct | 1.10 | 1.10 | +0.498 | pass |
| Move period vs Mod B Square 1/8 = 0.2000 s (error) | -5.00..5.00 % | 0.559 | -0.394 | +4.441 | pass |
| Move period ACF peak height | >= 0.500 | 0.988 | 0.976 | +0.476 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.204 | 0.212 | - | info |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | 0.429 | 0.426 | +2.571 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -7.05 |  | +4.049 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -84.15 |  | +34.153 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -86.51 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.247 |  | +0.753 | pass |
| Macro 1 "Bark Depth" sweep | >= 0.300 | 0.656 D oct |  | +0.356 | pass |
| Macro 2 "Tone" sweep | >= 0.300 | 0.965 C oct |  | +0.665 | pass |
| Macro 3 "Grit" sweep | >= 0.300 | 1.16 C oct |  | +0.859 | pass |
| Macro 4 "Chatter" sweep | >= 0.300 | 0.868 D oct |  | +0.568 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.127 |  | +1.873 | pass |
| Log-mel distance to the nearest riddim preset | >= 3.30 | 2.92 dB |  | -0.381 | FAIL |

Details of failed or skipped checks:

- bright@150: F1 1878.0, G1 1929.2, A#1 1935.7; power-weighted centroid for comparison: F1 327.8, G1 346.6, A#1 393.4
- bright@140: F1 1874.2, G1 1922.9, A#1 1924.4; power-weighted centroid for comparison: F1 324.4, G1 343.0, A#1 390.3
- pair_dist: nearest: Dotted Metal Stab

## Init Square Bass (init, Init)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 234.0 chars |  | +154.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Brightness (power centroid) | 300.0..1000.0 Hz | 654.8 | 654.8 | +345.199 | pass |
| Width: L/R correlation above 200 Hz | 0.950..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Peak at velocity 100, dBFS (Init band -12..-6) | -12.00..-6.00 dBFS | -9.59 |  | +2.410 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -9.59 |  | +6.590 | pass |
| Alias floor (frozen modulation) | <= -70.00 dB | -85.95 |  | +15.948 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -85.95 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.000 |  | +1.000 | pass |
| No sample above 0.95 | <= 0.950 | 0.332 |  | +0.618 | pass |
| Note-on step vs 2x steady step | <= 1.00 | 0.359 |  | +0.641 | pass |
| Note-off step vs 2x steady step | <= 1.00 | 0.010 |  | +0.990 | pass |
| Macro 1 "Macro 1" sweep | - | n/a  |  | - | skip |
| Macro 2 "Macro 2" sweep | - | n/a  |  | - | skip |
| Macro 3 "Macro 3" sweep | - | n/a  |  | - | skip |
| Macro 4 "Macro 4" sweep | - | n/a  |  | - | skip |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | - dB | 0.000 |  | - | info |
| Log-mel distance to the nearest preset of the family | - | n/a  |  | - | skip |

Details of failed or skipped checks:

- macro1: Init macros are unassigned
- macro2: Init macros are unassigned
- macro3: Init macros are unassigned
- macro4: Init macros are unassigned
- pair_dist: no other preset of this family in the run

Note: Init: the bit-identical comparison with the SPEC 1.12 reference render is not implemented (no reference file in the repo).

## Pulse Width Pocket (riddim, Bass/riddim)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 535.0 chars |  | +455.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -7.86 | -7.87 | +4.856 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.38 | -17.37 | +1.125 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -15.17 | -15.17 | +1.667 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 9.52 | 9.50 | +3.483 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.625 | 0.630 | +1.370 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 0.999 | 0.999 | +0.009 | pass |
| Low-band side/mid, dB | <= -30.00 dB | -33.90 | -33.93 | +3.899 | pass |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.002 | -0.002 | +0.998 | pass |
| Brightness (power centroid) | 250.0..1500.0 Hz | 1628.1 X | 1623.3 X | -128.055 | FAIL |
| Width: L/R correlation above 200 Hz | 0.600..1.00 | 0.996 | 0.996 | +0.004 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.600 oct | 0.619 | 0.632 | +0.019 | pass |
| Move period vs Mod C Triangle 1/4T = 0.2667 s (error) | -5.00..5.00 % | 1.07 | -0.444 | +3.925 | pass |
| Move period ACF peak height | >= 0.500 | 0.969 | 0.953 | +0.453 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.762 | 0.831 | - | info |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | 0.466 | 0.470 | +2.530 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -7.86 |  | +4.856 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -86.94 |  | +36.941 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -87.02 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.274 |  | +0.726 | pass |
| Macro 1 "Pocket Depth" sweep | >= 0.300 | 0.492 D oct |  | +0.192 | pass |
| Macro 2 "Hollowness" sweep | >= 0.300 | 0.544 D oct |  | +0.244 | pass |
| Macro 3 "Flicker" sweep | >= 0.300 | 0.619 D oct |  | +0.319 | pass |
| Macro 4 "Air" sweep | >= 0.300 | 0.999 C oct |  | +0.699 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.784 |  | +1.216 | pass |
| Log-mel distance to the nearest riddim preset | >= 3.30 | 3.56 dB |  | +0.263 | pass |

Details of failed or skipped checks:

- bright@150: F1 1536.1, G1 1603.5, A#1 1628.1; power-weighted centroid for comparison: F1 337.7, G1 372.6, A#1 411.3
- bright@140: F1 1536.3, G1 1604.8, A#1 1623.3; power-weighted centroid for comparison: F1 338.1, G1 373.0, A#1 411.3

## Robot Bark Drop (riddim, Bass/riddim)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 468.0 chars |  | +388.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -7.56 | -7.60 | +4.560 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -18.64 X | -18.64 X | -0.142 | FAIL |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -17.37 | -17.37 | +2.130 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 11.07 | 11.04 | +1.932 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -0.642 | -0.641 | +1.358 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | <= -30.00 dB | -300.0 | -300.0 | +270.000 | pass |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 250.0..1500.0 Hz | 1140.2 | 1153.2 | +346.794 | pass |
| Width: L/R correlation above 200 Hz | 0.600..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.600 oct | 0.640 | 0.639 | +0.039 | pass |
| Move period | - | skip | skip | - | skip |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.233 | 0.275 | - | info |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | -0.317 | -0.311 | +2.683 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -7.56 |  | +4.560 | pass |
| Pitch dive over 150 ms (expected 7 st +-1) | 6.00..8.00 | 7.22 st |  | +0.780 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -83.96 |  | +33.961 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -84.46 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.008 |  | +0.992 | pass |
| Macro 1 "Bite" sweep | >= 0.300 | 0.335 D oct |  | +0.035 | pass |
| Macro 2 "Dive" sweep | >= 0.300 | 0.313 D oct |  | +0.013 | pass |
| Macro 3 "Metal" sweep | >= 0.300 | 0.469 D oct |  | +0.169 | FAIL |
| Macro 4 "Drive" sweep | >= 0.300 | 1.19 C oct |  | +0.893 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.483 |  | +1.517 | pass |
| Log-mel distance to the nearest riddim preset | >= 3.30 | 4.89 dB |  | +1.593 | pass |

Details of failed or skipped checks:

- rms@150: per note: F1 -18.77, G1 -18.64, A#1 -18.52
- rms@140: per note: F1 -18.78, G1 -18.64, A#1 -18.51
- period_err@150: qa_period=none
- period_err@140: qa_period=none
- macro3: C 0.141 oct (min 0.3) [10.12 10.14 10.17 10.21 10.26]; D 0.469 oct (min 0.3) non-monotonic [0.640 0.902 1.11 1.06 1.01]; L 0.522 LU (min 1) non-monotonic [-17.59 -17.88 -18.10 -18.11 -17.90]; W 0.000 corr (min 0.1) [1.00 1.00 1.00 1.00 1.00]; no metric is both audible and monotonic

## Robotic Comb Chatter (riddim, Bass/riddim)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 508.0 chars |  | +428.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -6.51 | -6.20 | +3.201 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.56 | -17.54 | +0.936 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -15.43 | -15.40 | +1.903 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 11.06 | 11.34 | +1.656 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.436 | 0.455 | +1.545 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | <= -30.00 dB | -300.0 | -300.0 | +270.000 | pass |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 250.0..1500.0 Hz | 2765.0 X | 2845.5 X | -1345.544 | FAIL |
| Width: L/R correlation above 200 Hz | 0.600..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.600 oct | 0.699 | 0.672 | +0.072 | pass |
| Move period vs Mod B Square 1/16 = 0.1000 s (error) | -5.00..5.00 % | -0.485 | -0.282 | +4.515 | pass |
| Move period ACF peak height | >= 0.500 | 0.776 | 0.856 | +0.276 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 4.68 | 4.84 | - | info |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | -1.84 | -1.86 | +1.135 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -6.82 |  | +3.822 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -86.69 |  | +36.685 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -87.01 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.540 |  | +0.460 | pass |
| Macro 1 "Chatter" sweep | >= 0.300 | 1.68 D oct |  | +1.385 | pass |
| Macro 2 "Comb Tone" sweep | >= 1.00 | 3.56 L LU |  | +2.559 | pass |
| Macro 3 "Fold" sweep | >= 1.00 | 1.73 L LU |  | +0.726 | pass |
| Macro 4 "Square Gate" sweep | >= 0.300 | 0.966 D oct |  | +0.666 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.596 |  | +1.404 | pass |
| Log-mel distance to the nearest riddim preset | >= 3.30 | 2.84 dB |  | -0.461 | FAIL |

Details of failed or skipped checks:

- bright@150: F1 2718.6, G1 2765.0, A#1 2702.5; power-weighted centroid for comparison: F1 469.9, G1 422.3, A#1 383.3
- bright@140: F1 2746.1, G1 2845.5, A#1 2716.1; power-weighted centroid for comparison: F1 469.3, G1 424.7, A#1 381.6
- pair_dist: nearest: Triplet Bark AM

## Triplet Bark AM (riddim, Bass/riddim)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 497.0 chars |  | +417.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -8.20 | -8.58 | +5.197 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -18.97 X | -18.97 X | -0.475 | FAIL |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.73 | -16.73 | +2.768 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 10.31 | 10.38 | +2.616 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -0.975 | -0.967 | +1.025 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | <= -30.00 dB | -300.0 | -300.0 | +270.000 | pass |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 250.0..1500.0 Hz | 4889.8 X | 4888.9 X | -3389.805 | FAIL |
| Width: L/R correlation above 200 Hz | 0.600..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.600 oct | 0.630 | 0.614 | +0.014 | pass |
| Move period vs Mod A SawDown 1/8T = 0.1333 s (error) | -5.00..5.00 % | 1.06 | -1.79 | +3.210 | pass |
| Move period ACF peak height | >= 0.500 | 0.866 | 0.846 | +0.346 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.818 | 0.844 | - | info |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | 0.128 | 0.090 | +2.872 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -8.20 |  | +5.197 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -86.57 |  | +36.574 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -86.43 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.011 |  | +0.989 | pass |
| Macro 1 "Bark" sweep | >= 0.300 | 0.381 D oct |  | +0.081 | FAIL |
| Macro 2 "Metal" sweep | >= 0.300 | 0.400 D oct |  | +0.100 | FAIL |
| Macro 3 "Snap" sweep | >= 0.300 | 0.762 D oct |  | +0.462 | pass |
| Macro 4 "Drive" sweep | >= 1.00 | 1.27 L LU |  | +0.269 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.815 |  | +1.185 | pass |
| Log-mel distance to the nearest riddim preset | >= 3.30 | 2.84 dB |  | -0.461 | FAIL |

Details of failed or skipped checks:

- rms@150: per note: F1 -18.89, G1 -18.94, A#1 -19.10
- rms@140: per note: F1 -18.88, G1 -18.93, A#1 -19.09
- bright@150: F1 4599.6, G1 4679.6, A#1 4889.8; power-weighted centroid for comparison: F1 510.2, G1 546.2, A#1 615.9
- bright@140: F1 4623.6, G1 4628.5, A#1 4888.9; power-weighted centroid for comparison: F1 511.3, G1 541.8, A#1 616.2
- macro1: C 0.068 oct (min 0.3) [12.17 12.16 12.12 12.12 12.10]; D 0.381 oct (min 0.3) non-monotonic [0.780 0.659 0.708 0.988 1.04]; L 0.139 LU (min 1) [-16.74 -16.74 -16.72 -16.76 -16.86]; W 0.000 corr (min 0.1) [1.00 1.00 1.00 1.00 1.00]; no metric is both audible and monotonic
- macro2: C 0.135 oct (min 0.3) [12.17 12.03 12.06 12.10 12.11]; D 0.400 oct (min 0.3) non-monotonic [0.780 0.794 0.917 0.745 0.516]; L 1.20 LU (min 1) non-monotonic [-16.74 -17.46 -17.90 -17.94 -17.36]; W 0.000 corr (min 0.1) [1.00 1.00 1.00 1.00 1.00]; no metric is both audible and monotonic
- pair_dist: nearest: Robotic Comb Chatter

## Distinctiveness (log-mel distance, dB; threshold 3.3 dB)

Same-family pairs are judged; other pairs are listed for calibration.

| A | B | Same family | Distance |
|---|---|---|---|
| Robotic Comb Chatter | Triplet Bark AM | yes | 2.84 |
| Dotted Metal Stab | Hollow Square Reference | yes | 2.92 |
| Dotted Metal Stab | Pulse Width Pocket | yes | 3.56 |
| Hollow Square Reference | Pulse Width Pocket | yes | 3.88 |
| Call and Response Square | Triplet Bark AM | yes | 3.91 |
| Call and Response Square | Robotic Comb Chatter | yes | 4.36 |
| Init Square Bass | Robot Bark Drop | no | 4.64 |
| Pulse Width Pocket | Robot Bark Drop | yes | 4.89 |
| Call and Response Square | Hollow Square Reference | yes | 4.99 |
| Call and Response Square | Pulse Width Pocket | yes | 5.41 |
| Hollow Square Reference | Robotic Comb Chatter | yes | 5.89 |
| Dotted Metal Stab | Robot Bark Drop | yes | 5.89 |
| Hollow Square Reference | Triplet Bark AM | yes | 6.04 |
| Call and Response Square | Dotted Metal Stab | yes | 6.11 |
| Dotted Metal Stab | Init Square Bass | no | 6.77 |
| Init Square Bass | Pulse Width Pocket | no | 7.00 |
| Hollow Square Reference | Robot Bark Drop | yes | 7.02 |
| Dotted Metal Stab | Robotic Comb Chatter | yes | 7.04 |
| Pulse Width Pocket | Robotic Comb Chatter | yes | 7.09 |
| Dotted Metal Stab | Triplet Bark AM | yes | 7.38 |
| Pulse Width Pocket | Triplet Bark AM | yes | 7.49 |
| Hollow Square Reference | Init Square Bass | no | 8.11 |
| Call and Response Square | Robot Bark Drop | yes | 9.85 |
| Robot Bark Drop | Robotic Comb Chatter | yes | 11.33 |
| Call and Response Square | Init Square Bass | no | 11.38 |
| Robot Bark Drop | Triplet Bark AM | yes | 11.97 |
| Init Square Bass | Robotic Comb Chatter | no | 12.05 |
| Init Square Bass | Triplet Bark AM | no | 12.73 |

## Bank summary

8 preset(s) analysed; 7 use the FX chain; 1 use a reordered chain (the sheet asks 20 of 41 and 6 of 41 for the whole bank). Failed checks: 25.
