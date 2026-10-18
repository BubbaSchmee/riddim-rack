# GTS visual and motion addendum: Track 7 (modulator, warp, sub and noise, FX chain)

Status: built (SPEC v2 Track 7, UI wave); section 12 lists what the build adds or changes against this proposal. Extends `MOTION.md` and `DESIGN.md` for `docs/SPEC-v2-quality.md`
Track 7. Everything not named here is unchanged: window 1000 x 832, tokens, durations, easing,
knob sizes, group padding, painter-only drawing, glow rule, line rule, help bar, reduced motion.
The built panel is shown by `preview-gts-editor.png` (SYNTH, LFO A), `-mod-steps.png` (LFO C), `-mod-env.png`, `-lib.png`, `-noise.png`, `-fx.png` and `-chain.png` (a chain drag), rendered by `editor_preview` with the engine playing.
Running reference: `mock-track7.html` (self-contained; `?s=lfoa|lfoc|env|lib|fx|drag`, add `&still=1`
to freeze). Stills: `mock-track7-1.png` SYNTH with LFO A (riddim gate, playheads), `-2.png` MOD ENV
tab plus the NOISE tab, `-3.png` FX page with the chain bar, `-4.png` LFO C in step mode, `-5.png`
the stock-shape library open, `-6.png` a chain drag in progress. All coordinates are window px
unless marked "local" (relative to the group's top-left).

## 1. Summary of layout decisions

1. The MODULATOR group takes the whole Mod A + Mod B slot: **620, 80, 336 x 260** (the two old
   groups were 336 x 126 with an 8 px gap, so the height is the same 260).
2. Osc A/B stay 404 x 222. Row 1 goes from 7 to 8 cells (Pitch added), row 3 gains Warp + Amt in
   the room the RESET button used, and RESET moves to the title row.
3. SUB becomes SUB | NOISE tabs in the same 88 x 222 group.
4. FX page: a 20 px chain bar at the top, Phaser and Flanger join the second row, Dimension / Delay
   / Reverb share the third, and the Limiter becomes a 56 px strip at the bottom (see 8 for the
   cost: the Limiter's large output spectrum shrinks).
5. MATRIX page: rows only (section 7).

## 2. MODULATOR module (620, 80, 336 x 260)

Source colours (tab dot, curve, handles, playheads, chip, modulation rings). A and B are the
approved amber and blue; C, D and Mod Env are new and follow the MOTION.md rule that colour means
"this value is moved by that source" (mint stays reserved for instrument state):

| Source | Token | Hex | Glyph | On well / display / inset (WCAG) | Nearest existing family (CIE76 Delta E) |
|---|---|---|---|---|---|
| Mod A | `src_a` | #ffb454 | A | 10.3 / 11.5 / n.a. | approved |
| Mod B | `src_b` | #7aa7ff | B | 7.6 / 8.5 / n.a. | approved |
| Mod C | `src_c` | #ff8cd9 pink | C | 8.6 / 9.7 / 8.3 | Macros violet 29.7 |
| Mod D | `src_d` | #ffd900 yellow | D | 13.1 / 14.7 / 12.6 | Mod A amber 37.2 |
| Mod Env | `src_n` | #ff7f50 coral | M | 7.2 / 8.1 / 7.0 | Mod A amber 31.3 |

Pairwise among the three new colours: C-D 122, C-Env 68, D-Env 66. Distance to the accent mint: at
least 83 (rule: 40). Every colour is at least 7.0:1 on the well, the display and the inset (rule
for the families added in MOTION.md section 3 is 10:1 on the well for the new note, ctrl and bus
families; these three trade that for hue room because the palette is already crowded, and sit at the
approved blue's level, 7.0 to 13.1; the `rr-ui::theme` gates are 3:1 and Delta E 25 / 40 and all pass). The
glyph letter on the chip disc is dark `#0b0f11` as for the other families; Mod Env uses M (macros use
digits). Colour is never the only channel: the tab carries the name, the chip carries the glyph.
Contrast of the tab label (white or muted) is unchanged from the segmented control (15.1 / 6.7).

### 2.1 Frame (local coordinates)

| Part | Rect (local x, y, w, h) | Notes |
|---|---|---|
| Tab strip | 12, 8, 312 x 26 | segmented control, 2 px padding and gap: LFO A, LFO B, LFO C, LFO D (56 wide each), MOD ENV (76). Each tab: 9 px source dot, 11 px caps +0.08 em. Lit segment fill is the source colour at 18 %, 1 px stroke at 60 % (the mint segment stays for the page and mode switches). The dot's alpha follows the source's live value (0.35 to 1) so a quiet LFO is visible at a glance. The dot is a drag handle: dragging a tab onto a knob patches that source at +50 % exactly as a chip does (MOTION 5.6) |
| Toolbar | 12, 40, 312 x 24 | see 2.2 |
| Shape editor glass | 12, 68, 312 x 130 | radius 6, 1 px inner graticule, absolute 632, 148 to 944, 278 |
| Controls row | 12, 204, 312 x 48 | S knobs (32) in 40 to 44 px cells, combos 24 high under a 14 px label |
| Bottom padding | 8 | group ends at 260 |

### 2.2 Toolbar (24 px high)

LFO tabs, curve mode: LIB combo 126 (shows the loaded stock name, or "Custom"), STEP toggle 52,
SNAP toggle 52, GRID combo 64 (mono, "1/8T"), 6 px gaps. Step mode: LIB 76, STEP (lit) 52, SNAP 52,
STEPS combo 46 ("2" to "32"), GLIDE slider filling the rest (about 70, 0 to 1, value in the source
colour). MOD ENV tab: a 20 px source chip (drag to patch) and the hint "Drag onto a knob. Pitch All
gives a dive."; the library does not apply to the envelope.

The library is a popover (section 2.6), STEP switches the editor between Points and Steps with the
data kept per LFO, SNAP toggles grid snapping of x and (in step mode) quantises levels to 1/16.

### 2.3 Shape editor, points mode

Glass-local plot rect: x 28 to 304, y 26 to 114 (absolute 660, 174 to 936, 262), value +1 at the
top, -1 at the bottom, 0 at y 70. The glass is 130 high so the strips around the plot are used:

* Ruler, y 3 to 21: one tick per grid slot (3 px, 25 % white), one per beat (6 px, 50 %), where
  beats are quarters of the cycle. Right-aligned mono caption "GRID 1/8T · 12" (the grid and its slot
  count). The **phase offset marker** is a hollow 8 x 8 px down triangle in the source colour (25 %
  fill) on the ruler at x = 28 + phase * 276, a 1 px dotted line at 40 % down to the plot bottom,
  and the value ("60°", mono, source colour) to its right (flips to the left within 40 px of the
  right edge). Dragging the marker edits Phase (same parameter as the knob).
* Y labels "+1", "0", "-1" mono 11 px right-aligned at x 23.
* Grid: 1 px vertical lines every slot at 6 % white, every beat at 12 %; horizontal at +/-1 and
  +/-0.5 at 6 %, the 0 baseline at 12 %. Lines sit on pixel boundaries (MOTION 6.6).
* Curve: 1.5 px in the source colour, three-pass glow (6x / 3x / 1x at 7 / 17 / 100 %), gradient
  fill from the curve to the 0 baseline at 22 % to 0. Vertical steps (two points at the same x) are
  allowed and read as hard gate edges, which riddim gates need.
* **Fade-in ramp**, y 117 to 127 under the plot: a right triangle in the source colour (30 % fill,
  1 px stroke) from the phase marker's x, as wide as the fade expressed in cycle fractions
  (fade seconds / cycle seconds * 276, capped at the plot edge), 8 px high, labelled "FADE 0.35 s"
  (mono 11 px, muted) to its right, and a 6 % source-colour band over the plot behind the same
  span. Absent when fade = 0.
* **Points**: filled circle r 3.5 in the source colour, 1 px `#0b0f11` outline, hit radius 8. The
  selected point is r 5 with a 1.5 px white ring at r 7. At most 32.
* **Segment curve handle**: a 10 px hollow diamond (1.5 px white stroke, `#0b0f11` fill) at the
  midpoint of the hovered segment, plus two 6 px arrowheads above and below, and the segment
  drawn 2 px at 55 % white. Only the hovered segment shows it (the plot stays quiet); a segment with a
  non-zero curve keeps a 4 px dot at its midpoint at 50 %.
* **Playheads**, one per sounding voice, as MOTION 5.3: 1 px white line at 50 % across the plot, 2.4
  px white dot on the curve at the voice's phase, 18 px source-colour glow sprite at 30 %.
  Released voices fade over 250 ms. In Free and Envelope modes the same rule applies; in Retrigger
  they restart at the phase marker (not at 0) on each note-on.
* Value pill while dragging (MOTION 5.7): x and y of a point ("0.50  +1.00"), or the segment curve
  ("CURVE +0.35"), above the pointer.

### 2.4 Shape editor, step mode

Same glass and grid. N bars (2 to 32), each (276 / N) px wide minus a 3 px gutter, from the 0
baseline to the level, rx 1, source colour at 28 % (the hovered or playing bar at 55 %). The output
curve (1.5 px, three-pass glow) is drawn over the bars: each step ramps from the previous level to
its own over glide * step width (smoothstep) and holds. The playing step gets a 7 % white column.
The ruler caption reads "STEPS 16 · GLIDE 0.40". A hovered bar prints its value ("-0.9", mono,
source colour) under the plot. Painting: press on a bar and drag vertically to set it, sweep
horizontally to paint adjacent bars; the value pill shows the level.

### 2.5 MOD ENV tab

Unipolar plot (same rect), y labels "1", ".5", "0", grid at 0, .25, .5, .75, 1. Segment widths use
sqrt(time + 0.02) (MOTION 5.3), the notional sustain segment is 0.5 of the A + D + R widths.
Handles: **A** peak (drags x only, y fixed at 1), **D/S** corner (x = decay, y = sustain), **S end**
(y follows sustain; x is the note-off marker, fixed), **R** end (drags x). **Curve diamonds** at the
midpoints of attack, decay and release (4.5 px, hollow, the hovered one solid). A dashed vertical
"note off" tick at the sustain end with a mono caption. Time and level readouts in mono under the
segments ("12 ms", "260 ms", "55 %", "380 ms"), caption "ADSR · CURVE" at the top right. Playheads:
the voice in sustain creeps through the notional segment (0.15 of its width per second, wraps at the
end), the releasing voice rides the release, one dot per voice, as 2.3. Controls row: Att, Dec,
Sus, Rel, Curve as S knobs in five 62.4 px cells (12 + i * 62.4, 204), values in time units.

### 2.6 Controls row and library

LFO tabs: **SYNC + DIV** 76 wide (24 px SYNC LED toggle, 48 px DIV combo, mono, label "Sync · Div";
unsynced, the combo becomes a Rate knob and the label "Rate", value in Hz). The DIV menu groups
Straight (1/1 to 1/32), Dotted (1/2D to 1/16D) and Triplet (1/2T to 1/32T); the lit row is in
accent. **TRIGGER** combo 66 (Retrig, Free, Envelope). **1-SHOT** 42, a LED toggle that is lit when
the trigger mode is Envelope (the same switch as the old one-shot; the old `1-shot` parameter and
the Envelope mode are one state, so a saved v1 preset round-trips). **PHASE** 40, **FADE** 40,
**SMOOTH** 44 as S knobs. Cell x (local): 12, 90, 158, 200, 240, 280.

Library popover: anchored under the LIB combo, 336 x 190 at (620, 148), a 26 px header
("STOCK SHAPES", right: "Loads into LFO A as Custom"), two columns of six 28 px rows, each a
13 px name and a 34 x 16 thumbnail (1.3 px, amber). Twelve stock shapes, the loaded one in accent
with a dot: Riddim Gate 1/8T, Stepped Tri, Ramp and Hold, Bounce, Wub Saw, Triplet Chop, Call and
Response, Half-Time Gate, Skip Gate 1/16, Pluck Fall, Dotted Pump 1/8D, Growl Slew. Loading copies
the shape's points into the LFO and sets curve = Custom; the grid and the recommended division in
the name apply only to the grid choice, not the rate. Esc or a click elsewhere closes.

## 3. Interactions

| Gesture | Result |
|---|---|
| Double-click empty space in the plot | adds a point at that x (snapped when SNAP is on, y snapped to the 1/8 value lines); refused with a help-bar line at 32 points |
| Double-click a point | deletes it (the first and last points are pinned in x and cannot be deleted; they only move vertically) |
| Right-click a point | deletes it (same rule); right-click a segment opens a two-item menu: Straighten, Hold (a step) |
| Drag a point | moves x and y; x clamps between its neighbours (equal x allowed for a vertical step); the value pill follows |
| Drag the segment midpoint vertically | sets that segment's curve (-1 to +1, 200 px = full range); double-click on it = 0 |
| Shift | 1/10 sensitivity on any drag (applied per event, so pressing or releasing mid-drag never jumps) |
| SNAP toggle | grid snap for x; Alt while dragging temporarily inverts it |
| GRID combo | 1/2, 1/4, 1/8, 1/16, 1/32 and triplet 1/4T, 1/8T, 1/16T (fractions of the cycle: at DIV 1/1 they equal note values at the host tempo) |
| Drag on the phase marker | edits Phase; Shift = fine; double-click = 0 |
| Step bars | press and drag vertically to set, sweep to paint, Shift = fine, double-click = 0 |
| Tab click | switches LFO / Mod Env; the lit segment slides |
| Tab drag | patches that source (like a chip); the MOD ENV chip in the toolbar does the same |
| Ctrl/Cmd+Z, Y | undo / redo of the shape edits since the tab opened (one history entry per drag) |

Host automation contract: a point or step edit is one gesture per drag; the shape lives in persisted
state, not in a parameter, so it is written once on release.

## 4. Osc A / B: Warp and Pitch (local to each 404 x 222 group)

| Part | Rect |
|---|---|
| Title row | title, rule, **RESET** toggle 60 x 24 (moved here), Table combo 112, On 24, at y 8 |
| Row 1, y 34 | eight cells of 47.5 px from x 12: Oct, Semi, Cent, **Pitch**, Index (L 44), Motion, Shift, Vol; M knobs are 40 |
| Row 2, y 96 | Att, Dec, Sus, Rel, Curve in five 47.5 px cells; amp display 130 x 50 at x 262, y 102 |
| Row 3, y 158 | Unison, Detune, D.Crv, Blend, Width, Rand in six 44 px cells from x 12; **Warp** combo cell x 280, 58 wide (label "WARP", combo 24 high: Off, FM, Sync, Bend+, Bend-, PWM); **Amt** knob cell x 342, 50 wide |

Pitch is the continuous +/-48 st target. It is a knob in the tuning cluster, labelled "PITCH", value
"+0.0 st" (1 decimal), centre detent at 0. Oct, Semi and Cent stay the tuning; the help bar reads
"PITCH +/-48 st, modulation target. Mod Env on Pitch All gives a dive." It shows modulation rings
like any knob (the mock shows a Mod Env ring on Osc A Pitch). Warp Amt is dimmed to 45 % while Warp
is Off (the knob stays editable). Hovering the Warp combo shows the one-line description of the
selected mode in the help bar. The pitch_all control has no dedicated knob (it is a matrix target
only); its ring appears on Osc A Pitch, Osc B Pitch and the Sub Oct knob.

## 5. SUB | NOISE (88 x 222 at 868, 348)

Tab strip local (4, 8) 80 x 24, no engraved title, no gap: SUB 34, NOISE 44; the lit segment is the
mint page-style segment (these are instrument modes, not sources). The old On LED moves into the
Sub body (below) and the NOISE tab has no On: level at minus infinity is off.

| Tab | Control | Local rect |
|---|---|---|
| SUB | Shape segment SIN / TRI | 8, 40, 72 x 22 |
| SUB | Oct (stepped 0, -1, -2), Phase | S knobs in 36 x 48 cells at (8, 68) and (44, 68) |
| SUB | Drive, Level | (8, 116) and (44, 116) |
| SUB | Xover, On | (8, 164); On is a bare 24 px LED toggle under a "ON" label at (44, 164) |
| NOISE | Type combo (White, Pink, Brown) | 8, 38, 72 x 40 incl. label |
| NOISE | Level, Tone | S knobs at (8, 82) and (44, 82) |
| NOISE | slope display | 8, 142, 72 x 50: a line at the type's tilt (0, -3, -6 dB/oct) and its caption in mono |

Level reads "OFF" at the floor. The noise display tilts over 180 ms when the type changes.

## 6. FX page

Window and header unchanged. Order numbers are chain positions; the module grid itself does not move
when the order changes (muscle memory), only the numbers and the chain bar do.

| Block | Rect (x, y, w, h) |
|---|---|
| Chain bar | 44, 80, 912 x 20 |
| Distortion | 44, 108, 272 x 262 |
| Multiband | 324, 108, 384 x 262 |
| EQ | 716, 108, 240 x 262 |
| Chorus | 44, 378, 344 x 164 |
| **Phaser** | 396, 378, 276 x 164 |
| **Flanger** | 680, 378, 276 x 164 |
| Dimension | 44, 550, 204 x 164 |
| Delay | 256, 550, 352 x 164 |
| Reverb | 616, 550, 340 x 164 |
| Limiter strip | 44, 722, 912 x 56 |

Gaps 8. The groups end at 778. Phaser (local): Rate, Depth, Fdbk, Centre, Mix as five 50 px cells at
y 34 (M 40); Stages combo (4 to 12), Sync toggle, Div combo and a 54 x 52 notch display at y 94.
Flanger: Rate, Depth, Delay, Fdbk, Mix the same way; Sync, Div, a "Fdbk -" polarity toggle and a 56 x
50 sweep display. Chorus loses nothing (two rows; its voice-phase dots move to 52 x 48). Reverb's
decay curve shrinks to 100 x 50 in the empty cell; Delay and Dimension keep their displays.

**Module title row**: a 6-dot **grip** (2.5 px dots, two columns, muted, accent on drag) at the
left, then a 15 px **order badge** (mono 11 px, the chain position), the title, rule and the On
toggle. The Limiter strip has a lock glyph where the grip would be and no badge.

**Chain bar** (44, 80, 912 x 20): caption "CHAIN" (11 px caps, `title` colour) at 0, then chips
from x 64: 20 px high, radius 4, `inset` fill, 1 px 6 % white stroke, grip glyph, 11 px caps name
(Dist, Multi, EQ, Phaser, Chorus, Flanger, Dim, Delay, Reverb), a 6 px state LED (mint on, off
chips show a dim name and a dark LED), 14 px chevron separators, then the **pinned Limiter chip**
with a lock, darker fill, no grip, no hover. A chip click toggles nothing (the title row has the On
switch); a chip hover lights its module (3 px ring, others drop to 50 %, as a source chip does).

**What dragging looks like** (`mock-track7-6.png`): pressing a grip or a chip starts a drag after 4
px. The dragged item lifts: a ghost chip (mint 1.5 px ring, shadow, 5 px up, a 1.2 degree tilt) follows
the pointer, the chain bar opens a 78 px dashed mint drop gap at the nearest slot (the neighbours
slide to make room over 180 ms ease-out, the vacated slot closes at the same time), the module being
dragged gets a 1.5 px mint outline and its badge turns mint with the new number, the other modules'
badges renumber as a preview, and the help bar reads "Chain  Flanger 6 > 5". The Limiter slot refuses
a drop after it (the gap snaps to before it). Release: the ghost settles into the gap in 120 ms
ease-out, the gap closes to a chip, and the audio crossfades the two orders over 10 ms; Esc or a
release away from the bar cancels with the chips sliding back in 90 ms ease-in. Dragging a module's
grip does exactly the same (the module itself stays in the grid).

## 7. MATRIX page

Nothing changes in layout (912 x 226 built-in routes, 912 x 462 slots). The source combo, the source
glyph and the swatch gain Mod C (pink, C), Mod D (yellow, D) and Mod Env (coral, M); the target
picker gains `Osc A Pitch`, `Osc B Pitch` and `Pitch All` under their sections (Pitch All under
Voice). Mod C and Mod D get no built-in routes: they are slot-only sources. The old Mod A and Mod B
built-in rows (Pitch, Index, Shift; Motion, Vol, Cutoff, Mod A) stay in the "Built-in routes"
table, where they already live; see open question 1.

## 8. Cost and what moved

* Sources strip row 1: SOURCES + Mod A, B, C, D, Mod Env (chips 88 to 102 wide, 18 px activity bar,
  6 px gaps, 548 of 568 px). Filt Env, Amp A and Amp B move into the MORE grid (3 x 4, twelve
  sources), whose three dots still show hidden families with routings. Row 2 (macros + MORE) is
  unchanged.
* The old Mod A / Mod B fixed-route knobs leave the panel (they remain editable in the MATRIX
  built-in routes, and every source can still be dragged onto any knob).
* The Limiter strip drops the big output spectrum to a 260 x 44 display and a 320 px gain-reduction
  bar; the header SPEC tab remains the full spectrum.
* Estimated vertex counts (frame_bench): SYNTH 66k to about 72k (editor with 32 handles and three-pass
  glow about 3k, four new knobs about 1.6k, sub and noise knobs about 1.2k); FX 62k to about 67k (two
  strips with displays about 5k). The 98.6k cap holds on both.

## 9. Motion (durations are MOTION.md tokens)

Group A (live, no easing, time-based, 60 Hz target): playheads and their dots per voice, the step
column highlight, the tab dots following each source's live value, the ENV playheads, the live
arcs on rings of the new pitch targets. Group B (chrome):

| Element | Trigger | Duration | Easing |
|---|---|---|---|
| Tab lit segment | tab click | 180 ms slide (`t_state`), colour cross-fades in the same 180 ms | ease-out |
| Editor content | tab switch or STEP toggle | 90 ms out (fade), 180 ms in; the plot never re-lays out (rect is fixed) | ease-in / ease-out |
| Library popover | open / close | 130 ms (fade + 4 px drop) / 90 ms | ease-out / ease-in |
| Library load | pick a shape | points morph to the new positions over 180 ms (point count differences fade in or out); the playheads keep their phase | ease-out |
| Point, curve diamond hover | pointer in | 100 ms (`t_micro`): handle grows r 3.5 to 4.5 | ease-out |
| Point added | double-click | 100 ms scale from 0 | ease-out |
| Point removed | delete | 90 ms fade | ease-in |
| Value pill | first movement, release | same frame in; 400 ms hold, 120 ms fade out (MOTION 5.7) | ease-out |
| SNAP / STEP toggle LED | click | 180 ms | ease-out |
| Sub / Noise tab | click | segment slides 180 ms; body 90 ms out, 180 ms in | as above |
| Noise slope line | type change | 180 ms tilt | ease-out |
| Chain chip slide, drop gap | during a drag | 180 ms | ease-out |
| Chain drag settle / cancel | release | 120 ms / 90 ms | ease-out / ease-in |
| Order badge renumber | drag preview | 100 ms cross-fade | linear |
| Page SYNTH / FX | click | unchanged (60 ms out, 120 + 160 ms in, 10 ms row stagger, 260 ms total); the chain bar enters first as row 0 |
| Fade ramp | fade knob | follows the value live (data display, no easing) | none |

Reduced motion (MOTION 4): Group B durations become 0, the library load snaps, the drop gap opens
instantly, the ghost does not tilt; Group A keeps running at 15 Hz snapshots.

## 10. Acceptance

1. All MOTION.md section 11 gates hold on SYNTH, FX and MATRIX with eight voices sounding, all
   four LFOs custom and the Mod Env patched: >= 60 fps mean, p99 interval <= 32 ms, <= 5 % of frames
   above 20 ms, none above 50 ms (`plugin_loop -- gts gl`); UI + tessellation <= 4.0 ms mean and 8.0
   ms p99; vertices <= 98.6k; GPU <= 1.0 ms.
2. The shape editor repaints in at most 0.3 ms CPU and 3.5k vertices with 32 points and 8 playheads;
   dragging a point repaints in the same frame as the pointer event (MOTION 11 row 4); playheads are
   time-based (row 7).
3. A drag of a chain item shows the ghost, gap and renumbering within 1 frame and finishes its
   motion within 180 ms; the order crossfade is click-free (step <= 1.5x steady, SPEC Track 7 (9)).
4. Line rule: every grid line and ruler tick resolves to a single full-coverage pixel row or column
   at ppp 1.0, 1.25, 1.5 (extend `motion_gl -- aa` with an editor crop in points mode, step mode and
   Mod Env mode); curves are at least 1.5 px.
5. Contrast: the three new source colours are at least 7.0:1 on well, display and inset and Delta E
   at least 25 from every existing family and at least 40 from the accent (measured values in
   section 2); `rr-ui::theme` tests list all five modulator colours; no text below 11 px; all hit
   targets at least 16 px (point hit radius 8, diamond hit 10).
6. Layout test: the modulator group is 336 x 260 at (620, 80), the editor glass 312 x 130, the Osc
   row cells at least 44 px wide, the FX groups inside 44..956 and 80..778 with 8 px gaps.
7. Keyboard: every control is focusable; arrows move the selected point 1/20 of the plot (Shift 1/200),
   Delete removes it, Tab cycles points; a chain chip focuses and Alt+Left / Alt+Right moves it one slot.
8. Neutral defaults: opening a saved v1 preset shows these neutral defaults without any
   dirty dot: LFO tabs on a built-in curve, Warp Off, Pitch 0, Sub shape Sine, noise level at the
   floor, chain order default.

## 11. Open questions

1. Mod A and Mod B lose their on-panel route knobs (Pitch, Index, Shift; Motion, Vol, Cutoff, Mod A).
   They stay in MATRIX built-in routes and via drag. If the owner wants them on the SYNTH page, the
   cleanest place is a 24 px "ROUTES" strip appended under the controls row (the module would grow
   to 292 px and the Osc rows would lose 32 px, which they cannot spare), so this needs a decision.
2. Filt Env, Amp A and Amp B chips move into the MORE popover. If they must stay one click away, row
   1 needs the chips shortened to the glyph disc plus activity bar (about 40 px each).
3. The Limiter's large output spectrum shrinks to a 260 x 44 strip display to pay for the chain
   bar, Phaser and Flanger. Acceptable, or give the strip the two knob rows' height (the third FX row
   would have to give up the Reverb decay curve instead)?
4. Pink and yellow join amber, blue, rose and violet as source colours, and the pink/violet pair is
   only Delta E 29.7 apart (still above the 25 gate). A glyph (C, D, M) carries the difference; confirm.
5. One-shot is folded into the Trigger mode (Envelope) and mirrored by the 1-SHOT LED. If the owner
   prefers a single control, drop the LED and widen Trigger by 42 px.
6. Pitch All has no knob (matrix target only, ring on Osc A/B Pitch and Sub Oct). Say if a knob is wanted
   in the Voice group (which has room for one S cell).

## 12. As built

Code: `crates/gts/src/editor/modulator.rs` (module, tabs, shape editor, envelope editor, library, segment menu),
`editor/sub.rs` (SUB | NOISE), `editor/chain.rs` (chain bar, grips, badges, the drag), `fxview.rs` (phaser and
flanger displays, the compact reverb and limiter displays), `rr-ui` (`SRC_C`, `SRC_D`, `SRC_N`, `well_at`,
`group_at_indent`). Shape edits go through `MatrixShared::edit_shapes` (which calls `ShapesShared::edit`) and the
order through `MatrixShared::set_fx_order`: both move `matrix_rev` (the Live Set is marked modified), history
snapshots are whole documents (matrix, shapes, order) and a drag is one undo step (`begin_gesture` /
`end_gesture`); a preset load is one step for all three (`replace_doc`). The param id `matrix_rev` is unchanged.

What the build changes against the proposal, with the reason:

1. **Mod Env colour** is #ff8254, not #ff7f50: the mock value measures 6.97:1 on the inset and the gate is 7.0.
   `rr-ui::theme` tests list all five modulator colours (7:1 on well, display and inset, Delta E 25 from every
   family, 40 from the accent).
2. **The LIB popover has a CURVES row** above the stock shapes: the nine built-in curves and Custom as icon
   buttons (the mock has no way to pick Sine or Saw back). The stock list is the engine's fifteen shapes in two
   columns of eight 24 px rows, so the popover is 336 x 262. Editing a built-in curve in the plot (a drag, a
   double-click, STEP) turns the LFO Custom first, from the curve sampled into points (Sine 17 points, Triangle 4, the
   saws 2, Square 4, Decay and Attack 11); the random curves convert to their display pattern.
3. **The Mod A and Mod B targets** (A, B, Both) had no place in the module; they are two combos in the header of the
   MATRIX page's "Built-in routes" group ("Mod A moves", "Mod B moves"). The seven route knobs are MATRIX rows only.
4. **Phaser and Flanger Spread** (`phs_spread`, `flg_spread`) is a 22 px dial in the title row, left of the On
   switch: the strips of the proposal have no cell for it. The hover line names it and shows the angle.
5. **The Pitch knob** has a centre detent (0.75 % of the range, the host receives exactly 0 inside it, the drag keeps its
   own position so it leaves smoothly) and reads one decimal ("+0.0 st"): `params.rs` formats the three pitch params so.
6. **Step mode** draws the engine's glide (the last `glide` of a step ramps linearly to the next level), not a smoothstep,
   and the toolbar is LIB 74, STEP 52, SNAP 52, STEPS 46, GLIDE 64 (the sum of the proposal's widths left 62 px).
7. **Phaser and flanger displays** sweep on time (the free rate, or the division at the host tempo, which the engine
   now publishes as `gf::TEMPO`): the chain has no live phase telemetry. They show the response and the sweep, not the
   audio. The Mod C, Mod D and Mod Env playheads read per-voice records added to `viz` (`vf::PH_C`, `PH_D`, `OUT_C`, `OUT_D`,
   `MENV`, `MENV_STAGE`).
8. **The 1-SHOT LED and the Trigger combo** are one state: the LED is lit when the one-shot flag is set or the trigger is
   Envelope; the combo shows Env in either case; picking a mode clears the flag; the LED sets Envelope.
9. **Filter curves** for the new types are drawn from the DSP's own laws (HP12 and Notch from the SVF, LP24 from the two
   Butterworth sections with their global feedback, Formant from the three vowel band-passes at the cutoff's vowel; its
   dot rides the first formant). `editor::filter_db` is covered by `tests/rings.rs`.
10. **Not built**: the 1.2 degree tilt of the chain ghost (the ghost is lifted and shadowed, not rotated), the in-editor
    Ctrl+Z (the host undo covers shape and order edits through `matrix_rev`, one step per gesture), the extension of
    `motion_gl -- aa` with editor crops (acceptance 4: the grid lines and ticks are snapped to physical pixels by the same
    helper as the other displays, but no GL coverage test covers them yet) and the 15 Hz data snapshots of Reduced
    motion (deferred since MOTION.md). Built from the motion list: the tab slide and content cross-fade, the library
    popover and its morph, point grow-in (100 ms) and fade-out (90 ms), the value pill, the chain slide, gap, settle and
    cancel, and Reduced motion for all of them.

Measured (acceptance section 10), same machine and method as MOTION.md section 10: UI + tessellation per page and the shape editor as in MOTION.md section 10 (all within 4.0 / 8.0 ms, 98.6k vertices, 0.3 ms and 3.5k); `plugin_loop`: SYNTH 61.70 fps mean, p99 interval 30.86 ms, 4.40 % of frames over 20 ms (runs 4.40 / 4.40 / 3.90), frames over 50 ms in the three runs: 0 / 0 / 0; FX 62.00 fps mean, p99 interval 30.78 ms, 3.90 % of frames over 20 ms (runs 3.90 / 3.90 / 3.30), frames over 50 ms in the three runs: 0 / 0 / 0; MATRIX 62.20 fps mean, p99 interval 30.61 ms, 3.50 % of frames over 20 ms (runs 3.50 / 3.20 / 3.70), frames over 50 ms in the three runs: 0 / 0 / 0 (the runs were taken with no other compile or benchmark job on the machine; two earlier sets taken while another job was compiling showed 5 to 7 % of frames over 20 ms and the occasional 50 ms hitch from that job, not from the editor: update() takes 1.1 ms on SYNTH, 1.1 ms on FX and 0.8 ms on MATRIX).
