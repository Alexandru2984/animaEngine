mod dispatch;
mod hot_reload;
mod input;
mod lifecycle;
mod outcomes;
mod render_loop;
pub(crate) mod windows;

use crate::config::AppConfig;
use crate::constants::TOGGLE_BUTTON_SIZE;
use crate::event::AnimaEvent;
use crate::input::drag::DragController;
use crate::input::selection::SelectionState;
use crate::keybindings::{KeyChord, KeyCode, ModifierMask};
use crate::renderer::wgpu_renderer::WgpuRenderer;
use crate::scene::Scene;
use crate::ui::Warning;
use crate::ui::{EguiRenderer, ToastQueue};
use crate::window::overlay::OverlayPlatform;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Instant, SystemTime};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{Window, WindowId};

/// Main application state — implements winit's ApplicationHandler.
///
/// The overlay operates in two modes:
/// - **Pass-through mode** (default): clicks go through to the desktop,
///   except for a small toggle button in the top-right corner.
/// - **Edit mode** (click toggle button): the full overlay
///   receives input. You can drag characters, select them, use keyboard shortcuts.
pub struct App {
    /// The winit window (created on resume)
    window: Option<Arc<Window>>,
    /// GPU renderer
    renderer: Option<WgpuRenderer>,
    /// The scene with all entities
    scene: Scene,
    /// Application config
    config: AppConfig,
    /// Drag controller
    drag: DragController,
    /// Selection state
    selection: SelectionState,
    /// Current mouse position
    mouse_x: f32,
    mouse_y: f32,
    /// Whether config needs saving (dirty flag)
    config_dirty: bool,
    /// Whether the overlay is in "edit mode" (interactive) or "pass-through" mode
    /// Default: false (pass-through — clicks go to desktop)
    edit_mode: bool,
    /// Whether Shift key is currently held (for fine control)
    shift_held: bool,
    /// Whether Ctrl key is currently held (used for Ctrl-modified chords).
    ctrl_held: bool,
    /// Whether Alt is currently held — tracked so user-bound chords
    /// involving Alt resolve correctly through `KeyBindings::lookup`.
    alt_held: bool,
    /// Whether Super (Win/Cmd/Meta) is currently held — same reason.
    super_held: bool,
    /// Pooled X11 input manager (holds a single X11 connection)
    x11_input: Option<Box<dyn OverlayPlatform>>,
    /// Last time we checked config file for hot-reload
    last_config_check: Instant,
    /// Last known modification time of config file
    config_mtime: Option<SystemTime>,
    /// Receiver for an in-flight async hot-reload. `Some` means a worker
    /// thread is currently decoding the new config + assets off the UI thread.
    hot_reload_rx: Option<mpsc::Receiver<Result<HotReloadResult, String>>>,
    /// In-flight off-thread Shimeji import. `Some` while a worker copies a
    /// pack's sprites (the slow part) so a large pack can't freeze the UI;
    /// the result is applied on the UI thread at the captured drop point.
    pending_shimeji: Option<PendingShimejiImport>,
    /// egui integration. Paints in BOTH modes — the ⚙ toggle button is an
    /// egui widget that lives in pass-through too. Other UI (settings panel,
    /// context menu, toasts) is gated to edit mode inside the build closure.
    ui: Option<EguiRenderer>,
    /// Ephemeral UI state (currently just the context menu) kept separate
    /// from the egui renderer so it survives across resumed/suspended cycles.
    ui_state: UiState,
    /// Toast notification queue. Persistent across edit/pass-through
    /// transitions but only painted when in edit mode (no UI otherwise).
    toasts: ToastQueue,
    /// Session-lifetime warnings rendered as a banner at the top of
    /// the settings panel (D.5). Distinct from toasts: these persist
    /// until the underlying condition clears or the user dismisses
    /// the banner. Stored as `BTreeSet` so insertion is idempotent
    /// (the same warning fired twice doesn't duplicate the banner)
    /// and display order is deterministic.
    warnings: std::collections::BTreeSet<Warning>,
    /// Per-system frame-time + total sampler (D.6). Always populated;
    /// the overlay widget is what's actually toggled. Keeping the
    /// sampler always-on costs ~5 µs/frame which is below any
    /// perceivable noise, and lets the overlay show meaningful
    /// averages the moment it opens.
    perf_sampler: crate::perf::PerfSampler,
    /// Whether the perf overlay widget is currently visible. Toggled
    /// via `Action::TogglePerfOverlay` (`Ctrl+Shift+\`` by default).
    perf_overlay_visible: bool,
    /// Cached resident-set size (KiB) shown in the perf overlay. Updated
    /// every `RSS_REFRESH_FRAMES` frames so the proc-fs read doesn't
    /// land in the per-frame budget. `None` until the first read or on
    /// non-Linux platforms.
    perf_last_rss_kib: Option<u64>,
    /// Frame counter for the RSS refresh cadence.
    perf_frame_counter: u32,
    /// Soak-test metrics emitter (W.1). `None` unless
    /// `ANIMA_SOAK_METRICS` is set; ticked once per frame.
    soak: Option<crate::soak::SoakRecorder>,
    /// Previous frame's GPU texture uploads / draw calls, for the perf
    /// HUD (W.3). Captured-and-reset at each frame's start.
    gpu_uploads: u32,
    gpu_draws: u32,
    /// Consecutive frames the primary surface has come back `Lost` /
    /// `Outdated`. Reset on any successful present. A short streak is a
    /// transient blip the per-frame `surface.configure()` clears (resize,
    /// occlusion, the first frame or two after S3 resume); a streak past
    /// `render_loop::SURFACE_LOSS_REBUILD_THRESHOLD` means the surface
    /// reconfigure isn't recovering it (driver reset, GPU hot-unplug,
    /// device lost
    /// across suspend) and we rebuild the renderer wholesale.
    surface_loss_streak: u32,
    /// Snapshot of the monitor topology taken on the first `resumed()`
    /// — empty until then. Used by the picker UI (C.2) and the
    /// per-monitor render path (C.3); the data layer (this commit /
    /// C.1) only populates and logs it.
    monitors: Vec<crate::monitor::MonitorInfo>,
    /// EWMH window watcher for window-awareness. `None` until first
    /// use or when no X server exists; `watcher_probe_done` stops us
    /// re-attempting the connection every poll on Wayland. X11-only by
    /// nature — off unix the physics fall back to the screen floor, the
    /// same as a native Wayland session does today.
    #[cfg(unix)]
    window_watcher: Option<crate::window::x11_windows::WindowWatcher>,
    #[cfg(unix)]
    window_watcher_probe_done: bool,
    /// Last desktop-window poll — 300 ms cadence, see
    /// `poll_window_platforms`.
    last_window_poll: Instant,
    /// Whether the previous poll pushed a non-empty platform set into
    /// the scene — lets the disable path clear it exactly once.
    window_platforms_active: bool,
    /// Asset library index. `None` when no asset root was discovered
    /// at startup (env var unset, XDG dir missing, no exe-relative
    /// fallback). The UI shows an empty state in that case rather
    /// than failing.
    library: Option<crate::asset_library::LibraryIndex>,
    /// Asset root path used at startup. Kept so the "Add to scene"
    /// path can resolve relative asset paths to absolute without
    /// re-scanning.
    library_root: Option<std::path::PathBuf>,
    /// Human-readable description of the live global-hotkey backend,
    /// shown in the Keybindings tab (T.4). Set from `main` after
    /// strategy resolution; updated when the deferred portal fallback
    /// fires.
    hotkey_backend_status: String,
    /// Extra overlay windows for `MonitorMode::PerMonitor` (T.6) —
    /// one per non-primary monitor, sprite-only (egui stays on the
    /// primary). Keyed by `WindowId` for event routing.
    extra_windows: std::collections::HashMap<WindowId, windows::WindowSlot>,
    /// Mode snapshot from the last window (re)build — the redraw
    /// handler compares it against config to catch Appearance-tab
    /// switches.
    last_monitor_mode: crate::monitor::MonitorMode,
    /// Last hotplug poll — winit has no monitor-change event on X11,
    /// so the redraw cycle re-enumerates on this cadence (T.9).
    last_monitor_check: Instant,
    /// Last time we re-applied the click-through input shape as a
    /// self-heal. GNOME/XWayland Mutter resets the XShape region behind
    /// our back when it restacks the overlay for always-on-top, which
    /// can leave pass-through mode swallowing every click. Re-applying
    /// on a slow cadence guarantees click-through recovers within this
    /// window no matter what the compositor did. Pass-through only.
    last_shape_refresh: Instant,
}

/// Result of an async hot-reload — produced by a worker thread, consumed by
/// the UI thread on the next frame.
struct HotReloadResult {
    config: AppConfig,
    scene: Scene,
}

/// An off-thread Shimeji import in flight: the worker's result channel and
/// the drop position to spawn the imported characters at when it lands.
struct PendingShimejiImport {
    rx: mpsc::Receiver<std::result::Result<crate::shimeji::ImportReport, String>>,
    x: f32,
    y: f32,
}

/// Transient UI state owned by `App` (vs the persistent settings panel
/// which is stateless and rebuilt from `Scene` every frame).
#[derive(Default)]
pub(crate) struct UiState {
    pub context_menu: Option<ContextMenuState>,
}

#[derive(Clone)]
pub(crate) struct ContextMenuState {
    pub entity_idx: usize,
    /// Screen-space anchor for the floating menu.
    pub pos: egui::Pos2,
}

impl App {
    pub fn new(config: AppConfig, scene: Scene) -> Self {
        let last_monitor_mode = config.global.monitor_mode.clone();
        Self {
            window: None,
            renderer: None,
            scene,
            config,
            drag: DragController::new(),
            selection: SelectionState::new(),
            mouse_x: 0.0,
            mouse_y: 0.0,
            config_dirty: false,
            edit_mode: false,
            shift_held: false,
            ctrl_held: false,
            alt_held: false,
            super_held: false,
            x11_input: None,
            last_config_check: Instant::now(),
            config_mtime: Self::get_config_mtime(),
            hot_reload_rx: None,
            pending_shimeji: None,
            ui: None,
            ui_state: UiState::default(),
            toasts: ToastQueue::default(),
            warnings: std::collections::BTreeSet::new(),
            perf_sampler: crate::perf::PerfSampler::default(),
            perf_overlay_visible: false,
            perf_last_rss_kib: None,
            perf_frame_counter: 0,
            soak: crate::soak::SoakRecorder::from_env(),
            gpu_uploads: 0,
            gpu_draws: 0,
            surface_loss_streak: 0,
            monitors: Vec::new(),
            #[cfg(unix)]
            window_watcher: None,
            #[cfg(unix)]
            window_watcher_probe_done: false,
            last_window_poll: Instant::now(),
            window_platforms_active: false,
            library: None,
            library_root: None,
            hotkey_backend_status: String::new(),
            extra_windows: std::collections::HashMap::new(),
            last_monitor_mode,
            last_monitor_check: Instant::now(),
            last_shape_refresh: Instant::now(),
        }
    }

    /// Record which global-hotkey backend won resolution — purely
    /// informational, rendered in the Keybindings tab.
    pub fn set_hotkey_backend_status(&mut self, status: String) {
        self.hotkey_backend_status = status;
    }

    /// Mark a session-lifetime warning. Idempotent — setting the same
    /// variant twice does not duplicate the banner. Called by
    /// `main.rs` for startup-time conditions (global hotkeys
    /// unavailable) and from inside `App` for runtime conditions
    /// (hot-reload worker disconnected).
    pub fn push_warning(&mut self, w: Warning) {
        self.warnings.insert(w);
    }

    /// Clear a warning — used when the underlying condition resolves
    /// (e.g. the next hot-reload succeeds after a previous failure).
    #[allow(dead_code)]
    pub fn clear_warning(&mut self, w: Warning) {
        self.warnings.remove(&w);
    }

    /// Snapshot the current modifier-key state into the bitmask shape
    /// `KeyBindings::lookup` expects. Drains the four tracked booleans
    /// into one `ModifierMask` per call site so the chord build is
    /// allocation-free.
    fn modifier_mask(&self) -> ModifierMask {
        ModifierMask::from_state(
            self.ctrl_held,
            self.shift_held,
            self.alt_held,
            self.super_held,
        )
    }

    // `dispatch_action` lives in `src/app/dispatch.rs` (H.1) — same
    // `impl App` block, split across files so this module stays
    // focused on lifecycle / event-loop wiring.

    // Hot-reload (`get_config_mtime`, `check_hot_reload`,
    // `apply_hot_reload`) lives in `src/app/hot_reload.rs` (H.3).

    // Outcome handlers (`handle_{menu,library,palette}_outcome` +
    // `apply_menu_action`) live in `src/app/outcomes.rs` (H.2).

    /// The single shutdown path: persist any pending edits, then tear
    /// down GPU resources in the required order and stop the loop.
    ///
    /// Every exit routes through here — including the abnormal ones (GPU
    /// out of memory, a renderer rebuild that failed). Those used to call
    /// `event_loop.exit()` directly, which silently discarded whatever
    /// the user had changed since the last save; losing the GPU is not a
    /// reason to also lose their scene.
    pub(super) fn save_and_exit(&mut self, event_loop: &ActiveEventLoop) {
        self.save_config_if_needed();
        // Order matters: egui owns wgpu resources, drop it before the
        // renderer to avoid use-after-free during Vulkan cleanup.
        self.ui = None;
        self.renderer = None;
        self.x11_input = None;
        event_loop.exit();
    }

    /// Save config if dirty
    fn save_config_if_needed(&mut self) {
        if self.config_dirty {
            self.config.characters = self.scene.to_character_configs();
            self.config.global.playback_enabled = self.scene.global_playing;
            match self.config.save() {
                Ok(()) => {
                    self.toasts.success(crate::i18n::t("toast-config-saved"));
                    // Mirror the clean state into the crash-recovery slot
                    // so the panic hook has something useful to dump.
                    crate::crash::record_known_good(&self.config);
                    // Clear the dirty flag only on a successful write. A
                    // transient failure (disk full, permissions) must keep
                    // the scene dirty so the next save-triggering edit
                    // retries instead of silently dropping the change.
                    // Saves fire on discrete edits, not per frame, so a
                    // persistent failure re-toasts per edit, not per frame.
                    self.config_dirty = false;
                    // Update mtime so hot-reload doesn't trigger on our own save
                    self.config_mtime = Self::get_config_mtime();
                }
                Err(e) => {
                    tracing::warn!("Failed to save config: {}", e);
                    let mut args = fluent::FluentArgs::new();
                    args.set("error", e.to_string());
                    self.toasts
                        .error(crate::i18n::t_args("toast-save-failed", &args));
                }
            }
        }
    }

    /// Push the X11 input shape that matches the current `edit_mode`.
    ///
    /// Must be called any time the shape can desync from reality: mode
    /// toggle, window resize, regaining focus or visibility (compositors
    /// like Mutter occasionally clip the shape after fractional-scaling
    /// transitions or after the window is minimized and restored).
    fn reapply_input_shape(&mut self) {
        if let Some(x11) = &mut self.x11_input {
            let result = if self.edit_mode {
                x11.set_full_input()
            } else {
                x11.set_passthrough_with_button(TOGGLE_BUTTON_SIZE)
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
    fn reassert_always_on_top(&self) {
        if let Some(x11) = &self.x11_input {
            if let Err(e) = x11.reassert_above() {
                tracing::debug!("Re-assert always-on-top failed: {e}");
            }
        }
    }

    /// Toggle between edit mode and pass-through mode
    fn toggle_edit_mode(&mut self) {
        self.edit_mode = !self.edit_mode;
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
            // End any active drag when leaving edit mode
            if self.drag.is_dragging() {
                self.drag.end_drag();
                self.config_dirty = true;
            }
            self.selection.deselect();
            self.ui_state.context_menu = None;

            // Auto-save any pending changes when exiting edit mode
            if self.config_dirty {
                self.save_config_if_needed();
            }
        }
    }
}

impl App {
    /// Ask winit for a frame. Cheap and idempotent (winit coalesces);
    /// called from every input/user-event path so the paced render
    /// loop wakes up when state changed outside `RedrawRequested`.
    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl ApplicationHandler<AnimaEvent> for App {
    /// Timer wake-ups from the paced render loop (`ControlFlow::WaitUntil`).
    ///
    /// Re-arms the heartbeat *before* requesting the redraw so the
    /// chain survives even when the compositor suppresses redraw
    /// delivery for a hidden window; `check_hot_reload` runs here
    /// directly for the same reason (config edits must apply while
    /// the overlay is hidden). Both are cheap: the hot-reload check
    /// self-gates on a 2 s mtime poll.
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if matches!(cause, StartCause::ResumeTimeReached { .. }) {
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + render_loop::IDLE_HEARTBEAT,
            ));
            self.check_hot_reload();
            self.check_shimeji_import();
            self.request_redraw();
        }
    }

    /// Handle tray / global-hotkey commands routed through the event loop.
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AnimaEvent) {
        match event {
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
            AnimaEvent::ShowOverlay => {
                if let Some(window) = &self.window {
                    window.set_visible(true);
                    // Compositors sometimes clip our shape on unmap/map.
                    self.reapply_input_shape();
                }
            }
            AnimaEvent::HideOverlay => {
                if let Some(window) = &self.window {
                    window.set_visible(false);
                }
            }
            AnimaEvent::RaiseWindow => {
                // Someone launched a second instance. Make sure we're
                // visible and ask the WM to focus us. EWMH ABOVE keeps us
                // on top regardless; this is just a nudge.
                if let Some(window) = &self.window {
                    window.set_visible(true);
                    window.focus_window();
                    self.reapply_input_shape();
                }
                tracing::info!("Raised by second-instance handshake");
            }
            AnimaEvent::Quit => {
                tracing::info!("Quit requested from tray");
                self.save_and_exit(event_loop);
                return;
            }
            AnimaEvent::HotkeysUnavailable => {
                // Deferred hotkey resolution (portal handshake +
                // fallbacks on a background thread) ended with no
                // working backend.
                self.push_warning(Warning::GlobalHotkeysUnavailable);
                self.hotkey_backend_status = "none (tray + D-Bus methods only)".into();
            }
            AnimaEvent::PortalShortcutsDenied => {
                self.toasts
                    .warn(crate::i18n::t("portal-denied-x11-fallback-toast"));
                self.hotkey_backend_status = "X11 XGrabKey (portal declined)".into();
            }
        }
        // Every non-quit tray/hotkey action mutates visible state
        // (mode, playback, visibility) — wake the paced render loop.
        self.request_redraw();
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Init body extracted to src/app/lifecycle.rs (H.5).
        self.handle_resumed(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        // Events from extra (PerMonitor) windows get a narrow handler:
        // sprites repaint on expose, surfaces resize, shapes re-apply.
        // Pointer/keyboard routing for extras (T.8, below) only
        // matters in edit mode — pass-through extras are fully
        // click-through, so the compositor never delivers them input.
        let is_primary = self.window.as_ref().is_some_and(|w| w.id() == window_id);
        if !is_primary {
            match event {
                WindowEvent::RedrawRequested => self.render_one_extra(window_id),
                WindowEvent::Resized(size) => {
                    if let Some(renderer) = &self.renderer {
                        if let Some(slot) = self.extra_windows.get_mut(&window_id) {
                            slot.surface
                                .resize(&renderer.shared, size.width, size.height);
                        }
                    }
                }
                WindowEvent::CloseRequested => {
                    self.extra_windows.remove(&window_id);
                }
                WindowEvent::Focused(true) | WindowEvent::Occluded(false) => {
                    self.reapply_extra_input_shapes();
                }
                // T.8 — input from extra windows, translated to global
                // desktop coordinates by the window's monitor origin.
                // Only reachable in edit mode (pass-through shape on
                // extras is fully click-through).
                WindowEvent::CursorMoved { position, .. } => {
                    if let Some(slot) = self.extra_windows.get(&window_id) {
                        let (ox, oy) = (slot.monitor.x as f32, slot.monitor.y as f32);
                        self.handle_cursor_moved_global(
                            position.x as f32 + ox,
                            position.y as f32 + oy,
                        );
                        self.request_redraw_all();
                    }
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    self.handle_mouse_input(state, button);
                    self.request_redraw_all();
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    self.handle_mouse_wheel(delta);
                    self.request_redraw_all();
                }
                WindowEvent::ModifiersChanged(modifiers) => {
                    self.handle_modifiers_changed(modifiers);
                }
                WindowEvent::KeyboardInput {
                    event:
                        winit::event::KeyEvent {
                            state: ElementState::Pressed,
                            ref logical_key,
                            ..
                        },
                    ..
                } if self.edit_mode => {
                    if let Some(keycode) = KeyCode::from_winit(logical_key.as_ref()) {
                        let chord = KeyChord::new(self.modifier_mask(), keycode);
                        if let Some(action) = self.config.keybindings.lookup(chord) {
                            self.dispatch_action(action, event_loop);
                            self.request_redraw_all();
                        }
                    }
                }
                _ => {}
            }
            return;
        }

        // Let egui peek at the event first. The toggle ⚙ button is an egui
        // widget that lives in BOTH modes, so we always forward; a consumed
        // event short-circuits our own handlers (so e.g. clicking the button
        // doesn't also try to drag an entity).
        if let (Some(ui), Some(window)) = (self.ui.as_mut(), self.window.as_ref()) {
            if ui.handle_event(window, &event) {
                // egui reacted (hover state, button press…) — make sure
                // a frame shows it even when the paced loop is asleep.
                window.request_redraw();
                return;
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                tracing::info!("Close requested — saving config and exiting");
                self.save_and_exit(event_loop);
            }

            WindowEvent::Resized(physical_size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(physical_size.width, physical_size.height);
                }
                // The input shape mask is sized to the old window dimensions;
                // re-apply for the new size in whatever mode we're in.
                self.reapply_input_shape();
                self.request_redraw();
            }

            // Every focus / occlusion transition. Mutter (GNOME on
            // XWayland) both sinks our always-on-top *and* can reset the
            // XShape click-through region when it processes the resulting
            // window-state change. So re-assert ABOVE first, then re-apply
            // the input shape **last** — order is load-bearing: doing the
            // shape first (or not at all on focus loss, the earlier bug)
            // let the ABOVE re-assert clobber it, leaving the overlay
            // swallowing every click on Arch's newer Mutter (the user
            // couldn't reach windows under the sprites). Shape-last keeps
            // click-through and stay-on-top from desyncing.
            WindowEvent::Focused(_) | WindowEvent::Occluded(_) => {
                self.reassert_always_on_top();
                self.reapply_input_shape();
                self.request_redraw();
            }

            WindowEvent::RedrawRequested => {
                // The whole render pipeline lives in
                // `src/app/render_loop.rs` (H.4a) so this match arm
                // stays a one-line delegate.
                self.handle_redraw_requested(event_loop);
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.handle_cursor_moved(position);
                self.request_redraw();
            }

            WindowEvent::MouseInput { state, button, .. } => {
                self.handle_mouse_input(state, button);
                self.request_redraw();
            }

            // Edit-mode keyboard dispatch goes through the rebindable
            // `KeyBindings::lookup` table. Conversion failures (function
            // keys, IME, etc.) and unbound chords are silent no-ops —
            // every other path stays inside `dispatch_action`.
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        state: ElementState::Pressed,
                        ref logical_key,
                        ..
                    },
                ..
            } if self.edit_mode => {
                if let Some(keycode) = KeyCode::from_winit(logical_key.as_ref()) {
                    let chord = KeyChord::new(self.modifier_mask(), keycode);
                    if let Some(action) = self.config.keybindings.lookup(chord) {
                        self.dispatch_action(action, event_loop);
                        self.request_redraw();
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                self.handle_mouse_wheel(delta);
                self.request_redraw();
            }

            WindowEvent::DroppedFile(path) => {
                self.handle_dropped_file(path);
                self.request_redraw();
            }

            WindowEvent::HoveredFile(path) => {
                self.handle_hovered_file(path);
                self.request_redraw();
            }

            WindowEvent::ModifiersChanged(modifiers) => {
                self.handle_modifiers_changed(modifiers);
            }

            _ => {}
        }
    }
}
