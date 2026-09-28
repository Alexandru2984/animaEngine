//! Pointer + drag-drop event handlers. Extracted in H.4b so the
//! match in `App::window_event` reads as a short delegation table
//! instead of mixing rendering, IO and input concerns in one body.
//!
//! Keyboard is intentionally NOT here — it routes through
//! `dispatch_action` (see `src/app/dispatch.rs`) which is its own
//! concern. This module only owns mouse, scroll, drag-drop and
//! modifier tracking.

use super::{App, ContextMenuState};
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

        // A drag moves the whole selection; a rectangle over empty space
        // selects what it touches (`crate::input::multi`).
        if self.edit_mode {
            if let Some(marquee) = self.marquee.as_mut() {
                marquee.drag_to((gx, gy), &self.scene, &mut self.selection);
            } else {
                crate::input::multi::drag_to(
                    &mut self.scene,
                    &self.selection,
                    &self.drag,
                    (gx, gy),
                );
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
                // One of several selected keeps them all, so the menu acts
                // on the whole selection; otherwise it takes the character
                // with its group.
                crate::input::multi::select_for_menu(&self.scene, &mut self.selection, entity_idx);
                // egui draws in the primary window's own coordinates, and
                // `mouse_x/y` are global: without the origin a primary
                // monitor that is not at 0,0 put the menu off to one
                // side. A right-click on another monitor lands outside
                // the primary; egui then keeps the menu on screen, at
                // the primary's edge nearest the click.
                let (ox, oy) = self.primary_origin();
                self.ui_state.context_menu = Some(ContextMenuState {
                    entity_idx,
                    pos: egui::pos2(self.mouse_x - ox, self.mouse_y - oy),
                    // Armed after the first showing — see ContextMenuState.
                    armed: false,
                });
            }
            return;
        }

        match (button, state) {
            (MouseButton::Left, ElementState::Pressed) => {
                let at = (self.mouse_x, self.mouse_y);
                match self.scene.entity_at_point(at.0, at.1) {
                    // Select (with its group, or keeping the others selected,
                    // or toggled with Shift) and pick the selection up.
                    Some(entity_idx) => {
                        crate::input::multi::press_on(
                            &mut self.scene,
                            &mut self.selection,
                            &mut self.drag,
                            entity_idx,
                            at,
                            self.shift_held,
                        );
                        if let Some(entity) = self.scene.entities.get(entity_idx) {
                            tracing::info!("Clicked entity: {} ({})", entity.name, entity.id);
                        }
                    }
                    // Empty space: a selection rectangle, or with no drag
                    // a click that deselects.
                    None => {
                        self.marquee = Some(crate::input::multi::Marquee::begin(
                            at,
                            self.shift_held,
                            &mut self.selection,
                        ));
                    }
                }
            }
            (MouseButton::Left, ElementState::Released) if self.marquee.is_some() => {
                self.marquee = None;
            }
            (MouseButton::Left, ElementState::Released) if self.drag.is_dragging() => {
                // A press-release that never moved is a *tap*, not a drag →
                // poke the mascot (recoil / hop) instead of just dropping it.
                let tapped = self.drag.was_tap(
                    self.mouse_x,
                    self.mouse_y,
                    crate::constants::POKE_TAP_RADIUS,
                );
                if tapped {
                    // The hop that follows is play, not an edit.
                    self.history.finish(&self.scene);
                }
                // Same bounds the simulation tick uses, so a poke can't
                // shove a mascot off the region it's allowed to occupy —
                // and, on a multi-monitor desktop, doesn't clamp one that
                // lives on a secondary monitor back onto the primary.
                let fallback = self
                    .window
                    .as_ref()
                    .map(|w| {
                        let s = w.inner_size();
                        (s.width as f32, s.height as f32)
                    })
                    .unwrap_or((1920.0, 1080.0));
                let poke_bounds = crate::monitor::covered_bounds(
                    &crate::monitor::plan_windows(&self.config.global.monitor_mode, &self.monitors),
                    fallback,
                );
                // Drop the freeze. Physics remains whatever the user set —
                // off by default (entity stays put), on if they pressed G.
                if tapped {
                    if let Some(entity) = self
                        .drag
                        .dragging_entity()
                        .and_then(|idx| self.scene.entities.get_mut(idx))
                    {
                        entity.poke(self.mouse_x, poke_bounds);
                    }
                }
                crate::input::multi::end_drag(
                    &mut self.scene,
                    &mut self.selection,
                    &mut self.drag,
                    tapped,
                    self.shift_held,
                );
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
        let scroll_y = match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 50.0,
        };
        let factor = if scroll_y > 0.0 { 1.1 } else { 0.9 };
        // Every selected character, by the same factor. An index that
        // outlived its entity (a hot-reload swapped the scene between
        // selecting and scrolling) is skipped, not a panic.
        for idx in self.selection.selected_indices() {
            if let Some(entity) = self.scene.entities.get_mut(idx) {
                entity.scale = (entity.scale * factor).clamp(0.1, 10.0);
                tracing::debug!("Scale: {:.2}", entity.scale);
                self.config_dirty = true;
            }
        }
    }

    /// Start importing a Shimeji pack directory (U.4) off the UI thread,
    /// to land at `at`. One import at a time — a second pack dropped while
    /// one is in flight is ignored, and the first still lands its toast.
    pub(super) fn import_shimeji_pack(&mut self, pack: &std::path::Path, at: (f32, f32)) {
        if self.pending_shimeji.is_some() {
            return;
        }
        self.pending_shimeji = crate::outcomes::ShimejiImport::start(
            pack,
            self.library_root.as_deref(),
            at,
            &mut self.toasts,
        );
    }

    /// "Add file…": open the desktop's file chooser. What is picked lands
    /// in the middle of the primary window. One dialog at a time.
    pub(super) fn open_file_chooser(&mut self) {
        if self.pending_file_chooser.is_some() {
            return;
        }
        let (w, h) = self
            .window
            .as_ref()
            .map(|w| {
                let s = w.inner_size();
                (s.width as f32, s.height as f32)
            })
            .unwrap_or((1920.0, 1080.0));
        let (ox, oy) = self.primary_origin();
        self.pending_file_chooser = Some(crate::outcomes::FileChooserAdd::start((
            w / 2.0 + ox,
            h / 2.0 + oy,
        )));
    }

    /// Apply the file chooser's answer once it closes. Called once per
    /// frame beside the Shimeji check.
    pub(super) fn check_file_chooser(&mut self) {
        let Some(chooser) = &self.pending_file_chooser else {
            return;
        };
        let Some(added) = chooser.poll(&mut outcome_ctx!(self)) else {
            return;
        };
        self.pending_file_chooser = None;
        if added > 0 {
            self.save_config_if_needed();
        }
    }

    /// Apply a finished Shimeji import. Called once per frame beside the
    /// hot-reload check.
    pub(super) fn check_shimeji_import(&mut self) {
        let Some(import) = &self.pending_shimeji else {
            return;
        };
        let Some(added) = import.poll(&mut outcome_ctx!(self)) else {
            return;
        };
        self.pending_shimeji = None;
        if added > 0 {
            if !self.edit_mode {
                self.toggle_edit_mode();
            }
            self.save_config_if_needed();
        }
    }

    pub(super) fn handle_dropped_file(&mut self, path: PathBuf) {
        // Where the file was actually dropped.
        //
        // winit's `DroppedFile` carries a path and nothing else: XDND does
        // send the position and winit does parse it, but the API has
        // nowhere to put it (winit's own source says so). Falling back to
        // the last `CursorMoved` is wrong during a drag, because the drag
        // source holds a pointer grab and the overlay receives no motion —
        // so the character landed wherever the cursor had been *before*
        // the drag started, which in pass-through mode is the ⚙ corner or
        // nowhere at all (R37). Asking the X server closes that gap; root
        // coordinates are the same global desktop space `mouse_x` uses.
        //
        // The native Wayland path needs none of this: `wl_data_device`
        // delivers motion during the drag, so that backend has always
        // placed the drop correctly.
        let at = drop_position().unwrap_or((self.mouse_x, self.mouse_y));

        // A Shimeji pack folder goes to the importer; anything else to the
        // decoders, through the same code the Wayland loop uses.
        if crate::outcomes::is_shimeji_pack(&path) {
            self.import_shimeji_pack(&path, at);
            return;
        }
        if crate::outcomes::add_dropped_file(&path, at, &mut outcome_ctx!(self)).is_some() {
            if !self.edit_mode {
                self.toggle_edit_mode();
            }
            self.save_config_if_needed();
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

/// Pointer position at drop time, in global desktop coordinates.
///
/// `None` on anything but X11, and on X11 when the query fails — the
/// caller then keeps the old last-known-cursor behaviour rather than
/// dropping the file somewhere arbitrary.
fn drop_position() -> Option<(f32, f32)> {
    #[cfg(unix)]
    {
        crate::window::linux::pointer_root_position()
    }
    #[cfg(not(unix))]
    {
        None
    }
}
