//! Shared frame collection for the animated-image decoders.
//!
//! GIF and animated WebP both arrive as an `image::Frames` iterator from
//! `AnimationDecoder`, and both need the identical caps applied while
//! draining it. The two loaders carried byte-identical copies of this
//! logic, differing only in the format name inside their log lines — so
//! `MAX_ANIMATION_FRAMES` and `MAX_DECODED_ASSET_BYTES` were enforced
//! twice and could silently drift apart if only one copy were updated.

use super::frame::Frame;
use crate::constants::{MAX_ANIMATION_FRAMES, MAX_DECODED_ASSET_BYTES, MAX_IMAGE_DIM};
use crate::drop_validate::redact_path;
use crate::error::{AnimaError, Result};
use std::path::Path;

/// The limits a GIF or an animated WebP is decoded within: no frame over
/// `MAX_IMAGE_DIM` on a side, no buffer over `MAX_DECODED_ASSET_BYTES`.
/// The picture's size in its header is checked before
/// (`validate_image_dimensions`), but a GIF's frames carry sizes of their
/// own, and a decoder made with `new` has no limits: an 8×8 GIF whose one
/// frame said 46003×46003 had 8.4 GB asked for at once (found by the
/// `content_sniff` fuzz target). `image` checks these before it allocates.
pub(super) fn limits() -> image::Limits {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIM);
    limits.max_image_height = Some(MAX_IMAGE_DIM);
    limits.max_alloc = Some(MAX_DECODED_ASSET_BYTES as u64);
    limits
}

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
    let mut failed: Option<image::ImageError> = None;

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
                // The frames before it are kept; nothing after. `image`'s
                // GIF frames do not end at an error: a GIF cut short
                // answered every next frame with the same error, and this
                // loop — counting only the frames it kept — logged it as
                // fast as it could, for good. Fifty-five bytes froze a
                // drop and filled the log (found by `content_sniff`).
                tracing::warn!("Failed to decode {format} frame: {e}");
                failed = Some(e);
                break;
            }
        }
    }

    if out.is_empty() {
        // Not a picture at all, as a damaged one is said to be — rather
        // than one with none in it.
        return Err(match failed {
            Some(e) => AnimaError::ImageDecode(e),
            None => AnimaError::EmptyAsset(path.to_path_buf()),
        });
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
