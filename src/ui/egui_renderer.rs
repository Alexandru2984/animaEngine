//! Owns the egui state and bridges it to wgpu/winit.

use std::sync::Arc;

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

        // Push the initial design-system style; the rest of the UI code
        // can read `ctx.style()` and trust it matches docs/design-system.md.
        theme::apply(&context, theme);

        Self {
            context,
            state,
            renderer,
            current_theme: theme,
        }
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
        response.consumed
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
        let raw_input = self.state.take_egui_input(window);
        let full_output = self.context.run(raw_input, build_ui);

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
