//! Scene thumbnails (1.5): a small picture of a saved scene — its
//! characters as they stood when it was saved — beside its name in the
//! Scene tab (`ui::panels::scene`), and carried in a shared scene file
//! (`crate::scene_file`).
//!
//! The picture frames the characters, not the screen: a 128-pixel cat on
//! a 2560-pixel desktop would be a speck at this size. Each is drawn at
//! twice its size, too, about its middle, so they read at a glance. It
//! is drawn on the
//! CPU from the frames already decoded — each thumbnail pixel the average
//! of what it covers — and saved as a PNG beside the scene's TOML, under
//! the same name. Reading one back is capped twice: the file's size, and
//! the image's size before anything is decoded.

use crate::scene::Scene;
use std::io::Read;
use std::path::{Path, PathBuf};

pub const WIDTH: u32 = 160;
pub const HEIGHT: u32 = 100;

/// The most a thumbnail file may weigh. A 160×100 PNG is a few KB.
pub const MAX_BYTES: u64 = 256 * 1024;

/// The largest picture taken for one, either way.
const MAX_SIDE: u32 = 512;

/// The least of the world a thumbnail shows, so a lone character is not
/// blown up to fill it.
const MIN_VIEW: (f32, f32) = (480.0, 300.0);

/// How much bigger than life each character is drawn.
const BOOST: f32 = 2.0;

/// Where the thumbnail of the scene saved in `scene_file` goes.
pub fn path_for(scene_file: &Path) -> PathBuf {
    scene_file.with_extension("png")
}

/// The visible characters of `scene` as they look now, framed; `None`
/// when nobody can be seen.
pub fn render(scene: &Scene) -> Option<image::RgbaImage> {
    // Everyone in the scene, whatever the hour (`crate::hours`).
    let mut order: Vec<usize> = (0..scene.entities.len())
        .filter(|&i| scene.visible_any_hour(i))
        .collect();
    order.sort_by_key(|&i| scene.entities[i].z_index);
    let placed: Vec<(usize, crate::input::arrange::Rect)> = order
        .into_iter()
        .filter_map(|i| crate::input::arrange::drawn_rect(scene, i).map(|r| (i, boosted(r))))
        .filter(|(_, (l, t, r, b))| r > l && b > t)
        .collect();
    let view = frame_around(placed.iter().map(|(_, r)| *r))?;
    let scale = WIDTH as f32 / (view.2 - view.0);

    // Premultiplied, in 0..=1.
    let mut canvas = vec![[0.0f32; 4]; (WIDTH * HEIGHT) as usize];
    for (i, rect) in placed {
        let entity = &scene.entities[i];
        let Some(frame) = entity.animation().current_frame_data() else {
            continue;
        };
        let opacity = entity.opacity.clamp(0.0, 1.0);
        draw(
            &mut canvas,
            frame,
            rect,
            view,
            scale,
            entity.facing_left,
            opacity,
        );
    }

    let mut out = image::RgbaImage::new(WIDTH, HEIGHT);
    for (px, [r, g, b, a]) in out.pixels_mut().zip(canvas) {
        *px = if a <= 0.0 {
            image::Rgba([0, 0, 0, 0])
        } else {
            let un = |c: f32| ((c / a).clamp(0.0, 1.0) * 255.0).round() as u8;
            image::Rgba([
                un(r),
                un(g),
                un(b),
                (a.clamp(0.0, 1.0) * 255.0).round() as u8,
            ])
        };
    }
    Some(out)
}

/// `rect` [`BOOST`] times as big, about its middle.
fn boosted((l, t, r, b): crate::input::arrange::Rect) -> crate::input::arrange::Rect {
    let (cx, cy) = ((l + r) / 2.0, (t + b) / 2.0);
    let (hw, hh) = ((r - l) / 2.0 * BOOST, (b - t) / 2.0 * BOOST);
    (cx - hw, cy - hh, cx + hw, cy + hh)
}

/// What the thumbnail shows of the world: around everyone, with a margin,
/// at least [`MIN_VIEW`], widened to the thumbnail's shape.
fn frame_around(
    rects: impl Iterator<Item = crate::input::arrange::Rect>,
) -> Option<(f32, f32, f32, f32)> {
    let (l, t, r, b) =
        rects.reduce(|a, b| (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)))?;
    let (cx, cy) = ((l + r) / 2.0, (t + b) / 2.0);
    let margin = 0.08 * (r - l).max(b - t);
    let mut w = (r - l + 2.0 * margin).max(MIN_VIEW.0);
    let mut h = (b - t + 2.0 * margin).max(MIN_VIEW.1);
    let aspect = WIDTH as f32 / HEIGHT as f32;
    if w / h > aspect {
        h = w / aspect;
    } else {
        w = h * aspect;
    }
    Some((cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0))
}

/// Lay `frame`, drawn at `rect` in the world, over `canvas`, which shows
/// `view` at `scale` thumbnail pixels per world pixel.
fn draw(
    canvas: &mut [[f32; 4]],
    frame: &crate::animation::frame::Frame,
    rect: crate::input::arrange::Rect,
    view: (f32, f32, f32, f32),
    scale: f32,
    flipped: bool,
    opacity: f32,
) {
    let (fw, fh) = (frame.width as usize, frame.height as usize);
    if fw == 0 || fh == 0 || frame.rgba.len() < fw * fh * 4 {
        return;
    }
    let (l, t, r, b) = rect;
    // Frame pixels per world pixel.
    let (sx_per, sy_per) = (fw as f32 / (r - l), fh as f32 / (b - t));
    let to_thumb = |world: f32, origin: f32| (world - origin) * scale;
    let x0 = to_thumb(l, view.0).floor().max(0.0) as u32;
    let x1 = (to_thumb(r, view.0).ceil().max(0.0) as u32).min(WIDTH);
    let y0 = to_thumb(t, view.1).floor().max(0.0) as u32;
    let y1 = (to_thumb(b, view.1).ceil().max(0.0) as u32).min(HEIGHT);
    for ty in y0..y1 {
        // The frame's rows under this thumbnail row.
        let wy0 = view.1 + ty as f32 / scale;
        let wy1 = wy0 + 1.0 / scale;
        let sy0 = (((wy0 - t) * sy_per).floor().max(0.0) as usize).min(fh);
        let sy1 = (((wy1 - t) * sy_per).ceil().max(0.0) as usize).min(fh);
        for tx in x0..x1 {
            let wx0 = view.0 + tx as f32 / scale;
            let wx1 = wx0 + 1.0 / scale;
            let sx0 = (((wx0 - l) * sx_per).floor().max(0.0) as usize).min(fw);
            let sx1 = (((wx1 - l) * sx_per).ceil().max(0.0) as usize).min(fw);
            if sx1 <= sx0 || sy1 <= sy0 {
                continue;
            }
            let mut sum = [0.0f32; 4];
            for sy in sy0..sy1 {
                for sx in sx0..sx1 {
                    let x = if flipped { fw - 1 - sx } else { sx };
                    let p = &frame.rgba[(sy * fw + x) * 4..][..4];
                    let a = f32::from(p[3]) / 255.0;
                    sum[0] += f32::from(p[0]) / 255.0 * a;
                    sum[1] += f32::from(p[1]) / 255.0 * a;
                    sum[2] += f32::from(p[2]) / 255.0 * a;
                    sum[3] += a;
                }
            }
            let n = ((sx1 - sx0) * (sy1 - sy0)) as f32;
            let src = sum.map(|c| c / n * opacity);
            let dst = &mut canvas[(ty * WIDTH + tx) as usize];
            for c in 0..4 {
                dst[c] = src[c] + dst[c] * (1.0 - src[3]);
            }
        }
    }
}

/// `image` as a PNG file's bytes.
pub fn encode(image: &image::RgbaImage) -> crate::error::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .map_err(|e| crate::error::AnimaError::other(e.to_string()))?;
    Ok(bytes)
}

/// A thumbnail from a PNG file's `bytes`: refused past [`MAX_BYTES`], if
/// it is not a PNG, or if it is larger than 512 pixels a side — the size
/// is read before anything is decoded.
pub fn decode(bytes: &[u8]) -> crate::error::Result<image::RgbaImage> {
    let fail = |e: String| crate::error::AnimaError::other(format!("thumbnail: {e}"));
    if bytes.len() as u64 > MAX_BYTES {
        return Err(fail("too large".into()));
    }
    let mut reader =
        image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    limits.max_alloc = Some(u64::from(MAX_SIDE * MAX_SIDE * 4) * 2);
    reader.limits(limits);
    let image = reader.decode().map_err(|e| fail(e.to_string()))?;
    Ok(image.to_rgba8())
}

/// The thumbnail file at `path`, read capped, as decoded.
pub fn load(path: &Path) -> crate::error::Result<image::RgbaImage> {
    decode(&read(path)?)
}

/// The bytes of the thumbnail file at `path`, read capped.
pub fn read(path: &Path) -> crate::error::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(crate::error::AnimaError::other("thumbnail: too large"));
    }
    Ok(bytes)
}

/// Picture `scene` for the scene saved in `scene_file`; with nobody to be
/// seen, no picture — an old one goes. Failing only costs the picture.
pub fn save_for(scene_file: &Path, scene: &Scene) {
    let path = path_for(scene_file);
    let written = render(scene).map(|image| {
        encode(&image).and_then(|png| Ok(crate::util::atomic_write_bytes(&path, &png)?))
    });
    match written {
        Some(Err(e)) => tracing::debug!("Scene thumbnail not saved: {e}"),
        None => {
            let _ = std::fs::remove_file(&path);
        }
        Some(Ok(())) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, CharacterConfig};

    fn scene_at(spots: &[(f32, f32)]) -> Scene {
        let mut scene = Scene::from_config(&AppConfig::default());
        let template = AppConfig::default().characters.remove(0);
        let configs: Vec<_> = spots
            .iter()
            .enumerate()
            .map(|(i, &(x, y))| CharacterConfig {
                id: format!("c{i}"),
                name: format!("c{i}"),
                asset_path: String::new(),
                x,
                y,
                ..template.clone()
            })
            .collect();
        scene.restore_configs(&configs);
        scene
    }

    #[test]
    fn the_picture_frames_the_characters() {
        let scene = scene_at(&[(100.0, 100.0), (1400.0, 800.0)]);
        let image = render(&scene).expect("two characters");
        assert_eq!(image.dimensions(), (WIDTH, HEIGHT));
        let seen =
            |x0: u32, x1: u32| (x0..x1).any(|x| (0..HEIGHT).any(|y| image.get_pixel(x, y)[3] > 0));
        assert!(seen(0, WIDTH / 3), "the one top left");
        assert!(seen(2 * WIDTH / 3, WIDTH), "the one bottom right");
        assert!(!seen(WIDTH / 2 - 4, WIDTH / 2 + 4), "nobody in the middle");
    }

    #[test]
    fn nobody_seen_is_no_picture() {
        let mut scene = scene_at(&[(100.0, 100.0)]);
        scene.entities[0].visible = false;
        scene.mark_visible_dirty();
        assert!(render(&scene).is_none());
    }

    #[test]
    fn a_picture_survives_the_round_trip_and_a_big_one_is_refused() {
        let scene = scene_at(&[(300.0, 300.0)]);
        let image = render(&scene).unwrap();
        let png = encode(&image).unwrap();
        assert!(png.len() as u64 <= MAX_BYTES);
        assert_eq!(decode(&png).unwrap(), image);

        let big = image::RgbaImage::new(MAX_SIDE + 1, 10);
        assert!(
            decode(&encode(&big).unwrap()).is_err(),
            "wider than allowed"
        );
        assert!(decode(b"not a png").is_err());
    }
}
