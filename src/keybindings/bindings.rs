//! `KeyBindings` — the persisted Action→[KeyChord] table. Extracted in J.4.
//!
//! Stored in `config.toml` under `[keybindings.map]`. Missing entries
//! fall back to `Action::default_chords()` at lookup time so users
//! upgrading from an older binary don't silently lose chords introduced
//! in a newer one.
//!
//! Linear-scan lookup is fine: total bindings stay well under 50, so
//! each scan is sub-microsecond and runs only on key events.
//!
//! Read leniently (1.5): every release adds actions, and an action name
//! this build does not know — bound by a newer animaEngine — or a chord
//! it cannot read cost the whole `config.toml` before, which then loaded
//! as the default scene. An unknown action is now kept as written and
//! written back; a chord that does not read is left out, with a warning.

use super::action::Action;
use super::chord::KeyChord;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::BTreeMap;

/// User-overridable mapping from `Action` to one or more `KeyChord`s.
/// Empty chord vector means the action is disabled.
#[derive(Debug, Clone)]
pub struct KeyBindings {
    /// Per-action chord lists. Missing entries fall back to defaults
    /// at lookup time, so adding a new action in a future release
    /// doesn't silently disable it for existing users — less the chords
    /// the user has given other actions (`effective`).
    pub map: BTreeMap<Action, Vec<KeyChord>>,
    /// Actions this build does not have, as written: a newer animaEngine
    /// bound them, and finds them again after this one has saved.
    pub unknown: BTreeMap<String, toml::Value>,
}

/// `[keybindings]` as stored: action names to chord strings.
#[derive(Deserialize)]
struct Stored {
    #[serde(default)]
    map: BTreeMap<String, toml::Value>,
}

impl<'de> Deserialize<'de> for KeyBindings {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let stored = Stored::deserialize(deserializer)?;
        let mut map = BTreeMap::new();
        let mut unknown = BTreeMap::new();
        for (name, value) in stored.map {
            let Ok(action) = toml::Value::String(name.clone()).try_into::<Action>() else {
                tracing::warn!("keybindings: no action `{name}` in this version; kept as written");
                unknown.insert(name, value);
                continue;
            };
            // A list of chords, or one on its own.
            let written = match value {
                toml::Value::Array(list) => list,
                one @ toml::Value::String(_) => vec![one],
                other => {
                    tracing::warn!(
                        "keybindings: `{name}` is not a list of keys ({other}); its defaults apply"
                    );
                    continue;
                }
            };
            let chords = written
                .into_iter()
                .filter_map(|v| match v.as_str().map(str::parse::<KeyChord>) {
                    Some(Ok(chord)) => Some(chord),
                    _ => {
                        tracing::warn!(
                            "keybindings: `{name}`: {v} is not a key this version reads; left out"
                        );
                        None
                    }
                })
                .collect();
            map.insert(action, chords);
        }
        // Cycle character as it came before 1.5 — Tab, which the panel
        // takes for its focus — moves to the key that works. A Tab set by
        // hand among others stays.
        let tab = KeyChord::new(
            super::keys::ModifierMask::NONE,
            super::keys::KeyCode::Named(super::keys::NamedKey::Tab),
        );
        if map.get(&Action::CycleEntity) == Some(&vec![tab]) {
            tracing::info!("keybindings: Cycle character moves from Tab to its new default");
            map.insert(
                Action::CycleEntity,
                Action::CycleEntity.default_chords().to_vec(),
            );
        }
        Ok(Self { map, unknown })
    }
}

impl Serialize for KeyBindings {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        /// The known actions in their own order, then the unknown ones.
        struct Entries<'a>(&'a KeyBindings);
        impl Serialize for Entries<'_> {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                use serde::ser::SerializeMap;
                let mut out = serializer.serialize_map(None)?;
                for (action, chords) in &self.0.map {
                    out.serialize_entry(action, chords)?;
                }
                for (name, value) in &self.0.unknown {
                    out.serialize_entry(name, value)?;
                }
                out.end()
            }
        }
        #[derive(Serialize)]
        struct Out<'a> {
            map: Entries<'a>,
        }
        Out { map: Entries(self) }.serialize(serializer)
    }
}

impl Default for KeyBindings {
    fn default() -> Self {
        let mut map = BTreeMap::new();
        for &action in Action::ALL {
            map.insert(action, action.default_chords().to_vec());
        }
        Self {
            map,
            unknown: BTreeMap::new(),
        }
    }
}

impl KeyBindings {
    /// The chords `action` answers to: as the user set them, or — for an
    /// action they have never touched, one new since their config was
    /// written — its defaults, less any chord the user gave another
    /// action. A Ctrl+Z bound by hand in 1.4 stays theirs when 1.5's
    /// Undo, whose default it is, comes along.
    fn effective(&self, action: Action) -> Cow<'_, [KeyChord]> {
        match self.map.get(&action) {
            Some(chords) => Cow::Borrowed(chords),
            None => Cow::Owned(
                action
                    .default_chords()
                    .iter()
                    .filter(|chord| !self.map.values().any(|taken| taken.contains(chord)))
                    .copied()
                    .collect(),
            ),
        }
    }

    /// Live chord list for `action`, falling back to defaults if the
    /// map has no entry yet. Returns an owned slice so the caller can
    /// freely iterate and render.
    pub fn chords_for(&self, action: Action) -> Vec<KeyChord> {
        self.effective(action).into_owned()
    }

    /// Find the action bound to `chord`. Linear scan — total bindings
    /// stay under ~50 so this is well below any UI-event budget. When
    /// multiple actions are bound to the same chord (a conflict), the
    /// first match wins; `conflicts` surfaces the issue in the UI.
    pub fn lookup(&self, chord: KeyChord) -> Option<Action> {
        Action::ALL
            .iter()
            .copied()
            .find(|&action| self.effective(action).contains(&chord))
    }

    /// All chord collisions in the current binding table: pairs of
    /// `(chord, actions)` where `actions.len() > 1`. The UI uses this
    /// to render a yellow warning next to conflicting rows.
    pub fn conflicts(&self) -> Vec<(KeyChord, Vec<Action>)> {
        let mut chord_owners: BTreeMap<KeyChord, Vec<Action>> = BTreeMap::new();
        for &action in Action::ALL {
            for &chord in self.effective(action).iter() {
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
    /// button in the rebind UI. Actions this build does not have are not
    /// its to reset: a newer version's bindings stay.
    pub fn reset_all(&mut self) {
        let unknown = std::mem::take(&mut self.unknown);
        *self = Self::default();
        self.unknown = unknown;
    }

    /// Add a chord to `action`. No-op if the chord is already bound to
    /// the same action (idempotent). Returns the conflicting action,
    /// if any — the UI decides whether to surface as warning or block.
    pub fn add_chord(&mut self, action: Action, chord: KeyChord) -> Option<Action> {
        // An untouched action whose default this chord is keeps it,
        // written down, so the two show as the conflict they are — rather
        // than the other one losing it unseen, as a new default does to a
        // chord the user set before it existed (`effective`).
        for &other in Action::ALL {
            if other != action
                && !self.map.contains_key(&other)
                && self.effective(other).contains(&chord)
            {
                let theirs = self.effective(other).into_owned();
                self.map.insert(other, theirs);
            }
        }
        let mut chords = self.effective(action).into_owned();
        if !chords.contains(&chord) {
            chords.push(chord);
        }
        self.map.insert(action, chords);
        // Detect conflict against the rest of the table.
        Action::ALL
            .iter()
            .copied()
            .find(|&other| other != action && self.effective(other).contains(&chord))
    }

    /// Remove a chord from `action`. No-op if the chord wasn't bound.
    ///
    /// An action with no entry of its own shows its defaults, and its row
    /// a remove button for each: they are written down first, without
    /// this one — before, the button did nothing for such an action,
    /// which after an upgrade was every new one.
    pub fn remove_chord(&mut self, action: Action, chord: KeyChord) {
        let mut chords = self.effective(action).into_owned();
        chords.retain(|c| *c != chord);
        self.map.insert(action, chords);
    }
}
