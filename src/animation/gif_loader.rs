use super::frame::Frame;
use crate::error::Result;
use image::codecs::gif::GifDecoder;
use image::AnimationDecoder;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Load frames from a GIF file with per-frame delay extraction.
/// Each frame carries its own delay in milliseconds from the GIF metadata.
///
/// Truncates to `MAX_ANIMATION_FRAMES` and to `MAX_DECODED_ASSET_BYTES`
/// of total decoded RGBA — a multi-hour pathological GIF won't OOM us.
pub fn load_gif(path: &Path) -> Result<Vec<Frame>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let decoder = GifDecoder::new(reader)?;
    super::animated::collect_frames(decoder.into_frames(), "GIF", path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::MAX_ANIMATION_FRAMES;
    use crate::error::AnimaError;
    use image::codecs::gif::GifEncoder;
    use image::{Delay, Frame as ImageFrame, RgbaImage};
    use std::path::PathBuf;

    fn tmp_dir(name: &str) -> PathBuf {
        let d = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("gif_tests")
            .join(name);
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Encode a real animated GIF so the decode path is exercised end to
    /// end rather than mocked.
    fn write_gif(path: &Path, count: usize, delay_ms: u32) {
        let file = std::fs::File::create(path).unwrap();
        let mut enc = GifEncoder::new(file);
        let frames: Vec<ImageFrame> = (0..count)
            .map(|i| {
                let shade = (i * 20 % 256) as u8;
                let img = RgbaImage::from_pixel(4, 4, image::Rgba([shade, 0, 0, 255]));
                ImageFrame::from_parts(img, 0, 0, Delay::from_numer_denom_ms(delay_ms, 1))
            })
            .collect();
        enc.encode_frames(frames).unwrap();
    }

    #[test]
    fn decodes_frames_and_per_frame_delays() {
        let dir = tmp_dir("basic");
        let path = dir.join("anim.gif");
        write_gif(&path, 3, 100);

        let frames = load_gif(&path).expect("should decode");
        assert_eq!(frames.len(), 3);
        for f in &frames {
            assert_eq!((f.width, f.height), (4, 4));
            assert_eq!(f.rgba.len(), 4 * 4 * 4);
            // GIF stores delays in centiseconds, so 100ms round-trips.
            assert_eq!(f.delay_ms, Some(100), "per-frame delay preserved");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn frame_count_is_capped() {
        // The cap is shared with the WebP path via `animated::collect_frames`;
        // this pins it so the two can't silently diverge again.
        let dir = tmp_dir("capped");
        let path = dir.join("long.gif");
        write_gif(&path, MAX_ANIMATION_FRAMES + 25, 20);

        let frames = load_gif(&path).expect("should decode");
        assert_eq!(
            frames.len(),
            MAX_ANIMATION_FRAMES,
            "must truncate at MAX_ANIMATION_FRAMES, not decode the lot"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn garbage_is_rejected_not_silently_empty() {
        let dir = tmp_dir("garbage");
        let path = dir.join("not.gif");
        std::fs::write(&path, b"this is not a GIF").unwrap();
        assert!(load_gif(&path).is_err());

        let missing = dir.join("nope.gif");
        assert!(matches!(load_gif(&missing), Err(AnimaError::Io(_))));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
