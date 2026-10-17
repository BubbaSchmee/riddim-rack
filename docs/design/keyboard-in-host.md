# Text input and keys inside a host (SPEC v2 Track 6)

The preset browser needs real text input: a search box, and Save As / Rename fields. In a DAW the
plugin window is a **child** window of the host's, and Windows delivers key messages to the window
that owns the keyboard focus. Clicking a child window does not give it the focus, so a plain
baseview + egui window never sees the keys (egui-baseview issue 20).

## What the stack offers

`egui-baseview` (rev `ec70c3f`, the one `nih_plug_egui` pins) has a Cargo feature,
`windows_keyboard_workaround`. At the end of every frame it checks the egui `PlatformOutput`:

```rust
if !platform_output.events.is_empty() || platform_output.ime.is_some() {
    window.focus();            // baseview: SetFocus(hwnd)
}
```

egui reports an output event when a widget that calls `widget_info` is clicked, gains focus or
changes (our own widgets are drawn with `ui.interact` and report none), and it sets `ime` on every
frame in which a `TextEdit` has focus. So the plugin window takes the keyboard when a text field
is clicked and keeps it while the field has focus, and at no other time: opening the browser or
clicking a preset does not steal anything from the host.

`nih_plug_egui` does not pass the feature through. Cargo features are additive per package, so
`crates/gts/Cargo.toml` names `egui-baseview` once more as a direct dependency, at the **same
revision**, with `default-features = false, features = ["windows_keyboard_workaround"]`. The
unified `egui-baseview` that `nih_plug_egui` links then has the feature on (checked with
`cargo tree -p gts -e features -i egui-baseview`). No source change, no patched copy.

## Evidence

`cargo run -p gts --release --example plugin_loop -- typing` (source: `examples/support/typing.rs`)
opens the plugin window through `egui_baseview::EguiWindow::open_parented` with the exact GL config
`nih_plug_egui` uses, as a child of a borderless parent window that owns the keyboard focus the way
a host's main window does, and runs a Win32 message loop the way a host does. A script posts mouse
messages to the child and **key messages to whichever window `GetFocus()` reports**, which is where
Windows would deliver a real key press, then reads the browser state back. With the feature on:

```
  PASS before any click the host window owns the keyboard
  PASS a click on the preset name opens the browser
  PASS opening the browser leaves the keyboard with the host (focus: the host window)
  PASS clicking the search box moves the keyboard focus to the plugin window (focus: the plugin window)
  PASS typed "wub" reaches the search box (box shows "wub", list narrowed to 6 presets)
  PASS Down arrow moves the selection in the list and loads it (selection Some("User/Bass/Dark Wub 00"))
  PASS Esc clears the search first and keeps the browser open
  PASS a second Esc closes the browser
typing check: PASS (8 of 8 steps)
```

The same build with the feature removed (control run, 2026-10-17) fails exactly where the focus
matters: the click leaves the focus on the host window, so the typed letters and the arrow key go
to the host and the box stays empty (4 of 8 steps pass: the ones that need no keyboard,
plus Esc on an empty box). `plugin_loop -- typing manual` leaves the window open for 60 s for a human to type.

Headless coverage that needs no window: `tests/browser.rs` drives the real `draw` with egui text
and key events (typing, arrows, Enter, Esc, dialogs, rename, delete).

## What it does not prove

* Ableton Live itself. Live owns its own keyboard handling on top of the Windows rules, and may
  take some keys before the plugin window sees them (Space is the likely one while a text field is
  not focused). `tools/CHECKLIST-gts.md` item 24 and the "Preset browser" line list what to try in
  Live and report. If a key is taken by Live while a text field is focused, that is a host
  limitation, to be written down from that run.
* macOS and Linux: the workaround is Windows only (the feature is a no-op there); this machine is
  Windows.

## Behaviour to know

* The plugin window keeps the keyboard until the user clicks elsewhere in the host. While a text
  field has focus, `SetFocus` is called every frame; on an already focused window it does nothing.
* Esc: egui drops the focus of a text field when Escape is pressed, before the browser sees the key.
  The browser therefore remembers whether the search box had focus at the end of the previous frame
  to decide between "clear the search" and "close".
* A dialog's first frame is an invisible sizing pass in which egui discards a focus request, so
  `take_focus` repeats its request until the field reports the focus.
