//! Colours and type.
//!
//! One warm amber accent on a calm dark or paper-light base. The palette is a
//! plain struct so the UI code asks for roles ("surface", "weak text") rather
//! than hard-coding colours.

use eframe::egui::{
    self, Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle, Theme as EguiTheme,
    ThemePreference, Visuals,
};
use tsumiba_core::config::Theme;

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    /// The window background.
    pub bg: Color32,
    /// Columns and the header.
    pub surface: Color32,
    /// Inputs and cards sitting on a surface.
    pub raised: Color32,
    pub border: Color32,
    pub text: Color32,
    pub weak: Color32,
    /// Fills and strokes: buttons, selection, focus rings.
    pub accent: Color32,
    /// The accent when it is used *as text or an icon* on a surface; darker than
    /// `accent` in the light theme, where the fill colour would be too pale to read.
    pub accent_ink: Color32,
    /// Text drawn on top of the accent colour.
    pub on_accent: Color32,
    pub danger: Color32,
}

pub fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            bg: Color32::from_rgb(0x17, 0x19, 0x1f),
            surface: Color32::from_rgb(0x1e, 0x21, 0x28),
            raised: Color32::from_rgb(0x28, 0x2c, 0x36),
            border: Color32::from_rgb(0x35, 0x3a, 0x47),
            text: Color32::from_rgb(0xe8, 0xea, 0xf0),
            weak: Color32::from_rgb(0x9a, 0xa1, 0xb2),
            accent: Color32::from_rgb(0xf5, 0xa5, 0x24),
            accent_ink: Color32::from_rgb(0xf5, 0xa5, 0x24),
            on_accent: Color32::from_rgb(0x1a, 0x12, 0x05),
            danger: Color32::from_rgb(0xef, 0x6b, 0x6b),
        }
    } else {
        Palette {
            bg: Color32::from_rgb(0xf1, 0xee, 0xe6),
            surface: Color32::from_rgb(0xfa, 0xf8, 0xf3),
            raised: Color32::from_rgb(0xff, 0xff, 0xff),
            border: Color32::from_rgb(0xdc, 0xd6, 0xc8),
            text: Color32::from_rgb(0x23, 0x26, 0x2e),
            weak: Color32::from_rgb(0x5d, 0x62, 0x72),
            accent: Color32::from_rgb(0xe0, 0x93, 0x14),
            accent_ink: Color32::from_rgb(0x93, 0x55, 0x00),
            on_accent: Color32::from_rgb(0x1a, 0x12, 0x05),
            danger: Color32::from_rgb(0xc0, 0x39, 0x2b),
        }
    }
}

/// The palette for the theme egui is currently showing.
pub fn current(ctx: &egui::Context) -> Palette {
    palette(ctx.theme() == EguiTheme::Dark)
}

/// Install both themes and the user's preference between them.
pub fn apply(ctx: &egui::Context, preference: Theme) {
    ctx.set_visuals_of(EguiTheme::Dark, visuals(true));
    ctx.set_visuals_of(EguiTheme::Light, visuals(false));
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (
                TextStyle::Small,
                FontId::new(11.5, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(14.5, FontFamily::Proportional)),
            (
                TextStyle::Button,
                FontId::new(14.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Heading,
                FontId::new(17.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Monospace,
                FontId::new(13.5, FontFamily::Monospace),
            ),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
        style.spacing.scroll.floating = true;
        style.interaction.selectable_labels = false;
    });
    ctx.set_theme(match preference {
        Theme::System => ThemePreference::System,
        Theme::Light => ThemePreference::Light,
        Theme::Dark => ThemePreference::Dark,
    });
}

fn visuals(dark: bool) -> Visuals {
    let p = palette(dark);
    let mut v = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    let radius = CornerRadius::same(5);

    v.panel_fill = p.surface;
    v.window_fill = p.surface;
    v.extreme_bg_color = p.raised;
    v.faint_bg_color = p.raised;
    v.window_stroke = Stroke::new(1.0, p.border);
    v.window_corner_radius = CornerRadius::same(10);
    v.menu_corner_radius = CornerRadius::same(8);
    v.override_text_color = Some(p.text);
    v.hyperlink_color = p.accent_ink;
    v.selection.bg_fill = p.accent.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, p.accent);

    v.widgets.noninteractive.bg_fill = p.surface;
    v.widgets.noninteractive.weak_bg_fill = p.surface;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.noninteractive.corner_radius = radius;

    v.widgets.inactive.bg_fill = p.raised;
    v.widgets.inactive.weak_bg_fill = p.raised;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.inactive.corner_radius = radius;

    let hover = mix(p.raised, p.accent, 0.12);
    v.widgets.hovered.bg_fill = hover;
    v.widgets.hovered.weak_bg_fill = hover;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, p.accent.gamma_multiply(0.7));
    v.widgets.hovered.fg_stroke = Stroke::new(1.5, p.text);
    v.widgets.hovered.corner_radius = radius;

    let active = mix(p.raised, p.accent, 0.25);
    v.widgets.active.bg_fill = active;
    v.widgets.active.weak_bg_fill = active;
    v.widgets.active.bg_stroke = Stroke::new(1.0, p.accent);
    v.widgets.active.fg_stroke = Stroke::new(1.5, p.text);
    v.widgets.active.corner_radius = radius;

    v.widgets.open.bg_fill = active;
    v.widgets.open.weak_bg_fill = active;
    v.widgets.open.corner_radius = radius;
    v
}

/// `a` with `amount` (0..=1) of `b` mixed in.
pub fn mix(a: Color32, b: Color32, amount: f32) -> Color32 {
    let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * amount).round() as u8;
    Color32::from_rgb(lerp(a.r(), b.r()), lerp(a.g(), b.g()), lerp(a.b(), b.b()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WCAG relative luminance contrast ratio.
    fn contrast(a: Color32, b: Color32) -> f32 {
        fn lum(c: Color32) -> f32 {
            let f = |v: u8| {
                let v = v as f32 / 255.0;
                if v <= 0.03928 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * f(c.r()) + 0.7152 * f(c.g()) + 0.0722 * f(c.b())
        }
        let (la, lb) = (lum(a), lum(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    #[test]
    fn text_is_readable_on_every_surface_in_both_themes() {
        for dark in [true, false] {
            let p = palette(dark);
            for (name, background) in [("bg", p.bg), ("surface", p.surface), ("raised", p.raised)] {
                assert!(
                    contrast(p.text, background) >= 7.0,
                    "text on {name} (dark={dark}): {:.1}",
                    contrast(p.text, background)
                );
                assert!(
                    contrast(p.weak, background) >= 4.5,
                    "weak text on {name} (dark={dark}): {:.1}",
                    contrast(p.weak, background)
                );
            }
        }
    }

    #[test]
    fn the_accent_is_readable_as_text_on_every_surface() {
        for dark in [true, false] {
            let p = palette(dark);
            for (name, background) in [("bg", p.bg), ("surface", p.surface), ("raised", p.raised)] {
                assert!(
                    contrast(p.accent_ink, background) >= 4.5,
                    "accent text on {name} (dark={dark}): {:.1}",
                    contrast(p.accent_ink, background)
                );
            }
        }
    }

    #[test]
    fn text_on_the_accent_is_readable() {
        for dark in [true, false] {
            let p = palette(dark);
            assert!(
                contrast(p.on_accent, p.accent) >= 4.5,
                "dark={dark}: {:.1}",
                contrast(p.on_accent, p.accent)
            );
        }
    }

    #[test]
    fn mixing_endpoints() {
        let (a, b) = (Color32::from_rgb(0, 0, 0), Color32::from_rgb(200, 100, 50));
        assert_eq!(mix(a, b, 0.0), a);
        assert_eq!(mix(a, b, 1.0), b);
        assert_eq!(mix(a, b, 0.5), Color32::from_rgb(100, 50, 25));
    }
}
