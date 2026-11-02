# Bank QA report

Tool `bank_qa` (GTS), contract `docs/design/bank-recipes.md` section A. Renders at 150 and 140 BPM, 48 kHz, velocity 100 (127 for the peak check). Tags that steer the tool are listed in the source header of `crates/gts/examples/bank_qa.rs`.

Inputs: `docs/audits/bank-staging/riddim-seq-init`

## Summary

| Preset | Family | Checks | Fail | Info/Skip | Result |
|---|---|---|---|---|---|
| Call and Response Square | riddim (tags) | 46 | 0 | 3 | PASS |
| Dotted Metal Stab | riddim (tags) | 46 | 0 | 3 | PASS |
| Gated Triplet Call | seq (category) | 40 | 0 | 2 | PASS |
| Hollow Square Reference | riddim (tags) | 46 | 0 | 3 | PASS |
| Init Square Bass | init (category) | 21 | 0 | 7 | PASS |
| Pulse Width Pocket | riddim (tags) | 46 | 0 | 3 | PASS |
| Robot Bark Drop | riddim (tags) | 43 | 0 | 5 | PASS |
| Robotic Comb Chatter | riddim (tags) | 46 | 0 | 3 | PASS |
| Sixteenth Stutter Response | seq (category) | 40 | 0 | 2 | PASS |
| Triplet Bark AM | riddim (tags) | 46 | 0 | 3 | PASS |

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
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -6.68 | -6.74 | +3.684 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.18 | -17.18 | +1.317 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.13 | -16.11 | +2.611 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 10.50 | 10.44 | +2.502 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.817 | 0.819 | +1.181 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | <= -30.00 dB | -300.0 | -300.0 | +270.000 | pass |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 250.0..1500.0 Hz | 611.5 | 613.1 | +361.505 | pass |
| Width: L/R correlation above 200 Hz | 0.600..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.600 oct | 1.25 | 1.21 | +0.615 | pass |
| Move period vs Mod C Custom 1/1 = 1.6000 s (error) | -5.00..5.00 % | -0.015 | -0.020 | +4.980 | pass |
| Move period ACF peak height | >= 0.500 | 0.985 | 0.986 | +0.485 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 2.47 | 2.46 | - | info |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | 0.043 | 0.037 | +2.957 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -6.86 |  | +3.860 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -85.07 |  | +35.074 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -85.61 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.123 |  | +0.877 | pass |
| Macro 1 "Call Depth" sweep | >= 0.300 | 0.967 C oct |  | +0.667 | pass |
| Macro 2 "Response Gap" sweep | >= 0.300 | 0.591 C oct |  | +0.291 | pass |
| Macro 3 "Bite" sweep | >= 1.00 | 3.78 L LU |  | +2.780 | pass |
| Macro 4 "Grit" sweep | >= 0.300 | 0.860 C oct |  | +0.560 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.001 |  | +1.999 | pass |
| Log-mel distance to the nearest riddim preset | >= 3.30 | 4.13 dB |  | +0.831 | pass |

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
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -5.88 | -6.04 | +2.883 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.48 | -17.47 | +1.021 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -15.43 | -15.43 | +1.929 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 11.60 | 11.43 | +1.404 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.521 | 0.526 | +1.474 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 0.999 | 0.999 | +0.009 | pass |
| Low-band side/mid, dB | <= -30.00 dB | -35.05 | -35.36 | +5.047 | pass |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.001 | -0.001 | +0.999 | pass |
| Brightness (power centroid) | 250.0..1500.0 Hz | 791.6 | 808.8 | +541.573 | pass |
| Width: L/R correlation above 200 Hz | 0.600..1.00 | 0.973 | 0.970 | +0.027 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.600 oct | 0.652 | 0.653 | +0.052 | pass |
| Move period vs Mod C SawDown 1/8D = 0.3000 s (error) | -5.00..5.00 % | -0.156 | -0.158 | +4.842 | pass |
| Move period ACF peak height | >= 0.500 | 0.917 | 0.923 | +0.417 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.909 | 0.766 | - | info |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | 0.085 | 0.065 | +2.915 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -5.88 |  | +2.883 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -61.55 |  | +11.547 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -83.88 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.653 |  | +0.347 | pass |
| Macro 1 "Stab Depth" sweep | >= 0.300 | 1.55 D oct |  | +1.254 | pass |
| Macro 2 "FM Mix" sweep | >= 0.300 | 1.37 C oct |  | +1.073 | pass |
| Macro 3 "Notch Sweep" sweep | >= 1.00 | 3.62 L LU |  | +2.616 | pass |
| Macro 4 "Width" sweep | >= 0.100 | 0.279 W corr |  | +0.179 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.296 |  | +1.704 | pass |
| Log-mel distance to the nearest riddim preset | >= 3.30 | 3.72 dB |  | +0.423 | pass |

## Gated Triplet Call (seq, Seq)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 469.0 chars |  | +389.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -5.17 | -5.14 | +2.140 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.39 | -17.40 | +1.104 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.51 | -16.41 | +2.909 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 12.22 | 12.26 | +0.744 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.612 | 0.604 | +1.388 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Brightness (power centroid) | 250.0..1400.0 Hz | 870.8 | 852.9 | +529.210 | pass |
| Width: L/R correlation above 200 Hz | 0.850..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Slot RMS pattern correlation to the steps | >= 0.900 | 0.953 | 0.961 | +0.053 | pass |
| Active slots above the slots at rest, dB | >= 10.00 dB | 19.53 | 19.19 | +9.192 | pass |
| Bar period of the gate pattern vs 1.6000 s (error) | -5.00..5.00 % | 0.035 | 0.024 | +4.965 | pass |
| Bar period ACF peak height | >= 0.500 | 0.996 | 0.996 | +0.496 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -5.17 |  | +2.172 | pass |
| Alias floor (frozen modulation) | <= -55.00 dB | -86.21 |  | +31.208 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -87.06 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.256 |  | +0.744 | pass |
| Macro 1 "Gap Fill" sweep | >= 0.300 | 1.47 D oct |  | +1.171 | pass |
| Macro 2 "Pattern Tone" sweep | >= 0.300 | 0.996 C oct |  | +0.696 | pass |
| Macro 3 "Answer Fade" sweep | >= 0.300 | 0.825 C oct |  | +0.525 | pass |
| Macro 4 "Smear" sweep | >= 1.00 | 1.23 L LU |  | +0.235 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | - dB | -0.435 |  | - | info |
| Log-mel distance to the nearest seq preset | >= 3.30 | 5.38 dB |  | +2.083 | pass |

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
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -7.00 | -6.98 | +3.984 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -16.96 | -16.95 | +1.541 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -15.44 | -15.43 | +1.933 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 9.96 | 9.97 | +3.031 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 1.04 | 1.05 | +0.953 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | <= -30.00 dB | -300.0 | -300.0 | +270.000 | pass |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 250.0..1500.0 Hz | 1343.6 | 1346.9 | +153.121 | pass |
| Width: L/R correlation above 200 Hz | 0.600..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.600 oct | 1.03 | 1.03 | +0.435 | pass |
| Move period vs Mod B Square 1/8 = 0.2000 s (error) | -5.00..5.00 % | 0.419 | -0.213 | +4.581 | pass |
| Move period ACF peak height | >= 0.500 | 0.995 | 0.983 | +0.483 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.294 | 0.251 | - | info |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | 0.462 | 0.457 | +2.538 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -7.00 |  | +3.995 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -84.48 |  | +34.484 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -86.78 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.306 |  | +0.694 | pass |
| Macro 1 "Bark Depth" sweep | >= 0.300 | 1.09 D oct |  | +0.792 | pass |
| Macro 2 "Tone" sweep | >= 0.300 | 1.11 C oct |  | +0.808 | pass |
| Macro 3 "Grit" sweep | >= 0.300 | 1.52 C oct |  | +1.222 | pass |
| Macro 4 "Chatter" sweep | >= 0.300 | 1.26 D oct |  | +0.958 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.225 |  | +1.775 | pass |
| Log-mel distance to the nearest riddim preset | >= 3.30 | 4.14 dB |  | +0.844 | pass |

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
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -8.74 | -8.78 | +5.741 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.18 | -17.18 | +1.317 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -15.17 | -15.17 | +1.674 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 8.44 | 8.40 | +2.405 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.817 | 0.819 | +1.181 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 0.999 | 0.999 | +0.009 | pass |
| Low-band side/mid, dB | <= -30.00 dB | -33.96 | -33.97 | +3.959 | pass |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.002 | -0.002 | +0.998 | pass |
| Brightness (power centroid) | 250.0..1500.0 Hz | 949.0 | 944.2 | +550.967 | pass |
| Width: L/R correlation above 200 Hz | 0.600..1.00 | 0.997 | 0.997 | +0.003 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.600 oct | 0.770 | 0.764 | +0.164 | pass |
| Move period vs Mod C Triangle 1/4T = 0.2667 s (error) | -5.00..5.00 % | 0.247 | -0.118 | +4.753 | pass |
| Move period ACF peak height | >= 0.500 | 0.992 | 0.990 | +0.490 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.990 | 0.994 | - | info |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | 0.553 | 0.548 | +2.447 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -8.74 |  | +5.741 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -86.97 |  | +36.972 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -87.02 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.462 |  | +0.538 | pass |
| Macro 1 "Pocket Depth" sweep | >= 0.300 | 0.386 D oct |  | +0.086 | pass |
| Macro 2 "Hollowness" sweep | >= 0.300 | 0.457 D oct |  | +0.157 | pass |
| Macro 3 "Flicker" sweep | >= 0.300 | 0.397 D oct |  | +0.097 | pass |
| Macro 4 "Air" sweep | >= 0.300 | 0.696 C oct |  | +0.396 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.000 |  | +2.000 | pass |
| Log-mel distance to the nearest riddim preset | >= 3.30 | 3.90 dB |  | +0.604 | pass |

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
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -6.56 | -6.60 | +3.560 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.64 | -17.64 | +0.858 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.37 | -16.37 | +2.868 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 11.07 | 11.04 | +1.932 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.358 | 0.359 | +1.641 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | <= -30.00 dB | -300.0 | -300.0 | +270.000 | pass |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 250.0..1500.0 Hz | 1140.2 | 1153.2 | +346.794 | pass |
| Width: L/R correlation above 200 Hz | 0.600..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.600 oct | 0.640 | 0.639 | +0.039 | pass |
| Move period | - | skip | skip | - | skip |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.233 | 0.275 | - | info |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | -0.317 | -0.311 | +2.683 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -6.56 |  | +3.560 | pass |
| Pitch dive over 150 ms (expected 7 st +-1) | 6.00..8.00 | 7.22 st |  | +0.780 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -83.96 |  | +33.960 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -84.46 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.008 |  | +0.992 | pass |
| Macro 1 "Bite" sweep | >= 0.300 | 0.335 D oct |  | +0.035 | pass |
| Macro 2 "Dive" sweep | >= 0.300 | 0.313 D oct |  | +0.013 | pass |
| Macro 3 "Metal" sweep | >= 0.300 | 1.57 C oct |  | +1.270 | pass |
| Macro 4 "Drive" sweep | >= 0.300 | 1.19 C oct |  | +0.893 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.459 |  | +1.541 | pass |
| Log-mel distance to the nearest riddim preset | >= 3.30 | 3.90 dB |  | +0.604 | pass |

Details of failed or skipped checks:

- period_err@150: qa_period=none
- period_err@140: qa_period=none

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
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -5.73 | -5.78 | +2.732 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -16.48 | -16.41 | +1.911 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -14.41 | -14.33 | +0.831 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 10.75 | 10.63 | +2.250 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 1.52 | 1.59 | +0.411 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | <= -30.00 dB | -300.0 | -300.0 | +270.000 | pass |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 250.0..1500.0 Hz | 1302.8 | 1310.2 | +189.844 | pass |
| Width: L/R correlation above 200 Hz | 0.600..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.600 oct | 1.47 | 1.49 | +0.870 | pass |
| Move period vs Mod C Square 1/16 = 0.1000 s (error) | -5.00..5.00 % | 0.138 | -0.084 | +4.862 | pass |
| Move period ACF peak height | >= 0.500 | 0.980 | 0.982 | +0.480 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 5.11 | 5.51 | - | info |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | -1.06 | -1.04 | +1.940 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -5.86 |  | +2.862 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -86.74 |  | +36.742 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -87.01 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.264 |  | +0.736 | pass |
| Macro 1 "Chatter" sweep | >= 1.00 | 2.15 L LU |  | +1.152 | pass |
| Macro 2 "Comb Tone" sweep | >= 1.00 | 4.15 L LU |  | +3.153 | pass |
| Macro 3 "Fold" sweep | >= 0.300 | 0.841 C oct |  | +0.541 | pass |
| Macro 4 "Square Gate" sweep | >= 0.300 | 0.646 D oct |  | +0.346 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.701 |  | +1.299 | pass |
| Log-mel distance to the nearest riddim preset | >= 3.30 | 4.14 dB |  | +0.844 | pass |

## Sixteenth Stutter Response (seq, Seq)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 495.0 chars |  | +415.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -3.74 | -3.79 | +0.740 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -16.52 | -16.50 | +1.982 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -15.18 | -15.19 | +1.681 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 12.78 | 12.72 | +0.223 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 1.48 | 1.50 | +0.504 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Brightness (power centroid) | 250.0..1400.0 Hz | 1142.6 | 1103.4 | +257.354 | pass |
| Width: L/R correlation above 200 Hz | 0.850..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Slot RMS pattern correlation to the steps | >= 0.900 | 0.986 | 0.988 | +0.086 | pass |
| Active slots above the slots at rest, dB | >= 10.00 dB | 12.71 | 13.31 | +2.715 | pass |
| Bar period of the gate pattern vs 1.6000 s (error) | -5.00..5.00 % | 0.027 | 0.031 | +4.969 | pass |
| Bar period ACF peak height | >= 0.500 | 0.990 | 0.991 | +0.490 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -3.74 |  | +0.740 | pass |
| Alias floor (frozen modulation) | <= -55.00 dB | -86.58 |  | +31.580 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -86.72 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.769 |  | +0.231 | pass |
| Macro 1 "Stutter Depth" sweep | >= 0.300 | 1.04 D oct |  | +0.735 | pass |
| Macro 2 "Metal" sweep | >= 1.00 | 1.66 L LU |  | +0.662 | pass |
| Macro 3 "Rest Length" sweep | >= 1.00 | 3.69 L LU |  | +2.693 | pass |
| Macro 4 "Shimmer" sweep | >= 1.00 | 1.14 L LU |  | +0.143 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | - dB | 0.435 |  | - | info |
| Log-mel distance to the nearest seq preset | >= 3.30 | 5.38 dB |  | +2.083 | pass |

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
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -5.73 | -5.98 | +2.734 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -18.04 | -18.04 | +0.460 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -15.95 | -15.94 | +2.443 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 12.31 | 11.76 | +0.694 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -0.040 | -0.035 | +1.960 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | <= -30.00 dB | -300.0 | -300.0 | +270.000 | pass |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 250.0..1500.0 Hz | 1206.7 | 1188.7 | +293.335 | pass |
| Width: L/R correlation above 200 Hz | 0.600..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.600 oct | 0.951 | 0.880 | +0.280 | pass |
| Move period vs Mod B SawDown 1/8T = 0.1333 s (error) | -5.00..5.00 % | 0.219 | 0.184 | +4.781 | pass |
| Move period ACF peak height | >= 0.500 | 0.946 | 0.973 | +0.446 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.937 | 1.26 | - | info |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | 0.191 | 0.181 | +2.809 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -5.73 |  | +2.734 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -84.90 |  | +34.898 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -85.13 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.091 |  | +0.909 | pass |
| Macro 1 "Bark" sweep | >= 0.300 | 0.754 D oct |  | +0.454 | pass |
| Macro 2 "Metal" sweep | >= 0.300 | 0.796 D oct |  | +0.496 | pass |
| Macro 3 "Snap" sweep | >= 0.300 | 1.10 C oct |  | +0.802 | pass |
| Macro 4 "Drive" sweep | >= 0.300 | 1.04 C oct |  | +0.737 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.857 |  | +1.143 | pass |
| Log-mel distance to the nearest riddim preset | >= 3.30 | 3.72 dB |  | +0.423 | pass |

## Distinctiveness (log-mel distance, dB; threshold 3.3 dB)

Same-family pairs are judged; other pairs are listed for calibration.

| A | B | Same family | Distance |
|---|---|---|---|
| Dotted Metal Stab | Triplet Bark AM | yes | 3.72 |
| Pulse Width Pocket | Robot Bark Drop | yes | 3.90 |
| Call and Response Square | Dotted Metal Stab | yes | 4.13 |
| Hollow Square Reference | Robotic Comb Chatter | yes | 4.14 |
| Robot Bark Drop | Robotic Comb Chatter | yes | 4.39 |
| Init Square Bass | Pulse Width Pocket | no | 4.85 |
| Init Square Bass | Robot Bark Drop | no | 4.88 |
| Gated Triplet Call | Sixteenth Stutter Response | yes | 5.38 |
| Robot Bark Drop | Triplet Bark AM | yes | 5.42 |
| Call and Response Square | Triplet Bark AM | yes | 5.45 |
| Pulse Width Pocket | Triplet Bark AM | yes | 5.52 |
| Dotted Metal Stab | Pulse Width Pocket | yes | 5.94 |
| Init Square Bass | Triplet Bark AM | no | 6.03 |
| Hollow Square Reference | Robot Bark Drop | yes | 6.06 |
| Hollow Square Reference | Init Square Bass | no | 6.10 |
| Call and Response Square | Pulse Width Pocket | yes | 6.24 |
| Hollow Square Reference | Triplet Bark AM | yes | 6.33 |
| Dotted Metal Stab | Init Square Bass | no | 6.47 |
| Call and Response Square | Init Square Bass | no | 6.47 |
| Init Square Bass | Robotic Comb Chatter | no | 6.79 |
| Robotic Comb Chatter | Triplet Bark AM | yes | 6.87 |
| Dotted Metal Stab | Robot Bark Drop | yes | 7.04 |
| Pulse Width Pocket | Robotic Comb Chatter | yes | 7.33 |
| Hollow Square Reference | Pulse Width Pocket | yes | 7.62 |
| Dotted Metal Stab | Hollow Square Reference | yes | 7.68 |
| Call and Response Square | Robot Bark Drop | yes | 8.37 |
| Dotted Metal Stab | Robotic Comb Chatter | yes | 8.77 |
| Call and Response Square | Hollow Square Reference | yes | 9.22 |
| Call and Response Square | Robotic Comb Chatter | yes | 10.66 |
| Dotted Metal Stab | Sixteenth Stutter Response | no | 13.13 |
| Call and Response Square | Sixteenth Stutter Response | no | 13.36 |
| Sixteenth Stutter Response | Triplet Bark AM | no | 13.44 |
| Hollow Square Reference | Sixteenth Stutter Response | no | 14.30 |
| Pulse Width Pocket | Sixteenth Stutter Response | no | 15.18 |
| Robotic Comb Chatter | Sixteenth Stutter Response | no | 15.80 |
| Call and Response Square | Gated Triplet Call | no | 15.87 |
| Robot Bark Drop | Sixteenth Stutter Response | no | 15.95 |
| Init Square Bass | Sixteenth Stutter Response | no | 16.05 |
| Dotted Metal Stab | Gated Triplet Call | no | 16.13 |
| Gated Triplet Call | Triplet Bark AM | no | 16.99 |
| Gated Triplet Call | Hollow Square Reference | no | 17.48 |
| Gated Triplet Call | Pulse Width Pocket | no | 18.20 |
| Gated Triplet Call | Init Square Bass | no | 18.78 |
| Gated Triplet Call | Robot Bark Drop | no | 19.38 |
| Gated Triplet Call | Robotic Comb Chatter | no | 19.46 |

## Bank summary

10 preset(s) analysed; 9 use the FX chain; 1 use a reordered chain (the sheet asks 20 of 41 and 6 of 41 for the whole bank). Failed checks: 0.
