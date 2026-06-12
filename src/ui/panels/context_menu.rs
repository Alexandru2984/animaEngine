//! Right-click context menu. Extracted in I.3.
//!
//! Caller owns `ContextMenuState` (which entity, where to draw).
//! We only inspect it and report back via `ContextMenuOutcome` so
//! `App` can decide whether to keep the menu open, dismiss it, or
//! dispatch the picked action.

use super::{ContextMenuOutcome, MenuAction};
use crate::app::ContextMenuState;
use crate::ui::icons;

pub(crate) fn context_menu(ctx: &egui::Context, state: &ContextMenuState) -> ContextMenuOutcome {
    let idx = state.entity_idx;
    let mut picked: Option<MenuAction> = None;

    let area = egui::Area::new(egui::Id::new("anima_entity_context_menu"))
        .fixed_pos(state.pos)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(160.0);

                if ui
                    .button(format!(
                        "{}  {}",
                        icons::COPY,
                        crate::i18n::t("menu-duplicate")
                    ))
                    .clicked()
                {
                    picked = Some(MenuAction::Duplicate(idx));
                }
                if ui
                    .button(format!(
                        "{}  {}",
                        icons::RESET,
                        crate::i18n::t("menu-reset-transform")
                    ))
                    .clicked()
                {
                    picked = Some(MenuAction::ResetTransform(idx));
                }
                if ui
                    .button(format!(
                        "{}  {}",
                        icons::GRAVITY,
                        crate::i18n::t("menu-toggle-gravity")
                    ))
                    .clicked()
                {
                    picked = Some(MenuAction::ToggleGravity(idx));
                }
                ui.separator();
                if ui
                    .button(format!(
                        "{}  {}",
                        icons::BRING_FORWARD,
                        crate::i18n::t("menu-bring-forward")
                    ))
                    .clicked()
                {
                    picked = Some(MenuAction::BringForward(idx));
                }
                if ui
                    .button(format!(
                        "{}  {}",
                        icons::SEND_BACKWARD,
                        crate::i18n::t("menu-send-backward")
                    ))
                    .clicked()
                {
                    picked = Some(MenuAction::SendBackward(idx));
                }
                ui.separator();
                let error_color = ui.visuals().error_fg_color;
                if ui
                    .button(
                        egui::RichText::new(format!(
                            "{}  {}",
                            icons::TRASH,
                            crate::i18n::t("menu-delete")
                        ))
                        .color(error_color),
                    )
                    .clicked()
                {
                    picked = Some(MenuAction::Delete(idx));
                }
            });
        });

    if let Some(action) = picked {
        return ContextMenuOutcome::Action(action);
    }

    // Dismiss when the user clicks anywhere that isn't the menu itself, or
    // presses Escape. We deliberately check `any_click` (not `pressed`) so
    // a release that ended on a button still counts as "clicked the menu".
    let dismissed = ctx.input(|i| {
        let escape = i.key_pressed(egui::Key::Escape);
        let outside_click = i.pointer.any_click() && !area.response.contains_pointer();
        escape || outside_click
    });

    if dismissed {
        ContextMenuOutcome::Close
    } else {
        ContextMenuOutcome::Open
    }
}
