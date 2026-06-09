//! `Action` enum + every per-variant metadata table. Extracted in J.3.
//!
//! Adding a new action requires updating:
//! - the enum body,
//! - `ALL`,
//! - `label` and `description` (for the command palette),
//! - the `C_*` static chord array + the `default_chords` arm,
//! - the `i18n_key` arm,
//! - a Fluent message in every locale file.
//!
//! Declaring each default chord list as an associated `const` (rather
//! than `&[KeyChord::new(...)]` directly in a match arm) is required
//! because the `&[…]` form does not const-promote — the borrow checker
//! rejects it as a temporary.

use super::chord::KeyChord;
use super::keys::{KeyCode, ModifierMask, NamedKey, SymbolKey};
use serde::{Deserialize, Serialize};

/// One rebindable action. Single source of truth for the dispatch
/// table and the UI rebind tab. Adding a variant requires updating
/// `ALL`, `label`, `description`, and `default_chords`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

    // ── Z-order & rate ──
    BringForward,
    SendBackward,
    FpsUp,
    FpsDown,

    // ── Appearance modulation ──
    OpacityUp,
    OpacityDown,

    // ── Misc ──
    CycleMonitor,
    ShowEntityInfo,
    ShowHelp,

    // ── Dev tools ──
    /// Toggles the in-app frame-time + per-system perf overlay (D.6).
    /// Bound to `Ctrl+Shift+`` by default. Discoverable for power
    /// users via the Keybindings tab; doesn't appear in any onboarding
    /// or help surface — dev affordance, not a feature.
    TogglePerfOverlay,
}

impl Action {
    /// Every variant in canonical display order. Drives the settings
    /// table, the rebinding UI, and the command palette listing.
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
        Self::FpsUp,
        Self::FpsDown,
        Self::OpacityUp,
        Self::OpacityDown,
        Self::CycleMonitor,
        Self::ShowEntityInfo,
        Self::ShowHelp,
        Self::TogglePerfOverlay,
    ];

    /// Short human-readable label for the settings panel and command
    /// palette. Stays under ~35 chars so the right column never wraps.
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
            Self::FpsUp => "Increase FPS",
            Self::FpsDown => "Decrease FPS",
            Self::OpacityUp => "Increase opacity",
            Self::OpacityDown => "Decrease opacity",
            Self::CycleMonitor => "Cycle entity monitor pin",
            Self::ShowEntityInfo => "Show entity info",
            Self::ShowHelp => "Show keyboard help",
            Self::TogglePerfOverlay => "Toggle perf overlay",
        }
    }

    /// One-line *why* the action exists. Secondary text in the command
    /// palette so similar actions are distinguishable before firing.
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
            Self::FpsUp => "Speed the selected entity's animation up by 2 fps.",
            Self::FpsDown => "Slow the selected entity's animation by 2 fps.",
            Self::OpacityUp => "Make the selected entity 10 % more opaque.",
            Self::OpacityDown => "Make the selected entity 10 % more transparent.",
            Self::CycleMonitor => "Pin the selected entity to the next monitor.",
            Self::ShowEntityInfo => "Print the selection's full state to the log.",
            Self::ShowHelp => "Print every shortcut in a toast.",
            Self::TogglePerfOverlay => "Show or hide the live FPS / frame-time overlay.",
        }
    }

    // ── Per-action default chord arrays ──
    //
    // Declared as associated `const` items so each `&'static [KeyChord]`
    // gets proper static storage — `&[KeyChord::new(...)]` returned
    // directly from a match arm doesn't const-promote and fails to
    // borrow-check.
    const C_TOGGLE_EDIT_MODE: &'static [KeyChord] = &[
        KeyChord::new(ModifierMask(0b0011), KeyCode::Letter('A')),
        KeyChord::new(ModifierMask::NONE, KeyCode::Named(NamedKey::Escape)),
    ];
    const C_HIDE_OVERLAY: &'static [KeyChord] =
        &[KeyChord::new(ModifierMask(0b0011), KeyCode::Letter('H'))];
    const C_PAUSE_ALL: &'static [KeyChord] = &[
        KeyChord::new(ModifierMask(0b0011), KeyCode::Letter('P')),
        KeyChord::new(ModifierMask::NONE, KeyCode::Named(NamedKey::Space)),
    ];
    const C_QUIT_WITH_SAVE: &'static [KeyChord] =
        &[KeyChord::new(ModifierMask::NONE, KeyCode::Letter('Q'))];
    const C_SAVE_NOW: &'static [KeyChord] =
        &[KeyChord::new(ModifierMask::NONE, KeyCode::Letter('S'))];
    const C_OPEN_CMD_PALETTE: &'static [KeyChord] =
        &[KeyChord::new(ModifierMask::CTRL, KeyCode::Letter('K'))];
    const C_CYCLE_ENTITY: &'static [KeyChord] = &[KeyChord::new(
        ModifierMask::NONE,
        KeyCode::Named(NamedKey::Tab),
    )];
    const C_DELETE_SELECTED: &'static [KeyChord] = &[
        KeyChord::new(ModifierMask::NONE, KeyCode::Named(NamedKey::Delete)),
        KeyChord::new(ModifierMask::NONE, KeyCode::Named(NamedKey::Backspace)),
    ];
    const C_NUDGE_UP: &'static [KeyChord] = &[KeyChord::new(
        ModifierMask::NONE,
        KeyCode::Named(NamedKey::ArrowUp),
    )];
    const C_NUDGE_DOWN: &'static [KeyChord] = &[KeyChord::new(
        ModifierMask::NONE,
        KeyCode::Named(NamedKey::ArrowDown),
    )];
    const C_NUDGE_LEFT: &'static [KeyChord] = &[KeyChord::new(
        ModifierMask::NONE,
        KeyCode::Named(NamedKey::ArrowLeft),
    )];
    const C_NUDGE_RIGHT: &'static [KeyChord] = &[KeyChord::new(
        ModifierMask::NONE,
        KeyCode::Named(NamedKey::ArrowRight),
    )];
    const C_CENTER_ON_SCREEN: &'static [KeyChord] = &[KeyChord::new(
        ModifierMask::NONE,
        KeyCode::Named(NamedKey::Home),
    )];
    const C_TOGGLE_VISIBLE: &'static [KeyChord] =
        &[KeyChord::new(ModifierMask::NONE, KeyCode::Letter('V'))];
    const C_TOGGLE_GRAVITY: &'static [KeyChord] =
        &[KeyChord::new(ModifierMask::NONE, KeyCode::Letter('G'))];
    const C_TOGGLE_PLAYBACK: &'static [KeyChord] =
        &[KeyChord::new(ModifierMask::NONE, KeyCode::Letter('P'))];
    const C_DUPLICATE: &'static [KeyChord] =
        &[KeyChord::new(ModifierMask::NONE, KeyCode::Letter('D'))];
    const C_RESET_TRANSFORM: &'static [KeyChord] =
        &[KeyChord::new(ModifierMask::NONE, KeyCode::Letter('R'))];
    const C_BRING_FORWARD: &'static [KeyChord] = &[KeyChord::new(
        ModifierMask::NONE,
        KeyCode::Named(NamedKey::PageUp),
    )];
    const C_SEND_BACKWARD: &'static [KeyChord] = &[KeyChord::new(
        ModifierMask::NONE,
        KeyCode::Named(NamedKey::PageDown),
    )];
    const C_FPS_UP: &'static [KeyChord] = &[KeyChord::new(
        ModifierMask::NONE,
        KeyCode::Symbol(SymbolKey::BracketRight),
    )];
    const C_FPS_DOWN: &'static [KeyChord] = &[KeyChord::new(
        ModifierMask::NONE,
        KeyCode::Symbol(SymbolKey::BracketLeft),
    )];
    const C_OPACITY_UP: &'static [KeyChord] = &[
        KeyChord::new(ModifierMask::NONE, KeyCode::Symbol(SymbolKey::Plus)),
        KeyChord::new(ModifierMask::NONE, KeyCode::Symbol(SymbolKey::Equal)),
    ];
    const C_OPACITY_DOWN: &'static [KeyChord] = &[KeyChord::new(
        ModifierMask::NONE,
        KeyCode::Symbol(SymbolKey::Minus),
    )];
    const C_CYCLE_MONITOR: &'static [KeyChord] =
        &[KeyChord::new(ModifierMask::CTRL, KeyCode::Letter('M'))];
    const C_SHOW_ENTITY_INFO: &'static [KeyChord] =
        &[KeyChord::new(ModifierMask::NONE, KeyCode::Letter('I'))];
    const C_SHOW_HELP: &'static [KeyChord] =
        &[KeyChord::new(ModifierMask::NONE, KeyCode::Letter('H'))];
    const C_TOGGLE_PERF_OVERLAY: &'static [KeyChord] = &[KeyChord::new(
        ModifierMask(0b0011),
        KeyCode::Symbol(SymbolKey::Backquote),
    )];

    /// Default chord set for this action. The UI rebind tab starts
    /// from this; user overrides land in `KeyBindings.map`.
    pub fn default_chords(self) -> &'static [KeyChord] {
        match self {
            Self::ToggleEditMode => Self::C_TOGGLE_EDIT_MODE,
            Self::HideOverlay => Self::C_HIDE_OVERLAY,
            Self::PauseAll => Self::C_PAUSE_ALL,
            Self::QuitWithSave => Self::C_QUIT_WITH_SAVE,
            Self::SaveNow => Self::C_SAVE_NOW,
            Self::OpenCommandPalette => Self::C_OPEN_CMD_PALETTE,
            Self::CycleEntity => Self::C_CYCLE_ENTITY,
            Self::DeleteSelected => Self::C_DELETE_SELECTED,
            Self::NudgeUp => Self::C_NUDGE_UP,
            Self::NudgeDown => Self::C_NUDGE_DOWN,
            Self::NudgeLeft => Self::C_NUDGE_LEFT,
            Self::NudgeRight => Self::C_NUDGE_RIGHT,
            Self::CenterOnScreen => Self::C_CENTER_ON_SCREEN,
            Self::ToggleVisible => Self::C_TOGGLE_VISIBLE,
            Self::ToggleGravity => Self::C_TOGGLE_GRAVITY,
            Self::TogglePlayback => Self::C_TOGGLE_PLAYBACK,
            Self::DuplicateSelected => Self::C_DUPLICATE,
            Self::ResetTransform => Self::C_RESET_TRANSFORM,
            Self::BringForward => Self::C_BRING_FORWARD,
            Self::SendBackward => Self::C_SEND_BACKWARD,
            Self::FpsUp => Self::C_FPS_UP,
            Self::FpsDown => Self::C_FPS_DOWN,
            Self::OpacityUp => Self::C_OPACITY_UP,
            Self::OpacityDown => Self::C_OPACITY_DOWN,
            Self::CycleMonitor => Self::C_CYCLE_MONITOR,
            Self::ShowEntityInfo => Self::C_SHOW_ENTITY_INFO,
            Self::ShowHelp => Self::C_SHOW_HELP,
            Self::TogglePerfOverlay => Self::C_TOGGLE_PERF_OVERLAY,
        }
    }

    /// Join the default chords' display strings with `  /  ` for the
    /// legacy reference table in the Appearance tab. The rebind UI
    /// uses `KeyBindings::chords_for` instead.
    pub fn default_combo(self) -> String {
        self.default_chords()
            .iter()
            .map(|c| c.display_str())
            .collect::<Vec<_>>()
            .join("  /  ")
    }

    /// Stable Fluent message id like `action-toggle-edit-mode`. Used
    /// by the rebind UI and command palette so the action label
    /// localizes without forcing every locale file to retain English
    /// fallbacks. Fluent restricts message ids to ASCII letters,
    /// digits, `-`, and `_`; we use `-` to match the existing locale
    /// files' kebab-case convention.
    pub fn i18n_key(self) -> &'static str {
        match self {
            Self::ToggleEditMode => "action-toggle-edit-mode",
            Self::HideOverlay => "action-hide-overlay",
            Self::PauseAll => "action-pause-all",
            Self::QuitWithSave => "action-quit-with-save",
            Self::SaveNow => "action-save-now",
            Self::OpenCommandPalette => "action-open-command-palette",
            Self::CycleEntity => "action-cycle-entity",
            Self::DeleteSelected => "action-delete-selected",
            Self::NudgeUp => "action-nudge-up",
            Self::NudgeDown => "action-nudge-down",
            Self::NudgeLeft => "action-nudge-left",
            Self::NudgeRight => "action-nudge-right",
            Self::CenterOnScreen => "action-center-on-screen",
            Self::ToggleVisible => "action-toggle-visible",
            Self::ToggleGravity => "action-toggle-gravity",
            Self::TogglePlayback => "action-toggle-playback",
            Self::DuplicateSelected => "action-duplicate-selected",
            Self::ResetTransform => "action-reset-transform",
            Self::BringForward => "action-bring-forward",
            Self::SendBackward => "action-send-backward",
            Self::FpsUp => "action-fps-up",
            Self::FpsDown => "action-fps-down",
            Self::OpacityUp => "action-opacity-up",
            Self::OpacityDown => "action-opacity-down",
            Self::CycleMonitor => "action-cycle-monitor",
            Self::ShowEntityInfo => "action-show-entity-info",
            Self::ShowHelp => "action-show-help",
            Self::TogglePerfOverlay => "action-toggle-perf-overlay",
        }
    }
}
