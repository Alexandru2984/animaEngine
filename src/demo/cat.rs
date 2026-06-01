//! Procedural cat demo: 128×128, 8 frames, tail wave + ear twitch + slow breath.

use std::path::Path;

const DIR: &str = "assets/demo/cat";
const SIZE: u32 = 128;
const TOTAL_FRAMES: u32 = 8;

pub fn generate() {
    let path = Path::new(DIR);

    if path.exists() {
        if super::assets_already_at_size(path, SIZE) {
            tracing::debug!("Demo cat assets already exist at {}px: {}", SIZE, DIR);
            return;
        }
        tracing::info!("Regenerating cat demo assets at {SIZE}×{SIZE}…");
        let _ = std::fs::remove_dir_all(path);
    }

    tracing::info!("Generating cat demo assets ({SIZE}×{SIZE}, {TOTAL_FRAMES} frames)…");
    if let Err(e) = std::fs::create_dir_all(path) {
        tracing::warn!("Failed to create demo directory {DIR}: {e}");
        return;
    }

    let cx = SIZE as f32 / 2.0;

    for frame_idx in 1..=TOTAL_FRAMES {
        let mut img = image::RgbaImage::new(SIZE, SIZE);
        let phase = (frame_idx as f32 - 1.0) * std::f32::consts::TAU / TOTAL_FRAMES as f32;

        // Subtle breath: body expands ±2 % over the cycle.
        let breath = 1.0 + 0.02 * phase.sin();

        // Body — sitting cat silhouette, ellipse on a wider base.
        let body_cx = cx;
        let body_cy = 78.0;
        let body_rx = 30.0 * breath;
        let body_ry = 34.0 * breath;

        // Head — circle on top of body.
        let head_cx = cx;
        let head_cy = 38.0;
        let head_r = 22.0;

        for y in 0..SIZE {
            for x in 0..SIZE {
                let fx = x as f32;
                let fy = y as f32;

                // Tail — sinusoid starting at right side of body.
                let tail_base_x = body_cx + body_rx * 0.6;
                let tail_base_y = body_cy - 4.0;
                let tail_t = ((fx - tail_base_x) / 38.0).clamp(0.0, 1.0);
                let tail_y_offset =
                    -30.0 * tail_t + 6.0 * (tail_t * std::f32::consts::TAU + phase * 2.0).sin();
                let tail_thick = 5.0 * (1.0 - tail_t * 0.4);
                let in_tail = (fx - tail_base_x) > 0.0
                    && (fx - tail_base_x) < 38.0
                    && (fy - (tail_base_y + tail_y_offset)).abs() < tail_thick;

                // Body ellipse.
                let dx = (fx - body_cx) / body_rx;
                let dy = (fy - body_cy) / body_ry;
                let in_body = dx * dx + dy * dy < 1.0;

                // Head circle.
                let head_d = ((fx - head_cx).powi(2) + (fy - head_cy).powi(2)).sqrt();
                let in_head = head_d < head_r;

                // Front legs — two small bumps at the bottom of the body.
                let leg_y = body_cy + body_ry * 0.8;
                let in_left_leg =
                    (fx - (body_cx - 10.0)).abs() < 6.0 && fy > leg_y && fy < leg_y + 16.0;
                let in_right_leg =
                    (fx - (body_cx + 10.0)).abs() < 6.0 && fy > leg_y && fy < leg_y + 16.0;

                if in_body || in_head || in_tail || in_left_leg || in_right_leg {
                    // Orange tabby fur with slight gradient.
                    let depth = ((fy - 20.0) / 100.0).clamp(0.0, 1.0);
                    let r = (240.0 - depth * 50.0) as u8;
                    let g = (150.0 - depth * 60.0) as u8;
                    let b = (60.0 - depth * 30.0) as u8;
                    img.put_pixel(x, y, image::Rgba([r, g, b, 240]));
                }
            }
        }

        // Triangle ears — twitch on frame 4 (one ear flicks).
        let twitch = if frame_idx == 4 { 4.0 } else { 0.0 };
        draw_triangle_ear(&mut img, head_cx - 12.0, head_cy - 14.0, 10.0, 0.0);
        draw_triangle_ear(&mut img, head_cx + 12.0, head_cy - 14.0, 10.0, twitch);

        // Eyes — almond-shaped, blink on frame 7.
        let is_blink = frame_idx == 7;
        for side in [-1.0f32, 1.0f32] {
            let eye_cx = head_cx + side * 7.0;
            let eye_cy = head_cy - 1.0;
            if is_blink {
                for i in -4i32..=4 {
                    let px = (eye_cx + i as f32) as u32;
                    let py = eye_cy as u32;
                    if px < SIZE && py < SIZE {
                        img.put_pixel(px, py, image::Rgba([40, 30, 20, 240]));
                    }
                }
            } else {
                for dy in -3i32..=3 {
                    for dx in -4i32..=4 {
                        let nx = dx as f32 / 4.0;
                        let ny = dy as f32 / 3.0;
                        if nx * nx + ny * ny < 1.0 {
                            let px = (eye_cx + dx as f32) as u32;
                            let py = (eye_cy + dy as f32) as u32;
                            if px < SIZE && py < SIZE {
                                let inner = nx * nx + ny * ny;
                                let (r, g, b) = if inner < 0.35 {
                                    (60, 200, 80) // green pupil
                                } else {
                                    (240, 230, 220) // white sclera
                                };
                                img.put_pixel(px, py, image::Rgba([r, g, b, 245]));
                            }
                        }
                    }
                }
            }
        }

        // Nose — small pink triangle.
        for dy in 0i32..=3 {
            let half = 3 - dy;
            for dx in -half..=half {
                let px = (head_cx + dx as f32) as u32;
                let py = (head_cy + 6.0 + dy as f32) as u32;
                if px < SIZE && py < SIZE {
                    img.put_pixel(px, py, image::Rgba([220, 120, 130, 240]));
                }
            }
        }

        let frame_path = path.join(format!("frame_{frame_idx:03}.png"));
        if let Err(e) = img.save(&frame_path) {
            tracing::warn!("Failed to save demo frame {}: {e}", frame_path.display());
        }
    }

    tracing::info!("Generated {TOTAL_FRAMES} cat demo frames ({SIZE}×{SIZE})");
}

/// Filled triangle pointing up, centered at (cx, cy). `tilt` shears the apex
/// horizontally — used for the ear twitch.
fn draw_triangle_ear(img: &mut image::RgbaImage, cx: f32, cy: f32, height: f32, tilt: f32) {
    let apex_x = cx + tilt;
    let apex_y = cy - height;
    let base_y = cy;
    let base_half = height * 0.6;

    for y in (apex_y as i32)..=(base_y as i32) {
        let t = (y as f32 - apex_y) / (base_y - apex_y).max(0.001);
        let width = base_half * t;
        let row_cx = apex_x + (cx - apex_x) * t;
        for x in ((row_cx - width) as i32)..=((row_cx + width) as i32) {
            if x >= 0 && y >= 0 && (x as u32) < SIZE && (y as u32) < SIZE {
                img.put_pixel(x as u32, y as u32, image::Rgba([230, 140, 60, 240]));
            }
        }
    }
    // Inner ear — smaller triangle, pink.
    let inner_apex_y = apex_y + 3.0;
    for y in (inner_apex_y as i32)..=(base_y as i32 - 2) {
        let t = (y as f32 - inner_apex_y) / (base_y - 2.0 - inner_apex_y).max(0.001);
        let width = (base_half - 2.5) * t;
        let row_cx = apex_x + (cx - apex_x) * t;
        for x in ((row_cx - width) as i32)..=((row_cx + width) as i32) {
            if x >= 0 && y >= 0 && (x as u32) < SIZE && (y as u32) < SIZE {
                img.put_pixel(x as u32, y as u32, image::Rgba([255, 180, 170, 220]));
            }
        }
    }
}
