use crate::error::{AnimaError, Result};

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

    /// Resize this frame so neither dimension exceeds `max_dim`. Preserves
    /// aspect ratio. Returns `self` unchanged if already small enough.
    ///
    /// Errors with `FrameBufferCorrupt` if the RGBA buffer length does not
    /// match `width * height * 4` — we refuse to fabricate a fake frame at
    /// the wrong dimensions, which would later desync the GPU texture.
    pub fn resized(self, max_dim: u32) -> Result<Self> {
        if self.width <= max_dim && self.height <= max_dim {
            return Ok(self);
        }

        let orig_w = self.width;
        let orig_h = self.height;
        let delay = self.delay_ms;
        let expected_len = (orig_w as usize) * (orig_h as usize) * 4;

        let scale = if orig_w >= orig_h {
            max_dim as f32 / orig_w as f32
        } else {
            max_dim as f32 / orig_h as f32
        };

        // Clamp each side to at least 1px. An extreme aspect ratio — e.g.
        // 4096×1 scaled to fit 256 — rounds the short side to 0, producing a
        // zero-height image whose empty buffer later trips wgpu texture
        // validation (a texture dimension must be ≥ 1). 1px is the correct
        // degenerate result.
        let new_w = ((orig_w as f32 * scale).round() as u32).max(1);
        let new_h = ((orig_h as f32 * scale).round() as u32).max(1);

        let got_len = self.rgba.len();
        let img = image::RgbaImage::from_raw(orig_w, orig_h, self.rgba).ok_or(
            AnimaError::FrameBufferCorrupt {
                expected: expected_len,
                got: got_len,
            },
        )?;

        let resized =
            image::imageops::resize(&img, new_w, new_h, image::imageops::FilterType::Triangle);
        tracing::info!("Resized frame: {}x{} → {}x{}", orig_w, orig_h, new_w, new_h);

        Ok(Self {
            rgba: resized.into_raw(),
            width: new_w,
            height: new_h,
            delay_ms: delay,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smaller_than_max_returns_unchanged() {
        let f = Frame::new(vec![0; 4 * 4 * 4], 4, 4);
        let resized = f.resized(256).expect("ok");
        assert_eq!((resized.width, resized.height), (4, 4));
    }

    #[test]
    fn resize_preserves_aspect_ratio() {
        // 200×100 source, max 100 → should become 100×50.
        let rgba = vec![128u8; (200 * 100 * 4) as usize];
        let f = Frame::new(rgba, 200, 100);
        let resized = f.resized(100).expect("ok");
        assert_eq!(resized.width, 100);
        assert_eq!(resized.height, 50);
    }

    #[test]
    fn corrupted_buffer_errors_instead_of_lying() {
        // Buffer too small: claim 4×4 but provide only 8 bytes.
        let bad = Frame::new(vec![0u8; 8], 4, 4);
        // Must trigger resize path (otherwise it'd short-circuit).
        let err = bad.resized(2).unwrap_err();
        match err {
            AnimaError::FrameBufferCorrupt { expected, got } => {
                assert_eq!(expected, 4 * 4 * 4);
                assert_eq!(got, 8);
            }
            other => panic!("expected FrameBufferCorrupt, got {other:?}"),
        }
    }

    #[test]
    fn extreme_aspect_ratio_clamps_short_side_to_one() {
        // 4096×1 fit to 256: the long side scales to 256, the short side
        // rounds to 0 without the clamp. A zero dimension would later trip
        // wgpu texture validation; 1px is the correct degenerate result.
        let rgba = vec![0u8; 4096 * 4];
        let f = Frame::new(rgba, 4096, 1);
        let resized = f.resized(256).expect("ok");
        assert_eq!(resized.width, 256);
        assert_eq!(resized.height, 1);
        assert_eq!(
            resized.rgba.len(),
            (resized.width * resized.height * 4) as usize
        );
    }

    #[test]
    fn delay_ms_is_preserved_across_resize() {
        let rgba = vec![0u8; (200 * 200 * 4) as usize];
        let f = Frame::with_delay(rgba, 200, 200, 250);
        let resized = f.resized(100).expect("ok");
        assert_eq!(resized.delay_ms, Some(250));
    }
}
