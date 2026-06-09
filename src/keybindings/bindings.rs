//! `KeyBindings` — the persisted Action→[KeyChord] table. Extracted in J.4.
//!
//! Stored in `config.toml` under `[keybindings.map]`. Missing entries
//! fall back to `Action::default_chords()` at lookup time so users
//! upgrading from an older binary don't silently lose chords introduced
//! in a newer one.
//!
//! Linear-scan lookup is fine: total bindings stay well under 50, so
//! each scan is sub-microsecond and runs only on key events.

use super::action::Action;
use super::chord::KeyChord;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
