# Bank QA report

Tool `bank_qa` (GTS), contract `docs/design/bank-recipes.md` section A. Renders at 150 and 140 BPM, 48 kHz, velocity 100 (127 for the peak check). Tags that steer the tool are listed in the source header of `crates/gts/examples/bank_qa.rs`.

Inputs: `C:\Users\kolby\Claude Code Main Branch\riddim-rack\crates\gts\presets/factory`

## Summary

| Preset | Family | Checks | Fail | Info/Skip | Result |
|---|---|---|---|---|---|
| Growl Hound | tearout (tags) | 42 | 13 | 6 | FAIL |
| Reese Drift | reese (tags) | 43 | 10 | 6 | FAIL |
| Sub Weight | sub (tags) | 40 | 6 | 5 | FAIL |
| Talk Box | formant (tags) | 41 | 2 | 6 | FAIL |
| Wobble Shot | wobble (tags) | 45 | 7 | 4 | FAIL |
| Screech Lead | lead (category) | 31 | 7 | 3 | FAIL |

## Growl Hound (tearout, Bass/growl)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 1195.0 chars |  | +1115.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -6.10 | -6.15 | +3.100 | pass |
| RMS over the hold, dBFS | -20.00..-16.00 dBFS | -16.76 | -16.76 | +0.764 | pass |
| LUFS-I over the hold | -20.00..-14.00 LUFS | -15.92 | -15.92 | +1.915 | pass |
| Crest (peak - RMS) | 5.00..10.00 dB | 10.62 X | 10.62 X | -0.616 | FAIL |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 1.24 | 1.24 | +0.764 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.950 | 0.996 | 0.996 | +0.046 | pass |
| Low-band side/mid, dB | - dB | -28.10 | -28.10 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.008 | -0.008 | +0.992 | pass |
| Brightness (power centroid) | 600.0..3500.0 Hz | 154.5 X | 154.5 X | -445.504 | FAIL |
| Width: L/R correlation above 200 Hz | 0.200..0.800 | 0.893 X | 0.893 X | -0.093 | FAIL |
| Move depth (p95-p5 of log2 centroid) | >= 1.00 oct | 0.843 X | 0.843 X | -0.157 | FAIL |
| Move period vs Mod B Sine 0.3 Hz = 3.3333 s (error) | -5.00..5.00 % | -99.03 X | -99.03 X | -94.030 | FAIL |
| Move period ACF peak height | >= 0.500 | -0.022 X | -0.021 X | -0.522 | FAIL |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 4.21 | 4.21 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -5.89 |  | +2.893 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -26.07 X |  | -23.927 | FAIL |
| Alias floor, frozen and FX off (Info) | - dB | -29.00 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.176 |  | +0.824 | pass |
| Clean sub: 40-90 Hz RMS vs the sub oscillator alone | -2.00..2.00 dB | -1.23 |  | +0.769 | pass |
| Macro 1 "Growl" sweep | >= 0.300 | 0.720 C oct |  | +0.420 | pass |
| Macro 2 "Dirt" sweep | >= 1.00 | 1.20 L LU |  | +0.197 | pass |
| Macro 3 "Width" sweep | >= 0.100 | 0.189 W corr |  | +0.089 | pass |
| Macro 4 "Space" sweep | >= 0.100 | 0.177 W corr |  | +0.077 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.104 |  | +1.896 | pass |
| Log-mel distance to the nearest preset of the family | - | n/a  |  | - | skip |

Details of failed or skipped checks:

- bright@150: F1 154.5, G1 165.8, A#1 189.7; amplitude-weighted centroid for comparison: F1 602.5, G1 663.9, A#1 723.0
- bright@140: F1 154.5, G1 165.8, A#1 189.7; amplitude-weighted centroid for comparison: F1 602.5, G1 663.9, A#1 722.9
- width@150: F1 0.893, G1 0.848, A#1 0.808
- width@140: F1 0.893, G1 0.847, A#1 0.808
- depth@150: F1 0.843, G1 1.08, A#1 0.991
- depth@140: F1 0.843, G1 1.08, A#1 0.991
- period_err@150: F1 -98.63, G1 -97.04, A#1 -99.03 (measured 0.0323 s)
- period_err@140: F1 -98.63, G1 -97.04, A#1 -99.03 (measured 0.0323 s)
- period_peak@150: F1 0.003, G1 0.186, A#1 -0.022
- period_peak@140: F1 0.003, G1 0.186, A#1 -0.021
- alias@150: F1 frozen render, grid 43.65 Hz, spread 8.0 ct
- pair_dist: no other preset of this family in the run

## Reese Drift (reese, Bass/reese)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 1037.0 chars |  | +957.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -5.43 | -5.96 | +2.431 | pass |
| RMS over the hold, dBFS | -20.00..-16.00 dBFS | -16.06 | -16.06 | +0.059 | pass |
| LUFS-I over the hold | -21.00..-15.00 LUFS | -15.91 | -15.91 | +0.905 | pass |
| Crest (peak - RMS) | 5.00..10.00 dB | 9.95 | 9.95 | +0.051 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 1.94 | 1.94 | +0.059 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.950 | 0.970 | 0.970 | +0.020 | pass |
| Low-band side/mid, dB | - dB | -25.07 | -25.07 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.068 | -0.068 | +0.932 | pass |
| Brightness (power centroid) | 180.0..2200.0 Hz | 82.29 X | 82.29 X | -97.711 | FAIL |
| Width: L/R correlation above 200 Hz | 0.200..0.900 | 0.382 | 0.382 | +0.182 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.400 oct | 0.822 | 0.822 | +0.422 | pass |
| Move period vs Mod B Sine 0.2 Hz = 5.0000 s (error) | -5.00..5.00 % | -99.40 X | -99.40 X | -94.396 | FAIL |
| Move period ACF peak height | >= 0.500 | 0.083 X | 0.083 X | -0.417 | FAIL |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 1.71 | 1.71 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -5.43 |  | +2.431 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -30.44 X |  | -19.556 | FAIL |
| Alias floor, frozen and FX off (Info) | - dB | -28.93 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.029 |  | +0.971 | pass |
| Reese beat rate at f0 vs expected (6 unison voices, outer +-22 ct), error | -10.00..10.00 % | -34.28 X |  | -24.281 | FAIL |
| Beating depth, 100-300 Hz envelope (p95-p5) | >= 3.00 dB | 10.45 |  | +7.454 | pass |
| Macro 1 "Detune" sweep | >= 1.00 | 0.762 L LU |  | -0.238 | FAIL |
| Macro 2 "Sweep" sweep | >= 0.300 | 1.08 C oct |  | +0.783 | pass |
| Macro 3 "Width" sweep | >= 0.100 | 0.341 W corr |  | +0.241 | pass |
| Macro 4 "Grit" sweep | >= 0.100 | 0.095 W corr |  | -0.005 | FAIL |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.601 |  | +1.399 | pass |
| Log-mel distance to the nearest preset of the family | - | n/a  |  | - | skip |

Details of failed or skipped checks:

- bright@150: F1 82.29, G1 90.21, A#1 100.7; amplitude-weighted centroid for comparison: F1 336.7, G1 370.4, A#1 436.9
- bright@140: F1 82.29, G1 90.21, A#1 100.7; amplitude-weighted centroid for comparison: F1 336.7, G1 370.4, A#1 436.9
- period_err@150: F1 -99.38, G1 -99.38, A#1 -99.40 (measured 0.0302 s)
- period_err@140: F1 -99.38, G1 -99.38, A#1 -99.40 (measured 0.0302 s)
- period_peak@150: F1 0.083, G1 0.101, A#1 0.375
- period_peak@140: F1 0.083, G1 0.101, A#1 0.375
- alias@150: F1 frozen render, grid 43.65 Hz, spread 29.0 ct
- beat_err@150: F1: measured 0.200 Hz, expected 0.222 Hz; A#1: measured 0.195 Hz, expected 0.296 Hz
- macro1: C 0.031 oct (min 0.3) [6.36 6.37 6.39 6.36 6.37]; D 0.102 oct (min 0.3) non-monotonic [0.822 0.758 0.720 0.722 0.781]; L 0.762 LU (min 1) [-16.50 -16.58 -16.67 -17.23 -17.26]; W 0.028 corr (min 0.1) non-monotonic [0.403 0.397 0.419 0.392 0.398]; no metric is both audible and monotonic
- macro4: C 0.236 oct (min 0.3) [6.36 6.37 6.41 6.49 6.60]; D 0.155 oct (min 0.3) [0.822 0.826 0.845 0.915 0.977]; L 0.253 LU (min 1) [-16.50 -16.67 -16.75 -16.71 -16.59]; W 0.095 corr (min 0.1) [0.403 0.394 0.375 0.344 0.307]; no metric is both audible and monotonic
- pair_dist: no other preset of this family in the run

## Sub Weight (sub, Bass/sub)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 870.0 chars |  | +790.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -6.83 | -6.80 | +3.802 | pass |
| RMS over the hold, dBFS | -20.00..-16.00 dBFS | -14.71 X | -14.71 X | -1.292 | FAIL |
| LUFS-I over the hold | -21.00..-15.00 LUFS | -15.45 | -15.45 | +0.451 | pass |
| Crest (peak - RMS) | 3.00..7.00 dB | 6.82 | 6.82 | +0.182 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 3.29 X | 3.29 X | -1.292 | FAIL |
| Low-band (<120 Hz) L/R correlation | >= 0.995 | 1.00 | 1.00 | +0.005 | pass |
| Low-band side/mid, dB | - dB | -300.0 | -300.0 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 55.00..160.0 Hz | 57.50 | 57.50 | +2.504 | pass |
| Width: L/R correlation above 200 Hz | 0.950..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Static: Move depth | <= 0.250 oct | 0.299 X | 0.299 X | -0.049 | FAIL |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | -0.417 | -0.408 | +2.583 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -6.86 |  | +3.856 | pass |
| Alias floor (frozen modulation) | <= -60.00 dB | -87.02 |  | +27.024 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -86.99 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.000 |  | +1.000 | pass |
| Energy below 150 Hz (share) | >= 90.00 % | 95.74 |  | +5.739 | pass |
| THD below 150 Hz | - % | 48.44 |  | - | info |
| Macro 1 "Weight" sweep | >= 1.00 | 3.55 L LU |  | +2.546 | pass |
| Macro 2 "Warmth" sweep | >= 1.00 | 1.29 L LU |  | +0.290 | pass |
| Macro 3 "Bite" sweep | >= 0.300 | 0.728 C oct |  | +0.428 | pass |
| Macro 4 "Wub" sweep | >= 0.300 | 1.22 D oct |  | +0.919 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 1.95 |  | +0.048 | pass |
| Log-mel distance to the nearest preset of the family | - | n/a  |  | - | skip |

Details of failed or skipped checks:

- rms@150: per note: F1 -14.99, G1 -14.75, A#1 -14.40
- rms@140: per note: F1 -14.99, G1 -14.75, A#1 -14.41
- rms_cat_median@150: category median -18 dBFS
- rms_cat_median@140: category median -18 dBFS
- depth@150: F1 0.299, G1 0.205, A#1 0.157
- depth@140: F1 0.299, G1 0.205, A#1 0.157
- pair_dist: no other preset of this family in the run

## Talk Box (formant, Bass/formant)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 931.0 chars |  | +851.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -5.89 | -6.09 | +2.886 | pass |
| RMS over the hold, dBFS | -20.00..-16.00 dBFS | -17.64 | -17.63 | +1.629 | pass |
| LUFS-I over the hold | -21.00..-15.00 LUFS | -16.08 | -16.07 | +1.065 | pass |
| Crest (peak - RMS) | 6.00..12.00 dB | 11.75 | 11.54 | +0.248 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.361 | 0.371 | +1.629 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.000 | 1.000 | +0.010 | pass |
| Low-band side/mid, dB | - dB | -64.94 | -65.62 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.000 | -0.000 | +1.000 | pass |
| Brightness (power centroid) | 350.0..2200.0 Hz | 472.2 | 469.1 | +119.073 | pass |
| Width: L/R correlation above 200 Hz | 0.850..1.00 | 0.954 | 0.954 | +0.046 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 1.00 oct | 0.993 X | 0.999 X | -0.007 | FAIL |
| Move period vs Mod A Triangle 1/4 = 0.4000 s (error) | -5.00..5.00 % | 0.027 | -0.059 | +4.941 | pass |
| Move period ACF peak height | >= 0.500 | 0.996 | 0.991 | +0.491 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.359 | 0.353 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -5.92 |  | +2.920 | pass |
| Alias floor (frozen modulation) | <= -55.00 dB | -84.22 |  | +29.221 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -87.51 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.888 |  | +0.112 | pass |
| Macro 1 "Talk" sweep | >= 0.300 | 1.68 D oct |  | +1.380 | pass |
| Macro 2 "Vowel" sweep | >= 0.300 | 0.302 C oct |  | +0.002 | pass |
| Macro 3 "Nasal" sweep | >= 0.300 | 0.928 D oct |  | +0.628 | pass |
| Macro 4 "Dirt" sweep | >= 0.300 | 0.459 C oct |  | +0.159 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.978 |  | +1.022 | pass |
| Log-mel distance to the nearest preset of the family | - | n/a  |  | - | skip |

Details of failed or skipped checks:

- depth@150: F1 1.07, G1 1.03, A#1 0.993
- depth@140: F1 1.07, G1 1.02, A#1 0.999
- pair_dist: no other preset of this family in the run

## Wobble Shot (wobble, Bass/wobble)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 1015.0 chars |  | +935.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -4.95 | -5.12 | +1.946 | pass |
| RMS over the hold, dBFS | -20.00..-16.00 dBFS | -16.66 | -16.68 | +0.660 | pass |
| LUFS-I over the hold | -21.00..-15.00 LUFS | -15.85 | -15.87 | +0.855 | pass |
| Crest (peak - RMS) | 5.00..11.00 dB | 11.71 X | 11.55 X | -0.714 | FAIL |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 1.34 | 1.32 | +0.660 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 0.999 | 0.999 | +0.009 | pass |
| Low-band side/mid, dB | - dB | -36.55 | -36.73 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.003 | -0.003 | +0.997 | pass |
| Brightness (power centroid) | 180.0..1600.0 Hz | 106.3 X | 105.3 X | -74.655 | FAIL |
| Width: L/R correlation above 200 Hz | 0.700..1.00 | 0.943 | 0.944 | +0.056 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.800 oct | 1.31 | 1.30 | +0.502 | pass |
| Move period vs Mod B Sine 1/8T = 0.1333 s (error) | -5.00..5.00 % | 2.45 | -1.99 | +2.551 | pass |
| Move period ACF peak height | >= 0.500 | 0.891 | 0.961 | +0.391 | pass |
| Sub (<90 Hz) RMS variation over the cycle | <= 1.50 dB | 1.59 X | 1.87 X | -0.374 | FAIL |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | -0.717 | -0.686 | +2.283 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -4.93 |  | +1.933 | pass |
| Alias floor (frozen modulation) | <= -55.00 dB | -88.42 |  | +33.417 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -88.02 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.658 |  | +0.342 | pass |
| Macro 1 "Depth" sweep | >= 0.300 | 0.402 D oct |  | +0.102 | pass |
| Macro 2 "Squelch" sweep | >= 0.300 | 0.083 C oct |  | -0.217 | FAIL |
| Macro 3 "Bite" sweep | >= 1.00 | 1.26 L LU |  | +0.261 | pass |
| Macro 4 "Width" sweep | >= 0.100 | 0.450 W corr |  | +0.350 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.000 |  | +2.000 | pass |
| Log-mel distance to the nearest preset of the family | - | n/a  |  | - | skip |

Details of failed or skipped checks:

- bright@150: F1 106.3, G1 111.3, A#1 129.0; amplitude-weighted centroid for comparison: F1 428.8, G1 448.4, A#1 517.7
- bright@140: F1 105.3, G1 109.7, A#1 128.0; amplitude-weighted centroid for comparison: F1 419.7, G1 442.3, A#1 506.2
- sub_var@150: F1 1.59, G1 1.37, A#1 1.30
- sub_var@140: F1 1.87, G1 1.00, A#1 1.10
- macro2: C 0.083 oct (min 0.3) [6.73 6.76 6.79 6.80 6.81]; D 0.064 oct (min 0.3) [1.50 1.54 1.56 1.54 1.52]; L 0.229 LU (min 1) [-16.66 -16.58 -16.49 -16.43 -16.52]; W 0.009 corr (min 0.1) [0.905 0.905 0.906 0.909 0.913]; no metric is both audible and monotonic
- pair_dist: no other preset of this family in the run

## Screech Lead (lead, Lead)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 979.0 chars |  | +899.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -9.30 | -9.33 | +6.295 | pass |
| RMS over the hold, dBFS | -22.00..-18.00 dBFS | -21.93 | -21.76 | +0.065 | pass |
| LUFS-I over the hold | -22.00..-16.00 LUFS | -17.63 | -17.47 | +1.471 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 12.06 | 12.10 | +0.901 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -1.93 | -1.76 | +0.065 | pass |
| Brightness (power centroid) | 1500.0..4500.0 Hz | 1130.5 X | 1110.3 X | -389.689 | FAIL |
| Width: L/R correlation above 300 Hz | 0.100..0.700 | 0.796 X | 0.727 X | -0.096 | FAIL |
| Energy below 150 Hz vs the 300 Hz - 3 kHz band | <= -20.00 dB | -33.04 | -33.00 | +12.999 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -9.23 |  | +6.226 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -15.29 X |  | -34.707 | FAIL |
| Alias floor, frozen and FX off (Info) | - dB | -16.98 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.043 |  | +0.957 | pass |
| Macro 1 "Screech" sweep | >= 0.100 | 0.536 W corr |  | +0.436 | pass |
| Macro 2 "FM" sweep | >= 0.300 | 1.14 D oct |  | +0.842 | FAIL |
| Macro 3 "Edge" sweep | >= 0.100 | 0.087 W corr |  | -0.013 | FAIL |
| Macro 4 "Space" sweep | >= 0.100 | 0.301 W corr |  | +0.201 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | - dB | 0.000 |  | - | info |
| Log-mel distance to the nearest preset of the family | - | n/a  |  | - | skip |

Details of failed or skipped checks:

- bright@150: C3 1130.5, E3 1323.6, G4 2534.2; amplitude-weighted centroid for comparison: C3 2045.0, E3 2382.8, G4 3984.8
- bright@140: C3 1110.3, E3 1319.9, G4 2556.1; amplitude-weighted centroid for comparison: C3 2058.0, E3 2368.7, G4 3976.0
- width@150: C3 0.796, E3 0.696, G4 0.757
- width@140: C3 0.703, E3 0.719, G4 0.727
- alias@150: C3 frozen render, grid 130.81 Hz, spread 10.0 ct
- macro2: C 1.05 oct (min 0.3) non-monotonic [10.14 10.40 10.95 11.19 11.12]; D 1.14 oct (min 0.3) non-monotonic [1.56 1.86 1.58 0.716 0.846]; L 1.32 LU (min 1) non-monotonic [-19.62 -20.44 -20.41 -19.31 -19.12]; W 0.057 corr (min 0.1) non-monotonic [0.796 0.774 0.769 0.805 0.826]; no metric is both audible and monotonic
- macro3: C 0.141 oct (min 0.3) [10.14 10.16 10.19 10.21 10.28]; D 0.099 oct (min 0.3) [1.56 1.52 1.50 1.51 1.46]; L 0.496 LU (min 1) [-19.62 -19.69 -19.67 -19.66 -19.20]; W 0.087 corr (min 0.1) [0.796 0.780 0.760 0.737 0.709]; no metric is both audible and monotonic
- pair_dist: no other preset of this family in the run

## Distinctiveness (log-mel distance, dB; threshold 3.3 dB)

Same-family pairs are judged; other pairs are listed for calibration.

| A | B | Same family | Distance |
|---|---|---|---|
| Growl Hound | Reese Drift | no | 4.19 |
| Reese Drift | Wobble Shot | no | 4.94 |
| Growl Hound | Talk Box | no | 5.83 |
| Growl Hound | Wobble Shot | no | 7.21 |
| Reese Drift | Talk Box | no | 8.27 |
| Talk Box | Wobble Shot | no | 9.29 |
| Talk Box | Screech Lead | no | 9.84 |
| Growl Hound | Screech Lead | no | 11.46 |
| Reese Drift | Screech Lead | no | 13.27 |
| Wobble Shot | Screech Lead | no | 14.03 |
| Sub Weight | Wobble Shot | no | 16.11 |
| Reese Drift | Sub Weight | no | 16.37 |
| Growl Hound | Sub Weight | no | 20.11 |
| Sub Weight | Talk Box | no | 23.27 |
| Sub Weight | Screech Lead | no | 24.04 |

## Bank summary

6 preset(s) analysed; 6 use the FX chain; 0 use a reordered chain (the sheet asks 20 of 41 and 6 of 41 for the whole bank). Failed checks: 45.
