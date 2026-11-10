//! The keyboard and the pointer, in edit mode and out of it
//! ([`super::Native::iteration`]).

use super::*;

impl Native {
    /// The keyboard: text pasted into a text field, then the actions
    /// of this turn's keys and of last turn's palette picks, through one
    /// `match`. Gives back the turn's egui events and modifiers.
    pub(super) fn keys(
        &mut self,
        plan: &WindowPlan,
        monitors_now: &[MonitorInfo],
        primary_origin: (f32, f32),
        activity: &mut bool,
    ) -> (Vec<egui::Event>, egui::Modifiers) {
        let Self {
            quit_requested,
            drag_widened,
            history,
            palette_actions,
            pending_paste,
            pending_text_paste,
            pending_transfer,
            pending_shimeji,
            library_root,
            context_menu_state,
            perf_overlay_visible,
            config_watch,
            config_dirty,
            toasts,
            marquee,
            drag,
            selection,
            egui_renderer,
            renderer,
            layer,
            overlay_hidden,
            config,
            scene,
            ..
        } = self;
        let mut events = layer.drain_egui_events();
        // Text on its way from the clipboard into a text field, and Ctrl+V
        // in one now (`crate::system_clipboard`).
        if let Some(answer) = &pending_text_paste {
            match answer.try_recv() {
                Ok(content) => {
                    *pending_text_paste = None;
                    if let crate::system_clipboard::Content::Text(text) = content {
                        events.push(egui::Event::Paste(text));
                        *activity = true;
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => *pending_text_paste = None,
            }
        }
        if std::mem::take(&mut layer.state.text_paste_requested)
            && egui_renderer.wants_keyboard()
            && pending_text_paste.is_none()
        {
            match layer.read_clipboard(crate::system_clipboard::Want::Text) {
                crate::wayland::layer_window::ClipboardRead::Ready(
                    crate::system_clipboard::Content::Text(text),
                ) => events.push(egui::Event::Paste(text)),
                crate::wayland::layer_window::ClipboardRead::Ready(_) => {}
                crate::wayland::layer_window::ClipboardRead::Pending(answer) => {
                    *pending_text_paste = Some(answer);
                }
            }
        }
        let modifiers = layer.modifiers();
        // A focused text field owns the keyboard: the palette's search box,
        // a numeric field being typed into, the keybinding capture widget.
        // Without this gate every character double-fires as a global
        // shortcut — typing "vi" into the command palette also toggled the
        // selection's visibility and dumped its info to the log (R25).
        // `egui_winit` reports key events as consumed on exactly this
        // condition, which is why the winit path never had the bug. The
        // open palette, an open list and the right-click menu own it too:
        // Escape there closed them and also left edit mode.
        let egui_owns_keyboard = egui_renderer.wants_keyboard() || context_menu_state.is_some();
        // Last frame's palette picks first, then this frame's keys — one
        // path for both, so an action cannot behave differently depending
        // on how it was asked for.
        let mut actions = std::mem::take(palette_actions);
        *activity |= !actions.is_empty();
        actions.extend(
            events
                .iter()
                .filter_map(|event| binding_chord(event, egui_owns_keyboard))
                .filter_map(|chord| config.keybindings.action_for_press(chord)),
        );
        // The Menu key, the keyboard's own key for the right-click menu:
        // not a binding, egui having no name for it to record or match.
        if std::mem::take(&mut layer.state.menu_key_pressed) && !egui_owns_keyboard {
            actions.push(Action::OpenContextMenu);
        }
        for action in actions {
            if action.interrupts_drag() {
                *config_dirty |= crate::input::multi::cancel(
                    &mut *scene,
                    &mut *selection,
                    &mut *drag,
                    &mut *marquee,
                );
            }
            match action {
                Action::ToggleEditMode => {
                    flip_edit_mode(
                        &mut *layer,
                        &mut *config,
                        scene,
                        &mut *config_dirty,
                        &mut *config_watch,
                        "Wayland",
                    );
                }
                // In edit mode the keys are the overlay's, so the global
                // chord comes as a key press rather than through the portal
                // or a compositor binding — and was dropped here, while on
                // X11 the grab took it in edit mode too. Hidden as from the
                // tray: nothing drawn, no input taken.
                Action::HideOverlay => {
                    *overlay_hidden = true;
                    let region = region_for_state(
                        true,
                        layer.state.edit_mode || *drag_widened,
                        renderer.primary.window_width,
                        renderer.primary.window_height,
                        shown_button(&config.global),
                    );
                    if let Err(e) = layer.set_input_region(region) {
                        tracing::warn!("hide: {e}");
                    }
                }
                // As the global shortcuts do; snoozing leaves edit mode.
                Action::ToggleMute => {
                    config.global.sounds_muted = !config.global.sounds_muted;
                    *config_dirty = true;
                }
                // The right-click menu, from the keyboard: on the panel's
                // surface, where the keys and a screen reader are, nearest
                // the character selected, its first item focused
                // (`input::multi::keyboard_menu_at`).
                Action::OpenContextMenu => {
                    if let Some(idx) = selection.selected_index() {
                        if let Some(e) = scene.entities.get(idx) {
                            let at = egui::pos2(
                                e.x + e.scaled_width() / 2.0,
                                e.y + e.scaled_height() / 2.0,
                            );
                            crate::input::multi::select_for_menu(scene, &mut *selection, idx);
                            *context_menu_state = Some(crate::app::ContextMenuState {
                                entity_idx: idx,
                                pos: crate::input::multi::keyboard_menu_at(
                                    at - egui::vec2(primary_origin.0, primary_origin.1),
                                    egui::vec2(
                                        renderer.primary.window_width as f32,
                                        renderer.primary.window_height as f32,
                                    ),
                                ),
                                surface: None,
                                armed: false,
                                keyboard: true,
                            });
                            *activity = true;
                        }
                    }
                }
                Action::ToggleFocus | Action::ToggleSnooze => {
                    if timer_shortcut(action, config.global.focus) && layer.state.edit_mode {
                        flip_edit_mode(
                            &mut *layer,
                            &mut *config,
                            scene,
                            &mut *config_dirty,
                            &mut *config_watch,
                            "Wayland",
                        );
                    }
                }
                // The next four are not shareable — each one reaches for
                // something only this loop owns (its config, its shutdown
                // flag, its renderer's texture cache) — but every piece
                // they need is already here, and the in-app help screen
                // lists all four. They did nothing at all on this backend
                // until now (R27).
                Action::SaveNow => match sync_and_save(&mut *config, scene) {
                    Ok(()) => {
                        *config_dirty = false;
                        // Our own write changes the file's mtime and would
                        // otherwise look exactly like somebody else's edit
                        // to the watcher above, costing a pointless full
                        // reload — selection and every texture included.
                        config_watch.note_saved();
                        tracing::info!("Config saved manually");
                        toasts.success(crate::i18n::t("toast-config-saved"));
                    }
                    Err(e) => {
                        // `config_dirty` deliberately stays set: a
                        // transient failure must let the next edit retry
                        // rather than silently drop the scene, which is
                        // the same rule `save_config_if_needed` follows
                        // on the winit path.
                        tracing::warn!("Failed to save config: {e}");
                        let mut args = fluent::FluentArgs::new();
                        args.set("error", e.said());
                        toasts.error(crate::i18n::t_args("toast-save-failed", &args));
                    }
                },
                Action::TogglePerfOverlay => {
                    *perf_overlay_visible = !*perf_overlay_visible;
                    tracing::debug!(
                        "Perf overlay {}",
                        if *perf_overlay_visible {
                            "shown"
                        } else {
                            "hidden"
                        }
                    );
                }
                Action::QuitWithSave => {
                    // The shutdown path at the end of `run` persists
                    // `config_dirty` on its way out, so this needs no save
                    // of its own — and doing one here would write the
                    // scene twice on every quit.
                    tracing::info!("Quit action — saving and exiting");
                    *quit_requested = true;
                }
                Action::DeleteSelected | Action::DuplicateSelected | Action::CutSelected => {
                    // The same functions the right-click menu and the winit
                    // path use (`crate::outcomes`), so none of the three can
                    // drift from the others. Every selected character.
                    let targets = selection.selected_indices();
                    let mut ctx = outcome_ctx!(scene, selection, toasts, config_dirty, renderer);
                    match action {
                        Action::DeleteSelected => {
                            outcomes::delete_entities(&targets, &mut ctx);
                        }
                        Action::CutSelected => {
                            outcomes::cut_entities(&targets, &mut ctx);
                        }
                        _ => {
                            outcomes::duplicate_entities(&targets, &mut ctx);
                        }
                    }
                }
                // The desktop's clipboard first: an image or files copied
                // elsewhere since the last copy here come in as characters
                // (`crate::system_clipboard`), at the pointer.
                Action::Paste => {
                    let at = layer
                        .state
                        .cursor_pos
                        .map(|(x, y)| (x + primary_origin.0, y + primary_origin.1))
                        .unwrap_or((
                            renderer.primary.window_width as f32 / 2.0 + primary_origin.0,
                            renderer.primary.window_height as f32 / 2.0 + primary_origin.1,
                        ));
                    match layer.read_clipboard(crate::system_clipboard::Want::Characters) {
                        crate::wayland::layer_window::ClipboardRead::Ready(content) => {
                            let pasted = outcomes::apply_pasted(
                                content,
                                at,
                                &mut outcome_ctx!(scene, selection, toasts, config_dirty, renderer),
                            );
                            for path in pasted.to_import {
                                // A shared scene or a Shimeji pack copied in a file
                                // manager goes where a drop of it would.
                                if crate::scene_file::is_scene_file(&path) {
                                    if pending_transfer.is_none() {
                                        *pending_transfer = Some(
                                            crate::outcomes::SceneTransfer::import(path, Some(at)),
                                        );
                                    }
                                } else if pending_shimeji.is_none() {
                                    *pending_shimeji = ShimejiImport::start(
                                        &path,
                                        library_root.as_deref(),
                                        at,
                                        &mut *toasts,
                                    );
                                }
                            }
                        }
                        crate::wayland::layer_window::ClipboardRead::Pending(answer) => {
                            pending_paste.get_or_insert((answer, at));
                        }
                    }
                }
                // Everything that only touches the scene and the selection
                // is shared with the winit path. Before this, the table was
                // consulted here and matched ToggleEditMode alone, so the
                // other twenty-seven rebindable actions were inert on this
                // backend — the user could rebind them and see the config
                // persist while nothing happened (R19).
                other => {
                    let mut ctx = crate::keybindings::shared::ActionCtx {
                        scene: &mut *scene,
                        selection: &mut *selection,
                        config_dirty: &mut *config_dirty,
                        shift_held: layer.state.last_modifiers.shift,
                        bounds: monitor::covered_bounds(
                            plan,
                            (
                                renderer.primary.window_width as f32,
                                renderer.primary.window_height as f32,
                            ),
                        ),
                        monitors: monitors_now,
                        toasts: &mut *toasts,
                        history: &mut *history,
                    };
                    crate::keybindings::shared::dispatch_shared(other, &mut ctx);
                }
            }
        }
        // A copy just made: its names on the desktop's clipboard, while the
        // press that made it is the latest (setting it needs its serial).
        if let Some(text) = crate::clipboard::take_claim() {
            layer.claim_clipboard(text, true);
        }

        (events, modifiers)
    }

    /// The pointer: the right-click menu, on whichever monitor it is;
    /// letting go of a drag when edit mode ends or the overlay hides; in
    /// edit mode, selecting, dragging, a rectangle over empty space; in
    /// pass-through, a tap. Gives back the events for the menu's monitor,
    /// for its overlay's egui.
    pub(super) fn pointer(
        &mut self,
        events: &[egui::Event],
        plan: &WindowPlan,
        monitors_now: &[MonitorInfo],
        primary_origin: (f32, f32),
        activity: &mut bool,
    ) -> Vec<egui::Event> {
        let Self {
            extra_eguis,
            extra_surfaces,
            aside,
            history,
            tap_gesture,
            context_menu_state,
            config_dirty,
            was_editing,
            marquee,
            drag,
            selection,
            egui_renderer,
            renderer,
            layer,
            overlay_hidden,
            config,
            scene,
            ..
        } = self;
        // Right-click on an entity opens the context menu and selects
        // it, same gating and behavior as the X11 path's
        // `handle_mouse_input` (src/app/input.rs). Entity-less right
        // clicks (empty space) are ignored.
        //
        // Presses that egui owns are skipped. The winit path gets this for
        // free — `App::window_event` forwards to egui first and returns
        // early when the event is consumed — and without the equivalent
        // here, a left click anywhere on the settings panel's background
        // fell through to `entity_at_point`, found nothing, and cleared
        // the selection. So clicking blank space in the panel threw away
        // the very entity whose Inspector you were reading.
        //
        // Only the *press* arms are gated. Motion and release stay live so
        // a drag that began on a sprite still tracks and still finishes if
        // the pointer crosses the panel on the way.
        // Edit mode just ended — by any of the four ways — so let go of
        // what the pointer held and drop the selection, as the winit path
        // does. The release of a drag in progress will not come: in
        // pass-through the surface no longer gets the pointer, and the
        // drag used to stay open until the next move in edit mode, which
        // made the character jump to the pointer.
        if *was_editing && !layer.state.edit_mode {
            *config_dirty |= crate::input::multi::cancel(
                &mut *scene,
                &mut *selection,
                &mut *drag,
                &mut *marquee,
            );
            selection.deselect();
            *context_menu_state = None;
        }
        *was_editing = layer.state.edit_mode;
        // Hidden, nothing is dragged on. Sway still delivers the release —
        // the button's implicit grab outlives the empty input region — but
        // on X11 an unmapped window gets none, and the character jumped to
        // the pointer at the first move once shown; let go here too rather
        // than count on every compositor.
        if *overlay_hidden || aside.hidden {
            *config_dirty |= crate::input::multi::cancel(
                &mut *scene,
                &mut *selection,
                &mut *drag,
                &mut *marquee,
            );
        }
        let egui_owns_pointer = egui_renderer.owns_pointer();
        // The right-click menu on another monitor's overlay (`surface`):
        // that overlay's egui hears the pointer while it is over that
        // monitor, and a press anywhere else closes the menu — the egui
        // there cannot count a click it never hears as outside. Escape
        // closes it too: the keys go to the panel's egui. Presses on the
        // menu stay out of the scene, as the panel's do.
        let mut extra_menu_events: Vec<egui::Event> = Vec::new();
        let menu_monitor = context_menu_state
            .as_ref()
            .and_then(|m| m.surface.as_ref())
            .filter(|name| extra_surfaces.contains_key(*name))
            .and_then(|name| monitors_now.iter().find(|m| &m.name == name))
            .cloned();
        // Its monitor went — unplugged, or the overlay down to one: nothing
        // could draw the menu, and it kept every key from the shortcuts.
        if menu_monitor.is_none()
            && context_menu_state
                .as_ref()
                .is_some_and(|m| m.surface.is_some())
        {
            *context_menu_state = None;
        }
        let global =
            |pos: &egui::Pos2| egui::pos2(pos.x + primary_origin.0, pos.y + primary_origin.1);
        if let Some(mon) = &menu_monitor {
            let origin = egui::vec2(mon.x as f32, mon.y as f32);
            for event in events {
                match event {
                    egui::Event::PointerMoved(pos) => {
                        let at = global(pos);
                        extra_menu_events.push(if mon.contains(at.x, at.y) {
                            egui::Event::PointerMoved(at - origin)
                        } else {
                            egui::Event::PointerGone
                        });
                    }
                    egui::Event::PointerButton {
                        pos,
                        button,
                        pressed,
                        modifiers,
                    } => {
                        let at = global(pos);
                        if mon.contains(at.x, at.y) {
                            extra_menu_events.push(egui::Event::PointerButton {
                                pos: at - origin,
                                button: *button,
                                pressed: *pressed,
                                modifiers: *modifiers,
                            });
                        } else if *pressed {
                            *context_menu_state = None;
                        }
                    }
                    egui::Event::PointerGone => extra_menu_events.push(egui::Event::PointerGone),
                    egui::Event::Key {
                        key: egui::Key::Escape,
                        pressed: true,
                        ..
                    } => *context_menu_state = None,
                    _ => {}
                }
            }
        }
        let extra_owns_pointer = menu_monitor
            .as_ref()
            .is_some_and(|m| extra_eguis.get(&m.name).is_some_and(|e| e.owns_pointer()));
        let owned_press = |pos: &egui::Pos2| {
            let at = global(pos);
            egui_owns_pointer
                || (extra_owns_pointer
                    && menu_monitor
                        .as_ref()
                        .is_some_and(|m| m.contains(at.x, at.y)))
        };
        if layer.state.edit_mode {
            for event in events {
                match event {
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Secondary,
                        pressed: true,
                        ..
                    } if !owned_press(pos) => {
                        let at = global(pos);
                        if let Some(idx) = scene.entity_at_point(at.x, at.y) {
                            // One of several selected keeps them all, so
                            // the menu acts on the whole selection;
                            // otherwise it takes the character with its
                            // group.
                            crate::input::multi::select_for_menu(scene, &mut *selection, idx);
                            // On the monitor clicked: another monitor's
                            // overlay draws it with its own egui, in its
                            // own points.
                            let elsewhere = monitors_now.iter().find(|m| {
                                extra_surfaces.contains_key(&m.name) && m.contains(at.x, at.y)
                            });
                            *context_menu_state = Some(crate::app::ContextMenuState {
                                entity_idx: idx,
                                pos: match elsewhere {
                                    Some(m) => at - egui::vec2(m.x as f32, m.y as f32),
                                    None => *pos,
                                },
                                surface: elsewhere.map(|m| m.name.clone()),
                                // Armed after the first showing — see ContextMenuState.
                                armed: false,
                                keyboard: false,
                            });
                        }
                    }
                    // Left press: select and begin a drag, mirroring
                    // `App::handle_mouse_input` on the winit path. Both were
                    // missing here — selection was right-click only, and
                    // dragging a character (the core interaction of a
                    // desktop-pet overlay) was not implemented on this
                    // backend at all.
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers,
                        ..
                    } if !owned_press(pos) => {
                        let at = (pos.x + primary_origin.0, pos.y + primary_origin.1);
                        // Select (with its group, keeping the others selected,
                        // or toggled with Shift) and pick the selection up; on
                        // empty space, a selection rectangle
                        // (`crate::input::multi`).
                        match scene.entity_at_point(at.0, at.1) {
                            Some(idx) => {
                                crate::input::multi::press_on(
                                    &mut *scene,
                                    &mut *selection,
                                    &mut *drag,
                                    idx,
                                    at,
                                    modifiers.shift,
                                );
                            }
                            None => {
                                *marquee = Some(crate::input::multi::Marquee::begin(
                                    at,
                                    modifiers.shift,
                                    &mut *selection,
                                ));
                            }
                        }
                    }
                    egui::Event::PointerButton {
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        ..
                    } if marquee.is_some() => {
                        *marquee = None;
                    }
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers,
                        ..
                    } if drag.is_dragging() => {
                        let (gx, gy) = (pos.x + primary_origin.0, pos.y + primary_origin.1);
                        // A press/release that never moved is a *tap*, not a
                        // drag — poke the mascot instead of just dropping it.
                        let tapped = drag.was_tap(gx, gy, crate::constants::POKE_TAP_RADIUS);
                        if tapped {
                            // The hop that follows is play, not an edit.
                            history.finish(scene);
                        }
                        let poke_bounds = monitor::covered_bounds(
                            plan,
                            (
                                renderer.primary.window_width as f32,
                                renderer.primary.window_height as f32,
                            ),
                        );
                        if tapped {
                            if let Some(idx) = drag.dragging_entity() {
                                crate::tap::poke(&mut *scene, idx, gx, poke_bounds);
                            }
                        }
                        // Thrown: the flight is play, as a poke's hop is —
                        // the step closes where the character was let go.
                        crate::input::multi::end_drag(
                            &mut *scene,
                            &mut *selection,
                            &mut *drag,
                            tapped,
                            modifiers.shift,
                            |scene| {
                                history.finish(scene);
                            },
                        );
                        *config_dirty = true;
                    }
                    egui::Event::PointerMoved(pos) if marquee.is_some() => {
                        let at = (pos.x + primary_origin.0, pos.y + primary_origin.1);
                        if let Some(m) = marquee.as_mut() {
                            m.drag_to(at, scene, &mut *selection);
                        }
                    }
                    egui::Event::PointerMoved(pos) if drag.is_dragging() => {
                        let at = (pos.x + primary_origin.0, pos.y + primary_origin.1);
                        // Snapping, unless it is off or Alt is held.
                        let snap_to = (config.global.snap_while_dragging
                            && !layer.state.last_modifiers.alt)
                            .then(|| crate::input::arrange::monitor_rects(monitors_now));
                        crate::input::multi::drag_to(
                            &mut *scene,
                            selection,
                            &mut *drag,
                            at,
                            snap_to.as_deref(),
                        );
                    }
                    _ => {}
                }
            }
        } else if config.global.tap_characters && !(*overlay_hidden || aside.hidden || aside.paused)
        {
            // Pass-through with tapping on (`crate::tap`): a press and a
            // release close together on a character poke it. Presses on
            // the ⚙ button are egui's.
            for event in events {
                match event {
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        ..
                    } if !egui_owns_pointer => {
                        let at = global(pos);
                        tap_gesture.press(scene, at.x, at.y);
                    }
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        ..
                    } => {
                        let at = global(pos);
                        if let Some(idx) = tap_gesture.release(scene, at.x, at.y) {
                            let bounds = monitor::covered_bounds(
                                plan,
                                (
                                    renderer.primary.window_width as f32,
                                    renderer.primary.window_height as f32,
                                ),
                            );
                            crate::tap::poke(&mut *scene, idx, at.x, bounds);
                            *activity = true;
                        }
                    }
                    _ => {}
                }
            }
        }

        extra_menu_events
    }
}
