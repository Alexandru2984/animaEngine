//! Procedural heart demo: 128×128, 6 frames, pulse + sparkle.

use std::path::Path;

const DIR: &str = "assets/demo/heart";
const SIZE: u32 = 128;
const TOTAL_FRAMES: u32 = 6;

pub fn generate() {
    let path = Path::new(DIR);

    if path.exists() {
        if super::assets_already_at_size(path, SIZE) {
            tracing::debug!("Demo heart assets already exist at {}px: {}", SIZE, DIR);
            return;
        }
        tracing::info!("Regenerating heart demo assets at {SIZE}×{SIZE}…");
        let _ = std::fs::remove_dir_all(path);
    }

    tracing::info!("Generating heart demo assets ({SIZE}×{SIZE}, {TOTAL_FRAMES} frames)…");
    if let Err(e) = std::fs::create_dir_all(path) {
        tracing::warn!("Failed to create demo directory {DIR}: {e}");
        return;
    }

    let cx = SIZE as f32 / 2.0;
    let cy = SIZE as f32 / 2.0 + 4.0;

    for frame_idx in 1..=TOTAL_FRAMES {
        let mut img = image::RgbaImage::new(SIZE, SIZE);
        let phase = (frame_idx as f32 - 1.0) * std::f32::consts::TAU / TOTAL_FRAMES as f32;

        // Pulse: small heart at phase=0, bigger at phase=PI.
        let pulse = 1.0 + 0.10 * (-phase.cos());
        let scale = 38.0 * pulse;

        for y in 0..SIZE {
            for x in 0..SIZE {
                let fx = (x as f32 - cx) / scale;
                let fy = (y as f32 - cy) / scale;
                // Classic heart implicit equation.
                let heart = (fx * fx + fy * fy - 1.0).powi(3) - fx * fx * fy.powi(3);

                if heart < 0.0 {
                    // Inside heart — vertical gradient red → magenta.
                    let t = ((y as f32 - cy + scale) / (2.0 * scale)).clamp(0.0, 1.0);
                    let r = (230.0 - t * 30.0) as u8;
                    let g = (50.0 + t * 30.0) as u8;
                    let b = (90.0 + t * 60.0) as u8;
                    img.put_pixel(x, y, image::Rgba([r, g, b, 240]));
                } else if heart < 0.15 {
                    // Soft outline.
                    let edge = ((0.15 - heart) / 0.15).clamp(0.0, 1.0);
                    let a = (180.0 * edge) as u8;
                    img.put_pixel(x, y, image::Rgba([200, 30, 70, a]));
                }
            }
        }

        // Top-left specular highlight that pulses with the heart.
        let hl_cx = cx - 14.0;
        let hl_cy = cy - 8.0;
        let hl_r = 9.0 * pulse;
        for dy in -(hl_r as i32)..=(hl_r as i32) {
            for dx in -(hl_r as i32)..=(hl_r as i32) {
                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                if dist < hl_r {
                    let hx = (hl_cx + dx as f32) as i32;
                    let hy = (hl_cy + dy as f32) as i32;
                    if hx >= 0 && hy >= 0 && (hx as u32) < SIZE && (hy as u32) < SIZE {
                        let existing = img.get_pixel(hx as u32, hy as u32);
                        if existing[3] > 0 {
                            let intensity = (1.0 - dist / hl_r).powi(2);
                            let r = (existing[0] as f32
                                + (255.0 - existing[0] as f32) * intensity * 0.7)
                                as u8;
                            let g = (existing[1] as f32
                                + (255.0 - existing[1] as f32) * intensity * 0.5)
                                as u8;
                            let b = (existing[2] as f32
                                + (255.0 - existing[2] as f32) * intensity * 0.6)
                                as u8;
                            img.put_pixel(
                                hx as u32,
                                hy as u32,
                                image::Rgba([r, g, b, existing[3]]),
                            );
                        }
                    }
                }
            }
        }

        // Tiny sparkle that orbits the heart — small bright dot at varying angle.
        let sparkle_angle = phase * 1.5;
        let sparkle_r = 50.0;
        let sx = cx + sparkle_angle.cos() * sparkle_r;
        let sy = cy - 6.0 + sparkle_angle.sin() * sparkle_r * 0.6;
        for dy in -2i32..=2 {
            for dx in -2i32..=2 {
                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                if dist < 2.0 {
                    let px = (sx + dx as f32) as i32;
                    let py = (sy + dy as f32) as i32;
                    if px >= 0 && py >= 0 && (px as u32) < SIZE && (py as u32) < SIZE {
                        let a = (255.0 * (1.0 - dist / 2.0)) as u8;
                        img.put_pixel(px as u32, py as u32, image::Rgba([255, 250, 220, a]));
                    }
                }
            }
        }

        let frame_path = path.join(format!("frame_{frame_idx:03}.png"));
        if let Err(e) = img.save(&frame_path) {
            tracing::warn!("Failed to save demo frame {}: {e}", frame_path.display());
        }
    }

    tracing::info!("Generated {TOTAL_FRAMES} heart demo frames ({SIZE}×{SIZE})");
}
