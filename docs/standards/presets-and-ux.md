# Presets and UX: what professional soft synths do

Scope: checklist for Riddim Rack devices (GTS v1 = functional egui panel, 81 params, no presets, no mod visuals).

Verification status per item:
- **V** = confirmed in a page fetched during this research (source given, 2+ products where possible).
- **V1** = confirmed for one product only; second product not confirmable from fetched docs. Do not call it "industry standard" without a second check.
- **U** = widely true from general knowledge but NOT confirmed in fetched docs this session. Verify against a real install (open Serum 2 / Vital / Phase Plant and look) before building to it.

The fetch budget was limited and several manuals (Vital, Pigments, full Serum 2) were unreachable or incomplete. Gaps are listed at the end.

Sources:
- S1 Serum 2 manual, loading a preset: https://xferrecords.com/web-manual/serum-2/loading-a-serum-preset
- S2 Kilohearts preset browser (shared by Phase Plant, Snap Heap): https://kilohearts.com/docs/presets
- S3 Phase Plant manual: https://kilohearts.com/docs/phase_plant
- S4 Massive X manual, global controls: https://docs.native-instruments.com/online-guides/massive-x-manual/en/global-controls
- S5 Massive X manual, play view: https://docs.native-instruments.com/online-guides/massive-x-manual/en/play-view
- S6 Massive manual, browser and attributes: https://docs.native-instruments.com/online-guides/massive-manual/en/the-browser-and-attributes-views
- S7 Sound On Sound, Pigments review: https://www.soundonsound.com/reviews/arturia-pigments
- S8 Vital product page: https://vital.audio/
- S9 Vital forum, .vital is JSON: https://forum.vital.audio/t/free-presets-and-security/13282
- S10 Syntorial Vital guide (third party; default folder Documents\Vital\User\Presets): https://www.syntorial.com/es/wp-json/wp/v2/posts/6991
- S11 Arturia Pigments manual (third-party copy; Mod Source/Target views, 4 macros): https://mans.io/files/viewer/2161698/193
- S12 Minimal Audio Rift manual (Favorites + User packs, heart icon; secondary reference, not on the product list): https://manual.minimal.audio/rift-manual/presets/preset-browser

## 1. PRESETS

### 1a. Storage and format
1. **MUST** Ship a per-user preset folder separate from factory content, auto-created on first run, under Documents/<Vendor>/<Product>/... Vital: Documents\Vital\User\Presets (S10, third party). Kilohearts: User is the default save location, path not stated (S2). Factory is write-protected (S2); Massive X has "Show User Content Folder" (S4). Status V. GTS v1: no presets, no folder.
2. **MUST** A "Show/Reveal user folder" menu command (Massive X S4; Rift S12; Serum 1 "Show Serum Presets folder" per search results, U for Serum 2). Status V. GTS v1: absent.
3. **MUST** Plain, forward-compatible file format with a version field. Vital presets are JSON (S9, V1); Serum/Phase Plant/Massive X formats are not documented in fetched pages (U). Recommendation: JSON with `format_version`, diffable and tolerant of added params. Status V1. GTS v1: n/a.
4. **MUST** Metadata per preset: name, author, description, category/tags. Kilohearts: name, author, description editable in top bar, tags from `#word` in description (S2, S3). Massive: about 170 attributes plus author/meta info (S6). Pigments: per-preset designer description (S7). Status V. GTS v1: none.
5. **SHOULD** Preset may embed audio data (wavetables/samples) so it is self-contained. Vital stores `samples` and `wave_data` in the JSON (S9); Serum 2 has an "Embedding Content When Saving a Preset" page (S1, title only). Status V (fields/titles only). GTS v1: n/a unless user wavetables are added.
6. **SHOULD** Rescan/refresh command so hand-copied files appear (Massive X "Rescan Content", S4; Massive library auto-index, S6). Status V.
7. **SHOULD** Subfolders shown as browser hierarchy ("Create Subfolder" in Save dialog, S2; Serum < > steps through the current subfolder, S1). Status V.
8. **NICE** User can add extra library locations (Kilohearts "Add Location", S2). Status V1.

### 1b. Taxonomy and naming
9. **MUST** A fixed, small category set applied to every factory preset. Pigments and Massive use categories/Type attributes (S6, S7; exact lists not captured). The common set (U; screenshot Serum 2 and Vital browsers to confirm): Bass, Lead, Pad, Pluck, Keys, Synth, FX, Seq/Arp, Drums/Perc, Vocal, Ambient. Riddim Rack proposal: Bass, Lead, Pluck, FX, Drums, Seq, Init, with sub-tags (sub, growl, reese, neuro, wobble, formant). Status U for the exact list. GTS v1: none.
10. **SHOULD** Free-form tags in addition to category, searchable (`#tag`, S2; Pigments tags, S7; Massive attributes, S6). Status V.
11. **SHOULD** Factory naming "<CAT> <Descriptor> <NN>", e.g. "BA Growl 01", with author initials in the author field, not the name. Category-prefix naming is common in third-party banks (U); NI/Arturia/Xfer factory names are mostly purely descriptive (U). Treat as house style, not a standard. Status U.
12. **MUST** Names unique within a folder, no path-illegal characters, full name visible (tooltip if truncated). Status U.

### 1c. Browser UX
13. **MUST** Header shows the current preset name with previous/next arrows (Serum 2 "< >", S1; Kilohearts top-bar arrows and arrow keys, S2). Status V.
14. **MUST** Browser panel with list, folder/category filter and text search. Kilohearts search covers name, description, author, all words any order, quotes for exact (S2). Massive X filters by tags, text, user, favorites (S4 search snippet). Pigments: categories, tags, favourites, playlists, search (S7). Status V.
15. **MUST** Favorites (heart icon, auto folder, S2; Pigments, S7; Massive X Favorites filter; Rift S12). Status V.
16. **MUST** User vs Factory split in the browser (S2, S4, S12). Status V.
17. **MUST** Save and Save As: dialog with location (not factory/favorites), name, author (S2). Overwrite asks for confirmation (U, not confirmed in fetched docs). Status V for dialog, U for overwrite prompt.
18. **SHOULD** Double-click loads and closes browser; Escape closes (S2). Status V1.
19. **SHOULD** Audition/preview button per preset (Serum 2 play button with embedded MIDI clip or three fallback clips, S1). Status V1. Fits our sequencer devices: store an audition clip per preset.
20. **SHOULD** Right-click "set as default" so new instances load a chosen preset (S2). Status V1.
21. **SHOULD** Click an author or tag to search for it (S2). Status V1.
22. **NICE** Star rating (U, no fetched evidence). Skip; favorites cover it.
23. **NICE** Playlists/collections (Pigments, S7). Status V1.

### 1d. Init patch
24. **MUST** An "Init"/"New" action that resets every parameter to its default. Serum shows "- Init -" as the default preset name (S1); Kilohearts "New" clears the current patch (S2). Status V.
25. **MUST** Init is musically usable: a clean, in-tune, audible note on first key press at sane volume (no silence, no clipping). Expected across the field (U for citation). GTS v1: no Init concept; needs an audible-at-default audit.
26. **SHOULD** Init equals the param default set, so double-click-to-reset and Init agree. Massive X: double-click resets a knob (S5). Status V1.

### 1e. Count and quality
27. **MUST** Ship a real factory bank. Counts from fetched sources: Pigments 600+ (S7); Vital tiers 75 / 250 / 400+ (S8). Serum 2, Phase Plant, Massive X counts not confirmed (U). For a single-genre device, 30 to 60 hand-tuned presets across categories is a credible floor; under ~20 reads as a demo. Status V (Pigments, Vital).
28. **MUST** Every preset has a description saying what it is and what each macro does (Pigments, S7; Kilohearts description field, S2). Status V.
29. **SHOULD** Factory presets level-matched (within ~3 dB) and correct at default velocity so switching is not jarring. QA expectation (U).

### 1f. Host state vs plugin presets
30. **MUST** DAW-project state restores exactly (all params, macros, active preset name, modified flag), independent of preset files on disk. Standard VST3/CLAP behaviour (U for citation; nih-plug persist fields). GTS v1: params persist via host; no preset name.
31. **MUST** A "modified" indicator on the preset name after any edit (U; usually an asterisk or dot).
32. **SHOULD** Loading a preset must not change host-owned settings (window size, global quality, audition level) (U).

### 1g. Macros
33. **MUST** A small fixed set of macro knobs. Phase Plant: 8, routable to any parameter, one-to-many (S3). Massive X: first 8 of 16 shown, per-preset (S5). Pigments: 4 (S7, S11). Status V.
34. **MUST** Macros are saved per preset with a label and their assignments (Massive X: label, knob, per-preset, S5; Pigments descriptions explain macros, S7). Status V.
35. **MUST** Macros are host-automatable parameters (Massive X: macros bridge to host automation and MIDI, S4). Status V1.
36. **SHOULD** Macros usable as modulation sources like any other (Pigments, S7). Status V1.
37. **SHOULD** For our devices: 4 to 8 macros labelled by purpose ("Growl", "Dirt", "Width", "Motion"). Derived from 33.

## 2. UNDERSTANDING WHAT IS GOING ON

### 2a. Modulation visualization
38. **MUST** On every modulated control, draw a ring/arc showing modulation range in the source's colour. Pigments: grab a control's outer edge to add a destination, coloured ring shows amount, colour distinguishes sources (S7). Vital: animated controls (S8). Serum 2/Phase Plant presumed the same (U). Status V1 strong. GTS v1: none.
39. **MUST** Depth editable at the destination, and a source can list all its destinations (Pigments, S7, S11). Status V1.
40. **SHOULD** Drag-and-drop or click-to-assign a source onto a control (Pigments, S7, S11; Vital previews a modulation before commit, S8). Status V.
41. **SHOULD** Live movement: the ring/indicator animates with the real modulated value (Vital 60 fps animated controls, S8; Pigments animated scope per used source, S7). Status V.
42. **SHOULD** A mod-matrix list view for auditing (Pigments Mod Target view, S11). Status V1.
43. **MUST** LFO and envelope editors show the shape with a moving phase marker (Pigments animated source scopes, S7; Vital LFOs, S8). Per-voice playhead specifically is U. Status V partial.

### 2b. Output visualization
44. **SHOULD** Oscilloscope of output plus spectrum and filter-response displays (Vital states oscilloscopes, spectrograms and filter response, S8; Pigments scopes, S7). Status V. For Riddim devices a filter curve plus a small scope are highest value.

### 2c. Help and feedback
45. **MUST** Hover shows parameter name and current value with units (Massive X macro hover value, S5). Fuller help text per parameter: Pigments sound-design tips flag controls (S7). Status V1.
46. **SHOULD** A fixed status/help line describing the hovered control (U; seen in Vital, not confirmed in fetched docs).
47. **MUST** Typed value entry via double-click or text edit (U).
48. **MUST** Double-click resets a control to default (Massive X, S5). Status V1.
49. **MUST** Fine-adjust modifier (Shift/Ctrl-drag, U) and selectable control sensitivity (Massive X offers nine settings, S4). Status V1.
50. **MUST** Undo/redo for all edits (Phase Plant top-bar Undo and Redo, S3). Second product U. Status V1. GTS v1: none.
51. **SHOULD** A/B compare: two state slots plus swap (U).
52. **SHOULD** Randomize with scoped ranges; never randomize master volume or quality (U).
53. **SHOULD** MIDI learn on any control (U; Massive X only documents macro-to-MIDI, S4).

### 2d. Window and indicators
54. **MUST** Resizable/scalable UI. Massive X offers eight view sizes saved as default (S4). Others U. Typical default window is roughly 900 to 1200 px wide, 500 to 800 px high (U). GTS v1: fixed egui panel.
55. **SHOULD** Theme choice (Massive X six themes, S4; Vital skins, S8). Status V. Low priority.
56. **SHOULD** On-screen keyboard showing played keys and range (U).
57. **SHOULD** Voice-count and CPU indicator (Phase Plant docs tie high polyphony to CPU, S3; meters themselves U).
58. **SHOULD** Master volume with level meter in header (Massive X, S4). Status V1.

## 3. WORKFLOW AND SOUND QUALITY

59. **MUST** Default state is musically useful: playable low note, clear tone, no filter dialling needed first. Massive X and Pigments lean on macros and descriptions so presets are playable without menu diving (S5, S7); the general expectation is U.
60. **MUST** Global quality/oversampling control (Vital emphasises low noise floor and sharp Nyquist cutoff to limit aliasing, S8; explicit oversampling menus in Serum/Phase Plant are U). Offer 1x / 2x / 4x stored as a host setting, not per preset. Status V1 + U.
61. **MUST** Anti-aliased oscillators and distortion/fold stages. Heavy FM, wavefolding and distortion alias badly; this is the likeliest cause of "doesn't sound great" (S8 treats it as a headline feature; the rest U).
62. **MUST** Tooltip/help text for every control (S5, S7 partial). Enforce at build time: a param without help text fails CI.
63. **SHOULD** Output limiter or safety stage so no preset can blast on load (U). Pair with level-matched bank (29).
64. **SHOULD** Smoothed parameter changes (no zipper noise), including macro sweeps (U).

## Verification gaps (do these on a real install)
- Exact Serum 2 / Phase Plant / Massive X / Vital file formats and Windows folder paths (only Vital JSON and its default folder cited, the latter third party).
- Exact category lists in the Serum 2, Vital and Pigments browsers (screenshot and replace item 9).
- Whether Serum 2 and Phase Plant draw modulation rings (Pigments confirmed).
- Presence of A/B, randomize, MIDI learn, CPU meter per product.
- Typical window sizes.
- manual.vital.audio failed DNS; Pigments official manual not reached.

## GTS v1 scorecard
Lacks: all of section 1 (presets, folder, Init, browser, save), macros (33-37), mod visuals (38-43), scope/spectrum (44), undo/redo (50), A/B, randomize, MIDI learn, resizing (54), oversampling control (60), full tooltip coverage (62).
Has: 81 host-automatable parameters. Hover values and double-click reset are unconfirmed.
