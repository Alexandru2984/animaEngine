//! Procedurally generated ghost demo: 128×128, 8 frames,
//! floating bob + wavy tail + occasional blink.

use std::path::Path;

const DIR: &str = "assets/demo/ghost";
const SIZE: u32 = 128;
const TOTAL_FRAMES: u32 = 8;

pub fn generate() {
    let path = Path::new(DIR);

    if path.exists() {
        if super::assets_already_at_size(path, SIZE) {
            log::debug!("Demo ghost assets already exist at {}px: {}", SIZE, DIR);
            return;
        }
        log::info!("Regenerating ghost demo assets at {SIZE}×{SIZE}…");
        let _ = std::fs::remove_dir_all(path);
    }

    log::info!("Generating ghost demo assets ({SIZE}×{SIZE}, {TOTAL_FRAMES} frames)…");
    if let Err(e) = std::fs::create_dir_all(path) {
        log::warn!("Failed to create demo directory {DIR}: {e}");
        return;
    }

    let cx = SIZE as f32 / 2.0;

    for frame_idx in 1..=TOTAL_FRAMES {
        let mut img = image::RgbaImage::new(SIZE, SIZE);
        let phase = (frame_idx as f32 - 1.0) * std::f32::consts::TAU / TOTAL_FRAMES as f32;

        // Floating bob
        let float_y = phase.sin() * 4.0;

        // Body geometry
        let body_cx = cx;
        let body_cy = 42.0 + float_y;
        let body_rx = 32.0;
        let body_ry = 28.0;

        for y in 0..SIZE {
            for x in 0..SIZE {
                let fx = x as f32;
                let fy = y as f32;

                let dx = (fx - body_cx) / body_rx;
                let dy = (fy - body_cy) / body_ry;
                let dist_sq = dx * dx + dy * dy;

                let in_head = dist_sq < 1.0 && fy <= body_cy + body_ry * 0.5;

                let tail_top = body_cy;
                let tail_bottom = body_cy + 50.0 + float_y * 0.5;
                let tail_half_w =
                    body_rx * (1.0 - ((fy - tail_top) / (tail_bottom - tail_top)).max(0.0) * 0.15);

                let wave_freq = 3.0;
                let wave_amp = 5.0 + (phase * 0.5).sin() * 2.0;
                let wave_offset =
                    (fx / SIZE as f32 * wave_freq * std::f32::consts::TAU + phase * 2.0).sin()
                        * wave_amp;
                let effective_bottom = tail_bottom + wave_offset;

                let in_tail =
                    fy > tail_top && fy < effective_bottom && (fx - body_cx).abs() < tail_half_w;

                let in_body = in_head || in_tail;

                if in_body {
                    let center_dist = ((fx - body_cx).powi(2) + (fy - body_cy).powi(2)).sqrt();
                    let max_dist = 60.0;
                    let brightness = 1.0 - (center_dist / max_dist).min(1.0) * 0.25;

                    let r = (210.0 * brightness) as u8;
                    let g = (215.0 * brightness) as u8;
                    let b = (240.0 * brightness) as u8;

                    let alpha_base = 160u8;
                    let tail_fade = if fy > tail_top {
                        let progress = (fy - tail_top) / (effective_bottom - tail_top);
                        1.0 - progress * 0.6
                    } else {
                        1.0
                    };
                    let a = (alpha_base as f32 * tail_fade).max(30.0) as u8;

                    img.put_pixel(x, y, image::Rgba([r, g, b, a]));
                } else if in_head || dist_sq < 1.15 {
                    let edge = ((1.15 - dist_sq) / 0.15).max(0.0);
                    let a = (40.0 * edge) as u8;
                    if a > 0 {
                        img.put_pixel(x, y, image::Rgba([200, 210, 240, a]));
                    }
                }
            }
        }

        // Eyes (blink on frame 5)
        let eye_y = body_cy - 2.0 + float_y * 0.2;
        let eye_spacing = 12.0;
        let is_blink = frame_idx == 5;

        for side in [-1.0f32, 1.0f32] {
            let eye_cx = body_cx + side * eye_spacing;

            if is_blink {
                for dx in -5i32..=5 {
                    let ex = (eye_cx + dx as f32) as u32;
                    let ey = eye_y as u32;
                    if ex < SIZE && ey < SIZE {
                        img.put_pixel(ex, ey, image::Rgba([30, 30, 60, 240]));
                    }
                }
            } else {
                let eye_r = 7.0;
                let pupil_r = 4.0;
                let highlight_r = 2.0;

                for dy in -(eye_r as i32)..=(eye_r as i32) {
                    for dx in -(eye_r as i32)..=(eye_r as i32) {
                        let ex = (eye_cx + dx as f32) as u32;
                        let ey = (eye_y + dy as f32) as u32;
                        if ex >= SIZE || ey >= SIZE {
                            continue;
                        }

                        let dist = ((dx * dx + dy * dy) as f32).sqrt();

                        let hdx = dx as f32 - 2.0;
                        let hdy = dy as f32 + 2.0;
                        let h_dist = (hdx * hdx + hdy * hdy).sqrt();

                        if h_dist < highlight_r {
                            img.put_pixel(ex, ey, image::Rgba([255, 255, 255, 250]));
                        } else if dist < pupil_r {
                            img.put_pixel(ex, ey, image::Rgba([20, 20, 50, 240]));
                        } else if dist < eye_r {
                            let edge = (eye_r - dist) / 1.5;
                            let a = (220.0 * edge.min(1.0)) as u8;
                            img.put_pixel(ex, ey, image::Rgba([240, 240, 255, a]));
                        }
                    }
                }
            }
        }

        // Mouth
        let mouth_y = body_cy + 10.0 + float_y * 0.2;
        let mouth_r = 3.0;
        for dy in -(mouth_r as i32)..=(mouth_r as i32) {
            for dx in -(mouth_r as i32)..=(mouth_r as i32) {
                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                if dist > mouth_r - 1.5 && dist < mouth_r {
                    let ex = (body_cx + dx as f32) as u32;
                    let ey = (mouth_y + dy as f32) as u32;
                    if ex < SIZE && ey < SIZE {
                        img.put_pixel(ex, ey, image::Rgba([60, 60, 100, 180]));
                    }
                }
            }
        }

        let frame_path = path.join(format!("frame_{frame_idx:03}.png"));
        if let Err(e) = img.save(&frame_path) {
            log::warn!("Failed to save demo frame {}: {e}", frame_path.display());
        }
    }

    log::info!("Generated {TOTAL_FRAMES} ghost demo frames ({SIZE}×{SIZE})");
}
