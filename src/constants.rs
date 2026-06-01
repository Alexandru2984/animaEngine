//! Cross-module constants. Module-local values stay in their modules.

/// Maximum image dimension (width or height) accepted by loaders.
/// A 4096×4096 RGBA image is 64 MB in RAM — generous for an overlay app
/// while still rejecting decompression bombs.
pub const MAX_IMAGE_DIM: u32 = 4096;

/// Maximum dimension (px) to which dropped assets are resized.
/// Keeps overlay-friendly sprites and bounds GPU texture allocations.
pub const MAX_DROP_SIZE: u32 = 256;

/// Hard cap on entities loaded from a config file. Prevents resource
/// exhaustion from a malicious or runaway config.
pub const MAX_ENTITIES: usize = 64;

/// Maximum number of quads the renderer can batch in one frame
/// (entities + UI elements like the toggle button, edit bar, selection).
pub const MAX_QUADS: usize = 64;

/// Size (px) of the clickable toggle button in the top-right corner.
/// In pass-through mode this is the only area that receives mouse input.
pub const TOGGLE_BUTTON_SIZE: u32 = 64;

/// Maximum number of frames extracted from a video. ~20 seconds at 30 fps.
/// Caps memory at roughly MAX_VIDEO_FRAMES × MAX_DROP_SIZE² × 4 bytes
/// (≈150 MB for a 256-px square, much less for typical overlay sprites).
pub const MAX_VIDEO_FRAMES: usize = 600;
