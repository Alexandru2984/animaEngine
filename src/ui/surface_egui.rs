//! egui on a surface driven by hand: one frame of input in, one painted
//! frame over the sprites out (`LoadOp::Load`). No winit, no screen
//! reader, no input method — whoever owns the surface adds those.
//!
//! The native Wayland panel builds on it (`wayland::egui_render`), and
//! since 1.5 every other monitor's overlay has one too, on both backends:
//! egui is what draws text, and a speech bubble or a right-click menu on a
//! monitor without the panel had nothing to draw it with.

use crate::ui::{icons, theme};

pub struct SurfaceEgui {
    context: egui::Context,
    renderer: egui_wgpu::Renderer,
    /// Last applied theme, so `theme::apply` runs only on a change.
    current_theme: theme::Theme,
    /// When egui asked to be run again; `None` when it asked for nothing.
    repaint_at: Option<std::time::Instant>,
    /// egui's clock starts here (`RawInput::time`).
    started: std::time::Instant,
}

impl SurfaceEgui {
    pub fn new(
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
        theme: theme::Theme,
    ) -> Self {
        let context = egui::Context::default();
        let renderer = egui_wgpu::Renderer::new(device, output_format, None, 1, false);
        icons::install(&context);
        theme::apply(&context, theme);
        Self {
            context,
            renderer,
            current_theme: theme,
            repaint_at: None,
            started: std::time::Instant::now(),
        }
    }

    pub fn context(&self) -> &egui::Context {
        &self.context
    }

    /// Re-apply the design-system style if the active theme changed.
    pub fn ensure_theme(&mut self, theme: theme::Theme) {
        if self.current_theme != theme {
            theme::apply(&self.context, theme);
            self.current_theme = theme;
        }
    }

    /// When egui asked to be run again, if it did.
    pub fn repaint_at(&self) -> Option<std::time::Instant> {
        self.repaint_at
    }

    /// Whether egui asked to be run again by `now`.
    pub fn repaint_due(&self, now: std::time::Instant) -> bool {
        self.repaint_at.is_some_and(|at| at <= now)
    }

    /// Run one egui frame with `events` and paint it over `view`, whose
    /// size is `size_in_pixels`. Returns egui's platform output — the
    /// screen reader's tree, the input method's caret — for an owner that
    /// wants it.
    #[allow(clippy::too_many_arguments)]
    pub fn render<F>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        size_in_pixels: [u32; 2],
        pixels_per_point: f32,
        events: Vec<egui::Event>,
        modifiers: egui::Modifiers,
        build_ui: F,
    ) -> egui::PlatformOutput
    where
        F: FnMut(&egui::Context),
    {
        let pixels_per_point = pixels_per_point.max(0.5);
        let logical_size = egui::vec2(
            size_in_pixels[0] as f32 / pixels_per_point,
            size_in_pixels[1] as f32 / pixels_per_point,
        );
        let raw_input = egui::RawInput {
            viewport_id: self.context.viewport_id(),
            viewports: std::iter::once((
                self.context.viewport_id(),
                egui::ViewportInfo {
                    native_pixels_per_point: Some(pixels_per_point),
                    inner_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, logical_size)),
                    ..Default::default()
                },
            ))
            .collect(),
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, logical_size)),
            // Real time. `None` makes egui add 1/60 s per frame, which is
            // right only while frames come sixty a second; drawn on demand,
            // a tooltip's half-second delay took fifteen.
            time: Some(self.started.elapsed().as_secs_f64()),
            predicted_dt: 1.0 / 60.0,
            // egui answers `input.modifiers` from here, not from the
            // modifiers carried on individual key events.
            modifiers,
            events,
            hovered_files: Vec::new(),
            dropped_files: Vec::new(),
            focused: true,
            max_texture_side: None,
            system_theme: None,
        };

        let full_output = self.context.run(raw_input, build_ui);
        // `Duration::MAX` means "not unless something happens"; the add
        // overflows then, and there is nothing to schedule.
        self.repaint_at = full_output
            .viewport_output
            .get(&self.context.viewport_id())
            .and_then(|v| std::time::Instant::now().checked_add(v.repaint_delay));

        let paint_jobs = self
            .context
            .tessellate(full_output.shapes, pixels_per_point);
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels,
            pixels_per_point,
        };
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
                        // Load — the sprites underneath stay visible.
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let mut static_pass = render_pass.forget_lifetime();
            self.renderer
                .render(&mut static_pass, &paint_jobs, &screen_descriptor);
        }
        queue.submit(std::iter::once(encoder.finish()));
        for id in &full_output.textures_delta.free {
            self.renderer.free_texture(id);
        }
        full_output.platform_output
    }
}
