//! Empty / error / loading state helpers — the live implementation of
//! `docs/design-system.md` §8.
//!
//! Two exports:
//!
//! - [`empty`] — centered icon + headline + hint, for panels with no
//!   data yet ("nothing selected", "empty scene")
//! - [`error`] — same shape but the icon is the design-system error
//!   tone and an optional action button sits at the bottom (retry, etc.)
//!
//! A `spinner` used to live here too, for "loading" states. It never
//! acquired a caller and was removed; `docs/design-system.md` §8 still
//! describes the pattern if one is wanted again.
//!
//! Keeping these in a single module means every "we have nothing to
//! show" branch in the UI ends up looking the same; readers can
//! `grep states::` to find every such branch.

use crate::ui::theme::{self, h2, SPACE_2XL, SPACE_M, SPACE_XS};

/// Empty-state card. Renders centered in whatever container `ui` is.
///
/// Use for "nothing selected" / "no entities" / "no results" — places
/// where the absence of data is *expected* and the user just needs
/// guidance toward the next action.
pub fn empty(ui: &mut egui::Ui, icon: &str, headline: &str, hint: &str) {
    let _ = empty_with_action(ui, icon, headline, hint, None);
}

/// Same as [`empty`] but with an optional CTA button at the bottom.
/// Returns `true` for the frame the user clicked the button so the
/// caller can route the action (insert a demo preset, mkdir the
/// asset root, etc.). When `action_label` is `None`, the behaviour
/// is identical to `empty`.
#[must_use]
pub fn empty_with_action(
    ui: &mut egui::Ui,
    icon: &str,
    headline: &str,
    hint: &str,
    action_label: Option<&str>,
) -> bool {
    ui.add_space(SPACE_2XL);
    let mut clicked = false;
    ui.vertical_centered(|ui| {
        ui.label(
            egui::RichText::new(icon)
                .size(40.0)
                .color(ui.visuals().weak_text_color()),
        );
        ui.add_space(SPACE_M);
        ui.label(egui::RichText::new(headline).text_style(h2()));
        ui.add_space(SPACE_XS);
        ui.label(
            egui::RichText::new(hint)
                .text_style(theme::caption())
                .weak(),
        );
        if let Some(label) = action_label {
            ui.add_space(SPACE_M);
            if ui.button(label).clicked() {
                clicked = true;
            }
        }
    });
    clicked
}

/// Error-state card. Same shape as [`empty`] but tinted with the active
/// theme's error tone and optionally accompanied by an action button.
///
/// Returns `true` for the frame the user clicked the action button —
/// `false` (or always `false` when `action_label` is `None`).
///
/// Use for asset-load failures, decode errors, "couldn't reach disk
/// cache" — places where the user needs to know *something went wrong*
/// and how to recover.
#[must_use]
pub fn error(
    ui: &mut egui::Ui,
    icon: &str,
    headline: &str,
    detail: &str,
    action_label: Option<&str>,
) -> bool {
    ui.add_space(SPACE_2XL);
    let mut clicked = false;
    ui.vertical_centered(|ui| {
        let error_color = ui.visuals().error_fg_color;
        ui.label(egui::RichText::new(icon).size(40.0).color(error_color));
        ui.add_space(SPACE_M);
        ui.label(
            egui::RichText::new(headline)
                .text_style(h2())
                .color(error_color),
        );
        ui.add_space(SPACE_XS);
        ui.label(
            egui::RichText::new(detail)
                .text_style(theme::caption())
                .weak(),
        );
        if let Some(label) = action_label {
            ui.add_space(SPACE_M);
            if ui.button(label).clicked() {
                clicked = true;
            }
        }
    });
    clicked
}
