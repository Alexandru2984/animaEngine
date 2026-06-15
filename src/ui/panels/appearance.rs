//! Appearance tab — theme picker, language picker, accessibility,
//! onboarding reset, perf-overlay hint. Extracted in I.6.
//!
//! Also houses the persistent warning banner (rendered by the
//! settings panel between the tab switcher and the tab body) and
//! the theme-label helpers shared with the command palette.

use crate::i18n::t;
use crate::ui::banner::{Severity, Warning};
use crate::ui::icons;
use crate::ui::onboarding::{self, OnboardingProgress};
use crate::ui::theme::{self, h2, Theme, SPACE_2XL, SPACE_M, SPACE_S, SPACE_XS};

pub(super) fn appearance_tab(
    ui: &mut egui::Ui,
    theme: &mut Theme,
    locale: &mut Option<String>,
    config_dirty: &mut bool,
    onboarding: &mut OnboardingProgress,
    accesskit_enabled: &mut bool,
    reduced_motion: &mut bool,
) {
    ui.label(egui::RichText::new(t("appearance-theme-header")).text_style(h2()));
    ui.add_space(SPACE_S);
    if theme_picker(ui, theme) {
        *config_dirty = true;
    }
    ui.add_space(SPACE_S);
    if onboarding::hint(ui, &t("onboarding-theme"), &mut onboarding.theme) {
        *config_dirty = true;
    }
    ui.add_space(SPACE_2XL);

    // ── Language ─────────────────────────────────────────────────────
    ui.label(egui::RichText::new(t("appearance-language-header")).text_style(h2()));
    ui.add_space(SPACE_S);
    if language_picker(ui, locale) {
        *config_dirty = true;
    }
    ui.add_space(SPACE_2XL);

    // ── Accessibility ────────────────────────────────────────────────
    ui.label(egui::RichText::new(t("appearance-accessibility-header")).text_style(h2()));
    ui.add_space(SPACE_S);
    if ui
        .checkbox(accesskit_enabled, t("appearance-accesskit-label"))
        .on_hover_text(t("appearance-accesskit-hint"))
        .changed()
    {
        *config_dirty = true;
    }
    ui.add_space(SPACE_S);
    if ui
        .checkbox(reduced_motion, t("appearance-reduced-motion-label"))
        .on_hover_text(t("appearance-reduced-motion-hint"))
        .changed()
    {
        *config_dirty = true;
    }
    ui.add_space(SPACE_M);

    // ── Reset onboarding hints (D.7) ─────────────────────────────────
    // Single button — retakes every progressive hint plus the
    // What's new panel for this version.
    if ui
        .button(t("appearance-reset-onboarding"))
        .on_hover_text(t("appearance-reset-onboarding-hint"))
        .clicked()
    {
        onboarding.reset();
        *config_dirty = true;
    }
    ui.add_space(SPACE_M);

    // ── Perf-overlay hint (D.7, dev-affordance, gated by onboarding) ─
    if onboarding::hint(
        ui,
        &t("onboarding-perf-overlay"),
        &mut onboarding.perf_overlay,
    ) {
        *config_dirty = true;
    }
}

/// Locale dropdown. Each option is the locale's *autonym* (its name in
/// its own language) so users see "Română" / "日本語" / "Polski" and
/// can pick theirs without reading English first.
fn language_picker(ui: &mut egui::Ui, locale: &mut Option<String>) -> bool {
    use crate::i18n::{current_locale, set_locale, SUPPORTED};
    let mut changed = false;
    let active_code = locale
        .as_deref()
        .map(|s| s.to_string())
        .unwrap_or_else(current_locale);
    let active_label = SUPPORTED
        .iter()
        .find(|(c, _)| *c == active_code)
        .map(|(_, name)| (*name).to_string())
        .unwrap_or_else(|| active_code.clone());

    egui::ComboBox::from_id_salt("anima.language.picker")
        .selected_text(active_label)
        .show_ui(ui, |ui| {
            for (code, autonym) in SUPPORTED {
                let selected = *code == active_code;
                if ui.selectable_label(selected, *autonym).clicked() && !selected {
                    set_locale(code);
                    *locale = Some((*code).to_string());
                    changed = true;
                }
            }
        });
    changed
}

/// Render a single persistent warning banner inside the settings
/// panel. Severity drives the accent colour; the message body comes
/// from i18n via the `Warning::i18n_key`. No dismiss button for now —
/// banners auto-clear when the underlying condition resolves.
pub(super) fn warning_banner(ui: &mut egui::Ui, warning: Warning) {
    let accent = match warning.severity() {
        Severity::Warn => egui::Color32::from_rgb(220, 180, 60),
        Severity::Error => egui::Color32::from_rgb(220, 80, 80),
    };
    egui::Frame::group(ui.style())
        .stroke(egui::Stroke::new(1.0_f32, accent))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(icons::WARN).color(accent));
                ui.label(egui::RichText::new(t(warning.i18n_key())).text_style(theme::caption()));
            });
        });
    ui.add_space(SPACE_XS);
}

/// Theme dropdown. Returns `true` when the user picked a different
/// theme than the current value, so the caller can flag the config
/// dirty.
fn theme_picker(ui: &mut egui::Ui, theme: &mut Theme) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(format!("{}  Theme", icons::PALETTE));
        egui::ComboBox::from_id_salt("theme_picker")
            .selected_text(theme_label_with_icon(*theme))
            .show_ui(ui, |ui| {
                for option in Theme::ALL {
                    if ui
                        .selectable_label(*theme == *option, theme_label_with_icon(*option))
                        .clicked()
                        && *theme != *option
                    {
                        *theme = *option;
                        changed = true;
                    }
                }
            });
    });
    changed
}

fn theme_label_with_icon(t: Theme) -> String {
    let icon = match t {
        Theme::Dark | Theme::DarkHighContrast => icons::DARK_MODE,
        Theme::Light | Theme::LightHighContrast => icons::LIGHT_MODE,
    };
    format!("{icon}  {}", t.label())
}
