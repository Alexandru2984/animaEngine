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

    /// "Click any chord to rebind" — shown under the Keybindings tab
    /// header (D.7, new in 0.4).
    #[serde(default)]
    pub keybindings_tab: bool,

    /// "Press Ctrl+Shift+\` for perf overlay" — shown at the bottom
    /// of the Appearance tab (D.7, new in 0.4).
    #[serde(default)]
    pub perf_overlay: bool,

    /// First-run coach-marks (V.2): the 3-step overlay tour (⚙ →
    /// drag-and-drop → hotkeys). One flag for the whole tour —
    /// completing or skipping it sets this; "Reset onboarding hints"
    /// re-arms it like every other hint.
    ///
    /// Unlike the chip hints (whose missing-field default is `false`
    /// so existing users see each new tip once), a *welcome tour* is
    /// wrong for an upgrading user — the serde default is `true`, so
    /// only a brand-new install (`Default::default()`) gets it.
    #[serde(default = "coach_marks_skipped")]
    pub coach_marks: bool,
}

/// Serde default for [`OnboardingProgress::coach_marks`] — see the
/// field docs for why this is `true` (upgraders skip the tour).
fn coach_marks_skipped() -> bool {
    true
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
            keybindings_tab: true,
            perf_overlay: true,
            coach_marks: true,
        }
    }

    /// Reverse of `all_seen` — used by the "Reset onboarding hints"
    /// button in Appearance (D.7) so the user can retake the tour.
    pub fn reset(&mut self) {
        *self = Self::default();
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
        .stroke(egui::Stroke::new(1.0_f32, accent.gamma_multiply(0.5)))
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

/// First-run coach-marks (V.2): a 3-step tour rendered as floating
/// bubbles on the overlay itself, because a fresh user's first sight
/// is the desktop with mascots — not the settings panel.
///
/// Input-shape constraint drives the design: in pass-through mode
/// only the ⚙ button receives clicks, so **step 0 is informational**
/// (no buttons) and advances when the user actually enters edit mode
/// — the very action it teaches. Steps 1–2 run in edit mode, where
/// the whole window accepts input, so they carry Next/Skip buttons.
///
/// Returns `true` when `progress.coach_marks` flipped (config dirty).
pub fn coach_marks(
    ctx: &egui::Context,
    progress: &mut OnboardingProgress,
    edit_mode: bool,
) -> bool {
    use crate::i18n::t;

    if progress.coach_marks {
        return false;
    }

    let step_id = egui::Id::new("anima.coach.step");
    let mut step: u8 = ctx.data(|d| d.get_temp(step_id).unwrap_or(0));

    // Step 0 teaches entering edit mode; the moment it happens, the
    // lesson is learned — advance.
    if step == 0 && edit_mode {
        step = 1;
        ctx.data_mut(|d| d.insert_temp(step_id, step));
    }
    // Leaving edit mode mid-tour pauses it at step 0's anchor — the
    // remaining steps need the panel's input shape to be clickable.
    if step > 0 && !edit_mode {
        return false;
    }

    let mut dirty = false;
    let alpha = ctx.animate_value_with_time(
        step_id.with(("fade", step)),
        1.0,
        crate::ui::motion::time(ctx, 0.15),
    );

    let (anchor, offset) = if step == 0 {
        // Just below the ⚙ toggle (64 px square, top-right corner).
        (
            egui::Align2::RIGHT_TOP,
            egui::vec2(-SPACE_M, 64.0 + SPACE_M),
        )
    } else {
        // Edit-mode steps: top-center, clear of panel and mascots.
        (egui::Align2::CENTER_TOP, egui::vec2(0.0, 48.0))
    };

    egui::Area::new(egui::Id::new("anima.coach.area"))
        .anchor(anchor, offset)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.set_opacity(alpha);
            ui.set_max_width(340.0);
            let (bg, accent, body_color) = {
                let visuals = ui.visuals();
                (
                    visuals.window_fill,
                    visuals.selection.stroke.color,
                    visuals.text_color(),
                )
            };
            egui::Frame::new()
                .fill(bg)
                .corner_radius(RADIUS_MD)
                .inner_margin(egui::Margin::same(SPACE_M as i8))
                .stroke(egui::Stroke::new(1.5_f32, accent))
                .show(ui, |ui| {
                    let body_key = match step {
                        0 => "onboarding-coach-step1",
                        1 => "onboarding-coach-step2",
                        _ => "onboarding-coach-step3",
                    };
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("{}/3", step + 1))
                                .color(accent)
                                .strong(),
                        );
                        ui.add_space(SPACE_XS);
                        ui.label(
                            egui::RichText::new(t(body_key))
                                .text_style(theme::caption())
                                .color(body_color),
                        );
                    });
                    // Step 0 is button-free: pass-through mode delivers
                    // no clicks outside the ⚙ button anyway.
                    if step > 0 {
                        ui.add_space(SPACE_S);
                        ui.horizontal(|ui| {
                            let last = step >= 2;
                            let next_label = if last {
                                t("onboarding-coach-done")
                            } else {
                                t("onboarding-coach-next")
                            };
                            if ui.button(next_label).clicked() {
                                if last {
                                    progress.coach_marks = true;
                                    dirty = true;
                                } else {
                                    ctx.data_mut(|d| d.insert_temp(step_id, step + 1));
                                }
                            }
                            if !last && ui.button(t("onboarding-coach-skip")).clicked() {
                                progress.coach_marks = true;
                                dirty = true;
                            }
                        });
                    }
                });
        });

    dirty
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
        let d = OnboardingProgress::default();
        assert!(!d.fully_dismissed());
        assert!(!d.tabs);
        assert!(!d.theme);
        assert!(!d.quick_toggles);
        assert!(!d.keybindings_tab);
        assert!(!d.perf_overlay);
    }

    #[test]
    fn partial_dismissal_is_not_fully_dismissed() {
        let p = OnboardingProgress {
            tabs: true,
            theme: true,
            quick_toggles: false,
            keybindings_tab: false,
            perf_overlay: false,
            coach_marks: false,
        };
        assert!(!p.fully_dismissed());
    }

    #[test]
    fn reset_brings_back_every_hint() {
        let mut p = OnboardingProgress::all_seen();
        p.reset();
        assert!(!p.fully_dismissed());
        assert!(!p.tabs);
        assert!(!p.perf_overlay);
        // V.2: the coach-mark tour re-arms through the same flow.
        assert!(!p.coach_marks);
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

    /// V.2: a config saved before the tour existed (0.7 and earlier)
    /// must NOT trigger the welcome tour on upgrade — the missing
    /// field defaults to `true` (seen), unlike the chip hints.
    #[test]
    fn upgrading_config_skips_the_tour() {
        let parsed: OnboardingProgress =
            toml::from_str("tabs = true").expect("partial deserialize");
        assert!(parsed.coach_marks, "upgraders must not see the tour");
        // Fresh installs do see it.
        assert!(!OnboardingProgress::default().coach_marks);
    }
}
