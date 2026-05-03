mod animation;
mod app;
mod config;
mod entity;
mod input;
mod renderer;
mod scene;
mod window;

use app::App;
use config::AppConfig;
use scene::Scene;

fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log::info!("  animaEngine v{}", env!("CARGO_PKG_VERSION"));
    log::info!("  Linux-first animated desktop overlay engine");
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

    // Create event loop and run app
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
    log::info!("  Clicks go through to the desktop. Characters are visible but non-interactive.");
    log::info!("");
    log::info!("  Controls:");
    log::info!("    F1     — Toggle edit mode (drag characters) / pass-through mode");
    log::info!("    Space  — Toggle play/pause animations");
    log::info!("    S      — Save config");
    log::info!("    Escape — Save and exit");
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    if let Err(e) = event_loop.run_app(&mut app) {
        log::error!("Event loop error: {}", e);
        std::process::exit(1);
    }
}

/// Generate simple demo PNG assets if they don't exist.
/// Creates colored circles with slight frame-to-frame variation.
fn generate_demo_assets() {
    let demo_configs = vec![
        ("assets/demo/ghost", [180, 180, 240, 160], "Ghost"),
        ("assets/demo/slime", [80, 200, 80, 220], "Slime"),
    ];

    for (dir_path, base_color, name) in &demo_configs {
        let path = std::path::Path::new(dir_path);
        if path.exists() {
            log::debug!("Demo assets already exist: {}", dir_path);
            continue;
        }

        log::info!("Generating demo assets for '{}'...", name);

        if let Err(e) = std::fs::create_dir_all(path) {
            log::warn!("Failed to create demo directory {}: {}", dir_path, e);
            continue;
        }

        for frame_idx in 1..=4 {
            let size: u32 = 64;
            let mut img = image::RgbaImage::new(size, size);

            let cx = size as f32 / 2.0;
            let cy = size as f32 / 2.0;
            // Vary the radius per frame for a pulsing/bouncing effect
            let phase = (frame_idx as f32 - 1.0) * std::f32::consts::PI / 2.0;
            let radius = size as f32 * 0.35 + phase.sin() * 4.0;

            for y in 0..size {
                for x in 0..size {
                    let dx = x as f32 - cx;
                    let dy = y as f32 - cy;
                    let dist = (dx * dx + dy * dy).sqrt();

                    let pixel = if dist < radius {
                        // Inside the shape — add a slight gradient for depth
                        let brightness = 1.0 - (dist / radius) * 0.3;
                        let r = (base_color[0] as f32 * brightness).min(255.0) as u8;
                        let g = (base_color[1] as f32 * brightness).min(255.0) as u8;
                        let b = (base_color[2] as f32 * brightness).min(255.0) as u8;
                        image::Rgba([r, g, b, base_color[3]])
                    } else if dist < radius + 2.0 {
                        // Anti-aliased edge
                        let factor = (radius + 2.0 - dist) / 2.0;
                        let a = (base_color[3] as f32 * factor) as u8;
                        image::Rgba([base_color[0], base_color[1], base_color[2], a])
                    } else {
                        // Outside — transparent
                        image::Rgba([0, 0, 0, 0])
                    };

                    img.put_pixel(x, y, pixel);
                }
            }

            // Add simple "eyes" for the ghost
            if *name == "Ghost" {
                let eye_y = cy - 6.0;
                for eye_x in [cx - 8.0, cx + 8.0] {
                    for dy in -3i32..=3 {
                        for dx in -3i32..=3 {
                            let ex = (eye_x + dx as f32) as u32;
                            let ey = (eye_y + dy as f32) as u32;
                            if ex < size && ey < size {
                                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                                if dist < 3.0 {
                                    img.put_pixel(ex, ey, image::Rgba([40, 40, 60, 220]));
                                }
                            }
                        }
                    }
                }
            }

            // Add simple "face" for the slime
            if *name == "Slime" {
                let eye_y = cy - 4.0;
                // Eyes
                for eye_x in [cx - 7.0, cx + 7.0] {
                    for dy in -2i32..=2 {
                        for dx in -2i32..=2 {
                            let ex = (eye_x + dx as f32) as u32;
                            let ey = (eye_y + dy as f32) as u32;
                            if ex < size && ey < size {
                                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                                if dist < 2.5 {
                                    img.put_pixel(ex, ey, image::Rgba([20, 60, 20, 240]));
                                }
                            }
                        }
                    }
                }
                // Mouth
                let mouth_y = (cy + 4.0) as u32;
                for mx in (cx as u32 - 5)..=(cx as u32 + 5) {
                    if mx < size && mouth_y < size {
                        img.put_pixel(mx, mouth_y, image::Rgba([20, 80, 20, 200]));
                    }
                }
            }

            let frame_path = path.join(format!("frame_{:03}.png", frame_idx));
            if let Err(e) = img.save(&frame_path) {
                log::warn!("Failed to save demo frame {}: {}", frame_path.display(), e);
            }
        }

        log::info!("Generated 4 demo frames for '{}'", name);
    }
}
