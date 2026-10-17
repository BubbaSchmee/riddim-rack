//! The GTS preset library (SPEC v2 Track 6): the two preset trees on disk, favourites and settings,
//! search and filtering, and the user-preset commands (save, save as, rename, delete). Pure Rust:
//! no egui, no nih-plug, nothing here touches the audio thread; the browser calls it from the GUI
//! thread (or a short-lived loader thread at editor open).
//!
//! ```text
//! <Documents>\Riddim Rack\GTS\
//!   Presets\Factory\<Category>\<Name>.json   written read-only from the embedded bank (build.rs)
//!   Presets\Factory\.bank.json               bank hash + the plugin version that wrote it
//!   Presets\User\<Category>\<Name>.json      the user's presets (any depth below the category)
//!   favorites.json                           {"favorites": ["User/Bass/My Wobble", ...]}
//!   settings.json                            default preset, remembered author, browser state
//! ```
//!
//! A preset's identity is its **key**, the path below `Presets` without the extension and with `/`
//! separators (`"Factory/Bass/Growl Hound"`). Favourites and the default preset are stored by key.
//! The category of a file is the folder it sits in (the fixed list [`CATEGORIES`]); a file outside a
//! category folder falls back to the first segment of its own `"category"` field, then to Bass.
//!
//! Robustness rules: a file that does not parse is listed (name from the file name, a problem
//! text) and never stops the scan; a file that parses with warnings keeps them as its problem
//! text; unreadable settings or favourites fall back to defaults; every write goes through a
//! temp file and a rename, so a crash or a full disk never leaves half a preset.
//! ponytail: no file watcher, a Rescan command re-reads both trees (UX-6).

use crate::preset::Preset;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// The fixed category list (UX-9), in browser order.
pub const CATEGORIES: [&str; 7] = ["Bass", "Lead", "Pluck", "Pad", "FX", "Seq", "Init"];
/// The category the browser opens on (SPEC Track 6).
pub const DEFAULT_CATEGORY: &str = "Bass";
/// Longest preset name, in characters (a file name below Windows' 255 with room for the path).
pub const MAX_NAME_CHARS: usize = 64;
pub const MAX_AUTHOR_CHARS: usize = 48;
pub const MAX_DESCRIPTION_CHARS: usize = 2000;
pub const MAX_LABEL_CHARS: usize = 14;
pub const MAX_TAGS: usize = 12;
pub const MAX_TAG_CHARS: usize = 24;
/// The identity of the header's Init action (no file).
pub const INIT_KEY: &str = "Init";
/// Preset files over this size are listed as damaged instead of being read (a real one is ~7 KB).
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const BANK_MARKER: &str = ".bank.json";

/// Which tree a preset lives in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Origin {
    Factory,
    User,
}

impl Origin {
    pub fn folder(self) -> &'static str {
        match self {
            Origin::Factory => "Factory",
            Origin::User => "User",
        }
    }
}

/// Index of `category` in [`CATEGORIES`] (case-insensitive).
pub fn category_index(category: &str) -> Option<usize> {
    CATEGORIES.iter().position(|c| c.eq_ignore_ascii_case(category.trim()))
}

/// The fixed category a free-form category label belongs to ("Bass/growl" -> Bass).
pub fn category_of(label: &str) -> Option<&'static str> {
    category_index(label.split('/').next().unwrap_or("")).map(|i| CATEGORIES[i])
}

/// One line of the library: a preset file (or an embedded factory preset) and what the browser
/// shows and searches.
#[derive(Clone, Debug)]
pub struct Entry {
    pub key: String,
    pub origin: Origin,
    /// One of [`CATEGORIES`].
    pub category: &'static str,
    /// The category text as the file wrote it ("Bass/growl").
    pub label: String,
    pub name: String,
    pub author: String,
    pub description: String,
    pub tags: Vec<String>,
    /// The four macro labels (defaults where the file has none).
    pub macros: Vec<String>,
    /// `None` for an embedded factory preset the disk copy of which is missing.
    pub path: Option<PathBuf>,
    /// `None` when the file is damaged.
    pub preset: Option<Arc<Preset>>,
    /// Damaged (`preset` is `None`), or loaded with warnings, or a duplicate name.
    pub problem: Option<String>,
    name_lc: String,
    hay: String,
}

impl Entry {
    pub fn is_damaged(&self) -> bool {
        self.preset.is_none()
    }

    /// The first sentence of the description (the help bar line after a load).
    pub fn first_sentence(&self) -> &str {
        let d = self.description.trim();
        match d.find(". ") {
            Some(i) => &d[..=i],
            None => d,
        }
    }

    fn finish(mut self) -> Self {
        self.name_lc = self.name.to_lowercase();
        self.hay = format!("{}\n{}\n{}\n{}", self.name, self.author, self.description, self.tags.join(" ")).to_lowercase();
        self
    }
}

/// What the Source column filters on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SourceFilter {
    #[default]
    All,
    Factory,
    User,
    Favorites,
}

impl SourceFilter {
    pub const ALL: [SourceFilter; 4] = [SourceFilter::All, SourceFilter::Factory, SourceFilter::User, SourceFilter::Favorites];
    pub fn name(self) -> &'static str {
        match self {
            SourceFilter::All => "All",
            SourceFilter::Factory => "Factory",
            SourceFilter::User => "User",
            SourceFilter::Favorites => "Favorites",
        }
    }
    pub fn from_name(s: &str) -> Self {
        Self::ALL.into_iter().find(|f| f.name().eq_ignore_ascii_case(s)).unwrap_or_default()
    }
}

/// The browser's filters. They combine with AND.
#[derive(Clone, Debug, PartialEq)]
pub struct Filter {
    pub source: SourceFilter,
    /// One of [`CATEGORIES`]; `None` = every category.
    pub category: Option<&'static str>,
    pub tag: Option<String>,
    /// Search text: words in any order, `"quoted phrases"`, case-insensitive.
    pub text: String,
}

impl Default for Filter {
    fn default() -> Self {
        Self { source: SourceFilter::All, category: Some(DEFAULT_CATEGORY), tag: None, text: String::new() }
    }
}

/// How many presets each facet value would show under the other filters.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Counts {
    /// In [`SourceFilter::ALL`] order.
    pub sources: [usize; 4],
    /// In [`CATEGORIES`] order.
    pub categories: [usize; 7],
    /// Every category.
    pub all_categories: usize,
}

/// Splits search text into lowercase terms: words, and `"quoted phrases"` kept whole (an
/// unterminated quote runs to the end).
pub fn search_terms(text: &str) -> Vec<String> {
    let mut terms = vec![];
    let (mut cur, mut quoted) = (String::new(), false);
    for ch in text.chars() {
        match ch {
            '"' => {
                if !cur.trim().is_empty() {
                    terms.push(cur.trim().to_lowercase());
                }
                cur.clear();
                quoted = !quoted;
            }
            c if c.is_whitespace() && !quoted => {
                if !cur.is_empty() {
                    terms.push(cur.to_lowercase());
                    cur.clear();
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.trim().is_empty() {
        terms.push(cur.trim().to_lowercase());
    }
    terms
}

/// Tags from free text: split on commas, semicolons and spaces; a leading `#` dropped; lowercase;
/// de-duplicated, at most [`MAX_TAGS`] of at most [`MAX_TAG_CHARS`] characters.
pub fn parse_tags(text: &str) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for t in text.split(|c: char| c == ',' || c == ';' || c.is_whitespace()) {
        let t: String = t.trim().trim_start_matches('#').chars().filter(|c| !c.is_control()).take(MAX_TAG_CHARS).collect::<String>().to_lowercase();
        if !t.is_empty() && !out.contains(&t) && out.len() < MAX_TAGS {
            out.push(t);
        }
    }
    out
}

/// Why a preset name is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NameError {
    Empty,
    TooLong,
    /// Holds the first offending character.
    IllegalChar(char),
    /// A name Windows reserves (CON, NUL, COM1, ...), or one starting with a dot or ending in a dot.
    Reserved,
}

impl std::fmt::Display for NameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NameError::Empty => write!(f, "Give the preset a name."),
            NameError::TooLong => write!(f, "Use at most {MAX_NAME_CHARS} characters."),
            NameError::IllegalChar(c) => write!(f, "A name cannot contain {c:?}."),
            NameError::Reserved => write!(f, "That name is reserved by the file system."),
        }
    }
}

/// The trimmed name when it can be a file name on every platform: not empty, at most
/// [`MAX_NAME_CHARS`], none of `< > : " / \ | ? *` or a control character, not a reserved Windows
/// device name, not starting or ending with a dot.
pub fn validate_name(raw: &str) -> Result<String, NameError> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(NameError::Empty);
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(NameError::TooLong);
    }
    if let Some(c) = name.chars().find(|c| c.is_control() || "<>:\"/\\|?*".contains(*c)) {
        return Err(NameError::IllegalChar(c));
    }
    let stem = name.split('.').next().unwrap_or("").trim_end().to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CLOCK$")
        || ((stem.starts_with("COM") || stem.starts_with("LPT")) && stem.len() == 4 && stem.as_bytes()[3].is_ascii_digit() && stem.as_bytes()[3] != b'0');
    if reserved || name.starts_with('.') || name.ends_with('.') {
        return Err(NameError::Reserved);
    }
    Ok(name.to_string())
}

/// Why a library command failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LibError {
    Name(NameError),
    /// A user preset of that name already exists (holds its key). Retry with `overwrite`.
    Exists(String),
    /// The key is not in the library, or not a user preset where one is needed.
    NotUser,
    /// A damaged file cannot be renamed (its name lives inside it); delete it instead.
    Damaged,
    Io(String),
}

impl std::fmt::Display for LibError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LibError::Name(e) => e.fmt(f),
            LibError::Exists(_) => write!(f, "A preset with that name already exists."),
            LibError::NotUser => write!(f, "Only your own presets can be changed."),
            LibError::Damaged => write!(f, "This file is damaged and cannot be renamed."),
            LibError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl From<io::Error> for LibError {
    fn from(e: io::Error) -> Self {
        LibError::Io(e.to_string())
    }
}

/// What the browser remembers between sessions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BrowserPrefs {
    /// "All", "Factory", "User" or "Favorites".
    pub source: String,
    /// "All" or one of [`CATEGORIES`].
    pub category: String,
    pub load_on_select: bool,
}

impl Default for BrowserPrefs {
    fn default() -> Self {
        Self { source: "All".into(), category: DEFAULT_CATEGORY.into(), load_on_select: true }
    }
}

/// `settings.json`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Key of the preset new instances load; empty = Init.
    pub default_preset: String,
    /// Remembered author for Save As.
    pub author: String,
    pub browser: BrowserPrefs,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Favorites {
    favorites: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct Marker {
    plugin_version: String,
    bank_hash: String,
    files: Vec<String>,
}

/// What [`write_factory`] did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FactoryReport {
    pub written: usize,
    pub unchanged: usize,
    pub removed: usize,
    /// The folder was written by a newer plugin version: left alone.
    pub skipped_newer: bool,
    pub error: Option<String>,
}

// ---- paths -------------------------------------------------------------------------------------

/// The Windows Documents known folder (follows OneDrive redirection), else `%USERPROFILE%\Documents`
/// (or `$HOME/Documents`), else the current directory.
pub fn documents_dir() -> PathBuf {
    #[cfg(windows)]
    if let Some(p) = known_documents() {
        return p;
    }
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from);
    home.map(|h| h.join("Documents")).unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(windows)]
fn known_documents() -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::UI::Shell::{FOLDERID_Documents, SHGetKnownFolderPath};
    let mut out: *mut u16 = std::ptr::null_mut();
    // SAFETY: a valid known-folder id, no token, and an out pointer we free with CoTaskMemFree.
    let hr = unsafe { SHGetKnownFolderPath(&FOLDERID_Documents, 0, std::ptr::null_mut(), &mut out) };
    if hr < 0 || out.is_null() {
        return None;
    }
    // SAFETY: on success `out` is a NUL-terminated wide string.
    let path = unsafe {
        let len = (0..).take_while(|&i| *out.add(i) != 0).count();
        std::ffi::OsString::from_wide(std::slice::from_raw_parts(out, len))
    };
    // SAFETY: allocated by the shell for us.
    unsafe { CoTaskMemFree(out as *const _) };
    Some(PathBuf::from(path))
}

/// `<Documents>\Riddim Rack\GTS`.
pub fn default_root() -> PathBuf {
    documents_dir().join("Riddim Rack").join("GTS")
}

// ---- file helpers ------------------------------------------------------------------------------

/// Writes through a temp file in the same folder and renames over the target.
pub(crate) fn write_atomic(path: &Path, data: &[u8]) -> io::Result<()> {
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp = dir.join(format!(".{}.{}.{}.tmp", path.file_stem().and_then(OsStr::to_str).unwrap_or("x").chars().take(16).collect::<String>(), std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

fn set_readonly(path: &Path, ro: bool) {
    if let Ok(meta) = fs::metadata(path) {
        let mut perm = meta.permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        perm.set_readonly(ro);
        let _ = fs::set_permissions(path, perm);
    }
}

fn fnv64(parts: &[&[u8]]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for p in parts {
        for &b in *p {
            h = (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3);
        }
        h = (h ^ 0xff).wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// "0.1.2-beta" -> (0, 1, 2); unparsable parts count as 0.
fn version_tuple(v: &str) -> (u32, u32, u32) {
    let mut it = v.split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty()).map(|s| s.parse::<u32>().unwrap_or(0));
    (it.next().unwrap_or(0), it.next().unwrap_or(0), it.next().unwrap_or(0))
}

fn factory_dir(root: &Path) -> PathBuf {
    root.join("Presets").join("Factory")
}

fn user_dir(root: &Path) -> PathBuf {
    root.join("Presets").join("User")
}

/// Writes the embedded factory bank to `<root>/Presets/Factory/<Category>/<Name>.json`, read-only.
///
/// Version check: `.bank.json` records the plugin version and a hash of the bank. Same hash and
/// every file present: nothing is touched. A folder written by a newer plugin version is left
/// alone. Otherwise each file is compared with the embedded text and only differing or missing
/// ones are rewritten (read-only cleared for the write, set again after), and files the previous
/// bank wrote that this one no longer has are removed. Files in the User tree are never touched.
pub fn write_factory(root: &Path, bank: &[(&str, &str)]) -> FactoryReport {
    let mut r = FactoryReport::default();
    let dir = factory_dir(root);
    let hash = {
        let mut parts: Vec<&[u8]> = vec![];
        for (rel, text) in bank {
            parts.push(rel.as_bytes());
            parts.push(text.as_bytes());
        }
        format!("{:016x}", fnv64(&parts))
    };
    let version = env!("CARGO_PKG_VERSION");
    let marker_path = dir.join(BANK_MARKER);
    let old: Option<Marker> = fs::read_to_string(&marker_path).ok().and_then(|s| serde_json::from_str(&s).ok());
    if let Some(m) = &old {
        if version_tuple(&m.plugin_version) > version_tuple(version) {
            r.skipped_newer = true;
            return r;
        }
        if m.bank_hash == hash && bank.iter().all(|(rel, _)| dir.join(rel).is_file()) {
            r.unchanged = bank.len();
            return r;
        }
    }
    let mut run = || -> io::Result<()> {
        for (rel, text) in bank {
            let path = dir.join(rel);
            if fs::read_to_string(&path).is_ok_and(|cur| cur == *text) {
                r.unchanged += 1;
                continue;
            }
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            if path.exists() {
                set_readonly(&path, false);
            }
            write_atomic(&path, text.as_bytes())?;
            set_readonly(&path, true);
            r.written += 1;
        }
        let keep: BTreeSet<&str> = bank.iter().map(|(rel, _)| *rel).collect();
        for stale in old.iter().flat_map(|m| m.files.iter()).filter(|f| !keep.contains(f.as_str())) {
            let path = dir.join(stale);
            set_readonly(&path, false);
            if fs::remove_file(&path).is_ok() {
                r.removed += 1;
            }
        }
        fs::create_dir_all(&dir)?;
        let marker = Marker { plugin_version: version.into(), bank_hash: hash.clone(), files: bank.iter().map(|(rel, _)| rel.to_string()).collect() };
        write_atomic(&marker_path, serde_json::to_string_pretty(&marker).expect("marker serialises").as_bytes())
    };
    if let Err(e) = run() {
        r.error = Some(format!("factory presets: {e}"));
    }
    r
}

// ---- scanning ----------------------------------------------------------------------------------

fn key_of(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root.join("Presets")).unwrap_or(path);
    let mut key: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
    if let Some(last) = key.last_mut() {
        if let Some(stem) = last.strip_suffix(".json").or_else(|| last.strip_suffix(".JSON")) {
            *last = stem.to_string();
        }
    }
    key.join("/")
}

fn macro_labels(p: &Preset) -> Vec<String> {
    p.matrix_state().macros.to_vec()
}

fn entry_from_text(origin: Origin, key: String, path: Option<PathBuf>, folder_cat: Option<&'static str>, stem: &str, text: Result<String, String>) -> Entry {
    let parsed = text.and_then(|t| Preset::from_json(&t));
    match parsed {
        Ok((p, warnings)) => {
            let category = folder_cat.or_else(|| category_of(&p.category)).unwrap_or(DEFAULT_CATEGORY);
            let name = if p.name.trim().is_empty() { stem.to_string() } else { p.name.clone() };
            let problem = (!warnings.is_empty()).then(|| format!("Loaded with {} note{}: {}", warnings.len(), if warnings.len() == 1 { "" } else { "s" }, warnings[0]));
            Entry {
                key,
                origin,
                category,
                label: if p.category.trim().is_empty() { category.to_string() } else { p.category.clone() },
                name,
                author: p.author.clone(),
                description: p.description.clone(),
                tags: p.tags.clone(),
                macros: macro_labels(&p),
                path,
                preset: Some(Arc::new(p)),
                problem,
                name_lc: String::new(),
                hay: String::new(),
            }
            .finish()
        }
        Err(e) => Entry {
            key,
            origin,
            category: folder_cat.unwrap_or(DEFAULT_CATEGORY),
            label: folder_cat.unwrap_or(DEFAULT_CATEGORY).to_string(),
            name: stem.to_string(),
            author: String::new(),
            description: String::new(),
            tags: vec![],
            macros: vec![],
            path,
            preset: None,
            problem: Some(e),
            name_lc: String::new(),
            hay: String::new(),
        }
        .finish(),
    }
}

fn entry_from_file(root: &Path, origin: Origin, path: &Path) -> Entry {
    let rel_below_origin: Vec<String> = path
        .strip_prefix(root.join("Presets").join(origin.folder()))
        .map(|p| p.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect())
        .unwrap_or_default();
    let folder_cat = (rel_below_origin.len() > 1).then(|| category_of(&rel_below_origin[0])).flatten();
    let stem = path.file_stem().and_then(OsStr::to_str).unwrap_or("?").to_string();
    let text = match fs::metadata(path) {
        Ok(m) if m.len() > MAX_FILE_BYTES => Err(format!("File is {} MB; presets are a few KB.", m.len() / (1024 * 1024))),
        _ => fs::read_to_string(path).map_err(|e| format!("Cannot read the file: {e}")),
    };
    entry_from_text(origin, key_of(root, path), Some(path.to_path_buf()), folder_cat, &stem, text)
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if p.is_dir() {
            if depth < 4 {
                walk(&p, depth + 1, out);
            }
        } else if p.extension().and_then(OsStr::to_str).is_some_and(|x| x.eq_ignore_ascii_case("json")) {
            out.push(p);
        }
    }
}

fn sort_entries(v: &mut [Entry]) {
    v.sort_by(|a, b| {
        let ka = (category_index(a.category).unwrap_or(99), &a.name_lc, a.origin, &a.key);
        let kb = (category_index(b.category).unwrap_or(99), &b.name_lc, b.origin, &b.key);
        ka.cmp(&kb)
    });
}

/// A later preset with the same name in the same folder gets a note (the first keeps none).
fn mark_duplicates(v: &mut [Entry]) {
    let mut seen: std::collections::HashMap<(Origin, String, String), String> = Default::default();
    for e in v.iter_mut() {
        let folder = e.key.rsplit_once('/').map(|(f, _)| f.to_lowercase()).unwrap_or_default();
        let first = seen.entry((e.origin, folder, e.name_lc.clone())).or_insert_with(|| e.key.clone());
        if *first != e.key && e.problem.is_none() {
            e.problem = Some(format!("Same name as {} in this folder.", first.rsplit('/').next().unwrap_or("another preset")));
        }
    }
}

// ---- the library -------------------------------------------------------------------------------

pub struct Library {
    root: PathBuf,
    bank: &'static [(&'static str, &'static str)],
    entries: Vec<Entry>,
    favorites: BTreeSet<String>,
    settings: Settings,
    /// Bumps on every change to the entries, favourites or settings (UI caches key on it).
    rev: u32,
    /// Problems met while opening or writing (read-only folder, damaged settings, ...).
    pub issues: Vec<String>,
}

impl Library {
    /// Opens the library under `root`: creates both trees (and the category folders of `User`),
    /// writes the embedded `bank` (see [`write_factory`]), scans, reads favourites and settings.
    /// Never fails: what cannot be done is reported in [`Library::issues`], and embedded factory
    /// presets missing from disk are still listed.
    pub fn open(root: &Path, bank: &'static [(&'static str, &'static str)]) -> Library {
        let mut issues = vec![];
        let mut mk = |p: PathBuf| {
            if let Err(e) = fs::create_dir_all(&p) {
                issues.push(format!("cannot create {}: {e}", p.display()));
            }
        };
        mk(factory_dir(root));
        for c in CATEGORIES {
            mk(user_dir(root).join(c));
        }
        let rep = write_factory(root, bank);
        issues.extend(rep.error);
        let mut lib = Library { root: root.to_path_buf(), bank, entries: vec![], favorites: BTreeSet::new(), settings: Settings::default(), rev: 0, issues };
        lib.rescan();
        lib.favorites = lib.read_json::<Favorites>("favorites.json").favorites.into_iter().collect();
        lib.settings = lib.read_json::<Settings>("settings.json");
        lib
    }

    fn read_json<T: Default + for<'de> Deserialize<'de>>(&mut self, file: &str) -> T {
        match fs::read_to_string(self.root.join(file)) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
                self.issues.push(format!("{file} is damaged ({e}); defaults used"));
                T::default()
            }),
            Err(_) => T::default(),
        }
    }

    /// Re-reads both trees (UX-6). Favourites and settings stay as they are in memory.
    pub fn rescan(&mut self) {
        let mut entries = vec![];
        for origin in [Origin::Factory, Origin::User] {
            let dir = self.root.join("Presets").join(origin.folder());
            let mut files = vec![];
            walk(&dir, 0, &mut files);
            entries.extend(files.iter().map(|f| entry_from_file(&self.root, origin, f)));
        }
        // Embedded factory presets whose disk copy is missing (a read-only Documents folder).
        for (rel, text) in self.bank {
            let key = format!("Factory/{}", rel.trim_end_matches(".json"));
            if entries.iter().any(|e| e.key == key) {
                continue;
            }
            let (cat, file) = rel.split_once('/').unwrap_or(("", rel));
            let stem = file.trim_end_matches(".json");
            entries.push(entry_from_text(Origin::Factory, key, None, category_of(cat), stem, Ok(text.to_string())));
        }
        sort_entries(&mut entries);
        mark_duplicates(&mut entries);
        self.entries = entries;
        self.rev = self.rev.wrapping_add(1);
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn user_dir(&self) -> PathBuf {
        user_dir(&self.root)
    }
    pub fn factory_dir(&self) -> PathBuf {
        factory_dir(&self.root)
    }
    pub fn rev(&self) -> u32 {
        self.rev
    }
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
    pub fn get(&self, key: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.key == key)
    }
    pub fn index_of(&self, key: &str) -> Option<usize> {
        self.entries.iter().position(|e| e.key == key)
    }
    pub fn settings(&self) -> &Settings {
        &self.settings
    }
    pub fn is_favorite(&self, key: &str) -> bool {
        self.favorites.contains(key)
    }

    // ---- query ----

    fn matches(&self, e: &Entry, f: &Filter, terms: &[String], skip: (bool, bool)) -> bool {
        let (skip_source, skip_cat) = skip;
        (skip_source
            || match f.source {
                SourceFilter::All => true,
                SourceFilter::Factory => e.origin == Origin::Factory,
                SourceFilter::User => e.origin == Origin::User,
                SourceFilter::Favorites => self.favorites.contains(&e.key),
            })
            && (skip_cat || f.category.is_none_or(|c| e.category == c))
            && f.tag.as_ref().is_none_or(|t| e.tags.iter().any(|x| x.eq_ignore_ascii_case(t)))
            && terms.iter().all(|t| e.hay.contains(t.as_str()))
    }

    /// Indices into [`Library::entries`] that pass `f`, in list order (category, then name).
    pub fn query(&self, f: &Filter) -> Vec<usize> {
        let terms = search_terms(&f.text);
        (0..self.entries.len()).filter(|&i| self.matches(&self.entries[i], f, &terms, (false, false))).collect()
    }

    /// Facet counts: each source count ignores the source filter, each category count the category
    /// filter, and both honour the tag and the text.
    pub fn counts(&self, f: &Filter) -> Counts {
        let terms = search_terms(&f.text);
        let mut c = Counts::default();
        for e in &self.entries {
            if self.matches(e, f, &terms, (true, false)) {
                c.sources[0] += 1;
                c.sources[1] += (e.origin == Origin::Factory) as usize;
                c.sources[2] += (e.origin == Origin::User) as usize;
                c.sources[3] += self.favorites.contains(&e.key) as usize;
            }
            if self.matches(e, f, &terms, (false, true)) {
                c.all_categories += 1;
                if let Some(i) = category_index(e.category) {
                    c.categories[i] += 1;
                }
            }
        }
        c
    }

    /// Tags of the presets that pass the source, category and text filters (the tag filter itself
    /// is ignored so another tag can be picked), most used first, then alphabetical.
    pub fn tags(&self, f: &Filter) -> Vec<(String, usize)> {
        let terms = search_terms(&f.text);
        let g = Filter { tag: None, ..f.clone() };
        let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
        for e in self.entries.iter().filter(|e| self.matches(e, &g, &terms, (false, false))) {
            for t in &e.tags {
                *counts.entry(t.to_lowercase()).or_default() += 1;
            }
        }
        let mut v: Vec<(String, usize)> = counts.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        v
    }

    // ---- favourites, settings ----

    pub fn set_favorite(&mut self, key: &str, on: bool) -> Result<(), LibError> {
        // Another plugin instance may have changed the file since we read it: start from its copy.
        if let Some(f) = fs::read_to_string(self.root.join("favorites.json")).ok().and_then(|s| serde_json::from_str::<Favorites>(&s).ok()) {
            self.favorites = f.favorites.into_iter().collect();
        }
        if on {
            self.favorites.insert(key.to_string());
        } else {
            self.favorites.remove(key);
        }
        self.rev = self.rev.wrapping_add(1);
        self.save_favorites()
    }

    fn save_favorites(&self) -> Result<(), LibError> {
        let f = Favorites { favorites: self.favorites.iter().cloned().collect() };
        write_atomic(&self.root.join("favorites.json"), serde_json::to_string_pretty(&f).expect("favourites serialise").as_bytes())?;
        Ok(())
    }

    /// Edits the settings and writes them.
    pub fn update_settings(&mut self, edit: impl FnOnce(&mut Settings)) -> Result<(), LibError> {
        let before = self.settings.clone();
        edit(&mut self.settings);
        if self.settings == before {
            return Ok(());
        }
        self.rev = self.rev.wrapping_add(1);
        write_atomic(&self.root.join("settings.json"), serde_json::to_string_pretty(&self.settings).expect("settings serialise").as_bytes())?;
        Ok(())
    }

    /// The preset new instances load, if one is set and still exists.
    pub fn default_preset(&self) -> Option<&Entry> {
        (!self.settings.default_preset.is_empty()).then(|| self.get(&self.settings.default_preset)).flatten().filter(|e| !e.is_damaged())
    }

    pub fn set_default(&mut self, key: Option<&str>) -> Result<(), LibError> {
        let key = key.unwrap_or("").to_string();
        self.update_settings(|s| s.default_preset = key)
    }

    // ---- user commands ----

    fn user_target(&self, category: &str, name: &str) -> PathBuf {
        user_dir(&self.root).join(category).join(format!("{name}.json"))
    }

    /// The key of an existing user preset that `name` in `category` would replace (case-insensitive,
    /// as the file system is).
    pub fn conflict(&self, category: &str, name: &str) -> Option<String> {
        let key = format!("User/{category}/{}", name.trim());
        self.entries.iter().find(|e| e.key.eq_ignore_ascii_case(&key)).map(|e| e.key.clone())
    }

    fn put(&mut self, path: &Path) -> String {
        let e = entry_from_file(&self.root, Origin::User, path);
        let key = e.key.clone();
        self.entries.retain(|x| x.key != key);
        self.entries.push(e);
        sort_entries(&mut self.entries);
        for e in &mut self.entries {
            if e.problem.as_deref().is_some_and(|p| p.starts_with("Same name")) {
                e.problem = None;
            }
        }
        mark_duplicates(&mut self.entries);
        self.rev = self.rev.wrapping_add(1);
        key
    }

    /// Save As: writes `preset` as a new user preset `User/<category>/<name>.json`. `category` must
    /// be one of [`CATEGORIES`]. An existing preset of that name is replaced only with `overwrite`
    /// (otherwise [`LibError::Exists`]); factory presets can never be the target. Returns the key.
    pub fn save_as(&mut self, preset: &Preset, category: &str, overwrite: bool) -> Result<String, LibError> {
        let name = validate_name(&preset.name).map_err(LibError::Name)?;
        let category = category_of(category).unwrap_or(DEFAULT_CATEGORY);
        if let Some(key) = self.conflict(category, &name) {
            if !overwrite {
                return Err(LibError::Exists(key));
            }
        }
        let mut p = preset.clone();
        p.name = name.clone();
        if category_of(&p.category) != Some(category) {
            p.category = category.to_string();
        }
        p.author = p.author.chars().take(MAX_AUTHOR_CHARS).collect();
        p.description = p.description.chars().take(MAX_DESCRIPTION_CHARS).collect();
        let path = match self.conflict(category, &name).and_then(|k| self.get(&k)).and_then(|e| e.path.clone()) {
            Some(existing) => existing, // keep the file's own spelling on a case-only difference
            None => self.user_target(category, &name),
        };
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        write_atomic(&path, p.to_json().as_bytes())?;
        Ok(self.put(&path))
    }

    /// Save: overwrites the user preset `key` with `preset`, keeping its name and category.
    pub fn save(&mut self, key: &str, preset: &Preset) -> Result<String, LibError> {
        let e = self.get(key).filter(|e| e.origin == Origin::User).ok_or(LibError::NotUser)?;
        let (path, category) = (e.path.clone().ok_or(LibError::NotUser)?, e.category);
        let mut p = preset.clone();
        p.name = e.name.clone();
        if category_of(&p.category) != Some(category) {
            p.category = e.label.clone();
        }
        write_atomic(&path, p.to_json().as_bytes())?;
        Ok(self.put(&path))
    }

    /// Renames a user preset (its file and the name inside it). Favourite and default follow.
    pub fn rename(&mut self, key: &str, new_name: &str) -> Result<String, LibError> {
        let name = validate_name(new_name).map_err(LibError::Name)?;
        let e = self.get(key).filter(|e| e.origin == Origin::User).ok_or(LibError::NotUser)?;
        let preset = e.preset.clone().ok_or(LibError::Damaged)?;
        let old_path = e.path.clone().ok_or(LibError::NotUser)?;
        let dir = old_path.parent().map(Path::to_path_buf).unwrap_or_else(|| user_dir(&self.root));
        let new_path = dir.join(format!("{name}.json"));
        let same_file = old_path.file_name().and_then(OsStr::to_str).is_some_and(|o| o.eq_ignore_ascii_case(&format!("{name}.json")));
        if !same_file {
            let new_key = key_of(&self.root, &new_path);
            if new_path.exists() || self.entries.iter().any(|x| x.key.eq_ignore_ascii_case(&new_key)) {
                return Err(LibError::Exists(new_key));
            }
        }
        let mut p = (*preset).clone();
        p.name = name;
        if same_file && new_path != old_path {
            // A change of letter case only: one file on a case-insensitive file system.
            fs::rename(&old_path, &new_path)?;
        }
        write_atomic(&new_path, p.to_json().as_bytes())?;
        if !same_file {
            let _ = fs::remove_file(&old_path);
        }
        self.entries.retain(|x| x.key != key);
        let new_key = self.put(&new_path);
        if new_key != key {
            if self.favorites.remove(key) {
                self.favorites.insert(new_key.clone());
                self.save_favorites()?;
            }
            if self.settings.default_preset == key {
                self.update_settings(|s| s.default_preset = new_key.clone())?;
            }
        }
        Ok(new_key)
    }

    /// Deletes a user preset's file (the caller confirms first). Favourite and default are cleared.
    pub fn delete(&mut self, key: &str) -> Result<(), LibError> {
        let e = self.get(key).filter(|e| e.origin == Origin::User).ok_or(LibError::NotUser)?;
        if let Some(path) = &e.path {
            fs::remove_file(path)?;
        }
        self.entries.retain(|x| x.key != key);
        mark_duplicates(&mut self.entries);
        self.rev = self.rev.wrapping_add(1);
        if self.favorites.remove(key) {
            self.save_favorites()?;
        }
        if self.settings.default_preset == key {
            self.update_settings(|s| s.default_preset.clear())?;
        }
        Ok(())
    }

    /// The file or folder the Reveal command opens for `key`.
    pub fn reveal_path(&self, key: &str) -> Option<PathBuf> {
        self.get(key).and_then(|e| e.path.clone()).filter(|p| p.exists())
    }
}

/// Opens Explorer (or the platform's file manager) on a folder, or with a file selected. Spawns
/// and returns at once; never waits for the window.
pub fn reveal(path: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut c = std::process::Command::new("explorer");
        if path.is_file() {
            c.raw_arg(format!("/select,\"{}\"", path.display()));
        } else {
            c.arg(path);
        }
        c.spawn().map(|_| ())
    }
    #[cfg(not(windows))]
    {
        let dir = if path.is_file() { path.parent().unwrap_or(path) } else { path };
        std::process::Command::new(if cfg!(target_os = "macos") { "open" } else { "xdg-open" }).arg(dir).spawn().map(|_| ())
    }
}

/// Deterministic fixture trees for the tests, the browser tests and the frame benchmark: user
/// presets spread over every category with varied names, authors, tags and descriptions, plus the
/// awkward files the browser must survive (a corrupt file, a wrong-product file, duplicate names).
#[doc(hidden)]
pub mod fixture {
    use super::*;

    const ADJ: [&str; 12] = ["Dark", "Heavy", "Rusty", "Wide", "Tight", "Dirty", "Cold", "Hollow", "Sharp", "Deep", "Broken", "Liquid"];
    const NOUN: [&str; 10] = ["Wub", "Growl", "Reese", "Hound", "Stab", "Drop", "Pad", "Sweep", "Bite", "Rumble"];
    const TAGS: [&str; 10] = ["riddim", "tearout", "growl", "wobble", "reese", "neuro", "sub", "screech", "formant", "wub"];
    const AUTHORS: [&str; 4] = ["Riddim Rack", "Mara Voss", "Dub Lab", "K. Osei"];

    /// Writes `n` user presets plus the awkward files under `<root>/Presets/User`. Returns the
    /// number of files written (n + 4).
    pub fn write_user(root: &Path, n: usize) -> usize {
        for i in 0..n {
            let cat = CATEGORIES[(i * 7 / 3 + i) % CATEGORIES.len()];
            let mut tags = vec![TAGS[i % TAGS.len()].to_string(), TAGS[(i * 3 + 1) % TAGS.len()].to_string()];
            tags.dedup();
            let mut p = Preset {
                name: format!("{} {} {:02}", ADJ[i % ADJ.len()], NOUN[(i / 2) % NOUN.len()], i),
                category: cat.to_string(),
                author: AUTHORS[i % AUTHORS.len()].to_string(),
                tags,
                description: format!("A {} {} with a {} edge. Macro 1 pushes the {}.", ADJ[i % ADJ.len()].to_lowercase(), cat.to_lowercase(), TAGS[i % TAGS.len()], TAGS[i % TAGS.len()]),
                macros: vec!["Growl".into(), "Dirt".into(), "Width".into(), "Motion".into()],
                ..Preset::default()
            };
            p.patch.osc[0].index = (i % 128) as f32;
            write(root, cat, &p.name.clone(), &p.to_json());
        }
        // The awkward ones.
        write(root, "Bass", "Broken", "{ this is not json");
        write(root, "Lead", "Other Synth", r#"{"format_version": 1, "product": "VSS", "name": "Other Synth"}"#);
        // Two files, one name, one folder.
        let mut dup = Preset { name: "Twin".into(), category: "FX".into(), ..Preset::default() };
        write(root, "FX", "Twin", &dup.to_json());
        dup.description = "The second file called Twin.".into();
        write(root, "FX", "Twin (copy)", &dup.to_json());
        n + 4
    }

    fn write(root: &Path, cat: &str, file: &str, text: &str) {
        let dir = user_dir(root).join(cat);
        fs::create_dir_all(&dir).expect("fixture dir");
        fs::write(dir.join(format!("{file}.json")), text).expect("fixture file");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::bank::FACTORY_FILES;

    /// A directory removed on drop. Under CARGO_TARGET_DIR when set (C: is full on the dev machine).
    struct Tmp(PathBuf);
    impl Tmp {
        fn new() -> Self {
            static N: AtomicU32 = AtomicU32::new(0);
            let base = std::env::var_os("CARGO_TARGET_DIR").map(|d| PathBuf::from(d).join("tmp")).unwrap_or_else(std::env::temp_dir);
            let p = base.join(format!("gts-lib-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
            fs::create_dir_all(&p).unwrap();
            Tmp(p)
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            fn unlock(p: &Path) {
                if let Ok(rd) = fs::read_dir(p) {
                    for e in rd.flatten() {
                        let path = e.path();
                        if path.is_dir() {
                            unlock(&path)
                        } else {
                            set_readonly(&path, false)
                        }
                    }
                }
            }
            unlock(&self.0);
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn fixture_lib(n: usize) -> (Tmp, Library) {
        let t = Tmp::new();
        fixture::write_user(&t.0, n);
        let lib = Library::open(&t.0, FACTORY_FILES);
        (t, lib)
    }


    fn all() -> Filter {
        Filter { category: None, ..Filter::default() }
    }

    #[test]
    fn factory_bank_has_valid_categories_and_parses() {
        assert!(FACTORY_FILES.len() >= 6);
        for (rel, text) in FACTORY_FILES {
            let (cat, file) = rel.split_once('/').expect("<Category>/<Name>.json");
            assert!(category_index(cat).is_some(), "{rel}: unknown category folder");
            let (p, w) = Preset::from_json(text).unwrap_or_else(|e| panic!("{rel}: {e}"));
            assert!(w.is_empty(), "{rel}: {w:?}");
            assert_eq!(format!("{}.json", p.name), file, "{rel}: file name must be the preset name");
            assert_eq!(category_of(&p.category), Some(CATEGORIES[category_index(cat).unwrap()]), "{rel}: category field disagrees with the folder");
        }
    }

    #[test]
    fn first_run_creates_both_trees_and_a_second_run_rewrites_nothing() {
        let t = Tmp::new();
        let lib = Library::open(&t.0, FACTORY_FILES);
        assert!(lib.issues.is_empty(), "{:?}", lib.issues);
        for c in CATEGORIES {
            assert!(t.0.join("Presets/User").join(c).is_dir(), "User/{c}");
        }
        for (rel, text) in FACTORY_FILES {
            let p = t.0.join("Presets/Factory").join(rel);
            assert_eq!(fs::read_to_string(&p).unwrap(), *text);
            assert!(fs::metadata(&p).unwrap().permissions().readonly(), "{rel} must be read-only");
        }
        let mtimes: Vec<_> = FACTORY_FILES.iter().map(|(rel, _)| fs::metadata(t.0.join("Presets/Factory").join(rel)).unwrap().modified().unwrap()).collect();
        assert_eq!(lib.entries().len(), FACTORY_FILES.len());
        // Second run: nothing written, nothing changed.
        let rep = write_factory(&t.0, FACTORY_FILES);
        assert_eq!(rep, FactoryReport { unchanged: FACTORY_FILES.len(), ..Default::default() });
        // Marker lost: files are compared, none rewritten.
        fs::remove_file(t.0.join("Presets/Factory/.bank.json")).unwrap();
        let rep = write_factory(&t.0, FACTORY_FILES);
        assert_eq!((rep.written, rep.unchanged, rep.error), (0, FACTORY_FILES.len(), None));
        let after: Vec<_> = FACTORY_FILES.iter().map(|(rel, _)| fs::metadata(t.0.join("Presets/Factory").join(rel)).unwrap().modified().unwrap()).collect();
        assert_eq!(mtimes, after);
    }

    #[test]
    fn a_changed_bank_rewrites_only_what_changed_and_removes_what_left() {
        let t = Tmp::new();
        let a = r#"{"format_version":1,"product":"GTS","name":"A","category":"Bass"}"#;
        let b = r#"{"format_version":1,"product":"GTS","name":"B","category":"Lead"}"#;
        let a2 = r#"{"format_version":1,"product":"GTS","name":"A","category":"Bass","author":"x"}"#;
        let r = write_factory(&t.0, &[("Bass/A.json", a), ("Lead/B.json", b)]);
        assert_eq!((r.written, r.unchanged, r.removed), (2, 0, 0));
        let r = write_factory(&t.0, &[("Bass/A.json", a2)]);
        assert_eq!((r.written, r.unchanged, r.removed), (1, 0, 1), "{r:?}");
        assert_eq!(fs::read_to_string(t.0.join("Presets/Factory/Bass/A.json")).unwrap(), a2);
        assert!(!t.0.join("Presets/Factory/Lead/B.json").exists());
        assert!(fs::metadata(t.0.join("Presets/Factory/Bass/A.json")).unwrap().permissions().readonly());
    }

    #[test]
    fn a_folder_written_by_a_newer_plugin_is_left_alone() {
        let t = Tmp::new();
        write_factory(&t.0, FACTORY_FILES);
        let m = Marker { plugin_version: "99.0.0".into(), bank_hash: "x".into(), files: vec![] };
        fs::write(t.0.join("Presets/Factory/.bank.json"), serde_json::to_string(&m).unwrap()).unwrap();
        let victim = t.0.join("Presets/Factory").join(FACTORY_FILES[0].0);
        set_readonly(&victim, false);
        fs::write(&victim, "newer content").unwrap();
        let rep = write_factory(&t.0, FACTORY_FILES);
        assert!(rep.skipped_newer && rep.written == 0, "{rep:?}");
        assert_eq!(fs::read_to_string(&victim).unwrap(), "newer content");
    }

    #[test]
    fn a_user_edit_of_a_copy_elsewhere_is_never_overwritten() {
        let t = Tmp::new();
        let mut lib = Library::open(&t.0, FACTORY_FILES);
        let src = lib.entries().iter().find(|e| e.origin == Origin::Factory).unwrap().preset.clone().unwrap();
        let mut edited = (*src).clone();
        edited.description = "my edit".into();
        let key = lib.save_as(&edited, "Bass", false).unwrap();
        // A new bank revision rewrites Factory, not User.
        let changed = r#"{"format_version":1,"product":"GTS","name":"Zed","category":"Bass"}"#;
        write_factory(&t.0, &[("Bass/Zed.json", changed)]);
        let lib2 = Library::open(&t.0, FACTORY_FILES);
        assert_eq!(lib2.get(&key).unwrap().description, "my edit");
    }

    #[test]
    fn scans_60_plus_presets_and_survives_the_awkward_files() {
        let (_t, lib) = fixture_lib(64);
        let n = lib.entries().len();
        assert_eq!(n, 64 + 4 + FACTORY_FILES.len());
        let broken = lib.get("User/Bass/Broken").expect("the corrupt file is listed");
        assert!(broken.is_damaged() && broken.problem.as_deref().unwrap().contains("not a GTS preset"), "{:?}", broken.problem);
        assert_eq!(broken.name, "Broken");
        let other = lib.get("User/Lead/Other Synth").unwrap();
        assert!(other.is_damaged() && other.problem.as_deref().unwrap().contains("VSS"));
        // Duplicate names: both listed, the later one flagged.
        let twins: Vec<_> = lib.entries().iter().filter(|e| e.name == "Twin").collect();
        assert_eq!(twins.len(), 2);
        assert_eq!(twins.iter().filter(|e| e.problem.is_some()).count(), 1);
        // Same name in Factory and User is not a duplicate (different folders).
        // Sorted: category order, then name within category.
        let mut last = (0, String::new());
        for e in lib.entries() {
            let k = (category_index(e.category).unwrap(), e.name.to_lowercase());
            assert!(k >= last, "{} after {:?}", e.name, last);
            last = k;
        }
    }

    #[test]
    fn search_matches_all_words_in_any_order_phrases_and_ignores_case() {
        let (_t, lib) = fixture_lib(64);
        let f = |text: &str| Filter { text: text.into(), ..all() };
        let n = |text: &str| lib.query(&f(text)).len();
        assert_eq!(n(""), lib.entries().len());
        assert!(n("growl") > 0);
        assert_eq!(n("GROWL"), n("growl"));
        // Words in any order.
        assert_eq!(n("dark growl"), n("growl dark"));
        assert!(n("dark growl") <= n("dark"));
        // Over author, description and tags too.
        assert!(n("mara") > 0 && n("voss") == n("mara"));
        assert!(n("edge") > 60);
        assert!(n("tearout") > 0);
        // Quoted phrase: contiguous, in order.
        assert!(n("\"with a\"") > 60);
        assert_eq!(n("\"a with\""), 0);
        assert_eq!(n("\"mara voss\""), n("mara voss"));
        assert!(n("\"voss mara\"") == 0 && n("voss mara") > 0);
        // Fields do not run together: the author "Dub Lab" and a name starting "Dark" are separate lines.
        assert_eq!(n("\"lab dark\""), 0);
        // Unterminated quote runs to the end; nothing matches nonsense.
        assert!(n("\"dark growl") <= n("dark growl"));
        assert_eq!(n("zzzzqq"), 0);
        // Factory descriptions are searched.
        assert!(n("riddim") >= 2);
    }

    #[test]
    fn source_category_favourite_tag_and_text_filters_combine() {
        let (_t, mut lib) = fixture_lib(64);
        let total = lib.entries().len();
        let f = Filter { category: None, ..Filter::default() };
        assert_eq!(lib.query(&f).len(), total);
        let fac = Filter { source: SourceFilter::Factory, ..f.clone() };
        assert_eq!(lib.query(&fac).len(), FACTORY_FILES.len());
        let usr = Filter { source: SourceFilter::User, ..f.clone() };
        assert_eq!(lib.query(&usr).len(), total - FACTORY_FILES.len());
        // Category narrows.
        let bass_user = Filter { source: SourceFilter::User, category: Some("Bass"), ..f.clone() };
        assert!(lib.query(&bass_user).iter().all(|&i| lib.entries()[i].origin == Origin::User && lib.entries()[i].category == "Bass"));
        // Favourites.
        let k1 = lib.entries()[0].key.clone();
        let k2 = lib.entries()[5].key.clone();
        lib.set_favorite(&k1, true).unwrap();
        lib.set_favorite(&k2, true).unwrap();
        let fav = Filter { source: SourceFilter::Favorites, ..f.clone() };
        assert_eq!(lib.query(&fav).len(), 2);
        let fav_text = Filter { text: lib.entries()[5].name.to_lowercase(), ..fav.clone() };
        assert_eq!(lib.query(&fav_text), vec![5]);
        lib.set_favorite(&k1, false).unwrap();
        assert_eq!(lib.query(&fav).len(), 1);
        // Tag: exact, case-insensitive; combines with text and category.
        let tag = Filter { tag: Some("Growl".into()), ..f.clone() };
        let tagged = lib.query(&tag);
        assert!(!tagged.is_empty() && tagged.iter().all(|&i| lib.entries()[i].tags.iter().any(|t| t == "growl")));
        let tag_cat = Filter { category: Some("Lead"), ..tag.clone() };
        assert!(lib.query(&tag_cat).len() < tagged.len());
        let tag_text = Filter { text: "dark".into(), ..tag };
        assert!(lib.query(&tag_text).iter().all(|i| tagged.contains(i)));
        // Everything at once yields the intersection.
        let all_in = Filter { source: SourceFilter::User, category: Some("Bass"), tag: Some("sub".into()), text: "edge".into() };
        for &i in &lib.query(&all_in) {
            let e = &lib.entries()[i];
            assert!(e.origin == Origin::User && e.category == "Bass" && e.tags.contains(&"sub".to_string()) && e.hay.contains("edge"));
        }
    }

    #[test]
    fn counts_and_tags_follow_the_other_filters() {
        let (_t, lib) = fixture_lib(64);
        let f = Filter { category: Some("Bass"), ..Filter::default() };
        let c = lib.counts(&f);
        // Category counts ignore the category filter, so they add up to the all-categories count.
        assert_eq!(c.categories.iter().sum::<usize>(), c.all_categories);
        assert_eq!(c.all_categories, lib.entries().len());
        // Source counts honour the category filter.
        assert_eq!(c.sources[0], lib.query(&f).len());
        assert_eq!(c.sources[1] + c.sources[2], c.sources[0]);
        // Text narrows every facet.
        let c2 = lib.counts(&Filter { text: "wub".into(), ..f.clone() });
        assert!(c2.all_categories < c.all_categories);
        let tags = lib.tags(&f);
        assert!(tags.windows(2).all(|w| w[0].1 >= w[1].1), "most used first");
        // Selecting a tag does not hide the other tags.
        let with_tag = lib.tags(&Filter { tag: Some(tags[0].0.clone()), ..f.clone() });
        assert_eq!(with_tag, tags);
    }

    #[test]
    fn names_are_validated() {
        assert_eq!(validate_name("  Dark Wub  "), Ok("Dark Wub".into()));
        assert_eq!(validate_name(""), Err(NameError::Empty));
        assert_eq!(validate_name("   "), Err(NameError::Empty));
        assert_eq!(validate_name(&"x".repeat(MAX_NAME_CHARS + 1)), Err(NameError::TooLong));
        assert!(validate_name(&"x".repeat(MAX_NAME_CHARS)).is_ok());
        for bad in ["a/b", "a\\b", "a:b", "a*b", "a?b", "a\"b", "a<b", "a>b", "a|b", "a\u{7}b"] {
            assert!(matches!(validate_name(bad), Err(NameError::IllegalChar(_))), "{bad:?}");
        }
        for bad in ["CON", "nul", "Com1", "LPT9", "aux.txt", ".hidden", "trailing."] {
            assert_eq!(validate_name(bad), Err(NameError::Reserved), "{bad:?}");
        }
        assert!(validate_name("COM0").is_ok() && validate_name("Console Bass").is_ok());
        assert_eq!(parse_tags("Dark, #Heavy; dark  wub-wub"), ["dark", "heavy", "wub-wub"]);
        assert_eq!(parse_tags(&"a ".repeat(40)).len(), 1);
        assert_eq!(parse_tags(&(0..40).map(|i| format!("t{i}")).collect::<Vec<_>>().join(",")).len(), MAX_TAGS);
    }

    fn sample(name: &str) -> Preset {
        let mut p = Preset { name: name.into(), author: "Tester".into(), description: "d".into(), category: "Bass".into(), tags: vec!["wub".into()], ..Preset::default() };
        p.patch.volume_db = -3.0;
        p
    }

    #[test]
    fn save_as_overwrite_rename_delete_round_trip_on_disk_and_in_the_list() {
        let (t, mut lib) = fixture_lib(8);
        let before = lib.entries().len();
        // Save As: a new file, listed, in the right folder, parseable, the same preset.
        let key = lib.save_as(&sample("My Wobble"), "Bass", false).unwrap();
        assert_eq!(key, "User/Bass/My Wobble");
        let path = t.0.join("Presets/User/Bass/My Wobble.json");
        assert!(path.is_file());
        assert_eq!(lib.entries().len(), before + 1);
        let (back, w) = Preset::load(&path).unwrap();
        assert!(w.is_empty());
        assert_eq!((back.name.as_str(), back.author.as_str(), back.patch.volume_db), ("My Wobble", "Tester", -3.0));
        assert_eq!(lib.get(&key).unwrap().tags, ["wub"]);
        // The same name again asks first (also with other casing, as the file system does) ...
        assert_eq!(lib.save_as(&sample("My Wobble"), "Bass", false), Err(LibError::Exists(key.clone())));
        assert_eq!(lib.save_as(&sample("my wobble"), "Bass", false), Err(LibError::Exists(key.clone())));
        // ... and replaces on request, once, with the new content.
        let mut again = sample("My Wobble");
        again.description = "second".into();
        assert_eq!(lib.save_as(&again, "Bass", true).unwrap(), key);
        assert_eq!(lib.entries().len(), before + 1);
        assert_eq!(lib.get(&key).unwrap().description, "second");
        // Another category is another folder: no conflict.
        assert_eq!(lib.save_as(&sample("My Wobble"), "Lead", false).unwrap(), "User/Lead/My Wobble");
        // Save (overwrite the loaded user preset) keeps name and category.
        let mut edited = sample("ignored");
        edited.patch.volume_db = -9.0;
        lib.save(&key, &edited).unwrap();
        let (back, _) = Preset::load(&path).unwrap();
        assert_eq!((back.name.as_str(), back.patch.volume_db), ("My Wobble", -9.0));
        // Rename: file and name inside move; favourite and default follow.
        lib.set_favorite(&key, true).unwrap();
        lib.set_default(Some(&key)).unwrap();
        let new_key = lib.rename(&key, "Renamed Wobble").unwrap();
        assert_eq!(new_key, "User/Bass/Renamed Wobble");
        assert!(!path.exists() && t.0.join("Presets/User/Bass/Renamed Wobble.json").is_file());
        assert!(lib.get(&key).is_none() && lib.get(&new_key).is_some());
        assert!(lib.is_favorite(&new_key) && !lib.is_favorite(&key));
        assert_eq!(lib.settings().default_preset, new_key);
        assert_eq!(Preset::load(&t.0.join("Presets/User/Bass/Renamed Wobble.json")).unwrap().0.name, "Renamed Wobble");
        // A case-only rename works (same file on Windows).
        assert_eq!(lib.rename(&new_key, "renamed wobble").unwrap(), "User/Bass/renamed wobble");
        let new_key = "User/Bass/renamed wobble".to_string();
        // Renaming onto an existing preset is refused; so are bad names.
        assert!(matches!(lib.rename(&new_key, "Broken"), Err(LibError::Exists(_))));
        assert_eq!(lib.rename(&new_key, "a/b"), Err(LibError::Name(NameError::IllegalChar('/'))));
        // Delete: gone from disk and list, favourite and default cleared.
        lib.delete(&new_key).unwrap();
        assert!(!t.0.join("Presets/User/Bass/renamed wobble.json").exists());
        assert!(lib.get(&new_key).is_none() && !lib.is_favorite(&new_key) && lib.settings().default_preset.is_empty());
        // Everything above survives a restart.
        let lib2 = Library::open(&t.0, FACTORY_FILES);
        assert!(lib2.get("User/Lead/My Wobble").is_some() && lib2.get(&new_key).is_none());
        assert_eq!(lib2.entries().len(), lib.entries().len());
    }

    #[test]
    fn illegal_names_never_reach_the_disk_and_factory_is_never_the_target() {
        let (t, mut lib) = fixture_lib(4);
        let n = lib.entries().len();
        for bad in ["", "..", "a/../b", "CON", "x:y"] {
            assert!(matches!(lib.save_as(&sample(bad), "Bass", true), Err(LibError::Name(_))), "{bad:?}");
        }
        assert_eq!(lib.entries().len(), n);
        let factory_key = lib.entries().iter().find(|e| e.origin == Origin::Factory).unwrap().key.clone();
        assert_eq!(lib.delete(&factory_key), Err(LibError::NotUser));
        assert_eq!(lib.rename(&factory_key, "Mine"), Err(LibError::NotUser));
        assert_eq!(lib.save(&factory_key, &sample("x")), Err(LibError::NotUser));
        assert_eq!(lib.delete("User/Nope/Missing"), Err(LibError::NotUser));
        // Save As of a factory preset's content lands in User.
        let p = (*lib.get(&factory_key).unwrap().preset.clone().unwrap()).clone();
        let key = lib.save_as(&Preset { name: format!("{} Copy", p.name), ..p }, "Bass", false).unwrap();
        assert!(key.starts_with("User/"));
        // A damaged file can be deleted but not renamed.
        assert_eq!(lib.rename("User/Bass/Broken", "Fixed"), Err(LibError::Damaged));
        lib.delete("User/Bass/Broken").unwrap();
        assert!(!t.0.join("Presets/User/Bass/Broken.json").exists());
        // No stray temp files.
        let mut files = vec![];
        walk(&t.0, 0, &mut files);
        let mut stray = vec![];
        fn find_tmp(p: &Path, out: &mut Vec<PathBuf>) {
            for e in fs::read_dir(p).unwrap().flatten() {
                if e.path().is_dir() {
                    find_tmp(&e.path(), out)
                } else if e.file_name().to_string_lossy().ends_with(".tmp") {
                    out.push(e.path())
                }
            }
        }
        find_tmp(&t.0, &mut stray);
        assert!(stray.is_empty(), "{stray:?}");
    }

    #[test]
    fn favourites_settings_and_default_persist_and_bad_files_fall_back() {
        let (t, mut lib) = fixture_lib(6);
        let key = lib.entries()[2].key.clone();
        lib.set_favorite(&key, true).unwrap();
        lib.set_default(Some(&key)).unwrap();
        lib.update_settings(|s| {
            s.author = "Me".into();
            s.browser = BrowserPrefs { source: "User".into(), category: "Lead".into(), load_on_select: false };
        })
        .unwrap();
        let lib2 = Library::open(&t.0, FACTORY_FILES);
        assert!(lib2.is_favorite(&key));
        assert_eq!(lib2.default_preset().map(|e| e.key.clone()), Some(key.clone()));
        assert_eq!(lib2.settings().author, "Me");
        assert_eq!(lib2.settings().browser, BrowserPrefs { source: "User".into(), category: "Lead".into(), load_on_select: false });
        // Damaged files: defaults, an issue noted, no panic.
        fs::write(t.0.join("settings.json"), "][").unwrap();
        fs::write(t.0.join("favorites.json"), "{\"favorites\": 5}").unwrap();
        let lib3 = Library::open(&t.0, FACTORY_FILES);
        assert_eq!(lib3.settings(), &Settings::default());
        assert!(!lib3.is_favorite(&key) && lib3.issues.len() == 2, "{:?}", lib3.issues);
        // A default that was deleted by hand resolves to nothing (Init).
        let mut lib4 = lib2;
        lib4.set_default(Some("User/Bass/Gone")).unwrap();
        assert!(lib4.default_preset().is_none());
    }

    #[test]
    fn two_instances_do_not_overwrite_each_others_favorites() {
        let (t, mut a) = fixture_lib(4);
        let mut b = Library::open(&t.0, FACTORY_FILES);
        let (k1, k2) = (a.entries()[0].key.clone(), a.entries()[1].key.clone());
        a.set_favorite(&k1, true).unwrap();
        b.set_favorite(&k2, true).unwrap(); // b never saw k1
        let c = Library::open(&t.0, FACTORY_FILES);
        assert!(c.is_favorite(&k1) && c.is_favorite(&k2));
        a.set_favorite(&k1, false).unwrap();
        let d = Library::open(&t.0, FACTORY_FILES);
        assert!(!d.is_favorite(&k1) && d.is_favorite(&k2));
    }

    #[test]
    fn rescan_picks_up_hand_copied_files_and_a_read_only_root_still_lists_the_bank() {
        let (t, mut lib) = fixture_lib(4);
        let n = lib.entries().len();
        let mut p = sample("Copied In");
        p.category = "Pad".into();
        fs::write(t.0.join("Presets/User/Pad/Copied In.json"), p.to_json()).unwrap();
        fs::create_dir_all(t.0.join("Presets/User/Pad/Sub")).unwrap();
        fs::write(t.0.join("Presets/User/Pad/Sub/Deeper.json"), sample("Deeper").to_json()).unwrap();
        // A file straight in User uses its own category field.
        let mut q = sample("Loose");
        q.category = "Pluck/short".into();
        fs::write(t.0.join("Presets/User/Loose.json"), q.to_json()).unwrap();
        assert_eq!(lib.entries().len(), n);
        lib.rescan();
        assert_eq!(lib.entries().len(), n + 3);
        assert_eq!(lib.get("User/Pad/Copied In").unwrap().category, "Pad");
        assert_eq!(lib.get("User/Pad/Sub/Deeper").unwrap().category, "Pad");
        assert_eq!(lib.get("User/Loose").unwrap().category, "Pluck");
        // A file in the wrong folder takes the folder's category (folder wins).
        fs::write(t.0.join("Presets/User/Lead/Elsewhere.json"), sample("Elsewhere").to_json()).unwrap();
        lib.rescan();
        assert_eq!(lib.get("User/Lead/Elsewhere").unwrap().category, "Lead");
        // Hand-deleted files vanish on rescan.
        fs::remove_file(t.0.join("Presets/User/Lead/Elsewhere.json")).unwrap();
        lib.rescan();
        assert!(lib.get("User/Lead/Elsewhere").is_none());
        // A root that cannot be written: the embedded bank is listed anyway, the problem is reported.
        let file_as_root = t.0.join("not-a-folder");
        fs::write(&file_as_root, "x").unwrap();
        let ro = Library::open(&file_as_root, FACTORY_FILES);
        assert!(!ro.issues.is_empty());
        assert_eq!(ro.entries().iter().filter(|e| e.origin == Origin::Factory).count(), FACTORY_FILES.len());
        assert!(ro.entries().iter().all(|e| e.path.is_none() && !e.is_damaged()));
    }

    #[test]
    fn oversized_and_unreadable_files_are_listed_as_damaged() {
        let (t, mut lib) = fixture_lib(2);
        fs::write(t.0.join("Presets/User/FX/Huge.json"), vec![b' '; (MAX_FILE_BYTES + 1) as usize]).unwrap();
        fs::write(t.0.join("Presets/User/FX/Binary.json"), [0xff, 0xfe, 0x00, 0x81]).unwrap();
        lib.rescan();
        for k in ["User/FX/Huge", "User/FX/Binary"] {
            let e = lib.get(k).unwrap();
            assert!(e.is_damaged() && e.problem.is_some(), "{k}");
        }
    }

    #[test]
    fn warnings_become_a_problem_note_but_the_preset_loads() {
        let t = Tmp::new();
        let text = r#"{"format_version": 1, "product": "GTS", "name": "Odd", "category": "FX", "params": {"not_a_param": 1, "osca_index": 9999}}"#;
        fs::create_dir_all(t.0.join("Presets/User/FX")).unwrap();
        fs::write(t.0.join("Presets/User/FX/Odd.json"), text).unwrap();
        let lib = Library::open(&t.0, FACTORY_FILES);
        let e = lib.get("User/FX/Odd").unwrap();
        assert!(!e.is_damaged());
        assert!(e.problem.as_deref().unwrap().starts_with("Loaded with 2 notes"), "{:?}", e.problem);
    }

    #[test]
    fn documents_folder_resolves_to_a_real_directory() {
        let d = documents_dir();
        assert!(d.is_absolute(), "{d:?}");
        let r = default_root();
        assert!(r.ends_with("Riddim Rack/GTS") || r.ends_with("Riddim Rack\\GTS"), "{r:?}");
        #[cfg(windows)]
        assert!(d.is_dir(), "{d:?}");
    }

    #[test]
    fn first_sentence_and_terms() {
        let (_t, lib) = fixture_lib(1);
        let e = lib.entries().iter().find(|e| e.name == "Growl Hound").unwrap();
        assert!(e.first_sentence().ends_with('.') && e.first_sentence().len() < e.description.len());
        assert_eq!(search_terms("  a  \"b c\" D "), ["a", "b c", "d"]);
        assert_eq!(search_terms("\"open end"), ["open end"]);
        assert!(search_terms("\"\"  ").is_empty());
    }
}
