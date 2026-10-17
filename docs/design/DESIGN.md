# Riddim Rack design contract

The look shared by GTS and VSS: an anodised rack panel, flat arc knobs, glass displays and one
accent colour. Sections 6.1 / 6.1a and the no-animation principles are superseded by `MOTION.md` (v2 panel: 1000 x 832, motion, hover-revealed values, popover PATCH). This file is the contract; `crates/rr-ui` is its implementation and
`docs/design/preview-rr-ui.png` / `preview-rr-ui-vss.png` are what that implementation renders.
The approved direction is **Variant B, Mint** from `mockup-v1.html` (slate panel, mint accent).
Amber is not shipped.

All sizes are logical px (egui points) on an 8 px grid with 4 px half steps. Everything is drawn
with the painter: rects, strokes, circles, arcs as polylines, text. No images.

## 1. Principles

1. One hue. The accent marks state and nothing else: value arcs, lit LEDs, the display trace,
   active cells, slider fill, the playing step. Chrome stays neutral slate.
2. Hierarchy by depth, not by colour. Panel is the highest surface, wells sit one step down,
   insets (combos, toggles) one step further, display glass is the deepest.
3. Restraint. Glow is a hint of light, never a halo. If glow is the first thing you notice,
   it is too strong.
4. Fixed-size editors. GTS is 1000 x 760, VSS is 1064 x 560 (760 x 560 before the PATCH column). Layout is specified in px below.

## 2. Tokens

### 2.1 Colour (Mint)

| Token | Hex | Used for |
|---|---|---|
| `panel_top` / `panel_bottom` | `#232a2e` / `#192024` | panel face, vertical gradient |
| `well` | `#11171a` | group background |
| `inset` | `#141b1e` | combo and toggle bodies, popup |
| `rail` | `#2a3236` | rails (gradient to rail mixed 30 % with black) |
| `rail_slot` | `#0b1012` | rail slots |
| `text` | `#e6ece9` | readouts, combo text, active toggle label |
| `muted` | `#93a19b` | knob labels, toggle labels (off), descriptor, captions |
| `title` | `#83918b` | engraved group titles |
| `accent` | `#5ee6a8` | the only hue |
| `track` | `#334046` | knob track, slider track |
| `cap1` / `cap2` | `#273136` / `#36434a` | knob cap, thumb, screw (dark / light) |
| `led_off` | `#1b1b1e` | unlit LED |
| `plate_text` | `#d8d5ce` | nameplate name |
| `disp_top` / `disp_bottom` | `#0f1214` / `#050607` | display glass, top 40 % gradient |
| `disp_caption` | `#7b868b` | display and grid captions |
| `tick_off` | `#2a3034` | inactive grain tick |
| `cell_off` / `cell_off_stroke` | `#0d1012` / `#171c1f` | empty step cell |

Derived, never separate tokens:

| Name | Value |
|---|---|
| `hi` (raised edge) | white at 7 % |
| `lo` (recessed edge) | black at 60 % |
| accent at alpha *a* | `accent` x *a*, written `accent@a` below |

Deviation from the mockup sheet: `disp_caption` is `#7b868b` (mockup `#6c7478` measures 3.95:1 on
the display, below the 4.5:1 floor). The mockup gave no mint engraved-title colour; `title` is
chosen to sit between `muted` and the well at 5.5:1.

### 2.2 Type

Two families, both SIL OFL, embedded with `include_bytes!` from `crates/rr-ui/assets/fonts/`
(licences alongside).

* **Barlow** Medium 500 and SemiBold 600 for every label, title and the nameplate.
* **JetBrains Mono** Regular and Medium for numeric readouts. Monospace gives fixed-width
  digits, so readouts do not jitter while a knob is dragged.

| Role | Face | Size | Case / tracking | Colour |
|---|---|---|---|---|
| Nameplate name | Barlow SemiBold | 30 | caps, +8 | `plate_text`, 1 px shadow above (black 60 %) and below (white 8 %) |
| Descriptor | Barlow Medium | 11 | caps, +1.6 | `muted` |
| Status line | Barlow Medium | 11 | as typed | `muted` |
| Group title | Barlow SemiBold | 11 | caps, +1.4 | `title`, engraved: black 40 % 1 px above, `hi` 1 px below |
| Knob / slider / combo label | Barlow Medium | 11 | caps, +0.5 | `muted` |
| Toggle label | Barlow Medium | 11 | caps, +0.5 | `muted` off, `text` on |
| Readout | JetBrains Mono Regular | 12 | as typed | `text`; `accent` while hovered or dragged |
| Combo text | Barlow Medium | 13 | as typed | `text` |
| Display caption | Barlow Medium | 11 | caps, +0.5 | `disp_caption` |
| Display readout | Barlow Medium | 12 | caps | `accent` |
| Grid step numbers | Barlow Medium | 11 | | `disp_caption`; playing step `accent` |
| Grid row labels | Barlow SemiBold | 12 | caps, +0.5 | `muted` |

Barlow has a lower x-height than the mockup's original sans, so mixed-case text (combo values)
is 13 instead of 12. Caps labels stay at 11 because caps height matches.

egui has no letter spacing: tracked text is painted glyph by glyph with the extra advance.

### 2.3 Geometry

| Item | Value |
|---|---|
| Panel padding | top 16, bottom 20, sides 44 (28 rail + 16 gutter) |
| Group gap | 12 |
| Group | radius 6; padding 8 top / 12 sides / 8 bottom; title row 24; 4 between title row and body |
| Group chrome height | 8 + 24 + 4 + 8 = 44, body starts 36 below the group top |
| Control height (combo, toggle) | 24, radius 4 |
| Knob | 40 (GTS) or 44 (VSS) |
| Knob cell | width 56, or 52 / 50 / 44 in dense rows; height 13 label + diameter + 14 readout (67 for 40) |
| Knob rows | 6 between rows inside a group |
| Display (GTS) | 124 high, radius 6 |
| Step cell | 32 x 32, radius 4 |

## 3. Components

### 3.1 Rack panel

* Face: vertical gradient `panel_top` to `panel_bottom` over the whole window; 1 px black outline
  inside the window edge; 1 px `hi` line along the top edge. (The mockup's faint brushed
  texture is not drawn: at 1.2 % alpha it is invisible on screen.)
* Rails: 28 px wide at each side, full height. Horizontal gradient `rail` to rail mixed 30 % with
  black. 1 px `lo` line on the inner edge.
* Slots: 10 x 18 px, radius 5, `rail_slot`, `hi` 1 px lower. Left edge of the slot 8 px from the
  rail's left edge. First slot 44 px from the top, then every 40 px, stopping 60 px above the
  bottom.
* Screws: four, centred in the rail 14 px from the side, 19 px from top or bottom. Layers: black
  disc r 7.5; `cap1` r 7; `cap2` r 4.5 offset (-1.5, -1.5); slot line 10 x 2 px in `#050506`.
  Slot angles: top-left 20, bottom-left 75, top-right 130, bottom-right 50 degrees.
* Nameplate: at the content top-left, GTS 232 x 124, VSS 168 x 124. Radius 6, gradient
  `panel_bottom` to `well`, 1 px `lo` inner stroke, `hi` 1 px below. Content is a vertically
  centred block with 16 px left padding: name (36 line), 4, descriptor (14 per line, wraps to
  two lines when wider than the plate minus 32), 12, status line with an 8 px LED 4 px from the
  text. LED is lit.
* Header band: the 124 px row to the right of the nameplate, starting 16 px after it. Body
  starts 16 px below the header band.

### 3.2 Group (well)

Fill `well`, radius 6, 1 px `lo` line just inside the top edge, `hi` 1 px lower than the well
(painted first, so only the bottom edge shows). Title row 24 px: engraved title at the left; a
1 px hairline from the title end + 8 to the header controls - 8, white at 14 % fading to
transparent left to right; header controls (a toggle, a combo) right-aligned, 8 apart. Body
below. Groups in a row have explicit widths and a shared height so edges align.

### 3.3 Knob

* Cell: label (13 px band, centred, caps), knob, readout (14 px band, centred).
* Arc: centre at the knob centre, radius d/2 - 3 (17 for 40, 19 for 44), sweep 270 degrees
  starting at 135 degrees, clockwise with y down. Track: 3 px `track`, round caps. Value arc:
  `accent`, 3 px, round caps, from the start to 135 + 270 x v. Bipolar knobs draw from 270
  degrees (12 o'clock) to the value, and draw nothing within 1 degree of centre.
* Cap: disc of radius arc radius - 6 in `cap1` with a 1 px black ring, then a disc of radius
  arc radius - 8 in `cap2` shifted up 1 px.
* Indicator: 2 px line in `text` from 0.35 to 0.9 of the cap radius at the value angle, round
  ends.
* States: rest as above. Hover or drag: readout turns `accent`, a third glow pass is added
  (see 5), cursor is the vertical resize cursor.

### 3.4 LED toggle

24 px high, radius 4, inset body (`inset` fill, 1 px `lo` inner stroke, `hi` 1 px below).
Padding 8. LED 8 px at the left, 8 px gap, label. Without a label the control is 24 x 24.
Off LED: `led_off` disc with a black 1 px ring. On LED: accent disc plus glow (see 5); label
turns `text`. Hover lightens the body with white 4 %.

### 3.5 Combo (inset dropdown)

24 px high, radius 4, inset body. Text 8 px from the left, chevron (two 1.5 px `muted` strokes,
7 x 3.5) centred 11.5 px from the right. Width = widest item + 32, minimum 56. With a label the
combo is a knob-row cell: the label sits in the 13 px label band and the control is centred on
the knob band so it lines up with neighbouring knobs. The popup opens below, same minimum
width, `inset` fill with a black 1 px stroke and radius 4; rows 24 px; hover row fills
`accent@0.16`; the selected item's text is `accent`.

### 3.6 Horizontal slider

168 px wide (a parameter), 38 px high. Label left and readout right in the 13 px top band.
Track 4 px high, radius 2, `track`, spanning the width minus 7 px each side so the thumb never
leaves the cell; fill `accent` with a halo (expanded 2 px, `accent@0.10`). Thumb 14 x 16,
radius 3, `cap1` with a `cap2` top half and a black 1 px outline, centre indicator 2 x 10 px in
`text`. The track centre is 29 px below the top. Click or drag sets the value from the pointer
x position.

### 3.7 GTS display

124 px high, fills the header band. Glass: radius 6, `disp_bottom` fill, top 40 % gradient from
`disp_top`, inner shadow (black 50 % fading over 10 px from the top edge), 1 px black outline,
`hi` 1 px below. Contents inset 16 px each side:

* Graticule: 9 vertical lines (white 5 %) from y 22 to 92; centre line (white 10 %) at y 54.
* Waveform: amplitude +-34 px about y 54, the glow recipe in 5, round joins.
* Neighbour grains: 1 px `accent@0.14` traces behind.
* 32 grain ticks, 6 px wide, radius 1.5: inactive 8 px high at y 106 (`tick_off`), active 12 px
  at y 102 (`accent`). Marker: 10 x 5 triangle with its tip at y 102 over the active tick.
* Caption top-left (y 14), readout top-right (y 14).

### 3.8 VSS step grid

Fills the width of its well. Glass as the GTS display. The grid block is centred: row label
column 36, 4 gap, then 16 columns of 32 px cells with 4 px gaps; the header row of step numbers
is 16 px high with 4 below; rows are 32 px with 12 px gaps (an editing lane reads better with
more air between vowels than between steps). Radius 4.

| Cell state | Look |
|---|---|
| Empty | `cell_off`, 1 px inner stroke `cell_off_stroke` |
| Empty in the playing column | `accent@0.07` fill, 1 px `accent@0.18` stroke (column wash) |
| Selected | `accent@0.20` fill, 1 px `accent@0.55` stroke, outer ring (cell + 2) `accent@0.18` |
| Selected Hold row | vertical `accent@0.16` to `accent@0.08` fill, 1 px `accent@0.35` stroke, no ring |
| Selected and playing | solid `accent`; rings at +2 px `accent@0.55` and +6 px `accent@0.22` |
| Beyond the Steps value | everything above at 35 % alpha, still clickable |

Click or drag across cells sets the row of each column the pointer crosses; the widget reports
only cells whose row actually changes. Pointer hit-testing gives the 4 and 12 px gaps to the
nearest cell, so there are no dead strips.

### 3.9 PATCH strip (RR-BUS)

`widgets::patch_strip(ui, label, rows)`. A group (3.2) titled PATCH, 124 px high
(`PATCH_H` = 44 chrome + 3 x 24 + 2 x 4), width from the caller (300 on GTS, 292 on VSS; the
combos share what the fixed parts leave). The instance label sits in the 24 px header row, right
aligned: a 104 px inset box (same fill, stroke and lower highlight as a combo) holding editable
JetBrains Mono 12 text, `text` colour, 8 px side padding, 16 characters max, text cursor.

Body: three 24 px rows, 4 px apart, each `source combo | target combo | amount knob | LED` with
6 px between; the combos split the free width 54 / 46, the knob is 24 px and the LED cell 12 px.

| Part | Look |
|---|---|
| Source combo | combo (3.5) at 12 px; first item "None", then every live source as `<label> . <name>`; selected item `accent`. Text is cut with an ellipsis, the popup is as wide as its longest item |
| Source lost | the persisted name in `muted@0.75`, row knob at 40 % and LED forced off; picking a live source or None clears it |
| Source None | "None" in `muted@0.75`, knob at 40 %, LED off |
| Target combo | combo at 12 px, `text` colour |
| Amount knob | 24 px bipolar knob: 2.5 px track arc, value arc from 12 o'clock (`accent`, +5 px `accent@0.18` glow), 4 px-inset cap, 1.5 px pointer. Same gesture as 3.3: drag up raises, Shift is 10x finer, double-click resets to the centre |
| LED | the LED of 3.4 with its glow and ring alpha and the fill alpha scaled by `activity01` (|incoming value|); below 2 % it reads as off |
| Label focus | egui text-edit caret in `text`; no extra focus ring |

Events (see the doc comments): `source_changed` is `Some(None)` for "None",
`target_changed` an index into `targets`, `amount` a `KnobResponse` (wrap `drag_started`,
`changed`, `drag_ended` in begin / set / end), `label_changed` the new text. The strip never
mutates the rows; the caller applies the events.

### 3.10 Segmented page switch

Inset 24 px bar (the toggle body) holding caps labels in 11 px Barlow, 16 px padding per
segment, 2 px inside the bar. The selected segment is lit: `accent@0.20` fill, 1 px `accent@0.55`
stroke, `accent` text, radius 3. Others `muted`, `text` plus white 4 % on hover.

## 4. Interaction rules

| Control | Rule |
|---|---|
| Knob | Vertical drag. Up raises. 200 px of travel is the full range. Shift held: one tenth of that (fine). The delta is applied per frame, so pressing or releasing Shift mid-drag never jumps the value. Double-click resets to the default. Hover or drag shows the value in accent; the readout is always visible. |
| Slider | Click or drag sets the value from the pointer position (absolute). |
| Toggle | Click flips. |
| Combo | Click opens, click an item picks and closes, Escape or a click elsewhere closes. |
| Step grid | Click or drag paints the pointer's row into each column crossed. |

Gesture contract for the plugin editors: `KnobResponse { changed, drag_started, drag_ended,
reset }`. Call `begin_set_parameter` on `drag_started`, `set_parameter` on `changed`,
`end_set_parameter` on `drag_ended`. A `reset` arrives with `changed = Some(default)` and no
drag flags: wrap it in begin / set / end in one frame. Toggles, combos and grid cells wrap each
returned change in begin / set / end.

## 5. Glow recipes (painter strokes only)

| Element | Layers, drawn in order |
|---|---|
| Value arc | same arc at 7 px `accent@0.18`, then 3 px `accent`; hovered adds 11 px `accent@0.06` underneath |
| LED on | discs r 10 `@0.04`, r 8 `@0.07`, r 6.5 `@0.12`; 1 px ring r 5 `@0.4`; core disc r 4 `accent` |
| Waveform | the same polyline at 12 / 6 / 2 px with alpha 0.07 / 0.16 / 1.0 |
| Active step | ring at +2 px `@0.18`, fill `@0.20`, 1 px stroke `@0.55` |
| Playing step | rects at +6 px `@0.22` and +2 px `@0.55`, then solid `accent` |
| Slider fill | fill rect expanded 2 px `@0.10` under the solid fill |
| Shaper curve | line at 5 px `@0.25`, then 1.5 px `accent` |

Alpha layers are accent colour multiplied (premultiplied), never white over the panel.

## 6. Layouts

Coordinates are window px; x runs 44 to 956 inside the rails.

### 6.1 GTS, 1000 x 760

Header band y 16 to 140: nameplate (44, 16) 232 x 124, display (292, 16) 352 x 124, PATCH
(656, 16) 300 x 124.
Body starts at y 156. PATCH lives in the header band, not row 4: rows 1 to 4 are already 912
px wide, so no bottom-row group can take the 280 px a legible source | target | knob | LED row
needs, and tightening Velocity alone frees only about 60. Window stays 1000 x 760.

| Row | y | Height | Groups (x, width) |
|---|---|---|---|
| 1 | 156 | 184 | Osc A (44, 450), Osc B (506, 450) |
| 2 | 352 | 141 | Mod A (44, 316), Mod B (372, 356), Shaper (740, 216) |
| 3 | 505 | 111 | Filter A (44, 300), Filter B (356, 300), Filter Env (668, 288) |
| 4 | 628 | 111 | Voice (44, 258), Wheel (314, 200), Velocity (526, 290), Output (828, 128) |

Row 4 ends at y 739, leaving 21 px above the window edge.

Group contents (header controls right-aligned in the title row):

* Osc A / B: header Table combo, On toggle. Row one Oct, Semi, Cent, Index, Motion, Shift, Vol
  (7 x 56). Row two Att, Dec, Sus, Rel. Oct, Semi, Cent, Motion, Shift are bipolar.
* Mod A: header Curve combo. Strip: Sync toggle, Div combo, 1-shot toggle, Target combo.
  Knobs Rate, Pitch, Index, Shift (56; Pitch, Index, Shift bipolar).
* Mod B: same header and strip. Knobs Rate, Motion, Vol, Cutoff, Mod A (52).
* Shaper: header On toggle. Strip Mode combo. Amount knob and a 64 x 48 curve preview 8 px to
  its right.
* Filter A / B: header Env toggle. Type combo cell, then Freq, Res, Kbd (56).
* Filter Env: header Inv toggle. Att, Dec, Sus, Rel, Amt (52).
* Voice: header Legato toggle. Routing combo cell, Poly, Porta, Bend (50).
* Wheel: Index, Shift, Cutoff, Mod (44, bipolar). Velocity: Vol A, Vol B, F.Env, Att, Shift, Mod
  (44, bipolar). Output: Volume, Spread (52).

Fit check, inner width = group width - 24: Osc 426 holds 7 x 56 = 392; Mod A 292 holds the strip
(about 272) and 4 x 56; Mod B 332 holds 5 x 52; Filter 276 holds a combo cell (about 84) plus
3 x 56; Filter Env 264 holds 5 x 52 = 260; Voice 234 holds 84 + 3 x 50; Wheel 176 = 4 x 44;
Velocity 266 holds 6 x 44 = 264; Output 104 = 2 x 52.

### 6.1a GTS page switch and FX page

The header's settings column is 120 px wide: a segmented page switch (3.10) over the Quality
combo and the Preset combo (Init + the six factory presets; the closed control reads
`<name> *` once a loaded preset is edited, "Custom" before any is loaded). SYNTH is the layout above; FX swaps only the body (window stays 1000 x 832, `preview-gts-editor-fx.png`).
The FX body is 912 x 656: eight groups in chain order, each with an on LED in the title bar
(Limiter has none) and its Mix knob first. Knob cells are 56 px, 60 px where a readout can be
eight characters ("120.0 Hz", "-24.0 dB"); signed params (Bias, EQ and band gains, In / Out,
Chorus feedback) are bipolar.

| Row | Height | Groups (width) |
|---|---|---|
| 1 | 257 | Distortion (272), Multiband (384), EQ (232) |
| 2 | 184 | Chorus (336), Dimension (200), Delay (352) |
| 3 | 191 | Reverb (312), Limiter (588) |

The Limiter group holds the Gain knob and a horizontal gain-reduction meter (inset track, accent
fill from the left, 0 to -12 dB with a tick label every 3 dB) over an Output Peak readout.
Inactive controls are dimmed to 50 % opacity but stay editable: a module's body while its on LED
is off, Delay Time / Chorus Rate while Sync is on, Delay Div + Feel / Chorus Div while it is off,
Osc B while off, the Shaper while off.

Multiband is a 6 x 3 grid of 60 px cells: Mix / Depth / Time, In / Out, the two crossovers on the
left, and one column per band (Lo, Mid, Hi) with Up, Down, Gain rows on the right.

### 6.2 VSS, 1064 x 560

Header band y 16 to 140: nameplate (44, 16) 168 x 124, Voice group (228, 16) 376 x 124, Sequence
group (616, 16) 100 x 124, PATCH (728, 16) 292 x 124 as the fourth column. The four columns need
1064 px; the old 760 px window cannot hold them (Voice cannot go below 7 x 50 px cells), so VSS is
1064 x 560 with the Vowel pattern group stretched to 976 and the grid still centred. (Open: if the
VSS window must stay 760 wide, PATCH needs a second header band instead.) The Steps slider and Rate combo live in the Sequence group of the
header row.

* Voice: Formant (bipolar), Reson., Drive, Glide, Swing, Mix, Output; 44 px knobs in 50 px cells.
* Sequence: Rate combo (label above, 37 px cell), then a 76 px Steps slider 4 px below.

Body: the Vowel pattern group at (44, 156), 976 x 384, ending at y 540. Header caption on the
right reads "Step N . Vowel". The step grid fills the group body (940 x 340), grid block 612 x 272 (VSS stretches cells to fill)
centred.

## 7. Accessibility

* Contrast floor 4.5:1 for text and for text-like marks. Measured, and enforced by unit tests in
  `rr-ui::theme`:

| Pair | Ratio |
|---|---|
| `text` on `well` | 15.1 |
| `muted` on `well` (labels) | 6.7 |
| `title` on `well` | 5.5 |
| `accent` on `well` | 11.5 |
| `muted` on `inset` | 6.5 |
| `muted` on panel top (descriptor, status) | 5.4 |
| `disp_caption` on display | 5.0 |
| `accent` on display | 12.0 |

* The unfilled knob and slider track is a decorative background (1.7:1 on the well). State is
  always carried by something stronger: the value arc is 6.8:1 on the track and the cap
  indicator is `text`.
* On / selected is never colour alone: a lit LED is brighter, haloed and paired with a label
  that changes from `muted` to `text`; a selected cell has fill, stroke and ring; the playing
  step is also numbered in accent.
* Minimum text is 11 px. Smallest targets: toggles and combos 24 px high, knob cells 44 x 67,
  grid cells 32 px (VSS stretches its cells to fill the group).
* Nothing is animated. Nothing depends on hover to be readable.

## 8. Using the kit

`rr-ui` depends only on egui (the version pinned by `Cargo.lock`, currently 0.31.1). Plugin
editors call `rr_ui::theme::apply(ctx)` once in the editor build closure (it installs the fonts,
which only take effect from the next pass, so do not call it from the update closure), put an
`egui::CentralPanel::default().frame(egui::Frame::NONE)` over the whole window, and call
`rr_ui::panel::rack_panel` with the nameplate size. Widgets return values; the editor maps them
onto its param setters as in section 4. `cargo run -p rr-ui --example preview` regenerates the
GTS preview PNG; add `-- vss` for VSS and `-- live` to keep the window open.
