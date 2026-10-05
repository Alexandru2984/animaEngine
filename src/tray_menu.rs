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
    /// Snoozing (`crate::snooze`): the choices, or how long it lasts and
    /// a way out — see [`entries`].
    Snooze,
    /// The focus timer (`crate::focus`): a start, or which part it is in
    /// until when and a stop.
    Focus,
    Separator,
}

/// One line of the menu as shown now ([`entries`]).
#[derive(Debug, Clone)]
pub enum Entry {
    /// A label, and what choosing it sends — `None` shows it greyed out.
    Item {
        label: String,
        event: Option<AnimaEvent>,
    },
    Separator,
}

/// The menu as it should be shown now: [`MENU`] with the snooze and
/// focus blocks filled in. Every tray renders this.
pub fn entries() -> Vec<Entry> {
    use crate::snooze::SnoozeFor;
    let item = |label: &str, event: AnimaEvent| Entry::Item {
        label: label.to_string(),
        event: Some(event),
    };
    let mut out = Vec::new();
    for entry in MENU {
        match *entry {
            TrayItem::Action { label, event } => out.push(item(label, event)),
            TrayItem::Separator => out.push(Entry::Separator),
            TrayItem::Focus => match crate::focus::until_label() {
                Some((phase, until)) => {
                    let label = match phase {
                        crate::focus::Phase::Work => format!("Focusing until {until}"),
                        crate::focus::Phase::Break => format!("On a break until {until}"),
                    };
                    out.push(Entry::Item { label, event: None });
                    out.push(item("Stop focus", AnimaEvent::StopFocus));
                }
                None if crate::focus::phase().is_some() => {
                    out.push(item("Stop focus", AnimaEvent::StopFocus));
                }
                None => out.push(item("Start focus", AnimaEvent::StartFocus(0))),
            },
            TrayItem::Snooze => match crate::snooze::until_label() {
                Some(until) => {
                    out.push(Entry::Item {
                        label: format!("Snoozing until {until}"),
                        event: None,
                    });
                    out.push(item("Wake up now", AnimaEvent::Wake));
                }
                None if crate::snooze::active() => {
                    out.push(item("Wake up now", AnimaEvent::Wake));
                }
                None => {
                    out.push(item(
                        "Snooze 15 minutes",
                        AnimaEvent::Snooze(SnoozeFor::Minutes(15)),
                    ));
                    out.push(item(
                        "Snooze 1 hour",
                        AnimaEvent::Snooze(SnoozeFor::Minutes(60)),
                    ));
                    out.push(item(
                        "Snooze until morning",
                        AnimaEvent::Snooze(SnoozeFor::Morning),
                    ));
                }
            },
        }
    }
    out
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
    TrayItem::Focus,
    TrayItem::Snooze,
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

    /// The focus block offers a start, or until when and a stop.
    #[test]
    fn focusing_shows_in_the_menu() {
        let labels = |entries: Vec<Entry>| -> Vec<String> {
            entries
                .into_iter()
                .filter_map(|e| match e {
                    Entry::Item { label, .. } => Some(label),
                    Entry::Separator => None,
                })
                .collect()
        };
        let _turn = crate::focus::TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        crate::focus::stop();
        assert!(labels(entries()).iter().any(|l| l == "Start focus"));
        crate::focus::start(crate::focus::FocusConfig::default());
        let now = labels(entries());
        crate::focus::stop();
        assert!(now.iter().any(|l| l == "Stop focus"), "{now:?}");
        assert!(!now.iter().any(|l| l == "Start focus"));
    }

    /// The snooze block offers the choices, or how long and a way out.
    #[test]
    fn snoozing_shows_in_the_menu() {
        let labels = |entries: Vec<Entry>| -> Vec<String> {
            entries
                .into_iter()
                .filter_map(|e| match e {
                    Entry::Item { label, .. } => Some(label),
                    Entry::Separator => None,
                })
                .collect()
        };
        let _turn = crate::snooze::TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        crate::snooze::wake();
        assert!(labels(entries()).iter().any(|l| l == "Snooze 1 hour"));
        crate::snooze::snooze(crate::snooze::SnoozeFor::Minutes(30));
        let now = labels(entries());
        crate::snooze::wake();
        assert!(now.iter().any(|l| l == "Wake up now"), "{now:?}");
        assert!(!now.iter().any(|l| l == "Snooze 1 hour"));
    }

    #[test]
    fn the_menu_neither_starts_nor_ends_with_a_separator() {
        assert!(matches!(MENU.first(), Some(TrayItem::Action { .. })));
        assert!(matches!(MENU.last(), Some(TrayItem::Action { .. })));
    }
}
