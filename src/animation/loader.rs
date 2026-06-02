use super::cache;
use super::frame::Frame;
use super::gif_loader;
use super::png_sequence;
use super::spritesheet;
use super::video_loader;
use super::webp_loader;
use crate::config::AssetType;
use crate::constants::MAX_IMAGE_DIM;
use crate::error::{AnimaError, Result};
use std::fs;
use std::path::Path;

/// Validate that loading the asset at `path` won't blow up memory.
///
/// - For a single file, checks the header dimensions against `MAX_IMAGE_DIM`.
/// - For a directory (PNG sequence), validates **every** `.png` file inside.
///   The first oversized frame triggers an `ImageTooLarge` error so we never
///   start decoding a 64-frame sequence only to OOM on frame 32.
///
/// Returns the dimensions of the file (or `(0, 0)` for directories — the
/// caller doesn't use the value for sequences anyway).
pub fn validate_image_dimensions(path: &Path) -> Result<(u32, u32)> {
    if path.is_dir() {
        validate_directory(path)?;
        return Ok((0, 0));
    }
    validate_single_file(path)
}

/// Extensions we know how to validate via the `image` crate's
/// header-only probe. For these we fail closed if the probe fails.
const IMAGE_PROBE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp"];

/// Header-only dimension check for a single image file.
///
/// **Fail-closed for known image extensions** (PNG / JPEG / GIF / WebP):
/// if the header can't be read, return `Err` instead of letting the
/// real decoder paper over it later. For other extensions (e.g. video,
/// where `image_dimensions` doesn't apply) we return `(0, 0)` so the
/// format-specific loader can do its own checks.
fn validate_single_file(path: &Path) -> Result<(u32, u32)> {
    if !path.exists() {
        return Err(AnimaError::AssetNotFound(path.to_path_buf()));
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase());
    let is_image_ext = ext
        .as_deref()
        .map(|e| IMAGE_PROBE_EXTENSIONS.contains(&e))
        .unwrap_or(false);

    match image::image_dimensions(path) {
        Ok((w, h)) => {
            if w > MAX_IMAGE_DIM || h > MAX_IMAGE_DIM {
                Err(AnimaError::ImageTooLarge {
                    width: w,
                    height: h,
                    max: MAX_IMAGE_DIM,
                })
            } else {
                tracing::debug!("Image dimensions OK: {}×{}", w, h);
                Ok((w, h))
            }
        }
        Err(e) if is_image_ext => {
            // Known image extension but the header is unreadable —
            // refuse instead of trusting the decoder to fail later.
            Err(AnimaError::other(format!(
                "Unreadable {ext:?} header at {}: {e}",
                path.display()
            )))
        }
        Err(e) => {
            // Unknown extension (video, …) — defer to the format-specific
            // loader.
            tracing::debug!(
                "Could not read image dimensions for {} (deferring): {}",
                path.display(),
                e
            );
            Ok((0, 0))
        }
    }
}

/// Validate every `.png` inside a sequence directory before any decode.
fn validate_directory(dir: &Path) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("png") {
            validate_single_file(&path)?;
        }
    }
    Ok(())
}

/// Load animation frames based on asset type and path.
/// Returns a Vec of Frame on success.
#[tracing::instrument(skip(spritesheet_columns, spritesheet_rows), fields(path = %asset_path.display()))]
pub fn load_asset(
    asset_type: &AssetType,
    asset_path: &Path,
    spritesheet_columns: Option<u32>,
    spritesheet_rows: Option<u32>,
) -> Result<Vec<Frame>> {
    // Reject decompression bombs up-front (works for files AND directories).
    validate_image_dimensions(asset_path)?;

    // Try the on-disk RGBA cache first — skips PNG/GIF/WebP decoding
    // entirely when the asset hasn't changed since last run.
    if let Some(frames) = cache::try_load(asset_path) {
        tracing::info!(
            "Asset cache hit ({}): {} frames",
            asset_path.display(),
            frames.len()
        );
        return Ok(frames);
    }

    let frames = match asset_type {
        AssetType::PngSequence => {
            tracing::info!("Loading PNG sequence from: {}", asset_path.display());
            png_sequence::load_png_sequence(asset_path)?
        }
        AssetType::PngStatic => {
            tracing::info!("Loading static image from: {}", asset_path.display());
            vec![png_sequence::load_single_png(asset_path)?]
        }
        AssetType::Gif => {
            tracing::info!("Loading GIF from: {}", asset_path.display());
            gif_loader::load_gif(asset_path)?
        }
        AssetType::WebpAnimated => {
            tracing::info!("Loading animated WebP from: {}", asset_path.display());
            webp_loader::load_webp(asset_path)?
        }
        AssetType::WebpStatic => {
            tracing::info!("Loading static WebP from: {}", asset_path.display());
            webp_loader::load_static_webp(asset_path)?
        }
        AssetType::Spritesheet => {
            let cols = spritesheet_columns.unwrap_or(4);
            let rows = spritesheet_rows.unwrap_or(1);
            tracing::info!(
                "Loading spritesheet from: {} ({}x{} grid)",
                asset_path.display(),
                cols,
                rows
            );
            spritesheet::load_spritesheet(asset_path, cols, rows)?
        }
        AssetType::Video => {
            tracing::info!("Loading MP4 video from: {}", asset_path.display());
            video_loader::load_video(asset_path)?
        }
    };

    // Best-effort cache write — never fails the load.
    if let Err(e) = cache::try_save(asset_path, &frames) {
        tracing::warn!(
            "Failed to write asset cache for {}: {}",
            asset_path.display(),
            e
        );
    }

    Ok(frames)
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
        "mp4" | "m4v" | "mov" => (AssetType::Video, "MP4 / H.264 video"),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a temp dir under `target/` so we don't pollute the workspace.
    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("validate_tests")
            .join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_png(path: &Path, w: u32, h: u32) {
        let img = image::RgbaImage::new(w, h);
        img.save(path).unwrap();
    }

    #[test]
    fn validate_dir_with_safe_pngs_succeeds() {
        let dir = temp_dir("safe_seq");
        for i in 1..=3 {
            write_png(&dir.join(format!("frame_{i:03}.png")), 32, 32);
        }
        assert!(validate_image_dimensions(&dir).is_ok());
    }

    #[test]
    fn validate_dir_rejects_oversized_frame() {
        let dir = temp_dir("oversized_seq");
        // One safe frame, then an oversized one. Validation must catch the
        // big one regardless of file ordering inside the dir.
        write_png(&dir.join("frame_001.png"), 32, 32);
        write_png(&dir.join("frame_002.png"), MAX_IMAGE_DIM + 100, 32);

        let err = validate_image_dimensions(&dir).unwrap_err();
        match err {
            AnimaError::ImageTooLarge { width, .. } => {
                assert!(width > MAX_IMAGE_DIM);
            }
            other => panic!("expected ImageTooLarge, got {other:?}"),
        }
    }

    #[test]
    fn validate_dir_ignores_non_png_files() {
        let dir = temp_dir("mixed_seq");
        write_png(&dir.join("frame_001.png"), 32, 32);
        // A README that would fail dimension probing — must be ignored.
        std::fs::write(dir.join("README.txt"), b"not an image").unwrap();
        assert!(validate_image_dimensions(&dir).is_ok());
    }

    #[test]
    fn validate_single_oversized_file_errors() {
        let dir = temp_dir("oversized_file");
        let path = dir.join("big.png");
        write_png(&path, MAX_IMAGE_DIM + 50, MAX_IMAGE_DIM + 50);

        let err = validate_image_dimensions(&path).unwrap_err();
        assert!(matches!(err, AnimaError::ImageTooLarge { .. }));
    }

    #[test]
    fn validate_missing_file_errors() {
        let err = validate_image_dimensions(Path::new("/nonexistent/x.png")).unwrap_err();
        assert!(matches!(err, AnimaError::AssetNotFound(_)));
    }

    #[test]
    fn validate_corrupt_png_fails_closed() {
        // A file with a .png extension but no real PNG header. Old behavior
        // returned (0, 0) and deferred to the decoder; we now error out.
        let dir = temp_dir("corrupt_png");
        let path = dir.join("not_really.png");
        std::fs::write(&path, b"this is not a png").unwrap();
        let err = validate_image_dimensions(&path).unwrap_err();
        // We mostly care that it's NOT silently Ok((0, 0)) and NOT an
        // "asset not found" — the path exists, it just isn't a PNG.
        assert!(matches!(err, AnimaError::Other(_)));
    }

    #[test]
    fn validate_unknown_extension_defers() {
        // A .xyz file the image crate can't probe — we still return Ok
        // because the format-specific loader may know what to do with it.
        let dir = temp_dir("unknown_ext");
        let path = dir.join("blob.xyz");
        std::fs::write(&path, b"opaque bytes").unwrap();
        let dims = validate_image_dimensions(&path).expect("should defer, not error");
        assert_eq!(dims, (0, 0));
    }
}
