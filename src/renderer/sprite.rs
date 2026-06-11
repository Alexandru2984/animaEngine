use bytemuck::{Pod, Zeroable};

/// Vertex data for a sprite quad
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SpriteVertex {
    pub position: [f32; 2],
    pub tex_coord: [f32; 2],
    pub color: [f32; 4],
}

impl SpriteVertex {
    /// Vertex buffer layout for the render pipeline
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SpriteVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // tex_coord
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // color
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// Generate vertices for a sprite quad at the given position, size, and opacity.
/// Position is in screen pixels (origin top-left).
pub fn make_quad_vertices(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    opacity: f32,
    flip_x: bool,
) -> [SpriteVertex; 4] {
    let color = [1.0, 1.0, 1.0, opacity];
    // Horizontal mirror = swap the U texture coordinates (U.2). No
    // texture duplication, no extra pipeline state.
    let (u_left, u_right) = if flip_x { (1.0, 0.0) } else { (0.0, 1.0) };

    [
        // Top-left
        SpriteVertex {
            position: [x, y],
            tex_coord: [u_left, 0.0],
            color,
        },
        // Top-right
        SpriteVertex {
            position: [x + width, y],
            tex_coord: [u_right, 0.0],
            color,
        },
        // Bottom-left
        SpriteVertex {
            position: [x, y + height],
            tex_coord: [u_left, 1.0],
            color,
        },
        // Bottom-right
        SpriteVertex {
            position: [x + width, y + height],
            tex_coord: [u_right, 1.0],
            color,
        },
    ]
}

/// Standard quad indices (two triangles)
pub const QUAD_INDICES: [u16; 6] = [0, 1, 2, 1, 3, 2];

/// Orthographic projection matrix for screen-space rendering.
/// Maps pixel coordinates to clip space [-1, 1].
pub fn orthographic_projection(width: f32, height: f32) -> [[f32; 4]; 4] {
    [
        [2.0 / width, 0.0, 0.0, 0.0],
        [0.0, -2.0 / height, 0.0, 0.0], // Flip Y: screen Y goes down
        [0.0, 0.0, 1.0, 0.0],
        [-1.0, 1.0, 0.0, 1.0],
    ]
}
