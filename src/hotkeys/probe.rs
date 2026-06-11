//! Startup probe for the `org.freedesktop.portal.GlobalShortcuts`
//! portal (T.0).
//!
//! The probe answers one question — *can this session bind global
//! shortcuts through the desktop portal?* — by reading the portal
//! interface's `version` property on the session bus. GNOME ≥ 44 and
//! KDE Plasma ≥ 5.27 expose it; wlroots compositors depend on which
//! xdg-desktop-portal backend is installed.
//!
//! T.0 only *logs* the chosen strategy; registration still goes
//! through XGrabKey unconditionally. T.1/T.2 turn the strategy into
//! behavior.

/// Which mechanism global shortcuts will use for this session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyStrategy {
    /// `org.freedesktop.portal.GlobalShortcuts` is available.
    Portal { version: u32 },
    /// No portal, but an X11 display (incl. XWayland) — XGrabKey works.
    X11Grab,
    /// Neither — tray menu + D-Bus methods remain the only triggers.
    DbusOnly,
}

impl HotkeyStrategy {
    /// Human-readable form for the startup log.
    pub fn describe(self) -> String {
        match self {
            Self::Portal { version } => format!("portal (GlobalShortcuts v{version})"),
            Self::X11Grab => "X11 XGrabKey".to_string(),
            Self::DbusOnly => "none (tray + D-Bus methods only)".to_string(),
        }
    }
}

/// Pick the strategy from probe results. Pure so the truth table is
/// unit-testable: the portal wins whenever present (it's the only
/// mechanism that works on GNOME/KDE Wayland *and* it survives
/// sandboxing); XGrabKey is the X11 fallback; otherwise D-Bus only.
pub fn choose(portal_version: Option<u32>, x11_available: bool) -> HotkeyStrategy {
    match (portal_version, x11_available) {
        (Some(version), _) => HotkeyStrategy::Portal { version },
        (None, true) => HotkeyStrategy::X11Grab,
        (None, false) => HotkeyStrategy::DbusOnly,
    }
}

/// Read the GlobalShortcuts portal version from the session bus.
/// `None` covers every failure mode — no bus, no portal service, no
/// GlobalShortcuts interface — because the caller treats them all the
/// same way (fall back).
pub fn portal_version() -> Option<u32> {
    async_io::block_on(async {
        let conn = zbus::Connection::session().await.ok()?;
        let proxy = zbus::Proxy::new(
            &conn,
            "org.freedesktop.portal.Desktop",
            "/org/freedesktop/portal/desktop",
            "org.freedesktop.portal.GlobalShortcuts",
        )
        .await
        .ok()?;
        proxy.get_property::<u32>("version").await.ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portal_wins_over_x11() {
        assert_eq!(choose(Some(2), true), HotkeyStrategy::Portal { version: 2 });
        assert_eq!(
            choose(Some(1), false),
            HotkeyStrategy::Portal { version: 1 }
        );
    }

    #[test]
    fn x11_when_no_portal() {
        assert_eq!(choose(None, true), HotkeyStrategy::X11Grab);
    }

    #[test]
    fn dbus_only_when_neither() {
        assert_eq!(choose(None, false), HotkeyStrategy::DbusOnly);
    }

    #[test]
    fn describe_is_log_friendly() {
        assert_eq!(
            choose(Some(2), true).describe(),
            "portal (GlobalShortcuts v2)"
        );
        assert!(choose(None, false).describe().contains("tray"));
    }
}
