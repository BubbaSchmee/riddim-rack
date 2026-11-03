# Bank QA report

Tool `bank_qa` (GTS), contract `docs/design/bank-recipes.md` section A. Renders at 150 and 140 BPM, 48 kHz, velocity 100 (127 for the peak check). Tags that steer the tool are listed in the source header of `crates/gts/examples/bank_qa.rs`.

Inputs: `crates/gts/presets/factory`

## Summary

| Preset | Family | Checks | Fail | Info/Skip | Result |
|---|---|---|---|---|---|
| Call and Response Square | riddim (tags) | 47 | 0 | 3 | PASS |
| Dirty Sub Layer | sub (tags) | 41 | 0 | 4 | PASS |
| Dotted Metal Stab | riddim (tags) | 47 | 0 | 3 | PASS |
| Drift Random Wobble | wobble (tags) | 44 | 0 | 4 | PASS |
| Eighth Triangle Wub | wobble (tags) | 47 | 0 | 3 | PASS |
| Hollow Square Reference | riddim (tags) | 47 | 0 | 3 | PASS |
| Liquid Dark Reese | reese (tags) | 45 | 0 | 5 | PASS |
| Metal Gnash Fold | tearout (tags) | 44 | 0 | 5 | PASS |
| Neuro Comb Fold | reese (tags) | 44 | 0 | 6 | PASS |
| Phased Grind | tearout (tags) | 44 | 0 | 5 | PASS |
| Pitch Rip Growl | tearout (tags) | 44 | 0 | 5 | PASS |
| Pulse Width Pocket | riddim (tags) | 47 | 0 | 3 | PASS |
| Pure Sine Sub | sub (tags) | 42 | 0 | 3 | PASS |
| Robot Bark Drop | riddim (tags) | 43 | 0 | 5 | PASS |
| Robotic Comb Chatter | riddim (tags) | 47 | 0 | 3 | PASS |
| Sixteenth Gate Wub | wobble (tags) | 47 | 0 | 3 | PASS |
| Slow Sine Wub | wobble (tags) | 47 | 0 | 3 | PASS |
| Slow Sweep Reese | reese (tags) | 45 | 0 | 5 | PASS |
| Talking Chatter | formant (tags) | 43 | 0 | 5 | PASS |
| Triplet Bark AM | riddim (tags) | 47 | 0 | 3 | PASS |
| Triplet Saw Chop | wobble (tags) | 47 | 0 | 3 | PASS |
| Triplet Shredder | tearout (tags) | 44 | 0 | 5 | PASS |
| Vocal Growl Gnash | tearout (tags) | 44 | 0 | 5 | PASS |
| Vowel Pluck Bark | formant (qa_family tag) | 39 | 0 | 7 | PASS |
| Warm Tube Sub | sub (tags) | 41 | 0 | 4 | PASS |
| Wide Reese Chorus | reese (tags) | 45 | 0 | 5 | PASS |
| Yoi Sweep Slow | formant (tags) | 43 | 0 | 5 | PASS |
| Four Bar Downlifter | fx (category) | 40 | 0 | 2 | PASS |
| Four Bar Riser | fx (category) | 40 | 0 | 2 | PASS |
| Noise Sweep Band | fx (category) | 38 | 0 | 1 | PASS |
| Sub Drop Impact | fx (category) | 35 | 0 | 1 | PASS |
| Init Square Bass | init (category) | 21 | 0 | 7 | PASS |
| Diving Screech | lead (category) | 34 | 0 | 1 | PASS |
| Screech Formant Rise | lead (category) | 34 | 0 | 1 | PASS |
| Square Melody Lead | lead (category) | 33 | 0 | 1 | PASS |
| Wide Hoover Stack | lead (category) | 33 | 0 | 2 | PASS |
| Metal Ring Pluck | pluck (category) | 37 | 0 | 1 | PASS |
| Pitch Drop Sub Pluck | pluck (category) | 37 | 0 | 1 | PASS |
| Wide Chord Stab | pluck (category) | 37 | 0 | 1 | PASS |
| Gated Triplet Call | seq (category) | 41 | 0 | 2 | PASS |
| Sixteenth Stutter Response | seq (category) | 41 | 0 | 2 | PASS |

## Call and Response Square (riddim, Bass/riddim)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 449.0 chars |  | +369.000 | pass |
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
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | 0.398 |  | +0.602 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.311 |  | +1.689 | pass |
| Log-mel distance to the nearest riddim preset | >= 3.30 | 3.99 dB |  | +0.695 | pass |

## Dirty Sub Layer (sub, Bass/sub)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 538.0 chars |  | +458.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -11.71 | -11.70 | +8.705 | pass |
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
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -11.71 |  | +8.707 | pass |
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
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.372 |  | +1.628 | pass |
| Log-mel distance to the nearest sub preset | >= 3.30 | 4.83 dB |  | +1.529 | pass |

## Dotted Metal Stab (riddim, Bass/riddim)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 458.0 chars |  | +378.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -6.58 | -6.74 | +3.583 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -18.18 | -18.17 | +0.321 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.13 | -16.13 | +2.629 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 11.60 | 11.43 | +1.404 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -0.179 | -0.174 | +1.821 | pass |
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
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -6.58 |  | +3.583 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -61.55 |  | +11.547 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -83.88 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.653 |  | +0.347 | pass |
| Macro 1 "Stab Depth" sweep | >= 0.300 | 1.55 D oct |  | +1.254 | pass |
| Macro 2 "FM Mix" sweep | >= 0.300 | 1.37 C oct |  | +1.073 | pass |
| Macro 3 "Notch Sweep" sweep | >= 1.00 | 3.62 L LU |  | +2.616 | pass |
| Macro 4 "Width" sweep | >= 0.100 | 0.279 W corr |  | +0.179 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | 0.403 |  | +0.597 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.686 |  | +1.314 | pass |
| Log-mel distance to the nearest riddim preset | >= 3.30 | 3.72 dB |  | +0.419 | pass |

## Drift Random Wobble (wobble, Bass/wobble)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 533.0 chars |  | +453.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -7.29 | -6.90 | +3.901 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -16.96 | -16.91 | +1.541 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.63 | -16.54 | +2.870 | pass |
| Crest (peak - RMS) | 5.00..11.00 dB | 9.65 | 9.54 | +1.345 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 1.04 | 1.09 | +0.911 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.000 | 1.000 | +0.010 | pass |
| Low-band side/mid, dB | - dB | -70.51 | -69.34 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.000 | -0.000 | +1.000 | pass |
| Brightness (power centroid) | 180.0..1600.0 Hz | 916.9 | 928.6 | +671.412 | pass |
| Width: L/R correlation above 200 Hz | 0.700..1.00 | 0.998 | 0.998 | +0.002 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.800 oct | 1.10 | 1.22 | +0.299 | pass |
| Not periodic: largest ACF peak beyond 30 ms (Mod B SmoothRandom 1/16) | <= 0.750 | 0.327 | 0.473 | +0.277 | pass |
| Sub (<90 Hz) RMS variation over the cycle | <= 1.50 dB | 0.944 | 0.830 | +0.556 | pass |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | -0.383 | -0.430 | +2.570 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -7.29 |  | +4.290 | pass |
| Alias floor (frozen modulation) | <= -55.00 dB | -87.12 |  | +32.121 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -87.40 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.373 |  | +0.627 | pass |
| Macro 1 "Drift Depth" sweep | >= 0.300 | 2.98 D oct |  | +2.682 | pass |
| Macro 2 "Centre" sweep | >= 0.300 | 0.582 C oct |  | +0.282 | pass |
| Macro 3 "Slow Sweep" sweep | >= 0.300 | 0.952 C oct |  | +0.652 | pass |
| Macro 4 "Width" sweep | >= 0.100 | 0.352 W corr |  | +0.252 | pass |
| Repeatability (two renders, -80 dB) | - | n/a  |  | - | skip |
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | -0.098 |  | +0.902 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.534 |  | +1.466 | pass |
| Log-mel distance to the nearest wobble preset | >= 3.30 | 4.96 dB |  | +1.660 | pass |

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
| Description of at least 80 characters | >= 80.00 | 569.0 chars |  | +489.000 | pass |
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
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | -0.116 |  | +0.884 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.318 |  | +1.682 | pass |
| Log-mel distance to the nearest wobble preset | >= 3.30 | 4.15 dB |  | +0.854 | pass |

## Hollow Square Reference (riddim, Bass/riddim)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 396.0 chars |  | +316.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -8.00 | -7.98 | +4.984 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.96 | -17.95 | +0.541 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.44 | -16.43 | +2.933 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 9.96 | 9.97 | +3.031 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.041 | 0.047 | +1.953 | pass |
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
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -8.00 |  | +4.995 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -84.48 |  | +34.484 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -86.78 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.306 |  | +0.694 | pass |
| Macro 1 "Bark Depth" sweep | >= 0.300 | 1.09 D oct |  | +0.792 | pass |
| Macro 2 "Tone" sweep | >= 0.300 | 1.11 C oct |  | +0.808 | pass |
| Macro 3 "Grit" sweep | >= 0.300 | 1.52 C oct |  | +1.222 | pass |
| Macro 4 "Chatter" sweep | >= 0.300 | 1.26 D oct |  | +0.958 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | 0.094 |  | +0.906 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.466 |  | +1.534 | pass |
| Log-mel distance to the nearest riddim preset | >= 3.30 | 4.67 dB |  | +1.372 | pass |

## Liquid Dark Reese (reese, Bass/reese)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 508.0 chars |  | +428.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -7.59 | -7.82 | +4.594 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -16.11 | -16.14 | +1.613 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -17.10 | -17.12 | +2.383 | pass |
| Crest (peak - RMS) | 5.00..10.00 dB | 8.52 | 8.32 | +1.481 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 1.89 | 1.86 | +0.113 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.970 | 1.000 | 1.000 | +0.030 | pass |
| Low-band side/mid, dB | - dB | -39.29 | -39.24 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.001 | -0.001 | +0.999 | pass |
| Brightness (power centroid) | 180.0..2200.0 Hz | 346.8 | 347.1 | +166.845 | pass |
| Width: L/R correlation above 200 Hz | 0.200..0.900 | 0.807 | 0.807 | +0.093 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.400 oct | 1.21 | 1.21 | +0.808 | pass |
| Move period vs Mod B Triangle 2/1 = 3.2000 s (error) | -5.00..5.00 % | -0.802 | 3.92 | +1.081 | pass |
| Move period ACF peak height | >= 0.500 | 0.728 | 0.717 | +0.217 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.901 | 1.04 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -7.59 |  | +4.594 | pass |
| Alias floor (frozen modulation) | <= -24.00 dB | -24.84 |  | +0.840 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -15.76 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.013 |  | +0.987 | pass |
| Reese beat rate at f0 vs expected (qa_beat tag), error | -10.00..10.00 % | 4.97 |  | +5.027 | pass |
| Beating depth, 100-300 Hz envelope (p95-p5) | >= 3.00 dB | 6.43 |  | +3.435 | pass |
| Macro 1 "Beat" sweep | >= 0.005 | 0.009 W corr |  | +0.004 | pass |
| Macro 2 "Brightness" sweep | >= 0.500 | 2.92 C oct |  | +2.424 | pass |
| Macro 3 "Drift" sweep | >= 0.300 | 2.18 D oct |  | +1.883 | pass |
| Macro 4 "Warmth" sweep | >= 1.00 | 1.09 L LU |  | +0.086 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | -0.568 |  | +0.432 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 1.38 |  | +0.620 | pass |
| Log-mel distance to the nearest reese preset | >= 3.30 | 5.07 dB |  | +1.774 | pass |

## Metal Gnash Fold (tearout, Bass/tearout)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 475.0 chars |  | +395.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -6.51 | -6.65 | +3.511 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -16.09 | -16.06 | +1.556 | pass |
| LUFS-I over the hold | -18.50..-12.50 LUFS | -16.77 | -16.75 | +1.731 | pass |
| Crest (peak - RMS) | 5.00..10.00 dB | 9.58 | 9.29 | +0.420 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 1.91 | 1.94 | +0.056 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.950 | 0.980 | 0.982 | +0.030 | pass |
| Low-band side/mid, dB | - dB | -20.78 | -21.10 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.043 | -0.040 | +0.957 | pass |
| Brightness (power centroid) | 600.0..3500.0 Hz | 1470.8 | 1462.8 | +862.789 | pass |
| Width: L/R correlation above 200 Hz | 0.200..0.800 | 0.786 | 0.790 | +0.010 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 1.00 oct | 1.53 | 1.59 | +0.526 | pass |
| Move period vs Mod A Triangle 1/8T = 0.1333 s (error) | -5.00..5.00 % | 2.78 | -3.07 | +1.930 | pass |
| Move period ACF peak height | >= 0.500 | 0.876 | 0.899 | +0.376 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 1.53 | 1.50 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -6.68 |  | +3.685 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -65.16 |  | +15.164 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -85.22 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.213 |  | +0.787 | pass |
| Clean sub: 40-90 Hz RMS vs the sub oscillator alone | -2.00..2.00 dB | -0.550 |  | +1.450 | pass |
| Macro 1 "Gnash" sweep | >= 0.600 | 1.19 D oct |  | +0.588 | pass |
| Macro 2 "Ring" sweep | >= 0.500 | 1.21 C oct |  | +0.707 | pass |
| Macro 3 "Fold" sweep | >= 0.400 | 1.94 C oct |  | +1.545 | pass |
| Macro 4 "Flange" sweep | >= 0.100 | 0.695 W corr |  | +0.595 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | -0.238 |  | +0.762 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 1.40 |  | +0.597 | pass |
| Log-mel distance to the nearest tearout preset | >= 3.30 | 3.61 dB |  | +0.307 | pass |

## Neuro Comb Fold (reese, Bass/reese)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 526.0 chars |  | +446.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -6.39 | -6.76 | +3.387 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -16.09 | -16.14 | +1.589 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.90 | -16.91 | +2.592 | pass |
| Crest (peak - RMS) | 5.00..10.00 dB | 9.70 | 9.38 | +0.298 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 1.91 | 1.86 | +0.089 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.970 | 1.000 | 0.999 | +0.029 | pass |
| Low-band side/mid, dB | - dB | -36.15 | -35.21 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.001 | -0.001 | +0.999 | pass |
| Brightness (power centroid) | 180.0..2200.0 Hz | 725.3 | 706.4 | +526.432 | pass |
| Width: L/R correlation above 200 Hz | 0.200..0.900 | 0.837 | 0.830 | +0.063 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.800 oct | 1.92 | 2.17 | +1.117 | pass |
| Move period vs Mod B Triangle 1/8T = 0.1333 s (error) | -5.00..5.00 % | 1.64 | -2.49 | +2.509 | pass |
| Move period ACF peak height | >= 0.500 | 0.616 | 0.612 | +0.112 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 3.59 | 4.48 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -6.41 |  | +3.409 | pass |
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
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | -0.366 |  | +0.634 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 1.40 |  | +0.596 | pass |
| Log-mel distance to the nearest reese preset | >= 3.30 | 4.34 dB |  | +1.037 | pass |

## Phased Grind (tearout, Bass/tearout)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 484.0 chars |  | +404.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -5.94 | -3.29 | +0.288 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -16.08 | -16.08 | +1.575 | pass |
| LUFS-I over the hold | -18.50..-12.50 LUFS | -17.20 | -17.19 | +1.299 | pass |
| Crest (peak - RMS) | 5.00..10.00 dB | 7.76 | 8.23 | +1.768 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 1.92 | 1.92 | +0.075 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.950 | 0.975 | 0.975 | +0.025 | pass |
| Low-band side/mid, dB | - dB | -18.94 | -19.09 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.070 | -0.067 | +0.930 | pass |
| Brightness (power centroid) | 600.0..3500.0 Hz | 1134.3 | 1123.6 | +523.592 | pass |
| Width: L/R correlation above 200 Hz | 0.200..0.800 | 0.296 | 0.288 | +0.088 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 1.00 oct | 1.30 | 1.33 | +0.300 | pass |
| Move period vs Mod A Triangle 1/16 = 0.1000 s (error) | -5.00..5.00 % | 0.579 | -0.696 | +4.304 | pass |
| Move period ACF peak height | >= 0.500 | 0.796 | 0.787 | +0.287 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 1.27 | 1.33 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -5.94 |  | +2.943 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -50.01 |  | +0.006 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -34.84 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.535 |  | +0.465 | pass |
| Clean sub: 40-90 Hz RMS vs the sub oscillator alone | -2.00..2.00 dB | -1.76 |  | +0.241 | pass |
| Macro 1 "Grind" sweep | >= 0.500 | 1.33 D oct |  | +0.830 | pass |
| Macro 2 "Phaser Depth" sweep | >= 0.400 | 0.775 C oct |  | +0.375 | pass |
| Macro 3 "Grit" sweep | >= 0.300 | 1.19 C oct |  | +0.885 | pass |
| Macro 4 "Width" sweep | >= 0.150 | 0.258 W corr |  | +0.108 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | -0.669 |  | +0.331 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 1.41 |  | +0.588 | pass |
| Log-mel distance to the nearest tearout preset | >= 3.30 | 4.22 dB |  | +0.920 | pass |

## Pitch Rip Growl (tearout, Bass/tearout)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 455.0 chars |  | +375.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -6.51 | -6.23 | +3.227 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -16.06 | -16.03 | +1.528 | pass |
| LUFS-I over the hold | -18.50..-12.50 LUFS | -16.43 | -16.40 | +2.066 | pass |
| Crest (peak - RMS) | 5.00..10.00 dB | 8.55 | 9.30 | +0.704 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 1.94 | 1.97 | +0.028 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.950 | 0.996 | 0.996 | +0.046 | pass |
| Low-band side/mid, dB | - dB | -22.44 | -22.67 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.027 | -0.026 | +0.973 | pass |
| Brightness (power centroid) | 600.0..3500.0 Hz | 1715.6 | 1710.1 | +1110.139 | pass |
| Width: L/R correlation above 200 Hz | 0.200..0.800 | 0.723 | 0.717 | +0.077 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 1.00 oct | 1.52 | 1.47 | +0.474 | pass |
| Move period vs Mod A Custom 1/4 = 0.4000 s (error) | -5.00..5.00 % | 0.113 | 0.273 | +4.727 | pass |
| Move period ACF peak height | >= 0.500 | 0.903 | 0.938 | +0.403 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 2.19 | 2.25 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -6.51 |  | +3.505 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -81.55 |  | +31.550 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -65.16 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.105 |  | +0.895 | pass |
| Clean sub: 40-90 Hz RMS vs the sub oscillator alone | -2.00..2.00 dB | -0.050 |  | +1.950 | pass |
| Macro 1 "Rip" sweep | >= 0.300 | 0.337 C oct |  | +0.037 | pass |
| Macro 2 "Warp" sweep | >= 0.500 | 1.31 C oct |  | +0.809 | pass |
| Macro 3 "Grit" sweep | >= 0.400 | 1.22 C oct |  | +0.816 | pass |
| Macro 4 "Flange" sweep | >= 0.100 | 0.496 W corr |  | +0.396 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | 0.098 |  | +0.902 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 1.44 |  | +0.565 | pass |
| Log-mel distance to the nearest tearout preset | >= 3.30 | 3.61 dB |  | +0.307 | pass |

## Pulse Width Pocket (riddim, Bass/riddim)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 472.0 chars |  | +392.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -9.74 | -9.78 | +6.740 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -18.18 | -18.18 | +0.317 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.17 | -16.17 | +2.673 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 8.44 | 8.40 | +2.405 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -0.183 | -0.181 | +1.817 | pass |
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
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -9.74 |  | +6.740 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -86.97 |  | +36.972 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -87.02 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.462 |  | +0.538 | pass |
| Macro 1 "Pocket Depth" sweep | >= 0.300 | 0.386 D oct |  | +0.086 | pass |
| Macro 2 "Hollowness" sweep | >= 0.300 | 0.457 D oct |  | +0.157 | pass |
| Macro 3 "Flicker" sweep | >= 0.300 | 0.397 D oct |  | +0.097 | pass |
| Macro 4 "Air" sweep | >= 0.300 | 0.696 C oct |  | +0.396 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | 0.358 |  | +0.642 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.690 |  | +1.310 | pass |
| Log-mel distance to the nearest riddim preset | >= 3.30 | 4.03 dB |  | +0.733 | pass |

## Pure Sine Sub (sub, Bass/sub)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 636.0 chars |  | +556.000 | pass |
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
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.038 |  | +1.962 | pass |
| Log-mel distance to the nearest sub preset | >= 3.30 | 5.59 dB |  | +2.294 | pass |

## Robot Bark Drop (riddim, Bass/riddim)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 513.0 chars |  | +433.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -6.62 | -6.67 | +3.618 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.68 | -17.69 | +0.813 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.39 | -16.40 | +2.891 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 11.07 | 10.61 | +1.933 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.315 | 0.313 | +1.685 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | <= -30.00 dB | -300.0 | -300.0 | +270.000 | pass |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 250.0..1500.0 Hz | 1121.2 | 1121.5 | +378.535 | pass |
| Width: L/R correlation above 200 Hz | 0.600..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.600 oct | 0.603 | 0.612 | +0.003 | pass |
| Move period | - | skip | skip | - | skip |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.259 | 0.230 | - | info |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | -0.316 | -0.331 | +2.669 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -5.50 |  | +2.497 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -84.03 |  | +34.030 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -84.55 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.016 |  | +0.984 | pass |
| Macro 1 "Bite" sweep | >= 0.300 | 1.64 C oct |  | +1.340 | pass |
| Macro 2 "Dive" sweep | >= 0.300 | 0.700 C oct |  | +0.400 | pass |
| Macro 3 "Metal" sweep | >= 0.300 | 1.54 C oct |  | +1.240 | pass |
| Macro 4 "Drive" sweep | >= 0.300 | 1.16 C oct |  | +0.862 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | 0.140 |  | +0.860 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.192 |  | +1.808 | pass |
| Log-mel distance to the nearest riddim preset | >= 3.30 | 4.03 dB |  | +0.733 | pass |

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
| Description of at least 80 characters | >= 80.00 | 509.0 chars |  | +429.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -5.63 | -6.02 | +2.628 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -18.31 | -18.32 | +0.181 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.20 | -16.19 | +2.689 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 12.69 | 12.30 | +0.314 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -0.313 | -0.319 | +1.681 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | <= -30.00 dB | -300.0 | -300.0 | +270.000 | pass |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 250.0..1500.0 Hz | 1467.6 | 1475.1 | +24.887 | pass |
| Width: L/R correlation above 200 Hz | 0.600..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.600 oct | 0.776 | 0.775 | +0.175 | pass |
| Move period vs Mod B Square 1/16 = 0.1000 s (error) | -5.00..5.00 % | 0.188 | -0.117 | +4.812 | pass |
| Move period ACF peak height | >= 0.500 | 0.893 | 0.925 | +0.393 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 2.20 | 1.87 | - | info |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | -0.425 | -0.352 | +2.575 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -5.31 |  | +2.311 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -86.58 |  | +36.581 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -86.90 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.522 |  | +0.478 | pass |
| Macro 1 "Chatter" sweep | >= 0.400 | 1.43 D oct |  | +1.029 | pass |
| Macro 2 "Comb Tone" sweep | >= 0.400 | 0.878 C oct |  | +0.478 | pass |
| Macro 3 "Fold" sweep | >= 1.00 | 2.54 L LU |  | +1.536 | pass |
| Macro 4 "Square Gate" sweep | >= 0.300 | 0.779 D oct |  | +0.479 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | 0.333 |  | +0.667 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.820 |  | +1.180 | pass |
| Log-mel distance to the nearest riddim preset | >= 3.30 | 4.67 dB |  | +1.372 | pass |

## Sixteenth Gate Wub (wobble, Bass/wobble)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 546.0 chars |  | +466.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -7.33 | -7.32 | +4.323 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.78 | -17.77 | +0.721 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.63 | -16.63 | +2.868 | pass |
| Crest (peak - RMS) | 5.00..11.00 dB | 10.45 | 10.45 | +0.549 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.221 | 0.227 | +1.773 | pass |
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
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -7.33 |  | +4.327 | pass |
| Alias floor (frozen modulation) | <= -55.00 dB | -84.21 |  | +29.205 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -84.28 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.652 |  | +0.348 | pass |
| Macro 1 "Gate Depth" sweep | >= 0.300 | 1.66 C oct |  | +1.363 | pass |
| Macro 2 "Cutoff" sweep | >= 0.300 | 1.30 C oct |  | +1.002 | pass |
| Macro 3 "Smooth" sweep | >= 0.300 | 0.425 C oct |  | +0.125 | pass |
| Macro 4 "Pump" sweep | >= 1.00 | 2.59 L LU |  | +1.595 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | -0.100 |  | +0.900 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.286 |  | +1.714 | pass |
| Log-mel distance to the nearest wobble preset | >= 3.30 | 4.15 dB |  | +0.854 | pass |

## Slow Sine Wub (wobble, Bass/wobble)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 485.0 chars |  | +405.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -7.10 | -7.11 | +4.101 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.80 | -17.79 | +0.704 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.54 | -16.54 | +2.957 | pass |
| Crest (peak - RMS) | 5.00..11.00 dB | 10.68 | 10.67 | +0.322 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.204 | 0.212 | +1.788 | pass |
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
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -7.10 |  | +4.101 | pass |
| Alias floor (frozen modulation) | <= -55.00 dB | -68.03 |  | +13.031 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -68.35 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.877 |  | +0.123 | pass |
| Macro 1 "Wobble Depth" sweep | >= 0.300 | 1.14 D oct |  | +0.844 | pass |
| Macro 2 "Cutoff" sweep | >= 0.300 | 2.09 C oct |  | +1.787 | pass |
| Macro 3 "Resonance" sweep | >= 0.300 | 0.505 D oct |  | +0.205 | pass |
| Macro 4 "Warmth" sweep | >= 0.300 | 1.76 C oct |  | +1.464 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | -0.012 |  | +0.988 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.303 |  | +1.697 | pass |
| Log-mel distance to the nearest wobble preset | >= 3.30 | 4.39 dB |  | +1.090 | pass |

## Slow Sweep Reese (reese, Bass/reese)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 458.0 chars |  | +378.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -6.86 | -7.38 | +3.865 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -16.11 | -16.09 | +1.594 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -17.02 | -17.01 | +2.477 | pass |
| Crest (peak - RMS) | 5.00..10.00 dB | 9.25 | 8.19 | +0.752 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 1.89 | 1.91 | +0.094 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.970 | 1.000 | 0.999 | +0.029 | pass |
| Low-band side/mid, dB | - dB | -36.08 | -37.96 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.001 | -0.001 | +0.999 | pass |
| Brightness (power centroid) | 180.0..2200.0 Hz | 251.1 | 254.0 | +71.145 | pass |
| Width: L/R correlation above 200 Hz | 0.200..0.900 | 0.875 | 0.881 | +0.019 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.400 oct | 2.18 | 2.07 | +1.670 | pass |
| Move period vs Mod B Triangle 1/1 = 1.6000 s (error) | -5.00..5.00 % | 0.999 | 0.589 | +4.001 | pass |
| Move period ACF peak height | >= 0.500 | 0.942 | 0.931 | +0.431 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 2.39 | 2.79 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -6.86 |  | +3.865 | pass |
| Alias floor (frozen modulation) | <= -30.00 dB | -35.62 |  | +5.619 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -26.44 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.070 |  | +0.930 | pass |
| Reese beat rate at f0 vs expected (2 unison voices, outer +-12 ct), error | -10.00..10.00 % | 2.23 |  | +7.772 | pass |
| Beating depth, 100-300 Hz envelope (p95-p5) | >= 3.00 dB | 8.43 |  | +5.433 | pass |
| Macro 1 "Sweep Depth" sweep | >= 0.800 | 1.34 D oct |  | +0.537 | pass |
| Macro 2 "Base Cutoff" sweep | >= 0.700 | 3.46 C oct |  | +2.765 | pass |
| Macro 3 "Resonance" sweep | >= 0.300 | 0.311 C oct |  | +0.011 | pass |
| Macro 4 "Grit" sweep | >= 1.00 | 2.11 L LU |  | +1.113 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | -0.491 |  | +0.509 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 1.38 |  | +0.620 | pass |
| Log-mel distance to the nearest reese preset | >= 3.30 | 6.78 dB |  | +3.479 | pass |

## Talking Chatter (formant, Bass/formant)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 602.0 chars |  | +522.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -6.51 | -6.44 | +3.443 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.49 | -17.49 | +1.007 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.30 | -16.27 | +2.769 | pass |
| Crest (peak - RMS) | 6.00..12.00 dB | 10.98 | 11.05 | +0.952 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.507 | 0.509 | +1.491 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | - dB | -300.0 | -300.0 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 350.0..2200.0 Hz | 1119.4 | 1120.1 | +769.363 | pass |
| Width: L/R correlation above 200 Hz | 0.850..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 1.00 oct | 1.36 | 1.35 | +0.352 | pass |
| Move period vs Mod B Custom 1/2 = 0.8000 s (error) | -5.00..5.00 % | -0.353 | 0.130 | +4.647 | pass |
| Move period ACF peak height | >= 0.500 | 0.910 | 0.965 | +0.410 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.211 | 0.213 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -6.51 |  | +3.511 | pass |
| Alias floor (frozen modulation) | <= -55.00 dB | -86.69 |  | +31.687 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -86.74 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.436 |  | +0.564 | pass |
| Macro 1 "Syllables" sweep | >= 0.300 | 0.637 D oct |  | +0.337 | pass |
| Macro 2 "Mouth" sweep | >= 1.00 | 1.91 L LU |  | +0.914 | pass |
| Macro 3 "Glide" sweep | >= 0.300 | 0.345 D oct |  | +0.045 | pass |
| Macro 4 "Chatter" sweep | >= 0.300 | 0.707 D oct |  | +0.407 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | 0.236 |  | +0.764 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.000 |  | +2.000 | pass |
| Log-mel distance to the nearest formant preset | >= 3.30 | 4.17 dB |  | +0.874 | pass |

## Triplet Bark AM (riddim, Bass/riddim)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 602.0 chars |  | +522.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -5.59 | -5.76 | +2.592 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -18.28 | -18.29 | +0.207 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.17 | -16.18 | +2.666 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 12.69 | 12.27 | +0.309 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -0.283 | -0.293 | +1.707 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | <= -30.00 dB | -300.0 | -300.0 | +270.000 | pass |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 250.0..1500.0 Hz | 1209.0 | 1186.4 | +290.977 | pass |
| Width: L/R correlation above 200 Hz | 0.600..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.600 oct | 0.945 | 0.906 | +0.306 | pass |
| Move period vs Mod B SawDown 1/8T = 0.1333 s (error) | -5.00..5.00 % | 0.169 | 0.205 | +4.795 | pass |
| Move period ACF peak height | >= 0.500 | 0.946 | 0.977 | +0.446 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.949 | 1.21 | - | info |
| E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined) | -3.00..3.00 dB | 0.098 | 0.095 | +2.902 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -5.20 |  | +2.202 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -85.67 |  | +35.671 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -85.76 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.034 |  | +0.966 | pass |
| Macro 1 "Bark" sweep | >= 0.300 | 0.778 D oct |  | +0.478 | pass |
| Macro 2 "Metal" sweep | >= 0.300 | 0.835 D oct |  | +0.535 | pass |
| Macro 3 "Snap" sweep | >= 0.300 | 1.14 C oct |  | +0.842 | pass |
| Macro 4 "Drive" sweep | >= 0.300 | 0.952 C oct |  | +0.652 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | 0.366 |  | +0.634 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.790 |  | +1.210 | pass |
| Log-mel distance to the nearest riddim preset | >= 3.30 | 3.72 dB |  | +0.419 | pass |

## Triplet Saw Chop (wobble, Bass/wobble)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 536.0 chars |  | +456.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -7.29 | -7.25 | +4.249 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.91 | -17.93 | +0.574 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.56 | -16.58 | +2.922 | pass |
| Crest (peak - RMS) | 5.00..11.00 dB | 10.62 | 10.55 | +0.377 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.090 | 0.074 | +1.910 | pass |
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
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -7.29 |  | +4.287 | pass |
| Alias floor (frozen modulation) | <= -55.00 dB | -65.26 |  | +10.264 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -65.89 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.234 |  | +0.766 | pass |
| Macro 1 "Chop Depth" sweep | >= 0.300 | 0.877 D oct |  | +0.577 | pass |
| Macro 2 "Brightness" sweep | >= 0.300 | 1.14 C oct |  | +0.839 | pass |
| Macro 3 "Release" sweep | >= 0.300 | 0.498 D oct |  | +0.198 | pass |
| Macro 4 "Dirt" sweep | >= 0.300 | 1.77 C oct |  | +1.471 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | -0.030 |  | +0.970 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.417 |  | +1.583 | pass |
| Log-mel distance to the nearest wobble preset | >= 3.30 | 4.96 dB |  | +1.660 | pass |

## Triplet Shredder (tearout, Bass/tearout)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 431.0 chars |  | +351.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -5.10 | -7.03 | +2.103 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -16.13 | -16.12 | +1.622 | pass |
| LUFS-I over the hold | -18.50..-12.50 LUFS | -17.15 | -17.16 | +1.342 | pass |
| Crest (peak - RMS) | 5.00..10.00 dB | 7.45 | 7.05 | +2.051 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 1.87 | 1.88 | +0.122 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.950 | 1.000 | 1.000 | +0.050 | pass |
| Low-band side/mid, dB | - dB | -50.58 | -50.14 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.000 | -0.000 | +1.000 | pass |
| Brightness (power centroid) | 600.0..3500.0 Hz | 2473.8 | 2463.1 | +1026.193 | pass |
| Width: L/R correlation above 200 Hz | 0.200..0.800 | 0.684 | 0.698 | +0.102 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.400 oct | 1.37 | 1.46 | +0.970 | pass |
| Move period vs Mod A SawUp 1/16T = 0.0667 s (error) | -5.00..5.00 % | -0.504 | -1.04 | +3.964 | pass |
| Move period ACF peak height | >= 0.400 | 0.876 | 0.880 | +0.476 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.559 | 0.671 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -5.10 |  | +2.103 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -86.39 |  | +36.395 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -78.20 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.170 |  | +0.830 | pass |
| Clean sub: 40-90 Hz RMS vs the sub oscillator alone | -2.00..2.00 dB | -1.46 |  | +0.539 | pass |
| Macro 1 "Shred" sweep | >= 0.500 | 0.796 D oct |  | +0.296 | pass |
| Macro 2 "Sync" sweep | >= 0.500 | 0.951 C oct |  | +0.451 | pass |
| Macro 3 "Drive" sweep | >= 0.400 | 0.853 C oct |  | +0.453 | pass |
| Macro 4 "Width" sweep | >= 0.150 | 0.551 W corr |  | +0.401 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | -0.623 |  | +0.377 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 1.36 |  | +0.635 | pass |
| Log-mel distance to the nearest tearout preset | >= 3.30 | 5.22 dB |  | +1.921 | pass |

## Vocal Growl Gnash (tearout, Bass/tearout)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 530.0 chars |  | +450.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -6.58 | -6.54 | +3.544 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -16.09 | -16.09 | +1.592 | pass |
| LUFS-I over the hold | -18.50..-12.50 LUFS | -17.00 | -16.99 | +1.499 | pass |
| Crest (peak - RMS) | 5.00..10.00 dB | 8.33 | 8.52 | +1.481 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 1.91 | 1.91 | +0.092 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.950 | 0.997 | 0.997 | +0.047 | pass |
| Low-band side/mid, dB | - dB | -29.72 | -29.65 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.007 | -0.007 | +0.993 | pass |
| Brightness (power centroid) | 600.0..3500.0 Hz | 1616.9 | 1627.3 | +1016.943 | pass |
| Width: L/R correlation above 200 Hz | 0.200..0.800 | 0.741 | 0.739 | +0.059 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 1.00 oct | 1.20 | 1.17 | +0.173 | pass |
| Move period vs Mod A Custom 1/2 = 0.8000 s (error) | -5.00..5.00 % | 0.578 | 0.098 | +4.422 | pass |
| Move period ACF peak height | >= 0.500 | 0.572 | 0.590 | +0.072 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.245 | 0.241 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -6.58 |  | +3.584 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -67.31 |  | +17.309 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -68.02 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.051 |  | +0.949 | pass |
| Clean sub: 40-90 Hz RMS vs the sub oscillator alone | -2.00..2.00 dB | -0.760 |  | +1.240 | pass |
| Macro 1 "Growl" sweep | >= 0.700 | 0.932 D oct |  | +0.232 | pass |
| Macro 2 "Vowel" sweep | >= 0.500 | 0.705 C oct |  | +0.205 | pass |
| Macro 3 "Grit" sweep | >= 0.400 | 0.680 C oct |  | +0.280 | pass |
| Macro 4 "Width" sweep | >= 0.100 | 0.428 W corr |  | +0.328 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | -0.469 |  | +0.531 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 1.40 |  | +0.602 | pass |
| Log-mel distance to the nearest tearout preset | >= 3.30 | 5.17 dB |  | +1.865 | pass |

## Vowel Pluck Bark (formant, Bass/formant)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 596.0 chars |  | +516.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -7.30 | -7.33 | +4.299 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -18.11 | -18.11 | +0.388 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.52 | -16.52 | +2.980 | pass |
| Crest (peak - RMS) | 6.00..12.00 dB | 10.81 | 10.78 | +1.186 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -0.112 | -0.111 | +1.888 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | - dB | -300.0 | -300.0 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 350.0..2200.0 Hz | 999.5 | 1000.2 | +649.482 | pass |
| Width: L/R correlation above 200 Hz | 0.850..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.030 oct | 0.054 | 0.051 | +0.021 | pass |
| Move period | - | skip | skip | - | skip |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.073 | 0.074 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -6.37 |  | +3.372 | pass |
| Alias floor (frozen modulation) | <= -55.00 dB | -87.03 |  | +32.034 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -87.09 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.140 |  | +0.860 | pass |
| Macro 1 "Bark" sweep | >= 1.00 | 2.68 L LU |  | +1.680 | pass |
| Macro 2 "Vowel" sweep | >= 0.300 | 0.465 C oct |  | +0.165 | pass |
| Macro 3 "Decay" sweep | >= 0.300 | 1.06 D oct |  | +0.765 | pass |
| Macro 4 "Grit" sweep | >= 1.00 | 4.92 L LU |  | +3.923 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | 0.012 |  | +0.988 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.619 |  | +1.381 | pass |
| Log-mel distance to the nearest formant preset | >= 3.30 | 4.17 dB |  | +0.874 | pass |

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
| Description of at least 80 characters | >= 80.00 | 602.0 chars |  | +522.000 | pass |
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
| RMS against the median of its category in this run | -2.00..2.00 dB | 1.31 |  | +0.687 | pass |
| Log-mel distance to the nearest sub preset | >= 3.30 | 4.83 dB |  | +1.529 | pass |

## Wide Reese Chorus (reese, Bass/reese)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 434.0 chars |  | +354.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -7.22 | -7.24 | +4.219 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -16.08 | -16.08 | +1.576 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -17.15 | -17.14 | +2.354 | pass |
| Crest (peak - RMS) | 5.00..10.00 dB | 8.86 | 8.83 | +1.137 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 1.92 | 1.92 | +0.076 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.950 | 1.000 | 1.000 | +0.050 | pass |
| Low-band side/mid, dB | - dB | -41.56 | -41.62 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | -0.000 | -0.000 | +1.000 | pass |
| Brightness (power centroid) | 180.0..2200.0 Hz | 705.7 | 706.9 | +525.673 | pass |
| Width: L/R correlation above 200 Hz | 0.200..0.900 | 0.583 | 0.582 | +0.317 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 0.400 oct | 1.81 | 1.83 | +1.415 | pass |
| Move period vs Mod B Sine 2/1 = 3.2000 s (error) | -5.00..5.00 % | 3.44 | -3.43 | +1.558 | pass |
| Move period ACF peak height | >= 0.500 | 0.854 | 0.851 | +0.351 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.509 | 0.501 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -7.22 |  | +4.219 | pass |
| Alias floor (frozen modulation) | <= -32.00 dB | -32.60 |  | +0.601 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -28.58 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.012 |  | +0.988 | pass |
| Reese beat rate at f0 vs expected (4 unison voices, outer +-18 ct), error | -10.00..10.00 % | 4.12 |  | +5.879 | pass |
| Beating depth, 100-300 Hz envelope (p95-p5) | >= 3.00 dB | 11.56 |  | +8.564 | pass |
| Macro 1 "Width" sweep | >= 0.200 | 0.449 W corr |  | +0.249 | pass |
| Macro 2 "Brightness" sweep | >= 0.500 | 2.28 C oct |  | +1.784 | pass |
| Macro 3 "Beat" sweep | >= 0.010 | 0.013 W corr |  | +0.003 | pass |
| Macro 4 "Sweep" sweep | >= 0.300 | 2.17 D oct |  | +1.866 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | -0.614 |  | +0.386 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 1.41 |  | +0.589 | pass |
| Log-mel distance to the nearest reese preset | >= 3.30 | 4.34 dB |  | +1.037 | pass |

## Yoi Sweep Slow (formant, Bass/formant)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| A riddim / dubstep family tag (Bass) | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 625.0 chars |  | +545.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -8.01 | -8.10 | +5.009 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -18.39 | -18.38 | +0.113 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.49 | -16.48 | +2.979 | pass |
| Crest (peak - RMS) | 6.00..12.00 dB | 9.89 | 10.12 | +1.881 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -0.387 | -0.383 | +1.613 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Low-band side/mid, dB | - dB | -300.0 | -300.0 | - | info |
| Mono-sum vs stereo energy below 120 Hz | -1.00..1.00 dB | 0.000 | 0.000 | +1.000 | pass |
| Brightness (power centroid) | 350.0..2200.0 Hz | 961.9 | 963.7 | +611.890 | pass |
| Width: L/R correlation above 200 Hz | 0.850..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Move depth (p95-p5 of log2 centroid) | >= 1.00 oct | 1.06 | 1.05 | +0.052 | pass |
| Move period vs Mod A Triangle 1/2 = 0.8000 s (error) | -5.00..5.00 % | -0.557 | 0.164 | +4.443 | pass |
| Move period ACF peak height | >= 0.500 | 0.993 | 0.995 | +0.493 | pass |
| Sub (<90 Hz) RMS variation over the cycle | - dB | 0.680 | 0.698 | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -8.01 |  | +5.009 | pass |
| Alias floor (frozen modulation) | <= -55.00 dB | -84.09 |  | +29.088 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -84.49 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.178 |  | +0.822 | pass |
| Macro 1 "Vowel Sweep" sweep | >= 0.700 | 0.754 D oct |  | +0.054 | pass |
| Macro 2 "Vowel Position" sweep | >= 0.500 | 0.881 C oct |  | +0.381 | pass |
| Macro 3 "Sharpness" sweep | >= 0.300 | 0.719 D oct |  | +0.419 | pass |
| Macro 4 "Grit" sweep | >= 0.300 | 1.26 C oct |  | +0.959 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | 0.042 |  | +0.958 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.894 |  | +1.106 | pass |
| Log-mel distance to the nearest formant preset | >= 3.30 | 4.90 dB |  | +1.603 | pass |

## Four Bar Downlifter (fx, FX)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 405.0 chars |  | +325.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -7.17 | -7.21 | +4.173 | pass |
| RMS over the hold, dBFS | -26.00..-20.00 dBFS | -23.80 | -23.79 | +2.195 | pass |
| LUFS-I over the hold | -26.00..-18.00 LUFS | -19.37 | -19.39 | +1.370 | pass |
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
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.153 |  | +1.847 | pass |
| Log-mel distance to the nearest fx preset | >= 3.30 | 3.44 dB |  | +0.143 | pass |

## Four Bar Riser (fx, FX)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 370.0 chars |  | +290.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -5.76 | -5.98 | +2.759 | pass |
| RMS over the hold, dBFS | -26.00..-20.00 dBFS | -22.90 | -22.61 | +2.615 | pass |
| LUFS-I over the hold | -26.00..-18.00 LUFS | -18.64 | -18.22 | +0.218 | pass |
| Crest (peak - RMS) | 7.00..16.00 dB | 13.59 | 13.39 | +2.411 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -0.897 | -0.615 | +1.103 | pass |
| Energy below 150 Hz re peak (50 ms RMS) | <= -30.00 dB | -53.90 | -53.43 | +23.428 | pass |
| Brightness (power centroid, whole hold) | 1500.0..6000.0 Hz | 2166.7 | 2172.0 | +666.670 | pass |
| Width: L/R correlation above 200 Hz (level window) | 0.000..0.700 | 0.388 | 0.375 | +0.312 | pass |
| Centroid rise, first to last 10 % of the hold | >= 2.00 oct | 2.28 | 2.31 | +0.278 | pass |
| Share of 250 ms centroid steps rising | >= 0.900 | 0.960 | 0.926 | +0.026 | pass |
| Level rise, last bar minus first bar | 3.00..8.00 dB | 5.33 | 5.36 | +2.332 | pass |
| Widening: correlation change last bar vs first (must fall by 0.2; the sheet says "width rises") | <= -0.200 | -0.368 | -0.362 | +0.162 | pass |
| Pitch trend start -> end of the hold (Info) | - | 5.66 st |  | - | info |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -5.76 |  | +2.759 | pass |
| Alias floor (frozen) | - dB | 19.05 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.737 |  | +0.263 | pass |
| Macro 1 "Rise Width" sweep | >= 0.100 | 0.366 W corr |  | +0.266 | pass |
| Macro 2 "Noise Level" sweep | >= 1.00 | 1.34 L LU |  | +0.338 | pass |
| Macro 3 "Pitch Rise" sweep | >= 0.300 | 1.31 D oct |  | +1.010 | pass |
| Macro 4 "Reverb Tail" sweep | >= 0.100 | 0.450 W corr |  | +0.350 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.754 |  | +1.246 | pass |
| Log-mel distance to the nearest fx preset | >= 3.30 | 9.97 dB |  | +6.668 | pass |

## Noise Sweep Band (fx, FX)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 366.0 chars |  | +286.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -8.64 | -8.77 | +5.644 | pass |
| RMS over the hold, dBFS | -26.00..-20.00 dBFS | -23.97 | -23.94 | +2.033 | pass |
| LUFS-I over the hold | -26.00..-18.00 LUFS | -20.22 | -20.11 | +2.106 | pass |
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
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.316 |  | +1.684 | pass |
| Log-mel distance to the nearest fx preset | >= 3.30 | 3.44 dB |  | +0.143 | pass |

## Sub Drop Impact (fx, FX)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 341.0 chars |  | +261.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -10.23 | -10.23 | +7.231 | pass |
| RMS over the hold, dBFS | -26.00..-20.00 dBFS | -23.50 | -23.50 | +2.502 | pass |
| LUFS-I over the hold | -26.00..-18.00 LUFS | -21.98 | -21.98 | +3.981 | pass |
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
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.153 |  | +1.847 | pass |
| Log-mel distance to the nearest fx preset | >= 3.30 | 16.66 dB |  | +13.355 | pass |

## Init Square Bass (init, Init)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 195.0 chars |  | +115.000 | pass |
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

## Diving Screech (lead, Lead)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 435.0 chars |  | +355.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -8.37 | -8.30 | +5.298 | pass |
| RMS over the hold, dBFS | -22.00..-18.00 dBFS | -20.87 | -20.89 | +1.111 | pass |
| LUFS-I over the hold | -22.00..-16.00 LUFS | -16.42 | -16.44 | +0.420 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 12.50 | 12.20 | +0.498 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -0.872 | -0.889 | +1.111 | pass |
| Brightness (power centroid) | 1500.0..4500.0 Hz | 4119.8 | 4121.5 | +378.512 | pass |
| Width: L/R correlation above 300 Hz | 0.100..0.700 | 0.141 | 0.146 | +0.041 | pass |
| Energy below 150 Hz vs the 300 Hz - 3 kHz band | <= -20.00 dB | -21.02 | -20.71 | +0.715 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -7.54 |  | +4.541 | pass |
| Pitch dive over 90 ms (expected 12 st +-1) | 11.00..13.00 | 12.58 st |  | +0.415 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -61.89 |  | +11.887 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -79.95 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.199 |  | +0.801 | pass |
| Macro 1 "Dive" sweep | >= 0.300 | 0.752 C oct |  | +0.452 | pass |
| Macro 2 "Comb Sweep" sweep | >= 0.500 | 1.75 D oct |  | +1.246 | pass |
| Macro 3 "Screech" sweep | >= 0.400 | 0.472 C oct |  | +0.072 | pass |
| Macro 4 "Drive" sweep | >= 1.00 | 3.31 L LU |  | +2.310 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.054 |  | +1.946 | pass |
| Log-mel distance to the nearest lead preset | >= 3.30 | 3.78 dB |  | +0.476 | pass |

## Screech Formant Rise (lead, Lead)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 480.0 chars |  | +400.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -9.66 | -9.38 | +6.378 | pass |
| RMS over the hold, dBFS | -22.00..-18.00 dBFS | -21.37 | -21.40 | +0.604 | pass |
| LUFS-I over the hold | -22.00..-16.00 LUFS | -18.58 | -18.60 | +2.582 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 11.47 | 11.42 | +1.530 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -1.37 | -1.40 | +0.604 | pass |
| Brightness (power centroid) | 1500.0..4000.0 Hz | 3774.8 | 3784.9 | +215.075 | pass |
| Width: L/R correlation above 300 Hz | 0.100..0.700 | 0.679 | 0.685 | +0.015 | pass |
| Energy below 150 Hz vs the 300 Hz - 3 kHz band | <= -20.00 dB | -29.42 | -29.27 | +9.269 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -8.85 |  | +5.846 | pass |
| Pitch dive over 80 ms (expected -1.3 st +-1) | -2.30..-0.300 | -1.77 st |  | +0.526 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -58.61 |  | +8.609 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -88.38 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.155 |  | +0.845 | pass |
| Macro 1 "Rise" sweep | >= 0.600 | 0.623 D oct |  | +0.023 | pass |
| Macro 2 "Vowel Start" sweep | >= 0.500 | 0.535 C oct |  | +0.035 | pass |
| Macro 3 "Bend" sweep | >= 0.300 | 0.494 C oct |  | +0.194 | pass |
| Macro 4 "Grit" sweep | >= 0.400 | 0.496 C oct |  | +0.096 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.440 |  | +1.560 | pass |
| Log-mel distance to the nearest lead preset | >= 3.30 | 3.40 dB |  | +0.098 | pass |

## Square Melody Lead (lead, Lead)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 380.0 chars |  | +300.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -7.50 | -7.43 | +4.429 | pass |
| RMS over the hold, dBFS | -22.00..-18.00 dBFS | -20.98 | -20.78 | +1.020 | pass |
| LUFS-I over the hold | -22.00..-16.00 LUFS | -17.92 | -17.70 | +1.704 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 12.92 | 12.46 | +0.083 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -0.980 | -0.780 | +1.020 | pass |
| Brightness (power centroid) | 1500.0..4500.0 Hz | 3980.3 | 3887.4 | +519.692 | pass |
| Width: L/R correlation above 300 Hz | 0.600..0.950 | 0.896 | 0.888 | +0.054 | pass |
| Energy below 150 Hz vs the 300 Hz - 3 kHz band | <= -20.00 dB | -22.98 | -22.98 | +2.976 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -6.39 |  | +3.386 | pass |
| Alias floor (frozen modulation) | <= -60.00 dB | -66.09 |  | +6.094 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -74.53 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.049 |  | +0.951 | pass |
| Macro 1 "Brightness" sweep | >= 0.300 | 1.16 C oct |  | +0.860 | pass |
| Macro 2 "Vibrato" sweep | >= 0.300 | 0.325 C oct |  | +0.025 | pass |
| Macro 3 "Echo" sweep | >= 1.00 | 3.46 L LU |  | +2.459 | pass |
| Macro 4 "Space" sweep | >= 1.00 | 3.54 L LU |  | +2.540 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.054 |  | +1.946 | pass |
| Log-mel distance to the nearest lead preset | >= 3.30 | 4.20 dB |  | +0.899 | pass |

## Wide Hoover Stack (lead, Lead)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 574.0 chars |  | +494.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -6.54 | -5.92 | +2.918 | pass |
| RMS over the hold, dBFS | -22.00..-18.00 dBFS | -19.51 | -19.54 | +1.514 | pass |
| LUFS-I over the hold | -22.00..-16.00 LUFS | -16.39 | -16.37 | +0.369 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 12.61 | 12.64 | +0.365 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.486 | 0.464 | +1.514 | pass |
| Brightness (power centroid) | 1800.0..4500.0 Hz | 4188.4 | 4190.3 | +309.683 | pass |
| Width: L/R correlation above 300 Hz | 0.100..0.500 | 0.407 | 0.415 | +0.085 | pass |
| Energy below 150 Hz vs the 300 Hz - 3 kHz band | <= -20.00 dB | -28.12 | -28.12 | +8.125 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -5.80 |  | +2.797 | pass |
| Pitch dive over 140 ms (expected 5 st +-1) | 4.00..6.00 | 5.11 st |  | +0.891 | pass |
| Alias floor (frozen modulation) | <= -9.70 dB | -12.92 |  | +3.217 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -10.87 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.000 |  | +1.000 | pass |
| Macro 1 "Width" sweep | >= 0.200 | 0.405 W corr |  | +0.205 | pass |
| Macro 2 "Brightness" sweep | >= 0.500 | 0.574 C oct |  | +0.074 | pass |
| Macro 3 "Dive" sweep | >= 0.300 | 0.341 C oct |  | +0.041 | pass |
| Macro 4 "Chorus Depth" sweep | >= 0.100 | 0.238 W corr |  | +0.138 | pass |
| Repeatability (two renders, -80 dB) | - | n/a  |  | - | skip |
| RMS against the median of its category in this run | -2.00..2.00 dB | 1.41 |  | +0.588 | pass |
| Log-mel distance to the nearest lead preset | >= 3.30 | 3.40 dB |  | +0.098 | pass |

Details of failed or skipped checks:

- repeat: random LFO or free random phase: the engine seeds those from a global counter

## Metal Ring Pluck (pluck, Pluck)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 418.0 chars |  | +338.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -3.68 | -3.97 | +0.685 | pass |
| RMS over the hold, dBFS | -24.00..-18.00 dBFS | -22.62 | -22.62 | +1.378 | pass |
| LUFS-I over the hold | -24.00..-18.00 LUFS | -18.51 | -18.51 | +0.505 | pass |
| Crest (peak - RMS) | 10.00..20.00 dB | 14.66 | 14.66 | +4.665 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -1.62 | -1.62 | +0.378 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.970 | 0.993 | 0.993 | +0.023 | pass |
| Brightness (power centroid) | 600.0..3500.0 Hz | 1674.0 | 1674.0 | +1073.978 | pass |
| Width: L/R correlation above 200 Hz | 0.300..1.00 | 0.803 | 0.803 | +0.197 | pass |
| Time to -20 dB (expected 180 ms +-30 %) | 126.0..234.0 ms | 177.5 | 177.5 | +51.465 | pass |
| Centroid fall from 10 ms to 150 ms | >= 1.00 oct | 1.05 | 1.05 | +0.046 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -3.20 |  | +0.200 | pass |
| Alias floor (frozen modulation) | <= -35.00 dB | -42.69 |  | +7.689 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -52.81 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.009 |  | +0.991 | pass |
| Macro 1 "Ring" sweep | >= 0.400 | 1.21 C oct |  | +0.813 | pass |
| Macro 2 "Decay" sweep | >= 0.300 | 3.09 D oct |  | +2.791 | pass |
| Macro 3 "Brightness" sweep | >= 0.300 | 1.62 C oct |  | +1.325 | pass |
| Macro 4 "Echo" sweep | >= 0.300 | 0.938 C oct |  | +0.638 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | -0.054 |  | +1.946 | pass |
| Log-mel distance to the nearest pluck preset | >= 3.30 | 3.71 dB |  | +0.405 | pass |

## Pitch Drop Sub Pluck (pluck, Pluck)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 418.0 chars |  | +338.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -5.43 | -5.43 | +2.435 | pass |
| RMS over the hold, dBFS | -24.00..-18.00 dBFS | -21.27 | -21.27 | +2.733 | pass |
| LUFS-I over the hold | -24.00..-18.00 LUFS | -19.63 | -19.63 | +1.631 | pass |
| Crest (peak - RMS) | 10.00..20.00 dB | 10.70 | 10.70 | +0.696 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -0.267 | -0.267 | +1.733 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.970 | 1.00 | 1.00 | +0.030 | pass |
| Brightness (power centroid) | 150.0..3500.0 Hz | 317.4 | 317.4 | +167.437 | pass |
| Width: L/R correlation above 200 Hz | 0.300..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Time to -20 dB (expected 320 ms +-30 %) | 224.0..416.0 ms | 259.7 | 259.7 | +35.719 | pass |
| Centroid fall from 10 ms to 150 ms | >= 1.00 oct | 2.57 | 2.57 | +1.574 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -3.67 |  | +0.671 | pass |
| Alias floor (frozen modulation) | <= -50.00 dB | -67.84 |  | +17.845 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -76.21 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.029 |  | +0.971 | pass |
| Macro 1 "Drop Extra" sweep | >= 1.00 | 1.22 L LU |  | +0.223 | pass |
| Macro 2 "Length" sweep | >= 1.00 | 7.65 L LU |  | +6.653 | pass |
| Macro 3 "Click" sweep | >= 0.300 | 4.53 C oct |  | +4.234 | pass |
| Macro 4 "Drive" sweep | >= 1.00 | 2.00 L LU |  | +1.004 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 1.30 |  | +0.699 | pass |
| Log-mel distance to the nearest pluck preset | >= 3.30 | 6.34 dB |  | +3.041 | pass |

## Wide Chord Stab (pluck, Pluck)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 428.0 chars |  | +348.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -4.82 | -4.73 | +1.731 | pass |
| RMS over the hold, dBFS | -24.00..-18.00 dBFS | -22.57 | -22.57 | +1.432 | pass |
| LUFS-I over the hold | -24.00..-18.00 LUFS | -20.07 | -20.07 | +2.070 | pass |
| Crest (peak - RMS) | 10.00..20.00 dB | 12.12 | 12.12 | +2.116 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | -1.57 | -1.57 | +0.432 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.970 | 0.987 | 0.987 | +0.017 | pass |
| Brightness (power centroid) | 600.0..3500.0 Hz | 1279.3 | 1279.3 | +679.264 | pass |
| Width: L/R correlation above 200 Hz | 0.300..1.00 | 0.558 | 0.558 | +0.258 | pass |
| Time to -20 dB (expected 250 ms +-30 %) | 175.0..325.0 ms | 294.4 | 294.4 | +30.576 | pass |
| Centroid fall from 10 ms to 150 ms | >= 1.00 oct | 1.35 | 1.35 | +0.351 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -3.10 |  | +0.097 | pass |
| Alias floor (frozen modulation) | <= -12.00 dB | -15.56 |  | +3.564 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -14.57 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | 0.000 |  | +1.000 | pass |
| Macro 1 "Snap" sweep | >= 0.500 | 0.717 C oct |  | +0.217 | pass |
| Macro 2 "Decay" sweep | >= 1.00 | 4.97 L LU |  | +3.967 | pass |
| Macro 3 "Width" sweep | >= 0.100 | 0.552 W corr |  | +0.452 | pass |
| Macro 4 "Space" sweep | >= 0.100 | 0.524 W corr |  | +0.424 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| RMS against the median of its category in this run | -2.00..2.00 dB | 0.000 |  | +2.000 | pass |
| Log-mel distance to the nearest pluck preset | >= 3.30 | 3.71 dB |  | +0.405 | pass |

## Gated Triplet Call (seq, Seq)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 444.0 chars |  | +364.000 | pass |
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
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | 0.025 |  | +0.975 | pass |
| RMS against the median of its category in this run | - dB | 0.215 |  | - | info |
| Log-mel distance to the nearest seq preset | >= 3.30 | 4.95 dB |  | +1.652 | pass |

## Sixteenth Stutter Response (seq, Seq)

| Check | Target | 150 BPM | 140 BPM | Margin | Result |
|---|---|---|---|---|---|
| Loads without warnings | <= 0.000 | 0.000  |  | +0.000 | pass |
| Author is "Riddim Rack" | <= 0.000 | 0.000  |  | +0.000 | pass |
| Four labelled macros that each drive a slot | <= 0.000 | 0.000  |  | +0.000 | pass |
| No artist or product name in name or description | <= 0.000 | 0.000  |  | +0.000 | pass |
| Description of at least 80 characters | >= 80.00 | 489.0 chars |  | +409.000 | pass |
| No NaN or inf in the output | <= 0.000 | 0.000 | 0.000 | +0.000 | pass |
| Peak at velocity 100, dBFS | <= -3.00 dBFS | -5.04 | -5.09 | +2.040 | pass |
| RMS over the hold, dBFS | -18.50..-14.50 dBFS | -17.82 | -17.80 | +0.682 | pass |
| LUFS-I over the hold | -19.50..-13.50 LUFS | -16.48 | -16.49 | +2.981 | pass |
| Crest (peak - RMS) | 6.00..13.00 dB | 12.78 | 12.72 | +0.223 | pass |
| RMS within 2 dB of the category median | -2.00..2.00 dB | 0.182 | 0.196 | +1.804 | pass |
| Low-band (<120 Hz) L/R correlation | >= 0.990 | 1.00 | 1.00 | +0.010 | pass |
| Brightness (power centroid) | 250.0..1400.0 Hz | 1142.6 | 1103.4 | +257.354 | pass |
| Width: L/R correlation above 200 Hz | 0.850..1.00 | 1.00 | 1.00 | +0.000 | pass |
| Slot RMS pattern correlation to the steps | >= 0.900 | 0.986 | 0.988 | +0.086 | pass |
| Active slots above the slots at rest, dB | >= 10.00 dB | 12.72 | 13.31 | +2.715 | pass |
| Bar period of the gate pattern vs 1.6000 s (error) | -5.00..5.00 % | 0.027 | 0.031 | +4.969 | pass |
| Bar period ACF peak height | >= 0.500 | 0.990 | 0.991 | +0.490 | pass |
| Peak at velocity 127, dBFS | <= -3.00 dBFS | -5.04 |  | +2.040 | pass |
| Alias floor (frozen modulation) | <= -55.00 dB | -86.58 |  | +31.580 | pass |
| Alias floor, frozen and FX off (Info) | - dB | -86.72 |  | - | info |
| Frozen render within 1 dB RMS of live | -1.00..1.00 dB | -0.769 |  | +0.231 | pass |
| Macro 1 "Stutter Depth" sweep | >= 0.300 | 1.04 D oct |  | +0.735 | pass |
| Macro 2 "Metal" sweep | >= 1.00 | 1.66 L LU |  | +0.662 | pass |
| Macro 3 "Fill" sweep | >= 1.00 | 3.69 L LU |  | +2.693 | pass |
| Macro 4 "Shimmer" sweep | >= 1.00 | 1.14 L LU |  | +0.143 | pass |
| Repeatability: difference of two renders re signal | <= -80.00 | -300.0 dB |  | +220.000 | pass |
| LUFS against the median of the non-sub basses in this run (C5) | -1.00..1.00 LU | 0.051 |  | +0.949 | pass |
| RMS against the median of its category in this run | - dB | -0.215 |  | - | info |
| Log-mel distance to the nearest seq preset | >= 3.30 | 4.95 dB |  | +1.652 | pass |

## Distinctiveness (log-mel distance, dB; threshold 3.3 dB)

Same-family pairs are judged; other pairs are listed for calibration.

| A | B | Same family | Distance |
|---|---|---|---|
| Screech Formant Rise | Wide Hoover Stack | yes | 3.40 |
| Four Bar Downlifter | Noise Sweep Band | yes | 3.44 |
| Robot Bark Drop | Talking Chatter | no | 3.49 |
| Metal Gnash Fold | Pitch Rip Growl | yes | 3.61 |
| Metal Ring Pluck | Wide Chord Stab | yes | 3.71 |
| Call and Response Square | Wide Reese Chorus | no | 3.71 |
| Dotted Metal Stab | Triplet Bark AM | yes | 3.72 |
| Diving Screech | Wide Hoover Stack | yes | 3.78 |
| Pulse Width Pocket | Talking Chatter | no | 3.78 |
| Drift Random Wobble | Yoi Sweep Slow | no | 3.90 |
| Pulse Width Pocket | Vowel Pluck Bark | no | 3.92 |
| Call and Response Square | Dotted Metal Stab | yes | 3.99 |
| Pulse Width Pocket | Robot Bark Drop | yes | 4.03 |
| Eighth Triangle Wub | Sixteenth Gate Wub | yes | 4.15 |
| Talking Chatter | Vowel Pluck Bark | yes | 4.17 |
| Square Melody Lead | Wide Hoover Stack | yes | 4.20 |
| Metal Gnash Fold | Phased Grind | yes | 4.22 |
| Drift Random Wobble | Triplet Bark AM | no | 4.33 |
| Neuro Comb Fold | Wide Reese Chorus | yes | 4.34 |
| Call and Response Square | Neuro Comb Fold | no | 4.38 |
| Dotted Metal Stab | Yoi Sweep Slow | no | 4.38 |
| Sixteenth Gate Wub | Slow Sine Wub | yes | 4.39 |
| Robot Bark Drop | Vowel Pluck Bark | no | 4.39 |
| Vowel Pluck Bark | Init Square Bass | no | 4.40 |
| Call and Response Square | Eighth Triangle Wub | no | 4.54 |
| Triplet Bark AM | Yoi Sweep Slow | no | 4.55 |
| Dotted Metal Stab | Wide Reese Chorus | no | 4.55 |
| Liquid Dark Reese | Slow Sine Wub | no | 4.64 |
| Hollow Square Reference | Robotic Comb Chatter | yes | 4.67 |
| Robot Bark Drop | Robotic Comb Chatter | yes | 4.69 |
| Phased Grind | Pitch Rip Growl | yes | 4.69 |
| Hollow Square Reference | Triplet Saw Chop | no | 4.74 |
| Triplet Bark AM | Triplet Saw Chop | no | 4.79 |
| Drift Random Wobble | Metal Gnash Fold | no | 4.80 |
| Pulse Width Pocket | Yoi Sweep Slow | no | 4.83 |
| Dirty Sub Layer | Warm Tube Sub | yes | 4.83 |
| Eighth Triangle Wub | Slow Sine Wub | yes | 4.86 |
| Robot Bark Drop | Init Square Bass | no | 4.87 |
| Metal Gnash Fold | Robot Bark Drop | no | 4.87 |
| Pulse Width Pocket | Init Square Bass | no | 4.87 |
| Talking Chatter | Yoi Sweep Slow | yes | 4.90 |
| Diving Screech | Screech Formant Rise | yes | 4.91 |
| Metal Gnash Fold | Talking Chatter | no | 4.92 |
| Gated Triplet Call | Sixteenth Stutter Response | yes | 4.95 |
| Drift Random Wobble | Triplet Saw Chop | yes | 4.96 |
| Eighth Triangle Wub | Wide Reese Chorus | no | 4.99 |
| Vowel Pluck Bark | Yoi Sweep Slow | yes | 5.03 |
| Robot Bark Drop | Yoi Sweep Slow | no | 5.04 |
| Dotted Metal Stab | Triplet Saw Chop | no | 5.06 |
| Liquid Dark Reese | Wide Reese Chorus | yes | 5.07 |
| Dotted Metal Stab | Drift Random Wobble | no | 5.09 |
| Eighth Triangle Wub | Liquid Dark Reese | no | 5.10 |
| Call and Response Square | Triplet Bark AM | yes | 5.12 |
| Pitch Rip Growl | Vocal Growl Gnash | yes | 5.17 |
| Drift Random Wobble | Phased Grind | no | 5.17 |
| Sub Drop Impact | Pitch Drop Sub Pluck | no | 5.19 |
| Triplet Shredder | Vocal Growl Gnash | yes | 5.22 |
| Pitch Rip Growl | Triplet Shredder | yes | 5.26 |
| Wide Reese Chorus | Yoi Sweep Slow | no | 5.27 |
| Talking Chatter | Init Square Bass | no | 5.29 |

## Bank summary

41 preset(s) analysed; 40 use the FX chain; 16 use a reordered chain (the sheet asks 20 of 41 and 6 of 41 for the whole bank). Failed checks: 0.
