//! "What's new" highlight panel (D.7).
//!
//! Shows once per minor-version bump at the top of the settings
//! sidebar — just below the warning banners, above the tab switcher.
//! Dismissed with a small ✕; clicking it stamps the current version
//! into `last_seen_whats_new`, so the panel won't reappear on the
//! next session unless the user resets onboarding.
//!
//! The version key is bumped manually each release that introduces
//! a "What's new" surface (see `WHATS_NEW_VERSION` below). Pre-0.4
//! configs with no `last_seen_whats_new` field default through serde
//! to `None`, which triggers the panel exactly once — desired.

use crate::ui::icons;
use crate::ui::theme::{self, RADIUS_MD, SPACE_M, SPACE_S, SPACE_XS};

/// Anchor for "what's new" content the panel shows. Defined in
/// `constants` so the config loader can stamp it on a fresh install.
pub use crate::constants::WHATS_NEW_VERSION;

/// One highlight row inside the panel.
struct Highlight {
    icon: &'static str,
    /// i18n key for the highlight body.
    body_key: &'static str,
}

const HIGHLIGHTS: &[Highlight] = &[
    Highlight {
        icon: icons::BEHAVIOR_FOLLOW,
        body_key: "whats-new-poke",
    },
    Highlight {
        icon: icons::GHOST,
        body_key: "whats-new-startle",
    },
    // Not `CURSOR`: it reads almost identically to the `CURSOR_CLICK`
    // behind the poke row two lines up. Cardinal arrows say "drag".
    Highlight {
        icon: icons::BEHAVIOR_WANDER,
        body_key: "whats-new-wayland-drag",
    },
    Highlight {
        icon: icons::KEYBOARD,
        body_key: "whats-new-shortcuts",
    },
];

/// Decide whether to render the panel: only when the user hasn't
/// stamped the current `WHATS_NEW_VERSION` into their config.
pub fn should_show(last_seen: Option<&str>) -> bool {
    last_seen != Some(WHATS_NEW_VERSION)
}

/// Render the panel and update `last_seen` if the user dismissed
/// it. Returns `true` for the frame the user dismissed so the
/// caller can flag the config dirty.
pub fn show(ui: &mut egui::Ui, last_seen: &mut Option<String>) -> bool {
    if !should_show(last_seen.as_deref()) {
        return false;
    }
    let mut dismissed = false;
    let (bg, accent, body_color) = {
        let v = ui.visuals();
        (
            v.selection.bg_fill,
            v.selection.stroke.color,
            v.text_color(),
        )
    };
    egui::Frame::new()
        .fill(bg)
        .corner_radius(RADIUS_MD)
        .inner_margin(egui::Margin::same(SPACE_M as i8))
        .stroke(egui::Stroke::new(1.0_f32, accent.gamma_multiply(0.5)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(crate::i18n::t("whats-new-header"))
                        .text_style(theme::caption())
                        .strong()
                        .color(accent),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new(icons::CLOSE).size(12.0).weak())
                                .frame(false),
                        )
                        .on_hover_text(crate::i18n::t("onboarding-dismiss"))
                        .clicked()
                    {
                        *last_seen = Some(WHATS_NEW_VERSION.to_string());
                        dismissed = true;
                    }
                });
            });
            ui.add_space(SPACE_XS);
            for h in HIGHLIGHTS {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(h.icon).color(accent).size(14.0));
                    ui.add_space(SPACE_XS);
                    // Must wrap. A plain label asks for its natural width,
                    // and a `SidePanel` grows to satisfy its content — so
                    // the longest highlight string silently decided how
                    // wide the whole settings panel was. Measured: 320px
                    // (the configured width) with no panel, 424px with the
                    // old copy, 593px with longer copy.
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(crate::i18n::t(h.body_key))
                                .text_style(theme::caption())
                                .color(body_color),
                        )
                        .wrap(),
                    );
                });
                ui.add_space(SPACE_XS);
            }
        });
    ui.add_space(SPACE_S);
    dismissed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_show_when_never_seen() {
        assert!(should_show(None));
    }

    #[test]
    fn should_show_when_older_version_seen() {
        assert!(should_show(Some("0.3.2")));
    }

    #[test]
    fn should_hide_when_current_version_seen() {
        assert!(!should_show(Some(WHATS_NEW_VERSION)));
    }
}
