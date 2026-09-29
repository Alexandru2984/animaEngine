//! `Action` enum + every per-variant metadata table. Extracted in J.3.
//!
//! Adding a new action requires updating:
//! - the enum body,
//! - `ALL`,
//! - `label` and `description` (the English name and a one-line summary),
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

    // ── Edit history ──
    /// Undo the last edit to the scene (`crate::undo`).
    Undo,
    /// Redo the last undone edit.
    Redo,

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

    // ── Clipboard ──
    /// Keep the selection for a paste (`crate::clipboard`).
    CopySelected,
    /// Keep the selection for a paste and take it out of the scene.
    CutSelected,
    /// Add what was copied — into this scene or another one.
    Paste,

    // ── Groups ──
    /// Make a group of the selected characters (`crate::group`).
    GroupSelected,
    /// Dissolve the groups the selected characters are in.
    UngroupSelected,

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
    /// Every variant in canonical display order. Drives the Keybindings
    /// tab: its table and the rebinding UI.
    pub const ALL: &'static [Self] = &[
        Self::ToggleEditMode,
        Self::HideOverlay,
        Self::PauseAll,
        Self::QuitWithSave,
        Self::SaveNow,
        Self::OpenCommandPalette,
        Self::Undo,
        Self::Redo,
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
        Self::CopySelected,
        Self::CutSelected,
        Self::Paste,
        Self::GroupSelected,
        Self::UngroupSelected,
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

    /// Whether the action works on the selected character, and so does
    /// nothing while none is selected. The command palette lists these
    /// only when something is selected, rather than offer what cannot
    /// happen.
    pub fn acts_on_selection(self) -> bool {
        matches!(
            self,
            Self::DeleteSelected
                | Self::NudgeUp
                | Self::NudgeDown
                | Self::NudgeLeft
                | Self::NudgeRight
                | Self::CenterOnScreen
                | Self::ToggleVisible
                | Self::ToggleGravity
                | Self::TogglePlayback
                | Self::DuplicateSelected
                | Self::ResetTransform
                | Self::CopySelected
                | Self::CutSelected
                | Self::GroupSelected
                | Self::UngroupSelected
                | Self::BringForward
                | Self::SendBackward
                | Self::FpsUp
                | Self::FpsDown
                | Self::OpacityUp
                | Self::OpacityDown
                | Self::CycleMonitor
                | Self::ShowEntityInfo
        )
    }

    /// Whether a drag or selection rectangle in progress lets go before
    /// this runs (`crate::input::multi::cancel`). These change which
    /// characters exist or are selected, and a gesture holds indices into
    /// both: Undo pressed mid-drag was lost at the next pointer move, which
    /// put the characters back under it, and Duplicate sent the copies
    /// flying.
    pub fn interrupts_drag(self) -> bool {
        matches!(
            self,
            Self::Undo
                | Self::Redo
                | Self::CycleEntity
                | Self::DeleteSelected
                | Self::DuplicateSelected
                | Self::CutSelected
                | Self::Paste
        )
    }

    /// Whether the command palette offers this action. Not the palette's
    /// own shortcut, and not Hide overlay: that one is a global shortcut
    /// by design, because a hidden overlay can only be brought back from
    /// outside it — by the shortcut, the tray or D-Bus.
    pub fn in_palette(self) -> bool {
        !matches!(self, Self::OpenCommandPalette | Self::HideOverlay)
    }

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
            Self::Undo => "Undo",
            Self::Redo => "Redo",
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
            Self::CopySelected => "Copy selection",
            Self::CutSelected => "Cut selection",
            Self::Paste => "Paste",
            Self::GroupSelected => "Group selection",
            Self::UngroupSelected => "Ungroup selection",
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
            Self::OpenCommandPalette => {
                "Search themes and presets, and apply one from the keyboard."
            }
            Self::Undo => "Take back the last change to the scene.",
            Self::Redo => "Put back the last change you undid.",
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
            Self::CopySelected => "Keep the selection to paste, here or in another scene.",
            Self::CutSelected => "Copy the selection and remove it, to paste elsewhere.",
            Self::Paste => "Add the copied characters to this scene.",
            Self::GroupSelected => "Make a group of the selection; a click then takes all of it.",
            Self::UngroupSelected => "Dissolve the groups the selection is in.",
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
    const C_UNDO: &'static [KeyChord] = &[KeyChord::new(ModifierMask::CTRL, KeyCode::Letter('Z'))];
    const C_REDO: &'static [KeyChord] = &[
        KeyChord::new(ModifierMask(0b0011), KeyCode::Letter('Z')),
        KeyChord::new(ModifierMask::CTRL, KeyCode::Letter('Y')),
    ];
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
    const C_COPY: &'static [KeyChord] = &[KeyChord::new(ModifierMask::CTRL, KeyCode::Letter('C'))];
    const C_CUT: &'static [KeyChord] = &[KeyChord::new(ModifierMask::CTRL, KeyCode::Letter('X'))];
    const C_PASTE: &'static [KeyChord] = &[KeyChord::new(ModifierMask::CTRL, KeyCode::Letter('V'))];
    const C_GROUP: &'static [KeyChord] = &[KeyChord::new(ModifierMask::CTRL, KeyCode::Letter('G'))];
    const C_UNGROUP: &'static [KeyChord] =
        &[KeyChord::new(ModifierMask(0b0011), KeyCode::Letter('G'))];
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
            Self::Undo => Self::C_UNDO,
            Self::Redo => Self::C_REDO,
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
            Self::CopySelected => Self::C_COPY,
            Self::CutSelected => Self::C_CUT,
            Self::Paste => Self::C_PASTE,
            Self::GroupSelected => Self::C_GROUP,
            Self::UngroupSelected => Self::C_UNGROUP,
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

    /// Stable Fluent message id like `action-toggle-edit-mode`. Used
    /// by the Keybindings tab so the action label
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
            Self::Undo => "action-undo",
            Self::Redo => "action-redo",
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
            Self::CopySelected => "action-copy-selected",
            Self::CutSelected => "action-cut-selected",
            Self::Paste => "action-paste",
            Self::GroupSelected => "action-group-selected",
            Self::UngroupSelected => "action-ungroup-selected",
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
