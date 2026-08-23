use super::frame::Frame;
use crate::constants::{MAX_ANIMATION_FRAMES, MAX_DECODED_ASSET_BYTES};
use crate::error::{AnimaError, Result};
use image::codecs::gif::GifDecoder;
use image::AnimationDecoder;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Load frames from a GIF file with per-frame delay extraction.
/// Each frame carries its own delay in milliseconds from the GIF metadata.
///
/// Truncates to `MAX_ANIMATION_FRAMES` and to `MAX_DECODED_ASSET_BYTES`
/// of total decoded RGBA — a multi-hour pathological GIF won't OOM us.
pub fn load_gif(path: &Path) -> Result<Vec<Frame>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let decoder = GifDecoder::new(reader)?;
    let gif_frames = decoder.into_frames();

    let mut frames: Vec<Frame> = Vec::new();
    let mut total_bytes: usize = 0;
    let mut truncated_for_count = false;
    let mut truncated_for_bytes = false;

    for frame_result in gif_frames {
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

                // Refuse the frame if it would push us past the byte cap.
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
                tracing::warn!("Failed to decode GIF frame: {}", e);
            }
        }
    }

    if frames.is_empty() {
        return Err(AnimaError::EmptyAsset(path.to_path_buf()));
    }

    if truncated_for_count {
        tracing::warn!(
            "GIF {} truncated at MAX_ANIMATION_FRAMES = {}",
            crate::drop_validate::redact_path(path),
            MAX_ANIMATION_FRAMES
        );
    }
    if truncated_for_bytes {
        tracing::warn!(
            "GIF {} truncated at MAX_DECODED_ASSET_BYTES = {} MB ({} frames kept)",
            crate::drop_validate::redact_path(path),
            MAX_DECODED_ASSET_BYTES / (1024 * 1024),
            frames.len()
        );
    }

    let has_delays = frames.iter().any(|f| f.delay_ms.is_some());
    tracing::info!(
        "Loaded GIF: {} frames from {} (per-frame delays: {})",
        frames.len(),
        path.display(),
        if has_delays { "yes" } else { "no" }
    );
    Ok(frames)
}
