//! Linux-specific window utilities and platform notes.
//!
//! # X11 Transparency Requirements
//! - A compositor must be running (Picom, Mutter, KWin, etc.)
//! - GNOME/Mutter provides compositing by default
//! - On bare X11, you may need to start `picom` or `compton`
//!
//! # X11 Always-on-Top
//! - winit uses EWMH `_NET_WM_STATE_ABOVE` hint
//! - Should work on most modern window managers

use x11rb::connection::Connection;
use x11rb::protocol::xproto::ConnectionExt;

/// Log whether a compositor is available for the X11 path.
///
/// Informative only — the renderer independently refuses to start
/// without a transparent alpha mode, which is the authoritative gate.
/// This probe just makes the failure diagnosable from the log.
pub fn check_compositor() {
    match compositor_selection_owned() {
        Ok(true) => {
            tracing::info!("Compositor detected (_NET_WM_CM selection owned)");
        }
        Ok(false) => {
            tracing::warn!(
                "No compositor owns the _NET_WM_CM selection. Window \
                 transparency may not work — consider running picom."
            );
        }
        // No X display reachable (pure Wayland session without
        // XWayland, headless) — the Wayland path doesn't need this.
        Err(e) => {
            tracing::debug!("Compositor probe skipped: {e}");
        }
    }
}

/// The EWMH-standard compositor check: a running composite manager
/// owns the `_NET_WM_CM_S<screen>` selection. Querying it through the
/// protocol replaces the old `pgrep picom/compton/…` heuristic, which
/// could both false-positive (another user's process, different
/// display) and false-negative (any compositor not on the hardcoded
/// list).
fn compositor_selection_owned() -> Result<bool, Box<dyn std::error::Error>> {
    let (conn, screen_num) = x11rb::connect(None)?;
    let atom_name = format!("_NET_WM_CM_S{screen_num}");
    let atom = conn.intern_atom(false, atom_name.as_bytes())?.reply()?.atom;
    let owner = conn.get_selection_owner(atom)?.reply()?.owner;
    Ok(owner != x11rb::NONE)
}

/// Where the pointer is on the X root window, in global desktop pixels.
///
/// For file drops. XDND carries the drop position, and winit parses it —
/// its own source comment says so — but the API has nowhere to put it, so
/// `DroppedFile` arrives with a path and nothing else. The app was falling
/// back to the last `CursorMoved`, and during a drag the source holds a
/// pointer grab, so the overlay gets no motion at all: the character
/// landed wherever the cursor had last been *before* the drag, which in
/// pass-through mode may be the ⚙ corner or nowhere (R37).
///
/// Root coordinates are the same space `mouse_x` / `mouse_y` live in, so
/// the result is usable directly.
///
/// A short-lived connection is deliberate. A drop is a rare, human-paced
/// event, and the alternative is holding an X connection open for the
/// whole session to answer one question occasionally.
pub fn pointer_root_position() -> Option<(f32, f32)> {
    let (conn, screen_num) = x11rb::connect(None).ok()?;
    let root = conn.setup().roots.get(screen_num)?.root;
    let reply = conn.query_pointer(root).ok()?.reply().ok()?;
    // `same_screen` false means the pointer is on a different X screen
    // than the root we asked about, and the coordinates are meaningless.
    if !reply.same_screen {
        return None;
    }
    Some((reply.root_x as f32, reply.root_y as f32))
}
