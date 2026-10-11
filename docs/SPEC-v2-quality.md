# GTS v2 — the quality bar

Supersedes the "GUI", "Output", "Verification" and default-patch parts of `SPEC.md`'s GTS section
where they conflict. Every item below cites the standard it follows (`SQ-n` =
`docs/standards/sound-quality.md` item n, `UX-n` = `docs/standards/presets-and-ux.md` item n) and
the audit finding it fixes (`A-n` = `docs/audits/gts-v1-audit.md` rank n). Every item has a
measurable acceptance test. Numbers tagged **[E]** in the standards docs are engineering targets,
not published standards, and are labelled as such here. Nothing ships without its test passing.

Rule for agents: if an item cannot meet its acceptance test, report it; do not loosen the test.

---

## Track 1 — Sound core

| # | Item | Standard / audit | Acceptance |
|---|---|---|---|
| 1.1 | **Mip-mapped tables.** One mip per octave, 2048-sample frames, highest kept harmonic ≤ 17.5 kHz at the running sample rate (regenerate on sample-rate change), chosen by the effective read rate `f0·2^(shift/12)`, 4-point Hermite interpolation, per-block crossfade when the mip changes. | SQ-1,2,4,5,6,7,8; A-1, A-2 | Alias floor ≤ −80 dB re fundamental **[E]** for Square Morph and Saw Stack at C1–C7, shift 0 and +12 (non-harmonic energy outside ±1 % of k·f0). Bandwidth: at C2 index 64, energy above 4.2 kHz within 6 dB of a PolyBLEP square. Generation ≤ 150 ms release. |
| 1.2 | **Oversampled shaper.** 2×/4× polyphase half-band around Sine/Saturate/Clip (Quant/Noise stay 1×), bypassed at amt 0. Global quality setting 1×/2×/4× (default 2×) persisted as host state, never in presets. | SQ-11,12,13,14; UX-60; A-6 | Saturate 1.0 on a C3 saw: alias energy ≤ −70 dB at 4×, ≤ −60 dB at 2× **[E]**. 1× unchanged. |
| 1.3 | **Shaper gain staging.** Per-mode output compensation normalised to RMS 0.25 input; Clip threshold remapped to the real signal range (0.5 → 0.05). | A-5 | Toggling the shaper at amt 0.5 changes RMS by ≤ 1 dB in every mode; Clip 0.5 produces ≥ 5 % THD on a 0.25-RMS saw. |
| 1.4 | **Unison per oscillator.** 1–7 voices, detune 0–50 cents with curve (linear/power), blend (centre vs sides), stereo width, random-phase amount with a phase-reset mode, gain compensation 1/√N. | SQ-16,17,18,19,20,21 | Width 1 / 7 voices: inter-channel correlation ≤ 0.3; N voices at blend 1 within ±1 dB of 1 voice; phase-reset mode gives identical first 10 ms on repeated notes. CPU: 1 voice × 7 unison ≤ 1.5 % of a core at 48 kHz release build **[E]**, recorded in the audit. |
| 1.5 | **Sub oscillator + mono crossover.** Dedicated sine sub, −1/−2 octave, level, phase reset on note-on, low-cut 25 Hz; mono-ise the whole output below an adjustable crossover (default 120 Hz). | SQ-22,23,24,25 | Below 120 Hz the L−R difference ≤ −40 dB with width 1 on both oscs; sub THD ≤ 0.1 %. |
| 1.6 | **Comb filters tied to the note.** Comb types default Kbd 1.0 (fixed-frequency still possible); Comb− output scaled by √((1+fb)/(1−fb)) with a soft clip instead of (1−fb). | A-3 | Default-class patch at F1: fundamental loss ≤ 3 dB; Comb− res 1.0 overall level within ±3 dB of res 0; no sample > 1.0. |
| 1.7 | **Voice end respects the filter tail.** A voice stays active until both filter outputs are below −90 dBFS for a block, or fades 5 ms. | A-4; SQ-31 | LP12 res 1.0 and Comb− res 1.0: last-sample level ≤ −80 dBFS; note-off step ≤ 1.5× steady-state step at rel 1 ms. Voice steal: 2–5 ms declick, measured step ≤ 1.5× steady. |
| 1.8 | **SVF resonance.** Q range starts at 0.707; tanh on the band state so res 1.0 sings instead of hitting the limiter. | A-8; SQ-32,33 | Res 1.0 sweep peak ≤ 0 dBFS before the limiter; a clean sine (THD ≤ 3 %) at cutoff at res 1.0 with silent input + 1e-4 excitation. |
| 1.9 | **Shift playback evenness.** Output-frame 2-period Hann at every ratio (window sum exactly 1) with a small f0-rate phase wobble near integer ratios ≥ 2; per-grain, per-semitone level table built at generate time. (Wave 1 replaced the window-sum/Tukey proposal, which measured 46–69 % AM.) | A-7 | Sine AM depth ≤ 10 % at +12 (measured 1.1 %); level within ±3 dB across shift −24..+24 (measured ±0.3 dB); non-harmonic floor ≤ −80 dB. **Accepted limit:** a pure sine at a non-integer shift such as +7 beats between two neighbouring harmonics (99.9 % AM) with any constant-rate grain read; harmonic-rich tables do not show it. Kept as an ignored test. |
| 1.10 | **Loudness trims.** Per-table K-weighted trim; init peak −12..−6 dBFS at velocity 100 on C2. | A-10; SQ-37 | Table spread ≤ 1.5 dB K-weighted at C2 index 64; init render inside the band. |
| 1.11 | **Envelopes.** Minimum attack/decay/release 1 ms enforced; curve control (−1 log … 0 exp … +1 lin) per envelope; doc comment for release length corrected. | SQ-27,28,29; A-11 | Attack 1 ms note-on step ≤ 2× steady; curve 0 reproduces today's shape. |
| 1.12 | **Init patch = clean bass.** Osc A Square Morph index 40, unison 1, sub on −6 dB, Filter A LP12 2 kHz with env, no comb, shaper off, Mod A one-shot off. The riddim signature sounds move to the factory bank. | UX-24,25,26,59 | First note on C2 at velocity 100: peak −12..−6 dBFS, spectral centroid 150–600 Hz, no sample > 0.95, no click per 1.7/1.11. |

Deferred, documented: analog drift (SQ-35, NICE), sinc interpolation (SQ-7 NICE), ADAA (SQ-15).

## Track 2 — Modulation matrix, macros, presets

| # | Item | Standard | Acceptance |
|---|---|---|---|
| 2.1 | **Modulation matrix.** 8 slots: source ∈ {Macro 1–4, Mod A, Mod B, Filter Env, Amp Env, Velocity, Mod Wheel, Aftertouch, Bus CV 1–3} → any continuous parameter, bipolar amount, per-slot curve (lin/exp). The three bus CV-in slots become sources of this matrix (their persisted source strings stay). Existing fixed Mod A/B destinations and wheel/vel params remain for compatibility. Applied per 32 samples with 5 ms smoothing, as normalised offsets, never writing host values. | UX-38,39,42; SQ-41 | Each source moves its target within one block + 5 ms; 8 slots on one target sum and clamp; no allocation in process; CPU ≤ +5 % at 8 voices. |
| 2.2 | **Macros.** 4 host-automatable params `macro1..4`, labels persisted per preset, assignments live in the matrix. | UX-33,34,35,36,37 | Host automation of macro1 moves its assigned targets; label round-trips through preset and host state. |
| 2.3 | **Preset format.** JSON: `format_version`, `product`, `name`, `author`, `description`, `category`, `tags[]`, `macros[{label}]`, `matrix[]`, `params{id: plain}`, `created`. Unknown params ignored, missing params keep defaults. Factory presets embedded in the binary and also written read-only to `Documents\Riddim Rack\GTS\Presets\Factory\<Category>\`; user presets in `...\Presets\User\…`. | UX-1,2,3,4,6,7,12 | Round-trip test: save → load restores every param bit-exactly; a preset with an extra unknown field and a missing field loads; folder auto-created on first run. |
| 2.4 | **Browser.** Header: preset name, modified dot, ◀ ▶ (next/prev within the current filter), Init, Save, Save As, browser toggle. Browser overlay: Factory/User/Favorites, category filter (Bass, Lead, Pluck, FX, Seq, Init), tag chips, text search over name/author/description/tags, list with author + description, heart to favourite, double-click loads and closes, Esc closes, Reveal folder, Rescan. Save dialog: name, author, description, category, tags; overwrite confirm. | UX-13,14,15,16,17,18,20,31 | Headless egui tests: search filters, favourite persists to `favorites.json`, prev/next wraps within filter, overwrite prompt appears, Init resets all 87+ params, modified dot appears after any edit and clears on load/save. |
| 2.5 | **Factory bank.** 40 presets: Bass 24 (sub 4, growl 6, reese 3, wobble 5, formant/talking 4, neuro 2), Lead 6, Pluck 4, FX 4, Seq 2. Each has a description that says what it is and what the four macros do; macros labelled by purpose (e.g. Growl, Dirt, Width, Motion); descriptive names, category in metadata, author "Riddim Rack". Designed by rule (layer sub + processed mid, movement on a macro, in-tune, mono below crossover) and QA-rendered. | UX-9,10,11,27,28,29; SQ-37,40,47,48,52 | `cargo run --example preset_qa` renders every preset at C2 (Bass) / C4 (others), velocity 100, 2 s: peak ≤ −3 dBFS, RMS −24..−14 dBFS, bank spread ±2 dB, no NaN, no clipping, alias floor from 1.1 holds; test fails on any violation. |
| 2.6 | **Host state.** Preset name, modified flag, macro labels, matrix and bus strings persist in the DAW project independent of files. | UX-30,32 | Serialize/deserialize round-trip test; loading a preset never changes the quality setting or window scale. |

## Track 3 — Seeing what is going on

| # | Item | Standard | Acceptance |
|---|---|---|---|
| 3.1 | **Modulation rings.** Every modulated knob draws an arc for the modulation range in the source's colour and a live dot for the current modulated value (30 Hz). Hovering a source badge highlights its targets. | UX-38,41 | Preview PNG with 3 modulated knobs reviewed; live value read from per-target atomics written in process. |
| 3.2 | **Click-to-assign.** Click a source badge (Macro 1–4, Mod A/B, envs, Bus CV) then a knob to create a matrix slot; drag the ring to set depth; right-click ring to remove. | UX-39,40 | Headless test: badge + knob creates slot with amount 0.5; ring drag changes amount; remove clears. |
| 3.3 | **Matrix view.** A tab listing all slots (source, target, amount, curve) with edit/remove. | UX-42 | Headless test: list equals matrix state. |
| 3.4 | **Shape displays with playheads.** Mod A/B curve with phase marker; amp and filter env curves with position marker from the newest voice. | UX-43 | Preview PNG reviewed; marker position atomics updated in process. |
| 3.5 | **Header display tabs.** Grain | Scope | Spectrum, plus a filter response curve inside each filter group that follows cutoff/res/type. | UX-44 | Preview PNG reviewed; scope buffer is a lock-free ring written in process. |
| 3.6 | **Help line + tooltips.** Status line at the bottom: hovered parameter name, value with unit, one-line description. Every parameter has help text in a table; a test fails if any id lacks one. | UX-45,46,62 | `tests/help.rs`: every param id present with non-empty text; hover test shows the text. |
| 3.7 | **Value entry.** Ctrl+click (or right-click → Edit) opens typed entry with unit parsing; Shift = fine; double-click = reset (exists). | UX-47,48,49 | Headless test: typing "880 Hz" sets filta_freq; "0.5" sets a 0..1 param. |
| 3.8 | **Undo / redo.** Plugin-side history of parameter gestures and preset loads, Ctrl+Z / Ctrl+Y in the GUI, 100 steps. | UX-50 | Test: three edits, two undos, one redo → expected values; preset load is one step. |
| 3.9 | **A/B compare.** Two state slots with copy and swap in the header. | UX-51 | Test: edit in A, swap to B restores, swap back restores A. |
| 3.10 | **Scalable window.** View menu: 75 / 100 / 125 / 150 %, persisted as host-side setting; default 100 % = 1000×760. | UX-54 | Resize test: all four scales render without clipping (layout test at each scale). |
| 3.11 | **Header indicators.** Voice count, CPU %, output meter with the master volume knob. | UX-57,58 | Values come from atomics written in process; preview reviewed. |
| 3.12 | **Quality + settings menu.** Oversampling 1×/2×/4×, window scale, reveal folders, about (version, licence). | UX-60; SQ-12 | Settings persist as host state and are not touched by presets. |

Deferred, documented: randomize (UX-52), MIDI learn (UX-53), audition clips (UX-19), themes (UX-55), on-screen keyboard (UX-56).

## VSS v2 (smaller, same system)
Preset format/browser/factory bank (20 patterns with descriptions), help line with text for all 31 params, rings on CV targets, scope tab, scalable window, undo/redo, settings menu. Acceptance tests mirror the GTS ones.

## Build plan and models
1. **Sound core** (Track 1): Opus, three parallel agents on disjoint files — (a) tables/mips + unison + sub + crossover, (b) shaper oversampling + gain staging + filters (comb, SVF), (c) voice end, envelopes, shift evenness, trims, init patch. Each ships its acceptance tests; the audit script is re-run and must improve on every ranked finding.
2. **Kit widgets** (parallel with 1): Sonnet — rings, badges, shape displays, scope/spectrum, filter curve, browser overlay, help line, settings menu, tabs, value entry, scale support, in `rr-ui` with preview PNGs.
3. **Matrix, macros, presets** (Track 2): Opus for the matrix engine and host state; Sonnet for the file format, folders, browser wiring; Opus "sound designer" for the factory bank with the QA renderer gating it.
4. **Visibility wiring** (Track 3) in the GTS editor: Sonnet, from the kit widgets, with the help-text table.
5. **Reviews:** Opus real-time skeptic, Sonnet build/docs, then Fable final with a "first five minutes as a Serum user" lens; the audit is re-run and attached to the release notes.
6. **VSS v2** after GTS v2 is accepted, reusing everything.

Estimated size: about four VSS-sized builds. Each phase ends with a rendered preview and the measured numbers, before the next starts.

---

## Track 4 — FX section inside GTS (added 2026-10-11)

Rationale: the reference comparison (`docs/audits/serum-init-comparison.md`) showed the raw
oscillator equal to the reference within 0.03 dB; what the reference presets have and ours do
not is a built-in effects chain. Source checklist: `docs/standards/fx-section.md` (FX-n).

Architecture (FX "Chain architecture" items): fixed default order **Distortion → Multiband Comp
→ EQ → Chorus → Dimension → Delay → Reverb → Limiter**, re-ordering deferred (NICE); every
module has `on` and `mix` (except EQ and Limiter); FX state lives in the preset; the chain runs
once on the summed stereo output after the voice sum and before the master volume; the whole
chain is bypassed sample-exactly when every module is off. Each module smooths its parameters
per sample (5–20 ms). No allocation in process; all delay/reverb buffers preallocated for 192 kHz.

| # | Module | Params (id prefix) | Standard | Acceptance |
|---|---|---|---|---|
| 4.1 | **Distortion** `dist_` | on, type {Tube, HardClip, Overdrive, Diode, FoldLin, FoldSine, BitCrush, Downsample}, drive −12..+36 dB (0), bias −1..1 (0), filter_mode {Off, Pre, Post}, filter_type {LP, BP, HP}, filter_freq 20 Hz..20 kHz, filter_q 0.3..8, out −24..+12 dB, autogain (bool, on), mix 0..1 | FX distortion items | HardClip at +12 dB on a 0.25-RMS C2 saw: THD 15–40 %; autogain keeps RMS within ±1 dB across drive 0..+24 for Tube/Overdrive; alias ≤ −70 dB at the internal 4× (reuse `oversample.rs`); Off/Pre/Post filter changes measured; CPU reported. |
| 4.2 | **Multiband comp (OTT-style)** `mb_` | on, depth 0..200 % (100), time 0..100 % (attack 1–100 ms, release 20–500 ms mapped), xover_lo 60..400 Hz (120), xover_hi 1..8 kHz (2.5 k), per band up/down threshold −60..0 dB (−24 up / −12 down defaults), band gain −12..+12 dB ×3, in −24..+24 dB, out −24..+24 dB, mix | FX multiband items | Gain reduction 9.0 ± 0.5 dB at 12 dB over a 4:1 downward threshold in single-band check; upward compression raises a −40 dBFS band by ≥ 6 dB at depth 100 %; Linkwitz-Riley bands sum flat (±0.2 dB) at depth 0; no pumping artefact > 1 dB at time 50 % on a steady tone. |
| 4.3 | **EQ** `eq_` | on, low shelf (freq 30..500 Hz, gain ±15 dB), peak (freq 20 Hz..20 kHz, q 0.2..10, gain ±15 dB), high shelf (freq 1..20 kHz, gain ±15 dB), low cut 20..500 Hz on/off | FX EQ items | Measured response within ±0.5 dB of the designed curve at 20 test frequencies; unity at all gains 0. |
| 4.4 | **Chorus** `cho_` | on, voices 2..8 (4), rate 0.1..10 Hz or sync, depth 0..1, delay 1..20 ms (7), feedback −0.95..0.95, lpf 200 Hz..20 kHz, wet_lowcut 20..500 Hz (120), mix | FX chorus items | L/R correlation 0.2–0.8 above 400 Hz at depth 0.5; correlation ≥ 0.95 below the wet low cut; no pitch error of the dry path. |
| 4.5 | **Dimension** `dim_` | on, size 0..1, wet_lowcut 20..500 Hz (120), mix | FX widener items | Width increases monotonically with size (correlation falls); mono-sum within 1 dB of dry; sub-band ≥ 0.95 correlated. |
| 4.6 | **Delay** `dly_` | on, sync (bool), time 1 ms..2 s / division 1/64..4/1 incl. dotted and triplet, feedback 0..0.95, mode {Mono, Stereo, PingPong}, hp 20..2000 Hz, lp 200 Hz..20 kHz, mix | FX delay items | Echo lands within 1 sample of the set time at 48 and 44.1 kHz; feedback 0.95 decays (never grows); ping-pong alternates channels. |
| 4.7 | **Reverb** `rev_` | on, type {Plate, Hall}, size 0..1, decay 0.1..20 s, damp 0..1, predelay 0..300 ms, lowcut 20..1000 Hz (200), hicut 1..20 kHz, width 0..1, mix | FX reverb items | RT60 at decay 2.0 s measured 2.0 s ± 15 %; low cut keeps energy below 100 Hz ≥ 20 dB down in the wet path; no modal ringing peaks > 6 dB in the steady spectrum of a noise burst tail; stereo width adjustable to mono. |
| 4.8 | **Limiter** `lim_` | ceiling fixed −0.3 dBFS, always on, gain −24..+12 dB | FX limiter item | No output sample above −0.3 dBFS for any preset at any velocity; THD ≤ 1 % at 3 dB over the ceiling; latency reported if lookahead is used. |
| 4.9 | **Editor FX page** | A second page or a tab in the header area: eight module strips in chain order with on/mix and the module params, using rr-ui widgets; help text for every new param (the 3.6 help table rule applies). | UX-62, DESIGN.md | Binding test passes with the ~70 new ids; preview PNG reviewed; window still fits 1000×832 or a page switch. |
| 4.10 | **Presets use FX** | The six candidates are re-voiced with the chain (OTT depth, dimension above the sub, distortion type per character, EQ cut, reverb ≤ 15 % on basses) and re-rendered; the full bank follows. | FX riddim items; SQ-37 | QA renderer passes the level band; compare.py distance to a reference the owner picks is reported per preset. |

Build: three Opus agents in worktrees on disjoint module files (4.1+4.2+4.8, 4.3+4.4+4.5, 4.6+4.7)
with per-module tests; then one Opus integration agent wires the chain, params, presets and
host state; then Sonnet builds the editor FX page; then the review ladder.
