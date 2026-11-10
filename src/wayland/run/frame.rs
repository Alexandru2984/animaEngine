//! The scene's tick, the frame on every monitor, and what the panels asked
//! for in it ([`super::Native::iteration`]).

use super::*;

impl Native {
    /// The scene moves on: the settings it follows, a tick of every
    /// character (scripts and sounds in it), the fades, the sounds it
    /// asked for, and the textures of the frames it moved to.
    pub(super) fn tick(
        &mut self,
        plan: &WindowPlan,
        cursor_global: Option<(f32, f32)>,
        activity: &mut bool,
    ) {
        let Self {
            audio_host,
            script_host,
            aside,
            was_hidden,
            library_root,
            soak,
            perf_sampler,
            config_dirty,
            toasts,
            renderer,
            overlay_hidden,
            config,
            scene,
            ..
        } = self;
        // Tick the simulation. screen_w / screen_h match the surface so
        // walk-around behaviors stay inside the visible area.
        if let Some(soak) = soak.as_mut() {
            soak.maybe_sample(
                crate::perf::read_rss_kib(),
                scene.total_decoded_bytes(),
                renderer.shared.textures.len(),
                None,
            );
        }
        scene.set_reduced_motion(config.global.reduced_motion);
        scene.set_hover_startle(config.global.hover_startle);
        scene.set_bump(config.global.characters_bump);
        scene.set_greet(config.global.characters_greet);
        scene.set_large_bubbles(config.global.large_bubbles);
        scene.set_little_effects(config.global.little_effects);
        {
            let _s = perf_sampler.scope(crate::perf::Category::SceneUpdate);
            scene.tick(
                crate::monitor::covered_bounds(
                    plan,
                    (
                        renderer.primary.window_width as f32,
                        renderer.primary.window_height as f32,
                    ),
                ),
                // `cursor_pos` is tracked from every Motion/Enter pointer
                // event (pointer_handler.rs) in this surface's local
                // space; `cursor_global` adds `primary_origin` so it lands
                // in the same coordinate space entities use (identity
                // outside PerMonitor, T.8-equivalent otherwise). FollowCursor
                // sees it whenever the pointer is over the surface's active
                // input region — same X11 caveat applies: pass-through mode
                // still leaves it stale outside the toggle button, since
                // Wayland has no XQueryPointer equivalent (docs/threat-model.md).
                cursor_global,
                library_root
                    .as_deref()
                    .map(|root| crate::scripting::ScriptContext {
                        host: &mut *script_host,
                        audio: &mut *audio_host,
                        root,
                    }),
            );
        }
        // Back from hidden, everyone fades in; and the fades move on,
        // paused or not.
        let hidden_now = *overlay_hidden || aside.hidden;
        if *was_hidden && !hidden_now {
            scene.fade_all_in();
        }
        *was_hidden = hidden_now;
        scene.fade_step(Instant::now());
        // A character spoke up on its own (`crate::phrases`): show it.
        *activity |= scene.take_spoke();
        // A thrown one landed: save where.
        *config_dirty |= scene.take_landed();
        // Tap sounds (`crate::sounds`), panned across what the overlay covers.
        let sounds = scene.take_sounds();
        if let Some(root) = library_root.as_deref() {
            let bounds = crate::monitor::covered_bounds(
                plan,
                (
                    renderer.primary.window_width as f32,
                    renderer.primary.window_height as f32,
                ),
            );
            for (id, sound, x, volume) in sounds {
                audio_host.play(root, &sound, &id, x, bounds.min_x, bounds.max_x, volume);
            }
        }

        for (path, err) in audio_host.take_new_failures() {
            let mut args = fluent::FluentArgs::new();
            args.set("sound", path);
            args.set("error", err);
            toasts.error(crate::i18n::t_args("sound-failed-toast", &args));
        }

        // Update any dirty textures (animation frame advance).
        // The prune sweeps textures orphaned by preset Replace — same
        // rationale as the winit render loop.
        renderer.prune_stale_textures(&scene.entities);
        for entity in &mut scene.entities {
            if entity.texture_dirty {
                renderer.ensure_texture(entity);
                entity.texture_dirty = false;
                // An animation showing its next frame: this is what wakes
                // a still scene with playing sprites, at their own rate —
                // unless the overlay is hidden and shows nothing.
                *activity |= !(*overlay_hidden || aside.hidden);
            }
        }
    }

    /// A frame: the primary's scene and egui — the panel, the palette,
    /// the menu, the toasts — and what they ask for, then every other
    /// monitor's overlay. Outside edit mode a surface draws only when it
    /// has something new to show (`app::windows::needs_frame`).
    pub(super) fn draw(
        &mut self,
        outputs: &Outputs,
        input: EguiInput,
        gpu: (u32, u32),
        now: Instant,
        shown: bool,
    ) -> Result<Option<Asked>> {
        let Self {
            primary_frame,
            extra_render_errors,
            render_errors,
            extra_frames,
            extra_eguis,
            extra_surfaces,
            aside,
            library,
            context_menu_state,
            warnings,
            perf_overlay_visible,
            perf_sampler,
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
            config,
            scene,
            ..
        } = self;
        let Outputs {
            monitors_now,
            plan,
            awaiting_primary,
            primary_origin,
            ..
        } = outputs;
        let (awaiting_primary, primary_origin) = (*awaiting_primary, *primary_origin);
        let EguiInput {
            events,
            modifiers,
            mut extra_menu_events,
        } = input;
        let (gpu_uploads, gpu_draws) = gpu;
        // Render the scene. Pass `selected_id` so the highlight ring
        // appears in edit mode for the entity the user clicked.
        // `monitors_now` (refreshed above) covers the inspector's
        // picker hot-plug needs too — no separate snapshot needed.
        let monitors = &monitors_now;
        let selected_owned = selection.selected_ids(scene);
        let selected_ids: Vec<&str> = selected_owned.iter().map(String::as_str).collect();
        let marks = crate::renderer::wgpu_renderer::EditMarks {
            selected: &selected_ids,
            marquee: marquee.as_ref().map(|m| m.rect()),
            guides: drag.guides(),
        };
        // Those fading out are drawn too (`Scene::fade_step`).
        let visible = scene.drawn_entities();
        // In PerMonitor mode the primary surface covers exactly its
        // own output: entities live in global coords (once extras
        // exist) and need filtering to just the primary's monitor,
        // same as the X11 path's `entity_on_monitor` gate (T.6/T.8).
        let primary_monitor_name: Option<String> = if !extra_surfaces.is_empty() {
            plan.primary.as_ref().map(|m| m.name.clone())
        } else {
            None
        };
        // Speech bubbles for the characters on the primary surface
        // (`crate::speech`), gathered while the scene is free to borrow.
        let bubbles = crate::speech::shown(scene, primary_origin, |e| {
            primary_monitor_name
                .as_ref()
                .is_none_or(|name| crate::app::windows::entity_on_monitor(monitors, e, name))
        });
        // And the hearts, dust and z's on it (`crate::effects`).
        let effects = if *overlay_hidden || aside.hidden {
            Vec::new()
        } else {
            scene.effects_shown(
                primary_origin,
                (
                    renderer.primary.window_width as f32,
                    renderer.primary.window_height as f32,
                ),
                now,
            )
        };
        let drawn: Vec<&Entity> = if *overlay_hidden || aside.hidden || awaiting_primary {
            // Hidden: present a cleared (fully transparent) surface. The
            // layer surface stays mapped — that is what a layer shell
            // gives us — but nothing is painted into it, which is the
            // visible equivalent of the winit path unmapping its window.
            Vec::new()
        } else {
            match &primary_monitor_name {
                Some(name) => visible
                    .into_iter()
                    .filter(|e| crate::app::windows::entity_on_monitor(monitors, e, name))
                    .collect(),
                None => visible,
            }
        };
        egui_renderer.ensure_theme(config.global.theme);
        // Timed with an explicit instant rather than a `scope` guard:
        // the guard would have to be dropped in both match arms, and
        // holding it across the arm keeps `perf_sampler` mutably borrowed
        // to the end of the frame.
        let submit_start = Instant::now();
        // Outside edit mode the primary draws only when it has something
        // new to show (`app::windows::needs_frame`): a walker on another
        // monitor used to redraw it every frame. Busy — drawn every frame,
        // and once after — while its egui paints, has input or is asked
        // for anything. The other monitors and the outcomes below go on
        // either way.
        // The edit-mode bar is the overlay's too: hidden, it goes with the
        // rest — it stayed, a green line across the top of a hidden one.
        let edit_bar = layer.state.edit_mode && !(*overlay_hidden || aside.hidden);
        let primary_busy = layer.state.edit_mode
            || *perf_overlay_visible
            || !bubbles.is_empty()
            || !effects.is_empty()
            || !events.is_empty()
            || toasts_at(toasts, config, shown, now).is_some_and(|at| at <= now)
            || egui_renderer.repaint_due(now)
            || egui_renderer.has_screen_reader_requests();
        let primary_key = crate::app::windows::primary_key(renderer.primary.frame_key(
            &renderer.shared,
            &drawn,
            &scene.groups,
            edit_bar,
            marks,
            primary_origin,
        ));
        let primary =
            if crate::app::windows::needs_frame(*primary_frame, primary_key, primary_busy, now) {
                renderer
                    .render(&drawn, &scene.groups, edit_bar, marks, primary_origin)
                    .map(Some)
            } else {
                Ok(None)
            };
        let mut asked = None;
        match primary {
            Ok(output) => {
                if output.is_some() {
                    perf_sampler.add(crate::perf::Category::WgpuSubmit, submit_start.elapsed());
                    render_errors.drew();
                }
                let size = [
                    renderer.primary.window_width,
                    renderer.primary.window_height,
                ];
                // egui's points are the surface's units, the pointer's
                // and the input region's; its pixels are the buffer's, at
                // the surface's scale (`layer_window::scale`).
                let pixel_size = renderer.primary.pixel_size();
                let pixels_per_point = renderer.primary.pixel_scale();
                let edit_mode_snapshot = layer.state.edit_mode;
                let show_button_snapshot = config.global.show_toggle_button;
                let hidden_snapshot = *overlay_hidden || aside.hidden;
                // Snapshot the AccessKit flag BEFORE taking its mutable
                // borrow, same trick as the X11 path uses; the renderer
                // gates egui's tree on it.
                let accesskit_snapshot = config.global.accesskit_enabled;
                let mut toggle_requested = false;
                let mut palette_outcome: Option<panels::PaletteOutcome> = None;
                let mut library_outcome: Option<panels::LibraryOutcome> = None;
                let mut menu_outcome: Option<panels::ContextMenuOutcome> = None;
                let mut perf_export_request = false;
                // Built here, while `scene` and `renderer` are still
                // freely readable — after this point the closure takes
                // disjoint `&mut` borrows of both.
                let gpu_stats = crate::ui::perf_overlay::GpuStats {
                    decoded_bytes: scene.total_decoded_bytes(),
                    texture_bytes: renderer.shared.texture_bytes(),
                    texture_count: renderer.shared.textures.len(),
                    uploads_last_frame: gpu_uploads,
                    draws_last_frame: gpu_draws,
                };
                let mut shimeji_import: Option<String> = None;
                let mut scene_request: Option<panels::SceneRequest> = None;
                // The menu on another monitor is drawn by that overlay's
                // egui, below.
                let menu_state = context_menu_state.clone().filter(|m| m.surface.is_none());
                let extra_menu = context_menu_state.clone().filter(|m| m.surface.is_some());
                let menu_offers = context_menu_state
                    .as_ref()
                    .map(|m| crate::outcomes::menu_offers(m.entity_idx, scene, selection))
                    .unwrap_or_default();
                // Disjoint mut borrows for the closure.
                let scene_mut = &mut *scene;
                let selection_mut = &mut *selection;
                let config_dirty_mut = &mut *config_dirty;
                let theme_mut = &mut config.global.theme;
                let locale_mut = &mut config.global.locale;
                let onboarding_mut = &mut config.global.onboarding;
                let monitor_mode_mut = &mut config.global.monitor_mode;
                let window_awareness_mut = &mut config.global.window_awareness;
                let snap_mut = &mut config.global.snap_while_dragging;
                let bump_mut = &mut config.global.characters_bump;
                let greet_mut = &mut config.global.characters_greet;
                let active_scene_mut = &mut config.global.active_scene;
                let reminders_mut = &mut config.reminders;
                let focus_mut = &mut config.global.focus;
                let reminder_sound_mut = &mut config.global.reminder_sound;
                let notify_mut = &mut config.global.notify_reminders;
                let scene_schedule_mut = &mut config.scene_schedule;
                let reduced_motion_mut = &mut config.global.reduced_motion;
                let large_bubbles_mut = &mut config.global.large_bubbles;
                let hover_startle_mut = &mut config.global.hover_startle;
                let on_fullscreen_mut = &mut config.global.on_fullscreen;
                let always_on_top_mut = &mut config.global.always_on_top;
                let step_aside_apps_mut = &mut config.global.step_aside_apps;
                let bedtime_mut = &mut config.global.bedtime;
                let moods_mut = &mut config.global.moods;
                let tap_characters_mut = &mut config.global.tap_characters;
                let show_toggle_button_mut = &mut config.global.show_toggle_button;
                let little_effects_mut = &mut config.global.little_effects;
                let idle_minutes_mut = &mut config.global.pause_when_idle_minutes;
                let on_battery_mut = &mut config.global.pause_on_battery;
                let frame_rate_mut = &mut config.global.frame_rate;
                let welcome_back_mut = &mut config.global.welcome_back;
                let sound_volume_mut = &mut config.global.sound_volume;
                let sounds_muted_mut = &mut config.global.sounds_muted;
                let quiet_at_bedtime_mut = &mut config.global.quiet_at_bedtime;
                let start_at_login_mut = &mut config.global.start_at_login;
                let idle_available = Some(layer.idle_available());
                let accesskit_mut = &mut config.global.accesskit_enabled;
                let keybindings_mut = &mut config.keybindings;
                let collapse_state_mut = &mut config.collapse_state;
                let last_seen_whats_new_mut = &mut config.global.last_seen_whats_new;
                let warnings_ref = &warnings;
                let hotkey_backend_ref = *hotkey_backend_status;
                let shimeji_import_ref = &mut shimeji_import;
                let scene_request_ref = &mut scene_request;
                let monitors_ref = monitors.as_slice();
                let toasts_ref = &toasts;
                let toggle_requested_ref = &mut toggle_requested;
                let palette_ref = &mut palette_outcome;
                let library_ref = &mut library_outcome;
                let menu_outcome_ref = &mut menu_outcome;
                let perf_sampler_ref = &perf_sampler;
                let perf_export_ref = &mut perf_export_request;
                let perf_visible_snapshot = *perf_overlay_visible;
                if let Some(output) = output {
                    let view = output.create_view();
                    let egui_start = Instant::now();
                    egui_renderer.set_accesskit_allowed(accesskit_snapshot);
                    egui_renderer.set_window_state(
                        layer.has_keyboard_focus(),
                        primary_origin,
                        size,
                    );
                    egui_renderer.render(
                        &renderer.shared.device,
                        &renderer.shared.queue,
                        &view,
                        pixel_size,
                        pixels_per_point,
                        events,
                        modifiers,
                        |ctx| {
                            crate::ui::motion::set_reduced(ctx, *reduced_motion_mut);
                            if hidden_snapshot {
                                // Hidden means hidden: no sprites (the draw
                                // list is empty above) and no UI either, so
                                // the ⚙ button doesn't linger over a
                                // supposedly hidden overlay. Restore comes
                                // through the same D-Bus/keybinding channel
                                // that hid it, exactly as on the winit path.
                                return;
                            }
                            if (edit_mode_snapshot || show_button_snapshot)
                                && panels::toggle_button(ctx, edit_mode_snapshot)
                            {
                                *toggle_requested_ref = true;
                            }
                            // Positions here are already in points: pointer
                            // events map to them one to one.
                            crate::ui::effects::paint(ctx, &effects, 1.0);
                            crate::ui::speech::paint(ctx, &bubbles, 1.0);
                            // And to a screen reader (`crate::announce`), with
                            // the toasts.
                            crate::ui::speech::announce(ctx);
                            crate::ui::toasts::announce(ctx, toasts_ref);
                            if crate::ui::onboarding::coach_marks(
                                ctx,
                                onboarding_mut,
                                edit_mode_snapshot,
                                show_button_snapshot,
                                keybindings_mut,
                            ) {
                                *config_dirty_mut = true;
                            }
                            {
                                panels::settings(
                                    ctx,
                                    edit_mode_snapshot,
                                    scene_mut,
                                    selection_mut,
                                    config_dirty_mut,
                                    theme_mut,
                                    locale_mut,
                                    onboarding_mut,
                                    monitor_mode_mut,
                                    window_awareness_mut,
                                    // Native Wayland exposes no window positions.
                                    false,
                                    snap_mut,
                                    bump_mut,
                                    greet_mut,
                                    active_scene_mut,
                                    reminders_mut,
                                    scene_schedule_mut,
                                    focus_mut,
                                    reminder_sound_mut,
                                    notify_mut,
                                    // A layer surface belongs to one output, so no single
                                    // surface can span the desktop here.
                                    false,
                                    reduced_motion_mut,
                                    large_bubbles_mut,
                                    hover_startle_mut,
                                    on_fullscreen_mut,
                                    step_aside_apps_mut,
                                    always_on_top_mut,
                                    tap_characters_mut,
                                    show_toggle_button_mut,
                                    little_effects_mut,
                                    bedtime_mut,
                                    moods_mut,
                                    panels::AwayControls {
                                        idle_minutes: &mut *idle_minutes_mut,
                                        on_battery: &mut *on_battery_mut,
                                        frame_rate: &mut *frame_rate_mut,
                                        welcome_back: &mut *welcome_back_mut,
                                        idle_available,
                                    },
                                    panels::SoundControls {
                                        volume: &mut *sound_volume_mut,
                                        muted: &mut *sounds_muted_mut,
                                        quiet_at_bedtime: &mut *quiet_at_bedtime_mut,
                                    },
                                    start_at_login_mut,
                                    monitors_ref,
                                    library.as_ref(),
                                    library_ref,
                                    keybindings_mut,
                                    collapse_state_mut,
                                    accesskit_mut,
                                    // `crate::a11y` carries the AT-SPI bridge.
                                    true,
                                    warnings_ref,
                                    last_seen_whats_new_mut,
                                    hotkey_backend_ref,
                                    shimeji_import_ref,
                                    scene_request_ref,
                                );
                                if edit_mode_snapshot {
                                    if let Some(state) = &menu_state {
                                        *menu_outcome_ref =
                                            Some(panels::context_menu(ctx, state, menu_offers));
                                    }
                                    *palette_ref = panels::command_palette(
                                        ctx,
                                        keybindings_mut,
                                        selection_mut.selected_index().is_some(),
                                    );
                                }
                                // In both modes, as on the winit path.
                                panels::toasts(ctx, toasts_ref);
                                // Above every panel, so someone chasing a
                                // stutter doesn't have to hunt for it behind
                                // one — same placement as the winit path.
                                if perf_visible_snapshot
                                    && crate::ui::perf_overlay::show(
                                        ctx,
                                        perf_sampler_ref,
                                        crate::perf::read_rss_kib(),
                                        gpu_stats,
                                    )
                                    .is_some()
                                {
                                    *perf_export_ref = true;
                                }
                            }
                        },
                    );
                    perf_sampler.add(crate::perf::Category::EguiPaint, egui_start.elapsed());
                    // A focused text field turns the input method on, at its
                    // caret; losing it turns it off.
                    layer.sync_ime(egui_renderer.ime_caret());
                    // Copied from a text field: onto the desktop's clipboard, as
                    // plain text.
                    if let Some(text) = egui_renderer.take_copied_text() {
                        layer.claim_clipboard(text, false);
                    }
                    if perf_export_request {
                        match crate::perf::export_snapshot(perf_sampler) {
                            Ok(path) => {
                                // Toast shows the full path — the user asked
                                // for the export and wants to find the file.
                                // The log redacts, so journald doesn't carry
                                // their home directory.
                                tracing::info!("Perf snapshot written: {}", redact_path(&path));
                                tracing::debug!("Perf snapshot full path: {}", path.display());
                                let mut args = fluent::FluentArgs::new();
                                args.set("path", path.display().to_string());
                                toasts.success(crate::i18n::t_args("toast-perf-snapshot", &args));
                            }
                            Err(e) => {
                                tracing::error!("Perf snapshot failed: {e}");
                                let mut args = fluent::FluentArgs::new();
                                args.set("error", e.to_string());
                                toasts.error(crate::i18n::t_args(
                                    "toast-perf-snapshot-failed",
                                    &args,
                                ));
                            }
                        }
                    }
                    {
                        let _s = perf_sampler.scope(crate::perf::Category::Present);
                        renderer.present(output);
                    }
                    *primary_frame = Some(crate::app::windows::LastFrame {
                        key: primary_key,
                        overlay: primary_busy,
                        at: now,
                    });
                }
                // Extras: the entities pinned (or resolved by position) to
                // that monitor, translated by its own origin, and the speech
                // bubbles among them. Mirrors
                // `app::windows::render_extra_windows` on the X11 path.
                // Hidden, they clear like the primary: the X11 path unmaps
                // its extra windows, but these stay mapped, and they kept
                // showing the characters on the other monitors.
                if !extra_surfaces.is_empty() {
                    let extra_visible = if *overlay_hidden || aside.hidden {
                        Vec::new()
                    } else {
                        scene.drawn_entities()
                    };
                    for (name, surface) in extra_surfaces.iter_mut() {
                        let Some(mon) = monitors_now.iter().find(|m| &m.name == name) else {
                            continue;
                        };
                        let drawn: Vec<&Entity> = extra_visible
                            .iter()
                            .copied()
                            .filter(|e| {
                                crate::app::windows::entity_on_monitor(monitors_now, e, name)
                            })
                            .collect();
                        let origin = (mon.x as f32, mon.y as f32);
                        // Its own egui, run while there are bubbles here —
                        // and once more when one asked for the frame that
                        // clears it.
                        let bubbles = if *overlay_hidden || aside.hidden {
                            Vec::new()
                        } else {
                            crate::speech::shown(scene, origin, |e| {
                                crate::app::windows::entity_on_monitor(monitors_now, e, name)
                            })
                        };
                        let effects = if *overlay_hidden || aside.hidden {
                            Vec::new()
                        } else {
                            scene.effects_shown(
                                origin,
                                (surface.window_width as f32, surface.window_height as f32),
                                now,
                            )
                        };
                        let asked = extra_eguis.get(name).is_some_and(|e| e.repaint_due(now));
                        // Hidden means hidden, as on the panel's surface.
                        let menu_here = extra_menu.as_ref().filter(|m| {
                            m.surface.as_ref() == Some(name) && !(*overlay_hidden || aside.hidden)
                        });
                        let painted = !bubbles.is_empty()
                            || !effects.is_empty()
                            || asked
                            || menu_here.is_some();
                        // Not drawn when it would show what it shows
                        // already, as on X11 (`app::windows::needs_frame`):
                        // one monitor's walker no longer redraws the rest.
                        let key = surface.frame_key(
                            &renderer.shared,
                            &drawn,
                            &scene.groups,
                            edit_bar,
                            marks,
                            origin,
                        );
                        if !crate::app::windows::needs_frame(
                            extra_frames.get(name).copied(),
                            key,
                            painted,
                            now,
                        ) {
                            continue;
                        }
                        match surface.render(
                            &renderer.shared,
                            &drawn,
                            &scene.groups,
                            edit_bar,
                            marks,
                            origin,
                        ) {
                            Ok(extra_output) => {
                                if let Some(errors) = extra_render_errors.get_mut(name) {
                                    errors.drew();
                                }
                                if painted {
                                    let egui =
                                        extra_eguis.entry(name.clone()).or_insert_with(|| {
                                            crate::ui::surface_egui::SurfaceEgui::new(
                                                &renderer.shared.device,
                                                renderer.shared.surface_format,
                                                config.global.theme,
                                            )
                                        });
                                    egui.ensure_theme(config.global.theme);
                                    let view = extra_output.create_view();
                                    egui.render(
                                        &renderer.shared.device,
                                        &renderer.shared.queue,
                                        &view,
                                        surface.pixel_size(),
                                        surface.pixel_scale(),
                                        if menu_here.is_some() {
                                            std::mem::take(&mut extra_menu_events)
                                        } else {
                                            Vec::new()
                                        },
                                        egui::Modifiers::default(),
                                        |ctx| {
                                            // In points already, as on the primary.
                                            crate::ui::effects::paint(ctx, &effects, 1.0);
                                            crate::ui::speech::paint(ctx, &bubbles, 1.0);
                                            if let Some(state) = menu_here {
                                                menu_outcome = Some(panels::context_menu(
                                                    ctx,
                                                    state,
                                                    menu_offers,
                                                ));
                                            }
                                        },
                                    );
                                }
                                surface.present(&renderer.shared, extra_output);
                                extra_frames.insert(
                                    name.clone(),
                                    crate::app::windows::LastFrame {
                                        key,
                                        overlay: painted,
                                        at: now,
                                    },
                                );
                            }
                            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                                let (w, h) = (surface.window_width, surface.window_height);
                                surface.resize(&renderer.shared, w, h);
                                extra_frames.remove(name);
                            }
                            Err(wgpu::SurfaceError::Timeout) => {}
                            Err(e) => extra_render_errors
                                .entry(name.clone())
                                .or_default()
                                .failed(name, &e),
                        }
                    }
                }
                // Monitors gone, and their eguis and frames with them.
                extra_eguis.retain(|name, _| extra_surfaces.contains_key(name));
                extra_frames.retain(|name, _| extra_surfaces.contains_key(name));
                asked = Some(Asked {
                    toggle_requested,
                    scene_request,
                    shimeji_import,
                    menu_outcome,
                    palette_outcome,
                    library_outcome,
                });
            }
            // Reconfigured, as on X11: an outdated surface was left as it
            // was, and failed again every frame.
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                renderer.resize(
                    renderer.primary.window_width,
                    renderer.primary.window_height,
                );
                *primary_frame = None;
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                return Err(AnimaError::other("GPU out of memory"));
            }
            // Transient, as on X11: this frame is dropped, the next tries.
            Err(wgpu::SurfaceError::Timeout) => {
                tracing::debug!("surface acquire timed out; skipping frame");
            }
            Err(e) => render_errors.failed("the overlay", &e),
        }

        Ok(asked)
    }

    /// What the panels asked for this frame, once every overlay drew:
    /// edit mode from the ⚙, a scene to share, import, back up or restore,
    /// a file to add, a Shimeji pack, the right-click menu's action, the
    /// palette's pick, the library's.
    pub(super) fn apply_asked(&mut self, asked: Asked, primary_origin: (f32, f32)) {
        let Asked {
            toggle_requested,
            scene_request,
            shimeji_import,
            menu_outcome,
            palette_outcome,
            library_outcome,
        } = asked;
        let Self {
            palette_actions,
            pending_transfer,
            pending_file_chooser,
            pending_shimeji,
            library_root,
            library,
            context_menu_state,
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
        if toggle_requested {
            flip_edit_mode(
                &mut *layer,
                &mut *config,
                scene,
                &mut *config_dirty,
                &mut *config_watch,
                "Wayland, toggle button",
            );
        }
        // Palette / library outcomes apply outside the egui
        // closure where we can take &mut *renderer + &mut *toasts
        // without conflicting.
        match &scene_request {
            // Pasted with the palette's picks, next frame.
            Some(panels::SceneRequest::Paste) => palette_actions.push(Action::Paste),
            Some(panels::SceneRequest::Share(path)) if pending_transfer.is_none() => {
                *pending_transfer = crate::outcomes::SceneTransfer::share(path, &mut *toasts);
            }
            Some(panels::SceneRequest::Import) if pending_transfer.is_none() => {
                *pending_transfer = Some(crate::outcomes::SceneTransfer::import_chooser());
            }
            Some(panels::SceneRequest::BackUp) if pending_transfer.is_none() => {
                *pending_transfer = Some(crate::outcomes::SceneTransfer::back_up(config, scene));
            }
            Some(panels::SceneRequest::Restore) if pending_transfer.is_none() => {
                *pending_transfer = Some(crate::outcomes::SceneTransfer::restore_chooser());
            }
            _ => {}
        }
        if scene_request == Some(panels::SceneRequest::AddFile) && pending_file_chooser.is_none() {
            *pending_file_chooser = Some(crate::outcomes::FileChooserAdd::start((
                renderer.primary.window_width as f32 / 2.0 + primary_origin.0,
                renderer.primary.window_height as f32 / 2.0 + primary_origin.1,
            )));
        }
        if let Some(path) = shimeji_import {
            // Off the UI thread, like a dropped pack. This used to
            // import synchronously right here, freezing the overlay
            // for as long as the sprite copy took.
            if pending_shimeji.is_none() {
                let at = (
                    renderer.primary.window_width as f32 / 2.0 + primary_origin.0,
                    renderer.primary.window_height as f32 / 2.0 + primary_origin.1,
                );
                *pending_shimeji = ShimejiImport::start(
                    &crate::config::AppConfig::resolve_asset_path(&path),
                    library_root.as_deref(),
                    at,
                    &mut *toasts,
                );
            }
        }
        if let Some(out) = menu_outcome {
            match out {
                panels::ContextMenuOutcome::Open { settled } => {
                    // Showing, and the click that opened it is
                    // over: a *subsequent* click may now dismiss it.
                    if let Some(state) = context_menu_state.as_mut() {
                        state.armed |= settled;
                    }
                }
                panels::ContextMenuOutcome::Close => {
                    *context_menu_state = None;
                }
                // Asks where first, as sharing a scene does.
                panels::ContextMenuOutcome::Action(panels::MenuAction::SaveGif(idx)) => {
                    if pending_transfer.is_none() {
                        *pending_transfer = crate::outcomes::SceneTransfer::save_gif(scene, idx);
                    }
                    *context_menu_state = None;
                }
                panels::ContextMenuOutcome::Action(panels::MenuAction::ShareCharacter(idx)) => {
                    if pending_transfer.is_none() {
                        *pending_transfer =
                            crate::outcomes::SceneTransfer::share_character(scene, idx);
                    }
                    *context_menu_state = None;
                }
                panels::ContextMenuOutcome::Action(action) => {
                    outcomes::apply_menu_action(
                        action,
                        &mut outcome_ctx!(scene, selection, toasts, config_dirty, renderer),
                    );
                    *context_menu_state = None;
                }
            }
        }
        match palette_outcome {
            Some(panels::PaletteOutcome::RunAction(action)) => palette_actions.push(action),
            Some(panels::PaletteOutcome::AddFile) => {
                if pending_file_chooser.is_none() {
                    *pending_file_chooser = Some(crate::outcomes::FileChooserAdd::start((
                        renderer.primary.window_width as f32 / 2.0 + primary_origin.0,
                        renderer.primary.window_height as f32 / 2.0 + primary_origin.1,
                    )));
                }
            }
            // As the tray's would: snoozing, out of edit mode.
            Some(panels::PaletteOutcome::Snooze(what)) => {
                crate::snooze::snooze(what);
                if layer.state.edit_mode {
                    flip_edit_mode(
                        &mut *layer,
                        &mut *config,
                        scene,
                        &mut *config_dirty,
                        &mut *config_watch,
                        "Wayland, palette snooze",
                    );
                }
            }
            Some(out) => outcomes::apply_palette_outcome(
                out,
                &mut outcome_ctx!(scene, selection, toasts, config_dirty, renderer),
                &mut *config,
            ),
            None => {}
        }
        if let Some(out) = library_outcome {
            // The middle of the primary output, in the global
            // coordinates the scene uses.
            let at = (
                renderer.primary.window_width as f32 / 2.0 + primary_origin.0,
                renderer.primary.window_height as f32 / 2.0 + primary_origin.1,
            );
            outcomes::apply_library_outcome(
                out,
                &mut outcome_ctx!(scene, selection, toasts, config_dirty, renderer),
                library_root.as_deref(),
                &mut *library,
                at,
            );
        }
    }
}
