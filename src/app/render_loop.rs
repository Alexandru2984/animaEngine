//! Per-frame render pipeline. Extracted in H.4a so the
//! `WindowEvent::RedrawRequested` arm stays a single delegating call
//! instead of a 250-line nested block.
//!
//! Flow per frame:
//!
//! 1. Mark the perf-sampler start; refresh RSS once a second.
//! 2. Hot-reload check (drains worker channel, maybe spawns one).
//! 3. Scene tick (behaviours + physics + animation step).
//! 4. wgpu sprite render → surface texture.
//! 5. egui pass painting the toggle button + (in edit mode) the
//!    full settings panel, command palette, context menu, toasts,
//!    perf overlay.
//! 6. Apply outcomes the egui closure produced (toggle, menu,
//!    palette, library) — done outside the closure so we can take
//!    `&mut self.renderer` etc. without conflicting with the egui
//!    borrows captured above.
//! 7. Present + close the perf frame + request next redraw.

use super::App;
use crate::renderer::wgpu_renderer::WgpuRenderer;
use crate::ui::panels;
use std::time::{Duration, Instant};
use winit::event_loop::{ActiveEventLoop, ControlFlow};

/// How the render loop schedules its next frame. Computed at the end
/// of every redraw from the live scene/UI state.
pub(super) enum RedrawPacing {
    /// Something animates every tick (edit mode, toasts, behaviors,
    /// physics, perf overlay) — redraw at display refresh.
    Continuous,
    /// Scene is static except for playing animations — sleep until the
    /// soonest next-frame deadline (an 8 fps sprite wakes 8×/s, not 60×).
    Deadline(Instant),
    /// Nothing moves — sleep until the hot-reload heartbeat.
    Idle,
}

/// Idle wake-up cadence. Matches the hot-reload mtime poll interval —
/// the heartbeat exists so config edits still apply while the overlay
/// sits static.
pub(super) const IDLE_HEARTBEAT: Duration = Duration::from_secs(2);

/// Consecutive `Lost`/`Outdated` surface acquisitions before we stop
/// trusting `surface.configure()` to recover and rebuild the whole
/// renderer instead. A resize or an S3 resume produces a frame or two
/// of `Lost` that reconfigure clears; a driver reset / GPU hot-unplug /
/// device-lost-across-suspend does not, and only a fresh device+surface
/// recovers it. Kept small so recovery is prompt (each loss requests an
/// immediate redraw, so the streak burns through in a few frames), but
/// above the 1–2 frame transient so we don't rebuild for a blip.
pub(super) const SURFACE_LOSS_REBUILD_THRESHOLD: u32 = 8;

/// Escalation policy for a lost surface, factored out so it's unit-tested
/// without a GPU. Given the current consecutive-loss streak, returns the
/// next streak and whether to rebuild the renderer now. At the threshold
/// it signals a rebuild and resets the streak to 0 so a *post-rebuild*
/// loss starts a fresh count (and we don't thrash rebuilds every frame).
fn next_surface_loss_state(streak: u32) -> (u32, bool) {
    let next = streak.saturating_add(1);
    if next >= SURFACE_LOSS_REBUILD_THRESHOLD {
        (0, true)
    } else {
        (next, false)
    }
}

impl App {
    pub(super) fn handle_redraw_requested(&mut self, event_loop: &ActiveEventLoop) {
        // Mark the start of a perf frame. `begin_frame` resets
        // the in-progress sample; the overlay reads from the
        // ring buffer so it's safe to begin/end every frame
        // regardless of whether the overlay is visible.
        self.perf_sampler.begin_frame();
        // Refresh RSS once a second at 60 fps. /proc syscall is
        // cheap but per-frame would still be visible at the
        // microsecond scale the overlay reports.
        const RSS_REFRESH_FRAMES: u32 = 60;
        if self.perf_frame_counter.is_multiple_of(RSS_REFRESH_FRAMES) {
            self.perf_last_rss_kib = crate::perf::read_rss_kib();
        }
        self.perf_frame_counter = self.perf_frame_counter.wrapping_add(1);

        // Self-heal click-through. GNOME/XWayland Mutter resets our
        // XShape input region when it restacks the overlay for
        // always-on-top, which can leave pass-through mode swallowing
        // every click (user can't reach windows under the sprites).
        // Re-applying the shape on a slow cadence guarantees it recovers
        // even when the event-driven re-apply loses the race with the
        // compositor's reset. Pass-through only — edit mode owns the
        // full-window shape. The XShape set is cheap and invisible.
        if !self.edit_mode
            && self.last_shape_refresh.elapsed() >= std::time::Duration::from_millis(500)
        {
            self.reapply_input_shape();
            self.last_shape_refresh = Instant::now();
        }

        // Capture-and-reset the previous frame's GPU op counters (W.3).
        // Read-only on the renderer (the counters are `Cell`s), so it
        // doesn't conflict with the `&mut renderer` borrow later.
        if let Some(renderer) = self.renderer.as_ref() {
            let (uploads, draws) = renderer.shared.take_frame_gpu_counters();
            self.gpu_uploads = uploads;
            self.gpu_draws = draws;
        }

        // Soak metrics (W.1) — no-op unless ANIMA_SOAK_METRICS is set.
        // Read before the &mut renderer borrow below; texture count
        // lags by at most one frame, which is irrelevant at the
        // 60-second sampling interval.
        if let Some(soak) = self.soak.as_mut() {
            let decoded = self.scene.total_decoded_bytes();
            let textures = self
                .renderer
                .as_ref()
                .map(|r| r.shared.textures.len())
                .unwrap_or(0);
            let p95 = self
                .perf_sampler
                .recent_p95_total(60)
                .map(|d| d.as_micros());
            soak.maybe_sample(self.perf_last_rss_kib, decoded, textures, p95);
        }

        // Check for external config changes (hot-reload)
        self.check_hot_reload();
        self.check_shimeji_import();

        // Appearance-tab monitor-mode switches rebuild the extra
        // overlay windows (T.6); topology changes do the same (T.9).
        self.rebuild_windows_if_mode_changed(event_loop);
        self.check_monitor_topology(event_loop);
        self.poll_window_platforms();

        // Tick behavior + physics + animation.
        let (screen_w, screen_h) = self
            .window
            .as_ref()
            .map(|w| {
                let s = w.inner_size();
                (s.width as f32, s.height as f32)
            })
            .unwrap_or((1920.0, 1080.0));
        // FollowCursor uses the live mouse position. In pass-through
        // mode XShape blocks CursorMoved outside the toggle button, so
        // winit's last-tracked position goes stale there. In edit mode
        // the window has full input and CursorMoved keeps it fresh, so
        // only pay for the extra X11 round trip when both pass-through
        // is active and some entity actually needs it. Wayland has no
        // equivalent read (and none of its CursorMoved delivery is
        // shape-restricted in the way XShape is for X11), so this is
        // X11-only — the same scope as window-awareness.
        let cursor = if !self.edit_mode
            && (self.scene.has_cursor_follower() || self.config.global.hover_startle)
        {
            self.x11_input
                .as_ref()
                .and_then(|mgr| mgr.query_pointer_global())
                .or(Some((self.mouse_x, self.mouse_y)))
        } else {
            Some((self.mouse_x, self.mouse_y))
        };
        {
            let _s = self.perf_sampler.scope(crate::perf::Category::SceneUpdate);
            self.scene
                .set_reduced_motion(self.config.global.reduced_motion);
            self.scene
                .set_hover_startle(self.config.global.hover_startle);
            self.scene.tick(screen_w, screen_h, cursor);
        }

        // Precompute multi-window facts before the renderer borrow —
        // the methods take &self and would conflict with &mut renderer.
        let primary_origin = self.primary_origin();
        let primary_monitor_name: Option<String> = if self.has_extra_windows() {
            crate::monitor::plan_windows(&self.config.global.monitor_mode, &self.monitors)
                .primary
                .map(|m| m.name)
        } else {
            None
        };

        // Surface-recovery bookkeeping for this frame. Resolved after
        // the renderer borrow below is released (we can't reassign
        // `self.renderer` while it's borrowed).
        let mut surface_lost = false;
        let mut rendered_ok = false;

        // Update textures for entities with changed frames
        if let Some(renderer) = &mut self.renderer {
            // Scene replacement paths (preset / palette Replace) swap the
            // entity list without renderer access; sweep their orphaned
            // textures here. No-op (two compares) when nothing is stale.
            renderer.prune_stale_textures(&self.scene.entities);
            for entity in &mut self.scene.entities {
                if entity.texture_dirty {
                    renderer.ensure_texture(entity);
                    entity.texture_dirty = false;
                }
            }

            // Get selected entity ID for highlight rendering
            let selected_id = self
                .selection
                .selected_index()
                .and_then(|idx| self.scene.entities.get(idx))
                .map(|e| e.id.as_str());

            // Render all visible entities. WgpuRenderer hands back the
            // surface texture without presenting so egui can overlay on
            // top of the same frame.
            //
            // We drop `visible` before the egui block so the immutable
            // borrow on self.scene is released and the UI can take a
            // mutable one to drive sliders / list mutations.
            let render_result = {
                let _s = self.perf_sampler.scope(crate::perf::Category::WgpuSubmit);
                let visible = self.scene.visible_entities();
                // In PerMonitor mode the primary window covers exactly
                // its monitor: entities live in global desktop coords
                // and translate by the monitor's origin (T.6). The
                // single-window modes keep the pre-0.6 identity origin.
                let drawn: Vec<&crate::entity::Entity> = match &primary_monitor_name {
                    Some(name) => visible
                        .into_iter()
                        .filter(|e| super::windows::entity_on_monitor(&self.monitors, e, name))
                        .collect(),
                    None => visible,
                };
                renderer.render(
                    &drawn,
                    &self.scene.groups,
                    self.edit_mode,
                    selected_id,
                    primary_origin,
                )
            };
            match render_result {
                Ok(output) => {
                    rendered_ok = true;
                    // egui runs in BOTH modes. In pass-through it
                    // paints just the toggle ⚙ button; in edit mode it
                    // adds the settings panel, context menu, toasts.
                    self.toasts.prune();

                    // GPU stats for the perf HUD (W.3). Built here while
                    // `renderer` and `self.scene` are still freely
                    // readable — before the disjoint &mut borrows below.
                    let gpu_stats = crate::ui::perf_overlay::GpuStats {
                        decoded_bytes: self.scene.total_decoded_bytes(),
                        texture_bytes: renderer.shared.texture_bytes(),
                        texture_count: renderer.shared.textures.len(),
                        uploads_last_frame: self.gpu_uploads,
                        draws_last_frame: self.gpu_draws,
                    };

                    let mut menu_outcome: Option<panels::ContextMenuOutcome> = None;
                    let mut palette_outcome: Option<panels::PaletteOutcome> = None;
                    let mut library_outcome: Option<panels::LibraryOutcome> = None;
                    let mut shimeji_import: Option<String> = None;
                    let mut toggle_requested = false;

                    if let (Some(ui), Some(window)) = (self.ui.as_mut(), self.window.as_ref()) {
                        // Pick up theme changes made in the settings
                        // panel before any panel paints this frame.
                        ui.ensure_theme(self.config.global.theme);

                        let view = output.create_view();
                        let size = [
                            renderer.primary.window_width,
                            renderer.primary.window_height,
                        ];

                        // Disjoint mutable borrows on disjoint fields.
                        let scene_mut = &mut self.scene;
                        let selection_mut = &mut self.selection;
                        let config_dirty_mut = &mut self.config_dirty;
                        let theme_mut = &mut self.config.global.theme;
                        let locale_mut = &mut self.config.global.locale;
                        let onboarding_mut = &mut self.config.global.onboarding;
                        let monitor_mode_mut = &mut self.config.global.monitor_mode;
                        let window_awareness_mut = &mut self.config.global.window_awareness;
                        let reduced_motion_mut = &mut self.config.global.reduced_motion;
                        let hover_startle_mut = &mut self.config.global.hover_startle;
                        // Snapshot the AccessKit flag BEFORE taking
                        // its mutable borrow — the render closure
                        // syncs egui's runtime gate from this copy
                        // each frame, and the closure also writes
                        // back through `accesskit_mut`. A new toggle
                        // therefore applies one frame later, which
                        // is below any perceivable lag.
                        let accesskit_enabled = self.config.global.accesskit_enabled;
                        let keybindings_mut = &mut self.config.keybindings;
                        let collapse_state_mut = &mut self.config.collapse_state;
                        let accesskit_mut = &mut self.config.global.accesskit_enabled;
                        let warnings_ref = &self.warnings;
                        let hotkey_backend_ref = self.hotkey_backend_status.as_str();
                        let last_seen_whats_new_mut = &mut self.config.global.last_seen_whats_new;
                        let perf_sampler_ref = &self.perf_sampler;
                        let perf_overlay_visible = self.perf_overlay_visible;
                        let perf_rss_kib = self.perf_last_rss_kib;
                        let mut perf_export_request = false;
                        let perf_export_request_ref = &mut perf_export_request;
                        let monitors_ref = self.monitors.as_slice();
                        let toasts_ref = &self.toasts;
                        let menu_state = self.ui_state.context_menu.clone();
                        let menu_outcome_ref = &mut menu_outcome;
                        let palette_outcome_ref = &mut palette_outcome;
                        let library_outcome_ref = &mut library_outcome;
                        let library_ref = self.library.as_ref();
                        let toggle_requested_ref = &mut toggle_requested;
                        let edit_mode = self.edit_mode;

                        // Manual elapsed measurement for the egui pass —
                        // the Scope guard would conflict with the
                        // perf_sampler_ref the overlay needs to read.
                        let egui_start = std::time::Instant::now();
                        ui.render(
                            window,
                            &renderer.shared.device,
                            &renderer.shared.queue,
                            &view,
                            size,
                            |ctx| {
                                // Sync the runtime AccessKit gate
                                // with the persisted preference each
                                // frame — both calls are idempotent
                                // flag writes, so the cost is
                                // negligible compared to leaving
                                // tree-update generation running
                                // when the user has it off.
                                if accesskit_enabled {
                                    ctx.enable_accesskit();
                                } else {
                                    ctx.disable_accesskit();
                                }
                                crate::ui::motion::set_reduced(ctx, *reduced_motion_mut);
                                // Toggle button is the only UI in
                                // pass-through; in edit mode it sits
                                // on top of everything else.
                                if panels::toggle_button(ctx, edit_mode) {
                                    *toggle_requested_ref = true;
                                }
                                // First-run tour (V.2): floats on the
                                // overlay in both modes; advances to
                                // its interactive steps in edit mode.
                                if crate::ui::onboarding::coach_marks(
                                    ctx,
                                    onboarding_mut,
                                    edit_mode,
                                ) {
                                    *config_dirty_mut = true;
                                }

                                // Unconditional call: `open` drives
                                // SidePanel::show_animated, so leaving
                                // edit mode slides the panel out
                                // instead of snapping it away.
                                panels::settings(
                                    ctx,
                                    edit_mode,
                                    scene_mut,
                                    selection_mut,
                                    config_dirty_mut,
                                    theme_mut,
                                    locale_mut,
                                    onboarding_mut,
                                    monitor_mode_mut,
                                    window_awareness_mut,
                                    reduced_motion_mut,
                                    hover_startle_mut,
                                    monitors_ref,
                                    library_ref,
                                    library_outcome_ref,
                                    keybindings_mut,
                                    collapse_state_mut,
                                    accesskit_mut,
                                    warnings_ref,
                                    last_seen_whats_new_mut,
                                    hotkey_backend_ref,
                                    &mut shimeji_import,
                                );
                                if edit_mode {
                                    if let Some(state) = &menu_state {
                                        *menu_outcome_ref = Some(panels::context_menu(ctx, state));
                                    }
                                    // Ctrl+K opens the command palette.
                                    *palette_outcome_ref = panels::command_palette(ctx);
                                    panels::toasts(ctx, toasts_ref);
                                }
                                // Perf overlay sits on top of every
                                // other surface so a user investigating
                                // a stutter doesn't have to chase it
                                // behind a panel.
                                if perf_overlay_visible
                                    && crate::ui::perf_overlay::show(
                                        ctx,
                                        perf_sampler_ref,
                                        perf_rss_kib,
                                        gpu_stats,
                                    )
                                    .is_some()
                                {
                                    *perf_export_request_ref = true;
                                }
                            },
                        );
                        // Closure's done; perf_sampler_ref's borrow ended.
                        // Safe to take a fresh &mut self.perf_sampler.
                        self.perf_sampler
                            .add(crate::perf::Category::EguiPaint, egui_start.elapsed());
                        if perf_export_request {
                            match crate::perf::export_snapshot(&self.perf_sampler) {
                                Ok(path) => {
                                    // Toast shows full path (user requested
                                    // the export, they want to find the
                                    // file). Log redacts so journald
                                    // doesn't leak HOME directories.
                                    tracing::info!(
                                        "Perf snapshot written: {}",
                                        crate::drop_validate::redact_path(&path)
                                    );
                                    tracing::debug!("Perf snapshot full path: {}", path.display());
                                    {
                                        let mut args = fluent::FluentArgs::new();
                                        args.set("path", path.display().to_string());
                                        self.toasts.success(crate::i18n::t_args(
                                            "toast-perf-snapshot",
                                            &args,
                                        ));
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Perf snapshot failed: {e}");
                                    {
                                        let mut args = fluent::FluentArgs::new();
                                        args.set("error", e.to_string());
                                        self.toasts.error(crate::i18n::t_args(
                                            "toast-perf-snapshot-failed",
                                            &args,
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    {
                        let _s = self.perf_sampler.scope(crate::perf::Category::Present);
                        renderer.present(output);
                    }
                    // Close the perf frame. The Idle bucket falls
                    // out implicitly: total - sum(other categories).
                    self.perf_sampler.end_frame();

                    // Apply UI outcomes AFTER ui.render so we can
                    // take &mut self.renderer / call other &mut self
                    // methods that conflict with the egui borrow.
                    if toggle_requested {
                        self.toggle_edit_mode();
                    }
                    if let Some(outcome) = menu_outcome {
                        self.handle_menu_outcome(outcome);
                    }
                    if let Some(outcome) = palette_outcome {
                        self.handle_palette_outcome(outcome);
                    }
                    if let Some(outcome) = library_outcome {
                        self.handle_library_outcome(outcome);
                    }
                    if let Some(path) = shimeji_import {
                        let expanded = crate::config::AppConfig::resolve_asset_path(&path);
                        self.import_shimeji_pack(&expanded);
                    }
                }
                // Surface needs reconfiguring against the current size
                // (resize race, occlusion, or the surface coming back
                // after suspend). Reconfigure and flag for the streak
                // tracker — persistent loss escalates to a full rebuild.
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    renderer.resize(
                        renderer.primary.window_width,
                        renderer.primary.window_height,
                    );
                    surface_lost = true;
                }
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    tracing::error!("GPU out of memory!");
                    event_loop.exit();
                }
                // Timeout: the swapchain image wasn't ready in time —
                // transient, just drop this frame and try the next.
                Err(wgpu::SurfaceError::Timeout) => {
                    tracing::debug!("surface acquire timed out; skipping frame");
                }
                Err(e) => {
                    tracing::warn!("Render error: {:?}", e);
                }
            }
        }

        // Surface-loss recovery, now that the `self.renderer` borrow is
        // released. A clean present resets the streak; a lost surface
        // either reconfigures (handled above) or, once persistent,
        // triggers a wholesale renderer rebuild.
        if surface_lost {
            self.recover_after_surface_loss(event_loop);
        } else if rendered_ok {
            self.surface_loss_streak = 0;
        }

        // Render the PerMonitor extras inside the same cycle (one
        // pacing domain for T.6 — per-window pacing is a recorded
        // follow-up in the architecture notes).
        self.render_extra_windows();

        // Schedule the next frame. Pre-0.5.5 this was an unconditional
        // request_redraw() — 60 wake-ups/s with the GPU re-rendering an
        // unchanged scene for an overlay that idles 24/7. Pacing keeps
        // the loop event-driven: continuous only while something
        // actually animates, deadline-based for playing sprites, and a
        // 2 s heartbeat (hot-reload poll) when fully static. Input
        // events re-trigger redraws from `window_event` / `user_event`.
        match self.redraw_pacing() {
            RedrawPacing::Continuous => {
                event_loop.set_control_flow(ControlFlow::Wait);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            RedrawPacing::Deadline(due) => {
                let heartbeat = Instant::now() + IDLE_HEARTBEAT;
                event_loop.set_control_flow(ControlFlow::WaitUntil(due.min(heartbeat)));
            }
            RedrawPacing::Idle => {
                event_loop
                    .set_control_flow(ControlFlow::WaitUntil(Instant::now() + IDLE_HEARTBEAT));
            }
        }
    }

    /// React to a primary surface that came back `Lost`/`Outdated`.
    ///
    /// The per-frame `surface.configure()` (done in the match arm before
    /// this is called) clears the common, transient cases — a resize
    /// race, occlusion, the first frame or two after S3 resume. This
    /// tracks how many frames in a row that *hasn't* worked: once the
    /// streak passes [`SURFACE_LOSS_REBUILD_THRESHOLD`] the surface is
    /// genuinely gone (driver reset, GPU hot-unplug, device lost across
    /// suspend) and only a fresh device recovers it, so we rebuild the
    /// whole renderer from the retained window — the same path startup
    /// takes. If even that fails the GPU is unusable; save state and exit
    /// cleanly so the session restarts us rather than spin forever.
    ///
    /// An immediate redraw is requested so the streak burns through in a
    /// handful of frames instead of waiting on the idle heartbeat.
    fn recover_after_surface_loss(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
        let (next_streak, rebuild) = next_surface_loss_state(self.surface_loss_streak);
        self.surface_loss_streak = next_streak;
        if !rebuild {
            return; // give surface.configure() a few frames to recover
        }
        tracing::warn!("primary surface lost persistently; rebuilding the renderer");
        let Some(window) = self.window.clone() else {
            tracing::error!("no window to rebuild the renderer against; exiting");
            event_loop.exit();
            return;
        };
        match WgpuRenderer::new(window) {
            Ok(renderer) => {
                self.renderer = Some(renderer);
                // The rebuilt device has an empty texture cache; force
                // every entity to re-upload on the next frame.
                for e in &mut self.scene.entities {
                    e.texture_dirty = true;
                }
                tracing::info!("renderer rebuilt after persistent surface loss");
            }
            Err(e) => {
                tracing::error!("renderer rebuild failed ({e}); exiting cleanly for a restart");
                event_loop.exit();
            }
        }
    }

    /// Decide how soon the next frame is needed, from the live state.
    ///
    /// Continuous wins whenever per-tick motion exists: edit mode
    /// (egui interactions, drags), visible toasts (slide/fade), the
    /// perf overlay (live graph), autonomous behaviors or physics on a
    /// visible entity. Otherwise the soonest animation deadline across
    /// visible playing sprites; otherwise idle. Hidden entities don't
    /// hold the loop awake — their behaviors freeze while invisible
    /// (the 0.1 s dt clamp in `Scene::tick` absorbs the gap when they
    /// come back).
    fn redraw_pacing(&self) -> RedrawPacing {
        if self.edit_mode || self.perf_overlay_visible || !self.toasts.is_empty() {
            return RedrawPacing::Continuous;
        }
        if !self.scene.global_playing {
            return RedrawPacing::Idle;
        }
        let mut deadline: Option<Instant> = None;
        for entity in self.scene.visible_entities() {
            if entity.physics.enabled || !matches!(entity.behavior, crate::behavior::Behavior::Idle)
            {
                return RedrawPacing::Continuous;
            }
            if entity.animation().playing && entity.animation().frame_count() > 1 {
                let due = entity.animation().next_frame_due();
                deadline = Some(deadline.map_or(due, |d| d.min(due)));
            }
        }
        match deadline {
            Some(due) => RedrawPacing::Deadline(due),
            None => RedrawPacing::Idle,
        }
    }
}

#[cfg(test)]
mod surface_loss_tests {
    use super::{next_surface_loss_state, SURFACE_LOSS_REBUILD_THRESHOLD};

    #[test]
    fn below_threshold_increments_without_rebuild() {
        let (next, rebuild) = next_surface_loss_state(0);
        assert_eq!((next, rebuild), (1, false));
        let (next, rebuild) = next_surface_loss_state(SURFACE_LOSS_REBUILD_THRESHOLD - 2);
        assert_eq!((next, rebuild), (SURFACE_LOSS_REBUILD_THRESHOLD - 1, false));
    }

    #[test]
    fn reaching_threshold_rebuilds_and_resets() {
        // One more loss from THRESHOLD-1 hits THRESHOLD → rebuild, streak
        // resets so a post-rebuild loss counts fresh (no per-frame thrash).
        let (next, rebuild) = next_surface_loss_state(SURFACE_LOSS_REBUILD_THRESHOLD - 1);
        assert!(rebuild);
        assert_eq!(next, 0);
    }

    #[test]
    fn saturates_without_overflow() {
        let (_, rebuild) = next_surface_loss_state(u32::MAX);
        assert!(
            rebuild,
            "a huge streak must still resolve to a rebuild, not panic"
        );
    }
}
