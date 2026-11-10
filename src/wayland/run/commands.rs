//! What reaches the overlay from outside: files dropped on it, and the
//! D-Bus methods, the tray, signals and the portal's shortcuts
//! ([`super::Native::iteration`]).

use super::*;

impl Native {
    /// Files dropped on an overlay: a Shimeji pack to the importer, a
    /// shared scene to its import, anything else added where it landed.
    pub(super) fn drops(&mut self, primary_origin: (f32, f32), activity: &mut bool) {
        let Self {
            history,
            pending_transfer,
            pending_shimeji,
            library_root,
            config_watch,
            config_dirty,
            toasts,
            selection,
            renderer,
            layer,
            config,
            scene,
            ..
        } = self;
        // Files dropped on the overlay (E.3). Each drop carries the point
        // it landed on in the primary surface's space (shifted there from
        // an extra surface's); `primary_origin` turns that into the global
        // space the scene uses. Same as the winit path,
        // through the same code: a Shimeji pack folder goes to the
        // importer, anything else is validated, added, selected, toasted
        // and marked for saving.
        for dropped in layer.drain_dropped_files() {
            *activity = true;
            history.input(scene, selection, Instant::now());
            let at = (
                dropped.at.0 + primary_origin.0,
                dropped.at.1 + primary_origin.1,
            );
            let mut added = false;
            for path in &dropped.paths {
                if outcomes::is_shimeji_pack(path) {
                    if pending_shimeji.is_none() {
                        *pending_shimeji =
                            ShimejiImport::start(path, library_root.as_deref(), at, &mut *toasts);
                    }
                } else if crate::scene_file::is_scene_file(path) {
                    // A shared scene replaces this one.
                    if pending_transfer.is_none() {
                        *pending_transfer = Some(crate::outcomes::SceneTransfer::import(
                            path.clone(),
                            Some(at),
                        ));
                    }
                } else if outcomes::add_dropped_file(
                    path,
                    at,
                    &mut outcome_ctx!(scene, selection, toasts, config_dirty, renderer),
                )
                .is_some()
                {
                    added = true;
                }
            }
            // As on winit, a drop opens edit mode so the new character can
            // be moved into place straight away. It matters more here: in
            // pass-through the overlay only takes input — drops included —
            // on the ⚙ corner, so that is where a drop lands.
            if added && !layer.state.edit_mode {
                flip_edit_mode(
                    &mut *layer,
                    &mut *config,
                    scene,
                    &mut *config_dirty,
                    &mut *config_watch,
                    "Wayland, file drop",
                );
            }
        }
    }

    /// What came from outside: the D-Bus methods and the tray, a signal,
    /// a second start, the portal's global shortcuts.
    pub(super) fn commands(&mut self, activity: &mut bool) {
        let Self {
            quit_requested,
            drag_widened,
            on_battery,
            aside,
            history,
            config_watch,
            config_dirty,
            toasts,
            marquee,
            drag,
            selection,
            egui_renderer,
            renderer,
            layer,
            hotkey_backend_status,
            overlay_hidden,
            portal_rx,
            command_rx,
            config,
            scene,
            ..
        } = self;
        // Drain any D-Bus actions arriving from compositor bindings
        // (E.6). Each event maps onto the same surface the X11 path's
        // global hotkeys produce, so a `gdbus call … ToggleEditMode`
        // invoked from sway is indistinguishable from clicking the ⚙
        // button.
        //
        // F.3: coalesce idempotent toggle events so a spammy caller
        // (a thousand `ToggleEditMode` calls between two frames =
        // odd parity → noop = even parity → toggle) doesn't bounce
        // the input region a thousand times — we apply each toggle
        // class at most once per frame. ShowOverlay / HideOverlay are
        // distinct because the user might want either intent.
        {
            let mut toggle_edit_xor = false;
            let mut open_edit = false;
            let mut toggle_playback_xor = false;
            let mut last_visibility: Option<AnimaEvent> = None;
            let mut quit = false;
            let mut next_scene = false;
            let mut snoozing = false;
            let mut mute_changed = false;
            if let Some(rx) = &command_rx {
                while let Ok(ev) = rx.try_recv() {
                    *activity = true;
                    // A global shortcut: the recorder's while one is being
                    // recorded, else the event it stands for now.
                    let ev = match ev {
                        AnimaEvent::Shortcut(action, chord) => {
                            if egui_renderer.take_for_recording(chord) {
                                continue;
                            }
                            let visible = std::sync::atomic::AtomicBool::new(!*overlay_hidden);
                            match crate::hotkeys::action_to_event(action, &visible) {
                                Some(ev) => ev,
                                None => continue,
                            }
                        }
                        other => other,
                    };
                    match ev {
                        AnimaEvent::Shortcut(..) => {}
                        AnimaEvent::ToggleEditMode => toggle_edit_xor ^= true,
                        AnimaEvent::ToggleGlobalPlayback => toggle_playback_xor ^= true,
                        AnimaEvent::HideOverlay => last_visibility = Some(AnimaEvent::HideOverlay),
                        AnimaEvent::ShowOverlay => last_visibility = Some(AnimaEvent::ShowOverlay),
                        AnimaEvent::Quit => {
                            quit = true;
                            tracing::info!("Quit requested (tray, or a signal)");
                        }
                        // A second start from the menu: a layer surface has
                        // no focus to take, but one hidden from the tray (or
                        // snoozed) comes back, as on X11 — before, nothing
                        // happened and the start looked broken.
                        AnimaEvent::RaiseWindow => {
                            last_visibility = Some(AnimaEvent::ShowOverlay);
                            open_edit |= !config.global.show_toggle_button;
                            tracing::info!("Raised by second-instance handshake");
                        }
                        // Hotkey resolution events are winit-path UI; the
                        // native path logs the outcome where it resolves.
                        AnimaEvent::HotkeysUnavailable
                        | AnimaEvent::PortalShortcutsDenied
                        | AnimaEvent::PortalShortcutsActive => {}
                        // This loop learns it from the compositor's
                        // toplevel list instead.
                        AnimaEvent::FullscreenInFront(_) | AnimaEvent::ChosenAppInFront(_) => {}
                        AnimaEvent::OnBattery(now) => *on_battery = now,
                        // Idle comes from the compositor here.
                        AnimaEvent::Away(_) | AnimaEvent::IdleSource(_) => {}
                        AnimaEvent::NextScene => next_scene = true,
                        AnimaEvent::Snooze(what) => {
                            crate::snooze::snooze(what);
                            snoozing = true;
                        }
                        AnimaEvent::Wake => crate::snooze::wake(),
                        AnimaEvent::StartFocus(minutes) => {
                            crate::focus::start(crate::focus::lengths_for(
                                minutes,
                                config.global.focus,
                            ));
                            *activity = true;
                        }
                        AnimaEvent::StopFocus => crate::focus::stop(),
                        AnimaEvent::SetMuted(muted) => {
                            config.global.sounds_muted = muted;
                            *config_dirty = true;
                            mute_changed = true;
                        }
                        AnimaEvent::ToggleMute => {
                            config.global.sounds_muted = !config.global.sounds_muted;
                            *config_dirty = true;
                            mute_changed = true;
                        }
                    }
                }
            }
            // Portal shortcut activations land in the same per-frame
            // accumulators as the D-Bus methods — a Ctrl+Shift+A from
            // the portal is indistinguishable from `gdbus call …
            // ToggleEditMode`. T.2: the portal is the first mechanism
            // that gives the *native* path real global hotkeys.
            if let Some(rx) = &portal_rx {
                use crate::hotkeys::portal::PortalMsg;
                use crate::keybindings::Action as KbAction;
                while let Ok(msg) = rx.try_recv() {
                    *activity = true;
                    match msg {
                        PortalMsg::Ready => {
                            tracing::info!("Portal shortcuts active (native path)");
                            *hotkey_backend_status = HotkeyStatus::Portal;
                        }
                        PortalMsg::Failed => {
                            tracing::warn!(
                                "Portal shortcuts unavailable — compositor \
                                 bindings via D-Bus remain the fallback"
                            );
                            toasts.warn(crate::i18n::t("portal-denied-native-toast"));
                            *hotkey_backend_status = HotkeyStatus::CompositorBindings;
                        }
                        // The desktop owns the keys: which chord, it does
                        // not say, so a recording takes nothing — but the
                        // action waits while one is under way.
                        PortalMsg::Activated(_) if egui_renderer.take_for_recording(None) => {}
                        PortalMsg::Activated(action) => match action {
                            KbAction::ToggleFocus | KbAction::ToggleSnooze => {
                                snoozing |= timer_shortcut(action, config.global.focus);
                            }
                            KbAction::ToggleMute => {
                                config.global.sounds_muted = !config.global.sounds_muted;
                                *config_dirty = true;
                                mute_changed = true;
                            }
                            KbAction::ToggleEditMode => toggle_edit_xor ^= true,
                            KbAction::PauseAll => toggle_playback_xor ^= true,
                            KbAction::HideOverlay => {
                                *overlay_hidden = !*overlay_hidden;
                                last_visibility = Some(if *overlay_hidden {
                                    AnimaEvent::HideOverlay
                                } else {
                                    AnimaEvent::ShowOverlay
                                });
                            }
                            _ => {}
                        },
                    }
                }
            }
            // Snoozing out of edit mode: the panel goes with the rest.
            toggle_edit_xor ^= snoozing && layer.state.edit_mode;
            // Started again with the ⚙ button hidden: edit mode, the way
            // back every desktop has, with a tray and shortcuts or without.
            if open_edit {
                toggle_edit_xor = !layer.state.edit_mode;
            }
            // Muted or unmuted from the tray or a global shortcut: saved at
            // once outside edit mode. Nothing else would save it before the
            // app quits, and a config left dirty ignores the file's own
            // edits until then.
            if mute_changed && !layer.state.edit_mode {
                match sync_and_save(&mut *config, scene) {
                    Ok(()) => {
                        *config_dirty = false;
                        config_watch.note_saved();
                    }
                    Err(e) => tracing::warn!("Config save failed: {e}"),
                }
            }
            if toggle_edit_xor {
                // Through `flip_edit_mode`, like the keyboard and the ⚙
                // button. This path used to set the mode directly, so
                // leaving edit mode from the tray, a D-Bus call or a portal
                // shortcut skipped the save — R30 on a third entry point.
                flip_edit_mode(
                    &mut *layer,
                    &mut *config,
                    scene,
                    &mut *config_dirty,
                    &mut *config_watch,
                    "Wayland, tray / D-Bus / portal",
                );
            }
            if toggle_playback_xor {
                scene.toggle_global_playback();
                *config_dirty = true;
                // Said, as on the winit path: from the tray, a shortcut or
                // D-Bus there is nothing else to show it happened.
                toasts.info(crate::i18n::t(if scene.global_playing {
                    "toast-playback-resumed"
                } else {
                    "toast-playback-paused"
                }));
            }
            if let Some(vis) = last_visibility {
                // `overlay_hidden` is the single source of truth and it
                // gates rendering below. It used to be updated only by the
                // keybinding path, so a D-Bus Hide/Show left the flag and
                // the actual intent disagreeing.
                match vis {
                    AnimaEvent::HideOverlay => *overlay_hidden = true,
                    AnimaEvent::ShowOverlay => {
                        *overlay_hidden = false;
                        // That ends a snooze too (`crate::snooze`).
                        crate::snooze::wake();
                    }
                    _ => {}
                }
                // A layer surface can't be unmapped the way winit's
                // `set_visible(false)` unmaps a window, so "hidden" is
                // expressed as: paint nothing (see the render gate) and
                // accept no input. Previously hide only narrowed the input
                // region, which left every sprite fully visible on screen —
                // the overlay was not hidden in any sense the user would
                // recognise.
                let region = region_for_state(
                    *overlay_hidden || aside.hidden,
                    layer.state.edit_mode || *drag_widened,
                    renderer.primary.window_width,
                    renderer.primary.window_height,
                    shown_button(&config.global),
                );
                if let Err(e) = layer.set_input_region(region) {
                    tracing::warn!("visibility change: {e}");
                }
            }
            if next_scene {
                // Not input, so no undo step is open: open one, or a later
                // undo would take the switch back with the step before it.
                history.input(scene, selection, Instant::now());
                // A drag or a rectangle holds characters about to be
                // replaced: let go first (R56).
                crate::input::multi::cancel(
                    &mut *scene,
                    &mut *selection,
                    &mut *drag,
                    &mut *marquee,
                );
                crate::outcomes::next_scene(
                    &mut outcome_ctx!(scene, selection, toasts, config_dirty, renderer),
                    &mut config.global.active_scene,
                );
            }
            if quit {
                *quit_requested = true;
            }
        }
    }
}
