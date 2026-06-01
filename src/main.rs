use anima_engine::app::App;
use anima_engine::config::AppConfig;
use anima_engine::scene::Scene;
use anima_engine::window;

// Force X11 backend — XWayland on Wayland systems.
// This is required because:
// 1. X11 supports _NET_WM_WINDOW_TYPE_DOCK (true always-on-top overlay)
// 2. X11 supports _NET_WM_STATE_ABOVE reliably
// 3. X11 supports input shape (click-through) via XShape
// 4. Wayland compositors don't provide reliable always-on-top for overlay apps
// XWayland is available on virtually all Wayland systems.
#[cfg(target_os = "linux")]
use winit::platform::x11::EventLoopBuilderExtX11;

fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log::info!("  animaEngine v{}", env!("CARGO_PKG_VERSION"));
    log::info!("  Linux-first animated desktop overlay engine");
    log::info!("  Supported formats: PNG, GIF, WebP (animated), Spritesheets");
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Detect and log platform info
    window::platform::log_platform_info();
    window::linux::check_compositor();

    // Generate demo assets if they don't exist
    generate_demo_assets();

    // Load config (auto-creates default on first run)
    let config = AppConfig::load();
    log::info!(
        "Config loaded: {} characters, playback={}",
        config.characters.len(),
        config.global.playback_enabled
    );

    // Build scene from config
    let scene = Scene::from_config(&config);

    // Create event loop — force X11 backend for reliable overlay support.
    // On Wayland systems, this uses XWayland which supports all the window
    // management hints we need (DOCK type, always-on-top, click-through).
    #[cfg(target_os = "linux")]
    let event_loop = {
        let mut builder = winit::event_loop::EventLoop::builder();
        builder.with_x11();
        match builder.build() {
            Ok(el) => {
                log::info!("Event loop created with X11 backend (XWayland if on Wayland)");
                el
            }
            Err(e) => {
                log::error!("Failed to create X11 event loop: {}", e);
                log::info!("Falling back to default event loop...");
                match winit::event_loop::EventLoop::new() {
                    Ok(el) => el,
                    Err(e2) => {
                        log::error!("Failed to create fallback event loop: {}", e2);
                        std::process::exit(1);
                    }
                }
            }
        }
    };

    #[cfg(not(target_os = "linux"))]
    let event_loop = match winit::event_loop::EventLoop::new() {
        Ok(el) => el,
        Err(e) => {
            log::error!("Failed to create event loop: {}", e);
            std::process::exit(1);
        }
    };

    let mut app = App::new(config, scene);

    log::info!("Starting event loop...");
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log::info!("  PASS-THROUGH MODE (default)");
    log::info!("  Clicks go through to desktop. Characters float on top.");
    log::info!("");
    log::info!("  ⚙ Click the button in the top-right corner to toggle EDIT MODE");
    log::info!("");
    log::info!("  In edit mode:");
    log::info!("    Click+Drag  — Move characters");
    log::info!("    Escape      — Return to pass-through mode");
    log::info!("    Space       — Toggle play/pause animations");
    log::info!("    S           — Save config");
    log::info!("    Q           — Save and quit");
    log::info!("");
    log::info!("  Config: {}", AppConfig::config_path().display());
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    if let Err(e) = event_loop.run_app(&mut app) {
        log::error!("Event loop error: {}", e);
        std::process::exit(1);
    }
}

/// Generate elaborate demo PNG assets if they don't exist.
/// Creates visually appealing 128×128 sprites with proper animation.
fn generate_demo_assets() {
    generate_ghost_assets();
    generate_slime_assets();
}

/// Generate a ghost sprite: floating translucent figure with wavy tail, expressive eyes.
/// 8 frames with float up/down + tail wave animation.
fn generate_ghost_assets() {
    let dir_path = "assets/demo/ghost";
    let path = std::path::Path::new(dir_path);
    if path.exists() {
        // Delete old 64px assets to regenerate at 128px
        let first_frame = path.join("frame_001.png");
        if let Ok(meta) = image::image_dimensions(&first_frame) {
            if meta.0 >= 128 {
                log::debug!("Demo ghost assets already exist at 128px: {}", dir_path);
                return;
            }
        }
        log::info!("Regenerating ghost demo assets at 128×128...");
        let _ = std::fs::remove_dir_all(path);
    }

    log::info!("Generating ghost demo assets (128×128, 8 frames)...");
    if let Err(e) = std::fs::create_dir_all(path) {
        log::warn!("Failed to create demo directory {}: {}", dir_path, e);
        return;
    }

    let size: u32 = 128;
    let cx = size as f32 / 2.0;
    let total_frames = 8;

    for frame_idx in 1..=total_frames {
        let mut img = image::RgbaImage::new(size, size);
        let phase = (frame_idx as f32 - 1.0) * std::f32::consts::TAU / total_frames as f32;

        // Float offset (up/down bobbing)
        let float_y = phase.sin() * 4.0;

        // Ghost body center
        let body_cx = cx;
        let body_cy = 42.0 + float_y;
        let body_rx = 32.0; // horizontal radius
        let body_ry = 28.0; // vertical radius

        for y in 0..size {
            for x in 0..size {
                let fx = x as f32;
                let fy = y as f32;

                // Ghost body (elliptical top half + wavy bottom)
                let dx = (fx - body_cx) / body_rx;
                let dy = (fy - body_cy) / body_ry;
                let dist_sq = dx * dx + dy * dy;

                // Body region: ellipse top, rectangle+wave bottom
                let in_head = dist_sq < 1.0 && fy <= body_cy + body_ry * 0.5;

                // Tail section — extends below the head with wavy bottom edge
                let tail_top = body_cy;
                let tail_bottom = body_cy + 50.0 + float_y * 0.5;
                let tail_half_w =
                    body_rx * (1.0 - ((fy - tail_top) / (tail_bottom - tail_top)).max(0.0) * 0.15);

                // Wavy bottom edge
                let wave_freq = 3.0;
                let wave_amp = 5.0 + (phase * 0.5).sin() * 2.0;
                let wave_offset =
                    (fx / size as f32 * wave_freq * std::f32::consts::TAU + phase * 2.0).sin()
                        * wave_amp;
                let effective_bottom = tail_bottom + wave_offset;

                let in_tail =
                    fy > tail_top && fy < effective_bottom && (fx - body_cx).abs() < tail_half_w;

                let in_body = in_head || in_tail;

                if in_body {
                    // Ghost gradient: lighter at center, darker at edges
                    let center_dist = ((fx - body_cx).powi(2) + (fy - body_cy).powi(2)).sqrt();
                    let max_dist = 60.0;
                    let brightness = 1.0 - (center_dist / max_dist).min(1.0) * 0.25;

                    // Base ghost color: pale blue-white
                    let r = (210.0 * brightness) as u8;
                    let g = (215.0 * brightness) as u8;
                    let b = (240.0 * brightness) as u8;

                    // Semi-transparent ghost
                    let alpha_base = 160u8;
                    // Fade out at bottom (tail)
                    let tail_fade = if fy > tail_top {
                        let progress = (fy - tail_top) / (effective_bottom - tail_top);
                        1.0 - progress * 0.6
                    } else {
                        1.0
                    };
                    let a = (alpha_base as f32 * tail_fade).max(30.0) as u8;

                    img.put_pixel(x, y, image::Rgba([r, g, b, a]));
                } else if in_head || dist_sq < 1.15 {
                    // Anti-aliased edge glow
                    let edge = ((1.15 - dist_sq) / 0.15).max(0.0);
                    let a = (40.0 * edge) as u8;
                    if a > 0 {
                        img.put_pixel(x, y, image::Rgba([200, 210, 240, a]));
                    }
                }
            }
        }

        // Eyes — large expressive anime-style
        let eye_y = body_cy - 2.0 + float_y * 0.2;
        let eye_spacing = 12.0;
        // Eye blink on frame 5
        let is_blink = frame_idx == 5;

        for side in [-1.0f32, 1.0f32] {
            let eye_cx = body_cx + side * eye_spacing;

            if is_blink {
                // Blink: draw a horizontal line
                for dx in -5i32..=5 {
                    let ex = (eye_cx + dx as f32) as u32;
                    let ey = eye_y as u32;
                    if ex < size && ey < size {
                        img.put_pixel(ex, ey, image::Rgba([30, 30, 60, 240]));
                    }
                }
            } else {
                // Full eye: white sclera + dark pupil + highlight
                let eye_r = 7.0;
                let pupil_r = 4.0;
                let highlight_r = 2.0;

                for dy in -(eye_r as i32)..=(eye_r as i32) {
                    for dx in -(eye_r as i32)..=(eye_r as i32) {
                        let ex = (eye_cx + dx as f32) as u32;
                        let ey = (eye_y + dy as f32) as u32;
                        if ex >= size || ey >= size {
                            continue;
                        }

                        let dist = ((dx * dx + dy * dy) as f32).sqrt();

                        // Highlight (top-right of pupil)
                        let hdx = dx as f32 - 2.0;
                        let hdy = dy as f32 + 2.0;
                        let h_dist = (hdx * hdx + hdy * hdy).sqrt();

                        if h_dist < highlight_r {
                            img.put_pixel(ex, ey, image::Rgba([255, 255, 255, 250]));
                        } else if dist < pupil_r {
                            // Dark pupil
                            img.put_pixel(ex, ey, image::Rgba([20, 20, 50, 240]));
                        } else if dist < eye_r {
                            // White sclera with soft edge
                            let edge = (eye_r - dist) / 1.5;
                            let a = (220.0 * edge.min(1.0)) as u8;
                            img.put_pixel(ex, ey, image::Rgba([240, 240, 255, a]));
                        }
                    }
                }
            }
        }

        // Cute mouth — small "o" shape
        let mouth_y = body_cy + 10.0 + float_y * 0.2;
        let mouth_r = 3.0;
        for dy in -(mouth_r as i32)..=(mouth_r as i32) {
            for dx in -(mouth_r as i32)..=(mouth_r as i32) {
                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                if dist > mouth_r - 1.5 && dist < mouth_r {
                    let ex = (body_cx + dx as f32) as u32;
                    let ey = (mouth_y + dy as f32) as u32;
                    if ex < size && ey < size {
                        img.put_pixel(ex, ey, image::Rgba([60, 60, 100, 180]));
                    }
                }
            }
        }

        let frame_path = path.join(format!("frame_{:03}.png", frame_idx));
        if let Err(e) = img.save(&frame_path) {
            log::warn!("Failed to save demo frame {}: {}", frame_path.display(), e);
        }
    }

    log::info!("Generated 8 ghost demo frames (128×128)");
}

/// Generate a slime sprite: bouncy blob with squash-stretch, chibi eyes, mouth.
/// 6 frames with squash-stretch bounce animation.
fn generate_slime_assets() {
    let dir_path = "assets/demo/slime";
    let path = std::path::Path::new(dir_path);
    if path.exists() {
        let first_frame = path.join("frame_001.png");
        if let Ok(meta) = image::image_dimensions(&first_frame) {
            if meta.0 >= 128 {
                log::debug!("Demo slime assets already exist at 128px: {}", dir_path);
                return;
            }
        }
        log::info!("Regenerating slime demo assets at 128×128...");
        let _ = std::fs::remove_dir_all(path);
    }

    log::info!("Generating slime demo assets (128×128, 6 frames)...");
    if let Err(e) = std::fs::create_dir_all(path) {
        log::warn!("Failed to create demo directory {}: {}", dir_path, e);
        return;
    }

    let size: u32 = 128;
    let cx = size as f32 / 2.0;
    let total_frames = 6;

    for frame_idx in 1..=total_frames {
        let mut img = image::RgbaImage::new(size, size);
        let phase = (frame_idx as f32 - 1.0) * std::f32::consts::TAU / total_frames as f32;

        // Squash-stretch parameters
        // At phase=0: neutral. phase=PI/2: stretch (tall). phase=PI: squash (wide).
        let stretch = phase.sin() * 0.15;
        let sx = 1.0 - stretch; // horizontal scale
        let sy = 1.0 + stretch; // vertical scale

        let base_rx = 38.0;
        let base_ry = 32.0;
        let body_rx = base_rx * sx;
        let body_ry = base_ry * sy;

        // Body sits at bottom — adjust Y so bottom stays anchored
        let body_cy = size as f32 - body_ry - 10.0;
        let body_cx = cx;

        for y in 0..size {
            for x in 0..size {
                let fx = x as f32;
                let fy = y as f32;

                // Slime body: ellipse with flat bottom
                let dx = (fx - body_cx) / body_rx;
                let dy = (fy - body_cy) / body_ry;
                let dist_sq = dx * dx + dy * dy;

                // Only draw the top part as ellipse, bottom is flat
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
                    // Gradient: lighter at top, darker at bottom
                    let vert_progress =
                        ((fy - (body_cy - body_ry)) / (body_ry * 2.0)).clamp(0.0, 1.0);
                    let brightness = 1.0 - vert_progress * 0.35;

                    // Radial gradient for depth
                    let center_dist = ((fx - body_cx).powi(2) + (fy - body_cy).powi(2)).sqrt();
                    let radial = 1.0 - (center_dist / 50.0).min(1.0) * 0.15;

                    let b = brightness * radial;

                    // Base green color
                    let r = (60.0 * b) as u8;
                    let g = (200.0 * b) as u8;
                    let bb = (70.0 * b) as u8;

                    img.put_pixel(x, y, image::Rgba([r, g, bb, 230]));
                } else if is_top_half && dist_sq < 1.08 {
                    // Anti-aliased edge
                    let edge = ((1.08 - dist_sq) / 0.08).max(0.0);
                    let a = (180.0 * edge) as u8;
                    if a > 0 {
                        img.put_pixel(x, y, image::Rgba([50, 180, 60, a]));
                    }
                }
            }
        }

        // Highlight / specular on top-left
        let hl_cx = body_cx - body_rx * 0.3;
        let hl_cy = body_cy - body_ry * 0.4;
        let hl_r = 10.0;
        for dy in -(hl_r as i32)..=(hl_r as i32) {
            for dx in -(hl_r as i32)..=(hl_r as i32) {
                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                if dist < hl_r {
                    let hx = (hl_cx + dx as f32) as u32;
                    let hy = (hl_cy + dy as f32) as u32;
                    if hx < size && hy < size {
                        let intensity = (1.0 - dist / hl_r).powi(2);
                        let a = (120.0 * intensity) as u8;
                        // Blend white highlight
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

        // Eyes — cute chibi style (^_^ when squashed, normal otherwise)
        let eye_y = body_cy - body_ry * 0.15;
        let eye_spacing = body_rx * 0.35;
        let is_squash = stretch < -0.05; // squashed frame = happy ^_^

        for side in [-1.0f32, 1.0f32] {
            let eye_cx = body_cx + side * eye_spacing;

            if is_squash {
                // Happy ^_^ eyes (inverted V)
                for i in -5i32..=5 {
                    let ix = eye_cx + i as f32;
                    let iy = eye_y - (i as f32).abs() * 0.6;
                    let ex = ix as u32;
                    let ey = iy as u32;
                    if ex < size && ey < size {
                        img.put_pixel(ex, ey, image::Rgba([15, 50, 15, 250]));
                        // Thicken the line
                        if ey + 1 < size {
                            img.put_pixel(ex, ey + 1, image::Rgba([15, 50, 15, 200]));
                        }
                    }
                }
            } else {
                // Normal round eyes
                let eye_r = 5.5;
                let pupil_r = 3.0;
                let highlight_r = 1.8;

                for dy in -(eye_r as i32)..=(eye_r as i32) {
                    for ddx in -(eye_r as i32)..=(eye_r as i32) {
                        let ex = (eye_cx + ddx as f32) as u32;
                        let ey = (eye_y + dy as f32) as u32;
                        if ex >= size || ey >= size {
                            continue;
                        }

                        let dist = ((ddx * ddx + dy * dy) as f32).sqrt();

                        // Highlight
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

        // Mouth — cute smile
        let mouth_y = body_cy + body_ry * 0.15;
        let mouth_w = body_rx * 0.35;
        for mx in -(mouth_w as i32)..=(mouth_w as i32) {
            // Curved smile: y offset proportional to x²
            let curve = (mx as f32 / mouth_w).powi(2) * 3.0;
            let my = mouth_y + curve;
            let ex = (body_cx + mx as f32) as u32;
            let ey = my as u32;
            if ex < size && ey < size {
                img.put_pixel(ex, ey, image::Rgba([20, 80, 20, 200]));
                // Thicken
                if ey + 1 < size {
                    img.put_pixel(ex, ey + 1, image::Rgba([20, 80, 20, 150]));
                }
            }
        }

        // Optional: cute blush marks
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
                        if bx < size && by < size {
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

        let frame_path = path.join(format!("frame_{:03}.png", frame_idx));
        if let Err(e) = img.save(&frame_path) {
            log::warn!("Failed to save demo frame {}: {}", frame_path.display(), e);
        }
    }

    log::info!("Generated 6 slime demo frames (128×128)");
}
