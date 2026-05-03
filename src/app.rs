use crate::config::AppConfig;
use crate::input::drag::DragController;
use crate::input::selection::SelectionState;
use crate::renderer::wgpu_renderer::WgpuRenderer;
use crate::scene::Scene;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId, WindowLevel};

// X11-specific: set window type to DOCK so it stays above all normal windows
#[cfg(target_os = "linux")]
use winit::platform::x11::{WindowAttributesExtX11, WindowType};

/// Main application state — implements winit's ApplicationHandler.
///
/// The overlay operates in two modes:
/// - **Pass-through mode** (default): clicks go through to the desktop.
///   Characters are visible but non-interactive. You can use your desktop normally.
/// - **Edit mode** (toggle with F1): clicks are captured by the overlay.
///   You can drag characters, select them, etc.
///   Press F1 again to return to pass-through mode.
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
            edit_mode: false, // Start in pass-through mode — desktop is usable
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

        if let Some(window) = &self.window {
            // set_cursor_hittest(false) = clicks pass through the window to desktop
            // set_cursor_hittest(true)  = clicks are captured by the overlay (edit mode)
            match window.set_cursor_hittest(self.edit_mode) {
                Ok(_) => {}
                Err(e) => {
                    log::warn!(
                        "Failed to set cursor hit-test ({}): {}. \
                         Click-through may not work on this compositor.",
                        if self.edit_mode {
                            "edit mode"
                        } else {
                            "pass-through"
                        },
                        e
                    );
                }
            }
        }

        if self.edit_mode {
            log::info!(
                "━━━ EDIT MODE ON ━━━ Click and drag characters. Press F1 to exit edit mode."
            );
        } else {
            log::info!(
                "━━━ PASS-THROUGH MODE ━━━ Clicks go to desktop. Press F1 to enter edit mode."
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
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return; // Already created
        }

        log::info!("Creating window...");

        // Build window attributes: transparent, borderless, always-on-top
        // On X11: set window type to DOCK — this tells the window manager
        // to keep this window above all normal application windows, like a panel.
        let window_attrs = Window::default_attributes()
            .with_title("animaEngine")
            .with_transparent(true)
            .with_decorations(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.config.global.window_width,
                self.config.global.window_height,
            ));

        // X11-specific: Set window type to Dock (stays above all windows)
        #[cfg(target_os = "linux")]
        let window_attrs = window_attrs.with_x11_window_type(vec![WindowType::Dock]);

        match event_loop.create_window(window_attrs) {
            Ok(window) => {
                let window = Arc::new(window);
                log::info!(
                    "Window created: {:?} ({}x{})",
                    window.id(),
                    window.inner_size().width,
                    window.inner_size().height
                );

                // CRITICAL: Enable click-through immediately so the desktop is usable.
                // The window starts in pass-through mode by default.
                match window.set_cursor_hittest(false) {
                    Ok(_) => {
                        log::info!(
                            "Click-through enabled — desktop is usable. Press F1 to enter edit mode."
                        );
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to enable click-through: {}. \
                             The overlay may block input. Try running with: GDK_BACKEND=x11 cargo run",
                            e
                        );
                    }
                }

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
                event_loop.exit();
            }

            WindowEvent::Resized(physical_size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(physical_size.width, physical_size.height);
                }
            }

            WindowEvent::RedrawRequested => {
                // Tick animations
                self.scene.tick();

                // Update textures for entities with changed frames
                if let Some(renderer) = &mut self.renderer {
                    for entity in &mut self.scene.entities {
                        if entity.texture_dirty {
                            renderer.ensure_texture(entity);
                            entity.texture_dirty = false;
                        }
                    }

                    // Render all visible entities
                    let visible = self.scene.visible_entities();
                    match renderer.render(&visible) {
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

                // Only process drag when in edit mode
                if self.edit_mode {
                    if let Some((entity_idx, new_x, new_y)) =
                        self.drag.update(self.mouse_x, self.mouse_y)
                    {
                        if let Some(entity) = self.scene.entities.get_mut(entity_idx) {
                            entity.x = new_x;
                            entity.y = new_y;
                        }
                    }
                }
            }

            WindowEvent::MouseInput { state, button, .. }
                // Only process mouse clicks when in edit mode
                if self.edit_mode =>
            {
                match (button, state) {
                    (MouseButton::Left, ElementState::Pressed) => {
                        // Find entity under cursor
                        if let Some(entity_idx) =
                            self.scene.entity_at_point(self.mouse_x, self.mouse_y)
                        {
                            self.selection.select(entity_idx);

                            // Start drag
                            let entity = &self.scene.entities[entity_idx];
                            let offset_x = self.mouse_x - entity.x;
                            let offset_y = self.mouse_y - entity.y;
                            self.drag.start_drag(entity_idx, offset_x, offset_y);

                            log::info!("Clicked entity: {} ({})", entity.name, entity.id);
                        } else {
                            self.selection.deselect();
                        }
                    }
                    (MouseButton::Left, ElementState::Released) if self.drag.is_dragging() => {
                        self.drag.end_drag();
                        self.config_dirty = true;
                        // Save after drag ends
                        self.save_config_if_needed();
                    }
                    _ => {}
                }
            }

            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        state: ElementState::Pressed,
                        ref logical_key,
                        ..
                    },
                ..
            } => match logical_key.as_ref() {
                // F1: Toggle edit mode / pass-through mode
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::F1) => {
                    self.toggle_edit_mode();
                }
                // Space: toggle global play/pause (works in both modes)
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::Space) => {
                    self.scene.toggle_global_playback();
                    self.config_dirty = true;
                }
                // Escape: exit (works in both modes)
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) => {
                    log::info!("Escape pressed — saving and exiting");
                    self.save_config_if_needed();
                    // Drop renderer before exiting to avoid segfault on Vulkan cleanup
                    self.renderer = None;
                    event_loop.exit();
                }
                // S: save config (works in both modes)
                winit::keyboard::Key::Character("s") => {
                    self.config_dirty = true;
                    self.save_config_if_needed();
                    log::info!("Config saved manually");
                }
                _ => {}
            },

            _ => {}
        }
    }
}
