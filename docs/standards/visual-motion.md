# Visual and motion standard for soft-synth editors

Purpose: checklist for the GTS editor visual/motion pass. Tags: MUST / SHOULD / NICE; evidence [S] stated by vendor/manual, [D] developer or reviewer consensus, [E] engineering convention. "Verified" = read on the cited page this session; "unverified" = recalled or inferred, check hands-on before relying on it. Several vendor pages were thin; gaps are listed in section 7.

## Sources actually read
- Vital product page (vital.audio): "All animations run at 60 frames per second and are GPU optimized"; controls, filter responses, waveforms, LFOs, oscilloscope, spectrogram shown in real time; drag-and-drop mod with preview before commit; extra skins. Verified.
- Vital coverage (Synthtopia, MusicTech, Sonic State, 2020): animated real-time graphs, 3D waveform view option, scope/spectrum toggled by click, mod preview while dragging. Verified via search summaries.
- Serum 2 manual, "Using Knobs and Sliders" (xferrecords.com/web-manual/serum-2): drag shows a popup with the current value; Shift = fine; double-click = text entry; Ctrl/Cmd-click or context menu = reset; scroll wheel changes value without popup. Verified.
- Serum / Serum 2 reviews (MusicRadar, KVR, Future Proof): 2D waveform by default, click for 3D view that marks the current frame; animation of all modulation praised; Serum 2 adds smooth interpolation between frames and visual feedback for active modulations. Reviewer-level, verified via summaries.
- Kilohearts Phase Plant docs: generator modules draw a scope of their output from the real signal path (fixed frequency); granular/wavetable modules show the loaded sample or table, granular shows live grains and a draggable cursor; mod targets change colour (blue to green) when modulated; mod drag via a plus icon. Verified. Rings on knobs not stated there.
- Ultra / Evoke manuals (secondary): mod ring around a knob shows mod range; double-click reset and Shift fine adjust are common. Verified, other products.
- NN/g "Response Times: The 3 Important Limits": about 0.1 s = feels like direct manipulation; feedback must land inside it; 0.2-1 s noticeable. Verified.
- JUCE / HISE / KVR developer threads: cache the static background, redraw only moving parts; timer-driven repaint gives uneven frame pacing; vector drawing on a CPU rasteriser costs per frame; bitmap knobs resample at high DPI. Verified via search summaries.
- NI / Steinberg / Waves forums: persistent complaints about blurry HiDPI, non-scaling controls and text, resize that only grows the visualiser, tiny text, black borders. Verified via search summaries.
- egui docs: request_repaint_after(d) keeps the smallest requested duration; zoom factor / pixels_per_point settable. Verified.
- Not retrievable this session: Material 3 motion tokens, Apple HIG motion, Pigments / Massive X / u-he / Kilohearts snapin manuals, Vital guide. Material-style numbers below (about 100-350 ms, cubic-bezier(0.2,0,0,1)) are recalled and unverified.

## 1. Hero display
| # | Rule | Tag | Evidence | Products |
|---|------|-----|----------|----------|
| 1.1 | A large wavetable/grain display is the visual centre and the largest single element. Target >= 35-40% of window width, >= 25% of height. GTS now: 300x124 on 1000x832, about 4.5% of area | MUST | D (Serum, Vital, Pigments screenshots) | Serum 2, Vital, Pigments |
| 1.2 | Offer a 2D current-frame view and a 3D stacked-frames view, toggled by clicking the display | SHOULD | S/D | Serum (click for 3D), Vital (3D option) |
| 1.3 | Current position is unambiguous: in 3D the active frame is highlighted (brighter, thicker, accent colour) among dimmer frames; in 2D the drawn waveform is the live post-warp shape | MUST | D | Serum, Vital |
| 1.4 | Display reflects warp/position changes live while dragging and while modulated (not on release); draws the interpolated frame the DSP actually plays | MUST | S (Serum 2 smooth interpolation; Vital realtime) | Serum 2, Vital |
| 1.5 | For granular (GTS): source waveform with read window/position and live grains moving over it; scatter/size visibly respond | MUST | S (Phase Plant granular view shows playing grains and draggable cursor) | Phase Plant |
| 1.6 | Display is directly editable (drag position/range on the display) | SHOULD | S (Phase Plant draggable cursor) | Phase Plant |
| 1.7 | Bright 1.5-2 px stroke, subtle gradient under the curve, faint grid; no flat hairlines | SHOULD | D | Vital, Serum |

## 2. Live motion
| # | Rule | Tag | Evidence | Products |
|---|------|-----|----------|----------|
| 2.1 | Live displays animate at 60 fps (vsync). Vital states 60 fps outright; 30 Hz reads as stepping on moving parts | MUST | S (Vital) + E | Vital |
| 2.2 | Repaint only when something moves (voices active, mod nonzero, gesture); idle = no repaint; cache static parts | SHOULD | D (JUCE threads) + E | all |
| 2.3 | Mod rings/arcs on knobs: arc showing range plus a dot/arc following the live modulated value; per-source colour | MUST | S (Ultra ring; Serum 2 active-mod feedback; Phase Plant colour change), D (reviewers praise animated mod) | Serum 2, Vital, Phase Plant, Pigments (unverified) |
| 2.4 | LFO and envelope editors show a moving playhead; with voices, show most recent/loudest voice or a marker per voice (cap about 8), fading on release | MUST | D (per-voice detail unverified) | Vital, Serum 2, Pigments |
| 2.5 | Oscilloscope + spectrum on output, click to toggle, fed from the audio thread via lock-free ring (never blocking) | SHOULD | S (Vital) | Vital |
| 2.6 | Filter response curve moves with cutoff/resonance and with modulation, not only knob drags | SHOULD | S (Vital) | Vital, Serum |
| 2.7 | Output meter: instant rise, fall about 300-600 ms, peak hold | SHOULD | E | industry |
| 2.8 | Voice/activity indicator | NICE | D | Pigments, Phase Plant |
| 2.9 | Param change to pixels within 1-2 frames (<= 33 ms at 60 fps); same-frame coupling between knob and display | MUST | E + S (NN/g 0.1 s ceiling) | all |
| 2.10 | Read audio-side state from atomics each frame; smoothing only light (time constant <= 30-50 ms) to avoid jitter without lag | SHOULD | E | |

## 3. Interaction feedback
| # | Rule | Tag | Evidence | Products |
|---|------|-----|----------|----------|
| 3.1 | Value popup near the control while dragging (units, sensible precision), hides about 300-500 ms after release | MUST | S (Serum 2 popup) | Serum 2, Vital |
| 3.2 | Shift = fine adjust (about 10x slower) | MUST | S (Serum 2, Evoke) | Serum 2, Evoke |
| 3.3 | Reset by double-click and/or Ctrl/Cmd-click | MUST | S (Serum 2 Ctrl/Cmd; Evoke double-click) | Serum 2, Evoke |
| 3.4 | Typed entry (double-click or right-click); scroll wheel adjusts | SHOULD | S (Serum 2) | Serum 2 |
| 3.5 | Hover state on every control within 100 ms (subtle brighten/outline); cursor changes on draggables | MUST | E + S (NN/g) | all |
| 3.6 | Mod assign by drag from source onto target; valid targets highlight; live preview before drop | MUST | S (Vital preview; Phase Plant drag) | Vital, Phase Plant |
| 3.7 | Tab/page switch instant or <= 120-150 ms crossfade; never a slow slide | SHOULD | E (unverified) | |
| 3.8 | Dropdown open 100-150 ms fade + 4-8 px translate; close faster (about 80-100 ms) | NICE | E (unverified) | |
| 3.9 | Preset browser: keyboard nav, no frozen UI on load, selection highlight eases about 100 ms | SHOULD | D | Serum 2, Vital |
| 3.10 | Durations: micro (hover/press) 80-120 ms, state change 150-250 ms, large panel 250-350 ms; ease-out entering (cubic-bezier(0.2,0,0,1)), ease-in exiting; no bounce on parameter visuals | SHOULD | E (unverified numbers) | |
| 3.11 | Animate chrome only; data displays track the value at 60 fps without easing lag | MUST | E | |

## 4. Depth and finish
| # | Rule | Tag | Evidence | Products |
|---|------|-----|----------|----------|
| 4.1 | Restrained palette: one dark neutral family, one accent plus per-mod-source accents; saturation reserved for live/active elements | MUST | D | Vital, Serum 2, Pigments |
| 4.2 | 1-2 elevation levels: faint 1 px top highlight (6-10% white), soft shadow; avoid heavy bevels and texture noise | SHOULD | D | Vital, Pigments |
| 4.3 | Glow only on active signal (curve, mod dot): blur 4-10 px, alpha <= 35%; never on static chrome | SHOULD | D | Vital |
| 4.4 | Knobs: thin track arc + value arc + indicator, arc >= 2 px, 3 consistent sizes | MUST | D | all |
| 4.5 | One sans family, 2-3 sizes, body >= 11-12 px at 1x, consistent label placement | MUST | D (tiny-text complaints) | |
| 4.6 | Icons vector, one grid and stroke weight (1.5 px) | SHOULD | D | |
| 4.7 | Spacing from a 4/8 px base unit, grid-aligned sections; balance density and breathing room | MUST | D | |
| 4.8 | Resize/scale must scale controls and text, not only the visualiser; steps 100/125/150/175/200%, remember choice, honour host DPI | MUST | D (many complaints) | Vital, Serum 2; NI/iZotope as negative |
| 4.9 | Everything vector-drawn (egui painter), no raster knob sprites | SHOULD | D + E | |

## 5. What makes an interface feel cheap
- Static or stepping displays (30 Hz, repaint only on interaction) beside products that animate everything. [D]
- Lag between knob and display, or display updating only on release. [D/E]
- Aliased or hairline strokes: use >= 1.5 px curves at 1x, pixel-snapped grid lines, antialiased paths. [E]
- Blurry text/controls on HiDPI; resize that grows only the centre; tiny text. [D]
- Inconsistent spacing, mixed control sizes or fonts. [D]
- Dead space: empty regions or a tiny display in a big window. [D]
- No value feedback while dragging, no tooltips, no reset gesture. [D/S]
- Jitter from unsmoothed audio readbacks or uneven frame pacing (timer repaint vs vsync). [D]
- Glow and saturated colour everywhere; default grey widget look. [D]

## 6. Measurable criteria
- 60 fps while anything moves; frame time <= 16.7 ms; no frame > 33 ms in a 10 s capture. Idle: zero repaints.
- Knob-to-display: same frame, <= 33 ms worst case, 100 ms hard limit.
- Hover/press feedback <= 100 ms. Drag popup on first movement; hides 300-500 ms after release.
- Durations: micro 80-120, state 150-250, panel 250-350 ms; ease-out entering.
- Strokes: curves >= 1.5 px at 1x, grid lines 1 px on pixel boundary, mod arcs 2-3 px.
- Hero display >= 35% window width. Text >= 11-12 px at 1x. Scale steps 100/125/150/175/200%.
- Meter: rise instant, fall 300-600 ms.

## 7. Evidence gaps
Pigments, Massive X, u-he, Kilohearts snapins: no manual pages retrievable; claims there are reviewer-level. Material/Apple motion numbers are recalled. Per-voice playhead behaviour and exact mod-ring visuals in Serum 2 and Vital are unverified: confirm by running the products or watching demo footage. Serum 2 manual did not describe wavetable display animation.

## 8. Recommendation for the GTS editor (now: static egui 1000x832, 30 Hz, 300x124 display, no rings/LFO/env/scope/drag tooltips)
In order of impact:
1. Repaint: vsync 60 fps via request_repaint() while a voice is active, a mod is nonzero or a gesture is live; request_repaint_after(about 250 ms) idle. Read audio state via atomics each frame.
2. Hero display: grow to about 580x300 at top-centre; source waveform with live read position, window and moving grains (scatter/size visibly alive); antialiased 2 px stroke, gradient fill, faint grid; drag on it sets position; click toggles a stacked-frames view if the engine has frames.
3. Shared knob widget with track arc, value arc, live modulated dot per source colour; drag-from-source mod assignment with target highlight and preview.
4. LFO/envelope editors with playheads (latest voice plus faded markers up to 8), scope/spectrum from a lock-free ring, output meter with ballistics, moving filter curve if GTS has a filter.
5. Interaction: drag value popup, Shift fine, double-click reset, typed entry, wheel, cursor changes, hover <= 100 ms, tooltips after about 500 ms hover.
6. Theme tokens: palette, 4/8 px spacing, 3 knob sizes, one font, 1.5 px strokes, 1-2 elevations, glow on active signal only.
7. Scale 100-200% via set_zoom_factor, persisted; all vector.
8. Verify: 10 s frame-time capture (no frame > 33 ms), idle repaint count 0, crisp lines at 100% and 200%.
Defer (NICE): menu/tab transitions beyond a 120 ms fade, voice indicators, skins.
