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

    /// Resize this frame so that neither dimension exceeds `max_dim` pixels.
    /// Preserves aspect ratio. Returns a new Frame if resized, or self if already small enough.
    pub fn resized(self, max_dim: u32) -> Self {
        if self.width <= max_dim && self.height <= max_dim {
            return self;
        }

        let orig_w = self.width;
        let orig_h = self.height;
        let delay = self.delay_ms;

        let scale = if orig_w >= orig_h {
            max_dim as f32 / orig_w as f32
        } else {
            max_dim as f32 / orig_h as f32
        };

        let new_w = (orig_w as f32 * scale).round() as u32;
        let new_h = (orig_h as f32 * scale).round() as u32;

        // Use the image crate to resize
        if let Some(img) = image::RgbaImage::from_raw(orig_w, orig_h, self.rgba) {
            let resized = image::imageops::resize(
                &img,
                new_w,
                new_h,
                image::imageops::FilterType::Triangle,
            );
            log::info!(
                "Resized frame: {}x{} → {}x{}",
                orig_w, orig_h, new_w, new_h
            );
            Self {
                rgba: resized.into_raw(),
                width: new_w,
                height: new_h,
                delay_ms: delay,
            }
        } else {
            // Fallback: reconstruct original (shouldn't happen)
            Self {
                rgba: vec![0; (orig_w * orig_h * 4) as usize],
                width: orig_w,
                height: orig_h,
                delay_ms: delay,
            }
        }
    }
}

