mod dispatch;
mod hot_reload;
mod input;
mod outcomes;
mod render_loop;

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
use crate::window::x11_input::X11InputManager;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Instant, SystemTime};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId, WindowLevel};

// X11-specific: set window type. We use Normal (NOT Dock) because DOCK windows
// on XWayland/Mutter don't receive mouse events. EWMH hints handle always-on-top.
#[cfg(target_os = "linux")]
use winit::platform::x11::{WindowAttributesExtX11, WindowType};

// Wayland-specific: set app_id so the compositor (Mutter, KWin, sway) maps
// the window to the .desktop / launcher entry. Must match StartupWMClass
// in data/com.animaengine.Anima.desktop — otherwise the dock picks up the
// window as a separate, no-icon entry alongside the pinned one.
#[cfg(target_os = "linux")]
use winit::platform::wayland::WindowAttributesExtWayland;

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
    x11_input: Option<X11InputManager>,
    /// Last time we checked config file for hot-reload
    last_config_check: Instant,
    /// Last known modification time of config file
    config_mtime: Option<SystemTime>,
    /// Receiver for an in-flight async hot-reload. `Some` means a worker
    /// thread is currently decoding the new config + assets off the UI thread.
    hot_reload_rx: Option<mpsc::Receiver<HotReloadResult>>,
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
    /// Snapshot of the monitor topology taken on the first `resumed()`
    /// — empty until then. Used by the picker UI (C.2) and the
    /// per-monitor render path (C.3); the data layer (this commit /
    /// C.1) only populates and logs it.
    monitors: Vec<crate::monitor::MonitorInfo>,
    /// Asset library index. `None` when no asset root was discovered
    /// at startup (env var unset, XDG dir missing, no exe-relative
    /// fallback). The UI shows an empty state in that case rather
    /// than failing.
    library: Option<crate::asset_library::LibraryIndex>,
    /// Asset root path used at startup. Kept so the "Add to scene"
    /// path can resolve relative asset paths to absolute without
    /// re-scanning.
    library_root: Option<std::path::PathBuf>,
}

/// Result of an async hot-reload — produced by a worker thread, consumed by
/// the UI thread on the next frame.
struct HotReloadResult {
    config: AppConfig,
    scene: Scene,
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
            ui: None,
            ui_state: UiState::default(),
            toasts: ToastQueue::default(),
            warnings: std::collections::BTreeSet::new(),
            perf_sampler: crate::perf::PerfSampler::default(),
            perf_overlay_visible: false,
            perf_last_rss_kib: None,
            perf_frame_counter: 0,
            monitors: Vec::new(),
            library: None,
            library_root: None,
        }
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

    /// Save config if dirty
    fn save_config_if_needed(&mut self) {
        if self.config_dirty {
            self.config.characters = self.scene.to_character_configs();
            self.config.global.playback_enabled = self.scene.global_playing;
            match self.config.save() {
                Ok(()) => {
                    self.toasts.success("Config saved");
                    // Mirror the clean state into the crash-recovery slot
                    // so the panic hook has something useful to dump.
                    crate::crash::record_known_good(&self.config);
                }
                Err(e) => {
                    tracing::warn!("Failed to save config: {}", e);
                    self.toasts.error(format!("Save failed: {e}"));
                }
            }
            self.config_dirty = false;
            // Update mtime so hot-reload doesn't trigger on our own save
            self.config_mtime = Self::get_config_mtime();
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

    /// Toggle between edit mode and pass-through mode
    fn toggle_edit_mode(&mut self) {
        self.edit_mode = !self.edit_mode;
        self.reapply_input_shape();

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

impl ApplicationHandler<AnimaEvent> for App {
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
                    "Playback resumed"
                } else {
                    "Playback paused"
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
                self.save_config_if_needed();
                self.ui = None;
                self.renderer = None;
                self.x11_input = None;
                event_loop.exit();
            }
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return; // Already created
        }

        tracing::info!("Creating window...");

        // Snapshot the monitor topology once so the rest of the engine
        // can use the renderer-agnostic MonitorInfo instead of holding
        // a borrow on the event loop. The picker UI in C.2 will read
        // this list; for now we log it and keep the data ready.
        let monitors: Vec<crate::monitor::MonitorInfo> = {
            let primary = event_loop.primary_monitor();
            event_loop
                .available_monitors()
                .map(|m| {
                    let size = m.size();
                    let pos = m.position();
                    let is_primary = primary
                        .as_ref()
                        .is_some_and(|p| p.name() == m.name() && p.size() == size);
                    let name = m
                        .name()
                        .unwrap_or_else(|| format!("Display {}", self.monitors.len()));
                    crate::monitor::MonitorInfo {
                        name,
                        x: pos.x,
                        y: pos.y,
                        width: size.width,
                        height: size.height,
                        scale_factor: m.scale_factor(),
                        is_primary,
                    }
                })
                .collect()
        };
        crate::monitor::log_topology(&monitors);
        self.monitors = monitors;

        // Discover + load + merge-scan the asset library. Errors are
        // logged but never fatal — an empty library is fine.
        if let Some(root) = crate::asset_library::discover_asset_root() {
            let index_path = crate::asset_library::LibraryIndex::default_path();
            let mut idx = crate::asset_library::LibraryIndex::load(&index_path);
            let scanned = crate::asset_library::scan(&root);
            let scanned_count = scanned.len();
            idx.merge_scan(scanned);
            if let Err(e) = idx.save(&index_path) {
                tracing::warn!("Failed to persist library.toml: {e}");
            }
            tracing::info!(
                "Asset library at {}: {} indexed ({} from this scan)",
                root.display(),
                idx.assets.len(),
                scanned_count,
            );
            self.library = Some(idx);
            self.library_root = Some(root);
        } else {
            tracing::info!("No asset library root found; Library tab will show empty state.");
        }

        // Auto-detect screen resolution if config values are 0
        let (win_w, win_h) = if self.config.global.window_width == 0
            || self.config.global.window_height == 0
        {
            if let Some(monitor) = event_loop
                .primary_monitor()
                .or_else(|| event_loop.available_monitors().next())
            {
                let size = monitor.size();
                tracing::info!(
                    "Auto-detected monitor resolution: {}x{}",
                    size.width,
                    size.height
                );
                (size.width, size.height)
            } else {
                tracing::warn!("Could not detect monitor resolution, falling back to 1920x1080");
                (1920u32, 1080u32)
            }
        } else {
            (
                self.config.global.window_width,
                self.config.global.window_height,
            )
        };

        // Build window attributes: transparent, borderless, always-on-top
        let window_attrs = Window::default_attributes()
            .with_title("animaEngine")
            .with_transparent(true)
            .with_decorations(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_inner_size(winit::dpi::LogicalSize::new(win_w, win_h));

        // X11-specific: Use Normal type (NOT Dock).
        // Dock windows on XWayland/Mutter don't receive mouse events.
        // EWMH hints (ABOVE, SKIP_TASKBAR, etc.) are applied by X11InputManager.
        //
        // Plus: set X11 WM_CLASS *and* Wayland app_id to "animaEngine" so
        // the launcher entry (StartupWMClass=animaEngine in the .desktop)
        // matches the runtime window — without this, the dock shows a
        // separate "running" entry next to the pinned one with a
        // placeholder icon. Both APIs accept the same identifier; we
        // pass it identically to keep X11 and Wayland behaviour aligned.
        #[cfg(target_os = "linux")]
        let window_attrs = {
            // Disambiguated explicitly because both X11 and Wayland
            // extension traits provide a `with_name`-style API; using
            // UFCS keeps the intent clear and stops rustc from picking
            // the wrong one.
            let attrs = WindowAttributesExtX11::with_name(
                window_attrs.with_x11_window_type(vec![WindowType::Normal]),
                "animaEngine",
                "animaEngine",
            );
            WindowAttributesExtWayland::with_name(attrs, "animaEngine", "animaEngine")
        };

        match event_loop.create_window(window_attrs) {
            Ok(window) => {
                let window = Arc::new(window);
                tracing::info!(
                    "Window created: {:?} ({}x{})",
                    window.id(),
                    window.inner_size().width,
                    window.inner_size().height
                );

                // Create pooled X11 input manager (single connection)
                let mut x11_mgr = X11InputManager::new(&window);
                if let Some(ref mut mgr) = x11_mgr {
                    // Set initial input shape: click-through except toggle button
                    if let Err(e) = mgr.set_passthrough_with_button(TOGGLE_BUTTON_SIZE) {
                        tracing::warn!("Failed to set initial input shape: {}", e);
                        let _ = window.set_cursor_hittest(false);
                    }
                } else {
                    tracing::warn!(
                        "X11InputManager not available. Falling back to set_cursor_hittest."
                    );
                    let _ = window.set_cursor_hittest(false);
                }
                self.x11_input = x11_mgr;

                // Initialize wgpu renderer
                match WgpuRenderer::new(window.clone()) {
                    Ok(mut renderer) => {
                        // Create initial textures for all entities
                        for entity in &self.scene.entities {
                            renderer.ensure_texture(entity);
                        }
                        // Clear texture_dirty flags after initial upload
                        for entity in &mut self.scene.entities {
                            entity.texture_dirty = false;
                        }

                        // egui wraps the existing wgpu device + queue.
                        let ui = EguiRenderer::new(
                            &renderer.device,
                            renderer.config.format,
                            window.clone(),
                            self.config.global.theme,
                        );
                        self.ui = Some(ui);
                        self.renderer = Some(renderer);
                        tracing::info!("wgpu + egui renderers initialized");
                    }
                    Err(e) => {
                        tracing::error!("Failed to initialize wgpu renderer: {}", e);
                        event_loop.exit();
                        return;
                    }
                }

                self.window = Some(window);
            }
            Err(e) => {
                tracing::error!("Failed to create window: {}", e);
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // Let egui peek at the event first. The toggle ⚙ button is an egui
        // widget that lives in BOTH modes, so we always forward; a consumed
        // event short-circuits our own handlers (so e.g. clicking the button
        // doesn't also try to drag an entity).
        if let (Some(ui), Some(window)) = (self.ui.as_mut(), self.window.as_ref()) {
            if ui.handle_event(window, &event) {
                return;
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                tracing::info!("Close requested — saving config and exiting");
                self.save_config_if_needed();
                // Order matters: egui owns wgpu resources, drop it before
                // the renderer to avoid use-after-free during Vulkan cleanup.
                self.ui = None;
                self.renderer = None;
                self.x11_input = None;
                event_loop.exit();
            }

            WindowEvent::Resized(physical_size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(physical_size.width, physical_size.height);
                }
                // The input shape mask is sized to the old window dimensions;
                // re-apply for the new size in whatever mode we're in.
                self.reapply_input_shape();
            }

            // Re-apply input shape when the window regains focus.
            // Some compositors (notably Mutter with fractional scaling) clip
            // the input shape after a focus loss → restore cycle.
            WindowEvent::Focused(true) => {
                self.reapply_input_shape();
            }

            // Re-apply input shape when the window becomes visible again
            // after being occluded (e.g. user pressed Super+H, then restored).
            WindowEvent::Occluded(false) => {
                self.reapply_input_shape();
            }

            WindowEvent::RedrawRequested => {
                // The whole render pipeline lives in
                // `src/app/render_loop.rs` (H.4a) so this match arm
                // stays a one-line delegate.
                self.handle_redraw_requested(event_loop);
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.handle_cursor_moved(position);
            }

            WindowEvent::MouseInput { state, button, .. } => {
                self.handle_mouse_input(state, button);
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
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                self.handle_mouse_wheel(delta);
            }

            WindowEvent::DroppedFile(path) => {
                self.handle_dropped_file(path);
            }

            WindowEvent::HoveredFile(path) => {
                self.handle_hovered_file(path);
            }

            WindowEvent::ModifiersChanged(modifiers) => {
                self.handle_modifiers_changed(modifiers);
            }

            _ => {}
        }
    }
}
