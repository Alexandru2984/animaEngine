use super::frame::Frame;
use std::fs;
use std::path::Path;

/// Load a PNG sequence from a directory.
/// Files are sorted alphabetically — name them frame_001.png, frame_002.png, etc.
pub fn load_png_sequence(dir_path: &Path) -> Result<Vec<Frame>, Box<dyn std::error::Error>> {
    if !dir_path.is_dir() {
        return Err(format!(
            "PNG sequence path is not a directory: {}",
            dir_path.display()
        )
        .into());
    }

    let mut entries: Vec<_> = fs::read_dir(dir_path)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            // Only accept .png files
            if path.extension().and_then(|e| e.to_str()) == Some("png") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    entries.sort();

    if entries.is_empty() {
        return Err(format!("No PNG files found in: {}", dir_path.display()).into());
    }

    log::info!(
        "Loading PNG sequence: {} frames from {}",
        entries.len(),
        dir_path.display()
    );

    let mut frames = Vec::new();
    for path in &entries {
        match load_single_png(path) {
            Ok(frame) => frames.push(frame),
            Err(e) => {
                log::warn!("Failed to load PNG {}: {}", path.display(), e);
            }
        }
    }

    if frames.is_empty() {
        return Err(format!("All PNG files failed to load in: {}", dir_path.display()).into());
    }

    Ok(frames)
}

/// Load a single static PNG file
pub fn load_single_png(path: &Path) -> Result<Frame, Box<dyn std::error::Error>> {
    let img = image::open(path)?.to_rgba8();
    let (width, height) = img.dimensions();
    let rgba = img.into_raw();
    Ok(Frame::new(rgba, width, height))
}
