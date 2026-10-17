//! `plugin_loop -- typing`: does text typed on a real keyboard reach the preset browser's search box
//! through the plugin's window path inside a host-like parent window?
//!
//! The plugin window in a DAW is a CHILD of the host's window, and Windows sends key messages to the
//! window that owns the keyboard focus, which clicking a child does not give it (egui-baseview issue
//! 20). This opens the exact plugin window path (baseview + egui-baseview with the same GlConfig,
//! `open_parented`, a message loop the way a host pumps it) under a borderless parent that holds
//! the focus like the host's main window, then drives it with posted Win32 messages and reads the
//! result back. Keys are posted to whatever `GetFocus()` reports at that moment, exactly where
//! Windows would deliver a real key press, so the check passes only if the window path moved the
//! focus to the plugin window when the search box was clicked (the `windows_keyboard_workaround`
//! feature, enabled in crates/gts/Cargo.toml) and the keys then arrived as egui text and key events.
//!
//!   cargo run -p gts --release --example plugin_loop -- typing
//!   cargo run -p gts --release --example plugin_loop -- typing manual     (waits 60 s for you to type)
//!
//! Prints one PASS / FAIL line per step and exits non-zero on any FAIL.
#![allow(dead_code)]

#[cfg(not(windows))]
pub fn run(_manual: bool) -> bool {
    println!("typing: the keyboard-focus check is Windows only");
    true
}

#[cfg(windows)]
pub use win::run;

#[cfg(windows)]
mod win {
    use baseview::gl::GlConfig;
    use baseview::{Size, WindowOpenOptions, WindowScalePolicy};
    use eframe::egui;
    use egui_baseview::{EguiWindow, GraphicsConfig};
    use gts::dsp::tables::Tables;
    use gts::editor::{draw, Browser, BusView, LiveView, WINDOW};
    use raw_window_handle::{HasRawWindowHandle, RawWindowHandle, Win32WindowHandle};
    use std::ffi::c_void;
    use std::sync::{Arc, Mutex};

    type Hwnd = *mut c_void;

    #[repr(C)]
    struct Msg {
        hwnd: Hwnd,
        message: u32,
        wparam: usize,
        lparam: isize,
        time: u32,
        pt: [i32; 2],
    }

    #[link(name = "user32")]
    extern "system" {
        fn CreateWindowExW(ex: u32, class: *const u16, name: *const u16, style: u32, x: i32, y: i32, w: i32, h: i32, parent: Hwnd, menu: *mut c_void, inst: *mut c_void, param: *mut c_void) -> Hwnd;
        fn DestroyWindow(h: Hwnd) -> i32;
        fn ShowWindow(h: Hwnd, cmd: i32) -> i32;
        fn SetFocus(h: Hwnd) -> Hwnd;
        fn GetFocus() -> Hwnd;
        fn PostMessageW(h: Hwnd, msg: u32, w: usize, l: isize) -> i32;
        fn FindWindowExW(parent: Hwnd, after: Hwnd, class: *const u16, title: *const u16) -> Hwnd;
        fn GetMessageW(msg: *mut Msg, h: Hwnd, min: u32, max: u32) -> i32;
        fn DispatchMessageW(msg: *const Msg) -> isize;
        fn PostQuitMessage(code: i32);
    }

    const WM_KEYDOWN: u32 = 0x0100;
    const WM_KEYUP: u32 = 0x0101;
    const WM_CHAR: u32 = 0x0102;
    const WM_MOUSEMOVE: u32 = 0x0200;
    const WM_LBUTTONDOWN: u32 = 0x0201;
    const WM_LBUTTONUP: u32 = 0x0202;
    const WS_POPUP: u32 = 0x8000_0000;
    const WS_VISIBLE: u32 = 0x1000_0000;

    struct Parent(Hwnd);
    // SAFETY: only the handle value crosses; baseview reads it on this thread.
    unsafe impl HasRawWindowHandle for Parent {
        fn raw_window_handle(&self) -> RawWindowHandle {
            let mut h = Win32WindowHandle::empty();
            h.hwnd = self.0;
            RawWindowHandle::Win32(h)
        }
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn mouse(child: Hwnd, msg: u32, x: i32, y: i32) {
        let l = ((y as u32) << 16 | (x as u32 & 0xFFFF)) as i32 as isize;
        unsafe { PostMessageW(child, msg, if msg == WM_LBUTTONDOWN { 1 } else { 0 }, l) };
    }

    fn click(child: Hwnd, x: i32, y: i32) {
        mouse(child, WM_MOUSEMOVE, x, y);
        mouse(child, WM_LBUTTONDOWN, x, y);
        mouse(child, WM_LBUTTONUP, x, y);
    }

    /// A key press as the keyboard driver delivers it: key down, the character if it makes one, key up.
    fn key(target: Hwnd, vk: u32, scan: u32, extended: bool, ch: Option<char>) {
        let l = ((scan << 16) | 1 | if extended { 1 << 24 } else { 0 }) as isize;
        unsafe {
            PostMessageW(target, WM_KEYDOWN, vk as usize, l);
            if let Some(c) = ch {
                PostMessageW(target, WM_CHAR, c as usize, l);
            }
            PostMessageW(target, WM_KEYUP, vk as usize, l | (0xC000_0000u32 as i32 as isize));
        }
    }

    struct Script {
        parent: usize,
        child: usize,
        manual: bool,
        frame: u32,
        steps: Vec<(String, bool)>,
        done: bool,
        tables: Tables,
        ctl: crate::demo::MapCtl,
        bv: BusView,
        live: LiveView,
        wub_count: usize,
    }

    impl Script {
        fn record(&mut self, what: &str, ok: bool) {
            println!("  {} {what}", if ok { "PASS" } else { "FAIL" });
            self.steps.push((what.to_string(), ok));
        }
        fn focus(&self) -> &'static str {
            let f = unsafe { GetFocus() } as usize;
            if f == self.child {
                "the plugin window"
            } else if f == self.parent {
                "the host window"
            } else {
                "another window"
            }
        }
    }

    pub fn run(manual: bool) -> bool {
        println!("typing check: plugin window as a child of a host-like parent (the keyboard focus starts on the parent)");
        let (w, h) = (WINDOW.0 as i32, WINDOW.1 as i32);
        let class = wide("STATIC");
        let title = wide("host");
        // SAFETY: a plain system window used as the parent; destroyed below.
        let parent = unsafe { CreateWindowExW(0, class.as_ptr(), title.as_ptr(), WS_POPUP | WS_VISIBLE, 80, 40, w, h, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut()) };
        assert!(!parent.is_null(), "could not create the parent window");
        unsafe {
            ShowWindow(parent, 4); // SW_SHOWNOACTIVATE
            SetFocus(parent);
        }
        let (_, preset) = crate::demo::factory("Wobble Shot");
        let ctl = crate::demo::MapCtl::with_preset(Some(preset));
        let mut tables = Tables::new();
        tables.generate();
        let mut live = LiveView::idle();
        live.browser = Browser::with_library(crate::demo::fixture_library(60));
        live.browser.spawn_reveal = false;
        let script = Arc::new(Mutex::new(Script { parent: parent as usize, child: 0, manual, frame: 0, steps: vec![], done: false, tables, ctl, bv: BusView { label: "GTS 1".into(), oversampling: 2, ..Default::default() }, live, wub_count: 0 }));
        // The window state is the script itself (behind a mutex only to satisfy `Send`).
        let state = script.clone();
        let _handle = EguiWindow::open_parented(
            &Parent(parent),
            WindowOpenOptions {
                title: "plugin".into(),
                size: Size::new(WINDOW.0 as f64, WINDOW.1 as f64),
                scale: WindowScalePolicy::ScaleFactor(1.0),
                gl_config: Some(GlConfig { version: (3, 2), red_bits: 8, blue_bits: 8, green_bits: 8, alpha_bits: 8, depth_bits: 24, stencil_bits: 8, samples: None, srgb: true, double_buffer: true, vsync: true, ..Default::default() }),
            },
            GraphicsConfig::default(),
            state,
            |ctx, _q, _s| rr_ui::theme::apply(ctx),
            move |ctx, queue, st: &mut Arc<Mutex<Script>>| {
                ctx.request_repaint();
                let mut s = st.lock().unwrap();
                let s = &mut *s;
                s.frame += 1;
                if s.child == 0 {
                    s.child = unsafe { FindWindowExW(s.parent as Hwnd, std::ptr::null_mut(), std::ptr::null(), std::ptr::null()) } as usize;
                }
                egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
                    draw(ui, &s.ctl, &s.tables, &mut s.live, &mut s.bv);
                });
                let f = s.frame;
                let child = s.child as Hwnd;
                // Frames are ~15 ms apart; each step leaves time for the posted messages to be handled.
                match f {
                    20 => {
                        let on_host = unsafe { GetFocus() } as usize == s.parent;
                        s.record("before any click the host window owns the keyboard", on_host);
                    }
                    22 => click(child, 324, 44), // the preset name opens the browser
                    50 => {
                        let open = s.live.browser.open;
                        s.record("a click on the preset name opens the browser", open);
                        let still = unsafe { GetFocus() } as usize == s.parent;
                        s.record(&format!("opening the browser leaves the keyboard with the host (focus: {})", s.focus()), still);
                    }
                    52 => click(child, 164, 95), // the search box
                    80 => {
                        let on_plugin = unsafe { GetFocus() } == child;
                        s.record(&format!("clicking the search box moves the keyboard focus to the plugin window (focus: {})", s.focus()), on_plugin);
                        if s.manual {
                            println!("  manual: type into the search box now (60 s)");
                        } else {
                            let target = unsafe { GetFocus() };
                            for (c, scan) in [('w', 0x11), ('u', 0x16), ('b', 0x30)] {
                                key(target, c.to_ascii_uppercase() as u32, scan, false, Some(c));
                            }
                        }
                    }
                    110 if !s.manual => {
                        let text = s.live.browser.search_text().to_string();
                        s.wub_count = s.live.browser.visible_keys().len();
                        s.record(&format!("typed \"wub\" reaches the search box (box shows {text:?}, list narrowed to {} presets)", s.wub_count), text == "wub" && s.wub_count > 0 && s.wub_count < 74);
                        let target = unsafe { GetFocus() };
                        key(target, 0x28, 0x50, true, None); // Down: select + load the first match
                    }
                    130 if !s.manual => {
                        let picked = s.bv.preset_pick.is_some();
                        let cur = s.live.browser.cursor().map(str::to_string);
                        s.record(&format!("Down arrow moves the selection in the list and loads it (selection {cur:?})"), picked && cur.is_some());
                        let target = unsafe { GetFocus() };
                        key(target, 0x1B, 0x01, false, None); // Esc: clears the search
                    }
                    150 if !s.manual => {
                        let cleared = s.live.browser.search_text().is_empty() && s.live.browser.open;
                        s.record("Esc clears the search first and keeps the browser open", cleared);
                        let target = unsafe { GetFocus() };
                        key(target, 0x1B, 0x01, false, None); // Esc again: closes
                    }
                    170 if !s.manual => {
                        let closed = !s.live.browser.open;
                        s.record("a second Esc closes the browser", closed);
                        s.done = true;
                    }
                    n if s.manual && n == 4000 => s.done = true,
                    _ => {}
                }
                if s.done || f > 4200 {
                    queue.close_window();
                    unsafe { PostQuitMessage(0) };
                }
            },
        );
        // The host's message loop: frames come from the window's WM_TIMER, input from the queue.
        let mut msg = Msg { hwnd: std::ptr::null_mut(), message: 0, wparam: 0, lparam: 0, time: 0, pt: [0; 2] };
        // SAFETY: standard loop on the thread that created the windows.
        unsafe {
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                DispatchMessageW(&msg);
            }
            DestroyWindow(parent);
        }
        let s = script.lock().unwrap();
        let ok = !s.steps.is_empty() && s.steps.iter().all(|(_, ok)| *ok);
        println!("typing check: {} ({} of {} steps)", if ok { "PASS" } else { "FAIL" }, s.steps.iter().filter(|(_, ok)| *ok).count(), s.steps.len());
        ok
    }
}
