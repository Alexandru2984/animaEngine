//! The tray menu, defined once for every platform's tray.
//!
//! The StatusNotifierItem tray (`tray.rs`) and the Windows notification-
//! area icon (`win_tray.rs`) render this list; neither spells out its own.
//! Two hand-kept menus would drift the way the backends' handlers did
//! (R41 in docs/runtime-findings.md).

use crate::event::AnimaEvent;

/// One entry in the tray menu.
#[derive(Debug, Clone, Copy)]
pub enum TrayItem {
    Action {
        label: &'static str,
        event: AnimaEvent,
    },
    Separator,
}

/// The menu, top to bottom.
pub const MENU: &[TrayItem] = &[
    TrayItem::Action {
        label: "Toggle edit mode",
        event: AnimaEvent::ToggleEditMode,
    },
    TrayItem::Action {
        label: "Toggle playback",
        event: AnimaEvent::ToggleGlobalPlayback,
    },
    TrayItem::Action {
        label: "Next scene",
        event: AnimaEvent::NextScene,
    },
    TrayItem::Separator,
    TrayItem::Action {
        label: "Show overlay",
        event: AnimaEvent::ShowOverlay,
    },
    TrayItem::Action {
        label: "Hide overlay",
        event: AnimaEvent::HideOverlay,
    },
    TrayItem::Separator,
    TrayItem::Action {
        label: "Quit",
        event: AnimaEvent::Quit,
    },
];

/// What activating the icon itself does: the most-used action.
pub const ACTIVATE: AnimaEvent = AnimaEvent::ToggleEditMode;

/// The tooltip / title the icon carries.
pub const TITLE: &str = "animaEngine";

#[cfg(test)]
mod tests {
    use super::*;

    /// Quit must be reachable from the tray: on a session without global
    /// hotkeys and with the overlay hidden, it is the only way out.
    #[test]
    fn quit_is_on_the_menu() {
        assert!(MENU.iter().any(|item| matches!(
            item,
            TrayItem::Action {
                event: AnimaEvent::Quit,
                ..
            }
        )));
    }

    #[test]
    fn the_menu_neither_starts_nor_ends_with_a_separator() {
        assert!(matches!(MENU.first(), Some(TrayItem::Action { .. })));
        assert!(matches!(MENU.last(), Some(TrayItem::Action { .. })));
    }
}
