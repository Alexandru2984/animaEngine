use super::frame::Frame;
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

    tracing::info!(
        "Loading PNG sequence: {} frames from {} (parallel decode)",
        entries.len(),
        dir_path.display()
    );

    let frames: Vec<Frame> = entries
        .par_iter()
        .filter_map(|path| match load_single_png(path) {
            Ok(frame) => Some(frame),
            Err(e) => {
                tracing::warn!("Failed to load PNG {}: {}", path.display(), e);
                None
            }
        })
        .collect();

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
