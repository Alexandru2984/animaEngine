//! Direct X11 input management for overlay windows.
//!
//! On Wayland+XWayland (GNOME/Mutter), DOCK-type windows are treated as system
//! panels and don't receive normal mouse input. This module instead uses:
//!
//! - **Window type `_NET_WM_WINDOW_TYPE_NORMAL`** (not DOCK) so the window
//!   receives mouse events normally through the compositor.
//! - **EWMH hints** (`_NET_WM_STATE_ABOVE`, `_NET_WM_STATE_SKIP_TASKBAR`,
//!   `_NET_WM_STATE_SKIP_PAGER`, `_NET_WM_STATE_STICKY`) to keep the window
//!   always-on-top without appearing in the taskbar.
//! - **X11 Input Shape** to make most of the window click-through while keeping
//!   a control button area clickable.
//!
//! **Connection pooling**: `X11InputManager` holds a single X11 connection that is
//! reused across all operations.

use crate::error::Result;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;

/// Manages X11 window properties and input shape operations with a pooled connection.
pub struct X11InputManager {
    conn: RustConnection,
    x11_window: u32,
    screen_num: usize,
}

impl X11InputManager {
    /// Create a new manager for the given winit window.
    /// Returns `None` if the window is not running on X11.
    ///
    /// Also applies EWMH hints to make the window behave as an overlay:
    /// always-on-top, skip-taskbar, skip-pager, sticky.
    pub fn new(window: &winit::window::Window) -> Option<Self> {
        use raw_window_handle::HasWindowHandle;

        let handle = window.window_handle().ok()?;
        let raw = handle.as_raw();

        if let raw_window_handle::RawWindowHandle::Xlib(xlib_handle) = raw {
            let x11_window = xlib_handle.window as u32;

            match x11rb::connect(None) {
                Ok((conn, screen_num)) => {
                    tracing::info!(
                        "X11InputManager: connection established (window=0x{:x}, screen={})",
                        x11_window,
                        screen_num
                    );

                    let mgr = Self {
                        conn,
                        x11_window,
                        screen_num,
                    };

                    // Apply EWMH overlay hints
                    if let Err(e) = mgr.apply_overlay_hints() {
                        tracing::warn!("Failed to apply overlay EWMH hints: {}", e);
                    }

                    Some(mgr)
                }
                Err(e) => {
                    tracing::warn!("X11InputManager: failed to connect to X11: {}", e);
                    None
                }
            }
        } else {
            tracing::info!("X11InputManager: not running on X11, input shape unavailable");
            None
        }
    }

    /// Apply EWMH hints to make the window behave as an overlay:
    /// - `_NET_WM_STATE_ABOVE` — always on top
    /// - `_NET_WM_STATE_SKIP_TASKBAR` — don't show in taskbar
    /// - `_NET_WM_STATE_SKIP_PAGER` — don't show in pager
    /// - `_NET_WM_STATE_STICKY` — visible on all workspaces
    fn apply_overlay_hints(&self) -> Result<()> {
        let screen = &self.conn.setup().roots[self.screen_num];
        let root = screen.root;

        // Intern the atoms we need
        let net_wm_state = self.intern_atom("_NET_WM_STATE")?;
        let above = self.intern_atom("_NET_WM_STATE_ABOVE")?;
        let skip_taskbar = self.intern_atom("_NET_WM_STATE_SKIP_TASKBAR")?;
        let skip_pager = self.intern_atom("_NET_WM_STATE_SKIP_PAGER")?;
        let sticky = self.intern_atom("_NET_WM_STATE_STICKY")?;

        // Set the property directly
        let states = [above, skip_taskbar, skip_pager, sticky];
        change_property(
            &self.conn,
            PropMode::REPLACE,
            self.x11_window,
            net_wm_state,
            AtomEnum::ATOM,
            32,
            states.len() as u32,
            bytemuck::cast_slice(&states),
        )?;

        // Also send ClientMessage to the WM to activate _NET_WM_STATE_ABOVE
        // (some WMs require this in addition to the property)
        let data = ClientMessageData::from([
            1u32, // _NET_WM_STATE_ADD
            above, 0, 1, // source = normal application
            0,
        ]);
        let event = ClientMessageEvent::new(32, self.x11_window, net_wm_state, data);
        send_event(
            &self.conn,
            false,
            root,
            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
            event,
        )?;

        self.conn.flush()?;

        tracing::info!("EWMH overlay hints applied: ABOVE, SKIP_TASKBAR, SKIP_PAGER, STICKY");
        Ok(())
    }

    /// Re-send the `_NET_WM_STATE_ADD _NET_WM_STATE_ABOVE` request to nudge
    /// the WM to put us back above normal windows.
    ///
    /// `apply_overlay_hints` sets the ABOVE state once at startup, but some
    /// compositors — notably GNOME's Mutter for XWayland clients — quietly
    /// stop honoring it once another window is focused on top of us, so the
    /// overlay sinks behind that window (the "characters go under the app I
    /// clicked" symptom). Re-asserting on each focus/occlusion transition
    /// (see `App::window_event`) keeps reminding the WM without a continuous
    /// timer that would risk a restack flicker-war. This carries no focus
    /// request (it is not `_NET_ACTIVE_WINDOW`), so it never steals focus —
    /// it only affects stacking, which is exactly right for a click-through
    /// overlay that should float above yet let clicks pass through. Logs at
    /// debug since it fires on routine focus changes.
    pub fn reassert_above(&self) -> Result<()> {
        let screen = &self.conn.setup().roots[self.screen_num];
        let root = screen.root;
        let net_wm_state = self.intern_atom("_NET_WM_STATE")?;
        let above = self.intern_atom("_NET_WM_STATE_ABOVE")?;
        let data = ClientMessageData::from([
            1u32, // _NET_WM_STATE_ADD
            above, 0, 1, // source = normal application
            0,
        ]);
        let event = ClientMessageEvent::new(32, self.x11_window, net_wm_state, data);
        send_event(
            &self.conn,
            false,
            root,
            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
            event,
        )?;
        self.conn.flush()?;
        tracing::debug!("Re-asserted _NET_WM_STATE_ABOVE");
        Ok(())
    }

    /// Intern an X11 atom by name.
    fn intern_atom(&self, name: &str) -> Result<u32> {
        let reply = self.conn.intern_atom(false, name.as_bytes())?.reply()?;
        Ok(reply.atom)
    }

    /// Query the pointer position in global (root-window) coordinates
    /// — the same space entity positions live in (T.8) — regardless
    /// of the window's current input shape.
    ///
    /// `FollowCursor` needs this because in the default pass-through
    /// shape, `XShape` makes everything but the toggle button
    /// click-through, so winit's `CursorMoved` never fires there and
    /// the last-tracked position goes stale. `XQueryPointer` reads
    /// the pointer directly from the X server, the same read-only
    /// query any client on the display can already make (see
    /// docs/threat-model.md's "Trusted X11 / Wayland display server"
    /// scope note) — it doesn't depend on which window currently
    /// receives input. Returns `None` on any error; callers already
    /// treat a missing cursor as "stale, skip this tick".
    pub fn query_pointer_global(&self) -> Option<(f32, f32)> {
        let screen = &self.conn.setup().roots[self.screen_num];
        match self.conn.query_pointer(screen.root).ok()?.reply() {
            Ok(reply) => Some((reply.root_x as f32, reply.root_y as f32)),
            Err(e) => {
                tracing::debug!("XQueryPointer failed: {e}");
                None
            }
        }
    }

    /// Set the X11 input shape so that only a rectangle in the top-right corner
    /// receives mouse input. The rest of the window is click-through.
    pub fn set_passthrough_with_button(&mut self, button_size: u32) -> Result<()> {
        // Get window geometry to know where to place the button
        let geom = self.conn.get_geometry(self.x11_window)?.reply()?;
        let win_width = geom.width as i16;

        // Create a pixmap for the input shape
        let pixmap = self.conn.generate_id()?;
        create_pixmap(
            &self.conn,
            1,
            pixmap,
            self.x11_window,
            geom.width,
            geom.height,
        )?;

        // Fill the entire pixmap with 0 (transparent / pass-through)
        let gc = self.conn.generate_id()?;
        create_gc(
            &self.conn,
            gc,
            pixmap,
            &CreateGCAux::new().foreground(0).background(0),
        )?;
        poly_fill_rectangle(
            &self.conn,
            pixmap,
            gc,
            &[Rectangle {
                x: 0,
                y: 0,
                width: geom.width,
                height: geom.height,
            }],
        )?;

        // Draw the button area with 1 (opaque / receives input)
        change_gc(&self.conn, gc, &ChangeGCAux::new().foreground(1))?;
        let button_x = win_width - button_size as i16;
        poly_fill_rectangle(
            &self.conn,
            pixmap,
            gc,
            &[Rectangle {
                x: button_x,
                y: 0,
                width: button_size as u16,
                height: button_size as u16,
            }],
        )?;

        // Apply the input shape
        use x11rb::protocol::shape;
        shape::mask(
            &self.conn,
            shape::SO::SET,
            shape::SK::INPUT,
            self.x11_window,
            0,
            0,
            pixmap,
        )?;

        // Cleanup X resources
        free_gc(&self.conn, gc)?;
        free_pixmap(&self.conn, pixmap)?;
        self.conn.flush()?;

        // Debug, not info: this fires on every mode toggle, resize, focus
        // change, and the periodic click-through self-heal (twice a
        // second in pass-through) — far too chatty for info. The
        // user-facing mode transition is logged separately at info
        // ("PASS-THROUGH MODE" / "EDIT MODE ON").
        tracing::debug!(
            "X11 input shape set: {}x{} button at top-right (x={}), rest is click-through (window={}x{})",
            button_size,
            button_size,
            button_x,
            geom.width,
            geom.height
        );
        Ok(())
    }

    /// Make the entire window click-through — no button cutout. Used
    /// by the PerMonitor extra windows (T.8): the ⚙ toggle is a
    /// primary-window affordance, so in pass-through mode the extras
    /// reserve nothing.
    pub fn set_passthrough_total(&mut self) -> Result<()> {
        use x11rb::protocol::shape;
        let geom = self.conn.get_geometry(self.x11_window)?.reply()?;

        let pixmap = self.conn.generate_id()?;
        create_pixmap(
            &self.conn,
            1,
            pixmap,
            self.x11_window,
            geom.width,
            geom.height,
        )?;
        let gc = self.conn.generate_id()?;
        create_gc(
            &self.conn,
            gc,
            pixmap,
            &CreateGCAux::new().foreground(0).background(0),
        )?;
        poly_fill_rectangle(
            &self.conn,
            pixmap,
            gc,
            &[Rectangle {
                x: 0,
                y: 0,
                width: geom.width,
                height: geom.height,
            }],
        )?;
        shape::mask(
            &self.conn,
            shape::SO::SET,
            shape::SK::INPUT,
            self.x11_window,
            0,
            0,
            pixmap,
        )?;
        free_gc(&self.conn, gc)?;
        free_pixmap(&self.conn, pixmap)?;
        self.conn.flush()?;
        tracing::debug!("X11 input shape: fully click-through");
        Ok(())
    }

    /// Set the window to receive input on the entire surface (edit mode).
    pub fn set_full_input(&mut self) -> Result<()> {
        let geom = self.conn.get_geometry(self.x11_window)?.reply()?;

        // Create a full pixmap (all 1s = all receives input)
        let pixmap = self.conn.generate_id()?;
        create_pixmap(
            &self.conn,
            1,
            pixmap,
            self.x11_window,
            geom.width,
            geom.height,
        )?;

        let gc = self.conn.generate_id()?;
        create_gc(
            &self.conn,
            gc,
            pixmap,
            &CreateGCAux::new().foreground(1).background(1),
        )?;
        poly_fill_rectangle(
            &self.conn,
            pixmap,
            gc,
            &[Rectangle {
                x: 0,
                y: 0,
                width: geom.width,
                height: geom.height,
            }],
        )?;

        use x11rb::protocol::shape;
        shape::mask(
            &self.conn,
            shape::SO::SET,
            shape::SK::INPUT,
            self.x11_window,
            0,
            0,
            pixmap,
        )?;

        free_gc(&self.conn, gc)?;
        free_pixmap(&self.conn, pixmap)?;
        self.conn.flush()?;

        tracing::info!("X11 input shape: full window receives input (edit mode)");
        Ok(())
    }
}

// X11 is the first `OverlayPlatform` backend. The winit run loop holds a
// `Box<dyn OverlayPlatform>` and calls these; a Windows/macOS backend
// implements the same trait. Each method delegates to the inherent method
// of the same name above — inherent methods win method resolution over
// trait methods, so `self.foo()` here calls the inherent `foo`, never this
// trait method (no recursion). Keep the inherent methods.
impl crate::window::overlay::OverlayPlatform for X11InputManager {
    fn reassert_above(&self) -> Result<()> {
        self.reassert_above()
    }
    fn query_pointer_global(&self) -> Option<(f32, f32)> {
        self.query_pointer_global()
    }
    fn set_passthrough_with_button(&mut self, button_size: u32) -> Result<()> {
        self.set_passthrough_with_button(button_size)
    }
    fn set_passthrough_total(&mut self) -> Result<()> {
        self.set_passthrough_total()
    }
    fn set_full_input(&mut self) -> Result<()> {
        self.set_full_input()
    }
}
