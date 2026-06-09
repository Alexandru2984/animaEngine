//! Curated scene presets — gallery rendered inside the Scene tab.
//! Extracted in I.9.
//!
//! The gallery is a CollapsingHeader; each card is one `Preset` with
//! Append / Replace actions. `apply_preset` runs the diff and dispatches
//! to `Scene::reset_to_configs` (Replace) or repeated
//! `Scene::append_character_config` (Append, skipping duplicates).

use crate::i18n::t;
use crate::input::selection::SelectionState;
use crate::presets::{self, ApplyMode, Preset, PresetId};
use crate::scene::Scene;
use crate::ui::icons;
use crate::ui::theme::{self, h2, SPACE_M, SPACE_S, SPACE_XS};

pub(super) fn preset_gallery(
    ui: &mut egui::Ui,
    scene: &mut Scene,
    selection: &mut SelectionState,
    config_dirty: &mut bool,
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
                preset_card(ui, *id, scene, selection, config_dirty);
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
                    ui.label(egui::RichText::new(preset.name).strong().color(body_color));
                    ui.label(
                        egui::RichText::new(preset.description)
                            .text_style(theme::caption())
                            .weak(),
                    );
                });
            });
            ui.add_space(SPACE_XS);
            ui.horizontal(|ui| {
                if ui.button(t("scene-preset-append")).clicked() {
                    apply_preset(scene, selection, &preset, ApplyMode::Append);
                    *config_dirty = true;
                }
                let error_color = ui.visuals().error_fg_color;
                if ui
                    .button(egui::RichText::new(t("scene-preset-replace")).color(error_color))
                    .on_hover_text(t("scene-preset-replace-tooltip"))
                    .clicked()
                {
                    apply_preset(scene, selection, &preset, ApplyMode::Replace);
                    *config_dirty = true;
                }
            });
        });
}

fn apply_preset(
    scene: &mut Scene,
    selection: &mut SelectionState,
    preset: &Preset,
    mode: ApplyMode,
) {
    let existing = scene.to_character_configs();
    let new = presets::apply_to_scene(existing, preset, mode);
    if matches!(mode, ApplyMode::Replace) {
        scene.reset_to_configs(&new);
        selection.deselect();
    } else {
        // Append the suffixed preset characters that aren't already present.
        let already: std::collections::HashSet<&str> =
            scene.entities.iter().map(|e| e.id.as_str()).collect();
        let to_add: Vec<_> = new
            .iter()
            .filter(|c| !already.contains(c.id.as_str()))
            .cloned()
            .collect();
        for cfg in &to_add {
            if let Err(e) = scene.append_character_config(cfg) {
                tracing::warn!("Preset entity '{}' failed to append: {}", cfg.id, e);
            }
        }
    }
}
