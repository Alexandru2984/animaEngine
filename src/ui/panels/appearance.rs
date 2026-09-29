//! Appearance tab — theme picker, language picker, accessibility,
//! onboarding reset, perf-overlay hint. Extracted in I.6.
//!
//! Also houses the persistent warning banner (rendered by the
//! settings panel between the tab switcher and the tab body) and
//! the theme-label helpers shared with the command palette.

use crate::i18n::t;
use crate::ui::accessible::ComboOption;
use crate::ui::banner::{Severity, Warning};
use crate::ui::icons;
use crate::ui::onboarding::{self, OnboardingProgress};
use crate::ui::theme::{self, h2, Theme, SPACE_2XL, SPACE_M, SPACE_S, SPACE_XS};

#[allow(clippy::too_many_arguments)]
pub(super) fn appearance_tab(
    ui: &mut egui::Ui,
    theme: &mut Theme,
    locale: &mut Option<String>,
    config_dirty: &mut bool,
    onboarding: &mut OnboardingProgress,
    accesskit_enabled: &mut bool,
    accesskit_supported: bool,
    reduced_motion: &mut bool,
    hover_startle: &mut bool,
    on_fullscreen: &mut crate::fullscreen::OnFullscreen,
    away: super::AwayControls<'_>,
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
    // Disabled, with the reason on hover, where there is no bridge to
    // drive: a checkbox that changes nothing tells a screen-reader user
    // the feature exists here when it does not (same pattern as Span on
    // this backend).
    let accesskit = ui.add_enabled(
        accesskit_supported,
        egui::Checkbox::new(accesskit_enabled, t("appearance-accesskit-label")),
    );
    let accesskit = if accesskit_supported {
        accesskit.on_hover_text(t("appearance-accesskit-hint"))
    } else {
        accesskit.on_disabled_hover_text(t("appearance-accesskit-unsupported"))
    };
    if accesskit.changed() {
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
    ui.add_space(SPACE_S);
    if ui
        .checkbox(hover_startle, t("appearance-hover-startle-label"))
        .on_hover_text(t("appearance-hover-startle-hint"))
        .changed()
    {
        *config_dirty = true;
    }
    ui.add_space(SPACE_S);
    if fullscreen_picker(ui, on_fullscreen) {
        *config_dirty = true;
    }
    ui.add_space(SPACE_S);
    if away_controls(ui, away) {
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

    let combo_value = active_label.clone();
    let combo = egui::ComboBox::from_id_salt("anima.language.picker")
        .selected_text(active_label)
        .show_ui(ui, |ui| {
            // A language whose script the machine has no font for would
            // draw as a row of empty boxes — the whole UI, not one glyph.
            // Offer it disabled with a reason instead of letting someone
            // pick it and lose the ability to read the picker itself.
            let cjk_ok = crate::ui::icons::cjk_font_available();
            // Every entry is labelled with its *own* name — `日本語`, not
            // "Japanese" — and the CJK face is otherwise loaded only once a
            // CJK locale is already active. So the single row a Japanese
            // reader needs to find was the single row drawn as empty boxes
            // (R33). This closure runs only while the dropdown is open, so
            // the face is pulled in on a deliberate act rather than for
            // everyone at startup, which is what R23 was protecting.
            //
            // Requested every frame the list is open: `install_with_cjk` is
            // idempotent, and `set_fonts` takes effect on the next frame,
            // so asking once on the opening frame would leave the list
            // wrong for exactly that frame.
            if cjk_ok && !crate::ui::icons::locale_needs_cjk(&active_code) {
                crate::ui::icons::install_with_cjk(ui.ctx());
            }
            for (code, autonym) in SUPPORTED {
                let selected = *code == active_code;
                let drawable = cjk_ok || !crate::ui::icons::locale_needs_cjk(code);
                let resp = ui.add_enabled(drawable, egui::SelectableLabel::new(selected, *autonym));
                let resp = if drawable {
                    resp
                } else {
                    resp.on_disabled_hover_text(t("appearance-language-no-font"))
                };
                if resp.picked() && !selected {
                    set_locale(code);
                    *locale = Some((*code).to_string());
                    // Fonts are chosen per locale, so the stack has to be
                    // rebuilt here — otherwise switching to Japanese at
                    // runtime keeps the Latin-only fonts and draws boxes.
                    crate::ui::icons::install(ui.ctx());
                    changed = true;
                }
            }
        });
    crate::ui::accessible::name_combo(
        &combo.response,
        &t("appearance-language-header"),
        &combo_value,
    );
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
        ui.label(format!(
            "{}  {}",
            icons::PALETTE,
            t("appearance-theme-label")
        ));
        let combo_value = theme_label_with_icon(*theme);
        let combo = egui::ComboBox::from_id_salt("theme_picker")
            .selected_text(theme_label_with_icon(*theme))
            .show_ui(ui, |ui| {
                for option in Theme::ALL {
                    if ui
                        .selectable_label(*theme == *option, theme_label_with_icon(*option))
                        .picked()
                        && *theme != *option
                    {
                        *theme = *option;
                        changed = true;
                    }
                }
            });
        crate::ui::accessible::name_combo(
            &combo.response,
            &t("appearance-theme-label"),
            &combo_value,
        );
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

/// When to hold the characters still for someone being away
/// (`crate::away`): after how long idle, and on battery. Returns whether
/// either changed.
fn away_controls(ui: &mut egui::Ui, away: super::AwayControls<'_>) -> bool {
    let mut changed = false;
    let label = |minutes: u32| {
        if minutes == 0 {
            t("away-never")
        } else {
            let mut args = fluent::FluentArgs::new();
            args.set("minutes", minutes);
            crate::i18n::t_args("away-after-minutes", &args)
        }
    };
    let can_tell = away.idle_available != Some(false);
    ui.add_enabled_ui(can_tell, |ui| {
        ui.horizontal(|ui| {
            ui.label(t("appearance-away-label"));
            let combo_value = label(*away.idle_minutes);
            let combo = egui::ComboBox::from_id_salt("anima.pause_when_idle")
                .selected_text(combo_value.clone())
                .show_ui(ui, |ui| {
                    for minutes in crate::away::IDLE_CHOICES {
                        if ui
                            .selectable_label(*away.idle_minutes == minutes, label(minutes))
                            .picked()
                            && *away.idle_minutes != minutes
                        {
                            *away.idle_minutes = minutes;
                            changed = true;
                        }
                    }
                });
            crate::ui::accessible::name_combo(
                &combo.response,
                &t("appearance-away-label"),
                &combo_value,
            );
            combo.response.on_hover_text(t("appearance-away-hint"));
        });
    });
    if !can_tell {
        ui.add(
            egui::Label::new(
                egui::RichText::new(t("appearance-away-unavailable"))
                    .text_style(crate::ui::theme::caption())
                    .color(ui.visuals().weak_text_color()),
            )
            .wrap(),
        );
    }
    if ui
        .checkbox(away.on_battery, t("appearance-battery-label"))
        .on_hover_text(t("appearance-battery-hint"))
        .changed()
    {
        changed = true;
    }
    changed
}

/// What the overlay does while a full-screen app is in front
/// (`crate::fullscreen`). Returns whether the choice changed.
fn fullscreen_picker(ui: &mut egui::Ui, setting: &mut crate::fullscreen::OnFullscreen) -> bool {
    use crate::fullscreen::OnFullscreen;
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(t("appearance-fullscreen-label"));
        let combo_value = t(setting.i18n_key());
        let combo = egui::ComboBox::from_id_salt("anima.on_fullscreen")
            .selected_text(combo_value.clone())
            .show_ui(ui, |ui| {
                for option in OnFullscreen::ALL {
                    if ui
                        .selectable_label(*setting == option, t(option.i18n_key()))
                        .picked()
                        && *setting != option
                    {
                        *setting = option;
                        changed = true;
                    }
                }
            });
        crate::ui::accessible::name_combo(
            &combo.response,
            &t("appearance-fullscreen-label"),
            &combo_value,
        );
        combo
            .response
            .on_hover_text(t("appearance-fullscreen-hint"));
    });
    changed
}
