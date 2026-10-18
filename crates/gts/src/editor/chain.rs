//! The FX chain bar and the module grips (`docs/design/MOTION-addendum-track7.md` 6): a 20 px bar
//! of chips in processing order with the Limiter pinned last, and a 6-dot grip plus an order badge
//! in every module's title row. Dragging a chip or a grip reorders the signal path; the module
//! grid itself never moves, only the numbers and the bar do.
//!
//! The order lives in `params.fx_order` (state, not a parameter). A drag only previews; the order
//! is written once, on release, through `MatrixShared::set_fx_order`, which publishes it (the audio
//! side crossfades the two orders over about 10 ms), moves `matrix_rev` and records one undo step.

use super::*;
use crate::dsp::fx::{FxOrder, MODULES};
use egui::Shape;

/// The chain bar in window px.
pub const BAR: (f32, f32, f32, f32) = (44.0, 80.0, 912.0, 20.0);
/// Module grid rects by module index (the [`FxOrder`] index): Distortion, Multiband, EQ, Phaser,
/// Chorus, Flanger, Dimension, Delay, Reverb.
pub const RECTS: [(f32, f32, f32, f32); MODULES] = [
    (44.0, 108.0, 272.0, 262.0),
    (324.0, 108.0, 384.0, 262.0),
    (716.0, 108.0, 240.0, 262.0),
    (396.0, 378.0, 276.0, 164.0),
    (44.0, 378.0, 344.0, 164.0),
    (680.0, 378.0, 276.0, 164.0),
    (44.0, 550.0, 204.0, 164.0),
    (256.0, 550.0, 352.0, 164.0),
    (616.0, 550.0, 340.0, 164.0),
];
/// The Limiter strip.
pub const LIMITER: (f32, f32, f32, f32) = (44.0, 722.0, 912.0, 56.0);
/// Module names as on the chips and the group titles.
pub const NAMES: [&str; MODULES] = ["Dist", "Multi", "EQ", "Phaser", "Chorus", "Flanger", "Dim", "Delay", "Reverb"];
pub const TITLES: [&str; MODULES] = ["Distortion", "Multiband", "EQ", "Phaser", "Chorus", "Flanger", "Dimension", "Delay", "Reverb"];
/// The on switch of each module.
pub const ON_IDS: [&str; MODULES] = ["dist_on", "mb_on", "eq_on", "phs_on", "cho_on", "flg_on", "dim_on", "dly_on", "rev_on"];
/// The title indent that leaves room for the grip and the badge.
pub const TITLE_INDENT: f32 = 34.0;

fn rect_of(r: (f32, f32, f32, f32)) -> Rect {
    Rect::from_min_size(pos2(r.0, r.1), vec2(r.2, r.3))
}

/// A module's grid rect in window px.
pub fn module_rect(m: usize) -> Rect {
    rect_of(RECTS[m])
}

/// A module's grid rect through the editor's `at` (window origin applied).
pub(super) fn rect_at(at: &dyn Fn(f32, f32, f32, f32) -> Rect, m: usize) -> Rect {
    at(RECTS[m].0, RECTS[m].1, RECTS[m].2, RECTS[m].3)
}

/// The grip hit area of a module (window px).
pub fn grip_rect(m: usize) -> Rect {
    let r = module_rect(m);
    Rect::from_min_size(pos2(r.left() + 8.0, r.top() + 8.0), vec2(14.0, 24.0))
}

/// Position of module `m` in `order` (0 = first).
pub fn slot_of(order: &FxOrder, m: usize) -> usize {
    order.0.iter().position(|&x| x as usize == m).unwrap_or(m)
}

/// `order` with module `m` moved to chain position `slot`.
pub fn moved(order: &FxOrder, m: usize, slot: usize) -> FxOrder {
    let mut v: Vec<u8> = order.0.iter().copied().filter(|&x| x as usize != m).collect();
    v.insert(slot.min(v.len()), m as u8);
    let mut a = [0u8; MODULES];
    a.copy_from_slice(&v);
    FxOrder(a)
}

#[derive(Clone, Copy, Debug)]
struct Drag {
    module: usize,
    /// Started on a module's grip (the pointer picks a module) rather than a chip (the pointer picks a slot).
    grip: bool,
    press: Pos2,
    active: bool,
    slot: usize,
}

#[derive(Clone, Copy, Default)]
struct ChainSt {
    /// Previews and benchmarks: the drag stays alive without a pointer button (see [`debug_hold_drag`]).
    hold: bool,
    drag: Option<Drag>,
    /// A drop settling: the module, where the ghost was, when (egui time).
    settle: Option<(usize, Pos2, f64)>,
}

fn st_id() -> Id {
    Id::new("gts-chain-state")
}

/// What the frame's modules need to know about the chain.
#[derive(Clone, Copy)]
pub(super) struct View {
    /// The order as shown (the preview while a drag runs).
    pub order: FxOrder,
    /// The chip under the pointer: its module lights up and the others dim.
    pub hover: Option<usize>,
    /// The module being dragged.
    pub drag: Option<usize>,
}

/// Rects of the chain chips drawn in the last frame, by module index (test hook).
pub fn chip_rects(ctx: &egui::Context) -> Vec<(usize, Rect)> {
    ctx.data(|d| d.get_temp::<Vec<(usize, Rect)>>(Id::new("gts-hook-chain"))).unwrap_or_default()
}

/// Previews and benchmarks: a chain drag of module `m` that no pointer release ends, as if the chip
/// were being carried at the pointer (pair it with a held pointer position).
pub fn debug_hold_drag(ctx: &egui::Context, m: usize) {
    let mut st = ctx.data(|d| d.get_temp::<ChainSt>(st_id())).unwrap_or_default();
    if st.drag.is_none() {
        st.hold = true;
        st.drag = Some(Drag { module: m, grip: false, press: pos2(-100.0, -100.0), active: true, slot: m });
        ctx.data_mut(|d| d.insert_temp(st_id(), st));
    }
}

/// A drag in progress: the module and the slot it would drop into (test hook).
pub fn drag_state(ctx: &egui::Context) -> Option<(usize, usize)> {
    ctx.data(|d| d.get_temp::<ChainSt>(st_id())).and_then(|s| s.drag).filter(|d| d.active).map(|d| (d.module, d.slot))
}

const CHIP_H: f32 = 20.0;
const SEP_W: f32 = 14.0;
const GAP_W: f32 = 78.0;
const X0: f32 = 64.0;

fn chip_w(p: &egui::Painter, m: usize) -> f32 {
    // pad 6, grip 7, 5, name, 6, LED 6, pad 6
    6.0 + 7.0 + 5.0 + rr_ui::paint::text_width(p, &font::strong(11.0), &NAMES[m].to_uppercase(), 0.5) + 6.0 + 6.0 + 6.0
}

/// Six dots in two columns, 2.5 px each.
fn grip(p: &egui::Painter, c: Pos2, color: Color32) {
    for col in 0..2 {
        for row in 0..3 {
            p.circle_filled(c + vec2(-2.5 + col as f32 * 5.0, (row as f32 - 1.0) * 4.5), 1.25, color);
        }
    }
}

/// Draws the bar, runs the chip drags and the commit, and returns the view the module headers use.
pub(super) fn bar(ui: &mut Ui, c: &dyn Ctl, at: &dyn Fn(f32, f32, f32, f32) -> Rect) -> View {
    let ctx = ui.ctx().clone();
    let real = c.matrix().and_then(|m| m.fx_order()).unwrap_or_default();
    let mut st = ctx.data(|d| d.get_temp::<ChainSt>(st_id())).unwrap_or_default();
    let now = ui.input(|i| i.time);
    let (down, pos, esc) = ctx.input(|i| (i.pointer.primary_down(), i.pointer.latest_pos(), i.key_pressed(egui::Key::Escape)));
    let down = down || st.hold;
    let bar_r = at(BAR.0, BAR.1, BAR.2, BAR.3);
    let origin = at(0.0, 0.0, 0.0, 0.0).min;
    let record = ctx.data(|d| d.get_temp::<bool>(Id::new("gts-test-hooks"))).unwrap_or(false);
    if record {
        ctx.data_mut(|d| d.remove_temp::<Vec<(usize, Rect)>>(Id::new("gts-hook-chain")));
    }

    // ---- drag state machine ----------------------------------------------------------------------
    let mut preview = real;
    if let Some(mut d) = st.drag {
        let travelled = pos.map_or(0.0, |q| q.distance(d.press));
        if !d.active && travelled >= 4.0 && down {
            d.active = true;
        }
        if d.active {
            if let Some(q) = pos {
                d.slot = if d.grip { grip_slot(&real, d.module, q, origin) } else { chip_slot(ui.painter(), &real, d.module, q.x - bar_r.left()) };
            }
            preview = moved(&real, d.module, d.slot);
        }
        let away = !d.grip && pos.is_some_and(|q| (q.y - bar_r.center().y).abs() > 56.0);
        if esc || (!down && (away || !d.active)) {
            // Cancelled (or never started): the chips slide back.
            st.drag = None;
            preview = real;
        } else if !down {
            // Drop: the ghost settles into the gap, the order is written once.
            let from = slot_of(&real, d.module);
            if d.slot != from {
                if let Some(m) = c.matrix() {
                    m.set_fx_order(moved(&real, d.module, d.slot));
                }
                if let Some(q) = pos {
                    st.settle = Some((d.module, q, now));
                }
            }
            preview = moved(&real, d.module, d.slot);
            st.drag = None;
        } else {
            st.drag = Some(d);
        }
    }
    let dragging = st.drag.filter(|d| d.active);
    let shown = preview;

    // ---- layout: chips of the shown order (minus the dragged one), a gap where it will land -------
    let p = ui.painter().clone();
    rr_ui::paint::tracked(&p, pos2(bar_r.left(), bar_r.center().y), Align2::LEFT_CENTER, "CHAIN", font::label(11.0), T.title, 1.4);
    let seq: Vec<usize> = shown.0.iter().map(|&m| m as usize).filter(|&m| Some(m) != dragging.map(|d| d.module)).collect();
    let gap_at = dragging.map(|d| d.slot.min(seq.len()));
    let mut x = bar_r.left() + X0;
    let mut hover = None;
    let mut chips: Vec<(usize, Rect)> = vec![];
    let mut gap_x = None;
    let settle = st.settle.filter(|s| now - s.2 < 0.12);
    if st.settle.is_some() && settle.is_none() {
        st.settle = None;
    }
    for (i, m) in seq.iter().copied().chain(std::iter::once(usize::MAX)).enumerate() {
        if gap_at == Some(i) {
            gap_x = Some(x);
            x += GAP_W + SEP_W;
        }
        if m == usize::MAX {
            break;
        }
        let w = chip_w(&p, m);
        let tx = rr_ui::motion::tween_value(&ctx, Id::new(("gts-chain-x", m)), x, 0.18);
        let r = Rect::from_min_size(pos2(tx, bar_r.top()), vec2(w, CHIP_H));
        chips.push((m, r));
        x += w + SEP_W;
    }
    // The dragged chip's own x keeps tracking its gap while it is lifted, so it lands without a jump.
    if let (Some(d), Some(gx)) = (dragging, gap_x) {
        rr_ui::motion::tween_value(&ctx, Id::new(("gts-chain-x", d.module)), gx, 0.001);
    }
    // Chevrons between the items.
    let mut prev_right: Option<f32> = None;
    let mut items: Vec<(f32, f32)> = chips.iter().map(|(_, r)| (r.left(), r.right())).collect();
    if let Some(gx) = gap_x {
        items.push((gx, gx + GAP_W));
    }
    items.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (l, r) in &items {
        if let Some(pr) = prev_right {
            let cx = (pr + l) / 2.0;
            p.add(Shape::line(vec![pos2(cx - 2.0, bar_r.center().y - 3.5), pos2(cx + 1.5, bar_r.center().y), pos2(cx - 2.0, bar_r.center().y + 3.5)], rr_ui::paint::st(1.2, T.muted.gamma_multiply(0.7))));
        }
        prev_right = Some(*r);
    }
    // The pinned Limiter chip after the last item (with its own separator).
    {
        let lx = items.last().map_or(bar_r.left() + X0, |l| l.1) + SEP_W;
        let cx = lx - SEP_W / 2.0;
        p.add(Shape::line(vec![pos2(cx - 2.0, bar_r.center().y - 3.5), pos2(cx + 1.5, bar_r.center().y), pos2(cx - 2.0, bar_r.center().y + 3.5)], rr_ui::paint::st(1.2, T.muted.gamma_multiply(0.7))));
        let w = 6.0 + 9.0 + 5.0 + rr_ui::paint::text_width(&p, &font::strong(11.0), "LIMITER", 0.5) + 8.0;
        let r = Rect::from_min_size(pos2(lx, bar_r.top()), vec2(w, CHIP_H));
        p.rect_filled(r, 4.0, T.well);
        p.rect_stroke(r, 4.0, rr_ui::paint::hair(&p, Color32::WHITE.gamma_multiply(0.06)), StrokeKind::Inside);
        lock(&p, pos2(r.left() + 11.0, r.center().y), T.muted);
        rr_ui::paint::tracked(&p, pos2(r.left() + 20.0, r.center().y), Align2::LEFT_CENTER, "LIMITER", font::strong(11.0), T.muted, 0.5);
        let resp = ui.interact(r, Id::new("gts-chain-limiter"), Sense::hover());
        if resp.hovered() {
            set_hover(ui, "LIMITER", "always last", None, crate::help::ui_help("chain:limiter").unwrap_or(""));
        }
    }
    if let Some(gx) = gap_x {
        let gr = Rect::from_min_size(pos2(rr_ui::motion::tween_value(&ctx, Id::new("gts-chain-gap"), gx, 0.18), bar_r.top()), vec2(GAP_W, CHIP_H));
        let mint = T.accent;
        p.rect_filled(gr, 4.0, mint.gamma_multiply(0.06));
        for (a, b) in [(gr.left_top(), gr.right_top()), (gr.left_bottom(), gr.right_bottom()), (gr.left_top(), gr.left_bottom()), (gr.right_top(), gr.right_bottom())] {
            p.add(Shape::dashed_line(&[a, b], rr_ui::paint::st(1.2, mint.gamma_multiply(0.8)), 3.0, 3.0));
        }
    }

    // ---- chips: input and look -------------------------------------------------------------------
    for (m, r) in chips.iter().copied() {
        if settle.is_some_and(|s| s.0 == m) {
            continue; // drawn by the settling ghost
        }
        let resp = ui.interact(r, Id::new(("gts-chain-chip", m)), Sense::click_and_drag());
        // Alt+Left / Alt+Right on a focused chip moves it one slot (keyboard reorder).
        if resp.clicked() {
            resp.request_focus();
        }
        if resp.has_focus() {
            ctx.memory_mut(|m| m.set_focus_lock_filter(resp.id, egui::EventFilter { tab: false, horizontal_arrows: true, vertical_arrows: false, escape: false }));
            let (l, rt) = ctx.input(|i| (i.modifiers.alt && i.key_pressed(egui::Key::ArrowLeft), i.modifiers.alt && i.key_pressed(egui::Key::ArrowRight)));
            let s = slot_of(&real, m);
            if (l && s > 0) || (rt && s + 1 < MODULES) {
                if let Some(mx) = c.matrix() {
                    mx.set_fx_order(moved(&real, m, if l { s - 1 } else { s + 1 }));
                }
            }
        }
        if record {
            ui.data_mut(|d| d.get_temp_mut_or_default::<Vec<(usize, Rect)>>(Id::new("gts-hook-chain")).push((m, r)));
        }
        if resp.hovered() && st.drag.is_none() {
            hover = Some(m);
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            let pos_txt = format!("{} {}", TITLES[m], slot_of(&real, m) + 1);
            set_hover(ui, "CHAIN", &pos_txt, None, crate::help::ui_help("chain:chip").unwrap_or(""));
        }
        if resp.is_pointer_button_down_on() && st.drag.is_none() && ctx.input(|i| i.pointer.primary_pressed()) {
            st.drag = Some(Drag { module: m, grip: false, press: pos.unwrap_or(r.center()), active: false, slot: slot_of(&real, m) });
        }
        paint_chip(&p, r, m, !is_off(c, ON_IDS[m]), resp.hovered());
    }
    // The ghost: the dragged chip lifted at the pointer; or settling into its gap.
    if let (Some(d), Some(q)) = (dragging, pos) {
        let w = chip_w(&p, d.module);
        let ghost = Rect::from_center_size(pos2(q.x, q.y - 5.0), vec2(w, CHIP_H));
        ghost_chip(&ctx, ghost, d.module, !is_off(c, ON_IDS[d.module]), 1.0);
        let ordered = format!("{} {} \u{203a} {}", TITLES[d.module], slot_of(&real, d.module) + 1, d.slot + 1);
        set_hover(ui, "CHAIN", &ordered, None, crate::help::ui_help("chain:drag").unwrap_or(""));
        ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
    }
    if let Some((m, from, t0)) = settle {
        if let Some((_, target)) = chips.iter().find(|(cm, _)| *cm == m).copied() {
            let k = egui::emath::easing::cubic_out(((now - t0) / 0.12).clamp(0.0, 1.0) as f32);
            let cur = pos2(from.x + (target.center().x - from.x) * k, from.y - 5.0 + (target.center().y - (from.y - 5.0)) * k);
            ghost_chip(&ctx, Rect::from_center_size(cur, target.size()), m, !is_off(c, ON_IDS[m]), 1.0 - 0.6 * k);
        }
    }
    ctx.data_mut(|d| d.insert_temp(st_id(), st));
    View { order: shown, hover, drag: dragging.map(|d| d.module) }
}

/// The slot a chip dragged to window-x offset `dx` (from the bar's left) would land in: the number
/// of the other chips whose middle is left of it, in the order without the dragged one.
fn chip_slot(p: &egui::Painter, real: &FxOrder, m: usize, dx: f32) -> usize {
    let mut x = X0;
    let mut slot = 0;
    for &o in real.0.iter().filter(|&&o| o as usize != m) {
        let w = chip_w(p, o as usize);
        if x + w / 2.0 < dx {
            slot += 1;
        }
        x += w + SEP_W;
    }
    slot
}

/// The slot a module grip dragged over another module's tile would take: that module's chain position.
fn grip_slot(real: &FxOrder, m: usize, q: Pos2, origin: Pos2) -> usize {
    let q = q - origin.to_vec2();
    (0..MODULES).filter(|&o| o != m).find(|&o| module_rect(o).contains(q)).map_or_else(|| slot_of(real, m), |o| slot_of(real, o))
}

pub(super) fn lock(p: &egui::Painter, c: Pos2, color: Color32) {
    p.rect_filled(Rect::from_center_size(c + vec2(0.0, 2.0), vec2(8.0, 6.0)), 1.5, color);
    p.add(Shape::line(vec![pos2(c.x - 2.5, c.y), pos2(c.x - 2.5, c.y - 2.5), pos2(c.x + 2.5, c.y - 2.5), pos2(c.x + 2.5, c.y)], rr_ui::paint::st(1.3, color)));
}

fn paint_chip(p: &egui::Painter, r: Rect, m: usize, on: bool, hot: bool) {
    p.rect_filled(r, 4.0, T.inset);
    p.rect_stroke(r, 4.0, rr_ui::paint::hair(p, Color32::WHITE.gamma_multiply(if hot { 0.16 } else { 0.06 })), StrokeKind::Inside);
    grip(p, pos2(r.left() + 9.5, r.center().y), if hot { T.text } else { T.muted });
    let nx = r.left() + 6.0 + 7.0 + 5.0;
    let tr = rr_ui::paint::tracked(p, pos2(nx, r.center().y), Align2::LEFT_CENTER, &NAMES[m].to_uppercase(), font::strong(11.0), if on { T.text } else { T.muted.gamma_multiply(0.7) }, 0.5);
    let led = pos2(tr.right() + 9.0, r.center().y);
    p.circle_filled(led, 3.0, if on { T.accent } else { T.led_off });
    if !on {
        p.circle_stroke(led, 3.0, rr_ui::paint::hair(p, Color32::WHITE.gamma_multiply(0.18)));
    }
    if on {
        rr_ui::fx::glow(p, led, 7.0, T.accent, 0.3);
    }
}

fn ghost_chip(ctx: &egui::Context, r: Rect, m: usize, on: bool, a: f32) {
    let lp = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, Id::new("gts-chain-ghost")));
    lp.rect_filled(r.translate(vec2(0.0, 6.0)).expand(2.0), 8.0, Color32::BLACK.gamma_multiply(0.4 * a));
    lp.rect_filled(r, 4.0, T.inset);
    lp.rect_stroke(r, 4.0, rr_ui::paint::st(1.5, T.accent.gamma_multiply(a)), StrokeKind::Inside);
    paint_chip(&lp, r, m, on, true);
}

/// A module as a group in the chain: its grip and order badge in the title row, the On switch at the
/// right, and the chain's look (a ring while its chip is hovered with the other modules dimmed, a mint
/// outline and badge while it is dragged). `body` draws the module's controls.
pub(super) fn module(ui: &mut Ui, c: &dyn Ctl, at: &dyn Fn(f32, f32, f32, f32) -> Rect, v: &View, m: usize, extra_header: impl FnOnce(&mut Ui), body: impl FnOnce(&mut Ui)) {
    let ctx = ui.ctx().clone();
    let rect = at(RECTS[m].0, RECTS[m].1, RECTS[m].2, RECTS[m].3);
    let dim = rr_ui::motion::tween(&ctx, Id::new(("gts-chain-dim", m)), v.hover.is_some_and(|h| h != m), 0.14);
    ui.scope(|ui| {
        if dim > 0.0 {
            ui.multiply_opacity(1.0 - 0.5 * dim);
        }
        let on_id = ON_IDS[m];
        rr_ui::widgets::group_at_indent(ui, rect, TITLES[m], TITLE_INDENT, |ui| {
            tg(ui, c, on_id, "");
            extra_header(ui);
        }, |ui| {
            grip_and_badge(ui, c, v, m, rect);
            fade(ui, is_off(c, on_id));
            body(ui);
        });
    });
}

/// The grip and the order badge of module `m`: painted in the module's title row, grip drag starts a reorder.
fn grip_and_badge(ui: &mut Ui, c: &dyn Ctl, v: &View, m: usize, r: Rect) {
    let ctx = ui.ctx().clone();
    let grip_r = Rect::from_min_size(pos2(r.left() + 8.0, r.top() + 8.0), vec2(14.0, 24.0));
    let resp = ui.interact(grip_r, Id::new(("gts-chain-grip", m)), Sense::click_and_drag());
    let dragging = v.drag == Some(m);
    let p = ui.painter().clone();
    let col = if dragging { T.accent } else if resp.hovered() { T.text } else { T.muted };
    grip(&p, pos2(r.left() + 15.0, r.top() + 20.0), col);
    // Badge: the chain position in a 15 px mono disc; mint while dragged.
    let pos = slot_of(&v.order, m) + 1;
    let bc = pos2(r.left() + 29.0, r.top() + 20.0);
    let bcol = if dragging { T.accent } else { T.title };
    p.circle_filled(bc, 7.5, T.inset);
    p.circle_stroke(bc, 7.5, rr_ui::paint::hair(&p, bcol.gamma_multiply(if dragging { 1.0 } else { 0.45 })));
    p.text(bc, Align2::CENTER_CENTER, pos.to_string(), font::mono(11.0), bcol);
    if dragging {
        p.rect_stroke(r, 6.0, rr_ui::paint::st(1.5, T.accent), StrokeKind::Outside);
    }
    if v.hover == Some(m) {
        p.rect_stroke(r.expand(1.5), 7.0, rr_ui::paint::st(3.0, T.accent.gamma_multiply(0.8)), StrokeKind::Outside);
    }
    if resp.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::Grab);
        set_hover(ui, &TITLES[m].to_uppercase(), &format!("chain position {pos}"), None, crate::help::ui_help("chain:grip").unwrap_or(""));
    }
    // A press starts the drag; the bar's state machine takes it from here.
    if resp.is_pointer_button_down_on() && ctx.input(|i| i.pointer.primary_pressed()) {
        let mut st = ctx.data(|d| d.get_temp::<ChainSt>(st_id())).unwrap_or_default();
        if st.drag.is_none() {
            st.drag = Some(Drag { module: m, grip: true, press: ctx.input(|i| i.pointer.latest_pos()).unwrap_or(r.center()), active: false, slot: slot_of(&v.order, m) });
            ctx.data_mut(|d| d.insert_temp(st_id(), st));
        }
    }
    let _ = c;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moving_a_module_keeps_a_permutation() {
        let d = FxOrder::default();
        for m in 0..MODULES {
            for s in 0..MODULES {
                let o = moved(&d, m, s);
                assert_eq!(slot_of(&o, m), s);
                assert!(FxOrder::new(o.0).is_some(), "{m} -> {s}: {o:?}");
            }
        }
        // Flanger (5) to the slot of Chorus (4): it lands before it.
        assert_eq!(moved(&d, 5, 4).names()[4], "Flanger");
        assert_eq!(moved(&d, 5, 4).names()[5], "Chorus");
    }

    #[test]
    fn the_grid_is_inside_the_page_and_8px_apart() {
        for r in RECTS {
            assert!(r.0 >= 44.0 && r.0 + r.2 <= 956.0 && r.1 >= 80.0 && r.1 + r.3 <= 778.0, "{r:?}");
        }
        let rects: Vec<Rect> = (0..MODULES).map(module_rect).collect();
        for (i, a) in rects.iter().enumerate() {
            for b in &rects[..i] {
                assert!(!a.intersects(*b), "{a:?} overlaps {b:?}");
                // Neighbours (touching in one axis) are exactly 8 px apart.
                let (dx, dy) = ((a.left() - b.right()).max(b.left() - a.right()), (a.top() - b.bottom()).max(b.top() - a.bottom()));
                if dx.max(dy) < 20.0 && dx.min(dy) < 0.0 {
                    assert_eq!(dx.max(dy), 8.0, "{a:?} {b:?}");
                }
            }
        }
        let l = rect_of(LIMITER);
        assert_eq!(l.bottom(), 778.0);
        assert_eq!(l.top() - module_rect(6).bottom(), 8.0);
    }
}
