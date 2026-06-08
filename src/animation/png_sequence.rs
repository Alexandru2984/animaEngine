use super::frame::Frame;
use crate::constants::{MAX_DECODED_ASSET_BYTES, MAX_SEQUENCE_FILES};
use crate::error::{AnimaError, Result};
use rayon::prelude::*;
use std::fs;
use std::path::Path;

/// Load a PNG sequence from a directory.
/// Files are sorted alphabetically — name them frame_001.png, frame_002.png, etc.
///
/// Frames are decoded in parallel via rayon while preserving the sorted order
/// (par_iter().map().collect() guarantees source-order output). A single
/// corrupt PNG is logged and skipped; the whole load only fails if every
/// frame fails.
///
/// Two safety caps:
/// - `MAX_SEQUENCE_FILES` limits the enumeration so a directory with 50 k
///   files doesn't pin the CPU before we even start decoding.
/// - `MAX_DECODED_ASSET_BYTES` truncates after decode so the same cap
///   used by GIF/WebP applies here too.
pub fn load_png_sequence(dir_path: &Path) -> Result<Vec<Frame>> {
    if !dir_path.is_dir() {
        return Err(AnimaError::NotADirectory(dir_path.to_path_buf()));
    }

    let mut entries: Vec<_> = fs::read_dir(dir_path)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("png") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    entries.sort();

    if entries.is_empty() {
        return Err(AnimaError::EmptyAsset(dir_path.to_path_buf()));
    }

    let truncated_for_file_count = entries.len() > MAX_SEQUENCE_FILES;
    if truncated_for_file_count {
        tracing::warn!(
            "PNG sequence {} has {} files; capping at MAX_SEQUENCE_FILES = {}",
            crate::drop_validate::redact_path(dir_path),
            entries.len(),
            MAX_SEQUENCE_FILES
        );
        entries.truncate(MAX_SEQUENCE_FILES);
    }

    tracing::info!(
        "Loading PNG sequence: {} frames from {} (parallel decode)",
        entries.len(),
        crate::drop_validate::redact_path(dir_path)
    );

    let decoded: Vec<Frame> = entries
        .par_iter()
        .filter_map(|path| match load_single_png(path) {
            Ok(frame) => Some(frame),
            Err(e) => {
                tracing::warn!(
                    "Failed to load PNG {}: {}",
                    crate::drop_validate::redact_path(path),
                    e
                );
                None
            }
        })
        .collect();

    // Apply the decoded-bytes cap sequentially after parallel decode.
    // Truncating during par_iter would race the running total.
    let mut frames: Vec<Frame> = Vec::with_capacity(decoded.len());
    let mut total_bytes: usize = 0;
    let mut truncated_for_bytes = false;
    for frame in decoded {
        if total_bytes.saturating_add(frame.rgba.len()) > MAX_DECODED_ASSET_BYTES {
            truncated_for_bytes = true;
            break;
        }
        total_bytes += frame.rgba.len();
        frames.push(frame);
    }
    if truncated_for_bytes {
        tracing::warn!(
            "PNG sequence {} truncated at MAX_DECODED_ASSET_BYTES = {} MB ({} frames kept)",
            crate::drop_validate::redact_path(dir_path),
            MAX_DECODED_ASSET_BYTES / (1024 * 1024),
            frames.len()
        );
    }

    if frames.is_empty() {
        return Err(AnimaError::EmptyAsset(dir_path.to_path_buf()));
    }

    Ok(frames)
}

/// Load a single static PNG file
pub fn load_single_png(path: &Path) -> Result<Frame> {
    let img = image::open(path)?.to_rgba8();
    let (width, height) = img.dimensions();
    let rgba = img.into_raw();
    Ok(Frame::new(rgba, width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("png_seq_tests")
            .join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Build a PNG with a unique solid color so we can verify frame ordering.
    fn write_colored_png(path: &Path, w: u32, h: u32, color: [u8; 4]) {
        let mut img = image::RgbaImage::new(w, h);
        for px in img.pixels_mut() {
            *px = image::Rgba(color);
        }
        img.save(path).unwrap();
    }

    #[test]
    fn caps_file_count_at_max_sequence_files() {
        // Synthesize MAX_SEQUENCE_FILES + 50 tiny PNGs and verify the
        // loader truncates instead of trying to decode all of them.
        let dir = temp_dir("oversized_seq");
        let total = crate::constants::MAX_SEQUENCE_FILES + 50;
        for i in 1..=total {
            let path = dir.join(format!("frame_{i:06}.png"));
            // Tiny 1×1 PNGs so the test runs fast.
            write_colored_png(&path, 1, 1, [255, 0, 0, 255]);
        }
        let frames = load_png_sequence(&dir).expect("load");
        assert!(frames.len() <= crate::constants::MAX_SEQUENCE_FILES);
    }

    #[test]
    fn parallel_decode_preserves_filename_order() {
        let dir = temp_dir("ordered_seq");
        // Each frame has a distinctive red channel matching its index.
        // After parallel decode the colors must come back in 0..N order.
        let n = 12u8;
        for i in 1..=n {
            let path = dir.join(format!("frame_{i:03}.png"));
            // Use distinct red values so we can detect any reordering.
            write_colored_png(&path, 2, 2, [i * 10, 0, 0, 255]);
        }

        let frames = load_png_sequence(&dir).expect("decode");
        assert_eq!(frames.len(), n as usize);
        for (i, frame) in frames.iter().enumerate() {
            // Read the red channel of pixel (0, 0).
            assert_eq!(
                frame.rgba[0],
                ((i + 1) as u8) * 10,
                "frame {i} arrived out of order"
            );
        }
    }
}
