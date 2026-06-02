//! Custom winit user events. These are emitted from non-event-loop
//! threads (tray menu handler, future global hotkeys) and dispatched
//! through `EventLoopProxy::send_event` so `App::user_event` can react
//! on the UI thread.

/// Top-level commands carried over the winit user-event channel.
#[derive(Debug, Clone, Copy)]
pub enum AnimaEvent {
    /// Toggle edit ↔ pass-through mode.
    ToggleEditMode,
    /// Pause / resume global animation playback.
    ToggleGlobalPlayback,
    /// Hide the overlay window (set invisible).
    HideOverlay,
    /// Show the overlay window (visible).
    ShowOverlay,
    /// Save config and exit cleanly.
    Quit,
}
