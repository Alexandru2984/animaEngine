//! Runtime probe for Wayland native + `wlr-layer-shell-unstable-v1`.
//!
//! Heuristics via environment variables can't tell us whether a compositor
//! actually exposes the layer-shell protocol — Plasma 6 advertises `KDE` in
//! `XDG_CURRENT_DESKTOP` but has no native layer-shell, while a custom
//! wlroots setup may have any label. The reliable answer is to round-trip
//! with the compositor and see what globals it advertises.
//!
//! This runs once at startup; it adds ~10 ms when Wayland is present and
//! ~0 ms otherwise (we early-return on missing `WAYLAND_DISPLAY`).

use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::{Connection, Dispatch, QueueHandle};

/// What the probe learned about the current session.
#[derive(Debug, Clone, Copy)]
pub struct WaylandCapabilities {
    /// `WAYLAND_DISPLAY` was set and we successfully connected.
    pub session_present: bool,
    /// Compositor advertises `zwlr_layer_shell_v1`. wlroots-based
    /// compositors (sway, Hyprland, river, wayfire) do; Mutter/KWin don't.
    pub layer_shell: bool,
}

impl WaylandCapabilities {
    /// True when we *could* run a native Wayland overlay end-to-end.
    pub fn fully_capable(&self) -> bool {
        self.session_present && self.layer_shell
    }
}

/// Dummy state — we only need the registry init, not actual event handling.
struct ProbeState;

impl Dispatch<WlRegistry, GlobalListContents> for ProbeState {
    fn event(
        _state: &mut Self,
        _registry: &WlRegistry,
        _event: <WlRegistry as wayland_client::Proxy>::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // Registry events arrive during init; we just need the global list.
    }
}

/// Connect to the compositor (if any) and check the global registry.
/// Returns capabilities; on any failure we fall back to "no session".
pub fn detect() -> WaylandCapabilities {
    // Fast path: no env var → no Wayland.
    if std::env::var_os("WAYLAND_DISPLAY").is_none() {
        return WaylandCapabilities {
            session_present: false,
            layer_shell: false,
        };
    }

    let connection = match Connection::connect_to_env() {
        Ok(c) => c,
        Err(e) => {
            tracing::debug!("Wayland connect failed: {e}");
            return WaylandCapabilities {
                session_present: false,
                layer_shell: false,
            };
        }
    };

    let (globals, _queue) = match registry_queue_init::<ProbeState>(&connection) {
        Ok(g) => g,
        Err(e) => {
            tracing::debug!("Wayland registry init failed: {e}");
            return WaylandCapabilities {
                session_present: true,
                layer_shell: false,
            };
        }
    };

    // Scan for the layer-shell global by interface name. We don't bind it —
    // we just want to know whether it's advertised.
    let layer_shell = globals
        .contents()
        .with_list(|list| list.iter().any(|g| g.interface == "zwlr_layer_shell_v1"));

    WaylandCapabilities {
        session_present: true,
        layer_shell,
    }
}

/// Pretty-print the situation as a single log line at startup.
///
/// `native_active` mirrors the same `layer_shell && ANIMA_USE_WAYLAND_NATIVE`
/// check `main()` uses — passed in so this can't drift from what's
/// actually about to happen. Without it this used to unconditionally
/// claim the native path was "coming in a later sub-phase" even on a
/// run that, two log lines later, switches onto that exact path.
pub fn log_status(caps: &WaylandCapabilities, native_active: bool) {
    if !caps.session_present {
        tracing::info!("Native Wayland: not a Wayland session (or not connectable).");
        return;
    }
    if caps.layer_shell && native_active {
        tracing::info!(
            "Native Wayland: detected wlr-layer-shell — overlay-ready compositor \
             (sway / Hyprland / river / etc.). ANIMA_USE_WAYLAND_NATIVE=1 is set, \
             so this session uses the native layer-shell path, not XWayland."
        );
    } else if caps.layer_shell {
        tracing::info!(
            "Native Wayland: detected wlr-layer-shell — overlay-ready compositor \
             (sway / Hyprland / river / etc.). Still routing through XWayland; \
             set ANIMA_USE_WAYLAND_NATIVE=1 to use the native layer-shell path."
        );
    } else {
        tracing::info!(
            "Native Wayland: session present, but no wlr-layer-shell (likely \
             GNOME Mutter or KDE KWin). XWayland route is the only option here."
        );
    }
}
