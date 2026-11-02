# Bank QA report

Tool `bank_qa` (GTS), contract `docs/design/bank-recipes.md` section A. Renders at 150 and 140 BPM, 48 kHz, velocity 100 (127 for the peak check). Tags that steer the tool are listed in the source header of `crates/gts/examples/bank_qa.rs`.

Inputs: `docs/audits/bank-staging/leads-plucks-fx`, `docs/audits/bank-staging/riddim-seq-init`, `docs/audits/bank-staging/tearout-reese`, `docs/audits/bank-staging/wobble-formant-subs`

## Summary

| Preset | Family | Checks | Fail | Info/Skip | Result |
|---|---|---|---|---|---|
| Four Bar Downlifter | fx (category) | 40 | 3 | 2 | FAIL |
| Four Bar Riser | fx (category) | 40 | 5 | 2 | FAIL |
| Metal Ring Pluck | pluck (category) | 37 | 0 | 1 | PASS |
| Noise Sweep Band | fx (category) | 38 | 0 | 1 | PASS |
| Pitch Drop Sub Pluck | pluck (category) | 37 | 0 | 1 | PASS |
| Screech Dive One | lead (category) | 34 | 0 | 1 | PASS |
| Screech Formant Rise | lead (category) | 34 | 0 | 1 | PASS |
| Square Melody Lead | lead (category) | 33 | 0 | 1 | PASS |
| Sub Drop Impact | fx (category) | 35 | 0 | 1 | PASS |
| Wide Chord Stab | pluck (category) | 37 | 0 | 1 | PASS |
| Wide Hoover Stack | lead (category) | 33 | 0 | 2 | PASS |
| Call and Response Square | riddim (tags) | 46 | 0 | 3 | PASS |
| Dotted Metal Stab | riddim (tags) | 46 | 0 | 3 | PASS |
| Gated Triplet Call | seq (category) | 40 | 0 | 2 | PASS |
| Hollow Square Reference | riddim (tags) | 46 | 0 | 3 | PASS |
| Init Square Bass | init (category) | 21 | 0 | 7 | PASS |
| Pulse Width Pocket | riddim (tags) | 46 | 0 | 3 | PASS |
| Robot Bark Drop | riddim (tags) | 42 | 0 | 5 | PASS |
| Robotic Comb Chatter | riddim (tags) | 46 | 0 | 3 | PASS |
| Sixteenth Stutter Response | seq (category) | 40 | 0 | 2 | PASS |
| Triplet Bark AM | riddim (tags) | 46 | 0 | 3 | PASS |
| Liquid Dark Reese | reese (tags) | 44 | 0 | 5 | PASS |
| Metal Gnash Fold | tearout (tags) | 43 | 0 | 5 | PASS |
| Neuro Comb Fold | reese (tags) | 43 | 0 | 6 | PASS |
| Phased Grind | tearout (tags) | 43 | 0 | 5 | PASS |
| Pitch Rip Growl | tearout (tags) | 43 | 0 | 5 | PASS |
| Slow Sweep Reese | reese (tags) | 44 | 0 | 5 | PASS |
| Triplet Shredder | tearout (tags) | 43 | 0 | 5 | PASS |
| Vocal Growl Gnash | tearout (tags) | 43 | 0 | 5 | PASS |
| Wide Reese Chorus | reese (tags) | 44 | 0 | 5 | PASS |
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

## Four Bar Downlifter (fx, FX)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 1.00  |  | -1.000 | FAIL |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 344.0 chars |  | +264.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -7.17 | -7.21 | +4.173 | pass |
| RMS over the hold, dBFS | -26.00..-20.00 dBFS | -23.80 | -23.79 | +2.195 | pass |
| LUFS-I over the hold | -26.00..-20.00 LUFS | -19.37 X | -19.39 X | -0.630 | FAIL |
| Crest (peak - RMS) | 7.00..16.00 dB | 12.79 | 13.63 | +2.366 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -1.80 | -1.79 | +0.195 | pass |
| Energy below 150 Hz re peak (50 ms RMS) | <= -30.00 dB | -50.69 | -51.09 | +20.690 | pass |
| Brightness (power centroid, whole hold) | 800.0..5000.0 Hz | 1849.4 | 1845.9 | +1045.915 | pass |
| Width: L/R correlation above 200 Hz (level window) | 0.000..0.700 | 0.091 | 0.106 | +0.091 | pass |
| Centroid rise, first to last 10 % of the hold (must fall 2 oct) | <= -2.00 oct | -2.14 | -2.20 | +0.142 | pass |
| Share of 250 ms centroid steps falling | >= 0.900 | 0.920 | 0.926 | +0.020 | pass |
| Level change, last bar minus first bar (falls 6..14 dB) | -14.00..-6.00 dB | -6.85 | -7.23 | +0.846 | pass |
| Narrowing: correlation change last bar vs first (must rise by 0.15) | >= 0.150 | 0.471 | 0.492 | +0.321 | pass |
| Pitch trend start -> end of the hold (Info) | - | -33.95 st |  | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -7.17 |  | +4.173 | pass |
| Alias floor (frozen) | - dB | 7.33 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.174 |  | +0.826 | pass |
| Macro 1 "Fall Speed" sweep | >= 0.300 | 1.78 D oct |  | +1.478 | pass |
| Macro 2 "Noise Level" sweep | >= 1.00 | 1.39 L LU |  | +0.389 | pass |
| Macro 3 "Depth" sweep | >= 0.300 | 0.808 D oct |  | +0.508 | pass |
| Macro 4 "Reverb Tail" sweep | >= 0.100 | 0.198 W corr |  | +0.098 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.081 |  | +1.919 | pass |
| Log-mel distance to the nearest fx preset | >= 3.30 | 3.44 dB |  | +0.143 | pass |

Details of failed or skipped checks:

- meta_load: eq_cut_freq: 700 out of range 20..500, set to 500
- lufs@150: per note: C3 -19.37
- lufs@140: per note: C3 -19.39

## Four Bar Riser (fx, FX)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 1.00  |  | -1.000 | FAIL |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 337.0 chars |  | +257.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -6.82 | -7.04 | +3.815 | pass |
| RMS over the hold, dBFS | -26.00..-20.00 dBFS | -24.05 | -23.83 | +1.954 | pass |
| LUFS-I over the hold | -26.00..-20.00 LUFS | -20.40 | -19.96 X | -0.040 | FAIL |
| Crest (peak - RMS) | 7.00..16.00 dB | 13.39 | 13.44 | +2.556 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -2.05 X | -1.83 | -0.046 | FAIL |
| Energy below 150 Hz re peak (50 ms RMS) | <= -30.00 dB | -52.63 | -52.40 | +22.399 | pass |
| Brightness (power centroid, whole hold) | 1500.0..6000.0 Hz | 1911.3 | 1917.4 | +411.304 | pass |
| Width: L/R correlation above 200 Hz (level window) | 0.000..0.700 | 0.346 | 0.342 | +0.342 | pass |
| Centroid rise, first to last 10 % of the hold | >= 2.00 oct | 2.05 | 2.07 | +0.049 | pass |
| Share of 250 ms centroid steps rising | >= 0.900 | 0.880 X | 0.889 X | -0.020 | FAIL |
| Level rise, last bar minus first bar | 3.00..8.00 dB | 5.58 | 5.54 | +2.419 | pass |
| Widening: correlation change last bar vs first (must fall by 0.2; the sheet says "width rises") | <= -0.200 | -0.413 | -0.400 | +0.200 | pass |
| Pitch trend start -> end of the hold (Info) | - | 5.71 st |  | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -6.82 |  | +3.815 | pass |
| Alias floor (frozen) | - dB | 20.16 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.062 |  | +0.938 | pass |
| Macro 1 "Rise Width" sweep | >= 1.00 | 3.67 L LU |  | +2.672 | pass |
| Macro 2 "Noise Level" sweep | >= 0.300 | 0.685 D oct |  | +0.385 | pass |
| Macro 3 "Pitch Rise" sweep | >= 0.300 | 1.27 D oct |  | +0.967 | pass |
| Macro 4 "Reverb Tail" sweep | >= 0.100 | 0.448 W corr |  | +0.348 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.160 |  | +1.840 | pass |
| Log-mel distance to the nearest fx preset | >= 3.30 | 11.53 dB |  | +8.233 | pass |

Details of failed or skipped checks:

- meta_load: eq_cut_freq: 520 out of range 20..500, set to 500
- lufs@140: per note: C3 -19.96
- rms_cat_median@150: category median -22 dBFS

## Metal Ring Pluck (pluck, Pluck)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 294.0 chars |  | +214.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -3.17 | -3.73 | +0.172 | pass |
| RMS over the hold, dBFS | -24.00..-18.00 dBFS | -22.67 | -22.67 | +1.327 | pass |
| LUFS-I over the hold | -24.00..-18.00 LUFS | -18.66 | -18.66 | +0.657 | pass |
| Crest (peak - RMS) | 10.00..20.00 dB | 14.17 | 14.17 | +4.166 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -1.67 | -1.67 | +0.327 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.970 | 0.992 | 0.992 | +0.022 | pass |
| Brightness (power centroid) | 600.0..3500.0 Hz | 1625.1 | 1625.1 | +1025.075 | pass |
| Width: L/R correlation above 200 Hz | 0.300..1.00 | 0.812 | 0.812 | +0.188 | pass |
| Time to -20 dB (expected 180 ms +-30 %) | 126.0..234.0 ms | 176.5 | 176.5 | +50.535 | pass |
| Centroid fall from 10 ms to 150 ms | >= 1.00 oct | 1.04 | 1.04 | +0.036 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -3.04 |  | +0.037 | pass |
| Alias floor (frozen modulation) | <= -35.00 dB | -42.75 |  | +7.752 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -52.24 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.019 |  | +0.981 | pass |
| Macro 1 "Ring" sweep | >= 0.400 | 1.21 C oct |  | +0.809 | pass |
| Macro 2 "Decay" sweep | >= 1.00 | 8.47 L LU |  | +7.466 | pass |
| Macro 3 "Brightness" sweep | >= 0.300 | 1.64 C oct |  | +1.341 | pass |
| Macro 4 "Echo" sweep | >= 0.300 | 0.945 C oct |  | +0.645 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.138 |  | +1.862 | pass |
| Log-mel distance to the nearest pluck preset | >= 3.30 | 4.45 dB |  | +1.154 | pass |

## Noise Sweep Band (fx, FX)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 336.0 chars |  | +256.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -8.64 | -8.77 | +5.644 | pass |
| RMS over the hold, dBFS | -26.00..-20.00 dBFS | -23.97 | -23.94 | +2.033 | pass |
| LUFS-I over the hold | -26.00..-20.00 LUFS | -20.22 | -20.11 | +0.106 | pass |
| Crest (peak - RMS) | 7.00..16.00 dB | 15.32 | 15.17 | +0.677 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -1.97 | -1.94 | +0.033 | pass |
| Energy below 150 Hz re peak (50 ms RMS) | <= -30.00 dB | -54.22 | -55.65 | +24.216 | pass |
| Brightness (power centroid, whole hold) | 800.0..6000.0 Hz | 2459.5 | 2457.5 | +1657.471 | pass |
| Width: L/R correlation above 200 Hz (level window) | 0.000..0.600 | 0.118 | 0.118 | +0.118 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 1.50 oct | 1.69 | 1.69 | +0.187 | pass |
| Move period vs Mod B Triangle 2/1 = 3.2000 s (error) | -5.00..5.00 % | 1.11 | 0.240 | +3.888 | pass |
| Move period ACF peak height | >= 0.500 | 0.968 | 0.969 | +0.468 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -8.64 |  | +5.644 | pass |
| Alias floor (frozen) | - dB | 22.04 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.514 |  | +0.486 | pass |
| Macro 1 "Sweep Depth" sweep | >= 0.300 | 0.662 D oct |  | +0.362 | pass |
| Macro 2 "Centre" sweep | >= 0.300 | 2.08 C oct |  | +1.777 | pass |
| Macro 3 "Noise Colour" sweep | >= 0.500 | 0.594 C oct |  | +0.094 | pass |
| Macro 4 "Width" sweep | >= 0.100 | 0.137 W corr |  | +0.037 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.081 |  | +1.919 | pass |
| Log-mel distance to the nearest fx preset | >= 3.30 | 3.44 dB |  | +0.143 | pass |

## Pitch Drop Sub Pluck (pluck, Pluck)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 346.0 chars |  | +266.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -4.66 | -4.66 | +1.657 | pass |
| RMS over the hold, dBFS | -24.00..-18.00 dBFS | -21.27 | -21.27 | +2.731 | pass |
| LUFS-I over the hold | -24.00..-18.00 LUFS | -19.64 | -19.64 | +1.636 | pass |
| Crest (peak - RMS) | 10.00..20.00 dB | 10.45 | 10.45 | +0.450 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -0.269 | -0.269 | +1.731 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.970 | 1.00 | 1.00 | +0.030 | pass |
| Brightness (power centroid) | 150.0..3500.0 Hz | 317.2 | 317.2 | +167.212 | pass |
| Width: L/R correlation above 200 Hz | 0.300..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Time to -20 dB (expected 320 ms +-30 %) | 224.0..416.0 ms | 244.8 | 244.8 | +20.837 | pass |
| Centroid fall from 10 ms to 150 ms | >= 1.00 oct | 2.58 | 2.58 | +1.575 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -4.66 |  | +1.657 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -68.56 |  | +18.562 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -76.26 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.029 |  | +0.971 | pass |
| Macro 1 "Drop Extra" sweep | >= 1.00 | 1.24 L LU |  | +0.239 | pass |
| Macro 2 "Length" sweep | >= 1.00 | 7.51 L LU |  | +6.509 | pass |
| Macro 3 "Click" sweep | >= 0.300 | 4.50 C oct |  | +4.204 | pass |
| Macro 4 "Drive" sweep | >= 1.00 | 1.47 L LU |  | +0.467 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 1.26 |  | +0.735 | pass |
| Log-mel distance to the nearest pluck preset | >= 3.30 | 6.21 dB |  | +2.909 | pass |

## Screech Dive One (lead, Lead)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 347.0 chars |  | +267.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -8.26 | -8.22 | +5.225 | pass |
| RMS over the hold, dBFS | -22.00..-18.00 dBFS | -20.69 | -20.71 | +1.293 | pass |
| LUFS-I over the hold | -22.00..-16.00 LUFS | -16.24 | -16.25 | +0.236 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 12.21 | 12.15 | +0.792 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -0.689 | -0.707 | +1.293 | pass |
| Brightness (power centroid) | 1500.0..4500.0 Hz | 4073.8 | 4078.5 | +421.545 | pass |
| Width: L/R correlation above 300 Hz | 0.100..0.700 | 0.127 | 0.131 | +0.027 | pass |
| Energy below 150 Hz vs the 300 Hz - 3 kHz band | <= -20.00 dB | -20.78 | -20.68 | +0.680 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -8.26 |  | +5.258 | pass |
| Pitch dive over 90 ms (expected 12 st +-1) | 11.00..13.00 | 12.57 st |  | +0.431 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -61.92 |  | +11.921 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -79.95 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.242 |  | +0.758 | pass |
| Macro 1 "Dive" sweep | >= 0.300 | 0.784 C oct |  | +0.484 | pass |
| Macro 2 "Comb Sweep" sweep | >= 0.300 | 0.304 D oct |  | +0.004 | pass |
| Macro 3 "Screech" sweep | >= 0.300 | 0.393 C oct |  | +0.093 | pass |
| Macro 4 "Drive" sweep | >= 1.00 | 3.12 L LU |  | +2.120 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.552 |  | +1.448 | pass |
| Log-mel distance to the nearest lead preset | >= 3.30 | 3.61 dB |  | +0.312 | pass |

## Screech Formant Rise (lead, Lead)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 350.0 chars |  | +270.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -9.67 | -9.76 | +6.669 | pass |
| RMS over the hold, dBFS | -22.00..-18.00 dBFS | -19.41 | -19.45 | +1.411 | pass |
| LUFS-I over the hold | -22.00..-16.00 LUFS | -16.50 | -16.52 | +0.497 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 9.14 | 9.03 | +3.032 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.589 | 0.553 | +1.411 | pass |
| Brightness (power centroid) | 1500.0..4500.0 Hz | 3990.1 | 3980.2 | +509.903 | pass |
| Width: L/R correlation above 300 Hz | 0.100..0.700 | 0.640 | 0.645 | +0.055 | pass |
| Energy below 150 Hz vs the 300 Hz - 3 kHz band | <= -20.00 dB | -21.11 | -20.83 | +0.834 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -9.67 |  | +6.669 | pass |
| Pitch dive over 80 ms (expected -1.3 st +-1) | -2.30..-0.300 | -2.19 st |  | +0.106 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -52.44 |  | +2.440 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -88.43 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.168 |  | +0.832 | pass |
| Macro 1 "Rise" sweep | >= 0.600 | 0.674 D oct |  | +0.074 | pass |
| Macro 2 "Vowel Start" sweep | >= 0.300 | 0.700 C oct |  | +0.400 | pass |
| Macro 3 "Bend" sweep | >= 0.100 | 0.154 W corr |  | +0.054 | pass |
| Macro 4 "Grit" sweep | >= 1.00 | 4.38 L LU |  | +3.384 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.726 |  | +1.274 | pass |
| Log-mel distance to the nearest lead preset | >= 3.30 | 3.44 dB |  | +0.139 | pass |

## Square Melody Lead (lead, Lead)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 328.0 chars |  | +248.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -7.53 | -7.46 | +4.456 | pass |
| RMS over the hold, dBFS | -22.00..-18.00 dBFS | -21.00 | -20.80 | +0.999 | pass |
| LUFS-I over the hold | -22.00..-16.00 LUFS | -17.94 | -17.73 | +1.726 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 12.91 | 12.45 | +0.086 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -1.00 | -0.802 | +0.999 | pass |
| Brightness (power centroid) | 1500.0..4500.0 Hz | 3978.3 | 3885.4 | +521.679 | pass |
| Width: L/R correlation above 300 Hz | 0.600..0.950 | 0.896 | 0.888 | +0.054 | pass |
| Energy below 150 Hz vs the 300 Hz - 3 kHz band | <= -20.00 dB | -22.98 | -22.98 | +2.976 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -7.53 |  | +4.529 | pass |
| Alias floor (frozen modulation) | <= -60.00 dB | -65.65 |  | +5.654 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -72.23 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.049 |  | +0.951 | pass |
| Macro 1 "Brightness" sweep | >= 0.300 | 1.16 C oct |  | +0.860 | pass |
| Macro 2 "Vibrato" sweep | >= 0.300 | 0.325 C oct |  | +0.025 | pass |
| Macro 3 "Echo" sweep | >= 1.00 | 3.46 L LU |  | +2.460 | pass |
| Macro 4 "Space" sweep | >= 1.00 | 3.54 L LU |  | +2.541 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.864 |  | +1.136 | pass |
| Log-mel distance to the nearest lead preset | >= 3.30 | 3.40 dB |  | +0.102 | pass |

## Sub Drop Impact (fx, FX)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 283.0 chars |  | +203.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -10.23 | -10.23 | +7.231 | pass |
| RMS over the hold, dBFS | -26.00..-20.00 dBFS | -23.50 | -23.50 | +2.502 | pass |
| LUFS-I over the hold | -26.00..-20.00 LUFS | -21.98 | -21.98 | +1.981 | pass |
| Crest (peak - RMS) | 7.00..16.00 dB | 12.64 | 12.64 | +3.360 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -1.50 | -1.50 | +0.502 | pass |
| Low-band L/R correlation | >= 0.950 | 0.965 | 0.965 | +0.015 | pass |
| Brightness (power centroid, whole hold) | 60.00..400.0 Hz | 358.1 | 355.9 | +41.890 | pass |
| Width: L/R correlation above 200 Hz (level window) | 0.500..1.00 | 0.602 | 0.602 | +0.102 | pass |
| Level falls 20 dB within 1.5 s | <= 1.50 s | 0.710 | 0.710 | +0.790 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -10.23 |  | +7.231 | pass |
| Pitch dive over 450 ms (expected 36 st +-1) | 35.00..37.00 | 36.78 st |  | +0.215 | pass |
| Alias floor (frozen) | - dB | -25.51 |  | - | info |
| Frozen render within 1 dB RMS of live | -12.00..1.00 dB | -10.08 |  | +1.924 | pass |
| Macro 1 "Drop Depth" sweep | >= 0.300 | 0.316 C oct |  | +0.016 | pass |
| Macro 2 "Boom Length" sweep | >= 1.00 | 3.47 L LU |  | +2.470 | pass |
| Macro 3 "Noise Burst" sweep | >= 0.300 | 0.359 C oct |  | +0.059 | pass |
| Macro 4 "Tail" sweep | >= 0.100 | 0.591 W corr |  | +0.491 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.388 |  | +1.612 | pass |
| Log-mel distance to the nearest fx preset | >= 3.30 | 15.22 dB |  | +11.923 | pass |

## Wide Chord Stab (pluck, Pluck)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 315.0 chars |  | +235.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -5.29 | -4.63 | +1.628 | pass |
| RMS over the hold, dBFS | -24.00..-18.00 dBFS | -22.53 | -22.53 | +1.466 | pass |
| LUFS-I over the hold | -24.00..-18.00 LUFS | -19.99 | -19.99 | +1.991 | pass |
| Crest (peak - RMS) | 10.00..20.00 dB | 14.49 | 14.49 | +4.487 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -1.53 | -1.53 | +0.466 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.970 | 0.984 | 0.984 | +0.014 | pass |
| Brightness (power centroid) | 600.0..3500.0 Hz | 1485.4 | 1485.4 | +885.420 | pass |
| Width: L/R correlation above 200 Hz | 0.300..1.00 | 0.620 | 0.620 | +0.320 | pass |
| Time to -20 dB (expected 250 ms +-30 %) | 175.0..325.0 ms | 308.7 | 308.7 | +16.342 | pass |
| Centroid fall from 10 ms to 150 ms | >= 1.00 oct | 1.11 | 1.11 | +0.114 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -4.98 |  | +1.975 | pass |
| Alias floor (frozen modulation) | <= -10.00 dB | -16.60 |  | +6.601 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -16.49 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.000 |  | +1.000 | pass |
| Macro 1 "Snap" sweep | >= 0.300 | 0.458 C oct |  | +0.158 | pass |
| Macro 2 "Decay" sweep | >= 1.00 | 6.35 L LU |  | +5.350 | pass |
| Macro 3 "Width" sweep | >= 0.100 | 0.462 W corr |  | +0.362 | pass |
| Macro 4 "Space" sweep | >= 0.100 | 0.623 W corr |  | +0.523 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.000 |  | +2.000 | pass |
| Log-mel distance to the nearest pluck preset | >= 3.30 | 4.45 dB |  | +1.154 | pass |

## Wide Hoover Stack (lead, Lead)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 437.0 chars |  | +357.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -6.36 | -5.84 | +2.839 | pass |
| RMS over the hold, dBFS | -22.00..-18.00 dBFS | -19.58 | -19.64 | +1.585 | pass |
| LUFS-I over the hold | -22.00..-16.00 LUFS | -16.13 | -16.15 | +0.128 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 12.49 | 12.54 | +0.462 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.415 | 0.365 | +1.585 | pass |
| Brightness (power centroid) | 1500.0..4500.0 Hz | 3527.4 | 3558.1 | +941.905 | pass |
| Width: L/R correlation above 300 Hz | 0.100..0.700 | 0.484 | 0.474 | +0.216 | pass |
| Energy below 150 Hz vs the 300 Hz - 3 kHz band | <= -20.00 dB | -25.04 | -25.04 | +5.041 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -6.36 |  | +3.362 | pass |
| Pitch dive over 140 ms (expected 5 st +-1) | 4.00..6.00 | 4.68 st |  | +0.681 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -66.67 |  | +16.667 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -79.10 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.000 |  | +1.000 | pass |
| Macro 1 "Width" sweep | >= 0.100 | 0.353 W corr |  | +0.253 | pass |
| Macro 2 "Brightness" sweep | >= 0.300 | 0.651 C oct |  | +0.351 | pass |
| Macro 3 "Dive" sweep | >= 1.00 | 3.61 L LU |  | +2.612 | pass |
| Macro 4 "Chorus Depth" sweep | >= 0.300 | 0.406 D oct |  | +0.106 | pass |
| Repeatability (two renders, -80 dB) | - | n/a  |  | - | skip |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.552 |  | +1.448 | pass |
| Log-mel distance to the nearest lead preset | >= 3.30 | 3.40 dB |  | +0.102 | pass |

Details of failed or skipped checks:

- repeat: random LFO or free random phase: the engine seeds those from a global counter

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
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.000 |  | +2.000 | pass |
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
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.297 |  | +1.703 | pass |
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
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.224 |  | +1.776 | pass |
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
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.001 |  | +1.999 | pass |
| Log-mel distance to the nearest riddim preset | >= 3.30 | 3.96 dB |  | +0.656 | pass |

## Robot Bark Drop (riddim, Bass/riddim)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 518.0 chars |  | +438.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -6.60 | -6.71 | +3.597 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.64 | -17.64 | +0.860 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.35 | -16.36 | +2.852 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 11.04 | 10.60 | +1.959 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.363 | 0.360 | +1.637 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | <= -30.00 dB | -300.0 | -300.0 | +270.000 | pass |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 250.0..1500.0 Hz | 1138.7 | 1139.3 | +360.676 | pass |
| Width: L/R correlation above 200 Hz | 0.600..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.600 oct | 0.621 | 0.637 | +0.021 | pass |
| Move period | - | skip | skip | - | skip |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.253 | 0.225 | - | info |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | -0.320 | -0.336 | +2.664 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -6.60 |  | +3.597 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -83.96 |  | +33.955 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -84.46 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.016 |  | +0.984 | pass |
| Macro 1 "Bite" sweep | >= 0.300 | 1.67 C oct |  | +1.374 | pass |
| Macro 2 "Dive" sweep | >= 0.300 | 0.702 C oct |  | +0.402 | pass |
| Macro 3 "Metal" sweep | >= 0.300 | 1.56 C oct |  | +1.261 | pass |
| Macro 4 "Drive" sweep | >= 0.300 | 1.22 C oct |  | +0.919 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.455 |  | +1.545 | pass |
| Log-mel distance to the nearest riddim preset | >= 3.30 | 3.96 dB |  | +0.656 | pass |

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
| Description of at least 80 characters | >= 80.00 | 587.0 chars |  | +507.000 | pass |
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
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.858 |  | +1.142 | pass |
| Log-mel distance to the nearest riddim preset | >= 3.30 | 3.72 dB |  | +0.423 | pass |

## Liquid Dark Reese (reese, Bass/reese)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 547.0 chars |  | +467.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -8.51 | -8.74 | +5.514 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.03 | -17.06 | +1.438 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -18.02 | -18.04 | +1.463 | pass |
| Crest (peak - RMS) | 5.00..10.00 dB | 8.52 | 8.32 | +1.481 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.968 | 0.938 | +1.032 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.970 | 1.000 | 1.000 | +0.030 | pass |
| Low-band side/mid, dB | - dB | -39.29 | -39.24 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.001 | -0.001 | +0.999 | pass |
| Brightness (power centroid) | 180.0..2200.0 Hz | 346.8 | 347.1 | +166.845 | pass |
| Width: L/R correlation above 200 Hz | 0.200..0.900 | 0.807 | 0.807 | +0.093 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.400 oct | 1.21 | 1.21 | +0.808 | pass |
| Move period vs Mod B Triangle 2/1 = 3.2000 s (error) | -5.00..5.00 % | -0.802 | 3.92 | +1.081 | pass |
| Move period ACF peak height | >= 0.500 | 0.728 | 0.717 | +0.217 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.901 | 1.04 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -8.51 |  | +5.514 | pass |
| Alias floor (frozen modulation) | <= -24.00 dB | -24.84 |  | +0.840 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -15.76 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.013 |  | +0.987 | pass |
| Reese beat rate at f0 vs expected (qa_beat tag), error | -10.00..10.00 % | 4.97 |  | +5.027 | pass |
| Beating depth, 100-300 Hz envelope (p95-p5) | >= 3.00 dB | 6.43 |  | +3.435 | pass |
| Macro 1 "Beat" sweep | >= 0.005 | 0.009 W corr |  | +0.004 | pass |
| Macro 2 "Brightness" sweep | >= 0.500 | 2.92 C oct |  | +2.424 | pass |
| Macro 3 "Drift" sweep | >= 0.300 | 2.18 D oct |  | +1.883 | pass |
| Macro 4 "Warmth" sweep | >= 1.00 | 1.59 L LU |  | +0.586 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.150 |  | +1.850 | pass |
| Log-mel distance to the nearest reese preset | >= 3.30 | 4.86 dB |  | +1.556 | pass |

## Metal Gnash Fold (tearout, Bass/tearout)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 512.0 chars |  | +432.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -8.20 | -8.14 | +5.137 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.26 | -17.23 | +1.239 | pass |
| LUFS-I over the hold | -18.50..-12.50 LUFS | -17.99 | -17.97 | +0.512 | pass |
| Crest (peak - RMS) | 5.00..10.00 dB | 9.06 | 8.63 | +0.936 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.739 | 0.774 | +1.226 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.950 | 0.980 | 0.982 | +0.030 | pass |
| Low-band side/mid, dB | - dB | -20.75 | -21.07 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.043 | -0.040 | +0.957 | pass |
| Brightness (power centroid) | 600.0..3500.0 Hz | 1271.5 | 1264.7 | +664.726 | pass |
| Width: L/R correlation above 200 Hz | 0.200..0.800 | 0.785 | 0.789 | +0.011 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 1.00 oct | 1.53 | 1.61 | +0.528 | pass |
| Move period vs Mod A Triangle 1/8T = 0.1333 s (error) | -5.00..5.00 % | 2.95 | -2.97 | +2.035 | pass |
| Move period ACF peak height | >= 0.500 | 0.873 | 0.895 | +0.373 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 1.54 | 1.49 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -8.36 |  | +5.358 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -63.69 |  | +13.693 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -85.22 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.172 |  | +0.828 | pass |
| Clean sub: 40-90 Hz RMS vs the sub oscillator alone | -2.00..2.00 dB | -0.267 |  | +1.733 | pass |
| Macro 1 "Gnash" sweep | >= 0.600 | 0.909 D oct |  | +0.309 | pass |
| Macro 2 "Ring" sweep | >= 0.500 | 1.27 C oct |  | +0.770 | pass |
| Macro 3 "Fold" sweep | >= 0.400 | 1.99 C oct |  | +1.590 | pass |
| Macro 4 "Flange" sweep | >= 0.100 | 0.616 W corr |  | +0.516 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.078 |  | +1.922 | pass |
| Log-mel distance to the nearest tearout preset | >= 3.30 | 3.54 dB |  | +0.240 | pass |

## Neuro Comb Fold (reese, Bass/reese)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 579.0 chars |  | +499.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -6.76 | -7.13 | +3.757 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -16.46 | -16.51 | +1.959 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -17.27 | -17.28 | +2.222 | pass |
| Crest (peak - RMS) | 5.00..10.00 dB | 9.70 | 9.38 | +0.298 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 1.54 | 1.49 | +0.459 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.970 | 1.000 | 0.999 | +0.029 | pass |
| Low-band side/mid, dB | - dB | -36.15 | -35.21 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.001 | -0.001 | +0.999 | pass |
| Brightness (power centroid) | 180.0..2200.0 Hz | 725.3 | 706.4 | +526.432 | pass |
| Width: L/R correlation above 200 Hz | 0.200..0.900 | 0.837 | 0.830 | +0.063 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.800 oct | 1.92 | 2.17 | +1.117 | pass |
| Move period vs Mod B Triangle 1/8T = 0.1333 s (error) | -5.00..5.00 % | 1.64 | -2.49 | +2.509 | pass |
| Move period ACF peak height | >= 0.500 | 0.616 | 0.612 | +0.112 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 3.59 | 4.48 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -6.78 |  | +3.779 | pass |
| Alias floor (frozen modulation) | <= -21.00 dB | -21.45 |  | +0.446 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -11.69 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.149 |  | +0.851 | pass |
| Reese beat rate at f0 vs expected (oscillator offset 30.0 ct), error | -10.00..10.00 % | -1.38 |  | +8.620 | pass |
| Beating depth, 100-300 Hz envelope (p95-p5) | - dB | 7.85 |  | - | info |
| Macro 1 "Neuro Sweep" sweep | >= 0.600 | 1.37 D oct |  | +0.765 | pass |
| Macro 2 "Fold" sweep | >= 0.400 | 0.988 C oct |  | +0.588 | pass |
| Macro 3 "Comb" sweep | >= 0.400 | 1.92 C oct |  | +1.518 | pass |
| Macro 4 "Phase" sweep | >= 0.300 | 0.411 C oct |  | +0.111 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.724 |  | +1.276 | pass |
| Log-mel distance to the nearest reese preset | >= 3.30 | 4.34 dB |  | +1.043 | pass |

## Phased Grind (tearout, Bass/tearout)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 509.0 chars |  | +429.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -8.73 | -8.25 | +5.253 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -16.42 | -16.41 | +1.912 | pass |
| LUFS-I over the hold | -18.50..-12.50 LUFS | -17.82 | -17.81 | +0.676 | pass |
| Crest (peak - RMS) | 5.00..10.00 dB | 6.41 | 6.86 | +1.411 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 1.58 | 1.59 | +0.412 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.950 | 0.975 | 0.975 | +0.025 | pass |
| Low-band side/mid, dB | - dB | -18.95 | -19.10 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.070 | -0.067 | +0.930 | pass |
| Brightness (power centroid) | 600.0..3500.0 Hz | 885.3 | 876.9 | +276.895 | pass |
| Width: L/R correlation above 200 Hz | 0.200..0.800 | 0.299 | 0.291 | +0.091 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 1.00 oct | 1.39 | 1.41 | +0.394 | pass |
| Move period vs Mod A Triangle 1/16 = 0.1000 s (error) | -5.00..5.00 % | 0.453 | -0.841 | +4.159 | pass |
| Move period ACF peak height | >= 0.500 | 0.820 | 0.810 | +0.310 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 1.27 | 1.34 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -8.73 |  | +5.733 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -52.68 |  | +2.680 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -34.84 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.459 |  | +0.541 | pass |
| Clean sub: 40-90 Hz RMS vs the sub oscillator alone | -2.00..2.00 dB | 1.12 |  | +0.885 | pass |
| Macro 1 "Grind" sweep | >= 0.500 | 1.16 D oct |  | +0.662 | pass |
| Macro 2 "Phaser Depth" sweep | >= 0.400 | 0.794 C oct |  | +0.394 | pass |
| Macro 3 "Grit" sweep | >= 0.300 | 1.17 C oct |  | +0.870 | pass |
| Macro 4 "Width" sweep | >= 0.150 | 0.247 W corr |  | +0.097 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.764 |  | +1.236 | pass |
| Log-mel distance to the nearest tearout preset | >= 3.30 | 4.95 dB |  | +1.655 | pass |

## Pitch Rip Growl (tearout, Bass/tearout)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 474.0 chars |  | +394.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -7.71 | -7.21 | +4.212 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -16.35 | -16.32 | +1.822 | pass |
| LUFS-I over the hold | -18.50..-12.50 LUFS | -16.93 | -16.89 | +1.571 | pass |
| Crest (peak - RMS) | 5.00..10.00 dB | 7.87 | 8.52 | +1.484 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 1.65 | 1.68 | +0.322 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.950 | 0.996 | 0.996 | +0.046 | pass |
| Low-band side/mid, dB | - dB | -22.44 | -22.67 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.027 | -0.026 | +0.973 | pass |
| Brightness (power centroid) | 600.0..3500.0 Hz | 1429.2 | 1427.3 | +827.345 | pass |
| Width: L/R correlation above 200 Hz | 0.200..0.800 | 0.723 | 0.719 | +0.077 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 1.00 oct | 1.71 | 1.64 | +0.639 | pass |
| Move period vs Mod A Custom 1/4 = 0.4000 s (error) | -5.00..5.00 % | 0.097 | 0.276 | +4.724 | pass |
| Move period ACF peak height | >= 0.500 | 0.926 | 0.952 | +0.426 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 2.19 | 2.25 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -7.71 |  | +4.710 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -81.87 |  | +31.871 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -67.94 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.077 |  | +0.923 | pass |
| Clean sub: 40-90 Hz RMS vs the sub oscillator alone | -2.00..2.00 dB | -0.050 |  | +1.950 | pass |
| Macro 1 "Rip" sweep | >= 0.300 | 0.373 C oct |  | +0.073 | pass |
| Macro 2 "Warp" sweep | >= 0.500 | 1.37 C oct |  | +0.871 | pass |
| Macro 3 "Grit" sweep | >= 0.400 | 1.14 C oct |  | +0.744 | pass |
| Macro 4 "Flange" sweep | >= 0.100 | 0.394 W corr |  | +0.294 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.832 |  | +1.168 | pass |
| Log-mel distance to the nearest tearout preset | >= 3.30 | 3.54 dB |  | +0.240 | pass |

## Slow Sweep Reese (reese, Bass/reese)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 394.0 chars |  | +314.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -8.35 | -8.30 | +5.297 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -16.89 | -16.93 | +1.566 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -17.44 | -17.49 | +2.012 | pass |
| Crest (peak - RMS) | 5.00..10.00 dB | 8.08 | 8.20 | +1.801 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 1.11 | 1.07 | +0.894 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.970 | 0.999 | 0.998 | +0.028 | pass |
| Low-band side/mid, dB | - dB | -35.30 | -36.63 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.003 | -0.003 | +0.997 | pass |
| Brightness (power centroid) | 180.0..2200.0 Hz | 231.3 | 234.0 | +51.258 | pass |
| Width: L/R correlation above 200 Hz | 0.200..0.900 | 0.874 | 0.878 | +0.022 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.400 oct | 1.99 | 1.85 | +1.450 | pass |
| Move period vs Mod B Triangle 1/1 = 1.6000 s (error) | -5.00..5.00 % | 1.00 | 1.76 | +3.240 | pass |
| Move period ACF peak height | >= 0.500 | 0.807 | 0.819 | +0.307 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 7.31 | 7.21 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -8.35 |  | +5.347 | pass |
| Alias floor (frozen modulation) | <= -30.00 dB | -31.08 |  | +1.078 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -26.10 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.253 |  | +0.747 | pass |
| Reese beat rate at f0 vs expected (2 unison voices, outer +-12 ct), error | -10.00..10.00 % | -2.69 |  | +7.311 | pass |
| Beating depth, 100-300 Hz envelope (p95-p5) | >= 3.00 dB | 5.84 |  | +2.840 | pass |
| Macro 1 "Sweep Depth" sweep | >= 0.800 | 1.36 D oct |  | +0.563 | pass |
| Macro 2 "Base Cutoff" sweep | >= 0.700 | 3.49 C oct |  | +2.790 | pass |
| Macro 3 "Resonance" sweep | >= 0.300 | 0.345 C oct |  | +0.045 | pass |
| Macro 4 "Grit" sweep | >= 1.00 | 1.27 L LU |  | +0.269 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.289 |  | +1.711 | pass |
| Log-mel distance to the nearest reese preset | >= 3.30 | 6.88 dB |  | +3.578 | pass |

## Triplet Shredder (tearout, Bass/tearout)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 458.0 chars |  | +378.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -9.28 | -9.59 | +6.279 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -16.61 | -16.61 | +1.885 | pass |
| LUFS-I over the hold | -18.50..-12.50 LUFS | -17.89 | -17.88 | +0.611 | pass |
| Crest (peak - RMS) | 5.00..10.00 dB | 6.32 | 6.20 | +1.196 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 1.39 | 1.39 | +0.606 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.950 | 1.000 | 1.000 | +0.050 | pass |
| Low-band side/mid, dB | - dB | -50.09 | -49.65 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.000 | -0.000 | +1.000 | pass |
| Brightness (power centroid) | 600.0..3500.0 Hz | 1848.9 | 1786.3 | +1186.316 | pass |
| Width: L/R correlation above 200 Hz | 0.200..0.800 | 0.681 | 0.698 | +0.102 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.400 oct | 1.54 | 1.64 | +1.137 | pass |
| Move period vs Mod A SawUp 1/16T = 0.0667 s (error) | -5.00..5.00 % | -0.626 | -1.19 | +3.812 | pass |
| Move period ACF peak height | >= 0.400 | 0.890 | 0.897 | +0.490 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.558 | 0.671 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -9.28 |  | +6.279 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -86.43 |  | +36.427 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -78.20 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.129 |  | +0.871 | pass |
| Clean sub: 40-90 Hz RMS vs the sub oscillator alone | -2.00..2.00 dB | 0.714 |  | +1.286 | pass |
| Macro 1 "Shred" sweep | >= 0.500 | 0.798 D oct |  | +0.298 | pass |
| Macro 2 "Sync" sweep | >= 0.500 | 1.02 C oct |  | +0.517 | pass |
| Macro 3 "Drive" sweep | >= 0.400 | 0.850 C oct |  | +0.450 | pass |
| Macro 4 "Width" sweep | >= 0.150 | 0.567 W corr |  | +0.417 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.568 |  | +1.432 | pass |
| Log-mel distance to the nearest tearout preset | >= 3.30 | 4.66 dB |  | +1.358 | pass |

## Vocal Growl Gnash (tearout, Bass/tearout)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 576.0 chars |  | +496.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -8.22 | -8.44 | +5.215 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -16.25 | -16.25 | +1.750 | pass |
| LUFS-I over the hold | -18.50..-12.50 LUFS | -17.38 | -17.38 | +1.116 | pass |
| Crest (peak - RMS) | 5.00..10.00 dB | 7.59 | 7.81 | +2.192 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 1.75 | 1.75 | +0.250 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.950 | 0.997 | 0.997 | +0.047 | pass |
| Low-band side/mid, dB | - dB | -29.75 | -29.68 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.007 | -0.007 | +0.993 | pass |
| Brightness (power centroid) | 600.0..3500.0 Hz | 1474.4 | 1483.4 | +874.404 | pass |
| Width: L/R correlation above 200 Hz | 0.200..0.800 | 0.741 | 0.740 | +0.059 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 1.00 oct | 1.24 | 1.20 | +0.198 | pass |
| Move period vs Mod A Custom 1/2 = 0.8000 s (error) | -5.00..5.00 % | 0.591 | 0.094 | +4.409 | pass |
| Move period ACF peak height | >= 0.500 | 0.570 | 0.583 | +0.070 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.246 | 0.241 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -8.22 |  | +5.215 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -68.33 |  | +18.333 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -68.02 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.040 |  | +0.960 | pass |
| Clean sub: 40-90 Hz RMS vs the sub oscillator alone | -2.00..2.00 dB | 1.16 |  | +0.836 | pass |
| Macro 1 "Growl" sweep | >= 0.700 | 0.897 D oct |  | +0.197 | pass |
| Macro 2 "Vowel" sweep | >= 0.500 | 0.736 C oct |  | +0.236 | pass |
| Macro 3 "Grit" sweep | >= 0.400 | 0.715 C oct |  | +0.315 | pass |
| Macro 4 "Width" sweep | >= 0.100 | 0.428 W corr |  | +0.328 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.931 |  | +1.069 | pass |
| Log-mel distance to the nearest tearout preset | >= 3.30 | 4.66 dB |  | +1.358 | pass |

## Wide Reese Chorus (reese, Bass/reese)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 470.0 chars |  | +390.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -8.59 | -8.61 | +5.591 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.14 | -17.14 | +1.357 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -18.26 | -18.25 | +1.241 | pass |
| Crest (peak - RMS) | 5.00..10.00 dB | 8.55 | 8.52 | +1.448 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.857 | 0.862 | +1.138 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.950 | 1.000 | 1.000 | +0.050 | pass |
| Low-band side/mid, dB | - dB | -41.98 | -42.04 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.000 | -0.000 | +1.000 | pass |
| Brightness (power centroid) | 180.0..2200.0 Hz | 690.4 | 691.7 | +510.367 | pass |
| Width: L/R correlation above 200 Hz | 0.200..0.900 | 0.583 | 0.582 | +0.317 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.400 oct | 1.81 | 1.84 | +1.413 | pass |
| Move period vs Mod B Sine 2/1 = 3.2000 s (error) | -5.00..5.00 % | 3.44 | -3.43 | +1.558 | pass |
| Move period ACF peak height | >= 0.500 | 0.855 | 0.851 | +0.351 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.509 | 0.500 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -8.59 |  | +5.591 | pass |
| Alias floor (frozen modulation) | <= -32.00 dB | -33.08 |  | +1.079 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -29.15 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.011 |  | +0.989 | pass |
| Reese beat rate at f0 vs expected (4 unison voices, outer +-18 ct), error | -10.00..10.00 % | 4.12 |  | +5.879 | pass |
| Beating depth, 100-300 Hz envelope (p95-p5) | >= 3.00 dB | 11.56 |  | +8.564 | pass |
| Macro 1 "Width" sweep | >= 0.200 | 0.449 W corr |  | +0.249 | pass |
| Macro 2 "Brightness" sweep | >= 0.500 | 2.29 C oct |  | +1.794 | pass |
| Macro 3 "Beat" sweep | >= 0.010 | 0.013 W corr |  | +0.003 | pass |
| Macro 4 "Sweep" sweep | >= 0.300 | 2.17 D oct |  | +1.869 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.039 |  | +1.961 | pass |
| Log-mel distance to the nearest reese preset | >= 3.30 | 4.34 dB |  | +1.043 | pass |

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
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.683 |  | +1.317 | pass |
| Log-mel distance to the nearest sub preset | >= 3.30 | 4.83 dB |  | +1.529 | pass |

## Drift Random Wobble (wobble, Bass/wobble)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 736.0 chars |  | +656.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -7.80 | -7.83 | +4.802 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.73 | -17.77 | +0.726 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -17.40 | -17.44 | +2.058 | pass |
| Crest (peak - RMS) | 5.00..11.00 dB | 9.49 | 9.56 | +1.440 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.265 | 0.226 | +1.735 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.000 | 1.000 | +0.010 | pass |
| Low-band side/mid, dB | - dB | -69.01 | -70.25 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.000 | -0.000 | +1.000 | pass |
| Brightness (power centroid) | 180.0..1600.0 Hz | 880.8 | 947.0 | +653.006 | pass |
| Width: L/R correlation above 200 Hz | 0.700..1.00 | 0.998 | 0.998 | +0.002 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.800 oct | 1.35 | 1.35 | +0.550 | pass |
| Not periodic: largest ACF peak beyond 30 ms (Mod B SmoothRandom 1/16) | <= 0.750 | 0.349 | 0.372 | +0.378 | pass |
| Sub (<90 Hz) RMS variation over the cycle | <= 1.50 dB | 0.868 | 0.925 | +0.575 | pass |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | -0.532 | -0.533 | +2.467 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -8.22 |  | +5.221 | pass |
| Alias floor (frozen modulation) | <= -55.00 dB | -87.12 |  | +32.121 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -87.40 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.225 |  | +0.775 | pass |
| Macro 1 "Drift Depth" sweep | >= 0.300 | 2.59 D oct |  | +2.291 | pass |
| Macro 2 "Centre" sweep | >= 0.300 | 0.894 D oct |  | +0.594 | pass |
| Macro 3 "Slow Sweep" sweep | >= 0.300 | 0.642 C oct |  | +0.342 | pass |
| Macro 4 "Width" sweep | >= 0.100 | 0.353 W corr |  | +0.253 | pass |
| Repeatability (two renders, -80 dB) | - | n/a  |  | - | skip |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.552 |  | +1.448 | pass |
| Log-mel distance to the nearest wobble preset | >= 3.30 | 5.11 dB |  | +1.806 | pass |

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
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.628 |  | +1.372 | pass |
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
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.273 |  | +1.727 | pass |
| Log-mel distance to the nearest sub preset | >= 3.30 | 5.59 dB |  | +2.294 | pass |

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
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.896 |  | +1.104 | pass |
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
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.913 |  | +1.087 | pass |
| Log-mel distance to the nearest wobble preset | >= 3.30 | 4.38 dB |  | +1.079 | pass |

## Talking Chatter (formant, Bass/formant)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 716.0 chars |  | +636.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -5.91 | -5.84 | +2.843 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -16.89 | -16.89 | +1.607 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -15.70 | -15.67 | +2.169 | pass |
| Crest (peak - RMS) | 6.00..12.00 dB | 10.98 | 11.05 | +0.952 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 1.11 | 1.11 | +0.891 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | - dB | -300.0 | -300.0 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 350.0..2200.0 Hz | 1119.4 | 1120.1 | +769.363 | pass |
| Width: L/R correlation above 200 Hz | 0.850..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 1.00 oct | 1.36 | 1.35 | +0.352 | pass |
| Move period vs Mod B Custom 1/2 = 0.8000 s (error) | -5.00..5.00 % | -0.353 | 0.130 | +4.647 | pass |
| Move period ACF peak height | >= 0.500 | 0.910 | 0.965 | +0.410 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.211 | 0.213 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -5.91 |  | +2.911 | pass |
| Alias floor (frozen modulation) | <= -55.00 dB | -86.69 |  | +31.687 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -86.74 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.436 |  | +0.564 | pass |
| Macro 1 "Syllables" sweep | >= 0.300 | 0.637 D oct |  | +0.337 | pass |
| Macro 2 "Mouth" sweep | >= 1.00 | 1.91 L LU |  | +0.914 | pass |
| Macro 3 "Glide" sweep | >= 0.300 | 0.345 D oct |  | +0.045 | pass |
| Macro 4 "Chatter" sweep | >= 0.300 | 0.707 D oct |  | +0.407 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.289 |  | +1.711 | pass |
| Log-mel distance to the nearest formant preset | >= 3.30 | 4.29 dB |  | +0.986 | pass |

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
| RMS against the median of its category in this run | -2.00..2.00 dB | -1.03 |  | +0.973 | pass |
| Log-mel distance to the nearest wobble preset | >= 3.30 | 5.11 dB |  | +1.806 | pass |

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
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.981 |  | +1.019 | pass |
| Log-mel distance to the nearest formant preset | >= 3.30 | 4.29 dB |  | +0.986 | pass |

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
| Description of at least 80 characters | >= 80.00 | 696.0 chars |  | +616.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -11.40 | -11.40 | +8.399 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -16.18 | -16.18 | +1.679 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -17.72 | -17.72 | +1.783 | pass |
| Crest (peak - RMS) | 3.00..7.00 dB | 4.14 | 4.14 | +1.138 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 1.82 | 1.82 | +0.179 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.995 | 1.00 | 1.00 | +0.005 | pass |
| Low-band side/mid, dB | - dB | -300.0 | -300.0 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 55.00..160.0 Hz | 55.92 | 55.92 | +0.917 | pass |
| Width: L/R correlation above 200 Hz | 0.950..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Static: Move depth | <= 0.250 oct | 0.171 | 0.171 | +0.079 | pass |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | -0.255 | -0.255 | +2.745 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -11.40 |  | +8.400 | pass |
| Alias floor (frozen modulation) | <= -60.00 dB | -87.59 |  | +27.587 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -87.66 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.000 |  | +1.000 | pass |
| Energy below 150 Hz (share) | >= 90.00 % | 99.99 |  | +9.985 | pass |
| THD below 150 Hz | - % | 9.85 |  | - | info |
| Macro 1 "Warmth" sweep | >= 0.300 | 1.05 C oct |  | +0.753 | pass |
| Macro 2 "Small Speaker" sweep | >= 0.300 | 0.336 C oct |  | +0.036 | pass |
| Macro 3 "Level" sweep | >= 1.00 | 6.00 L LU |  | +5.000 | pass |
| Macro 4 "Release" sweep | >= 50.00 | 561.7 T ms |  | +511.671 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 1.00 |  | +0.998 | pass |
| Log-mel distance to the nearest sub preset | >= 3.30 | 4.83 dB |  | +1.529 | pass |

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
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -6.51 | -6.60 | +3.509 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -16.89 | -16.88 | +1.613 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -14.99 | -14.98 | +1.479 | pass |
| Crest (peak - RMS) | 6.00..12.00 dB | 9.89 | 10.12 | +1.881 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 1.11 | 1.12 | +0.883 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | - dB | -300.0 | -300.0 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 350.0..2200.0 Hz | 961.9 | 963.7 | +611.890 | pass |
| Width: L/R correlation above 200 Hz | 0.850..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 1.00 oct | 1.06 | 1.05 | +0.052 | pass |
| Move period vs Mod A Triangle 1/2 = 0.8000 s (error) | -5.00..5.00 % | -0.557 | 0.164 | +4.443 | pass |
| Move period ACF peak height | >= 0.500 | 0.993 | 0.995 | +0.493 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.680 | 0.698 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -6.51 |  | +3.509 | pass |
| Alias floor (frozen modulation) | <= -55.00 dB | -84.09 |  | +29.088 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -84.49 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.178 |  | +0.822 | pass |
| Macro 1 "Vowel Sweep" sweep | >= 0.700 | 0.754 D oct |  | +0.054 | pass |
| Macro 2 "Vowel Position" sweep | >= 1.00 | 4.13 L LU |  | +3.128 | pass |
| Macro 3 "Sharpness" sweep | >= 0.300 | 0.719 D oct |  | +0.419 | pass |
| Macro 4 "Grit" sweep | >= 0.300 | 1.26 C oct |  | +0.959 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.295 |  | +1.705 | pass |
| Log-mel distance to the nearest formant preset | >= 3.30 | 4.88 dB |  | +1.581 | pass |

## Distinctiveness (log-mel distance, dB; threshold 3.3 dB)

Same-family pairs are judged; other pairs are listed for calibration.

| A | B | Same family | Distance |
|---|---|---|---|
| Square Melody Lead | Wide Hoover Stack | yes | 3.40 |
| Robot Bark Drop | Talking Chatter | no | 3.43 |
| Screech Formant Rise | Wide Hoover Stack | yes | 3.44 |
| Four Bar Downlifter | Noise Sweep Band | yes | 3.44 |
| Metal Gnash Fold | Pitch Rip Growl | yes | 3.54 |
| Screech Dive One | Wide Hoover Stack | yes | 3.61 |
| Dotted Metal Stab | Triplet Bark AM | yes | 3.72 |
| Pulse Width Pocket | Talking Chatter | no | 3.76 |
| Drift Random Wobble | Yoi Sweep Slow | no | 3.79 |
| Screech Dive One | Screech Formant Rise | yes | 3.92 |
| Pulse Width Pocket | Robot Bark Drop | yes | 3.96 |
| Call and Response Square | Wide Reese Chorus | no | 4.00 |
| Pulse Width Pocket | Vowel Pluck Bark | no | 4.00 |
| Call and Response Square | Dotted Metal Stab | yes | 4.13 |
| Hollow Square Reference | Robotic Comb Chatter | yes | 4.14 |
| Triplet Bark AM | Drift Random Wobble | no | 4.16 |
| Triplet Bark AM | Yoi Sweep Slow | no | 4.17 |
| Metal Gnash Fold | Drift Random Wobble | no | 4.17 |
| Eighth Triangle Wub | Sixteenth Gate Wub | yes | 4.17 |
| Talking Chatter | Vowel Pluck Bark | yes | 4.29 |
| Init Square Bass | Vowel Pluck Bark | no | 4.34 |
| Neuro Comb Fold | Wide Reese Chorus | yes | 4.34 |
| Sixteenth Gate Wub | Slow Sine Wub | yes | 4.38 |
| Call and Response Square | Neuro Comb Fold | no | 4.38 |
| Robot Bark Drop | Robotic Comb Chatter | yes | 4.38 |
| Metal Gnash Fold | Talking Chatter | no | 4.40 |
| Metal Ring Pluck | Wide Chord Stab | yes | 4.45 |
| Robot Bark Drop | Vowel Pluck Bark | no | 4.48 |
| Call and Response Square | Eighth Triangle Wub | no | 4.54 |
| Dotted Metal Stab | Yoi Sweep Slow | no | 4.56 |
| Triplet Shredder | Vocal Growl Gnash | yes | 4.66 |
| Liquid Dark Reese | Slow Sine Wub | no | 4.68 |
| Robot Bark Drop | Metal Gnash Fold | no | 4.71 |
| Robot Bark Drop | Yoi Sweep Slow | no | 4.74 |
| Dirty Sub Layer | Warm Tube Sub | yes | 4.83 |
| Triplet Bark AM | Triplet Saw Chop | no | 4.84 |
| Init Square Bass | Pulse Width Pocket | no | 4.85 |
| Pitch Rip Growl | Triplet Shredder | yes | 4.85 |
| Liquid Dark Reese | Wide Reese Chorus | yes | 4.86 |
| Talking Chatter | Yoi Sweep Slow | yes | 4.88 |
| Init Square Bass | Robot Bark Drop | no | 4.90 |
| Pulse Width Pocket | Yoi Sweep Slow | no | 4.90 |
| Eighth Triangle Wub | Slow Sine Wub | yes | 4.94 |
| Phased Grind | Pitch Rip Growl | yes | 4.95 |
| Dotted Metal Stab | Drift Random Wobble | no | 4.98 |
| Wide Reese Chorus | Eighth Triangle Wub | no | 5.02 |
| Dotted Metal Stab | Triplet Saw Chop | no | 5.03 |
| Metal Gnash Fold | Vowel Pluck Bark | no | 5.04 |
| Metal Gnash Fold | Phased Grind | yes | 5.06 |
| Vowel Pluck Bark | Yoi Sweep Slow | yes | 5.08 |
| Pitch Drop Sub Pluck | Sub Drop Impact | no | 5.09 |
| Drift Random Wobble | Triplet Saw Chop | yes | 5.11 |
| Phased Grind | Wide Reese Chorus | no | 5.11 |
| Hollow Square Reference | Triplet Shredder | no | 5.17 |
| Dotted Metal Stab | Wide Reese Chorus | no | 5.22 |
| Hollow Square Reference | Triplet Saw Chop | no | 5.23 |
| Metal Gnash Fold | Yoi Sweep Slow | no | 5.25 |
| Init Square Bass | Talking Chatter | no | 5.31 |
| Triplet Bark AM | Metal Gnash Fold | no | 5.35 |
| Neuro Comb Fold | Triplet Saw Chop | no | 5.35 |

## Bank summary

41 preset(s) analysed; 40 use the FX chain; 16 use a reordered chain (the sheet asks 20 of 41 and 6 of 41 for the whole bank). Failed checks: 8.
