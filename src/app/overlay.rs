//! The overlay window itself: shown or hidden, above the windows or
//! below, the X11 input shape that lets clicks through, stepping aside for
//! a full-screen window, and edit mode on and off.

use super::*;

impl App {
    /// Show or hide **every** overlay surface — the primary window and
    /// each per-monitor extra.
    ///
    /// Hide/Show used to touch only `self.window`, so in `PerMonitor`
    /// mode hiding the overlay left every secondary monitor's sprites on
    /// screen: the overlay was only partly hidden. Compositors also
    /// sometimes clip our input shape across an unmap/map cycle, so the
    /// shape is re-applied on the way back in — for the extras too.
    pub(super) fn set_overlay_visible(&mut self, visible: bool) {
        self.overlay_hidden = !visible;
        // Asked to show them: that ends a snooze too (`crate::snooze`).
        if visible {
            crate::snooze::wake();
            self.update_step_aside();
        }
        self.sync_window_visibility();
    }

    /// Map or unmap every overlay window to match the user's hide and
    /// stepping aside for a full-screen app together.
    pub(super) fn sync_window_visibility(&mut self) {
        let visible = !self.overlay_hidden && !self.stepped_aside.hidden;
        // Back in sight, everyone fades in (`Scene::fade_all_in`).
        if visible && self.was_hidden {
            self.scene.fade_all_in();
        }
        self.was_hidden = !visible;
        if let Some(window) = &self.window {
            window.set_visible(visible);
        }
        // Mapped again, a window has no contents until it draws.
        self.primary_frame = None;
        for slot in self.extra_windows.values_mut() {
            slot.window.set_visible(visible);
            slot.last_frame = None;
        }
        if visible {
            self.reapply_input_shape();
            self.reapply_extra_input_shapes();
        } else {
            // An unmapped window gets no release: let go of a drag now, or
            // the character jumps to the pointer at the first move after
            // it shows again.
            self.config_dirty |= crate::input::multi::cancel(
                &mut self.scene,
                &mut self.selection,
                &mut self.drag,
                &mut self.marquee,
            );
        }
    }

    /// Push the X11 input shape that matches the current `edit_mode`.
    ///
    /// Must be called any time the shape can desync from reality: mode
    /// toggle, window resize, regaining focus or visibility (compositors
    /// like Mutter occasionally clip the shape after fractional-scaling
    /// transitions or after the window is minimized and restored).
    pub(super) fn reapply_input_shape(&mut self) {
        // Read the scale before taking the `&mut self.x11_input` borrow;
        // no ⚙ corner when the button is hidden.
        let button_px = if self.config.global.show_toggle_button {
            toggle_button_px(
                self.window
                    .as_ref()
                    .map(|w| w.scale_factor())
                    .unwrap_or(1.0),
            )
        } else {
            0
        };
        if let Some(x11) = &mut self.x11_input {
            let result = if self.edit_mode {
                x11.set_full_input()
            } else {
                // The ⚙ corner, and the characters when they take taps
                // (`crate::tap`).
                x11.set_passthrough_with_rects(button_px, &self.tap_rects)
            };
            if let Err(e) = result {
                tracing::warn!("Failed to apply input shape: {}", e);
                // Fall back to winit's cursor-hittest so we never end up
                // in a totally unclickable state.
                if let Some(window) = &self.window {
                    let _ = window.set_cursor_hittest(self.edit_mode);
                }
            }
        } else if let Some(window) = &self.window {
            // No X11 manager available — winit fallback only.
            let _ = window.set_cursor_hittest(self.edit_mode);
        }
    }

    /// Re-assert always-on-top (X11 `_NET_WM_STATE_ABOVE`).
    ///
    /// Mutter (GNOME on XWayland) drops the ABOVE state when another window
    /// is focused over us, sinking the overlay behind it; re-sending the
    /// EWMH request on every focus/occlusion transition keeps it floating
    /// on top. No-op without an X11 manager (winit-only fallback, or the
    /// native Wayland path) — there's no portable equivalent, and the
    /// native Wayland path gets always-on-top from the layer surface
    /// instead.
    pub(super) fn reassert_always_on_top(&self) {
        // Behind the windows by choice: nothing to keep above.
        if !self.on_top() {
            return;
        }
        if let Some(x11) = &self.x11_input {
            if let Err(e) = x11.reassert_above() {
                tracing::debug!("Re-assert always-on-top failed: {e}");
            }
        }
    }

    /// Toggle between edit mode and pass-through mode
    /// Step aside while a full-screen app is in front, and come back when
    /// it is not (`crate::fullscreen`). Recomputed from the setting, the
    /// watch's last report and edit mode, so nothing has to be undone;
    /// called on each report, on an edit-mode change and every frame.
    /// Whether the overlay is above other windows: unless the user put it
    /// behind them — and in edit mode always, so the panel is in reach.
    pub(super) fn on_top(&self) -> bool {
        self.config.global.always_on_top || self.edit_mode
    }

    /// Put every window above the others or under them, as `on_top` says.
    /// Cheap when nothing changes; called every frame.
    pub(super) fn apply_stacking(&mut self) {
        let on_top = self.on_top();
        if self.stacked_on_top == Some(on_top) || self.window.is_none() {
            return;
        }
        let level = if on_top {
            WindowLevel::AlwaysOnTop
        } else {
            WindowLevel::AlwaysOnBottom
        };
        if let Some(window) = &self.window {
            window.set_window_level(level);
        }
        for slot in self.extra_windows.values() {
            slot.window.set_window_level(level);
        }
        if on_top {
            self.reassert_always_on_top();
        }
        self.stacked_on_top = Some(on_top);
        tracing::info!(
            "Overlay {}",
            if on_top {
                "above windows"
            } else {
                "behind windows"
            }
        );
    }

    pub(super) fn update_step_aside(&mut self) {
        self.apply_stacking();
        let wanted = crate::fullscreen::step_aside(
            self.config.global.on_fullscreen,
            self.fullscreen_in_front,
            self.edit_mode,
        )
        // An app the user chose in front (`crate::chosen_apps`).
        .or_chosen_app(self.chosen_app_in_front, self.edit_mode)
        // Snoozing (`crate::snooze`): out of sight and still until it ends.
        .or_snoozing(crate::snooze::active());
        crate::chosen_apps::configure(&self.config.global.step_aside_apps);
        // Bedtime by the clock (`crate::bedtime`) — not while editing.
        self.bedtime_now = !self.edit_mode && self.bedtime_watch.now(&self.config.global.bedtime);
        // Characters with hours of their own (`crate::hours`) — everyone
        // while editing.
        let minute = if self.edit_mode {
            None
        } else {
            self.hours_clock.minute()
        };
        if self.scene.set_clock(minute) {
            self.request_redraw_all();
        }
        // A global shortcut changed in the Keybindings tab or by a hot
        // reload is grabbed anew (`crate::hotkeys::follow`).
        crate::hotkeys::follow(&self.config.keybindings);
        // Away or on battery, if asked: held still too (`crate::away`).
        let global = &self.config.global;
        crate::away::configure(
            crate::away::idle_watch_minutes(
                global.pause_when_idle_minutes,
                !self.config.reminders.is_empty() || global.welcome_back,
            ),
            global.pause_on_battery || global.frame_rate.watches_battery(),
        );
        let still = crate::away::stillness(
            global.pause_when_idle_minutes,
            self.away,
            global.pause_on_battery,
            self.on_battery,
        )
        .with_bedtime(self.bedtime_now)
        .watching(self.fullscreen_in_front);
        // Asleep, then held still: the Zzz… and the landing want frames.
        if self.scene.set_dozing(still.doze) {
            self.request_redraw_all();
        }
        self.scene.set_suspended(wanted.paused || still.freeze);
        if wanted != self.stepped_aside {
            self.stepped_aside = wanted;
            tracing::info!(
                "{}: overlay {}",
                if crate::snooze::active() {
                    "Snoozing"
                } else if self.chosen_app_in_front && !self.edit_mode {
                    "An app chosen in Appearance in front"
                } else if self.fullscreen_in_front {
                    "Full-screen app in front"
                } else {
                    "Full-screen app gone"
                },
                if wanted.hidden {
                    "hidden"
                } else if wanted.paused {
                    "paused"
                } else {
                    "back"
                }
            );
            self.sync_window_visibility();
        }
    }

    pub(super) fn toggle_edit_mode(&mut self) {
        self.edit_mode = !self.edit_mode;
        // Editing wants them in view: it ends a snooze (`crate::snooze`),
        // or the panel would open hidden.
        if self.edit_mode {
            crate::snooze::wake();
        }
        self.update_step_aside();
        self.reapply_input_shape();
        // PerMonitor extras flip their input regions in lockstep —
        // edit mode is global across every overlay window (T.6).
        self.reapply_extra_input_shapes();
        self.request_redraw_all();

        if self.edit_mode {
            tracing::info!(
                "━━━ EDIT MODE ON ━━━ Click and drag characters. Press Escape or click ⚙ button to exit."
            );
        } else {
            tracing::info!(
                "━━━ PASS-THROUGH MODE ━━━ Clicks go to desktop. Click ⚙ button to enter edit mode."
            );
            // Let go of anything the pointer held when leaving edit mode —
            // a drag of the whole selection, a selection rectangle. Ending
            // the drag alone left the characters frozen and in their Drag
            // state.
            self.config_dirty |= crate::input::multi::cancel(
                &mut self.scene,
                &mut self.selection,
                &mut self.drag,
                &mut self.marquee,
            );
            self.selection.deselect();
            self.ui_state.context_menu = None;

            // Auto-save any pending changes when exiting edit mode
            if self.config_dirty {
                self.save_config_if_needed();
            }
        }
    }
}
