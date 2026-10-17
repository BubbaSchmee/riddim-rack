# GTS visual and motion spec (v2 panel)

Status: built and approved; chip drag-and-drop patching, OS-default Reduced motion and 15 Hz data snapshots were deferred. Companion to `DESIGN.md` (the static contract) and
`motion-feasibility.md` (what the stack can carry). The running reference is
`prototype-v2.html`; the stills are `prototype-v2-1.png` to `prototype-v2-5.png`.

Keep from `DESIGN.md`: Variant B mint, Barlow + JetBrains Mono, rack rails, slots, screws, the
engraved nameplate, one accent hue, 6 px group radius, painter-only drawing. Everything below
raises the finish and makes the panel move; nothing here adds a parameter or a section that
`crates/gts/src/params.rs` does not have.

## 0. Open the prototype

`prototype-v2.html` is one self-contained file (inline CSS, JS, canvas, SVG; Barlow and JetBrains
Mono embedded from `crates/rr-ui/assets/fonts`). It scales the 1000 x 832 panel to the window.
A simulated riddim 1/8 pattern at 150 BPM (F1, G1, A#1, 16 steps) drives every display, so the
panel moves with no input. The page toolbar has Pause, Always show values and Reduced motion.

Real, working interaction: drag a knob (Shift = fine, double-click = reset, wheel, arrow keys),
click a combo, click the preset name or its arrows, click SYNTH / FX, click PATCH, click the
scope to flip to the spectrum, click 2D / 3D or A / B on the hero, drag the hero sideways to
scrub Index, hover a source chip to light its targets, drag a chip onto any knob to patch it
(live preview ring while you hover a target, release to keep it, right-click a knob to remove
the rings you added).

Still-frame switches (used for the PNGs): `?at=<seconds>` freezes the simulation at that time
after a 1.5 s pre-roll, `&page=fx`, `&mode=2d`, `&osc=b`, `&scope=spec`, `&open=preset|patch`,
`&hover=<param id>`, `&drag=<param id>`, `&src=A|B|E|M`, `&rm=1`, `&values=1`, `&ta=<table 0-8>`.

## 1. What changed and why (against the ten gaps in `docs/standards/visual-motion.md`)

| # | Gap | Answer in this spec |
|---|---|---|
| 1 | 30 Hz, interaction-driven repaint | Everything moving is time-based at the stock ~62 Hz cadence; the window already repaints every tick, so no new repaint plumbing (section 8) |
| 2 | Hero is 300 x 124 (4.5 %) | Hero is 568 x 232 = 56.8 % of window width, 27.9 % of height, 15.8 % of area (3.4 x today's area) |
| 3 | No live motion on the display | Lit frame follows the live Index every frame; Mod A one-shot visibly pulls it on each note; glow level follows the amp envelope |
| 4 | No modulation rings | Ring + live dot on every modulated knob, source colour, 60 fps |
| 5 | No LFO / envelope editors with playheads | Mod A, Mod B, Osc A amp, Osc B amp, Filter Env displays, one playhead per sounding note |
| 6 | No scope / spectrum / meter ballistics / moving filter curve | Header scope + spectrum, stereo meter with hold, both filter curves move with cutoff modulation |
| 7 | No popup / fine adjust / reset / tooltips | Value popup on first movement, Shift = 1/10, double-click reset, wheel, keys, one-line help bar replaces floating tooltips |
| 8 | No mod drag-and-drop | Source chips; targets highlight, live ring preview, drop to commit |
| 9 | No hover feedback, durations, easing, tokens | Section 3 and 4: duration tokens, one easing family, colour roles, 8 px grid, depth rules |
| 10 | No UI scale, HiDPI line crispness | Line rule in section 6; scale steps stay a separate patch (open question 4) |

The single large trade: **resting knobs no longer print their value.** The label swaps to the
value (accent, mono) on hover, drag and focus, and the help bar shows value + live value + a
sentence. That frees 14 px per knob row, which is what pays for a hero this size without
shrinking any control below a 32 px knob or an 11 px label. A persisted "Always show values"
setting swaps every label for its value with no layout change (the toolbar checkbox shows it).
This is open question 1: the owner decides.

## 2. Layout (window stays 1000 x 832, no resizing)

Coordinates are window px. Content x runs 44 to 956. Group gap is **8** (was 12), group padding
12 sides, title row 24 at y+8, body from y+34, bottom padding 8. Knob sizes (3, as the standards
ask): **L 44** (Osc Index, Filter Freq, Limiter Gain), **M 40** (default), **S 32** (dense rows:
Filter Env, Shaper, Voice, Wheel, Velocity, header Volume and Spread, FX EQ), **XS 24**
(PATCH amounts). Cell height = 14 label band + dial + 2 (L 60, M 56, S 48).

### 2.1 SYNTH page

| Block | x, y | w x h | Contents |
|---|---|---|---|
| Nameplate | 44, 16 | 156 x 56 | GTS wordmark, descriptor, note LED + note name (lit while the gate is open) |
| Preset bar | 208, 16 | 232 x 56 | prev arrow, category + name (name gets a dot while edited), next arrow; click name opens the list |
| Page switch | 448, 16 | 132 x 26 | SYNTH / FX segmented, sliding lit segment |
| Quality | 448, 48 | 54 x 24 | 1x / 2x / 4x combo |
| PATCH chip | 508, 48 | 72 x 24 | three activity LEDs; opens the PATCH popover |
| Scope / spectrum | 588, 16 | 184 x 56 | click to flip; tabs SCOPE / SPEC |
| Meter | 780, 16 | 14 x 56 | stereo, peak hold |
| Output | 802, 16 | 154 x 56 | Volume, Spread (S knobs) |
| **Hero** | 44, 80 | **568 x 232** | wavetable, A / B tabs, table name, index readout, 2D / 3D |
| Sources strip | 44, 320 | 568 x 20 | chips Mod A, Mod B, Env, Macro 1 with a live activity bar |
| Mod A | 620, 80 | 336 x 126 | Curve combo; Sync, Div, 1-shot, Target; shape display; Rate, Pitch, Index, Shift |
| Mod B | 620, 214 | 336 x 126 | same strip; shape display; Rate, Motion, Vol, Cutoff, Mod A |
| Osc A | 44, 348 | 404 x 222 | Table combo + On; Oct Semi Cent Index(L) Motion Shift Vol; Att Dec Sus Rel Curve + amp display; Unison Detune D.Crv Blend Width Rand + Reset |
| Osc B | 456, 348 | 404 x 222 | same; body dims to 45 % while Osc B is off |
| Sub | 868, 348 | 88 x 222 | On; Level, Oct, Xover |
| Filter A | 44, 578 | 262 x 102 | Type combo + Env; Freq(L) Res Kbd; response display |
| Filter B | 314, 578 | 262 x 102 | same |
| Filter Env | 584, 578 | 372 x 102 | Inv; Att Dec Sus Rel Amt Curve (S); envelope display |
| Shaper | 44, 688 | 212 x 90 | Mode combo + On; Amt; transfer curve |
| Voice | 264, 688 | 224 x 90 | Routing combo; Poly Porta Bend; Legato |
| Wheel | 496, 688 | 192 x 90 | Index Shift Cutoff Mod |
| Velocity | 696, 688 | 260 x 90 | Vol A, Vol B, F.Env, Att, Shift, Mod |
| Help bar | 44, 792 | 912 x 28 | hovered control: name, value, live value, one-line description; right side: tempo, pattern, note |

Groups end at y 778. The hero's 568 px is 56.8 % of the window width, its 232 px is 27.9 % of
the height.

Where things moved: **PATCH** left the header band and became a popover (312 x 146) anchored
under the PATCH chip (section 5.9); its three rows (source, target, amount, activity LED) are
unchanged and the chip carries the live activity so the feature is never hidden. **Quality**
moved under the page switch. **Preset** moved into the header as a bar with arrows. The grain
display's slot, the settings column and the PATCH strip are gone; the hero takes the header
band's height and more.

### 2.2 FX page (window and header unchanged, body swaps)

| Row | y, h | Groups (x, w) and displays |
|---|---|---|
| 1 | 80, 262 | Distortion (44, 272): Mix Drive Bias Out, Type, Auto, Filter F.Type F.Freq F.Q, transfer curve with live dot. Multiband (324, 384): the 6 x 3 grid of DESIGN 6.1a, three band meters with up/down thresholds in the empty cell. EQ (716, 240): response curve over a faint live spectrum, three bands, Peak Q, Low Cut + Cut Freq |
| 2 | 350, 164 | Chorus (44, 340) with voice-phase dots. Dimension (392, 204) with L / R traces whose offset follows Size. Delay (604, 352) with tap bars that flash when the echo lands |
| 3 | 522, 256 | Reverb (44, 312) with decay curve and a playhead. Limiter (364, 592): Gain(L), gain-reduction meter (instant attack, release 6 dB/s), output spectrum, Output Peak |

A module that is off dims its body to 45 % but stays editable (as in DESIGN 6.1a). Both pages
share the header, so the scope and meter keep moving while you tweak FX.

## 3. Tokens

Colour (all existing tokens unchanged; new rows marked +).

| Role | Token | Hex | Use |
|---|---|---|---|
| Panel | `panel_top` / `panel_bottom` | #232a2e / #192024 | face gradient |
| Well | `well` | #11171a | groups |
| Inset | `inset` | #141b1e | combos, toggles, popup |
| Text | `text` / `muted` / `title` | #e6ece9 / #93a19b / #83918b | readouts, labels, engraved titles |
| State | `accent` | #5ee6a8 | value arcs, LEDs, live display trace, lit frame |
| Display | `disp_top` / `disp_bottom` / `disp_caption` | #0f1214 / #050607 / #7b868b | glass |
| + Source A | `src_a` | #ffb454 | Mod A: its ring, shape display, ruler range, chip |
| + Source B | `src_b` | #7aa7ff | Mod B |
| + Source E | `src_e` | #ff7a90 | Filter Env (also its display and the cutoff dot while Env is on) |
| + Source M | `src_m` | #c792ff | Macro 1 (and later macros) |

Colour is reserved by meaning: mint is state of the instrument, a source colour is "this value
is being moved by that source". No other hue exists on the panel. Sources are also coded by a
glyph (A, B, E, M) on the chip so colour is never the only channel.

Duration tokens (all time-based, never frame counts): `t_micro` 100 ms (hover, press, glow),
`t_state` 180 ms (toggles, segments, dim, chevron), `t_panel` 260 ms (page, popover). Easing:
enter = ease-out `cubic-bezier(0.2, 0, 0, 1)`, in egui `emath::easing::cubic_out`; exit =
ease-in `cubic_in`, always shorter than the enter. Data displays are never eased (they show the
value, section 4 group A). No bounce, no overshoot on anything that shows a parameter.

Type: unchanged except the knob value readout is 11 px (was 12) and the hero index readout is
26 px JetBrains Mono. Minimum text stays 11 px.

## 4. Motion inventory

Group A: live data, continuous, 60 Hz target (about 62 Hz achieved), time-based, no easing.

| Element | Trigger | Duration / rate | Easing | What it says |
|---|---|---|---|---|
| Hero lit frame | live Index (base + Mod A + Mod B motion), every frame | continuous | none | which grain plays now; Mod A visibly kicks it each note |
| Hero glow level | amp envelope of the shown oscillator | continuous | none | the note is sounding; 0.55 to 1.0 of the glow passes |
| Hero camera sway | idle | 0.42 rad/s, about plus or minus 13 px of depth skew | sine | depth; off in reduced motion |
| Hero parallax | pointer over the hero | smoother tau 140 ms | exp smoother | depth; off in reduced motion |
| Modulation ring | any frame a source is active on that knob | continuous | none | range arc (50 % alpha), live arc from base to live (95 %), live dot with glow sprite |
| Mod A / Mod B shape | note-on retrigger | one-shot spans the division, loops otherwise | none | where in the cycle each sounding note is: playhead + dot on the curve |
| Amp A, Amp B, Filter Env | note-on / note-off | follows the envelope | none | one playhead per sounding note, older notes fade over 250 ms after their release ends |
| Filter curves | cutoff modulation, every frame | continuous | none | dashed base curve vs live curve and a dot at the live cutoff |
| Shaper / Distortion curve | live signal | continuous | none | a dot rides the transfer curve with the signal |
| Scope | audio ring | 192 points, trigger locked at 2 cycles | none | the output waveform |
| Spectrum | audio ring | 120 log columns, rise instant, fall 0.9 of the scale per s, peak hold 0.22 of the scale per s | none | harmonic content; flips with the scope over tau 60 ms |
| Level meter | output peak | rise instant, fall 24 dB/s, hold 1.1 s then 0.6 of the 48 dB scale per s | none | loudness with ballistics |
| Limiter GR | output peak | attack instant, release 6 dB/s | none | gain reduction |
| Source chip bar | source value | continuous | none | how hard that source is moving right now |
| Note LED | gate | while gate is open | none | a note is held |
| FX displays | live | continuous | none | tap flashes, voice phases, reverb tail playhead, band levels |

Group B: chrome and interaction, triggered, eased.

| Element | Trigger | Duration | Easing | What it says |
|---|---|---|---|---|
| Knob label to value | hover, drag, focus | 90 ms cross-fade | linear | exact value on demand |
| Knob glow | hover, drag | 100 ms | ease-out | this is the control you are about to move |
| Value popup | first pointer movement of a drag | same frame in; hides 400 ms after release, 120 ms fade | ease-out | what you are setting, in units |
| Toggle LED | click | 180 ms | ease-out | state changed |
| Segmented lit segment | click | 180 ms slide | ease-out | which page / mode |
| Menu / combo list | open | 130 ms (fade + 4 px drop) | ease-out | list attached to its anchor |
| Menu / combo list | close | 90 ms | ease-in | |
| Combo chevron | open | 180 ms rotate 180 degrees | ease-out | open state |
| 2D / 3D | click | smoother tau 75 ms (about 220 ms to settle) | exp smoother | frames collapse into the single live frame and back |
| Page SYNTH / FX | click | out 60 ms (fade, up 4 px) then in 120 ms fade + 160 ms rise of 8 px, 10 ms stagger per row (rows by y / 130, capped at 40 ms); total at most 260 ms, first new content at 60 ms | ease-in out, ease-out in | content swapped, order of reading preserved |
| PATCH popover | chip click | in 150 ms (fade, 6 px drop, scale 0.985 to 1), out 90 ms | ease-out / ease-in | a layer above the panel |
| Preset load | prev, next, list | continuous parameters glide, tau 70 ms (about 250 ms to settle); switches and enums snap; the edited dot fades in 180 ms | exp smoother | the whole patch moves to the new one, rings re-form |
| Module dim | On toggled | 180 ms to 45 % | ease-out | module bypassed |
| Chip hover | pointer on a source chip | 140 ms: its targets get a 3 px ring, every other knob drops to 50 % | ease-out | what this source drives |
| Patch drag | drag a chip | target rings 1.5 px dashed, offset pulses 3 to 5 px over 1.1 s; hovered target solid + glow + live preview ring | ease-in-out loop | where it can go, and what it will do |

Reduced motion: Group B durations become 0 (instant state), sway, parallax, glow pulse, page and
popover motion are off; Group A keeps running because it is the instrument's data, but display
refresh drops to 15 Hz snapshots. The setting defaults from the OS (Windows
`SPI_GETCLIENTAREAANIMATION`) and has an editor toggle.

## 5. Components

### 5.1 Hero display (568 x 232)

Glass as DESIGN 3.7 (radius 8 here), fogged 70 px at the top and 38 px at the bottom so overlay
text stays legible. Overlays: A / B tabs, table name (Barlow SemiBold 17), INDEX caption, live
index (26 px mono, accent, 12 px soft glow), "base 40.0" under it, 2D / 3D switch, hint text.

3D view: 32 frames, 256 points each (decimated from the 512-sample cycle), painter's algorithm
back to front. Projection (px, 3D state): front frame centre x 205, width 330, baseline y 176,
amplitude 30; depth steps add 190 x and subtract 104 y over the 32 frames, width shrinks 28 %.
Per frame: opaque fill under the curve in `disp_bottom` (hidden-line removal), then a 0.9 to
1.15 px line whose alpha falls with depth (0.68 at the front to 0.16 at the back) with a
Gaussian lift around the live position. The lit frame is the DSP's frame (linear crossfade of
the two neighbours at the live Index) at its fractional depth, spliced into the draw order so
nearer frames occlude it: translucent frame plane (mint 7 %), gradient fill under the curve
(mint 26 % to 0), then three glow passes (6x, 3x, 1x of a 2.1 px core at 7 % / 17 % / 100 %, all
scaled 0.55 to 1.0 by the amp envelope). A soft radial wash (mint 10 %, radius 190) sits behind
it, and a 30 % x-ray trace of the lit frame is drawn over the nearer frames so its whole shape
stays readable where they hide it. Frames inside the modulation range tint 16 % toward `src_a`. A depth ruler runs along the
front-left edge: a grey tick for the base Index, a mint marker for the live Index, and a 2 px
`src_a` segment for the Mod A range.

2D view: the same code with the frame stack collapsed (one smoother, tau 75 ms): the lit frame
fills 500 x 124, graticule fades in, the two modulation extremes appear as dashed `src_a`
ghosts, neighbours ghost at 30 %. Drag on the hero changes Index (380 px of travel per full
range, Shift = 1/10) and shows the value popup at the pointer.

egui implementation (feasibility section 2): **a GL `PaintCallback`** (`egui_glow::CallbackFn`,
add `egui_glow = "=0.31.1"`), GLSL 150. The 31 background frames are static vertex data
uploaded when the table changes (about 0.7 ms, once); per frame only the lit ribbon (256 points,
about 0.02 ms) and six uniforms (camera skew, depth height, 2D mix, glow level, plane alpha,
highlight depth) change. Cost: 0.05 ms CPU, 0.1 to 0.4 ms GPU, 0 egui vertices. If GL is
refused, the CPU fallback is 32 x 128 points (stride 4) with fills and a three-pass lit frame:
about 16 to 25k vertices, 0.5 ms, no soft ribbons; the look degrades to hairlines but nothing
is lost functionally.

### 5.2 Knob and modulation ring

Dial: 270 degree sweep from 135 degrees; track 2.5 px `track`; value arc 2.5 px `accent`;
cap and indicator as DESIGN 3.3. **No resting glow** (the old 7 px duplicate arc is dropped; it
was about a quarter of every knob's vertices); hover and drag add 7 px at 18 % and 11 px at 6 %.
Ring radius = track radius + 3.9 px, outside the dial box by about 1.5 px (cells already leave
the room). Per active source on that knob:

1. range arc, 2 px, source colour at 50 % alpha: bipolar sources (sine, triangle, ...) span
   base minus to plus amount, unipolar (decay, attack, envelope, macro) span base to base plus amount;
2. live arc, 2 px, source colour at 95 %, from base to the live value;
3. live dot: 2.3 px core in the source colour with a 1 px dark outline, plus a 5 px glow sprite
   (additive, 26 %).

Two sources on one knob (Filter Freq: Env + Mod B) share the ring; the dots are told apart by
colour. A ring is shown only while `|amount| > 0.004` and its routing applies (Osc B rings
vanish when Osc B is off or the target excludes it; Filter B Freq's Env ring needs Filter B Env
on). Amount knobs themselves (Mod A Index, Filter Env Amt, ...) change the ring range live.

Labels: 11 px Barlow caps, +0.04 em. Hover, drag or focus cross-fades to the 11 px mono value.

### 5.3 Shape and envelope displays (Mod A, Mod B, Amp A, Amp B, Filter Env)

Glass radius 4, 8 px inner margin, 1 px `white 6 %` graticule on pixel boundaries, 1 px
`white 12 %` baseline, curve 1.5 px in the colour of the thing it describes (`src_a` for Mod A,
`src_b` for Mod B, `src_e` for the filter envelope, `accent` for amp), under-curve gradient at
20 % to 0, three-pass glow. Segment widths of the ADSR displays use sqrt(time + 0.02) so a 5 ms
attack stays visible next to a 300 ms decay; a notional sustain segment is 0.5 of the width.
Playhead: 1 px white at 50 % vertical line, 2.4 px white dot on the curve, 18 px radial glow
sprite in the display colour. One per sounding voice; voices whose release has ended fade out in
250 ms. Mod A one-shot shows one cycle and holds its end value; periodic modulators show two
cycles.

### 5.4 Filter response (Filter A, Filter B)

Log frequency 20 Hz to 20 kHz across, minus 30 to plus 18 dB up. Dashed grey curve = the knob's
base cutoff; live mint curve + under-fill = the live cutoff (Env + Mod B + wheel). A dot sits on
the live curve at the live cutoff, rose while the filter's Env is on and blue otherwise; a tick
under the axis marks the base. Type is drawn per mode (LP12, BP12, Comb+, Comb-; AM draws flat).
Filter B dims while Routing is Bypass.

### 5.5 Output scope, spectrum, meter

Scope: 192 points, two cycles, triggered at the rising zero (`rr_ui::ring::rising_zero`). Spectrum:
2048-sample Hann FFT from the same ring, 120 log columns 30 Hz to 20 kHz, 66 dB range, smoothing as
section 4 (66 dB range, so a 0.9-per-s fall is about 59 dB/s). Meter: two 5 px bars, mint gradient, peak line white (rose above 0.93). Both read the
audio-side ring and atomics only.

### 5.6 Source chips

20 px high pill, `inset` fill, 1 px stroke of the source colour at 35 % (80 % on hover), glyph
disc 14 px in the source colour with a dark letter, name in 11 px caps, 22 x 4 px activity bar.
Hover lights targets (section 4). Drag: a ghost of the chip follows the pointer; every assignable
knob gets the dashed target ring; the knob under the pointer shows a solid ring, a glow, and a
live preview ring at 50 % amount; the help bar reads "Patch Mod A to Filter A Res". Release
commits (a user-added slot, until the matrix engine of SPEC v2 Track 2 exists this is a prototype
affordance only), leaving a target highlights nothing and cancels. Right-click a knob removes the
rings you added.

### 5.7 Value popup

Pill above the knob: `#0b0f11` fill, 1 px `accent@0.35` stroke, 5 px radius, 12 px mono text, a
7 px notch. Appears on the first pointer movement (same frame), tracks the value while dragging,
hides 400 ms after release. Wheel and key changes show it for 500 ms.

### 5.8 Menus (combo, preset)

DESIGN 3.5 popup with 24 px rows. The preset list shows name left, category right in 11 px caps
`muted`, selected row in `accent`, hover `accent@0.16`; prev / next arrows wrap. Loading glides
(section 4).

### 5.9 PATCH popover

312 x 146 group at the chip, instance label in the title row, three rows of source combo, target
combo, 24 px amount knob, 12 px activity LED (alpha and glow follow `activity01`). A lost source
keeps the persisted name in `muted@0.75` and the row at 60 %. Esc or a click elsewhere closes.

### 5.10 Help bar

28 px, `black@50 %` fill. Idle: "Hover a control" and the gesture hint. Hovering any knob, toggle,
combo, the hero, the scope or a chip shows `NAME` (11 px caps), value (12 px mono, accent),
`> live` value in the source colour when modulated, and a one-line description (12 px Barlow
`muted`). After a preset load it shows the preset's first sentence for 4 s. This replaces
floating tooltips: nothing ever covers another control.

## 6. Depth, shadow and glow as egui steps

Fixed budget: about 48 shadow quads (two per group), 24 glow sprites, 2 panel gradient sprites,
2 rail tiles: about 0.1 ms and 4k vertices (feasibility section 4 measured 24 + 24 + a tile at
0.07 ms and 3.5k). All textures are generated once in the
editor build closure.

1. **Panel light.** Vertical `panel_top` to `panel_bottom` mesh (4 vertices). Add one 128 px
   radial sprite, additive (premultiplied colour, alpha 0), stretched to 760 x 420, centred at
   34 % x and just above the top edge, white at 5.5 %. A second faint mint wash, 520 x 380 at the
   lower right, 2.5 %. Inner 1 px black outline and a 1 px `hi` line along the top edge.
2. **Rails.** Horizontal gradient mesh per rail, plus a repeating 3 px vertical line tile at
   1.8 % white (wrap Repeat, one quad each). Slots and screws as DESIGN 3.1.
3. **Group shadow.** Two small 9-slice textures serve every group size: a soft one (sigma 9) and a
   tight one (sigma 2.5). Soft: group rect inset 8 px, offset (0, +10), black at 60 %. Tight: the
   group rect offset (0, +2), black at 28 %. Then the group fill: `well` rounded rect, a vertex-coloured mesh
   over the top 36 % from white 2.2 % to transparent, a 1 px black 60 % line just inside the top
   edge, a 1 px white 6 % line below the bottom edge.
4. **Glass.** `disp_bottom` fill, top 40 % gradient from `disp_top`, inner top shadow (black 75 %
   over 8 px), 1 px black outline, 1 px `hi` below. The hero adds a 1 px white 3.5 % inner rim.
5. **Glow rule.** Glow only marks live signal: curves (three passes 6x / 3x / 1x at 7 / 17 /
   100 %), mod dots and playheads (5 to 9 px additive sprite), lit LEDs (7 px at 30 %), the
   hero's lit frame. Never on resting chrome. Alpha layers use the source colour premultiplied,
   never white over the panel. Anything brighter than 35 % alpha outside the 1x pass is too much.
6. **Line rule (feasibility).** Every hairline, divider, graticule line and outline has width
   `n / ctx.pixels_per_point()` and sits on a pixel boundary; curves and arcs are 1.5 to 2.5
   physical-px wide at 1x and get egui's 1 px feathering. A 1 pt line at ppp 1.25 or 1.5 is not
   1 px and is a defect.
7. **Dither.** Any custom GL gradient (hero wash) adds egui's interleaved-gradient dither so it
   does not band next to egui's own gradients.

## 7. Interaction rules

| Control | Rule |
|---|---|
| Knob | Vertical drag, 200 px = full range, applied per event so pressing or releasing Shift mid-drag never jumps. Shift = 1/10. Double-click = default. Wheel = 2 % (0.2 % with Shift). Arrow keys = 5 % (0.5 %), Home = default. Hover, drag and focus show the value. Gesture contract is unchanged (`begin_set_parameter` on drag start, set on change, end on release; a reset is begin / set / end in one frame). The popup and arc read the local drag value the same frame, not the host round trip |
| Toggle, combo | As DESIGN 4; Space / Enter activate; focus ring 1.5 px `accent@0.55` |
| Hero | Hover = depth parallax; drag horizontally = Index; click 2D / 3D; A / B tabs; double-click = reset Index |
| Source chip | Hover = highlight targets; drag onto a knob = patch with live preview; release elsewhere = cancel |
| Scope | Click flips scope and spectrum |
| Preset | Arrows wrap; list click loads; edit marks the name with a dot |
| Page | Click or the segmented control; editing state and scroll are not lost |
| Esc | closes menus and the PATCH popover |

## 8. Plumbing the plugin needs (what feeds the live pixels)

The prototype synthesises its own demo data; the plugin must publish the real thing. All
lock-free, none allocate, reader is the GUI thread:

* `rr_ui::ring` (existing) for output samples: scope and spectrum.
* `Meters`-style atomics (existing pattern): L / R peak, limiter gain reduction.
* New `VoiceViz`: 8 voices x {note-on time (sample counter), gate, note, velocity, amp A level,
  amp B level, filter env level, Mod A phase, Mod B phase} as `AtomicU32`, written per block.
  The GUI draws playheads and mod signals from these; it never re-runs the envelopes.
* New `ModLive`: post-modulation normalised value for each ring target (about 16), per block.
* Time: use `ctx.input(|i| i.time)` and `stable_dt` clamped to 50 ms for every smoother; a
  doubled 31 ms frame then costs nothing visible. Never count frames.

## 9. Accessibility

* Reduced motion as in section 4; toggle in the editor, default from the OS.
* Contrast, measured (WCAG ratio): text on well 15.1; muted on well 6.7; title on well 5.5;
  accent on well 11.5; muted on inset 6.5; disp caption on display 5.4; help hint and display
  hints (#7b868b) on display 5.4; source colours on well: amber 10.3, blue 7.6, rose 7.3, violet
  7.7; on display: 11.5, 8.5, 8.2, 8.7; accent on display 12.9. The unfilled track is decorative
  (1.7); state is carried by the value arc (6.8 on the track) and the indicator. The
  range arc at 50 % alpha over the well is about 3.4 and is a secondary cue; the live arc and
  dot carry the meaning at above 7. The old display hint colour (#6c777c, 4.41) was raised to
  #7b868b. New unit tests in `rr-ui::theme` enforce 4.5 for text and 3 for graphics.
* Colour is never the only channel: sources have letters, patch targets have a dashed ring,
  lit states have a label change, the playing frame has a plane and a ruler marker.
* Minimum text 11 px; smallest targets: toggles and combos 24 px high, S knob cells 40 x 48,
  chips 20 px high (below the 24 px control height; open question 5 asks whether to grow them).
* Keyboard: every control is focusable and adjustable (section 7).

## 10. Frame budget (estimates to be replaced by measurements in the first spike)

Baseline from the feasibility report, 1000 x 832, ppp 1: UI + tessellation 2.8 ms mean, 5.4 ms
p99; 98.6k vertices; 0.43 ms GPU. A frame is 16.7 ms.

| Item | Vertices | CPU ms | GPU ms |
|---|---|---|---|
| Today | 98.6k | 2.8 | 0.43 |
| Remove: grain display | -12k | -0.25 | |
| Remove: resting knob glow (about 25 % of each of 76 knobs) | about -21k | -0.40 | |
| Remove: printed readouts, inline PATCH strip | about -6.5k | -0.12 | |
| Add: modulation rings (16 active, about 200 each) | +3.2k | +0.06 | |
| Add: 8 mini displays (about 1.3k each) | +10.4k | +0.20 | |
| Add: scope + spectrum, meter | +3k | +0.06 | |
| Add: shadows, glow sprites, light, rails | +3.5k | +0.07 | |
| Add: header, chips, help bar, hero text | +6k | +0.11 | |
| Add: hero GL callback | 0 | +0.07 | +0.4 |
| Add: FFT 2048 + smoothers + 500 tweens | | +0.08 | |
| **Estimated total** | **about 85k** | **about 2.7 mean, about 5.2 p99** | **about 0.85** |

Hard caps: 98.6k vertices (today), 4.0 ms mean and 8.0 ms p99 CPU, 1.0 ms GPU. Headroom to the
16.7 ms frame stays above 8 ms at p99. The FX page has fewer knobs (about 60) and similar
displays; it is estimated lower.

## 11. Acceptance criteria (measurable in the real plugin)

| # | Criterion | How |
|---|---|---|
| 1 | At least 60 fps mean (stock cadence about 62), p99 interval at most 32 ms, at most 5 % of frames above 20 ms, none above 50 ms, over a 10 s capture with the demo pattern playing | `plugin_loop -- gts gl` |
| 2 | UI + tessellation at most 4.0 ms mean and 8.0 ms p99 on both pages; total vertices at most 98.6k | `frame_bench` and `frame_bench -- fx` |
| 3 | GPU at most 1.0 ms mean; hero callback at most 0.5 ms | `frame_gpu` |
| 4 | Knob drag to repainted arc and popup in the same frame as the pointer event (at most 1 frame); audio-side change to pixels at most 2 frames (33 ms); never above 100 ms | test hook stamping the event and the painted frame |
| 5 | Hover and press feedback first visible within 1 frame, complete within 100 ms | same hook |
| 6 | No line aliasing at ppp 1.0, 1.25, 1.5: every hairline, divider and graticule line resolves to a single full-coverage pixel row or column (coverage {255}); no two-row 75 % pair; curves at least 1.5 px | extend `motion_gl -- aa` with panel, display and hero crops at each ppp |
| 7 | Time-based: positions of playheads, rings and the lit frame at time t agree within 1 px whether the editor runs at 30 or 62 fps | run with a throttled `update` |
| 8 | Hero is at least 35 % of window width and 25 % of height | unit test on the layout constants (568 x 232) |
| 9 | Page switch finishes within 260 ms, first new content within 60 ms, no frame above 33 ms during it | `plugin_loop` with a scripted switch |
| 10 | Preset load settles within 300 ms; switches snap in one frame; no audio-side change | script |
| 11 | Reduced motion: no sway, parallax, glow pulse, page or popover motion, zero-length chrome transitions; data displays at 15 Hz | two captures diffed outside the data displays |
| 12 | Contrast: text at least 4.5, graphics at least 3 for the live marks, per the table in section 9 | `rr-ui::theme` tests |
| 13 | Idle (no notes, no gesture): at most today's 2.8 ms mean (egui cannot skip a frame under the stock stack, so idle is a cost cap, not zero repaints) | `frame_bench` with silent state |
| 14 | In-host: the same capture as 1 inside a DAW with the transport running; no frame above 50 ms | manual capture; untested today (feasibility notes the shared GUI thread) |

## 12. Known differences between the prototype and the plugin

* The prototype synthesises tables, envelopes, modulators and the output in JS from the real
  parameter ids and ranges; the plugin reads the DSP's own state (section 8).
* Frequency knobs use a pure log map in the prototype; the plugin keeps its skew factors, so ring
  spans for Filter Freq are in octaves over the log range and need the plugin's own mapping.
* The prototype ignores keyboard tracking in the filter curve and does not model every comb and
  AM response; the plugin should draw the filter's actual transfer.
* Source chips beyond Mod A, Mod B and the Filter Env, and user-added slots, depend on the
  modulation matrix of SPEC v2 Track 2 (Macro 1 is shown because the preset format already
  carries macro labels). In v1 the fixed routes are Mod A / B amounts, Filter Env Amt and the
  wheel / velocity amounts.
* Canvas strokes in the prototype are fractionally scaled; the line rule (section 6) is the
  real requirement.

## 13. Open questions for the owner

1. Resting values hidden (label swaps to value on hover, drag and in the help bar) versus always
   printed. The hero size depends on it; "Always show values" mode costs nothing in layout.
2. PATCH as a popover under a chip with live LEDs, or keep a permanent strip and give up about
   40 px of hero height.
3. OK to take the `egui_glow` dependency and a GL callback for the hero (0.05 ms CPU), with the
   CPU fallback documented above?
4. UI scale steps still need the 31-line egui-baseview patch and a nih_plug_egui fork
   (feasibility section 6); at 150 % the panel is 1500 x 1248, so a 75 % step matters on laptops.
5. Source chips are 20 px high (the dense row's limit). Grow to 24 px and drop the hint text?
6. One shared hero with A / B tabs, or two smaller always-visible displays inside Osc A and Osc B?
