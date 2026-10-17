//! The preset browser (SPEC v2 Track 6, `docs/standards/presets-and-ux.md` section 1): a panel that
//! opens over the page body from the header's preset bar. A child of [`super`] (the editor), so it
//! shares `set_hover`, `place` and the editor's types. The data lives in [`crate::library`]; this
//! file is the view and the commands.
//!
//! ```text
//! +--------------------------------------------------------------+
//! | [ search ............ ]                   [Init][Save][Save As][x]
//! | SOURCE      | TAGS  riddim tearout growl wobble ...           |
//! |  All     64 | +------------------------------------------+ |
//! |  Factory  6 | | * Growl Hound        BASS      Riddim   v  | |
//! |  User    58 | |   Riddim growl: Square Morph ...  FACTORY   | |
//! |  Favorites  | | ...                                       # | |
//! | CATEGORY    | +------------------------------------------+ |
//! |  All  Bass  | 12 of 64 presets                  Saved "X"   |
//! |  ...        | +------------------------------------------+ |
//! | [x] Load on | | NAME   Bass . Factory . by Riddim Rack     | |
//! |  [Rescan]   | | description (scrolls) ...   MACROS  TAGS   | |
//! |  [Reveal]   | +------------------------------------------+ |
//! +--------------------------------------------------------------+
//! ```
//!
//! Everything is laid out in absolute coordinates inside the body rect, drawn with the `rr-ui`
//! controls, and moves on the MOTION.md clock: the panel fades in over 150 ms (out over 90 ms) with
//! a 6 px rise, honouring Reduced motion. A load does not happen here: the browser sets
//! [`BusView::preset_pick`] and the host editor applies it with the usual gesture glide, exactly
//! as the old header list did. The library is opened on a loader thread when the editor opens
//! (factory write-out and a scan of a few hundred files stay off the GUI frame); everything after
//! that (save, rename, delete, favourites) is a few milliseconds of GUI-thread file I/O.
//!
//! Keyboard (when the window has it): Up/Down/PageUp/PageDown/Home/End move the selection, Enter
//! loads it, Esc clears the search, then closes; typing starts a search; Ctrl+F focuses the search
//! box; Delete asks to delete a user preset. Inside a host the plugin window gets the keyboard when
//! a text field is clicked (egui-baseview's Windows keyboard workaround, enabled in Cargo.toml;
//! `docs/design/keyboard-in-host.md`); the browser never takes it on opening or on a plain click.

use super::*;
use crate::library::{self, BrowserPrefs, Counts, Entry, Filter, LibError, Library, Origin, SourceFilter, CATEGORIES, INIT_KEY};
use egui::{Key, Modifiers};
use rr_ui::controls::{self, ButtonKind, MenuEvent, MenuItem, CHIP_H};
use rr_ui::theme::SRC_M;
use std::path::PathBuf;
use std::sync::mpsc;

/// The page body the panel covers (window px), the same rect as the SYNTH / FX / MATRIX body.
pub const BODY: (f32, f32, f32, f32) = (44.0, 80.0, 912.0, 704.0);

/// Layout in px relative to [`BODY`]'s top left (public for the headless tests and the previews).
pub const LEFT_W: f32 = 176.0;
pub const MAIN_X: f32 = LEFT_W + 12.0;
const MAIN_W: f32 = 912.0 - MAIN_X;
pub const TOP_H: f32 = 30.0;
pub const TAGS_Y: f32 = 42.0;
pub const LIST_Y: f32 = 72.0;
pub const LIST_H: f32 = 424.0;
const STATUS_Y: f32 = LIST_Y + LIST_H + 4.0;
const DETAIL_Y: f32 = STATUS_Y + 24.0;
const DETAIL_H: f32 = 704.0 - DETAIL_Y;
pub const ROW_H: f32 = 40.0;
pub const NAV_H: f32 = 26.0;
/// Save As box size: the description field is the tall one.
pub const SAVE_AS_SIZE: (f32, f32) = (560.0, 444.0);
/// The confirmation dialogs (replace, overwrite, delete): a sentence and two buttons.
pub const CONFIRM_SIZE: (f32, f32) = (440.0, 160.0);
/// Notices (saved, deleted, errors) stay this long, seconds.
const NOTICE_SECS: f64 = 4.0;

/// What the browser asks the host editor to load.
#[derive(Clone, Debug, PartialEq)]
pub enum Pick {
    /// Every sound parameter and the matrix back to the defaults.
    Init,
    Preset(Arc<Preset>),
}

#[derive(Clone, Debug)]
struct Notice {
    text: String,
    error: bool,
    until: f64,
}

struct MenuState {
    key: String,
    pos: Pos2,
}

/// Save As: the form, plus the sound as it was when the dialog opened.
struct SaveAs {
    preset: Preset,
    name: String,
    category: usize,
    author: String,
    tags: String,
    description: String,
    macros: [String; 4],
    error: Option<String>,
    /// Key of the existing preset the name would replace: the dialog then asks to confirm.
    replace: Option<String>,
    fresh: bool,
}

enum Dialog {
    SaveAs(Box<SaveAs>),
    ConfirmSave { key: String, name: String },
    ConfirmDelete { key: String, name: String },
    Rename { key: String, name: String, error: Option<String>, fresh: bool },
}

/// One thing the user asked for this frame; applied after drawing so the draw code only reads.
enum Cmd {
    Source(SourceFilter),
    Category(Option<&'static str>),
    Tag(Option<String>),
    ClearSearch,
    ClearFilters,
    SearchAuthor(String),
    Select { key: String, load: bool, close: bool },
    Heart(String),
    Menu { key: String, pos: Pos2 },
    Init,
    Save,
    SaveAs,
    Rescan,
    Reveal,
    Close,
    LoadOnSelect(bool),
}

/// The browser's state. Lives in [`LiveView`] (editor-session state, never persisted; what must
/// survive is in the library's `settings.json` and the project's `preset_key`).
pub struct Browser {
    lib: Option<Library>,
    loader: Option<Mutex<mpsc::Receiver<Library>>>,
    /// The loader thread, joined when the browser goes (a thread must not outlive the plugin's code).
    loader_thread: Option<std::thread::JoinHandle<()>>,
    pub open: bool,
    filter: Filter,
    search: String,
    load_on_select: bool,
    visible: Vec<usize>,
    counts: Counts,
    tag_list: Vec<(String, usize)>,
    cache: Option<(u32, Filter)>,
    cursor: Option<String>,
    loaded: Option<String>,
    scroll: f32,
    scroll_target: f32,
    detail_scroll: f32,
    detail_key: Option<String>,
    menu: Option<MenuState>,
    dialog: Option<Dialog>,
    notice: Option<Notice>,
    /// Focus the search box on the next frame.
    focus_search: bool,
    /// The search box had focus at the end of the last frame (egui drops focus on Escape before
    /// the browser sees the key).
    search_focused: bool,
    /// Last egui time seen (for notices raised outside `show`).
    now: f64,
    /// Spawn the file manager on Reveal. Tests switch it off and read [`Browser::revealed`].
    pub spawn_reveal: bool,
    pub revealed: Vec<PathBuf>,
}

impl Default for Browser {
    fn default() -> Self {
        Self {
            lib: None,
            loader: None,
            loader_thread: None,
            open: false,
            filter: Filter::default(),
            search: String::new(),
            load_on_select: true,
            visible: vec![],
            counts: Counts::default(),
            tag_list: vec![],
            cache: None,
            cursor: None,
            loaded: None,
            scroll: 0.0,
            scroll_target: 0.0,
            detail_scroll: 0.0,
            detail_key: None,
            menu: None,
            dialog: None,
            notice: None,
            focus_search: false,
            search_focused: false,
            now: 0.0,
            spawn_reveal: true,
            revealed: vec![],
        }
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        // The scan is tens of milliseconds; waiting keeps the thread inside the plugin's lifetime.
        if let Some(t) = self.loader_thread.take() {
            let _ = t.join();
        }
    }
}

fn search_id() -> Id {
    Id::new("gts-br-search")
}

fn hover(ui: &Ui, name: &str, value: &str, key: &str) {
    set_hover(ui, name, value, None, crate::help::ui_help(key).unwrap_or(""));
}

impl Browser {
    /// A browser over an already opened library (tests, previews).
    pub fn with_library(lib: Library) -> Self {
        let mut b = Self::default();
        b.set_library(lib);
        b
    }

    /// Starts opening the library on a loader thread, once. Safe to call every time the editor
    /// opens.
    pub fn ensure_loading(&mut self) {
        if self.lib.is_some() || self.loader.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new().name("gts-presets".into()).spawn(move || {
            let _ = tx.send(Library::open(&library::default_root(), crate::preset::bank::FACTORY_FILES));
        });
        match spawned {
            Ok(handle) => {
                self.loader = Some(Mutex::new(rx));
                self.loader_thread = Some(handle);
            }
            // No thread to be had: open here, once. Rare, and still off the audio thread.
            Err(_) => self.set_library(Library::open(&library::default_root(), crate::preset::bank::FACTORY_FILES)),
        }
    }

    /// Takes the library from the loader thread when it is done.
    pub fn poll(&mut self) {
        let Some(rx) = &self.loader else { return };
        let got = rx.lock().unwrap_or_else(PoisonError::into_inner).try_recv();
        match got {
            Ok(lib) => {
                self.loader = None;
                self.loader_thread.take().map(|t| t.join());
                self.set_library(lib);
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.loader = None;
                self.loader_thread.take().map(|t| t.join());
                self.say("The preset folders could not be opened.", true);
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }

    fn set_library(&mut self, lib: Library) {
        let p = &lib.settings().browser;
        self.filter.source = SourceFilter::from_name(&p.source);
        self.filter.category = library::category_index(&p.category).map(|i| CATEGORIES[i]);
        self.load_on_select = p.load_on_select;
        if let Some(first) = lib.issues.first() {
            self.notice = Some(Notice { text: first.clone(), error: true, until: self.now + 8.0 });
        }
        self.lib = Some(lib);
        self.cache = None;
    }

    pub fn library(&self) -> Option<&Library> {
        self.lib.as_ref()
    }
    pub fn library_mut(&mut self) -> Option<&mut Library> {
        self.cache = None;
        self.lib.as_mut()
    }
    pub fn ready(&self) -> bool {
        self.lib.is_some()
    }
    pub fn filter(&self) -> &Filter {
        &self.filter
    }
    /// The loaded preset's key (`"Init"` for Init), if one is.
    pub fn loaded(&self) -> Option<&str> {
        self.loaded.as_deref()
    }
    /// (category, name) of the loaded preset for the header bar: from the library when the entry
    /// is there, else from the stored key (a file deleted since, a library still loading).
    pub fn loaded_label(&self) -> Option<(String, String)> {
        let key = self.loaded.as_deref()?;
        if key == INIT_KEY {
            return Some(("Init".into(), "Init".into()));
        }
        if let Some(e) = self.entry(key) {
            return Some((e.label.replace('/', " / "), e.name.clone()));
        }
        let mut parts = key.split('/');
        let (_, cat, name) = (parts.next(), parts.next(), parts.next_back());
        Some((cat.unwrap_or("").to_string(), name.unwrap_or(key).to_string()))
    }
    pub fn cursor(&self) -> Option<&str> {
        self.cursor.as_deref()
    }
    pub fn search_text(&self) -> &str {
        &self.search
    }
    pub fn dialog_open(&self) -> bool {
        self.dialog.is_some()
    }
    pub fn menu_open(&self) -> bool {
        self.menu.is_some()
    }
    pub fn notice_text(&self) -> Option<&str> {
        self.notice.as_ref().map(|n| n.text.as_str())
    }
    /// Keys of the presets the current filters show, in list order.
    pub fn visible_keys(&mut self) -> Vec<String> {
        self.refresh();
        match &self.lib {
            Some(l) => self.visible.iter().map(|&i| l.entries()[i].key.clone()).collect(),
            None => vec![],
        }
    }
    pub fn counts(&mut self) -> Counts {
        self.refresh();
        self.counts.clone()
    }
    pub fn set_filter(&mut self, f: Filter) {
        self.search = f.text.clone();
        self.filter = f;
    }
    /// The loaded identity as the project stored it (no library needed).
    pub fn restore(&mut self, key: &str) {
        self.loaded = (!key.is_empty()).then(|| key.to_string());
        if self.cursor.is_none() {
            self.cursor = self.loaded.clone();
        }
    }
    pub fn set_open(&mut self, open: bool) {
        if open && !self.open {
            self.cursor = self.loaded.clone().or(self.cursor.take());
            self.scroll_to_cursor(true);
            self.detail_scroll = 0.0;
        }
        if !open {
            self.menu = None;
            self.dialog = None;
        }
        self.open = open;
    }

    fn say(&mut self, text: &str, error: bool) {
        self.notice = Some(Notice { text: text.to_string(), error, until: self.now + NOTICE_SECS });
    }

    /// Recomputes the list, counts and tags when the library or a filter changed.
    fn refresh(&mut self) {
        let Some(lib) = &self.lib else { return };
        if self.cache.as_ref().is_some_and(|(rev, f)| *rev == lib.rev() && *f == self.filter) {
            return;
        }
        self.visible = lib.query(&self.filter);
        self.counts = lib.counts(&self.filter);
        self.tag_list = lib.tags(&self.filter);
        self.cache = Some((lib.rev(), self.filter.clone()));
        let max = (self.visible.len() as f32 * ROW_H - LIST_H).max(0.0);
        self.scroll_target = self.scroll_target.clamp(0.0, max);
        self.scroll = self.scroll.clamp(0.0, max);
    }

    fn entry(&self, key: &str) -> Option<&Entry> {
        self.lib.as_ref().and_then(|l| l.get(key))
    }

    fn index_in_list(&self, key: &str) -> Option<usize> {
        let lib = self.lib.as_ref()?;
        self.visible.iter().position(|&i| lib.entries()[i].key == key)
    }

    fn scroll_to_cursor(&mut self, snap: bool) {
        self.refresh();
        let Some(i) = self.cursor.as_deref().and_then(|k| self.index_in_list(k)) else { return };
        let (top, bottom) = (i as f32 * ROW_H, (i + 1) as f32 * ROW_H);
        let max = (self.visible.len() as f32 * ROW_H - LIST_H).max(0.0);
        if snap {
            // Opening: put the loaded row a third of the way down, on a row boundary.
            self.scroll_target = (((top - LIST_H / 3.0) / ROW_H).round() * ROW_H).clamp(0.0, max);
            self.scroll = self.scroll_target;
        } else if top < self.scroll_target {
            self.scroll_target = top;
        } else if bottom > self.scroll_target + LIST_H {
            self.scroll_target = (bottom - LIST_H).min(max);
        }
    }

    // ---- loading -----------------------------------------------------------------------------

    /// Loads the preset `key` (a library key, or [`INIT_KEY`]) through the host editor.
    pub fn load_key(&mut self, bv: &mut BusView, key: &str) -> bool {
        if key == INIT_KEY {
            bv.preset_pick = Some(Pick::Init);
            self.loaded = Some(INIT_KEY.into());
            return true;
        }
        let Some(e) = self.entry(key) else { return false };
        match e.preset.clone() {
            Some(p) => {
                bv.preset_pick = Some(Pick::Preset(p));
                self.loaded = Some(key.to_string());
                true
            }
            None => {
                let msg = format!("{} is damaged: {}", e.name, e.problem.as_deref().unwrap_or("unreadable"));
                self.say(&msg, true);
                false
            }
        }
    }

    /// Steps to the previous (-1) or next (+1) preset of the current filtered list, wrapping,
    /// skipping damaged files. With nothing loaded from the list it starts at an end.
    pub fn step(&mut self, dir: i32, bv: &mut BusView) {
        self.refresh();
        let Some(lib) = &self.lib else { return };
        let n = self.visible.len() as i32;
        if n == 0 {
            return;
        }
        let cur = self.loaded.as_deref().and_then(|k| self.index_in_list(k)).map(|i| i as i32);
        let mut idx = match cur {
            Some(i) => i + dir,
            None if dir > 0 => 0,
            None => n - 1,
        };
        for _ in 0..n {
            idx = idx.rem_euclid(n);
            let e = &lib.entries()[self.visible[idx as usize]];
            if !e.is_damaged() {
                let key = e.key.clone();
                self.cursor = Some(key.clone());
                self.load_key(bv, &key);
                self.scroll_to_cursor(false);
                return;
            }
            idx += dir;
        }
    }

    /// Loads the default preset (settings.json) if one is set. True when a load was requested.
    pub fn load_default(&mut self, bv: &mut BusView) -> bool {
        let Some(key) = self.lib.as_ref().and_then(|l| l.default_preset()).map(|e| e.key.clone()) else { return false };
        self.cursor = Some(key.clone());
        self.load_key(bv, &key)
    }

    // ---- commands ----------------------------------------------------------------------------

    fn save_prefs(&mut self) {
        let prefs = BrowserPrefs {
            source: self.filter.source.name().into(),
            category: self.filter.category.unwrap_or("All").into(),
            load_on_select: self.load_on_select,
        };
        if let Some(lib) = &mut self.lib {
            let _ = lib.update_settings(|s| s.browser = prefs);
        }
    }

    fn apply(&mut self, c: &dyn Ctl, bv: &mut BusView, cmd: Cmd) {
        match cmd {
            Cmd::Source(s) => {
                self.filter.source = s;
                self.save_prefs();
            }
            Cmd::Category(cat) => {
                self.filter.category = cat;
                self.save_prefs();
            }
            Cmd::Tag(t) => self.filter.tag = t,
            Cmd::ClearSearch => {
                self.search.clear();
                self.filter.text.clear();
            }
            Cmd::ClearFilters => {
                self.search.clear();
                self.filter = Filter { source: SourceFilter::All, category: None, tag: None, text: String::new() };
                self.save_prefs();
            }
            Cmd::SearchAuthor(a) => {
                self.search = if a.contains(' ') { format!("\"{a}\"") } else { a };
                self.filter.text = self.search.clone();
            }
            Cmd::Select { key, load, close } => {
                self.cursor = Some(key.clone());
                self.scroll_to_cursor(false);
                if load || close {
                    self.load_key(bv, &key);
                }
                if close {
                    self.set_open(false);
                }
            }
            Cmd::Heart(key) => {
                let on = self.lib.as_ref().is_some_and(|l| !l.is_favorite(&key));
                if let Some(Err(e)) = self.lib.as_mut().map(|l| l.set_favorite(&key, on)) {
                    self.say(&format!("Could not save favorites: {e}"), true);
                }
            }
            Cmd::Menu { key, pos } => {
                self.cursor = Some(key.clone());
                self.menu = Some(MenuState { key, pos });
            }
            Cmd::Init => {
                self.load_key(bv, INIT_KEY);
            }
            Cmd::Save => self.begin_save(c),
            Cmd::SaveAs => self.open_save_as(c),
            Cmd::Rescan => {
                if let Some(l) = &mut self.lib {
                    l.rescan();
                    let n = l.entries().len();
                    self.say(&format!("Rescanned: {n} presets."), false);
                }
            }
            Cmd::Reveal => {
                if let Some(l) = &self.lib {
                    let dir = l.user_dir();
                    self.reveal(&dir);
                }
            }
            Cmd::Close => self.set_open(false),
            Cmd::LoadOnSelect(on) => {
                self.load_on_select = on;
                self.save_prefs();
            }
        }
    }

    fn reveal(&mut self, path: &std::path::Path) {
        self.revealed.push(path.to_path_buf());
        if self.spawn_reveal {
            if let Err(e) = library::reveal(path) {
                self.say(&format!("Could not open the folder: {e}"), true);
            }
        }
    }

    fn user_loaded(&self) -> Option<&Entry> {
        self.loaded.as_deref().and_then(|k| self.entry(k)).filter(|e| e.origin == Origin::User && !e.is_damaged())
    }

    /// Save: a loaded user preset is overwritten after a confirmation; anything else is Save As.
    pub fn begin_save(&mut self, c: &dyn Ctl) {
        match self.user_loaded().map(|e| (e.key.clone(), e.name.clone())) {
            Some((key, name)) if c.current_preset().is_some() => self.dialog = Some(Dialog::ConfirmSave { key, name }),
            _ => self.open_save_as(c),
        }
    }

    /// Opens the Save As dialog over the sound as it is now.
    pub fn open_save_as(&mut self, c: &dyn Ctl) {
        let Some(preset) = c.current_preset() else {
            self.say("This host cannot save presets.", true);
            return;
        };
        let base = self.loaded.as_deref().and_then(|k| self.entry(k)).filter(|e| !e.is_damaged());
        let author = self.lib.as_ref().map(|l| l.settings().author.clone()).filter(|a| !a.is_empty()).or_else(|| base.filter(|e| e.origin == Origin::User).map(|e| e.author.clone())).unwrap_or_default();
        let mut name = base.map(|e| e.name.clone()).unwrap_or_default();
        let category = base.map(|e| e.category).or(self.filter.category).unwrap_or(library::DEFAULT_CATEGORY);
        if let (Some(e), Some(l)) = (base, &self.lib) {
            // A copy of a factory preset (or a name already taken) gets a suffix to start from.
            if e.origin == Origin::Factory || l.conflict(category, &name).is_some() {
                name = format!("{name} Copy");
            }
        }
        let mut macros: [String; 4] = std::array::from_fn(|i| preset.macros.get(i).cloned().unwrap_or_default());
        for (i, m) in macros.iter_mut().enumerate() {
            if m.starts_with("Macro ") && m[6..].parse::<usize>().ok() == Some(i + 1) {
                m.clear();
            }
        }
        self.dialog = Some(Dialog::SaveAs(Box::new(SaveAs {
            name,
            category: library::category_index(category).unwrap_or(0),
            author,
            tags: base.map(|e| e.tags.join(", ")).unwrap_or_default(),
            description: base.map(|e| e.description.clone()).unwrap_or_default(),
            macros,
            error: None,
            replace: None,
            fresh: true,
            preset,
        })));
    }

    fn finish_save_as(&mut self, c: &dyn Ctl, bv: &mut BusView, replace: bool) {
        let Some(Dialog::SaveAs(d)) = &mut self.dialog else { return };
        let Some(lib) = &mut self.lib else { return };
        let mut p = d.preset.clone();
        p.name = d.name.clone();
        p.author = d.author.trim().to_string();
        p.description = d.description.trim().to_string();
        p.tags = library::parse_tags(&d.tags);
        let labels: Vec<String> = d.macros.iter().enumerate().map(|(i, m)| if m.trim().is_empty() { format!("Macro {}", i + 1) } else { m.trim().chars().take(library::MAX_LABEL_CHARS).collect() }).collect();
        p.macros = labels.clone();
        let category = CATEGORIES[d.category];
        match lib.save_as(&p, category, replace) {
            Ok(key) => {
                let author = p.author.clone();
                let _ = lib.update_settings(|s| s.author = author);
                let name = p.name.clone();
                self.dialog = None;
                self.loaded = Some(key.clone());
                self.cursor = Some(key);
                self.cache = None;
                self.scroll_to_cursor(false);
                if let Some(m) = c.matrix() {
                    m.edit(|m| {
                        for (slot, l) in m.macros.iter_mut().zip(&labels) {
                            *slot = l.clone();
                        }
                    });
                }
                bv.preset_saved = true;
                self.say(&format!("Saved \u{201c}{name}\u{201d} to User / {category}."), false);
            }
            Err(LibError::Exists(key)) => d.replace = Some(key),
            Err(e) => d.error = Some(e.to_string()),
        }
    }

    fn finish_confirm_save(&mut self, c: &dyn Ctl, bv: &mut BusView, key: &str) {
        let Some(mut p) = c.current_preset() else { return };
        let Some(lib) = &mut self.lib else { return };
        if let Some(base) = lib.get(key).and_then(|e| e.preset.clone()) {
            p.author = base.author.clone();
            p.description = base.description.clone();
            p.tags = base.tags.clone();
        }
        match lib.save(key, &p) {
            Ok(_) => {
                self.dialog = None;
                self.cache = None;
                bv.preset_saved = true;
                let name = self.entry(key).map(|e| e.name.clone()).unwrap_or_default();
                self.say(&format!("Saved \u{201c}{name}\u{201d}."), false);
            }
            Err(e) => {
                self.dialog = None;
                self.say(&e.to_string(), true);
            }
        }
    }

    fn finish_delete(&mut self, key: &str) {
        let Some(lib) = &mut self.lib else { return };
        let name = lib.get(key).map(|e| e.name.clone()).unwrap_or_default();
        self.dialog = None;
        match lib.delete(key) {
            Ok(()) => {
                if self.loaded.as_deref() == Some(key) {
                    self.loaded = None;
                }
                if self.cursor.as_deref() == Some(key) {
                    self.cursor = None;
                }
                self.cache = None;
                self.say(&format!("Deleted \u{201c}{name}\u{201d}."), false);
            }
            Err(e) => self.say(&e.to_string(), true),
        }
    }

    fn finish_rename(&mut self, key: &str, new_name: &str) {
        let Some(lib) = &mut self.lib else { return };
        match lib.rename(key, new_name) {
            Ok(new_key) => {
                if self.loaded.as_deref() == Some(key) {
                    self.loaded = Some(new_key.clone());
                }
                if self.cursor.as_deref() == Some(key) {
                    self.cursor = Some(new_key);
                }
                self.dialog = None;
                self.cache = None;
                self.scroll_to_cursor(false);
                self.say("Renamed.", false);
            }
            Err(e) => {
                if let Some(Dialog::Rename { error, .. }) = &mut self.dialog {
                    *error = Some(e.to_string());
                }
            }
        }
    }

    /// Asks to delete the user preset `key` (nothing for a factory preset or a missing key).
    pub fn begin_delete(&mut self, key: &str) {
        if let Some(e) = self.entry(key).filter(|e| e.origin == Origin::User) {
            self.dialog = Some(Dialog::ConfirmDelete { key: e.key.clone(), name: e.name.clone() });
        }
    }

    /// Opens the Rename dialog for the user preset `key`.
    pub fn begin_rename(&mut self, key: &str) {
        if let Some(e) = self.entry(key).filter(|e| e.origin == Origin::User && !e.is_damaged()) {
            self.dialog = Some(Dialog::Rename { key: e.key.clone(), name: e.name.clone(), error: None, fresh: true });
        }
    }

    fn menu_items(&self, key: &str) -> Vec<(String, MenuAction, bool, bool, bool)> {
        // (label, action, enabled, danger, checked)
        let Some(e) = self.entry(key) else { return vec![] };
        let lib = self.lib.as_ref().expect("an entry implies a library");
        let user = e.origin == Origin::User;
        let is_default = lib.settings().default_preset == key;
        let mut v = vec![
            ("Load".to_string(), MenuAction::Load, !e.is_damaged(), false, false),
            (if lib.is_favorite(key) { "Remove from favorites" } else { "Add to favorites" }.to_string(), MenuAction::Favorite, true, false, lib.is_favorite(key)),
            (if is_default { "Clear default preset" } else { "Set as default" }.to_string(), MenuAction::Default, !e.is_damaged() || is_default, false, is_default),
        ];
        v.push(("Reveal in folder".to_string(), MenuAction::Reveal, e.path.is_some(), false, false));
        if user {
            v.push(("Rename\u{2026}".to_string(), MenuAction::Rename, !e.is_damaged(), false, false));
            v.push(("Delete\u{2026}".to_string(), MenuAction::Delete, true, true, false));
        }
        v
    }

    fn run_menu(&mut self, key: &str, action: MenuAction, bv: &mut BusView) {
        let Some(e) = self.entry(key) else { return };
        let (name, path) = (e.name.clone(), e.path.clone());
        match action {
            MenuAction::Load => {
                self.load_key(bv, key);
            }
            MenuAction::Favorite => {
                let on = self.lib.as_ref().is_some_and(|l| !l.is_favorite(key));
                if let Some(Err(e)) = self.lib.as_mut().map(|l| l.set_favorite(key, on)) {
                    self.say(&format!("Could not save favorites: {e}"), true);
                }
            }
            MenuAction::Default => {
                let set = self.lib.as_ref().is_some_and(|l| l.settings().default_preset != key);
                if let Some(l) = &mut self.lib {
                    let r = l.set_default(set.then_some(key));
                    match r {
                        Ok(()) => self.say(&if set { format!("New instances will start from \u{201c}{name}\u{201d}.") } else { "New instances start from Init.".to_string() }, false),
                        Err(e) => self.say(&e.to_string(), true),
                    }
                }
            }
            MenuAction::Reveal => {
                if let Some(p) = path {
                    self.reveal(&p);
                }
            }
            MenuAction::Rename => self.begin_rename(key),
            MenuAction::Delete => self.begin_delete(key),
        }
    }

    // ---- drawing -----------------------------------------------------------------------------

    /// 0 closed .. 1 open: 150 ms in, 90 ms out, instant under Reduced motion.
    pub fn open_k(&self, ctx: &egui::Context) -> f32 {
        rr_ui::motion::tween(ctx, Id::new("gts-browser-open"), self.open, if self.open { 0.15 } else { 0.09 })
    }

    /// Draws the panel into `body` (window px). `k` is [`Browser::open_k`].
    pub fn show(&mut self, ui: &mut Ui, c: &dyn Ctl, bv: &mut BusView, body: Rect, k: f32) {
        let ctx = ui.ctx().clone();
        self.now = ctx.input(|i| i.time);
        self.poll();
        self.refresh();
        if self.notice.as_ref().is_some_and(|n| n.until < self.now) {
            self.notice = None;
        }
        if std::mem::take(&mut self.focus_search) {
            ctx.memory_mut(|m| m.request_focus(search_id()));
        }
        let modal = self.dialog.is_some() || self.menu.is_some();
        let mut cmds: Vec<Cmd> = vec![];
        let keys = if self.open && !modal { self.read_keys(&ctx) } else { Keys::default() };

        let rise = 6.0 * (1.0 - k);
        let mut bui = ui.new_child(UiBuilder::new().max_rect(body.translate(vec2(0.0, rise))).layout(egui::Layout::top_down(egui::Align::Min)));
        bui.multiply_opacity(k);
        // Swallow pointer input over the whole body so nothing under the panel reacts.
        bui.interact(body, Id::new("gts-br-veil"), Sense::click_and_drag());
        let o = body.min.to_vec2() + vec2(0.0, rise);
        let at = |x: f32, y: f32, w: f32, h: f32| Rect::from_min_size(pos2(x, y) + o, vec2(w, h));
        self.top_bar(&mut bui, bv, &at, &mut cmds);
        self.left_column(&mut bui, &at, &mut cmds);
        self.tag_row(&mut bui, &at, &mut cmds);
        self.list(&mut bui, &at, &mut cmds);
        self.status_row(&mut bui, &at);
        self.detail(&mut bui, &at, &mut cmds);

        // Keys act on the selection after the widgets had their say.
        if self.open && !modal {
            self.handle_keys(&keys, &mut cmds);
        }
        for cmd in cmds {
            self.apply(c, bv, cmd);
        }
        self.search_focused = ctx.memory(|m| m.has_focus(search_id()));
        self.menu_ui(&ctx, body, bv);
        self.dialog_ui(&ctx, c, body, bv);
    }

    fn read_keys(&mut self, ctx: &egui::Context) -> Keys {
        let searching = self.search_focused || ctx.memory(|m| m.has_focus(search_id()));
        let typing_elsewhere = ctx.wants_keyboard_input() && !searching;
        let mut k = Keys::default();
        if typing_elsewhere {
            return k;
        }
        ctx.input_mut(|i| {
            let mut take = |key: Key, mods: Modifiers| i.consume_key(mods, key);
            k.up = take(Key::ArrowUp, Modifiers::NONE);
            k.down = take(Key::ArrowDown, Modifiers::NONE);
            k.page_up = take(Key::PageUp, Modifiers::NONE);
            k.page_down = take(Key::PageDown, Modifiers::NONE);
            k.enter = take(Key::Enter, Modifiers::NONE);
            k.esc = take(Key::Escape, Modifiers::NONE);
            k.find = take(Key::F, Modifiers::COMMAND);
            if !searching {
                k.home = take(Key::Home, Modifiers::NONE);
                k.end = take(Key::End, Modifiers::NONE);
                k.delete = take(Key::Delete, Modifiers::NONE);
            }
            k.typed = !searching && i.events.iter().any(|e| matches!(e, egui::Event::Text(t) if t.chars().any(|c| !c.is_whitespace() && !c.is_control())));
        });
        k.searching = searching;
        // Typing starts a search: focus the box now, so this frame's characters land in it.
        if k.find || k.typed {
            ctx.memory_mut(|m| m.request_focus(search_id()));
        }
        k
    }

    fn handle_keys(&mut self, k: &Keys, cmds: &mut Vec<Cmd>) {
        let n = self.visible.len() as i32;
        let cur = self.cursor.as_deref().and_then(|c| self.index_in_list(c)).map(|i| i as i32);
        let page = (LIST_H / ROW_H) as i32 - 1;
        let target = if k.up {
            Some(cur.map_or(0, |i| i - 1))
        } else if k.down {
            Some(cur.map_or(0, |i| i + 1))
        } else if k.page_up {
            Some(cur.map_or(0, |i| i - page))
        } else if k.page_down {
            Some(cur.map_or(0, |i| i + page))
        } else if k.home {
            Some(0)
        } else if k.end {
            Some(n - 1)
        } else {
            None
        };
        if let (Some(t), Some(lib)) = (target, &self.lib) {
            if n > 0 {
                let t = t.clamp(0, n - 1) as usize;
                let key = lib.entries()[self.visible[t]].key.clone();
                cmds.push(Cmd::Select { key, load: self.load_on_select, close: false });
            }
        }
        if k.enter {
            if let Some(key) = self.cursor.clone() {
                cmds.push(Cmd::Select { key, load: true, close: false });
            }
        }
        if k.delete {
            if let Some(key) = self.cursor.clone() {
                self.begin_delete(&key);
            }
        }
        if k.esc {
            if k.searching && !self.search.is_empty() {
                cmds.push(Cmd::ClearSearch);
            } else {
                cmds.push(Cmd::Close);
            }
        }
    }

    fn top_bar(&mut self, ui: &mut Ui, bv: &mut BusView, at: &dyn Fn(f32, f32, f32, f32) -> Rect, cmds: &mut Vec<Cmd>) {
        // Search.
        let sr = at(0.0, 0.0, 360.0, TOP_H);
        let focused = ui.memory(|m| m.has_focus(search_id()));
        controls::field_box(ui, sr, focused, ui.rect_contains_pointer(sr));
        let p = ui.painter().clone();
        let ic = pos2(sr.left() + 17.0, sr.center().y - 1.0);
        let icol = if focused { T.accent } else { T.muted };
        p.circle_stroke(ic, 5.0, rr_ui::paint::st(1.5, icol));
        p.line_segment([ic + vec2(3.6, 3.6), ic + vec2(8.0, 8.0)], rr_ui::paint::st(1.5, icol));
        let tr = Rect::from_min_max(pos2(sr.left() + 24.0, sr.top()), pos2(sr.right() - 28.0, sr.bottom()));
        let out = controls::text_edit(ui, tr, search_id(), &mut self.search, "Search name, author, tag", 80, false);
        if out.changed {
            self.filter.text = self.search.clone();
        }
        if out.resp.hovered() {
            hover(ui, "SEARCH", &self.search, "browser:search");
        }
        if !self.search.is_empty() {
            let xr = Rect::from_center_size(pos2(sr.right() - 15.0, sr.center().y), vec2(22.0, 22.0));
            let resp = ui.interact(xr, Id::new("gts-br-clear"), Sense::click());
            let col = if resp.hovered() { T.text } else { T.muted };
            let c = xr.center();
            p.line_segment([c + vec2(-3.5, -3.5), c + vec2(3.5, 3.5)], rr_ui::paint::st(1.5, col));
            p.line_segment([c + vec2(-3.5, 3.5), c + vec2(3.5, -3.5)], rr_ui::paint::st(1.5, col));
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                hover(ui, "CLEAR", "", "browser:clear");
            }
            if resp.clicked() {
                cmds.push(Cmd::ClearSearch);
                self.focus_search = true;
            }
        }

        // Buttons, right to left: close, Save As, Save, Init.
        let bh = 26.0;
        let by = (TOP_H - bh) / 2.0;
        let cr = at(912.0 - 30.0, 0.0, 30.0, TOP_H);
        let resp = controls::button(ui, Rect::from_center_size(cr.center(), vec2(30.0, bh)), "", ButtonKind::Normal, true);
        let cc = cr.center();
        for s in [1.0, -1.0] {
            ui.painter().line_segment([cc + vec2(-4.0, -4.0 * s), cc + vec2(4.0, 4.0 * s)], rr_ui::paint::st(1.5, T.text));
        }
        if resp.hovered() {
            hover(ui, "CLOSE", "", "browser:close");
        }
        if resp.clicked() {
            cmds.push(Cmd::Close);
        }
        let mut x = 912.0 - 30.0 - 8.0;
        for (label, kind, key, cmd) in [
            ("Save As\u{2026}", ButtonKind::Normal, "browser:saveas", Cmd::SaveAs),
            ("Save", if bv.preset_dirty && self.user_loaded().is_some() { ButtonKind::Primary } else { ButtonKind::Normal }, "browser:save", Cmd::Save),
            ("Init", ButtonKind::Normal, "browser:init", Cmd::Init),
        ] {
            let w = controls::button_width(ui.painter(), label);
            x -= w;
            let r = at(x, by, w, bh);
            let resp = controls::button(ui, r, label, kind, true);
            if resp.hovered() {
                hover(ui, &label.trim_end_matches('\u{2026}').to_uppercase(), "", key);
            }
            if resp.clicked() {
                cmds.push(cmd);
            }
            x -= 8.0;
        }
    }

    fn left_column(&mut self, ui: &mut Ui, at: &dyn Fn(f32, f32, f32, f32) -> Rect, cmds: &mut Vec<Cmd>) {
        let col = at(0.0, TAGS_Y, LEFT_W, 704.0 - TAGS_Y);
        let p = ui.painter().clone();
        rr_ui::paint::highlight_below(&p, col, GROUP_R);
        p.rect_filled(col, GROUP_R, T.well);
        p.rect_stroke(col, GROUP_R, rr_ui::paint::hair(&p, T.lo()), StrokeKind::Inside);
        let caption = |y: f32, text: &str| {
            rr_ui::paint::tracked(&p, pos2(col.left() + 14.0, col.top() + y), Align2::LEFT_CENTER, text, font::strong(11.0), T.title, 1.4);
        };
        caption(18.0, "SOURCE");
        let c = self.counts.clone();
        for (i, s) in SourceFilter::ALL.into_iter().enumerate() {
            let r = Rect::from_min_size(pos2(col.left() + 6.0, col.top() + 34.0 + i as f32 * NAV_H), vec2(LEFT_W - 12.0, NAV_H));
            let resp = nav_row(ui, r, Id::new(("gts-br-src", i)), s.name(), c.sources[i], self.filter.source == s);
            if resp.hovered() {
                hover(ui, "SOURCE", s.name(), "browser:source");
            }
            if resp.clicked() {
                cmds.push(Cmd::Source(s));
            }
        }
        let cy = 34.0 + 4.0 * NAV_H + 18.0;
        caption(cy, "CATEGORY");
        for i in 0..=CATEGORIES.len() {
            let (label, count, cat) = if i == 0 { ("All", c.all_categories, None) } else { (CATEGORIES[i - 1], c.categories[i - 1], Some(CATEGORIES[i - 1])) };
            let r = Rect::from_min_size(pos2(col.left() + 6.0, col.top() + cy + 16.0 + i as f32 * NAV_H), vec2(LEFT_W - 12.0, NAV_H));
            let resp = nav_row(ui, r, Id::new(("gts-br-cat", i)), label, count, self.filter.category == cat);
            if resp.hovered() {
                hover(ui, "CATEGORY", label, "browser:category");
            }
            if resp.clicked() {
                cmds.push(Cmd::Category(cat));
            }
        }
        let by = cy + 16.0 + (CATEGORIES.len() + 1) as f32 * NAV_H + 14.0;
        rr_ui::paint::hline(&p, col.left() + 12.0, col.right() - 12.0, col.top() + by - 8.0, Color32::WHITE.gamma_multiply(0.08));
        let lr = Rect::from_min_size(pos2(col.left() + 8.0, col.top() + by), vec2(LEFT_W - 16.0, CONTROL_H));
        let (pick, hovered) = place(ui, lr, |ui| led_toggle_ex(ui, "Load on select", self.load_on_select));
        if hovered {
            hover(ui, "LOAD ON SELECT", if self.load_on_select { "On" } else { "Off" }, "browser:load");
        }
        if let Some(on) = pick {
            cmds.push(Cmd::LoadOnSelect(on));
        }
        for (i, (label, key, cmd)) in [("Rescan", "browser:rescan", Cmd::Rescan), ("Reveal folder", "browser:reveal", Cmd::Reveal)].into_iter().enumerate() {
            let r = Rect::from_min_size(pos2(col.left() + 8.0, col.top() + by + 34.0 + i as f32 * 32.0), vec2(LEFT_W - 16.0, 26.0));
            let resp = controls::button(ui, r, label, ButtonKind::Normal, self.lib.is_some());
            if resp.hovered() {
                hover(ui, &label.to_uppercase(), "", key);
            }
            if resp.clicked() {
                cmds.push(cmd);
            }
        }
        let total = self.lib.as_ref().map_or(0, |l| l.entries().len());
        p.text(pos2(col.left() + 14.0, col.bottom() - 16.0), Align2::LEFT_CENTER, format!("{total} presets in library"), font::label(11.0), T.muted);
    }

    fn tag_row(&mut self, ui: &mut Ui, at: &dyn Fn(f32, f32, f32, f32) -> Rect, cmds: &mut Vec<Cmd>) {
        if self.tag_list.is_empty() && self.filter.tag.is_none() {
            return;
        }
        let row = at(MAIN_X, TAGS_Y, MAIN_W, CHIP_H + 2.0);
        let p = ui.painter().clone();
        rr_ui::paint::tracked(&p, pos2(row.left() + 2.0, row.center().y), Align2::LEFT_CENTER, "TAGS", font::strong(11.0), T.title, 1.4);
        let mut x = row.left() + 46.0;
        let mut tags: Vec<&(String, usize)> = self.tag_list.iter().collect();
        // A tag filter that no result carries any more still shows, lit, so it can be cleared.
        let extra = self.filter.tag.clone().filter(|t| !tags.iter().any(|(n, _)| n.eq_ignore_ascii_case(t)));
        let mut shown = 0;
        if let Some(t) = &extra {
            let w = controls::chip_width(&p, t);
            let r = Rect::from_min_size(pos2(x, row.top() + 1.0), vec2(w, CHIP_H));
            if controls::chip(ui, r, t, true).clicked() {
                cmds.push(Cmd::Tag(None));
            }
            x += w + 6.0;
        }
        let limit = row.right() - 40.0;
        for (name, _) in tags.drain(..) {
            let w = controls::chip_width(&p, name);
            if x + w > limit {
                break;
            }
            let active = self.filter.tag.as_deref().is_some_and(|t| t.eq_ignore_ascii_case(name));
            let r = Rect::from_min_size(pos2(x, row.top() + 1.0), vec2(w, CHIP_H));
            let resp = controls::chip(ui, r, name, active);
            if resp.hovered() {
                hover(ui, "TAG", name, "browser:tag");
            }
            if resp.clicked() {
                cmds.push(Cmd::Tag(if active { None } else { Some(name.clone()) }));
            }
            x += w + 6.0;
            shown += 1;
        }
        let more = self.tag_list.len().saturating_sub(shown);
        if more > 0 {
            p.text(pos2(row.right() - 4.0, row.center().y), Align2::RIGHT_CENTER, format!("+{more}"), font::label(12.0), T.muted);
        }
    }

    fn list(&mut self, ui: &mut Ui, at: &dyn Fn(f32, f32, f32, f32) -> Rect, cmds: &mut Vec<Cmd>) {
        let lr = at(MAIN_X, LIST_Y, MAIN_W, LIST_H);
        let p = ui.painter().clone();
        rr_ui::paint::highlight_below(&p, lr, GROUP_R);
        p.rect_filled(lr, GROUP_R, T.well);
        let Some(lib) = &self.lib else {
            p.text(lr.center(), Align2::CENTER_CENTER, "Loading presets\u{2026}", font::label(14.0), T.muted);
            p.rect_stroke(lr, GROUP_R, rr_ui::paint::hair(&p, T.lo()), StrokeKind::Inside);
            return;
        };
        let n = self.visible.len();
        let content_h = n as f32 * ROW_H;
        let max = (content_h - LIST_H).max(0.0);
        // Wheel over the list, then ease toward the target.
        let ctx = ui.ctx().clone();
        if ui.rect_contains_pointer(lr) && self.dialog.is_none() && self.menu.is_none() {
            let dy = ctx.input(|i| i.smooth_scroll_delta.y);
            if dy != 0.0 {
                self.scroll_target = (self.scroll_target - dy).clamp(0.0, max);
                if rr_ui::motion::reduced_motion(&ctx) {
                    self.scroll = self.scroll_target;
                }
            }
        }
        let dt = ctx.input(|i| i.stable_dt).clamp(0.001, 0.05);
        self.scroll = if rr_ui::motion::reduced_motion(&ctx) || (self.scroll_target - self.scroll).abs() < 0.4 {
            self.scroll_target
        } else {
            self.scroll + (self.scroll_target - self.scroll) * (1.0 - (-dt * 26.0).exp())
        };
        self.scroll = self.scroll.clamp(0.0, max);
        let inner = lr.shrink(1.0);
        let clip = p.with_clip_rect(inner);
        let sb_w = if max > 0.0 { controls::SCROLLBAR_W + 4.0 } else { 0.0 };
        let row_w = lr.width() - sb_w;
        if n == 0 {
            let filtered = self.filter != Filter { source: SourceFilter::All, category: None, tag: None, text: String::new() };
            p.text(lr.center() - vec2(0.0, 14.0), Align2::CENTER_CENTER, "No presets match.", font::strong(15.0), T.text);
            p.text(lr.center() + vec2(0.0, 8.0), Align2::CENTER_CENTER, "Clear the search, or pick another source or category.", font::label(12.0), T.muted);
            if filtered {
                let r = Rect::from_center_size(lr.center() + vec2(0.0, 44.0), vec2(120.0, 26.0));
                if controls::button(ui, r, "Clear filters", ButtonKind::Normal, true).clicked() {
                    cmds.push(Cmd::ClearFilters);
                }
            }
        }
        let first = (self.scroll / ROW_H).floor().max(0.0) as usize;
        let last = (((self.scroll + LIST_H) / ROW_H).ceil() as usize).min(n);
        for vi in first..last {
            let e = &lib.entries()[self.visible[vi]];
            let top = lr.top() + vi as f32 * ROW_H - self.scroll;
            let row = Rect::from_min_size(pos2(lr.left(), top), vec2(row_w, ROW_H));
            let hit = row.intersect(inner);
            let id = Id::new(("gts-br-row", e.key.as_str()));
            let resp = ui.interact(hit, id, Sense::click());
            let heart_r = Rect::from_center_size(pos2(row.right() - 22.0, row.center().y), vec2(28.0, 28.0));
            let heart_resp = ui.interact(heart_r.intersect(inner), Id::new(("gts-br-heart", e.key.as_str())), Sense::click());
            let cursor = self.cursor.as_deref() == Some(e.key.as_str());
            let is_loaded = self.loaded.as_deref() == Some(e.key.as_str());
            let fav = lib.is_favorite(&e.key);
            if resp.hovered() || heart_resp.hovered() {
                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if cursor {
                clip.rect_filled(row, 0.0, T.a(0.16));
                clip.rect_filled(Rect::from_min_size(row.min, vec2(2.0, ROW_H)), 0.0, T.accent);
            } else if resp.hovered() || heart_resp.hovered() {
                clip.rect_filled(row, 0.0, Color32::WHITE.gamma_multiply(0.04));
            }
            if vi > 0 {
                clip.hline(row.x_range().shrink(10.0), row.top(), rr_ui::paint::hair(&p, Color32::WHITE.gamma_multiply(0.05)));
            }
            draw_row(&clip, e, row, is_loaded, fav, heart_resp.hovered());
            let mut badge_hot = false;
            if let Some(problem) = &e.problem {
                let bx = row.left() + 22.0 + clip.layout_no_wrap(e.name.clone(), font::label(14.0), T.text).size().x + 12.0;
                let bc = pos2(bx.min(row.right() - 250.0), row.top() + 14.0);
                let col = if e.is_damaged() { SRC_E } else { SRC_A };
                clip.circle_filled(bc, 7.0, col.gamma_multiply(0.18));
                clip.circle_stroke(bc, 7.0, rr_ui::paint::hair(&p, col.gamma_multiply(0.7)));
                clip.text(bc, Align2::CENTER_CENTER, "!", font::strong(11.0), col);
                if ui.rect_contains_pointer(Rect::from_center_size(bc, vec2(16.0, 16.0)).intersect(inner)) {
                    badge_hot = true;
                    hover(ui, if e.is_damaged() { "DAMAGED FILE" } else { "FILE NOTE" }, problem, "browser:warn");
                }
            }
            if heart_resp.hovered() {
                hover(ui, "FAVORITE", if fav { "On" } else { "Off" }, "browser:heart");
            } else if resp.hovered() && !badge_hot {
                hover(ui, &e.category.to_uppercase(), &e.name, "browser:list");
            }
            if heart_resp.clicked() {
                cmds.push(Cmd::Heart(e.key.clone()));
            } else if resp.double_clicked() {
                // A damaged file does not load: the notice says why, and the browser stays open.
                cmds.push(Cmd::Select { key: e.key.clone(), load: true, close: !e.is_damaged() });
            } else if resp.clicked() {
                cmds.push(Cmd::Select { key: e.key.clone(), load: self.load_on_select, close: false });
            } else if resp.secondary_clicked() {
                let pos = resp.interact_pointer_pos().unwrap_or(row.center());
                cmds.push(Cmd::Menu { key: e.key.clone(), pos });
            }
        }
        p.rect_stroke(lr, GROUP_R, rr_ui::paint::hair(&p, T.lo()), StrokeKind::Inside);
        let track = Rect::from_min_max(pos2(lr.right() - controls::SCROLLBAR_W - 3.0, lr.top() + 4.0), pos2(lr.right() - 3.0, lr.bottom() - 4.0));
        let mut off = self.scroll;
        if controls::scrollbar(ui, track, Id::new("gts-br-scroll"), content_h, LIST_H, &mut off) {
            self.scroll = off;
            self.scroll_target = off;
        }
    }

    fn status_row(&mut self, ui: &mut Ui, at: &dyn Fn(f32, f32, f32, f32) -> Rect) {
        let r = at(MAIN_X, STATUS_Y, MAIN_W, 20.0);
        let p = ui.painter().clone();
        let total = self.lib.as_ref().map_or(0, |l| l.entries().len());
        let text = if self.lib.is_some() { format!("{} of {} presets", self.visible.len(), total) } else { "Loading\u{2026}".to_string() };
        p.text(pos2(r.left() + 4.0, r.center().y), Align2::LEFT_CENTER, text, font::label(12.0), T.muted);
        if let Some(n) = &self.notice {
            let col = if n.error { SRC_E } else { T.accent };
            let a = ((n.until - self.now) / 0.4).clamp(0.0, 1.0) as f32;
            let s = rr_ui::paint::elide(&p, &font::label(12.0), &n.text, r.width() - 200.0);
            p.text(pos2(r.right() - 4.0, r.center().y), Align2::RIGHT_CENTER, s, font::label(12.0), col.gamma_multiply(a));
        }
    }

    fn detail(&mut self, ui: &mut Ui, at: &dyn Fn(f32, f32, f32, f32) -> Rect, cmds: &mut Vec<Cmd>) {
        let dr = at(MAIN_X, DETAIL_Y, MAIN_W, DETAIL_H);
        let p = ui.painter().clone();
        rr_ui::paint::highlight_below(&p, dr, GROUP_R);
        p.rect_filled(dr, GROUP_R, T.well);
        p.rect_stroke(dr, GROUP_R, rr_ui::paint::hair(&p, T.lo()), StrokeKind::Inside);
        let key = self.cursor.clone().or_else(|| self.loaded.clone()).filter(|k| k != INIT_KEY);
        let Some(e) = key.as_deref().and_then(|k| self.entry(k)).cloned() else {
            let msg = if self.loaded.as_deref() == Some(INIT_KEY) { "Init: every setting at its default and an empty modulation matrix." } else { "Select a preset to read about it." };
            p.text(dr.center(), Align2::CENTER_CENTER, msg, font::label(13.0), T.muted);
            return;
        };
        if self.detail_key.as_deref() != Some(e.key.as_str()) {
            self.detail_key = Some(e.key.clone());
            self.detail_scroll = 0.0;
        }
        // Header line: name, then category . origin . author.
        let pad = 14.0;
        let nr = p.text(pos2(dr.left() + pad, dr.top() + 20.0), Align2::LEFT_CENTER, &e.name, font::strong(16.0), T.text);
        let sub = format!("{}  \u{b7}  {}", e.label.replace('/', " / "), if e.origin == Origin::Factory { "Factory" } else { "User" });
        let sr = p.text(pos2(nr.right() + 12.0, dr.top() + 21.0), Align2::LEFT_CENTER, sub.to_uppercase(), font::label(11.0), T.muted);
        if !e.author.is_empty() {
            let ar = Rect::from_min_size(pos2(sr.right() + 12.0, dr.top() + 10.0), vec2(8.0 + rr_ui::paint::text_width(&p, &font::label(12.0), &format!("by {}", e.author), 0.0), 20.0));
            let resp = ui.interact(ar, Id::new("gts-br-author"), Sense::click());
            let col = if resp.hovered() { T.accent } else { T.muted };
            p.text(pos2(ar.left(), dr.top() + 21.0), Align2::LEFT_CENTER, format!("by {}", e.author), font::label(12.0), col);
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                hover(ui, "AUTHOR", &e.author, "browser:author");
            }
            if resp.clicked() {
                cmds.push(Cmd::SearchAuthor(e.author.clone()));
            }
        }
        // Description: wrapped, scrolls inside its box.
        let col_x = dr.left() + pad;
        let text_w = 470.0;
        let body = Rect::from_min_size(pos2(col_x, dr.top() + 38.0), vec2(text_w + 12.0, DETAIL_H - 46.0));
        let text = e.problem.as_ref().filter(|_| e.is_damaged()).cloned().unwrap_or_else(|| if e.description.is_empty() { "No description.".to_string() } else { e.description.clone() });
        let mut job = egui::text::LayoutJob::single_section(text, egui::TextFormat { font_id: font::label(12.5), color: if e.is_damaged() { SRC_E } else { T.text.gamma_multiply(0.86) }, line_height: Some(16.5), ..Default::default() });
        job.wrap.max_width = text_w;
        let galley = p.layout_job(job);
        let max = (galley.size().y - body.height()).max(0.0);
        if ui.rect_contains_pointer(dr) && self.dialog.is_none() && self.menu.is_none() {
            let dy = ui.ctx().input(|i| i.smooth_scroll_delta.y);
            self.detail_scroll = (self.detail_scroll - dy).clamp(0.0, max);
        }
        self.detail_scroll = self.detail_scroll.clamp(0.0, max);
        p.with_clip_rect(body).galley(body.min - vec2(0.0, self.detail_scroll), galley.clone(), T.text);
        let track = Rect::from_min_max(pos2(body.right() + 2.0, body.top()), pos2(body.right() + 2.0 + controls::SCROLLBAR_W, body.bottom()));
        controls::scrollbar(ui, track, Id::new("gts-br-detail-scroll"), galley.size().y, body.height(), &mut self.detail_scroll);
        if ui.rect_contains_pointer(body) {
            hover(ui, "DESCRIPTION", &e.name, "browser:detail");
        }
        // Right column: macros, tags.
        let rx = dr.left() + pad + text_w + 40.0;
        let rw = dr.right() - rx - pad;
        rr_ui::paint::tracked(&p, pos2(rx, dr.top() + 20.0), Align2::LEFT_CENTER, "MACROS", font::strong(11.0), T.title, 1.4);
        for (i, l) in e.macros.iter().enumerate() {
            let y = dr.top() + 42.0 + i as f32 * 20.0;
            p.circle_filled(pos2(rx + 7.0, y), 7.0, SRC_M.gamma_multiply(0.22));
            p.circle_stroke(pos2(rx + 7.0, y), 7.0, rr_ui::paint::hair(&p, SRC_M.gamma_multiply(0.8)));
            p.text(pos2(rx + 7.0, y), Align2::CENTER_CENTER, (i + 1).to_string(), font::strong(10.0), SRC_M);
            let s = rr_ui::paint::elide(&p, &font::label(12.5), l, rw - 24.0);
            p.text(pos2(rx + 22.0, y), Align2::LEFT_CENTER, s, font::label(12.5), T.text);
        }
        rr_ui::paint::tracked(&p, pos2(rx, dr.top() + 140.0), Align2::LEFT_CENTER, "TAGS", font::strong(11.0), T.title, 1.4);
        let (mut x, mut y) = (rx, dr.top() + 154.0);
        for t in &e.tags {
            let w = controls::chip_width(&p, t);
            if x + w > rx + rw {
                x = rx;
                y += CHIP_H + 4.0;
            }
            if y + CHIP_H > dr.bottom() - 4.0 {
                break;
            }
            let active = self.filter.tag.as_deref().is_some_and(|f| f.eq_ignore_ascii_case(t));
            let resp = controls::chip(ui, Rect::from_min_size(pos2(x, y), vec2(w, CHIP_H)), t, active);
            if resp.hovered() {
                hover(ui, "TAG", t, "browser:tag");
            }
            if resp.clicked() {
                cmds.push(Cmd::Tag(if active { None } else { Some(t.clone()) }));
            }
            x += w + 6.0;
        }
    }

    fn menu_ui(&mut self, ctx: &egui::Context, _body: Rect, bv: &mut BusView) {
        let Some(m) = self.menu.as_ref().map(|m| (m.key.clone(), m.pos)) else { return };
        let rows = self.menu_items(&m.0);
        if rows.is_empty() {
            self.menu = None;
            return;
        }
        let items: Vec<MenuItem> = rows
            .iter()
            .map(|(label, action, enabled, danger, checked)| {
                let mut it = MenuItem::new(label).enabled(*enabled).checked(*checked);
                if *danger {
                    it = it.danger();
                }
                if matches!(action, MenuAction::Reveal) {
                    it = it.sep();
                }
                it
            })
            .collect();
        let name = self.entry(&m.0).map(|e| e.name.clone()).unwrap_or_default();
        ctx.data_mut(|d| d.insert_temp(hover_id(), Hover { name: "PRESET".into(), value: name, line: crate::help::ui_help("browser:menu"), ..Default::default() }));
        let k = rr_ui::motion::tween(ctx, Id::new("gts-br-menu-k"), true, 0.09);
        match controls::context_menu(ctx, Id::new("gts-br-menu"), m.1, &items, k) {
            MenuEvent::Open => {}
            MenuEvent::Dismissed => self.menu = None,
            MenuEvent::Picked(i) => {
                self.menu = None;
                let action = rows[i].1;
                self.run_menu(&m.0, action, bv);
            }
        }
    }

    fn dialog_ui(&mut self, ctx: &egui::Context, c: &dyn Ctl, body: Rect, bv: &mut BusView) {
        if self.dialog.is_none() {
            // Re-arm the fade for the next dialog.
            let _ = rr_ui::motion::tween(ctx, Id::new("gts-br-dialog-k"), false, 0.0);
            return;
        }
        let k = rr_ui::motion::tween(ctx, Id::new("gts-br-dialog-k"), true, 0.12);
        let (enter, esc) = ctx.input(|i| (i.key_pressed(Key::Enter), i.key_pressed(Key::Escape)));
        let mut close = esc;
        // Take the dialog out so its UI can borrow the browser's other fields.
        let mut dialog = self.dialog.take().expect("checked above");
        let mut action: Option<DialogAction> = None;
        let backdrop;
        match &mut dialog {
            Dialog::SaveAs(d) => {
                let confirming = d.replace.clone();
                let size = if confirming.is_some() { vec2(CONFIRM_SIZE.0, CONFIRM_SIZE.1) } else { vec2(SAVE_AS_SIZE.0, SAVE_AS_SIZE.1) };
                backdrop = controls::modal(ctx, Id::new("gts-br-dialog"), body, size, if confirming.is_some() { "Replace preset" } else { "Save as" }, k, |ui, inner| {
                    if let Some(key) = &confirming {
                        let name = key.rsplit('/').next().unwrap_or("");
                        let cat = key.split('/').nth(1).unwrap_or("");
                        let msg = format!("\u{201c}{name}\u{201d} already exists in User / {cat}. Replacing it cannot be undone.");
                        wrapped(ui, inner.min, inner.width(), &msg, T.text, 13.5);
                        let (cancel, ok) = footer(ui, inner, "Back", "Replace", ButtonKind::Danger, true);
                        if cancel {
                            action = Some(DialogAction::Back);
                        }
                        if ok || enter {
                            action = Some(DialogAction::Replace);
                        }
                    } else {
                        save_as_form(ui, inner, d, enter, &mut action);
                    }
                });
                if confirming.is_some() {
                    close = false; // Esc steps back (below), it does not cancel the whole save
                    if esc {
                        action = Some(DialogAction::Back);
                    }
                }
            }
            Dialog::ConfirmSave { key, name } => {
                let (key, name) = (key.clone(), name.clone());
                backdrop = controls::modal(ctx, Id::new("gts-br-dialog"), body, vec2(CONFIRM_SIZE.0, CONFIRM_SIZE.1), "Save preset", k, |ui, inner| {
                    wrapped(ui, inner.min, inner.width(), &format!("Overwrite \u{201c}{name}\u{201d} with the sound as it is now?"), T.text, 13.5);
                    let (cancel, ok) = footer(ui, inner, "Cancel", "Save", ButtonKind::Primary, true);
                    if cancel {
                        action = Some(DialogAction::Cancel);
                    }
                    if ok || enter {
                        action = Some(DialogAction::ConfirmSave(key.clone()));
                    }
                });
            }
            Dialog::ConfirmDelete { key, name } => {
                let (key, name) = (key.clone(), name.clone());
                backdrop = controls::modal(ctx, Id::new("gts-br-dialog"), body, vec2(CONFIRM_SIZE.0, CONFIRM_SIZE.1), "Delete preset", k, |ui, inner| {
                    wrapped(ui, inner.min, inner.width(), &format!("Delete \u{201c}{name}\u{201d}? The file is removed from your User folder and cannot be recovered."), T.text, 13.5);
                    let (cancel, ok) = footer(ui, inner, "Cancel", "Delete", ButtonKind::Danger, true);
                    if cancel {
                        action = Some(DialogAction::Cancel);
                    }
                    if ok || enter {
                        action = Some(DialogAction::ConfirmDelete(key.clone()));
                    }
                });
            }
            Dialog::Rename { key, name, error, fresh } => {
                let key = key.clone();
                backdrop = controls::modal(ctx, Id::new("gts-br-dialog"), body, vec2(440.0, 184.0), "Rename preset", k, |ui, inner| {
                    field_label(ui, inner.min, "NAME");
                    let fr = Rect::from_min_size(inner.min + vec2(0.0, 16.0), vec2(inner.width(), 30.0));
                    let id = Id::new("gts-br-rename");
                    take_focus(ui, id, fresh);
                    let out = controls::text_field(ui, fr, id, name, "Preset name", library::MAX_NAME_CHARS + 8, false);
                    if out.resp.hovered() {
                        hover(ui, "NAME", name, "dialog:name");
                    }
                    if out.changed {
                        *error = None;
                    }
                    if let Some(e) = error.as_deref().map(str::to_string).or_else(|| library::validate_name(name).err().filter(|_| !name.is_empty()).map(|e| e.to_string())) {
                        ui.painter().text(pos2(inner.left(), fr.bottom() + 14.0), Align2::LEFT_CENTER, e, font::label(12.0), SRC_E);
                    }
                    let ok_enabled = library::validate_name(name).is_ok();
                    let (cancel, ok) = footer(ui, inner, "Cancel", "Rename", ButtonKind::Primary, ok_enabled);
                    if cancel {
                        action = Some(DialogAction::Cancel);
                    }
                    if (ok || out.enter) && ok_enabled {
                        action = Some(DialogAction::Rename(key.clone(), name.clone()));
                    }
                });
            }
        }
        if backdrop || close {
            action = action.or(Some(DialogAction::Cancel));
        }
        // Escape on the replace step goes back; a backdrop click there is ignored.
        if backdrop && matches!(&dialog, Dialog::SaveAs(d) if d.replace.is_some()) {
            action = Some(DialogAction::Back);
        }
        self.dialog = Some(dialog);
        match action {
            None => {}
            Some(DialogAction::Cancel) => self.dialog = None,
            Some(DialogAction::Back) => {
                if let Some(Dialog::SaveAs(d)) = &mut self.dialog {
                    d.replace = None;
                    d.fresh = true;
                }
            }
            Some(DialogAction::Save) => self.finish_save_as(c, bv, false),
            Some(DialogAction::Replace) => self.finish_save_as(c, bv, true),
            Some(DialogAction::ConfirmSave(key)) => self.finish_confirm_save(c, bv, &key),
            Some(DialogAction::ConfirmDelete(key)) => self.finish_delete(&key),
            Some(DialogAction::Rename(key, name)) => self.finish_rename(&key, &name),
        }
    }
}

#[derive(Clone, Copy)]
enum MenuAction {
    Load,
    Favorite,
    Default,
    Reveal,
    Rename,
    Delete,
}

enum DialogAction {
    Cancel,
    Back,
    Save,
    Replace,
    ConfirmSave(String),
    ConfirmDelete(String),
    Rename(String, String),
}

/// The keys of one frame the browser acts on.
#[derive(Default)]
struct Keys {
    up: bool,
    down: bool,
    page_up: bool,
    page_down: bool,
    home: bool,
    end: bool,
    enter: bool,
    esc: bool,
    delete: bool,
    find: bool,
    typed: bool,
    searching: bool,
}

// ---- pieces ------------------------------------------------------------------------------------

/// A Source / Category row: label left, count right; lit rows get the accent bar.
fn nav_row(ui: &mut Ui, r: Rect, id: Id, label: &str, count: usize, active: bool) -> egui::Response {
    let resp = ui.interact(r, id, Sense::click());
    let p = ui.painter();
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if active {
        p.rect_filled(r, 4.0, T.a(0.16));
        p.rect_filled(Rect::from_min_size(r.min + vec2(0.0, 5.0), vec2(2.0, r.height() - 10.0)), 1.0, T.accent);
    } else if resp.hovered() {
        p.rect_filled(r, 4.0, Color32::WHITE.gamma_multiply(0.04));
    }
    let dim = count == 0 && !active;
    let col = if active { T.accent } else if dim { T.muted.gamma_multiply(0.6) } else if resp.hovered() { T.text } else { T.text.gamma_multiply(0.86) };
    p.text(pos2(r.left() + 12.0, r.center().y), Align2::LEFT_CENTER, label, font::label(13.0), col);
    p.text(pos2(r.right() - 10.0, r.center().y), Align2::RIGHT_CENTER, count.to_string(), font::mono(11.0), if active { T.accent } else { T.muted });
    resp
}

/// One list row's content (the backgrounds are the caller's).
fn draw_row(p: &egui::Painter, e: &Entry, row: Rect, loaded: bool, fav: bool, heart_hot: bool) {
    let name_col = if e.is_damaged() { SRC_E } else if loaded { T.accent } else { T.text };
    if loaded {
        p.circle_filled(pos2(row.left() + 12.0, row.top() + 14.0), 2.5, T.accent);
    }
    p.text(pos2(row.left() + 22.0, row.top() + 14.0), Align2::LEFT_CENTER, &e.name, font::label(14.0), name_col);
    let desc_w = row.width() - 22.0 - 250.0;
    let line = if e.is_damaged() { e.problem.as_deref().unwrap_or("") } else { e.first_sentence() };
    let s = rr_ui::paint::elide(p, &font::label(12.0), line, desc_w);
    p.text(pos2(row.left() + 22.0, row.top() + 29.0), Align2::LEFT_CENTER, s, font::label(12.0), T.muted);
    // Category over origin, then author, then the heart.
    let cx = row.right() - 232.0;
    rr_ui::paint::tracked(p, pos2(cx, row.top() + 14.0), Align2::LEFT_CENTER, &e.category.to_uppercase(), font::label(11.0), T.text.gamma_multiply(0.8), 0.8);
    p.text(pos2(cx, row.top() + 29.0), Align2::LEFT_CENTER, if e.origin == Origin::Factory { "Factory" } else { "User" }, font::label(11.0), T.muted.gamma_multiply(0.85));
    let ax = row.right() - 150.0;
    let author = rr_ui::paint::elide(p, &font::label(12.0), &e.author, 100.0);
    p.text(pos2(ax, row.center().y), Align2::LEFT_CENTER, author, font::label(12.0), T.muted);
    let hc = pos2(row.right() - 22.0, row.center().y);
    let col = if fav { T.accent } else if heart_hot { T.text } else { T.muted.gamma_multiply(0.7) };
    controls::heart(p, hc, 13.0, fav, col);
}

/// Asks for focus on `id` until it has it (`wanted` is then cleared). A dialog's first frame is an
/// invisible sizing pass in which egui drops a focus request, so a single request is not enough.
fn take_focus(ui: &Ui, id: Id, wanted: &mut bool) {
    if *wanted {
        if ui.memory(|m| m.has_focus(id)) {
            *wanted = false; // a request of an earlier frame stuck
        } else {
            ui.memory_mut(|m| m.request_focus(id));
        }
    }
}

fn field_label(ui: &Ui, pos: Pos2, text: &str) {
    rr_ui::paint::tracked(ui.painter(), pos + vec2(0.0, 7.0), Align2::LEFT_CENTER, text, font::label(11.0), T.muted, 0.8);
}

/// Wrapped text at `pos`.
fn wrapped(ui: &Ui, pos: Pos2, width: f32, text: &str, color: Color32, size: f32) {
    let g = ui.painter().layout(text.to_string(), font::label(size), color, width);
    ui.painter().galley(pos, g, color);
}

/// Cancel and confirm buttons at the bottom right of a dialog's inner rect. Returns (cancel, confirm).
fn footer(ui: &mut Ui, inner: Rect, cancel: &str, confirm: &str, kind: ButtonKind, enabled: bool) -> (bool, bool) {
    let (cw, kw) = (controls::button_width(ui.painter(), cancel), controls::button_width(ui.painter(), confirm).max(84.0));
    let y = inner.bottom() - 28.0;
    let kr = Rect::from_min_size(pos2(inner.right() - kw, y), vec2(kw, 28.0));
    let cr = Rect::from_min_size(pos2(kr.left() - 8.0 - cw, y), vec2(cw, 28.0));
    let a = controls::button(ui, cr, cancel, ButtonKind::Normal, true);
    let b = controls::button(ui, kr, confirm, kind, enabled);
    if a.hovered() {
        hover(ui, &cancel.to_uppercase(), "", "dialog:cancel");
    }
    if b.hovered() {
        hover(ui, &confirm.to_uppercase(), "", "dialog:confirm");
    }
    (a.clicked(), b.clicked())
}

fn save_as_form(ui: &mut Ui, inner: Rect, d: &mut SaveAs, enter: bool, action: &mut Option<DialogAction>) {
    let w = inner.width();
    let mut y = inner.top();
    // Name.
    field_label(ui, pos2(inner.left(), y), "NAME");
    let nr = Rect::from_min_size(pos2(inner.left(), y + 16.0), vec2(w, 30.0));
    let nid = Id::new("gts-br-sa-name");
    take_focus(ui, nid, &mut d.fresh);
    let out = controls::text_field(ui, nr, nid, &mut d.name, "Preset name", library::MAX_NAME_CHARS + 8, false);
    if out.resp.hovered() {
        hover(ui, "NAME", &d.name, "dialog:name");
    }
    if out.changed {
        d.error = None;
    }
    y += 54.0;
    // Category chips.
    field_label(ui, pos2(inner.left(), y), "CATEGORY");
    let mut x = inner.left();
    for (i, c) in CATEGORIES.iter().enumerate() {
        let cw = controls::chip_width(ui.painter(), c).max(52.0);
        let r = Rect::from_min_size(pos2(x, y + 16.0), vec2(cw, controls::CHIP_H));
        let resp = controls::chip(ui, r, c, d.category == i);
        if resp.hovered() {
            hover(ui, "CATEGORY", c, "dialog:category");
        }
        if resp.clicked() {
            d.category = i;
            d.error = None;
        }
        x += cw + 6.0;
    }
    y += 46.0;
    // Author | tags.
    let half = (w - 12.0) / 2.0;
    field_label(ui, pos2(inner.left(), y), "AUTHOR");
    field_label(ui, pos2(inner.left() + half + 12.0, y), "TAGS");
    let ar = Rect::from_min_size(pos2(inner.left(), y + 16.0), vec2(half, 30.0));
    let tr = Rect::from_min_size(pos2(inner.left() + half + 12.0, y + 16.0), vec2(half, 30.0));
    let a = controls::text_field(ui, ar, Id::new("gts-br-sa-author"), &mut d.author, "Your name", library::MAX_AUTHOR_CHARS, false);
    let t = controls::text_field(ui, tr, Id::new("gts-br-sa-tags"), &mut d.tags, "riddim, growl, wobble", 160, false);
    if a.resp.hovered() {
        hover(ui, "AUTHOR", &d.author, "dialog:author");
    }
    if t.resp.hovered() {
        hover(ui, "TAGS", &d.tags, "dialog:tags");
    }
    y += 54.0;
    // Description.
    field_label(ui, pos2(inner.left(), y), "DESCRIPTION");
    let dr = Rect::from_min_size(pos2(inner.left(), y + 16.0), vec2(w, 100.0));
    let desc = controls::text_field(ui, dr, Id::new("gts-br-sa-desc"), &mut d.description, "What it is, how to play it, what the macros do", library::MAX_DESCRIPTION_CHARS, true);
    if desc.resp.hovered() {
        hover(ui, "DESCRIPTION", "", "dialog:description");
    }
    y += 128.0;
    // Macro names.
    field_label(ui, pos2(inner.left(), y), "MACRO NAMES");
    let mw = (w - 3.0 * 8.0) / 4.0;
    for i in 0..4 {
        let r = Rect::from_min_size(pos2(inner.left() + i as f32 * (mw + 8.0), y + 16.0), vec2(mw, 28.0));
        let hint = format!("Macro {}", i + 1);
        let o = controls::text_field(ui, r, Id::new(("gts-br-sa-macro", i)), &mut d.macros[i], &hint, library::MAX_LABEL_CHARS, false);
        if o.resp.hovered() {
            hover(ui, &format!("MACRO {}", i + 1), &d.macros[i], "dialog:macro");
        }
    }
    // Footer: message left, buttons right.
    let name_err = library::validate_name(&d.name).err();
    let msg = d.error.clone().or_else(|| name_err.as_ref().filter(|_| !d.name.trim().is_empty()).map(|e| e.to_string()));
    if let Some(m) = msg {
        ui.painter().text(pos2(inner.left(), inner.bottom() - 14.0), Align2::LEFT_CENTER, m, font::label(12.0), SRC_E);
    }
    let ok_enabled = name_err.is_none();
    let (cancel, ok) = footer(ui, inner, "Cancel", "Save", ButtonKind::Primary, ok_enabled);
    if cancel {
        *action = Some(DialogAction::Cancel);
    }
    if (ok || out.enter || (enter && !desc.resp.has_focus())) && ok_enabled {
        *action = Some(DialogAction::Save);
    }
}
