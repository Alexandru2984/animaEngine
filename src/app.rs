use crate::config::AppConfig;
use crate::input::drag::DragController;
use crate::input::selection::SelectionState;
use crate::renderer::wgpu_renderer::WgpuRenderer;
use crate::scene::Scene;
use crate::window::x11_input::{X11InputManager, TOGGLE_BUTTON_SIZE};
use std::sync::Arc;
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
    /// Pooled X11 input manager (holds a single X11 connection)
    x11_input: Option<X11InputManager>,
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
            x11_input: None,
        }
    }

    /// Save config if dirty
    fn save_config_if_needed(&mut self) {
        if self.config_dirty {
            self.config.characters = self.scene.to_character_configs();
            self.config.global.playback_enabled = self.scene.global_playing;
            if let Err(e) = self.config.save() {
                log::warn!("Failed to save config: {}", e);
            }
            self.config_dirty = false;
        }
    }

    /// Toggle between edit mode and pass-through mode
    fn toggle_edit_mode(&mut self) {
        self.edit_mode = !self.edit_mode;

        if let Some(x11) = &mut self.x11_input {
            if self.edit_mode {
                // Edit mode: full window receives input
                if let Err(e) = x11.set_full_input() {
                    log::warn!("Failed to set full input shape: {}", e);
                    // Fallback to winit's method
                    if let Some(window) = &self.window {
                        let _ = window.set_cursor_hittest(true);
                    }
                }
            } else {
                // Pass-through mode: only the toggle button receives input
                if let Err(e) = x11.set_passthrough_with_button(TOGGLE_BUTTON_SIZE) {
                    log::warn!("Failed to set passthrough input shape: {}", e);
                    if let Some(window) = &self.window {
                        let _ = window.set_cursor_hittest(false);
                    }
                }
            }
        } else if let Some(window) = &self.window {
            // No X11 manager — use winit fallback
            let _ = window.set_cursor_hittest(self.edit_mode);
        }

        if self.edit_mode {
            log::info!(
                "━━━ EDIT MODE ON ━━━ Click and drag characters. Press Escape or click ⚙ button to exit."
            );
        } else {
            log::info!(
                "━━━ PASS-THROUGH MODE ━━━ Clicks go to desktop. Click ⚙ button to enter edit mode."
            );
            // End any active drag when leaving edit mode
            if self.drag.is_dragging() {
                self.drag.end_drag();
                self.config_dirty = true;
                self.save_config_if_needed();
            }
            self.selection.deselect();
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

        log::info!("Creating window...");

        // Auto-detect screen resolution if config values are 0
        let (win_w, win_h) = if self.config.global.window_width == 0
            || self.config.global.window_height == 0
        {
            if let Some(monitor) = event_loop
                .primary_monitor()
                .or_else(|| event_loop.available_monitors().next())
            {
                let size = monitor.size();
                log::info!(
                    "Auto-detected monitor resolution: {}x{}",
                    size.width,
                    size.height
                );
                (size.width, size.height)
            } else {
                log::warn!("Could not detect monitor resolution, falling back to 1920x1080");
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
                log::info!(
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
                        log::warn!("Failed to set initial input shape: {}", e);
                        let _ = window.set_cursor_hittest(false);
                    }
                } else {
                    log::warn!("X11InputManager not available. Falling back to set_cursor_hittest.");
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
                        self.renderer = Some(renderer);
                        log::info!("wgpu renderer initialized successfully");
                    }
                    Err(e) => {
                        log::error!("Failed to initialize wgpu renderer: {}", e);
                        event_loop.exit();
                        return;
                    }
                }

                self.window = Some(window);
            }
            Err(e) => {
                log::error!("Failed to create window: {}", e);
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
        match event {
            WindowEvent::CloseRequested => {
                log::info!("Close requested — saving config and exiting");
                self.save_config_if_needed();
                // Drop renderer before exiting to avoid segfault on Vulkan cleanup
                self.renderer = None;
                // Drop X11 connection
                self.x11_input = None;
                event_loop.exit();
            }

            WindowEvent::Resized(physical_size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(physical_size.width, physical_size.height);
                }
                // Re-apply input shape after resize
                if !self.edit_mode {
                    if let Some(x11) = &mut self.x11_input {
                        let _ = x11.set_passthrough_with_button(TOGGLE_BUTTON_SIZE);
                    }
                }
            }

            WindowEvent::RedrawRequested => {
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

                    // Render all visible entities + UI
                    let visible = self.scene.visible_entities();
                    match renderer.render(&visible, self.edit_mode, selected_id) {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => {
                            renderer.resize(renderer.window_width, renderer.window_height);
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            log::error!("GPU out of memory!");
                            event_loop.exit();
                        }
                        Err(e) => {
                            log::warn!("Render error: {:?}", e);
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

                // Debug: log when cursor enters the button area
                if self.is_toggle_button_click(self.mouse_x, self.mouse_y) {
                    log::debug!(
                        "Cursor in button area: ({:.0}, {:.0})",
                        self.mouse_x,
                        self.mouse_y
                    );
                }

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
                log::debug!(
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
                    log::info!("Toggle button clicked at ({:.0}, {:.0})", self.mouse_x, self.mouse_y);
                    self.toggle_edit_mode();
                    return;
                }

                // Edit mode: handle entity selection and drag
                if self.edit_mode {
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

                                log::info!("Clicked entity: {} ({})", entity.name, entity.id);
                            } else {
                                self.selection.deselect();
                            }
                        }
                        (MouseButton::Left, ElementState::Released)
                            if self.drag.is_dragging() =>
                        {
                            // Unfreeze physics but keep grounded — entity stays where placed
                            if let Some(idx) = self.drag.dragging_entity() {
                                if idx < self.scene.entities.len() {
                                    self.scene.entities[idx].physics.frozen = false;
                                    self.scene.entities[idx].physics.grounded = true;
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
                    log::info!("Q pressed — saving and exiting");
                    self.save_config_if_needed();
                    self.renderer = None;
                    self.x11_input = None;
                    event_loop.exit();
                }
                winit::keyboard::Key::Character("s") => {
                    self.config_dirty = true;
                    self.save_config_if_needed();
                    log::info!("Config saved manually");
                }
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::Space) => {
                    self.scene.toggle_global_playback();
                    self.config_dirty = true;
                }
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::Delete)
                | winit::keyboard::Key::Named(winit::keyboard::NamedKey::Backspace) => {
                    // Delete selected entity
                    if let Some(idx) = self.selection.selected_index() {
                        // Remove GPU texture for this entity
                        if let Some(renderer) = &mut self.renderer {
                            let entity_id = &self.scene.entities[idx].id;
                            renderer.textures.remove(entity_id);
                        }
                        if let Some(removed_id) = self.scene.remove_entity(idx) {
                            log::info!("Deleted entity: {}", removed_id);
                            self.selection.deselect();
                            self.config_dirty = true;
                            self.save_config_if_needed();
                        }
                    }
                }
                // Arrow keys: nudge selected entity position
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowUp) => {
                    if let Some(idx) = self.selection.selected_index() {
                        self.scene.entities[idx].y -= 10.0;
                        self.config_dirty = true;
                    }
                }
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowDown) => {
                    if let Some(idx) = self.selection.selected_index() {
                        self.scene.entities[idx].y += 10.0;
                        self.config_dirty = true;
                    }
                }
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowLeft) => {
                    if let Some(idx) = self.selection.selected_index() {
                        self.scene.entities[idx].x -= 10.0;
                        self.config_dirty = true;
                    }
                }
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowRight) => {
                    if let Some(idx) = self.selection.selected_index() {
                        self.scene.entities[idx].x += 10.0;
                        self.config_dirty = true;
                    }
                }
                // +/= increase opacity, - decrease opacity
                winit::keyboard::Key::Character("+" | "=") => {
                    if let Some(idx) = self.selection.selected_index() {
                        let entity = &mut self.scene.entities[idx];
                        entity.opacity = (entity.opacity + 0.1).min(1.0);
                        log::info!("Opacity: {:.0}%", entity.opacity * 100.0);
                        self.config_dirty = true;
                    }
                }
                winit::keyboard::Key::Character("-") => {
                    if let Some(idx) = self.selection.selected_index() {
                        let entity = &mut self.scene.entities[idx];
                        entity.opacity = (entity.opacity - 0.1).max(0.05);
                        log::info!("Opacity: {:.0}%", entity.opacity * 100.0);
                        self.config_dirty = true;
                    }
                }
                // V: toggle visibility of selected entity
                winit::keyboard::Key::Character("v") => {
                    if let Some(idx) = self.selection.selected_index() {
                        let entity = &mut self.scene.entities[idx];
                        entity.visible = !entity.visible;
                        log::info!(
                            "Entity '{}' visibility: {}",
                            entity.name,
                            if entity.visible { "visible" } else { "hidden" }
                        );
                        self.config_dirty = true;
                    }
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
                    log::debug!("Scale: {:.2}", entity.scale);
                    self.config_dirty = true;
                }
            }

            // --- Drag and drop: add new assets ---
            WindowEvent::DroppedFile(path) => {
                log::info!("File dropped: {}", path.display());

                // If not in edit mode, enter it automatically
                if !self.edit_mode {
                    self.toggle_edit_mode();
                }

                // Try to add the entity at the current mouse position
                match self.scene.add_entity_from_path(&path, self.mouse_x, self.mouse_y) {
                    Ok(idx) => {
                        // Create texture for the new entity
                        if let Some(renderer) = &mut self.renderer {
                            renderer.ensure_texture(&self.scene.entities[idx]);
                            self.scene.entities[idx].texture_dirty = false;
                        }
                        // Select the new entity
                        self.selection.select(idx);
                        self.config_dirty = true;
                        self.save_config_if_needed();
                        log::info!(
                            "Added '{}' at ({:.0}, {:.0})",
                            self.scene.entities[idx].name,
                            self.mouse_x,
                            self.mouse_y
                        );
                    }
                    Err(e) => {
                        log::error!("Failed to load dropped file {}: {}", path.display(), e);
                    }
                }
            }

            WindowEvent::HoveredFile(path) => {
                log::debug!("File hovering: {}", path.display());
            }

            _ => {}
        }
    }
}


