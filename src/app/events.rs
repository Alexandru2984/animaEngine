//! What reaches the winit path from outside its window, as `AnimaEvent`s
//! through the event loop: the tray, the D-Bus methods, signals, global
//! shortcuts and the portal, the away and battery watches.

use super::*;

impl App {
    /// What came from outside the window: the tray, the D-Bus methods, a
    /// signal, a global shortcut, the portal, the away and battery watches.
    pub(super) fn on_event(&mut self, event_loop: &ActiveEventLoop, event: AnimaEvent) {
        match event {
            AnimaEvent::Shortcut(action, chord) => {
                // Being recorded, the shortcut is the recorder's.
                if self
                    .ui
                    .as_ref()
                    .is_some_and(|ui| ui.take_for_recording(chord))
                {
                    self.request_redraw();
                    return;
                }
                // Hide or show by what the overlay is now.
                let visible = std::sync::atomic::AtomicBool::new(!self.overlay_hidden);
                if let Some(event) = crate::hotkeys::action_to_event(action, &visible) {
                    self.user_event(event_loop, event);
                }
            }
            AnimaEvent::ToggleEditMode => {
                self.toggle_edit_mode();
            }
            AnimaEvent::ToggleGlobalPlayback => {
                self.scene.toggle_global_playback();
                self.config_dirty = true;
                let label = if self.scene.global_playing {
                    crate::i18n::t("toast-playback-resumed")
                } else {
                    crate::i18n::t("toast-playback-paused")
                };
                self.toasts.info(label);
            }
            AnimaEvent::ShowOverlay => self.set_overlay_visible(true),
            AnimaEvent::HideOverlay => self.set_overlay_visible(false),
            AnimaEvent::RaiseWindow => {
                // Someone launched a second instance. Make sure we're
                // visible and ask the WM to focus us. EWMH ABOVE keeps us
                // on top regardless; this is just a nudge.
                self.set_overlay_visible(true);
                if let Some(window) = &self.window {
                    window.focus_window();
                }
                // With the ⚙ button hidden, edit mode: the way back every
                // desktop has, with a tray and shortcuts or without.
                if !self.config.global.show_toggle_button && !self.edit_mode {
                    self.toggle_edit_mode();
                }
                tracing::info!("Raised by second-instance handshake");
            }
            AnimaEvent::ChosenAppInFront(in_front) => {
                self.chosen_app_in_front = in_front;
                self.update_step_aside();
            }
            AnimaEvent::Snooze(what) => {
                crate::snooze::snooze(what);
                // The panel goes with the rest.
                if self.edit_mode {
                    self.toggle_edit_mode();
                }
                self.update_step_aside();
            }
            AnimaEvent::ToggleMute => {
                let muted = !self.config.global.sounds_muted;
                self.on_event(event_loop, AnimaEvent::SetMuted(muted));
            }
            AnimaEvent::SetMuted(muted) => {
                self.config.global.sounds_muted = muted;
                self.config_dirty = true;
                // From the tray or a global shortcut, outside edit mode:
                // saved at once, quietly. Nothing else would save it before
                // the app quits, and a config left dirty ignores the file's
                // own edits until then.
                if !self.edit_mode {
                    match self.config_watch.save(&mut self.config, &self.scene) {
                        Ok(()) => self.config_dirty = false,
                        Err(e) => tracing::warn!("Config save failed: {e}"),
                    }
                }
            }
            AnimaEvent::Wake => {
                crate::snooze::wake();
                self.update_step_aside();
                self.request_redraw_all();
            }
            AnimaEvent::StartFocus(minutes) => {
                crate::focus::start(crate::focus::lengths_for(minutes, self.config.global.focus));
                self.request_redraw_all();
            }
            AnimaEvent::StopFocus => crate::focus::stop(),
            AnimaEvent::Quit => {
                tracing::info!("Quit requested (tray, or a signal)");
                self.save_and_exit(event_loop);
                return;
            }
            AnimaEvent::HotkeysUnavailable => {
                // Deferred hotkey resolution (portal handshake +
                // fallbacks on a background thread) ended with no
                // working backend.
                self.push_warning(Warning::GlobalHotkeysUnavailable);
                self.hotkey_backend_status = crate::hotkeys::probe::HotkeyStatus::TrayOnly;
            }
            AnimaEvent::FullscreenInFront(in_front) => {
                self.fullscreen_in_front = in_front;
                self.update_step_aside();
            }
            AnimaEvent::Away(away) => {
                tracing::info!("User {}", if away { "away" } else { "back" });
                self.away = away;
                self.update_step_aside();
                // Back: the welcome is said on the next frame (`crate::welcome`).
                if !away {
                    self.request_redraw_all();
                }
            }
            AnimaEvent::OnBattery(on_battery) => {
                tracing::info!("On battery: {on_battery}");
                self.on_battery = on_battery;
                self.update_step_aside();
            }
            AnimaEvent::IdleSource(found) => self.idle_source = Some(found),
            AnimaEvent::NextScene => {
                // Not input, so no undo step is open: open one, or a later
                // undo would take the switch back with the step before it.
                self.history
                    .input(&self.scene, &self.selection, std::time::Instant::now());
                // A drag or a rectangle holds characters about to be
                // replaced: let go first (R56).
                crate::input::multi::cancel(
                    &mut self.scene,
                    &mut self.selection,
                    &mut self.drag,
                    &mut self.marquee,
                );
                if crate::outcomes::next_scene(
                    &mut outcome_ctx!(self),
                    &mut self.config.global.active_scene,
                ) {
                    self.save_config_if_needed();
                }
            }
            AnimaEvent::PortalShortcutsDenied => {
                self.toasts
                    .warn(crate::i18n::t("portal-denied-x11-fallback-toast"));
                self.hotkey_backend_status =
                    crate::hotkeys::probe::HotkeyStatus::GrabPortalDeclined;
            }
            AnimaEvent::PortalShortcutsActive => {
                self.hotkey_backend_status = crate::hotkeys::probe::HotkeyStatus::Portal;
            }
        }
        // Every non-quit tray/hotkey action mutates visible state
        // (mode, playback, visibility) — wake the paced render loop.
        self.request_redraw();
    }
}
