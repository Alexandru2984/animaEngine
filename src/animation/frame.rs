/// Frame data — raw RGBA pixels for a single animation frame
#[derive(Debug, Clone)]
pub struct Frame {
    /// RGBA pixel data
    pub rgba: Vec<u8>,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Optional per-frame delay in milliseconds (from GIF/WebP metadata).
    /// When `Some`, the animation system uses this instead of the global FPS.
    pub delay_ms: Option<u32>,
}

impl Frame {
    pub fn new(rgba: Vec<u8>, width: u32, height: u32) -> Self {
        Self {
            rgba,
            width,
            height,
            delay_ms: None,
        }
    }

    /// Create a frame with an explicit delay (for GIF/animated WebP)
    pub fn with_delay(rgba: Vec<u8>, width: u32, height: u32, delay_ms: u32) -> Self {
        Self {
            rgba,
            width,
            height,
            delay_ms: Some(delay_ms),
        }
    }
}
