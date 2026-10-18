# Factory bank recipes: 41 presets for the riddim / dubstep bank

Written 2026-10-17. Planning only: nothing here is built. It turns the bank plan in
`docs/standards/riddim-dubstep-fundamentals.md` section 4 into one recipe sheet per preset, voiced for the engine
as it will be after SPEC-v2 Track 7 (Tracks 1, 4, 5, 7). The preset QA renderer reads the targets; if a preset
cannot meet its targets, report it and revoice the preset. Do not loosen the number.

Engine assumed: 9 tables (Basic, Vowel, FM Morph, Metal, Reed, Grit, Square Morph, Saw Stack, Harmonic Rise);
per-oscillator Warp (Off, FM from other osc, Sync, Bend+, Bend-, PWM); four LFOs (A-D) with Custom (up to 32
points) and Step (2-32 steps, glide) shapes, dotted and triplet sync, start phase, fade-in, smooth, trigger mode
(Retrigger, Free, Envelope); Mod Env (ADSR with curve); pitch targets (osc A pitch, osc B pitch, pitch_all, +-48 st);
filters LP12, LP24, HP12, BP12, Notch, Comb+, Comb-, AM, Formant (two filters, Parallel or Series); noise layer
(White, Pink, Brown); sub (Sine or Triangle, octave 0/-1/-2, phase, drive); FX chain (Distortion, Multiband,
EQ, Phaser, Chorus, Flanger, Dimension, Delay, Reverb, Limiter fixed last, reorderable); 32-slot matrix; 4 macros.

Conventions. Bass demo notes F1, G1, A#1 (floor E1). F1 43.65 Hz, G1 49.0, A#1 58.27. Tempo 140 and 150 BPM.
All presets: author "Riddim Rack", four purpose-labelled macros, descriptive unique name, no artist or product names.
"Mono" means poly 1, legato on. "Sub" is the sub oscillator (always clean unless stated). Filter and FX values are
design starting points; the QA numbers are the contract.

---

## A. QA definitions (what the renderer measures)

Render. Each preset is rendered offline at 48 kHz, velocity 100 (and 127 for the peak check), host tempo 140 and
again at 150. Bass: held note F1, A#1 (and E1 for the floor check), hold 2 bars (3.43 s at 140, 3.20 s at 150) plus
release tail, one note at a time. Lead, Seq, Pluck and FX use the family note in the table. The first 50 ms are
excluded from loudness and spectral stats (attack). Macros rest at their stored value unless a sweep is being tested.
Presets with a period of a bar or longer are rendered for the longer of 2 bars or 2.2 periods.

| Metric | Definition |
|---|---|
| Peak | Max abs sample after the limiter, dBFS. |
| RMS | RMS over the hold window, dBFS, 20 Hz to 20 kHz, both channels. |
| LUFS-I | K-weighted integrated loudness of the hold window. |
| Crest | Peak minus RMS, dB. |
| Low corr | Pearson correlation of L and R after a 4th-order Linkwitz-Riley low-pass at 120 Hz. |
| Low S/M | Side-to-mid energy ratio below 120 Hz, dB (mid = (L+R)/2). |
| Sub share | Energy fraction below 150 Hz of the whole spectrum (subs only). |
| Brightness | Power-weighted spectral centroid, Hz, 20 Hz to 20 kHz, 4096-sample frames, mean over the hold. |
| Move centroid | Spectral centroid of the 120 Hz to 8 kHz band, 1024-sample window, hop 256 (zero padded to 4096), as a time series. |
| Move depth | (p95 - p5) of log2(Move centroid) over the hold, octaves. |
| Move period | Lag of the largest autocorrelation peak (lag 30 ms to 1 s, or the stated lag) of the detrended Move centroid. It must fall within +-5 % of the expected period and the peak must be >= 0.5 (>= 0.4 for periods under 80 ms). |
| Width | Pearson correlation of L and R after a 4th-order Linkwitz-Riley high-pass at 200 Hz (300 Hz for leads), mean over the hold. |
| Alias floor | Inharmonic energy (outside +-1 % of k times f0 after the note settles) relative to the fundamental, dB, 0 to 20 kHz. Measured on a frozen-modulation render (LFO shapes, macros and Mod Env held at their rest value) because modulation makes the spectrum non-stationary. The frozen render must be within 1 dB RMS of the live render. |
| THD | Total harmonic distortion of the signal below 150 Hz at F1, subs only. |
| Macro sweep | Macro i swept 0 to 1 in 5 steps (0, .25, .5, .75, 1), others at rest, same note. Pass if the stated metric moves by at least the threshold and is monotonic (no step reverses by more than 0.05 oct, 0.2 LU, 0.02 correlation or 5 ms). Metric letters: C = change of mean Brightness (octaves, threshold 0.3); D = change of Move depth (octaves, threshold 0.3); L = loudness change (LU, threshold 1); W = Width change (correlation, threshold 0.1); T = time change (ms, threshold stated). |

Sync periods (seconds). The QA tool derives the expected value from the preset's own division and the render tempo.

| Division | 140 BPM | 150 BPM | Division | 140 BPM | 150 BPM |
|---|---|---|---|---|---|
| 1/4 | 0.4286 | 0.4000 | 1/4T | 0.2857 | 0.2667 |
| 1/8 | 0.2143 | 0.2000 | 1/8T | 0.1429 | 0.1333 |
| 1/8D | 0.3214 | 0.3000 | 1/16 | 0.1071 | 0.1000 |
| 1/16T | 0.0714 | 0.0667 | 1/16D | 0.1607 | 0.1500 |
| 1/2 | 0.8571 | 0.8000 | 1/1 (1 bar) | 1.7143 | 1.6000 |
| 2/1 | 3.4286 | 3.2000 | 4/1 (4 bars) | 6.8571 | 6.4000 |

Note on a figure in the brief: a periodicity of 0.267 s at 150 BPM is a quarter-note triplet (1/4T), not a 1/8T. A
1/8T wub repeats every 0.1333 s at 150 BPM (0.1429 s at 140). This sheet uses the table above (preset 6 is the 0.267 s
case); a 1/8T example is preset 2.

### A.1 Per-family QA table (the checks a tool runs for every preset in the family)

| Family (count) | Cat | Test notes | RMS dBFS | LUFS-I | Crest dB | Low end | Brightness (Hz) | Movement | Width (> 200 Hz corr) | Alias floor | Macros |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Riddim (7) | Bass | F1, G1, A#1; E1 floor | -20..-16 | -21..-15 | 6..13 | Low corr >= 0.99; Low S/M <= -30 dB; mono-sum within 1 dB below 120 Hz | 250..1500 | Period at the preset's division +-5 %; depth >= 0.6 oct (>= 0.4 for periods under 80 ms) | 0.6..1.0 (mono mid, some presets add width above 200 Hz) | <= -50 dB (warp and fold excepted from -60) | each passes its check, monotonic |
| Wobble (5) | Bass | F1, G1, A#1; E1 | -20..-16 | -21..-15 | 5..11 | Low corr >= 0.99; sub never modulated (RMS below 90 Hz varies <= 1.5 dB over the cycle, 2 dB for gates) | 180..1600 | Period +-5 %; depth >= 0.8 oct | 0.7..1.0 | <= -55 dB | as above |
| Tearout (5) | Bass | F1, G1, A#1 | -20..-16 | -20..-14 | 5..10 | Low corr >= 0.95; clean sub band: RMS 40-90 Hz within 2 dB of the sub setting | 600..3500 | Period of the main LFO +-5 % or the stated custom-shape lag; depth >= 1.0 oct | 0.2..0.8 | <= -50 dB | as above |
| Formant (3) | Bass | F1, G1, A#1 | -20..-16 | -21..-15 | 6..12 | Low corr >= 0.99 | 350..2200 | Period +-5 %; depth >= 1.0 oct; vowel peaks (F1, F2 of the spectrum) within 8 % of the Vowel table targets in the stated frames | 0.85..1.0 | <= -55 dB | as above |
| Reese / neuro (4) | Bass | F1, G1, A#1 | -20..-16 | -21..-15 | 5..10 | Low corr >= 0.97 (>= 0.95 for the wide one); beat rate of the fundamental in the frozen dry render = f0 x (2^(detune span/1200) - 1) +-10 % | 180..2200 | Slow patches: period +-5 %; neuro: depth >= 0.8 oct; pure reese beating depth (amplitude modulation of 100-300 Hz) >= 3 dB | 0.2..0.9 | <= -50 dB | as above |
| Subs (3) | Bass | F1, G1, A#1; E1 | -20..-16 | -21..-15 | 3..7 | Sub share >= 90 % below 150 Hz (>= 99 % for the pure sine); Low corr >= 0.995 (sub correlation >= 0.95) | 55..160 | Static: Move depth <= 0.25 oct | 0.95..1.0 | <= -60 dB (-70 for the pure sine) | as above |
| Leads (4) | Lead | C3, E3, G4 (C3-C5); no bass notes | -22..-18 | -22..-16 | 6..13 | Energy below 150 Hz >= 20 dB below the 300 Hz-3 kHz band | 1500..4500 | Stated per preset | 0.1..0.7 wide; 0.6..0.95 for the plain one (Width taken above 300 Hz) | <= -50 dB (sync, FM), <= -60 dB for the square lead | as above |
| Plucks (3) | Pluck | C3 (C2 for the sub pluck) | -24..-18 | -24..-18 | 10..20 | Low corr >= 0.97 | 600..3500 | Level falls 20 dB within the stated decay window; centroid falls >= 1.0 oct from attack to 150 ms | 0.3..1.0 | <= -50 dB | as above |
| FX (4) | FX | C3 | -26..-20 | -26..-20 | 7..16 | Low corr >= 0.9 where there is any low energy; no energy below 150 Hz above -30 dB re peak (impact excepted) | 800..6000 | Trend direction and magnitude stated per preset | 0.0..0.7 | <= -45 dB | as above |
| Seq (2) | Seq | F1 held 1 bar (and A#1) | -20..-16 | -21..-15 | 6..13 | Low corr >= 0.99 | 250..1400 | Gate pattern: slot RMS (per step) on active slots >= 10 dB above the off slots, locked to the bar | 0.85..1.0 | <= -55 dB | as above |
| Init (1) | Init | C2, velocity 100 | peak -12..-6 dBFS | n/a | n/a | Low corr >= 0.99 | 150..600 (SPEC 1.12) | None (static) | 0.95..1.0 | <= -70 dB | n/a (unassigned) |

Category medians: Bass -18 dBFS RMS, Lead -20, Pluck -21, Seq -18, FX -22 (4 dB under Bass, inside the 3-6 dB rule). Every preset's
RMS must be within 2 dB of its category median at velocity 100; the Bass spread across the bank stays within +-2 dB.
Peak <= -3 dBFS at velocity 127 for all 41.

### A.2 Bank-level checks

1. All 41 pass their family row. No NaN. No sample above -3 dBFS (after the limiter, any velocity).
2. Every Bass preset: stereo output within 1 dB of the mono sum below 120 Hz at F1, G1 and A#1.
3. Every preset has four macros with non-default names (Init excepted); every macro sweep passes its stated check.
4. At least 20 of 41 use the FX chain; at least 6 use a reordered chain (Phaser or Flanger before Distortion).
5. Every Bass preset carries at least one riddim/dubstep tag; no name or description names an artist or product.
6. Repeatability: two renders of a preset with the same phase-reset setting match within -80 dB; presets with random LFOs or
   random phase are checked with a fixed seed.

Tag list (Track 6): riddim, tearout, growl, wobble, wub, reese, neuro, sub, screech, formant, yoi, talking, hybrid,
metal, square, triplet, call-response, stab, pluck, lead, riser, downlifter, impact, sweep, mono, wide, clean, dirty.

---

## B. Recipes

Shorthand: A/B = Osc A/B. LFO A-D. Idx = table Index. Shift = grain shift. cut = filter cutoff. Filt A/B = filters A and
B. Matrix lines read "source -> target (amount)". Macro lines read "M1 Name: what it does. Check: metric".

### Riddim (Bass, 7)

**1. Hollow Square Reference** | Bass | riddim, square, mono, clean
- Role. The minimal dry reference riddim: a hollow, square-edged grunt that answers a half-time kick with tight eighth-note
  barks. Nothing hidden. At 140 or 150 BPM on F1/G1/A#1 it is a mono pulse that opens and closes like a gate.
- Signal. A: Square Morph, Idx 38; B off; no warp. Sub: sine, octave -1, -6 dB, phase 0. LFO A: Square, 1/8, Retrigger,
  smooth 0.1, to Idx (+0.30) and Filt B cut (+0.25). Filt A: HP12 at 110 Hz, res 0.25; Filt B (Series): LP24 at 2.2 kHz.
  Amp A: A 1 ms, D 0, S 1, R 80 ms, mono, legato. LFO B: Square 1/16, depth 0 until M4, to Idx. FX: Dist Hard Clip +6 dB,
  mix 60 %; Multiband depth 60 %; EQ low cut 30 Hz and -3 dB at 320 Hz; limiter.
- Macros. M1 Bark Depth: size of the LFO A move on Idx and cut. Check D. M2 Tone: Filt B cut 900 Hz to 4 kHz. Check C.
  M3 Grit: distortion drive 0 to 18 dB. Check L. M4 Chatter: LFO B 1/16 gating on Idx. Check D.
- Targets. RMS -20..-16; Low corr >= 0.995; Brightness 280..900 Hz; Move period 0.2000 s @150 (0.2143 @140), depth >= 0.8 oct;
  Width 0.95..1.0; Alias <= -55 dB; macros M1 D >= 0.5 oct, M2 C >= 0.5 oct, M3 L >= 1 LU, M4 D >= 0.3 oct.

**2. Triplet Bark AM** | Bass | riddim, metal, triplet, mono
- Role. The robotic bark: a metallic, ring-modulated square that snaps open on every triplet eighth and fades, over a
  clean sine. It swings against the straight-eighth Reference.
- Signal. A: Metal, Idx 30, Warp Off; B off. Sub sine -1, -6 dB. Filt A: AM, Kbd 1.0 (frequency follows the note), amount
  0.55; Filt B (Series): Notch, cut 650 Hz, Q 0.4. LFO A: Saw Down, 1/8T, Retrigger, to Filt A AM frequency (+0.35) and
  Idx (+0.40). Mod Env (A 1 ms, D 140 ms, S 0.2) -> Filt B cut (+0.5). FX: Dist Diode +9 dB, mix 70 %; Multiband 100 %;
  EQ low cut 35 Hz; limiter.
- Macros. M1 Bark: LFO A to AM and Idx. Check C. M2 Metal: AM amount 0 to 0.9. Check C. M3 Snap: Mod Env depth. Check D.
  M4 Drive: distortion drive. Check L.
- Targets. RMS -20..-16; Low corr >= 0.99; Brightness 350..1200 Hz; Move period 0.1333 s @150 (0.1429 @140), depth >= 0.7 oct;
  Width 0.95..1.0; Alias <= -55 dB; macros M1 C >= 0.4 oct, M2 C >= 0.5 oct, M3 D >= 0.3 oct, M4 L >= 1 LU.

**3. Call and Response Square** | Bass | riddim, square, call-response, mono
- Role. One bar, two phrases: a bouncing call of several hits, then a thinner answer that leaves a hole before the next
  downbeat. One held F1 sounds like a played riff.
- Signal. A: Square Morph, Idx 34; Sub sine -1, -6 dB. LFO C: Step mode, 16 steps over 1/1 (each a 1/16 slot), glide 0.15,
  Retrigger. Steps 1-8 (call): 1.0, 0.2, 1.0, 1.0, 0.2, 1.0, 0.5, 0.0. Steps 9-16 (response): 0.7, 0.0, 0.0, 0.7, 0.0, 0.0,
  0.3, 0.0. Matrix: LFO C -> Idx (+0.35); LFO C -> Filt A (HP12) cut (+0.4); LFO C -> Amp A level (+0.5). LFO D: Square 1/16,
  depth 0 until M4. FX order: Phaser (4 stages, 0.2 Hz, mix 15 %), Distortion (Hard Clip +6 dB), Multiband 100 %, EQ low cut 30 Hz.
- Macros. M1 Call Depth: LFO C to Idx and cut. Check D. M2 Response Gap: scales the response half 1 to 0. Check L.
  M3 Bite: HP12 cut 60 to 400 Hz. Check C. M4 Stutter: LFO D 1/16 on Idx. Check D.
- Targets. RMS -20..-16; Low corr >= 0.99; Brightness 250..1000 Hz; Move: autocorrelation peak at the bar length 1.6000 s @150
  (1.7143 @140) +-5 %; slot RMS of the call half is >= 3 dB above the response half; depth >= 0.8 oct; Width 0.95..1.0;
  Alias <= -55 dB; macros M1 D >= 0.5 oct, M2 L >= 1 LU, M3 C >= 0.4 oct, M4 D >= 0.3 oct.

**4. Robotic Comb Chatter** | Bass | riddim, metal, dirty
- Role. A tight machine chatter: comb-filtered pulse that stutters in sixteenths, harsh but mono, a ratchet over the sub.
- Signal. A: Square Morph, Idx 48, Warp PWM 0.35. Sub sine -1, -5 dB. Filt A: HP12 at 130 Hz, res 0.5; Filt B (Series): Comb-
  (Kbd 1.0, fb 0.55). LFO A: Triangle, 1/16, Retrigger, to Filt B comb offset (+0.30). LFO B: Square, 1/8, to Idx (+0.25).
  Velocity -> Filt B res (+0.2). FX: Dist Fold Sine +9 dB; Flanger (0.1 Hz, depth 0.3, mix 25 %); Multiband 100 %; EQ low cut 35 Hz.
- Macros. M1 Chatter: LFO A depth. Check D. M2 Comb Tone: comb feedback and offset. Check C. M3 Fold: drive. Check C.
  M4 Square Gate: LFO B depth. Check D.
- Targets. RMS -20..-16; Low corr >= 0.99; Brightness 450..1500 Hz; Move period 0.1000 s @150 (0.1071 @140), autocorrelation peak >= 0.5,
  depth >= 0.6 oct; Width 0.9..1.0; Alias <= -50 dB (fold); macros M1 D >= 0.4 oct, M2 C >= 0.4 oct, M3 C >= 0.3 oct, M4 D >= 0.3 oct.

**5. Dotted Metal Stab** | Bass | riddim, metal, hybrid
- Role. Off-beat pocket riddim: a metallic FM-flavoured square on a dotted-eighth cadence, so across two bars it drifts
  against the kick and lands back on the downbeat. Mono below, a little wide above.
- Signal. A: Metal, Idx 25, Warp FM 0.30 (from B); B: Square Morph at -14 dB, Idx 50, pitch +12 st (kept on, level down).
  Sub sine -1, -6 dB. LFO A: Saw Down, 1/8D, Retrigger, to Warp amount (+0.35) and Filt A cut (+0.4). Filt A: LP24 at 1.4 kHz;
  Filt B: Notch at 800 Hz (Parallel). FX: Dist Hard Clip +9 dB; Multiband 100 %; Chorus (mix 15 %, wet low cut 120 Hz);
  EQ low cut 30 Hz.
- Macros. M1 Stab Depth: LFO A to Warp and cut. Check D. M2 FM Mix: Warp amount 0 to 0.7. Check C. M3 Notch Sweep: centre
  400 Hz to 2 kHz. Check C. M4 Width: chorus mix 0 to 40 %. Check W.
- Targets. RMS -20..-16; Low corr >= 0.97; Brightness 350..1400 Hz; Move period 0.3000 s @150 (0.3214 @140), depth >= 0.8 oct;
  Width 0.6..0.95; Alias <= -50 dB at warp 0.30; macros M1 D >= 0.5 oct, M2 C >= 0.4 oct, M3 C >= 0.5 oct, M4 W >= 0.1.

**6. Pulse Width Pocket** | Bass | riddim, square, triplet, mono
- Role. Swung pulse bass: a hollow pulse that narrows and widens in a quarter-note-triplet pocket, with a faster square flicker
  for bounce. Sounds like a wobbling hollow tube.
- Signal. A: Square Morph, Idx 45, Warp PWM 0.60. Sub sine -1, -6 dB. LFO A: Triangle, 1/4T, Retrigger, to PWM amount (+0.5) and
  Filt A (Notch) cut (+0.4). LFO B: Square, 1/8, smooth 0.2, to Idx (+0.2). Filt A: Notch 700 Hz, Q 0.5; Filt B: LP12 at 2.5 kHz.
  FX: Dist Tube +6 dB; Multiband 80 %; Dimension (size 0.2, wet low cut 120 Hz, mix 15 %); EQ low cut 30 Hz.
- Macros. M1 Pocket Depth: LFO A to PWM and notch. Check D. M2 Hollowness: Filt A Q 0.2 to 0.9. Check C. M3 Flicker: LFO B depth.
  Check D. M4 Air: Filt B cut 1.2 to 6 kHz. Check C.
- Targets. RMS -20..-16; Low corr >= 0.99; Brightness 300..1100 Hz; Move period 0.2667 s @150 (0.2857 @140), depth >= 0.7 oct;
  Width 0.8..1.0; Alias <= -55 dB; macros M1 D >= 0.5 oct, M2 C >= 0.3 oct, M3 D >= 0.3 oct, M4 C >= 0.5 oct.

**7. Robot Bark Drop** | Bass | riddim, metal, mono, dirty
- Role. A one-shot bark per note: each note starts with a bright metallic bite that dives a fifth and settles to a hollow
  growl. A gating machine for stuttered playing.
- Signal. A: Metal, Idx 55; Sub sine -1, -6 dB. Mod Env (A 1 ms, D 120 ms, S 0, curve -0.5) -> pitch_all (+7 st) and -> Filt A
  cut (+0.6). Filt A: AM, Kbd 1.0, amount 0.45; Filt B: HP12 at 100 Hz. LFO A: Square, 1/16, trigger Envelope (one-shot), to Idx
  (+0.3). FX: Dist Diode +9 dB; Multiband 100 %; EQ low cut 35 Hz; limiter.
- Macros. M1 Bite: Mod Env to cut. Check C. M2 Dive: pitch dive 0 to 12 st. Check C. M3 Metal: AM amount. Check C. M4 Drive: distortion drive. Check L.
- Targets. RMS -20..-16 (over repeated eighth notes, 1 s); Low corr >= 0.99; Brightness 350..1300 Hz; per note the Move centroid falls
  >= 1.0 oct from 20 ms to 200 ms and the mid-layer pitch drops 7 st +-0.5 over the first 150 ms; Width 0.95..1.0; Alias <= -55 dB
  frozen; macros M1 C >= 0.4 oct, M2 C >= 0.3 oct, M3 C >= 0.4 oct, M4 L >= 1 LU.

### Wobble / wub (Bass, 5)

**8. Slow Sine Wub** | Bass | wobble, wub, clean, mono
- Role. The quarter-note wub: a sine-smooth open and close over a saw stack, the half-time bass that breathes with the kick.
  Full, round, no grit.
- Signal. A: Saw Stack, Idx 60; B: Saw Stack, Idx 60, detune +7 ct. Sub sine -1, -6 dB. Filt A: LP24 at 300 Hz, res 0.35. LFO A:
  Sine, 1/4, Retrigger, to Filt A cut (+0.55). Mod Env -> Filt A cut (+0.1). FX: Dist Tube +3 dB, mix 40 %; Multiband 60 %; EQ low cut 30 Hz.
- Macros. M1 Wobble Depth: LFO A to cut. Check D. M2 Cutoff: base cut. Check C. M3 Res: Filt A res 0.1 to 0.7. Check C. M4 Warmth:
  distortion drive. Check L.
- Targets. RMS -20..-16; sub RMS (< 90 Hz) varies <= 1.5 dB across the cycle; Low corr >= 0.995; Brightness 180..900 Hz; Move period
  0.4000 s @150 (0.4286 @140), depth >= 1.4 oct; Width 0.9..1.0; Alias <= -60 dB; macros M1 D >= 0.8 oct, M2 C >= 0.7 oct, M3 C >= 0.3 oct, M4 L >= 1 LU.

**9. Eighth Triangle Wub** | Bass | wobble, wub, mono
- Role. Mid-speed workhorse wub: a triangular sweep in straight eighths with a growl of distortion as it opens. The riddim-dubstep default.
- Signal. A: Basic (saw region, morph toward square +0.3); B: Saw Stack at -3 dB, detune +9 ct. Sub sine -1, -6 dB. Filt A: LP24 at 250 Hz,
  res 0.4. LFO A: Triangle, 1/8, Retrigger, to Filt A cut (+0.6). FX: Dist Overdrive +9 dB; Multiband 100 %; EQ -3 dB at 280 Hz, low cut 30 Hz.
- Macros. M1 Wobble Depth. Check D. M2 Cutoff. Check C. M3 Grit: distortion drive. Check C. M4 Phase Shift: LFO A start phase 0 to 180 degrees;
  period unchanged. Check: lag of the wobble against the note onset moves by 0.5 x period +-10 %, monotonic.
- Targets. RMS -20..-16; Low corr >= 0.99; Brightness 220..1300 Hz; Move period 0.2000 s @150 (0.2143 @140), depth >= 1.2 oct; sub variation
  <= 1.5 dB; Width 0.9..1.0; Alias <= -60 dB; macros M1 D >= 0.7 oct, M2 C >= 0.5 oct, M3 C >= 0.3 oct, M4 phase lag as stated.

**10. Triplet Saw Chop** | Bass | wobble, wub, triplet, dirty
- Role. A saw-down pluck into a fade on every triplet eighth: each pulse opens bright and rolls off, a chopped, rolling triplet wub.
- Signal. A: Saw Stack, Idx 70; B: Saw Stack, detune -8 ct. Sub sine -1, -6 dB. Filt A: LP24 at 400 Hz, res 0.45. LFO A: Saw Down, 1/8T,
  Retrigger, smooth 0.1, to Filt A cut (+0.65). FX: Dist Hard Clip +9 dB; Multiband 100 %; EQ low cut 35 Hz.
- Macros. M1 Chop Depth. Check D. M2 Brightness: base cut. Check C. M3 Release: LFO A smooth 0 to 0.6. Check D (falls). M4 Dirt: distortion drive. Check L.
- Targets. RMS -20..-16; Low corr >= 0.99; Brightness 250..1500 Hz; Move period 0.1333 s @150 (0.1429 @140), depth >= 1.0 oct; sub variation
  <= 1.5 dB; Width 0.9..1.0; Alias <= -55 dB; macros M1 D >= 0.6 oct, M2 C >= 0.5 oct, M3 D >= 0.3 oct, M4 L >= 1 LU.

**11. Sixteenth Gate Wub** | Bass | wobble, wub, riddim, mono
- Role. A stuttering gate: a square LFO snaps the filter open and shut every sixteenth with a hint of glide, half way between a wobble and
  a gate. Machine-gun but warm.
- Signal. A: Square Morph, Idx 42; B: Saw Stack at -6 dB. Sub sine -1, -6 dB. Filt A: LP24 at 350 Hz, res 0.3. LFO A: Square, 1/16, smooth 0.2,
  Retrigger, to Filt A cut (+0.6). LFO B: Square, 1/16, same phase, to Amp A level (+0.25). FX: Dist Tube +6 dB; Multiband 100 %; EQ low cut 30 Hz.
- Macros. M1 Gate Depth. Check D. M2 Cutoff. Check C. M3 Smooth: LFO A smooth. Check D (falls). M4 Volume Dip: LFO B depth. Check L.
- Targets. RMS -20..-16; Low corr >= 0.99; Brightness 220..1100 Hz; Move period 0.1000 s @150 (0.1071 @140), depth >= 0.8 oct; sub variation
  <= 2 dB; Width 0.9..1.0; Alias <= -55 dB; macros M1 D >= 0.5 oct, M2 C >= 0.5 oct, M3 D >= 0.3 oct, M4 L >= 1 LU.

**12. Drift Random Wobble** | Bass | wobble, wub, mono
- Role. An unpredictable wub: a smooth random LFO in sixteenth rhythm that never repeats exactly, like a hand riding a filter. For fills and variation.
- Signal. A: Saw Stack, Idx 55; B: Basic saw at -4 dB. Sub sine -1, -6 dB. Filt A: BP12 at 420 Hz, res 0.5; Filt B (Series): LP12 at 3 kHz. LFO A:
  Smooth Random, 1/16, Free trigger, to Filt A cut (+0.5). LFO B: Triangle, 1/2, to Filt B cut (+0.25). FX: Dist Tube +6 dB; Multiband 100 %;
  Chorus (mix 10 %, wet low cut 120 Hz); EQ low cut 30 Hz.
- Macros. M1 Drift Depth. Check D. M2 Centre: Filt A cut. Check C. M3 Slow Sweep: LFO B depth. Check D. M4 Width: chorus mix. Check W.
- Targets. RMS -20..-16; Low corr >= 0.97; Brightness 250..1400 Hz; Move: not periodic (largest autocorrelation peak < 0.4 beyond 30 ms), depth >= 1.0 oct;
  sub variation <= 1.5 dB; Width 0.7..0.95; Alias <= -55 dB; macros M1 D >= 0.5 oct, M2 C >= 0.5 oct, M3 D >= 0.3 oct, M4 W >= 0.1.

### Tearout / growl (Bass, 5)

**13. Vocal Growl Gnash** | Bass | tearout, growl, formant, dirty
- Role. The talking tearout growl: a drawn half-note pattern that opens and closes a vowel-like mouth, with FM grit and a clean sub underneath.
  Menacing; mono low end, wide above 200 Hz.
- Signal. A: FM Morph, Idx 80, Warp FM 0.50; B: Reed at -8 dB, Idx 40 (kept on). Unison 3, detune 12 ct. Sub sine -1, -6 dB (clean). LFO A: Custom, 12
  points over 1/2, Retrigger, (x, y): (0.00, -1.0), (0.10, 0.9), (0.18, 0.2), (0.27, 1.0), (0.35, -0.4), (0.45, 0.6), (0.55, -1.0), (0.65, 0.4), (0.75, 0.9),
  (0.85, -0.2), (0.93, 0.5), (1.00, -1.0), segment curve +0.3. Matrix: LFO A -> Idx (+0.5); LFO A -> Warp (+0.4); LFO A -> Filt B (Formant) cut (+0.6).
  Filt A: Notch at 900 Hz. FX order: Phaser (6 stages, 0.15 Hz, mix 20 %), Distortion (Diode +15 dB, post high-pass 150 Hz), Flanger (mix 15 %),
  Multiband 100 %, EQ (low cut 35 Hz, -4 dB at 300 Hz).
- Macros. M1 Growl: LFO A depth to Idx and Warp. Check D. M2 Vowel: Formant offset. Check C. M3 Grit: drive. Check C. M4 Width: flanger and phaser mix. Check W.
- Targets. RMS -20..-16; Low corr >= 0.95; clean sub band intact (40-90 Hz RMS within 2 dB of its mix); Brightness 800..3000 Hz; Move: autocorrelation at
  0.8000 s @150 (0.8571 @140) +-5 %, depth >= 1.5 oct; Width 0.3..0.8; Alias <= -50 dB; macros M1 D >= 0.7 oct, M2 C >= 0.5 oct, M3 C >= 0.4 oct, M4 W >= 0.1.

**14. Triplet Shredder** | Bass | tearout, growl, triplet, wide
- Role. Fast triplet shred: a sync-flavoured saw stack hammered in sixteenth triplets, thick and abrasive; the heavy-drop chord of the bank.
- Signal. A: Saw Stack, Idx 70, Warp Sync 0.45 (to B); B: Saw Stack, detune +14 ct, level -6 dB. Unison 5, detune 16 ct, blend 0.5. Sub sine -1, -6 dB.
  LFO A: Saw Down, 1/16T, Retrigger, to Sync amount (+0.5) and Filt A cut (+0.4). LFO B: Triangle, 1/4T, to Idx (+0.25). Filt A: BP12 at 700 Hz, res 0.65;
  Filt B: LP24 at 4 kHz. FX: Dist Diode +18 dB; Multiband 100 %; Chorus (mix 20 %, wet low cut 120 Hz); EQ low cut 35 Hz.
- Macros. M1 Shred: LFO A depth. Check D. M2 Sync: sync amount. Check C. M3 Drive. Check C. M4 Width. Check W.
- Targets. RMS -20..-16; Low corr >= 0.95; Brightness 900..3500 Hz; Move period 0.0667 s @150 (0.0714 @140), autocorrelation peak >= 0.4, depth >= 1.0 oct;
  Width 0.3..0.7; Alias <= -50 dB at sync 0.45 (document the ceiling); macros M1 D >= 0.5 oct, M2 C >= 0.5 oct, M3 C >= 0.4 oct, M4 W >= 0.15.

**15. Metal Gnash Fold** | Bass | tearout, growl, metal, dirty
- Role. A metallic, ringing tearout: comb resonance and wave-folding give a razor-edged, bell-like growl in triplet eighths. Cold and mechanical.
- Signal. A: Metal, Idx 60, Warp Bend+ 0.4; B: Reed at -7 dB, Idx 30. Unison 3, detune 10 ct. Sub sine -1, -6 dB. LFO A: Triangle, 1/8T, Retrigger, to
  Filt A comb offset (+0.5) and Idx (+0.3). Filt A: Comb+ (Kbd 1.0, fb 0.6); Filt B (Series): Notch at 1.1 kHz. FX order: Distortion (Fold Sine +12 dB),
  Flanger (0.3 Hz, depth 0.4, mix 30 %), Multiband 100 %, Dimension (size 0.25, wet low cut 120 Hz, mix 15 %), EQ low cut 35 Hz.
- Macros. M1 Gnash: LFO A depth. Check D. M2 Ring: comb feedback and offset. Check C. M3 Fold: drive. Check C. M4 Flange: flanger mix. Check W.
- Targets. RMS -20..-16; Low corr >= 0.95; Brightness 1000..3500 Hz; Move period 0.1333 s @150 (0.1429 @140), depth >= 1.2 oct; Width 0.4..0.8; Alias <= -50 dB;
  macros M1 D >= 0.6 oct, M2 C >= 0.5 oct, M3 C >= 0.4 oct, M4 W >= 0.1.

**16. Phased Grind** | Bass | tearout, growl, dirty, wide
- Role. A grinding wide tearout where a phaser feeds the distortion, so the notches sweep through a wall of saturation over a gritty noise bed.
- Signal. A: Reed, Idx 65; B: Grit at -10 dB. Noise: Pink at -22 dB through Filt A (BP12). Unison 4, detune 14 ct. Sub sine -1, -6 dB. LFO A: Triangle, 1/16,
  Retrigger, to Filt A cut (+0.5) and phaser centre (+0.4). LFO B: Square, 1/8, smooth 0.3, to Idx (+0.25). FX order: Phaser (8 stages, fb 0.5, mix 55 %),
  Distortion (Diode +12 dB), Multiband 100 %, EQ (low cut 35 Hz, -4 dB at 350 Hz), Chorus (mix 20 %, wet low cut 120 Hz).
- Macros. M1 Grind: LFO A depth. Check D. M2 Phaser Depth: phaser mix and centre. Check C. M3 Grit: noise level and drive. Check C. M4 Width: chorus mix. Check W.
- Targets. RMS -20..-16; Low corr >= 0.95; Brightness 900..3200 Hz; Move period 0.1000 s @150 (0.1071 @140), depth >= 1.0 oct; Width 0.25..0.7; Alias <= -50 dB;
  macros M1 D >= 0.5 oct, M2 C >= 0.4 oct, M3 C >= 0.3 oct, M4 W >= 0.15.

**17. Pitch Rip Growl** | Bass | tearout, growl, hybrid, dirty
- Role. A growl whose pitch rips along a drawn curve each beat, rising a fourth and snapping back, adding a screaming edge on top of a steady sub.
- Signal. A: Saw Stack, Idx 75, Warp FM 0.35; B: FM Morph at -8 dB. The sub stays fixed (clean sine, -1 oct, -6 dB, excluded from the rip because the rip is
  routed to osc A pitch and osc B pitch, not pitch_all). LFO A: Custom, 8 points over 1/4, Retrigger: (0.00, 0.0), (0.12, 1.0), (0.30, 0.35), (0.50, 0.85),
  (0.70, 0.0), (0.82, -0.3), (0.92, 0.2), (1.00, 0.0), segment curve -0.4. Matrix: LFO A -> osc A pitch (+5 st); LFO A -> osc B pitch (+5 st); LFO A -> Filt A
  (BP12) cut (+0.5). LFO C: Saw Down, 1/8T, to Warp (+0.2). FX: Dist Diode +15 dB; Multiband 100 %; Flanger mix 20 %; EQ low cut 35 Hz.
- Macros. M1 Rip: pitch amount 0 to 7 st. Check C. M2 Warp: FM amount. Check C. M3 Grit. Check C. M4 Flange: flanger mix. Check W.
- Targets. RMS -20..-16; sub band (40-90 Hz) unmodulated, RMS varies <= 1.5 dB; Low corr >= 0.95; Brightness 800..3200 Hz; Move period 0.4000 s @150 (0.4286 @140),
  depth >= 1.2 oct; mid-layer pitch excursion 5 st +-0.7 (autocorrelation pitch track); Width 0.4..0.8; Alias <= -50 dB; macros M1 C >= 0.3 oct, M2 C >= 0.5 oct,
  M3 C >= 0.4 oct, M4 W >= 0.1.

### Formant / talking (Bass, 3)

**18. Yoi Sweep Slow** | Bass | formant, yoi, talking, mono
- Role. The classic oo-ee-yoi sweep: a slow vowel glide across half a bar and back, voice-like but pitched at F1.
- Signal. A: Vowel, Idx 50, Shift 0; B: Reed at -6 dB. Sub sine -1, -6 dB. Filt A: Formant, res 0.6 (peak sharpness), cut position drives A-E-I-O-U. LFO A: Triangle,
  1/2, Retrigger, to Filt A cut (+0.75) and Idx (+0.2). Filt B: LP12 at 4 kHz. FX: Dist Tube +6 dB; Multiband 80 %; EQ low cut 30 Hz, +2 dB at 1.8 kHz.
- Macros. M1 Vowel Sweep: LFO A depth. Check D. M2 Vowel Position: base Formant position, A to U. Check C. M3 Sharpness: Formant res. Check C. M4 Grit: drive. Check L.
- Targets. RMS -20..-16; Low corr >= 0.99; Brightness 450..1600 Hz; Move period 0.8000 s @150 (0.8571 @140), depth >= 1.2 oct; at the sweep ends the spectral F1/F2
  peaks land within 8 % of the Vowel table (for example A: 730 Hz, 1090 Hz; I: 270 Hz, 2290 Hz); Width 0.9..1.0; Alias <= -55 dB; macros M1 D >= 0.7 oct, M2 C >= 0.5 oct,
  M3 C >= 0.3 oct, M4 L >= 1 LU.

**19. Talking Chatter** | Bass | formant, talking, call-response, mono
- Role. A mouth that says four different syllables per half bar: stepped vowel changes in eighth notes, slightly smoothed, like a vocoded robot.
- Signal. A: Reed, Idx 45; B: Vowel at -8 dB. Sub sine -1, -6 dB. LFO C: Step mode, 4 steps over 1/2 (each step an eighth), glide 0.35, Retrigger; levels (A) 0.15,
  (E) 0.45, (O) 0.80, (I) 1.00. Matrix: LFO C -> Filt A (Formant) cut (+0.7); LFO C -> Idx (+0.15). LFO D: Saw Down, 1/16, depth 0 until M4. FX: Dist Tube +9 dB;
  Multiband 100 %; EQ low cut 30 Hz.
- Macros. M1 Syllables: LFO C depth. Check D. M2 Mouth: Formant res. Check C. M3 Glide: step glide 0 to 0.8. Check D (falls). M4 Chatter: LFO D depth. Check D.
- Targets. RMS -20..-16; Low corr >= 0.99; Brightness 400..2000 Hz; Move: centroid steps every 0.2000 s @150 (0.2143 @140), autocorrelation at 0.8000 s @150 (0.8571 @140)
  +-5 %; depth >= 1.0 oct; four plateaus per half bar with centroids in rising order (A lowest, I highest) within 0.3 oct of design; Width 0.9..1.0; Alias <= -55 dB;
  macros M1 D >= 0.6 oct, M2 C >= 0.3 oct, M3 D >= 0.3 oct, M4 D >= 0.3 oct.

**20. Vowel Pluck Bark** | Bass | formant, talking, pluck, mono
- Role. A short, vowel-shaped bark on every note: it opens on "ee" and closes on "oo" in under a fifth of a second. For staccato riffs.
- Signal. A: Reed, Idx 55; B: Vowel at -8 dB. Sub sine -1, -6 dB. Mod Env (A 1 ms, D 160 ms, S 0, curve -0.6) -> Filt A (Formant) cut (-0.7); Mod Env -> Idx (+0.3).
  LFO A: Saw Down, 1/16, trigger Envelope (one-shot). Filt B: LP12 at 3 kHz. FX: Dist Tube +9 dB; Multiband 100 %; EQ low cut 30 Hz.
- Macros. M1 Bark: Mod Env depth. Check C. M2 Vowel: base Formant position. Check C. M3 Decay: Mod Env decay 60 to 400 ms. Check: T, per-note centroid fall time changes
  >= 100 ms. M4 Grit. Check L.
- Targets. RMS -20..-16 (over repeated eighth notes, 1 s); Low corr >= 0.99; Brightness 350..1800 Hz; per-note centroid falls >= 1.2 oct from 20 ms to 200 ms; Width 0.9..1.0;
  Alias <= -55 dB; macros M1 C >= 0.4 oct, M2 C >= 0.5 oct, M3 T >= 100 ms, M4 L >= 1 LU.

### Reese / neuro (Bass, 4)

**21. Liquid Dark Reese** | Bass | reese, mono, clean
- Role. The dark rolling reese: two detuned saws beating slowly against each other under a low-pass; warm, thick, steady. A living low drone.
- Signal. A: Saw Stack, Idx 60; B: Saw Stack, Idx 60, detune +10 ct. Unison 2, detune 14 ct (outer span 28 ct). Random phase 0, phase reset on. Sub sine -1, -8 dB.
  Filt A: LP24 at 520 Hz, res 0.2. LFO A: Triangle, 1/1, to Filt A cut (+0.15). FX order: Phaser (6 stages, 0.1 Hz, mix 15 %), Distortion (Tube +3 dB, mix 30 %),
  Multiband 60 %, EQ low cut 30 Hz.
- Macros. M1 Beat: unison detune 6 to 28 ct. Check: beat rate scales with detune within 10 %, beating depth >= 2 dB. M2 Darkness: base cut. Check C. M3 Drift: LFO A depth.
  Check D. M4 Warmth: drive. Check L.
- Targets. RMS -20..-16; Low corr >= 0.97; Brightness 180..700 Hz; beat rate of the fundamental in the frozen dry render 0.71 Hz at F1 (0.95 Hz at A#1) +-10 %; beating depth >= 3 dB;
  Move period 1.6000 s @150 (1.7143 @140), depth >= 0.4 oct; Width 0.5..0.9; Alias <= -55 dB; macros M1 as stated, M2 C >= 0.5 oct, M3 D >= 0.3 oct, M4 L >= 1 LU.

**22. Neuro Comb Fold** | Bass | neuro, reese, dirty, mono
- Role. A snarling neuro bass: three detuned saws pushed through parallel comb and notch filters moving in triplets, then phased and folded. Sharp and futuristic.
- Signal. A: Saw Stack, Idx 70; B: Saw Stack, detune +12 ct. Unison 3, detune 24 ct. Sub sine -1, -6 dB. Filt A: Comb- (Kbd 1.0, fb 0.5); Filt B: Notch at 700 Hz (Parallel).
  LFO A: Triangle, 1/8T, Retrigger, to Filt B notch cut (+0.55). LFO B: Smooth Random, 1/4, to Filt A comb offset (+0.2). FX order: Phaser (6 stages, mix 40 %),
  Distortion (Fold Sine +12 dB), Multiband 100 %, EQ low cut 35 Hz.
- Macros. M1 Neuro Sweep: LFO A depth. Check D. M2 Fold: drive. Check C. M3 Comb: comb feedback and offset. Check C. M4 Phase: phaser mix. Check C.
- Targets. RMS -20..-16; Low corr >= 0.97; Brightness 450..2200 Hz; Move period 0.1333 s @150 (0.1429 @140), depth >= 1.0 oct; Width 0.6..0.9; Alias <= -50 dB;
  macros M1 D >= 0.6 oct, M2 C >= 0.4 oct, M3 C >= 0.4 oct, M4 C >= 0.3 oct.

**23. Wide Reese Chorus** | Bass | reese, wide, clean
- Role. A wide, lush reese: mono and tight below 120 Hz, huge above, for sustained drops where the mix needs width.
- Signal. A: Saw Stack, Idx 55; B: Basic saw, detune -9 ct. Unison 4, detune 18 ct, width 1.0. Sub sine -1, -6 dB. Filt A: LP24 at 900 Hz, res 0.2. LFO A: Triangle, 2/1,
  to Filt A cut (+0.2). Mono crossover 120 Hz. FX: Distortion (Tube +3 dB, mix 25 %); Multiband 60 %; EQ low cut 30 Hz; Chorus (4 voices, 0.3 Hz, depth 0.5, wet low
  cut 120 Hz, mix 35 %); Dimension (size 0.5, wet low cut 120 Hz, mix 25 %); limiter.
- Macros. M1 Width: chorus and dimension mix. Check W. M2 Brightness: Filt A cut. Check C. M3 Beat: unison detune. Check: beating depth change >= 2 dB. M4 Sweep: LFO A depth. Check D.
- Targets. RMS -20..-16; Low corr >= 0.95 (L-R below 120 Hz <= -26 dB re M); Brightness 250..1600 Hz; Move period 3.2000 s @150 (3.4286 @140), depth >= 0.4 oct (render 3 bars);
  Width 0.2..0.6; Alias <= -55 dB; macros M1 W >= 0.2, M2 C >= 0.5 oct, M3 as stated, M4 D >= 0.3 oct.

**24. Slow Sweep Reese** | Bass | reese, mono, wub
- Role. A one-bar breathing reese: slow cutoff sweep over a detuned pair with a touch of grit; the drone for long breakdowns.
- Signal. A: Saw Stack, Idx 65; B: Saw Stack, detune +11 ct. Unison 2, detune 12 ct, phase reset on. Sub sine -1, -7 dB. Filt A: LP24 at 220 Hz, res 0.45; Filt B: Notch at 900 Hz
  (Parallel). LFO A: Triangle, 1/1, Retrigger, to Filt A cut (+0.7). FX order: Phaser (mix 20 %), Distortion (Overdrive +6 dB), Multiband 80 %, EQ low cut 30 Hz.
- Macros. M1 Sweep Depth. Check D. M2 Base Cutoff. Check C. M3 Resonance. Check C. M4 Grit: drive. Check L.
- Targets. RMS -20..-16; Low corr >= 0.97; Brightness 200..1200 Hz; Move period 1.6000 s @150 (1.7143 @140), depth >= 1.5 oct; Width 0.5..0.9; Alias <= -55 dB;
  macros M1 D >= 0.8 oct, M2 C >= 0.7 oct, M3 C >= 0.3 oct, M4 L >= 1 LU.

### Subs (Bass, 3)

**25. Pure Sine Sub** | Bass | sub, clean, mono
- Role. The reference sub: a flat sine with a soft attack and nothing else. For layering and for checking a system.
- Signal. A: Basic (sine), Idx 0; B off. Sub: sine, octave 0, 0 dB. Filters off. Amp A: A 2 ms, D 0, S 1, R 60 ms. FX: EQ low cut 25 Hz; limiter only.
- Macros. M1 Level: output trim -6 to 0 dB. Check L. M2 Soft Attack: attack 2 to 40 ms. Check T, time to reach 90 % of peak changes >= 10 ms. M3 Octave Up: adds osc A sine one octave
  up, off to -12 dB. Check C. M4 Release: 20 to 300 ms. Check T, time to fall 40 dB after note-off changes >= 50 ms.
- Targets. RMS -20..-16; Sub share >= 99 % below 150 Hz; THD <= 0.3 %; Low corr >= 0.999; Brightness 55..90 Hz; Move depth <= 0.1 oct; Width 0.99..1.0; Alias <= -70 dB;
  macros M1 L >= 1 LU, M2 T >= 10 ms, M3 C >= 0.3 oct, M4 T >= 50 ms.

**26. Warm Tube Sub** | Bass | sub, clean, mono
- Role. A sub with a little saturation so it speaks on small speakers and laptops: a warm 2nd and 3rd harmonic over a steady fundamental.
- Signal. A: Basic (sine), Idx 0; B off. Sub: Triangle, octave 0, -2 dB, drive 0.25. Filt B: LP12 at 180 Hz. Amp A: A 3 ms, S 1, R 80 ms. FX: Dist Tube +3 dB, mix 100 %, post
  filter LP 250 Hz; EQ low cut 25 Hz; limiter.
- Macros. M1 Warmth: tube drive 0 to 9 dB. Check C. M2 Small Speaker: harmonic level (sub drive). Check C. M3 Level. Check L. M4 Release: 20 to 300 ms. Check T >= 50 ms.
- Targets. RMS -20..-16; Sub share >= 92 % below 150 Hz at F1, >= 90 % at A#1; 2nd + 3rd harmonics 15..25 dB below the fundamental; Low corr >= 0.995; Brightness 60..140 Hz;
  Move depth <= 0.25 oct; Alias <= -70 dB; macros M1 C >= 0.3 oct, M2 C >= 0.3 oct, M3 L >= 1 LU, M4 T >= 50 ms.

**27. Dirty Sub Layer** | Bass | sub, dirty, hybrid, mono
- Role. A layered sub: a clean sine sub underneath, a saturated square an octave up on top so the note reads loud on any speaker. The bass-house pocket.
- Signal. A: Square Morph, Idx 20 (octave up, -12 dB) into Filt A LP24 at 320 Hz; Sub: sine, octave -1, -2 dB (clean, no drive). Mono crossover 120 Hz. FX: Dist Tube +12 dB with pre
  high-pass at 120 Hz (mid layer only); Multiband 60 %; EQ low cut 25 Hz.
- Macros. M1 Dirt: mid-layer drive. Check C. M2 Layer Mix: mid level against sub. Check C. M3 Sub Level. Check L. M4 Cutoff: Filt A cut. Check C.
- Targets. RMS -20..-16; Sub share >= 90 % below 150 Hz at F1 (harmonics >= 10 dB below the fundamental); Low corr >= 0.995; Brightness 70..160 Hz; THD of the band below 70 Hz
  <= 1 %; Move depth <= 0.25 oct; Alias <= -60 dB; macros M1 C >= 0.3 oct, M2 C >= 0.3 oct, M3 L >= 1 LU, M4 C >= 0.3 oct.

### Leads / screech (Lead, 4)

**28. Screech Dive One** | Lead | screech, lead, mono
- Role. A bright, screaming lead with a quick pitch dive at the start of each note and a rising comb sweep: the high-energy screech at C3-C5.
- Signal. A: Saw Stack, Idx 80, Warp Sync 0.5; B: Metal at -6 dB, detune +3 ct. Unison 5, detune 18 ct. Mod Env (A 1 ms, D 90 ms, S 0, curve -0.5) -> pitch_all (+12 st). LFO A: Saw
  Up, 1/2, Retrigger, to Filt A (Comb+) comb offset (+0.6). Filt B: HP12 at 220 Hz. FX: Dist Hard Clip +18 dB; EQ low cut 200 Hz; Flanger mix 20 %; Reverb 8 % (low cut 300 Hz); limiter.
- Macros. M1 Dive: pitch dive 0 to 12 st. Check C (first 100 ms). M2 Comb Sweep: LFO A depth. Check D. M3 Screech: sync amount. Check C. M4 Drive. Check C.
- Targets. RMS -22..-18; energy below 150 Hz >= 20 dB below 300 Hz-3 kHz; Brightness 2000..4500 Hz; per-note pitch dive 12 st +-1 over 90 ms; comb sweep depth >= 1.0 oct, period 0.8000 s @150
  (0.8571 @140); Width (above 300 Hz) 0.2..0.7; Alias <= -50 dB; macros M1 C >= 0.3 oct, M2 D >= 0.5 oct, M3 C >= 0.4 oct, M4 C >= 0.4 oct.

**29. Screech Formant Rise** | Lead | screech, formant, lead, dirty
- Role. A vocal screech: a formant filter rises from "oh" to "ee" across a beat while the pitch bends up; a human scream rather than a metal one.
- Signal. A: Reed, Idx 70, Warp Bend+ 0.4; B: Saw Stack at -6 dB. Unison 5, detune 14 ct. LFO A: Saw Up, 1/4, Retrigger, to Filt A (Formant) cut (+0.8). Mod Env (D 120 ms) -> osc A pitch
  (+5 st). Filt B: HP12 at 250 Hz. FX order: Phaser (mix 25 %), Distortion (Diode +15 dB), EQ low cut 220 Hz, Reverb 8 %.
- Macros. M1 Rise: LFO A depth. Check D. M2 Vowel Start: base Formant position. Check C. M3 Bend: pitch amount. Check C. M4 Grit: drive. Check C.
- Targets. RMS -22..-18; Brightness 1500..4000 Hz; Move period 0.4000 s @150 (0.4286 @140), depth >= 1.2 oct; the F2 peak rises through at least 800 Hz each beat; energy below 150 Hz
  >= 20 dB below 300 Hz-3 kHz; Width (above 300 Hz) 0.2..0.7; Alias <= -50 dB; macros M1 D >= 0.6 oct, M2 C >= 0.5 oct, M3 C >= 0.3 oct, M4 C >= 0.4 oct.

**30. Wide Hoover Stack** | Lead | lead, wide, hybrid
- Role. A hoover-style supersaw: a thick detuned wall with a falling pitch on each note and chorus; wide, sweet and loud.
- Signal. A: Saw Stack, Idx 55; B: Saw Stack, detune +6 ct. Unison 7, detune 28 ct, width 1.0. Mod Env (D 140 ms, curve -0.7) -> pitch_all (+5 st). Filt A: LP24 at 4.5 kHz, res 0.2, Kbd 0.5.
  Mono crossover 120 Hz. FX: Distortion (Overdrive +6 dB); Chorus (6 voices, mix 40 %, wet low cut 200 Hz); Dimension (size 0.7, mix 30 %); EQ low cut 150 Hz; Reverb 12 %; limiter.
- Macros. M1 Width: chorus and dimension mix. Check W. M2 Brightness: Filt A cut. Check C. M3 Dive: pitch drop amount. Check C (first 100 ms). M4 Chorus Depth. Check W.
- Targets. RMS -22..-18; energy below 150 Hz >= 20 dB below 300 Hz-3 kHz; Brightness 1800..4500 Hz; Width (above 300 Hz) 0.1..0.5; mono-sum within 1.5 dB of the stereo sum above 200 Hz;
  Alias <= -55 dB; macros M1 W >= 0.2, M2 C >= 0.5 oct, M3 C >= 0.3 oct, M4 W >= 0.1.

**31. Square Melody Lead** | Lead | lead, square, clean, mono
- Role. A plain melodic square lead with a slow breath of vibrato and a dotted echo. Clean and cutting, for hooks.
- Signal. A: Square Morph, Idx 48; B: Basic saw at -9 dB, detune +5 ct. Unison 2, detune 6 ct. Filt A: LP24 at 3.5 kHz, res 0.2, Kbd 0.5. LFO A: Sine, 1/4, to pitch_all (+0.15 st), fade-in 0.3 s.
  Amp A: A 4 ms, D 200 ms, S 0.6, R 120 ms. FX: Chorus (mix 15 %, wet low cut 200 Hz); Delay 1/8D (mix 20 %, fb 0.35, HP 400 Hz); Reverb 10 %.
- Macros. M1 Brightness: Filt A cut. Check C. M2 Vibrato: LFO A depth. Check: pitch deviation 0 to 0.5 st, >= 0.1 st. M3 Echo: delay mix. Check L. M4 Space: reverb mix. Check L.
- Targets. RMS -22..-18; Brightness 1500..3000 Hz; Width (above 300 Hz) 0.6..0.95; vibrato period 0.4000 s @150 (0.4286 @140) +-5 %; echo spacing 0.3000 s @150 (0.3214 @140) within one
  sample; Alias <= -60 dB; macros M1 C >= 0.5 oct, M2 pitch deviation >= 0.1 st, M3 L >= 1 LU, M4 L >= 1 LU.

### Plucks / stabs (Pluck, 3)

**32. Wide Chord Stab** | Pluck | stab, pluck, wide
- Role. A short, bright chord hit with a quick filter close, wide and full: a drop-day stab to answer a bass note.
- Signal. A: Saw Stack, Idx 65; B: Saw Stack, detune +8 ct. Unison 4, detune 20 ct, width 1.0. Amp A: A 1 ms, D 180 ms, S 0, R 80 ms. Filter env (Amt 0.6, D 220 ms) on Filt A LP24 at 600 Hz.
  FX: Chorus (mix 30 %, wet low cut 160 Hz); Reverb 12 %; EQ low cut 120 Hz.
- Macros. M1 Snap: filter env amount. Check C. M2 Decay: amp decay 60 to 400 ms. Check T, time to -20 dB changes >= 150 ms. M3 Width. Check W. M4 Space: reverb mix. Check L.
- Targets. RMS -24..-18 (first 300 ms); level falls 20 dB within 250 ms +-30 %; centroid falls >= 1.2 oct from 10 to 150 ms; Brightness 800..3500 Hz; Low corr >= 0.97; Width 0.3..0.7;
  Alias <= -55 dB; macros M1 C >= 0.5 oct, M2 T >= 150 ms, M3 W >= 0.15, M4 L >= 1 LU.

**33. Pitch Drop Sub Pluck** | Pluck | pluck, sub, impact, mono
- Role. A kick-like bass pluck: a sine with a fast downward pitch drop, then a clean low tail. Fills the hole between riddim bars.
- Signal. A: Basic (sine); Sub sine -1, -3 dB. Mod Env (A 1 ms, D 90 ms, S 0, curve -0.7) -> pitch_all (+24 st). Amp A: A 1 ms, D 280 ms, S 0, R 60 ms. FX: Dist Tube +3 dB; EQ low cut 25 Hz; limiter.
- Macros. M1 Drop: pitch drop 0 to 36 st. Check C. M2 Length: amp decay. Check T >= 150 ms. M3 Click: Mod Env to a short HP12. Check C. M4 Drive. Check L.
- Targets. RMS -24..-18 (C2 test); autocorrelation pitch falls 24 st +-1 over the first 90 ms; level falls 20 dB within 320 ms +-30 %; Low corr >= 0.995; sub share >= 85 % below 150 Hz after
  120 ms; Width 0.95..1.0; Alias <= -60 dB; macros M1 pitch at 20 ms scales with amount within 1 st, M2 T >= 150 ms, M3 C >= 0.3 oct, M4 L >= 1 LU.

**34. Metal Ring Pluck** | Pluck | metal, pluck, stab
- Role. A short bell-metal pluck, FM-bright with a quick tone decay, for lead-ins and echoing call-response fills.
- Signal. A: Metal, Idx 45, Warp FM 0.35; B: FM Morph at -10 dB, pitch +12 st. Unison 2, detune 8 ct. Amp A: A 1 ms, D 140 ms, S 0, R 100 ms. Mod Env (D 120 ms) -> Warp (-0.35) and Filt A (LP24 at
  1.2 kHz) cut. FX: Chorus (mix 15 %, wet low cut 200 Hz); Delay 1/8T (mix 20 %, fb 0.35); Reverb 14 %; EQ low cut 120 Hz.
- Macros. M1 Ring: FM amount. Check C. M2 Decay. Check T >= 100 ms. M3 Brightness. Check C. M4 Echo: delay mix. Check L.
- Targets. RMS -24..-18; level falls 20 dB within 180 ms +-30 %; centroid falls >= 1.0 oct from 10 to 150 ms; Brightness 1200..3500 Hz; Width 0.5..0.9; Alias <= -50 dB; echo at 0.1333 s @150
  (0.1429 @140) within one sample; macros M1 C >= 0.4 oct, M2 T >= 100 ms, M3 C >= 0.5 oct, M4 L >= 1 LU.

### FX (FX, 4)

**35. Four Bar Riser** | FX | riser, sweep
- Role. A noisy rising riser into the drop: pitch and brightness climb over four bars, widening as it goes.
- Signal. A: Saw Stack, Idx 80; Unison 7, detune 24 ct. Noise: White at -12 dB through Filt A (BP12). LFO A: Saw Up, 4/1, trigger Envelope (one-shot), to Filt A cut (+0.9), Filt B (HP12) cut (+0.7),
  pitch_all (+12 st). LFO B: Saw Up, 4/1 one-shot, to Reverb mix (+0.2). FX: Chorus (mix 30 %, rising with M1); Flanger mix 20 %; Reverb 20 % (decay 3 s); EQ low cut 200 Hz; limiter.
- Macros. M1 Rise Width: width at the end. Check W. M2 Noise Level. Check L. M3 Pitch Rise: pitch amount 0 to 24 st. Check C. M4 Reverb Tail. Check L.
- Targets. RMS -26..-20 (last bar); hold 4 bars (6.857 s @140, 6.400 s @150); centroid rises >= 2.0 oct from the first to the last 10 % of the hold, and >= 90 % of consecutive 250 ms differences
  >= 0; level rises 3..8 dB; Brightness 1500..6000 Hz; Width rises >= 0.2 in correlation; no energy below 150 Hz above -30 dB re peak; Alias <= -45 dB; macros M1 W >= 0.2, M2 L >= 1 LU,
  M3 C >= 0.3 oct, M4 L >= 1 LU.

**36. Four Bar Downlifter** | FX | downlifter, sweep
- Role. The mirror: after the drop the energy falls away, darker and narrower over four bars with a reverb wash.
- Signal. A: Saw Stack, Idx 70; Unison 6, detune 20 ct. Noise: Pink at -14 dB. LFO A: Saw Down, 4/1, trigger Envelope (one-shot), to Filt A cut (+0.9) and pitch_all (+12 st). LFO B: Saw Down, 4/1,
  to Amp A level (+0.35). FX order: Phaser (mix 20 %), Chorus (mix 25 %, falling with M1), Reverb 25 %, EQ low cut 180 Hz, limiter.
- Macros. M1 Fall Speed: LFO A depth. Check C. M2 Noise Level. Check L. M3 Depth: pitch fall amount. Check C. M4 Reverb Tail. Check L.
- Targets. RMS -26..-20 (first bar); centroid falls >= 2.0 oct, >= 90 % of 250 ms differences <= 0; level falls 6..14 dB over the hold; Brightness 800..5000 Hz; Width falls >= 0.15 in correlation;
  Alias <= -45 dB; macros M1 C >= 0.3 oct, M2 L >= 1 LU, M3 C >= 0.3 oct, M4 L >= 1 LU.

**37. Sub Drop Impact** | FX | impact, sub, clean
- Role. A short boom: a sine drops three octaves with a noise burst on top, then a long soft tail. For transitions and drop hits.
- Signal. A: Basic (sine); Sub sine, octave 0, -3 dB. Mod Env (A 1 ms, D 450 ms, S 0, curve -0.7) -> pitch_all (+36 st). Noise: Brown at -14 dB, amp shaped by a 60 ms decay. Amp A: A 1 ms, D 900 ms,
  S 0, R 300 ms. FX: Dist Tube +3 dB; EQ low cut 22 Hz; Reverb 15 % (decay 2.5 s, low cut 150 Hz); limiter.
- Macros. M1 Drop Depth: 12 to 48 st. Check C. M2 Boom Length: amp decay. Check T >= 300 ms. M3 Noise Burst. Check L. M4 Tail: reverb mix. Check L.
- Targets. RMS -26..-20 (first 800 ms); autocorrelation pitch falls 3 octaves +-1 st; level -20 dB within 1.5 s; Low corr >= 0.95; Brightness 60..400 Hz (then falls); Width 0.5..1.0; Alias <= -50 dB;
  macros M1 pitch excursion scales within 2 st, M2 T >= 300 ms, M3 L >= 1 LU, M4 L >= 1 LU.

**38. Noise Sweep Band** | FX | sweep, riser
- Role. A slow breathing noise sweep: band-passed noise moving up and down over two bars, for backgrounds and builds.
- Signal. Noise: White at -6 dB plus Pink at -10 dB; osc A off. Filt A: BP12, cut 1.1 kHz, res 0.6; Filt B: Formant, res 0.3 (Parallel). LFO A: Triangle, 2/1, to Filt A cut (+0.7) and Filt B cut (+0.5).
  LFO B: Triangle, 1/1, to pan (+0.5). FX: Phaser mix 25 %; Dimension (mix 30 %, wet low cut 200 Hz); Reverb 20 %; EQ low cut 250 Hz.
- Macros. M1 Sweep Depth. Check D. M2 Centre: base cut. Check C. M3 Noise Colour: White to Brown. Check C. M4 Width. Check W.
- Targets. RMS -26..-20; Brightness 800..5000 Hz; Move period 3.2000 s @150 (3.4286 @140), depth >= 1.5 oct (hold 4 bars); Width 0.0..0.6; no energy below 150 Hz above -30 dB re peak; noise slope within
  +-1 dB of the chosen type; macros M1 D >= 0.7 oct, M2 C >= 0.5 oct, M3 C >= 0.5 oct, M4 W >= 0.15.

### Seq (Seq, 2)

**39. Gated Triplet Call** | Seq | riddim, call-response, triplet, mono
- Role. A held note turned into a triplet pattern: twelve gate slots per bar, a call in the first half and a thinner answer, so a single chord note yields a bass phrase.
- Signal. A: Square Morph, Idx 36; Sub sine -1, -6 dB. LFO C: Step mode, 12 steps over 1/1 (each a 1/8T slot), glide 0.1, Retrigger: 1.0, 1.0, 0.0, 1.0, 0.0, 0.0 | 1.0, 0.0, 0.0, 0.0, 1.0, 0.0
  (call | answer). Matrix: LFO C -> Amp A level (+1.0); LFO C -> Idx (+0.3); LFO C -> Filt A (HP12) cut (+0.4). FX: Dist Hard Clip +6 dB; Multiband 100 %; EQ low cut 30 Hz.
- Macros. M1 Gate Depth: LFO C to level. Check L. M2 Pattern Tone: Idx and cut. Check C. M3 Answer Level: answer slots 0 to 1. Check L. M4 Swing: step glide 0 to 0.8. Check D.
- Targets. RMS -20..-16 (whole bar, held note); active slots >= 10 dB above slots at 0.0; slot RMS pattern correlation to the steps >= 0.9; bar period 1.6000 s @150 (1.7143 @140) +-5 %; slot length
  0.1333 s @150 (0.1429 @140); Low corr >= 0.99; Brightness 250..900 Hz; Width 0.9..1.0; Alias <= -55 dB; macros M1 L >= 3 LU, M2 C >= 0.4 oct, M3 answer-slot RMS changes >= 6 dB, M4 D >= 0.3 oct.

**40. Sixteenth Stutter Response** | Seq | riddim, call-response, stab, mono
- Role. A fast sixteenth stutter that answers the Triplet Call: a ratchet bar of sixteen slots with a rest at the end of each beat, for fills.
- Signal. A: Metal, Idx 28; B: Square Morph at -8 dB. Sub sine -1, -6 dB. LFO C: Step mode, 16 steps over 1/1 (each a 1/16 slot), glide 0.2: 1.0, 0.0, 0.7, 0.0 | 1.0, 0.0, 1.0, 0.0 | 0.7, 0.0, 0.7, 0.0 |
  1.0, 0.5, 0.0, 0.0. Retrigger. Matrix: LFO C -> Amp A level (+0.9); LFO C -> Idx (+0.25); LFO C -> Warp FM amount (+0.2). LFO D: Square 1/16, depth 0 until M4. FX: Dist Diode +9 dB; Multiband 100 %;
  Flanger mix 10 %; EQ low cut 30 Hz.
- Macros. M1 Stutter Depth. Check L. M2 Metal: Idx and warp. Check C. M3 Rest Length: LFO C smooth 0 to 0.7. Check: slot contrast falls >= 3 dB. M4 Shimmer: LFO D depth. Check D.
- Targets. RMS -20..-16; slot RMS pattern correlation >= 0.9; slot period 0.1000 s @150 (0.1071 @140) +-5 %, bar period 1.6000 s @150 (1.7143 @140) +-5 %; Low corr >= 0.99; Brightness 350..1300 Hz;
  Width 0.85..1.0; Alias <= -55 dB; macros M1 L >= 3 LU, M2 C >= 0.4 oct, M3 contrast change >= 3 dB, M4 D >= 0.3 oct.

### Init (Init, 1)

**41. Init Square Bass** | Init | clean, square, mono
- Role. The v2 init patch: a clean mono square bass with the sub on and everything else neutral, always first in the list. The blank sheet.
- Signal. A: Square Morph, Idx 40, unison 1; B off; warp off. Sub sine -1, -6 dB. Filt A: LP12 at 2 kHz with filter env; Filt B off. Amp A: A 1 ms, D 0, S 1, R 80 ms. LFO A-D off, Mod Env default,
  noise off, matrix empty, all FX off.
- Macros. M1-M4 unassigned, named Macro 1 to 4 (the only preset with unassigned macros).
- Targets. At C2, velocity 100: peak -12..-6 dBFS; no sample above 0.95; Brightness 150..600 Hz; Low corr >= 0.99; Width 0.95..1.0; Alias <= -70 dB; no click (note-on and note-off step <= 2x steady);
  bit-identical to the SPEC 1.12 reference render.

---

## C. Coverage check

| Family | Count | Presets |
|---|---|---|
| Riddim | 7 | 1-7 |
| Wobble | 5 | 8-12 |
| Tearout | 5 | 13-17 |
| Formant | 3 | 18-20 |
| Reese / neuro | 4 | 21-24 |
| Subs | 3 | 25-27 |
| Leads | 4 | 28-31 |
| Plucks | 3 | 32-34 |
| FX | 4 | 35-38 |
| Seq | 2 | 39-40 |
| Init | 1 | 41 |
| Total | 41 | Bass 27, Lead 4, Pluck 3, FX 4, Seq 2, Init 1 |

Engine features exercised: all four LFOs; custom shapes (13, 17); Step mode (3, 19, 39, 40); dotted sync (5, 31); triplets (2, 6, 10, 14, 15, 22, 34, 39); Mod Env dives (7, 28, 30, 33, 37); all nine tables;
all four warp types (FM 5, 13, 17, 34; Sync 14, 28; Bend+ 15, 29; PWM 4, 6); all nine filter types; noise (16, 35-38); sub triangle (26); reordered FX chain (3, 13, 15, 16, 21, 22, 24, 29, 36).

## Calibration update (2026-10-18)

From the QA tool's baseline run: Brightness is judged on the amplitude-weighted centroid (a −6..0 dB sub drags the power-weighted centroid below every bass band); hold-window RMS/LUFS bands for Riddim, Wobble, Tearout, Formant, Reese, Sub and Seq are +1.5 dB (−18.5..−14.5 dBFS RMS), because the original bands came from pattern measurements with rests; Init brightness band 300–1000 Hz (amplitude-weighted). Reese beat rate uses the unison voice spacing span/(n−1); FX width checks read correlation (a widening riser lowers correlation).
