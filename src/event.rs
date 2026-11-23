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
    /// The portal bound the shortcuts: the user allowed them, or had.
    PortalShortcutsActive,
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

/// Commands from outside a second may pass: the D-Bus methods, the tray,
/// Windows' raise. Human use is a handful a second at worst, so past this
/// is a flood, and dropping it keeps the overlay answering: the winit
/// queue has no bound of its own, and the native loop's channel is shared
/// with the signal handler (SEC-012).
pub(crate) const MAX_COMMANDS_PER_SECOND: u32 = 20;

/// A fixed-window count of the commands of this second.
pub(crate) struct CommandRate {
    pub(crate) window_start: std::time::Instant,
    pub(crate) count: u32,
    pub(crate) dropped: u64,
}

impl CommandRate {
    pub(crate) fn new() -> Self {
        Self {
            window_start: std::time::Instant::now(),
            count: 0,
            dropped: 0,
        }
    }

    /// One budget for several senders.
    #[cfg(unix)]
    pub(crate) fn shared() -> std::sync::Arc<std::sync::Mutex<Self>> {
        std::sync::Arc::new(std::sync::Mutex::new(Self::new()))
    }

    /// Whether a command arriving at `now` fits this second's budget.
    pub(crate) fn allow(&mut self, now: std::time::Instant) -> bool {
        if now.duration_since(self.window_start) >= std::time::Duration::from_secs(1) {
            if self.dropped > 0 {
                tracing::warn!(
                    "dropped {} flooding command(s) in the last second",
                    self.dropped
                );
            }
            self.window_start = now;
            self.count = 0;
            self.dropped = 0;
        }
        if self.count >= MAX_COMMANDS_PER_SECOND {
            self.dropped = self.dropped.saturating_add(1);
            return false;
        }
        self.count += 1;
        true
    }

    /// [`Self::allow`] for one behind a lock; a poisoned lock lets the
    /// command through, as a control surface must not go dead.
    #[cfg(unix)]
    pub(crate) fn allowed(rate: &std::sync::Mutex<Self>) -> bool {
        match rate.lock() {
            Ok(mut rate) => rate.allow(std::time::Instant::now()),
            Err(_) => true,
        }
    }
}

/// What became of an event handed to an [`EventSink`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    Sent,
    /// The bounded channel is full: the loop is there, and busy.
    Full,
    /// The receiving loop is gone.
    Gone,
}

impl EventSink {
    /// Deliver `event`. `false` if the receiving loop is gone, or — for the
    /// bounded channel — momentarily full: a background thread must never
    /// block on the UI, and a dropped click is better than a stalled tray.
    pub fn send(&self, event: AnimaEvent) -> bool {
        self.deliver(event) == Delivery::Sent
    }

    /// [`Self::send`], saying which way it did not go: a full channel is
    /// a loop that is there, which a quit can reach another way
    /// (`quit_signal::asked`).
    pub fn deliver(&self, event: AnimaEvent) -> Delivery {
        match self {
            Self::Winit(proxy) => match proxy.send_event(event) {
                Ok(()) => Delivery::Sent,
                Err(_) => Delivery::Gone,
            },
            Self::Channel(tx) => match tx.try_send(event) {
                Ok(()) => {
                    // The native Wayland loop sleeps until woken — for
                    // what was sent: woken for what was not, a flood kept
                    // it turning (SEC-012).
                    #[cfg(unix)]
                    crate::wayland::waker::wake();
                    Delivery::Sent
                }
                Err(std::sync::mpsc::TrySendError::Full(_)) => Delivery::Full,
                Err(std::sync::mpsc::TrySendError::Disconnected(_)) => Delivery::Gone,
            },
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

    /// Full and gone are told apart: a full channel's loop is there.
    #[test]
    fn a_full_channel_is_not_a_gone_loop() {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let sink = EventSink::Channel(tx);
        assert_eq!(sink.deliver(AnimaEvent::Quit), Delivery::Sent);
        assert_eq!(sink.deliver(AnimaEvent::Quit), Delivery::Full);
        drop(rx);
        assert_eq!(sink.deliver(AnimaEvent::Quit), Delivery::Gone);
    }
}
