//! Linux-specific window utilities and platform notes.
//!
//! # X11 Transparency Requirements
//! - A compositor must be running (Picom, Mutter, KWin, etc.)
//! - GNOME/Mutter provides compositing by default
//! - On bare X11, you may need to start `picom` or `compton`
//!
//! # X11 Always-on-Top
//! - winit uses EWMH `_NET_WM_STATE_ABOVE` hint
//! - Should work on most modern window managers
//!
//! # Click-Through (TODO for future)
//! - X11 supports input shape via XShape extension
//! - Would need x11-dl or xcb crate for direct X11 calls
//! - Not implemented in MVP

/// Check if a compositor is likely running (heuristic).
pub fn check_compositor() {
    // On GNOME/Mutter, compositing is always active
    if let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") {
        let desktop_lower = desktop.to_lowercase();
        if desktop_lower.contains("gnome")
            || desktop_lower.contains("kde")
            || desktop_lower.contains("cinnamon")
            || desktop_lower.contains("mate")
        {
            log::info!(
                "Desktop environment '{}' typically includes a compositor",
                desktop
            );
            return;
        }
    }

    // Check for common compositors
    let compositors = ["picom", "compton", "xcompmgr", "compiz"];
    for comp in &compositors {
        if std::process::Command::new("pgrep")
            .arg("-x")
            .arg(comp)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            log::info!("Compositor detected: {}", comp);
            return;
        }
    }

    log::warn!(
        "No compositor detected. Window transparency may not work. \
         Consider running a compositor like picom."
    );
}
