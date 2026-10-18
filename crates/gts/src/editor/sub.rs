//! The SUB | NOISE module (`docs/design/MOTION-addendum-track7.md` 5): one 88 x 222 well at
//! (868, 348) with two tabs (instrument modes, so the lit segment is the mint page style), the sub
//! oscillator on one and the noise layer with its slope display on the other.

use super::modulator::{dropdown, put, DdStyle, Row};
use super::*;
use rr_ui::theme::metrics::CONTROL_H;

/// The module in window px.
pub const RECT: (f32, f32, f32, f32) = (868.0, 348.0, 88.0, 222.0);
/// Tab cells, local to the module: SUB 34 and NOISE 44 inside the 80 x 24 strip at (4, 8).
const TAB_W: [f32; 2] = [34.0, 44.0];
const TABS: [&str; 2] = ["SUB", "NOISE"];

fn tab_id() -> Id {
    Id::new("gts-subnoise-tab")
}

/// Selects the tab from outside (previews and tests): 0 = SUB, 1 = NOISE.
pub fn select_tab(ctx: &egui::Context, tab: usize) {
    ctx.data_mut(|d| d.insert_temp(tab_id(), tab.min(1)));
}

/// The selected tab.
pub fn selected(ctx: &egui::Context) -> usize {
    ctx.data(|d| d.get_temp::<usize>(tab_id())).unwrap_or(0)
}

/// Tilt of a noise type's spectrum, dB per octave.
pub fn tilt_db_oct(ty: usize) -> f32 {
    [0.0, -3.0, -6.0][ty.min(2)]
}

/// The noise spectrum at `f` Hz relative to 1 kHz, dB: the type's tilt plus the tone's one-pole
/// (low-pass below the middle, high-pass above it; `dsp::noise`: 20 kHz down to 200 Hz, 20 Hz up to 10 kHz).
pub fn noise_db(tilt: f32, tone: f32, f: f32) -> f32 {
    let mut db = tilt * (f / 1000.0).log2();
    if tone < 0.5 {
        let fc = 20000.0 * 0.01f32.powf((0.5 - tone) / 0.5);
        db -= 10.0 * (1.0 + (f / fc).powi(2)).log10();
    } else if tone > 0.5 {
        let fc = 20.0 * 500f32.powf((tone - 0.5) / 0.5);
        let r = (f / fc).powi(2);
        db += 10.0 * (r / (1.0 + r)).max(1e-6).log10();
    }
    db
}

pub(super) fn sub_noise(ui: &mut Ui, c: &dyn Ctl, at: &dyn Fn(f32, f32, f32, f32) -> Rect) {
    let ctx = ui.ctx().clone();
    let mrect = at(RECT.0, RECT.1, RECT.2, RECT.3);
    let mut tab = selected(&ctx);
    let mut shown = ctx.data(|d| d.get_temp::<(usize, f64)>(Id::new("gts-subnoise-shown"))).unwrap_or((tab, -1.0));
    let now = ui.input(|i| i.time);
    // Body: 90 ms out, 180 ms in (instant under reduced motion).
    let mut alpha = 1.0;
    if shown.1 > now {
        shown = (tab, -1.0);
    }
    if tab != shown.0 && shown.1 < 0.0 {
        if rr_ui::motion::reduced_motion(&ctx) {
            shown.0 = tab;
        } else {
            shown.1 = now;
        }
    }
    if shown.1 >= 0.0 {
        let t = (now - shown.1) as f32;
        if t < 0.09 {
            alpha = 1.0 - egui::emath::easing::cubic_in(t / 0.09);
        } else {
            shown.0 = tab;
            alpha = egui::emath::easing::cubic_out(((t - 0.09) / 0.18).min(1.0));
            if t >= 0.27 {
                shown.1 = -1.0;
            }
        }
    }
    let body = shown.0;
    rr_ui::widgets::well_at(ui, mrect, |ui, m| {
        // Tab strip: (4, 8) 80 x 24, no gap, the lit segment slides in the mint of the page switch.
        let strip = Rect::from_min_size(m.min + vec2(4.0, 8.0), vec2(80.0, 24.0));
        let p = ui.painter().clone();
        rr_ui::paint::inset_box(&p, strip, false);
        let t = rr_ui::motion::tween_value(ui.ctx(), Id::new("gts-subnoise-lit"), tab as f32, 0.18);
        let (lx, lw) = (2.0 + (TAB_W[0]) * t, TAB_W[0] + (TAB_W[1] - TAB_W[0]) * t);
        let lit = Rect::from_min_size(strip.min + vec2(lx, 2.0), vec2(lw - 2.0 + 0.0, 20.0));
        p.rect_filled(lit, 3.0, T.a(0.20));
        p.rect_stroke(lit, 3.0, rr_ui::paint::hair(&p, T.a(0.55)), StrokeKind::Inside);
        let mut x = strip.left() + 2.0;
        for (i, name) in TABS.iter().enumerate() {
            let seg = Rect::from_min_size(pos2(x, strip.top() + 2.0), vec2(TAB_W[i] - if i == 1 { 2.0 } else { 0.0 }, 20.0));
            x += TAB_W[i];
            let resp = ui.interact(seg, Id::new(("gts-subnoise-tab-seg", i)), Sense::click());
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                set_hover(ui, name, if i == 0 { "sub oscillator" } else { "noise layer" }, None, crate::help::ui_help(if i == 0 { "sub:tab" } else { "noise:tab" }).unwrap_or(""));
            }
            if resp.clicked() {
                tab = i;
            }
            let col = if tab == i { T.accent } else if resp.hovered() { T.text } else { T.muted };
            rr_ui::paint::caps(&p, seg.center(), Align2::CENTER_CENTER, name, 11.0, col);
        }
        ui.scope(|ui| {
            if alpha < 0.999 {
                ui.multiply_opacity(alpha);
            }
            if body == 0 {
                sub_body(ui, c, m);
            } else {
                noise_body(ui, c, m);
            }
        });
    });
    ctx.data_mut(|d| {
        d.insert_temp(tab_id(), tab);
        d.insert_temp(Id::new("gts-subnoise-shown"), shown);
    });
}

fn cell(m: Rect, x: f32, y: f32, w: f32) -> Rect {
    Rect::from_min_size(m.min + vec2(x, y), vec2(w, 48.0))
}

fn sub_body(ui: &mut Ui, c: &dyn Ctl, m: Rect) {
    let on = !is_off(c, "sub_on");
    // Shape: SIN | TRI.
    let sr = Rect::from_min_size(m.min + vec2(8.0, 40.0), vec2(72.0, 22.0));
    let cur = enum_idx::<SubShape>(c, "sub_shape");
    let p = ui.painter().clone();
    rr_ui::paint::inset_box(&p, sr, false);
    for (i, name) in ["SIN", "TRI"].iter().enumerate() {
        let seg = Rect::from_min_size(sr.min + vec2(2.0 + i as f32 * 34.0, 2.0), vec2(34.0, 18.0));
        let resp = ui.interact(seg, Id::new(("gts-sub-shape", i)), Sense::click());
        let lit = rr_ui::motion::tween(ui.ctx(), resp.id.with("lit"), cur == i, 0.18);
        if lit > 0.0 {
            p.rect_filled(seg, 3.0, T.a(0.20 * lit));
            p.rect_stroke(seg, 3.0, rr_ui::paint::hair(&p, T.a(0.55 * lit)), StrokeKind::Inside);
        }
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            set_hover(ui, &c.name("sub_shape").to_uppercase(), SubShape::variants()[cur], None, crate::help::help("sub_shape").unwrap_or(""));
        }
        if resp.clicked() && cur != i {
            one_shot(c, "sub_shape", i as f32);
        }
        rr_ui::paint::caps(&p, seg.center(), Align2::CENTER_CENTER, name, 11.0, if cur == i { T.accent } else if resp.hovered() { T.text } else { T.muted });
    }
    // Knobs: the sub's own are dimmed while it is off; Xover is the whole-output crossover and stays live.
    ui.scope(|ui| {
        fade(ui, !on);
        put(ui, cell(m, 8.0, 68.0, 36.0), |ui| kn(ui, c, "sub_oct", "Oct", ks(36.0).bipolar()));
        put(ui, cell(m, 44.0, 68.0, 36.0), |ui| kn(ui, c, "sub_phase", "Phase", ks(36.0)));
        put(ui, cell(m, 8.0, 116.0, 36.0), |ui| kn(ui, c, "sub_drive", "Drive", ks(36.0)));
        put(ui, cell(m, 44.0, 116.0, 36.0), |ui| kn(ui, c, "sub_level", "Level", ks(36.0)));
    });
    put(ui, cell(m, 8.0, 164.0, 36.0), |ui| kn(ui, c, "mono_xover", "Xover", ks(36.0)));
    // ON: a bare LED toggle under its label.
    let on_cell = cell(m, 44.0, 164.0, 36.0);
    rr_ui::paint::caps(ui.painter(), pos2(on_cell.center().x, on_cell.top() + 7.0), Align2::CENTER_CENTER, "On", 11.0, T.muted);
    put(ui, Rect::from_min_size(pos2(on_cell.center().x - 12.0, on_cell.top() + 14.0 + 4.0), vec2(24.0, 24.0)), |ui| tg(ui, c, "sub_on", ""));
}

fn noise_body(ui: &mut Ui, c: &dyn Ctl, m: Rect) {
    // Type: a 72 x 40 cell, label over a 24 px combo.
    let tc = Rect::from_min_size(m.min + vec2(8.0, 38.0), vec2(72.0, 40.0));
    rr_ui::paint::caps(ui.painter(), pos2(tc.center().x, tc.top() + 7.0), Align2::CENTER_CENTER, "Type", 11.0, T.muted);
    let cur = enum_idx::<NoiseType>(c, "noise_type");
    let rows: Vec<Row> = NoiseType::variants().iter().map(|n| Row::Item(n)).collect();
    let (pick, hov) = dropdown(ui, Rect::from_min_size(tc.min + vec2(0.0, 14.0), vec2(72.0, CONTROL_H)), "noise-type", NoiseType::variants()[cur], DdStyle::Normal, &rows, Some(cur));
    if hov {
        set_hover(ui, &c.name("noise_type").to_uppercase(), NoiseType::variants()[cur], None, crate::help::help("noise_type").unwrap_or(""));
    }
    if let Some(i) = pick {
        one_shot(c, "noise_type", i as f32 / (NoiseType::variants().len() - 1) as f32);
    }
    put(ui, cell(m, 8.0, 82.0, 36.0), |ui| kn(ui, c, "noise_level", "Level", ks(36.0)));
    put(ui, cell(m, 44.0, 82.0, 36.0), |ui| kn(ui, c, "noise_tone", "Tone", ks(36.0).bipolar()));
    // Slope display: the spectrum's tilt, with the tone's shelf; the line tilts over 180 ms when the type changes.
    let r = Rect::from_min_size(m.min + vec2(8.0, 142.0), vec2(72.0, 50.0));
    let p = ui.painter().clone();
    rr_ui::scope::small_glass(&p, r);
    let tilt = rr_ui::motion::tween_value(ui.ctx(), Id::new("gts-noise-tilt"), tilt_db_oct(cur), 0.18);
    let tone = c.get01("noise_tone");
    let inner = Rect::from_min_max(r.min + vec2(6.0, 6.0), pos2(r.right() - 6.0, r.bottom() - 16.0));
    let level_on = c.plain("noise_level", c.get01("noise_level")) > -59.95;
    let n = 36;
    let pts: Vec<Pos2> = (0..=n)
        .map(|i| {
            let f = 20.0 * 1000f32.powf(i as f32 / n as f32);
            let db = noise_db(tilt, tone, f);
            pos2(inner.left() + inner.width() * i as f32 / n as f32, inner.center().y - (db / 40.0).clamp(-1.0, 1.0) * inner.height() / 2.0)
        })
        .collect();
    p.with_clip_rect(r).add(egui::Shape::line(pts, rr_ui::paint::st(1.5, if level_on { T.accent } else { T.accent.gamma_multiply(0.35) })));
    let cap = if tilt_db_oct(cur) == 0.0 { "flat".to_string() } else { format!("{:.0} dB/oct", tilt_db_oct(cur)) };
    p.text(pos2(r.center().x, r.bottom() - 8.0), Align2::CENTER_CENTER, cap, font::mono(11.0), T.disp_caption);
    if ui.rect_contains_pointer(r) {
        set_hover(ui, "NOISE SLOPE", &cap_of(cur), None, crate::help::ui_help("noise:slope").unwrap_or(""));
    }
}

fn cap_of(ty: usize) -> String {
    format!("{} dB/oct", tilt_db_oct(ty))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tilt_and_tone_shape_the_slope() {
        assert_eq!(noise_db(-3.0, 0.5, 1000.0), 0.0);
        assert!((noise_db(-3.0, 0.5, 2000.0) + 3.0).abs() < 1e-4, "-3 dB per octave");
        assert!((noise_db(-6.0, 0.5, 4000.0) + 12.0).abs() < 1e-4);
        // Dark tone rolls the top off, bright tone rolls the bottom off.
        assert!(noise_db(0.0, 0.0, 10000.0) < -10.0 && noise_db(0.0, 0.0, 100.0) > -1.0);
        assert!(noise_db(0.0, 1.0, 100.0) < -10.0 && noise_db(0.0, 1.0, 20000.0) > -1.0);
    }
}
