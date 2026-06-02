use anima_engine::app::App;
use anima_engine::config::AppConfig;
use anima_engine::event::AnimaEvent;
use anima_engine::scene::Scene;
use anima_engine::{demo, tray, window};

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
    init_tracing();

    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    tracing::info!("  animaEngine v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("  Linux-first animated desktop overlay engine");
    tracing::info!("  Supported formats: PNG, GIF, WebP (animated), Spritesheets");
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    window::platform::log_platform_info();
    window::linux::check_compositor();

    // First-run demo so users see something on screen. Safe to delete from config.
    demo::generate_assets();

    let config = AppConfig::load();
    tracing::info!(
        "Config loaded: {} characters, playback={}",
        config.characters.len(),
        config.global.playback_enabled
    );

    let scene = Scene::from_config(&config);

    // Force X11 backend for reliable overlay support.
    // On Wayland systems, XWayland provides all the window hints we need.
    // Use `with_user_event` so the tray (and future global hotkeys) can
    // post commands back to the UI thread.
    #[cfg(target_os = "linux")]
    let event_loop = {
        let mut builder = winit::event_loop::EventLoop::<AnimaEvent>::with_user_event();
        builder.with_x11();
        match builder.build() {
            Ok(el) => {
                tracing::info!("Event loop created with X11 backend (XWayland if on Wayland)");
                el
            }
            Err(e) => {
                tracing::error!("Failed to create X11 event loop: {e}");
                tracing::info!("Falling back to default event loop…");
                match winit::event_loop::EventLoop::<AnimaEvent>::with_user_event().build() {
                    Ok(el) => el,
                    Err(e2) => {
                        tracing::error!("Failed to create fallback event loop: {e2}");
                        std::process::exit(1);
                    }
                }
            }
        }
    };

    #[cfg(not(target_os = "linux"))]
    let event_loop = match winit::event_loop::EventLoop::<AnimaEvent>::with_user_event().build() {
        Ok(el) => el,
        Err(e) => {
            tracing::error!("Failed to create event loop: {e}");
            std::process::exit(1);
        }
    };

    // Spawn the tray on its own thread. It posts AnimaEvent commands back
    // to us via this proxy; ignore the join handle — the tray dies with
    // the process.
    let _tray_thread = tray::spawn(event_loop.create_proxy());

    let mut app = App::new(config, scene);

    tracing::info!("Starting event loop…");
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    tracing::info!("  PASS-THROUGH MODE (default)");
    tracing::info!("  Clicks go through to desktop. Characters float on top.");
    tracing::info!("");
    tracing::info!("  ⚙ Click the button in the top-right corner to toggle EDIT MODE");
    tracing::info!("");
    tracing::info!("  In edit mode:");
    tracing::info!("    Click+Drag  — Move characters");
    tracing::info!("    Escape      — Return to pass-through mode");
    tracing::info!("    Space       — Toggle play/pause animations");
    tracing::info!("    S           — Save config");
    tracing::info!("    Q           — Save and quit");
    tracing::info!("");
    tracing::info!("  Config: {}", AppConfig::config_path().display());
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    if let Err(e) = event_loop.run_app(&mut app) {
        tracing::error!("Event loop error: {e}");
        std::process::exit(1);
    }
}

/// Initialize tracing-subscriber with millisecond timestamps and RUST_LOG support.
/// Default level: info. Override with `RUST_LOG=debug` (or any standard env-filter syntax).
fn init_tracing() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_target(false)
                .with_timer(fmt::time::uptime()),
        )
        .init();
}
