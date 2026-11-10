//! What the desktop and the time ask of the overlay, and where it takes
//! the pointer ([`super::Native::iteration`]).

use super::*;

impl Native {
    /// What the desktop and the time ask of the overlay: in front of the
    /// windows or behind; stepping aside for a full-screen window, an app
    /// chosen in front, a snooze; held still for someone away, the battery
    /// or bedtime; the scene by the time of day; reminders and the focus
    /// timer; the pace by the hour; a greeting for someone back.
    pub(super) fn circumstances(&mut self, activity: &mut bool) {
        let Self {
            library_scan,
            audio_host,
            drag_widened,
            scene_schedule,
            reminder_timers,
            held_for_away,
            on_battery,
            aside,
            history,
            moods,
            welcome,
            hours_clock,
            bedtime_watch,
            library,
            config_dirty,
            toasts,
            marquee,
            drag,
            selection,
            renderer,
            layer,
            overlay_hidden,
            config,
            scene,
            ..
        } = self;
        // Drain pointer + keyboard events. Until egui paint lands
        // (E.4) we don't have a UI consumer, but we already need to
        // detect the `Action::ToggleEditMode` chord so click-through
        // can flip in lock-step. Scan key-press events, match against
        // the user's bindings, dispatch the few actions that make
        // sense without a UI thread (just edit mode for now).
        // Step aside while another window is full screen, and come back
        // when it is not. Recomputed every iteration from the setting, the
        // window in front and edit mode, so nothing has to be undone.
        // In front of the windows or behind them; edit mode always in
        // front, so the panel is in reach.
        layer.set_on_top(config.global.always_on_top || layer.state.edit_mode);
        let chosen_in_front = layer
            .state
            .toplevels
            .chosen_in_front(&config.global.step_aside_apps);
        let wanted = crate::fullscreen::step_aside(
            config.global.on_fullscreen,
            layer.fullscreen_in_front(),
            layer.state.edit_mode,
        )
        // An app the user chose in front (`crate::chosen_apps`).
        .or_chosen_app(chosen_in_front, layer.state.edit_mode)
        // Snoozing (`crate::snooze`): out of sight and still until it ends.
        .or_snoozing(crate::snooze::active());
        // The open apps, for the picker, while it shows.
        if crate::chosen_apps::open_list_wanted() {
            crate::chosen_apps::set_open(layer.state.toplevels.open_apps());
        }
        // In the Flatpak, the Background portal's answer to "start at
        // login", once it comes (`crate::autostart`).
        if let Some(granted) = crate::autostart::take_portal_answer() {
            config.global.start_at_login = granted;
            *config_dirty = true;
        }
        // Held still while nobody is there, too (`crate::away`): idle from
        // the compositor, the battery from the away watch.
        let global = &config.global;
        // Watched for reminders even with pausing off.
        let idle_watch = crate::away::idle_watch_minutes(
            global.pause_when_idle_minutes,
            !config.reminders.is_empty() || global.welcome_back,
        );
        layer.set_idle_timeout(idle_watch);
        crate::away::configure(
            idle_watch,
            global.pause_on_battery || global.frame_rate.watches_battery(),
        );
        // Bedtime by the clock (`crate::bedtime`) — not while editing.
        let bedtime_now = !layer.state.edit_mode && bedtime_watch.now(&global.bedtime);
        // The library's fresh scan, once it is done (`asset_library::open`).
        if let Some(rx) = &library_scan {
            match rx.try_recv() {
                Ok(index) => {
                    *library = Some(index);
                    *library_scan = None;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => *library_scan = None,
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }
        // How loud the sounds are, if at all (`crate::audio`), before the
        // tick: scripts play from inside it.
        audio_host.set_level(
            global.sound_volume,
            global.sounds_muted,
            global.quiet_at_bedtime && bedtime_now,
        );
        // Characters with hours of their own (`crate::hours`) — everyone
        // while editing.
        let minute = if layer.state.edit_mode {
            None
        } else {
            hours_clock.minute()
        };
        if scene.set_clock(minute) {
            *activity = true;
        }
        let still = crate::away::stillness(
            global.pause_when_idle_minutes,
            layer.is_idle(),
            global.pause_on_battery,
            *on_battery,
        )
        .with_bedtime(bedtime_now)
        .watching(layer.fullscreen_in_front());
        if still.any() != *held_for_away {
            *held_for_away = still.any();
            *activity = true;
            tracing::info!(
                "Away: scene {}",
                if still.doze {
                    "dozing off"
                } else if still.freeze {
                    "held still"
                } else {
                    "back"
                }
            );
        }
        *activity |= scene.set_dozing(still.doze);
        scene.set_suspended(wanted.paused || still.freeze);
        // Scenes by time of day (`crate::schedule`), as on the winit path:
        // not in edit mode, the undo step opened here, gestures let go.
        if !layer.state.edit_mode {
            if let Some(name) = scene_schedule.due_now(&config.scene_schedule) {
                history.input(scene, selection, Instant::now());
                crate::input::multi::cancel(
                    &mut *scene,
                    &mut *selection,
                    &mut *drag,
                    &mut *marquee,
                );
                if crate::outcomes::apply_scheduled(
                    &name,
                    &mut outcome_ctx!(scene, selection, toasts, config_dirty, renderer),
                    &mut config.global.active_scene,
                ) {
                    *activity = true;
                }
            }
        }
        // Reminders due now (`crate::reminders`), said by any character:
        // every monitor draws its bubbles. Before the frame gate: a still
        // scene would never reach the drawing code.
        // While the focus timer runs (`crate::focus`), reminders wait out
        // the work, and its break counts as a break.
        let focus = crate::focus::phase();
        if crate::reminders::deliver(
            &mut *reminder_timers,
            &config.reminders,
            &config.global.reminder_sound,
            config.global.notify_reminders,
            &mut *scene,
            !(*overlay_hidden || wanted.hidden),
            // The night counts as a break too.
            crate::reminders::Away {
                on_a_break: layer.is_idle()
                    || bedtime_now
                    || focus == Some(crate::focus::Phase::Break),
                idle: layer.is_idle(),
                working: focus == Some(crate::focus::Phase::Work),
            },
            |_| true,
        ) {
            *activity = true;
        }
        // Each turn of the timer, said once it can be seen.
        if crate::focus::say_news(
            &mut *scene,
            !(*overlay_hidden || wanted.hidden),
            &config.global.reminder_sound,
            config.global.notify_reminders,
            |_| true,
        ) {
            *activity = true;
        }
        // The pace by the time of day, and a yawn before bedtime
        // (`crate::moods`).
        moods.step(
            &mut *scene,
            config.global.moods,
            &config.global.bedtime,
            !(*overlay_hidden || wanted.hidden),
        );
        // Back after being away (`crate::welcome`): greeted where it can be
        // seen, and not by sleepers.
        let minute = hours_clock.minute();
        if let Some(greeting) = welcome.check(layer.is_idle(), minute).filter(|_| {
            config.global.welcome_back && !(*overlay_hidden || wanted.hidden) && !bedtime_now
        }) {
            if let Some(i) = scene.frontmost_visible() {
                scene.entities[i].say(&greeting.line(), crate::welcome::SAY_SECONDS);
                *activity = true;
            }
        }
        if wanted != *aside {
            *aside = wanted;
            *activity = true;
            tracing::info!(
                "{}: overlay {}",
                if crate::snooze::active() {
                    "Snoozing"
                } else if chosen_in_front && !layer.state.edit_mode {
                    "An app chosen in Appearance in front"
                } else if layer.fullscreen_in_front() {
                    "Full-screen app in front"
                } else {
                    "Full-screen app gone"
                },
                if aside.hidden {
                    "hidden"
                } else if aside.paused {
                    "paused"
                } else {
                    "back"
                }
            );
            let region = region_for_state(
                *overlay_hidden || aside.hidden,
                layer.state.edit_mode || *drag_widened,
                renderer.primary.window_width,
                renderer.primary.window_height,
                shown_button(&config.global),
            );
            if let Err(e) = layer.set_input_region(region) {
                tracing::warn!("stepping aside: {e}");
            }
        }
    }

    /// Where the overlays take the pointer: wider while a drag is over
    /// the primary in pass-through, the characters' rectangles when they
    /// take taps, and the other monitors' overlays following the primary.
    pub(super) fn input_regions(
        &mut self,
        monitors_now: &[MonitorInfo],
        primary_origin: (f32, f32),
    ) {
        let Self {
            extra_surfaces,
            drag_widened,
            aside,
            tap_refreshed,
            egui_renderer,
            renderer,
            layer,
            overlay_hidden,
            config,
            scene,
            ..
        } = self;
        // Widen the input region while a drag is over us in pass-through,
        // and put it back when the drag ends.
        let widen = drag_region(
            *overlay_hidden || aside.hidden,
            layer.state.edit_mode,
            layer.state.drag_over,
        );
        if widen != *drag_widened {
            *drag_widened = widen;
            let region = region_for_state(
                *overlay_hidden || aside.hidden,
                layer.state.edit_mode || widen,
                renderer.primary.window_width,
                renderer.primary.window_height,
                shown_button(&config.global),
            );
            if let Err(e) = layer.set_input_region(region) {
                tracing::warn!("drag region: {e}");
            }
        }

        // Tapping (`crate::tap`): in pass-through, with the setting on,
        // the characters take clicks too — their rectangles join the ⚙
        // corner, a few times a second at most — and a click on one must
        // not take the keyboard. A click on the ⚙ still does, so edit mode
        // has the keys from its first moment: the pointer is over it before
        // the press.
        // Not over a full-screen app either, even one they only pause
        // for: a game's clicks are the game's.
        let tapping = config.global.tap_characters
            && !layer.state.edit_mode
            && !(*overlay_hidden || aside.hidden || aside.paused)
            && !widen;
        layer.set_keyboard_wanted(
            layer.state.edit_mode || !config.global.tap_characters || egui_renderer.owns_pointer(),
        );
        let tap_refresh = tapping && (*tap_refreshed).elapsed() >= crate::tap::REFRESH;
        if tap_refresh {
            *tap_refreshed = Instant::now();
            let base = region_for_state(
                false,
                false,
                renderer.primary.window_width,
                renderer.primary.window_height,
                shown_button(&config.global),
            );
            let rects = crate::tap::rects(
                scene,
                primary_origin,
                (
                    renderer.primary.window_width,
                    renderer.primary.window_height,
                ),
            );
            layer.sync_tap_region(base, rects);
        } else if !tapping && layer.tap_region_applied() {
            // Tapping just went off: the plain region again.
            let region = region_for_state(
                *overlay_hidden || aside.hidden,
                layer.state.edit_mode || widen,
                renderer.primary.window_width,
                renderer.primary.window_height,
                shown_button(&config.global),
            );
            if let Err(e) = layer.set_input_region(region) {
                tracing::warn!("tap region: {e}");
            }
        }

        // The per-monitor extra surfaces follow the primary: open to
        // input in edit mode, or while a drag is over the overlay, and
        // click-through otherwise — but for the characters, when tapping.
        // Their offsets into the primary's space come from the same output
        // snapshot as `primary_origin`, so a click, drag or drop on
        // another monitor lands in the right global place.
        layer.sync_extra_layers(
            extras_take_input(
                *overlay_hidden || aside.hidden,
                layer.state.edit_mode,
                widen,
            ),
            |name| {
                monitors_now
                    .iter()
                    .find(|m| m.name == name)
                    .map(|m| (m.x as f32 - primary_origin.0, m.y as f32 - primary_origin.1))
            },
            |name| {
                if !tapping {
                    return Some(Vec::new());
                }
                if !tap_refresh {
                    return None;
                }
                let mon = monitors_now.iter().find(|m| m.name == name)?;
                let surface = extra_surfaces.get(name)?;
                Some(crate::tap::rects(
                    scene,
                    (mon.x as f32, mon.y as f32),
                    (surface.window_width, surface.window_height),
                ))
            },
        );
    }
}
