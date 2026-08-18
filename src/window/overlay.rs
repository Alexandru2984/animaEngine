//! Platform-neutral overlay integration for the **winit run-loop path**.
//!
//! The winit path (`src/app/`) drives the overlay on X11/XWayland today
//! and is the path a Windows or macOS backend will ride. The operations
//! that make a plain window behave like a desktop overlay — always-on-top
//! re-assertion, click-through region control, and a focus-independent
//! global cursor read — are inherently OS-specific. This trait is the seam:
//! the app holds a `Box<dyn OverlayPlatform>` instead of a concrete X11
//! type, so a new backend slots in by implementing it and extending
//! [`for_window`].
//!
//! The **native Wayland path** (`src/wayland/`, wlr-layer-shell) has its
//! own run loop and its own input-region handling; it does not go through
//! this trait and stays Linux/BSD-only.
//!
//! First implementor: [`crate::window::x11_input::X11InputManager`].

use crate::error::Result;
use winit::window::Window;

/// The OS-specific overlay operations the winit run loop needs. Object-safe
/// by construction (no generics, no `Self` returns) so it can live behind a
/// `Box<dyn OverlayPlatform>`.
pub trait OverlayPlatform {
    /// Re-assert always-on-top / skip-taskbar so a newly focused window
    /// can't sink the overlay behind it. Cheap; called on focus and
    /// occlusion transitions.
    fn reassert_above(&self) -> Result<()>;

    /// The pointer position in global (desktop) coordinates, read without
    /// depending on which window currently has input — needed by
    /// cursor-follower entities while the overlay is click-through.
    /// `None` when the platform can't answer.
    fn query_pointer_global(&self) -> Option<(f32, f32)>;

    /// Click-through everywhere except a `button_size`-sided corner (the
    /// ⚙ toggle) — the default pass-through shape.
    fn set_passthrough_with_button(&mut self, button_size: u32) -> Result<()>;

    /// Fully click-through — no interactive region at all.
    fn set_passthrough_total(&mut self) -> Result<()>;

    /// Fully interactive — the whole surface receives input (edit mode).
    fn set_full_input(&mut self) -> Result<()>;
}

/// Build the overlay backend for `window` on the current OS, or `None`
/// when the window server can't support these operations (e.g. the winit
/// window isn't X11, so the caller falls back to `set_cursor_hittest`).
pub fn for_window(window: &Window) -> Option<Box<dyn OverlayPlatform>> {
    #[cfg(target_os = "linux")]
    {
        super::x11_input::X11InputManager::new(window)
            .map(|m| Box::new(m) as Box<dyn OverlayPlatform>)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = window;
        None
    }
}
