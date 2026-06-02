use super::frame::Frame;
use crate::constants::{MAX_ANIMATION_FRAMES, MAX_DECODED_ASSET_BYTES};
use crate::error::{AnimaError, Result};
use image::codecs::webp::WebPDecoder;
use image::AnimationDecoder;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Load frames from an animated WebP file.
/// Falls back to loading as a static image if animation decoding fails.
/// Same caps as GIF: `MAX_ANIMATION_FRAMES` and `MAX_DECODED_ASSET_BYTES`.
pub fn load_webp(path: &Path) -> Result<Vec<Frame>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let decoder = WebPDecoder::new(reader)?;

    if decoder.has_animation() {
        let webp_frames = decoder.into_frames();
        let mut frames: Vec<Frame> = Vec::new();
        let mut total_bytes: usize = 0;
        let mut truncated_for_count = false;
        let mut truncated_for_bytes = false;

        for frame_result in webp_frames {
            if frames.len() >= MAX_ANIMATION_FRAMES {
                truncated_for_count = true;
                break;
            }
            match frame_result {
                Ok(frame) => {
                    let (numerator, denominator) = frame.delay().numer_denom_ms();
                    let delay_ms = numerator.checked_div(denominator).unwrap_or(100);

                    let rgba_image = frame.into_buffer();
                    let (width, height) = rgba_image.dimensions();
                    let rgba = rgba_image.into_raw();

                    if total_bytes.saturating_add(rgba.len()) > MAX_DECODED_ASSET_BYTES {
                        truncated_for_bytes = true;
                        break;
                    }
                    total_bytes += rgba.len();

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

        if truncated_for_count {
            tracing::warn!(
                "WebP {} truncated at MAX_ANIMATION_FRAMES = {}",
                path.display(),
                MAX_ANIMATION_FRAMES
            );
        }
        if truncated_for_bytes {
            tracing::warn!(
                "WebP {} truncated at MAX_DECODED_ASSET_BYTES = {} MB ({} frames kept)",
                path.display(),
                MAX_DECODED_ASSET_BYTES / (1024 * 1024),
                frames.len()
            );
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
