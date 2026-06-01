use anima_engine::app::App;
use anima_engine::config::AppConfig;
use anima_engine::scene::Scene;
use anima_engine::{demo, window};

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
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log::info!("  animaEngine v{}", env!("CARGO_PKG_VERSION"));
    log::info!("  Linux-first animated desktop overlay engine");
    log::info!("  Supported formats: PNG, GIF, WebP (animated), Spritesheets");
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    window::platform::log_platform_info();
    window::linux::check_compositor();

    // First-run demo so users see something on screen. Safe to delete from config.
    demo::generate_assets();

    let config = AppConfig::load();
    log::info!(
        "Config loaded: {} characters, playback={}",
        config.characters.len(),
        config.global.playback_enabled
    );

    let scene = Scene::from_config(&config);

    // Force X11 backend for reliable overlay support.
    // On Wayland systems, XWayland provides all the window hints we need.
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
                log::error!("Failed to create X11 event loop: {e}");
                log::info!("Falling back to default event loop…");
                match winit::event_loop::EventLoop::new() {
                    Ok(el) => el,
                    Err(e2) => {
                        log::error!("Failed to create fallback event loop: {e2}");
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
            log::error!("Failed to create event loop: {e}");
            std::process::exit(1);
        }
    };

    let mut app = App::new(config, scene);

    log::info!("Starting event loop…");
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
        log::error!("Event loop error: {e}");
        std::process::exit(1);
    }
}
