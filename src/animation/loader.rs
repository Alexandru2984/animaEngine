use super::frame::Frame;
use super::gif_loader;
use super::png_sequence;
use super::spritesheet;
use super::webp_loader;
use crate::config::AssetType;
use crate::constants::MAX_IMAGE_DIM;
use crate::error::{AnimaError, Result};
use std::path::Path;

/// Validate image dimensions by reading only the file header (no full decode).
/// Returns an error if either dimension exceeds `MAX_IMAGE_DIM`.
pub fn validate_image_dimensions(path: &Path) -> Result<(u32, u32)> {
    if path.is_dir() {
        return Ok((0, 0)); // Directories are validated per-frame
    }
    if !path.exists() {
        return Err(AnimaError::AssetNotFound(path.to_path_buf()));
    }

    match image::image_dimensions(path) {
        Ok((w, h)) => {
            if w > MAX_IMAGE_DIM || h > MAX_IMAGE_DIM {
                Err(AnimaError::ImageTooLarge {
                    width: w,
                    height: h,
                    max: MAX_IMAGE_DIM,
                })
            } else {
                log::debug!("Image dimensions OK: {}×{}", w, h);
                Ok((w, h))
            }
        }
        Err(e) => {
            // Can't read dimensions (might be a format we don't recognize at header level)
            // Allow loading — the image crate will fail later if truly invalid
            log::debug!(
                "Could not read image dimensions for {}: {}",
                path.display(),
                e
            );
            Ok((0, 0))
        }
    }
}

/// Load animation frames based on asset type and path.
/// Returns a Vec of Frame on success.
pub fn load_asset(
    asset_type: &AssetType,
    asset_path: &Path,
    spritesheet_columns: Option<u32>,
    spritesheet_rows: Option<u32>,
) -> Result<Vec<Frame>> {
    // Validate dimensions for file-based assets (not directories)
    if !asset_path.is_dir() {
        validate_image_dimensions(asset_path)?;
    }

    match asset_type {
        AssetType::PngSequence => {
            log::info!("Loading PNG sequence from: {}", asset_path.display());
            png_sequence::load_png_sequence(asset_path)
        }
        AssetType::PngStatic => {
            log::info!("Loading static image from: {}", asset_path.display());
            let frame = png_sequence::load_single_png(asset_path)?;
            Ok(vec![frame])
        }
        AssetType::Gif => {
            log::info!("Loading GIF from: {}", asset_path.display());
            gif_loader::load_gif(asset_path)
        }
        AssetType::WebpAnimated => {
            log::info!("Loading animated WebP from: {}", asset_path.display());
            webp_loader::load_webp(asset_path)
        }
        AssetType::WebpStatic => {
            log::info!("Loading static WebP from: {}", asset_path.display());
            webp_loader::load_static_webp(asset_path)
        }
        AssetType::Spritesheet => {
            let cols = spritesheet_columns.unwrap_or(4);
            let rows = spritesheet_rows.unwrap_or(1);
            log::info!(
                "Loading spritesheet from: {} ({}x{} grid)",
                asset_path.display(),
                cols,
                rows
            );
            spritesheet::load_spritesheet(asset_path, cols, rows)
        }
    }
}

/// Detect the best AssetType from a file path's extension and properties.
/// Returns the detected type and a human-readable description.
pub fn detect_asset_type(path: &Path) -> (AssetType, &'static str) {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "gif" => (AssetType::Gif, "GIF animation"),
        "webp" => (
            AssetType::WebpAnimated,
            "WebP (auto-detect animated/static)",
        ),
        "png" => {
            // If it's a directory, treat as PNG sequence
            if path.is_dir() {
                (AssetType::PngSequence, "PNG sequence (directory)")
            } else {
                (AssetType::PngStatic, "Static PNG")
            }
        }
        "jpg" | "jpeg" => (AssetType::PngStatic, "JPEG image"),
        _ => {
            // Check if it's a directory (PNG sequence)
            if path.is_dir() {
                (AssetType::PngSequence, "PNG sequence (directory)")
            } else {
                // Default to static PNG for unknown extensions
                (
                    AssetType::PngStatic,
                    "Unknown format (trying as static image)",
                )
            }
        }
    }
}

/// Generate a fallback frame — a simple colored rectangle
/// Used when asset loading fails
pub fn generate_fallback_frame(color: [u8; 4], size: u32) -> Frame {
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            // Create a simple shape with rounded corners
            let cx = size as f32 / 2.0;
            let cy = size as f32 / 2.0;
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let radius = size as f32 * 0.4;

            if dist < radius {
                rgba.extend_from_slice(&color);
            } else if dist < radius + 2.0 {
                // Anti-aliased edge
                let alpha = ((radius + 2.0 - dist) / 2.0 * color[3] as f32) as u8;
                rgba.extend_from_slice(&[color[0], color[1], color[2], alpha]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    Frame::new(rgba, size, size)
}
