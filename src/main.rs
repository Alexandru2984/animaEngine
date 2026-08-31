use anima_engine::app::App;
use anima_engine::config::AppConfig;
use anima_engine::crash::{self, RecoverOutcome};
use anima_engine::event::AnimaEvent;
use anima_engine::scene::Scene;
use anima_engine::{demo, hotkeys, window};

// The D-Bus single-instance handshake, the StatusNotifierItem tray and the
// native wlr-layer-shell path are unix-desktop-only. Their Windows
// counterparts (named mutex, Shell_NotifyIcon) land with the backend in C4.
#[cfg(unix)]
use anima_engine::single_instance::{self, AcquireOutcome};
#[cfg(unix)]
use anima_engine::{tray, wayland};

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
    // Reject anything flag-shaped we don't recognise instead of
    // silently launching the overlay: pre-fix, a typo like `--recovr`
    // started the app as if nothing was wrong — the worst possible
    // answer to a user who was explicitly asking for crash recovery.
    let unknown = unknown_flags(&args[1..]);
    if !unknown.is_empty() {
        eprintln!("Unknown option(s): {}\n", unknown.join(", "));
        print_help();
        std::process::exit(2);
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
    #[cfg(unix)]
    let mut instance: InstanceHandle = match single_instance::try_acquire() {
        AcquireOutcome::Claimed(conn) => conn,
        AcquireOutcome::HandedOff => {
            tracing::info!("Another instance is already running. Asked it to raise.");
            std::process::exit(0);
        }
    };
    // No handshake off unix yet: a second launch starts a second overlay
    // until the named-mutex backend lands (C4).
    #[cfg(not(unix))]
    let instance: InstanceHandle = None;

    // Probe native Wayland capabilities before the platform-info log so
    // its Wayland warning can tell whether the native layer-shell path
    // (no XWayland caveats) is actually about to be used, instead of
    // unconditionally telling a sway/Hyprland/river user that
    // click-through doesn't work when it's about to work fine.
    #[cfg(unix)]
    let native_wayland_active = {
        let wayland_caps = wayland::detect();
        let active =
            wayland_caps.layer_shell && std::env::var_os("ANIMA_USE_WAYLAND_NATIVE").is_some();
        wayland::log_status(&wayland_caps, active);
        active
    };
    #[cfg(not(unix))]
    let native_wayland_active = false;

    window::platform::log_platform_info(native_wayland_active);
    #[cfg(unix)]
    {
        window::linux::check_compositor();
    }

    // First-run demo so users see something on screen. Safe to delete from config.
    demo::generate_assets();

    let config = AppConfig::load();
    tracing::info!(
        "Config loaded: {} characters, playback={}",
        config.characters.len(),
        config.global.playback_enabled
    );

    // Bound the on-disk decoded-frame cache (W.2). Editing or swapping
    // assets orphans old cache files; this evicts the oldest once the
    // directory exceeds its cap. Off-thread so a large sweep never
    // delays the window appearing.
    if let Err(e) = std::thread::Builder::new()
        .name("anima-cache-sweep".into())
        .spawn(|| {
            anima_engine::animation::cache::sweep();
        })
    {
        tracing::warn!("Cache-sweep thread failed to spawn: {e}");
    }

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
    #[cfg(unix)]
    {
        if native_wayland_active {
            tracing::info!("ANIMA_USE_WAYLAND_NATIVE=1 set — trying native layer-shell path");
            // Wire the D-Bus activation service for the Wayland path so
            // compositor bindings (sway/Hyprland) can dispatch the same
            // actions the X11 path's global hotkeys produce.
            let dbus_rx = instance
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
                    run_winit_path(config, scene, instance);
                    return;
                }
            }
        }
    }

    run_winit_path(config, scene, instance);
}

/// What the single-instance handshake hands back to be kept alive for the
/// process lifetime: on unix the owned D-Bus connection the activation
/// service is installed on. Off unix there is no handshake yet, so the
/// handle carries nothing — the Windows named mutex lands in C4.
#[cfg(unix)]
type InstanceHandle = Option<zbus::Connection>;
#[cfg(not(unix))]
type InstanceHandle = Option<()>;

/// X11 / XWayland path — the default. Factored into its own function so
/// the native-Wayland branch above can fall back to it cleanly.
fn run_winit_path(config: AppConfig, scene: Scene, instance: InstanceHandle) {
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
    #[cfg(unix)]
    {
        let _tray_thread = tray::spawn(event_loop.create_proxy());
    }

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

    // Each arm yields (controller, available): the controller has to
    // outlive the app — dropping it un-registers every binding — and the
    // flag drives the warning banner below.
    let (_hotkeys, hotkeys_available): (Option<hotkeys::HotkeyController>, bool) = match strategy {
        hotkeys::probe::HotkeyStrategy::Portal { .. } => {
            #[cfg(unix)]
            {
                let rx = hotkeys::portal::spawn_bg(&config.keybindings);
                let proxy = event_loop.create_proxy();
                let bindings = config.keybindings.clone();
                let spawned = std::thread::Builder::new()
                    .name("anima-portal-bridge".into())
                    .spawn(move || portal_bridge(rx, proxy, &bindings));
                let started = match spawned {
                    Ok(_) => true,
                    Err(e) => {
                        tracing::warn!("Portal bridge thread failed to spawn: {e}");
                        false
                    }
                };
                // Banner decisions for this branch arrive later through
                // AnimaEvent::HotkeysUnavailable — assume available now.
                (None, started)
            }
            // Only reachable off unix when the user pinned
            // `hotkey_backend = "portal"` in config — the probe never
            // returns a version there. Say so instead of going silent.
            #[cfg(not(unix))]
            {
                tracing::warn!(
                    "hotkey_backend = portal is unix-only; no global hotkeys this session"
                );
                (None, false)
            }
        }
        hotkeys::probe::HotkeyStrategy::X11Grab => {
            // Register the user's globally-bound chords (ToggleEditMode,
            // HideOverlay, PauseAll — anything else with a modifier).
            let ctrl = hotkeys::register(event_loop.create_proxy(), &config.keybindings);
            let registered = ctrl.is_some();
            (ctrl, registered)
        }
        hotkeys::probe::HotkeyStrategy::DbusOnly => (None, false),
    };

    // Now that we have a proxy, install the single-instance service so a
    // future redundant launch can ask us to raise instead of starting up.
    #[cfg(unix)]
    {
        if let Some(conn) = instance {
            single_instance::install_service(conn, event_loop.create_proxy());
        }
    }
    // Nothing to install off unix yet — the handle is inert until C4.
    #[cfg(not(unix))]
    let _ = instance;

    let mut app = App::new(config, scene);
    app.set_hotkey_backend_status(strategy.describe());
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
#[cfg(unix)]
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
                    let _ = proxy.send_event(AnimaEvent::PortalShortcutsDenied);
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

/// Args that look like flags (`-…`) but aren't ones we know. Positional
/// args pass through untouched (there are none today, but a future
/// `anima-engine scene.toml` shouldn't be rejected by this gate).
/// Extracted from `main` so the decision is unit-testable.
fn unknown_flags(args: &[String]) -> Vec<String> {
    const KNOWN: &[&str] = &["-h", "--help", "-r", "--recover"];
    args.iter()
        .filter(|a| a.starts_with('-') && !KNOWN.contains(&a.as_str()))
        .cloned()
        .collect()
}

#[cfg(test)]
mod cli_tests {
    use super::unknown_flags;

    fn v(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn known_flags_pass() {
        assert!(unknown_flags(&v(&["--recover"])).is_empty());
        assert!(unknown_flags(&v(&["-h", "--help", "-r"])).is_empty());
        assert!(unknown_flags(&v(&[])).is_empty());
    }

    #[test]
    fn typos_and_strangers_are_caught() {
        // The exact failure that motivated this: a --recover typo must
        // not silently launch the overlay.
        assert_eq!(unknown_flags(&v(&["--recovr"])), v(&["--recovr"]));
        assert_eq!(
            unknown_flags(&v(&["--frobnicate", "-x"])),
            v(&["--frobnicate", "-x"])
        );
    }

    #[test]
    fn positional_args_are_not_flags() {
        assert!(unknown_flags(&v(&["scene.toml"])).is_empty());
    }
}
