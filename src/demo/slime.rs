//! Procedurally generated slime demo: 128×128, 6 frames,
//! squash-stretch bounce + chibi expressions.

use std::path::Path;

const DIR: &str = "assets/demo/slime";
const SIZE: u32 = 128;
const TOTAL_FRAMES: u32 = 6;

pub fn generate() {
    let path = Path::new(DIR);

    if path.exists() {
        if super::assets_already_at_size(path, SIZE) {
            tracing::debug!("Demo slime assets already exist at {}px: {}", SIZE, DIR);
            return;
        }
        tracing::info!("Regenerating slime demo assets at {SIZE}×{SIZE}…");
        let _ = std::fs::remove_dir_all(path);
    }

    tracing::info!("Generating slime demo assets ({SIZE}×{SIZE}, {TOTAL_FRAMES} frames)…");
    if let Err(e) = std::fs::create_dir_all(path) {
        tracing::warn!("Failed to create demo directory {DIR}: {e}");
        return;
    }

    let cx = SIZE as f32 / 2.0;

    for frame_idx in 1..=TOTAL_FRAMES {
        let mut img = image::RgbaImage::new(SIZE, SIZE);
        let phase = (frame_idx as f32 - 1.0) * std::f32::consts::TAU / TOTAL_FRAMES as f32;

        // Squash-stretch
        let stretch = phase.sin() * 0.15;
        let sx = 1.0 - stretch;
        let sy = 1.0 + stretch;

        let base_rx = 38.0;
        let base_ry = 32.0;
        let body_rx = base_rx * sx;
        let body_ry = base_ry * sy;

        let body_cy = SIZE as f32 - body_ry - 10.0;
        let body_cx = cx;

        for y in 0..SIZE {
            for x in 0..SIZE {
                let fx = x as f32;
                let fy = y as f32;

                let dx = (fx - body_cx) / body_rx;
                let dy = (fy - body_cy) / body_ry;
                let dist_sq = dx * dx + dy * dy;

                let is_top_half = fy <= body_cy;
                let is_bottom = fy > body_cy
                    && fy < body_cy + body_ry * 0.8
                    && (fx - body_cx).abs()
                        < body_rx
                            * (1.0 - ((fy - body_cy) / (body_ry * 0.8)).powi(2))
                                .max(0.0)
                                .sqrt();

                let in_body = (is_top_half && dist_sq < 1.0) || is_bottom;

                if in_body {
                    let vert_progress =
                        ((fy - (body_cy - body_ry)) / (body_ry * 2.0)).clamp(0.0, 1.0);
                    let brightness = 1.0 - vert_progress * 0.35;

                    let center_dist = ((fx - body_cx).powi(2) + (fy - body_cy).powi(2)).sqrt();
                    let radial = 1.0 - (center_dist / 50.0).min(1.0) * 0.15;

                    let b = brightness * radial;

                    let r = (60.0 * b) as u8;
                    let g = (200.0 * b) as u8;
                    let bb = (70.0 * b) as u8;

                    img.put_pixel(x, y, image::Rgba([r, g, bb, 230]));
                } else if is_top_half && dist_sq < 1.08 {
                    let edge = ((1.08 - dist_sq) / 0.08).max(0.0);
                    let a = (180.0 * edge) as u8;
                    if a > 0 {
                        img.put_pixel(x, y, image::Rgba([50, 180, 60, a]));
                    }
                }
            }
        }

        // Top-left specular highlight
        let hl_cx = body_cx - body_rx * 0.3;
        let hl_cy = body_cy - body_ry * 0.4;
        let hl_r = 10.0;
        for dy in -(hl_r as i32)..=(hl_r as i32) {
            for dx in -(hl_r as i32)..=(hl_r as i32) {
                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                if dist < hl_r {
                    let hx = (hl_cx + dx as f32) as u32;
                    let hy = (hl_cy + dy as f32) as u32;
                    if hx < SIZE && hy < SIZE {
                        let intensity = (1.0 - dist / hl_r).powi(2);
                        let a = (120.0 * intensity) as u8;
                        let existing = img.get_pixel(hx, hy);
                        if existing[3] > 0 {
                            let r = (existing[0] as f32
                                + (255.0 - existing[0] as f32) * intensity * 0.6)
                                as u8;
                            let g = (existing[1] as f32
                                + (255.0 - existing[1] as f32) * intensity * 0.5)
                                as u8;
                            let b = (existing[2] as f32
                                + (255.0 - existing[2] as f32) * intensity * 0.6)
                                as u8;
                            img.put_pixel(hx, hy, image::Rgba([r, g, b, existing[3].max(a)]));
                        }
                    }
                }
            }
        }

        // Eyes
        let eye_y = body_cy - body_ry * 0.15;
        let eye_spacing = body_rx * 0.35;
        let is_squash = stretch < -0.05;

        for side in [-1.0f32, 1.0f32] {
            let eye_cx = body_cx + side * eye_spacing;

            if is_squash {
                // Happy ^_^
                for i in -5i32..=5 {
                    let ix = eye_cx + i as f32;
                    let iy = eye_y - (i as f32).abs() * 0.6;
                    let ex = ix as u32;
                    let ey = iy as u32;
                    if ex < SIZE && ey < SIZE {
                        img.put_pixel(ex, ey, image::Rgba([15, 50, 15, 250]));
                        if ey + 1 < SIZE {
                            img.put_pixel(ex, ey + 1, image::Rgba([15, 50, 15, 200]));
                        }
                    }
                }
            } else {
                let eye_r = 5.5;
                let pupil_r = 3.0;
                let highlight_r = 1.8;

                for dy in -(eye_r as i32)..=(eye_r as i32) {
                    for ddx in -(eye_r as i32)..=(eye_r as i32) {
                        let ex = (eye_cx + ddx as f32) as u32;
                        let ey = (eye_y + dy as f32) as u32;
                        if ex >= SIZE || ey >= SIZE {
                            continue;
                        }

                        let dist = ((ddx * ddx + dy * dy) as f32).sqrt();

                        let hdx = ddx as f32 - 1.5;
                        let hdy = dy as f32 + 1.5;
                        let h_dist = (hdx * hdx + hdy * hdy).sqrt();

                        if h_dist < highlight_r {
                            img.put_pixel(ex, ey, image::Rgba([255, 255, 255, 250]));
                        } else if dist < pupil_r {
                            img.put_pixel(ex, ey, image::Rgba([15, 50, 15, 245]));
                        } else if dist < eye_r {
                            let edge = (eye_r - dist) / 1.0;
                            let a = (240.0 * edge.min(1.0)) as u8;
                            img.put_pixel(ex, ey, image::Rgba([240, 240, 240, a]));
                        }
                    }
                }
            }
        }

        // Smile
        let mouth_y = body_cy + body_ry * 0.15;
        let mouth_w = body_rx * 0.35;
        for mx in -(mouth_w as i32)..=(mouth_w as i32) {
            let curve = (mx as f32 / mouth_w).powi(2) * 3.0;
            let my = mouth_y + curve;
            let ex = (body_cx + mx as f32) as u32;
            let ey = my as u32;
            if ex < SIZE && ey < SIZE {
                img.put_pixel(ex, ey, image::Rgba([20, 80, 20, 200]));
                if ey + 1 < SIZE {
                    img.put_pixel(ex, ey + 1, image::Rgba([20, 80, 20, 150]));
                }
            }
        }

        // Blush
        for side in [-1.0f32, 1.0f32] {
            let blush_cx = body_cx + side * (eye_spacing + 8.0);
            let blush_cy = eye_y + 8.0;
            let blush_r = 4.0;
            for dy in -(blush_r as i32)..=(blush_r as i32) {
                for ddx in -(blush_r as i32 + 2)..=(blush_r as i32 + 2) {
                    let dist = ((ddx as f32 / 1.5).powi(2) + (dy * dy) as f32).sqrt();
                    if dist < blush_r {
                        let bx = (blush_cx + ddx as f32) as u32;
                        let by = (blush_cy + dy as f32) as u32;
                        if bx < SIZE && by < SIZE {
                            let existing = img.get_pixel(bx, by);
                            if existing[3] > 100 {
                                let intensity = (1.0 - dist / blush_r).powi(2);
                                let r = (existing[0] as f32
                                    + (255.0 - existing[0] as f32) * intensity * 0.5)
                                    .min(255.0) as u8;
                                let g = (existing[1] as f32 * (1.0 - intensity * 0.15)) as u8;
                                img.put_pixel(
                                    bx,
                                    by,
                                    image::Rgba([r, g, existing[2], existing[3]]),
                                );
                            }
                        }
                    }
                }
            }
        }

        let frame_path = path.join(format!("frame_{frame_idx:03}.png"));
        if let Err(e) = img.save(&frame_path) {
            tracing::warn!("Failed to save demo frame {}: {e}", frame_path.display());
        }
    }

    tracing::info!("Generated {TOTAL_FRAMES} slime demo frames ({SIZE}×{SIZE})");
}
