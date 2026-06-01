//! UI panels rendered through egui.
//!
//! Each function takes only the data it actually mutates plus `&egui::Context`.
//! This keeps `App` borrow-safe: the caller passes disjoint `&mut` references
//! to scene / selection / dirty flag instead of `&mut self`.

use crate::app::ContextMenuState;
use crate::behavior::Behavior;
use crate::constants::TOGGLE_BUTTON_SIZE;
use crate::input::selection::SelectionState;
use crate::scene::Scene;
use crate::ui::toasts::{ToastKind, ToastQueue};

/// Entity-targeted action requested from the right-click context menu.
/// `App` applies it after `EguiRenderer::render` returns so it can grab a
/// mutable borrow on the renderer for texture management (Duplicate, Delete).
pub enum MenuAction {
    Duplicate(usize),
    Delete(usize),
    ResetTransform(usize),
    ToggleGravity(usize),
    BringForward(usize),
    SendBackward(usize),
}

/// What `context_menu` decided about its own state for this frame.
pub enum ContextMenuOutcome {
    /// Menu remains visible — nothing happened this frame.
    Open,
    /// User dismissed the menu (clicked outside).
    Close,
    /// User picked an action — caller should apply it and close the menu.
    Action(MenuAction),
}

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

    // Behavior
    ui.add_space(6.0);
    ui.label("Behavior");
    if behavior_picker(ui, &mut entity.behavior) {
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

/// Behavior dropdown + variant-specific sliders. Returns `true` when the
/// user touched anything in this section.
fn behavior_picker(ui: &mut egui::Ui, behavior: &mut Behavior) -> bool {
    let mut changed = false;

    // ComboBox with the three concrete variants. selectable_value compares
    // via PartialEq, so picking the same variant a second time is a no-op.
    let current_label = behavior_label(behavior);
    egui::ComboBox::from_id_salt("behavior_picker")
        .selected_text(current_label)
        .show_ui(ui, |ui| {
            let prev = behavior.clone();
            ui.selectable_value(behavior, Behavior::Idle, "Idle");
            ui.selectable_value(
                behavior,
                Behavior::WalkAround { speed: 60.0 },
                "Walk around",
            );
            ui.selectable_value(
                behavior,
                Behavior::FollowCursor {
                    speed: 240.0,
                    comfort_distance: 80.0,
                },
                "Follow cursor",
            );
            if *behavior != prev {
                changed = true;
            }
        });

    // Variant-specific sliders.
    match behavior {
        Behavior::Idle => {}
        Behavior::WalkAround { speed } => {
            if ui
                .add(egui::Slider::new(speed, 10.0..=400.0).text("Speed (px/s)"))
                .changed()
            {
                changed = true;
            }
        }
        Behavior::FollowCursor {
            speed,
            comfort_distance,
        } => {
            if ui
                .add(egui::Slider::new(speed, 50.0..=800.0).text("Speed (px/s)"))
                .changed()
            {
                changed = true;
            }
            if ui
                .add(egui::Slider::new(comfort_distance, 0.0..=400.0).text("Comfort distance (px)"))
                .changed()
            {
                changed = true;
            }
        }
    }

    changed
}

fn behavior_label(b: &Behavior) -> &'static str {
    match b {
        Behavior::Idle => "Idle",
        Behavior::WalkAround { .. } => "Walk around",
        Behavior::FollowCursor { .. } => "Follow cursor",
    }
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

/// Floating right-click context menu anchored at `state.pos`. Caller
/// owns the `ContextMenuState`; this function only inspects it and
/// reports back via `ContextMenuOutcome`.
pub(crate) fn context_menu(ctx: &egui::Context, state: &ContextMenuState) -> ContextMenuOutcome {
    let idx = state.entity_idx;
    let mut picked: Option<MenuAction> = None;

    let area = egui::Area::new(egui::Id::new("anima_entity_context_menu"))
        .fixed_pos(state.pos)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(160.0);

                if ui.button("Duplicate").clicked() {
                    picked = Some(MenuAction::Duplicate(idx));
                }
                if ui.button("Reset transform").clicked() {
                    picked = Some(MenuAction::ResetTransform(idx));
                }
                if ui.button("Toggle gravity").clicked() {
                    picked = Some(MenuAction::ToggleGravity(idx));
                }
                ui.separator();
                if ui.button("Bring forward").clicked() {
                    picked = Some(MenuAction::BringForward(idx));
                }
                if ui.button("Send backward").clicked() {
                    picked = Some(MenuAction::SendBackward(idx));
                }
                ui.separator();
                if ui
                    .button(egui::RichText::new("Delete").color(egui::Color32::LIGHT_RED))
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

/// Stack of toast notifications anchored to the bottom-right corner.
/// Renders above the settings panel and the context menu.
pub fn toasts(ctx: &egui::Context, queue: &ToastQueue) {
    if queue.is_empty() {
        return;
    }

    // While there are visible toasts, drive continuous repaints so they
    // disappear at the moment they expire (without waiting for the next
    // input event).
    ctx.request_repaint();

    egui::Area::new(egui::Id::new("anima_toasts"))
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-12.0, -12.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                for toast in queue.iter() {
                    let (bg, fg) = match toast.kind {
                        ToastKind::Info => {
                            (egui::Color32::from_rgb(40, 40, 45), egui::Color32::WHITE)
                        }
                        ToastKind::Success => {
                            (egui::Color32::from_rgb(30, 100, 50), egui::Color32::WHITE)
                        }
                        ToastKind::Warn => {
                            (egui::Color32::from_rgb(150, 110, 30), egui::Color32::WHITE)
                        }
                        ToastKind::Error => {
                            (egui::Color32::from_rgb(140, 40, 40), egui::Color32::WHITE)
                        }
                    };

                    egui::Frame::new()
                        .fill(bg)
                        .corner_radius(4.0)
                        .inner_margin(egui::Margin::symmetric(10, 6))
                        .show(ui, |ui| {
                            ui.colored_label(fg, &toast.message);
                        });
                    ui.add_space(4.0);
                }
            });
        });
}

/// Top-right ⚙ button that toggles between pass-through and edit mode.
/// Returns `true` for the frame the user clicked it.
///
/// Geometry must match `TOGGLE_BUTTON_SIZE` because the X11 input shape
/// in pass-through mode uses the same constant to decide which pixels
/// receive clicks.
pub fn toggle_button(ctx: &egui::Context, edit_mode: bool) -> bool {
    let size = TOGGLE_BUTTON_SIZE as f32;
    let screen = ctx.screen_rect();
    let pos = egui::pos2(screen.right() - size, 0.0);

    let bg = if edit_mode {
        egui::Color32::from_rgb(40, 160, 60) // active = green
    } else {
        egui::Color32::from_rgba_unmultiplied(50, 50, 60, 200) // pass-through = dim
    };
    let tooltip = if edit_mode {
        "Exit edit mode"
    } else {
        "Enter edit mode"
    };

    let mut clicked = false;
    egui::Area::new(egui::Id::new("anima_toggle_button"))
        .fixed_pos(pos)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let response = ui
                .add_sized(
                    egui::vec2(size, size),
                    egui::Button::new(
                        egui::RichText::new("⚙")
                            .size(28.0)
                            .color(egui::Color32::WHITE),
                    )
                    .fill(bg)
                    .corner_radius(0.0),
                )
                .on_hover_text(tooltip);
            if response.clicked() {
                clicked = true;
            }
        });
    clicked
}
