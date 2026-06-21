use std::env;

/// Detected display server type
#[derive(Debug, Clone, PartialEq)]
pub enum DisplayServer {
    X11,
    Wayland,
    Unknown(String),
}

impl std::fmt::Display for DisplayServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisplayServer::X11 => write!(f, "X11"),
            DisplayServer::Wayland => write!(f, "Wayland"),
            DisplayServer::Unknown(s) => write!(f, "Unknown ({})", s),
        }
    }
}

/// Detect the current display server
pub fn detect_display_server() -> DisplayServer {
    // Check XDG_SESSION_TYPE first (most reliable on modern systems)
    if let Ok(session_type) = env::var("XDG_SESSION_TYPE") {
        match session_type.to_lowercase().as_str() {
            "x11" => return DisplayServer::X11,
            "wayland" => return DisplayServer::Wayland,
            other => return DisplayServer::Unknown(other.to_string()),
        }
    }

    // Fallback: check for Wayland-specific env vars
    if env::var("WAYLAND_DISPLAY").is_ok() {
        return DisplayServer::Wayland;
    }

    // Fallback: check for X11-specific env vars
    if env::var("DISPLAY").is_ok() {
        return DisplayServer::X11;
    }

    DisplayServer::Unknown("no display detected".to_string())
}

/// Log platform information and any relevant warnings.
///
/// `native_wayland_active` is the same `layer_shell && ANIMA_USE_WAYLAND_NATIVE`
/// check `main()` uses to decide whether to try the native layer-shell
/// path — passed in (not re-derived here) so this can never drift from
/// the actual decision. On that path click-through, positioning, and
/// always-on-top all work as documented (a fullscreen `Overlay` layer
/// surface with `wl_surface::set_input_region`), so the generic
/// XWayland caveats below would be actively wrong advice.
pub fn log_platform_info(native_wayland_active: bool) {
    let server = detect_display_server();
    tracing::info!("Display server: {}", server);

    if let Ok(desktop) = env::var("XDG_CURRENT_DESKTOP") {
        tracing::info!("Desktop environment: {}", desktop);
    }

    match server {
        DisplayServer::Wayland if native_wayland_active => {
            tracing::info!(
                "Wayland session with layer-shell support — using the native \
                 Wayland path (ANIMA_USE_WAYLAND_NATIVE=1). Click-through and \
                 overlay positioning work natively here, no XWayland caveats."
            );
        }
        DisplayServer::Wayland => {
            tracing::warn!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            tracing::warn!("Running on Wayland. Some features may be limited:");
            tracing::warn!("  • Absolute window positioning may not work");
            tracing::warn!("  • Always-on-top behavior depends on compositor");
            tracing::warn!("  • Click-through is not supported");
            tracing::warn!("For best results, run under X11, or try the native");
            tracing::warn!("Wayland path on wlroots compositors (sway, Hyprland, river):");
            tracing::warn!("  GDK_BACKEND=x11 cargo run");
            tracing::warn!("  ANIMA_USE_WAYLAND_NATIVE=1 cargo run");
            tracing::warn!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        }
        DisplayServer::Unknown(_) => {
            tracing::warn!(
                "Could not detect display server. Overlay features may not work correctly."
            );
        }
        _ => {}
    }
}
