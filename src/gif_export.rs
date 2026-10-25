//! Saving a character as a GIF (1.5): its animation as it plays now — the
//! frames already decoded, at its size on screen (at most [`MAX_SIDE`] a
//! side), facing the way it faces, each shown as long as it is here —
//! looping forever, written where the user picks
//! (`crate::outcomes::SceneTransfer`).
//!
//! The frames are copied out of the scene, scaled, on the UI thread
//! ([`GifJob::of`]); the encoding, which takes a moment, happens on a
//! worker. A GIF has no half-transparent pixels, so soft edges come out
//! hard, as they do in any GIF.

use crate::entity::Entity;
use crate::error::{AnimaError, Result};
use std::path::Path;

/// The largest side of a saved GIF, in pixels.
pub const MAX_SIDE: u32 = 400;

/// At most this many frames: a long video would make a huge GIF.
pub const MAX_FRAMES: usize = 120;

/// The shortest a frame shows, in milliseconds: most viewers slow down
/// anything faster than this.
const MIN_DELAY_MS: u32 = 20;

/// A character's frames, ready to encode.
pub struct GifJob {
    /// The character's name, for the file's.
    pub name: String,
    frames: Vec<(image::RgbaImage, u32)>,
}

impl GifJob {
    /// The frames of what `entity` plays now, scaled and turned as it
    /// shows; `None` if it has none to show.
    pub fn of(entity: &Entity) -> Option<Self> {
        let animation = entity.animation();
        let first = animation.frames.first()?;
        // At the size it shows, a picture kept sharp for a HiDPI screen
        // as one that is not (`Animation::pixel_scale`).
        let scale = entity.scale * animation.pixel_scale();
        let (w, h) = (
            (first.width as f32 * scale).max(1.0),
            (first.height as f32 * scale).max(1.0),
        );
        let fit = (MAX_SIDE as f32 / w.max(h)).min(1.0);
        let size = (
            ((w * fit).round() as u32).max(1),
            ((h * fit).round() as u32).max(1),
        );
        let default_delay = (1000.0 / animation.fps.max(0.1)) as u32;
        let frames: Vec<_> = animation
            .frames
            .iter()
            .take(MAX_FRAMES)
            .filter_map(|frame| {
                let image =
                    image::RgbaImage::from_raw(frame.width, frame.height, frame.rgba.clone())?;
                let mut image = if (frame.width, frame.height) == size {
                    image
                } else {
                    image::imageops::resize(
                        &image,
                        size.0,
                        size.1,
                        image::imageops::FilterType::Triangle,
                    )
                };
                if entity.facing_left {
                    image::imageops::flip_horizontal_in_place(&mut image);
                }
                let delay = frame.delay_ms.unwrap_or(default_delay).max(MIN_DELAY_MS);
                Some((image, delay))
            })
            .collect();
        (!frames.is_empty()).then(|| Self {
            name: entity.name.clone(),
            frames,
        })
    }

    /// The file name to offer: the character's name, made safe.
    pub fn file_name(&self) -> String {
        format!("{}.gif", crate::scenes::slug(&self.name))
    }

    /// Encode it and write it to `to`. Returns the size written. On failure
    /// nothing is left at `to`.
    pub fn write(self, to: &Path) -> Result<u64> {
        let written = self.write_inner(to);
        if written.is_err() {
            let _ = std::fs::remove_file(to);
        }
        written
    }

    fn write_inner(self, to: &Path) -> Result<u64> {
        use image::codecs::gif::{GifEncoder, Repeat};
        let fail = |e: image::ImageError| AnimaError::other(format!("GIF: {e}"));
        // Straight to `to`: in the Flatpak it is the one file the portal
        // opened to us.
        let file = std::fs::File::create(to)?;
        let mut encoder = GifEncoder::new_with_speed(std::io::BufWriter::new(&file), 10);
        encoder.set_repeat(Repeat::Infinite).map_err(fail)?;
        let frames = self.frames.into_iter().map(|(image, delay)| {
            image::Frame::from_parts(image, 0, 0, image::Delay::from_numer_denom_ms(delay, 1))
        });
        encoder.encode_frames(frames).map_err(fail)?;
        drop(encoder);
        file.sync_all()?;
        Ok(file.metadata()?.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::frame::Frame;
    use crate::animation::Animation;

    fn entity(frames: usize, side: u32, scale: f32) -> Entity {
        let frames = (0..frames)
            .map(|i| {
                let mut rgba = vec![0u8; (side * side * 4) as usize];
                for px in rgba.chunks_mut(4) {
                    px.copy_from_slice(&[200, (i * 40) as u8, 30, 255]);
                }
                Frame::new(rgba, side, side)
            })
            .collect();
        let mut config = crate::config::AppConfig::default().characters[0].clone();
        config.name = "Cat / Demo".into();
        config.scale = scale;
        Entity::from_config(&config, Animation::new(frames, 8.0, true))
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("gif_tests");
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn a_character_becomes_a_looping_gif_of_its_frames() {
        let job = GifJob::of(&entity(4, 64, 1.0)).unwrap();
        assert_eq!(job.file_name(), crate::scenes::slug("Cat / Demo") + ".gif");
        let path = scratch("loop.gif");
        let bytes = job.write(&path).unwrap();
        assert!(bytes > 0);
        let decoder = image::codecs::gif::GifDecoder::new(std::io::BufReader::new(
            std::fs::File::open(&path).unwrap(),
        ))
        .unwrap();
        use image::AnimationDecoder;
        let frames: Vec<_> = decoder
            .into_frames()
            .collect::<std::result::Result<_, _>>()
            .unwrap();
        assert_eq!(frames.len(), 4);
        assert_eq!(frames[0].buffer().dimensions(), (64, 64));
        let (n, d) = frames[0].delay().numer_denom_ms();
        // GIF counts in hundredths of a second: 125 ms comes out as 120.
        assert!((n / d).abs_diff(125) <= 10, "8 fps: {} ms", n / d);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_big_one_is_scaled_down_and_frames_are_capped() {
        let job = GifJob::of(&entity(MAX_FRAMES + 10, 220, 2.0)).unwrap();
        assert_eq!(job.frames.len(), MAX_FRAMES);
        assert_eq!(job.frames[0].0.dimensions(), (MAX_SIDE, MAX_SIDE));
    }
}
