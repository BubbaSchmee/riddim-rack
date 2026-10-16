# Motion and GPU feasibility for the Riddim Rack editors

Engineering answer to: can the egui editors (nih_plug_egui 0.0.0 -> egui-baseview ec70c3f -> egui 0.31.1 -> egui_glow, OpenGL 3.2 core)
carry an animated, premium look? Everything below was run, not guessed, unless it says **(source only)** or **(untested)**.

Machine for all numbers: Windows 11, Ryzen 7 8845HS, GL context created on the **Radeon 780M iGPU** ("GL 3.2.0 Core Profile Context 24.10", the
same driver path a DAW gets; an RTX 4070 laptop GPU is also present), display 144 Hz. Ableton Live 12 was running in the background, so
p99 and max values are noisy (an individual run sometimes came out 5x slower); medians and means from quiet runs are what is quoted.

## How to reproduce (all examples are kept in the repo)

Shell: `export PATH="$USERPROFILE/.cargo/bin:$PATH" CARGO_TARGET_DIR=/d/cargo-target/riddim-rack CARGO_INCREMENTAL=0`

| Question | Command |
|---|---|
| CPU budget of the real GTS draw (headless, 600 frames) | `cargo run -p gts --release --example frame_bench [-- fx] [-- ppp=1.5]` |
| Real window path of the plugin (egui-baseview, exact GlConfig), frame cadence, GL view, textures, MSAA | `cargo run -p gts --release --example plugin_loop -- gts gl fx [vsync=0] [repaint=every\|after33\|after16] [tbp] [msaa=4] [shot]` |
| GPU time of the whole GTS editor, uncapped (eframe) | `cargo run -p gts --release --example frame_gpu [-- gl] [-- fx] [-- ppp=1.5]` |
| GL 3D wavetable, CPU fallback, textures: per-scene tables | `cargo run -p rr-ui --release --example motion_gl -- headless\|bench\|shot\|aa ppp=1.25\|live` |
| Animation cost, ring read cost, widget cost, zoom behaviour | `cargo run -p rr-ui --release --example anim_probe` |
| Lock-free scope ring unit tests (incl. a concurrent torn-read test) | `cargo test -p rr-ui --release ring` |
| User zoom through a patched egui-baseview | copy the egui-baseview checkout (`~/.cargo/git/checkouts/egui-baseview-*/ec70c3f`), `git apply docs/design/egui-baseview-zoom.patch` in it, then `cargo run -p gts --release --example plugin_loop --features zoom-patch --config "patch.'https://github.com/BillyDM/egui-baseview.git'.egui-baseview.path='<that copy>'" -- gts gl shot zoom=1.5`; `--config patch` rewrites `Cargo.lock`, restore it afterwards |

Shared prototype code: `crates/rr-ui/examples/support/wavegl.rs` (raw-GL wavetable view and its CPU twin), `support/fx_tex.rs` (generated textures),
`crates/rr-ui/src/ring.rs` (the one piece that is library code). Screenshots: `docs/design/motion-proto.png` (eframe gallery),
`motion-proto-plugin.png` (real GTS editor with GL view and texture effects, through the plugin's window path), `motion-proto-zoom.png` (1.5x).

---

## 1. Frame rate: 60 Hz inside the plugin

**Verdict: 60 Hz works today (measured 62 fps). The hard ceiling is about 64 Hz. 120/144 Hz is not reachable without patching baseview.**

How a frame is scheduled on Windows (source, then confirmed by measurement):

- baseview `win/window.rs:767`: `SetTimer(hwnd, WIN_FRAME_TIMER, 15, None)`. Every `WM_TIMER` calls `on_frame`. That timer is the only clock. Windows
  quantizes it to the 15.6 ms system tick, so frames come at about 64 Hz at best.
- egui-baseview `window.rs on_frame`: runs the user `update` closure on **every tick**, then renders only if `repaint_delay == 0` or its stored
  deadline has passed. `swap_buffers` is inside `render`.
- nih_plug_egui `editor.rs`: opens the GL context with `vsync: true`, `samples: None`, `depth_bits: 24`, `stencil_bits: 8`, `srgb: true`, GL 3.2 core, and
  **calls `egui_ctx.request_repaint()` unconditionally before every update** ("most plugin GUIs have meters").

Measured with `plugin_loop` (egui-baseview opened with that exact GlConfig, interval taken inside a PaintCallback just before swap, 570 frames):

| Config | painted-frame interval | fps |
|---|---|---|
| trivial panel, vsync on | mean 16.3, p50 15.6, p99 30.9 ms | 61.3 |
| real GTS editor, vsync on | mean 16.0, p50 15.6, p99 30.2 ms, 2.8 % of frames > 20 ms | 62.5 |
| GTS + GL 3D view | mean 15.9, p50 15.6, p99 30.7 ms | 62.8 |
| GTS, vsync **off** | mean 16.0, p50 15.7 ms | 62.5 (identical: the timer, not vsync, paces it) |
| GTS, `timeBeginPeriod(1)` | mean 16.1, p50 15.7 ms | 62.0 (no change) |
| GTS at scale 1.5 | mean 16.1 ms | 62.0 |
| GTS, only `request_repaint_after(16 ms)` | mean 16.0 ms | 62.5 (egui subtracts the predicted dt, so it degenerates to every tick) |
| GTS, only `request_repaint_after(33 ms)` | **mean 105 ms** | **9.5** (egui-baseview moves the deadline forward on every tick, so it starves) |

Consequences:

- `request_repaint_after(33 ms)` at the end of `crates/gts/src/editor.rs create()` does nothing: nih_plug_egui's unconditional `request_repaint()` already
  makes the editor repaint on every timer tick (about 64 Hz, not 30 Hz). It is also the only thing keeping the window from starving, because any
  requested delay above roughly 32 ms is starved by egui-baseview. **Throttle by skipping work inside `update`, never by `request_repaint_after`.**
- About 2 to 5 % of frames are doubled (about 31 ms) because a WM_TIMER tick is occasionally skipped. Motion that is time-based (use `input.time`, not a
  frame counter) hides this; frame-counted animation visibly stutters.
- `update` runs on every tick whether or not it paints, and runs on the **host's GUI thread** (this was measured standalone; inside a DAW the host's own UI
  work shares that thread, **untested**).
- To go beyond 64 Hz, baseview's timer would need replacing (shorter `SetTimer`, or a vsync thread posting a message). **Untested.**

### CPU frame time of the CURRENT GTS editor (`frame_bench`, headless, 1000x832, 600 frames after 30 warm-up, release)

| Part | mean | p50 | p99 | max |
|---|---|---|---|---|
| UI pass (`ctx.run` of `editor::draw`) | 0.98 ms | 0.91 | 1.92 | 2.53 |
| Tessellation (`ctx.tessellate`) | 1.80 ms | 1.64 | 3.74 | 4.80 |
| **UI + tessellation** | **2.77 ms** | 2.61 | **5.39** | 5.86 |
| FX page | 2.69 ms | 2.54 | 4.77 | 7.0 |
| pointer sweeping across the panel | 2.83 ms | 2.78 | 4.83 | 6.8 |
| ppp 1.5 / ppp 2.0 | 2.67 / 3.36 ms | | 4.76 / 5.99 | |

Mesh handed to the GL painter per frame: **31 primitives, 98,554 vertices, 420,525 indices** (about 1,100 vertices per knob, see section 5). In the
plugin window path the `update` closure itself measured 1.0 ms mean (UI only; tessellation happens in render). GPU side
(`frame_gpu`, GL timer query around the entire egui frame): **0.43 ms of GPU time**, and the whole frame (build + tessellate + driver + swap) uncapped
takes **7.2 ms median (about 138 fps capacity)** on the iGPU.

**Budget at 60 Hz (16.7 ms): the editor uses about 2.8 ms CPU (5.4 at p99) and 0.4 ms GPU. Roughly 11 ms of CPU headroom at p99 is available for motion.**

## 2. Custom GPU rendering (shaders inside an egui rect)

**Verdict: yes, with one added dependency. It was built and runs in the plugin's own window path.**

- egui-baseview's OpenGL `Renderer` is a plain `egui_glow::Painter`. Its `paint_primitives` dispatches every `Primitive::Callback` to an
  `egui_glow::CallbackFn`, sets `glViewport` to the callback rect (scissor = clip rect), and afterwards re-runs `prepare_painting` (program, VAO, blend,
  scissor, depth-test-off restored). So `egui::PaintCallback { rect, callback: Arc::new(CallbackFn::new(|info, painter| ...)) }` works.
- nih_plug_egui does **not** re-export `egui_glow` (only `egui`). Add `egui_glow = { version = "=0.31.1", default-features = false }` to the crate
  that draws the callback (it is already in Cargo.lock; the pin keeps one copy). Raw GL types come from `egui_glow::glow`. `painter.gl()` gives the context.
- The context is **GL 3.2 core** (reported as "3.2.0 Core Profile Context" on this machine): use `#version 150`, VAOs are mandatory, depth 24 and stencil 8
  exist, there is no MSAA. `GL_ARB_timer_query` is available (used for the GPU numbers).
- egui_glow disables `FRAMEBUFFER_SRGB` and blends **premultiplied, in gamma space** (`ONE, ONE_MINUS_SRC_ALPHA`), and dithers its output. A custom
  shader should write premultiplied gamma-space colour and add the same interleaved-gradient dither (done in `wavegl.rs`) or dark gradients band next to egui's.
- GL objects must be created lazily inside the callback and kept in per-editor state (not a `static`): a reopened editor gets a **new context**. They are
  not deleted explicitly (the context dies with the window); deleting from `Drop` could run with no context current.

### Prototype: 32 layers x 256 points, depth fade, glow, hidden-line fill (`support/wavegl.rs`)

Thick anti-aliased ribbons: each segment is a quad expanded in screen space in the vertex shader, coverage from distance in the fragment shader
(8,160 quads, 32,640 vertices, 96 draw calls per frame: opaque fill, additive soft ribbon for glow, crisp ribbon, per layer, back to front).

| Scene (640x400 view) | CPU in callback | GPU (timer query) | whole-frame interval, vsync off |
|---|---|---|---|
| GL, vertices uploaded once | 0.05 ms | 0.25 to 0.33 ms mean (p50 0.19) | within noise of the empty-window floor (about 2 ms median) |
| GL, glow pass off | 0.03 ms | 0.10 ms | same |
| GL, **re-upload all vertices every frame** (worst case: waves morphing every frame) | 0.69 to 0.81 ms (incl. building 1 MB) | 0.23 ms | same |
| inside the real GTS editor, plugin path | 0.07 ms | 0.42 ms mean, p50 0.16 | 7.2 -> 7.8 ms uncapped; cadence still 62 fps |

So a 3D wavetable with glow costs well under 1 ms and runs at 60 fps with room to spare. Camera motion, playhead highlight and time-based glow are
uniforms: free. Per-frame vertex rebuild is only needed when the wave data itself changes (about 0.7 ms).

**Fallback (CPU-painted projection with egui shapes, no GL)**, same scene: `Shape::line` per layer (1.3 px feathered) plus an opaque mesh strip for hidden-line fill:

| | build | tessellate | total mean | p99 | vertices |
|---|---|---|---|---|---|
| 32 x 256 lines only | 0.09 ms | 0.86 ms | **0.94 ms** | 1.8 ms | 32,816 |
| 32 x 256 lines + fills | 0.24 ms | 0.73 ms | **0.96 ms** | 2.2 ms | 49,200 |

It is cheap in CPU too, but it is hairline-only (no thick/glowing ribbons without more geometry), and it adds about one third of the editor's current vertex count.

Not built: a true multi-pass bloom through an FBO inside the callback. It is expected to work (`Painter::intermediate_fbo()` is `None`, so restore
framebuffer 0 afterwards) but is **untested**; the additive soft-ribbon pass already gives a convincing glow for line art without any FBO.

## 3. Anti-aliasing quality

**Verdict: egui's 1 px feathering is good and axis-aligned lines snap to the pixel grid. MSAA is not available through nih_plug_egui and is not worth enabling. The one real rule is: give widths in physical pixels (`n / pixels_per_point`).**

Numbers from `motion_gl -- aa ppp=...` (screenshot pixel values, white line on black, coverage down a column; 255 = full pixel):

| Stroke (points) | ppp 1.0 | ppp 1.25 | ppp 1.5 |
|---|---|---|---|
| 1.0 wide, any y (20.0, 40.5, 60.3) | `[255]` crisp 1 px | `[32, 255, 32]` (1 px + 12 % halo) | `[191, 191]` **blurry, two 75 % rows** |
| 0.5 wide | one 50 % pixel | one 62 % pixel | one 75 % pixel |
| 1.5 wide | `[191, 191]` | `[239, 239]` | `[31, 255, 255, 31]` |
| 2.0 wide | `[255, 255]` | `[64, 255, 255, 64]` | `[255, 255, 255]` (3 px) |
| **`1/ppp` wide (1 physical px), any y** | `[255]` | `[255]` | `[255]` |
| **`2/ppp` wide** | `[255, 255]` | `[255, 255]` | `[255, 255]` |

- `TessellationOptions::round_line_segments_to_pixels` and `round_rects_to_pixels` are on by default: horizontal/vertical lines and rect strokes are
  snapped to whole pixels (y = 20.0 and y = 40.5 give identical output at ppp 1). A **1 pt stroke at a fractional scale is therefore not 1 px**
  (1.25 or 1.5 px, blurred). Widths expressed as `k / ui.ctx().pixels_per_point()` are exact (verified at 1.25, 1.5, 2.0). Every hairline and divider in
  rr-ui should use that.
- Diagonal lines and arcs get 1 px feathering only (no snapping): a 1.5 pt, 270 degree knob arc and a 1 pt circle came out smooth (93, 105 and 140 distinct
  grey levels across the stroke at ppp 1, 1.25, 1.5; no stair-steps in the 5x crop). Slight brightness wobble on a 1 px curve is inherent to feathering.
- Text is snapped to the pixel grid (`round_text_to_pixels`) and is re-rasterized at the real ppp, so it stays crisp at any scale **as long as ppp is
  the real one** (see section 6).
- **MSAA:** nih_plug_egui hard-codes `samples: None` (verified `GL_SAMPLES 0`). baseview itself supports it (`WGL_SAMPLES_ARB`), so enabling it is a
  one-line fork. Measured with the plugin's exact config and `msaa=N`: **4x -> 62 fps becomes 46 fps (p99 112 ms); 8x -> 14 fps** on the iGPU. egui already
  feathers, and custom GL passes use analytic edge coverage, so there is nothing to gain. Do not enable it.

## 4. Textures: noise, brushed metal, glow sprites, soft shadows

**Verdict: yes, cheap, and verified in the plugin window path (`plugin_loop -- gts fx shot`).**

- `ctx.load_texture(name, ColorImage, TextureOptions)` works unchanged in nih_plug_egui (uploads ride `textures_delta` into `Painter::set_texture`).
  `TextureOptions.wrap_mode = Repeat` is supported; max side comes from the GL context. Images are **sRGB bytes, premultiplied alpha, blended in gamma space**.
- Built in `support/fx_tex.rs`:
  - **soft drop shadow**: a 72x72 blurred rounded rect (gaussian, sigma 7), drawn as a **9-slice** (16 vertices, 9 quads), so one texture serves any rect size;
  - **radial glow sprite** (128x128, `(1 - r^2)^3` falloff, zero at the rim): drawn with a vertex tint of **premultiplied colour and alpha 0**, which blends
    **additively** (egui's blend is `src + dst * (1 - src.a)`), so LED and knob glows add light instead of dimming;
  - **brushed-metal noise** (256x256, row-wise streaks blurred with wrap-around, alpha 3 to 31): tiled over a panel as one quad via uv > 1.
- Cost: generating the textures is a one-time **shadow 0.35 to 0.65 ms, glow 0.06 to 0.12 ms, noise 0.8 to 1.2 ms**. Per frame, 24 textured shadows + 24 additive glow
  sprites + a full-panel noise quad = **0.074 ms build + tessellate, 3,476 vertices, 98 meshes**. egui's built-in `Shadow::as_shape` for the same 24 panels is
  0.085 ms, 7,844 vertices, 3 meshes: **both are negligible; the textured one looks better and scales with 9-slice, the built-in one needs no assets.**
- Limits: 8-bit alpha, so a very wide soft shadow banding is possible (the painter's dithering softens it); shadows are baked black (tint only alpha);
  mipmaps are a glow-only option (`mipmap_mode`), not needed here.

## 5. Animation plumbing

**Verdict: yes. Everything needed exists or is a few lines; measured costs are noise.**

- egui tweens: `ctx.animate_bool_with_time_and_easing(id, target, secs, easing)` and `animate_value_with_time(id, target, secs)` (linear). Easings are plain
  `fn(f32) -> f32` in `egui::emath::easing` (`cubic_out`, `quadratic_out`, ...). Default `style.animation_time` is 0.083 s. They request a repaint while in flight
  (irrelevant here: the window repaints every tick anyway). They advance on `input.time` deltas, so they are **frame-rate independent** even with the doubled frames
  from section 1. Verified: a 0.2 s tween steps 0.00, 0.08, 0.17, 0.25 ... 1.00 across 60 Hz frames (cubic_out 0.23, 0.42, 0.58, 0.70, 0.80 ...).
- There is no built-in spring or glide. A frame-rate-independent exponential smoother in `ctx.data` (`v += (target - v) * (1 - exp(-rate * dt))`, in
  `anim_probe.rs`) approaches 1.0 as 0.34, 0.57, 0.71, 0.81, 0.88 at rate 25/s, 60 Hz.
- Cost: **500 animated widgets per frame = 0.016 ms (tween) / 0.014 ms (smoother)**, about 30 ns each. Per-widget animation state is free; do not budget for it.
- Reading audio-thread values at 60 Hz without locks: the pattern already used by GTS `Meters` (`AtomicU32` holding f32 bits, `swap(0)` in the GUI) is
  correct and free. For waveforms there is now **`rr_ui::ring`** (`crates/rr-ui/src/ring.rs`): a single-producer/single-consumer overwrite ring of
  `AtomicU32` slots. `RingWriter::push(&[f32])` is wait-free and allocation-free (compatible with `assert_process_allocs`); `RingReader::latest(&mut [f32; 2048])`
  copies the most recent samples oldest-first and detects being lapped (retries, then reports failure) so a returned window is never torn. Window limit: half the
  capacity. `rising_zero()` finds a trigger point for a steady scope trace. Tests: wraparound, trigger, and a concurrent writer/reader run that asserts every
  returned window is strictly consecutive (passes). Measured **2.2 to 2.7 us per 2048-sample read** with a saturating writer thread (20,000 of 20,000 reads succeeded).
  Create one with `ring(8192)`: writer in the plugin's audio state, reader cloned into the editor.
- What one rr-ui widget emits (UI build + tessellation, upper bounds from `anim_probe`): **knob about 1,140 vertices**, `led_toggle` 384, the 300x140 grain display
  with two ghosts 12,030 vertices. Vertex count, not shape count, is what costs: tessellation is 1.8 ms for 98,554 vertices (about 18 ns/vertex).

## 6. Scaling and resizing

**Verdict: nothing in the stock stack gives a user zoom or drag-resize. A 30-line egui-baseview patch (validated) gives crisp zoom; a nih_plug_egui fork is needed to drive it from the plugin.**

What the stock stack does (source, plus one headless run):

- **DPI:** nih_plug_egui initializes `scaling_factor` to `Some(1.0)` on Windows/Linux, so baseview uses `WindowScalePolicy::ScaleFactor(1.0)`. It changes
  only if the host calls `IPlugViewContentScaleSupport::set_scale_factor` **before** the editor opens (`set_scale_factor` returns false once open). Ableton
  does not call it (nih-plug's own TODO says so), so there the editor is 1000x832 physical pixels: small and un-scaled on a HiDPI display. macOS uses the OS scale.
- **Host-driven resize is refused:** VST3 `can_resize` returns `kResultFalse`, CLAP `can_resize` returns false, `on_size` rejects any other size (all marked TODO in nih-plug).
- **Plugin-driven resize:** `EguiState::set_requested_size` is private; the only caller is `ResizableWindow` (a drag corner), which asks the host via
  `context.request_resize()` and then `queue.resize(PhySize(new))`. That gives the layout **more room, not a larger picture** (egui lays out in points), and it passes a
  logical size as `PhySize`, so it is only exact at scale 1.0.
- **`ctx.set_pixels_per_point` / `set_zoom_factor` do not zoom** under egui-baseview. Probe: with zoom 1.25 egui's `pixels_per_point()` becomes 1.25 but the backend
  keeps `screen_rect` at 1000x832 points and the painter multiplies by its own native ppp (1.0), so the rightmost vertex stays at 1000.5 pt: the picture is unchanged.
  (eframe zoom works because egui-winit divides the screen rect by it; egui-baseview does not.)

The fix (`docs/design/egui-baseview-zoom.patch`, applies to egui-baseview rev ec70c3f, about 30 changed lines in `window.rs`): `Queue::zoom(f32)` stores a user
zoom; the effective ppp becomes `native * zoom` for the screen rect, the tessellation/paint scale and egui's `set_zoom_factor`; mouse positions are divided by the zoom.
The caller also sends `ViewportCommand::InnerSize(base * zoom)`. **Verified** with `plugin_loop -- zoom=1.5`: 1500x1248 window, the full GTS editor re-laid out and
re-rasterized crisp (text, knobs, GL view), cadence unchanged (61.9 fps), GL callback unchanged. **Untested:** mouse hit-testing at zoom (the code divides logical position
by zoom), host resize round-trip, and persisting the chosen step.

Remaining work for fixed steps 75/100/125/150 %: fork or `[patch]` nih_plug_egui to make `set_requested_size` public, keep the zoom in `EguiState` (persisted), and fix the
logical-versus-physical `PhySize` in its resize path. At 100 % the editor is 1000x832, so 150 % is 1500x1248, which does not fit a 1080p screen: the 75 % step (750x624) matters on laptops.
Render cost at higher scale is flat: tessellation at ppp 2.0 is 3.4 ms versus 2.8 ms at 1.0 (headless).

---

## Constraints for the designer

**Cheap (spend freely; numbers are per frame at 60 Hz, 16.7 ms budget)**
- Time-based tweens and smoothers on every widget: 500 of them cost 0.015 ms. Use `input.time`, never frame counts.
- Textured effects: drop shadows (9-slice), additive glow sprites, brushed/noise tile: 24 + 24 + a tile cost 0.07 ms and about 3,500 vertices. Generate once at build (about 2 ms total).
- A raw-GL 3D wavetable (32 x 256, depth fade, thick AA ribbons, soft glow, hidden-line fill) in any rect: 0.1 to 0.4 ms GPU, 0.05 ms CPU; camera, playhead, colour, fade all free as uniforms.
- Reading audio data: atomics for scalars (free), `rr_ui::ring` for waveforms (2.5 us per 2048-sample read).
- Thin lines and arcs with egui's feathering are crisp when the width is `n / pixels_per_point`.

**Moderate (budget deliberately)**
- Total emitted vertices: current editor 98.6k verts = 2.8 ms CPU; knob about 1,140 verts, grain display 12k. Every extra 10k vertices is about 0.2 ms of tessellation. Glows/arcs built from many segments are where it goes; sprites are far cheaper than geometry.
- Rebuilding the GL wave vertices every frame (morphing waves): +0.7 ms CPU. Upload only when the data changes.
- CPU fallback 3D view (32 x 256 egui shapes): about 1 ms and +33k to +49k vertices; hairline only, no real glow.
- A UI zoom step: needs the egui-baseview patch plus a nih_plug_egui fork (small, validated).

**Expensive or hazardous**
- MSAA: 4x costs 25 % of frame rate on the iGPU (62 -> 46 fps, p99 112 ms), 8x 14 fps. Not available through nih_plug_egui anyway. Do not plan on it.
- Per-frame heavy CPU art (large particle meshes, per-pixel work in egui shapes): 100k+ extra vertices would double the frame cost; do it in a shader instead.
- `request_repaint_after(d)` for throttling: any d beyond about 32 ms starves the window (9.5 fps). Throttle inside `update`.
- GL state: never bind your own FBO/program without restoring; the painter restores program, VAO, blend, scissor, depth-off, **not** the framebuffer. Resources are per-context.

**Impossible with the stock stack**
- More than about 64 fps (SetTimer-driven frames; a baseview patch could change it, untested). Design motion for 60 Hz, 16 ms frames, with an occasional 31 ms frame.
- Host or user drag-resize through the host (`can_resize` is false in nih-plug VST3/CLAP); user zoom or `set_pixels_per_point` without the patch.
- Automatic HiDPI crispness on hosts that never call `set_scale_factor` (Ableton): the editor stays at 1.0 until the host or a fork supplies a scale.
- A custom framebuffer format, MSAA, or WGL features not exposed by `GlConfig` (it is hard-coded in nih_plug_egui).
- Blend modes other than egui's premultiplied alpha and the additive trick (alpha 0 premultiplied tint) from plain egui shapes; anything else (multiply, screen) needs a GL callback.
- GL 3.3+ or compute shaders: the context is 3.2 core (GLSL 150, no compute, no shader storage).

**Design rules that follow**
1. Hairlines, dividers, outlines: width = `1.0 / ctx.pixels_per_point()` (or `2.0 / ...`), snapped by egui; never a bare `1.0`.
2. Time-based motion at 60 Hz with a 31 ms worst case; ease with `cubic_out` or the smoother, durations 80 to 200 ms.
3. Atmosphere (shadow, glow, grain) from generated textures; hero visuals (3D wavetable, scope trace) from a GL callback with analytic AA and additive glow.
4. Keep the egui vertex count at or below today's (about 100k) and move new heavy visuals to the GPU.
5. Assume 1.0 scale on Windows until the zoom patch lands; design the layout to be legible at 1000x832 physical pixels.
