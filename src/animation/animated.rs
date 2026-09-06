//! Shared frame collection for the animated-image decoders.
//!
//! GIF and animated WebP both arrive as an `image::Frames` iterator from
//! `AnimationDecoder`, and both need the identical caps applied while
//! draining it. The two loaders carried byte-identical copies of this
//! logic, differing only in the format name inside their log lines — so
//! `MAX_ANIMATION_FRAMES` and `MAX_DECODED_ASSET_BYTES` were enforced
//! twice and could silently drift apart if only one copy were updated.

use super::frame::Frame;
use crate::constants::{MAX_ANIMATION_FRAMES, MAX_DECODED_ASSET_BYTES};
use crate::drop_validate::redact_path;
use crate::error::{AnimaError, Result};
use std::path::Path;

/// Drain `frames` into [`Frame`]s, enforcing the frame-count and
/// decoded-byte caps, and report truncation against `format` (`"GIF"`,
/// `"animated WebP"`, …) and `path`.
///
/// Errors with [`AnimaError::EmptyAsset`] when nothing decoded — an
/// animation with zero usable frames is not something the renderer can
/// show, and silently returning an empty vec would surface later as a
/// blank sprite with no explanation.
pub(super) fn collect_frames(
    frames: image::Frames<'_>,
    format: &str,
    path: &Path,
) -> Result<Vec<Frame>> {
    let mut out: Vec<Frame> = Vec::new();
    let mut total_bytes: usize = 0;
    let mut truncated_for_count = false;
    let mut truncated_for_bytes = false;

    for frame_result in frames {
        if out.len() >= MAX_ANIMATION_FRAMES {
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
                    out.push(Frame::with_delay(rgba, width, height, delay_ms));
                } else {
                    out.push(Frame::new(rgba, width, height));
                }
            }
            Err(e) => {
                tracing::warn!("Failed to decode {format} frame: {e}");
            }
        }
    }

    if out.is_empty() {
        return Err(AnimaError::EmptyAsset(path.to_path_buf()));
    }

    if truncated_for_count {
        tracing::warn!(
            "{format} {} truncated at MAX_ANIMATION_FRAMES = {}",
            redact_path(path),
            MAX_ANIMATION_FRAMES
        );
    }
    if truncated_for_bytes {
        tracing::warn!(
            "{format} {} truncated at MAX_DECODED_ASSET_BYTES = {} MB ({} frames kept)",
            redact_path(path),
            MAX_DECODED_ASSET_BYTES / (1024 * 1024),
            out.len()
        );
    }

    let has_delays = out.iter().any(|f| f.delay_ms.is_some());
    // Redacted, unlike the old GIF copy which logged `path.display()` at
    // info — asset filenames are user content and can carry control or
    // bidi characters (the same reason `redact_path` exists).
    tracing::info!(
        "Loaded {format}: {} frames from {} (per-frame delays: {})",
        out.len(),
        redact_path(path),
        if has_delays { "yes" } else { "no" }
    );
    Ok(out)
}
