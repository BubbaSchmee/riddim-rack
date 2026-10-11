# Sound-quality checklist for a professional wavetable synth

Evidence tags: **[S]** stated in a primary source (URL given), **[D]** developer-forum consensus, **[E]** engineering convention; the researcher found no published standard. Where no industry number exists this file says so. Do not cite [E] numbers as "industry standard".

GTS v1 baseline: 64-harmonic additive tables, no mipmaps, no oversampling, no unison, mono default, tanh shaper, SVF + comb + AM filters.

## Sources
- Vital source, kMaxOversample = 8: https://github.com/mtytel/vital/blob/main/src/synthesis/framework/common.h
- Vital source, kMaxUnison = 16, detune / detune power / detune range / stereo spread / random phase inputs: https://github.com/mtytel/vital/blob/main/src/synthesis/producers/synth_oscillator.h
- Vital wavetable (kNumHarmonics = size/2+1, per-frequency-bin tables): https://github.com/mtytel/vital/blob/main/src/synthesis/lookups/wavetable.h
- KVR "Wavetable oscillator implementation" (Vital author mtytel, Urs/u-he, Vaneev, Richard_Synapse): https://www.kvraudio.com/forum/viewtopic.php?p=7139015
- Earlevel wavetable oscillator part 3 (1 table/octave, 2048 samples, 9 tables): https://www.earlevel.com/main/2012/05/09/a-wavetable-oscillator%E2%80%94part-3/
- Surge XT manual (16-voice unison, band-limited oscillators): https://surge-synthesizer.github.io/manual-xt/
- OB-Xf envelopes (attack from 1 ms, curve control, velocity amount): https://surge-synth-team.org/ob-xf/manual/envelopes/
- Cherry Audio VC envelope (0 or 2 ms minimum avoids retrigger pops): https://store.cherryaudio.com/modules/voltage-controlled-envelope
- Vital forum, click at envelope start: https://forum.vital.audio/t/click-at-start-of-envelope/8251
- Trigger modes: https://support.apple.com/en-is/guide/logicpro/lgsia125300/mac , https://steinberg.help/halion/v7/en/halion/topics/midi_modules/mono_envelope_parameters_r.html
- Preset levels: https://www.kvraudio.com/forum/viewtopic.php?p=4900736 , https://www.kvraudio.com/forum/viewtopic.php?p=7600217 , https://mixinggpt.com/blog/ultimate-guide-to-gain-staging-2026
- Bass design and mono sub: https://violetrecording.com/how-to-design-a-bass-sound/ , https://www.musicradar.com/news/6-synth-bass-mixing-tips , https://blog.dubspot.com/electronic-music-theory-analyzing-dubstep-basslines-datsiks-3-fist-style , https://forum.vital.audio/t/dubstep-downtempo-bass-tutorial-using-free-tube-distortion/11581
- Smoothing and denormals: https://docs.hise.dev/scriptnode/list/core/smoother.html , https://www.gvst.co.uk/gnormal_manual.htm

NOT found (do not claim): Serum's mip count or harmonics per level (Xfer publishes none; forum posts only guess it is similar to per-octave band-limiting), Vital's exact mip count and interpolation, any published alias-floor dB target, any published init-patch loudness standard, any published per-voice CPU budget.

## 1. Oscillator band-limiting
1. MUST band-limit every table read. At 48 kHz, C5 (523 Hz) with 64 harmonics reaches 33 kHz and folds; C7 is far worse. [S: Surge says its oscillators are strictly band-limited]. GTS v1: FAILS, no mipmaps.
2. MUST use mip-mapped tables chosen by pitch (phase increment). Octave spacing is the documented floor: mtytel's scheme is 11 levels for 2048 samples (fundamental only, then 2, 4, 8... harmonics up to full) [D, KVR]; Earlevel uses 1 table per octave, 2048 samples, 9 tables for a saw [S]. GTS v1: FAILS.
3. SHOULD use 2-4 tables per octave when RAM allows (Urs/u-he: 3-4 per octave with no oversampling; another well-known synth uses 2 per octave with band edges meeting near 17 kHz at 44.1 kHz) [D]. GTS v1: n/a.
4. SHOULD cut each level so the highest kept harmonic is at or below about 17-18 kHz (Urs: above ~18 kHz is easy to keep out; Earlevel uses one-third of the sample rate) [D]. GTS v1: n/a.
5. MUST scale mip selection with sample rate (Vital rescales phase increment per power of two above 44.1 kHz) [S]. GTS v1: verify.
6. SHOULD use 2048-sample frames (Vital, Earlevel) [S]. GTS v1: check frame length.
7. MUST use at least cubic (4-point Hermite) interpolation; linear only if tables are 2x oversampled (Vaneev: 4-6 point spline is enough when oversampled) [D]. NICE: long sinc (>70 taps) for premium/offline mode [D]. GTS v1: verify; linear = FAIL.
8. SHOULD pick one mip per note or fade once per block when switching; mtytel suggests ~50 ms fades on table change [D]. GTS v1: n/a.
9. MUST add an alias-floor regression test on a saw sweep C5-C7. Target of <= -80 dB re fundamental is an [E] engineering target: no published standard exists (matt42's -60 dB question on KVR went unanswered; the Serum "~-150 dB" figure is unverified). GTS v1: untested.
10. SHOULD store frame spectra (or per-octave mips) for tables with hundreds of frames, as RAM forces (mtytel, Urs) [D]. GTS v1: verify morph path.

## 2. Oversampling of nonlinear stages
11. MUST oversample hard nonlinearities (tanh, fold, clip, drive in filter feedback) at 2x minimum, 4x for heavy folding. [E]; Vital supports up to 8x (kMaxOversample = 8) [S]. GTS v1: FAILS, tanh at 1x.
12. SHOULD expose a quality switch (1x / 2x / 4x, offline render may go higher). Vital exposes oversampling in its Advanced view [D]. GTS v1: no switch.
13. SHOULD use a half-band polyphase FIR/IIR for up and down conversion (KVR consensus) [D]. GTS v1: none.
14. SHOULD oversample only the nonlinear block and bypass it when drive = 0 [E].
15. NICE: antiderivative anti-aliasing (ADAA) for tanh/clip as a cheaper option [E, not verified here].

## 3. Unison, detune, width
16. MUST offer unison; 16 voices is the shared ceiling (Vital kMaxUnison = 16 [S]; Surge XT up to 16 [S]). Init = 1, typical bass presets 3-7 [E]. GTS v1: FAILS.
17. MUST give detune amount plus detune curve/power and range (Vital exposes all three) [S]. Useful bass range about 5-40 cents [E]. GTS v1: FAILS.
18. MUST give stereo spread and a blend/centre control (Vital stereo_spread, stereoBlend) [S]. GTS v1: FAILS.
19. MUST give random-phase amount with a phase-reset option for punchy bass (Vital random_phase) [S]. GTS v1: FAILS.
20. MUST compensate gain for voice count (about 1/sqrt(N) for incoherent voices) [E]. GTS v1: n/a.
21. Why single voices read thin: one phase-coherent voice has no inter-voice beating and a narrow, static image; unison detune adds movement and decorrelation [E, consistent with bass guides]. GTS v1: mono default is right for the sub layer only.

## 4. Sub oscillator and bass norms
22. MUST have a dedicated sine sub, mono and undistorted, with nothing else competing below ~100 Hz [S: violetrecording, Dubspot]. GTS v1: verify.
23. MUST keep content below 100-150 Hz mono (sources disagree: 100 Hz general, 120 Hz Ableton Utility, 150 Hz vinyl) [S/D]. Implement a mono crossover, default 120 Hz, adjustable. GTS v1: mono default fits, no crossover.
24. MUST align sub and mid phase (reset start phase on note-on or a phase-offset knob) to avoid low-end cancellation [E]. GTS v1: verify.
25. SHOULD high-pass the mid layer (~100-150 Hz) and low-cut the sub near 20-30 Hz [E]. GTS v1: verify.
26. SHOULD offer glide with legato and a pitch envelope for sub punch [E]. GTS v1: verify.

## 5. Envelopes
27. MUST enforce a minimum attack of 1-2 ms and a few ms of release: OB-Xf attack starts at 1 ms [S]; Cherry Audio min 0 or 2 ms "avoids pops" [S]; Vital users report clicks at 0 ms attack [S, forum]. GTS v1: verify.
28. MUST default to exponential/analog-style segments with an adjustable curve (OB-Xf curve, Cherry log/exp/lin, HALION curve) [S]. GTS v1: verify; linear = FAIL.
29. MUST provide retrigger modes: legato/first-note vs each-note (Logic ES2 Mono/Retrig, HALION First/Each Note) [S]. SHOULD restart from the current level, not zero (Prophet-5 behaviour per KVR) [D].
30. SHOULD have velocity-to-amplitude amount (OB-Xf) with an optional curve; velocity-to-time is NICE (HALION) [S].
31. SHOULD apply a 2-5 ms declick on voice steal [E].

## 6. Filters
32. MUST have a modelled SVF (LP/BP/HP/notch) with drive in the loop, 12 and 24 dB/oct, and key-tracking [E from common practice in Vital, Serum, Surge; no single manual requires it]. GTS v1: SVF present; verify drive and 24 dB.
33. MUST stay stable through self-oscillation (clamp Q, soft-clip state), with a clean sine at cutoff [E]. GTS v1: verify.
34. SHOULD provide comb, formant/vowel and phaser-style modes (Vital has them) [D]. GTS v1: comb present, formant missing.
35. NICE: analog drift and noise (slight per-voice pitch/cutoff jitter, low noise floor) [E, no published figure]. GTS v1: none.
36. MUST scale cutoff in octaves, not linear Hz [E]. GTS v1: verify.

## 7. Level and headroom
37. MUST calibrate presets to a band. Practitioner convention: about -18 dBFS RMS working level and -6 to -3 dBFS peaks (mixing guides) [D/E]; no vendor standard exists, and KVR complains that factory presets peaking at 0 dBFS is the industry habit. Recommended: init peak -12 to -6 dBFS; factory presets peak <= -3 dBFS, RMS -24 to -14 dBFS, +-2 dB spread across the bank [E]. GTS v1: FAILS (no measured calibration).
38. MUST keep a master volume that persists across presets (or per-preset normalisation) and a soft-clip/limiter at the output [E]. GTS v1: verify.
39. MUST compensate gain for unison and resonance [E]. GTS v1: n/a.
40. SHOULD ship a script that renders every preset and fails outside the item 37 band [E].

## 8. Click-free changes, DC, denormals
41. MUST smooth audible parameters per sample with a one-pole: about 5-30 ms for gain/pan, 5-20 ms for cutoff/pitch [E; HISE smoother default is 100 ms, range 0-2000 ms]. GTS v1: verify; per-block stepping = FAIL.
42. MUST DC-block after asymmetric nonlinear stages (one-pole HPF, R about 0.995-0.999) [E]. GTS v1: verify.
43. MUST set flush-to-zero / denormals-are-zero on the audio thread and add a tiny offset in feedback loops [S: GNormal docs; standard practice].
44. SHOULD smooth wavetable-position changes [D].

## 9. CPU budgets (all [E]; none published)
45. SHOULD keep one voice (osc + SVF + shaper at chosen oversampling) under about 1% of a core at 48 kHz on a 2020-class laptop, so a 16-voice patch stays under ~25-30%. MUST record a benchmark per quality tier. GTS v1: unmeasured.
46. MUST not allocate or lock on the audio thread; SIMD encouraged (Vital uses SSE poly_float) [S].

## 10. Riddim / dubstep bass design
47. MUST support layering: clean mono sub + heavily processed mid + optional top for small speakers [S: violetrecording, Dubspot, MusicRadar].
48. MUST allow the mid layer to be distorted without touching the sub (per-layer FX or multiband) [S: MusicRadar, Vital forum tutorial].
49. Belongs INSIDE the synth: oscillators, unison, FM/AM/ring, wavetable and formant/comb filters, oversampled waveshaper, per-layer level/pan, sub, tempo-synced LFO/envelope movement, mono-ise below crossover [E].
50. Belongs AFTER the synth (or in a bundled FX chain): OTT/multiband compression, EQ cleanup of resonances, saturation glue (mild tube about 10% per the Vital forum tutorial), reverb/delay, final limiter [S/E].
51. NICE: vowel-morph filter with LFO/macro for talking riddim (tutorials cite comb/formant) [S].
52. NICE: role-tagged presets (Sub, Growl, Riddim, Screech), macros named for what they do, and a signal-flow/scope view for readability [E].

## Biggest GTS v1 gaps
No mip-mapped band-limiting, no oversampling on the shaper, no unison/width/random phase, no output calibration to a loudness band, no measured CPU or alias tests. Rebuild presets as role-tagged layers (sub / mid / top) at the item 37 levels.
