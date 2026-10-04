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

    /// Delete every selected entity and save.
    pub(super) fn delete_selected(&mut self) {
        let targets = self.selection.selected_indices();
        if outcomes::delete_entities(&targets, &mut outcome_ctx!(self)) > 0 {
            self.save_config_if_needed();
        }
    }

    /// Duplicate every selected entity and save.
    pub(super) fn duplicate_selected(&mut self) {
        let targets = self.selection.selected_indices();
        if !outcomes::duplicate_entities(&targets, &mut outcome_ctx!(self)).is_empty() {
            self.save_config_if_needed();
        }
    }

    /// Cut every selected entity and save.
    pub(super) fn cut_selected(&mut self) {
        let targets = self.selection.selected_indices();
        if outcomes::cut_entities(&targets, &mut outcome_ctx!(self)) > 0 {
            self.save_config_if_needed();
        }
    }

    /// Paste: ask the desktop's clipboard first — an image or files
    /// copied elsewhere since the last copy here come in as characters —
    /// and otherwise the characters copied here (`crate::system_clipboard`).
    /// Lands at the pointer.
    pub(super) fn paste(&mut self) {
        #[cfg(unix)]
        {
            let at = (self.mouse_x, self.mouse_y);
            if let Some(clipboard) = self.x11_clipboard() {
                let answer = clipboard.read(crate::system_clipboard::Want::Characters);
                if self.pending_paste.is_none() {
                    self.pending_paste = Some((answer, at));
                }
                return;
            }
        }
        if outcomes::paste_copied(&mut outcome_ctx!(self)) {
            self.save_config_if_needed();
        }
    }

    /// The panel's text fields and the desktop's clipboard: text copied
    /// there goes onto it — as plain text, so a Ctrl+V on the overlay does
    /// not take it for characters — and Ctrl+V there brings its text in.
    fn check_text_fields_clipboard(&mut self) {
        let (wants_paste, copied) = match self.ui.as_mut() {
            Some(ui) => (ui.take_paste_request(), ui.take_copied_text()),
            None => (false, None),
        };
        #[cfg(unix)]
        {
            if let Some(text) = copied {
                if let Some(clipboard) = self.x11_clipboard() {
                    clipboard.claim(text, false);
                }
            }
            if wants_paste && self.pending_text_paste.is_none() {
                self.pending_text_paste = self
                    .x11_clipboard()
                    .map(|c| c.read(crate::system_clipboard::Want::Text));
            }
        }
        #[cfg(not(unix))]
        let _ = (wants_paste, copied);
        let Some(answer) = &self.pending_text_paste else {
            return;
        };
        let content = match answer.try_recv() {
            Ok(content) => content,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                crate::system_clipboard::Content::Nothing
            }
        };
        self.pending_text_paste = None;
        if let (crate::system_clipboard::Content::Text(text), Some(ui)) =
            (content, self.ui.as_mut())
        {
            ui.paste_text(text);
            self.request_redraw();
        }
    }

    /// The clipboard thread, started at first use.
    #[cfg(unix)]
    fn x11_clipboard(&mut self) -> Option<&crate::x11_clipboard::X11Clipboard> {
        self.x11_clipboard
            .get_or_insert_with(crate::x11_clipboard::X11Clipboard::start)
            .as_ref()
    }

    /// Hand a copy's names to the desktop's clipboard, and apply a paste
    /// once the clipboard has answered. Called once per frame.
    pub(super) fn check_clipboard(&mut self) {
        self.check_text_fields_clipboard();
        if let Some(text) = crate::clipboard::take_claim() {
            #[cfg(unix)]
            if let Some(clipboard) = self.x11_clipboard() {
                clipboard.claim(text, true);
            }
            #[cfg(not(unix))]
            let _ = text;
        }
        let Some((answer, at)) = &self.pending_paste else {
            return;
        };
        let content = match answer.try_recv() {
            Ok(content) => content,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                crate::system_clipboard::Content::Nothing
            }
        };
        let at = *at;
        self.pending_paste = None;
        // The key that asked may be a while ago: open a step if it closed.
        self.history
            .input(&self.scene, &self.selection, std::time::Instant::now());
        let pasted = outcomes::apply_pasted(content, at, &mut outcome_ctx!(self));
        if pasted.added {
            self.save_config_if_needed();
        }
        // A shared scene or a Shimeji pack copied in a file manager goes
        // where a drop of it would.
        for path in pasted.to_import {
            if crate::scene_file::is_scene_file(&path) {
                self.import_scene(Some(path));
            } else {
                self.import_shimeji_pack(&path, at);
            }
        }
        self.request_redraw_all();
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
