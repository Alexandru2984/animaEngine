//! Keyboard actions that both backends can run.
//!
//! These twenty touch only the scene, the selection, the dirty flag and
//! the toast queue — nothing about a window, a renderer or an event loop —
//! so they belong to neither backend in particular.
//!
//! They live here because they were living in `app::dispatch` instead,
//! reachable only from the winit path. The native Wayland loop consulted
//! the keybinding table in one place and matched one action, so of the
//! twenty-eight rebindable actions the Keybindings tab offers, twenty-seven
//! did nothing there: the user could rebind them and watch the config
//! persist while nudge, delete, cycle and the rest stayed inert (R19 in
//! docs/runtime-findings.md).
//!
//! The remaining six stay per-backend because they genuinely differ:
//! quitting and saving (each loop owns its shutdown), the edit-mode
//! toggle (Wayland has to reshape its input region), deleting and
//! duplicating (both reach into the renderer's texture cache), and the
//! perf overlay (a winit-only counter).
//!
//! Centring and monitor-cycling used to be in that list. They only looked
//! window-bound: centring needs a rectangle, not a window, and cycling
//! needs the monitor list — both of which the Wayland loop already has.

use crate::input::selection::SelectionState;
use crate::keybindings::Action;
use crate::monitor::{DesktopBounds, MonitorInfo};
use crate::scene::Scene;
use crate::ui::toasts::ToastQueue;

/// What a shared action is allowed to touch.
///
/// A struct rather than a parameter list: this started at five arguments
/// and centring plus monitor-cycling would have made it eight, at which
/// point call sites stop being readable.
pub struct ActionCtx<'a> {
    pub scene: &'a mut Scene,
    pub selection: &'a mut SelectionState,
    pub config_dirty: &'a mut bool,
    /// Fine-nudge modifier: 1 px instead of 10.
    pub shift_held: bool,
    /// The region an entity may occupy — what "centre on screen" centres
    /// against. The winit path passes its window, the Wayland path the
    /// area its layer surfaces cover.
    pub bounds: DesktopBounds,
    pub monitors: &'a [MonitorInfo],
    pub toasts: &'a mut ToastQueue,
}

/// Run `action` if it is one of the backend-independent ones.
///
/// Returns `false` when the action is not handled here, so a caller can
/// fall through to its own arms rather than silently swallowing it.
pub fn dispatch_shared(action: Action, ctx: &mut ActionCtx<'_>) -> bool {
    let ActionCtx {
        scene,
        selection,
        config_dirty,
        shift_held,
        bounds,
        monitors,
        toasts,
    } = ctx;
    let shift_held = *shift_held;
    let bounds = *bounds;
    // Destructuring gives `&mut &mut bool`; reborrow once so the arms can
    // keep writing `*config_dirty = true` as they did before the move.
    let config_dirty: &mut bool = config_dirty;
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
        Action::CenterOnScreen => {
            if let Some(idx) = selection.selected_index() {
                if let Some(entity) = scene.entities.get_mut(idx) {
                    // Centres on the region the overlay covers, which the
                    // caller supplies: the window on winit, the layer
                    // surfaces' area on Wayland.
                    entity.x =
                        bounds.min_x + (bounds.max_x - bounds.min_x - entity.scaled_width()) / 2.0;
                    entity.y =
                        bounds.min_y + (bounds.max_y - bounds.min_y - entity.scaled_height()) / 2.0;
                    entity.behavior_state.bounce_invalidate();
                    tracing::info!(
                        "Centered '{}' at ({:.0}, {:.0})",
                        entity.name,
                        entity.x,
                        entity.y
                    );
                    *config_dirty = true;
                }
            }
        }
        Action::CycleMonitor => {
            if let Some(idx) = selection.selected_index() {
                if let Some(entity) = scene.entities.get_mut(idx) {
                    let toast =
                        crate::ui::panels::cycle_entity_monitor(&mut entity.monitor, monitors);
                    toasts.info(toast);
                    *config_dirty = true;
                }
            }
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
        let mut toasts = ToastQueue::default();
        let mut ctx = ActionCtx {
            scene,
            selection: sel,
            config_dirty: &mut dirty,
            shift_held: shift,
            bounds: DesktopBounds::from_size(1920.0, 1080.0),
            monitors: &[],
            toasts: &mut toasts,
        };
        dispatch_shared(action, &mut ctx)
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
            Action::DuplicateSelected,
            Action::TogglePerfOverlay,
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
        let mut toasts = ToastQueue::default();
        let mut ctx = ActionCtx {
            scene: &mut scene,
            selection: &mut sel,
            config_dirty: &mut dirty,
            shift_held: false,
            bounds: DesktopBounds::from_size(1920.0, 1080.0),
            monitors: &[],
            toasts: &mut toasts,
        };
        dispatch_shared(Action::NudgeUp, &mut ctx);
        assert!(dirty, "a move that changes the scene must persist");
    }

    // ── centring and monitor-cycling ────────────────────────────────
    //
    // These two moved here from `app::dispatch` after the context struct
    // landed. They had looked window-bound; they are not, and the Wayland
    // path was missing them for no reason.

    /// Centring has to use the *bounds the caller passed*, not a guess at
    /// the primary screen. On Wayland the two differ whenever the overlay
    /// spans more than one output.
    #[test]
    fn centring_uses_the_supplied_bounds() {
        let mut scene = scene_with(1);
        scene.entities[0].scale = 1.0;
        let mut sel = SelectionState::default();
        sel.select(0);
        let mut dirty = false;
        let mut toasts = ToastQueue::default();
        let (w, h) = (
            scene.entities[0].scaled_width(),
            scene.entities[0].scaled_height(),
        );
        // An off-origin rectangle: a second monitor to the right of the
        // first. Centring against it must land at its middle, not at the
        // desktop's.
        let bounds = DesktopBounds {
            min_x: 1920.0,
            min_y: 0.0,
            max_x: 3200.0,
            max_y: 720.0,
        };
        let mut ctx = ActionCtx {
            scene: &mut scene,
            selection: &mut sel,
            config_dirty: &mut dirty,
            shift_held: false,
            bounds,
            monitors: &[],
            toasts: &mut toasts,
        };
        assert!(dispatch_shared(Action::CenterOnScreen, &mut ctx));
        assert_eq!(scene.entities[0].x, 1920.0 + (1280.0 - w) / 2.0);
        assert_eq!(scene.entities[0].y, (720.0 - h) / 2.0);
        assert!(dirty);
    }

    #[test]
    fn centring_without_a_selection_is_a_no_op() {
        let mut scene = scene_with(1);
        let before = (scene.entities[0].x, scene.entities[0].y);
        let mut sel = SelectionState::default();
        // Handled — it is one of ours — but it must not move anything.
        assert!(run(Action::CenterOnScreen, &mut scene, &mut sel, false));
        assert_eq!((scene.entities[0].x, scene.entities[0].y), before);
    }

    fn monitor(name: &str, x: i32) -> MonitorInfo {
        MonitorInfo {
            name: name.to_string(),
            x,
            y: 0,
            width: 1920,
            height: 1080,
            scale_factor: 1.0,
            is_primary: x == 0,
        }
    }

    #[test]
    fn cycling_walks_the_monitor_list_and_toasts() {
        let mut scene = scene_with(1);
        let mut sel = SelectionState::default();
        sel.select(0);
        let mons = [monitor("DP-1", 0), monitor("DP-2", 1920)];
        let mut dirty = false;
        let mut toasts = ToastQueue::default();
        let mut ctx = ActionCtx {
            scene: &mut scene,
            selection: &mut sel,
            config_dirty: &mut dirty,
            shift_held: false,
            bounds: DesktopBounds::from_size(3840.0, 1080.0),
            monitors: &mons,
            toasts: &mut toasts,
        };
        assert!(dispatch_shared(Action::CycleMonitor, &mut ctx));
        let first = scene.entities[0].monitor.clone();
        assert!(first.is_some(), "cycling off None must pick a monitor");
        assert!(dirty, "the new assignment has to persist");
        assert_eq!(
            toasts.iter().count(),
            1,
            "the user needs to see where it went"
        );
    }

    /// The Wayland loop can hand over an empty monitor list on the frame
    /// before the compositor has advertised any output. Cycling then has
    /// nothing to pick and must leave the entity where it is.
    #[test]
    fn cycling_with_no_monitors_leaves_the_entity_alone() {
        let mut scene = scene_with(1);
        let mut sel = SelectionState::default();
        sel.select(0);
        assert!(run(Action::CycleMonitor, &mut scene, &mut sel, false));
        assert_eq!(scene.entities[0].monitor, None);
    }
}
