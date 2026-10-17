//! Embeds every `presets/factory/<Category>/*.json` as `(relative path, contents)` in
//! `$OUT_DIR/factory_bank.rs`, so a new factory preset is a file drop (no Rust edit). The
//! categories are checked against `library::CATEGORIES` by a unit test.

use std::path::Path;
use std::{env, fs};

fn main() {
    let root = Path::new(&env::var("CARGO_MANIFEST_DIR").expect("cargo sets it")).join("presets/factory");
    println!("cargo:rerun-if-changed=presets/factory");
    println!("cargo:rerun-if-changed=build.rs");
    let mut files: Vec<(String, String)> = vec![];
    if let Ok(cats) = fs::read_dir(&root) {
        for cat in cats.flatten().filter(|c| c.path().is_dir()) {
            let cname = cat.file_name().to_string_lossy().to_string();
            for f in fs::read_dir(cat.path()).expect("category folder").flatten() {
                let p = f.path();
                if p.extension().is_some_and(|e| e == "json") {
                    let rel = format!("{cname}/{}", f.file_name().to_string_lossy());
                    files.push((rel, p.to_string_lossy().replace('\\', "/")));
                }
            }
        }
    }
    files.sort();
    let mut out = String::from("/// Embedded factory bank: (\"<Category>/<Name>.json\", file contents), sorted.\npub const FACTORY_FILES: &[(&str, &str)] = &[\n");
    for (rel, abs) in &files {
        out += &format!("    ({rel:?}, include_str!({abs:?})),\n");
    }
    out += "];\n";
    let dest = Path::new(&env::var("OUT_DIR").expect("cargo sets it")).join("factory_bank.rs");
    fs::write(dest, out).expect("write factory_bank.rs");
}
