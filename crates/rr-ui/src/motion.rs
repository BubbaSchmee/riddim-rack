//! Motion helpers shared by the widgets (docs/design/MOTION.md 3, 4): the "Reduced motion"
//! switch, durations that honour it, and the SYNTH / FX page transition (staggered fade and rise).
//!
//! Everything is time-based on egui's clock. With reduced motion on, every chrome transition is
//! instant (zero length) and the page transition is skipped; live data displays are unaffected.

use egui::emath::{easing, TSTransform};
use egui::{vec2, Context, Id, Ui};

fn rm_id() -> Id {
    Id::new("rr-reduced-motion")
}

/// Turns reduced motion on or off for this frame (the host editor calls it every frame).
pub fn set_reduced_motion(ctx: &Context, on: bool) {
    ctx.data_mut(|d| d.insert_temp(rm_id(), on));
}

pub fn reduced_motion(ctx: &Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(rm_id())).unwrap_or(false)
}

/// `secs` for a chrome transition, 0 under reduced motion.
pub fn dur(ctx: &Context, secs: f32) -> f32 {
    if reduced_motion(ctx) {
        0.0
    } else {
        secs
    }
}

/// `animate_bool_with_time` that honours reduced motion (instant) and eases out (MOTION.md 3).
pub fn tween(ctx: &Context, id: Id, target: bool, secs: f32) -> f32 {
    if reduced_motion(ctx) {
        return target as u8 as f32;
    }
    ctx.animate_bool_with_time_and_easing(id, target, secs, easing::cubic_out)
}

/// Linear variant (the label / value cross-fade).
pub fn tween_linear(ctx: &Context, id: Id, target: bool, secs: f32) -> f32 {
    if reduced_motion(ctx) {
        return target as u8 as f32;
    }
    ctx.animate_bool_with_time(id, target, secs)
}

// ---- page transition --------------------------------------------------------------------------

/// Out phase (60 ms) then in phase (120 ms fade + 160 ms rise, 10 ms stagger per row, 40 ms cap).
pub const PAGE_OUT: f32 = 0.06;
pub const PAGE_IN: f32 = 0.20;

/// Where the page transition is: `out` = the old page leaving, `t` = seconds into the phase.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PageTrans {
    pub out: bool,
    pub t: f32,
}

fn pt_id() -> Id {
    Id::new("rr-page-trans")
}

pub fn set_page_trans(ctx: &Context, t: Option<PageTrans>) {
    ctx.data_mut(|d| match t {
        Some(t) => d.insert_temp(pt_id(), t),
        None => {
            d.remove_temp::<PageTrans>(pt_id());
        }
    });
}

pub fn page_trans(ctx: &Context) -> Option<PageTrans> {
    ctx.data(|d| d.get_temp::<PageTrans>(pt_id()))
}

/// (opacity, dy in px) of an element whose top edge is at window y `y` during `t`.
pub fn page_fx_at(t: PageTrans, y: f32) -> (f32, f32) {
    if t.out {
        let e = easing::cubic_in((t.t / PAGE_OUT).clamp(0.0, 1.0));
        (1.0 - e, -4.0 * e)
    } else {
        let stagger = ((y / 130.0).floor().max(0.0) * 0.010).min(0.040);
        let tt = t.t - stagger;
        let a = easing::cubic_out((tt / 0.12).clamp(0.0, 1.0));
        let r = easing::cubic_out((tt / 0.16).clamp(0.0, 1.0));
        (a, 8.0 * (1.0 - r))
    }
}

/// Opacity and offset for an element at `y` right now (identity when no transition runs).
pub fn page_fx(ctx: &Context, y: f32) -> (f32, f32) {
    page_trans(ctx).map_or((1.0, 0.0), |t| page_fx_at(t, y))
}

/// Runs `add` with the page transition applied to everything it paints (fade and rise by `y`).
/// Layout and hit areas are not moved: only the visuals, for a few frames.
pub fn with_page_fx<R>(ui: &mut Ui, y: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let (a, dy) = page_fx(ui.ctx(), y);
    if a >= 0.999 && dy.abs() < 0.01 {
        return add(ui);
    }
    ui.with_visual_transform(TSTransform::from_translation(vec2(0.0, dy)), |ui| {
        ui.multiply_opacity(a);
        add(ui)
    })
    .inner
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_totals_at_most_260_ms_and_first_content_at_60() {
        // The old page is gone at the end of the out phase.
        let (a, _) = page_fx_at(PageTrans { out: true, t: PAGE_OUT }, 100.0);
        assert!(a < 0.01);
        // The slowest element (row cap 40 ms) is fully in 160 ms + 40 ms after the in phase starts.
        let (a, dy) = page_fx_at(PageTrans { out: false, t: 0.20 }, 700.0);
        assert!(a > 0.999 && dy.abs() < 1e-3, "{a} {dy}");
        assert!(PAGE_OUT + PAGE_IN <= 0.26 + 1e-6);
        // First new content is already visible a few ms into the in phase for the top row.
        assert!(page_fx_at(PageTrans { out: false, t: 0.02 }, 90.0).0 > 0.1);
    }
}
