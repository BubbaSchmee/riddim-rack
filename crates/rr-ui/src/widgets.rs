//! Instrument widgets. Each takes plain values and reports changes through its return value;
//! the plugin editor adapts that to its param setter (begin / set / end around a drag).

use crate::paint;
use crate::theme::{font, metrics::*, T};
use egui::{
    pos2, vec2, Align, Align2, Color32, CursorIcon, Layout, Rect, Sense, Shape, Stroke, StrokeKind, Ui, UiBuilder, Vec2,
};

// ---------------------------------------------------------------------------------------------
// Group (well)
// ---------------------------------------------------------------------------------------------

/// Engraved title + body in a recessed well. Width fills the available width, height fits the
/// body. See [`group_ext`] for a fixed size and header controls.
pub fn group<R>(ui: &mut Ui, title: &str, body: impl FnOnce(&mut Ui) -> R) -> R {
    group_ext(ui, Vec2::ZERO, title, |_| {}, body)
}

/// Group with an explicit `size` (a zero axis means "fill available width" / "fit content") and
/// `header` controls laid out right-to-left at the right end of the 24 px title row. The title's
/// hairline fills the space between the title and the header controls.
pub fn group_ext<R>(ui: &mut Ui, size: Vec2, title: &str, header: impl FnOnce(&mut Ui), body: impl FnOnce(&mut Ui) -> R) -> R {
    let w = if size.x > 0.0 { size.x } else { ui.available_width() };
    let origin = ui.cursor().min;
    let p = ui.painter().clone();
    let hi_idx = p.add(Shape::Noop);
    let bg_idx = p.add(Shape::Noop);

    let x0 = origin.x + GROUP_PAD_X;
    let inner_w = w - 2.0 * GROUP_PAD_X;
    let header_rect = Rect::from_min_size(pos2(x0, origin.y + GROUP_PAD_TOP), vec2(inner_w, GROUP_HEADER_H));

    // Engraved title: shadow above, highlight below, then the title colour.
    let title_up = title.to_uppercase();
    let f = font::strong(11.0);
    let at = header_rect.left_center();
    paint::tracked(&p, at + vec2(0.0, -1.0), Align2::LEFT_CENTER, &title_up, f.clone(), Color32::BLACK.gamma_multiply(0.4), 1.4);
    paint::tracked(&p, at + vec2(0.0, 1.0), Align2::LEFT_CENTER, &title_up, f.clone(), T.hi(), 1.4);
    let title_rect = paint::tracked(&p, at, Align2::LEFT_CENTER, &title_up, f, T.title, 1.4);

    let mut hui = ui.new_child(UiBuilder::new().max_rect(header_rect).layout(Layout::right_to_left(Align::Center)));
    hui.spacing_mut().item_spacing.x = 8.0;
    header(&mut hui);
    let used = hui.min_rect();
    let right_edge = if used.is_finite() && used.width() > 0.5 { used.left() } else { header_rect.right() };
    let (lx0, lx1) = (title_rect.right() + 8.0, right_edge - 8.0);
    if lx1 > lx0 {
        paint::hgradient(&p, Rect::from_min_max(pos2(lx0, at.y), pos2(lx1, at.y + 1.0)), Color32::WHITE.gamma_multiply(0.14), Color32::TRANSPARENT);
    }

    let body_top = header_rect.bottom() + GROUP_HEADER_GAP;
    let mut bui = ui.new_child(
        UiBuilder::new()
            .max_rect(Rect::from_min_size(pos2(x0, body_top), vec2(inner_w, 4000.0)))
            .layout(Layout::top_down(Align::Min)),
    );
    bui.spacing_mut().item_spacing = vec2(8.0, 6.0);
    let out = body(&mut bui);
    let h = (bui.min_rect().bottom() + GROUP_PAD_BOTTOM - origin.y).max(size.y);
    let rect = Rect::from_min_size(origin, vec2(w, h));

    p.set(
        hi_idx,
        Shape::rect_filled(rect.translate(vec2(0.0, 1.0)), GROUP_R, T.hi()),
    );
    p.set(
        bg_idx,
        Shape::Vec(vec![
            Shape::rect_filled(rect, GROUP_R, T.well),
            Shape::line_segment([rect.left_top() + vec2(GROUP_R, 0.5), rect.right_top() + vec2(-GROUP_R, 0.5)], paint::st(1.0, T.lo())),
        ]),
    );
    ui.allocate_rect(rect, Sense::hover());
    out
}

/// Horizontal run of knob cells with no spacing between them (cell widths do the spacing).
pub fn knob_row<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        add(ui)
    })
    .inner
}

/// Horizontal run of 24 px controls (toggles, combos) with 8 px between them.
pub fn strip<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.set_min_height(CONTROL_H);
        add(ui)
    })
    .inner
}

/// Small muted caption (e.g. "Step 6 · E" in a group header).
pub fn caption(ui: &mut Ui, text: &str) {
    ui.label(egui::RichText::new(text).font(font::label(12.0)).color(T.muted));
}

// ---------------------------------------------------------------------------------------------
// Knob
// ---------------------------------------------------------------------------------------------

/// Pointer travel for a full-range sweep, in px.
pub const DRAG_PX: f32 = 200.0;
/// Shift multiplier for fine adjustment.
pub const FINE: f32 = 0.1;
const ARC_START: f32 = 135.0;
const ARC_SWEEP: f32 = 270.0;

/// New normalised value after the pointer moved `dy_up_px` px upward (negative = down).
/// `fine` (Shift held) scales the travel by [`FINE`]. Applied per frame, so toggling Shift
/// mid-drag never jumps the value.
pub fn drag_value(value01: f32, dy_up_px: f32, fine: bool) -> f32 {
    let k = if fine { FINE } else { 1.0 };
    (value01 + dy_up_px / DRAG_PX * k).clamp(0.0, 1.0)
}

/// What a knob (or slider) did this frame. Map `drag_started` / `drag_ended` to the host's
/// begin / end gesture and `changed` to the set call. A double-click reports `reset` together
/// with `changed = Some(default)` and no drag flags: wrap it in begin / set / end.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct KnobResponse {
    pub changed: Option<f32>,
    pub drag_started: bool,
    pub drag_ended: bool,
    pub reset: bool,
}

/// Knob geometry/behaviour options.
#[derive(Clone, Copy, Debug)]
pub struct KnobStyle {
    /// Knob diameter (40 GTS, 44 VSS).
    pub diameter: f32,
    /// Cell width (56 normal, 44 to 52 in dense rows).
    pub cell_w: f32,
    /// Value arc runs from 12 o'clock to the value.
    pub bipolar: bool,
}

impl Default for KnobStyle {
    fn default() -> Self {
        Self { diameter: KNOB_D, cell_w: KNOB_CELL_W, bipolar: false }
    }
}

impl KnobStyle {
    pub fn bipolar(mut self) -> Self {
        self.bipolar = true;
        self
    }
    pub fn cell(mut self, w: f32) -> Self {
        self.cell_w = w;
        self
    }
    pub fn size(mut self, d: f32) -> Self {
        self.diameter = d;
        self
    }
}

/// Standard 40 px unipolar knob in a 56 px cell: label, arc + cap, readout.
pub fn knob(ui: &mut Ui, label: &str, value01: f32, default01: f32, readout: &str) -> KnobResponse {
    knob_styled(ui, label, value01, default01, readout, KnobStyle::default())
}

pub fn knob_styled(ui: &mut Ui, label: &str, value01: f32, default01: f32, readout: &str, style: KnobStyle) -> KnobResponse {
    let d = style.diameter;
    let (rect, resp) = ui.allocate_exact_size(vec2(style.cell_w, knob_cell_h(d)), Sense::click_and_drag());
    let hot = resp.hovered() || resp.dragged();
    if hot {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
    }

    let out = knob_interact(ui, &resp, value01, default01);

    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        paint::caps(p, pos2(rect.center().x, rect.top() + KNOB_LABEL_H / 2.0), Align2::CENTER_CENTER, label, 11.0, T.muted);
        let c = pos2(rect.center().x, rect.top() + KNOB_LABEL_H + d / 2.0);
        draw_knob(p, c, d, value01, style.bipolar, hot);
        let col = if hot { T.accent } else { T.text };
        p.text(pos2(rect.center().x, rect.bottom() - KNOB_READOUT_H / 2.0), Align2::CENTER_CENTER, readout, font::mono(12.0), col);
    }
    out
}

/// Vertical-drag / double-click-reset gesture shared by every knob.
fn knob_interact(ui: &Ui, resp: &egui::Response, value01: f32, default01: f32) -> KnobResponse {
    let mut out = KnobResponse { drag_started: resp.drag_started(), drag_ended: resp.drag_stopped(), ..Default::default() };
    if resp.double_clicked() {
        out.reset = true;
        out.changed = Some(default01);
    } else if resp.dragged() {
        let fine = ui.input(|i| i.modifiers.shift);
        let next = drag_value(value01, -resp.drag_delta().y, fine);
        if next != value01 {
            out.changed = Some(next);
        }
    }
    out
}

/// Arc, glow, cap and indicator for a knob of diameter `d` centred at `c`.
pub fn draw_knob(p: &egui::Painter, c: egui::Pos2, d: f32, value01: f32, bipolar: bool, hot: bool) {
    let r = d / 2.0 - 3.0;
    paint::arc_stroke(p, c, r, ARC_START, ARC_START + ARC_SWEEP, 3.0, T.track, true);
    let a_val = ARC_START + ARC_SWEEP * value01.clamp(0.0, 1.0);
    let from = if bipolar { ARC_START + ARC_SWEEP / 2.0 } else { ARC_START };
    if (a_val - from).abs() > 1.0 {
        let (a0, a1) = (from.min(a_val), from.max(a_val));
        if hot {
            paint::arc_stroke(p, c, r, a0, a1, 11.0, T.a(0.06), false);
        }
        paint::arc_stroke(p, c, r, a0, a1, 7.0, T.a(0.18), false);
        paint::arc_stroke(p, c, r, a0, a1, 3.0, T.accent, true);
    }
    let rc = r - 6.0;
    p.circle_filled(c, rc, T.cap1);
    p.circle_stroke(c, rc, paint::st(1.0, Color32::BLACK));
    p.circle_filled(c - vec2(0.0, 1.0), rc - 2.0, T.cap2);
    let a = a_val.to_radians();
    let dir = vec2(a.cos(), a.sin());
    let (i0, i1) = (c + dir * rc * 0.35, c + dir * rc * 0.9);
    p.line_segment([i0, i1], paint::st(2.0, T.text));
    p.circle_filled(i0, 1.0, T.text);
    p.circle_filled(i1, 1.0, T.text);
}

// ---------------------------------------------------------------------------------------------
// LED toggle
// ---------------------------------------------------------------------------------------------

/// 24 px toggle: 8 px LED plus an optional caps label. Returns the new state on click.
pub fn led_toggle(ui: &mut Ui, label: &str, on: bool) -> Option<bool> {
    let up = label.to_uppercase();
    let lw = if up.is_empty() { 0.0 } else { 8.0 + paint::text_width(ui.painter(), &font::label(11.0), &up, 0.5) };
    let (rect, resp) = ui.allocate_exact_size(vec2(8.0 + 8.0 + lw + 8.0, CONTROL_H), Sense::click());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        paint::inset_box(p, rect, resp.hovered());
        paint::led(p, pos2(rect.left() + 12.0, rect.center().y), on);
        if !up.is_empty() {
            let col = if on { T.text } else { T.muted };
            paint::tracked(p, pos2(rect.left() + 24.0, rect.center().y), Align2::LEFT_CENTER, &up, font::label(11.0), col, 0.5);
        }
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp.clicked().then_some(!on)
}

// ---------------------------------------------------------------------------------------------
// Combo
// ---------------------------------------------------------------------------------------------

const COMBO_FONT: f32 = 13.0;

/// Inset dropdown. With a non-empty `label` it is a knob-row cell (label above, control
/// centred on the knob band) so combos line up with knobs; with an empty label it is a bare
/// 24 px control for strips and group headers. Returns the picked index when it changes.
pub fn combo(ui: &mut Ui, label: &str, selected: usize, items: &[&str]) -> Option<usize> {
    combo_impl(ui, label, selected, items, true)
}

/// Like [`combo`] but compact: the labelled control sits directly under its label (37 px
/// cell) instead of being centred on the 40 px knob band. For stacks of controls.
pub fn combo_stacked(ui: &mut Ui, label: &str, selected: usize, items: &[&str]) -> Option<usize> {
    combo_impl(ui, label, selected, items, false)
}

fn combo_impl(ui: &mut Ui, label: &str, selected: usize, items: &[&str], knob_band: bool) -> Option<usize> {
    let f = font::label(COMBO_FONT);
    let text_w = items.iter().map(|s| paint::text_width(ui.painter(), &f, s, 0.0)).fold(0.0, f32::max);
    let ctrl_w = (text_w + 8.0 + 24.0).max(56.0).ceil();
    let labelled = !label.is_empty();
    let (cell, ctrl) = if labelled {
        let cell_h = if knob_band { knob_cell_h(KNOB_D) } else { KNOB_LABEL_H + CONTROL_H };
        let band = if knob_band { KNOB_D } else { CONTROL_H };
        let lw = paint::text_width(ui.painter(), &font::label(11.0), &label.to_uppercase(), 0.5);
        let (cell, _) = ui.allocate_exact_size(vec2(ctrl_w.max(lw).max(KNOB_CELL_W - 8.0) + 8.0, cell_h), Sense::hover());
        let ctrl = Rect::from_min_size(
            pos2(cell.center().x - ctrl_w / 2.0, cell.top() + KNOB_LABEL_H + (band - CONTROL_H) / 2.0),
            vec2(ctrl_w, CONTROL_H),
        );
        (cell, ctrl)
    } else {
        let (r, _) = ui.allocate_exact_size(vec2(ctrl_w, CONTROL_H), Sense::hover());
        (r, r)
    };
    let resp = ui.interact(ctrl, ui.id().with(("combo", label, ctrl.min.x as i32, ctrl.min.y as i32)), Sense::click());
    let popup_id = resp.id.with("popup");
    if resp.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup_id));
    }
    let open = ui.memory(|m| m.is_popup_open(popup_id));

    if ui.is_rect_visible(cell) {
        let p = ui.painter();
        if labelled {
            paint::caps(p, pos2(cell.center().x, cell.top() + KNOB_LABEL_H / 2.0), Align2::CENTER_CENTER, label, 11.0, T.muted);
        }
        paint::inset_box(p, ctrl, resp.hovered() || open);
        let cur = items.get(selected).copied().unwrap_or("");
        p.text(pos2(ctrl.left() + 8.0, ctrl.center().y), Align2::LEFT_CENTER, cur, f.clone(), T.text);
        let (cx, cy) = (ctrl.right() - 11.5, ctrl.center().y - 1.0);
        p.add(Shape::line(vec![pos2(cx - 3.5, cy - 1.5), pos2(cx, cy + 2.0), pos2(cx + 3.5, cy - 1.5)], paint::st(1.5, T.muted)));
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    let mut picked = None;
    egui::popup::popup_below_widget(ui, popup_id, &resp, egui::popup::PopupCloseBehavior::CloseOnClick, |ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        ui.set_min_width(ctrl_w - 8.0);
        for (i, item) in items.iter().enumerate() {
            let (r, rr) = ui.allocate_exact_size(vec2(ui.available_width(), CONTROL_H), Sense::click());
            if rr.hovered() {
                ui.painter().rect_filled(r, 3.0, T.a(0.16));
            }
            let col = if i == selected { T.accent } else { T.text };
            ui.painter().text(pos2(r.left() + 8.0, r.center().y), Align2::LEFT_CENTER, item, f.clone(), col);
            if rr.clicked() && i != selected {
                picked = Some(i);
            }
        }
    });
    picked
}

// ---------------------------------------------------------------------------------------------
// Horizontal slider
// ---------------------------------------------------------------------------------------------

const SLIDER_H: f32 = 38.0;
const THUMB: Vec2 = vec2(14.0, 16.0);

/// Value for a pointer x on a track spanning `[x0, x1]`.
pub fn slider_value(x: f32, x0: f32, x1: f32) -> f32 {
    ((x - x0) / (x1 - x0)).clamp(0.0, 1.0)
}

/// Labelled 168 px slider with a right-aligned readout. Absolute positioning (click or drag).
pub fn hslider(ui: &mut Ui, label: &str, value01: f32, readout: &str) -> Option<f32> {
    hslider_resp(ui, SLIDER_W, label, value01, readout).changed
}

/// Same as [`hslider`] with explicit width and the full gesture response.
pub fn hslider_resp(ui: &mut Ui, width: f32, label: &str, value01: f32, readout: &str) -> KnobResponse {
    let (rect, resp) = ui.allocate_exact_size(vec2(width, SLIDER_H), Sense::click_and_drag());
    let (x0, x1) = (rect.left() + THUMB.x / 2.0, rect.right() - THUMB.x / 2.0);
    let mut out = KnobResponse { drag_started: resp.drag_started(), drag_ended: resp.drag_stopped(), ..Default::default() };
    if resp.is_pointer_button_down_on() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let v = slider_value(pos.x, x0, x1);
            if (v - value01).abs() > f32::EPSILON {
                out.changed = Some(v);
            }
        }
    }
    let hot = resp.hovered() || resp.dragged();
    if hot {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        paint::caps(p, pos2(rect.left(), rect.top() + KNOB_LABEL_H / 2.0), Align2::LEFT_CENTER, label, 11.0, T.muted);
        p.text(pos2(rect.right(), rect.top() + KNOB_LABEL_H / 2.0), Align2::RIGHT_CENTER, readout, font::mono(12.0), if hot { T.accent } else { T.text });
        let cy = rect.top() + KNOB_LABEL_H + 16.0;
        let track = Rect::from_min_max(pos2(x0, cy - 2.0), pos2(x1, cy + 2.0));
        p.rect_filled(track, 2.0, T.track);
        let tx = x0 + (x1 - x0) * value01.clamp(0.0, 1.0);
        if tx > x0 + 0.5 {
            let fill = Rect::from_min_max(track.min, pos2(tx, track.bottom()));
            p.rect_filled(fill.expand(2.0), 4.0, T.a(0.10));
            p.rect_filled(fill, 2.0, T.accent);
        }
        let thumb = Rect::from_center_size(pos2(tx, cy), THUMB);
        p.rect_filled(thumb.expand(1.0), 4.0, Color32::BLACK);
        p.rect_filled(thumb, 3.0, T.cap1);
        p.rect_filled(Rect::from_min_size(thumb.min, vec2(THUMB.x, THUMB.y / 2.0)), egui::CornerRadius { nw: 3, ne: 3, sw: 0, se: 0 }, T.cap2);
        p.rect_filled(Rect::from_center_size(thumb.center(), vec2(2.0, 10.0)), 1.0, T.text);
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Step grid
// ---------------------------------------------------------------------------------------------

/// Step grid geometry (px).
pub mod grid {
    pub const LABEL_W: f32 = 36.0;
    pub const GAP: f32 = 4.0;
    pub const CELL: f32 = 32.0;
    pub const HEADER_H: f32 = 16.0;
    pub const PAD: f32 = 16.0;
    pub const ROW_GAP: f32 = 12.0;

    /// Grid block size (without padding) for `cols` x `rows`.
    pub fn block(cols: usize, rows: usize) -> (f32, f32) {
        let w = LABEL_W + GAP + cols as f32 * CELL + cols.saturating_sub(1) as f32 * GAP;
        let h = HEADER_H + GAP + rows as f32 * CELL + rows.saturating_sub(1) as f32 * ROW_GAP;
        (w, h)
    }

    /// Cell under `local`, a point relative to the top-left of the block (label column and
    /// header row included). Gaps belong to the nearest cell so there are no dead strips.
    pub fn hit(local: egui::Vec2, cols: usize, rows: usize) -> Option<(usize, usize)> {
        let x = local.x - (LABEL_W + GAP) + GAP / 2.0;
        let y = local.y - (HEADER_H + GAP) + ROW_GAP / 2.0;
        if x < 0.0 || y < 0.0 {
            return None;
        }
        let (col, row) = ((x / (CELL + GAP)) as usize, (y / (CELL + ROW_GAP)) as usize);
        (col < cols && row < rows).then_some((col, row))
    }
}

/// VSS-style step grid. `rows` are the row names (top to bottom), `cells[col]` is the selected
/// row of each column, `active_cols` is the sequence length (later columns are dimmed but
/// still editable), `current_col` the playing step. A row named "Hold" gets the quieter hold
/// look. Returns `(col, row)` when the pointer sets a different row on a column (click or drag).
pub fn step_grid(ui: &mut Ui, rows: &[&str], cols: usize, active_cols: usize, current_col: Option<usize>, cells: &[u8]) -> Option<(usize, usize)> {
    let hold = rows.iter().position(|r| r.eq_ignore_ascii_case("hold"));
    step_grid_ex(ui, rows, cols, active_cols, current_col, cells, hold, None)
}

/// [`step_grid`] with an explicit hold row and an optional fixed `height` (the grid is centred
/// vertically in it; the default is the grid plus 16 px padding).
pub fn step_grid_ex(
    ui: &mut Ui,
    rows: &[&str],
    cols: usize,
    active_cols: usize,
    current_col: Option<usize>,
    cells: &[u8],
    hold_row: Option<usize>,
    height: Option<f32>,
) -> Option<(usize, usize)> {
    let (bw, bh) = grid::block(cols, rows.len());
    let size = vec2(ui.available_width().max(bw + 2.0 * grid::PAD), height.unwrap_or(0.0).max(bh + 2.0 * grid::PAD));
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let origin = pos2(rect.center().x - bw / 2.0, rect.center().y - bh / 2.0);

    let mut picked = None;
    if resp.is_pointer_button_down_on() {
        if let Some(pos) = resp.interact_pointer_pos() {
            if let Some((c, r)) = grid::hit(pos - origin, cols, rows.len()) {
                if cells.get(c).is_none_or(|&v| v as usize != r) {
                    picked = Some((c, r));
                }
            }
        }
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    if !ui.is_rect_visible(rect) {
        return picked;
    }

    let p = ui.painter();
    paint::display_bg(p, rect);
    let cell_rect = |c: usize, r: usize| {
        Rect::from_min_size(
            pos2(origin.x + grid::LABEL_W + grid::GAP + c as f32 * (grid::CELL + grid::GAP), origin.y + grid::HEADER_H + grid::GAP + r as f32 * (grid::CELL + grid::ROW_GAP)),
            vec2(grid::CELL, grid::CELL),
        )
    };
    let dim = |c: usize| if c >= active_cols { 0.35 } else { 1.0 };

    for c in 0..cols {
        let col = if Some(c) == current_col && c < active_cols { T.accent } else { T.disp_caption };
        let x = cell_rect(c, 0).center().x;
        p.text(pos2(x, origin.y + grid::HEADER_H / 2.0), Align2::CENTER_CENTER, format!("{}", c + 1), font::label(11.0), col.gamma_multiply(dim(c)));
    }
    for (r, name) in rows.iter().enumerate() {
        let y = cell_rect(0, r).center().y;
        paint::tracked(p, pos2(origin.x, y), Align2::LEFT_CENTER, &name.to_uppercase(), font::strong(12.0), T.muted, 0.5);
    }
    // Pass 1: empty cells and the current-column wash.
    for c in 0..cols {
        let is_cur = Some(c) == current_col && c < active_cols;
        for r in 0..rows.len() {
            if cells.get(c).is_some_and(|&v| v as usize == r) {
                continue;
            }
            let rc = cell_rect(c, r);
            let k = dim(c);
            let (fill, stroke) = if is_cur { (T.a(0.07), T.a(0.18)) } else { (T.cell_off, T.cell_off_stroke) };
            p.rect_filled(rc, 4.0, fill.gamma_multiply(k));
            p.rect_stroke(rc, 4.0, paint::st(1.0, stroke.gamma_multiply(k)), StrokeKind::Inside);
        }
    }
    // Pass 2: selected cells (glow spills over neighbours), the playing one last.
    let mut selected: Vec<(usize, usize)> = (0..cols).filter_map(|c| cells.get(c).map(|&r| (c, r as usize))).filter(|&(_, r)| r < rows.len()).collect();
    selected.sort_by_key(|&(c, _)| Some(c) == current_col && c < active_cols);
    for (c, r) in selected {
        let rc = cell_rect(c, r);
        let k = dim(c);
        let is_cur = Some(c) == current_col && c < active_cols;
        let is_hold = hold_row == Some(r);
        if is_cur {
            p.rect_filled(rc.expand(6.0), 8.0, T.a(0.22 * k));
            p.rect_filled(rc.expand(2.0), 6.0, T.a(0.55 * k));
            p.rect_filled(rc, 4.0, T.accent.gamma_multiply(k));
        } else if is_hold {
            paint::vgradient(p, rc.shrink2(vec2(0.0, 2.0)), T.a(0.16 * k), T.a(0.08 * k));
            p.rect_filled(rc, 4.0, T.a(0.08 * k));
            p.rect_stroke(rc, 4.0, paint::st(1.0, T.a(0.35 * k)), StrokeKind::Inside);
        } else {
            p.rect_stroke(rc.expand(2.0), 6.0, paint::st(1.0, T.a(0.18 * k)), StrokeKind::Inside);
            p.rect_filled(rc, 4.0, T.a(0.20 * k));
            p.rect_stroke(rc, 4.0, paint::st(1.0, T.a(0.55 * k)), StrokeKind::Inside);
        }
    }
    picked
}

// ---------------------------------------------------------------------------------------------
// Curve preview
// ---------------------------------------------------------------------------------------------

/// Small boxed transfer-curve preview (e.g. the shaper): `curve` holds y in 0..1 for evenly
/// spaced x. Frame, identity diagonal, glow line.
pub fn curve_preview(ui: &mut Ui, size: Vec2, curve: &[f32]) {
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    if !ui.is_rect_visible(rect) || curve.len() < 2 {
        return;
    }
    let p = ui.painter();
    p.rect_stroke(rect.shrink(0.5), 4.0, paint::st(1.0, Color32::WHITE.gamma_multiply(0.08)), StrokeKind::Inside);
    let inner = rect.shrink(4.0);
    p.line_segment([inner.left_bottom(), inner.right_top()], paint::st(1.0, Color32::WHITE.gamma_multiply(0.10)));
    let n = curve.len() - 1;
    let pts: Vec<_> = curve.iter().enumerate().map(|(i, &y)| pos2(inner.left() + inner.width() * i as f32 / n as f32, inner.bottom() - inner.height() * y.clamp(0.0, 1.0))).collect();
    p.add(Shape::line(pts.clone(), paint::st(5.0, T.a(0.25))));
    p.add(Shape::line(pts, paint::st(1.5, T.accent)));
}

// ---------------------------------------------------------------------------------------------
// Waveform display
// ---------------------------------------------------------------------------------------------

const GRAIN_TICKS: usize = 32;

fn wave_points(rect: Rect, samples: &[f32]) -> Vec<egui::Pos2> {
    let (x0, w, mid, amp) = (rect.left() + 16.0, rect.width() - 32.0, rect.top() + 54.0, 34.0);
    let n = samples.len().max(2) - 1;
    samples.iter().enumerate().map(|(i, &s)| pos2(x0 + w * i as f32 / n as f32, mid - s.clamp(-1.0, 1.0) * amp)).collect()
}

/// 124 px display filling the available width: graticule, glowing waveform, 32 grain ticks and
/// an index marker. `samples` are -1..1; `marker01` is the grain index position (0..1).
pub fn waveform_display(ui: &mut Ui, samples: &[f32], marker01: f32, title: &str, readout: &str) {
    waveform_display_ex(ui, samples, &[], marker01, title, readout)
}

/// [`waveform_display`] plus faint neighbour-grain traces drawn behind the main waveform.
pub fn waveform_display_ex(ui: &mut Ui, samples: &[f32], ghosts: &[&[f32]], marker01: f32, title: &str, readout: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), DISPLAY_H), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let p = ui.painter();
    paint::display_bg(p, rect);
    let (x0, w) = (rect.left() + 16.0, rect.width() - 32.0);
    let mid = rect.top() + 54.0;
    for i in 0..=8 {
        let x = x0 + w * i as f32 / 8.0;
        p.line_segment([pos2(x, rect.top() + 22.0), pos2(x, rect.top() + 92.0)], paint::st(1.0, Color32::WHITE.gamma_multiply(0.05)));
    }
    p.line_segment([pos2(x0, mid), pos2(x0 + w, mid)], paint::st(1.0, Color32::WHITE.gamma_multiply(0.10)));
    for g in ghosts {
        p.add(Shape::line(wave_points(rect, g), paint::st(1.0, T.a(0.14))));
    }
    let pts = wave_points(rect, samples);
    for (width, alpha) in [(12.0, 0.07), (6.0, 0.16)] {
        p.add(Shape::line(pts.clone(), paint::st(width, T.a(alpha))));
    }
    p.add(Shape::line(pts, paint::st(2.0, T.accent)));

    let m = marker01.clamp(0.0, 1.0);
    let active = ((m * GRAIN_TICKS as f32) as usize).min(GRAIN_TICKS - 1);
    for i in 0..GRAIN_TICKS {
        let x = x0 + (i as f32 + 0.5) * w / GRAIN_TICKS as f32;
        let (y, h, col) = if i == active { (102.0, 12.0, T.accent) } else { (106.0, 8.0, T.tick_off) };
        p.rect_filled(Rect::from_min_size(pos2(x - 3.0, rect.top() + y), vec2(6.0, h)), 1.5, col);
    }
    let mx = x0 + m * w;
    p.add(Shape::convex_polygon(
        vec![pos2(mx - 5.0, rect.top() + 97.0), pos2(mx + 5.0, rect.top() + 97.0), pos2(mx, rect.top() + 102.0)],
        T.accent,
        Stroke::NONE,
    ));
    paint::tracked(p, pos2(x0, rect.top() + 14.0), Align2::LEFT_CENTER, &title.to_uppercase(), font::label(11.0), T.disp_caption, 0.5);
    p.text(pos2(x0 + w, rect.top() + 14.0), Align2::RIGHT_CENTER, readout.to_uppercase(), font::label(12.0), T.accent);
}

// ---------------------------------------------------------------------------------------------
// PATCH strip (RR-BUS)
// ---------------------------------------------------------------------------------------------

/// Height of the PATCH group: chrome + three 24 px rows with 4 px gaps. Equals the 124 px
/// nameplate / header row.
pub const PATCH_H: f32 = GROUP_CHROME_H + 3.0 * CONTROL_H + 2.0 * PATCH_ROW_GAP;
const PATCH_ROW_GAP: f32 = 4.0;
const PATCH_KNOB_D: f32 = 24.0;
const PATCH_LED_W: f32 = 12.0;
const PATCH_GAP: f32 = 6.0;
const PATCH_FONT: f32 = 12.0;
const PATCH_LABEL_W: f32 = 104.0;
const PATCH_LABEL_CHARS: usize = 16;

/// One PATCH row as the caller sees it. `sources` are display names ("GTS 1 · Mod A"); the
/// combo adds a leading "None" entry itself. `lost_name` is the persisted name of a bound
/// source that is no longer live (shown dimmed while `source_idx` is `None`).
pub struct PatchRow<'a> {
    pub sources: &'a [String],
    pub source_idx: Option<usize>,
    pub lost_name: Option<&'a str>,
    pub targets: &'a [&'a str],
    pub target_idx: usize,
    /// Bipolar amount, centre 0.5.
    pub amount01: f32,
    /// |incoming value|, 0..1, drives the LED brightness.
    pub activity01: f32,
}

/// What one row did this frame. `source_changed` is `Some(None)` for "None" (unbind).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PatchRowResponse {
    pub source_changed: Option<Option<usize>>,
    pub target_changed: Option<usize>,
    pub amount: KnobResponse,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PatchResponse {
    pub label_changed: Option<String>,
    pub rows: Vec<PatchRowResponse>,
}

/// Control rects of one PATCH row.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PatchRects {
    pub src: Rect,
    pub tgt: Rect,
    pub knob: Rect,
    pub led: Rect,
}

pub(crate) fn patch_row_rects(row: Rect) -> PatchRects {
    let combos = (row.width() - PATCH_KNOB_D - PATCH_LED_W - 3.0 * PATCH_GAP).max(40.0);
    let src_w = (combos * 0.54).round();
    let y = row.top();
    let mut x = row.left();
    let mut take = |w: f32| {
        let r = Rect::from_min_size(pos2(x, y), vec2(w, CONTROL_H));
        x += w + PATCH_GAP;
        r
    };
    let (src, tgt, knob, led) = (take(src_w), take(combos - src_w), take(PATCH_KNOB_D), take(PATCH_LED_W));
    PatchRects { src, tgt, knob, led }
}

/// `text` cut to `max_w` px with a trailing ellipsis.
fn elide(p: &egui::Painter, f: &egui::FontId, text: &str, max_w: f32) -> String {
    if paint::text_width(p, f, text, 0.0) <= max_w {
        return text.to_string();
    }
    let mut s: String = text.to_string();
    while !s.is_empty() && paint::text_width(p, f, &format!("{s}\u{2026}"), 0.0) > max_w {
        s.pop();
    }
    format!("{}\u{2026}", s.trim_end())
}

/// Inset dropdown at an exact rect. `cur` is the shown text (`dim` for none / lost), `selected`
/// the highlighted item. Returns the picked item index.
fn patch_dropdown(ui: &mut Ui, id: egui::Id, ctrl: Rect, cur: &str, dim: bool, selected: Option<usize>, items: &[String]) -> Option<usize> {
    let f = font::label(PATCH_FONT);
    let resp = ui.interact(ctrl, id, Sense::click());
    let popup_id = id.with("popup");
    if resp.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup_id));
    }
    let open = ui.memory(|m| m.is_popup_open(popup_id));
    if ui.is_rect_visible(ctrl) {
        let p = ui.painter();
        paint::inset_box(p, ctrl, resp.hovered() || open);
        let col = if dim { T.muted.gamma_multiply(0.75) } else { T.text };
        let shown = elide(p, &f, cur, ctrl.width() - 8.0 - 18.0);
        p.text(pos2(ctrl.left() + 8.0, ctrl.center().y), Align2::LEFT_CENTER, shown, f.clone(), col);
        let (cx, cy) = (ctrl.right() - 10.0, ctrl.center().y - 1.0);
        p.add(Shape::line(vec![pos2(cx - 3.0, cy - 1.5), pos2(cx, cy + 1.5), pos2(cx + 3.0, cy - 1.5)], paint::st(1.5, T.muted)));
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    let mut picked = None;
    let min_w = items.iter().map(|s| paint::text_width(ui.painter(), &f, s, 0.0)).fold(ctrl.width() - 8.0, f32::max);
    egui::popup::popup_below_widget(ui, popup_id, &resp, egui::popup::PopupCloseBehavior::CloseOnClick, |ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        ui.set_min_width(min_w + 16.0);
        for (i, item) in items.iter().enumerate() {
            let (r, rr) = ui.allocate_exact_size(vec2(ui.available_width(), CONTROL_H), Sense::click());
            if rr.hovered() {
                ui.painter().rect_filled(r, 3.0, T.a(0.16));
            }
            let col = if Some(i) == selected { T.accent } else { T.text };
            ui.painter().text(pos2(r.left() + 8.0, r.center().y), Align2::LEFT_CENTER, item, f.clone(), col);
            if rr.clicked() && Some(i) != selected {
                picked = Some(i);
            }
        }
    });
    picked
}

/// 24 px bipolar knob for dense rows: 2.5 px arc from 12 o'clock, small cap and pointer.
fn draw_knob_small(p: &egui::Painter, c: egui::Pos2, d: f32, value01: f32, hot: bool, live: bool) {
    let r = d / 2.0 - 2.0;
    let k = if live { 1.0 } else { 0.4 };
    paint::arc_stroke(p, c, r, ARC_START, ARC_START + ARC_SWEEP, 2.5, T.track, true);
    let a_val = ARC_START + ARC_SWEEP * value01.clamp(0.0, 1.0);
    let mid = ARC_START + ARC_SWEEP / 2.0;
    if (a_val - mid).abs() > 1.0 {
        let (a0, a1) = (mid.min(a_val), mid.max(a_val));
        if hot {
            paint::arc_stroke(p, c, r, a0, a1, 8.0, T.a(0.06), false);
        }
        paint::arc_stroke(p, c, r, a0, a1, 5.0, T.a(0.18 * k), false);
        paint::arc_stroke(p, c, r, a0, a1, 2.5, T.accent.gamma_multiply(k), true);
    }
    let rc = r - 4.0;
    p.circle_filled(c, rc, T.cap1);
    p.circle_stroke(c, rc, paint::st(1.0, Color32::BLACK));
    p.circle_filled(c - vec2(0.0, 0.5), rc - 1.5, T.cap2);
    let a = a_val.to_radians();
    let dir = vec2(a.cos(), a.sin());
    p.line_segment([c + dir * rc * 0.3, c + dir * rc * 0.95], paint::st(1.5, T.text.gamma_multiply(k)));
}

/// The RR-BUS PATCH group: engraved title, editable instance label (inset box, mono text) in
/// the header, then one row per entry of `rows`: source combo | target combo | small bipolar
/// knob | activity LED. Width fills the available width, height is [`PATCH_H`] for three rows.
/// `label` is edited in place; the caller applies row changes from the response.
pub fn patch_strip(ui: &mut Ui, label: &mut String, rows: &mut [PatchRow]) -> PatchResponse {
    let mut out = PatchResponse { rows: vec![PatchRowResponse::default(); rows.len()], ..Default::default() };
    let height = PATCH_H.max(GROUP_CHROME_H + rows.len() as f32 * (CONTROL_H + PATCH_ROW_GAP) - PATCH_ROW_GAP);
    let base = ui.id().with("patch");
    group_ext(
        ui,
        vec2(0.0, height),
        "Patch",
        |hui| {
            let (rect, _) = hui.allocate_exact_size(vec2(PATCH_LABEL_W, CONTROL_H), Sense::hover());
            paint::inset_box(hui.painter(), rect, false);
            let edit = egui::TextEdit::singleline(label)
                .id(base.with("label"))
                .font(font::mono(PATCH_FONT))
                .text_color(T.text)
                .frame(false)
                .margin(vec2(8.0, 4.0))
                .char_limit(PATCH_LABEL_CHARS)
                .vertical_align(Align::Center);
            let r = hui.put(rect, edit);
            if r.hovered() {
                hui.ctx().set_cursor_icon(CursorIcon::Text);
            }
            if r.changed() {
                out.label_changed = Some(label.clone());
            }
        },
        |ui| {
            ui.spacing_mut().item_spacing.y = PATCH_ROW_GAP;
            for (i, row) in rows.iter().enumerate() {
                let (rr, _) = ui.allocate_exact_size(vec2(ui.available_width(), CONTROL_H), Sense::hover());
                let rc = patch_row_rects(rr);
                let ev = &mut out.rows[i];

                // Source: leading "None", then the live sources.
                let mut items: Vec<String> = Vec::with_capacity(row.sources.len() + 1);
                items.push("None".into());
                items.extend(row.sources.iter().cloned());
                let bound = row.source_idx.filter(|&s| s < row.sources.len());
                let (cur, dim) = match (bound, row.lost_name) {
                    (Some(s), _) => (row.sources[s].as_str(), false),
                    (None, Some(lost)) => (lost, true),
                    (None, None) => ("None", true),
                };
                if let Some(k) = patch_dropdown(ui, base.with((i, 0)), rc.src, cur, dim, Some(bound.map_or(0, |s| s + 1)), &items) {
                    ev.source_changed = Some(k.checked_sub(1));
                }
                let targets: Vec<String> = row.targets.iter().map(|s| s.to_string()).collect();
                let tcur = row.targets.get(row.target_idx).copied().unwrap_or("");
                ev.target_changed = patch_dropdown(ui, base.with((i, 1)), rc.tgt, tcur, false, Some(row.target_idx), &targets);

                // Amount knob.
                let kr = ui.interact(rc.knob, base.with((i, 2)), Sense::click_and_drag());
                let hot = kr.hovered() || kr.dragged();
                if hot {
                    ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
                }
                ev.amount = knob_interact(ui, &kr, row.amount01, 0.5);

                if ui.is_rect_visible(rr) {
                    let live = bound.is_some();
                    draw_knob_small(ui.painter(), rc.knob.center(), PATCH_KNOB_D, row.amount01, hot, live);
                    paint::led_level(ui.painter(), rc.led.center(), if live { row.activity01 } else { 0.0 });
                }
            }
        },
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_up_raises_down_lowers_and_clamps() {
        assert!((drag_value(0.5, 20.0, false) - 0.6).abs() < 1e-6);
        assert!((drag_value(0.5, -20.0, false) - 0.4).abs() < 1e-6);
        assert_eq!(drag_value(0.9, 500.0, false), 1.0);
        assert_eq!(drag_value(0.1, -500.0, false), 0.0);
    }

    #[test]
    fn shift_is_ten_times_finer() {
        let coarse = drag_value(0.5, 20.0, false) - 0.5;
        let fine = drag_value(0.5, 20.0, true) - 0.5;
        assert!((coarse / fine - 10.0).abs() < 1e-4);
    }

    #[test]
    fn full_range_takes_drag_px() {
        assert!((drag_value(0.0, DRAG_PX, false) - 1.0).abs() < 1e-6);
        assert!((drag_value(0.0, DRAG_PX, true) - FINE).abs() < 1e-6);
    }

    #[test]
    fn double_click_resets_to_default() {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx);
        let mut out = KnobResponse::default();
        let click = |pos| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() };
        let release = |pos| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() };
        let at = pos2(40.0, 40.0);
        let run = |ctx: &egui::Context, events: Vec<egui::Event>, t: f64, out: &mut KnobResponse| {
            let mut raw = egui::RawInput { time: Some(t), events, ..Default::default() };
            raw.screen_rect = Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(200.0, 200.0)));
            let _ = ctx.run(raw, |ctx| {
                egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
                    let r = knob(ui, "Cut", 0.8, 0.25, "x");
                    if r.reset {
                        *out = r;
                    }
                });
            });
        };
        run(&ctx, vec![egui::Event::PointerMoved(at)], 0.0, &mut out);
        run(&ctx, vec![click(at)], 0.05, &mut out);
        run(&ctx, vec![release(at)], 0.10, &mut out);
        run(&ctx, vec![click(at)], 0.20, &mut out);
        run(&ctx, vec![release(at)], 0.25, &mut out);
        assert!(out.reset, "double click did not report reset");
        assert_eq!(out.changed, Some(0.25));
    }

    #[test]
    fn combo_opens_and_picks() {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx);
        let picked = std::cell::Cell::new(None);
        let frame = |events: Vec<egui::Event>, t: f64| {
            let raw = egui::RawInput {
                time: Some(t),
                events,
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 300.0))),
                ..Default::default()
            };
            let _ = ctx.run(raw, |ctx| {
                egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
                    if let Some(i) = combo(ui, "", 0, &["Alpha", "Beta", "Gamma"]) {
                        picked.set(Some(i));
                    }
                });
            });
        };
        let btn = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
        let on = pos2(20.0, 12.0);
        frame(vec![egui::Event::PointerMoved(on)], 0.0);
        frame(vec![btn(on, true)], 0.1);
        frame(vec![btn(on, false)], 0.15);
        frame(vec![], 0.2);
        assert!(ctx.memory(|m| m.any_popup_open()), "popup did not open");
        // Second row of the popup, just under the control and its frame margin.
        let item = pos2(20.0, 24.0 + 8.0 + 24.0 + 12.0);
        frame(vec![egui::Event::PointerMoved(item)], 0.3);
        frame(vec![btn(item, true)], 0.4);
        frame(vec![btn(item, false)], 0.45);
        frame(vec![], 0.5);
        assert_eq!(picked.get(), Some(1));
    }

    #[test]
    fn grid_hit_maps_cells_and_rejects_labels() {
        use grid::*;
        let origin = vec2(LABEL_W + GAP, HEADER_H + GAP);
        // centre of cell (3, 2)
        let c = origin + vec2(3.0 * (CELL + GAP) + CELL / 2.0, 2.0 * (CELL + ROW_GAP) + CELL / 2.0);
        assert_eq!(hit(c, 16, 6), Some((3, 2)));
        // first and last cells
        assert_eq!(hit(origin + vec2(1.0, 1.0), 16, 6), Some((0, 0)));
        let last = origin + vec2(15.0 * (CELL + GAP) + CELL - 1.0, 5.0 * (CELL + ROW_GAP) + CELL - 1.0);
        assert_eq!(hit(last, 16, 6), Some((15, 5)));
        // label column, header row and out of range are misses
        assert_eq!(hit(vec2(10.0, 60.0), 16, 6), None);
        assert_eq!(hit(vec2(100.0, 5.0), 16, 6), None);
        assert_eq!(hit(origin + vec2(16.0 * (CELL + GAP) + 2.0, 10.0), 16, 6), None);
        assert_eq!(hit(origin + vec2(10.0, 6.0 * (CELL + ROW_GAP) + 2.0), 16, 6), None);
    }

    #[test]
    fn grid_gap_goes_to_nearest_cell() {
        use grid::*;
        let origin = vec2(LABEL_W + GAP, HEADER_H + GAP);
        // 1 px into the gap after cell 0 still belongs to cell 0; 3 px in belongs to cell 1
        assert_eq!(hit(origin + vec2(CELL + 1.0, 4.0), 16, 6), Some((0, 0)));
        assert_eq!(hit(origin + vec2(CELL + 3.0, 4.0), 16, 6), Some((1, 0)));
    }

    #[test]
    fn grid_block_size_matches_tokens() {
        let (w, h) = grid::block(16, 6);
        assert_eq!(w, 36.0 + 4.0 + 16.0 * 32.0 + 15.0 * 4.0);
        assert_eq!(h, 16.0 + 4.0 + 6.0 * 32.0 + 5.0 * 12.0);
    }

    #[test]
    fn slider_value_maps_track() {
        assert_eq!(slider_value(7.0, 7.0, 107.0), 0.0);
        assert_eq!(slider_value(57.0, 7.0, 107.0), 0.5);
        assert_eq!(slider_value(500.0, 7.0, 107.0), 1.0);
    }

    #[test]
    fn arc_end_angles() {
        // 0 -> 135 deg (lower left), 1 -> 405 = 45 deg (lower right), 0.5 -> 270 (top)
        let c = pos2(0.0, 0.0);
        let p = paint::arc_points(c, 10.0, ARC_START + ARC_SWEEP / 2.0, ARC_START + ARC_SWEEP / 2.0);
        assert!(p[0].x.abs() < 1e-3 && (p[0].y + 10.0).abs() < 1e-3);
    }

    // --- PATCH strip -------------------------------------------------------------------------

    struct PatchHarness {
        ctx: egui::Context,
        label: std::cell::RefCell<String>,
        seen: std::cell::RefCell<Vec<PatchResponse>>,
    }

    impl PatchHarness {
        fn new() -> Self {
            let ctx = egui::Context::default();
            crate::fonts::install(&ctx);
            Self { ctx, label: "GTS 1".to_string().into(), seen: Default::default() }
        }
        fn frame(&self, events: Vec<egui::Event>, t: f64) {
            let raw = egui::RawInput {
                time: Some(t),
                events,
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 400.0))),
                ..Default::default()
            };
            let sources: Vec<String> = ["GTS 1 \u{b7} Mod A", "VSS \u{b7} Step Gate"].iter().map(|s| s.to_string()).collect();
            let _ = self.ctx.run(raw, |ctx| {
                egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
                    let mk = |s: Option<usize>| PatchRow { sources: &sources, source_idx: s, lost_name: None, targets: &["Cutoff", "Index", "Pitch"], target_idx: 0, amount01: 0.5, activity01: 0.6 };
                    let mut rows = [mk(None), mk(Some(0)), mk(None)];
                    let mut label = self.label.borrow().clone();
                    let r = ui.allocate_ui(vec2(292.0, PATCH_H), |ui| patch_strip(ui, &mut label, &mut rows)).inner;
                    *self.label.borrow_mut() = label;
                    self.seen.borrow_mut().push(r);
                });
            });
        }
        fn rects(&self, row: usize) -> PatchRects {
            // Group at the origin: body starts 12 px in and 36 px down; rows are 28 px apart.
            patch_row_rects(Rect::from_min_size(pos2(12.0, 36.0 + 28.0 * row as f32), vec2(292.0 - 24.0, 24.0)))
        }
        fn click(&self, at: egui::Pos2, t: f64) {
            let btn = |pressed| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
            self.frame(vec![egui::Event::PointerMoved(at)], t);
            self.frame(vec![btn(true)], t + 0.05);
            self.frame(vec![btn(false)], t + 0.10);
            self.frame(vec![], t + 0.15);
        }
        fn any<T>(&self, f: impl Fn(&PatchResponse) -> Option<T>) -> Option<T> {
            self.seen.borrow().iter().find_map(f)
        }
    }

    #[test]
    fn patch_source_combo_pick_maps_to_source_index() {
        let h = PatchHarness::new();
        h.frame(vec![], 0.0);
        let src = h.rects(2).src;
        h.click(src.center(), 0.1);
        assert!(h.ctx.memory(|m| m.any_popup_open()), "source popup did not open");
        // Popup rows: None, "GTS 1 . Mod A", "VSS . Step Gate"; pick the third (sources[1]).
        let item = pos2(src.center().x, src.bottom() + 8.0 + 24.0 * 2.0 + 12.0);
        h.click(item, 0.5);
        assert_eq!(h.any(|r| r.rows[2].source_changed), Some(Some(1)));
        assert!(h.any(|r| r.rows[0].source_changed).is_none() && h.any(|r| r.rows[1].source_changed).is_none());
    }

    #[test]
    fn patch_source_none_entry_unbinds() {
        let h = PatchHarness::new();
        h.frame(vec![], 0.0);
        let src = h.rects(1).src; // row 1 is bound to sources[0]
        h.click(src.center(), 0.1);
        h.click(pos2(src.center().x, src.bottom() + 8.0 + 12.0), 0.5);
        assert_eq!(h.any(|r| r.rows[1].source_changed), Some(None));
    }

    #[test]
    fn patch_target_combo_pick_maps_to_target_index() {
        let h = PatchHarness::new();
        h.frame(vec![], 0.0);
        let tgt = h.rects(0).tgt;
        h.click(tgt.center(), 0.1);
        h.click(pos2(tgt.center().x, tgt.bottom() + 8.0 + 24.0 * 2.0 + 12.0), 0.5);
        assert_eq!(h.any(|r| r.rows[0].target_changed), Some(2));
    }

    #[test]
    fn patch_knob_drag_up_raises_amount() {
        let h = PatchHarness::new();
        h.frame(vec![], 0.0);
        let c = h.rects(1).knob.center();
        let btn = |pressed| egui::Event::PointerButton { pos: c, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
        h.frame(vec![egui::Event::PointerMoved(c)], 0.1);
        h.frame(vec![btn(true)], 0.15);
        h.frame(vec![egui::Event::PointerMoved(c - vec2(0.0, 10.0))], 0.2);
        h.frame(vec![egui::Event::PointerMoved(c - vec2(0.0, 20.0))], 0.25);
        h.frame(vec![egui::Event::PointerButton { pos: c - vec2(0.0, 20.0), button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() }], 0.3);
        let seen = h.seen.borrow();
        let moved: f32 = seen.iter().filter_map(|r| r.rows[1].amount.changed).fold(0.5, f32::max);
        assert!(moved > 0.5 && moved <= 0.5 + 20.0 / DRAG_PX + 1e-4, "amount {moved}");
        assert!(seen.iter().any(|r| r.rows[1].amount.drag_started) && seen.iter().any(|r| r.rows[1].amount.drag_ended));
        assert!(seen.iter().all(|r| r.rows[0].amount.changed.is_none()));
    }

    #[test]
    fn patch_label_edit_reports_new_text() {
        let h = PatchHarness::new();
        h.frame(vec![], 0.0);
        // Header label box: right-aligned in the header row (group x 0..292, pad 12).
        let at = pos2(292.0 - 12.0 - 6.0, 8.0 + 12.0);
        h.click(at, 0.1);
        h.frame(vec![egui::Event::Text("b".into())], 0.5);
        h.frame(vec![], 0.6);
        assert_eq!(h.any(|r| r.label_changed.clone()).as_deref(), Some("GTS 1b"));
        assert_eq!(*h.label.borrow(), "GTS 1b");
    }

    #[test]
    fn patch_row_rects_tile_the_row() {
        let r = patch_row_rects(Rect::from_min_size(pos2(10.0, 20.0), vec2(268.0, 24.0)));
        assert!((r.led.right() - 278.0).abs() < 1.0);
        assert!(r.src.right() < r.tgt.left() && r.tgt.right() < r.knob.left() && r.knob.right() < r.led.left());
    }
}
