//! Keyboard action dispatch — the 27-arm match originally inlined in
//! `app.rs`. Extracted in H.1 so the parent module's
//! `ApplicationHandler` impl stays readable; nothing about per-arm
//! behaviour changes here.

use super::App;
use crate::keybindings::Action;
use crate::ui::panels;
use winit::event_loop::ActiveEventLoop;

impl App {
    /// Run the handler bound to `action`. The match preserves the
    /// per-arm behaviour previously inlined in the `KeyboardInput`
    /// match: per-entity actions silently no-op without a selection,
    /// `QuitWithSave` tears down GPU/X11 state before calling
    /// `event_loop.exit()`, etc.
    pub(super) fn dispatch_action(&mut self, action: Action, event_loop: &ActiveEventLoop) {
        match action {
            Action::ToggleEditMode => {
                self.toggle_edit_mode();
            }
            Action::QuitWithSave => {
                tracing::info!("Quit action — saving and exiting");
                self.save_config_if_needed();
                self.ui = None;
                self.renderer = None;
                self.x11_input = None;
                event_loop.exit();
            }
            Action::SaveNow => {
                self.config_dirty = true;
                self.save_config_if_needed();
                tracing::info!("Config saved manually");
            }
            Action::PauseAll => {
                self.scene.toggle_global_playback();
                self.config_dirty = true;
            }
            Action::DeleteSelected => {
                if let Some(idx) = self.selection.selected_index() {
                    let removed_name = self
                        .scene
                        .entities
                        .get(idx)
                        .map(|e| e.name.clone())
                        .unwrap_or_default();
                    if let Some(renderer) = &mut self.renderer {
                        if let Some(entity) = self.scene.entities.get(idx) {
                            renderer.shared.textures.remove(&entity.id);
                        }
                    }
                    if let Some(removed_id) = self.scene.remove_entity(idx) {
                        tracing::info!("Deleted entity: {}", removed_id);
                        self.selection.deselect();
                        self.config_dirty = true;
                        self.toasts.info(format!("Deleted {removed_name}"));
                        self.save_config_if_needed();
                    }
                }
            }
            // Arrow nudges: Shift = 1 px fine, normal = 10 px. Every
            // nudge invalidates Bounce rest so the entity doesn't
            // snap back after the keypress. All selection-driven arms
            // go through `get_mut` — the deselect-on-removal invariant
            // holds everywhere today, but a panic on a stale index is
            // the wrong failure mode for a keypress either way.
            Action::NudgeUp => {
                if let Some(idx) = self.selection.selected_index() {
                    let step = if self.shift_held { 1.0 } else { 10.0 };
                    if let Some(entity) = self.scene.entities.get_mut(idx) {
                        entity.y -= step;
                        entity.behavior_state.bounce_invalidate();
                        self.config_dirty = true;
                    }
                }
            }
            Action::NudgeDown => {
                if let Some(idx) = self.selection.selected_index() {
                    let step = if self.shift_held { 1.0 } else { 10.0 };
                    if let Some(entity) = self.scene.entities.get_mut(idx) {
                        entity.y += step;
                        entity.behavior_state.bounce_invalidate();
                        self.config_dirty = true;
                    }
                }
            }
            Action::NudgeLeft => {
                if let Some(idx) = self.selection.selected_index() {
                    let step = if self.shift_held { 1.0 } else { 10.0 };
                    if let Some(entity) = self.scene.entities.get_mut(idx) {
                        entity.x -= step;
                        entity.behavior_state.bounce_invalidate();
                        self.config_dirty = true;
                    }
                }
            }
            Action::NudgeRight => {
                if let Some(idx) = self.selection.selected_index() {
                    let step = if self.shift_held { 1.0 } else { 10.0 };
                    if let Some(entity) = self.scene.entities.get_mut(idx) {
                        entity.x += step;
                        entity.behavior_state.bounce_invalidate();
                        self.config_dirty = true;
                    }
                }
            }
            Action::ResetTransform => {
                if let Some(idx) = self.selection.selected_index() {
                    if let Some(entity) = self.scene.entities.get_mut(idx) {
                        entity.scale = 1.0;
                        entity.opacity = 1.0;
                        tracing::info!("Reset '{}' scale=1.0, opacity=1.0", entity.name);
                        self.config_dirty = true;
                    }
                }
            }
            Action::CenterOnScreen => {
                if let Some(idx) = self.selection.selected_index() {
                    if let Some(window) = &self.window {
                        let size = window.inner_size();
                        if let Some(entity) = self.scene.entities.get_mut(idx) {
                            entity.x = (size.width as f32 - entity.scaled_width()) / 2.0;
                            entity.y = (size.height as f32 - entity.scaled_height()) / 2.0;
                            entity.behavior_state.bounce_invalidate();
                            tracing::info!(
                                "Centered '{}' at ({:.0}, {:.0})",
                                entity.name,
                                entity.x,
                                entity.y
                            );
                            self.config_dirty = true;
                        }
                    }
                }
            }
            Action::OpacityUp => {
                if let Some(idx) = self.selection.selected_index() {
                    if let Some(entity) = self.scene.entities.get_mut(idx) {
                        entity.opacity = (entity.opacity + 0.1).min(1.0);
                        tracing::info!("Opacity: {:.0}%", entity.opacity * 100.0);
                        self.config_dirty = true;
                    }
                }
            }
            Action::OpacityDown => {
                if let Some(idx) = self.selection.selected_index() {
                    if let Some(entity) = self.scene.entities.get_mut(idx) {
                        entity.opacity = (entity.opacity - 0.1).max(0.05);
                        tracing::info!("Opacity: {:.0}%", entity.opacity * 100.0);
                        self.config_dirty = true;
                    }
                }
            }
            Action::ToggleVisible => {
                if let Some(idx) = self.selection.selected_index() {
                    if let Some(entity) = self.scene.entities.get_mut(idx) {
                        entity.visible = !entity.visible;
                        tracing::info!(
                            "Entity '{}' visibility: {}",
                            entity.name,
                            if entity.visible { "visible" } else { "hidden" }
                        );
                        self.scene.mark_visible_dirty();
                        self.config_dirty = true;
                    }
                }
            }
            // Gravity: off by default — entity stays put. Toggling on
            // makes it fall from its current position; off pins it.
            Action::ToggleGravity => {
                if let Some(idx) = self.selection.selected_index() {
                    if let Some(entity) = self.scene.entities.get_mut(idx) {
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
                        self.config_dirty = true;
                    }
                }
            }
            Action::TogglePlayback => {
                if let Some(idx) = self.selection.selected_index() {
                    if let Some(entity) = self.scene.entities.get_mut(idx) {
                        entity.animation.toggle_playback();
                        tracing::info!(
                            "Entity '{}': {}",
                            entity.name,
                            if entity.animation.playing {
                                "playing"
                            } else {
                                "paused"
                            }
                        );
                        self.config_dirty = true;
                    }
                }
            }
            Action::DuplicateSelected => {
                if let Some(idx) = self.selection.selected_index() {
                    let Some(src) = self.scene.entities.get(idx) else {
                        return;
                    };
                    let src_path = std::path::PathBuf::from(&src.asset_path);
                    let new_x = src.x + 30.0;
                    let new_y = src.y + 30.0;
                    // Copy before the add — push can't invalidate idx
                    // today, but reading through the stale borrow after
                    // a Vec mutation is exactly the pattern get/get_mut
                    // is here to retire.
                    let orig_scale = src.scale;
                    let orig_opacity = src.opacity;
                    match self.scene.add_entity_from_path(&src_path, new_x, new_y) {
                        Ok(new_idx) => {
                            self.scene.entities[new_idx].scale = orig_scale;
                            self.scene.entities[new_idx].opacity = orig_opacity;
                            if let Some(renderer) = &mut self.renderer {
                                renderer.ensure_texture(&self.scene.entities[new_idx]);
                                self.scene.entities[new_idx].texture_dirty = false;
                            }
                            self.selection.select(new_idx);
                            self.config_dirty = true;
                            self.save_config_if_needed();
                            tracing::info!("Duplicated entity at ({:.0}, {:.0})", new_x, new_y);
                        }
                        Err(e) => {
                            tracing::error!("Failed to duplicate: {}", e);
                            self.toasts.error(format!("Duplicate failed: {e}"));
                        }
                    }
                }
            }
            Action::CycleEntity => {
                // Empty scene: nothing to cycle through, silently
                // no-op so the user's `Tab` doesn't grab focus from
                // the egui panel (which Tab would otherwise navigate).
                if self.scene.entities.is_empty() {
                    return;
                }
                let next = match self.selection.selected_index() {
                    Some(idx) => (idx + 1) % self.scene.entities.len(),
                    None => 0,
                };
                self.selection.select(next);
                tracing::info!(
                    "Selected: {} ({})",
                    self.scene.entities[next].name,
                    self.scene.entities[next].id
                );
            }
            Action::BringForward => {
                if let Some(idx) = self.selection.selected_index() {
                    if let Some(entity) = self.scene.entities.get_mut(idx) {
                        entity.z_index += 10;
                        tracing::info!("z-index: {} ({})", entity.z_index, entity.name);
                        self.scene.mark_visible_dirty();
                        self.config_dirty = true;
                    }
                }
            }
            Action::SendBackward => {
                if let Some(idx) = self.selection.selected_index() {
                    if let Some(entity) = self.scene.entities.get_mut(idx) {
                        entity.z_index -= 10;
                        tracing::info!("z-index: {} ({})", entity.z_index, entity.name);
                        self.scene.mark_visible_dirty();
                        self.config_dirty = true;
                    }
                }
            }
            Action::FpsDown => {
                if let Some(idx) = self.selection.selected_index() {
                    if let Some(entity) = self.scene.entities.get_mut(idx) {
                        entity
                            .animation
                            .set_fps((entity.animation.fps - 2.0).max(1.0));
                        tracing::info!("FPS: {:.0} ({})", entity.animation.fps, entity.name);
                        self.config_dirty = true;
                    }
                }
            }
            Action::FpsUp => {
                if let Some(idx) = self.selection.selected_index() {
                    if let Some(entity) = self.scene.entities.get_mut(idx) {
                        entity.animation.set_fps(entity.animation.fps + 2.0);
                        tracing::info!("FPS: {:.0} ({})", entity.animation.fps, entity.name);
                        self.config_dirty = true;
                    }
                }
            }
            Action::CycleMonitor => {
                if let Some(idx) = self.selection.selected_index() {
                    if let Some(entity) = self.scene.entities.get_mut(idx) {
                        let toast =
                            panels::cycle_entity_monitor(&mut entity.monitor, &self.monitors);
                        self.toasts.info(toast);
                        self.config_dirty = true;
                    }
                }
            }
            Action::ShowEntityInfo => {
                if let Some(e) = self
                    .selection
                    .selected_index()
                    .and_then(|idx| self.scene.entities.get(idx))
                {
                    tracing::info!(
                        "━━━ Entity Info ━━━\n  Name: {}\n  ID: {}\n  Position: ({:.0}, {:.0})\n  Scale: {:.2}\n  Opacity: {:.0}%\n  FPS: {:.0}\n  Frames: {}\n  z-index: {}\n  Visible: {}\n  Playing: {}\n  Asset: {}",
                        e.name, e.id, e.x, e.y, e.scale,
                        e.opacity * 100.0, e.animation.fps,
                        e.animation.frame_count(), e.z_index,
                        e.visible, e.animation.playing, e.asset_path
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
            Action::TogglePerfOverlay => {
                self.perf_overlay_visible = !self.perf_overlay_visible;
                tracing::debug!(
                    "Perf overlay {}",
                    if self.perf_overlay_visible {
                        "shown"
                    } else {
                        "hidden"
                    }
                );
            }
            // Actions whose runtime path lives outside the in-app
            // dispatch: HideOverlay fires only as a global hotkey;
            // OpenCommandPalette is intercepted by `panels.rs` reading
            // egui's keyboard input. Both reach this match arm when
            // a user rebinds them onto a chord that's still active in
            // edit mode — we leave the handling to the original sites
            // rather than duplicate it here.
            Action::HideOverlay | Action::OpenCommandPalette => {}
        }
    }
}
