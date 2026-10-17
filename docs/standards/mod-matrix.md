# Modulation matrix and drag-to-patch standard

Researched 2026-10-16. Tags: [S] stated in a fetched or searched source, [D] consensus across products or reviews, [E] convention or inference (verify before relying on it). Many official manual pages were unreachable (Serum 2 manual 404, Pigments manual 403, Vital slot count not stated), so those items are tagged [D] or [E] and listed under Unverified.

Sources used: Kilohearts Phase Plant docs (kilohearts.com/docs/phase_plant, /docs/modulation); Vital user guide (davidmvogel.com/docs/Vital/UserGuide/Modulation); NI Massive X manual (docs.native-instruments.com/online-guides/massive-x-manual/en/modulation); FabFilter Twin 3 help (fabfilter.com/help/twin/using/modulationslots); monosounds.studio Serum 2 modulation guide (third party); KVR and review snippets for Serum 2, Vital, Pigments.

## 1. Matrix size and shape

- MUST: a matrix/list view that shows every routing in the preset as a row with source, amount, destination. [S] Serum 2 (rows: source, amount, destination), [S] Vital (Matrix tab lists every mapping).
- MUST: per-row polarity (bipolar/unipolar) toggle and a per-row amount control. [S] Vital (bipolar button, amount slider per row), [S] Serum 2 (flip unipolar/bipolar in matrix).
- MUST: per-row bypass/mute that keeps the routing. [S] Vital (click the row number to bypass; per review), [S] FabFilter Twin 3 (hover slider shows on/off button).
- SHOULD: slots are effectively unlimited or large; Vital reportedly 64 slots [D] (KVR forum mention, not confirmed in manual); Massive X uses a fixed 2 slots plus 1 sidechain slot per parameter [S]; Phase Plant has no stated cap ("open ended") [S].
- SHOULD: aux/side-chain source that scales the primary source's amount. [S] Serum 2 (aux source scales the first; example velocity scaling an envelope), [S] Massive X (sidechain slot between the two slots).
- SHOULD: response curve per row (linear/log or shaped). [S] Serum 2 (curve reshapes response), [S] Vital (matrix edits response curve, linear or logarithmic).
- SHOULD: per-row stereo toggle only if the synth is stereo-modulated; Vital has it [S], skip for GTS unless voices are stereo.
- NICE: "delete/bypass/invert all" menu per target. [S] FabFilter Twin 3.
- NICE: list sorting/filter. No source states it; [E] sort by source or destination is a common convention, do not treat as a standard.

## 2. Sources offered

- MUST: 2+ LFOs, envelopes, velocity, mod wheel. [S] Phase Plant (LFOs, envelopes, random, MIDI, pressure, velocity), [S] Massive X (19 macros including pitch bend, mod wheel, aftertouch).
- MUST: aftertouch/pressure as a source. [S] Phase Plant, [S] Massive X.
- SHOULD: note/keytrack source and random-per-note. [S] Vital (Note sources, Velocity, Oct Note; Random modulates "each time a note is played"), [S] Phase Plant (Note modulator, Random modulator).
- SHOULD: macros, 4 to 16, drivable to several destinations. [S] Serum 2 (4 macros, one macro drove 5 destinations in the guide), [S] Phase Plant (8 macros, one routes to several parameters), [S] Massive X (16 macro knobs).
- SHOULD: MPE pressure and timbre sources. [S] Phase Plant (Pressure modulator MPE compatible; MPE Timbre; Note modulator catches per-note bend).
- Per-voice vs global: Phase Plant draws the most recent voice's modulator value in blue and other voices in grey, with a separate global value for some modulators [S]. Serum 2 LFOs have Trig (restart on note) and Off (free-run vs host clock) modes [S]. Rule of thumb for GTS: LFO with retrigger = per-voice, free-run = global. [E]
- SHOULD: for per-voice sources feeding global targets (e.g. FX), the resolution rule is not stated in any fetched source. [E] Recommendation: use a mono (global) LFO or the newest voice's value, never sum voices; document it in the UI.

## 3. Destinations

- MUST: any continuous parameter can be a destination. [S] Phase Plant ("almost any parameter"), [S] Vital (drag to any parameter), [S] Massive X (any number of parameters).
- SHOULD: modulation amount of another slot can be a destination. [S] Vital (modulate the amount of a modulation with a macro), [S] FabFilter Twin 3 (slots modulating other slot levels appear indented), [S] Phase Plant (depth can itself be modulated).
- SHOULD: effects parameters are destinations. [S] Phase Plant (effects in the modulation system, per docs overview), [D] Vital (FX parameters modulatable).
- NICE: enums/discrete params. No source states it; [E] treat as unsupported or step-quantized, hide from highlight set.

## 4. Amount semantics

- MUST: depth is in normalized parameter space (fraction of the param's range), displayed as a ring/arc on the knob. [D] Vital/Serum/Massive X rings show range travelled; Serum 2 ring "shows how far and how fast it travels" [S].
- MUST: bipolar swings both directions "around where the knob sits". [S] Serum 2 guide.
- MUST: result clamps to the parameter's range after summing. [S] Phase Plant warns the total of modulations "may lead to values outside the bounds of each individual modulation" so clamp the sum at the end; [E] clamping at the destination.
- MUST: multiple slots to one target sum (additive) in normalized space. [S] Phase Plant (each modulation its own row, totals add), [S] FabFilter Twin 3 (several sources per control), [S] Massive X (two slots per parameter).
- SHOULD: negative amount equals inversion. [S] Massive X (drag down = inverted), [S] FabFilter Twin 3 (+/- button inverts).
- NICE: nonlinear parameter taper: apply depth in the param's normalized (skewed) space so the ring matches the knob feel. [E]

## 5. Drag-to-assign UX

- MUST: dragging a source highlights valid targets; drop creates the routing. [S] Vital (modulatable parameters highlighted; mapping occurs only on mouse release), [S] FabFilter Twin 3 (targets light up when dragging begins).
- MUST: drop on an unmodulated knob creates a slot; ring appears immediately. [S] Serum 2 guide.
- MUST: depth edited by dragging the ring/arc on the knob. [S] Serum 2 guide (drag the ring), [S] Vital (small ring on target sets amount and direction; per forum).
- SHOULD: auditioning while dragging (hover the target while playing). [S] Vital.
- SHOULD: modifier keys: Shift fine adjust, Ctrl/Cmd-click reset. [S] FabFilter Twin 3. Alt edits all of a source's slot levels. [S] FabFilter Twin 3.
- SHOULD: right-click on a modulated target lists its modulations with "Disconnect from X" and "Disconnect All". [S] Vital. Right-click a slot offers Delete [S] Massive X; right-click a source mutes/deletes all of its assignments [S] Massive X.
- SHOULD: right-click ring offers Unipolar/Bipolar. [D] Vital forum answer.
- SHOULD: drop on an already-modulated knob adds a second slot when the source differs; same source updates the existing slot (avoid duplicate source-target pairs). [E] (Massive X replaces an occupied slot and keeps the amount [S]; FabFilter lists multiple [S].) Pick: same source = edit, different source = add.
- SHOULD: a click-then-click path as drag alternative. [S] Massive X (click assign icon, then click slot), [S] Phase Plant (click "+" on module, targets light up).
- SHOULD: ring colour = source colour; a control with multiple sources shows one dot per source type. [S] FabFilter Twin 3 (colored dots), [S] Massive X (color-coded rings by source type).
- SHOULD: selecting a source highlights all its destinations. [D] Serum 2 (halo only active when corresponding modulator selected, KVR), [S] Phase Plant ("+" lights all destinations).
- Default amount on drop: not stated in any fetched source (Phase Plant docs explicitly silent). [E] Common choices: 0 then drag, or a visible default around 50%. Recommend 50% bipolar-off (positive) so the drop is audible but not extreme.

## 6. Macros

- MUST: macros are labelled and one macro drives several destinations at independent depths. [S] Phase Plant, [S] Serum 2.
- SHOULD: 4 to 16 macros. [S] Serum 2 (4), Phase Plant (8), Massive X (16).
- SHOULD: macro values saved in presets. [E] (no source states it; universal practice).
- SHOULD: macros expose as DAW-automatable parameters. [E] (no fetched source states it; standard for host automation of macro knobs).
- NICE: user-renamable macros and colours. [E] Serum 2 guide does not say names are editable; Vital macros have an assignable display area [S].

## 7. Persistence

- MUST: routings (source, destination, amount, polarity, curve, aux) live in preset state and project state, not in transient UI state. [E] (no fetched source quantifies this; Vital Matrix tab shows "all mappings in your preset" [S]).
- SHOULD: modulation edits are undoable. No source states undo coverage. [E] Treat each completed drag/disconnect as one undo step.
- SHOULD: load order independence: store destination by stable param ID, not index. [E]

## 8. Performance norms

- Phase Plant: generators update at audio rate, modulators at control rate, typically every 64 samples; fast modulation (100 Hz+) should use the audio-rate path. [S]
- Vital advertises audio-rate modulation for drum transients and filter-FM. [D] (feature-list claim in reviews).
- SHOULD: control-rate updates in 16 to 64 sample blocks with per-block smoothing/interpolation for audible params. [E] (supported by Phase Plant's 64 samples [S]).
- NICE: audio-rate path only for a small named target list (pitch, filter cutoff, wavetable index). [S] Phase Plant, [D] Vital.

## Recommendation for GTS (fixed routes today, about 182 params, egui, source chips and live rings drawn)

1. Replace fixed routes with a flat slot array (start 32, grow to 64; flat Vec of {source, dest param id, amount f32 in [-1,1], bipolar, enabled}). Back it with host-visible persistence. Matches Vital [D] and Phase Plant [S] shapes.
2. Keep current fixed routes as default slots at preset load (migration), so existing presets sound identical.
3. Sources in v1: Mod A, Mod B, filter env, amp env(s), velocity, mod wheel, aftertouch, key track, random-per-note, 4 to 8 macros, bus CV1-3. Treat retriggered LFOs and envelopes as per-voice; free-run LFOs as global. For FX targets use the global/mono source or newest voice; never sum voices. [E]
4. Destinations: every continuous param (skip enums); amounts of other slots allowed for Mod B to Mod A amount (already present) generalised to slot-amount targets, with a depth-1 limit to avoid cycles. [S] Vital, FabFilter.
5. Amount semantics: normalized, additive across slots, clamp the sum once at the destination, bipolar flag per slot. Negative amount inverts. [S/E]
6. Drag-to-patch: dragging a source chip highlights every valid knob (the existing chips and live rings make this cheap); drop on unmodulated knob creates a slot at +50%; drop of same source on a modulated knob edits that slot; different source adds a slot with its own ring colour. Release-only creation, no mapping on hover. [S] Vital.
7. Edit depth by dragging the knob ring; Shift = fine, Ctrl/Cmd-click = reset, Alt = all of that source's slots. Right-click ring: Bipolar/Unipolar, Invert, Bypass, Disconnect, Disconnect All. [S] FabFilter, Vital.
8. Matrix tab: one row per slot with bypass number, source selector, polarity, amount slider, destination selector. Add aux source and curve columns in a later phase (SHOULD). [S] Vital, Serum 2.
9. Source highlight: selecting a source chip lights all of its destination rings.
10. Rate: evaluate sources per 32 to 64 samples with linear smoothing between; keep pitch/cutoff at sample or short-block rate with interpolation. [S] Phase Plant.
11. Undo: one history entry per drag-release or menu action; persist slots via stable param IDs.

## Unverified

- Serum 2 official manual: slot count, default amount, aux behavior (manual and Xfer URLs returned 404).
- Vital: slot count of 64, per-voice vs global resolution, right-click wording for bipolar.
- Pigments manual: not fetched (403); voice vs global rules not confirmed.
- Massive X: macro count taken from a search snippet, not the fetched page.
- Default drop amount, undo coverage, preset storage of macro labels: no source states them.
