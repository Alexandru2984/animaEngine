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

/// User preference for the hotkey backend, persisted in
/// `config.global.hotkey_backend`. `Auto` (default) trusts
/// [`choose`]; the explicit values pin a backend for debugging or for
/// desktops where the probe misjudges.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HotkeyBackend {
    #[default]
    Auto,
    Portal,
    X11,
    None,
}

/// Resolve preference + probe results into the strategy to run.
/// Explicit preferences are honored even when the probe disagrees —
/// pinning `portal` on a session without one yields `DbusOnly` after
/// the portal handshake fails (the deferred-fallback path), and that
/// is the user's stated intent.
pub fn resolve(
    pref: HotkeyBackend,
    portal_version: Option<u32>,
    x11_available: bool,
) -> HotkeyStrategy {
    match pref {
        HotkeyBackend::Auto => choose(portal_version, x11_available),
        HotkeyBackend::Portal => HotkeyStrategy::Portal {
            version: portal_version.unwrap_or(0),
        },
        HotkeyBackend::X11 => HotkeyStrategy::X11Grab,
        HotkeyBackend::None => HotkeyStrategy::DbusOnly,
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
    fn resolve_auto_delegates_to_choose() {
        assert_eq!(
            resolve(HotkeyBackend::Auto, Some(2), true),
            HotkeyStrategy::Portal { version: 2 }
        );
        assert_eq!(
            resolve(HotkeyBackend::Auto, None, true),
            HotkeyStrategy::X11Grab
        );
    }

    #[test]
    fn resolve_explicit_overrides_probe() {
        // Pinned x11 ignores an available portal.
        assert_eq!(
            resolve(HotkeyBackend::X11, Some(2), true),
            HotkeyStrategy::X11Grab
        );
        // Pinned none disables even with both available.
        assert_eq!(
            resolve(HotkeyBackend::None, Some(2), true),
            HotkeyStrategy::DbusOnly
        );
        // Pinned portal without a probe hit still tries the portal —
        // the handshake failure then runs the deferred fallback.
        assert_eq!(
            resolve(HotkeyBackend::Portal, None, false),
            HotkeyStrategy::Portal { version: 0 }
        );
    }

    #[test]
    fn backend_serde_round_trip() {
        for (s, v) in [
            ("auto", HotkeyBackend::Auto),
            ("portal", HotkeyBackend::Portal),
            ("x11", HotkeyBackend::X11),
            ("none", HotkeyBackend::None),
        ] {
            let toml_str = format!("backend = \"{s}\"");
            #[derive(serde::Deserialize)]
            struct W {
                backend: HotkeyBackend,
            }
            let w: W = toml::from_str(&toml_str).unwrap();
            assert_eq!(w.backend, v);
        }
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
