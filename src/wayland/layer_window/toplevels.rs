//! Other applications' windows, through `wlr-foreign-toplevel-management`,
//! for stepping aside while one is full screen (`crate::fullscreen`).
//!
//! The compositor announces every toplevel window and, for each, its state
//! — activated, full screen, maximized, minimized — applied on `done`.
//! Only those two flags are kept: the protocol also sends every window's
//! title and app id, which the overlay has no business knowing, and they
//! are dropped as they arrive. The overlay's own layer surfaces are not
//! toplevels and never appear here. Compositors without the protocol
//! (GNOME, KDE) leave the list empty, so nothing is ever in front.

use super::state::WaylandState;
use std::collections::HashMap;
use wayland_client::{event_created_child, Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::{self, ZwlrForeignToplevelHandleV1},
    zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
};

/// The flags a window's state carries that matter here.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Flags {
    activated: bool,
    fullscreen: bool,
}

#[derive(Default)]
struct Window {
    /// Received since the last `done`.
    pending: Flags,
    /// In effect.
    current: Flags,
}

/// Every toplevel the compositor has announced and not yet closed, by
/// protocol id — unique among the connection's live objects.
#[derive(Default)]
pub struct Toplevels {
    windows: HashMap<u32, Window>,
}

impl Toplevels {
    /// Whether the window in front — the activated one — is full screen.
    pub fn fullscreen_in_front(&self) -> bool {
        self.windows
            .values()
            .any(|w| w.current.activated && w.current.fullscreen)
    }
}

/// Read a `state` event's array: native-endian `u32`s, one per state the
/// window is in.
fn flags(array: &[u8]) -> Flags {
    let (words, _) = array.as_chunks::<4>();
    let mut flags = Flags::default();
    for word in words {
        match u32::from_ne_bytes(*word) {
            x if x == zwlr_foreign_toplevel_handle_v1::State::Activated as u32 => {
                flags.activated = true;
            }
            x if x == zwlr_foreign_toplevel_handle_v1::State::Fullscreen as u32 => {
                flags.fullscreen = true;
            }
            _ => {}
        }
    }
    flags
}

impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrForeignToplevelManagerV1,
        event: zwlr_foreign_toplevel_manager_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let zwlr_foreign_toplevel_manager_v1::Event::Toplevel { toplevel } = event {
            state
                .toplevels
                .windows
                .insert(toplevel.id().protocol_id(), Window::default());
        }
    }

    event_created_child!(WaylandState, ZwlrForeignToplevelManagerV1, [
        zwlr_foreign_toplevel_manager_v1::EVT_TOPLEVEL_OPCODE => (ZwlrForeignToplevelHandleV1, ()),
    ]);
}

impl Dispatch<ZwlrForeignToplevelHandleV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        proxy: &ZwlrForeignToplevelHandleV1,
        event: zwlr_foreign_toplevel_handle_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use zwlr_foreign_toplevel_handle_v1::Event;
        let windows = &mut state.toplevels.windows;
        match event {
            Event::State { state: array } => {
                if let Some(window) = windows.get_mut(&proxy.id().protocol_id()) {
                    window.pending = flags(&array);
                }
            }
            Event::Done => {
                if let Some(window) = windows.get_mut(&proxy.id().protocol_id()) {
                    window.current = window.pending;
                }
            }
            Event::Closed => {
                windows.remove(&proxy.id().protocol_id());
                proxy.destroy();
            }
            // Title, app id, outputs, parent: not ours to keep.
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zwlr_foreign_toplevel_handle_v1::State;

    fn array(states: &[State]) -> Vec<u8> {
        states
            .iter()
            .flat_map(|s| (*s as u32).to_ne_bytes())
            .collect()
    }

    #[test]
    fn a_state_array_reads_as_its_flags() {
        assert_eq!(flags(&array(&[])), Flags::default());
        assert_eq!(
            flags(&array(&[State::Maximized, State::Activated])),
            Flags {
                activated: true,
                fullscreen: false
            }
        );
        assert_eq!(
            flags(&array(&[State::Fullscreen, State::Activated])),
            Flags {
                activated: true,
                fullscreen: true
            }
        );
        // A trailing partial word is ignored, not misread.
        let mut odd = array(&[State::Fullscreen]);
        odd.push(2);
        assert!(flags(&odd).fullscreen);
    }

    #[test]
    fn only_the_window_in_front_counts() {
        let window = |activated, fullscreen| Window {
            pending: Flags::default(),
            current: Flags {
                activated,
                fullscreen,
            },
        };
        let mut toplevels = Toplevels::default();
        assert!(!toplevels.fullscreen_in_front(), "no windows at all");
        toplevels.windows.insert(1, window(false, true));
        toplevels.windows.insert(2, window(true, false));
        assert!(
            !toplevels.fullscreen_in_front(),
            "full screen but behind, and in front but windowed"
        );
        toplevels.windows.insert(3, window(true, true));
        assert!(toplevels.fullscreen_in_front());
        toplevels.windows.remove(&3);
        assert!(!toplevels.fullscreen_in_front(), "closed");
    }
}
