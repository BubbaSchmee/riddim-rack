//! SPEC v2 2.5 subset: every candidate preset in presets/candidates loads cleanly, renders the
//! examples/render_preset.rs pattern inside the SQ-37 band, the bank sits within ±1 dB K-weighted,
//! and the listen proxies hold (pitch, movement, sub energy, reese width, no NaN / clipping).
#[path = "../examples/render_preset.rs"]
#[allow(dead_code)]
mod rp;

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
        let r = rp::render(&preset.patch, rp::is_lead(&preset));
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
