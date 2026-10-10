# Research: Reason devices in riddim production (2026-10-10, web survey)

Scoping input for SPEC.md. Behavior reference only; nothing here is copied content.

**Typical chain:** Malstrom (mono, legato, Square 4 graintable, Mod/One-Shot wobble, Comb/BP12 filter, Shaper) -> Scream 4 (Damage + Cut + Body) -> Alligator (pattern chop) and/or Pulveriser -> MClass compressor/EQ/limiter with kick sidechain (ordering inferred from sources; no single tutorial states it)

## Devices by frequency in riddim tutorials

### Malstrom — synth — core
Graintable oscillator whose Index/Motion/Shift sweeps plus comb/AM filters and shaper give the metallic, vocal, formant-moving square-ish riddim bass; the standard source patch.

Key controls:
- Osc A (often osc B off or detuned): graintable select, Index 0-127, Motion (centre default; fully left freezes to a static waveform), Shift (formant-like change without pitch change), Octave -4..+4, Semi, Cent
- Polyphony 1 (mono) + Legato on + Portamento (Synthtopia tutorial used polyphony 1, porta 36)
- Mod:A routed to Index/Shift/Pitch; Mod:B to Motion/Vol/Cutoff/Mod:A amount; tempo Sync (16 divisions) and One Shot (envelope-like) for wobble and growl
- Filter A/B type: LP12, BP12, Comb+, Comb-, AM; Res shapes comb peaks or AM mix; Kbd tracking (BP12 res ~61 with Kbd in the Synthtopia bass)
- Shaper (Sine/Saturate/Clip/Quant/Noise) + Amt before filter A
- Filter env Amt and Inv, amp ADSR, Spread
- Mod wheel to Index/Shift/Filter freq/Mod amount; velocity to Lvl, F.env, Atk, Shift, Mod
Signature settings:
- Square 4 graintable (Synthtopia dirty electro bass used it on osc A; Square4 is the 'OG riddim' basis, per the tutorial titles/packs 'Malstrom Monsters')
- Comb+/Comb- or AM filter for metallic/ring tone (recommended by tutorials/third-party guides; specific values not stated)
- Shaper Saturate/Clip into BP12 with high resonance (Synthtopia bass)
- FM and metal-style graintables with mod-wheel moves toward distortion (reasonexperts bass patches page)
- Exact graintable names beyond Square 4 are not verified in sources found

### Scream 4 — distortion — core
Sound Destruction Unit used after the synth to add harmonics, tearing and cabinet-like formant resonance; Damage + Body (Scale/Auto) is the main growl-shaping stage.

Key controls:
- Damage on/off, Damage Control (input gain), Damage Type (10)
- P1 and P2, whose meanings change by type
- Cut on/off with Lo/Mid/Hi sliders, +/-18 dB
- Body on/off, Type A-E, Reso, Scale (inverted size), Auto (envelope follower into Scale)
- Master level (compensate for gain)
- CV inputs for Damage Control, P1, P2, Scale; Auto CV output
Signature settings:
- Damage types most mentioned for aggression: Scream, Modulate, Warp, Digital, Feedback, Fuzz (which exact type tutorials favour is not confirmed by sources found)
- Body Reso high with Scale automated (or Auto envelope) for vowel-like movement
- Scream 4 on top of bass to open up the sound (reasonexperts)

### Alligator — gate — core
Pattern-driven 3-band filtered gate with per-band drive/phaser/delay for rhythmic chopping, stuttering and movement in the bass.

Key controls:
- Pattern on/off, 64 patterns (3 tracks each), Resolution (default 1/16), Shift +/-16 steps, Shuffle
- Manual trig buttons; MIDI F#1/G#1/A#1; CV gates
- Per channel: amp attack/decay/release, filter on/off, freq, res, env amount, LFO amount, drive, phaser amount, delay send, pan, volume
- Shared phaser rate and feedback; global delay time/sync/feedback/pan; global LFO (9 waveforms, sync)
- Dry volume, dry pan, dry ducking, master volume
Signature settings:
- Percussagator preset recommended as a starting point (MusicTech)
- Short decay and attack for snappy chops; raised release for pad-like blur
- Pattern numbers favoured by riddim tutorials are not verified

### Pulveriser — distortion — common
Distortion/compressor/filter/tremolo mangler used for crush and rhythmic gain pumping on bass buses.

Key controls:
- Squash (compressor), Dirt (distortion), Tremor (modulation of filter and volume with Amount dials), filter section, Blend/Dry-wet, Output
Signature settings:
- Tremor amounts to the filter and volume for rhythmic modulation (MusicTech); no riddim numeric values found

### Thor — synth — occasional
Alternative bass source; its Shaper oscillator and wavetable oscillators are used for layering aggressive bass.

Key controls:
- Wavetable/Shaper oscillator, filters with Comb/Formant, modulation bus
Signature settings:
- Specific riddim values not found

### Subtractor — synth — occasional
Simple sub or layering oscillator beneath the main bass.

Key controls:
- Osc waveform, Osc 2 detune, low-pass filter, FM
Signature settings:
- Sub sine layer

### RV7000 — other — occasional
Reverb/gated reverb for FX and impacts rather than the main bass.

Key controls:
- Algorithm, decay, EQ

### MClass Compressor (and mixer sidechain) — dynamics — common
Glues and tames the distorted bass; Synthtopia's bass used MClass with fast attack, threshold ~29.2 and an infinite:1 ratio, plus sidechain pumping.

Key controls:
- Threshold, ratio, attack, release, makeup gain
- Sidechain with kick
Signature settings:
- Threshold ~29.2, ratio infinite:1, fast attack (Synthtopia, Reason 4 dirty electro bass)

## Architecture notes (from the official operation manual)

### Graintable synth (Malström lineage)
Two oscillators (A,B), each a graintable player: graintable select, Octave -4..+4, Semi 0..12, Cent +/-50, Index 0-127 (start grain, spans full table regardless of grain count), Motion (playback speed of fixed Forward or Forward-Backward pattern; centre default, full left freezes), Shift (resamples each grain: changes harmonic content/formant while pitch stays constant), Vol. Each osc has its own ADSR amp envelope. Two LFO modulators: Curve (waveform), Rate (or 16 sync divisions), Sync, One Shot (envelope mode), A/B/both selector; bipolar destination knobs. Mod:A targets Pitch, Index, Shift; Mod:B targets Motion, Vol, filter Cutoff, and Mod:A amount. Optional Shaper (Sine, Saturate, Clip, Quant, Noise; Amt) sits before filter A. Two identical filters A/B: LP12, BP12, Comb+, Comb-, AM (ring mod with internal sine; Freq becomes sine freq, Res becomes dry/modulated mix); Kbd tracking, Env on/off, Freq, Res (comb feedback shape). Shared filter ADSR with Amt and Inv. Routing: bypass, single filter, A->FA and B->FB (stereo), A to both filters in parallel (only A), series, and shaper in various positions (e.g. B->FB->Shaper->FA). Output Volume, Spread. Polyphony 1-16, Portamento, Legato, pitch bend up to 24 semitones, mod wheel (Index, Shift, Filter freq, Mod amount). Velocity (bipolar) to Lvl A/B, F.env, Atk, Shift, Mod amounts. CV/gate in and many mod CV inputs/outputs; osc A/B direct outs bypass filters and shaper. Not stated in manual: LFO waveform list, exact Hz ranges, graintable contents (Square 4 etc. need to be recreated/analysed separately).

### Damage unit (Scream 4 lineage)
Stereo effect with three independently switchable sections, Damage, Cut, Body, plus Master level; explicit processing order is not stated in the manual (assume Damage -> Cut -> Body, to verify by ear). Damage: Damage Control (input gain/amount) and 10 types with P1/P2: Overdrive (Tone, Presence), Distortion (Tone, Presence), Fuzz (Tone, Presence), Tube (Contour, Bias: symmetric centre gives odd harmonics), Tape (Speed, Compression), Feedback (Size of loop, Frequency of howl; Damage Control = loop gain), Modulate (Ring=filter resonance, Frequency=cutoff; multiplies signal with a filtered, compressed copy of itself then distorts), Warp (Sharpness, Bias restores fundamental; self-multiplication removes fundamental), Digital (Bit resolution to 1-bit, Sample rate reduction), Scream (Tone, Filter frequency: high-resonance bandpass before distortion). Cut: 3-band EQ (lo, mid, hi), +/-18 dB each; corner frequencies not documented. Body: cabinet/enclosure resonator, Types A-E, Reso, Scale (size; clockwise = smaller), Auto (envelope follower amount modulating Scale from input level; Auto CV output). CV inputs for Damage Control, P1, P2, Scale. Some Damage algorithms misbehave below 14 kHz sample rate. No dry/wet control documented.

### Pattern band gate (Alligator lineage)
Stereo input split into three parallel channels: gate1 high-pass, gate2 band-pass, gate3 low-pass (independent, overlapping, no crossover). Per channel: gate -> amp envelope (attack, decay (max = infinite sustain), release) -> filter (HP/BP/LP, freq, res, bipolar envelope amount, bipolar global LFO amount, shared filter env controls) -> drive -> phaser (amount per channel; rate and feedback shared) -> delay send -> pan and volume -> mix. Single global delay (time up to 1 s or synced, feedback, pan, per-channel send post-volume). Dry path with dry volume, pan, optional ducking (inverted amp envelope). Master volume. Gates triggered by 64-pattern player (3 tracks each, mostly 2 bars; Resolution default 1/16; Shift +/-16 steps; Shuffle on every second 1/16 using ReGroove global shuffle), manual trig, MIDI F#1/G#1/A#1, or CV gate (>=7 opens, higher values raise level). Global LFO with 9 waveforms, rate/sync. Per-channel outputs tapped before master. CV inputs for filter freqs and LFO rate; gate outputs mirror gate state. Numeric ranges and the actual pattern data are not in the manual and need to be recreated by ear.

## Sources
- https://docs.reasonstudios.com/reason14/malstrom-synthesizer
- https://docs.propellerheads.se/reason9/Scream4.html
- https://docs.reasonstudios.com/reason14/alligator-triple-filtered-gate
- https://docs.propellerheads.se/reason9/Malstrom.32.2.html
- https://docs.propellerheads.se/reason9/Malstrom.32.3.html
- https://www.synthtopia.com/content/2009/01/28/create-a-dirty-electro-bassline-in-reason-40/
- https://www.reasonexperts.com/malstrom-bass-patches.html
- https://musictech.com/2015/09/reason-rhythmic-sbs2/
- https://www.reasonstudios.com/news/post/dirty-reese-bass-custom-patch-design-2
- https://www.reasonstudios.com/news/post/sound-design-with-suspicious-stench
- https://www.youtube.com/watch?v=qvj_dmvBIqs
