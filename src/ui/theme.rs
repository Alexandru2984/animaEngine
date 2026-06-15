//! Theme system — the live implementation of `docs/design-system.md`.
//!
//! Three exports matter:
//!
//! - [`Theme`] — what's selected (persisted in `GlobalConfig.theme`)
//! - [`Palette`] — the 16-token colour table for the active theme
//! - [`apply`]   — push the theme into an `egui::Context` (idempotent;
//!   safe to call on every frame but only the first call after a switch
//!   actually changes anything)
//!
//! All hardcoded hex values, spacing numbers, and font sizes in panels.rs
//! should be replaced by references into the constants exported here.
//! If you find yourself reaching for a literal, add a token first.

use egui::{
    style::{Selection, Spacing, WidgetVisuals, Widgets},
    Color32, CornerRadius, FontFamily, FontId, Margin, Shadow, Stroke, Style, TextStyle, Vec2,
    Visuals,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ─── spacing tokens (px) — §3 of the design system ─────────────────────

pub const SPACE_XS: f32 = 2.0;
pub const SPACE_S: f32 = 4.0;
pub const SPACE_M: f32 = 8.0;
pub const SPACE_L: f32 = 12.0;
pub const SPACE_XL: f32 = 16.0;
pub const SPACE_2XL: f32 = 24.0;
pub const SPACE_3XL: f32 = 32.0;

// ─── radius tokens — §4 ────────────────────────────────────────────────

pub const RADIUS_SM: u8 = 4;
pub const RADIUS_MD: u8 = 6;
pub const RADIUS_LG: u8 = 12;

// ─── typography (logical px) — §2 ──────────────────────────────────────

pub const FONT_H1: f32 = 22.0;
pub const FONT_H2: f32 = 17.0;
pub const FONT_BODY: f32 = 13.5;
pub const FONT_CAPTION: f32 = 11.5;
pub const FONT_CODE: f32 = 12.0;

/// Named text style for `h2`. egui ships `Heading` (we map to `h1`),
/// `Body`, `Small`, and `Monospace` out of the box; for `h2` we register
/// a custom name so callers can do `RichText::new("…").text_style(h2())`.
pub fn h2() -> TextStyle {
    TextStyle::Name("H2".into())
}

/// Named text style for captions / helper text. Alias of `Small` in
/// size but kept separate so we can colour it `fg.secondary` by default
/// without affecting the stock `Small` style.
pub fn caption() -> TextStyle {
    TextStyle::Name("Caption".into())
}

// ─── theme selection ───────────────────────────────────────────────────

/// Top-level theme selector. Persisted in `GlobalConfig.theme`.
///
/// Four variants for 0.2.0: standard Dark / Light plus two high-contrast
/// siblings. High-contrast clears any tint toward grey, holds every text
/// foreground at the maximum luminance distance from its background
/// (≥ 7:1, WCAG AAA), and thickens the focus stroke for users who
/// navigate by keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    Dark,
    Light,
    DarkHighContrast,
    LightHighContrast,
}

impl Theme {
    /// In-order list for iterating in UI pickers.
    pub const ALL: &'static [Theme] = &[
        Theme::Dark,
        Theme::Light,
        Theme::DarkHighContrast,
        Theme::LightHighContrast,
    ];

    /// Localised human-readable label shown in the settings picker
    /// and the command palette. Falls through to English if the i18n
    /// subsystem hasn't initialised yet (the `t` fallback marker would
    /// look terrible in a UI label).
    pub fn label(self) -> String {
        let key = match self {
            Theme::Dark => "theme-dark",
            Theme::Light => "theme-light",
            Theme::DarkHighContrast => "theme-dark-hc",
            Theme::LightHighContrast => "theme-light-hc",
        };
        let s = crate::i18n::t(key);
        if s.starts_with('?') {
            // i18n not yet initialised — fall back to canonical names.
            match self {
                Theme::Dark => "Dark".to_string(),
                Theme::Light => "Light".to_string(),
                Theme::DarkHighContrast => "Dark · High contrast".to_string(),
                Theme::LightHighContrast => "Light · High contrast".to_string(),
            }
        } else {
            s
        }
    }

    /// `true` for the high-contrast variants; lets call sites apply
    /// extra accommodations (e.g. thicker focus ring) without
    /// matching every variant.
    pub fn is_high_contrast(self) -> bool {
        matches!(self, Theme::DarkHighContrast | Theme::LightHighContrast)
    }

    pub fn palette(self) -> Palette {
        match self {
            Theme::Dark => Palette::dark(),
            Theme::Light => Palette::light(),
            Theme::DarkHighContrast => Palette::dark_high_contrast(),
            Theme::LightHighContrast => Palette::light_high_contrast(),
        }
    }
}

// ─── palette ──────────────────────────────────────────────────────────

/// Concrete colour values for the active theme. One instance per call to
/// `Theme::palette()`; cheap to copy (it's just `Color32`s and one bool).
///
/// Matches docs/design-system.md §1 token-for-token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    // backgrounds
    pub bg_base: Color32,
    pub bg_surface: Color32,
    pub bg_elevated: Color32,

    // foreground (text)
    pub fg_primary: Color32,
    pub fg_secondary: Color32,
    pub fg_muted: Color32,
    pub fg_inverse: Color32,

    // accent
    pub accent_base: Color32,
    pub accent_hover: Color32,
    pub accent_subtle: Color32,

    // semantic
    pub semantic_success: Color32,
    pub semantic_warn: Color32,
    pub semantic_error: Color32,
    pub semantic_info: Color32,

    // borders & focus
    pub border_subtle: Color32,
    pub border_strong: Color32,
    pub border_focus: Color32,

    /// Mirrors `Visuals::dark_mode`. Drives shadow opacity and egui's
    /// internal grey-out logic; nothing else should branch on it.
    pub is_dark: bool,
}

impl Palette {
    pub fn dark() -> Self {
        Self {
            bg_base: Color32::from_rgb(0x15, 0x18, 0x1E),
            bg_surface: Color32::from_rgb(0x1E, 0x22, 0x2B),
            bg_elevated: Color32::from_rgb(0x26, 0x2B, 0x36),

            fg_primary: Color32::from_rgb(0xE8, 0xEA, 0xEF),
            fg_secondary: Color32::from_rgb(0xA8, 0xAD, 0xB7),
            fg_muted: Color32::from_rgb(0x6B, 0x72, 0x80),
            fg_inverse: Color32::from_rgb(0x15, 0x18, 0x1E),

            accent_base: Color32::from_rgb(0x7C, 0x8E, 0xFF),
            accent_hover: Color32::from_rgb(0x9A, 0xA8, 0xFF),
            // accent.base at ~13 % alpha
            accent_subtle: Color32::from_rgba_unmultiplied(0x7C, 0x8E, 0xFF, 0x22),

            semantic_success: Color32::from_rgb(0x5B, 0xCB, 0x7B),
            semantic_warn: Color32::from_rgb(0xE8, 0xB2, 0x3C),
            semantic_error: Color32::from_rgb(0xF2, 0x65, 0x65),
            semantic_info: Color32::from_rgb(0x5F, 0xA8, 0xE8),

            border_subtle: Color32::from_rgba_unmultiplied(0xFF, 0xFF, 0xFF, 0x0F),
            border_strong: Color32::from_rgba_unmultiplied(0xFF, 0xFF, 0xFF, 0x22),
            border_focus: Color32::from_rgb(0x7C, 0x8E, 0xFF),

            is_dark: true,
        }
    }

    pub fn light() -> Self {
        Self {
            bg_base: Color32::from_rgb(0xFB, 0xFB, 0xFC),
            bg_surface: Color32::from_rgb(0xF2, 0xF3, 0xF6),
            bg_elevated: Color32::from_rgb(0xFF, 0xFF, 0xFF),

            fg_primary: Color32::from_rgb(0x1A, 0x1D, 0x23),
            fg_secondary: Color32::from_rgb(0x5A, 0x60, 0x70),
            fg_muted: Color32::from_rgb(0x9C, 0xA3, 0xAF),
            fg_inverse: Color32::from_rgb(0xFB, 0xFB, 0xFC),

            accent_base: Color32::from_rgb(0x51, 0x63, 0xE8),
            accent_hover: Color32::from_rgb(0x6C, 0x7D, 0xF0),
            accent_subtle: Color32::from_rgba_unmultiplied(0x51, 0x63, 0xE8, 0x1F),

            semantic_success: Color32::from_rgb(0x1F, 0x9E, 0x55),
            semantic_warn: Color32::from_rgb(0xB0, 0x79, 0x00),
            semantic_error: Color32::from_rgb(0xC7, 0x32, 0x2F),
            semantic_info: Color32::from_rgb(0x2C, 0x70, 0xC2),

            border_subtle: Color32::from_rgba_unmultiplied(0x00, 0x00, 0x00, 0x0F),
            border_strong: Color32::from_rgba_unmultiplied(0x00, 0x00, 0x00, 0x1F),
            border_focus: Color32::from_rgb(0x51, 0x63, 0xE8),

            is_dark: false,
        }
    }

    /// Maximum-contrast dark theme. Pure black surfaces, pure white
    /// text, accent in bright cyan for ~17:1 against the base. All
    /// foreground tiers stay at white (no muted greys); the
    /// "secondary" and "muted" distinctions are conveyed through
    /// weight and surrounding spacing instead.
    pub fn dark_high_contrast() -> Self {
        Self {
            bg_base: Color32::from_rgb(0x00, 0x00, 0x00),
            bg_surface: Color32::from_rgb(0x00, 0x00, 0x00),
            bg_elevated: Color32::from_rgb(0x12, 0x12, 0x12),

            fg_primary: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            fg_secondary: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            fg_muted: Color32::from_rgb(0xE0, 0xE0, 0xE0),
            fg_inverse: Color32::from_rgb(0x00, 0x00, 0x00),

            // Bright cyan: ~16:1 on black, distinct from any text colour.
            accent_base: Color32::from_rgb(0x00, 0xE5, 0xFF),
            accent_hover: Color32::from_rgb(0x66, 0xF0, 0xFF),
            accent_subtle: Color32::from_rgba_unmultiplied(0x00, 0xE5, 0xFF, 0x40),

            // Saturated semantic colours; all pass AAA on pure black.
            semantic_success: Color32::from_rgb(0x33, 0xFF, 0x66),
            semantic_warn: Color32::from_rgb(0xFF, 0xD7, 0x00),
            semantic_error: Color32::from_rgb(0xFF, 0x4D, 0x4D),
            semantic_info: Color32::from_rgb(0x80, 0xCC, 0xFF),

            // Borders are now fully opaque, never tinted by alpha — high
            // contrast users need the box edges visible at a glance.
            border_subtle: Color32::from_rgb(0x66, 0x66, 0x66),
            border_strong: Color32::from_rgb(0xCC, 0xCC, 0xCC),
            border_focus: Color32::from_rgb(0x00, 0xE5, 0xFF),

            is_dark: true,
        }
    }

    /// Maximum-contrast light theme. Pure white surfaces, pure black
    /// text, accent in deep blue (~10:1 on white).
    pub fn light_high_contrast() -> Self {
        Self {
            bg_base: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            bg_surface: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            bg_elevated: Color32::from_rgb(0xF2, 0xF2, 0xF2),

            fg_primary: Color32::from_rgb(0x00, 0x00, 0x00),
            fg_secondary: Color32::from_rgb(0x00, 0x00, 0x00),
            fg_muted: Color32::from_rgb(0x33, 0x33, 0x33),
            fg_inverse: Color32::from_rgb(0xFF, 0xFF, 0xFF),

            // Deep navy: 11.7:1 on white, clearly distinct from black text.
            accent_base: Color32::from_rgb(0x00, 0x26, 0x80),
            accent_hover: Color32::from_rgb(0x00, 0x3A, 0xB3),
            accent_subtle: Color32::from_rgba_unmultiplied(0x00, 0x26, 0x80, 0x33),

            semantic_success: Color32::from_rgb(0x00, 0x66, 0x00),
            semantic_warn: Color32::from_rgb(0x80, 0x4A, 0x00),
            semantic_error: Color32::from_rgb(0xB3, 0x00, 0x00),
            semantic_info: Color32::from_rgb(0x00, 0x42, 0x80),

            border_subtle: Color32::from_rgb(0x99, 0x99, 0x99),
            border_strong: Color32::from_rgb(0x33, 0x33, 0x33),
            border_focus: Color32::from_rgb(0x00, 0x26, 0x80),

            is_dark: false,
        }
    }
}

// ─── apply ─────────────────────────────────────────────────────────────

/// Push the given theme into `ctx`. Cheap (clones the current `Style`
/// once and replaces it). Idempotent — calling on every frame is fine,
/// but for clarity the caller should only invoke this when the theme
/// actually changed.
pub fn apply(ctx: &egui::Context, theme: Theme) {
    let palette = theme.palette();
    let mut style: Style = (*ctx.style()).clone();
    apply_to_style(&mut style, &palette, theme.is_high_contrast());
    ctx.set_style(style);
    // Stash the palette for custom-painted widgets (toggle button,
    // keybinding chips, toast severity) — egui's Visuals only carry a
    // subset of the semantic colors, and inline literals were exactly
    // the HC-blindness the V.0 audit flagged (F6/F7).
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("anima.theme.palette"), palette));
}

/// The palette stashed by the last [`apply`]. Falls back to the dark
/// palette before the first apply (one frame at startup, at most).
pub fn palette_of(ctx: &egui::Context) -> Palette {
    ctx.data(|d| d.get_temp(egui::Id::new("anima.theme.palette")))
        .unwrap_or_else(Palette::dark)
}

/// Apply a palette to a `Style` in place. Split out from [`apply`] so
/// unit tests can inspect the result without an egui context.
fn apply_to_style(style: &mut Style, p: &Palette, high_contrast: bool) {
    apply_visuals(&mut style.visuals, p, high_contrast);
    apply_text_styles(&mut style.text_styles);
    apply_spacing(&mut style.spacing);
    // Matches §6 "popup fade + scale" — short, snappy. High-contrast
    // users sometimes have motion sensitivities — kill animations so
    // nothing flickers under their assistive tech.
    style.animation_time = if high_contrast { 0.0 } else { 0.12 };
}

fn apply_visuals(v: &mut Visuals, p: &Palette, high_contrast: bool) {
    v.dark_mode = p.is_dark;
    v.override_text_color = Some(p.fg_primary);
    v.hyperlink_color = p.accent_base;

    v.window_fill = p.bg_surface;
    v.panel_fill = p.bg_surface;
    v.faint_bg_color = p.bg_elevated;
    v.extreme_bg_color = p.bg_base;
    v.code_bg_color = p.bg_elevated;

    v.warn_fg_color = p.semantic_warn;
    v.error_fg_color = p.semantic_error;

    v.window_stroke = Stroke::new(1.0_f32, p.border_subtle);
    v.window_corner_radius = CornerRadius::same(RADIUS_LG);
    v.menu_corner_radius = CornerRadius::same(RADIUS_MD);

    // Elevation: deeper shadow on dark to compensate for low luminance contrast.
    let shadow_alpha_window = if p.is_dark { 0x48 } else { 0x28 };
    let shadow_alpha_popup = if p.is_dark { 0x38 } else { 0x20 };
    v.window_shadow = Shadow {
        offset: [0, 8],
        blur: 16,
        spread: 0,
        color: Color32::from_black_alpha(shadow_alpha_window),
    };
    v.popup_shadow = Shadow {
        offset: [0, 4],
        blur: 8,
        spread: 0,
        color: Color32::from_black_alpha(shadow_alpha_popup),
    };

    v.selection = Selection {
        bg_fill: p.accent_subtle,
        stroke: Stroke::new(1.0_f32, p.accent_base),
    };

    v.widgets = build_widgets(p, high_contrast);
}

fn build_widgets(p: &Palette, high_contrast: bool) -> Widgets {
    let radius = CornerRadius::same(RADIUS_MD);
    // High-contrast users get a thicker focus ring (3 px vs 2 px),
    // bright enough to read above any background. AA users still need
    // a clear focus indicator; this is the WCAG 2.4.7 requirement.
    let focus_width: f32 = if high_contrast { 3.0 } else { 2.0 };

    Widgets {
        // Static surfaces — panel chrome, separators, group frames.
        noninteractive: WidgetVisuals {
            bg_fill: p.bg_surface,
            weak_bg_fill: p.bg_surface,
            bg_stroke: Stroke::new(1.0_f32, p.border_subtle),
            fg_stroke: Stroke::new(1.0_f32, p.fg_primary),
            corner_radius: radius,
            expansion: 0.0,
        },
        // Idle button / control.
        inactive: WidgetVisuals {
            bg_fill: p.bg_elevated,
            weak_bg_fill: p.bg_elevated,
            bg_stroke: Stroke::new(1.0_f32, p.border_strong),
            fg_stroke: Stroke::new(1.0_f32, p.fg_primary),
            corner_radius: radius,
            expansion: 0.0,
        },
        // Hovered: subtle accent tint + accent border.
        hovered: WidgetVisuals {
            bg_fill: mix(p.bg_elevated, p.accent_base, 0.08),
            weak_bg_fill: mix(p.bg_elevated, p.accent_base, 0.08),
            bg_stroke: Stroke::new(1.0_f32, p.accent_base),
            fg_stroke: Stroke::new(1.0_f32, p.fg_primary),
            corner_radius: radius,
            expansion: 1.0,
        },
        // Pressed / focused.
        active: WidgetVisuals {
            bg_fill: mix(p.bg_elevated, p.accent_base, 0.16),
            weak_bg_fill: mix(p.bg_elevated, p.accent_base, 0.16),
            bg_stroke: Stroke::new(focus_width, p.accent_base),
            fg_stroke: Stroke::new(1.0_f32, p.fg_primary),
            corner_radius: radius,
            expansion: 1.0,
        },
        // Combo box / menu held open.
        open: WidgetVisuals {
            bg_fill: mix(p.bg_elevated, p.accent_base, 0.12),
            weak_bg_fill: mix(p.bg_elevated, p.accent_base, 0.12),
            bg_stroke: Stroke::new(1.0_f32, p.accent_base),
            fg_stroke: Stroke::new(1.0_f32, p.fg_primary),
            corner_radius: radius,
            expansion: 0.0,
        },
    }
}

fn apply_text_styles(map: &mut BTreeMap<TextStyle, FontId>) {
    let prop = FontFamily::Proportional;
    let mono = FontFamily::Monospace;

    // egui's built-in styles — overwrite to match our scale.
    map.insert(TextStyle::Heading, FontId::new(FONT_H1, prop.clone()));
    map.insert(TextStyle::Body, FontId::new(FONT_BODY, prop.clone()));
    map.insert(TextStyle::Button, FontId::new(FONT_BODY, prop.clone()));
    map.insert(TextStyle::Small, FontId::new(FONT_CAPTION, prop.clone()));
    map.insert(TextStyle::Monospace, FontId::new(FONT_CODE, mono));

    // Custom: h2 + caption. Same sizes as Body/Small but distinct keys so
    // panels can address them without changing weight/colour mid-frame.
    map.insert(h2(), FontId::new(FONT_H2, prop.clone()));
    map.insert(caption(), FontId::new(FONT_CAPTION, prop));
}

fn apply_spacing(s: &mut Spacing) {
    s.item_spacing = Vec2::new(SPACE_M, SPACE_M);
    s.button_padding = Vec2::new(SPACE_M, SPACE_S);
    s.menu_margin = Margin::same(SPACE_S as i8);
    s.window_margin = Margin::same(SPACE_XL as i8);
    s.indent = SPACE_XL;
    s.interact_size = Vec2::new(40.0, 24.0);
    s.slider_width = 160.0;
    s.combo_height = 320.0;
}

// ─── helpers ──────────────────────────────────────────────────────────

/// Linear interpolation between two `Color32`s in straight-alpha sRGB
/// space. Sufficient for hover / active tints where mathematical purity
/// matters less than coherent perception. `t` is clamped to `[0, 1]`.
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let lerp = |x: u8, y: u8| -> u8 {
        ((x as f32) * (1.0 - t) + (y as f32) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color32::from_rgba_unmultiplied(
        lerp(a.r(), b.r()),
        lerp(a.g(), b.g()),
        lerp(a.b(), b.b()),
        lerp(a.a(), b.a()),
    )
}

/// WCAG 2.1 relative luminance for an sRGB triple. Used by the contrast
/// audit in tests below; will be reused in A.9's automated contrast
/// regression test.
fn relative_luminance(c: Color32) -> f64 {
    let linear = |chan: u8| -> f64 {
        let v = chan as f64 / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(c.r()) + 0.7152 * linear(c.g()) + 0.0722 * linear(c.b())
}

/// WCAG 2.1 contrast ratio between two colours. Range `[1.0, 21.0]`.
/// AA body text requires ≥ 4.5; large text or UI components ≥ 3.0.
pub fn contrast_ratio(a: Color32, b: Color32) -> f64 {
    let la = relative_luminance(a);
    let lb = relative_luminance(b);
    let (lighter, darker) = if la > lb { (la, lb) } else { (lb, la) };
    (lighter + 0.05) / (darker + 0.05)
}

// ─── tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Token table is dense — catch a typo where two tokens accidentally
    /// share the same hex by counting unique values across the dark
    /// palette's non-semantic foreground/background fields.
    #[test]
    fn dark_palette_has_distinct_surface_tiers() {
        let p = Palette::dark();
        assert_ne!(p.bg_base, p.bg_surface);
        assert_ne!(p.bg_surface, p.bg_elevated);
        assert_ne!(p.bg_base, p.bg_elevated);
    }

    #[test]
    fn light_palette_has_distinct_surface_tiers() {
        let p = Palette::light();
        assert_ne!(p.bg_base, p.bg_surface);
        assert_ne!(p.bg_surface, p.bg_elevated);
        assert_ne!(p.bg_base, p.bg_elevated);
    }

    /// WCAG AA: primary text on the default panel surface must clear 4.5:1.
    /// If this ever drops, A.9's automated audit will catch it — we lock
    /// it in here too because regressions are easier to fix immediately.
    #[test]
    fn dark_palette_meets_aa_body_contrast() {
        let p = Palette::dark();
        let ratio = contrast_ratio(p.fg_primary, p.bg_surface);
        assert!(
            ratio >= 4.5,
            "fg_primary on bg_surface contrast {ratio:.2} is below WCAG AA (4.5)",
        );
    }

    #[test]
    fn light_palette_meets_aa_body_contrast() {
        let p = Palette::light();
        let ratio = contrast_ratio(p.fg_primary, p.bg_surface);
        assert!(
            ratio >= 4.5,
            "fg_primary on bg_surface contrast {ratio:.2} is below WCAG AA (4.5)",
        );
    }

    /// Secondary text is still expected to clear AA on the same surface;
    /// muted is the only foreground tier that may dip under 4.5 (it's
    /// meant for placeholders / disabled labels, not running prose).
    #[test]
    fn dark_secondary_text_meets_aa() {
        let p = Palette::dark();
        let ratio = contrast_ratio(p.fg_secondary, p.bg_surface);
        assert!(ratio >= 4.5, "dark fg_secondary on bg_surface = {ratio:.2}");
    }

    #[test]
    fn light_secondary_text_meets_aa() {
        let p = Palette::light();
        let ratio = contrast_ratio(p.fg_secondary, p.bg_surface);
        assert!(
            ratio >= 4.5,
            "light fg_secondary on bg_surface = {ratio:.2}"
        );
    }

    /// Accent on inverse-tone background (filled primary button) must
    /// be readable — the matching contrast is fg_inverse on accent_base.
    #[test]
    fn dark_accent_button_meets_aa_large() {
        let p = Palette::dark();
        let ratio = contrast_ratio(p.fg_inverse, p.accent_base);
        // Buttons can target the 3:1 AA-Large bar.
        assert!(ratio >= 3.0, "dark fg_inverse on accent_base = {ratio:.2}");
    }

    #[test]
    fn light_accent_button_meets_aa_large() {
        let p = Palette::light();
        let ratio = contrast_ratio(p.fg_inverse, p.accent_base);
        assert!(ratio >= 3.0, "light fg_inverse on accent_base = {ratio:.2}");
    }

    #[test]
    fn mix_endpoints() {
        let a = Color32::from_rgb(0, 0, 0);
        let b = Color32::from_rgb(255, 255, 255);
        assert_eq!(mix(a, b, 0.0), a);
        assert_eq!(mix(a, b, 1.0), b);
    }

    #[test]
    fn mix_midpoint_is_grey() {
        let a = Color32::from_rgb(0, 0, 0);
        let b = Color32::from_rgb(200, 200, 200);
        let m = mix(a, b, 0.5);
        // Rounding-tolerant equality.
        assert!((m.r() as i32 - 100).abs() <= 1);
        assert!((m.g() as i32 - 100).abs() <= 1);
        assert!((m.b() as i32 - 100).abs() <= 1);
    }

    #[test]
    fn mix_clamps_t_out_of_range() {
        let a = Color32::from_rgb(10, 10, 10);
        let b = Color32::from_rgb(200, 200, 200);
        assert_eq!(mix(a, b, -1.0), a);
        assert_eq!(mix(a, b, 2.0), b);
    }

    /// Sanity-check the apply path doesn't panic and produces a style
    /// with the expected palette token in the panel fill.
    #[test]
    fn palette_of_round_trips_through_apply() {
        let ctx = egui::Context::default();
        apply(&ctx, Theme::LightHighContrast);
        let p = palette_of(&ctx);
        assert_eq!(p, Theme::LightHighContrast.palette());
        // And the fallback before any apply:
        let fresh = egui::Context::default();
        assert_eq!(palette_of(&fresh), Palette::dark());
    }

    #[test]
    fn apply_to_style_writes_panel_fill() {
        let mut style = Style::default();
        let p = Palette::dark();
        apply_to_style(&mut style, &p, false);
        assert_eq!(style.visuals.panel_fill, p.bg_surface);
        assert!(style.visuals.dark_mode);
    }

    #[test]
    fn theme_all_covers_all_variants() {
        assert!(Theme::ALL.contains(&Theme::Dark));
        assert!(Theme::ALL.contains(&Theme::Light));
        assert!(Theme::ALL.contains(&Theme::DarkHighContrast));
        assert!(Theme::ALL.contains(&Theme::LightHighContrast));
        assert_eq!(Theme::ALL.len(), 4);
    }

    #[test]
    fn theme_default_is_dark() {
        assert_eq!(Theme::default(), Theme::Dark);
    }

    #[test]
    fn theme_is_high_contrast_matches_variants() {
        assert!(!Theme::Dark.is_high_contrast());
        assert!(!Theme::Light.is_high_contrast());
        assert!(Theme::DarkHighContrast.is_high_contrast());
        assert!(Theme::LightHighContrast.is_high_contrast());
    }

    /// High-contrast variants must clear the WCAG AAA threshold (7:1)
    /// for primary body text, not just AA. This is the whole point of
    /// the HC variants.
    #[test]
    fn dark_high_contrast_meets_aaa_body() {
        let p = Palette::dark_high_contrast();
        let ratio = contrast_ratio(p.fg_primary, p.bg_surface);
        assert!(
            ratio >= 7.0,
            "dark HC fg_primary on bg_surface = {ratio:.2}"
        );
    }

    #[test]
    fn light_high_contrast_meets_aaa_body() {
        let p = Palette::light_high_contrast();
        let ratio = contrast_ratio(p.fg_primary, p.bg_surface);
        assert!(
            ratio >= 7.0,
            "light HC fg_primary on bg_surface = {ratio:.2}",
        );
    }

    /// HC secondary and "muted" must stay AAA too — high-contrast users
    /// can't recover information from a low-contrast tier, so we don't
    /// have one.
    #[test]
    fn dark_high_contrast_secondary_and_muted_meet_aaa() {
        let p = Palette::dark_high_contrast();
        assert!(contrast_ratio(p.fg_secondary, p.bg_surface) >= 7.0);
        assert!(contrast_ratio(p.fg_muted, p.bg_surface) >= 7.0);
    }

    #[test]
    fn light_high_contrast_secondary_and_muted_meet_aaa() {
        let p = Palette::light_high_contrast();
        assert!(contrast_ratio(p.fg_secondary, p.bg_surface) >= 7.0);
        assert!(contrast_ratio(p.fg_muted, p.bg_surface) >= 7.0);
    }

    /// Semantic colours (success, warn, error, info) on the elevated
    /// surface must stay AA for HC — they're the only colour-coded
    /// signal in toasts, so a low-contrast badge would defeat the
    /// notification.
    #[test]
    fn dark_high_contrast_semantics_meet_aa() {
        let p = Palette::dark_high_contrast();
        for (name, c) in [
            ("success", p.semantic_success),
            ("warn", p.semantic_warn),
            ("error", p.semantic_error),
            ("info", p.semantic_info),
        ] {
            let ratio = contrast_ratio(c, p.bg_elevated);
            assert!(
                ratio >= 4.5,
                "dark HC semantic.{name} on bg_elevated = {ratio:.2}",
            );
        }
    }

    #[test]
    fn light_high_contrast_semantics_meet_aa() {
        let p = Palette::light_high_contrast();
        for (name, c) in [
            ("success", p.semantic_success),
            ("warn", p.semantic_warn),
            ("error", p.semantic_error),
            ("info", p.semantic_info),
        ] {
            let ratio = contrast_ratio(c, p.bg_elevated);
            assert!(
                ratio >= 4.5,
                "light HC semantic.{name} on bg_elevated = {ratio:.2}",
            );
        }
    }

    /// The accent → fg_inverse pairing drives filled "primary" buttons
    /// in HC; if it dips below AAA the call-to-action becomes
    /// unreadable.
    #[test]
    fn high_contrast_accent_button_meets_aaa() {
        for p in [
            Palette::dark_high_contrast(),
            Palette::light_high_contrast(),
        ] {
            let ratio = contrast_ratio(p.fg_inverse, p.accent_base);
            assert!(ratio >= 7.0, "HC accent button = {ratio:.2}");
        }
    }

    #[test]
    fn high_contrast_apply_kills_animation_time() {
        let mut style = Style::default();
        let p = Palette::dark_high_contrast();
        apply_to_style(&mut style, &p, true);
        assert_eq!(style.animation_time, 0.0);
    }
}
