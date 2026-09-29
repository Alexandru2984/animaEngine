//! Keyboard actions that both backends can run.
//!
//! These twenty-four touch only the scene, the selection, the dirty flag,
//! the toast queue and the undo history — nothing about a window, a
//! renderer or an event loop — so they belong to neither backend in
//! particular.
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
    /// Edits so far, for undo and redo.
    pub history: &'a mut crate::undo::UndoHistory,
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
        history,
    } = ctx;
    let shift_held = *shift_held;
    let bounds = *bounds;
    // Destructuring gives `&mut &mut bool`; reborrow once so the arms can
    // keep writing `*config_dirty = true` as they did before the move.
    let config_dirty: &mut bool = config_dirty;
    match action {
        Action::Undo | Action::Redo => {
            // The step brings back its selection too (`crate::undo`).
            let (done, done_key, none_key) = if action == Action::Undo {
                (
                    history.undo(scene, selection),
                    "toast-undone",
                    "toast-nothing-to-undo",
                )
            } else {
                (
                    history.redo(scene, selection),
                    "toast-redone",
                    "toast-nothing-to-redo",
                )
            };
            if done {
                toasts.info(crate::i18n::t(done_key));
                *config_dirty = true;
            } else {
                toasts.info(crate::i18n::t(none_key));
            }
        }
        Action::PauseAll => {
            scene.toggle_global_playback();
            *config_dirty = true;
        }
        Action::NudgeUp => {
            for idx in selection.selected_indices() {
                let step = if shift_held { 1.0 } else { 10.0 };
                if let Some(entity) = scene.entities.get_mut(idx) {
                    entity.y -= step;
                    entity.behavior_state.bounce_invalidate();
                    *config_dirty = true;
                }
            }
        }
        Action::NudgeDown => {
            for idx in selection.selected_indices() {
                let step = if shift_held { 1.0 } else { 10.0 };
                if let Some(entity) = scene.entities.get_mut(idx) {
                    entity.y += step;
                    entity.behavior_state.bounce_invalidate();
                    *config_dirty = true;
                }
            }
        }
        Action::NudgeLeft => {
            for idx in selection.selected_indices() {
                let step = if shift_held { 1.0 } else { 10.0 };
                if let Some(entity) = scene.entities.get_mut(idx) {
                    entity.x -= step;
                    entity.behavior_state.bounce_invalidate();
                    *config_dirty = true;
                }
            }
        }
        Action::NudgeRight => {
            for idx in selection.selected_indices() {
                let step = if shift_held { 1.0 } else { 10.0 };
                if let Some(entity) = scene.entities.get_mut(idx) {
                    entity.x += step;
                    entity.behavior_state.bounce_invalidate();
                    *config_dirty = true;
                }
            }
        }
        Action::ResetTransform => {
            for idx in selection.selected_indices() {
                if let Some(entity) = scene.entities.get_mut(idx) {
                    entity.scale = 1.0;
                    entity.opacity = 1.0;
                    tracing::info!("Reset '{}' scale=1.0, opacity=1.0", entity.name);
                    *config_dirty = true;
                }
            }
        }
        Action::OpacityUp => {
            for idx in selection.selected_indices() {
                if let Some(entity) = scene.entities.get_mut(idx) {
                    entity.opacity = (entity.opacity + 0.1).min(1.0);
                    tracing::info!("Opacity: {:.0}%", entity.opacity * 100.0);
                    *config_dirty = true;
                }
            }
        }
        Action::OpacityDown => {
            for idx in selection.selected_indices() {
                if let Some(entity) = scene.entities.get_mut(idx) {
                    entity.opacity = (entity.opacity - 0.1).max(0.05);
                    tracing::info!("Opacity: {:.0}%", entity.opacity * 100.0);
                    *config_dirty = true;
                }
            }
        }
        // With several selected, the primary decides: all take the
        // state it toggles to, rather than each flipping its own.
        Action::ToggleVisible => {
            if let Some(target) = primary(scene, selection).map(|e| !e.visible) {
                for idx in selection.selected_indices() {
                    if let Some(entity) = scene.entities.get_mut(idx) {
                        entity.visible = target;
                        tracing::info!(
                            "Entity '{}' visibility: {}",
                            entity.name,
                            if target { "visible" } else { "hidden" }
                        );
                    }
                }
                scene.mark_visible_dirty();
                *config_dirty = true;
            }
        }
        // Gravity: off by default — entity stays put. Toggling on
        // makes it fall from its current position; off pins it.
        Action::ToggleGravity => {
            if let Some(target) = primary(scene, selection).map(|e| !e.physics.enabled) {
                for idx in selection.selected_indices() {
                    if let Some(entity) = scene.entities.get_mut(idx) {
                        if target {
                            entity.physics.enable();
                        } else {
                            entity.physics.disable();
                        }
                        tracing::info!(
                            "Entity '{}' gravity: {}",
                            entity.name,
                            if target {
                                "ON (falling)"
                            } else {
                                "OFF (pinned)"
                            }
                        );
                    }
                }
                *config_dirty = true;
            }
        }
        Action::TogglePlayback => {
            if let Some(target) = primary(scene, selection).map(|e| !e.animation().playing) {
                for idx in selection.selected_indices() {
                    if let Some(entity) = scene.entities.get_mut(idx) {
                        if entity.animation().playing != target {
                            entity.animation_mut().toggle_playback();
                        }
                        tracing::info!(
                            "Entity '{}': {}",
                            entity.name,
                            if target { "playing" } else { "paused" }
                        );
                    }
                }
                *config_dirty = true;
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
            for idx in selection.selected_indices() {
                if let Some(entity) = scene.entities.get_mut(idx) {
                    entity.z_index += 10;
                    tracing::info!("z-index: {} ({})", entity.z_index, entity.name);
                    scene.mark_visible_dirty();
                    *config_dirty = true;
                }
            }
        }
        Action::SendBackward => {
            for idx in selection.selected_indices() {
                if let Some(entity) = scene.entities.get_mut(idx) {
                    entity.z_index -= 10;
                    tracing::info!("z-index: {} ({})", entity.z_index, entity.name);
                    scene.mark_visible_dirty();
                    *config_dirty = true;
                }
            }
        }
        Action::FpsDown => {
            for idx in selection.selected_indices() {
                if let Some(entity) = scene.entities.get_mut(idx) {
                    let fps = entity.animation().fps;
                    entity.animation_mut().set_fps((fps - 2.0).max(1.0));
                    tracing::info!("FPS: {:.0} ({})", entity.animation().fps, entity.name);
                    *config_dirty = true;
                }
            }
        }
        Action::FpsUp => {
            for idx in selection.selected_indices() {
                if let Some(entity) = scene.entities.get_mut(idx) {
                    let fps = entity.animation().fps;
                    entity.animation_mut().set_fps(fps + 2.0);
                    tracing::info!("FPS: {:.0} ({})", entity.animation().fps, entity.name);
                    *config_dirty = true;
                }
            }
        }
        Action::ShowEntityInfo => {
            for e in selection
                .selected_indices()
                .into_iter()
                .filter_map(|idx| scene.entities.get(idx))
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
                \n    Click      — Select entity (a grouped one: its group; again: just it)\n\
                \n    Shift+Click — Add to / remove from the selection\n\
                \n    Drag on empty space — Select what the rectangle touches\n\
                \n    Escape     — Exit edit mode (auto-saves)\n\
                \n\n  Position:\n\
                \n    Drag       — Move entity (every selected one), snapping\n\
                \n    Alt+Drag   — Move without snapping\n\
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
                \n    Ctrl+Z     — Undo\n\
                \n    Ctrl+Shift+Z — Redo\n\
                \n    D          — Duplicate\n\
                \n    Ctrl+C/X/V — Copy, cut, paste (into any scene)\n\
                \n    Ctrl+G     — Group the selection\n\
                \n    Ctrl+Shift+G — Ungroup\n\
                \n    Del/Bksp   — Delete\n\
                \n    I          — Show entity info\n\
                \n    S          — Save config\n\
                \n    Q          — Save and exit\n\
                \n    H          — This help"
            );
        }
        // Copying only reads the scene. Cutting and pasting take
        // characters out and put them in, textures and all, so the
        // backends run those, as they do Delete and Duplicate.
        Action::CopySelected => {
            crate::outcomes::copy_entities(&selection.selected_indices(), scene, toasts);
        }
        // With nothing selected both do nothing, like every other
        // selection action; the right-click menu shares them.
        Action::GroupSelected => {
            *config_dirty |=
                crate::outcomes::group_entities(&selection.selected_indices(), scene, toasts);
        }
        Action::UngroupSelected => {
            if selection.count() > 0 {
                *config_dirty |=
                    crate::outcomes::ungroup_entities(&selection.selected_indices(), scene, toasts);
            }
        }
        Action::CenterOnScreen => {
            // Centres on the region the overlay covers, which the caller
            // supplies: the window on winit, the layer surfaces' area on
            // Wayland. Several selected move as one block, keeping their
            // arrangement, instead of piling up in the middle.
            let selected = selection.selected_indices();
            if let Some((left, top, right, bottom)) = bounding_box(scene, &selected) {
                let dx = bounds.min_x + (bounds.max_x - bounds.min_x - (right - left)) / 2.0 - left;
                let dy = bounds.min_y + (bounds.max_y - bounds.min_y - (bottom - top)) / 2.0 - top;
                for idx in selected {
                    if let Some(entity) = scene.entities.get_mut(idx) {
                        entity.x += dx;
                        entity.y += dy;
                        entity.behavior_state.bounce_invalidate();
                        tracing::info!(
                            "Centered '{}' at ({:.0}, {:.0})",
                            entity.name,
                            entity.x,
                            entity.y
                        );
                    }
                }
                *config_dirty = true;
            }
        }
        Action::CycleMonitor => {
            // The primary moves to the next monitor; the others go with it.
            if let Some(idx) = selection.selected_index() {
                let mut pin = None;
                if let Some(entity) = scene.entities.get_mut(idx) {
                    let previous = entity.monitor.clone();
                    let toast =
                        crate::ui::panels::cycle_entity_monitor(&mut entity.monitor, monitors);
                    crate::ui::panels::move_to_pinned_monitor(
                        entity,
                        previous.as_deref(),
                        monitors,
                    );
                    pin = Some(entity.monitor.clone());
                    toasts.info(toast);
                    *config_dirty = true;
                }
                if let Some(pin) = pin {
                    for other in selection.selected_indices().into_iter().skip(1) {
                        if let Some(entity) = scene.entities.get_mut(other) {
                            let previous = std::mem::replace(&mut entity.monitor, pin.clone());
                            crate::ui::panels::move_to_pinned_monitor(
                                entity,
                                previous.as_deref(),
                                monitors,
                            );
                        }
                    }
                }
            }
        }
        _ => return false,
    }
    true
}

/// The primary selected entity.
fn primary<'s>(scene: &'s Scene, selection: &SelectionState) -> Option<&'s crate::entity::Entity> {
    selection
        .selected_index()
        .and_then(|idx| scene.entities.get(idx))
}

/// Left, top, right, bottom around the entities at `indices`; `None` for
/// none.
pub(crate) fn bounding_box(scene: &Scene, indices: &[usize]) -> Option<(f32, f32, f32, f32)> {
    indices
        .iter()
        .filter_map(|&idx| scene.entities.get(idx))
        .map(|e| (e.x, e.y, e.x + e.scaled_width(), e.y + e.scaled_height()))
        .reduce(|(l, t, r, b), (l2, t2, r2, b2)| (l.min(l2), t.min(t2), r.max(r2), b.max(b2)))
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
        run_with(
            action,
            scene,
            sel,
            shift,
            &mut crate::undo::UndoHistory::default(),
        )
    }

    fn run_with(
        action: Action,
        scene: &mut Scene,
        sel: &mut SelectionState,
        shift: bool,
        history: &mut crate::undo::UndoHistory,
    ) -> bool {
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
            history,
        };
        dispatch_shared(action, &mut ctx)
    }

    #[test]
    fn undo_takes_back_a_nudge_and_keeps_the_selection_on_its_character() {
        use crate::undo::SETTLE;
        use std::time::Instant;
        let mut scene = scene_with(2);
        let mut sel = SelectionState::default();
        let mut history = crate::undo::UndoHistory::default();
        sel.select(1);
        let t0 = Instant::now();
        history.input(&scene, &sel, t0);
        assert!(run_with(
            Action::NudgeRight,
            &mut scene,
            &mut sel,
            false,
            &mut history
        ));
        assert!(history.settle(&scene, false, t0 + SETTLE));
        assert!(run_with(
            Action::Undo,
            &mut scene,
            &mut sel,
            false,
            &mut history
        ));
        assert_eq!(scene.entities[1].x, 100.0);
        assert_eq!(sel.selected_index(), Some(1));
        assert!(run_with(
            Action::Redo,
            &mut scene,
            &mut sel,
            false,
            &mut history
        ));
        assert_eq!(scene.entities[1].x, 110.0);
        // Nothing more to redo: still handled, it only says so.
        assert!(run_with(
            Action::Redo,
            &mut scene,
            &mut sel,
            false,
            &mut history
        ));
        assert_eq!(scene.entities[1].x, 110.0);
    }

    #[test]
    fn grouping_and_ungrouping_from_the_keyboard_undo_as_one_step_each() {
        use crate::undo::SETTLE;
        use std::time::Instant;
        let mut scene = scene_with(3);
        let mut sel = SelectionState::default();
        let mut history = crate::undo::UndoHistory::default();
        let t0 = Instant::now();
        // Nothing selected: nothing to group.
        assert!(run(Action::GroupSelected, &mut scene, &mut sel, false));
        assert!(scene.groups.is_empty());
        sel.select_all_of(&[0, 2]);
        history.input(&scene, &sel, t0);
        assert!(run_with(
            Action::GroupSelected,
            &mut scene,
            &mut sel,
            false,
            &mut history
        ));
        assert_eq!(scene.groups.len(), 1);
        assert_eq!(scene.groups[0].member_ids, ["e0", "e2"]);
        assert!(history.settle(&scene, false, t0 + SETTLE));
        history.input(&scene, &sel, t0 + SETTLE);
        assert!(run_with(
            Action::UngroupSelected,
            &mut scene,
            &mut sel,
            false,
            &mut history
        ));
        assert!(scene.groups.is_empty());
        // Undo brings the group back, and a second undo takes it away.
        for expected in [1, 0] {
            run_with(Action::Undo, &mut scene, &mut sel, false, &mut history);
            assert_eq!(scene.groups.len(), expected);
        }
    }

    #[test]
    fn actions_reach_every_selected_character() {
        let mut scene = scene_with(3);
        let mut sel = SelectionState::default();
        sel.select_all_of(&[2, 0]);
        assert!(run(Action::NudgeRight, &mut scene, &mut sel, false));
        let xs: Vec<f32> = scene.entities.iter().map(|e| e.x).collect();
        assert_eq!(xs, [110.0, 100.0, 110.0], "the unselected one stays");
        // Mixed states: the primary (2) decides, and both follow it.
        scene.entities[0].visible = false;
        assert!(run(Action::ToggleVisible, &mut scene, &mut sel, false));
        let visible: Vec<bool> = scene.entities.iter().map(|e| e.visible).collect();
        assert_eq!(visible, [false, true, false]);
    }

    #[test]
    fn centring_several_keeps_their_arrangement() {
        let mut scene = scene_with(2);
        scene.entities[1].x = 400.0;
        scene.entities[1].y = 250.0;
        let mut sel = SelectionState::default();
        sel.select_all_of(&[0, 1]);
        assert!(run(Action::CenterOnScreen, &mut scene, &mut sel, false));
        let (a, b) = (&scene.entities[0], &scene.entities[1]);
        assert_eq!((b.x - a.x, b.y - a.y), (300.0, 50.0), "not piled up");
        let (l, t, r, bottom) = bounding_box(&scene, &[0, 1]).unwrap();
        assert_eq!(((l + r) / 2.0, (t + bottom) / 2.0), (960.0, 540.0));
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
        // Every action the palette hides without a selection, except the
        // ones the backends dispatch themselves (delete, duplicate, cut).
        for &action in Action::ALL.iter().filter(|a| {
            a.acts_on_selection()
                && !matches!(
                    a,
                    Action::DeleteSelected | Action::DuplicateSelected | Action::CutSelected
                )
        }) {
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
            Action::CutSelected,
            Action::Paste,
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
            history: &mut crate::undo::UndoHistory::default(),
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
            history: &mut crate::undo::UndoHistory::default(),
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
            history: &mut crate::undo::UndoHistory::default(),
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
