//! Pointer + drag-drop event handlers. Extracted in H.4b so the
//! match in `App::window_event` reads as a short delegation table
//! instead of mixing rendering, IO and input concerns in one body.
//!
//! Keyboard is intentionally NOT here — it routes through
//! `dispatch_action` (see `src/app/dispatch.rs`) which is its own
//! concern. This module only owns mouse, scroll, drag-drop and
//! modifier tracking.

use super::{App, ContextMenuState};
use crate::drop_validate::{pre_validate_dropped_file, redact_path};
use std::path::PathBuf;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, Modifiers, MouseButton, MouseScrollDelta};

impl App {
    /// Primary-window wrapper: translate window-local winit coords by
    /// the primary origin (identity outside PerMonitor) so the stored
    /// mouse position is always **global desktop** coordinates — the
    /// same space entity positions live in (T.8).
    pub(super) fn handle_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        let origin = self.primary_origin();
        self.handle_cursor_moved_global(position.x as f32 + origin.0, position.y as f32 + origin.1);
    }

    pub(super) fn handle_cursor_moved_global(&mut self, gx: f32, gy: f32) {
        self.mouse_x = gx;
        self.mouse_y = gy;

        // Handle drag in edit mode
        if self.edit_mode {
            if let Some((entity_idx, new_x, new_y)) = self.drag.update(self.mouse_x, self.mouse_y) {
                if let Some(entity) = self.scene.entities.get_mut(entity_idx) {
                    entity.x = new_x;
                    entity.y = new_y;
                    // Drag relocates the entity → invalidate any
                    // Bounce rest position so the next tick
                    // re-snaps it from the new (x, y) and the
                    // sprite doesn't spring back to the old
                    // centre as soon as drag ends.
                    entity.behavior_state.bounce_invalidate();
                }
            }
        }
    }

    pub(super) fn handle_mouse_input(&mut self, state: ElementState, button: MouseButton) {
        tracing::debug!(
            "MouseInput: {:?} {:?} at ({:.0}, {:.0}) edit_mode={}",
            button,
            state,
            self.mouse_x,
            self.mouse_y,
            self.edit_mode
        );

        // Toggle ⚙ button click is handled by egui (consumed at the
        // top of window_event) → if we got here in pass-through, the
        // click was on a transparent area we don't care about.

        // Edit mode: handle entity selection, drag, and right-click context menu.
        if !self.edit_mode {
            return;
        }

        // Right-click on an entity opens the context menu and
        // selects it. Right-click on empty space does nothing
        // (entity-less menu is reserved for a later phase).
        if button == MouseButton::Right && state == ElementState::Pressed {
            if let Some(entity_idx) = self.scene.entity_at_point(self.mouse_x, self.mouse_y) {
                self.selection.select(entity_idx);
                self.ui_state.context_menu = Some(ContextMenuState {
                    entity_idx,
                    pos: egui::pos2(self.mouse_x, self.mouse_y),
                });
            }
            return;
        }

        match (button, state) {
            (MouseButton::Left, ElementState::Pressed) => {
                // Find entity under cursor
                if let Some(entity_idx) = self.scene.entity_at_point(self.mouse_x, self.mouse_y) {
                    self.selection.select(entity_idx);

                    // Start drag — freeze physics, flag the Drag
                    // animation state (U.2).
                    if let Some(entity) = self.scene.entities.get_mut(entity_idx) {
                        entity.physics.freeze();
                        entity.dragging = true;
                        let offset_x = self.mouse_x - entity.x;
                        let offset_y = self.mouse_y - entity.y;
                        self.drag.start_drag(
                            entity_idx,
                            offset_x,
                            offset_y,
                            self.mouse_x,
                            self.mouse_y,
                        );

                        tracing::info!("Clicked entity: {} ({})", entity.name, entity.id);
                    }
                } else {
                    self.selection.deselect();
                }
            }
            (MouseButton::Left, ElementState::Released) if self.drag.is_dragging() => {
                // A press-release that never moved is a *tap*, not a drag →
                // poke the mascot (recoil / hop) instead of just dropping it.
                let tapped = self.drag.was_tap(
                    self.mouse_x,
                    self.mouse_y,
                    crate::constants::POKE_TAP_RADIUS,
                );
                let screen_w = self
                    .window
                    .as_ref()
                    .map(|w| w.inner_size().width as f32)
                    .unwrap_or(1920.0);
                // Drop the freeze. Physics remains whatever the user set —
                // off by default (entity stays put), on if they pressed G.
                if let Some(idx) = self.drag.dragging_entity() {
                    if let Some(entity) = self.scene.entities.get_mut(idx) {
                        entity.physics.unfreeze();
                        entity.dragging = false;
                        if tapped {
                            entity.poke(self.mouse_x, screen_w);
                        }
                    }
                }
                self.drag.end_drag();
                self.config_dirty = true;
                self.save_config_if_needed();
            }
            _ => {}
        }
    }

    /// Scroll wheel: resize selected entity. The match arm in
    /// `window_event` only fires in edit mode, but we guard here
    /// too so future call sites can't accidentally bypass it.
    pub(super) fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        if !self.edit_mode {
            return;
        }
        let Some(idx) = self.selection.selected_index() else {
            return;
        };
        let Some(entity) = self.scene.entities.get_mut(idx) else {
            // Selection outlived the entity it pointed at (e.g. a
            // hot-reload swapped the scene between selecting and
            // scrolling) — stale index, not a bug worth a panic.
            return;
        };
        let scroll_y = match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 50.0,
        };
        let factor = if scroll_y > 0.0 { 1.1 } else { 0.9 };
        entity.scale = (entity.scale * factor).clamp(0.1, 10.0);
        tracing::debug!("Scale: {:.2}", entity.scale);
        self.config_dirty = true;
    }

    /// Drag-drop entry point. Runs the validation gate, then enters
    /// edit mode and adds the entity at the current cursor position.
    /// Import a dropped/typed Shimeji pack directory (U.4): run the
    /// importer against the asset-library root, spawn the resulting
    /// characters at the cursor, toast the summary. Skip reasons go
    /// to the log at info — the toast keeps the count only.
    pub(super) fn import_shimeji_pack(&mut self, pack: &std::path::Path) {
        let Some(library_root) = self.library_root.clone() else {
            self.toasts
                .error(crate::i18n::t("shimeji-no-library-toast"));
            return;
        };
        match crate::shimeji::import_pack(pack, &library_root) {
            Ok(report) => {
                if !self.edit_mode {
                    self.toggle_edit_mode();
                }
                let mut imported_names: Vec<String> = Vec::new();
                for mut cfg in report.characters {
                    cfg.x = self.mouse_x.max(50.0);
                    cfg.y = self.mouse_y.max(50.0);
                    // Avoid id collisions with an already-imported copy.
                    if self.scene.entities.iter().any(|e| e.id == cfg.id) {
                        cfg.id = format!("{}-{}", cfg.id, self.scene.entities.len());
                    }
                    match self.scene.append_character_config(&cfg) {
                        Ok(()) => imported_names.push(cfg.name.clone()),
                        Err(e) => {
                            tracing::warn!("Imported character '{}' rejected: {e}", cfg.name);
                            {
                                let mut args = fluent::FluentArgs::new();
                                args.set("name", cfg.name.clone());
                                args.set("error", e.to_string());
                                self.toasts
                                    .error(crate::i18n::t_args("toast-entity-load-failed", &args));
                            }
                        }
                    }
                }
                for (what, why) in &report.skipped {
                    tracing::info!("Shimeji import skip [{what}]: {why}");
                }
                if !imported_names.is_empty() {
                    let mut args = fluent::FluentArgs::new();
                    args.set("name", report.pack_name.clone());
                    args.set("n", report.skipped.len() as i64);
                    self.toasts
                        .success(crate::i18n::t_args("shimeji-imported-toast", &args));
                    self.config_dirty = true;
                    self.save_config_if_needed();
                }
            }
            Err(reason) => {
                tracing::warn!("Shimeji import failed: {reason}");
                let mut args = fluent::FluentArgs::new();
                args.set("reason", reason);
                self.toasts
                    .error(crate::i18n::t_args("shimeji-import-failed-toast", &args));
            }
        }
    }

    pub(super) fn handle_dropped_file(&mut self, path: PathBuf) {
        let label = redact_path(&path);
        tracing::info!("File dropped: {label}");
        tracing::debug!("Dropped full path: {}", path.display());

        // U.4: a dropped *directory* shaped like a Shimeji pack
        // (conf/ + img/) routes to the importer instead of the asset
        // decoders. Any other directory keeps the existing rejection.
        if path.is_dir() && path.join("conf").is_dir() && path.join("img").is_dir() {
            self.import_shimeji_pack(&path);
            return;
        }

        // Pre-validate before we hand the path to the decoders.
        // Catches the obvious bad cases (wrong extension, huge
        // file) with a fast, clear error toast instead of letting
        // the decoder spin up and fail somewhere deeper.
        if let Err(reason) = pre_validate_dropped_file(&path) {
            tracing::warn!("Rejecting dropped file {label}: {reason}");
            tracing::debug!("Rejected full path: {}", path.display());
            {
                let mut args = fluent::FluentArgs::new();
                args.set("reason", reason.clone());
                self.toasts
                    .error(crate::i18n::t_args("toast-rejected", &args));
            }
            return;
        }

        // If not in edit mode, enter it automatically
        if !self.edit_mode {
            self.toggle_edit_mode();
        }

        // Try to add the entity at the current mouse position
        match self
            .scene
            .add_entity_from_path(&path, self.mouse_x, self.mouse_y)
        {
            Ok(idx) => {
                // Create texture for the new entity
                if let Some(renderer) = &mut self.renderer {
                    renderer.ensure_texture(&self.scene.entities[idx]);
                    self.scene.entities[idx].texture_dirty = false;
                }
                // Select the new entity
                self.selection.select(idx);
                self.config_dirty = true;
                let added_name = self.scene.entities[idx].name.clone();
                self.save_config_if_needed();
                tracing::info!(
                    "Added '{}' at ({:.0}, {:.0})",
                    added_name,
                    self.mouse_x,
                    self.mouse_y
                );
                {
                    let mut args = fluent::FluentArgs::new();
                    args.set("name", added_name.clone());
                    self.toasts
                        .success(crate::i18n::t_args("toast-added", &args));
                }
            }
            Err(e) => {
                tracing::error!(
                    "Failed to load dropped file {}: {}",
                    crate::drop_validate::redact_path(&path),
                    e
                );
                {
                    let mut args = fluent::FluentArgs::new();
                    args.set("error", e.to_string());
                    self.toasts
                        .error(crate::i18n::t_args("toast-load-failed", &args));
                }
            }
        }
    }

    pub(super) fn handle_hovered_file(&mut self, path: PathBuf) {
        tracing::debug!("File hovering: {}", path.display());
    }

    /// Track all four modifiers so user-bound chords involving
    /// Alt or Super resolve correctly via `KeyBindings::lookup`.
    pub(super) fn handle_modifiers_changed(&mut self, modifiers: Modifiers) {
        self.shift_held = modifiers.state().shift_key();
        self.ctrl_held = modifiers.state().control_key();
        self.alt_held = modifiers.state().alt_key();
        self.super_held = modifiers.state().super_key();
    }
}
