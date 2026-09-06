use super::frame::Frame;
use crate::error::Result;
use image::codecs::webp::WebPDecoder;
use image::AnimationDecoder;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Load frames from an animated WebP file.
/// Falls back to loading as a static image if animation decoding fails.
/// Same caps as GIF: `MAX_ANIMATION_FRAMES` and `MAX_DECODED_ASSET_BYTES`.
pub fn load_webp(path: &Path) -> Result<Vec<Frame>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let decoder = WebPDecoder::new(reader)?;

    if decoder.has_animation() {
        super::animated::collect_frames(decoder.into_frames(), "animated WebP", path)
    } else {
        load_static_webp(path)
    }
}

/// Load a static (non-animated) WebP as a single frame.
pub fn load_static_webp(path: &Path) -> Result<Vec<Frame>> {
    let img = image::open(path)?.to_rgba8();
    let (width, height) = img.dimensions();
    let rgba = img.into_raw();
    tracing::info!(
        "Loaded static WebP: {}x{} from {}",
        width,
        height,
        crate::drop_validate::redact_path(path)
    );
    Ok(vec![Frame::new(rgba, width, height)])
}
