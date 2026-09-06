//! EWMH desktop-window snapshots for window-awareness (X11 only).
//!
//! Polls `_NET_CLIENT_LIST` and per-window EWMH properties to produce
//! the [`PlatformRect`] list `crate::platforms` consumes. One pooled
//! connection, same pattern as `X11InputManager`.
//!
//! What qualifies as a platform:
//! - `_NET_WM_WINDOW_TYPE` is NORMAL (or the property is absent, which
//!   EWMH says to treat as normal). Docks, menus, tooltips — and our
//!   own dock-type overlay windows — are excluded by type.
//! - not `_NET_WM_STATE_HIDDEN` (minimized windows have no edges).
//!
//! Geometry is the WM frame rectangle: client geometry translated to
//! root coordinates, expanded by `_NET_FRAME_EXTENTS` so the entity
//! stands on the title bar, not inside it.
//!
//! Per-window errors are skipped silently — windows come and go
//! between the list query and the property reads; a torn snapshot is
//! one poll (~300 ms) from being corrected.
//!
//! On XWayland this connects and works, but only X11 clients appear
//! in the list — native Wayland windows are invisible to it. The
//! config docs call this out; the protocol simply offers no global
//! window geometry on Wayland.

use crate::platforms::PlatformRect;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{Atom, AtomEnum, ConnectionExt, Window};
use x11rb::rust_connection::RustConnection;

/// Upper bound on desktop windows considered in one poll.
///
/// `snapshot` runs several *synchronous* X11 round-trips per entry
/// (window type, state, geometry, frame extents), and the watcher polls
/// on the UI thread every 300 ms — so an unbounded `_NET_CLIENT_LIST`
/// turns into thousands of blocking requests per second. Any local
/// application can extend that list, so its length is not ours to trust.
/// Window-awareness only needs the windows a sprite could plausibly rest
/// on; beyond this many, the extra rects change nothing visible.
const MAX_WATCHED_WINDOWS: usize = 256;

/// Pooled connection + interned atoms for the EWMH window walk.
pub struct WindowWatcher {
    conn: RustConnection,
    root: Window,
    net_client_list: Atom,
    net_wm_window_type: Atom,
    net_wm_window_type_normal: Atom,
    net_wm_state: Atom,
    net_wm_state_hidden: Atom,
    net_frame_extents: Atom,
}

impl WindowWatcher {
    /// Connect and intern the atoms. `None` when no X server is
    /// reachable (native Wayland session) — the feature then stays
    /// inert without further checks.
    pub fn new() -> Option<Self> {
        let (conn, screen_num) = x11rb::connect(None).ok()?;
        let root = conn.setup().roots[screen_num].root;

        let intern = |name: &str| -> Option<Atom> {
            conn.intern_atom(false, name.as_bytes())
                .ok()?
                .reply()
                .ok()
                .map(|r| r.atom)
        };
        let net_client_list = intern("_NET_CLIENT_LIST")?;
        let net_wm_window_type = intern("_NET_WM_WINDOW_TYPE")?;
        let net_wm_window_type_normal = intern("_NET_WM_WINDOW_TYPE_NORMAL")?;
        let net_wm_state = intern("_NET_WM_STATE")?;
        let net_wm_state_hidden = intern("_NET_WM_STATE_HIDDEN")?;
        let net_frame_extents = intern("_NET_FRAME_EXTENTS")?;

        tracing::info!("WindowWatcher: X11 connection up, EWMH atoms interned");
        Some(Self {
            conn,
            root,
            net_client_list,
            net_wm_window_type,
            net_wm_window_type_normal,
            net_wm_state,
            net_wm_state_hidden,
            net_frame_extents,
        })
    }

    /// Snapshot every qualifying desktop window as a platform rect in
    /// global desktop coordinates. Errors collapse to an empty / short
    /// list — the next poll retries from scratch.
    pub fn snapshot(&self) -> Vec<PlatformRect> {
        let Some(windows) = self.read_window_list() else {
            return Vec::new();
        };
        windows
            .into_iter()
            // Defence in depth: the request below already bounds the
            // list, but `platform_rect` costs several blocking X11
            // round-trips each, so never iterate an unbounded one.
            .take(MAX_WATCHED_WINDOWS)
            .filter_map(|w| self.platform_rect(w))
            .collect()
    }

    fn read_window_list(&self) -> Option<Vec<Window>> {
        let reply = self
            .conn
            .get_property(
                false,
                self.root,
                self.net_client_list,
                AtomEnum::WINDOW,
                0,
                // `long_length` counts 4-byte units and a window id is
                // exactly one, so this asks for at most
                // MAX_WATCHED_WINDOWS ids rather than `u32::MAX` — which
                // requested a 16 GiB ceiling and left the bound entirely
                // up to the server.
                MAX_WATCHED_WINDOWS as u32,
            )
            .ok()?
            .reply()
            .ok()?;
        let windows: Vec<Window> = reply.value32()?.collect();
        if windows.len() >= MAX_WATCHED_WINDOWS {
            tracing::debug!(
                "_NET_CLIENT_LIST hit the {MAX_WATCHED_WINDOWS}-window cap; \
                 ignoring the rest for window-awareness"
            );
        }
        Some(windows)
    }

    /// Atom-array property reader (window type, state).
    fn read_atoms(&self, window: Window, prop: Atom) -> Vec<Atom> {
        self.conn
            .get_property(false, window, prop, AtomEnum::ATOM, 0, 32)
            .ok()
            .and_then(|c| c.reply().ok())
            .and_then(|r| r.value32().map(|v| v.collect()))
            .unwrap_or_default()
    }

    fn platform_rect(&self, window: Window) -> Option<PlatformRect> {
        // Type filter: absent property counts as NORMAL per EWMH §1.4.
        let types = self.read_atoms(window, self.net_wm_window_type);
        if !types.is_empty() && !types.contains(&self.net_wm_window_type_normal) {
            return None;
        }
        if self
            .read_atoms(window, self.net_wm_state)
            .contains(&self.net_wm_state_hidden)
        {
            return None;
        }

        let geom = self.conn.get_geometry(window).ok()?.reply().ok()?;
        let trans = self
            .conn
            .translate_coordinates(window, self.root, 0, 0)
            .ok()?
            .reply()
            .ok()?;

        // Frame extents (left, right, top, bottom); absent → zeros
        // (undecorated or non-reparenting WM).
        let extents = self
            .conn
            .get_property(
                false,
                window,
                self.net_frame_extents,
                AtomEnum::CARDINAL,
                0,
                4,
            )
            .ok()
            .and_then(|c| c.reply().ok())
            .and_then(|r| r.value32().map(|v| v.collect::<Vec<u32>>()))
            .filter(|v| v.len() == 4)
            .unwrap_or_else(|| vec![0; 4]);
        // `_NET_FRAME_EXTENTS` is set by the WM but any X11 client can
        // write it. A real titlebar/border is at most a few hundred px;
        // clamp so a hostile or buggy `u32::MAX` can't blow the platform
        // rect (and the physics layer that rides on it) up to absurd
        // sizes. X11 is in the trust boundary, but cheap to harden.
        const MAX_FRAME_EXTENT: u32 = 1024;
        let clamp = |v: u32| v.min(MAX_FRAME_EXTENT) as f32;
        let (left, right, top, bottom) = (
            clamp(extents[0]),
            clamp(extents[1]),
            clamp(extents[2]),
            clamp(extents[3]),
        );

        Some(PlatformRect {
            x: trans.dst_x as f32 - left,
            y: trans.dst_y as f32 - top,
            w: geom.width as f32 + left + right,
            h: geom.height as f32 + top + bottom,
        })
    }
}
