mod dispatch;

use crate::config::AppConfig;
use crate::constants::TOGGLE_BUTTON_SIZE;
use crate::drop_validate::{pre_validate_dropped_file, redact_path, resolve_library_asset};
use crate::event::AnimaEvent;
use crate::input::drag::DragController;
use crate::input::selection::SelectionState;
use crate::keybindings::{KeyChord, KeyCode, ModifierMask};
use crate::renderer::wgpu_renderer::WgpuRenderer;
use crate::scene::Scene;
use crate::ui::Warning;
use crate::ui::{panels, EguiRenderer, ToastQueue};
use crate::window::x11_input::X11InputManager;
use std::collections::HashSet;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Instant, SystemTime};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
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

    /// Get the modification time of the config file
    fn get_config_mtime() -> Option<SystemTime> {
        let path = AppConfig::config_path();
        std::fs::metadata(&path).ok()?.modified().ok()
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

    /// Drive the hot-reload pipeline:
    /// 1. Apply any result already produced by a worker (non-blocking).
    /// 2. If the config file changed on disk, spawn a worker to load it.
    ///
    /// The UI thread never blocks on asset decoding — even for users with
    /// dozens of GIFs the reload happens off-thread.
    #[tracing::instrument(skip(self))]
    fn check_hot_reload(&mut self) {
        // Phase 1: drain a finished worker, if any.
        if let Some(rx) = &self.hot_reload_rx {
            match rx.try_recv() {
                Ok(result) => {
                    self.apply_hot_reload(result);
                    self.hot_reload_rx = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    tracing::warn!("Hot-reload worker disconnected unexpectedly");
                    self.hot_reload_rx = None;
                    // Surface the silent crash to the user — without
                    // this banner the in-flight edit would just not
                    // apply and they'd assume the file save took.
                    self.warnings.insert(Warning::HotReloadDisconnected);
                }
                Err(mpsc::TryRecvError::Empty) => {} // still working
            }
        }

        // Phase 2: check if we should kick off a new worker. Cheap syscall — OK
        // to do every couple of seconds.
        if self.last_config_check.elapsed().as_secs() < 2 {
            return;
        }
        self.last_config_check = Instant::now();

        // Skip if there are unsaved local changes (we'd clobber them) or a
        // previous reload is still in flight.
        if self.config_dirty || self.hot_reload_rx.is_some() {
            return;
        }

        let new_mtime = Self::get_config_mtime();
        if new_mtime == self.config_mtime {
            return;
        }
        self.config_mtime = new_mtime;
        tracing::info!("Config file changed externally, spawning reload worker…");

        let (tx, rx) = mpsc::channel();
        self.hot_reload_rx = Some(rx);
        std::thread::spawn(move || {
            // AppConfig::load already falls back to defaults on parse errors,
            // so this thread can't panic in practice.
            let config = AppConfig::load();
            let scene = Scene::from_config(&config);
            // Receiver dropped (e.g. app exiting) → ignore send error.
            let _ = tx.send(HotReloadResult { config, scene });
        });
    }

    /// Apply a finished hot-reload result on the UI thread.
    /// Diffs textures by entity ID so unchanged entities keep their GPU
    /// memory instead of being re-uploaded from scratch.
    fn apply_hot_reload(&mut self, result: HotReloadResult) {
        // Drop textures whose entity ID is no longer in the new scene.
        if let Some(renderer) = &mut self.renderer {
            let new_ids: HashSet<&str> = result
                .scene
                .entities
                .iter()
                .map(|e| e.id.as_str())
                .collect();
            renderer
                .textures
                .retain(|id, _| new_ids.contains(id.as_str()));
        }

        self.config = result.config;
        self.scene = result.scene;
        self.selection.deselect();

        // For each entity: ensure_texture either creates new, updates in
        // place (same dimensions), or recreates (different dimensions).
        if let Some(renderer) = &mut self.renderer {
            for entity in &mut self.scene.entities {
                renderer.ensure_texture(entity);
                entity.texture_dirty = false;
            }
        }

        let n = self.scene.entities.len();
        tracing::info!("Hot-reload applied: {n} entities");
        self.toasts
            .info(format!("Reloaded {n} entities from config"));
    }

    /// Dispatch a context menu outcome. Called after `ui.render` so we can
    /// freely take `&mut self.renderer` for texture operations.
    fn handle_menu_outcome(&mut self, outcome: panels::ContextMenuOutcome) {
        match outcome {
            panels::ContextMenuOutcome::Open => {}
            panels::ContextMenuOutcome::Close => {
                self.ui_state.context_menu = None;
            }
            panels::ContextMenuOutcome::Action(action) => {
                self.apply_menu_action(action);
                self.ui_state.context_menu = None;
            }
        }
    }

    fn handle_library_outcome(&mut self, outcome: panels::LibraryOutcome) {
        let Some(root) = self.library_root.as_ref() else {
            tracing::warn!("Library outcome received but no library_root is set; ignoring.");
            return;
        };
        // M2 hardening (0.5.2): a hand-edited `library.toml` could
        // carry an absolute path or `../` segment that lifts the
        // resolved target out of the asset root. `resolve_library_asset`
        // canonicalises both sides and rejects anything that escapes.
        let abs_path =
            match resolve_library_asset(root, std::path::Path::new(&outcome.relative_path)) {
                Ok(p) => p,
                Err(reason) => {
                    tracing::warn!("Library asset {} rejected: {reason}", outcome.relative_path,);
                    self.toasts.warn(format!("Rejected: {reason}"));
                    return;
                }
            };
        // The shared stat/whitelist gate still applies — a path that
        // stays inside the root can still be the wrong shape.
        if let Err(reason) = pre_validate_dropped_file(&abs_path) {
            tracing::warn!(
                "Library asset {} rejected: {reason}",
                redact_path(&abs_path)
            );
            tracing::debug!("Rejected library full path: {}", abs_path.display());
            self.toasts.warn(format!("Rejected: {reason}"));
            return;
        }
        // Drop in the middle of the visible viewport, falling back to
        // a sensible default when the window isn't fully wired yet.
        let (x, y) = self
            .window
            .as_ref()
            .map(|w| {
                let size = w.inner_size();
                (size.width as f32 / 2.0, size.height as f32 / 2.0)
            })
            .unwrap_or((400.0, 300.0));
        // `add_entity_from_path` runs the full asset-cap + extension
        // detection pipeline — same path as drag-drop — so audit L2
        // is preserved even though the asset came from the library
        // index instead of a user drop.
        match self.scene.add_entity_from_path(&abs_path, x, y) {
            Ok(_) => {
                let mut args = fluent::FluentArgs::new();
                args.set("name", outcome.display_name.clone());
                self.toasts
                    .success(crate::i18n::t_args("library-asset-added-toast", &args));
                // Bump last_used_at so the asset surfaces in the future
                // "Recent" sort introduced in C.9 polish.
                if let Some(library) = self.library.as_mut() {
                    if let Some(asset) =
                        library.assets.iter_mut().find(|a| a.id == outcome.asset_id)
                    {
                        asset.last_used_at = Some(std::time::SystemTime::now());
                    }
                    // Best-effort persist; failure is non-fatal.
                    let _ = library.save(&crate::asset_library::LibraryIndex::default_path());
                }
                self.config_dirty = true;
            }
            Err(e) => {
                tracing::warn!("Library add failed for {}: {e}", outcome.relative_path);
                let mut args = fluent::FluentArgs::new();
                args.set("name", outcome.display_name);
                self.toasts
                    .error(crate::i18n::t_args("library-asset-add-failed-toast", &args));
            }
        }
    }

    fn handle_palette_outcome(&mut self, outcome: panels::PaletteOutcome) {
        use crate::presets::{self, Preset};
        match outcome {
            panels::PaletteOutcome::SwitchTheme(theme) => {
                self.config.global.theme = theme;
                self.config_dirty = true;
                self.toasts.success(format!("Theme: {}", theme.label()));
            }
            panels::PaletteOutcome::ApplyPreset(id, mode) => {
                let preset = Preset::for_id(id);
                let existing = self.scene.to_character_configs();
                let new = presets::apply_to_scene(existing, &preset, mode);
                match mode {
                    presets::ApplyMode::Replace => {
                        self.scene.reset_to_configs(&new);
                        self.selection.deselect();
                    }
                    presets::ApplyMode::Append => {
                        let already: std::collections::HashSet<String> =
                            self.scene.entities.iter().map(|e| e.id.clone()).collect();
                        for cfg in new.iter().filter(|c| !already.contains(&c.id)) {
                            if let Err(e) = self.scene.append_character_config(cfg) {
                                tracing::warn!("Palette preset append failed: {e}");
                                self.toasts.warn(format!("Couldn't add preset entry: {e}"));
                            }
                        }
                    }
                }
                self.config_dirty = true;
                self.toasts
                    .success(format!("Loaded preset: {}", preset.name));
            }
        }
    }

    fn apply_menu_action(&mut self, action: panels::MenuAction) {
        match action {
            panels::MenuAction::Duplicate(idx) => {
                let Some(src) = self.scene.entities.get(idx) else {
                    return;
                };
                let src_name = src.name.clone();
                let src_path = std::path::PathBuf::from(&src.asset_path);
                let new_x = src.x + 30.0;
                let new_y = src.y + 30.0;
                let orig_scale = src.scale;
                let orig_opacity = src.opacity;

                match self.scene.add_entity_from_path(&src_path, new_x, new_y) {
                    Ok(new_idx) => {
                        self.scene.entities[new_idx].scale = orig_scale;
                        self.scene.entities[new_idx].opacity = orig_opacity;
                        if let Some(renderer) = &mut self.renderer {
                            renderer.ensure_texture(&self.scene.entities[new_idx]);
                            self.scene.entities[new_idx].texture_dirty = false;
                        }
                        self.selection.select(new_idx);
                        self.config_dirty = true;
                        self.toasts.success(format!("Duplicated {src_name}"));
                        self.save_config_if_needed();
                    }
                    Err(e) => {
                        tracing::error!("Context menu duplicate failed: {}", e);
                        self.toasts.error(format!("Duplicate failed: {e}"));
                    }
                }
            }
            panels::MenuAction::Delete(idx) => {
                let removed_name = self
                    .scene
                    .entities
                    .get(idx)
                    .map(|e| e.name.clone())
                    .unwrap_or_default();
                if let Some(renderer) = &mut self.renderer {
                    if let Some(entity) = self.scene.entities.get(idx) {
                        renderer.textures.remove(&entity.id);
                    }
                }
                if self.scene.remove_entity(idx).is_some() {
                    self.selection.deselect();
                    self.config_dirty = true;
                    self.toasts.info(format!("Deleted {removed_name}"));
                    self.save_config_if_needed();
                }
            }
            panels::MenuAction::ResetTransform(idx) => {
                if let Some(e) = self.scene.entities.get_mut(idx) {
                    e.scale = 1.0;
                    e.opacity = 1.0;
                    self.config_dirty = true;
                }
            }
            panels::MenuAction::ToggleGravity(idx) => {
                if let Some(e) = self.scene.entities.get_mut(idx) {
                    e.physics.toggle();
                    self.config_dirty = true;
                }
            }
            panels::MenuAction::BringForward(idx) => {
                if let Some(e) = self.scene.entities.get_mut(idx) {
                    e.z_index += 10;
                    self.scene.mark_visible_dirty();
                    self.config_dirty = true;
                }
            }
            panels::MenuAction::SendBackward(idx) => {
                if let Some(e) = self.scene.entities.get_mut(idx) {
                    e.z_index -= 10;
                    self.scene.mark_visible_dirty();
                    self.config_dirty = true;
                }
            }
        }
    }

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

                            if let (Some(ui), Some(window)) =
                                (self.ui.as_mut(), self.window.as_ref())
                            {
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
                                let last_seen_whats_new_mut =
                                    &mut self.config.global.last_seen_whats_new;
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
                                            );
                                            if let Some(state) = &menu_state {
                                                *menu_outcome_ref =
                                                    Some(panels::context_menu(ctx, state));
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
                                            tracing::info!(
                                                "Perf snapshot written: {}",
                                                path.display()
                                            );
                                            self.toasts.success(format!(
                                                "Perf snapshot: {}",
                                                path.display()
                                            ));
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

                // Request next frame
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_x = position.x as f32;
                self.mouse_y = position.y as f32;

                // Handle drag in edit mode
                if self.edit_mode {
                    if let Some((entity_idx, new_x, new_y)) =
                        self.drag.update(self.mouse_x, self.mouse_y)
                    {
                        if entity_idx < self.scene.entities.len() {
                            let entity = &mut self.scene.entities[entity_idx];
                            entity.x = new_x;
                            entity.y = new_y;
                            // Drag relocates the entity → invalidate any
                            // Bounce rest position so the next tick
                            // re-snaps it from the new (x, y) and the
                            // sprite doesn't spring back to the old
                            // centre as soon as drag ends.
                            entity.behavior_state.bounce_invalidate();
                        }
                    }
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                tracing::debug!(
                    "MouseInput: {:?} {:?} at ({:.0}, {:.0}) edit_mode={}",
                    button,
                    state,
                    self.mouse_x,
                    self.mouse_y,
                    self.edit_mode
                );

                // Toggle ⚙ button click is handled by egui (consumed at the
                // top of window_event) → if we got here in pass-through, the
                // click was on a transparent area we don't care about.

                // Edit mode: handle entity selection, drag, and right-click context menu.
                if self.edit_mode {
                    // Right-click on an entity opens the context menu and
                    // selects it. Right-click on empty space does nothing
                    // (entity-less menu is reserved for a later phase).
                    if button == MouseButton::Right && state == ElementState::Pressed {
                        if let Some(entity_idx) =
                            self.scene.entity_at_point(self.mouse_x, self.mouse_y)
                        {
                            self.selection.select(entity_idx);
                            self.ui_state.context_menu = Some(ContextMenuState {
                                entity_idx,
                                pos: egui::pos2(self.mouse_x, self.mouse_y),
                            });
                        }
                        return;
                    }

                    match (button, state) {
                        (MouseButton::Left, ElementState::Pressed) => {
                            // Find entity under cursor
                            if let Some(entity_idx) =
                                self.scene.entity_at_point(self.mouse_x, self.mouse_y)
                            {
                                self.selection.select(entity_idx);

                                // Start drag — freeze physics
                                let entity = &mut self.scene.entities[entity_idx];
                                entity.physics.freeze();
                                let offset_x = self.mouse_x - entity.x;
                                let offset_y = self.mouse_y - entity.y;
                                self.drag.start_drag(entity_idx, offset_x, offset_y);

                                tracing::info!("Clicked entity: {} ({})", entity.name, entity.id);
                            } else {
                                self.selection.deselect();
                            }
                        }
                        (MouseButton::Left, ElementState::Released) if self.drag.is_dragging() => {
                            // Drop the freeze. Physics remains whatever the user set —
                            // off by default (entity stays put), on if they pressed G.
                            if let Some(idx) = self.drag.dragging_entity() {
                                if idx < self.scene.entities.len() {
                                    self.scene.entities[idx].physics.unfreeze();
                                }
                            }
                            self.drag.end_drag();
                            self.config_dirty = true;
                            self.save_config_if_needed();
                        }
                        _ => {}
                    }
                }
            }

            // Keyboard input works in edit mode (when window has full input shape)
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

            // Scroll wheel: resize selected entity in edit mode
            WindowEvent::MouseWheel { delta, .. } if self.edit_mode => {
                if let Some(idx) = self.selection.selected_index() {
                    let scroll_y = match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                        winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 50.0,
                    };
                    let entity = &mut self.scene.entities[idx];
                    let factor = if scroll_y > 0.0 { 1.1 } else { 0.9 };
                    entity.scale = (entity.scale * factor).clamp(0.1, 10.0);
                    tracing::debug!("Scale: {:.2}", entity.scale);
                    self.config_dirty = true;
                }
            }

            // --- Drag and drop: add new assets ---
            WindowEvent::DroppedFile(path) => {
                let label = redact_path(&path);
                tracing::info!("File dropped: {label}");
                tracing::debug!("Dropped full path: {}", path.display());

                // Pre-validate before we hand the path to the decoders.
                // Catches the obvious bad cases (wrong extension, huge
                // file) with a fast, clear error toast instead of letting
                // the decoder spin up and fail somewhere deeper.
                if let Err(reason) = pre_validate_dropped_file(&path) {
                    tracing::warn!("Rejecting dropped file {label}: {reason}");
                    tracing::debug!("Rejected full path: {}", path.display());
                    self.toasts.error(format!("Rejected: {reason}"));
                    return;
                }

                // If not in edit mode, enter it automatically
                if !self.edit_mode {
                    self.toggle_edit_mode();
                }

                // Try to add the entity at the current mouse position
                match self
                    .scene
                    .add_entity_from_path(&path, self.mouse_x, self.mouse_y)
                {
                    Ok(idx) => {
                        // Create texture for the new entity
                        if let Some(renderer) = &mut self.renderer {
                            renderer.ensure_texture(&self.scene.entities[idx]);
                            self.scene.entities[idx].texture_dirty = false;
                        }
                        // Select the new entity
                        self.selection.select(idx);
                        self.config_dirty = true;
                        let added_name = self.scene.entities[idx].name.clone();
                        self.save_config_if_needed();
                        tracing::info!(
                            "Added '{}' at ({:.0}, {:.0})",
                            added_name,
                            self.mouse_x,
                            self.mouse_y
                        );
                        self.toasts.success(format!("Added {added_name}"));
                    }
                    Err(e) => {
                        tracing::error!("Failed to load dropped file {}: {}", path.display(), e);
                        self.toasts.error(format!("Load failed: {e}"));
                    }
                }
            }

            WindowEvent::HoveredFile(path) => {
                tracing::debug!("File hovering: {}", path.display());
            }

            // Track all four modifiers so user-bound chords involving
            // Alt or Super resolve correctly via `KeyBindings::lookup`.
            WindowEvent::ModifiersChanged(modifiers) => {
                self.shift_held = modifiers.state().shift_key();
                self.ctrl_held = modifiers.state().control_key();
                self.alt_held = modifiers.state().alt_key();
                self.super_held = modifiers.state().super_key();
            }

            _ => {}
        }
    }
}
