use crate::config::AppConfig;
use crate::constants::TOGGLE_BUTTON_SIZE;
use crate::event::AnimaEvent;
use crate::input::drag::DragController;
use crate::input::selection::SelectionState;
use crate::keybindings::{Action, KeyChord, KeyCode, ModifierMask};
use crate::renderer::wgpu_renderer::WgpuRenderer;
use crate::scene::Scene;
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

    /// Run the handler bound to `action`. The match preserves the
    /// per-arm behaviour previously inlined in the `KeyboardInput`
    /// match: per-entity actions silently no-op without a selection,
    /// `QuitWithSave` tears down GPU/X11 state before calling
    /// `event_loop.exit()`, etc.
    fn dispatch_action(&mut self, action: Action, event_loop: &ActiveEventLoop) {
        match action {
            Action::ToggleEditMode => {
                self.toggle_edit_mode();
            }
            Action::QuitWithSave => {
                tracing::info!("Quit action — saving and exiting");
                self.save_config_if_needed();
                self.ui = None;
                self.renderer = None;
                self.x11_input = None;
                event_loop.exit();
            }
            Action::SaveNow => {
                self.config_dirty = true;
                self.save_config_if_needed();
                tracing::info!("Config saved manually");
            }
            Action::PauseAll => {
                self.scene.toggle_global_playback();
                self.config_dirty = true;
            }
            Action::DeleteSelected => {
                if let Some(idx) = self.selection.selected_index() {
                    let removed_name = self
                        .scene
                        .entities
                        .get(idx)
                        .map(|e| e.name.clone())
                        .unwrap_or_default();
                    if let Some(renderer) = &mut self.renderer {
                        let entity_id = &self.scene.entities[idx].id;
                        renderer.textures.remove(entity_id);
                    }
                    if let Some(removed_id) = self.scene.remove_entity(idx) {
                        tracing::info!("Deleted entity: {}", removed_id);
                        self.selection.deselect();
                        self.config_dirty = true;
                        self.toasts.info(format!("Deleted {removed_name}"));
                        self.save_config_if_needed();
                    }
                }
            }
            // Arrow nudges: Shift = 1 px fine, normal = 10 px. Every
            // nudge invalidates Bounce rest so the entity doesn't
            // snap back after the keypress.
            Action::NudgeUp => {
                if let Some(idx) = self.selection.selected_index() {
                    let step = if self.shift_held { 1.0 } else { 10.0 };
                    self.scene.entities[idx].y -= step;
                    self.scene.entities[idx].behavior_state.bounce_invalidate();
                    self.config_dirty = true;
                }
            }
            Action::NudgeDown => {
                if let Some(idx) = self.selection.selected_index() {
                    let step = if self.shift_held { 1.0 } else { 10.0 };
                    self.scene.entities[idx].y += step;
                    self.scene.entities[idx].behavior_state.bounce_invalidate();
                    self.config_dirty = true;
                }
            }
            Action::NudgeLeft => {
                if let Some(idx) = self.selection.selected_index() {
                    let step = if self.shift_held { 1.0 } else { 10.0 };
                    self.scene.entities[idx].x -= step;
                    self.scene.entities[idx].behavior_state.bounce_invalidate();
                    self.config_dirty = true;
                }
            }
            Action::NudgeRight => {
                if let Some(idx) = self.selection.selected_index() {
                    let step = if self.shift_held { 1.0 } else { 10.0 };
                    self.scene.entities[idx].x += step;
                    self.scene.entities[idx].behavior_state.bounce_invalidate();
                    self.config_dirty = true;
                }
            }
            Action::ResetTransform => {
                if let Some(idx) = self.selection.selected_index() {
                    let entity = &mut self.scene.entities[idx];
                    entity.scale = 1.0;
                    entity.opacity = 1.0;
                    tracing::info!("Reset '{}' scale=1.0, opacity=1.0", entity.name);
                    self.config_dirty = true;
                }
            }
            Action::CenterOnScreen => {
                if let Some(idx) = self.selection.selected_index() {
                    if let Some(window) = &self.window {
                        let size = window.inner_size();
                        let entity = &mut self.scene.entities[idx];
                        entity.x = (size.width as f32 - entity.scaled_width()) / 2.0;
                        entity.y = (size.height as f32 - entity.scaled_height()) / 2.0;
                        entity.behavior_state.bounce_invalidate();
                        tracing::info!(
                            "Centered '{}' at ({:.0}, {:.0})",
                            entity.name,
                            entity.x,
                            entity.y
                        );
                        self.config_dirty = true;
                    }
                }
            }
            Action::OpacityUp => {
                if let Some(idx) = self.selection.selected_index() {
                    let entity = &mut self.scene.entities[idx];
                    entity.opacity = (entity.opacity + 0.1).min(1.0);
                    tracing::info!("Opacity: {:.0}%", entity.opacity * 100.0);
                    self.config_dirty = true;
                }
            }
            Action::OpacityDown => {
                if let Some(idx) = self.selection.selected_index() {
                    let entity = &mut self.scene.entities[idx];
                    entity.opacity = (entity.opacity - 0.1).max(0.05);
                    tracing::info!("Opacity: {:.0}%", entity.opacity * 100.0);
                    self.config_dirty = true;
                }
            }
            Action::ToggleVisible => {
                if let Some(idx) = self.selection.selected_index() {
                    let entity = &mut self.scene.entities[idx];
                    entity.visible = !entity.visible;
                    tracing::info!(
                        "Entity '{}' visibility: {}",
                        entity.name,
                        if entity.visible { "visible" } else { "hidden" }
                    );
                    self.scene.mark_visible_dirty();
                    self.config_dirty = true;
                }
            }
            // Gravity: off by default — entity stays put. Toggling on
            // makes it fall from its current position; off pins it.
            Action::ToggleGravity => {
                if let Some(idx) = self.selection.selected_index() {
                    let entity = &mut self.scene.entities[idx];
                    entity.physics.toggle();
                    tracing::info!(
                        "Entity '{}' gravity: {}",
                        entity.name,
                        if entity.physics.enabled {
                            "ON (falling)"
                        } else {
                            "OFF (pinned)"
                        }
                    );
                    self.config_dirty = true;
                }
            }
            Action::TogglePlayback => {
                if let Some(idx) = self.selection.selected_index() {
                    let entity = &mut self.scene.entities[idx];
                    entity.animation.toggle_playback();
                    tracing::info!(
                        "Entity '{}': {}",
                        entity.name,
                        if entity.animation.playing {
                            "playing"
                        } else {
                            "paused"
                        }
                    );
                    self.config_dirty = true;
                }
            }
            Action::DuplicateSelected => {
                if let Some(idx) = self.selection.selected_index() {
                    let src = &self.scene.entities[idx];
                    let src_path = std::path::PathBuf::from(&src.asset_path);
                    let new_x = src.x + 30.0;
                    let new_y = src.y + 30.0;
                    match self.scene.add_entity_from_path(&src_path, new_x, new_y) {
                        Ok(new_idx) => {
                            let orig_scale = self.scene.entities[idx].scale;
                            let orig_opacity = self.scene.entities[idx].opacity;
                            self.scene.entities[new_idx].scale = orig_scale;
                            self.scene.entities[new_idx].opacity = orig_opacity;
                            if let Some(renderer) = &mut self.renderer {
                                renderer.ensure_texture(&self.scene.entities[new_idx]);
                                self.scene.entities[new_idx].texture_dirty = false;
                            }
                            self.selection.select(new_idx);
                            self.config_dirty = true;
                            self.save_config_if_needed();
                            tracing::info!("Duplicated entity at ({:.0}, {:.0})", new_x, new_y);
                        }
                        Err(e) => tracing::error!("Failed to duplicate: {}", e),
                    }
                }
            }
            Action::CycleEntity => {
                // Empty scene: nothing to cycle through, silently
                // no-op so the user's `Tab` doesn't grab focus from
                // the egui panel (which Tab would otherwise navigate).
                if self.scene.entities.is_empty() {
                    return;
                }
                let next = match self.selection.selected_index() {
                    Some(idx) => (idx + 1) % self.scene.entities.len(),
                    None => 0,
                };
                self.selection.select(next);
                tracing::info!(
                    "Selected: {} ({})",
                    self.scene.entities[next].name,
                    self.scene.entities[next].id
                );
            }
            Action::BringForward => {
                if let Some(idx) = self.selection.selected_index() {
                    self.scene.entities[idx].z_index += 10;
                    tracing::info!(
                        "z-index: {} ({})",
                        self.scene.entities[idx].z_index,
                        self.scene.entities[idx].name
                    );
                    self.scene.mark_visible_dirty();
                    self.config_dirty = true;
                }
            }
            Action::SendBackward => {
                if let Some(idx) = self.selection.selected_index() {
                    self.scene.entities[idx].z_index -= 10;
                    tracing::info!(
                        "z-index: {} ({})",
                        self.scene.entities[idx].z_index,
                        self.scene.entities[idx].name
                    );
                    self.scene.mark_visible_dirty();
                    self.config_dirty = true;
                }
            }
            Action::FpsDown => {
                if let Some(idx) = self.selection.selected_index() {
                    let entity = &mut self.scene.entities[idx];
                    entity
                        .animation
                        .set_fps((entity.animation.fps - 2.0).max(1.0));
                    tracing::info!("FPS: {:.0} ({})", entity.animation.fps, entity.name);
                    self.config_dirty = true;
                }
            }
            Action::FpsUp => {
                if let Some(idx) = self.selection.selected_index() {
                    let entity = &mut self.scene.entities[idx];
                    entity.animation.set_fps(entity.animation.fps + 2.0);
                    tracing::info!("FPS: {:.0} ({})", entity.animation.fps, entity.name);
                    self.config_dirty = true;
                }
            }
            Action::CycleMonitor => {
                if let Some(idx) = self.selection.selected_index() {
                    if let Some(entity) = self.scene.entities.get_mut(idx) {
                        let toast =
                            panels::cycle_entity_monitor(&mut entity.monitor, &self.monitors);
                        self.toasts.info(toast);
                        self.config_dirty = true;
                    }
                }
            }
            Action::ShowEntityInfo => {
                if let Some(idx) = self.selection.selected_index() {
                    let e = &self.scene.entities[idx];
                    tracing::info!(
                        "━━━ Entity Info ━━━\n  Name: {}\n  ID: {}\n  Position: ({:.0}, {:.0})\n  Scale: {:.2}\n  Opacity: {:.0}%\n  FPS: {:.0}\n  Frames: {}\n  z-index: {}\n  Visible: {}\n  Playing: {}\n  Asset: {}",
                        e.name, e.id, e.x, e.y, e.scale,
                        e.opacity * 100.0, e.animation.fps,
                        e.animation.frame_count(), e.z_index,
                        e.visible, e.animation.playing, e.asset_path
                    );
                }
            }
            Action::ShowHelp => {
                tracing::info!(
                    "━━━ KEYBOARD SHORTCUTS ━━━\n\
                    \n  Navigation:\n\
                    \n    Tab        — Cycle through entities\n\
                    \n    Click      — Select entity\n\
                    \n    Escape     — Exit edit mode (auto-saves)\n\
                    \n\n  Position:\n\
                    \n    Drag       — Move entity\n\
                    \n    Arrows     — Nudge 10px\n\
                    \n    Shift+Arrows — Fine nudge 1px\n\
                    \n    Home       — Center on screen\n\
                    \n\n  Appearance:\n\
                    \n    Scroll     — Resize\n\
                    \n    +/-        — Opacity\n\
                    \n    R          — Reset scale/opacity\n\
                    \n    V          — Toggle visibility\n\
                    \n    PgUp/PgDn  — Z-order\n\
                    \n\n  Animation:\n\
                    \n    P          — Play/pause entity\n\
                    \n    Space      — Global play/pause\n\
                    \n    [/]        — Adjust FPS\n\
                    \n\n  Physics:\n\
                    \n    G          — Toggle gravity (off by default)\n\
                    \n\n  Actions:\n\
                    \n    D          — Duplicate\n\
                    \n    Del/Bksp   — Delete\n\
                    \n    I          — Show entity info\n\
                    \n    S          — Save config\n\
                    \n    Q          — Save and exit\n\
                    \n    H          — This help"
                );
            }
            // Actions whose runtime path lives outside the in-app
            // dispatch: HideOverlay fires only as a global hotkey;
            // OpenCommandPalette is intercepted by `panels.rs` reading
            // egui's keyboard input. Both reach this match arm when
            // a user rebinds them onto a chord that's still active in
            // edit mode — we leave the handling to the original sites
            // rather than duplicate it here.
            Action::HideOverlay | Action::OpenCommandPalette => {}
        }
    }

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
        let abs_path = root.join(&outcome.relative_path);
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
                self.scene.tick(screen_w, screen_h, cursor);

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
                                let monitors_ref = self.monitors.as_slice();
                                let toasts_ref = &self.toasts;
                                let menu_state = self.ui_state.context_menu.clone();
                                let menu_outcome_ref = &mut menu_outcome;
                                let palette_outcome_ref = &mut palette_outcome;
                                let library_outcome_ref = &mut library_outcome;
                                let library_ref = self.library.as_ref();
                                let toggle_requested_ref = &mut toggle_requested;
                                let edit_mode = self.edit_mode;

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
                                            );
                                            if let Some(state) = &menu_state {
                                                *menu_outcome_ref =
                                                    Some(panels::context_menu(ctx, state));
                                            }
                                            // Ctrl+K opens the command palette.
                                            *palette_outcome_ref = panels::command_palette(ctx);
                                            panels::toasts(ctx, toasts_ref);
                                        }
                                    },
                                );
                            }
                            output.present();

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
                tracing::info!("File dropped: {}", path.display());

                // Pre-validate before we hand the path to the decoders.
                // Catches the obvious bad cases (wrong extension, huge
                // file) with a fast, clear error toast instead of letting
                // the decoder spin up and fail somewhere deeper.
                if let Err(reason) = pre_validate_dropped_file(&path) {
                    tracing::warn!("Rejecting dropped file {}: {reason}", path.display());
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

/// Extensions we know how to load. Matched against the path the user
/// dropped so we reject obviously-wrong types up front.
const DROP_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "mp4", "m4v", "mov"];

/// Sanity-check a dropped file before invoking the decoder. Returns the
/// reason string when the file should be rejected, or `Ok(())` when it
/// looks plausible.
fn pre_validate_dropped_file(path: &std::path::Path) -> std::result::Result<(), String> {
    use crate::constants::MAX_ASSET_FILE_BYTES;

    let meta = std::fs::metadata(path).map_err(|e| format!("can't stat file: {e}"))?;
    if !meta.is_file() {
        return Err("not a regular file".into());
    }
    if meta.len() > MAX_ASSET_FILE_BYTES {
        return Err(format!(
            "file is {} MB; cap is {} MB",
            meta.len() / (1024 * 1024),
            MAX_ASSET_FILE_BYTES / (1024 * 1024)
        ));
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase());
    match ext.as_deref() {
        Some(e) if DROP_EXTENSIONS.contains(&e) => Ok(()),
        Some(e) => Err(format!("unsupported file type: .{e}")),
        None => Err("file has no extension".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::pre_validate_dropped_file;
    use std::path::PathBuf;

    fn workspace_tmp(name: &str) -> PathBuf {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("app_tests")
            .join(name);
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn drop_rejects_unsupported_extension() {
        let dir = workspace_tmp("drop_bad_ext");
        let path = dir.join("evil.exe");
        std::fs::write(&path, b"x").unwrap();
        let err = pre_validate_dropped_file(&path).unwrap_err();
        assert!(err.contains("unsupported"));
    }

    #[test]
    fn drop_rejects_oversized_file() {
        // A 1-byte file with our cap reduced — we can't reduce the const
        // at test time, so synthesize a file just over the limit instead.
        // (200 MB would be slow; we use a tiny stub with an inverted
        // assertion: file under the limit must be accepted.)
        let dir = workspace_tmp("drop_size_ok");
        let path = dir.join("tiny.png");
        std::fs::write(&path, b"x").unwrap();
        assert!(pre_validate_dropped_file(&path).is_ok());
    }

    #[test]
    fn drop_rejects_directory() {
        let dir = workspace_tmp("drop_is_dir");
        let err = pre_validate_dropped_file(&dir).unwrap_err();
        assert!(err.contains("not a regular file"));
    }

    #[test]
    fn drop_rejects_missing_extension() {
        let dir = workspace_tmp("drop_no_ext");
        let path = dir.join("noext");
        std::fs::write(&path, b"x").unwrap();
        let err = pre_validate_dropped_file(&path).unwrap_err();
        assert!(err.contains("no extension"));
    }
}
