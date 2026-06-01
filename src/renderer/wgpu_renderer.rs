use super::sprite::{make_quad_vertices, orthographic_projection, SpriteVertex, QUAD_INDICES};
use super::texture::GpuTexture;
use crate::animation::frame::Frame;
use crate::entity::Entity;
use crate::error::{AnimaError, Result};
use crate::window::x11_input::TOGGLE_BUTTON_SIZE;
use bytemuck;
use std::collections::HashMap;
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Maximum number of quads we can batch in one draw call.
/// Entities + UI elements (button, edit bar, selection highlights).
/// 64 quads = 256 vertices × 32 bytes = 8 KB — trivial.
const MAX_QUADS: usize = 64;

/// The main GPU renderer.
/// Manages the wgpu device, pipeline, and renders entities to the window surface.
pub struct WgpuRenderer {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub render_pipeline: wgpu::RenderPipeline,
    pub uniform_bind_group_layout: wgpu::BindGroupLayout,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub index_buffer: wgpu::Buffer,
    /// Cached GPU textures per entity id
    pub textures: HashMap<String, GpuTexture>,
    pub window_width: u32,
    pub window_height: u32,
    /// UI: toggle button texture (normal / pass-through state)
    button_tex_normal: GpuTexture,
    /// UI: toggle button texture (active / edit mode state)
    button_tex_active: GpuTexture,
    /// UI: edit mode indicator bar texture (1x1 stretched)
    edit_bar_tex: GpuTexture,
    /// UI: selection highlight texture (semi-transparent border)
    selection_tex: GpuTexture,
    /// Pre-allocated vertex buffer for dynamic quad drawing.
    /// Reused every frame via `queue.write_buffer()` to avoid per-frame allocations.
    dynamic_vertex_buffer: wgpu::Buffer,
}

/// Generate a simple circular button icon.
/// Creates a circle with a ring and center dot — looks like a settings/target icon.
fn generate_button_frame(
    bg: [u8; 4],
    icon: [u8; 4],
    size: u32,
) -> Frame {
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    let cx = size as f32 / 2.0;
    let cy = size as f32 / 2.0;
    let outer_r = size as f32 * 0.42;
    let ring_r = size as f32 * 0.28;
    let ring_w = 2.5;
    let dot_r = size as f32 * 0.09;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist < dot_r {
                // Center dot
                rgba.extend_from_slice(&icon);
            } else if (dist - ring_r).abs() < ring_w {
                // Ring
                let edge = (ring_w - (dist - ring_r).abs()) / ring_w;
                let a = (icon[3] as f32 * edge.min(1.0)) as u8;
                rgba.extend_from_slice(&[icon[0], icon[1], icon[2], a]);
            } else if dist < outer_r {
                // Background fill
                rgba.extend_from_slice(&bg);
            } else if dist < outer_r + 1.5 {
                // Anti-aliased edge
                let factor = (outer_r + 1.5 - dist) / 1.5;
                let a = (bg[3] as f32 * factor) as u8;
                rgba.extend_from_slice(&[bg[0], bg[1], bg[2], a]);
            } else {
                // Outside — fully transparent
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    Frame::new(rgba, size, size)
}

/// Generate a selection highlight frame — a rounded rectangle border with glow.
fn generate_selection_frame(size: u32) -> Frame {
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    let border_w = 3.0f32;
    let glow_w = 5.0f32;
    let corner_r = 6.0f32;

    for y in 0..size {
        for x in 0..size {
            let fx = x as f32;
            let fy = y as f32;
            let w = size as f32;
            let h = size as f32;

            // Distance to the nearest edge (accounting for rounded corners)
            let dx = if fx < corner_r {
                corner_r - fx
            } else if fx > w - corner_r {
                fx - (w - corner_r)
            } else {
                0.0
            };
            let dy = if fy < corner_r {
                corner_r - fy
            } else if fy > h - corner_r {
                fy - (h - corner_r)
            } else {
                0.0
            };

            // Distance from the border (outer edge of the rect)
            let dist_to_edge = if dx > 0.0 && dy > 0.0 {
                // Corner: distance to the rounded corner center
                let corner_dist = (dx * dx + dy * dy).sqrt();
                corner_dist - corner_r + fx.min(w - fx).min(fy).min(h - fy)
            } else {
                fx.min(w - fx).min(fy).min(h - fy)
            };

            if dist_to_edge < border_w {
                // Solid border — cyan-ish
                let edge_factor = (dist_to_edge / border_w).max(0.0);
                let a = (220.0 * (1.0 - edge_factor * 0.3)) as u8;
                rgba.extend_from_slice(&[80, 200, 255, a]);
            } else if dist_to_edge < border_w + glow_w {
                // Glow falloff
                let glow_factor = 1.0 - (dist_to_edge - border_w) / glow_w;
                let a = (100.0 * glow_factor * glow_factor) as u8;
                rgba.extend_from_slice(&[80, 200, 255, a]);
            } else {
                // Interior — transparent
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    Frame::new(rgba, size, size)
}

impl WgpuRenderer {
    /// Initialize the wgpu renderer with the given window
    pub fn new(window: Arc<winit::window::Window>) -> Result<Self> {
        let size = window.inner_size();
        let window_width = size.width.max(1);
        let window_height = size.height.max(1);

        log::info!(
            "Initializing wgpu renderer ({}x{})",
            window_width,
            window_height
        );

        // Create wgpu instance
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::GL,
            ..Default::default()
        });

        // Create surface from window
        let surface = instance.create_surface(window.clone())?;

        // Request adapter
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or(AnimaError::NoAdapter)?;

        log::info!("GPU adapter: {}", adapter.get_info().name);
        log::info!("Backend: {:?}", adapter.get_info().backend);

        // Request device
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("animaEngine Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
            },
            None,
        ))?;

        // Configure surface with transparency
        let surface_caps = surface.get_capabilities(&adapter);
        log::info!("Available alpha modes: {:?}", surface_caps.alpha_modes);
        log::info!("Available formats: {:?}", surface_caps.formats);

        // Pick the best alpha mode for transparency
        let alpha_mode = if surface_caps
            .alpha_modes
            .contains(&wgpu::CompositeAlphaMode::PreMultiplied)
        {
            wgpu::CompositeAlphaMode::PreMultiplied
        } else if surface_caps
            .alpha_modes
            .contains(&wgpu::CompositeAlphaMode::PostMultiplied)
        {
            wgpu::CompositeAlphaMode::PostMultiplied
        } else {
            log::warn!(
                "No premultiplied/postmultiplied alpha mode available. \
                 Transparency may not work. Available modes: {:?}",
                surface_caps.alpha_modes
            );
            surface_caps.alpha_modes[0]
        };
        log::info!("Using alpha mode: {:?}", alpha_mode);

        // Prefer sRGB format
        let format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        log::info!("Using surface format: {:?}", format);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: window_width,
            height: window_height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // --- Create shader module ---
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sprite Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/sprite.wgsl").into()),
        });

        // --- Uniform bind group layout (projection matrix) ---
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Uniform Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // --- Texture bind group layout ---
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        // --- Pipeline layout ---
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sprite Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        // --- Render pipeline ---
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Sprite Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[SpriteVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // No culling for 2D sprites
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // --- Uniform buffer (projection matrix) ---
        let projection = orthographic_projection(window_width as f32, window_height as f32);
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Projection Uniform Buffer"),
            contents: bytemuck::cast_slice(&projection),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // --- Index buffer (shared for all quads) ---
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Quad Index Buffer"),
            contents: bytemuck::cast_slice(&QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        // --- Pre-allocated dynamic vertex buffer ---
        // MAX_QUADS quads × 4 vertices per quad × sizeof(SpriteVertex)
        let vb_size = (MAX_QUADS * 4 * std::mem::size_of::<SpriteVertex>()) as u64;
        let dynamic_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Dynamic Vertex Buffer"),
            size: vb_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        log::info!(
            "Dynamic vertex buffer allocated: {} bytes ({} quads max)",
            vb_size,
            MAX_QUADS
        );

        // --- UI textures ---
        let btn_size = TOGGLE_BUTTON_SIZE;
        // Normal: dark semi-transparent with white ring
        let btn_normal_frame =
            generate_button_frame([50, 50, 60, 160], [200, 200, 220, 200], btn_size);
        let button_tex_normal = GpuTexture::from_frame(
            &device,
            &queue,
            &btn_normal_frame,
            &texture_bind_group_layout,
            "btn_normal",
        );

        // Active: green with white ring
        let btn_active_frame =
            generate_button_frame([40, 160, 60, 200], [255, 255, 255, 240], btn_size);
        let button_tex_active = GpuTexture::from_frame(
            &device,
            &queue,
            &btn_active_frame,
            &texture_bind_group_layout,
            "btn_active",
        );

        // Edit bar: solid green semi-transparent, 1x1 stretched
        let bar_frame = Frame::new(vec![50, 200, 80, 140], 1, 1);
        let edit_bar_tex = GpuTexture::from_frame(
            &device,
            &queue,
            &bar_frame,
            &texture_bind_group_layout,
            "edit_bar",
        );

        // Selection highlight: rounded border with glow, 64x64 stretched over entity
        let sel_frame = generate_selection_frame(64);
        let selection_tex = GpuTexture::from_frame(
            &device,
            &queue,
            &sel_frame,
            &texture_bind_group_layout,
            "selection_highlight",
        );

        Ok(Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
            uniform_bind_group_layout,
            texture_bind_group_layout,
            uniform_buffer,
            uniform_bind_group,
            index_buffer,
            textures: HashMap::new(),
            window_width,
            window_height,
            button_tex_normal,
            button_tex_active,
            edit_bar_tex,
            selection_tex,
            dynamic_vertex_buffer,
        })
    }

    /// Handle window resize
    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        if new_width == 0 || new_height == 0 {
            return;
        }

        self.window_width = new_width;
        self.window_height = new_height;
        self.config.width = new_width;
        self.config.height = new_height;
        self.surface.configure(&self.device, &self.config);

        // Update projection matrix
        let projection = orthographic_projection(new_width as f32, new_height as f32);
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&projection));

        log::debug!("Resized to {}x{}", new_width, new_height);
    }

    /// Ensure a texture exists for an entity, creating or updating as needed
    pub fn ensure_texture(&mut self, entity: &Entity) {
        let needs_create = !self.textures.contains_key(&entity.id);

        if needs_create {
            if let Some(frame) = entity.animation.current_frame_data() {
                let gpu_tex = GpuTexture::from_frame(
                    &self.device,
                    &self.queue,
                    frame,
                    &self.texture_bind_group_layout,
                    &entity.id,
                );
                self.textures.insert(entity.id.clone(), gpu_tex);
            }
        } else if entity.texture_dirty {
            if let Some(frame) = entity.animation.current_frame_data() {
                if let Some(gpu_tex) = self.textures.get(&entity.id) {
                    // If same size, update in place
                    if gpu_tex.width == frame.width && gpu_tex.height == frame.height {
                        gpu_tex.update_from_frame(&self.queue, frame);
                    } else {
                        // Different size: recreate texture
                        let new_tex = GpuTexture::from_frame(
                            &self.device,
                            &self.queue,
                            frame,
                            &self.texture_bind_group_layout,
                            &entity.id,
                        );
                        self.textures.insert(entity.id.clone(), new_tex);
                    }
                }
            }
        }
    }

    /// Write quad vertices into the dynamic buffer at the given quad index offset.
    /// Returns the byte offset where the vertices were written.
    fn write_quad(
        &self,
        quad_index: usize,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        opacity: f32,
    ) -> u64 {
        let vertices = make_quad_vertices(x, y, w, h, opacity);
        let offset = (quad_index * 4 * std::mem::size_of::<SpriteVertex>()) as u64;
        self.queue.write_buffer(
            &self.dynamic_vertex_buffer,
            offset,
            bytemuck::cast_slice(&vertices),
        );
        offset
    }

    /// Render all visible entities + UI overlay to the surface.
    ///
    /// Returns `wgpu::SurfaceError` directly (not `AnimaError`) because the
    /// caller needs to match on specific variants like `Lost` and `OutOfMemory`
    /// to drive recovery and shutdown logic.
    pub fn render(
        &mut self,
        entities: &[&Entity],
        edit_mode: bool,
        selected_entity_id: Option<&str>,
    ) -> std::result::Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // --- Pre-compute all quad vertices into the dynamic buffer ---
        let mut quad_idx: usize = 0;

        // Build a draw list: (quad_index, bind_group reference)
        // We'll store the info we need, then issue draws in the render pass.

        // Entity quads
        struct DrawCmd {
            quad_index: usize,
            texture_entity_id: Option<String>, // entity ID or special UI element
            is_button: bool,
            is_edit_bar: bool,
            is_selection: bool,
        }

        let mut draws: Vec<DrawCmd> = Vec::with_capacity(entities.len() + 4);

        for entity in entities {
            if quad_idx >= MAX_QUADS - 3 {
                // Reserve 3 quads for UI (edit bar + button + selection)
                log::warn!("MAX_QUADS ({}) reached, skipping remaining entities", MAX_QUADS);
                break;
            }

            if let Some(gpu_tex) = self.textures.get(&entity.id) {
                let width = gpu_tex.width as f32 * entity.scale;
                let height = gpu_tex.height as f32 * entity.scale;

                self.write_quad(quad_idx, entity.x, entity.y, width, height, entity.opacity);
                draws.push(DrawCmd {
                    quad_index: quad_idx,
                    texture_entity_id: Some(entity.id.clone()),
                    is_button: false,
                    is_edit_bar: false,
                    is_selection: false,
                });
                quad_idx += 1;

                // Selection highlight overlay (drawn right after the selected entity)
                if let Some(sel_id) = selected_entity_id {
                    if entity.id == sel_id && edit_mode {
                        let pad = 6.0; // padding around entity
                        self.write_quad(
                            quad_idx,
                            entity.x - pad,
                            entity.y - pad,
                            width + pad * 2.0,
                            height + pad * 2.0,
                            0.9,
                        );
                        draws.push(DrawCmd {
                            quad_index: quad_idx,
                            texture_entity_id: None,
                            is_button: false,
                            is_edit_bar: false,
                            is_selection: true,
                        });
                        quad_idx += 1;
                    }
                }
            }
        }

        // Edit mode indicator bar
        if edit_mode {
            self.write_quad(
                quad_idx,
                0.0,
                0.0,
                self.window_width as f32,
                4.0,
                1.0,
            );
            draws.push(DrawCmd {
                quad_index: quad_idx,
                texture_entity_id: None,
                is_button: false,
                is_edit_bar: true,
                is_selection: false,
            });
            quad_idx += 1;
        }

        // Toggle button
        let btn_size = TOGGLE_BUTTON_SIZE as f32;
        let btn_x = self.window_width as f32 - btn_size;
        self.write_quad(quad_idx, btn_x, 0.0, btn_size, btn_size, 1.0);
        draws.push(DrawCmd {
            quad_index: quad_idx,
            texture_entity_id: None,
            is_button: true,
            is_edit_bar: false,
            is_selection: false,
        });
        // quad_idx += 1; // last one, no need to increment

        // --- Render pass ---
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Sprite Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Clear to fully transparent
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            // Issue draw calls from the pre-computed list
            for cmd in &draws {
                // Bind the right texture
                if cmd.is_button {
                    let btn_tex = if edit_mode {
                        &self.button_tex_active
                    } else {
                        &self.button_tex_normal
                    };
                    render_pass.set_bind_group(1, &btn_tex.bind_group, &[]);
                } else if cmd.is_edit_bar {
                    render_pass.set_bind_group(1, &self.edit_bar_tex.bind_group, &[]);
                } else if cmd.is_selection {
                    render_pass.set_bind_group(1, &self.selection_tex.bind_group, &[]);
                } else if let Some(ref entity_id) = cmd.texture_entity_id {
                    if let Some(gpu_tex) = self.textures.get(entity_id) {
                        render_pass.set_bind_group(1, &gpu_tex.bind_group, &[]);
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }

                // Set the vertex buffer slice for this quad
                let byte_offset =
                    (cmd.quad_index * 4 * std::mem::size_of::<SpriteVertex>()) as u64;
                let byte_end = byte_offset + (4 * std::mem::size_of::<SpriteVertex>()) as u64;
                render_pass
                    .set_vertex_buffer(0, self.dynamic_vertex_buffer.slice(byte_offset..byte_end));
                render_pass.draw_indexed(0..6, 0, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
