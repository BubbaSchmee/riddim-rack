# Built-in FX section standard (GTS) - fact-checked checklist

Researched 2026-10-11. Tags: MUST / SHOULD / NICE. Evidence: [S] stated in source, [S-3rd] stated in a third-party article, [D] developer-forum consensus, [E] engineering convention (our judgement, not sourced). Products: Serum 1/2 (Xfer), Vital, Phase Plant (Kilohearts), Pigments (Arturia), Massive X (NI). Coverage is uneven: Serum 1 manual, Vital guide mirror and Massive X Stereo Effects page were read in full; Serum 2, Phase Plant and Pigments were NOT read from a manual (see "Could not be verified").

## Sources
- Serum 1 manual (PDF, FX chapter p22-28): https://s3.amazonaws.com/decembercymatics/Serum_Manual.pdf
- Vital user guide, Effects (third-party mirror): https://davidmvogel.com/docs/Vital/UserGuide/Effects
- Massive X manual, Stereo Effects: https://docs.native-instruments.com/online-guides/massive-x-manual/en/stereo-effects ; Insert Effects: https://native-instruments.com/ni-tech-manuals/massive-x-manual/en/insert-effects ; Routing: https://native-instruments.com/ni-tech-manuals/massive-x-manual/en/routing
- Kilohearts Snapins index (Phase Plant effects are Snapins): https://kilohearts.com/docs/snapins ; Phase Plant lanes: https://kilohearts.com/docs/phase_plant
- Serum 2 effects walkthrough (third party): https://monosounds.studio/serum-2-effects-guide/
- Pigments manual (not read): https://www.arturia.com/api/download/products/pigments/manual/pigments_Manual_7_0_1_EN.pdf ; support page (403 to fetch): https://support.arturia.com/hc/en-us/articles/4405741074962-Pigments-Effects
- OTT: https://www.audiotent.com/production-tips/ott-multiband-compression ; https://rekkerd.org/xfer-records-updates-ott-multiband-compressor-effect-v1-2
- Riddim/dubstep in-synth FX: https://bassgorilla.com/serum-tutorial-snails-bass-hybrid-trap-dubstep-bass/ ; https://www.musicradar.com/how-to/a-quick-guide-to-xfer-records-serums-effects ; https://musicradar.com/how-to/how-to-use-serums-effects-to-boost-a-bass-sound

## 1. Which modules ship

| Module | Serum 1 | Serum 2 | Vital | Phase Plant | Pigments | Massive X | Tag |
|---|---|---|---|---|---|---|---|
| Distortion | 13 types [S] | yes, + Overdrive mode [S-3rd] | 6 types [S] | Snapin, 6 types [S] | yes (list unread) | Nonlinear Lab: Hard/Soft/Overdrive + cabinets [S] | MUST |
| Multiband comp (up+down) | "Multiband" switch [S] | carried over | 4 modes [S] | Snapins (Multipass/Compactor, unread) | Multiband Compressor [S-3rd] | Multi Compressor [S] | MUST |
| Chorus | 4-voice [S] | yes | 4-16 voices [S] | Snapin, Taps [S] | Jun-6 Chorus [S-3rd] | Quad Chorus [S] | MUST |
| Dimension / widener | Dimension + Hyper [S] | yes | none | Stereo Snapin (Width/Mid) [S] | unverified | Dimension Expander + Stereo Expander [S] | MUST (one of) |
| EQ | 2-band, no Mix [S] | yes | 3-band +-15 dB [S] | 3-Band EQ, Filter [S] | yes | +-24 dB [S] | MUST |
| Delay | sync, ping-pong [S] | yes | Mono/Stereo/PingPong/MidPingPong [S] | Snapin: sync, ping-pong, duck [S] | Pitch-shifting delay [S-3rd] | Stereo Delay [S] | MUST |
| Reverb | Plate (Tal-derived) [S] | + 3 new types [S-3rd] | single algo [S] | Snapin [S] | yes | 17 modes [S] | MUST |
| Flanger | yes [S] | yes | yes [S] | Snapin [S] | BL-20 [S-3rd] | 7 modes [S] | SHOULD |
| Phaser | yes [S] | not confirmed | yes [S] | Snapin [S] | not confirmed | yes [S] | SHOULD |
| Filter FX | same as voice filter [S] | yes | yes [S] | Filter/Ladder Snapins [S] | Filter/EQ category [S-3rd] | n/a (insert) | SHOULD |
| Hyper (unison micro-delay chorus) | 1-7 voices [S] | yes | no | no | no | no | NICE |
| Bit/sample reduction | inside Distortion (Downsample) [S] | same | Bit crush, Down sample in Distortion [S] | Bitcrush Snapin [S] | unverified | Bit Crusher insert [S] | SHOULD |
| Frequency shifter | no | Bode [S-3rd] | no | Snapin [S] | no | no | NICE |

At least two products with [S] for: distortion, multiband comp, chorus, EQ, delay, reverb, flanger, phaser, filter FX. Widener: Serum 1, Massive X, Phase Plant [S].

### 1a. Distortion
- MUST [S] Mode menu with >=6 types. Found: Vital = soft clip, hard clip, linear fold, sine fold, bit crush, down sample; Kilohearts = overdrive, saturate, foldback, sine, hard clip, quantize; Massive X = Hard Clip, Soft Clip, Overdrive, each with HP-pre and Comp variants; Serum 1 = 13 types, default "Tube", manual names "Downsample", "X-Shaper", "X-Shaper (Asym)". A Serum tutorial uses "Diode-2" [S-3rd]. The full Serum list is not in the manual text; do not copy names from memory.
- MUST [S] Drive. Vital -30..+30 dB. Serum unitless (on Downsample it sets rate reduction; on X-Shaper it morphs shape A to B).
- MUST [S] Mix 0-100% (Vital 0-1; Kilohearts, Massive X Mix).
- MUST [S] Filter Off/Pre/Post: Serum (LP/BP/HP morph, F, Q); Vital (None/Pre/Post, cutoff 8-136 semitones, resonance 0-100%, LP/HP/BP/peak). Two products agree.
- SHOULD [S] Bias/asymmetry for even harmonics: Kilohearts Bias + Spread; Serum "X-Shaper (Asym)" is documented for even-order harmonics.
- SHOULD [S] Dynamics/compensation: Kilohearts Dynamics param; Massive X "Comp" drive variants.
- NICE [S] User waveshaper (Serum X-Shaper A/B editor).
- Character [S-3rd monosounds]: Serum 2 Overdrive "compresses into saturation"; hard clip and tube are "better suited to real aggression". Folds and bit-reduction are the harsh digital end [E].

### 1b. Compressor / OTT-style
- MUST [S] Up AND down compression in a multiband mode. Serum 1 Multiband switch = "multiband up/downwards compressor", bands not user-adjustable. Vital: Multiband / Low Band / High Band / Single Band, threshold -79..-1 dB shared by bands, upward below the lower threshold. Massive X Multi Compressor: Up and Down 0-200% (default 100%), per-band (low/mid/high) up/down thresholds and Out, XLow/XHigh crossovers, Attack, Release, In/Out Gain -24..+24 dB, Mix 0-100%.
- MUST [S] Single-band params (Serum): Thresh (0 = 0 dB, 100% = -120 dB), Ratio (typical 2:1-4:1; at max it becomes a true-peak "Limit"), Attack ms, Release, Gain makeup (~36 dB), Mix. Kilohearts: Attack, Release, RMS/Peak, Ratio, Threshold, Makeup, Sidechain.
- SHOULD [S] Vital attack/release are normalized 0-100% (no ms). For GTS expose ms plus a Time macro [E].
- OTT facts [S-3rd]: Xfer OTT = Depth (changelog: ratio scaling, 0% = 1:1 = bypass), Time (scales attack/release of all bands), In/Out gain, per-band output trims, per-band thresholds (orange upward, green downward); "based on the Ableton Multiband Compressor preset". The classic OTT default numbers and crossover frequencies were NOT verified. Descriptions of Depth conflict (dry/wet vs ratio scaling).
- Riddim usage [S-3rd bassgorilla]: Serum multiband comp gain ~13.6 dB, threshold -16.7 dB; external OTT preset outputs High +10.6 dB, Low +16.3 dB, mid unchanged.

### 1c. Chorus / flanger / phaser / dimension / hyper
- Chorus MUST [S]: Rate (Serum 0-20 Hz or sync to 1/32; Vital 0.125-64 Hz or tempo 32/1-1/16), Depth (Vital 0-100%), Delay 1/2 (Vital 0.976-19.99 ms; Serum Delay 1/2 in ms), Feedback (Vital -95..+95%; Serum Feed), Voices (Vital 4-16; Serum fixed 4 = 2L+2R; Kilohearts Taps), Mix 0-100%, post-LPF (Serum LPF Hz; Vital filter cutoff 8-136).
- Flanger SHOULD [S]: Rate (Serum 0-20 Hz / sync; Vital 0.25-32 Hz or tempo), Depth 0-100%, Feedback (Vital -100..+100%), stereo phase (Serum Phase 0-50% = 0-180 deg; Vital Offset 0-360 deg), Center (Vital 8-136 semitones), Mix.
- Phaser SHOULD [S]: Rate (as flanger), Depth (Vital 0-48 semitones), Freq/Center, Feedback 0-100%, Phase/Offset, Blend 0-2 (Vital), Mix. Massive X stages 2/4/5/6/8.
- Dimension MUST(one widener) [S]: Serum Dimension = 4 delay lines summed out of phase and slowly amplitude-modulated; params Size + Mix only; "perceived width to an otherwise mono signal". Massive X Dimension Expander: Modes 1-4, Amount, Bright, Stereo. Kilohearts Stereo: Mid, Width, Pan (needs stereo input).
- Hyper NICE [S]: Serum Hyper = micro-delay chorus, 1-7 voices, Rate, Detune, Retrig, Unison, Mix; set Unison to 0 to use Dimension alone.

### 1d. EQ
- MUST [S] Vital 3 bands (low/band/high), gain +-15 dB, cutoff 8-136 semitones, resonance 0-100%. Serum 2 bands: low = LowShelf/Peak/HiPass, high = HighShelf/Peak/LowPass, Freq/Q/Gain each, no Mix. Massive X: high shelf 1.2-23.6 kHz, mid 90 Hz-14 kHz, gains +-24 dB (low -36..+18). Kilohearts 3-Band EQ.
- MUST [S] a low-cut option (Serum HiPass band; reverb low cut in Serum and Vital).

### 1e. Delay
- MUST [S] Tempo sync with dotted/triplet: Vital 4/1-1/64 Tempo/Dotted/Triplet, free 0.001-4 Hz; Serum per-side time with scalar (1.333 TRIP, 1.5 DOT); Kilohearts ms or beat divisions.
- MUST [S] Ping-pong: Serum Normal/Ping-Pong/Tap->Delay; Vital Mono/Stereo/Ping Pong/Mid Ping Pong; Kilohearts Ping-Pong; Massive X Route Parallel/Cross.
- MUST [S] Feedback (Vital -100..+100%) and Mix 0-100%.
- MUST [S] Feedback filter: Vital cutoff 8-136 + spread 0-1, AP/LP/HP/BP; Serum F + Q.
- SHOULD [S] L/R Link (Serum), Duck (Kilohearts).

### 1f. Reverb
- MUST [S] Size, Decay, Damp, Pre-delay, Mix. Serum: Size, Pre Delay ms, Low Cut %, Damp %, Hi Cut %, Width %, Mix (Plate). Vital: Time 0.015-64 s, Size 0-100%, Delay 0-0.3 s, pre low/high cut 0-128 semitones, feedback-EQ shelf -6..0 dB, chorus amount 0-100%, chorus freq 0-8 Hz, Mix. Kilohearts: Decay, Dampen, Size, Width, Early, Mix. Massive X: 17 modes, Size, Color, Delay, Chorus, Mix.
- MUST [S] Low cut inside the reverb (Serum, Vital).
- SHOULD [S] More than one type: Massive X 17 modes; Serum 2 original + 3 new types (names not in source).

## 2. Chain architecture
- Serum 1 [S]: fixed set of 10 modules, re-orderable by dragging name tiles, signal top-to-bottom, left-edge enable switch (hides module) plus per-module bypass power button, Mix on every effect except EQ. FX run on the summed output ("like plugin inserts after Serum"), so per-voice mod sources retrigger FX params. Default order is not stated in the manual.
- Serum 2 [S-3rd monosounds]: multiple FX buses, each oscillator routable to any bus, reorder by drag in FX view only, duplicates allowed, splitters for multiband. That article also says Serum 1 had a "fixed order" rack, which contradicts the Serum 1 manual (re-orderable); trust the manual.
- Vital: per-effect enable and Mix; ordering not described in the guide page [unknown].
- Massive X [S]: 3 Insert slots A/B/C (per voice, placeable in voice structure) and 3 Stereo slots X/Y/Z (global on summed voices), routing X>Y>Z, X+Y>Z or X+Y+Z.
- Phase Plant [S]: 3 effect lanes, each holding several Snapins, lane Mix/Gain/Mute/Solo/Poly, lanes serial or parallel via "Send to". Mix is per lane, not per Snapin. In-lane order not stated.
- Pigments [S-3rd]: Bus A and Bus B (3 effects each) routable A>B, B>A, A//B, plus Aux bus with Send/Return and Pre/Post, 20 effect types per slot (older FAQ says 18). Manual unread.
- Pattern: re-orderable chain = SHOULD (Serum, Pigments, Massive X routing, Phase Plant); per-module enable + Mix = MUST [S]; send-style buses exist (Pigments Aux, Phase Plant parallel lanes) = NICE.
- MUST [E] FX state (all params, enable flags, order) saved in the preset.
- Oversampling inside distortion: NOT stated by any fetched source (Serum, Vital, Kilohearts, Massive X). MUST [E] anyway: clippers and folders alias measurably. Use 2-4x polyphase/halfband, ADAA optional.

## 3. Level handling
- Serum compressor makeup ~36 dB and Ratio max switches to a true-peak limiter [S].
- Auto gain compensation in distortion: no source states automatic level matching. Kilohearts "Dynamics" preserves dynamic range, Phase Distortion "Normalize" reduces input-level sensitivity, Massive X has Comp variants [S]. SHOULD [E] output trim plus optional auto-gain so Drive changes move loudness no more than ~3 dB.
- Output limiter: no source documents a master safety limiter on Serum/Vital/Massive X (Serum's is the compressor Limit mode). SHOULD [E] always-on final limiter, ceiling -0.3 dBFS, plus NaN/Inf guard.
- Threshold mapping differs: Serum 0..100% = 0..-120 dB; Vital -79..-1 dB.

## 4. What riddim / dubstep tutorials do inside the synth FX
No riddim-specific tutorial with full settings was found. All below is [S-3rd] unless noted.
- Bassgorilla hybrid dubstep Serum patch: Hyper mix 25; Dimension size 11, mix ~45; Distortion "Diode-2" drive 37; Flanger depth 51% with LFO mod; Phaser frequency LFO-modulated (~232 Hz); EQ peak +9.4 dB at ~113 Hz; multiband comp gain 13.6 dB, threshold -16.7 dB. Outside the synth: OTT preset; reverb decay ~1.22 s, ~21% wet, low cut applied; EQ Eight with 4x low cut ~137 Hz and -15 dB cuts at 197 Hz and 449 Hz.
- MusicRadar Serum guide: Drive/Morph ~30% is crunchier but loses some sub; low-pass ~300 Hz keeps bass warm.
- Serum 2 guide (monosounds): sub on a clean bus with EQ only; process the top layer with distortion first; distortion before filter = smoother, after filter = resonant peaks louder; EQ directly after distortion; compression and delay before reverb; reverb under ~15% on bass with a tight type; Overdrive 25-40% for weight without fizz; example bass chain Overdrive > EQ > Compressor > Flanger.
- DNB Academy lists a module "Using Flangers & Phasers to Create Riddim Bass", no settings: https://edmprod.teachable.com/courses/1135170
- Consensus used for GTS [E, supported above]: distortion, then OTT-style multiband, then EQ; width (chorus/dimension/hyper) only above the sub; reverb with low cut and low mix; sub never through width or reverb. NOT verified: OTT depth number on riddim bass (common claim 50-100%, unsourced).

## Could not be verified
- Serum 2 manual (404): module list, reverb type names, distortion list (including any "Sine Fold" mode), default FX order. Serum 1 default FX order and full 13 distortion names.
- Phase Plant/Kilohearts docs for Compressor/Chorus/Delay/Reverb ranges (sub-pages 404); oversampling or limiter behaviour in Kilohearts.
- Pigments effect types and ranges (support page 403, PDF unread). Massive X numeric ranges for Delay/Reverb/Chorus/Flanger/Phaser (not given).
- OTT default values and crossover frequencies. Vital chain ordering and oversampling. Any product's documented auto-gain in distortion.

## Recommendation: minimal GTS FX chain a Serum user would not call missing

Default order (user drag-reorderable, per Serum/Massive X): Distortion > Multiband Comp (OTT) > EQ > Chorus/Dimension > Delay > Reverb > Limiter. Optional: Flanger, Phaser, Filter, Hyper. Every module: Enable, Mix 0-100% (EQ none), state stored in the preset, FX on summed signal.

| Module | Params and ranges | Tag |
|---|---|---|
| Distortion | Type (>=8: Tube/Soft clip, Hard clip, Overdrive, Diode, Linear fold, Sine fold, Bit crush, Downsample); Drive -12..+36 dB (Vital spans +-30); Bias -1..1; Filter Off/Pre/Post, cutoff 20 Hz-20 kHz, Q 0.3-8, LP/BP/HP; Out trim -24..+12 dB; Auto-gain on/off; Mix; internal 4x oversampling | MUST |
| Multiband comp (OTT) | Depth 0-200% (default 100%, 0% = 1:1); Time 0-100% scaling attack 1-100 ms / release 20-500 ms; crossovers 120 Hz and 2.5 kHz (ranges 40-400 Hz, 1-8 kHz); per-band up/down threshold -60..0 dB, band Out -12..+12 dB; In/Out +-24 dB; Mix. Plus Single-band mode: Thresh -60..0 dB, Ratio 1:1-20:1 (+Limit), Attack 0.1-100 ms, Release 5-1000 ms | MUST |
| EQ | 3-4 bands: Low shelf/HP, Peak (20 Hz-20 kHz, Q 0.2-10), High shelf/LP; gain +-15 dB (Vital) to +-24 dB (Massive X); no Mix | MUST |
| Chorus | Voices 2-8, Rate 0.1-10 Hz or synced, Depth 0-100%, Delay 1-20 ms, Feedback -95..95%, post-LPF 200 Hz-20 kHz, wet low cut 20-500 Hz, Mix | MUST |
| Dimension | Size 0-100%, Mix 0-100%, wet low cut 20-500 Hz (our addition, guarantees mono sub) | MUST (one widener) |
| Delay | Time L/R 1 ms-2 s or sync 1/64-4/1 with dotted/triplet; Feedback -100..+100% (effective cap 95%); Mono/Stereo/Ping-pong; feedback HP+LP 20 Hz-20 kHz; Link; Mix | MUST |
| Reverb | Type >=2 (Plate, Hall; Room SHOULD); Size 0-100%; Decay 0.1-20 s; Damp 0-100%; Pre-delay 0-300 ms; Low cut 20-1000 Hz (default 200 Hz); Hi cut; Width 0-100%; Mix | MUST |
| Flanger/Phaser | Rate 0.05-20 Hz or sync, Depth, Feedback -95..95%, Phase/Offset 0-360 deg, Center/Freq, Mix | SHOULD |
| Output limiter | Ceiling -0.3 dBFS, always on; Gain -24..+12 dB | SHOULD |

Most ranges come from Vital/Serum/Massive X [S]; OTT time ranges, crossovers, limiter ceiling are [E].

### Measurable acceptance tests (all [E]; offline DSP harness, 48 kHz)
1. Distortion THD: Hard clip, 100 Hz sine at -12 dBFS, Drive +12 dB, Mix 100%, oversampling on: THD+N 15-40%. Soft clip at Drive 0 dB: THD < 5%. Mix 0% or module off: THD+N < -90 dB and bit-transparent. Aliasing: 5 kHz sine at +12 dB drive, non-harmonic partials >= 60 dB below fundamental with oversampling on.
2. Auto-gain: with Auto-gain on, RMS change across Drive 0..+24 dB for pink noise at -18 dBFS within +-3 dB.
3. Compressor gain reduction: 1 kHz sine, Thresh -20 dB, Ratio 4:1, input -8 dBFS (12 dB over): GR = 9.0 dB +-0.5 dB; time to 63% of final GR within +-20% of set Attack; same for Release.
4. OTT: Depth 0% (1:1) gives output equal to input within 0.1 dB and null < -60 dB; crossover sum flat within +-0.2 dB. Depth 100% on -30 dBFS tones at 100 Hz/1 kHz/8 kHz: each raised >= 6 dB; a -3 dBFS burst reduced >= 3 dB.
5. Chorus stereo correlation: mono 220 Hz saw, Mix 50%, Rate 0.5 Hz, Depth 50%, >=4 voices: L/R correlation over 5 s between 0.2 and 0.8; Mix 0% gives exactly 1.0; sub band (< 120 Hz) correlation >= 0.95 with wet low cut at default.
6. Dimension: S/M energy ratio rises >= 6 dB above 300 Hz at Mix 50% and < 1 dB below 100 Hz.
7. Delay time accuracy: impulse, Time 250 ms and sync 1/8 at 140 BPM (214.29 ms): first echo within +-1 sample; ping-pong alternates channels; Feedback 50% gives echo ratio 0.5 +-0.02; dotted = 1.5x, triplet = 0.667x.
8. Reverb RT60: Decay 2.0 s, Damp 0: Schroeder-integrated RT60 (T20 extrapolated) 2.0 s +-15%; wet low cut 200 Hz puts energy below 100 Hz at least 20 dB under the unfiltered case; Mix 0% nulls to dry.
9. Chain: all modules on, white noise -6 dBFS, extreme settings: output never exceeds ceiling, no NaN, no denormal CPU spike; preset save/load round-trips every param plus module order and enable flags bit-exact.
