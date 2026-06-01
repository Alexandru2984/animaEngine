use super::frame::Frame;
use crate::error::{AnimaError, Result};
use image::codecs::webp::WebPDecoder;
use image::AnimationDecoder;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Load frames from an animated WebP file.
/// Falls back to loading as a static image if animation decoding fails.
pub fn load_webp(path: &Path) -> Result<Vec<Frame>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let decoder = WebPDecoder::new(reader)?;

    // Check if it has animation
    if decoder.has_animation() {
        let webp_frames = decoder.into_frames();
        let mut frames = Vec::new();

        for frame_result in webp_frames {
            match frame_result {
                Ok(frame) => {
                    let (numerator, denominator) = frame.delay().numer_denom_ms();
                    let delay_ms = numerator.checked_div(denominator).unwrap_or(100);

                    let rgba_image = frame.into_buffer();
                    let (width, height) = rgba_image.dimensions();
                    let rgba = rgba_image.into_raw();

                    if delay_ms > 0 {
                        frames.push(Frame::with_delay(rgba, width, height, delay_ms));
                    } else {
                        frames.push(Frame::new(rgba, width, height));
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to decode WebP frame: {}", e);
                }
            }
        }

        if frames.is_empty() {
            return Err(AnimaError::EmptyAsset(path.to_path_buf()));
        }

        let has_delays = frames.iter().any(|f| f.delay_ms.is_some());
        tracing::info!(
            "Loaded animated WebP: {} frames from {} (per-frame delays: {})",
            frames.len(),
            path.display(),
            if has_delays { "yes" } else { "no" }
        );
        Ok(frames)
    } else {
        // Static WebP — load as a single frame
        load_static_webp(path)
    }
}

/// Load a static (non-animated) WebP as a single frame.
pub fn load_static_webp(path: &Path) -> Result<Vec<Frame>> {
    let img = image::open(path)?.to_rgba8();
    let (width, height) = img.dimensions();
    let rgba = img.into_raw();
    tracing::info!(
        "Loaded static WebP: {}x{} from {}",
        width,
        height,
        path.display()
    );
    Ok(vec![Frame::new(rgba, width, height)])
}
