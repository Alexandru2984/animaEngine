//! Persistent warning banner (D.5).
//!
//! Distinct from [`crate::ui::toasts::ToastQueue`] in two ways:
//! transient toasts auto-expire after 3-6 s; a banner stays visible
//! until the underlying condition clears or the user dismisses it.
//! That's the right tool for session-lifetime gripes — "global
//! hotkeys couldn't register on this Wayland session" — that a
//! 3-second toast would let the user miss while they were tabbed
//! into another window.
//!
//! Banners live in [`App.warnings`](crate::app::App), a `BTreeSet`
//! of [`Warning`] variants. Each enum variant identifies a single
//! condition; setting the same warning twice is a no-op. The
//! settings panel renders the active set at the top of its body,
//! before the tab content, so the user sees it the moment they
//! open the panel.

use serde::{Deserialize, Serialize};

/// Stable identifier for one session-lifetime warning. New
/// variants are added when a new "you should know this state
/// exists" surface lands; removed when the underlying source goes
/// away. The `Ord` derive drives display order — variants higher
/// in the declaration land at the top of the banner stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Warning {
    /// `hotkeys::register` returned `None` at startup — typically
    /// because the session is native Wayland without XWayland
    /// fallback, where `XGrabKey` isn't exposed. The tray menu and
    /// the `⚙` button still work; only Ctrl+Shift+A/H/P are dead.
    GlobalHotkeysUnavailable,
    /// The hot-reload worker thread disconnected before delivering a
    /// reload result. Surfacing this matters because the user's
    /// in-flight config edit silently won't apply — they'd think
    /// it took until the next manual save or restart.
    HotReloadDisconnected,
}

impl Warning {
    /// Severity (Warn = caution, Error = blocking).
    pub fn severity(self) -> Severity {
        match self {
            Self::GlobalHotkeysUnavailable => Severity::Warn,
            Self::HotReloadDisconnected => Severity::Warn,
        }
    }

    /// Fluent message id for the banner body. Translations live in
    /// the locale `.ftl` files.
    pub fn i18n_key(self) -> &'static str {
        match self {
            Self::GlobalHotkeysUnavailable => "warning-global-hotkeys-unavailable",
            Self::HotReloadDisconnected => "warning-hot-reload-disconnected",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warn,
    Error,
}
