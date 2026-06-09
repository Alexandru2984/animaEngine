//! `KeyChord` — a (modifier mask, key) pair — plus its serialisation
//! and parsing. Extracted in J.2.
//!
//! The chord format is intentionally TOML-friendly so users can
//! hand-edit `config.toml`. Format: `Ctrl+Shift+A`, with modifiers
//! in canonical order (Ctrl, Shift, Alt, Super) and the key segment
//! parsed by `KeyCode::FromStr`.
//!
//! Modifier aliases accepted on parse: `Control`, `Option`, `Cmd`,
//! `Meta`, `Win` — but `canonical_str` always emits the canonical
//! form so a load → save round-trip stabilises the file.

use super::keys::{KeyCode, ModifierMask};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

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
