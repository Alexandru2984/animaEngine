//! Owns the egui state and bridges it to wgpu/winit.

use std::sync::Arc;

use crate::ui::icons;
use crate::ui::theme::{self, Theme};

/// Single-window egui integration. All three pieces (`Context`, `State`,
/// `Renderer`) are kept together because they share an implicit invariant:
/// the viewport id used to construct `State` must match the context the
/// painting code uses, otherwise input events go nowhere.
pub struct EguiRenderer {
    context: egui::Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
    /// Last theme pushed into `context.style`. We track it so
    /// `ensure_theme` can be called on every frame for free — only an
    /// actual theme switch reaches `theme::apply`.
    current_theme: Theme,
    /// Screen readers: this window's tree on AT-SPI (`crate::a11y`).
    #[cfg(unix)]
    screen_reader: crate::a11y::ScreenReaderBridge,
    /// The Appearance setting that allows the tree at all.
    #[cfg(unix)]
    accesskit_allowed: bool,
    /// When egui asked to be run again; `None` when it asked for nothing.
    repaint_at: Option<std::time::Instant>,
}

impl EguiRenderer {
    pub fn new(
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
        window: Arc<winit::window::Window>,
        theme: Theme,
    ) -> Self {
        let context = egui::Context::default();
        let viewport_id = context.viewport_id();
        let state = egui_winit::State::new(
            context.clone(),
            viewport_id,
            window.as_ref(),
            Some(window.scale_factor() as f32),
            None,
            None,
        );

        // Single-sample, no depth — matches our sprite pipeline.
        let renderer = egui_wgpu::Renderer::new(device, output_format, None, 1, false);

        // Push the initial design-system style + register the icon font.
        // Order matters: `set_fonts` invalidates the glyph atlas, so we
        // do it before any panel paints — both calls happen at startup.
        icons::install(&context);
        theme::apply(&context, theme);

        // A reader's request needs a frame to be answered, and the paced
        // loop may be asleep. `request_redraw` may be called from any
        // thread; the weak handle lets the window go when the app does.
        #[cfg(unix)]
        let screen_reader = {
            let weak = Arc::downgrade(&window);
            let mut bridge = crate::a11y::ScreenReaderBridge::new(move || {
                if let Some(window) = weak.upgrade() {
                    window.request_redraw();
                }
            });
            update_bounds(&mut bridge, &window);
            bridge
        };

        Self {
            context,
            state,
            renderer,
            current_theme: theme,
            #[cfg(unix)]
            screen_reader,
            #[cfg(unix)]
            accesskit_allowed: true,
            repaint_at: None,
        }
    }

    /// When egui asked to be run again — a tooltip's delay, a caret's
    /// blink, an animation. The render loop schedules its next frame by it.
    pub fn repaint_at(&self) -> Option<std::time::Instant> {
        self.repaint_at
    }

    /// Whether an input is under way that an edit is still coming from:
    /// a pointer button held, a text field focused, a list or the palette
    /// open. Holds an undo step open (`crate::undo`).
    pub fn input_in_progress(&self) -> bool {
        self.context.input(|i| i.pointer.any_down())
            || self.context.wants_keyboard_input()
            || crate::ui::panels::keyboard_held(&self.context)
    }

    /// Whether a screen reader has requests waiting for the next frame.
    pub fn has_screen_reader_requests(&self) -> bool {
        #[cfg(unix)]
        {
            self.screen_reader.has_requests()
        }
        #[cfg(not(unix))]
        {
            false
        }
    }

    /// Follow the Appearance setting that allows screen readers the tree.
    /// Applies from the next frame.
    pub fn set_accesskit_allowed(&mut self, allowed: bool) {
        #[cfg(unix)]
        {
            self.accesskit_allowed = allowed;
        }
        // No bridge elsewhere yet: the tree is never built.
        #[cfg(not(unix))]
        let _ = allowed;
    }

    /// Re-apply the design-system style if the active theme changed.
    /// Cheap when stable (one enum comparison); call once per frame
    /// from the event loop with the value currently in `AppConfig`.
    pub fn ensure_theme(&mut self, theme: Theme) {
        if self.current_theme != theme {
            theme::apply(&self.context, theme);
            self.current_theme = theme;
        }
    }

    /// Forward a window event to egui. Returns `true` when egui consumed it
    /// — the caller should then skip its own handling so we don't, e.g.,
    /// drag an entity while the user is typing in a text box.
    pub fn handle_event(
        &mut self,
        window: &winit::window::Window,
        event: &winit::event::WindowEvent,
    ) -> bool {
        let response = self.state.on_window_event(window, event);
        #[cfg(unix)]
        match event {
            winit::event::WindowEvent::Focused(focused) => {
                self.screen_reader.set_focused(*focused);
            }
            winit::event::WindowEvent::Moved(_) | winit::event::WindowEvent::Resized(_) => {
                update_bounds(&mut self.screen_reader, window);
            }
            _ => {}
        }
        // The open command palette, or an open list, takes every key — see
        // `panels::keyboard_held`.
        response.consumed
            || (matches!(event, winit::event::WindowEvent::KeyboardInput { .. })
                && crate::ui::panels::keyboard_held(&self.context))
    }

    /// Paint the UI for one frame on top of the already-rendered scene.
    ///
    /// Must run **after** `WgpuRenderer::render` in the same frame, sharing
    /// the same `output` texture — egui appends a `LoadOp::Load` pass so it
    /// preserves sprites underneath.
    pub fn render<F>(
        &mut self,
        window: &winit::window::Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        size_in_pixels: [u32; 2],
        build_ui: F,
    ) where
        F: FnMut(&egui::Context),
    {
        #[cfg(unix)]
        {
            crate::a11y::sync_egui(&self.context, self.accesskit_allowed, &self.screen_reader);
            let requests = self.screen_reader.drain_requests();
            self.state.egui_input_mut().events.extend(requests);
        }
        #[cfg(not(unix))]
        self.context.disable_accesskit();
        let raw_input = self.state.take_egui_input(window);
        #[cfg_attr(not(unix), allow(unused_mut))]
        let mut full_output = self.context.run(raw_input, build_ui);
        // `Duration::MAX` means "not unless something happens"; the add
        // overflows then, and there is nothing to schedule.
        self.repaint_at = full_output
            .viewport_output
            .get(&self.context.viewport_id())
            .and_then(|v| std::time::Instant::now().checked_add(v.repaint_delay));
        #[cfg(unix)]
        self.screen_reader.publish(
            self.accesskit_allowed,
            full_output.platform_output.accesskit_update.take(),
        );

        // Apply platform-side effects (cursor, clipboard, IME requests).
        self.state
            .handle_platform_output(window, full_output.platform_output);

        let pixels_per_point = full_output.pixels_per_point;
        let paint_jobs = self
            .context
            .tessellate(full_output.shapes, pixels_per_point);
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels,
            pixels_per_point,
        };

        // Texture deltas — egui notifies us about font / image atlases.
        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("egui encoder"),
        });

        self.renderer
            .update_buffers(device, queue, &mut encoder, &paint_jobs, &screen_descriptor);

        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Load — don't wipe the sprites already drawn.
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            // egui-wgpu requires a 'static render pass — `forget_lifetime`
            // is the documented escape hatch when we control the encoder.
            let mut static_pass = render_pass.forget_lifetime();
            self.renderer
                .render(&mut static_pass, &paint_jobs, &screen_descriptor);
        }

        queue.submit(std::iter::once(encoder.finish()));

        // Free textures egui no longer needs.
        for id in &full_output.textures_delta.free {
            self.renderer.free_texture(id);
        }
    }
}

/// The window's content area on the screen, for the screen reader.
#[cfg(unix)]
fn update_bounds(bridge: &mut crate::a11y::ScreenReaderBridge, window: &winit::window::Window) {
    if let Ok(position) = window.inner_position() {
        let size = window.inner_size();
        bridge.set_bounds(
            f64::from(position.x),
            f64::from(position.y),
            f64::from(size.width),
            f64::from(size.height),
        );
    }
}
