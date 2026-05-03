use super::frame::Frame;
use super::gif_loader;
use super::png_sequence;
use crate::config::AssetType;
use std::path::Path;

/// Load animation frames based on asset type and path.
/// Returns a Vec of Frame on success.
pub fn load_asset(
    asset_type: &AssetType,
    asset_path: &Path,
) -> Result<Vec<Frame>, Box<dyn std::error::Error>> {
    match asset_type {
        AssetType::PngSequence => {
            log::info!("Loading PNG sequence from: {}", asset_path.display());
            png_sequence::load_png_sequence(asset_path)
        }
        AssetType::PngStatic => {
            log::info!("Loading static PNG from: {}", asset_path.display());
            let frame = png_sequence::load_single_png(asset_path)?;
            Ok(vec![frame])
        }
        AssetType::Gif => {
            log::info!("Loading GIF from: {}", asset_path.display());
            gif_loader::load_gif(asset_path)
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
