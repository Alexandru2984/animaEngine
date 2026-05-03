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

/// Log platform information and any relevant warnings
pub fn log_platform_info() {
    let display = detect_display_server();
    log::info!("Display server: {}", display);

    if let Ok(desktop) = env::var("XDG_CURRENT_DESKTOP") {
        log::info!("Desktop environment: {}", desktop);
    }

    match display {
        DisplayServer::Wayland => {
            log::warn!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            log::warn!("Running on Wayland. Some features may be limited:");
            log::warn!("  • Absolute window positioning may not work");
            log::warn!("  • Always-on-top behavior depends on compositor");
            log::warn!("  • Click-through is not supported");
            log::warn!("For best results, run under X11:");
            log::warn!("  GDK_BACKEND=x11 cargo run");
            log::warn!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        }
        DisplayServer::Unknown(_) => {
            log::warn!("Could not detect display server. Overlay features may not work correctly.");
        }
        _ => {}
    }
}
