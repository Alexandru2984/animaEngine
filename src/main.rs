use anima_engine::app::App;
use anima_engine::config::AppConfig;
use anima_engine::crash::{self, RecoverOutcome};
use anima_engine::event::AnimaEvent;
use anima_engine::scene::Scene;
use anima_engine::single_instance::{self, AcquireOutcome};
use anima_engine::{demo, hotkeys, tray, wayland, window};

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

    // CLI: `anima-engine --recover` restores a crash-recovery snapshot
    // and exits. Anything else runs the app. We deliberately keep the
    // flag handling argv-only (no clap dep) — it's one switch.
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--recover" || a == "-r") {
        match crash::try_recover() {
            RecoverOutcome::NoSnapshot => {
                eprintln!("No crash-recovery snapshot found — nothing to restore.");
                std::process::exit(0);
            }
            RecoverOutcome::Restored { backup } => {
                if let Some(b) = backup {
                    eprintln!(
                        "Snapshot restored. Previous config kept at: {}",
                        b.display()
                    );
                } else {
                    eprintln!("Snapshot restored.");
                }
                std::process::exit(0);
            }
            RecoverOutcome::Failed(e) => {
                eprintln!("Recovery failed: {e}");
                std::process::exit(1);
            }
        }
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        std::process::exit(0);
    }

    // Install before any work — even our own startup can panic.
    crash::install_panic_hook();

    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    tracing::info!("  animaEngine v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("  Linux-first animated desktop overlay engine");
    tracing::info!("  Supported formats: PNG, GIF, WebP (animated), Spritesheets");
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Single-instance handshake — must happen before any work, otherwise
    // a redundant launch would do all the setup just to hand off.
    let mut dbus_connection = match single_instance::try_acquire() {
        AcquireOutcome::Claimed(conn) => conn,
        AcquireOutcome::HandedOff => {
            tracing::info!("Another instance is already running. Asked it to raise.");
            std::process::exit(0);
        }
    };

    window::platform::log_platform_info();
    window::linux::check_compositor();

    // Probe native Wayland capabilities. The result is only logged for now;
    // the native code path will consume it in a later sub-phase.
    let wayland_caps = wayland::detect();
    wayland::log_status(&wayland_caps);

    // First-run demo so users see something on screen. Safe to delete from config.
    demo::generate_assets();

    let config = AppConfig::load();
    tracing::info!(
        "Config loaded: {} characters, playback={}",
        config.characters.len(),
        config.global.playback_enabled
    );

    // i18n: prefer the explicit setting in config; otherwise detect from
    // the user's environment locale. After this call, anima_engine::i18n::t
    // is available everywhere.
    anima_engine::i18n::init(config.global.locale.as_deref());
    tracing::info!("Active locale: {}", anima_engine::i18n::current_locale());
    // Seed the crash-recovery slot so a panic between now and the first
    // user-driven save still produces a useful snapshot.
    crash::record_known_good(&config);

    let scene = Scene::from_config(&config);

    // Opt-in native Wayland path. Requires:
    //   - ANIMA_USE_WAYLAND_NATIVE=1 in the environment
    //   - A compositor that advertises zwlr_layer_shell_v1 (wlroots, sway,
    //     Hyprland, river, …). Mutter / KWin will fail the probe and we
    //     fall through to the X11 path.
    //
    // On success this never returns. On failure we log a warning and
    // continue with winit + XWayland as if the flag weren't set.
    if wayland_caps.layer_shell && std::env::var_os("ANIMA_USE_WAYLAND_NATIVE").is_some() {
        tracing::info!("ANIMA_USE_WAYLAND_NATIVE=1 set — trying native layer-shell path");
        // Wire the D-Bus activation service for the Wayland path so
        // compositor bindings (sway/Hyprland) can dispatch the same
        // actions the X11 path's global hotkeys produce.
        let dbus_rx = dbus_connection
            .take()
            .map(single_instance::install_wayland_service);
        // T.2: the portal is the only global-hotkey mechanism that
        // exists on the native path — XGrabKey has no X server here.
        let portal_strategy = hotkeys::probe::resolve(
            config.global.hotkey_backend,
            hotkeys::probe::portal_version(),
            false,
        );
        tracing::info!("Hotkey strategy (native): {}", portal_strategy.describe());
        let portal_rx = match portal_strategy {
            hotkeys::probe::HotkeyStrategy::Portal { .. } => {
                Some(hotkeys::portal::spawn_bg(&config.keybindings))
            }
            _ => None,
        };
        match wayland::run_native(scene, config.clone(), dbus_rx, portal_rx) {
            Ok(()) => {
                tracing::info!("Native Wayland session ended cleanly.");
                return;
            }
            Err(e) => {
                tracing::warn!("Native Wayland init failed: {e}. Falling back to X11 path.");
                // Re-load scene because the previous one was consumed.
                let scene = Scene::from_config(&config);
                run_winit_path(config, scene, dbus_connection);
                return;
            }
        }
    }

    run_winit_path(config, scene, dbus_connection);
}

/// X11 / XWayland path — the default. Factored into its own function so
/// the native-Wayland branch above can fall back to it cleanly.
fn run_winit_path(config: AppConfig, scene: Scene, dbus_connection: Option<zbus::Connection>) {
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

    // T.2: resolve the hotkey backend (config preference + portal
    // probe) and wire whichever mechanism won. The portal handshake
    // can sit behind a system permission dialog for minutes, so it
    // runs on a background bridge thread — startup never blocks, and
    // a handshake failure triggers the *deferred* XGrabKey fallback
    // (or the warning banner, via AnimaEvent::HotkeysUnavailable).
    let strategy = hotkeys::probe::resolve(
        config.global.hotkey_backend,
        hotkeys::probe::portal_version(),
        // The winit path always has an X server (native or XWayland) —
        // the event loop above was just built with the X11 backend.
        true,
    );
    tracing::info!("Hotkey strategy: {}", strategy.describe());

    let mut hotkeys_available = true;
    let _hotkeys: Option<hotkeys::HotkeyController> = match strategy {
        hotkeys::probe::HotkeyStrategy::Portal { .. } => {
            let rx = hotkeys::portal::spawn_bg(&config.keybindings);
            let proxy = event_loop.create_proxy();
            let bindings = config.keybindings.clone();
            let spawned = std::thread::Builder::new()
                .name("anima-portal-bridge".into())
                .spawn(move || portal_bridge(rx, proxy, &bindings));
            if let Err(e) = spawned {
                tracing::warn!("Portal bridge thread failed to spawn: {e}");
                hotkeys_available = false;
            }
            // Banner decisions for this branch arrive later through
            // AnimaEvent::HotkeysUnavailable — assume available now.
            None
        }
        hotkeys::probe::HotkeyStrategy::X11Grab => {
            // Register the user's globally-bound chords (ToggleEditMode,
            // HideOverlay, PauseAll — anything else with a modifier).
            // The controller must live as long as the app — dropping it
            // un-registers the bindings.
            let ctrl = hotkeys::register(event_loop.create_proxy(), &config.keybindings);
            hotkeys_available = ctrl.is_some();
            ctrl
        }
        hotkeys::probe::HotkeyStrategy::DbusOnly => {
            hotkeys_available = false;
            None
        }
    };

    // Now that we have a proxy, install the single-instance service so a
    // future redundant launch can ask us to raise instead of starting up.
    if let Some(conn) = dbus_connection {
        single_instance::install_service(conn, event_loop.create_proxy());
    }

    let mut app = App::new(config, scene);
    if !hotkeys_available {
        // hotkeys::register returned None — typically a native Wayland
        // session without XGrabKey. The tray + ⚙ button still work;
        // the banner makes the loss discoverable.
        app.push_warning(anima_engine::ui::Warning::GlobalHotkeysUnavailable);
    }

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
    tracing::info!(
        "  Config: {}",
        anima_engine::drop_validate::redact_path(&AppConfig::config_path())
    );
    tracing::debug!("  Config (full): {}", AppConfig::config_path().display());
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    if let Err(e) = event_loop.run_app(&mut app) {
        tracing::error!("Event loop error: {e}");
        std::process::exit(1);
    }
}

fn print_help() {
    eprintln!(
        "animaEngine v{} — animated desktop overlay engine

USAGE:
    anima-engine [OPTIONS]

OPTIONS:
    -h, --help       Print this help and exit
    -r, --recover    Restore a crash-recovery snapshot over the live
                     config (the live config is backed up to
                     config.toml.bak), then exit.

ENVIRONMENT:
    RUST_LOG=anima_engine=debug    Verbose logs
    ANIMA_NO_CACHE=1               Bypass the on-disk RGBA cache
    ANIMA_USE_WAYLAND_NATIVE=1     Try native wlr-layer-shell

See README.md for more.",
        env!("CARGO_PKG_VERSION")
    );
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

/// Bridge the portal message stream onto the winit event loop. First
/// message decides: `Ready` starts the activation pump; anything else
/// runs the deferred XGrabKey fallback, and only when that also fails
/// does the warning banner fire. Runs for the process lifetime.
fn portal_bridge(
    rx: std::sync::mpsc::Receiver<hotkeys::portal::PortalMsg>,
    proxy: winit::event_loop::EventLoopProxy<AnimaEvent>,
    bindings: &anima_engine::keybindings::KeyBindings,
) {
    use hotkeys::portal::PortalMsg;
    use std::sync::atomic::AtomicBool;

    match rx.recv() {
        Ok(PortalMsg::Ready) => {
            tracing::info!("Portal shortcuts active");
            let visible = AtomicBool::new(true);
            while let Ok(msg) = rx.recv() {
                let PortalMsg::Activated(action) = msg else {
                    continue;
                };
                let Some(ev) = hotkeys::action_to_event(action, &visible) else {
                    continue;
                };
                if proxy.send_event(ev).is_err() {
                    return; // event loop gone — exit with it
                }
            }
            tracing::warn!("Portal channel closed; shortcuts inactive");
            let _ = proxy.send_event(AnimaEvent::HotkeysUnavailable);
        }
        _ => {
            // Failed or sender already dropped → deferred fallback.
            tracing::warn!("Portal unavailable; falling back to XGrabKey");
            match hotkeys::register(proxy.clone(), bindings) {
                Some(_ctrl) => {
                    tracing::info!("XGrabKey fallback active");
                    // The controller un-registers on drop — park this
                    // thread for the process lifetime to keep it alive.
                    loop {
                        std::thread::park();
                    }
                }
                None => {
                    let _ = proxy.send_event(AnimaEvent::HotkeysUnavailable);
                }
            }
        }
    }
}
