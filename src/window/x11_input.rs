//! Direct X11 input management for overlay windows.
//!
//! Dock-type windows on X11 never receive keyboard focus, and `set_cursor_hittest(false)`
//! makes the entire window invisible to ALL input. This module provides:
//!
//! **Input Shape**: Use the X11 Shape extension to make most of the window
//! click-through while keeping a small control button area clickable.

use x11rb::connection::Connection;

/// Size of the clickable toggle button in the corner (pixels)
pub const TOGGLE_BUTTON_SIZE: u32 = 48;

/// Set the X11 input shape so that only a small rectangle in the top-right corner
/// receives mouse input. The rest of the window is click-through.
///
/// This replaces winit's `set_cursor_hittest(false)` which makes the ENTIRE
/// window invisible to input — including the parts we need clickable.
pub fn set_passthrough_with_button(
    window: &winit::window::Window,
    button_size: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    use raw_window_handle::HasWindowHandle;

    let handle = window.window_handle()?;
    let raw = handle.as_raw();

    if let raw_window_handle::RawWindowHandle::Xlib(xlib_handle) = raw {
        let x11_window = xlib_handle.window as u32;

        // Connect to X11
        let (conn, _screen_num) = x11rb::connect(None)?;

        // Get window geometry to know where to place the button
        let geom = conn.get_geometry(x11_window)?.reply()?;
        let win_width = geom.width as i16;

        // Create a pixmap for the input shape: a small rectangle in the top-right corner
        use x11rb::protocol::xproto::*;
        let pixmap = conn.generate_id()?;
        create_pixmap(&conn, 1, pixmap, x11_window, geom.width, geom.height)?;

        // Fill the entire pixmap with 0 (transparent / pass-through)
        let gc = conn.generate_id()?;
        create_gc(
            &conn,
            gc,
            pixmap,
            &CreateGCAux::new().foreground(0).background(0),
        )?;
        poly_fill_rectangle(
            &conn,
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
        change_gc(&conn, gc, &ChangeGCAux::new().foreground(1))?;
        let button_x = win_width - button_size as i16;
        poly_fill_rectangle(
            &conn,
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
        use x11rb::protocol::shape::{self};
        shape::mask(
            &conn,
            shape::SO::SET,
            shape::SK::INPUT,
            x11_window,
            0,
            0,
            pixmap,
        )?;

        // Cleanup
        free_gc(&conn, gc)?;
        free_pixmap(&conn, pixmap)?;
        conn.flush()?;

        log::info!(
            "X11 input shape set: {}x{} button at top-right, rest is click-through",
            button_size,
            button_size
        );
        Ok(())
    } else {
        Err("Not running on X11 — input shape not supported".into())
    }
}

/// Set the window to receive input on the entire surface (edit mode).
pub fn set_full_input(
    window: &winit::window::Window,
) -> Result<(), Box<dyn std::error::Error>> {
    use raw_window_handle::HasWindowHandle;

    let handle = window.window_handle()?;
    let raw = handle.as_raw();

    if let raw_window_handle::RawWindowHandle::Xlib(xlib_handle) = raw {
        let x11_window = xlib_handle.window as u32;
        let (conn, _screen_num) = x11rb::connect(None)?;

        // Remove the input shape entirely — full window receives input
        use x11rb::protocol::shape::{self};
        let geom = conn.get_geometry(x11_window)?.reply()?;

        // Create a full pixmap (all 1s = all receives input)
        use x11rb::protocol::xproto::*;
        let pixmap = conn.generate_id()?;
        create_pixmap(&conn, 1, pixmap, x11_window, geom.width, geom.height)?;

        let gc = conn.generate_id()?;
        create_gc(
            &conn,
            gc,
            pixmap,
            &CreateGCAux::new().foreground(1).background(1),
        )?;
        poly_fill_rectangle(
            &conn,
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
            &conn,
            shape::SO::SET,
            shape::SK::INPUT,
            x11_window,
            0,
            0,
            pixmap,
        )?;

        free_gc(&conn, gc)?;
        free_pixmap(&conn, pixmap)?;
        conn.flush()?;

        log::info!("X11 input shape removed: full window receives input (edit mode)");
        Ok(())
    } else {
        Err("Not running on X11".into())
    }
}
