//! Whether the user is away, through `ext-idle-notify-v1`, for holding the
//! scene still (`crate::away`). The compositor says when the seat has had
//! no input for the time asked, and when it has again. It honours idle
//! inhibitors, so a video playing in a window keeps the characters going.
//! Compositors without the protocol leave it unavailable, and the setting
//! says so.

use super::state::WaylandState;
use wayland_client::{protocol::wl_seat::WlSeat, Connection, Dispatch, QueueHandle};
use wayland_protocols::ext::idle_notify::v1::client::{
    ext_idle_notification_v1::{self, ExtIdleNotificationV1},
    ext_idle_notifier_v1::{self, ExtIdleNotifierV1},
};

pub struct Idle {
    notifier: Option<ExtIdleNotifierV1>,
    notification: Option<ExtIdleNotificationV1>,
    /// What the notification was asked for, in milliseconds; 0 none.
    timeout_ms: u32,
    /// Idle past it, as last told.
    pub(super) idle: bool,
}

impl Idle {
    pub fn new(notifier: Option<ExtIdleNotifierV1>) -> Self {
        Self {
            notifier,
            notification: None,
            timeout_ms: 0,
            idle: false,
        }
    }

    /// Whether the compositor can tell.
    pub fn available(&self) -> bool {
        self.notifier.is_some()
    }

    /// Ask to be told after `timeout_ms` without input, 0 not at all. The
    /// notification is made again when the time changes, and until there
    /// is a seat to make it for.
    pub fn set_timeout(
        &mut self,
        timeout_ms: u32,
        seat: Option<&WlSeat>,
        qh: &QueueHandle<WaylandState>,
    ) {
        let made = timeout_ms == 0 || self.notification.is_some();
        if timeout_ms == self.timeout_ms && made {
            return;
        }
        if let Some(old) = self.notification.take() {
            old.destroy();
        }
        self.idle = false;
        self.timeout_ms = timeout_ms;
        if let (true, Some(notifier), Some(seat)) = (timeout_ms > 0, &self.notifier, seat) {
            self.notification = Some(notifier.get_idle_notification(timeout_ms, seat, qh, ()));
        }
    }
}

impl Dispatch<ExtIdleNotifierV1, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &ExtIdleNotifierV1,
        _event: ext_idle_notifier_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // The notifier has no events.
    }
}

impl Dispatch<ExtIdleNotificationV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _proxy: &ExtIdleNotificationV1,
        event: ext_idle_notification_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            ext_idle_notification_v1::Event::Idled => state.idle.idle = true,
            ext_idle_notification_v1::Event::Resumed => state.idle.idle = false,
            _ => {}
        }
    }
}
