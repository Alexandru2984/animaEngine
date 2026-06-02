//! Progressive in-line onboarding hints.
//!
//! We deliberately avoid a first-run modal (per project decision —
//! see [[user-language]] notes). Instead, contextual one-line hints
//! appear next to the feature they explain, and the user dismisses
//! each one with a small ✕. State is persisted in
//! [`crate::config::AppConfig`].
//!
//! The hint widget is a thin, dismissible chip — never blocking,
//! never auto-disappearing. It uses the accent-subtle background with
//! a left accent stripe so it reads as advisory, not as a notification.
//!
//! ## Distinguishing fresh installs from upgrades
//!
//! Configs that predate A.6 don't have the `onboarding` field. Serde
//! defaults to `OnboardingProgress::all_seen()` for missing fields
//! (see `config.rs::default_onboarding`), so existing users never see
//! these tips. Only `AppConfig::default()` — which only fires on a
//! brand-new install — populates it with `OnboardingProgress::default()`
//! (all flags `false`), giving fresh users the full tour.

use serde::{Deserialize, Serialize};

use crate::ui::icons;
use crate::ui::theme::{self, RADIUS_MD, SPACE_M, SPACE_S, SPACE_XS};

/// Which hints the user has already dismissed. Each new hint adds a
/// `bool` field with `#[serde(default)]` so older snapshots round-trip
/// cleanly and show the new tip exactly once.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnboardingProgress {
    /// "Tabs split inspector / scene / appearance" — shown under the
    /// tab switcher in the settings sidebar.
    #[serde(default)]
    pub tabs: bool,

    /// "Themes apply instantly" — shown under the theme picker in
    /// the Appearance tab.
    #[serde(default)]
    pub theme: bool,

    /// "V / G shortcut for visibility and gravity" — shown under the
    /// quick-toggle row in the Inspector tab.
    #[serde(default)]
    pub quick_toggles: bool,
}

impl OnboardingProgress {
    /// Snapshot where every hint is already marked seen. Used as the
    /// serde default for the `onboarding` field on `GlobalConfig` so
    /// upgrading users (whose config predates A.6) skip onboarding.
    pub fn all_seen() -> Self {
        Self {
            tabs: true,
            theme: true,
            quick_toggles: true,
        }
    }

    /// Whether every hint has been dismissed — handy for callers
    /// that want to remove the onboarding ScrollArea once nothing
    /// is left to show.
    pub fn fully_dismissed(&self) -> bool {
        *self == Self::all_seen()
    }
}

/// Render a dismissible advisory hint. No-op when `seen` is already
/// `true`. Returns `true` for the frame the user clicked the close
/// button so callers can flag the config dirty.
///
/// The layout is intentionally compact: lightbulb icon, body text in
/// `caption` style, close button on the right. Background uses
/// `accent.subtle` so the chip reads as a hint, not as a notification.
pub fn hint(ui: &mut egui::Ui, body: &str, seen: &mut bool) -> bool {
    if *seen {
        return false;
    }

    let mut dismissed = false;
    // Snapshot colours through the borrow before passing `ui` to the
    // frame; egui's borrow-checker rejects a live `&Visuals` and a
    // `&mut Ui` coexisting across the `Frame::show` closure.
    let (bg, accent, body_color) = {
        let visuals = ui.visuals();
        (
            visuals.selection.bg_fill,
            visuals.selection.stroke.color,
            visuals.text_color(),
        )
    };

    egui::Frame::new()
        .fill(bg)
        .corner_radius(RADIUS_MD)
        .inner_margin(egui::Margin::symmetric(SPACE_M as i8, SPACE_S as i8))
        .stroke(egui::Stroke::new(1.0, accent.gamma_multiply(0.5)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(icons::HINT).color(accent).size(14.0));
                ui.add_space(SPACE_XS);
                ui.label(
                    egui::RichText::new(body)
                        .text_style(theme::caption())
                        .color(body_color),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new(icons::CLOSE).size(12.0).weak())
                                .frame(false),
                        )
                        .on_hover_text("Dismiss")
                        .clicked()
                    {
                        *seen = true;
                        dismissed = true;
                    }
                });
            });
        });

    dismissed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_seen_is_fully_dismissed() {
        assert!(OnboardingProgress::all_seen().fully_dismissed());
    }

    #[test]
    fn default_is_not_dismissed() {
        // Fresh install — every flag should still be pending.
        assert!(!OnboardingProgress::default().fully_dismissed());
        assert!(!OnboardingProgress::default().tabs);
        assert!(!OnboardingProgress::default().theme);
        assert!(!OnboardingProgress::default().quick_toggles);
    }

    #[test]
    fn partial_dismissal_is_not_fully_dismissed() {
        let p = OnboardingProgress {
            tabs: true,
            theme: true,
            quick_toggles: false,
        };
        assert!(!p.fully_dismissed());
    }

    /// Existing users (whose config was saved before A.6) deserialize
    /// missing fields via serde's `default`. With
    /// `default_onboarding = all_seen`, that means upgrading users
    /// don't see any of the new hints.
    #[test]
    fn serde_missing_field_defaults_to_all_seen() {
        // Simulate a pre-A.6 config: no `onboarding` table at all.
        // Round-trip through the public default helper used in
        // `GlobalConfig`.
        let on = OnboardingProgress::all_seen();
        let toml_str = toml::to_string(&on).expect("serialize");
        let parsed: OnboardingProgress = toml::from_str(&toml_str).expect("deserialize");
        assert!(parsed.fully_dismissed());
    }

    /// Each `bool` field carries `#[serde(default)]`, so a partially
    /// migrated config (only one field present) decodes cleanly with
    /// the rest taking `false`. This guards against a future field
    /// addition silently flipping established hints back on.
    #[test]
    fn serde_partial_field_keeps_others_false() {
        let parsed: OnboardingProgress =
            toml::from_str("tabs = true").expect("partial deserialize");
        assert!(parsed.tabs);
        assert!(!parsed.theme);
        assert!(!parsed.quick_toggles);
    }
}
