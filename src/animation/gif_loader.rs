use super::frame::Frame;
use image::codecs::gif::GifDecoder;
use image::AnimationDecoder;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Load frames from a GIF file.
/// Best-effort: complex GIFs with disposal modes may not render perfectly.
pub fn load_gif(path: &Path) -> Result<Vec<Frame>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let decoder = GifDecoder::new(reader)?;
    let gif_frames = decoder.into_frames();

    let mut frames = Vec::new();
    for frame_result in gif_frames {
        match frame_result {
            Ok(frame) => {
                let rgba_image = frame.into_buffer();
                let (width, height) = rgba_image.dimensions();
                let rgba = rgba_image.into_raw();
                frames.push(Frame::new(rgba, width, height));
            }
            Err(e) => {
                log::warn!("Failed to decode GIF frame: {}", e);
            }
        }
    }

    if frames.is_empty() {
        return Err(format!("No frames decoded from GIF: {}", path.display()).into());
    }

    log::info!(
        "Loaded GIF: {} frames from {}",
        frames.len(),
        path.display()
    );
    Ok(frames)
}
