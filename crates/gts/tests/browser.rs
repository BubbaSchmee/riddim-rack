//! The preset browser (SPEC v2 Track 6), driven headless through the real editor `draw`: typing,
//! clicks, keys and the dialogs over a fixture library of 60+ presets (including a corrupt file and
//! duplicate names) in a scratch folder. What the browser asks of the host (`BusView::preset_pick`,
//! `preset_saved`) is read back; files are checked on disk.
use gts::dsp::tables::Tables;
use gts::editor::browser::{BODY, LIST_Y, NAV_H, ROW_H, TAGS_Y, TOP_H};
use gts::editor::{apply_preset, draw, Browser, BusView, Ctl, LiveView, ParamTable, Pick};
use gts::library::{self, Filter, Library, Origin, SourceFilter};
use gts::params::GtsParams;
use gts::preset::{bank::FACTORY_FILES, Preset};
use nih_plug::prelude::{ParamPtr, Params};
use nih_plug_egui::egui::{self, pos2, vec2, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A scratch directory removed on drop (factory files are read-only, so unlock first).
struct Tmp(PathBuf);

impl Tmp {
    fn new(tag: &str) -> Self {
        let p = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("browser-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Tmp(p)
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        fn unlock(p: &Path) {
            for e in std::fs::read_dir(p).into_iter().flatten().flatten() {
                let path = e.path();
                if path.is_dir() {
                    unlock(&path);
                } else if let Ok(m) = std::fs::metadata(&path) {
                    let mut perm = m.permissions();
                    #[allow(clippy::permissions_set_readonly_false)]
                    perm.set_readonly(false);
                    let _ = std::fs::set_permissions(&path, perm);
                }
            }
        }
        unlock(&self.0);
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Holds the values a host would hold (nih-plug has no public setter outside a host), reads the
/// rest through the real param table, and builds `current_preset` from both, as `PlugCtl` does from
/// the real params.
struct RealCtl {
    table: ParamTable,
    vals: RefCell<BTreeMap<String, f32>>,
}

impl Ctl for RealCtl {
    fn get01(&self, id: &str) -> f32 {
        self.vals.borrow().get(id).copied().unwrap_or_else(|| self.table.get01(id))
    }
    fn default01(&self, id: &str) -> f32 {
        self.table.default01(id)
    }
    fn readout(&self, id: &str) -> String {
        self.table.readout_at(id, self.get01(id))
    }
    fn steps(&self, id: &str) -> Option<usize> {
        self.table.steps(id)
    }
    fn begin(&self, _: &str) {}
    fn set01(&self, id: &str, v: f32) {
        self.vals.borrow_mut().insert(id.to_string(), v.clamp(0.0, 1.0));
    }
    fn end(&self, _: &str) {}
    fn readout_at(&self, id: &str, v: f32) -> String {
        self.table.readout_at(id, v)
    }
    fn name(&self, id: &str) -> String {
        self.table.name(id)
    }
    fn matrix(&self) -> Option<&gts::matrix::MatrixShared> {
        Some(self.table.matrix())
    }
    fn current_preset(&self) -> Option<Preset> {
        let mut params = serde_json::Map::new();
        for id in self.table.params().param_map().into_iter().map(|(id, _, _)| id).filter(|id| !id.starts_with("cv")) {
            let (v, ptr) = (self.get01(&id), self.table.ptr(&id));
            // SAFETY: the table keeps the params alive.
            let value = unsafe {
                match ptr {
                    ParamPtr::BoolParam(_) => serde_json::Value::from(v > 0.5),
                    ParamPtr::EnumParam(_) => serde_json::Value::from(ptr.normalized_value_to_string(v, false)),
                    _ => serde_json::json!(ptr.preview_plain(v)),
                }
            };
            params.insert(id, value);
        }
        let file = serde_json::json!({ "format_version": 1, "product": "GTS", "params": params });
        let (mut p, w) = Preset::from_json(&file.to_string()).unwrap();
        assert!(w.is_empty(), "{w:?}");
        let m = self.table.matrix().get();
        p.matrix = m.slots.clone();
        p.macros = m.macros.to_vec();
        Some(p)
    }
}

struct H {
    _tmp: Tmp,
    ctx: egui::Context,
    ctl: RealCtl,
    tables: Tables,
    live: LiveView,
    bv: BusView,
    t: f64,
}

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 832.0))
}

impl H {
    /// A browser over a fixture library: 64 user presets + 4 awkward files + the factory bank.
    fn new(tag: &str) -> Self {
        let tmp = Tmp::new(tag);
        library::fixture::write_user(&tmp.0, 64);
        let mut browser = Browser::with_library(Library::open(&tmp.0, FACTORY_FILES));
        browser.spawn_reveal = false;
        let ctx = egui::Context::default();
        rr_ui::theme::apply(&ctx);
        let mut tables = Tables::new();
        tables.generate();
        let mut live = LiveView::idle();
        live.browser = browser;
        let mut h = H { _tmp: tmp, ctx, ctl: RealCtl { table: ParamTable::new(Arc::new(GtsParams::default())), vals: Default::default() }, tables, live, bv: BusView { oversampling: 2, ..Default::default() }, t: 0.0 };
        h.frames(3);
        h
    }

    fn frame(&mut self, events: Vec<Event>) {
        self.t += 1.0 / 60.0;
        let input = RawInput { screen_rect: Some(screen()), time: Some(self.t), events, ..Default::default() };
        let _ = self.ctx.run(input, |ctx| {
            egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
                draw(ui, &self.ctl, &self.tables, &mut self.live, &mut self.bv);
            });
        });
    }

    fn frames(&mut self, n: usize) {
        for _ in 0..n {
            self.frame(vec![]);
        }
    }

    fn b(&mut self) -> &mut Browser {
        &mut self.live.browser
    }

    fn open(&mut self) {
        self.b().set_open(true);
        self.frames(14);
    }

    fn click(&mut self, p: Pos2) {
        self.frame(vec![Event::PointerMoved(p)]);
        self.frame(vec![Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }]);
        self.frame(vec![Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }]);
        self.frames(2);
    }

    fn double_click(&mut self, p: Pos2) {
        self.frame(vec![Event::PointerMoved(p)]);
        for _ in 0..2 {
            self.frame(vec![Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }]);
            self.frame(vec![Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }]);
        }
        self.frames(2);
    }

    fn right_click(&mut self, p: Pos2) {
        self.frame(vec![Event::PointerMoved(p)]);
        self.frame(vec![Event::PointerButton { pos: p, button: PointerButton::Secondary, pressed: true, modifiers: Modifiers::NONE }]);
        self.frame(vec![Event::PointerButton { pos: p, button: PointerButton::Secondary, pressed: false, modifiers: Modifiers::NONE }]);
        self.frames(3);
    }

    fn key(&mut self, key: Key) {
        self.key_mod(key, Modifiers::NONE);
    }

    fn key_mod(&mut self, key: Key, modifiers: Modifiers) {
        self.frame(vec![Event::Key { key, physical_key: Some(key), pressed: true, repeat: false, modifiers }]);
        self.frame(vec![Event::Key { key, physical_key: Some(key), pressed: false, repeat: false, modifiers }]);
        self.frames(1);
    }

    fn type_text(&mut self, s: &str) {
        self.frame(vec![Event::Text(s.to_string())]);
        self.frames(1);
    }

    /// Replaces the focused field's text.
    fn retype(&mut self, s: &str) {
        self.key_mod(Key::A, Modifiers::COMMAND);
        self.type_text(s);
    }

    fn pick(&mut self) -> Option<Pick> {
        self.bv.preset_pick.take()
    }

    fn picked_name(&mut self) -> Option<String> {
        match self.pick() {
            Some(Pick::Preset(p)) => Some(p.name.clone()),
            Some(Pick::Init) => Some("<Init>".into()),
            None => None,
        }
    }

    fn lib(&mut self) -> &mut Library {
        self.live.browser.library_mut().unwrap()
    }
}

fn body(x: f32, y: f32) -> Pos2 {
    pos2(BODY.0 + x, BODY.1 + y)
}

/// Centre of the Source (`group` 0) or Category (`group` 1) row `i`.
fn nav(group: usize, i: usize) -> Pos2 {
    let y = if group == 0 { 34.0 } else { 34.0 + 4.0 * NAV_H + 18.0 + 16.0 };
    body(60.0, TAGS_Y + y + i as f32 * NAV_H + NAV_H / 2.0)
}

fn row(i: usize) -> Pos2 {
    body(gts::editor::browser::MAIN_X + 120.0, LIST_Y + i as f32 * ROW_H + ROW_H / 2.0)
}

fn names(b: &mut Browser) -> Vec<String> {
    let keys = b.visible_keys();
    keys.iter().map(|k| b.library().unwrap().get(k).unwrap().name.clone()).collect()
}

fn lib_names(lib: &Library, f: &Filter) -> Vec<String> {
    lib.query(f).into_iter().map(|i| lib.entries()[i].name.clone()).collect()
}

// ---- tests --------------------------------------------------------------------------------------

#[test]
fn typing_in_the_search_box_filters_the_list() {
    let mut h = H::new("search");
    h.b().set_filter(Filter { category: None, ..Filter::default() });
    h.open();
    let all = names(h.b()).len();
    assert_eq!(all, 74);
    h.click(body(120.0, TOP_H / 2.0)); // the search box
    h.type_text("wub");
    assert_eq!(h.b().search_text(), "wub");
    let got = names(h.b());
    let want = lib_names(h.b().library().unwrap(), &Filter { category: None, text: "wub".into(), ..Filter::default() });
    assert_eq!(got, want);
    assert!(!got.is_empty() && got.len() < all);
    // Several words in any order; a phrase.
    h.retype("mara dark");
    let got = names(h.b());
    assert!(got.iter().all(|n| n.starts_with("Dark")), "{got:?}");
    h.retype("\"dark bass\"");
    assert!(!names(h.b()).is_empty());
    h.retype("zzzz");
    assert!(names(h.b()).is_empty());
    // Nothing matches: the empty state offers a way out, and it works.
    h.click(body(gts::editor::browser::MAIN_X + 362.0, LIST_Y + 212.0 + 44.0));
    assert_eq!(h.b().search_text(), "", "Clear filters empties the search");
    assert_eq!(names(h.b()).len(), all);
}

#[test]
fn source_category_and_favorites_filters_combine_on_click() {
    let mut h = H::new("filters");
    h.open();
    // Default filter is Bass, source All.
    assert_eq!(h.b().filter().category, Some("Bass"));
    assert_eq!(h.b().filter().source, SourceFilter::All);
    let bass = names(h.b()).len();
    h.click(nav(0, 1)); // Factory
    assert_eq!(h.b().filter().source, SourceFilter::Factory);
    let bass_factory = names(h.b());
    assert_eq!(bass_factory, ["Growl Hound", "Reese Drift", "Sub Weight", "Talk Box", "Wobble Shot"]);
    h.click(nav(1, 2)); // Lead
    assert_eq!(names(h.b()), ["Screech Lead"]);
    h.click(nav(1, 0)); // All categories
    assert_eq!(names(h.b()).len(), 6);
    h.click(nav(0, 0)); // source All again
    h.click(nav(1, 1)); // Bass
    assert_eq!(names(h.b()).len(), bass);
    // A favourite through the heart, then the Favorites source.
    let key0 = h.b().visible_keys()[0].clone();
    let heart = pos2(BODY.0 + gts::editor::browser::MAIN_X + (912.0 - gts::editor::browser::MAIN_X) - 12.0 - 22.0, BODY.1 + LIST_Y + ROW_H / 2.0);
    h.click(heart);
    assert!(h.lib().is_favorite(&key0), "heart click marks the first row");
    h.click(nav(0, 3)); // Favorites
    assert_eq!(h.b().visible_keys(), [key0.clone()]);
    // Counts follow the other filters.
    let c = h.b().counts();
    assert_eq!(c.sources[3], 1);
    // The favourite survives a restart.
    let root = h.lib().root().to_path_buf();
    assert!(Library::open(&root, FACTORY_FILES).is_favorite(&key0));
    // Unmark: the Favorites list is empty and says so.
    h.click(heart);
    assert!(!h.lib().is_favorite(&key0));
    assert!(h.b().visible_keys().is_empty());
}

#[test]
fn tag_chips_filter_and_clear() {
    let mut h = H::new("tags");
    h.b().set_filter(Filter { category: None, ..Filter::default() });
    h.open();
    // First chip of the results: click lights it and narrows; click again clears.
    let tags = h.lib().tags(&Filter { category: None, ..Filter::default() });
    let (first, n) = tags[0].clone();
    let x = BODY.0 + gts::editor::browser::MAIN_X + 46.0 + 20.0;
    let y = BODY.1 + TAGS_Y + 11.0;
    h.click(pos2(x, y));
    assert_eq!(h.b().filter().tag.as_deref(), Some(first.as_str()));
    assert_eq!(names(h.b()).len(), n);
    h.click(pos2(x, y));
    assert_eq!(h.b().filter().tag, None);
    assert_eq!(names(h.b()).len(), 74);
}

#[test]
fn arrows_enter_and_escape_drive_the_list() {
    let mut h = H::new("keys");
    h.open();
    let list = h.b().visible_keys();
    assert!(list.len() > 10);
    // The first Bass row is the damaged fixture file: selecting it says why it cannot load.
    assert!(h.lib().get(&list[0]).unwrap().is_damaged());
    h.key(Key::ArrowDown);
    assert_eq!(h.b().cursor(), Some(list[0].as_str()));
    assert!(h.pick().is_none() && h.b().notice_text().is_some_and(|n| n.contains("damaged")));
    // Load on select (the default): Down selects and loads in one go.
    h.key(Key::ArrowDown);
    assert_eq!(h.b().cursor(), Some(list[1].as_str()));
    let first = h.picked_name().expect("load on select loads");
    assert_eq!(first, h.lib().get(&list[1]).unwrap().name);
    h.key(Key::ArrowDown);
    h.key(Key::ArrowDown);
    assert_eq!(h.b().cursor(), Some(list[3].as_str()));
    h.key(Key::ArrowUp);
    assert_eq!(h.b().cursor(), Some(list[2].as_str()));
    h.bv.preset_pick = None;
    h.key(Key::End);
    assert_eq!(h.b().cursor(), Some(list.last().unwrap().as_str()));
    h.key(Key::Home);
    assert_eq!(h.b().cursor(), Some(list[0].as_str()));
    h.key(Key::PageDown);
    assert!(h.b().cursor() != Some(list[0].as_str()) && h.b().cursor() != Some(list[1].as_str()));
    // Off (the toggle in the left column): arrows only move the selection; Enter loads.
    h.b().set_filter(Filter::default());
    h.bv.preset_pick = None;
    let toggle = body(30.0, TAGS_Y + 34.0 + 4.0 * NAV_H + 18.0 + 16.0 + 8.0 * NAV_H + 14.0 + 12.0);
    h.click(toggle);
    h.bv.preset_pick = None;
    h.key(Key::ArrowDown);
    assert!(h.pick().is_none(), "Load on select is off");
    let key = h.b().cursor().unwrap().to_string();
    h.key(Key::Enter);
    assert_eq!(h.picked_name(), Some(h.lib().get(&key).unwrap().name.clone()));
    assert_eq!(h.b().loaded(), Some(key.as_str()));
    // Esc closes.
    assert!(h.b().open);
    h.key(Key::Escape);
    assert!(!h.b().open);
}

#[test]
fn escape_clears_the_search_first_and_typing_starts_a_search() {
    let mut h = H::new("esc");
    h.open();
    h.type_text("g"); // nothing has focus: the character starts a search
    assert_eq!(h.b().search_text(), "g");
    h.type_text("r");
    assert_eq!(h.b().search_text(), "gr");
    h.key(Key::Escape);
    assert_eq!(h.b().search_text(), "");
    assert!(h.b().open, "the first Escape only clears the search");
    h.key(Key::Escape);
    assert!(!h.b().open);
}

#[test]
fn double_click_loads_and_closes_and_a_damaged_file_does_not_load() {
    let mut h = H::new("dbl");
    h.open();
    let list = h.b().visible_keys();
    h.double_click(row(1));
    assert_eq!(h.picked_name(), Some(h.lib().get(&list[1]).unwrap().name.clone()));
    assert!(!h.b().open, "double-click closes the browser");
    // The corrupt file is listed with a badge and refuses to load, with a reason.
    h.b().set_filter(Filter { category: Some("Bass"), text: "broken".into(), ..Filter::default() });
    h.open();
    assert_eq!(names(h.b()), ["Broken"]);
    h.click(row(0));
    assert!(h.pick().is_none());
    assert!(h.b().notice_text().is_some_and(|n| n.contains("damaged") && n.contains("not a GTS preset")), "{:?}", h.b().notice_text());
    assert!(h.b().open);
    h.double_click(row(0));
    assert!(h.pick().is_none() && h.b().open, "a damaged file never loads or closes the browser");
}

#[test]
fn header_arrows_follow_the_filtered_list_and_wrap() {
    let mut h = H::new("arrows");
    h.b().set_filter(Filter { category: Some("Bass"), source: SourceFilter::Factory, ..Filter::default() });
    let list = h.b().visible_keys();
    assert_eq!(list.len(), 5);
    let (prev, next) = (pos2(208.0 + 14.0, 44.0), pos2(208.0 + 232.0 - 14.0, 44.0));
    h.click(next);
    assert_eq!(h.b().loaded(), Some(list[0].as_str()), "from nothing, Next starts at the top");
    assert!(matches!(h.pick(), Some(Pick::Preset(_))));
    h.click(next);
    assert_eq!(h.b().loaded(), Some(list[1].as_str()));
    h.click(prev);
    h.click(prev);
    assert_eq!(h.b().loaded(), Some(list[4].as_str()), "Previous wraps to the end");
    h.click(next);
    assert_eq!(h.b().loaded(), Some(list[0].as_str()), "Next wraps to the start");
    // Another filter, another list.
    h.b().set_filter(Filter { category: Some("Lead"), source: SourceFilter::Factory, ..Filter::default() });
    h.click(next);
    assert_eq!(h.b().loaded().map(|k| k.starts_with("Factory/Lead")), Some(true), "{:?}", h.b().loaded());
    // The header shows the name of what is loaded.
    assert_eq!(h.b().loaded_label().unwrap().1, "Screech Lead");
    // Damaged files are skipped by the arrows.
    h.b().set_filter(Filter { category: Some("Bass"), text: "broken".into(), ..Filter::default() });
    h.bv.preset_pick = None;
    h.click(next);
    assert!(h.pick().is_none());
    // An empty list: the arrows do nothing.
    h.b().set_filter(Filter { text: "zzzzzz".into(), ..Filter::default() });
    h.click(next);
    assert!(h.pick().is_none());
}

#[test]
fn clicking_the_name_opens_and_the_close_button_and_a_page_tab_close() {
    let mut h = H::new("open");
    assert!(!h.b().open);
    h.click(pos2(208.0 + 116.0, 44.0));
    assert!(h.b().open);
    h.frames(12);
    h.click(pos2(BODY.0 + 912.0 - 15.0, BODY.1 + TOP_H / 2.0));
    assert!(!h.b().open, "the close button");
    h.click(pos2(208.0 + 116.0, 44.0));
    assert!(h.b().open);
    h.frames(12);
    h.click(pos2(448.0 + 100.0, 29.0)); // the MATRIX segment
    assert!(!h.b().open, "a page tab closes the browser");
    assert_eq!(h.bv.page, gts::editor::Page::Matrix);
}

#[test]
fn the_panel_fades_in_over_150_ms_and_honours_reduced_motion() {
    let mut h = H::new("motion");
    h.b().set_open(true);
    h.frame(vec![]);
    let k1 = h.live.browser.open_k(&h.ctx);
    assert!(k1 < 0.9, "starts a fade, not a pop: {k1}");
    h.frames(11); // 12 frames at 60 Hz = 200 ms
    assert!(h.live.browser.open_k(&h.ctx) > 0.999);
    let mut r = H::new("motion-rm");
    r.bv.reduced_motion = true;
    r.frames(2);
    r.b().set_open(true);
    r.frame(vec![]);
    assert!(r.live.browser.open_k(&r.ctx) > 0.999, "reduced motion: instant");
}

#[test]
fn save_as_writes_a_user_preset_asks_before_replacing_and_rejects_illegal_names() {
    let mut h = H::new("saveas");
    // Load a factory preset so Save As starts from it.
    let key = "Factory/Bass/Growl Hound";
    h.live.browser.load_key(&mut h.bv, key);
    let Some(Pick::Preset(p)) = h.pick() else { panic!("loaded") };
    apply_preset(&h.ctl.table, &h.ctl, Some(p.as_ref()));
    h.open();
    let live = &mut h.live.browser;
    live.open_save_as(&h.ctl);
    h.frames(14);
    assert!(h.b().dialog_open());
    // The name starts as "<name> Copy" (a factory preset is never overwritten) and has focus.
    h.retype("My Growl");
    h.key(Key::Enter);
    assert!(!h.b().dialog_open(), "Enter in the name field saves");
    let path = h._tmp.0.join("Presets/User/Bass/My Growl.json");
    assert!(path.is_file());
    let (saved, warnings) = Preset::load(&path).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(saved.name, "My Growl");
    assert_eq!(saved.category, "Bass");
    assert_eq!(saved.macros, ["Growl", "Dirt", "Width", "Space"]);
    assert_eq!(saved.tags, ["growl", "riddim", "metallic"]);
    assert!(h.bv.preset_saved, "the host editor is told to take a new baseline");
    assert_eq!(h.b().loaded(), Some("User/Bass/My Growl"));
    // What was saved is the sound that was loaded.
    for (id, a) in params_of(&saved) {
        let b = params_of(&p).into_iter().find(|(k, _)| *k == id).unwrap().1;
        match (a.as_f64(), b.as_f64()) {
            (Some(x), Some(y)) => assert!((x - y).abs() <= 1e-3 * y.abs().max(1.0), "{id}: {x} vs {y}"),
            _ => assert_eq!(a, b, "{id}"),
        }
    }
    assert_eq!(saved.matrix.len(), p.matrix.len());
    // The same name again: the dialog asks before replacing.
    h.bv.preset_saved = false;
    h.live.browser.open_save_as(&h.ctl);
    h.frames(14);
    h.retype("my growl"); // other case, same file on this file system
    h.key(Key::Enter);
    assert!(h.b().dialog_open(), "still open: asking to replace");
    let before = std::fs::read_to_string(&path).unwrap();
    h.key(Key::Escape); // Esc steps back to the form, does not save or close
    assert!(h.b().dialog_open() && std::fs::read_to_string(&path).unwrap() == before);
    h.key(Key::Enter); // save again
    assert!(h.b().dialog_open(), "asks again");
    // Confirm with the Replace button (right edge of the box, bottom row).
    let size = vec2(gts::editor::browser::CONFIRM_SIZE.0, gts::editor::browser::CONFIRM_SIZE.1);
    let c = pos2(BODY.0 + 912.0 / 2.0, BODY.1 + 704.0 / 2.0 + 6.0 * 0.0);
    let inner_br = pos2(c.x + size.x / 2.0 - 20.0, c.y + size.y / 2.0 - 20.0);
    h.click(pos2(inner_br.x - 42.0, inner_br.y - 14.0));
    assert!(!h.b().dialog_open(), "Replace saved");
    assert!(h.bv.preset_saved);
    assert_eq!(h.lib().entries().iter().filter(|e| e.name.eq_ignore_ascii_case("my growl")).count(), 1, "replaced, not duplicated");
    // Illegal names are refused: the dialog stays, nothing is written.
    h.bv.preset_saved = false;
    h.live.browser.open_save_as(&h.ctl);
    h.frames(14);
    for bad in ["a/b", "CON", "x:y", "q?"] {
        h.click(name_field());
        h.retype(bad);
        h.key(Key::Enter);
        assert!(h.b().dialog_open(), "{bad:?} must not save");
        assert!(!h.bv.preset_saved, "{bad:?}");
    }
    assert!(!h._tmp.0.join("Presets/User/Bass/a").exists());
    h.key(Key::Escape);
    assert!(!h.b().dialog_open());
}

/// Centre of the Save As name field (the box is centred in the body).
fn name_field() -> Pos2 {
    let (cx, cy) = (BODY.0 + BODY.2 / 2.0, BODY.1 + BODY.3 / 2.0);
    let (w, h) = gts::editor::browser::SAVE_AS_SIZE;
    pos2(cx - w / 2.0 + 20.0 + (w - 40.0) / 2.0, cy - h / 2.0 + 42.0 + 31.0)
}

fn params_of(p: &Preset) -> Vec<(String, serde_json::Value)> {
    let v: serde_json::Value = serde_json::from_str(&p.to_json()).unwrap();
    v["params"].as_object().unwrap().iter().map(|(k, v)| (k.clone(), v.clone())).collect()
}

#[test]
fn save_overwrites_the_loaded_user_preset_after_a_confirmation() {
    let mut h = H::new("save");
    let key = h.lib().save_as(&Preset { name: "Mine".into(), author: "Me".into(), ..Preset::default() }, "Pad", false).unwrap();
    h.live.browser.load_key(&mut h.bv, &key);
    h.pick();
    h.open();
    h.live.browser.begin_save(&h.ctl);
    h.frames(14);
    assert!(h.b().dialog_open());
    // Change the sound, then confirm with Enter.
    h.ctl.set01("volume", 0.9);
    h.key(Key::Enter);
    assert!(!h.b().dialog_open());
    let (saved, _) = Preset::load(&h._tmp.0.join("Presets/User/Pad/Mine.json")).unwrap();
    assert_eq!(saved.author, "Me", "metadata of the loaded preset is kept");
    assert!(saved.patch.volume_db > 0.0, "the new volume was saved: {}", saved.patch.volume_db);
    assert!(h.bv.preset_saved);
    // A factory preset's Save is Save As.
    h.live.browser.load_key(&mut h.bv, "Factory/Bass/Sub Weight");
    h.live.browser.begin_save(&h.ctl);
    h.frames(14);
    assert!(h.b().dialog_open());
    h.key(Key::Escape);
    assert!(!h.b().dialog_open());
}

#[test]
fn context_menu_rename_delete_and_default() {
    let mut h = H::new("menu");
    h.b().set_filter(Filter { category: None, source: SourceFilter::User, text: "dark wub 00".into(), ..Filter::default() });
    h.open();
    let keys = h.b().visible_keys();
    assert_eq!(keys.len(), 1, "{keys:?}");
    let key = keys[0].clone();
    let at = row(0);
    h.right_click(at);
    assert!(h.b().menu_open());
    // Rows: Load, favorites, default, (sep) Reveal, Rename, Delete.
    let item = |i: usize, sep: bool| pos2(at.x + 40.0, at.y + 4.0 + 12.0 + i as f32 * 24.0 + if sep { 9.0 } else { 0.0 });
    // Set as default.
    h.click(item(2, false));
    assert!(!h.b().menu_open());
    assert_eq!(h.lib().settings().default_preset, key);
    // Rename through the dialog.
    h.right_click(at);
    h.click(item(4, true));
    assert!(h.b().dialog_open());
    h.retype("Renamed Wub");
    h.key(Key::Enter);
    assert!(!h.b().dialog_open());
    let new_key = "User/Bass/Renamed Wub";
    assert!(h._tmp.0.join("Presets/User/Bass/Renamed Wub.json").is_file());
    assert!(!h._tmp.0.join("Presets/User/Bass/Dark Wub 00.json").exists());
    assert_eq!(h.lib().settings().default_preset, new_key, "the default follows the rename");
    // Rename onto an existing name is refused inside the dialog.
    h.b().set_filter(Filter { category: None, source: SourceFilter::User, text: "renamed wub".into(), ..Filter::default() });
    h.frames(2);
    h.right_click(at);
    h.click(item(4, true));
    h.retype("Broken");
    h.key(Key::Enter);
    assert!(h.b().dialog_open(), "a taken name keeps the dialog open");
    h.key(Key::Escape);
    // Delete asks first.
    h.right_click(at);
    h.click(item(5, true));
    assert!(h.b().dialog_open());
    h.key(Key::Escape);
    assert!(h._tmp.0.join("Presets/User/Bass/Renamed Wub.json").is_file(), "Cancel keeps it");
    h.right_click(at);
    h.click(item(5, true));
    h.key(Key::Enter);
    assert!(!h._tmp.0.join("Presets/User/Bass/Renamed Wub.json").exists());
    assert!(h.lib().get(new_key).is_none() && h.lib().settings().default_preset.is_empty());
    // A factory preset's menu has no Rename or Delete; Reveal reports its folder.
    h.b().set_filter(Filter { category: None, source: SourceFilter::Factory, text: "\"sub weight\"".into(), ..Filter::default() });
    h.frames(2);
    h.right_click(at);
    assert!(h.b().menu_open());
    h.click(item(3, true)); // Reveal
    let revealed = h.b().revealed.clone();
    assert_eq!(revealed.len(), 1);
    assert!(revealed[0].ends_with("Sub Weight.json") || revealed[0].to_string_lossy().contains("Sub Weight"), "{revealed:?}");
}

#[test]
fn the_new_instance_default_and_loading_a_preset_reproduces_it_through_save() {
    // apply_preset through real params, then the sound read back (what Save writes) matches the file.
    let h = H::new("roundtrip");
    for p in gts::preset::factory() {
        apply_preset(&h.ctl.table, &h.ctl, Some(p));
        let back = h.ctl.current_preset().unwrap();
        let (a, b) = (params_of(&back), params_of(p));
        for ((ka, va), (kb, vb)) in a.iter().zip(&b) {
            assert_eq!(ka, kb);
            match (va.as_f64(), vb.as_f64()) {
                (Some(x), Some(y)) => assert!((x - y).abs() <= 1e-3 * y.abs().max(1.0), "{}: {ka} {x} vs {y}", p.name),
                _ => assert_eq!(va, vb, "{}: {ka}", p.name),
            }
        }
        assert_eq!(back.matrix, p.matrix, "{}", p.name);
        assert_eq!(back.macros, p.matrix_state().macros.to_vec(), "{}", p.name);
    }
    // A pristine table equals its default snapshot (what the default-preset check relies on).
    let fresh = ParamTable::new(Arc::new(GtsParams::default()));
    assert_eq!(fresh.snapshot(), fresh.default_snapshot());
    // The real table's own read-back of an untouched instance is the init patch.
    let p = fresh.current_preset();
    assert_eq!(p.patch, gts::dsp::Patch::default());
    assert!(p.matrix.is_empty() && p.macros == ["Macro 1", "Macro 2", "Macro 3", "Macro 4"]);
}

#[test]
fn every_help_key_the_browser_uses_has_a_line() {
    let src = include_str!("../src/editor/browser.rs");
    let mut found = std::collections::BTreeSet::new();
    for pre in ["\"browser:", "\"dialog:"] {
        for part in src.split(pre).skip(1) {
            let name: String = part.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
            found.insert(format!("{}{name}", &pre[1..]));
        }
    }
    let bar = include_str!("../src/editor.rs");
    for part in bar.split("\"browser:").skip(1) {
        let name: String = part.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
        found.insert(format!("browser:{name}"));
    }
    assert!(found.len() >= 20, "{found:?}");
    for k in &found {
        assert!(gts::help::ui_help(k).is_some(), "no help line for {k}");
    }
    // Every browser help line is used by something (no orphans).
    for (k, _) in gts::help::UI_HELP.iter().filter(|(k, _)| k.starts_with("browser:") || k.starts_with("dialog:")) {
        assert!(found.contains(*k), "help line {k} is unused");
    }
}

#[test]
fn only_user_presets_can_be_changed_and_the_library_is_what_the_list_shows() {
    let mut h = H::new("lib");
    h.open();
    let total = h.lib().entries().len();
    assert_eq!(total, 64 + 4 + FACTORY_FILES.len());
    let users = h.lib().entries().iter().filter(|e| e.origin == Origin::User).count();
    assert_eq!(users, 68);
    // Rescan picks up a hand-copied file.
    let mut p = Preset { name: "Dropped In".into(), category: "FX".into(), ..Preset::default() };
    p.tags = vec!["dropped".into()];
    std::fs::write(h._tmp.0.join("Presets/User/FX/Dropped In.json"), p.to_json()).unwrap();
    assert!(h.lib().get("User/FX/Dropped In").is_none());
    // The Rescan button: left column, below the toggle.
    let rescan = body(88.0, TAGS_Y + 34.0 + 4.0 * NAV_H + 18.0 + 16.0 + 8.0 * NAV_H + 14.0 + 34.0 + 13.0);
    h.click(rescan);
    assert!(h.lib().get("User/FX/Dropped In").is_some(), "Rescan reads the folders again");
    assert_eq!(h.lib().entries().len(), total + 1);
}

#[test]
fn the_loaded_preset_and_its_edited_flag_are_host_state() {
    use std::sync::atomic::Ordering;
    let a = GtsParams::default();
    *a.preset_key.write().unwrap() = "User/Bass/My Growl".into();
    a.preset_dirty.store(true, Ordering::Relaxed);
    let b = GtsParams::default();
    b.deserialize_fields(&a.serialize_fields());
    assert_eq!(*b.preset_key.read().unwrap(), "User/Bass/My Growl");
    assert!(b.preset_dirty.load(Ordering::Relaxed));
    // A project saved before this existed keeps the defaults: nothing loaded, not edited.
    let c = GtsParams::default();
    c.deserialize_fields(&BTreeMap::new());
    assert!(c.preset_key.read().unwrap().is_empty() && !c.preset_dirty.load(Ordering::Relaxed));
}

#[test]
fn a_new_instance_loads_the_default_preset_when_one_is_set() {
    let mut h = H::new("default");
    assert!(!h.live.browser.load_default(&mut h.bv), "no default set: nothing to load");
    assert!(h.bv.preset_pick.is_none());
    h.lib().set_default(Some("Factory/Bass/Talk Box")).unwrap();
    assert!(h.live.browser.load_default(&mut h.bv));
    assert_eq!(h.picked_name(), Some("Talk Box".into()));
    assert_eq!(h.b().loaded(), Some("Factory/Bass/Talk Box"));
    // A default that no longer exists, or is damaged, falls back to Init (nothing is loaded).
    h.lib().set_default(Some("User/Bass/Broken")).unwrap();
    assert!(!h.live.browser.load_default(&mut h.bv));
    h.lib().set_default(Some("User/Bass/Gone")).unwrap();
    assert!(!h.live.browser.load_default(&mut h.bv));
    // It survives a restart.
    h.lib().set_default(Some("Factory/Lead/Screech Lead")).unwrap();
    let root = h.lib().root().to_path_buf();
    let again = Library::open(&root, FACTORY_FILES);
    assert_eq!(again.default_preset().map(|e| e.name.as_str()), Some("Screech Lead"));
}
