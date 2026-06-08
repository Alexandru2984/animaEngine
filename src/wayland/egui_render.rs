//! Egui paint integration on the native Wayland path (E.4).
//!
//! The X11 path uses `egui_winit::State` to bridge winit events into
//! egui. We don't have a winit window here — the Wayland queue
//! produces sctk events that the layer-window translator has already
//! mapped into `Vec<egui::Event>`. Building the `egui::RawInput`
//! manually from that vector is straightforward; the rest is plain
//! `egui::Context` + `egui_wgpu::Renderer` wiring.
//!
//! Per-frame protocol:
//!
//! 1. Caller drains pointer/keyboard events from `LayerWindow`.
//! 2. Caller invokes [`WaylandEguiRenderer::render`] with the events,
//!    surface size, and a UI closure.
//! 3. The renderer builds `RawInput`, runs egui, tessellates the
//!    output, paints over the wgpu surface with `LoadOp::Load` so the
//!    sprites underneath survive.
//!
//! HiDPI is intentionally pegged at 1× for now (compositor scale
//! plumbing is part of E.7's multi-monitor work; E.9 picks up the
//! polish).

use crate::ui::{icons, theme};

/// Single-window egui integration for the native Wayland path. Mirrors
/// `crate::ui::EguiRenderer` but skips the `egui_winit::State`.
pub struct WaylandEguiRenderer {
    context: egui::Context,
    renderer: egui_wgpu::Renderer,
    /// Last applied theme — guards `theme::apply` so it only fires on
    /// a real change. Matches the X11 path's `ensure_theme` pattern.
    current_theme: theme::Theme,
}

impl WaylandEguiRenderer {
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
        }
    }

    /// Re-apply the design-system style if the active theme changed.
    pub fn ensure_theme(&mut self, theme: theme::Theme) {
        if self.current_theme != theme {
            theme::apply(&self.context, theme);
            self.current_theme = theme;
        }
    }

    /// Run one egui frame on top of an already-rendered surface.
    ///
    /// `events` is consumed in place — every drained event from the
    /// layer-window's translator goes in unchanged. `size_in_pixels`
    /// must match the surface's current dimensions; `pixels_per_point`
    /// is the compositor's reported scale (1.0 on single-DPI displays,
    /// 2.0 on most "Retina"-grade panels, etc.). Egui's layout snaps
    /// to this scale so glyphs stay crisp.
    pub fn render<F>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        size_in_pixels: [u32; 2],
        pixels_per_point: f32,
        events: Vec<egui::Event>,
        build_ui: F,
    ) where
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
            time: None,
            predicted_dt: 1.0 / 60.0,
            modifiers: egui::Modifiers::default(),
            events,
            hovered_files: Vec::new(),
            dropped_files: Vec::new(),
            focused: true,
            max_texture_side: None,
            system_theme: None,
        };

        let full_output = self.context.run(raw_input, build_ui);

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
            label: Some("egui encoder (wayland)"),
        });
        self.renderer
            .update_buffers(device, queue, &mut encoder, &paint_jobs, &screen_descriptor);

        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui render pass (wayland)"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Load — sprites underneath stay visible.
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
    }
}
