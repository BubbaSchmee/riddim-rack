//! Design tokens (Variant B, "Mint") and the egui `Visuals` built from them.

use egui::{Color32, Context, CornerRadius, FontFamily, FontId, Stroke, TextStyle};

/// Colour tokens. Alpha variants are derived with [`Theme::a`] so the table stays opaque.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub panel_top: Color32,
    pub panel_bottom: Color32,
    pub well: Color32,
    pub inset: Color32,
    pub rail: Color32,
    pub rail_slot: Color32,
    pub text: Color32,
    pub muted: Color32,
    /// Engraved group titles.
    pub title: Color32,
    pub accent: Color32,
    pub track: Color32,
    pub cap1: Color32,
    pub cap2: Color32,
    pub led_off: Color32,
    pub plate_text: Color32,
    pub disp_top: Color32,
    pub disp_bottom: Color32,
    pub disp_caption: Color32,
    pub tick_off: Color32,
    pub cell_off: Color32,
    pub cell_off_stroke: Color32,
}

/// Mint on slate, the approved look.
pub const MINT: Theme = Theme {
    panel_top: Color32::from_rgb(0x23, 0x2a, 0x2e),
    panel_bottom: Color32::from_rgb(0x19, 0x20, 0x24),
    well: Color32::from_rgb(0x11, 0x17, 0x1a),
    inset: Color32::from_rgb(0x14, 0x1b, 0x1e),
    rail: Color32::from_rgb(0x2a, 0x32, 0x36),
    rail_slot: Color32::from_rgb(0x0b, 0x10, 0x12),
    text: Color32::from_rgb(0xe6, 0xec, 0xe9),
    muted: Color32::from_rgb(0x93, 0xa1, 0x9b),
    title: Color32::from_rgb(0x83, 0x91, 0x8b),
    accent: Color32::from_rgb(0x5e, 0xe6, 0xa8),
    track: Color32::from_rgb(0x33, 0x40, 0x46),
    cap1: Color32::from_rgb(0x27, 0x31, 0x36),
    cap2: Color32::from_rgb(0x36, 0x43, 0x4a),
    led_off: Color32::from_rgb(0x1b, 0x1b, 0x1e),
    plate_text: Color32::from_rgb(0xd8, 0xd5, 0xce),
    disp_top: Color32::from_rgb(0x0f, 0x12, 0x14),
    disp_bottom: Color32::from_rgb(0x05, 0x06, 0x07),
    // Mockup #6c7478 measures 4.3:1 on the display; lifted to clear 4.5:1.
    disp_caption: Color32::from_rgb(0x7b, 0x86, 0x8b),
    tick_off: Color32::from_rgb(0x2a, 0x30, 0x34),
    cell_off: Color32::from_rgb(0x0d, 0x10, 0x12),
    cell_off_stroke: Color32::from_rgb(0x17, 0x1c, 0x1f),
};

/// The active theme. One variant ships; widgets read this directly.
pub const T: Theme = MINT;

impl Theme {
    /// Accent at alpha `a` (0..=1).
    pub fn a(&self, a: f32) -> Color32 {
        self.accent.gamma_multiply(a)
    }
    /// Raised-edge highlight, white at 7 %.
    pub fn hi(&self) -> Color32 {
        Color32::WHITE.gamma_multiply(0.07)
    }
    /// Recessed-edge shadow, black at 60 %.
    pub fn lo(&self) -> Color32 {
        Color32::BLACK.gamma_multiply(0.6)
    }
}

/// Geometry in px (logical points). 8 px grid with 4 px half steps.
pub mod metrics {
    pub const RAIL_W: f32 = 28.0;
    pub const PAD_X: f32 = 44.0;
    pub const PAD_TOP: f32 = 16.0;
    pub const PAD_BOTTOM: f32 = 20.0;
    pub const GROUP_R: f32 = 6.0;
    pub const GROUP_PAD_X: f32 = 12.0;
    pub const GROUP_PAD_TOP: f32 = 8.0;
    pub const GROUP_PAD_BOTTOM: f32 = 8.0;
    pub const GROUP_HEADER_H: f32 = 24.0;
    pub const GROUP_HEADER_GAP: f32 = 4.0;
    /// Vertical space a group adds around its body.
    pub const GROUP_CHROME_H: f32 = GROUP_PAD_TOP + GROUP_HEADER_H + GROUP_HEADER_GAP + GROUP_PAD_BOTTOM;
    pub const GAP: f32 = 12.0;
    pub const CONTROL_H: f32 = 24.0;
    pub const KNOB_D: f32 = 40.0;
    pub const KNOB_D_LG: f32 = 44.0;
    pub const KNOB_CELL_W: f32 = 56.0;
    pub const KNOB_LABEL_H: f32 = 13.0;
    pub const KNOB_READOUT_H: f32 = 14.0;
    pub const DISPLAY_H: f32 = 124.0;
    pub const SLIDER_W: f32 = 168.0;
    pub const CELL: f32 = 32.0;
    /// Height of a knob cell (label + knob + readout) for a knob of diameter `d`.
    pub const fn knob_cell_h(d: f32) -> f32 {
        KNOB_LABEL_H + d + KNOB_READOUT_H
    }
}

/// Font helpers. Families are registered by [`crate::fonts::install`].
pub mod font {
    use super::*;
    /// Barlow Medium: labels, combo text.
    pub fn label(size: f32) -> FontId {
        FontId::new(size, FontFamily::Proportional)
    }
    /// Barlow SemiBold: nameplate, group titles, vowel labels.
    pub fn strong(size: f32) -> FontId {
        FontId::new(size, FontFamily::Name("barlow-semibold".into()))
    }
    /// JetBrains Mono Regular: numeric readouts.
    pub fn mono(size: f32) -> FontId {
        FontId::new(size, FontFamily::Monospace)
    }
}

/// Install fonts and set the egui style. Call once when the editor is built.
pub fn apply(ctx: &Context) {
    crate::fonts::install(ctx);
    let mut v = egui::Visuals::dark();
    v.panel_fill = T.panel_bottom;
    v.window_fill = T.inset;
    v.window_stroke = Stroke::new(1.0_f32, Color32::BLACK);
    v.window_corner_radius = CornerRadius::same(4);
    v.window_shadow = egui::Shadow { offset: [0, 4], blur: 12, spread: 0, color: Color32::BLACK.gamma_multiply(0.5) };
    v.popup_shadow = v.window_shadow;
    v.extreme_bg_color = T.well;
    v.faint_bg_color = T.inset;
    v.override_text_color = Some(T.text);
    v.selection.bg_fill = T.a(0.25);
    v.selection.stroke = Stroke::new(1.0_f32, T.accent);
    v.menu_corner_radius = CornerRadius::same(4);
    ctx.set_visuals(v);
    ctx.style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8.0, 8.0);
        s.spacing.window_margin = egui::Margin::same(4);
        s.text_styles = [
            (TextStyle::Heading, font::strong(18.0)),
            (TextStyle::Body, font::label(13.0)),
            (TextStyle::Button, font::label(13.0)),
            (TextStyle::Small, font::label(11.0)),
            (TextStyle::Monospace, font::mono(12.0)),
        ]
        .into();
    });
}

/// WCAG relative luminance of an opaque colour.
pub fn luminance(c: Color32) -> f64 {
    let ch = |v: u8| {
        let s = v as f64 / 255.0;
        if s <= 0.03928 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * ch(c.r()) + 0.7152 * ch(c.g()) + 0.0722 * ch(c.b())
}

/// WCAG contrast ratio, always >= 1.
pub fn contrast(a: Color32, b: Color32) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_clear_aa_on_the_well() {
        for (name, fg) in [("text", T.text), ("muted", T.muted), ("title", T.title), ("accent", T.accent)] {
            let c = contrast(fg, T.well);
            assert!(c >= 4.5, "{name} on well is {c:.2}:1");
        }
    }

    #[test]
    fn text_clears_aa_on_every_surface() {
        for (name, bg) in [("inset", T.inset), ("panel_top", T.panel_top), ("panel_bottom", T.panel_bottom), ("display", T.disp_top)] {
            for (fname, fg) in [("text", T.text), ("muted", T.muted)] {
                let c = contrast(fg, bg);
                assert!(c >= 4.5, "{fname} on {name} is {c:.2}:1");
            }
        }
        assert!(contrast(T.disp_caption, T.disp_top) >= 4.5);
        assert!(contrast(T.accent, T.disp_top) >= 4.5);
    }

    #[test]
    fn contrast_math_known_values() {
        assert!((contrast(Color32::WHITE, Color32::BLACK) - 21.0).abs() < 1e-6);
        assert!((contrast(Color32::WHITE, Color32::WHITE) - 1.0).abs() < 1e-9);
    }
}
