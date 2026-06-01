use crate::config::AppConfig;
use crate::constants::TOGGLE_BUTTON_SIZE;
use crate::input::drag::DragController;
use crate::input::selection::SelectionState;
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
    /// Pooled X11 input manager (holds a single X11 connection)
    x11_input: Option<X11InputManager>,
    /// Last time we checked config file for hot-reload
    last_config_check: Instant,
    /// Last known modification time of config file
    config_mtime: Option<SystemTime>,
    /// Receiver for an in-flight async hot-reload. `Some` means a worker
    /// thread is currently decoding the new config + assets off the UI thread.
    hot_reload_rx: Option<mpsc::Receiver<HotReloadResult>>,
    /// egui integration. Only paints in edit mode but always receives events
    /// (mouse position tracking, etc.).
    ui: Option<EguiRenderer>,
    /// Ephemeral UI state (currently just the context menu) kept separate
    /// from the egui renderer so it survives across resumed/suspended cycles.
    ui_state: UiState,
    /// Toast notification queue. Persistent across edit/pass-through
    /// transitions but only painted when in edit mode (no UI otherwise).
    toasts: ToastQueue,
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
            x11_input: None,
            last_config_check: Instant::now(),
            config_mtime: Self::get_config_mtime(),
            hot_reload_rx: None,
            ui: None,
            ui_state: UiState::default(),
            toasts: ToastQueue::default(),
        }
    }

    /// Get the modification time of the config file
    fn get_config_mtime() -> Option<SystemTime> {
        let path = AppConfig::config_path();
        std::fs::metadata(&path).ok()?.modified().ok()
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
                Ok(()) => self.toasts.success("Config saved"),
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

    /// Check if a click is on the toggle button (top-right corner)
    fn is_toggle_button_click(&self, x: f32, y: f32) -> bool {
        if let Some(window) = &self.window {
            let win_width = window.inner_size().width as f32;
            let button_x = win_width - TOGGLE_BUTTON_SIZE as f32;
            x >= button_x && y <= TOGGLE_BUTTON_SIZE as f32
        } else {
            false
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return; // Already created
        }

        tracing::info!("Creating window...");

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
        #[cfg(target_os = "linux")]
        let window_attrs = window_attrs.with_x11_window_type(vec![WindowType::Normal]);

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
        // Let egui peek at the event first when we're in edit mode. If a
        // widget consumes it (e.g. typing in a text box, dragging a slider)
        // we skip our own handler so the same click doesn't also move an
        // entity. Outside edit mode there's no UI, so we never forward.
        if self.edit_mode {
            if let (Some(ui), Some(window)) = (self.ui.as_mut(), self.window.as_ref()) {
                if ui.handle_event(window, &event) {
                    return;
                }
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

                // Tick animations + physics
                let screen_h = self
                    .window
                    .as_ref()
                    .map(|w| w.inner_size().height as f32)
                    .unwrap_or(1080.0);
                self.scene.tick(screen_h);

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
                            let mut menu_outcome: Option<panels::ContextMenuOutcome> = None;
                            if self.edit_mode {
                                // Drop expired toasts before painting.
                                self.toasts.prune();

                                if let (Some(ui), Some(window)) =
                                    (self.ui.as_mut(), self.window.as_ref())
                                {
                                    let view = output
                                        .texture
                                        .create_view(&wgpu::TextureViewDescriptor::default());
                                    let size = [renderer.window_width, renderer.window_height];

                                    // Disjoint mutable borrows on disjoint fields.
                                    let scene_mut = &mut self.scene;
                                    let selection_mut = &mut self.selection;
                                    let config_dirty_mut = &mut self.config_dirty;
                                    let toasts_ref = &self.toasts;
                                    let menu_state = self.ui_state.context_menu.clone();
                                    let menu_outcome_ref = &mut menu_outcome;

                                    ui.render(
                                        window,
                                        &renderer.device,
                                        &renderer.queue,
                                        &view,
                                        size,
                                        |ctx| {
                                            panels::settings(
                                                ctx,
                                                scene_mut,
                                                selection_mut,
                                                config_dirty_mut,
                                            );
                                            if let Some(state) = &menu_state {
                                                *menu_outcome_ref =
                                                    Some(panels::context_menu(ctx, state));
                                            }
                                            panels::toasts(ctx, toasts_ref);
                                        },
                                    );
                                }
                            }
                            output.present();

                            // Apply menu outcome AFTER the UI render so we
                            // can take &mut renderer for texture management
                            // (Duplicate creates a texture; Delete drops one).
                            if let Some(outcome) = menu_outcome {
                                self.handle_menu_outcome(outcome);
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
                            self.scene.entities[entity_idx].x = new_x;
                            self.scene.entities[entity_idx].y = new_y;
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

                // Toggle button click: works in BOTH modes (pass-through has input shape for it)
                if button == MouseButton::Left
                    && state == ElementState::Pressed
                    && self.is_toggle_button_click(self.mouse_x, self.mouse_y)
                {
                    tracing::info!(
                        "Toggle button clicked at ({:.0}, {:.0})",
                        self.mouse_x,
                        self.mouse_y
                    );
                    self.toggle_edit_mode();
                    return;
                }

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
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        state: ElementState::Pressed,
                        ref logical_key,
                        ..
                    },
                ..
            } if self.edit_mode => match logical_key.as_ref() {
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) => {
                    self.toggle_edit_mode();
                }
                winit::keyboard::Key::Character("q") => {
                    tracing::info!("Q pressed — saving and exiting");
                    self.save_config_if_needed();
                    self.ui = None;
                    self.renderer = None;
                    self.x11_input = None;
                    event_loop.exit();
                }
                winit::keyboard::Key::Character("s") => {
                    self.config_dirty = true;
                    self.save_config_if_needed();
                    tracing::info!("Config saved manually");
                }
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::Space) => {
                    self.scene.toggle_global_playback();
                    self.config_dirty = true;
                }
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::Delete)
                | winit::keyboard::Key::Named(winit::keyboard::NamedKey::Backspace) => {
                    // Delete selected entity
                    if let Some(idx) = self.selection.selected_index() {
                        let removed_name = self
                            .scene
                            .entities
                            .get(idx)
                            .map(|e| e.name.clone())
                            .unwrap_or_default();
                        // Remove GPU texture for this entity
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
                // Arrow keys: nudge selected entity position (Shift = 1px fine, normal = 10px)
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowUp) => {
                    if let Some(idx) = self.selection.selected_index() {
                        let step = if self.shift_held { 1.0 } else { 10.0 };
                        self.scene.entities[idx].y -= step;
                        self.config_dirty = true;
                    }
                }
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowDown) => {
                    if let Some(idx) = self.selection.selected_index() {
                        let step = if self.shift_held { 1.0 } else { 10.0 };
                        self.scene.entities[idx].y += step;
                        self.config_dirty = true;
                    }
                }
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowLeft) => {
                    if let Some(idx) = self.selection.selected_index() {
                        let step = if self.shift_held { 1.0 } else { 10.0 };
                        self.scene.entities[idx].x -= step;
                        self.config_dirty = true;
                    }
                }
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowRight) => {
                    if let Some(idx) = self.selection.selected_index() {
                        let step = if self.shift_held { 1.0 } else { 10.0 };
                        self.scene.entities[idx].x += step;
                        self.config_dirty = true;
                    }
                }
                // R: reset scale and opacity to defaults
                winit::keyboard::Key::Character("r") => {
                    if let Some(idx) = self.selection.selected_index() {
                        let entity = &mut self.scene.entities[idx];
                        entity.scale = 1.0;
                        entity.opacity = 1.0;
                        tracing::info!("Reset '{}' scale=1.0, opacity=1.0", entity.name);
                        self.config_dirty = true;
                    }
                }
                // Home: center selected entity on screen
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::Home) => {
                    if let Some(idx) = self.selection.selected_index() {
                        if let Some(window) = &self.window {
                            let size = window.inner_size();
                            let entity = &mut self.scene.entities[idx];
                            entity.x = (size.width as f32 - entity.scaled_width()) / 2.0;
                            entity.y = (size.height as f32 - entity.scaled_height()) / 2.0;
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
                // +/= increase opacity, - decrease opacity
                winit::keyboard::Key::Character("+" | "=") => {
                    if let Some(idx) = self.selection.selected_index() {
                        let entity = &mut self.scene.entities[idx];
                        entity.opacity = (entity.opacity + 0.1).min(1.0);
                        tracing::info!("Opacity: {:.0}%", entity.opacity * 100.0);
                        self.config_dirty = true;
                    }
                }
                winit::keyboard::Key::Character("-") => {
                    if let Some(idx) = self.selection.selected_index() {
                        let entity = &mut self.scene.entities[idx];
                        entity.opacity = (entity.opacity - 0.1).max(0.05);
                        tracing::info!("Opacity: {:.0}%", entity.opacity * 100.0);
                        self.config_dirty = true;
                    }
                }
                // V: toggle visibility of selected entity
                winit::keyboard::Key::Character("v") => {
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
                // G: toggle gravity for selected entity (off by default).
                // When toggled on, the entity falls from its current position.
                // When toggled off, the entity is pinned where it is.
                winit::keyboard::Key::Character("g") => {
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
                // P: toggle play/pause for selected entity
                winit::keyboard::Key::Character("p") => {
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
                // D: duplicate selected entity
                winit::keyboard::Key::Character("d") => {
                    if let Some(idx) = self.selection.selected_index() {
                        let src = &self.scene.entities[idx];
                        let src_path = std::path::PathBuf::from(&src.asset_path);
                        let new_x = src.x + 30.0;
                        let new_y = src.y + 30.0;

                        match self.scene.add_entity_from_path(&src_path, new_x, new_y) {
                            Ok(new_idx) => {
                                // Copy scale/opacity from original
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
                // Tab: cycle selection through entities
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::Tab)
                    if !self.scene.entities.is_empty() =>
                {
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
                // Page Up: increase z-index (bring forward)
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::PageUp) => {
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
                // Page Down: decrease z-index (send backward)
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::PageDown) => {
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
                // [: decrease FPS (slower animation)
                winit::keyboard::Key::Character("[") => {
                    if let Some(idx) = self.selection.selected_index() {
                        let entity = &mut self.scene.entities[idx];
                        entity
                            .animation
                            .set_fps((entity.animation.fps - 2.0).max(1.0));
                        tracing::info!("FPS: {:.0} ({})", entity.animation.fps, entity.name);
                        self.config_dirty = true;
                    }
                }
                // ]: increase FPS (faster animation)
                winit::keyboard::Key::Character("]") => {
                    if let Some(idx) = self.selection.selected_index() {
                        let entity = &mut self.scene.entities[idx];
                        entity.animation.set_fps(entity.animation.fps + 2.0);
                        tracing::info!("FPS: {:.0} ({})", entity.animation.fps, entity.name);
                        self.config_dirty = true;
                    }
                }
                // I: show entity info
                winit::keyboard::Key::Character("i") => {
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
                // H: show help (all keyboard shortcuts)
                winit::keyboard::Key::Character("h") => {
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
                _ => {}
            },

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

            // Track modifier keys (Shift for fine nudge)
            WindowEvent::ModifiersChanged(modifiers) => {
                self.shift_held = modifiers.state().shift_key();
            }

            _ => {}
        }
    }
}
