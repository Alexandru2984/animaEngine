//! Direct X11 input management for overlay windows.
//!
//! Dock-type windows on X11 never receive keyboard focus, and `set_cursor_hittest(false)`
//! makes the entire window invisible to ALL input. This module provides:
//!
//! 1. **Input Shape**: Use the X11 Shape extension to make most of the window
//!    click-through while keeping a small control button area clickable.
//! 2. **Global Key Grab**: Use `XGrabKey` to intercept hotkeys regardless of
//!    which window has focus.

use std::sync::mpsc;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::ConnectionExt as XprotoConnectionExt;

/// Size of the clickable toggle button in the corner (pixels)
pub const TOGGLE_BUTTON_SIZE: u32 = 48;

/// Commands sent from the X11 hotkey thread to the main app
#[derive(Debug, Clone, Copy)]
pub enum HotkeyCommand {
    ToggleEditMode,
    Quit,
    SaveConfig,
    TogglePlayback,
}

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

/// Spawn a background thread that grabs global hotkeys via X11.
/// Returns a receiver that the main event loop can poll for commands.
///
/// This works even when the overlay window doesn't have keyboard focus,
/// because XGrabKey intercepts keys at the X server level.
pub fn spawn_global_hotkey_listener() -> Result<mpsc::Receiver<HotkeyCommand>, Box<dyn std::error::Error>>
{
    let (tx, rx) = mpsc::channel();

    std::thread::Builder::new()
        .name("x11-hotkey".to_string())
        .spawn(move || {
            if let Err(e) = hotkey_thread(tx) {
                log::error!("Global hotkey thread failed: {}", e);
            }
        })?;

    Ok(rx)
}

fn hotkey_thread(tx: mpsc::Sender<HotkeyCommand>) -> Result<(), Box<dyn std::error::Error>> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::*;

    let (conn, screen_num) = x11rb::connect(None)?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    // Key codes for our hotkeys (these are X11 keycodes, not keysyms)
    // We'll look up the keycodes from keysyms for portability
    let escape_keycode = keysym_to_keycode(&conn, screen_num, 0xff1b)?; // Escape
    let q_keycode = keysym_to_keycode(&conn, screen_num, 0x0071)?; // q
    let s_keycode = keysym_to_keycode(&conn, screen_num, 0x0073)?; // s
    let space_keycode = keysym_to_keycode(&conn, screen_num, 0x0020)?; // Space

    // Grab Ctrl+Escape for toggle edit mode
    if let Some(kc) = escape_keycode {
        grab_key(
            &conn,
            false,
            root,
            ModMask::CONTROL,
            kc,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
        )?;
        log::info!("Global hotkey registered: Ctrl+Escape → toggle edit mode");
    }

    // Grab Ctrl+Q for quit
    if let Some(kc) = q_keycode {
        grab_key(
            &conn,
            false,
            root,
            ModMask::CONTROL,
            kc,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
        )?;
        log::info!("Global hotkey registered: Ctrl+Q → quit");
    }

    // Grab Ctrl+S for save
    if let Some(kc) = s_keycode {
        grab_key(
            &conn,
            false,
            root,
            ModMask::CONTROL,
            kc,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
        )?;
        log::info!("Global hotkey registered: Ctrl+S → save");
    }

    // Grab Ctrl+Space for play/pause
    if let Some(kc) = space_keycode {
        grab_key(
            &conn,
            false,
            root,
            ModMask::CONTROL,
            kc,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
        )?;
        log::info!("Global hotkey registered: Ctrl+Space → play/pause");
    }

    conn.flush()?;

    // Event loop — blocks waiting for grabbed key events
    log::info!("Global hotkey listener active");
    loop {
        let event = conn.wait_for_event()?;
        match event {
            x11rb::protocol::Event::KeyPress(ev) => {
                let cmd = if escape_keycode == Some(ev.detail) {
                    Some(HotkeyCommand::ToggleEditMode)
                } else if q_keycode == Some(ev.detail) {
                    Some(HotkeyCommand::Quit)
                } else if s_keycode == Some(ev.detail) {
                    Some(HotkeyCommand::SaveConfig)
                } else if space_keycode == Some(ev.detail) {
                    Some(HotkeyCommand::TogglePlayback)
                } else {
                    None
                };

                if let Some(cmd) = cmd {
                    log::info!("Global hotkey: {:?}", cmd);
                    if tx.send(cmd).is_err() {
                        // Main thread exited
                        break;
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}

/// Convert an X11 keysym to a keycode using the keyboard mapping
fn keysym_to_keycode(
    conn: &impl x11rb::connection::Connection,
    _screen_num: usize,
    keysym: u32,
) -> Result<Option<u8>, Box<dyn std::error::Error>> {
    let setup = conn.setup();
    let min_keycode = setup.min_keycode;
    let max_keycode = setup.max_keycode;

    let mapping = conn
        .get_keyboard_mapping(min_keycode, max_keycode - min_keycode + 1)?
        .reply()?;

    let keysyms_per_keycode = mapping.keysyms_per_keycode as usize;

    for keycode in min_keycode..=max_keycode {
        let offset = (keycode - min_keycode) as usize * keysyms_per_keycode;
        for i in 0..keysyms_per_keycode {
            if offset + i < mapping.keysyms.len() && mapping.keysyms[offset + i] == keysym {
                return Ok(Some(keycode));
            }
        }
    }

    log::warn!("Could not find keycode for keysym 0x{:04x}", keysym);
    Ok(None)
}
