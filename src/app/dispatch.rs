//! Keyboard action dispatch — the 27-arm match originally inlined in
//! `app.rs`. Extracted in H.1 so the parent module's
//! `ApplicationHandler` impl stays readable; nothing about per-arm
//! behaviour changes here.

use super::App;
use crate::keybindings::Action;
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
                self.save_and_exit(event_loop);
            }
            Action::SaveNow => {
                self.config_dirty = true;
                self.save_config_if_needed();
                tracing::info!("Config saved manually");
            }
            Action::DeleteSelected => {
                if let Some(idx) = self.selection.selected_index() {
                    self.delete_entity(idx);
                }
            }
            // Arrow nudges: Shift = 1 px fine, normal = 10 px. Every
            // nudge invalidates Bounce rest so the entity doesn't
            // snap back after the keypress. All selection-driven arms
            // go through `get_mut` — the deselect-on-removal invariant
            // holds everywhere today, but a panic on a stale index is
            // the wrong failure mode for a keypress either way.
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
                            {
                                let mut args = fluent::FluentArgs::new();
                                args.set("error", e.to_string());
                                self.toasts
                                    .error(crate::i18n::t_args("toast-duplicate-failed", &args));
                            }
                        }
                    }
                }
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
            // Everything else is backend-independent and lives in
            // `keybindings::shared`, so the native Wayland loop can run it
            // too — it previously handled one action out of twenty-eight.
            other => {
                let bounds = self
                    .window
                    .as_ref()
                    .map(|w| {
                        let s = w.inner_size();
                        crate::monitor::DesktopBounds::from_size(s.width as f32, s.height as f32)
                    })
                    .unwrap_or_else(|| crate::monitor::DesktopBounds::from_size(1920.0, 1080.0));
                let mut ctx = crate::keybindings::shared::ActionCtx {
                    scene: &mut self.scene,
                    selection: &mut self.selection,
                    config_dirty: &mut self.config_dirty,
                    shift_held: self.shift_held,
                    bounds,
                    monitors: &self.monitors,
                    toasts: &mut self.toasts,
                };
                crate::keybindings::shared::dispatch_shared(other, &mut ctx);
            }
        }
    }
}
