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
        /// The i18n key of its label: the menu is in the app's language,
        /// shown again when that changes (see `crate::tray`).
        key: &'static str,
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

/// The menu as it should be shown now, in the app's language: [`MENU`]
/// with the snooze and focus blocks filled in. Every tray renders this.
pub fn entries() -> Vec<Entry> {
    use crate::i18n::{t, t_args};
    use crate::snooze::SnoozeFor;
    let item = |label: String, event: AnimaEvent| Entry::Item {
        label,
        event: Some(event),
    };
    let until = |key: &str, until: String| {
        let mut args = fluent::FluentArgs::new();
        args.set("until", until);
        Entry::Item {
            label: t_args(key, &args),
            event: None,
        }
    };
    let snooze_for = |minutes: u32| {
        let mut args = fluent::FluentArgs::new();
        args.set("minutes", minutes);
        item(
            t_args("palette-snooze-minutes", &args),
            AnimaEvent::Snooze(SnoozeFor::Minutes(minutes)),
        )
    };
    let mut out = Vec::new();
    for entry in MENU {
        match *entry {
            TrayItem::Action { key, event } => out.push(item(t(key), event)),
            TrayItem::Separator => out.push(Entry::Separator),
            TrayItem::Focus => match crate::focus::until_label() {
                Some((phase, at)) => {
                    out.push(until(
                        match phase {
                            crate::focus::Phase::Work => "scene-focus-working",
                            crate::focus::Phase::Break => "scene-focus-on-break",
                        },
                        at,
                    ));
                    out.push(item(t("palette-focus-stop"), AnimaEvent::StopFocus));
                }
                None if crate::focus::phase().is_some() => {
                    out.push(item(t("palette-focus-stop"), AnimaEvent::StopFocus));
                }
                None => out.push(item(t("palette-focus-start"), AnimaEvent::StartFocus(0))),
            },
            TrayItem::Snooze => match crate::snooze::until_label() {
                Some(at) => {
                    out.push(until("tray-snoozing-until", at));
                    out.push(item(t("tray-wake"), AnimaEvent::Wake));
                }
                None if crate::snooze::active() => {
                    out.push(item(t("tray-wake"), AnimaEvent::Wake));
                }
                None => {
                    out.push(snooze_for(15));
                    out.push(snooze_for(60));
                    out.push(item(
                        t("palette-snooze-morning"),
                        AnimaEvent::Snooze(SnoozeFor::Morning),
                    ));
                }
            },
        }
    }
    out
}

/// What the menu says now, for noticing that it changed: snoozing or
/// the focus timer began or ended, or the language is another.
pub fn labels() -> Vec<String> {
    entries()
        .into_iter()
        .filter_map(|e| match e {
            Entry::Item { label, .. } => Some(label),
            Entry::Separator => None,
        })
        .collect()
}

/// The menu, top to bottom.
pub const MENU: &[TrayItem] = &[
    TrayItem::Action {
        key: "action-toggle-edit-mode",
        event: AnimaEvent::ToggleEditMode,
    },
    TrayItem::Action {
        key: "action-toggle-playback",
        event: AnimaEvent::ToggleGlobalPlayback,
    },
    TrayItem::Action {
        key: "tray-next-scene",
        event: AnimaEvent::NextScene,
    },
    TrayItem::Separator,
    TrayItem::Action {
        key: "tray-show-overlay",
        event: AnimaEvent::ShowOverlay,
    },
    TrayItem::Action {
        key: "tray-hide-overlay",
        event: AnimaEvent::HideOverlay,
    },
    TrayItem::Separator,
    TrayItem::Focus,
    TrayItem::Snooze,
    TrayItem::Separator,
    TrayItem::Action {
        key: "tray-quit",
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
        let _turn = crate::focus::TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let (start, stop) = (
            crate::i18n::t("palette-focus-start"),
            crate::i18n::t("palette-focus-stop"),
        );
        crate::focus::stop();
        assert!(labels().contains(&start));
        crate::focus::start(crate::focus::FocusConfig::default());
        let now = labels();
        crate::focus::stop();
        assert!(now.contains(&stop), "{now:?}");
        assert!(!now.contains(&start));
    }

    /// The snooze block offers the choices, or how long and a way out.
    #[test]
    fn snoozing_shows_in_the_menu() {
        let _turn = crate::snooze::TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let mut hour = fluent::FluentArgs::new();
        hour.set("minutes", 60);
        let (an_hour, wake) = (
            crate::i18n::t_args("palette-snooze-minutes", &hour),
            crate::i18n::t("tray-wake"),
        );
        crate::snooze::wake();
        assert!(labels().contains(&an_hour));
        crate::snooze::snooze(crate::snooze::SnoozeFor::Minutes(30));
        let now = labels();
        crate::snooze::wake();
        assert!(now.contains(&wake), "{now:?}");
        assert!(!now.contains(&an_hour));
    }

    #[test]
    fn the_menu_neither_starts_nor_ends_with_a_separator() {
        assert!(matches!(MENU.first(), Some(TrayItem::Action { .. })));
        assert!(matches!(MENU.last(), Some(TrayItem::Action { .. })));
    }
}
