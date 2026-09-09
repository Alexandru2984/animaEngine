//! Keyboard actions that both backends can run.
//!
//! These eighteen touch only the scene, the selection and the dirty flag —
//! nothing about a window, a renderer or an event loop — so they belong to
//! neither backend in particular.
//!
//! They live here because they were living in `app::dispatch` instead,
//! reachable only from the winit path. The native Wayland loop consulted
//! the keybinding table in one place and matched one action, so of the
//! twenty-eight rebindable actions the Keybindings tab offers, twenty-seven
//! did nothing there: the user could rebind them and watch the config
//! persist while nudge, delete, cycle and the rest stayed inert (R19 in
//! docs/runtime-findings.md).
//!
//! The remaining ten stay per-backend because they genuinely differ —
//! quitting, saving, duplicating through the renderer's texture cache,
//! centring against a window, the perf overlay.

use crate::input::selection::SelectionState;
use crate::keybindings::Action;
use crate::scene::Scene;

/// Run `action` if it is one of the backend-independent ones.
///
/// Returns `false` when the action is not handled here, so a caller can
/// fall through to its own arms rather than silently swallowing it.
pub fn dispatch_shared(
    action: Action,
    scene: &mut Scene,
    selection: &mut SelectionState,
    config_dirty: &mut bool,
    shift_held: bool,
) -> bool {
    match action {
        Action::PauseAll => {
            scene.toggle_global_playback();
            *config_dirty = true;
        }
        Action::NudgeUp => {
            if let Some(idx) = selection.selected_index() {
                let step = if shift_held { 1.0 } else { 10.0 };
                if let Some(entity) = scene.entities.get_mut(idx) {
                    entity.y -= step;
                    entity.behavior_state.bounce_invalidate();
                    *config_dirty = true;
                }
            }
        }
        Action::NudgeDown => {
            if let Some(idx) = selection.selected_index() {
                let step = if shift_held { 1.0 } else { 10.0 };
                if let Some(entity) = scene.entities.get_mut(idx) {
                    entity.y += step;
                    entity.behavior_state.bounce_invalidate();
                    *config_dirty = true;
                }
            }
        }
        Action::NudgeLeft => {
            if let Some(idx) = selection.selected_index() {
                let step = if shift_held { 1.0 } else { 10.0 };
                if let Some(entity) = scene.entities.get_mut(idx) {
                    entity.x -= step;
                    entity.behavior_state.bounce_invalidate();
                    *config_dirty = true;
                }
            }
        }
        Action::NudgeRight => {
            if let Some(idx) = selection.selected_index() {
                let step = if shift_held { 1.0 } else { 10.0 };
                if let Some(entity) = scene.entities.get_mut(idx) {
                    entity.x += step;
                    entity.behavior_state.bounce_invalidate();
                    *config_dirty = true;
                }
            }
        }
        Action::ResetTransform => {
            if let Some(idx) = selection.selected_index() {
                if let Some(entity) = scene.entities.get_mut(idx) {
                    entity.scale = 1.0;
                    entity.opacity = 1.0;
                    tracing::info!("Reset '{}' scale=1.0, opacity=1.0", entity.name);
                    *config_dirty = true;
                }
            }
        }
        Action::OpacityUp => {
            if let Some(idx) = selection.selected_index() {
                if let Some(entity) = scene.entities.get_mut(idx) {
                    entity.opacity = (entity.opacity + 0.1).min(1.0);
                    tracing::info!("Opacity: {:.0}%", entity.opacity * 100.0);
                    *config_dirty = true;
                }
            }
        }
        Action::OpacityDown => {
            if let Some(idx) = selection.selected_index() {
                if let Some(entity) = scene.entities.get_mut(idx) {
                    entity.opacity = (entity.opacity - 0.1).max(0.05);
                    tracing::info!("Opacity: {:.0}%", entity.opacity * 100.0);
                    *config_dirty = true;
                }
            }
        }
        Action::ToggleVisible => {
            if let Some(idx) = selection.selected_index() {
                if let Some(entity) = scene.entities.get_mut(idx) {
                    entity.visible = !entity.visible;
                    tracing::info!(
                        "Entity '{}' visibility: {}",
                        entity.name,
                        if entity.visible { "visible" } else { "hidden" }
                    );
                    scene.mark_visible_dirty();
                    *config_dirty = true;
                }
            }
        }
        // Gravity: off by default — entity stays put. Toggling on
        // makes it fall from its current position; off pins it.
        Action::ToggleGravity => {
            if let Some(idx) = selection.selected_index() {
                if let Some(entity) = scene.entities.get_mut(idx) {
                    entity.physics.toggle();
                    tracing::info!(
                        "Entity '{}' gravity: {}",
                        entity.name,
                        if entity.physics.enabled {
                            "ON (falling)"
                        } else {
                            "OFF (pinned)"
                        }
                    );
                    *config_dirty = true;
                }
            }
        }
        Action::TogglePlayback => {
            if let Some(idx) = selection.selected_index() {
                if let Some(entity) = scene.entities.get_mut(idx) {
                    entity.animation_mut().toggle_playback();
                    tracing::info!(
                        "Entity '{}': {}",
                        entity.name,
                        if entity.animation().playing {
                            "playing"
                        } else {
                            "paused"
                        }
                    );
                    *config_dirty = true;
                }
            }
        }
        Action::CycleEntity => {
            // Empty scene: nothing to cycle through, silently
            // no-op so the user's `Tab` doesn't grab focus from
            // the egui panel (which Tab would otherwise navigate).
            if scene.entities.is_empty() {
                return true;
            }
            let next = match selection.selected_index() {
                Some(idx) => (idx + 1) % scene.entities.len(),
                None => 0,
            };
            selection.select(next);
            tracing::info!(
                "Selected: {} ({})",
                scene.entities[next].name,
                scene.entities[next].id
            );
        }
        Action::BringForward => {
            if let Some(idx) = selection.selected_index() {
                if let Some(entity) = scene.entities.get_mut(idx) {
                    entity.z_index += 10;
                    tracing::info!("z-index: {} ({})", entity.z_index, entity.name);
                    scene.mark_visible_dirty();
                    *config_dirty = true;
                }
            }
        }
        Action::SendBackward => {
            if let Some(idx) = selection.selected_index() {
                if let Some(entity) = scene.entities.get_mut(idx) {
                    entity.z_index -= 10;
                    tracing::info!("z-index: {} ({})", entity.z_index, entity.name);
                    scene.mark_visible_dirty();
                    *config_dirty = true;
                }
            }
        }
        Action::FpsDown => {
            if let Some(idx) = selection.selected_index() {
                if let Some(entity) = scene.entities.get_mut(idx) {
                    let fps = entity.animation().fps;
                    entity.animation_mut().set_fps((fps - 2.0).max(1.0));
                    tracing::info!("FPS: {:.0} ({})", entity.animation().fps, entity.name);
                    *config_dirty = true;
                }
            }
        }
        Action::FpsUp => {
            if let Some(idx) = selection.selected_index() {
                if let Some(entity) = scene.entities.get_mut(idx) {
                    let fps = entity.animation().fps;
                    entity.animation_mut().set_fps(fps + 2.0);
                    tracing::info!("FPS: {:.0} ({})", entity.animation().fps, entity.name);
                    *config_dirty = true;
                }
            }
        }
        Action::ShowEntityInfo => {
            if let Some(e) = selection
                .selected_index()
                .and_then(|idx| scene.entities.get(idx))
            {
                tracing::info!(
                    "━━━ Entity Info ━━━\n  Name: {}\n  ID: {}\n  Position: ({:.0}, {:.0})\n  Scale: {:.2}\n  Opacity: {:.0}%\n  FPS: {:.0}\n  Frames: {}\n  z-index: {}\n  Visible: {}\n  Playing: {}\n  Asset: {}",
                    e.name, e.id, e.x, e.y, e.scale,
                    e.opacity * 100.0, e.animation().fps,
                    e.animation().frame_count(), e.z_index,
                    e.visible, e.animation().playing, e.asset_path
                );
            }
        }
        Action::ShowHelp => {
            tracing::info!(
                "━━━ KEYBOARD SHORTCUTS ━━━\n\
                \n  Navigation:\n\
                \n    Tab        — Cycle through entities\n\
                \n    Click      — Select entity\n\
                \n    Escape     — Exit edit mode (auto-saves)\n\
                \n\n  Position:\n\
                \n    Drag       — Move entity\n\
                \n    Arrows     — Nudge 10px\n\
                \n    Shift+Arrows — Fine nudge 1px\n\
                \n    Home       — Center on screen\n\
                \n\n  Appearance:\n\
                \n    Scroll     — Resize\n\
                \n    +/-        — Opacity\n\
                \n    R          — Reset scale/opacity\n\
                \n    V          — Toggle visibility\n\
                \n    PgUp/PgDn  — Z-order\n\
                \n\n  Animation:\n\
                \n    P          — Play/pause entity\n\
                \n    Space      — Global play/pause\n\
                \n    [/]        — Adjust FPS\n\
                \n\n  Physics:\n\
                \n    G          — Toggle gravity (off by default)\n\
                \n\n  Actions:\n\
                \n    D          — Duplicate\n\
                \n    Del/Bksp   — Delete\n\
                \n    I          — Show entity info\n\
                \n    S          — Save config\n\
                \n    Q          — Save and exit\n\
                \n    H          — This help"
            );
        }
        _ => return false,
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behavior::Behavior;

    fn scene_with(n: usize) -> Scene {
        let mut scene = Scene::from_config(&crate::config::AppConfig::default());
        scene.entities.clear();
        for i in 0..n {
            let cfg = crate::config::CharacterConfig {
                id: format!("e{i}"),
                name: format!("E{i}"),
                asset_type: crate::config::AssetType::PngStatic,
                asset_path: String::new(),
                x: 100.0,
                y: 200.0,
                scale: 1.0,
                opacity: 1.0,
                fps: 8.0,
                visible: true,
                playing: false,
                z_index: i as i32,
                physics_enabled: false,
                behavior: Behavior::Idle,
                spritesheet_columns: None,
                spritesheet_rows: None,
                monitor: None,
                easing: None,
                animations: std::collections::BTreeMap::new(),
            };
            let frame = crate::animation::frame::Frame::new(vec![0u8; 4], 1, 1);
            let anim = crate::animation::Animation::new(vec![frame], 1.0, false);
            scene
                .entities
                .push(crate::entity::Entity::from_config(&cfg, anim));
        }
        scene
    }

    fn run(action: Action, scene: &mut Scene, sel: &mut SelectionState, shift: bool) -> bool {
        let mut dirty = false;
        dispatch_shared(action, scene, sel, &mut dirty, shift)
    }

    #[test]
    fn a_nudge_moves_the_selected_entity() {
        let mut scene = scene_with(1);
        let mut sel = SelectionState::default();
        sel.select(0);
        assert!(run(Action::NudgeRight, &mut scene, &mut sel, false));
        assert_eq!(scene.entities[0].x, 110.0, "normal nudge should be 10 px");
        assert!(run(Action::NudgeRight, &mut scene, &mut sel, true));
        assert_eq!(scene.entities[0].x, 111.0, "shift nudge should be 1 px");
    }

    /// Every selection-driven action has to survive being pressed with
    /// nothing selected — a keypress is the wrong place to panic.
    #[test]
    fn selection_actions_are_no_ops_without_a_selection() {
        let mut scene = scene_with(1);
        let mut sel = SelectionState::default();
        for action in [
            Action::NudgeUp,
            Action::NudgeDown,
            Action::NudgeLeft,
            Action::NudgeRight,
            Action::ToggleVisible,
            Action::ToggleGravity,
            Action::TogglePlayback,
            Action::ResetTransform,
            Action::OpacityUp,
            Action::OpacityDown,
            Action::BringForward,
            Action::SendBackward,
            Action::FpsUp,
            Action::FpsDown,
            Action::ShowEntityInfo,
        ] {
            assert!(run(action, &mut scene, &mut sel, false), "{action:?}");
        }
        assert_eq!(
            scene.entities[0].x, 100.0,
            "something moved with no selection"
        );
    }

    #[test]
    fn cycling_wraps_and_tolerates_an_empty_scene() {
        let mut empty = scene_with(0);
        let mut sel = SelectionState::default();
        assert!(run(Action::CycleEntity, &mut empty, &mut sel, false));
        assert_eq!(sel.selected_index(), None);

        let mut scene = scene_with(3);
        for expected in [0, 1, 2, 0] {
            run(Action::CycleEntity, &mut scene, &mut sel, false);
            assert_eq!(sel.selected_index(), Some(expected));
        }
    }

    #[test]
    fn toggling_visibility_round_trips() {
        let mut scene = scene_with(1);
        let mut sel = SelectionState::default();
        sel.select(0);
        run(Action::ToggleVisible, &mut scene, &mut sel, false);
        assert!(!scene.entities[0].visible);
        run(Action::ToggleVisible, &mut scene, &mut sel, false);
        assert!(scene.entities[0].visible);
    }

    /// The backend-specific actions must be declined, not silently
    /// swallowed, or a caller's own arms would never run.
    #[test]
    fn backend_specific_actions_are_declined() {
        let mut scene = scene_with(1);
        let mut sel = SelectionState::default();
        for action in [
            Action::ToggleEditMode,
            Action::QuitWithSave,
            Action::SaveNow,
            Action::DeleteSelected,
            Action::CenterOnScreen,
            Action::DuplicateSelected,
        ] {
            assert!(!run(action, &mut scene, &mut sel, false), "{action:?}");
        }
    }

    #[test]
    fn a_handled_action_marks_the_config_dirty() {
        let mut scene = scene_with(1);
        let mut sel = SelectionState::default();
        sel.select(0);
        let mut dirty = false;
        dispatch_shared(Action::NudgeUp, &mut scene, &mut sel, &mut dirty, false);
        assert!(dirty, "a move that changes the scene must persist");
    }
}
