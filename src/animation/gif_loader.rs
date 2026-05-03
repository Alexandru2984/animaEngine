use super::frame::Frame;
use image::codecs::gif::GifDecoder;
use image::AnimationDecoder;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Load frames from a GIF file with per-frame delay extraction.
/// Each frame carries its own delay in milliseconds from the GIF metadata.
pub fn load_gif(path: &Path) -> Result<Vec<Frame>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let decoder = GifDecoder::new(reader)?;
    let gif_frames = decoder.into_frames();

    let mut frames = Vec::new();
    for frame_result in gif_frames {
        match frame_result {
            Ok(frame) => {
                // Extract the delay from GIF frame metadata
                let (numerator, denominator) = frame.delay().numer_denom_ms();
                let delay_ms = numerator.checked_div(denominator).unwrap_or(100);

                let rgba_image = frame.into_buffer();
                let (width, height) = rgba_image.dimensions();
                let rgba = rgba_image.into_raw();

                // Use per-frame delay if it's meaningful (> 0ms)
                if delay_ms > 0 {
                    frames.push(Frame::with_delay(rgba, width, height, delay_ms));
                } else {
                    frames.push(Frame::new(rgba, width, height));
                }
            }
            Err(e) => {
                log::warn!("Failed to decode GIF frame: {}", e);
            }
        }
    }

    if frames.is_empty() {
        return Err(format!("No frames decoded from GIF: {}", path.display()).into());
    }

    let has_delays = frames.iter().any(|f| f.delay_ms.is_some());
    log::info!(
        "Loaded GIF: {} frames from {} (per-frame delays: {})",
        frames.len(),
        path.display(),
        if has_delays { "yes" } else { "no" }
    );
    Ok(frames)
}
