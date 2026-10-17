# Riddim / dubstep fundamentals: what a bass synth and its factory bank must do

Written 2026-10-17 for GTS (SPEC.md, `docs/SPEC-v2-quality.md` Tracks 1, 4, 5, 6). Feeds the Track 6
factory taxonomy and the engine gap list in section 3. The legal boundary of SPEC.md applies: no artist
or product preset names; families are described by behaviour and the bank uses our own descriptive names.

Tags. Priority: **MUST** (a bass bank or engine without it reads as a toy for the genre), **SHOULD**
(expected by working producers), **NICE** (differentiator). Evidence: **[S]** published manual, spec or
vendor documentation; **[D]** producer tutorial, educator article or product listing; **[E]**
engineering inference or genre convention not confirmed by a fetched source (verify by ear and by
measurement). Untagged numbers are arithmetic.

Research limits. 14 fetches were allowed and 11 were used. Best sources: Serum growl and robotic-bass
recipes (Bass Gorilla), sub-bass and layering guides (EDM Production, Violet Recording, MusicTech,
Dubspot, Pointblank), reese guides (Future Proof, Noise Engineering, DnB Forum), Vital coverage
(MusicTech, forum) and preset-pack listings (Loopmasters). Several pages returned navigation only (WA
Production riddim article, MusicRadar wobble, Splice talking bass, Dubspot analysis) and the Serum 2
PDF was not machine readable, so Serum 2 facts come from secondary reviews and are tagged [D]. No
source with a dedicated riddim recipe and numbers was found; riddim-specific claims (triplet rates, key
window, call-and-response) rest on [E] plus the generic wobble sources. The soundcy page is low trust.

---

## 1. Sound families a riddim / dubstep bank must cover

### 1.0 Conventions common to all families

| # | Convention | Tag | Basis |
|---|---|---|---|
| C1 | Tempo 140 BPM (half-time feel at 70) is the dubstep default; riddim sits 140-150, with 150 common. Neuro / DnB-adjacent reese at 170-174 is out of scope for the bank but presets must work at any tempo through sync. Factory presets demo cleanly at 140 and 150. | MUST | [D] riddim 140-150 and half-time "70" (wobble / riddim guide extracts); [E] 150 preference |
| C2 | Bass register: root notes F1-A#1 (F1 43.65 Hz, G1 49.0, A1 55.0, A#1 58.27) with E1 (41.2 Hz) as the practical floor; fundamentals below about 40 Hz are not reproduced by many systems. Voice every Bass preset to read at F1, G1 and A#1 and not collapse at E1. Scientific pitch notation, C4 = MIDI 60 (matches the GTS Kbd pivot). | MUST | [D] sub range about 25-80 Hz, power zone just above 40 Hz, heavy keys F / F# / G minor (EDM Production, who label the octave F0-A0); [E] exact F1-A#1 window (the SPEC demo already uses F1/G1/A#1) |
| C3 | Low end mono and clean: sub content mono below about 100-125 Hz, saturation on the mid layer not the sub, layered subs checked for 180 degree cancellation, sub phase adjustable. | MUST | [D] Pointblank, MusicTech, EDM Production, Violet Recording |
| C4 | Width lives above the low end: wide presets (unison, chorus, dimension, reverb) use a wet low cut or `mono_xover` near 120 Hz (GTS already defaults wet low cuts to 120 Hz, xover range 20-300 Hz). | MUST | [D] as C3; [E] 120 Hz |
| C5 | Level discipline: every preset passes the QA level band, peaks under the limiter ceiling at any velocity, within about 2 LU of other bass presets. | MUST | `docs/standards/presets-and-ux.md` items 29, 63; [E] |
| C6 | Four labelled macros per preset that matter for the family (Wobble Depth, Rate, Grit, Formant ...). | SHOULD | SPEC-v2 Track 5; [S] Serum / Vital macro convention |
| C7 | Chain order: EQ low cut and cuts at 200-500 Hz, distortion, multiband (OTT-style), width, limiter; producers often distort twice. | SHOULD | [D] Bass Gorilla growl chain (Diode 2 drive, multiband compressor, EQ low cut at 153 Hz, -15 dB at 110 Hz) |

### 1.1 Riddim (metallic / square, minimal, call-and-response)

- Traits. Sparse, rhythmic, mono bass on a square- or pulse-like core with a hollow, metallic, robotic
  character. Movement is a tempo-synced LFO on wavetable / pulse position and on a high-pass or notch
  filter, over a clean sine sub. Phrases are call-and-response: a short wobble figure answered by a
  different figure or a rest, in 1/8, 1/8T, 1/16 and 1/16T rhythms over a half-time drum pattern.
  [D] sine or square sub plus saw / square mid layer (Violet Recording, soundcy); [D] HP12 filter with LFO
  on cutoff and resonance in the mid 40s of 100 for the sharp edge (Bass Gorilla robotic bass); [D]
  flange and allpass filters for barking tone (WA Production course blurb); [E] triplet emphasis and
  call-and-response.
- LFO. Divisions 1/8, 1/8T, 1/16, 1/16T, 1/4. Shapes Square (gate), Saw Down (pluck into fade),
  Triangle. MUST: shapes and triplets. SHOULD: drawn patterns. One-shot per note for the per-note figure
  (GTS has it).
- Width: mid layer mono or near mono, sub mono, FX width only above ~120 Hz.

### 1.2 Tearout / heavy dubstep (growls)

- Traits. Dense, aggressive, vocal-like movement: wavetable position and warp driven by a drawn LFO, FM
  from a second oscillator (muted in level, never switched off), notch / formant filtering, pitch bends,
  distortion in two stages, multiband compression, phaser / flanger, noisy top layer. [D] Bass Gorilla
  growl: Osc A position about 110 with Bend warp, LFO to warp and position, LFO rate 1/2 in Trigger
  mode, High Notch 12 filter with the LFO on cutoff and on frequency (inverted), Formant filter FX at 40%
  mix with LFO to formant, Diode 2 drive 20, flanger, phaser, multiband compressor, external vocoder,
  OTT and EQ cuts; pitch bend range +/-12 st with drawn bend curves. [D] robotic-bass tutorial: the FM
  oscillator must stay on with only its volume down.
- Level / width. Heavy distortion eats the fundamental: keep a clean sine sub layer or a clean low band
  in the multiband stage [D] EDM Production.

### 1.3 Formant / "yoi" / talking basses

- Traits. Vowel-like sweeps (oo-ee-yoi-ow) from a formant or band-pass resonance moved by LFO or
  envelope, slow (1/4-1/2) for yoi sweeps or fast (1/8-1/16) for chatter, saw or pulse source with
  unison. [D] Serum's Formant filter (MISC group) is described as tuned for vowel transitions as cutoff
  moves; [D] band-pass with moderate resonance linked to an LFO imitates it (Serum growl guide). GTS has
  the Vowel table (Peterson and Barney F1-F3) and Shift; a real formant filter is gap G5.

### 1.4 Wobble / wub (synced LFO bass)

- Traits. Low-pass (or notch / band-pass) filter with a tempo-synced LFO on cutoff over a saw / square
  stack, sine sub below. [D] rates 1/4, 1/8, 1/16 as typical; sine LFO smooth, saw / ramp a pluck into
  fade, square a gated stutter; sine sub keeps the low end steady (wobble guide extracts).
- Rates at 150 BPM: 1/4 = 2.5 Hz, 1/8 = 5 Hz, 1/8T = 7.5 Hz, 1/16 = 10 Hz, 1/16T = 15 Hz, 1/32 = 20 Hz,
  1/32T = 30 Hz (equal to the current free-rate ceiling; at 170 BPM 1/32T exceeds it). At 140 BPM
  multiply by 0.933. MUST: 1/4, 1/8, 1/8T, 1/16, 1/16T. SHOULD: dotted 1/4, 1/8, plus 1/2, 1/1, 2/1
  for slow evolves.
- LFO shapes. MUST: Sine, Triangle, Saw Up/Down, Square. SHOULD: S&H, smooth random, one-shot decay.
  NICE: drawn.

### 1.5 Reese / neuro

- Traits. Two or more detuned saws (or squares) beating against each other, mono, often a 2-voice
  unison with phase randomisation off so the beat repeats; the beat speeds up with pitch; low-pass for
  liquid / dark, notch or band-reject for neuro, then distortion, phaser and OTT. [D] Future Proof (two
  saws, random phase off, or 2-voice unison, mono voicing); [D] Noise Engineering (detuned-saw phase
  cancellation is the motion); [D] neuro forum notes: detune so phasing varies per note, phaser on the
  sub oscillator before distortion, parallel band-reject filters modulated, clipping distortion; [D]
  keep an unmodulated sine layer for the low end.
- Detune. GTS 0-50 ct per outer voice covers reese; neuro wants wider and per-voice variation (G11).

### 1.6 Subs

- Traits. Sine (cleanest) or triangle (a little 3rd harmonic for small speakers); mono, centred, no
  modulation beyond glide; constant level; optional gentle saturation for small-speaker audibility
  (adds harmonics, costs fundamental). [D] EDM Production; [D] Dubspot (sub band about 40-60 Hz, filter
  above 100-200 Hz); [D] DnB forum (sub low-passed 60-70 Hz plus an octave sine near 120-150 Hz).
- Level / width. Mono below 100-125 Hz always; sub about equal RMS to the mid layer before mixing (GTS
  sub default -6 dB re grain RMS).

### 1.7 Leads / screeches

- Traits. Screech: bright saw / sync-like source, fast pitch movement (up bends, pitch dive at note
  start), formant / comb resonance sweeping 1-5 kHz, hard distortion, high-pass so it stays out of the
  bass register; plays about C3-C5. Lead: hoover / supersaw with unison, wide above 300 Hz. [E]: no
  screech tutorial was retrievable; Loopmasters listings show "lead" as a separate pack category [D].
- Pitch / formant movement is the signature, hence G2 (pitch routing) and G4 (sync / FM).

### 1.8 Plucks / stabs

- Traits. Short amp envelope (A 1-5 ms, D 80-300 ms, S 0-0.3, R under 100 ms), filter envelope
  sweeping down, optional quick pitch drop, register C2-C4 [E]. [D] pack listings count plucks as a
  standard category (Dabro Dubstep: 39 wobble basses, 4 plucks, 9 basses, 3 chords, 9 leads).

### 1.9 FX (risers, downlifters, impacts, noise sweeps)

- Traits. Riser: noise or detuned saw through a rising band-pass / high-pass with a pitch rise (96 st
  max in one Vital tutorial, or -12 to 0 st over 8 bars with 5-7 unison voices), EQ / flanger to tame
  brightness [D] (Vital riser tutorial, Unison Audio). Downlifter: the mirror. Impact: pitch-dropped
  sine plus noise burst with long reverb [E]. Noise sweep: band-passed noise under a slow LFO [D] (white
  noise through a modulated filter is the riser basis, Bonedo). FX presets sit 3-6 dB below bass [E].
- GTS has no noise oscillator (Grit table and shaper Noise are the only noise) and its slowest LFO is
  0.03 Hz (33 s) which is fine; the gap is noise (G8) and pitch routing (G2).

### 1.10 Market bank shape (taxonomy evidence)

[D] Loopmasters: Prismatic Bass Presets for Serum targets riddim, dubstep, trap and hybrid with 64
presets including growl and reese-style; The Kraken Vol 1 has 300 presets across bass, plucks, arps, FX,
pads; 100 Serum Presets Dubstep lists bass 48 / synth 30 / drums 15 / FX 7; Ghosthack Dubstep Xplosion
45 bass of 70. Genre banks are half to three quarters bass. `presets-and-ux.md` item 27: 30-60
hand-tuned presets is a credible floor.

---

## 2. Synthesis techniques by family (commercial wavetable synths)

"GTS today" is checked against `params.rs` and `matrix.rs`.

| Technique | Where used | GTS today | Tag |
|---|---|---|---|
| Wavetable position (Index) modulated by synced LFO | riddim, tearout, formant | Index + Motion + Mod A -> Index; matrix | MUST [D] Bass Gorilla (LFO to position and warp) |
| Warp modes (Bend, sync, FM, PWM) | tearout, screech | only Shift and the FM Morph / Square Morph tables | SHOULD [D][S]; G4 |
| Osc A -> Osc B FM (B kept active, level down) | tearout | none between oscillators | SHOULD [D]; G4 |
| Ring mod / AM | metallic riddim, FX | filter type AM (internal sine at Freq) | NICE [E] |
| Formant filter / vowel band-pass | yoi, tearout | Vowel and Reed tables, Shift, BP12; no formant filter | SHOULD [D][S]; G5 |
| Comb / flange filter | riddim bark | Comb+ / Comb- | MUST [D] WA Production; [S] Vital |
| Notch or HP12 with LFO on cutoff | riddim, tearout | no notch, no HP type (HP only in distortion filter and EQ cut) | MUST [D] Bass Gorilla; G3 |
| 24 dB low-pass | wobble, reese | LP12 only; Series of two LP12 gives 24 dB | SHOULD [E]; G3 |
| Unison with detune | reese, leads | 1-7 voices, 0-50 ct | MUST [D] Future Proof; more for leads [E]; G11 |
| Phase randomisation off for repeatable reese beat | reese | `rand_phase`, `phase_reset` | MUST [D] |
| Mono voicing, glide / legato | reese, riddim slides | poly 1, legato, portamento | MUST [D] |
| Two-stage distortion (Diode, tube, fold), filter pre / post | heavy families | one module, 8 types, Pre/Post filter | MUST [D] Bass Gorilla; second stage NICE (G9) |
| OTT-style multiband after distortion | tearout, wobble | `mb_` | MUST [D] |
| Phaser / flanger | tearout, neuro | none | SHOULD [D]; G9 |
| Noise layer | risers, tearout air | none dedicated | SHOULD [D]; G8 |
| LFO retrigger on note | riddim, wobble | always per voice | MUST [D] Bass Gorilla (Trigger) |
| Drawn / pattern LFO | riddim, tearout | nine fixed shapes | MUST for parity [D] Bass Gorilla custom LFO pattern in Trigger mode; Vital paint tool; Serum 2 up to 10 LFOs with independent grid; G1 |
| LFO step rhythms | riddim | Square, S&H as proxies | SHOULD [D] Vital forum (grid LFO as step workaround); G1 |
| Envelope to pitch for punch / drop | plucks, impacts, screech | Mod A one-shot Decay -> pitch +/-24 st (works, occupies Mod A); matrix pitch target is `cent` +/-50 ct | MUST [E]; G2 |
| Sub layering: sine 1-2 octaves down, mono | all | Sub osc -1/-2 oct, -60..0 dB, 25 Hz low cut | MUST [D]; G10 |
| Mid layer one octave above sub, saturation on mid only | riddim, wobble | Osc A/B; Sub joins after filters and is undistorted | MUST [D] Violet Recording |
| Low cut 100-153 Hz and notch cuts 200-500 Hz on processed layer | tearout, riddim | EQ with 24 dB/oct low cut and peak | MUST [D] Bass Gorilla EQ settings |
| Pitch bend +/-12 st with bend curves | tearout, screech | `bend_range` 0-24 | SHOULD [D] |
| Vocoder FX | tearout | none | NICE [D]; G13 |
| Chorus / dimension above low end only | leads, reese | `cho_`, `dim_` with wet low cut | MUST [D] |

Starting recipes from the sources (for bank voicing, not for copying a preset):

- Riddim / robotic [D]: square-ish source; second oscillator as FM source with level down and still on; drawn
  LFO in Trigger mode to wavetable position and level; HP12 filter with the LFO on cutoff and resonance high;
  sine sub; distortion then OTT.
- Growl [D]: LFO to warp and position on two oscillators with different amounts; High Notch 12 with LFO on
  cutoff and (inverted) frequency; Formant filter FX at 40% mix; Diode drive about 20; flanger and phaser at
  low rates; multiband compression; EQ cuts.
- Reese [D]: 2 saws or 2-voice unison, phase random off, mono, optional glide, low-pass; phaser before
  distortion and parallel notches for neuro.
- Riser [D]: pitch rise (-12 to 0 st over 8 bars, up to 96 st in one demo), 5-7 unison voices, noise, EQ and
  flanger to tame highs.

---

## 3. Gap analysis: GTS against riddim / dubstep needs

What exists (read from `params.rs`, `matrix.rs`, SPEC): 9 procedural tables (Square Morph, Saw Stack,
Harmonic Rise, Vowel, FM Morph, Metal, Reed, Grit, Basic), grain Shift, Index, Motion, unison 1-7 with detune
0-50 ct, sine sub (-1/-2 oct), two LFOs (Mod A, Mod B) with nine fixed curves, rates 0.03-30 Hz or sync
{4/1 .. 1/32T} with straight and triplet divisions, one-shot, per-voice retrigger; one filter envelope and two
amp envelopes with curve; two filters (LP12, BP12, Comb+, Comb-, AM) in Parallel / Series / Bypass; shaper (Sine,
Saturate, Clip, Quant, Noise); portamento and legato; FX chain (distortion with 8 types and Pre / Post filter,
OTT-style multiband, EQ with 24 dB low cut, chorus, dimension, delay, reverb, limiter); 32-slot matrix with 18
sources (Mod A/B, Filter Env, Amp Env A/B, Velocity, Keytrack, Random, poly and channel aftertouch, mod wheel,
4 macros, 3 bus CV) and every continuous parameter as a target.

Working workarounds already in the engine: Mod A one-shot Decay into `moda_pitch` is a pitch envelope; Series
routing of two LP12 gives 24 dB; Filter Env and Mod B are matrix sources for any target.

Ranking: effect on riddim / dubstep sound design first, then how many bank presets are blocked. Size: S =
under about a day with tests, M = a few days, L = a week or more.

| Rank | Gap | Why it matters | Standard (who has it) | Size |
|---|---|---|---|---|
| 1 (G1) | **Drawable / step LFO shape**: user shape with 16-32 points or steps, per-point curve, smoothing; plus extra stock shapes (ramp-and-hold, stepped triangle, bounce) | The riddim signature is a pattern inside the wobble that no fixed shape gives; tutorials load a custom LFO pattern at 1/2 in Trigger mode [D]; step rhythms are the call-and-response engine. Without it the bank cannot do riddim and tearout movement honestly. | Serum 1/2 (draw, grid), Vital (paint tool, 8 LFOs), Phase Plant [D][S] | M |
| 2 (G2) | **Pitch routing and pitch envelope**: +/-24..48 st pitch target in the matrix (Mod B, Filter Env, Mod Env, any source), dedicated punch / dive envelope | Plucks, impacts, screech dives, boom hits and "yoi" use fast pitch movement; only Mod A reaches +/-24 st and the matrix pitch target is `cent` (+/-50 ct). Blocks screech and FX presets and makes Mod A do two jobs. | Every wavetable synth: pitch is a target for every modulator [S][D] | S |
| 3 (G3) | **HP12 / Notch / LP24 filter types** (SVF provides HP and notch for free; add a 24 dB cascade or ladder with drive) | Riddim and tearout recipes lean on HP12 and High-Notch filters with LFO on cutoff [D]; wobble wants a 24 dB low-pass; today only LP12, BP12, combs and AM. | Serum (HP12, notch, 24 dB models), Vital (many models) [D][S] | S |
| 4 (G4) | **Oscillator FM (A->B) and hard sync, basic warp (Bend, PWM, Sync)** | Growls and screeches are built on FM from a second oscillator and sync-like warp; GTS has only the internal FM Morph table (fixed 2:1) and Shift. | Serum (FM from B, sync, bend), Vital, Pigments, Phase Plant [D][S] | M |
| 5 (G5) | **Formant / vowel filter mode** (3-peak vowel morph, vowel position as mod target) | Yoi / talking family and the growl "voice"; today only Vowel / Reed tables (pre-filter), Shift and a BP12 workaround. | Serum Formant filters, Vital formant filter [D][S] | M |
| 6 (G6) | **LFO extensions**: dotted divisions (1/2D .. 1/16D), start phase offset, fade-in / delay, smoothing, rate ceiling above 30 Hz, free-run vs retrigger vs host-locked | Dotted rates give the off-beat riddim pocket; phase offset aligns the wobble so the downbeat lands on the open part; fade-in serves risers and evolving growls; audio-rate LFO gives FM-like texture [E]. GTS has triplets but no dotted. | Serum (Env Delay, Anchor, smooth, trigger modes), Vital (7 trigger modes), Phase Plant [D] | S |
| 7 (G7) | **More modulators**: LFO 3 / 4 and a dedicated mod envelope (ADSR + curve) | Two LFOs plus a filter envelope is tight for tearout (position, warp, filter, formant on separate rates). Filter Env is reusable via matrix but is coupled to filter Amt / Inv. | Serum 2 (10 LFOs, 4 envelopes) [D]; Vital (8 LFOs) [D]; Phase Plant | M |
| 8 (G8) | **Noise oscillator / layer** (white, pink, filtered, own level) | Risers, downlifters, impacts, air on screeches, dirt bed in tearout. Currently only the Grit table and shaper Noise. | Serum noise osc, Pigments [D] | S |
| 9 (G9) | **FX: phaser, flanger; reorderable chain; second distortion slot** | Phaser before distortion is a standard neuro move; flanger after distortion in growls [D]; chains like EQ -> dist -> OTT -> dist -> EQ are usual. Chain is fixed (reorder was deferred as NICE in Track 4); raise to SHOULD. | Serum FX rack, Vital FX reorder [D][S] | M |
| 10 (G10) | **Sub upgrades**: sine / triangle selector, phase offset, octave 0, follow glide, soft saturation | Sub phase is the standard fix for layer cancellation [D]; triangle for small speakers. Today sine, phase 0 at note-on, -1/-2 oct only. | Serum / Vital sub osc [D] | S |
| 11 (G11) | **Unison upgrade**: up to 16 voices, detune to 100 ct+, per-voice phase / stereo spread | Supersaw leads and neuro want more; reese works with 2-4 so the gain is lead / FX specific. CPU limit (7 x 2 osc x 8 voices). | Serum, Vital 16 voices [E] | M |
| 12 (G12) | **User wavetable import / more procedural tables** (Screech, Hyper, noisy formant) | Non-goal in v1; extra tables give dedicated screech and growl starting points. Blocks no family. | Serum / Vital / Pigments import [S][D] | L |
| 13 (G13) | **Vocoder / pitch-shift FX, trance-gate FX** | Tearout chains use a vocoder [D]; a trance gate is today a Mod B Vol square LFO. | Phase Plant, Serum 2 [D] | L |

Not gaps (present): HP in distortion filter and EQ low cut, triplet divisions, Comb +/-, unison phase controls,
multiband, glide / legato, mono crossover, macros and drag-to-patch.

---

## 4. Factory bank plan (41 presets)

Conventions (Track 6): the folder is the category; categories are exactly Bass, Lead, Pluck, Pad, FX, Seq, Init
(Pad stays empty in this bank). Free-form tags come from `riddim, tearout, growl, wobble, wub, reese, neuro,
sub, screech, formant, yoi, talking, hybrid, metal, square, triplet, call-response, stab, pluck, lead, riser,
downlifter, impact, sweep, mono, wide, clean, dirty`. Names are descriptive, 2-4 words, unique per folder, no
artist or product names, a number suffix only on collision; author in the author field ("Riddim Rack"). Each
preset has a one-sentence description, 4 labelled macros, a demo note range, and passes the QA level band;
sub mono; wide presets set `mono_xover` or wet low cuts to about 120 Hz. Level: C2 velocity 100 within 2 LU of
other Bass presets after the limiter; FX 3-6 dB lower. `(now)` = buildable on the current engine; `(needs G#)`
= ships with the best honest approximation and is revoiced when the gap closes.

| Family | Category | Count | Defining traits | Buildable |
|---|---|---|---|---|
| Riddim | Bass | 7 | Square Morph / Metal, narrow pulse (index 20-60), mono, legato, sub -6 dB sine; Filter Comb- / BP12 (HP12 after G3); Mod A synced 1/8 and 1/8T with one-shot Decay per-note figures, Mod B Square 1/16 on Index; distortion Hard Clip / Fold, OTT depth 100%, EQ low cut 30 Hz; tags `riddim metal square triplet call-response mono`; two use Metal + AM filter for the robotic bark; one minimal and dry as the reference | 6 now, 1 needs G1 |
| Wobble / wub | Bass | 5 | Saw Stack / Basic, LP12 (LP24 after G3), Mod B -> cutoff; rates 1/4, 1/8, 1/8T, 1/16, 1/16 with shapes Sine, Triangle, Saw Down, Square, Smooth Random; sub -6 dB; macro Wobble Depth; tags `wobble wub clean` | now |
| Tearout / growl | Bass | 5 | FM Morph + Reed / Saw Stack, unison 3-5, Index and Shift modulated by Mod A (1/16, 1/16T) and Mod B (filter); BP12 high Q, comb for metal; distortion Diode / Fold Sine drive 10-24 dB; OTT 100%; chorus / dimension above 120 Hz; clean sub under; macros Grit, Growl Rate, Warp, Width; tags `tearout growl dirty` | 4 now, 1 needs G4 |
| Formant / talking | Bass | 3 | Vowel table (one Reed variant), Mod A on Index at 1/4, 1/2, 1/8, Shift via Mod B, BP12 resonance, mono, Tube distortion; macro Vowel; tags `formant yoi talking` | now; revoice with G5 |
| Reese / neuro | Bass | 4 | Saw Stack / Basic, unison 2-4, detune 8-24 ct, `rand_phase` 0, `phase_reset` on, mono, LP12; liquid dark LP, neuro with Comb- + Fold, wide with chorus above 120 Hz, slow sweep with Mod A Triangle 1/1; tags `reese neuro mono wide` | now |
| Subs | Bass | 3 | Basic table sine, Osc B off; pure sine, warm (soft Tube about 3 dB), distorted sub (Tube 12 dB with clean sub retained); no width; tags `sub clean`; layer sources and load-test | now; G10 adds triangle |
| Leads / screech | Lead | 4 | Two screech (Saw Stack + Metal / Reed, high Shift, unison 5-7, Mod A Decay pitch dive, comb sweep, Hard Clip, 200 Hz HP, C3-C5), one wide hoover-style lead (unison 7, chorus, dimension), one square melodic lead; tags `screech lead wide` | 3 now, 1 needs G2 |
| Plucks / stabs | Pluck | 3 | Amp A 1-5 ms, D 80-250 ms, S 0, R 60-100 ms; filter env Amt 0.5-0.8; one chord stab (unison 4), one pitch-drop sub pluck (Mod A one-shot Decay to pitch), one metal pluck; tags `stab pluck` | now |
| FX | FX | 4 | Riser (Grit + saw, Mod A Attack on Shift and cutoff, 4/1 sync, reverb), downlifter (Decay mirror), impact (pitch-dropped sine, short, reverb about 15%), noise sweep (Grit, BP12, slow Triangle); 3-6 dB lower; tags `riser downlifter impact sweep` | 3 now, 1 needs G8 |
| Rhythm | Seq | 2 | Gated patterns: synced Square / S&H LFO into Volume and Index, call-and-response in 1/8T and 1/16, made for a one-bar held note; tags `riddim call-response triplet`; drawn versions replace them after G1 | now |
| Init | Init | 1 | The v2 init patch (clean mono square bass, sub on, all FX off); always first | now |
| Total | | 41 | Bass 27 (66%), Lead 4, Pluck 3, FX 4, Seq 2, Init 1 | |

Bank acceptance (feeds Track 6): QA renderer passes the level band and peak ceiling for all 41; every Bass
preset carries at least one riddim / dubstep tag from the list; every preset has four meaningful macros; no
preset name or description contains a third-party trademark; at least 20 of 41 use the FX chain; for every Bass
preset rendered at F1, G1 and A#1 the stereo output is within 1 dB of its mono sum below 120 Hz.

---

## Sources

- Bass Gorilla, super growl bass in Ableton Live (fetched): https://bassgorilla.com/serum-tutorial-super-growl-bass-in-ableton-live/
- Bass Gorilla, robotic digital dubstep bass (search extract): https://bassgorilla.com/serum-bass-tutorial-robotic-digital-dubstep-bass/ ; hybrid trap dubstep bass (extract): https://bassgorilla.com/serum-tutorial-snails-bass-hybrid-trap-dubstep-bass/
- WA Production riddim bass course (thin page): https://www.waproduction.com/videos/view/riddim-bass ; https://blog.waproduction.com/how-to-create-virtual-riot-bass (extract only)
- Soundcy, how to make riddim sounds (low trust): https://soundcy.com/article/how-to-make-riddim-sounds
- Future Proof, reese: https://futureproofmusicschool.com/blog/reese-bass-sound-design-everything-you-need-to-know ; Noise Engineering: https://www.noiseengineering.us/blog/reese ; DnB Forum: https://dnbforum.com/threads/neuro-bass-processing-tutorial.178895/
- EDM Production sub-bass (fetched): https://www.edmprod.com/sub-bass/ ; Violet Recording: https://violetrecording.com/how-to-design-a-bass-sound/ and https://violetrecording.com/how-to-make-a-wobble-bass/ ; MusicTech: https://musictech.com/tutorials/weekend-workshop-sculpt-sub-bass/ ; Dubspot: https://blog.dubspot.com/sub-bass-mixing-tips ; Pointblank: https://www.pointblankmusicschool.com/blog/creating-the-perfect-bassline/
- Not retrievable bodies: MusicRadar https://musicradar.com/how-to/lfo-wobble-bass ; Splice https://splice.com/blog/?p=1691
- Vital: https://musictech.com/tutorials/weekend-workshop-sound-design-vital/ ; https://forum.vital.audio/t/16-32-step-sequencer/10309
- Serum: https://splice.com/blog/exploring-the-feautres-of-xfer-records-serum/ ; Serum 2 overview https://create.routenote.com/blog/serum-2-everything-you-need-to-know/ ; Xfer PDF (unreadable here) https://static.xferrecords.com/Serum%202%20What's%20New.pdf
- Risers: https://unison.audio/how-to-create-risers/ ; https://www.bonedo.de/artikel/tutorial-sounddesign-mit-kilohearts-phase-plant
- Pack taxonomy: https://www.loopmasters.com/genres/49-Dubstep/products/7511-Prismatic-Bass-Presets-For-Serum ; https://www.loopmasters.com/genres/49-Dubstep/products/13688-100-Serum-Presets-Dubstep ; https://www.loopmasters.com/genres/49-Dubstep/products/11861-The-Kraken-Vol-1-Dubstep-Serum-Presets
- Project: `SPEC.md`, `docs/SPEC-v2-quality.md`, `docs/standards/presets-and-ux.md`, `crates/gts/src/params.rs`, `crates/gts/src/matrix.rs`
