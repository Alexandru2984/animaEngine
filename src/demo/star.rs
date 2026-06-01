//! Procedural star demo: 128×128, 6 frames, slow rotation + twinkle.

use std::path::Path;

const DIR: &str = "assets/demo/star";
const SIZE: u32 = 128;
const TOTAL_FRAMES: u32 = 6;

pub fn generate() {
    let path = Path::new(DIR);

    if path.exists() {
        if super::assets_already_at_size(path, SIZE) {
            tracing::debug!("Demo star assets already exist at {}px: {}", SIZE, DIR);
            return;
        }
        tracing::info!("Regenerating star demo assets at {SIZE}×{SIZE}…");
        let _ = std::fs::remove_dir_all(path);
    }

    tracing::info!("Generating star demo assets ({SIZE}×{SIZE}, {TOTAL_FRAMES} frames)…");
    if let Err(e) = std::fs::create_dir_all(path) {
        tracing::warn!("Failed to create demo directory {DIR}: {e}");
        return;
    }

    let cx = SIZE as f32 / 2.0;
    let cy = SIZE as f32 / 2.0;

    for frame_idx in 1..=TOTAL_FRAMES {
        let mut img = image::RgbaImage::new(SIZE, SIZE);
        let phase = (frame_idx as f32 - 1.0) * std::f32::consts::TAU / TOTAL_FRAMES as f32;

        // Slow rotation (small angle per frame).
        let rotation = phase * 0.25;
        // Twinkle: outer radius pulses ±10 %.
        let twinkle = 1.0 + 0.10 * phase.sin();

        let outer_r = 48.0 * twinkle;
        let inner_r = 18.0 * twinkle;
        let points = 5;

        for y in 0..SIZE {
            for x in 0..SIZE {
                let fx = x as f32 - cx;
                let fy = y as f32 - cy;
                let r = (fx * fx + fy * fy).sqrt();
                let angle = fy.atan2(fx) - rotation;

                // Star polar equation: alternate outer/inner radius every
                // PI/points radians, lerp between them to form straight edges.
                let segment = angle * points as f32 / std::f32::consts::PI;
                let frac = segment.rem_euclid(2.0);
                let t = (frac - 1.0).abs(); // 0 at peaks, 1 at valleys
                let edge = outer_r * (1.0 - t) + inner_r * t;

                if r < edge {
                    // Gold gradient — brighter at center.
                    let lum = 1.0 - (r / outer_r).clamp(0.0, 1.0) * 0.35;
                    let red = (255.0 * lum) as u8;
                    let g = (215.0 * lum) as u8;
                    let b = (60.0 * lum) as u8;
                    img.put_pixel(x, y, image::Rgba([red, g, b, 240]));
                } else if r < edge + 1.5 {
                    // Anti-aliased outline.
                    let aa = ((edge + 1.5 - r) / 1.5).clamp(0.0, 1.0);
                    let a = (180.0 * aa) as u8;
                    img.put_pixel(x, y, image::Rgba([200, 150, 30, a]));
                }
            }
        }

        // Center hot-spot — small bright dot that glints with twinkle.
        let hot_r = 5.0 + 1.5 * phase.cos();
        for dy in -(hot_r as i32 + 1)..=(hot_r as i32 + 1) {
            for dx in -(hot_r as i32 + 1)..=(hot_r as i32 + 1) {
                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                if dist < hot_r {
                    let px = (cx + dx as f32) as u32;
                    let py = (cy + dy as f32) as u32;
                    if px < SIZE && py < SIZE {
                        let intensity = (1.0 - dist / hot_r).powi(2);
                        let r = (255.0 * intensity + 200.0 * (1.0 - intensity)) as u8;
                        let g = (250.0 * intensity + 180.0 * (1.0 - intensity)) as u8;
                        let b = (210.0 * intensity + 40.0 * (1.0 - intensity)) as u8;
                        img.put_pixel(px, py, image::Rgba([r, g, b, 250]));
                    }
                }
            }
        }

        let frame_path = path.join(format!("frame_{frame_idx:03}.png"));
        if let Err(e) = img.save(&frame_path) {
            tracing::warn!("Failed to save demo frame {}: {e}", frame_path.display());
        }
    }

    tracing::info!("Generated {TOTAL_FRAMES} star demo frames ({SIZE}×{SIZE})");
}
