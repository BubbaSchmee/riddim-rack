//! The rack panel: gradient face, two rails with slot pattern, four corner screws and the
//! nameplate. Call inside `CentralPanel::default().frame(Frame::NONE)` so it owns the window.

use crate::paint;
use crate::theme::{font, metrics::*, T};
use egui::{pos2, vec2, Align2, Color32, Rect, Sense, StrokeKind, Ui, UiBuilder, Vec2};

/// Screw slot angles in degrees: top-left, bottom-left, top-right, bottom-right.
const SCREW_ANGLES: [f32; 4] = [20.0, 75.0, 130.0, 50.0];

fn screw(p: &egui::Painter, c: egui::Pos2, angle_deg: f32) {
    p.circle_filled(c, 7.5, Color32::BLACK);
    p.circle_filled(c, 7.0, T.cap1);
    p.circle_filled(c - vec2(1.5, 1.5), 4.5, T.cap2);
    let a = angle_deg.to_radians();
    let d = vec2(a.cos(), a.sin()) * 5.0;
    p.line_segment([c - d, c + d], paint::st(2.0, Color32::from_rgb(0x05, 0x05, 0x06)));
}

fn rail(p: &egui::Painter, r: Rect, left: bool) {
    let dark = T.rail.lerp_to_gamma(Color32::BLACK, 0.3);
    paint::hgradient(p, r, T.rail, dark);
    let edge_x = if left { r.right() - 0.5 } else { r.left() + 0.5 };
    p.line_segment([pos2(edge_x, r.top()), pos2(edge_x, r.bottom())], paint::st(1.0, T.lo()));
    let mut y = r.top() + 44.0;
    while y < r.bottom() - 60.0 {
        let slot = Rect::from_min_size(pos2(r.left() + 8.0, y), vec2(10.0, 18.0));
        paint::highlight_below(p, slot, 5.0);
        p.rect_filled(slot, 5.0, T.rail_slot);
        y += 40.0;
    }
}

/// Nameplate: engraved name, tracked descriptor and a status line with an LED.
fn nameplate(p: &egui::Painter, r: Rect, name: &str, descriptor: &str, status: &str) {
    paint::highlight_below(p, r, GROUP_R);
    paint::rounded_vgradient(p, r, GROUP_R, T.panel_bottom, T.well, true);
    p.rect_stroke(r, GROUP_R, paint::st(1.0, T.lo()), StrokeKind::Inside);
    // Descriptor wraps to two lines when it does not fit the plate.
    let desc = descriptor.to_uppercase();
    let fits = |s: &str| paint::text_width(p, &font::label(11.0), s, 1.6) <= r.width() - 32.0;
    let lines: Vec<String> = if fits(&desc) {
        vec![desc]
    } else {
        let words: Vec<&str> = desc.split(' ').collect();
        let k = (1..words.len()).min_by_key(|&k| (words[..k].join(" ").len() as i32 - words[k..].join(" ").len() as i32).abs()).unwrap_or(1);
        vec![words[..k].join(" "), words[k..].join(" ")]
    };
    // Stacked block, centred: name 36, descriptor lines of 14 (+4 gap), status 14 (+12 gap).
    let block = 36.0 + 4.0 + 14.0 * lines.len() as f32 + 12.0 + 14.0;
    let x = r.left() + 16.0;
    let mut y = r.center().y - block / 2.0;
    let f = font::strong(30.0);
    let name_pos = pos2(x, y);
    paint::tracked(p, name_pos + vec2(0.0, -1.0), Align2::LEFT_TOP, name, f.clone(), Color32::BLACK.gamma_multiply(0.6), 8.0);
    paint::tracked(p, name_pos + vec2(0.0, 1.0), Align2::LEFT_TOP, name, f.clone(), Color32::WHITE.gamma_multiply(0.08), 8.0);
    paint::tracked(p, name_pos, Align2::LEFT_TOP, name, f, T.plate_text, 8.0);
    y += 36.0 + 4.0;
    for l in &lines {
        paint::tracked(p, pos2(x, y + 7.0), Align2::LEFT_CENTER, l, font::label(11.0), T.muted, 1.6);
        y += 14.0;
    }
    y += 12.0;
    paint::led(p, pos2(x + 4.0, y + 7.0), true);
    paint::tracked(p, pos2(x + 16.0, y + 7.0), Align2::LEFT_CENTER, status, font::label(11.0), T.muted, 0.0);
}

/// Panel face, rails, slots and screws over `ui.max_rect()` (returned). Lay the content out
/// yourself (the v2 GTS panel), or use [`rack_panel`].
pub fn rack_frame(ui: &Ui) -> Rect {
    let full = ui.max_rect();
    let p = ui.painter();
    paint::vgradient(p, full, T.panel_top, T.panel_bottom);
    p.line_segment([full.left_top() + vec2(0.0, 0.5), full.right_top() + vec2(0.0, 0.5)], paint::st(1.0, T.hi()));
    rail(p, Rect::from_min_size(full.min, vec2(RAIL_W, full.height())), true);
    rail(p, Rect::from_min_size(pos2(full.right() - RAIL_W, full.top()), vec2(RAIL_W, full.height())), false);
    let (l, r, t, b) = (full.left() + 14.0, full.right() - 14.0, full.top() + 19.0, full.bottom() - 19.0);
    for (c, a) in [(pos2(l, t), 0), (pos2(l, b), 1), (pos2(r, t), 2), (pos2(r, b), 3)] {
        screw(p, c, SCREW_ANGLES[a]);
    }
    p.rect_stroke(full, 0.0, paint::st(1.0, Color32::BLACK), StrokeKind::Inside);
    full
}

/// Compact v2 nameplate (MOTION.md 2.1, 156 x 56): engraved name with the note LED and note name
/// right of it (lit while a gate is open), tracked descriptor below.
pub fn nameplate_v2(p: &egui::Painter, r: Rect, name: &str, descriptor: &str, note: Option<&str>) {
    paint::highlight_below(p, r, GROUP_R);
    paint::rounded_vgradient(p, r, GROUP_R, T.panel_bottom, T.well, true);
    p.rect_stroke(r, GROUP_R, paint::st(1.0, T.lo()), StrokeKind::Inside);
    let f = font::strong(24.0);
    let at = pos2(r.left() + 14.0, r.top() + 7.0);
    paint::tracked(p, at + vec2(0.0, -1.0), Align2::LEFT_TOP, name, f.clone(), Color32::BLACK.gamma_multiply(0.6), 6.0);
    paint::tracked(p, at + vec2(0.0, 1.0), Align2::LEFT_TOP, name, f.clone(), Color32::WHITE.gamma_multiply(0.08), 6.0);
    let nr = paint::tracked(p, at, Align2::LEFT_TOP, name, f, T.plate_text, 6.0);
    let led = pos2(r.right() - 44.0, nr.center().y);
    paint::led(p, led, note.is_some());
    if let Some(n) = note {
        p.text(led + vec2(10.0, 0.0), Align2::LEFT_CENTER, n, font::mono(12.0), T.text);
    }
    paint::tracked(p, pos2(r.left() + 14.0, r.bottom() - 12.0), Align2::LEFT_CENTER, &descriptor.to_uppercase(), font::label(11.0), T.muted, 1.6);
}

/// Draws the panel over `ui.max_rect()` and runs `add` with two child uis: `header` (right of
/// the nameplate, `header_h` tall) and `body` (below the header, to the bottom padding).
/// `plate` is the nameplate size (GTS 232 x 124, VSS 184 x 116); `status` is the line with the
/// LED, e.g. "Two-osc graintable · v1.0".
pub fn rack_panel<R>(
    ui: &mut Ui,
    name: &str,
    descriptor: &str,
    status: &str,
    plate: Vec2,
    add: impl FnOnce(&mut Ui, &mut Ui) -> R,
) -> R {
    let full = rack_frame(ui);
    let p = ui.painter().clone();

    let content = Rect::from_min_max(
        pos2(full.left() + PAD_X, full.top() + PAD_TOP),
        pos2(full.right() - PAD_X, full.bottom() - PAD_BOTTOM),
    );
    let plate_rect = Rect::from_min_size(content.min, plate);
    nameplate(&p, plate_rect, name, descriptor, status);

    let header = Rect::from_min_max(pos2(plate_rect.right() + 16.0, content.top()), pos2(content.right(), plate_rect.bottom()));
    let body = Rect::from_min_max(pos2(content.left(), plate_rect.bottom() + 16.0), content.max);
    let mut hui = ui.new_child(UiBuilder::new().max_rect(header));
    let mut bui = ui.new_child(UiBuilder::new().max_rect(body));
    let out = add(&mut hui, &mut bui);
    // Swallow the area so the parent does not lay anything underneath.
    ui.allocate_rect(full, Sense::hover());
    out
}
