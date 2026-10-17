//! SPEC v2 2.5 subset: every candidate preset in presets/candidates loads cleanly, renders the
//! examples/render_preset.rs pattern inside the SQ-37 band, the bank sits within ±1 dB K-weighted,
//! and the listen proxies hold (pitch, movement, sub energy, reese width, no NaN / clipping).
//! Every candidate also carries four labelled macros that each drive at least one matrix slot, and
//! every macro sweep (one held note, the macro 0 -> 1 over 4 s) is audible and stays inside the QA
//! band: see `rp::sweep_violations`.
#[path = "../examples/render_preset.rs"]
#[allow(dead_code)]
mod rp;

use gts::matrix::Source;
use gts::preset::Preset;

#[test]
fn candidate_presets_pass_qa() {
    let paths = rp::candidates();
    assert!(paths.len() >= 6, "expected the six candidates, found {}", paths.len());
    let mut fails = vec![];
    let (mut names, mut louds) = (vec![], vec![]);
    for path in &paths {
        let text = std::fs::read_to_string(path).unwrap();
        let (preset, warnings) = Preset::from_json(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let name = preset.name.clone();
        if !warnings.is_empty() {
            fails.push(format!("{name}: load warnings {warnings:?}"));
        }
        if text.replace("\r\n", "\n") != preset.to_json() {
            fails.push(format!("{name}: not in canonical complete form (run render_preset -- --canon)"));
        }
        if preset.author != "Riddim Rack" || preset.description.len() < 80 || preset.category.is_empty() {
            fails.push(format!("{name}: metadata incomplete"));
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
        let r = rp::render_preset(&preset);
        let q = rp::measure(&r);
        rp::print(&name, &q, r.held_note);
        fails.extend(rp::sq37(&q).into_iter().map(|v| format!("{name}: {v}")));
        if q.f0_err_pct.abs() > 1.0 || q.f0_rel_db < -40.0 {
            fails.push(format!("{name}: held-note fundamental {:.2} Hz ({:+.2} %, {:.1} dB re max)", q.f0_hz, q.f0_err_pct, q.f0_rel_db));
        }
        let check = |ok: bool, what: String, fails: &mut Vec<String>| {
            if !ok {
                fails.push(format!("{name}: {what}"));
            }
        };
        match name.as_str() {
            "Growl Hound" | "Talk Box" => {
                check(q.centroid_oct > 1.0, format!("centroid moves {:.2} oct in 300 ms (need > 1)", q.centroid_oct), &mut fails)
            }
            "Sub Weight" => check(q.low_share >= 0.9, format!("{:.1} % of energy below 150 Hz (need >= 90)", 100.0 * q.low_share), &mut fails),
            "Reese Drift" => check(q.corr_hi <= 0.6, format!("L/R correlation above 200 Hz {:.3} (need <= 0.6)", q.corr_hi), &mut fails),
            _ => {}
        }
        names.push(name);
        louds.push(q.lufs);
    }
    fails.extend(rp::bank_violations(&names, &louds));
    for want in ["Sub Weight", "Growl Hound", "Reese Drift", "Wobble Shot", "Talk Box", "Screech Lead"] {
        if !names.iter().any(|n| n == want) {
            fails.push(format!("missing candidate {want}"));
        }
    }
    assert!(fails.is_empty(), "preset QA failures:\n{}", fails.join("\n"));
}

#[test]
fn candidate_macro_sweeps_pass_qa() {
    let mut fails = vec![];
    for path in &rp::candidates() {
        let (preset, _) = Preset::load(path).unwrap_or_else(|e| panic!("{e}"));
        let labels = preset.matrix_state().macros;
        for (k, sw) in rp::sweeps(&preset).iter().enumerate() {
            let (c, l, lo, ch) = rp::effect(sw);
            println!("{} M{} {}: {c:.2} oct, {l:.2} LU, {lo:.2} dB, corr {ch:.3}, peak {:.2} dBFS", preset.name, k + 1, labels[k], sw.peak_db);
            fails.extend(rp::sweep_violations(&labels[k], sw).into_iter().map(|v| format!("{} M{} {}: {v}", preset.name, k + 1, labels[k])));
        }
    }
    assert!(fails.is_empty(), "macro sweep failures:
{}", fails.join("
"));
}
