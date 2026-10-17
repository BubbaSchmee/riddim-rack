//! The help bar needs one accurate line for every parameter, and no line without a parameter.
use gts::help::{help, HELP};
use gts::params::GtsParams;
use nih_plug::prelude::Params;
use nih_plug_egui::egui;
use std::collections::BTreeSet;

fn ids() -> BTreeSet<String> {
    GtsParams::default().param_map().into_iter().map(|(id, _, _)| id).collect()
}

#[test]
fn every_param_has_a_help_line_and_every_line_a_param() {
    let all = ids();
    assert_eq!(all.len(), 182);
    for id in &all {
        let line = help(id).unwrap_or_else(|| panic!("no help for {id}"));
        assert!(line.trim().len() >= 20, "help for {id} is too thin: {line:?}");
    }
    let listed: Vec<&str> = HELP.iter().map(|(k, _)| *k).collect();
    let unique: BTreeSet<&str> = listed.iter().copied().collect();
    assert_eq!(listed.len(), unique.len(), "a param has two help lines");
    for k in &listed {
        assert!(all.contains(*k), "help for unknown param {k}");
    }
    assert_eq!(listed.len(), 182);
}

#[test]
fn help_lines_are_plain_sentences_with_no_third_party_names() {
    // No product, brand or device names anywhere in the shipped text.
    let banned = ["serum", "vital", "massive", "reason", "propellerhead", "malstr", "ableton", "fl studio", "xfer", "native instruments", "reaktor", "diva", "pigments"];
    for (id, line) in HELP {
        assert!(line.ends_with('.'), "{id}: help should be a sentence: {line:?}");
        assert!(line.len() <= 115, "{id}: help is {} chars: {line:?}", line.len());
        let low = line.to_lowercase();
        for b in banned {
            assert!(!low.contains(b), "{id}: help names {b}");
        }
    }
}

/// The help bar prints name, value and live value first; the line gets what is left of the
/// 912 px bar (about 520 px at the worst). Longer lines are clipped, not wrapped, so keep them short.
#[test]
fn help_lines_fit_the_bar() {
    let ctx = egui::Context::default();
    rr_ui::theme::apply(&ctx);
    let mut widest = (0.0f32, "");
    let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(100.0, 100.0))), ..Default::default() };
    for _ in 0..2 {
        let _ = ctx.run(input.clone(), |ctx| {
            for (id, line) in HELP {
                let w = ctx.fonts(|f| f.layout_no_wrap(line.to_string(), rr_ui::theme::font::label(12.0), egui::Color32::WHITE).size().x);
                if w > widest.0 {
                    widest = (w, id);
                }
            }
        });
    }
    assert!(widest.0 <= 560.0, "{} is {:.0} px wide", widest.1, widest.0);
}
