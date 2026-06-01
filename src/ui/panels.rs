//! UI panels rendered through egui.
//!
//! Each function takes only the data it actually mutates plus `&egui::Context`.
//! This keeps `App` borrow-safe: the caller passes disjoint `&mut` references
//! to scene / selection / dirty flag instead of `&mut self`.

use crate::input::selection::SelectionState;
use crate::scene::Scene;

/// Right-side settings panel. Renders an inspector for the selected entity
/// plus a scene list. Mutations flow directly through the supplied mutable
/// references; `config_dirty` is set when anything changes so the existing
/// save-on-exit-edit-mode path picks them up.
pub fn settings(
    ctx: &egui::Context,
    scene: &mut Scene,
    selection: &mut SelectionState,
    config_dirty: &mut bool,
) {
    egui::SidePanel::right("anima_settings")
        .resizable(false)
        .default_width(280.0)
        .show(ctx, |ui| {
            ui.heading("Anima");
            ui.separator();

            // ── Selected entity inspector ─────────────────────────────────
            let selected_idx = selection.selected_index();
            if let Some(idx) = selected_idx {
                if let Some(entity) = scene.entities.get_mut(idx) {
                    let changed = entity_inspector(ui, entity);
                    if changed.any() {
                        *config_dirty = true;
                    }
                    if changed.touches_visibility_or_z_order {
                        scene.mark_visible_dirty();
                    }
                }
            } else {
                ui.label("Nothing selected.");
                ui.label("Click an entity or press Tab.");
            }

            ui.separator();

            // ── Scene list ────────────────────────────────────────────────
            ui.label("Entities");
            scene_list(ui, scene, selection, config_dirty);
        });
}

/// Tracks which fields of an entity were modified, so the caller can mark
/// the right caches dirty without scanning the entity afterwards.
#[derive(Default)]
struct EntityChange {
    any_field: bool,
    touches_visibility_or_z_order: bool,
}

impl EntityChange {
    fn any(&self) -> bool {
        self.any_field || self.touches_visibility_or_z_order
    }
}

fn entity_inspector(ui: &mut egui::Ui, entity: &mut crate::entity::Entity) -> EntityChange {
    let mut change = EntityChange::default();

    ui.label(format!("Selected: {}", entity.name));
    ui.label(
        egui::RichText::new(format!("id: {}", entity.id))
            .small()
            .weak(),
    );
    ui.add_space(4.0);

    // Position
    ui.label("Position");
    if ui
        .add(egui::Slider::new(&mut entity.x, -200.0..=4000.0).text("X"))
        .changed()
    {
        change.any_field = true;
    }
    if ui
        .add(egui::Slider::new(&mut entity.y, -200.0..=4000.0).text("Y"))
        .changed()
    {
        change.any_field = true;
    }

    // Appearance
    ui.add_space(6.0);
    ui.label("Appearance");
    if ui
        .add(egui::Slider::new(&mut entity.scale, 0.1..=5.0).text("Scale"))
        .changed()
    {
        change.any_field = true;
    }
    if ui
        .add(egui::Slider::new(&mut entity.opacity, 0.0..=1.0).text("Opacity"))
        .changed()
    {
        change.any_field = true;
    }

    // Animation
    ui.add_space(6.0);
    ui.label("Animation");
    let mut fps = entity.animation.fps;
    if ui
        .add(egui::Slider::new(&mut fps, 1.0..=60.0).text("FPS"))
        .changed()
    {
        entity.animation.set_fps(fps);
        change.any_field = true;
    }
    let mut playing = entity.animation.playing;
    if ui.checkbox(&mut playing, "Playing").changed() {
        entity.animation.playing = playing;
        change.any_field = true;
    }

    // Toggles
    ui.add_space(6.0);
    if ui.checkbox(&mut entity.visible, "Visible").changed() {
        change.touches_visibility_or_z_order = true;
    }
    let mut gravity = entity.physics.enabled;
    if ui.checkbox(&mut gravity, "Gravity (G)").changed() {
        if gravity {
            entity.physics.enable();
        } else {
            entity.physics.disable();
        }
        change.any_field = true;
    }

    // Z-order
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label("z-index");
        if ui
            .add(
                egui::DragValue::new(&mut entity.z_index)
                    .speed(1.0)
                    .range(-10_000..=10_000),
            )
            .changed()
        {
            change.touches_visibility_or_z_order = true;
        }
    });

    change
}

fn scene_list(
    ui: &mut egui::Ui,
    scene: &mut Scene,
    selection: &mut SelectionState,
    config_dirty: &mut bool,
) {
    // Hint about adding entities. File picker comes in a later phase.
    ui.label(
        egui::RichText::new("Drop a PNG / GIF / WebP onto the overlay to add one.")
            .small()
            .weak(),
    );
    ui.add_space(4.0);

    // Gather actions to apply *after* the loop so we don't hold a borrow
    // of scene.entities while we mutate the scene.
    let mut action: Option<ListAction> = None;

    egui::ScrollArea::vertical()
        .max_height(220.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for (idx, entity) in scene.entities.iter().enumerate() {
                let is_selected = selection.is_selected(idx);
                ui.horizontal(|ui| {
                    let label = if entity.visible {
                        entity.name.clone()
                    } else {
                        format!("{} (hidden)", entity.name)
                    };
                    if ui.selectable_label(is_selected, label).clicked() {
                        action = Some(ListAction::Select(idx));
                    }
                    // Small delete button on the right.
                    if ui.small_button("×").on_hover_text("Delete").clicked() {
                        action = Some(ListAction::Delete(idx));
                    }
                });
            }
        });

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
