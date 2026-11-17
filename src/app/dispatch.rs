//! The keyboard on the winit path: a key pressed to the action bound to it
//! (`dispatch_key`, with the modifiers held), and the action run
//! (`dispatch_action`) — here when it belongs to this backend, in
//! `keybindings::shared` when both backends run it.
use super::App;
use crate::keybindings::{Action, KeyChord, KeyCode, ModifierMask};
use winit::event_loop::ActiveEventLoop;

impl App {
    /// Run the handler bound to `action`. The match preserves the
    /// per-arm behaviour previously inlined in the `KeyboardInput`
    /// match: per-entity actions silently no-op without a selection,
    /// `QuitWithSave` tears down GPU/X11 state before calling
    /// `event_loop.exit()`, etc.
    pub(super) fn dispatch_action(&mut self, action: Action, event_loop: &ActiveEventLoop) {
        if action.interrupts_drag() {
            self.config_dirty |= crate::input::multi::cancel(
                &mut self.scene,
                &mut self.selection,
                &mut self.drag,
                &mut self.marquee,
            );
        }
        match action {
            Action::ToggleEditMode => {
                self.toggle_edit_mode();
            }
            Action::QuitWithSave => {
                tracing::info!("Quit action — saving and exiting");
                self.save_and_exit(event_loop);
            }
            Action::Paste => self.paste(),
            // Actions whose runtime path lives outside the in-app
            // dispatch: HideOverlay fires only as a global hotkey;
            // OpenCommandPalette is intercepted by `panels.rs` reading
            // egui's keyboard input. Both reach this match arm when
            // a user rebinds them onto a chord that's still active in
            // edit mode — we leave the handling to the original sites
            // rather than duplicate it here.
            Action::HideOverlay | Action::OpenCommandPalette => {}
            Action::OpenContextMenu => self.open_menu_from_keyboard(),
            // As the global shortcuts do: the event each stands for now.
            Action::ToggleFocus | Action::ToggleSnooze | Action::ToggleMute => {
                let visible = std::sync::atomic::AtomicBool::new(!self.overlay_hidden);
                if let Some(event) = crate::hotkeys::action_to_event(action, &visible) {
                    winit::application::ApplicationHandler::user_event(self, event_loop, event);
                }
            }
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
                    history: &mut self.history,
                    config: &mut self.config,
                    config_watch: &mut self.config_watch,
                    perf_overlay_visible: &mut self.perf_overlay_visible,
                    renderer: self.renderer.as_mut(),
                };
                crate::keybindings::shared::dispatch_shared(other, &mut ctx);
            }
        }
    }
}

impl App {
    /// Snapshot the current modifier-key state into the bitmask shape
    /// `KeyBindings::lookup` expects. Drains the four tracked booleans
    /// into one `ModifierMask` per call site so the chord build is
    /// allocation-free.
    pub(super) fn modifier_mask(&self) -> ModifierMask {
        ModifierMask::from_state(
            self.ctrl_held,
            self.shift_held,
            self.alt_held,
            self.super_held,
        )
    }

    /// Resolve a pressed key to a bound action and dispatch it.
    ///
    /// `window_event` handles the per-monitor extra windows in one
    /// branch and the primary window in another, and both need identical
    /// chord resolution — they differ only in which redraw they request
    /// afterwards. The lookup lived in both arms, so any change to how a
    /// keypress becomes an action had to be remembered twice.
    /// `redraw_all` picks the follow-up repaint: the extra-window branch
    /// refreshes every surface, the primary one only its own. Taking it
    /// as a parameter keeps each call site a single statement — a
    /// bool-returning helper would leave a nested `if` that clippy asks
    /// to fold into the match guard, and a guard is the wrong home for a
    /// call that actually performs the action.
    pub(super) fn dispatch_key(
        &mut self,
        logical_key: winit::keyboard::Key<&str>,
        physical_key: winit::keyboard::PhysicalKey,
        event_loop: &ActiveEventLoop,
        redraw_all: bool,
    ) {
        // The number row by position, everything else by label when it has
        // one — the same rule the rebinder uses, so a chord fires from the
        // key it was recorded on whatever the layout. See `KeyCode::resolve`.
        let physical = match physical_key {
            winit::keyboard::PhysicalKey::Code(code) => KeyCode::from_physical_winit(code),
            winit::keyboard::PhysicalKey::Unidentified(_) => None,
        };
        // The Menu key, the keyboard's own key for the right-click menu,
        // does what `Shift+F10` does: not a binding, egui having no name
        // for it to record in the Keybindings tab.
        if logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::ContextMenu)
            && self.ui_state.context_menu.is_none()
        {
            self.dispatch_action(crate::keybindings::Action::OpenContextMenu, event_loop);
            return;
        }
        let Some(keycode) = KeyCode::resolve(KeyCode::from_winit(logical_key), physical) else {
            return;
        };
        // The open right-click menu takes every key, as the palette and an
        // open list do (`panels::keyboard_held`): Escape closed it and also
        // left edit mode. egui has the key already; draw the frame that
        // acts on it. A menu on another monitor's overlay has an egui that
        // hears no keys, so Escape closes that one here.
        if let Some(menu) = &self.ui_state.context_menu {
            if menu.surface.is_some()
                && keycode == KeyCode::Named(crate::keybindings::NamedKey::Escape)
            {
                self.ui_state.context_menu = None;
            }
            self.request_redraw_all();
            return;
        }
        let chord = KeyChord::new(self.modifier_mask(), keycode);
        let Some(action) = self.config.keybindings.action_for_press(chord) else {
            return;
        };
        self.dispatch_action(action, event_loop);
        if redraw_all {
            self.request_redraw_all();
        } else {
            self.request_redraw();
        }
    }
}
