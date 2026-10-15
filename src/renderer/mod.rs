// The Vulkan driver of a laptop's NVIDIA GPU, left unloaded where an
// integrated GPU draws: Linux's DRM devices tell which there are.
#[cfg(target_os = "linux")]
pub mod hybrid;
pub mod sprite;
pub mod texture;
pub mod wgpu_renderer;
// Windows presents through UpdateLayeredWindow rather than a swapchain —
// see the module docs for why a swapchain can't be transparent there.
#[cfg(windows)]
pub mod win_layered;
