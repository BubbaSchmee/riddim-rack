//! Factory bank structure: every preset in presets/factory loads cleanly, has complete metadata,
//! four labelled macros that each drive a matrix slot, and a description naming them. The audio
//! checks (levels, pitch, movement, macro sweeps, per-family targets) are the bank_qa tool, run by
//! the ignored `factory_bank_full_qa`.
#[path = "../examples/render_preset.rs"]
#[allow(dead_code)]
mod rp;

use gts::matrix::Source;
use gts::preset::{bank::FACTORY_FILES, Preset};

#[test]
fn candidate_presets_pass_qa() {
    // Structure of every factory preset (all families). The audio checks (loudness band, pitch,
    // macro sweeps, per-family targets) live in the bank_qa tool, run by `factory_bank_full_qa`.
    let paths = rp::candidates();
    assert_eq!(paths.len(), FACTORY_FILES.len(), "the factory tree and the embedded bank differ");
    let mut fails = vec![];
    for path in &paths {
        let text = std::fs::read_to_string(path).unwrap();
        let (preset, warnings) = Preset::from_json(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let name = preset.name.clone();
        if !warnings.is_empty() {
            fails.push(format!("{name}: load warnings {warnings:?}"));
        }
        if preset.author != "Riddim Rack" || preset.description.len() < 80 || preset.category.is_empty() {
            fails.push(format!("{name}: metadata incomplete"));
        }
        if name == "Init Square Bass" {
            continue;
        }
        let m = preset.matrix_state();
        for (k, label) in m.macros.iter().enumerate() {
            if label.trim().is_empty() || label.starts_with("Macro ") {
                fails.push(format!("{name}: macro {} has no purpose label ({label:?})", k + 1));
            }
            let src = Source::from_index(Source::Macro1 as usize + k).unwrap();
            if !m.slots.iter().any(|s| s.source == src && s.enabled && s.amount != 0.0) {
                fails.push(format!("{name}: macro {} ({label}) drives no matrix slot", k + 1));
            }
        }
        let last = preset.description.rsplit(". ").next().unwrap_or("");
        if let Some(l) = m.macros.iter().find(|l| !last.contains(l.as_str())) {
            fails.push(format!("{name}: the last sentence of the description does not name the macro {l:?}"));
        }
    }
    assert!(fails.is_empty(), "preset QA failures:
{}", fails.join("
"));
}

/// Fast structure test: every factory preset (the whole tree, all categories) loads clean, has four
/// non-empty macro labels (Init excepted) and a non-empty description.
#[test]
fn factory_bank_is_well_formed() {
    let mut fails = vec![];
    let mut seen = std::collections::HashSet::new();
    for path in &rp::candidates() {
        let text = std::fs::read_to_string(path).unwrap();
        let (p, warnings) = Preset::from_json(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        if !warnings.is_empty() {
            fails.push(format!("{}: load warnings {warnings:?}", p.name));
        }
        if !seen.insert(p.name.clone()) {
            fails.push(format!("{}: duplicate name", p.name));
        }
        if path.file_stem().unwrap().to_string_lossy() != p.name {
            fails.push(format!("{}: file name differs from preset name", p.name));
        }
        if p.name != "Init Square Bass" {
            let labels = p.matrix_state().macros;
            if labels.iter().any(|l| l.trim().is_empty()) {
                fails.push(format!("{}: empty macro label {labels:?}", p.name));
            }
        }
        if p.description.trim().is_empty() {
            fails.push(format!("{}: missing producer description", p.name));
        }
    }
    assert!(fails.is_empty(), "factory bank problems:
{}", fails.join("
"));
}

/// Slow: the full bank_qa tool over the whole factory tree in one run (bank checks included; they
/// are only judged when the whole bank is in one run). Needs `cargo build --release --example bank_qa`.
#[test]
#[ignore]
fn factory_bank_full_qa() {
    // <target>/release/deps/preset_qa-*.exe (or debug/deps) -> <target>/release/examples/bank_qa
    let me = std::env::current_exe().unwrap();
    let target = me.parent().unwrap().parent().unwrap().parent().unwrap();
    let exe = target.join("release/examples").join(format!("bank_qa{}", std::env::consts::EXE_SUFFIX));
    assert!(exe.exists(), "build it first: cargo build --release --example bank_qa ({})", exe.display());
    let out = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("gts_bank_qa");
    let st = std::process::Command::new(exe)
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .arg("--out")
        .arg(&out)
        .arg(rp::candidates_dir())
        .status()
        .expect("run bank_qa");
    assert!(st.success(), "bank_qa reported failed checks, see {}", out.display());
}
