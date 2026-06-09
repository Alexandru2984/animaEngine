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

mod action;
mod bindings;
mod chord;
mod keys;

pub use action::Action;
pub use bindings::KeyBindings;
pub use chord::{ChordParseError, KeyChord};
pub use keys::{KeyCode, ModifierMask, NamedKey, SymbolKey};

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, HashSet};

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
