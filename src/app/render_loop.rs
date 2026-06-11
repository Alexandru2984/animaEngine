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
        if self.perf_frame_counter % RSS_REFRESH_FRAMES == 0 {
            self.perf_last_rss_kib = crate::perf::read_rss_kib();
        }
        self.perf_frame_counter = self.perf_frame_counter.wrapping_add(1);

        // Check for external config changes (hot-reload)
        self.check_hot_reload();

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
        // mode XShape blocks CursorMoved outside the toggle button,
        // so the position is stale — accepted trade-off.
        let cursor = Some((self.mouse_x, self.mouse_y));
        {
            let _s = self.perf_sampler.scope(crate::perf::Category::SceneUpdate);
            self.scene.tick(screen_w, screen_h, cursor);
        }

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
                renderer.render(&visible, self.edit_mode, selected_id)
            };
            match render_result {
                Ok(output) => {
                    // egui runs in BOTH modes. In pass-through it
                    // paints just the toggle ⚙ button; in edit mode it
                    // adds the settings panel, context menu, toasts.
                    self.toasts.prune();

                    let mut menu_outcome: Option<panels::ContextMenuOutcome> = None;
                    let mut palette_outcome: Option<panels::PaletteOutcome> = None;
                    let mut library_outcome: Option<panels::LibraryOutcome> = None;
                    let mut toggle_requested = false;

                    if let (Some(ui), Some(window)) = (self.ui.as_mut(), self.window.as_ref()) {
                        // Pick up theme changes made in the settings
                        // panel before any panel paints this frame.
                        ui.ensure_theme(self.config.global.theme);

                        let view = output
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default());
                        let size = [renderer.window_width, renderer.window_height];

                        // Disjoint mutable borrows on disjoint fields.
                        let scene_mut = &mut self.scene;
                        let selection_mut = &mut self.selection;
                        let config_dirty_mut = &mut self.config_dirty;
                        let theme_mut = &mut self.config.global.theme;
                        let locale_mut = &mut self.config.global.locale;
                        let onboarding_mut = &mut self.config.global.onboarding;
                        let monitor_mode_mut = &mut self.config.global.monitor_mode;
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
                            &renderer.device,
                            &renderer.queue,
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
                                // Toggle button is the only UI in
                                // pass-through; in edit mode it sits
                                // on top of everything else.
                                if panels::toggle_button(ctx, edit_mode) {
                                    *toggle_requested_ref = true;
                                }

                                if edit_mode {
                                    panels::settings(
                                        ctx,
                                        scene_mut,
                                        selection_mut,
                                        config_dirty_mut,
                                        theme_mut,
                                        locale_mut,
                                        onboarding_mut,
                                        monitor_mode_mut,
                                        monitors_ref,
                                        library_ref,
                                        library_outcome_ref,
                                        keybindings_mut,
                                        collapse_state_mut,
                                        accesskit_mut,
                                        warnings_ref,
                                        last_seen_whats_new_mut,
                                        hotkey_backend_ref,
                                    );
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
                                    self.toasts
                                        .success(format!("Perf snapshot: {}", path.display()));
                                }
                                Err(e) => {
                                    tracing::error!("Perf snapshot failed: {e}");
                                    self.toasts.error(format!("Snapshot failed: {e}"));
                                }
                            }
                        }
                    }
                    {
                        let _s = self.perf_sampler.scope(crate::perf::Category::Present);
                        output.present();
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
                }
                Err(wgpu::SurfaceError::Lost) => {
                    renderer.resize(renderer.window_width, renderer.window_height);
                }
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    tracing::error!("GPU out of memory!");
                    event_loop.exit();
                }
                Err(e) => {
                    tracing::warn!("Render error: {:?}", e);
                }
            }
        }

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
            if entity.animation.playing && entity.animation.frame_count() > 1 {
                let due = entity.animation.next_frame_due();
                deadline = Some(deadline.map_or(due, |d| d.min(due)));
            }
        }
        match deadline {
            Some(due) => RedrawPacing::Deadline(due),
            None => RedrawPacing::Idle,
        }
    }
}
