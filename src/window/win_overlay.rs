//! Windows overlay backend — the `OverlayPlatform` seam on Win32 (C4).
//!
//! X11 gives us a real **input shape**: an arbitrary region that receives
//! clicks while the rest of the window passes them through, set once and
//! honoured by the server. Win32 has no equivalent for a window that
//! presents through a DXGI swapchain — hit-testing is all-or-nothing, via
//! the `WS_EX_TRANSPARENT` extended style. So the corner cut-out for the ⚙
//! toggle is **emulated**: a small tracker thread reads the global cursor
//! and drops `WS_EX_TRANSPARENT` only while the pointer sits inside the
//! button square, restoring it on the way out.
//!
//! The thread is not laziness. A click-through window receives no mouse
//! messages at all, so nothing event-driven can notice the pointer
//! arriving; and the app's own redraw loop parks for `IDLE_HEARTBEAT` (2 s)
//! over a static scene — far too coarse to gate a button on.
//!
//! The bits set here are exactly the pair winit uses for
//! `Window::set_cursor_hittest` (`WS_EX_TRANSPARENT | WS_EX_LAYERED`), and
//! every write is read-modify-write, so this never disagrees with — or
//! clobbers — the styles winit owns (topmost, tool window, …).
//!
//! Not covered here, by design: the tray (`Shell_NotifyIcon`) and the
//! single-instance mutex live behind their own seams; `RegisterHotKey`
//! already comes free through the `global-hotkey` crate.

use crate::error::Result;
use crate::window::overlay::OverlayPlatform;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use windows_sys::Win32::Foundation::{POINT, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetWindowLongPtrW, GetWindowRect, IsWindow, SetWindowLongPtrW, SetWindowPos,
    GWL_EXSTYLE, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WS_EX_LAYERED,
    WS_EX_TRANSPARENT,
};

/// Cursor sampling cadence while a button corner is reserved. 60 Hz — the
/// ⚙ has to feel like a button, and `GetCursorPos` reads shared memory
/// rather than making a round trip.
const TRACK_INTERVAL: Duration = Duration::from_millis(16);

/// Cadence with no corner to watch. Every setter already applies its mode
/// synchronously, so this is purely a self-heal in case winit recomputes
/// the extended style from its own cached flags.
const IDLE_INTERVAL: Duration = Duration::from_millis(250);

/// `WS_EX_TRANSPARENT` is the bit that actually decides hit-testing, and
/// it is the only one that toggles here.
///
/// `WS_EX_LAYERED` is set alongside it and then *left alone*: the overlay
/// is presented with `UpdateLayeredWindow`
/// ([`crate::renderer::win_layered`]), which refuses a window without it.
/// winit clears both together in `set_cursor_hittest` — doing the same
/// would blank the overlay every time it went interactive.
const HIT_TEST: isize = WS_EX_TRANSPARENT as isize;
const LAYERED: isize = WS_EX_LAYERED as isize;

/// State shared with the tracker thread. Two independent atomics rather
/// than a mutex: a tick that observes a half-updated pair costs at most one
/// frame of the previous shape, and no lock can be poisoned across the
/// thread boundary.
struct Shared {
    /// The `HWND` as a plain integer — which is exactly what windows-sys
    /// types it as, and what makes this `Send` without a wrapper.
    hwnd: isize,
    /// `false` = the whole surface takes input (edit mode).
    passthrough: AtomicBool,
    /// Side length in physical px of the interactive top-right square, or
    /// 0 for none.
    button_px: AtomicU32,
    /// Set on drop; ends the tracker within one interval.
    stop: AtomicBool,
}

/// Win32 implementation of [`OverlayPlatform`]. Built by
/// [`crate::window::overlay::for_window`].
pub struct WinOverlay {
    shared: Arc<Shared>,
}

impl WinOverlay {
    /// Attach to `window`'s HWND. `None` when the winit window isn't a
    /// Win32 one, so the caller falls back to `set_cursor_hittest`.
    pub fn new(window: &winit::window::Window) -> Option<Self> {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};

        let handle = window.window_handle().ok()?;
        let RawWindowHandle::Win32(win32) = handle.as_raw() else {
            tracing::info!("WinOverlay: not a Win32 window, overlay control unavailable");
            return None;
        };
        let hwnd = win32.hwnd.get();

        let shared = Arc::new(Shared {
            hwnd,
            // Start click-through: the app's very next call is
            // `set_passthrough_with_button`, and a window that grabbed every
            // click for the moment in between would be the worse default.
            passthrough: AtomicBool::new(true),
            button_px: AtomicU32::new(0),
            stop: AtomicBool::new(false),
        });
        apply_hit_test(hwnd, true);
        spawn_tracker(Arc::clone(&shared));

        tracing::info!("WinOverlay: attached to HWND {hwnd:#x}");
        Some(Self { shared })
    }

    /// Apply the current mode now instead of waiting for the tracker's next
    /// tick — a mode flip has to land on the frame the user asked for it.
    fn sync_now(&self) {
        let button = self.shared.button_px.load(Ordering::Relaxed);
        apply_hit_test(self.shared.hwnd, want_transparent(&self.shared, button));
    }
}

impl Drop for WinOverlay {
    fn drop(&mut self) {
        // The tracker holds its own `Arc`, so the flag — not the refcount —
        // is what ends it. Not joined: it wakes on its own within an
        // interval, and shutdown shouldn't block on a sleep.
        self.shared.stop.store(true, Ordering::Relaxed);
    }
}

impl OverlayPlatform for WinOverlay {
    fn reassert_above(&self) -> Result<()> {
        // Z-order only, and deliberately not an activation: a click-through
        // overlay floats above without ever taking focus. winit's
        // `WindowLevel::AlwaysOnTop` sets `WS_EX_TOPMOST` once at creation;
        // this re-claims the position after another topmost window (a
        // full-screen game, an installer) has pushed past us.
        //
        // SAFETY: `SetWindowPos` on our own window handle, with NOMOVE and
        // NOSIZE so the zeroed geometry arguments are ignored.
        let ok = unsafe {
            SetWindowPos(
                self.shared.hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )
        };
        if ok == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(())
    }

    fn query_pointer_global(&self) -> Option<(f32, f32)> {
        // Screen coordinates, the same desktop-global space X11's
        // `XQueryPointer` reports against the root window — and, like it,
        // independent of which window currently has input, which is the
        // whole point while the overlay is click-through.
        let mut pt = POINT { x: 0, y: 0 };
        // SAFETY: writes into a caller-owned, fully initialised `POINT`.
        if unsafe { GetCursorPos(&mut pt) } == 0 {
            tracing::debug!("GetCursorPos failed: {}", std::io::Error::last_os_error());
            return None;
        }
        Some((pt.x as f32, pt.y as f32))
    }

    fn set_passthrough_with_button(&mut self, button_size: u32) -> Result<()> {
        self.shared.button_px.store(button_size, Ordering::Relaxed);
        self.shared.passthrough.store(true, Ordering::Relaxed);
        self.sync_now();
        Ok(())
    }

    fn set_passthrough_total(&mut self) -> Result<()> {
        self.shared.button_px.store(0, Ordering::Relaxed);
        self.shared.passthrough.store(true, Ordering::Relaxed);
        self.sync_now();
        Ok(())
    }

    fn set_full_input(&mut self) -> Result<()> {
        // `button_px` is left alone: it is only consulted in pass-through,
        // and keeping it means the return to pass-through restores the same
        // corner without the caller having to re-state its size.
        self.shared.passthrough.store(false, Ordering::Relaxed);
        self.sync_now();
        Ok(())
    }
}

/// Start the cursor tracker. A spawn failure is survivable — pass-through
/// and edit mode still work, because both setters apply synchronously —
/// so it degrades to a warning rather than failing construction.
fn spawn_tracker(shared: Arc<Shared>) {
    let spawned = std::thread::Builder::new()
        .name("anima-win-hittest".into())
        .spawn(move || {
            while !shared.stop.load(Ordering::Relaxed) {
                // The window can be destroyed before `Drop` runs (event
                // loop exit); poking a dead HWND would be pointless.
                // SAFETY: `IsWindow` accepts any handle value, including a
                // stale one — answering that question is its job.
                if unsafe { IsWindow(shared.hwnd) } == 0 {
                    break;
                }
                let button = shared.button_px.load(Ordering::Relaxed);
                apply_hit_test(shared.hwnd, want_transparent(&shared, button));
                std::thread::sleep(if button > 0 {
                    TRACK_INTERVAL
                } else {
                    IDLE_INTERVAL
                });
            }
        });
    if let Err(e) = spawned {
        tracing::warn!(
            "WinOverlay: cursor tracker didn't start ({e}); the ⚙ button won't be \
             clickable in pass-through — use the global hotkey to reach edit mode"
        );
    }
}

/// Should the window be click-through right now?
fn want_transparent(shared: &Shared, button_px: u32) -> bool {
    if !shared.passthrough.load(Ordering::Relaxed) {
        return false; // edit mode — the whole surface takes input
    }
    button_px == 0 || !cursor_in_corner(shared.hwnd, button_px)
}

/// Is the pointer inside the `size`-sided square at the window's top-right?
///
/// Screen coordinates throughout. The overlay is borderless, so
/// `GetWindowRect` *is* its client rect — the same geometry the X11 backend
/// shapes and the same one `ui::panels::toggle_button` draws the ⚙ into.
fn cursor_in_corner(hwnd: isize, size: u32) -> bool {
    let mut pt = POINT { x: 0, y: 0 };
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: both calls write into caller-owned, fully initialised structs;
    // `hwnd` was checked live by the caller this tick.
    unsafe {
        if GetCursorPos(&mut pt) == 0 || GetWindowRect(hwnd, &mut rect) == 0 {
            return false;
        }
    }
    let size = size as i32;
    pt.x >= rect.right - size && pt.x < rect.right && pt.y >= rect.top && pt.y < rect.top + size
}

/// Add or drop the click-through bit, read-modify-write so the styles
/// winit owns survive untouched, and always re-assert `WS_EX_LAYERED` —
/// which doubles as the self-heal if winit ever recomputes the ex-style
/// from its own flags and drops it. A no-op when the bits already match:
/// the tracker calls this 60×/s and most ticks change nothing.
fn apply_hit_test(hwnd: isize, transparent: bool) {
    // SAFETY: a style read and a conditional write on a live HWND. Neither
    // dereferences anything; `GWL_EXSTYLE` is valid for every window.
    unsafe {
        let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let next = if transparent {
            current | HIT_TEST | LAYERED
        } else {
            (current & !HIT_TEST) | LAYERED
        };
        if next != current {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next);
        }
    }
}
