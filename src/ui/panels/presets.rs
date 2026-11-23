//! Curated scene presets — gallery rendered inside the Scene tab.
//! Extracted in I.9.
//!
//! The gallery is a CollapsingHeader; each card is one `Preset` with
//! Append / Replace actions, which go through `presets::apply` as the
//! command palette's do.

use crate::i18n::{t, t_args};
use crate::input::selection::SelectionState;
use crate::presets::{self, ApplyMode, Preset, PresetId};
use crate::scene::Scene;
use crate::ui::accessible::AccessibleName;
use crate::ui::icons;
use crate::ui::theme::{self, h2, SPACE_M, SPACE_S, SPACE_XS};

pub(super) fn preset_gallery(
    ui: &mut egui::Ui,
    scene: &mut Scene,
    selection: &mut SelectionState,
    config_dirty: &mut bool,
    active_scene: &mut Option<String>,
    open: &mut bool,
) {
    let header = egui::RichText::new(format!("{}  {}", icons::SPARKLE, t("scene-presets-header")))
        .text_style(h2());
    let response = egui::CollapsingHeader::new(header)
        .id_salt("anima.scene.presets")
        .default_open(*open)
        .show(ui, |ui| {
            ui.add_space(SPACE_S);
            for id in PresetId::ALL {
                preset_card(ui, *id, scene, selection, config_dirty, active_scene);
                ui.add_space(SPACE_S);
            }
        });
    let visually_open = response.openness > 0.5;
    if visually_open != *open {
        *open = visually_open;
        *config_dirty = true;
    }
}

fn preset_card(
    ui: &mut egui::Ui,
    id: PresetId,
    scene: &mut Scene,
    selection: &mut SelectionState,
    config_dirty: &mut bool,
    active_scene: &mut Option<String>,
) {
    let preset = Preset::for_id(id);
    let (bg, accent, body_color) = {
        let v = ui.visuals();
        (v.faint_bg_color, v.hyperlink_color, v.text_color())
    };

    egui::Frame::new()
        .fill(bg)
        .corner_radius(theme::RADIUS_MD)
        .inner_margin(egui::Margin::symmetric(SPACE_M as i8, SPACE_S as i8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(preset.icon).size(18.0).color(accent));
                ui.add_space(SPACE_S);
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(&preset.name).strong().color(body_color));
                    ui.label(
                        egui::RichText::new(&preset.description)
                            .text_style(theme::caption())
                            .weak(),
                    );
                });
            });
            ui.add_space(SPACE_XS);
            // Every card has the same two buttons; a screen reader,
            // stepping through them, needs to hear which preset each is for.
            // The palette's rows already say exactly that.
            let mut args = fluent::FluentArgs::new();
            args.set("preset", preset.name.clone());
            ui.horizontal(|ui| {
                if ui
                    .button(t("scene-preset-append"))
                    .named(t_args("palette-append-row", &args))
                    .clicked()
                {
                    apply_preset(scene, selection, active_scene, &preset, ApplyMode::Append);
                    *config_dirty = true;
                }
                let error_color = ui.visuals().error_fg_color;
                if ui
                    .button(egui::RichText::new(t("scene-preset-replace")).color(error_color))
                    .named(t_args("palette-replace-row", &args))
                    .on_hover_text(t("scene-preset-replace-tooltip"))
                    .clicked()
                {
                    apply_preset(scene, selection, active_scene, &preset, ApplyMode::Replace);
                    *config_dirty = true;
                }
            });
        });
}

/// `presets::apply`, and what follows it here: after a "Replace" nothing
/// is selected, and the saved scene that was on is not on screen any
/// more (`GlobalConfig::active_scene`).
pub(super) fn apply_preset(
    scene: &mut Scene,
    selection: &mut SelectionState,
    active_scene: &mut Option<String>,
    preset: &Preset,
    mode: ApplyMode,
) {
    let applied = presets::apply(scene, preset, mode);
    if mode == ApplyMode::Replace {
        selection.deselect();
        *active_scene = None;
    }
    for e in &applied.failed {
        tracing::warn!(
            "Preset '{}': a character failed to append: {e}",
            preset.name
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// After "Replace" the saved scene that was on is not what is on
    /// screen: "Next scene" and the Scene tab's "Update" must not take it
    /// for that one. "Append" keeps it, as any edit does.
    #[test]
    fn replace_forgets_the_saved_scene_that_was_on() {
        let mut scene = Scene::from_config(&crate::config::AppConfig::default());
        let mut selection = SelectionState::default();
        let mut active = Some("Work".to_string());
        let preset = Preset::for_id(PresetId::CozyCompanion);
        apply_preset(
            &mut scene,
            &mut selection,
            &mut active,
            &preset,
            ApplyMode::Append,
        );
        assert_eq!(active.as_deref(), Some("Work"));
        apply_preset(
            &mut scene,
            &mut selection,
            &mut active,
            &preset,
            ApplyMode::Replace,
        );
        assert_eq!(active, None);
    }
}
