//! Custom winit user events. These are emitted from non-event-loop
//! threads (tray menu handler, future global hotkeys) and dispatched
//! through `EventLoopProxy::send_event` so `App::user_event` can react
//! on the UI thread.

/// Top-level commands carried over the winit user-event channel.
#[derive(Debug, Clone, Copy)]
pub enum AnimaEvent {
    /// Toggle edit ↔ pass-through mode.
    ToggleEditMode,
    /// A global shortcut was pressed: `action`, by the chord where the
    /// backend knows it — the X11 grab and Windows do, the portal does not
    /// (the desktop owns those keys). The loops turn it into the action's
    /// event (`hotkeys::action_to_event`) — unless a shortcut is being
    /// recorded, which takes the chord instead and leaves the action be.
    Shortcut(
        crate::keybindings::Action,
        Option<crate::keybindings::KeyChord>,
    ),
    /// Pause / resume global animation playback.
    ToggleGlobalPlayback,
    /// Hide the overlay window (set invisible).
    HideOverlay,
    /// Show the overlay window (visible).
    ShowOverlay,
    /// A second launch attempt asked us to come back to the front
    /// (single-instance handshake).
    RaiseWindow,
    /// Save config and exit cleanly.
    Quit,
    /// The deferred hotkey resolution (portal handshake + fallbacks)
    /// finished without a working backend — surface the warning
    /// banner. Emitted at most once per run.
    HotkeysUnavailable,
    /// The portal handshake failed (denied / unavailable) but the
    /// XGrabKey fallback took over — toast the downgrade so the user
    /// knows why the system shortcut dialog had no effect.
    PortalShortcutsDenied,
    /// Whether the window in front is full screen changed (winit path;
    /// see `window::x11_windows::spawn_fullscreen_watch`).
    FullscreenInFront(bool),
    /// The user has been idle past the chosen time, or is back (winit
    /// path; `crate::away`).
    Away(bool),
    /// Whether the machine runs on battery changed (`crate::away`).
    OnBattery(bool),
    /// Whether this session has an idle time to read, once it is known.
    IdleSource(bool),
    /// Switch to the next saved scene, by name (`crate::scenes`): the
    /// tray's "Next scene" and D-Bus `NextScene`.
    NextScene,
    /// Whether an app the user chose is the window in front changed
    /// (winit path; `crate::chosen_apps`).
    ChosenAppInFront(bool),
    /// Snooze for a while (`crate::snooze`): the tray, D-Bus.
    Snooze(crate::snooze::SnoozeFor),
    /// Wake up from a snooze.
    Wake,
    /// Start the focus timer (`crate::focus`): this many minutes of work,
    /// or 0 for the length set in the Scene tab. The tray, D-Bus, the
    /// palette.
    StartFocus(u32),
    /// Stop the focus timer.
    StopFocus,
    /// Mute the sounds, or not (`sounds_muted`): the tray's and a global
    /// shortcut's way in, which cannot reach the config themselves.
    SetMuted(bool),
    /// Mute the sounds, or bring them back — the other way from
    /// `sounds_muted` when the loop gets to it: a global shortcut, D-Bus
    /// `ToggleMute`. Decided there, not where it is sent, so two in a row
    /// are two toggles.
    ToggleMute,
}

/// Where a background thread — the tray, the D-Bus service — sends its
/// events: the winit loop's proxy, or the native Wayland loop's channel.
///
/// The tray used to take a winit proxy only, so the native Wayland path,
/// which has no winit event loop, never started one (the README listed it
/// as supported there all the same).
#[derive(Clone)]
pub enum EventSink {
    Winit(winit::event_loop::EventLoopProxy<AnimaEvent>),
    Channel(std::sync::mpsc::SyncSender<AnimaEvent>),
}

impl EventSink {
    /// Deliver `event`. `false` if the receiving loop is gone, or — for the
    /// bounded channel — momentarily full: a background thread must never
    /// block on the UI, and a dropped click is better than a stalled tray.
    pub fn send(&self, event: AnimaEvent) -> bool {
        match self {
            Self::Winit(proxy) => proxy.send_event(event).is_ok(),
            Self::Channel(tx) => {
                let sent = tx.try_send(event).is_ok();
                // The native Wayland loop sleeps until woken.
                #[cfg(unix)]
                crate::wayland::waker::wake();
                sent
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_channel_sink_delivers_and_never_blocks() {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let sink = EventSink::Channel(tx);
        assert!(sink.send(AnimaEvent::Quit));
        assert!(!sink.send(AnimaEvent::Quit), "full: refused, not blocked");
        assert!(matches!(rx.try_recv(), Ok(AnimaEvent::Quit)));
        drop(rx);
        assert!(!sink.send(AnimaEvent::Quit), "receiver gone");
    }
}
