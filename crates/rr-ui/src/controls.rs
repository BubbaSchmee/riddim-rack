//! Controls a browser or a dialog needs beyond the instrument widgets: buttons, text inputs, tag
//! chips, the heart, a scrollbar, a context menu and a modal frame. All in the approved look
//! (docs/design/DESIGN.md): inset bodies, accent for state, caps labels, no stock egui chrome.
//! Everything takes an exact `Rect` (the plugin lays out in absolute window coordinates) and
//! honours Reduced motion through [`crate::motion`].

use crate::motion;
use crate::paint;
use crate::theme::{font, metrics::*, SRC_E, T};
use egui::epaint::{Mesh, Vertex, WHITE_UV};
use egui::{pos2, vec2, Align, Align2, Color32, Context, CursorIcon, Id, Key, Order, Painter, Pos2, Rect, Response, Sense, Shape, StrokeKind, Ui, Vec2};

// ---- button ------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonKind {
    /// Inset, caps label.
    Normal,
    /// The action a dialog is for: accent fill and stroke.
    Primary,
    /// Destructive: the error rose of the source palette.
    Danger,
}

/// A 24 px caps-label button in `rect`. A disabled one is dimmed to 45 % and senses nothing.
pub fn button(ui: &mut Ui, rect: Rect, label: &str, kind: ButtonKind, enabled: bool) -> Response {
    let id = ui.id().with(("rr-button", label, rect.min.x as i32, rect.min.y as i32));
    let resp = ui.interact(rect, id, if enabled { Sense::click() } else { Sense::hover() });
    let (hot, down) = (enabled && resp.hovered(), enabled && resp.is_pointer_button_down_on());
    if enabled && resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    let a = if enabled { 1.0 } else { 0.45 };
    let p = ui.painter();
    let tint = match kind {
        ButtonKind::Normal => None,
        ButtonKind::Primary => Some(T.accent),
        ButtonKind::Danger => Some(SRC_E),
    };
    paint::highlight_below(p, rect, 4.0);
    match tint {
        None => {
            p.rect_filled(rect, 4.0, T.inset.gamma_multiply(a));
            if hot {
                p.rect_filled(rect, 4.0, Color32::WHITE.gamma_multiply(if down { 0.02 } else { 0.05 }));
            }
            p.rect_stroke(rect, 4.0, paint::hair(p, T.lo()), StrokeKind::Inside);
        }
        Some(c) => {
            p.rect_filled(rect, 4.0, T.inset.gamma_multiply(a));
            p.rect_filled(rect, 4.0, c.gamma_multiply(a * if down { 0.34 } else if hot { 0.28 } else { 0.18 }));
            p.rect_stroke(rect, 4.0, paint::hair(p, c.gamma_multiply(a * 0.6)), StrokeKind::Inside);
        }
    }
    let col = match tint {
        Some(c) => c,
        None if hot => T.text,
        None => T.muted,
    };
    paint::caps(p, rect.center() + vec2(0.0, down as u8 as f32), Align2::CENTER_CENTER, label, 11.0, col.gamma_multiply(a));
    resp
}

/// Width a [`button`] needs for `label`.
pub fn button_width(p: &Painter, label: &str) -> f32 {
    (paint::text_width(p, &font::label(11.0), &label.to_uppercase(), 0.5) + 24.0).ceil().max(56.0)
}

// ---- text input --------------------------------------------------------------------------------

/// What a text input reports for a frame.
pub struct TextOut {
    pub resp: Response,
    pub changed: bool,
    /// Enter was pressed while it had focus (single line).
    pub enter: bool,
}

/// A bare egui text edit in `rect` (no box): label font, the panel's text colour, hint text in
/// muted, at most `max_chars` characters. `multiline` wraps and takes Enter as a new line.
pub fn text_edit(ui: &mut Ui, rect: Rect, id: Id, text: &mut String, hint: &str, max_chars: usize, multiline: bool) -> TextOut {
    let hint = egui::RichText::new(hint).font(font::label(13.0)).color(T.muted);
    let edit = if multiline { egui::TextEdit::multiline(text).desired_rows(3).lock_focus(true) } else { egui::TextEdit::singleline(text) };
    let edit = edit
        .id(id)
        .font(font::label(13.0))
        .text_color(T.text)
        .frame(false)
        .margin(vec2(8.0, 4.0))
        .char_limit(max_chars)
        .hint_text(hint)
        .desired_width(rect.width())
        .vertical_align(if multiline { Align::Min } else { Align::Center });
    if !multiline {
        let out = ui.put(rect, edit);
        let enter = out.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        return TextOut { changed: out.changed(), enter, resp: out };
    }
    // Multi-line: a scroll area the size of `rect` (egui grows a bare text edit past it), its own
    // bar drawn in the theme, the wheel and the caret scroll it.
    let off_id = id.with("scroll-offset");
    let mut area = egui::ScrollArea::vertical()
        .id_salt(id.with("scroll-area"))
        .auto_shrink([false, false])
        .max_height(rect.height())
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
    if let Some(o) = ui.data_mut(|d| d.remove_temp::<f32>(off_id)) {
        area = area.vertical_scroll_offset(o);
    }
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    let shown = area.show(&mut child, |ui| ui.add(edit));
    let track = Rect::from_min_max(pos2(rect.right() - SCROLLBAR_W - 3.0, rect.top() + 4.0), pos2(rect.right() - 3.0, rect.bottom() - 4.0));
    let mut off = shown.state.offset.y;
    if scrollbar(ui, track, id.with("scroll-bar"), shown.content_size.y, rect.height(), &mut off) {
        ui.data_mut(|d| d.insert_temp(off_id, off));
    }
    let out = shown.inner;
    TextOut { changed: out.changed(), enter: false, resp: out }
}

/// The input's body: inset box, with a 1.5 px accent ring while it has focus.
pub fn field_box(ui: &Ui, rect: Rect, focused: bool, hot: bool) {
    let p = ui.painter();
    paint::inset_box(p, rect, hot && !focused);
    if focused {
        p.rect_stroke(rect, 4.0, paint::st(1.5, T.accent.gamma_multiply(0.55)), StrokeKind::Inside);
    }
}

/// A labelled-by-hint text field: box + [`text_edit`].
pub fn text_field(ui: &mut Ui, rect: Rect, id: Id, text: &mut String, hint: &str, max_chars: usize, multiline: bool) -> TextOut {
    let focused = ui.memory(|m| m.has_focus(id));
    let hot = ui.rect_contains_pointer(rect);
    field_box(ui, rect, focused, hot);
    text_edit(ui, rect, id, text, hint, max_chars, multiline)
}

// ---- chip --------------------------------------------------------------------------------------

pub const CHIP_H: f32 = 20.0;

pub fn chip_width(p: &Painter, label: &str) -> f32 {
    (paint::text_width(p, &font::label(12.0), label, 0.0) + 18.0).ceil()
}

/// A tag pill. `active` lights it in accent. Returns the response (click toggles in the caller).
pub fn chip(ui: &mut Ui, rect: Rect, label: &str, active: bool) -> Response {
    let id = ui.id().with(("rr-chip", label, rect.min.x as i32, rect.min.y as i32));
    let resp = ui.interact(rect, id, Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    let p = ui.painter();
    let r = CHIP_H / 2.0;
    if active {
        p.rect_filled(rect, r, T.a(0.20));
        p.rect_stroke(rect, r, paint::hair(p, T.a(0.65)), StrokeKind::Inside);
    } else {
        p.rect_filled(rect, r, T.well);
        p.rect_stroke(rect, r, paint::hair(p, if resp.hovered() { Color32::WHITE.gamma_multiply(0.18) } else { T.lo() }), StrokeKind::Inside);
    }
    let col = if active { T.accent } else if resp.hovered() { T.text } else { T.muted };
    p.text(rect.center(), Align2::CENTER_CENTER, label, font::label(12.0), col);
    resp
}

// ---- heart -------------------------------------------------------------------------------------

/// Outline of a heart `size` px wide centred on `c` (closed, 28 points).
pub fn heart_points(c: Pos2, size: f32) -> Vec<Pos2> {
    let s = size / 32.0;
    (0..28)
        .map(|i| {
            let t = i as f32 / 28.0 * std::f32::consts::TAU;
            let x = 16.0 * t.sin().powi(3);
            let y = 13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
            pos2(c.x + x * s, c.y - (y + 2.5) * s)
        })
        .collect()
}

/// A heart: an outline, or filled (a fan from the centre, edged with a stroke for smooth borders).
pub fn heart(p: &Painter, c: Pos2, size: f32, filled: bool, color: Color32) {
    let pts = heart_points(c, size);
    if filled {
        let mut m = Mesh::default();
        m.vertices.push(Vertex { pos: c, uv: WHITE_UV, color });
        m.vertices.extend(pts.iter().map(|&pos| Vertex { pos, uv: WHITE_UV, color }));
        for i in 0..pts.len() as u32 {
            m.indices.extend_from_slice(&[0, 1 + i, 1 + (i + 1) % pts.len() as u32]);
        }
        p.add(Shape::mesh(m));
    }
    p.add(Shape::closed_line(pts, paint::st(if filled { 1.0 } else { 1.3 }, color)));
}

// ---- scrollbar ---------------------------------------------------------------------------------

pub const SCROLLBAR_W: f32 = 8.0;

/// A thin vertical scrollbar in `track` for content `content_h` tall seen through `view_h`.
/// Dragging the thumb or clicking the track edits `offset` (px, 0 ..= content_h - view_h).
/// Nothing is drawn when everything fits. Returns whether `offset` changed.
pub fn scrollbar(ui: &mut Ui, track: Rect, id: Id, content_h: f32, view_h: f32, offset: &mut f32) -> bool {
    let max = (content_h - view_h).max(0.0);
    if max <= 0.5 {
        return false;
    }
    let thumb_h = (track.height() * view_h / content_h).max(28.0).min(track.height());
    let travel = (track.height() - thumb_h).max(1.0);
    let thumb_y = track.top() + travel * (*offset / max).clamp(0.0, 1.0);
    let thumb = Rect::from_min_size(pos2(track.left(), thumb_y), vec2(track.width(), thumb_h));
    let resp = ui.interact(track, id, Sense::click_and_drag());
    let before = *offset;
    if resp.dragged() {
        *offset += resp.drag_delta().y * max / travel;
    } else if resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            if pos.y < thumb.top() {
                *offset -= view_h * 0.9;
            } else if pos.y > thumb.bottom() {
                *offset += view_h * 0.9;
            }
        }
    }
    *offset = offset.clamp(0.0, max);
    let hot = resp.hovered() || resp.dragged();
    if hot {
        ui.ctx().set_cursor_icon(CursorIcon::Default);
    }
    let k = motion::tween(ui.ctx(), id.with("hot"), hot, 0.12);
    let p = ui.painter();
    let w = 4.0 + 2.0 * k;
    let lane = Rect::from_center_size(track.center(), vec2(w, track.height()));
    p.rect_filled(lane, w / 2.0, T.rail_slot.gamma_multiply(0.8));
    let body = Rect::from_center_size(pos2(track.center().x, thumb.center().y), vec2(w, thumb_h));
    let col = if resp.dragged() { T.accent.gamma_multiply(0.7) } else { T.track };
    p.rect_filled(body, w / 2.0, col);
    if k > 0.0 && !resp.dragged() {
        p.rect_filled(body, w / 2.0, T.accent.gamma_multiply(0.35 * k));
    }
    *offset != before
}

// ---- context menu ------------------------------------------------------------------------------

/// One row of a [`context_menu`].
#[derive(Clone, Copy, Debug)]
pub struct MenuItem<'a> {
    pub label: &'a str,
    pub enabled: bool,
    pub danger: bool,
    pub checked: bool,
    /// A divider above this row.
    pub sep_before: bool,
}

impl<'a> MenuItem<'a> {
    pub fn new(label: &'a str) -> Self {
        Self { label, enabled: true, danger: false, checked: false, sep_before: false }
    }
    pub fn enabled(mut self, on: bool) -> Self {
        self.enabled = on;
        self
    }
    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }
    pub fn checked(mut self, on: bool) -> Self {
        self.checked = on;
        self
    }
    pub fn sep(mut self) -> Self {
        self.sep_before = true;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuEvent {
    Open,
    Picked(usize),
    /// A click elsewhere or Esc.
    Dismissed,
}

pub const MENU_ROW_H: f32 = CONTROL_H;

/// A context menu at `pos` (kept inside the screen), 24 px rows in the DESIGN popup look. It fades
/// in over 90 ms; a click anywhere else or Esc dismisses it, and that click is swallowed.
pub fn context_menu(ctx: &Context, id: Id, pos: Pos2, items: &[MenuItem], k: f32) -> MenuEvent {
    let f = font::label(13.0);
    let w = ctx.fonts(|fo| items.iter().map(|i| fo.layout_no_wrap(i.label.to_string(), f.clone(), Color32::WHITE).size().x).fold(0.0, f32::max)) + 44.0;
    let h = items.len() as f32 * MENU_ROW_H + items.iter().filter(|i| i.sep_before).count() as f32 * 9.0 + 8.0;
    let screen = ctx.screen_rect();
    let at = pos2(pos.x.min(screen.right() - w - 4.0).max(screen.left() + 4.0), pos.y.min(screen.bottom() - h - 4.0).max(screen.top() + 4.0));
    let mut ev = MenuEvent::Open;
    egui::Area::new(id).order(Order::Foreground).fixed_pos(screen.min).interactable(true).show(ctx, |ui| {
        ui.multiply_opacity(k);
        let veil = ui.interact(screen, id.with("veil"), Sense::click_and_drag());
        if veil.clicked() || veil.secondary_clicked() || veil.drag_started() {
            ev = MenuEvent::Dismissed;
        }
        let r = Rect::from_min_size(at + vec2(0.0, 4.0 * (1.0 - k)), vec2(w, h));
        let p = ui.painter();
        p.rect_filled(r.translate(vec2(0.0, 10.0)).expand(4.0), 10.0, Color32::BLACK.gamma_multiply(0.35));
        p.rect_filled(r.translate(vec2(0.0, 2.0)), 6.0, Color32::BLACK.gamma_multiply(0.5));
        p.rect_filled(r, 6.0, T.inset);
        p.rect_stroke(r, 6.0, paint::hair(p, T.lo()), StrokeKind::Inside);
        let mut y = r.top() + 4.0;
        for (i, item) in items.iter().enumerate() {
            if item.sep_before {
                paint::hline(p, r.left() + 8.0, r.right() - 8.0, y + 4.5, Color32::WHITE.gamma_multiply(0.10));
                y += 9.0;
            }
            let row = Rect::from_min_size(pos2(r.left() + 4.0, y), vec2(w - 8.0, MENU_ROW_H));
            y += MENU_ROW_H;
            let resp = ui.interact(row, id.with(("row", i)), if item.enabled { Sense::click() } else { Sense::hover() });
            let hot = item.enabled && resp.hovered();
            if hot {
                let c = if item.danger { SRC_E } else { T.accent };
                p.rect_filled(row, 3.0, c.gamma_multiply(0.16));
                ctx.set_cursor_icon(CursorIcon::PointingHand);
            }
            let base = if item.danger { SRC_E } else { T.text };
            let col = if item.enabled { base } else { T.muted.gamma_multiply(0.6) };
            if item.checked {
                p.circle_filled(pos2(row.left() + 10.0, row.center().y), 2.5, T.accent);
            }
            p.text(pos2(row.left() + 20.0, row.center().y), Align2::LEFT_CENTER, item.label, f.clone(), col);
            if resp.clicked() {
                ev = MenuEvent::Picked(i);
            }
        }
    });
    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        ev = MenuEvent::Dismissed;
    }
    ev
}

// ---- modal -------------------------------------------------------------------------------------

/// Padding inside a modal box.
pub const MODAL_PAD: f32 = 20.0;
/// Height of the modal's title row.
pub const MODAL_TITLE_H: f32 = 36.0;

/// A modal box centred in `region`: the region dims to black at up to 55 % and swallows every
/// pointer event, the box (`size`) rises 6 px while it fades in (`k`, 0..=1), and `content` gets
/// the box's inner rect (below the engraved `title`) to lay out in absolute coordinates.
/// Returns true when the dimmed backdrop outside the box was clicked.
pub fn modal(ctx: &Context, id: Id, region: Rect, size: Vec2, title: &str, k: f32, content: impl FnOnce(&mut Ui, Rect)) -> bool {
    let mut outside = false;
    egui::Area::new(id).order(Order::Foreground).fixed_pos(region.min).interactable(true).show(ctx, |ui| {
        ui.multiply_opacity(k);
        let veil = ui.interact(region, id.with("veil"), Sense::click_and_drag());
        let r = Rect::from_center_size(region.center() + vec2(0.0, 6.0 * (1.0 - k)), size);
        outside = veil.clicked() && !r.contains(veil.interact_pointer_pos().unwrap_or(r.center()));
        let p = ui.painter().clone();
        p.rect_filled(region, 0.0, Color32::BLACK.gamma_multiply(0.55));
        p.rect_filled(r.translate(vec2(0.0, 14.0)).expand(6.0), 14.0, Color32::BLACK.gamma_multiply(0.30));
        p.rect_filled(r.translate(vec2(0.0, 3.0)), GROUP_R + 2.0, Color32::BLACK.gamma_multiply(0.5));
        paint::highlight_below(&p, r, GROUP_R + 2.0);
        paint::rounded_vgradient(&p, r, GROUP_R + 2.0, T.panel_top, T.panel_bottom, true);
        p.rect_stroke(r, GROUP_R + 2.0, paint::hair(&p, T.lo()), StrokeKind::Inside);
        // Engraved title, as on a group.
        let up = title.to_uppercase();
        let at = pos2(r.left() + MODAL_PAD, r.top() + MODAL_TITLE_H / 2.0 + 2.0);
        let f = font::strong(11.0);
        paint::tracked(&p, at + vec2(0.0, -1.0), Align2::LEFT_CENTER, &up, f.clone(), Color32::BLACK.gamma_multiply(0.4), 1.4);
        paint::tracked(&p, at + vec2(0.0, 1.0), Align2::LEFT_CENTER, &up, f.clone(), T.hi(), 1.4);
        let tr = paint::tracked(&p, at, Align2::LEFT_CENTER, &up, f, T.title, 1.4);
        paint::hgradient(&p, Rect::from_min_max(pos2(tr.right() + 8.0, at.y), pos2(r.right() - MODAL_PAD, at.y + 1.0)), Color32::WHITE.gamma_multiply(0.14), Color32::TRANSPARENT);
        let inner = Rect::from_min_max(pos2(r.left() + MODAL_PAD, r.top() + MODAL_TITLE_H + 6.0), pos2(r.right() - MODAL_PAD, r.bottom() - MODAL_PAD));
        content(ui, inner);
    });
    outside
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(ctx: &Context, input: egui::RawInput, f: impl FnMut(&Context)) {
        let _ = ctx.run(input, f);
    }

    fn input() -> egui::RawInput {
        egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0))), ..Default::default() }
    }

    #[test]
    fn heart_fits_its_box_and_is_one_closed_loop() {
        let pts = heart_points(pos2(100.0, 100.0), 12.0);
        let r = Rect::from_points(&pts);
        assert!((r.width() - 12.0).abs() < 0.6, "{r:?}");
        assert!(r.height() > 9.0 && r.height() < 13.0, "{r:?}");
        assert!((r.center().x - 100.0).abs() < 0.2 && (r.center().y - 100.0).abs() < 0.8, "centred: {r:?}");
        assert_eq!(pts.len(), 28);
    }

    #[test]
    fn scrollbar_maps_drag_and_hides_when_content_fits() {
        let ctx = Context::default();
        crate::theme::apply(&ctx);
        let track = Rect::from_min_size(pos2(380.0, 10.0), vec2(SCROLLBAR_W, 200.0));
        let mut off = 0.0;
        // Fits: untouched.
        run(&ctx, input(), |c| {
            egui::CentralPanel::default().show(c, |ui| assert!(!scrollbar(ui, track, Id::new("sb"), 100.0, 200.0, &mut off)));
        });
        assert_eq!(off, 0.0);
        // Overflows: a click below the thumb pages down, clamped to the end.
        let mut inp = input();
        inp.events.push(egui::Event::PointerMoved(pos2(384.0, 200.0)));
        run(&ctx, inp.clone(), |c| {
            egui::CentralPanel::default().show(c, |ui| {
                scrollbar(ui, track, Id::new("sb"), 1000.0, 200.0, &mut off);
            });
        });
        let mut down = input();
        down.events.push(egui::Event::PointerButton { pos: pos2(384.0, 200.0), button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
        run(&ctx, down, |c| {
            egui::CentralPanel::default().show(c, |ui| {
                scrollbar(ui, track, Id::new("sb"), 1000.0, 200.0, &mut off);
            });
        });
        let mut up = input();
        up.events.push(egui::Event::PointerButton { pos: pos2(384.0, 200.0), button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
        run(&ctx, up, |c| {
            egui::CentralPanel::default().show(c, |ui| {
                scrollbar(ui, track, Id::new("sb"), 1000.0, 200.0, &mut off);
            });
        });
        assert!((off - 180.0).abs() < 1.0, "paged down by 90 % of the view: {off}");
    }

    #[test]
    fn button_widths_grow_with_the_label() {
        let ctx = Context::default();
        crate::theme::apply(&ctx);
        let mut w = (0.0, 0.0);
        run(&ctx, input(), |c| {
            egui::CentralPanel::default().show(c, |ui| {
                w = (button_width(ui.painter(), "Ok"), button_width(ui.painter(), "Save As\u{2026}"));
            });
        });
        assert!(w.0 >= 56.0 && w.1 > w.0, "{w:?}");
    }
}
