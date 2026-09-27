//! UI outcome handlers — the bridges between egui panels and `App`.
//!
//! Each handler runs once per frame after `panels::settings` returns,
//! consuming the `Option<…Outcome>` the panel emitted. The applying itself
//! is shared with the native Wayland loop in [`crate::outcomes`]; what is
//! left here is what only this backend knows — its save policy and its
//! viewport.

use super::App;
use crate::outcomes;
use crate::ui::panels;

impl App {
    pub(super) fn handle_menu_outcome(&mut self, outcome: panels::ContextMenuOutcome) {
        match outcome {
            panels::ContextMenuOutcome::Open { settled } => {
                // Showing, and the click that opened it is over: a
                // *subsequent* click may now dismiss it.
                if let Some(state) = self.ui_state.context_menu.as_mut() {
                    state.armed |= settled;
                }
            }
            panels::ContextMenuOutcome::Close => {
                self.ui_state.context_menu = None;
            }
            panels::ContextMenuOutcome::Action(action) => {
                let structural = matches!(
                    action,
                    panels::MenuAction::Duplicate(_) | panels::MenuAction::Delete(_)
                );
                outcomes::apply_menu_action(action, &mut outcome_ctx!(self));
                // This backend writes adds and removals straight away
                // rather than waiting for the next save point.
                if structural {
                    self.save_config_if_needed();
                }
                self.ui_state.context_menu = None;
            }
        }
    }

    /// Delete entity `idx` and save — the `DeleteSelected` action and the
    /// context menu both come through here.
    pub(super) fn delete_entity(&mut self, idx: usize) {
        if outcomes::delete_entity(idx, &mut outcome_ctx!(self)) {
            self.save_config_if_needed();
        }
    }

    /// Duplicate entity `idx` and save.
    pub(super) fn duplicate_entity(&mut self, idx: usize) {
        if outcomes::duplicate_entity(idx, &mut outcome_ctx!(self)).is_some() {
            self.save_config_if_needed();
        }
    }

    pub(super) fn handle_library_outcome(&mut self, outcome: panels::LibraryOutcome) {
        // Drop in the middle of the visible viewport, falling back to
        // a sensible default when the window isn't fully wired yet.
        let at = self
            .window
            .as_ref()
            .map(|w| {
                let size = w.inner_size();
                (size.width as f32 / 2.0, size.height as f32 / 2.0)
            })
            .unwrap_or((400.0, 300.0));
        outcomes::apply_library_outcome(
            outcome,
            &mut outcome_ctx!(self),
            self.library_root.as_deref(),
            &mut self.library,
            at,
        );
    }

    pub(super) fn handle_palette_outcome(
        &mut self,
        outcome: panels::PaletteOutcome,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) {
        match outcome {
            // The same function its shortcut goes through.
            panels::PaletteOutcome::RunAction(action) => self.dispatch_action(action, event_loop),
            panels::PaletteOutcome::AddFile => self.open_file_chooser(),
            other => {
                outcomes::apply_palette_outcome(other, &mut outcome_ctx!(self), &mut self.config)
            }
        }
    }
}
