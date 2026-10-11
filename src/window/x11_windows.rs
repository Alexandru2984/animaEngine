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

/// Whether the window in front is full screen, from EWMH — for stepping
/// aside (`crate::fullscreen`). The active window is `_NET_ACTIVE_WINDOW`
/// on the root; it counts when its type is NORMAL (or absent) and its
/// `_NET_WM_STATE` holds `_NET_WM_STATE_FULLSCREEN`. The overlay's own
/// windows are docks, so they never count. On XWayland only X11 clients
/// are seen, as for window-awareness.
pub struct FullscreenWatch {
    conn: RustConnection,
    root: Window,
    net_active_window: Atom,
    net_client_list: Atom,
    net_wm_window_type: Atom,
    net_wm_window_type_normal: Atom,
    net_wm_state: Atom,
    net_wm_state_fullscreen: Atom,
}

impl FullscreenWatch {
    /// Connect and intern the atoms. `None` without an X server.
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
        let net_active_window = intern("_NET_ACTIVE_WINDOW")?;
        let net_client_list = intern("_NET_CLIENT_LIST")?;
        let net_wm_window_type = intern("_NET_WM_WINDOW_TYPE")?;
        let net_wm_window_type_normal = intern("_NET_WM_WINDOW_TYPE_NORMAL")?;
        let net_wm_state = intern("_NET_WM_STATE")?;
        let net_wm_state_fullscreen = intern("_NET_WM_STATE_FULLSCREEN")?;
        Some(Self {
            conn,
            root,
            net_active_window,
            net_client_list,
            net_wm_window_type,
            net_wm_window_type_normal,
            net_wm_state,
            net_wm_state_fullscreen,
        })
    }

    /// The window in front, if there is one.
    fn active_window(&self) -> Option<Window> {
        self.conn
            .get_property(
                false,
                self.root,
                self.net_active_window,
                AtomEnum::WINDOW,
                0,
                1,
            )
            .ok()
            .and_then(|c| c.reply().ok())
            .and_then(|r| r.value32().and_then(|mut v| v.next()))
            .filter(|&w| w != 0)
    }

    /// A window's app: the two halves of its `WM_CLASS`, instance and
    /// class (`crate::chosen_apps`). Nothing else about it is read.
    fn classes_of(&self, window: Window) -> Vec<String> {
        self.conn
            .get_property(false, window, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 64)
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|r| {
                r.value
                    .split(|&b| b == 0)
                    .filter(|part| !part.is_empty())
                    .map(|part| String::from_utf8_lossy(part).into_owned())
                    .take(2)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Whether the window in front is an app the user chose.
    pub fn chosen_app_in_front(&self) -> bool {
        self.active_window().is_some_and(|w| {
            self.classes_of(w)
                .iter()
                .any(|class| crate::chosen_apps::chosen_now(class))
        })
    }

    /// The apps with a window open: each window's class, the second half
    /// of `WM_CLASS` (the first where there is no second).
    pub fn open_apps(&self) -> Vec<String> {
        let windows: Vec<Window> = self
            .conn
            .get_property(
                false,
                self.root,
                self.net_client_list,
                AtomEnum::WINDOW,
                0,
                1024,
            )
            .ok()
            .and_then(|c| c.reply().ok())
            .and_then(|r| r.value32().map(|v| v.take(256).collect()))
            .unwrap_or_default();
        windows
            .into_iter()
            .filter_map(|w| self.classes_of(w).pop())
            .collect()
    }

    /// Be told of the property changes that can change the answers: which
    /// window is in front (on the root), and the state and type of the one
    /// in front — no longer of `was`, the one before.
    fn listen(&self, was: Option<Window>, now: Option<Window>) {
        use x11rb::protocol::xproto::{ChangeWindowAttributesAux, EventMask};
        let mask = |m: EventMask| ChangeWindowAttributesAux::new().event_mask(m);
        let _ = self
            .conn
            .change_window_attributes(self.root, &mask(EventMask::PROPERTY_CHANGE));
        // Gone already, it answers with an error, read and dropped with
        // the events.
        if let Some(was) = was.filter(|&w| Some(w) != now) {
            let _ = self
                .conn
                .change_window_attributes(was, &mask(EventMask::NO_EVENT));
        }
        if let Some(now) = now {
            let _ = self
                .conn
                .change_window_attributes(now, &mask(EventMask::PROPERTY_CHANGE));
        }
        let _ = self.conn.flush();
    }

    /// Sleep until the X server says something — a property changed — or
    /// `timeout` passes, then take what it said.
    fn wait(&self, timeout: std::time::Duration) {
        use std::os::fd::AsRawFd;
        if !self.drain() {
            let mut pfd = libc::pollfd {
                fd: self.conn.stream().as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            let ms = timeout.as_millis().min(i32::MAX as u128) as i32;
            // SAFETY: one initialised `pollfd` we own for the call; the fd
            // stays open, `self.conn` holding it.
            unsafe { libc::poll(&mut pfd, 1, ms) };
            self.drain();
        }
    }

    /// Read every event and error waiting. Whether there were any.
    fn drain(&self) -> bool {
        let mut any = false;
        while let Ok(Some(_)) = self.conn.poll_for_event() {
            any = true;
        }
        any
    }

    /// Two or three round-trips; any error reads as "not full screen".
    pub fn fullscreen_in_front(&self) -> bool {
        let Some(active) = self.active_window() else {
            return false;
        };
        let atoms = |prop: Atom| -> Vec<Atom> {
            self.conn
                .get_property(false, active, prop, AtomEnum::ATOM, 0, 32)
                .ok()
                .and_then(|c| c.reply().ok())
                .and_then(|r| r.value32().map(|v| v.collect()))
                .unwrap_or_default()
        };
        let types = atoms(self.net_wm_window_type);
        if !types.is_empty() && !types.contains(&self.net_wm_window_type_normal) {
            return false;
        }
        atoms(self.net_wm_state).contains(&self.net_wm_state_fullscreen)
    }
}

/// Look whether the window in front is full screen, or an app the user
/// chose, each time the X server says that may have changed, and tell the
/// event loop when it has. A thread of its own, with its own connection,
/// so the answer comes as quickly while the loop sleeps through an idle
/// scene. It used to ask twice a second, whatever happened — a dozen
/// round-trips a second, all day. Ends when the loop is gone, or at once
/// without an X server.
pub fn spawn_fullscreen_watch(sink: crate::event::EventSink) {
    let spawned = std::thread::Builder::new()
        .name("anima-fullscreen".into())
        .spawn(move || {
            let Some(watch) = FullscreenWatch::new() else {
                return;
            };
            let mut last = None;
            let mut last_chosen = None;
            let mut listed: Option<std::time::Instant> = None;
            let mut in_front = watch.active_window();
            watch.listen(None, in_front);
            loop {
                let now = watch.fullscreen_in_front();
                if last != Some(now) {
                    if !sink.send(crate::event::AnimaEvent::FullscreenInFront(now)) {
                        return;
                    }
                    last = Some(now);
                }
                // An app the user chose (`crate::chosen_apps`).
                let chosen = crate::chosen_apps::any_chosen() && watch.chosen_app_in_front();
                if last_chosen != Some(chosen) {
                    if !sink.send(crate::event::AnimaEvent::ChosenAppInFront(chosen)) {
                        return;
                    }
                    last_chosen = Some(chosen);
                }
                // The open apps, for the picker — only while it shows, at
                // most every two seconds: a round-trip per window.
                if crate::chosen_apps::open_list_wanted()
                    && listed.is_none_or(|t| t.elapsed() >= std::time::Duration::from_secs(2))
                {
                    crate::chosen_apps::set_open(watch.open_apps());
                    listed = Some(std::time::Instant::now());
                }
                // Until something changes: woken by the server, else
                // every two seconds while the picker wants the open apps,
                // and every ten for the chosen apps' list changing.
                watch.wait(std::time::Duration::from_secs(
                    if crate::chosen_apps::open_list_wanted() {
                        2
                    } else {
                        10
                    },
                ));
                let now_in_front = watch.active_window();
                if now_in_front != in_front {
                    watch.listen(in_front, now_in_front);
                    in_front = now_in_front;
                }
            }
        });
    if let Err(e) = spawned {
        tracing::warn!("Full-screen watch thread failed to start: {e}");
    }
}
