//! Scene tab — monitor distribution, entity list, preset gallery,
//! groups summary. Extracted in I.8.
//!
//! The `scene_list` selection pulse and the read-only groups summary
//! live here too since they're only used by this tab.

use super::monitor::{monitor_mode_picker, pulse_alpha_at};
use super::presets::preset_gallery;
use crate::i18n::t;
use crate::input::selection::SelectionState;
use crate::monitor::{MonitorInfo, MonitorMode};
use crate::scene::Scene;
use crate::ui::collapse::CollapseState;
use crate::ui::icons;
use crate::ui::states;
use crate::ui::theme::{self, h2, SPACE_L, SPACE_M, SPACE_S};

// UI plumbing fans out one settings struct into per-tab params — same
// allow as `panels::settings` itself.
#[allow(clippy::too_many_arguments)]
pub(super) fn scene_tab(
    ui: &mut egui::Ui,
    scene: &mut Scene,
    selection: &mut SelectionState,
    config_dirty: &mut bool,
    monitor_mode: &mut MonitorMode,
    window_awareness: &mut bool,
    monitors: &[MonitorInfo],
    collapse_state: &mut CollapseState,
) {
    // ── Monitor distribution section ─────────────────────────────────
    monitor_mode_picker(ui, monitor_mode, monitors, config_dirty);

    // ── Window awareness (X11) ────────────────────────────────────────
    ui.add_space(SPACE_S);
    if ui
        .checkbox(window_awareness, t("scene-window-awareness"))
        .on_hover_text(t("scene-window-awareness-tooltip"))
        .changed()
    {
        *config_dirty = true;
    }
    ui.add_space(SPACE_L);
    ui.separator();
    ui.add_space(SPACE_M);

    let is_empty = scene.entities.is_empty();

    if is_empty {
        // D.8: zero-config CTA — open the preset gallery so a fresh
        // install can land in a curated scene with one click.
        if states::empty_with_action(
            ui,
            icons::GHOST,
            &t("scene-empty-headline"),
            &t("scene-empty-hint"),
            Some(&t("scene-empty-action-browse-presets")),
        ) {
            collapse_state.scene_presets = true;
            *config_dirty = true;
        }
    } else {
        ui.label(
            egui::RichText::new(t("scene-drop-hint"))
                .text_style(theme::caption())
                .weak(),
        );
        ui.add_space(SPACE_M);
        scene_list(ui, scene, selection, config_dirty);
        ui.add_space(SPACE_L);
        ui.separator();
    }

    ui.add_space(SPACE_M);
    preset_gallery(
        ui,
        scene,
        selection,
        config_dirty,
        &mut collapse_state.scene_presets,
    );

    if !scene.groups.is_empty() {
        ui.add_space(SPACE_L);
        ui.separator();
        ui.add_space(SPACE_M);
        groups_section(ui, scene);
    }
}

/// Read-only summary of sprite groups (C.8). Edits go through
/// `config.toml` hand-editing for now; full inline edit lands with
/// the C.9 polish that also wires up offset/scale composition in
/// the renderer.
fn groups_section(ui: &mut egui::Ui, scene: &Scene) {
    ui.label(egui::RichText::new(format!("{}  Groups", icons::STACK)).text_style(h2()));
    ui.add_space(SPACE_S);
    let body_color = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    for group in &scene.groups {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(&group.name).strong().color(body_color));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let visibility_marker = if group.visible { "" } else { " · hidden" };
                let count = group.member_ids.len();
                let plural = if count == 1 { "entity" } else { "entities" };
                ui.label(
                    egui::RichText::new(format!("{count} {plural}{visibility_marker}"))
                        .text_style(theme::caption())
                        .color(weak),
                );
            });
        });
    }
}

fn scene_list(
    ui: &mut egui::Ui,
    scene: &mut Scene,
    selection: &mut SelectionState,
    config_dirty: &mut bool,
) {
    // Gather actions to apply *after* the loop so we don't hold a borrow
    // of scene.entities while we mutate the scene.
    let mut action: Option<ListAction> = None;

    // Selection pulse — design-system §6, sine 2s cycle, low amplitude.
    // We paint a subtle accent stripe at the left of the selected row
    // after the row itself has been laid out, so a keyboard-only user
    // can spot which row Tab landed on without scanning opacity / weight
    // differences.
    let now = ui.ctx().input(|i| i.time);
    let pulse_alpha = pulse_alpha_at(now);
    if selection.selected_index().is_some() {
        ui.ctx().request_repaint();
    }
    let accent = ui.visuals().selection.stroke.color;
    let delete_tooltip = t("menu-delete");

    for (idx, entity) in scene.entities.iter().enumerate() {
        let is_selected = selection.is_selected(idx);
        let row_response = ui.horizontal(|ui| {
            let label = if entity.visible {
                entity.name.clone()
            } else {
                format!("{}  {}", icons::HIDDEN, entity.name)
            };
            if ui.selectable_label(is_selected, label).clicked() {
                action = Some(ListAction::Select(idx));
            }
            if ui
                .small_button(icons::TRASH)
                .on_hover_text(&delete_tooltip)
                .clicked()
            {
                action = Some(ListAction::Delete(idx));
            }
        });
        if is_selected {
            let rect = row_response.response.rect;
            let stripe = egui::Rect::from_min_max(
                rect.left_top(),
                egui::pos2(rect.left() + 3.0, rect.bottom()),
            );
            ui.painter()
                .rect_filled(stripe, 1.5, accent.gamma_multiply(pulse_alpha));
        }
    }

    match action {
        Some(ListAction::Select(idx)) => {
            selection.select(idx);
        }
        Some(ListAction::Delete(idx)) if scene.remove_entity(idx).is_some() => {
            selection.deselect();
            *config_dirty = true;
        }
        _ => {}
    }
}

enum ListAction {
    Select(usize),
    Delete(usize),
}
