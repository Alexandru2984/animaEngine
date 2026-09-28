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
pub mod shared;

pub use action::Action;
pub use bindings::KeyBindings;
pub use chord::{ChordParseError, KeyChord, ModifierNames};
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
        assert_eq!(Action::ALL.len(), 32);
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

    // ── layout independence: `KeyCode::resolve` ─────────────────────
    //
    // The number row follows the position; letters and known symbols follow
    // the label; anything unnamed falls back to the position.
    // These replace tests that pinned US-only aliases (R29, R35, R36);
    // the aliases bound the wrong key on AZERTY, and pinning them is what
    // made that look like correct behaviour.

    fn winit_press(logical: &str, physical: winit::keyboard::KeyCode) -> Option<KeyCode> {
        KeyCode::resolve(
            KeyCode::from_winit(winit::keyboard::Key::Character(logical)),
            KeyCode::from_physical_winit(physical),
        )
    }

    /// On a US layout nothing changes: position and symbol agree, and the
    /// shifted symbol no longer needs an alias because the position
    /// decides.
    #[test]
    fn on_us_the_shifted_symbol_is_the_same_key() {
        use winit::keyboard::KeyCode as P;
        assert_eq!(
            winit_press("~", P::Backquote),
            winit_press("`", P::Backquote)
        );
        assert_eq!(winit_press("!", P::Digit1), Some(KeyCode::Digit(1)));
        assert_eq!(
            winit_press("{", P::BracketLeft),
            Some(KeyCode::Symbol(SymbolKey::BracketLeft))
        );
    }

    /// The defect this replaces: AZERTY types `&` on the 1 key unshifted,
    /// and the US alias turned that into a 7 — the wrong key, silently.
    #[test]
    fn on_azerty_the_number_row_is_its_position() {
        use winit::keyboard::KeyCode as P;
        assert_eq!(winit_press("&", P::Digit1), Some(KeyCode::Digit(1)));
        assert_eq!(winit_press("é", P::Digit2), Some(KeyCode::Digit(2)));
        assert_eq!(winit_press("\"", P::Digit3), Some(KeyCode::Digit(3)));
    }

    /// The number row stays positional even where the label is a symbol
    /// we know: AZERTY's 6 key types `-`, and reading that label would
    /// make Ctrl+6 unreachable and fire opacity-down instead.
    #[test]
    fn azerty_six_is_six_not_minus() {
        use winit::keyboard::KeyCode as P;
        assert_eq!(winit_press("-", P::Digit6), Some(KeyCode::Digit(6)));
        assert_eq!(
            winit_press(")", P::Minus),
            Some(KeyCode::Symbol(SymbolKey::Minus))
        );
    }

    /// A symbol we have a name for keeps it. German `+` sits where US has
    /// `]`; by position it would fire FPS-up while the Keybindings tab
    /// told the user `+` is opacity-up. A wrong action is worse than an
    /// unreachable one.
    #[test]
    fn a_known_symbol_follows_its_label() {
        use winit::keyboard::KeyCode as P;
        let bindings = KeyBindings::default();
        let plus = KeyChord::new(
            ModifierMask::NONE,
            winit_press("+", P::BracketRight).unwrap(),
        );
        assert_eq!(bindings.lookup(plus), Some(Action::OpacityUp));
    }

    /// Letters follow the label: on AZERTY Ctrl+A is the key with A on
    /// it, which sits where US has Q.
    #[test]
    fn letters_follow_the_label_not_the_position() {
        use winit::keyboard::KeyCode as P;
        assert_eq!(winit_press("a", P::KeyQ), Some(KeyCode::Letter('A')));
        assert_eq!(winit_press("z", P::KeyW), Some(KeyCode::Letter('Z')));
    }

    /// A layout with no Latin letters falls back to the position, so
    /// Ctrl+C still exists on Cyrillic.
    #[test]
    fn non_latin_letters_fall_back_to_the_position() {
        use winit::keyboard::KeyCode as P;
        assert_eq!(winit_press("с", P::KeyC), Some(KeyCode::Letter('C')));
    }

    /// Recording (through egui) and dispatch (through winit or the
    /// keysym table) must land on the same chord for the same press, or
    /// a binding records and then never fires. egui reports AZERTY's
    /// unshifted `&` as the physical `Num1`, because it has no key for `&`.
    #[test]
    fn recording_and_dispatch_agree_on_every_layout() {
        use winit::keyboard::KeyCode as P;
        let ctrl = egui::Modifiers {
            ctrl: true,
            command: true,
            ..egui::Modifiers::NONE
        };
        let cases = [
            // (egui key, egui physical, winit logical, winit physical)
            (egui::Key::Num1, Some(egui::Key::Num1), "&", P::Digit1),
            (
                egui::Key::Exclamationmark,
                Some(egui::Key::Num1),
                "!",
                P::Digit1,
            ),
            (
                egui::Key::OpenCurlyBracket,
                Some(egui::Key::OpenBracket),
                "{",
                P::BracketLeft,
            ),
            (egui::Key::A, Some(egui::Key::Q), "a", P::KeyQ),
            (egui::Key::Minus, Some(egui::Key::Num6), "-", P::Digit6),
            (
                egui::Key::Plus,
                Some(egui::Key::CloseBracket),
                "+",
                P::BracketRight,
            ),
        ];
        for (key, physical, logical, pos) in cases {
            let recorded = KeyChord::from_egui_event(key, physical, ctrl).unwrap();
            let dispatched = KeyChord::new(ModifierMask::CTRL, winit_press(logical, pos).unwrap());
            assert_eq!(recorded, dispatched, "{logical:?} at {pos:?}");
        }
    }

    /// End to end: the default perf-overlay chord, Ctrl+Shift+`, has to be
    /// reachable from the key that types it — on US (`~` with Shift) and
    /// now also on layouts where that position types something else.
    #[test]
    fn the_default_perf_overlay_chord_is_reachable() {
        use winit::keyboard::KeyCode as P;
        let bindings = KeyBindings::default();
        let cs = ModifierMask::CTRL | ModifierMask::SHIFT;
        for logical in ["~", "°", "|"] {
            let chord = KeyChord::new(cs, winit_press(logical, P::Backquote).unwrap());
            assert_eq!(
                bindings.lookup(chord),
                Some(Action::TogglePerfOverlay),
                "{logical}"
            );
        }
    }
    // ── localized modifier names (R39) ───────────────────────────────

    fn german() -> ModifierNames {
        ModifierNames {
            ctrl: "Strg".into(),
            ..ModifierNames::CANONICAL
        }
    }

    /// The display follows the language; the config file never does.
    #[test]
    fn german_display_says_strg_but_the_config_says_ctrl() {
        let chord: KeyChord = "Ctrl+Shift+A".parse().unwrap();
        assert_eq!(chord.display_str(&german()), "Strg+Shift+A");
        assert_eq!(chord.canonical_str(), "Ctrl+Shift+A");
        assert_eq!(chord.display_str(&ModifierNames::CANONICAL), "Ctrl+Shift+A");
    }

    /// Someone who reads `Strg+K` in the UI may type it into config.toml.
    /// It is accepted, and written back in the one canonical form.
    #[test]
    fn strg_parses_as_ctrl_and_is_written_back_as_ctrl() {
        let chord: KeyChord = "Strg+K".parse().unwrap();
        assert_eq!(chord, "Ctrl+K".parse().unwrap());
        assert_eq!(chord.canonical_str(), "Ctrl+K");
    }

    // ── egui → chord (R26) ───────────────────────────────────────────

    /// Off macOS, egui sets `command` to the same value as `ctrl`. It must
    /// not also count as Super, or every Ctrl chord becomes Ctrl+Super and
    /// matches nothing — which killed Ctrl+M and Ctrl+Shift+A on the native
    /// Wayland backend and made the rebinder record unpressable chords.
    #[test]
    fn linux_command_is_ctrl_not_super() {
        let mods = egui::Modifiers {
            ctrl: true,
            command: true,
            ..egui::Modifiers::NONE
        };
        assert_eq!(
            KeyChord::from_egui(egui::Key::M, mods),
            Some(KeyChord::new(ModifierMask::CTRL, KeyCode::Letter('M')))
        );
    }

    /// The default Ctrl chords have to survive the round trip that the
    /// Wayland loop and the rebinder both make; this is the lookup that
    /// was silently failing.
    #[test]
    fn a_default_ctrl_chord_still_resolves_from_egui() {
        let mods = egui::Modifiers {
            ctrl: true,
            command: true,
            ..egui::Modifiers::NONE
        };
        let chord = KeyChord::from_egui(egui::Key::M, mods).unwrap();
        let bindings = KeyBindings::default();
        assert_eq!(bindings.lookup(chord), Some(Action::CycleMonitor));

        let mods = egui::Modifiers {
            ctrl: true,
            command: true,
            shift: true,
            ..egui::Modifiers::NONE
        };
        let chord = KeyChord::from_egui(egui::Key::A, mods).unwrap();
        assert_eq!(bindings.lookup(chord), Some(Action::ToggleEditMode));
    }

    /// macOS still needs a way to express Super, and `mac_cmd` is it.
    #[test]
    fn mac_cmd_is_still_super() {
        let mods = egui::Modifiers {
            mac_cmd: true,
            command: true,
            ..egui::Modifiers::NONE
        };
        assert_eq!(
            KeyChord::from_egui(egui::Key::M, mods),
            Some(KeyChord::new(ModifierMask::SUPER, KeyCode::Letter('M')))
        );
    }
}
