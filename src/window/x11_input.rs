//! Direct X11 input management for overlay windows.
//!
//! Dock-type windows on X11 never receive keyboard focus, and `set_cursor_hittest(false)`
//! makes the entire window invisible to ALL input. This module provides:
//!
//! **Input Shape**: Use the X11 Shape extension to make most of the window
//! click-through while keeping a small control button area clickable.
//!
//! **Connection pooling**: `X11InputManager` holds a single X11 connection that is
//! reused across all input shape operations, avoiding the overhead of opening a
//! new connection on every toggle or resize.

use x11rb::connection::Connection;
use x11rb::rust_connection::RustConnection;

/// Size of the clickable toggle button in the corner (pixels)
pub const TOGGLE_BUTTON_SIZE: u32 = 48;

/// Manages X11 input shape operations with a pooled connection.
///
/// Instead of opening a new `x11rb` connection on every call to
/// `set_passthrough_with_button()` or `set_full_input()`, this struct keeps
/// one connection alive and reuses it.
pub struct X11InputManager {
    conn: RustConnection,
    x11_window: u32,
}

impl X11InputManager {
    /// Create a new manager for the given winit window.
    /// Returns `None` if the window is not running on X11.
    pub fn new(window: &winit::window::Window) -> Option<Self> {
        use raw_window_handle::HasWindowHandle;

        let handle = window.window_handle().ok()?;
        let raw = handle.as_raw();

        if let raw_window_handle::RawWindowHandle::Xlib(xlib_handle) = raw {
            let x11_window = xlib_handle.window as u32;

            match x11rb::connect(None) {
                Ok((conn, _screen_num)) => {
                    log::info!(
                        "X11InputManager: connection established (window=0x{:x})",
                        x11_window
                    );
                    Some(Self { conn, x11_window })
                }
                Err(e) => {
                    log::warn!("X11InputManager: failed to connect to X11: {}", e);
                    None
                }
            }
        } else {
            log::info!("X11InputManager: not running on X11, input shape unavailable");
            None
        }
    }

    /// Set the X11 input shape so that only a small rectangle in the top-right corner
    /// receives mouse input. The rest of the window is click-through.
    pub fn set_passthrough_with_button(
        &self,
        button_size: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use x11rb::protocol::xproto::*;

        // Get window geometry to know where to place the button
        let geom = self.conn.get_geometry(self.x11_window)?.reply()?;
        let win_width = geom.width as i16;

        // Create a pixmap for the input shape
        let pixmap = self.conn.generate_id()?;
        create_pixmap(&self.conn, 1, pixmap, self.x11_window, geom.width, geom.height)?;

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

        log::info!(
            "X11 input shape set: {}x{} button at top-right, rest is click-through",
            button_size,
            button_size
        );
        Ok(())
    }

    /// Set the window to receive input on the entire surface (edit mode).
    pub fn set_full_input(&self) -> Result<(), Box<dyn std::error::Error>> {
        use x11rb::protocol::xproto::*;

        let geom = self.conn.get_geometry(self.x11_window)?.reply()?;

        // Create a full pixmap (all 1s = all receives input)
        let pixmap = self.conn.generate_id()?;
        create_pixmap(&self.conn, 1, pixmap, self.x11_window, geom.width, geom.height)?;

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

        log::info!("X11 input shape removed: full window receives input (edit mode)");
        Ok(())
    }
}
