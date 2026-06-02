//! Keyboard shortcut registry.
//!
//! The actual key dispatch still lives in [`crate::app::App`] (winit's
//! `WindowEvent::KeyboardInput`) — this module is the *canonical
//! reference* of what each binding does and how to render it in the
//! settings UI / command palette.
//!
//! Single source of truth for:
//!
//! - The human-readable label of each action (`Action::label`)
//! - Its default shortcut (`Action::default_combo`)
//! - A short rationale (`Action::description`)
//!
//! Adding a new shortcut means: add a variant, fill in the three
//! arms, then wire the actual handler in `App::user_event` /
//! `WindowEvent::KeyboardInput`. The unit tests at the bottom enforce
//! that every variant returns non-empty strings so a missing
//! description can't slip through.

/// One discrete keyboard-triggered action. Order matches the natural
/// grouping shown in the Appearance tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    // ── Global / overlay ──
    ToggleEditMode,
    HideOverlay,
    PauseAll,

    // ── Window / persistence ──
    QuitWithSave,
    SaveNow,
    OpenCommandPalette,

    // ── Selection / navigation ──
    CycleEntity,
    DeleteSelected,
    NudgeUp,
    NudgeDown,
    NudgeLeft,
    NudgeRight,
    CenterOnScreen,

    // ── Per-entity quick toggles ──
    ToggleVisible,
    ToggleGravity,
    TogglePlayback,
    DuplicateSelected,
    ResetTransform,

    // ── Z-order ──
    BringForward,
    SendBackward,

    // ── Misc ──
    ShowHelp,
}

impl Action {
    /// All actions, in their canonical display order.
    pub const ALL: &'static [Self] = &[
        Self::ToggleEditMode,
        Self::HideOverlay,
        Self::PauseAll,
        Self::QuitWithSave,
        Self::SaveNow,
        Self::OpenCommandPalette,
        Self::CycleEntity,
        Self::DeleteSelected,
        Self::NudgeUp,
        Self::NudgeDown,
        Self::NudgeLeft,
        Self::NudgeRight,
        Self::CenterOnScreen,
        Self::ToggleVisible,
        Self::ToggleGravity,
        Self::TogglePlayback,
        Self::DuplicateSelected,
        Self::ResetTransform,
        Self::BringForward,
        Self::SendBackward,
        Self::ShowHelp,
    ];

    /// Short human-readable label for the settings panel and command
    /// palette. Stays under ~30 chars so the right column never wraps.
    pub fn label(self) -> &'static str {
        match self {
            Self::ToggleEditMode => "Toggle edit mode",
            Self::HideOverlay => "Hide / show overlay",
            Self::PauseAll => "Pause all animations",
            Self::QuitWithSave => "Quit (save config)",
            Self::SaveNow => "Save config now",
            Self::OpenCommandPalette => "Command palette",
            Self::CycleEntity => "Cycle to next entity",
            Self::DeleteSelected => "Delete selected entity",
            Self::NudgeUp => "Nudge selection up",
            Self::NudgeDown => "Nudge selection down",
            Self::NudgeLeft => "Nudge selection left",
            Self::NudgeRight => "Nudge selection right",
            Self::CenterOnScreen => "Center selection on screen",
            Self::ToggleVisible => "Toggle visibility",
            Self::ToggleGravity => "Toggle gravity",
            Self::TogglePlayback => "Toggle play/pause",
            Self::DuplicateSelected => "Duplicate selection",
            Self::ResetTransform => "Reset scale / opacity",
            Self::BringForward => "Bring selection forward",
            Self::SendBackward => "Send selection backward",
            Self::ShowHelp => "Show keyboard help",
        }
    }

    /// One-line *why* this action exists. Surfaced as the secondary
    /// text in the command palette so users can tell similar actions
    /// apart even before they trigger one.
    pub fn description(self) -> &'static str {
        match self {
            Self::ToggleEditMode => "Switch between pass-through and edit mode.",
            Self::HideOverlay => "Show or hide the whole overlay (tray-compatible).",
            Self::PauseAll => "Freeze every animation. Useful for screenshots.",
            Self::QuitWithSave => "Persist any pending edits and exit.",
            Self::SaveNow => "Force-write the config without exiting.",
            Self::OpenCommandPalette => "Search and run any action from one prompt.",
            Self::CycleEntity => "Step through every entity in z-order.",
            Self::DeleteSelected => "Remove the selected entity from the scene.",
            Self::NudgeUp => "Move the selection 10 px up (1 px with Shift).",
            Self::NudgeDown => "Move the selection 10 px down (1 px with Shift).",
            Self::NudgeLeft => "Move the selection 10 px left (1 px with Shift).",
            Self::NudgeRight => "Move the selection 10 px right (1 px with Shift).",
            Self::CenterOnScreen => "Snap the selection to the screen centre.",
            Self::ToggleVisible => "Show or hide the selected entity.",
            Self::ToggleGravity => "Enable or disable physics on the selection.",
            Self::TogglePlayback => "Play or pause the selected entity's animation.",
            Self::DuplicateSelected => "Spawn a copy of the selected entity nearby.",
            Self::ResetTransform => "Restore scale 1.0 and opacity 1.0.",
            Self::BringForward => "Raise the selected entity by one z-step.",
            Self::SendBackward => "Lower the selected entity by one z-step.",
            Self::ShowHelp => "Print every shortcut in a toast.",
        }
    }

    /// Pre-formatted key combo as displayed in the UI. Mirrors the
    /// actual binding registered in `App::handle_window_event` /
    /// `hotkeys::register`. If you change the dispatch, change the
    /// string here too — the unit tests assert non-empty but cannot
    /// cross-check against winit.
    pub fn default_combo(self) -> &'static str {
        match self {
            Self::ToggleEditMode => "Ctrl+Shift+A  /  Esc",
            Self::HideOverlay => "Ctrl+Shift+H",
            Self::PauseAll => "Ctrl+Shift+P  /  Space",
            Self::QuitWithSave => "Q",
            Self::SaveNow => "S",
            Self::OpenCommandPalette => "Ctrl+K",
            Self::CycleEntity => "Tab",
            Self::DeleteSelected => "Delete  /  Backspace",
            Self::NudgeUp => "↑",
            Self::NudgeDown => "↓",
            Self::NudgeLeft => "←",
            Self::NudgeRight => "→",
            Self::CenterOnScreen => "Home",
            Self::ToggleVisible => "V",
            Self::ToggleGravity => "G",
            Self::TogglePlayback => "P",
            Self::DuplicateSelected => "D",
            Self::ResetTransform => "R",
            Self::BringForward => "Page Up",
            Self::SendBackward => "Page Down",
            Self::ShowHelp => "H",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Each variant must populate all three fields. Empty strings here
    /// would render as blank rows in the UI — catch them at build
    /// time instead.
    #[test]
    fn every_action_has_full_metadata() {
        for action in Action::ALL {
            assert!(!action.label().is_empty(), "{action:?} has empty label");
            assert!(
                !action.description().is_empty(),
                "{action:?} has empty description",
            );
            assert!(
                !action.default_combo().is_empty(),
                "{action:?} has empty combo",
            );
        }
    }

    /// `ALL` must include every variant exactly once, otherwise the
    /// command palette / settings table would silently skip some.
    #[test]
    fn all_covers_every_variant_uniquely() {
        let set: HashSet<&Action> = Action::ALL.iter().collect();
        assert_eq!(set.len(), Action::ALL.len(), "ALL contains duplicates");
        // Sanity-check the count matches what we wrote in the enum.
        // Bumped manually each time a variant is added.
        assert_eq!(Action::ALL.len(), 21);
    }

    /// Labels feed into accessibility tools — bound the width so a
    /// new action can't accidentally make the settings column wrap.
    #[test]
    fn labels_fit_in_settings_column() {
        for action in Action::ALL {
            assert!(
                action.label().len() <= 35,
                "{action:?} label too long: {:?}",
                action.label(),
            );
        }
    }
}
