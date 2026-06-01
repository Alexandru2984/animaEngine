use super::frame::Frame;
use crate::error::{AnimaError, Result};
use std::fs;
use std::path::Path;

/// Load a PNG sequence from a directory.
/// Files are sorted alphabetically — name them frame_001.png, frame_002.png, etc.
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
        "Loading PNG sequence: {} frames from {}",
        entries.len(),
        dir_path.display()
    );

    let mut frames = Vec::new();
    for path in &entries {
        match load_single_png(path) {
            Ok(frame) => frames.push(frame),
            Err(e) => {
                tracing::warn!("Failed to load PNG {}: {}", path.display(), e);
            }
        }
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
