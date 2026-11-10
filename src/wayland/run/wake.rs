//! The start of a turn: the wait for the compositor, the overlay closed
//! or resized, the hot reload and the results the loop waits on, and the
//! monitors as they are now ([`super::Native::iteration`]).

use super::*;

impl Native {
    /// Waits for the compositor — at most `wait`, or until the waker
    /// rings — and takes what it sent: whether any of it is something a
    /// frame should show.
    pub(super) fn wake(&mut self) -> Result<bool> {
        let Self {
            wait,
            waker,
            history,
            perf_sampler,
            selection,
            egui_renderer,
            layer,
            scene,
            ..
        } = self;
        // Opens the perf frame before the blocking dispatch, so the wait
        // for the compositor lands in `Idle` rather than vanishing — the
        // same placement the winit loop uses relative to its own wait.
        perf_sampler.begin_frame();
        let dispatched = dispatch_with_timeout(
            &mut layer.event_queue,
            &mut layer.state,
            waker.as_ref(),
            *wait,
        )?;
        // Whether anything happened this iteration that a frame should
        // show. Input and every other compositor event count; so does
        // whatever the loop picks up from its channels below.
        let activity = dispatched > 0;
        // A step to undo opens at the first thing someone does, before it
        // is applied (`crate::undo`).
        if layer.has_user_action() || egui_renderer.has_screen_reader_requests() {
            history.input(scene, selection, Instant::now());
        }

        Ok(activity)
    }

    /// The overlay closed by the compositor, or a quit asked for
    /// ([`Closing`]): what the loop does next, if not carry on.
    pub(super) fn overlay_closed(&mut self, activity: &mut bool) -> Option<Flow> {
        let Self {
            quit_requested,
            reopens,
            wait,
            primary_frame,
            drag_widened,
            aside,
            renderer,
            layer,
            overlay_hidden,
            config,
            ..
        } = self;
        // The compositor closed the overlay: its monitor went — unplugged,
        // or Hyprland's stand-in for none replaced by a real one, which
        // ended an app started from `exec-once`. The session goes on, so it
        // is made again once a monitor is there; only a compositor that
        // keeps closing it ends the app, as every close did before
        // (`Reopens`). A quit is not a close (`Closing`).
        match closing(
            *quit_requested,
            layer.state.close_requested,
            !layer.monitors().is_empty(),
            &mut *reopens,
            Instant::now(),
        ) {
            Closing::Open => {}
            Closing::Quit => return Some(Flow::Exit),
            Closing::WaitForMonitor => {
                *wait = Duration::from_millis(500);
                return Some(Flow::Next);
            }
            Closing::GiveUp => {
                tracing::info!("Layer surface closed by compositor again and again — exiting.");
                return Some(Flow::Exit);
            }
            Closing::Reopen => match reopen_primary(&mut *layer, &mut *renderer) {
                Ok(()) => {
                    tracing::info!("Layer surface closed by compositor — made again");
                    let region = region_for_state(
                        *overlay_hidden || aside.hidden,
                        layer.state.edit_mode || *drag_widened,
                        renderer.primary.window_width,
                        renderer.primary.window_height,
                        shown_button(&config.global),
                    );
                    if let Err(e) = layer.set_input_region(region) {
                        tracing::warn!("input region after reopening: {e}");
                    }
                    *primary_frame = None;
                    *activity = true;
                }
                Err(e) => {
                    tracing::warn!(
                        "Layer surface closed by compositor, not made again ({e}) — exiting."
                    );
                    return Some(Flow::Exit);
                }
            },
        }

        None
    }

    /// A new size from the compositor: the buffers made again at it,
    /// and the input region from the live state.
    pub(super) fn resized(&mut self) -> Result<()> {
        let Self {
            drag_widened,
            aside,
            renderer,
            layer,
            overlay_hidden,
            config,
            ..
        } = self;
        // Pick up any resize the compositor sent us.
        if let Some((new_w, new_h)) = layer.state.pending_size.take() {
            if new_w != renderer.primary.window_width || new_h != renderer.primary.window_height {
                renderer.resize(new_w, new_h);
                // Re-derive from the live state: this used to re-apply the
                // pass-through corner unconditionally, so a resize while
                // in edit mode silently reverted the surface to
                // click-through, and a resize while hidden made it
                // clickable again.
                let region = region_for_state(
                    *overlay_hidden || aside.hidden,
                    layer.state.edit_mode || *drag_widened,
                    new_w,
                    new_h,
                    shown_button(&config.global),
                );
                layer.set_input_region(region)?;
                tracing::info!("Layer surface resized to {new_w}×{new_h}");
            }
        }

        Ok(())
    }

    /// config.toml edited by hand (the hot reload), and the results the
    /// loop waits on: a Shimeji import, the file chooser's answer, a
    /// paste, a shared scene coming in or going out.
    pub(super) fn results(&mut self, activity: &mut bool) {
        let Self {
            history,
            pending_paste,
            pending_transfer,
            pending_file_chooser,
            pending_shimeji,
            library_root,
            warnings,
            config_watch,
            config_dirty,
            toasts,
            marquee,
            drag,
            selection,
            renderer,
            layer,
            config,
            scene,
            ..
        } = self;
        // Hot-reload. The README lists it as a feature and its parity
        // table calls it stable on both backends; this loop never did it
        // at all, so editing config.toml with the overlay running on
        // Wayland did nothing (R28).
        let poll = config_watch.poll(*config_dirty);
        *activity |= !matches!(poll, crate::config_watch::Poll::Idle);
        let reloaded = matches!(poll, crate::config_watch::Poll::Ready(_));
        if reloaded {
            // The characters a drag or a rectangle holds are about to be
            // replaced: let go while they are still the ones it froze.
            crate::input::multi::cancel(&mut *scene, &mut *selection, &mut *drag, &mut *marquee);
        }
        crate::config_watch::handle(
            poll,
            &mut outcome_ctx!(scene, selection, toasts, config_dirty, renderer),
            &mut *config,
            &mut *warnings,
        );
        if reloaded {
            // The scene was replaced from the file: the steps no longer
            // describe it.
            history.clear();
        }
        if let Some(import) = &pending_shimeji {
            if import
                .poll(&mut outcome_ctx!(
                    scene,
                    selection,
                    toasts,
                    config_dirty,
                    renderer
                ))
                .is_some()
            {
                *pending_shimeji = None;
                *activity = true;
            }
        }
        if let Some(chooser) = &pending_file_chooser {
            if let Some(added) = chooser.poll(&mut outcome_ctx!(
                scene,
                selection,
                toasts,
                config_dirty,
                renderer
            )) {
                *pending_file_chooser = None;
                *activity = true;
                if added > 0 {
                    *config_dirty = true;
                }
            }
        }
        if let Some((answer, at)) = &pending_paste {
            let content = match answer.try_recv() {
                Ok(content) => Some(content),
                Err(std::sync::mpsc::TryRecvError::Empty) => None,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    Some(crate::system_clipboard::Content::Nothing)
                }
            };
            if let Some(content) = content {
                let at = *at;
                *pending_paste = None;
                *activity = true;
                // The key that asked may be a while ago: open a step if it
                // closed.
                history.input(scene, selection, Instant::now());
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
                            *pending_transfer =
                                Some(crate::outcomes::SceneTransfer::import(path, Some(at)));
                        }
                    } else if pending_shimeji.is_none() {
                        *pending_shimeji =
                            ShimejiImport::start(&path, library_root.as_deref(), at, &mut *toasts);
                    }
                }
            }
        }
        if let Some(transfer) = &mut *pending_transfer {
            match transfer.poll(&mut *toasts) {
                crate::outcomes::Transfer::Pending => {}
                crate::outcomes::Transfer::Done => {
                    *pending_transfer = None;
                    *activity = true;
                }
                crate::outcomes::Transfer::Imported(saved) => {
                    *pending_transfer = None;
                    *activity = true;
                    // Not input, so no undo step is open: open one, as
                    // "Next scene" does; and let go of any gesture.
                    history.input(scene, selection, Instant::now());
                    crate::input::multi::cancel(
                        &mut *scene,
                        &mut *selection,
                        &mut *drag,
                        &mut *marquee,
                    );
                    let bounds = monitor::covered_bounds(
                        &monitor::plan_windows(&config.global.monitor_mode, &layer.monitors()),
                        (
                            renderer.primary.window_width as f32,
                            renderer.primary.window_height as f32,
                        ),
                    );
                    outcomes::apply_imported(
                        saved,
                        &mut outcome_ctx!(scene, selection, toasts, config_dirty, renderer),
                        &mut config.global.active_scene,
                        bounds,
                    );
                }
            }
        }
    }

    /// The monitors as they are now: the other monitors' overlays made
    /// or closed to match them, every surface at its output's scale,
    /// and where the primary sits among them ([`Outputs`]).
    pub(super) fn outputs(&mut self, activity: &mut bool) -> Outputs {
        let Self {
            loop_started,
            last_monitor_mode,
            extra_frames,
            extra_surfaces,
            renderer,
            layer,
            config,
            scene,
            ..
        } = self;
        // Rebuild PerMonitor extras whenever the user switches mode or
        // the output topology changes (hotplug) — mirrors the X11
        // path's `rebuild_windows_if_mode_changed` + `check_monitor_topology`.
        // Computed early in the frame: file-drop landing coordinates
        // and the right-click hit test below both need `primary_origin`
        // translated into entity-space before they touch `scene`.
        let monitors_now = layer.monitors();
        let plan = monitor::plan_windows(&config.global.monitor_mode, &monitors_now);
        let plan_names: std::collections::HashSet<&str> =
            plan.extras.iter().map(|m| m.name.as_str()).collect();
        let current_names: std::collections::HashSet<&str> =
            extra_surfaces.keys().map(|s| s.as_str()).collect();
        // Which monitor the primary landed on is known once the compositor
        // says it entered one — after its first frame. Until then, for a
        // moment at most, no other monitor gets its overlay and nothing is
        // drawn: the plan guessed the first monitor listed, made an
        // overlay for the primary's own monitor when the guess was wrong,
        // and closed it again some 40 ms later.
        let awaiting_primary = awaiting_primary_output(
            &config.global.monitor_mode,
            monitors_now.len(),
            layer.state.primary_output_name.is_some(),
            (*loop_started).elapsed(),
        );
        if !awaiting_primary
            && (config.global.monitor_mode != *last_monitor_mode || plan_names != current_names)
        {
            rebuild_extra_surfaces(&mut *layer, renderer, &plan, &mut *extra_surfaces);
            // New surfaces, with nothing on them yet.
            extra_frames.clear();
            *last_monitor_mode = config.global.monitor_mode.clone();
        }
        for (name, new_w, new_h) in layer.drain_extra_resizes() {
            if let Some(surface) = extra_surfaces.get_mut(&name) {
                surface.resize(&renderer.shared, new_w, new_h);
            }
        }
        // Each surface at its output's scale (`layer_window::scale`),
        // as the compositor last said: its buffer made again at it.
        let primary_scale = layer.state.primary_scale.scale();
        if renderer.primary.pixel_scale() != primary_scale {
            renderer
                .primary
                .set_pixel_scale(&renderer.shared, primary_scale);
            tracing::info!("Overlay drawn at scale {primary_scale}");
            *activity = true;
        }
        for (name, surface) in extra_surfaces.iter_mut() {
            let scale = layer.extra_scale(name);
            if surface.pixel_scale() != scale {
                surface.set_pixel_scale(&renderer.shared, scale);
                tracing::info!("Overlay on {name} drawn at scale {scale}");
                *activity = true;
            }
        }
        // The pictures as sharp as the sharpest of them shows them
        // (`Scene::sharpen`): the scene was loaded before any said.
        let detail = (extra_surfaces.keys())
            .map(|name| layer.extra_scale(name))
            .fold(primary_scale, f32::max);
        crate::entity::set_detail_scale(detail);
        if scene.sharpen(crate::entity::picture_cap()) {
            *activity = true;
        }

        // Entities live in window-local coordinates in single-output
        // modes (today's only well-exercised case) and in global
        // desktop coordinates once PerMonitor extras exist — same
        // duality as the X11 path's `App::primary_origin` (T.8).
        // `primary_output_name` is learned from `surface_enter`
        // (handlers.rs); it stays `None` (→ identity origin) if the
        // compositor never sends it, so the fallback is always the
        // already-shipped single-output behavior, never a wrong offset.
        let primary_origin: (f32, f32) = compute_primary_origin(
            !extra_surfaces.is_empty(),
            layer.state.primary_output_name.as_deref(),
            &monitors_now,
        );
        let cursor_global = layer
            .state
            .cursor_pos
            .map(|(x, y)| (x + primary_origin.0, y + primary_origin.1));

        Outputs {
            monitors_now,
            plan,
            awaiting_primary,
            primary_origin,
            cursor_global,
        }
    }
}
