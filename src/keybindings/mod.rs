//! Rebindable keyboard shortcut system.
//!
//! Single source of truth for every action the user can trigger by
//! keyboard. The dispatch path (`KeyBindings::lookup`) accepts a
//! `KeyChord` built from winit input state and returns the bound
//! `Action`, which the call site dispatches via one match. Replaces
//! the scattered `Key::Character(...)` arms previously inlined in
//! `app.rs` and the hard-coded global shortcuts in `hotkeys.rs`.
//!
//! Bindings persist in `config.toml` under `[keybindings.map]`. Each
//! action maps to a list of chord strings (empty list = disabled).
//! Multiple bindings per action are allowed; for example
//! `ToggleEditMode` is bound to both `Ctrl+Shift+A` and `Esc` by
//! default. Chord strings round-trip through `KeyChord::FromStr`, so
//! hand-editing the config is supported.
//!
//! On config decode, any action missing from the map falls back to
//! its default chord set, so users upgrading from 0.3 don't silently
//! lose bindings introduced in 0.4.

mod keys;

pub use keys::{KeyCode, ModifierMask, NamedKey, SymbolKey};

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

// ── KeyChord ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyChord {
    pub mods: ModifierMask,
    pub key: KeyCode,
}

impl KeyChord {
    pub const fn new(mods: ModifierMask, key: KeyCode) -> Self {
        Self { mods, key }
    }

    /// Build a chord from egui's per-event input, returning `None`
    /// when the key isn't in our supported set (function keys, etc.).
    /// Used by the rebinding UI to capture the chord the user pressed.
    pub fn from_egui(key: egui::Key, mods: egui::Modifiers) -> Option<Self> {
        // Treat egui's `mac_cmd` / `command` as Super for cross-platform
        // consistency — the recorded chord can later be re-pressed on
        // any platform without losing the modifier identity.
        let mask = ModifierMask::from_state(
            mods.ctrl,
            mods.shift,
            mods.alt,
            mods.mac_cmd || mods.command,
        );
        Some(Self::new(mask, KeyCode::from_egui(key)?))
    }

    /// Render the chord in canonical TOML form (`Ctrl+Shift+A`).
    pub fn canonical_str(&self) -> String {
        let mut s = String::new();
        if self.mods.ctrl() {
            s.push_str("Ctrl+");
        }
        if self.mods.shift() {
            s.push_str("Shift+");
        }
        if self.mods.alt() {
            s.push_str("Alt+");
        }
        if self.mods.sup() {
            s.push_str("Super+");
        }
        s.push_str(&self.key.canonical_str());
        s
    }

    /// Render with arrow glyphs / abbreviations for the UI.
    pub fn display_str(&self) -> String {
        let mut s = String::new();
        if self.mods.ctrl() {
            s.push_str("Ctrl+");
        }
        if self.mods.shift() {
            s.push_str("Shift+");
        }
        if self.mods.alt() {
            s.push_str("Alt+");
        }
        if self.mods.sup() {
            s.push_str("Super+");
        }
        s.push_str(&self.key.display_str());
        s
    }
}

impl fmt::Display for KeyChord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.canonical_str())
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ChordParseError {
    #[error("empty chord")]
    Empty,
    #[error("empty key segment in chord")]
    EmptyKey,
    #[error("unknown modifier `{0}`")]
    UnknownModifier(String),
    #[error("unknown key `{0}`")]
    UnknownKey(String),
}

impl FromStr for KeyChord {
    type Err = ChordParseError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(ChordParseError::Empty);
        }
        // Walk modifier prefixes from the front, stopping at the first
        // segment that isn't a known modifier. The remainder is the
        // key. This handles `+` as a bound key (e.g. `Ctrl+Shift++` →
        // {ctrl+shift, '+'}) without naive `split('+')` ambiguity.
        let mut mods = ModifierMask::NONE;
        let mut rest = s;
        while let Some(plus_idx) = rest.find('+') {
            let prefix = &rest[..plus_idx];
            let bit = match prefix {
                "Ctrl" | "Control" => ModifierMask::CTRL,
                "Shift" => ModifierMask::SHIFT,
                "Alt" | "Option" => ModifierMask::ALT,
                "Super" | "Cmd" | "Meta" | "Win" => ModifierMask::SUPER,
                _ => break,
            };
            mods |= bit;
            rest = &rest[plus_idx + 1..];
        }
        let key = KeyCode::from_str(rest)?;
        Ok(Self { mods, key })
    }
}

impl Serialize for KeyChord {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.canonical_str())
    }
}

impl<'de> Deserialize<'de> for KeyChord {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

// ── Action ─────────────────────────────────────────────────────────────

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

// ── KeyBindings ────────────────────────────────────────────────────────

/// User-overridable mapping from `Action` to one or more `KeyChord`s.
/// Empty chord vector means the action is disabled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindings {
    /// Per-action chord lists. Missing entries fall back to defaults
    /// at lookup time, so adding a new action in a future release
    /// doesn't silently disable it for existing users.
    #[serde(default)]
    pub map: BTreeMap<Action, Vec<KeyChord>>,
}

impl Default for KeyBindings {
    fn default() -> Self {
        let mut map = BTreeMap::new();
        for &action in Action::ALL {
            map.insert(action, action.default_chords().to_vec());
        }
        Self { map }
    }
}

impl KeyBindings {
    /// Live chord list for `action`, falling back to defaults if the
    /// map has no entry yet. Returns an owned slice so the caller can
    /// freely iterate and render.
    pub fn chords_for(&self, action: Action) -> Vec<KeyChord> {
        self.map
            .get(&action)
            .cloned()
            .unwrap_or_else(|| action.default_chords().to_vec())
    }

    /// Find the action bound to `chord`. Linear scan — total bindings
    /// stay under ~50 so this is well below any UI-event budget. When
    /// multiple actions are bound to the same chord (a conflict), the
    /// first match wins; `conflicts` surfaces the issue in the UI.
    pub fn lookup(&self, chord: KeyChord) -> Option<Action> {
        for &action in Action::ALL {
            let chords = self
                .map
                .get(&action)
                .map(|v| v.as_slice())
                .unwrap_or_else(|| action.default_chords());
            if chords.contains(&chord) {
                return Some(action);
            }
        }
        None
    }

    /// All chord collisions in the current binding table: pairs of
    /// `(chord, actions)` where `actions.len() > 1`. The UI uses this
    /// to render a yellow warning next to conflicting rows.
    pub fn conflicts(&self) -> Vec<(KeyChord, Vec<Action>)> {
        let mut chord_owners: BTreeMap<KeyChord, Vec<Action>> = BTreeMap::new();
        for &action in Action::ALL {
            let chords = self
                .map
                .get(&action)
                .map(|v| v.as_slice())
                .unwrap_or_else(|| action.default_chords());
            for &chord in chords {
                chord_owners.entry(chord).or_default().push(action);
            }
        }
        chord_owners.retain(|_, actions| actions.len() > 1);
        chord_owners.into_iter().collect()
    }

    /// Reset a single action to its defaults; used by the per-row
    /// reset button in the rebind UI.
    pub fn reset_action(&mut self, action: Action) {
        self.map.insert(action, action.default_chords().to_vec());
    }

    /// Reset every action to its defaults; the footer "Reset all"
    /// button in the rebind UI.
    pub fn reset_all(&mut self) {
        *self = Self::default();
    }

    /// Add a chord to `action`. No-op if the chord is already bound to
    /// the same action (idempotent). Returns the conflicting action,
    /// if any — the UI decides whether to surface as warning or block.
    pub fn add_chord(&mut self, action: Action, chord: KeyChord) -> Option<Action> {
        let entry = self
            .map
            .entry(action)
            .or_insert_with(|| action.default_chords().to_vec());
        if !entry.contains(&chord) {
            entry.push(chord);
        }
        // Detect conflict against the rest of the table.
        for &other in Action::ALL {
            if other == action {
                continue;
            }
            let chords = self
                .map
                .get(&other)
                .map(|v| v.as_slice())
                .unwrap_or_else(|| other.default_chords());
            if chords.contains(&chord) {
                return Some(other);
            }
        }
        None
    }

    /// Remove a chord from `action`. No-op if the chord wasn't bound.
    pub fn remove_chord(&mut self, action: Action, chord: KeyChord) {
        if let Some(list) = self.map.get_mut(&action) {
            list.retain(|c| *c != chord);
        }
    }

    /// Persist by piggy-backing on `AppConfig::save` (no separate
    /// file). Provided for symmetry with other config-side helpers
    /// even though we never call it directly.
    pub fn validate(&self) -> Result<()> {
        // Every chord parses through Display → FromStr; nothing else
        // to enforce here. Future: warn on extremely unusual chords.
        Ok(())
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_action_has_full_metadata() {
        for &action in Action::ALL {
            assert!(!action.label().is_empty(), "{action:?} has empty label");
            assert!(
                !action.description().is_empty(),
                "{action:?} has empty description"
            );
            assert!(
                !action.default_chords().is_empty(),
                "{action:?} has no default chords"
            );
        }
    }

    #[test]
    fn all_covers_every_variant_uniquely() {
        let set: HashSet<&Action> = Action::ALL.iter().collect();
        assert_eq!(set.len(), Action::ALL.len(), "ALL contains duplicates");
        // Bumped manually when a variant is added.
        assert_eq!(Action::ALL.len(), 28);
    }

    #[test]
    fn labels_fit_in_settings_column() {
        for &action in Action::ALL {
            assert!(
                action.label().len() <= 35,
                "{action:?} label too long: {:?}",
                action.label(),
            );
        }
    }

    #[test]
    fn chord_round_trips_through_string() {
        let cases = [
            "Ctrl+Shift+A",
            "Esc",
            "Space",
            "ArrowUp",
            "Shift+ArrowDown",
            "Ctrl+K",
            "Tab",
            "PageUp",
            "+",
            "-",
            "Ctrl+M",
            "Q",
        ];
        for input in cases {
            let parsed: KeyChord = input.parse().expect(input);
            let round = parsed.canonical_str();
            let reparsed: KeyChord = round.parse().expect(&round);
            assert_eq!(parsed, reparsed, "round-trip failed for {input}");
        }
    }

    #[test]
    fn lowercase_letter_normalizes_to_upper() {
        let a: KeyChord = "a".parse().unwrap();
        let big_a: KeyChord = "A".parse().unwrap();
        assert_eq!(a, big_a);
    }

    #[test]
    fn modifier_aliases_parse() {
        let a: KeyChord = "Control+Shift+A".parse().unwrap();
        let b: KeyChord = "Ctrl+Shift+A".parse().unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn default_bindings_have_no_conflicts() {
        let bindings = KeyBindings::default();
        let conflicts = bindings.conflicts();
        assert!(
            conflicts.is_empty(),
            "default bindings conflict: {:?}",
            conflicts
        );
    }

    #[test]
    fn lookup_finds_default_chord() {
        let bindings = KeyBindings::default();
        let chord: KeyChord = "Ctrl+Shift+A".parse().unwrap();
        assert_eq!(bindings.lookup(chord), Some(Action::ToggleEditMode));
        let chord: KeyChord = "Esc".parse().unwrap();
        assert_eq!(bindings.lookup(chord), Some(Action::ToggleEditMode));
        let chord: KeyChord = "Space".parse().unwrap();
        assert_eq!(bindings.lookup(chord), Some(Action::PauseAll));
        let chord: KeyChord = "Q".parse().unwrap();
        assert_eq!(bindings.lookup(chord), Some(Action::QuitWithSave));
    }

    #[test]
    fn add_chord_detects_conflict() {
        let mut bindings = KeyBindings::default();
        // Try to bind Q (already QuitWithSave) to ToggleVisible.
        let chord: KeyChord = "Q".parse().unwrap();
        let conflict = bindings.add_chord(Action::ToggleVisible, chord);
        assert_eq!(conflict, Some(Action::QuitWithSave));
    }

    #[test]
    fn reset_action_restores_default() {
        let mut bindings = KeyBindings::default();
        let chord: KeyChord = "F".parse().unwrap();
        bindings.add_chord(Action::ToggleVisible, chord);
        assert!(bindings.chords_for(Action::ToggleVisible).contains(&chord));
        bindings.reset_action(Action::ToggleVisible);
        assert!(!bindings.chords_for(Action::ToggleVisible).contains(&chord));
    }

    #[test]
    fn serde_round_trips_through_toml() {
        let bindings = KeyBindings::default();
        let s = toml::to_string(&bindings).unwrap();
        let back: KeyBindings = toml::from_str(&s).unwrap();
        // Every action's chord set must survive round-trip.
        for &action in Action::ALL {
            assert_eq!(
                bindings.chords_for(action),
                back.chords_for(action),
                "{action:?} differs after TOML round-trip"
            );
        }
    }

    #[test]
    fn empty_keybindings_section_falls_back_to_defaults_at_lookup() {
        // Simulates a config that has no [keybindings] section.
        let bindings = KeyBindings {
            map: BTreeMap::new(),
        };
        let chord: KeyChord = "Q".parse().unwrap();
        assert_eq!(bindings.lookup(chord), Some(Action::QuitWithSave));
    }

    #[test]
    fn from_winit_covers_letters_named_and_symbols() {
        use winit::keyboard::{Key, NamedKey as WK};
        assert_eq!(
            KeyCode::from_winit(Key::Character("a")),
            Some(KeyCode::Letter('A'))
        );
        assert_eq!(
            KeyCode::from_winit(Key::Named(WK::Escape)),
            Some(KeyCode::Named(NamedKey::Escape))
        );
        assert_eq!(
            KeyCode::from_winit(Key::Character("+")),
            Some(KeyCode::Symbol(SymbolKey::Plus))
        );
    }
}
