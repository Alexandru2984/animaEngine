//! Modifier mask + key-code enum hierarchy. Extracted in J.1.
//!
//! `KeyCode` is the canonical key identity used by every shortcut.
//! It deliberately exposes only the subset of winit's input space we
//! actually bind so:
//!
//! - the on-disk serialization (`canonical_str` / `FromStr`) is stable
//!   across winit major bumps,
//! - the UI rebinder can recognise every chord the user types
//!   without an `Other(String)` escape hatch hiding mistakes.
//!
//! Conversion to/from egui's `Key` and winit's `Key` lives here too —
//! same enum surface, same exhaustive matches.
//!
//! `ChordParseError` is owned by the chord submodule but referenced
//! from `FromStr for KeyCode` via `super::`; the dependency is
//! one-way (parsing errors → chord), never the reverse.
//!
//! `Letter('A'..='Z')` is upper-case-normalized on construction.
//! `Digit(0..=9)` is range-checked at construction time.

use super::ChordParseError;
use std::str::FromStr;

/// Bit-mask of modifier keys held when a chord fires. Stored as `u8`
/// so `KeyChord` stays `Copy` and cheap to compare.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModifierMask(pub(crate) u8);

impl ModifierMask {
    pub const NONE: Self = Self(0);
    pub const CTRL: Self = Self(0b0001);
    pub const SHIFT: Self = Self(0b0010);
    pub const ALT: Self = Self(0b0100);
    pub const SUPER: Self = Self(0b1000);

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn from_state(ctrl: bool, shift: bool, alt: bool, sup: bool) -> Self {
        let mut bits = 0u8;
        if ctrl {
            bits |= Self::CTRL.0;
        }
        if shift {
            bits |= Self::SHIFT.0;
        }
        if alt {
            bits |= Self::ALT.0;
        }
        if sup {
            bits |= Self::SUPER.0;
        }
        Self(bits)
    }

    pub const fn ctrl(self) -> bool {
        self.contains(Self::CTRL)
    }
    pub const fn shift(self) -> bool {
        self.contains(Self::SHIFT)
    }
    pub const fn alt(self) -> bool {
        self.contains(Self::ALT)
    }
    pub const fn sup(self) -> bool {
        self.contains(Self::SUPER)
    }
}

impl std::ops::BitOr for ModifierMask {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for ModifierMask {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Canonical key identity. Subset of winit's input space large enough
/// to cover every shortcut animaEngine binds, with a stable
/// round-tripable serialization that survives winit major bumps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum KeyCode {
    /// 'A'..='Z' — normalized to upper case on construction.
    Letter(char),
    /// 0..=9
    Digit(u8),
    /// Named non-printable key (Escape, arrows, etc.)
    Named(NamedKey),
    /// Printable punctuation we explicitly bind.
    Symbol(SymbolKey),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NamedKey {
    Escape,
    Space,
    Tab,
    Enter,
    Backspace,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SymbolKey {
    Plus,         // '+'
    Minus,        // '-'
    Equal,        // '='
    BracketLeft,  // '['
    BracketRight, // ']'
    Backquote,    // '`'
}

impl KeyCode {
    /// Canonical TOML-friendly string. Round-trips through `FromStr`.
    pub fn canonical_str(self) -> String {
        match self {
            Self::Letter(c) => c.to_string(),
            Self::Digit(d) => d.to_string(),
            Self::Named(n) => n.canonical_str().to_string(),
            Self::Symbol(s) => s.canonical_str().to_string(),
        }
    }

    /// Pretty UI form — arrows render as glyphs, named keys abbreviate.
    pub fn display_str(self) -> String {
        match self {
            Self::Named(NamedKey::ArrowUp) => "↑".to_string(),
            Self::Named(NamedKey::ArrowDown) => "↓".to_string(),
            Self::Named(NamedKey::ArrowLeft) => "←".to_string(),
            Self::Named(NamedKey::ArrowRight) => "→".to_string(),
            Self::Named(NamedKey::Escape) => "Esc".to_string(),
            Self::Named(NamedKey::PageUp) => "PgUp".to_string(),
            Self::Named(NamedKey::PageDown) => "PgDn".to_string(),
            Self::Named(NamedKey::Backspace) => "Bksp".to_string(),
            Self::Named(NamedKey::Delete) => "Del".to_string(),
            other => other.canonical_str(),
        }
    }

    /// Build from an `egui::Key` — used by the rebinding UI when the
    /// user presses a chord to record. Mirrors `from_winit` but with
    /// egui's pre-mapped enum so the panel doesn't have to reach into
    /// winit's input space.
    pub fn from_egui(key: egui::Key) -> Option<Self> {
        use egui::Key as E;
        Some(match key {
            E::A => Self::Letter('A'),
            E::B => Self::Letter('B'),
            E::C => Self::Letter('C'),
            E::D => Self::Letter('D'),
            E::E => Self::Letter('E'),
            E::F => Self::Letter('F'),
            E::G => Self::Letter('G'),
            E::H => Self::Letter('H'),
            E::I => Self::Letter('I'),
            E::J => Self::Letter('J'),
            E::K => Self::Letter('K'),
            E::L => Self::Letter('L'),
            E::M => Self::Letter('M'),
            E::N => Self::Letter('N'),
            E::O => Self::Letter('O'),
            E::P => Self::Letter('P'),
            E::Q => Self::Letter('Q'),
            E::R => Self::Letter('R'),
            E::S => Self::Letter('S'),
            E::T => Self::Letter('T'),
            E::U => Self::Letter('U'),
            E::V => Self::Letter('V'),
            E::W => Self::Letter('W'),
            E::X => Self::Letter('X'),
            E::Y => Self::Letter('Y'),
            E::Z => Self::Letter('Z'),
            E::Num0 => Self::Digit(0),
            E::Num1 => Self::Digit(1),
            E::Num2 => Self::Digit(2),
            E::Num3 => Self::Digit(3),
            E::Num4 => Self::Digit(4),
            E::Num5 => Self::Digit(5),
            E::Num6 => Self::Digit(6),
            E::Num7 => Self::Digit(7),
            E::Num8 => Self::Digit(8),
            E::Num9 => Self::Digit(9),
            E::Escape => Self::Named(NamedKey::Escape),
            E::Space => Self::Named(NamedKey::Space),
            E::Tab => Self::Named(NamedKey::Tab),
            E::Enter => Self::Named(NamedKey::Enter),
            E::Backspace => Self::Named(NamedKey::Backspace),
            E::Delete => Self::Named(NamedKey::Delete),
            E::Home => Self::Named(NamedKey::Home),
            E::End => Self::Named(NamedKey::End),
            E::PageUp => Self::Named(NamedKey::PageUp),
            E::PageDown => Self::Named(NamedKey::PageDown),
            E::ArrowUp => Self::Named(NamedKey::ArrowUp),
            E::ArrowDown => Self::Named(NamedKey::ArrowDown),
            E::ArrowLeft => Self::Named(NamedKey::ArrowLeft),
            E::ArrowRight => Self::Named(NamedKey::ArrowRight),
            E::Plus => Self::Symbol(SymbolKey::Plus),
            E::Minus => Self::Symbol(SymbolKey::Minus),
            E::Equals => Self::Symbol(SymbolKey::Equal),
            E::OpenBracket => Self::Symbol(SymbolKey::BracketLeft),
            E::CloseBracket => Self::Symbol(SymbolKey::BracketRight),
            E::Backtick => Self::Symbol(SymbolKey::Backquote),
            // No aliases for egui's shifted-symbol keys (`Exclamationmark`,
            // `OpenCurlyBracket`, …). R36 added them to make Shift+[ record
            // on winit, but they assume a US layout — on AZERTY `!` is not
            // on the 1 key. Every egui key event also carries the physical
            // key, and `KeyChord::from_egui_event` resolves digits and
            // symbols from that, so the aliases were never needed.
            _ => return None,
        })
    }

    /// The key at this **physical position**, named as it is on a US
    /// layout. Letters, the digit row and the punctuation we bind; `None`
    /// for anything else.
    ///
    /// Letters are here for layouts with no Latin letters at all — on a
    /// Cyrillic layout the logical key for C is `с`, which maps to nothing,
    /// and falling back to the position is what lets Ctrl+C work there.
    pub fn from_physical_winit(code: winit::keyboard::KeyCode) -> Option<Self> {
        use winit::keyboard::KeyCode as P;
        let letter = |c: char| Some(Self::Letter(c));
        match code {
            P::KeyA => letter('A'),
            P::KeyB => letter('B'),
            P::KeyC => letter('C'),
            P::KeyD => letter('D'),
            P::KeyE => letter('E'),
            P::KeyF => letter('F'),
            P::KeyG => letter('G'),
            P::KeyH => letter('H'),
            P::KeyI => letter('I'),
            P::KeyJ => letter('J'),
            P::KeyK => letter('K'),
            P::KeyL => letter('L'),
            P::KeyM => letter('M'),
            P::KeyN => letter('N'),
            P::KeyO => letter('O'),
            P::KeyP => letter('P'),
            P::KeyQ => letter('Q'),
            P::KeyR => letter('R'),
            P::KeyS => letter('S'),
            P::KeyT => letter('T'),
            P::KeyU => letter('U'),
            P::KeyV => letter('V'),
            P::KeyW => letter('W'),
            P::KeyX => letter('X'),
            P::KeyY => letter('Y'),
            P::KeyZ => letter('Z'),
            P::Digit0 => Some(Self::Digit(0)),
            P::Digit1 => Some(Self::Digit(1)),
            P::Digit2 => Some(Self::Digit(2)),
            P::Digit3 => Some(Self::Digit(3)),
            P::Digit4 => Some(Self::Digit(4)),
            P::Digit5 => Some(Self::Digit(5)),
            P::Digit6 => Some(Self::Digit(6)),
            P::Digit7 => Some(Self::Digit(7)),
            P::Digit8 => Some(Self::Digit(8)),
            P::Digit9 => Some(Self::Digit(9)),
            P::Minus => Some(Self::Symbol(SymbolKey::Minus)),
            P::Equal => Some(Self::Symbol(SymbolKey::Equal)),
            P::BracketLeft => Some(Self::Symbol(SymbolKey::BracketLeft)),
            P::BracketRight => Some(Self::Symbol(SymbolKey::BracketRight)),
            P::Backquote => Some(Self::Symbol(SymbolKey::Backquote)),
            _ => None,
        }
    }

    /// The one rule that turns a key press into the key a chord names.
    ///
    /// **A key means what is printed on it when we know that name, and
    /// its position when we don't — except the number row, which is always
    /// its position.** On AZERTY, Ctrl+Z is the key with Z printed on it,
    /// while Ctrl+1 is the top-row key left of 2 even though it types `&`:
    /// AZERTY digits are *shifted*, so reading the label would leave the
    /// number row unbindable. That is what browsers and most desktop
    /// applications do.
    ///
    /// Symbols try the label first because the position can be actively
    /// wrong. German `+` sits where US has `]`; going by position, the key
    /// the Keybindings tab calls "+" (opacity up) would fire `]` (FPS up).
    /// A symbol we have no name for — AZERTY `²`, `)`, `$`, a dead key, any
    /// US shifted symbol — falls back to the position, as does a non-Latin
    /// letter; the same fallback egui-winit uses for non-Latin layouts.
    ///
    /// It replaces two rounds of guessing (R29, R35) that listed the *US*
    /// shifted symbols as aliases: correct on US, and on AZERTY it turned
    /// the unshifted `&` into a 7.
    ///
    /// Stored chords keep their meaning: they were always written with US
    /// names, and on a US layout every case above agrees with what the
    /// aliases used to produce.
    pub fn resolve(logical: Option<Self>, physical: Option<Self>) -> Option<Self> {
        match (logical, physical) {
            (Some(key @ (Self::Letter(_) | Self::Named(_))), _) => Some(key),
            (_, Some(digit @ Self::Digit(_))) => Some(digit),
            (Some(symbol @ Self::Symbol(_)), _) => Some(symbol),
            (logical, physical) => physical.or(logical),
        }
    }

    /// Build from winit's logical `Key`. Returns `None` for inputs not
    /// in our dispatch table (function keys, IME composition events,
    /// etc.) — callers ignore those.
    pub fn from_winit(key: winit::keyboard::Key<&str>) -> Option<Self> {
        use winit::keyboard::{Key, NamedKey as WK};
        Some(match key {
            Key::Character(s) => {
                let c = s.chars().next()?;
                match c {
                    'a'..='z' => Self::Letter(c.to_ascii_uppercase()),
                    'A'..='Z' => Self::Letter(c),
                    '0'..='9' => Self::Digit(c.to_digit(10)? as u8),
                    // Only the unshifted US symbols. Shifted forms ('!',
                    // '~', '{') used to be listed here too (R29, R35), which
                    // is a US-layout guess: on AZERTY '&' is the *unshifted*
                    // 1 key, and mapping it to 7 bound the wrong key. Digits
                    // and symbols are now decided by physical position in
                    // `resolve`, so this logical form only matters when no
                    // position is known.
                    '+' => Self::Symbol(SymbolKey::Plus),
                    '-' => Self::Symbol(SymbolKey::Minus),
                    '=' => Self::Symbol(SymbolKey::Equal),
                    '[' => Self::Symbol(SymbolKey::BracketLeft),
                    ']' => Self::Symbol(SymbolKey::BracketRight),
                    '`' => Self::Symbol(SymbolKey::Backquote),
                    _ => return None,
                }
            }
            Key::Named(WK::Escape) => Self::Named(NamedKey::Escape),
            Key::Named(WK::Space) => Self::Named(NamedKey::Space),
            Key::Named(WK::Tab) => Self::Named(NamedKey::Tab),
            Key::Named(WK::Enter) => Self::Named(NamedKey::Enter),
            Key::Named(WK::Backspace) => Self::Named(NamedKey::Backspace),
            Key::Named(WK::Delete) => Self::Named(NamedKey::Delete),
            Key::Named(WK::Home) => Self::Named(NamedKey::Home),
            Key::Named(WK::End) => Self::Named(NamedKey::End),
            Key::Named(WK::PageUp) => Self::Named(NamedKey::PageUp),
            Key::Named(WK::PageDown) => Self::Named(NamedKey::PageDown),
            Key::Named(WK::ArrowUp) => Self::Named(NamedKey::ArrowUp),
            Key::Named(WK::ArrowDown) => Self::Named(NamedKey::ArrowDown),
            Key::Named(WK::ArrowLeft) => Self::Named(NamedKey::ArrowLeft),
            Key::Named(WK::ArrowRight) => Self::Named(NamedKey::ArrowRight),
            _ => return None,
        })
    }
}

impl NamedKey {
    pub(super) fn canonical_str(self) -> &'static str {
        match self {
            Self::Escape => "Escape",
            Self::Space => "Space",
            Self::Tab => "Tab",
            Self::Enter => "Enter",
            Self::Backspace => "Backspace",
            Self::Delete => "Delete",
            Self::Home => "Home",
            Self::End => "End",
            Self::PageUp => "PageUp",
            Self::PageDown => "PageDown",
            Self::ArrowUp => "ArrowUp",
            Self::ArrowDown => "ArrowDown",
            Self::ArrowLeft => "ArrowLeft",
            Self::ArrowRight => "ArrowRight",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "Escape" | "Esc" => Self::Escape,
            "Space" => Self::Space,
            "Tab" => Self::Tab,
            "Enter" | "Return" => Self::Enter,
            "Backspace" | "Bksp" => Self::Backspace,
            "Delete" | "Del" => Self::Delete,
            "Home" => Self::Home,
            "End" => Self::End,
            "PageUp" | "PgUp" => Self::PageUp,
            "PageDown" | "PgDn" => Self::PageDown,
            "ArrowUp" | "Up" => Self::ArrowUp,
            "ArrowDown" | "Down" => Self::ArrowDown,
            "ArrowLeft" | "Left" => Self::ArrowLeft,
            "ArrowRight" | "Right" => Self::ArrowRight,
            _ => return None,
        })
    }
}

impl SymbolKey {
    pub(super) fn canonical_str(self) -> &'static str {
        match self {
            Self::Plus => "+",
            Self::Minus => "-",
            Self::Equal => "=",
            Self::BracketLeft => "[",
            Self::BracketRight => "]",
            Self::Backquote => "`",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "+" | "Plus" => Self::Plus,
            "-" | "Minus" => Self::Minus,
            "=" | "Equal" => Self::Equal,
            "[" | "BracketLeft" => Self::BracketLeft,
            "]" | "BracketRight" => Self::BracketRight,
            "`" | "Backquote" => Self::Backquote,
            _ => return None,
        })
    }
}

impl FromStr for KeyCode {
    type Err = ChordParseError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(ChordParseError::EmptyKey);
        }
        // Single-character fast path: covers letters, digits, and the
        // punctuation symbols we bind. Avoids ambiguity with multi-char
        // named-key aliases like `Esc`.
        if s.chars().count() == 1 {
            let c = s
                .chars()
                .next()
                .expect("count==1 guard checked above guarantees one char");
            return Ok(match c {
                'a'..='z' => Self::Letter(c.to_ascii_uppercase()),
                'A'..='Z' => Self::Letter(c),
                '0'..='9' => Self::Digit(
                    c.to_digit(10)
                        .expect("'0'..='9' match arm guarantees to_digit(10) returns Some")
                        as u8,
                ),
                '+' => Self::Symbol(SymbolKey::Plus),
                '-' => Self::Symbol(SymbolKey::Minus),
                '=' => Self::Symbol(SymbolKey::Equal),
                '[' => Self::Symbol(SymbolKey::BracketLeft),
                ']' => Self::Symbol(SymbolKey::BracketRight),
                '`' => Self::Symbol(SymbolKey::Backquote),
                _ => return Err(ChordParseError::UnknownKey(s.to_string())),
            });
        }
        if let Some(n) = NamedKey::parse(s) {
            return Ok(Self::Named(n));
        }
        if let Some(sym) = SymbolKey::parse(s) {
            return Ok(Self::Symbol(sym));
        }
        Err(ChordParseError::UnknownKey(s.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every key the engine can represent, so the round-trip test below
    /// is exhaustive rather than a spot check.
    fn all_keycodes() -> Vec<KeyCode> {
        let named = [
            NamedKey::Escape,
            NamedKey::Space,
            NamedKey::Tab,
            NamedKey::Enter,
            NamedKey::Backspace,
            NamedKey::Delete,
            NamedKey::Home,
            NamedKey::End,
            NamedKey::PageUp,
            NamedKey::PageDown,
            NamedKey::ArrowUp,
            NamedKey::ArrowDown,
            NamedKey::ArrowLeft,
            NamedKey::ArrowRight,
        ];
        let symbols = [
            SymbolKey::Plus,
            SymbolKey::Minus,
            SymbolKey::Equal,
            SymbolKey::BracketLeft,
            SymbolKey::BracketRight,
            SymbolKey::Backquote,
        ];
        let mut all: Vec<KeyCode> = ('A'..='Z').map(KeyCode::Letter).collect();
        all.extend((0..=9u8).map(KeyCode::Digit));
        all.extend(named.into_iter().map(KeyCode::Named));
        all.extend(symbols.into_iter().map(KeyCode::Symbol));
        all
    }

    /// The canonical string is what gets written to `config.toml`, so a
    /// key that doesn't survive `canonical_str` → parse would silently
    /// drop or rebind the user's shortcut on the next load. Exhaustive
    /// on purpose: a mapping typo in one arm is exactly the kind of bug
    /// a spot check misses.
    #[test]
    fn every_keycode_round_trips_through_its_canonical_string() {
        let all = all_keycodes();
        assert_eq!(all.len(), 26 + 10 + 14 + 6, "table drifted from the enums");
        for key in all {
            let s = key.canonical_str();
            let parsed: KeyCode = s
                .parse()
                .unwrap_or_else(|_| panic!("{key:?} serialises to {s:?}, which does not parse"));
            assert_eq!(parsed, key, "round-trip changed {key:?} (via {s:?})");
        }
    }

    /// Two different keys must never share a canonical form — a
    /// collision would make one of them unbindable.
    #[test]
    fn canonical_strings_are_unique() {
        let all = all_keycodes();
        let mut seen = std::collections::BTreeMap::new();
        for key in all {
            let s = key.canonical_str();
            if let Some(prev) = seen.insert(s.clone(), key) {
                panic!("{prev:?} and {key:?} both serialise to {s:?}");
            }
        }
    }

    /// `display_str` is UI-only, but an empty label would render a blank
    /// shortcut in the keybindings tab.
    #[test]
    fn display_strings_are_never_empty() {
        for key in all_keycodes() {
            assert!(!key.display_str().is_empty(), "{key:?} has no display form");
        }
    }

    #[test]
    fn letters_normalise_to_upper_case() {
        let lower: KeyCode = "a".parse().expect("lower-case letter parses");
        let upper: KeyCode = "A".parse().expect("upper-case letter parses");
        assert_eq!(lower, upper, "case must not create two distinct bindings");
        assert_eq!(lower, KeyCode::Letter('A'));
    }

    #[test]
    fn unknown_keys_are_rejected() {
        for bad in ["", "NotAKey", "F13", "ArrowSideways", "??"] {
            assert!(
                bad.parse::<KeyCode>().is_err(),
                "{bad:?} should not parse as a key"
            );
        }
    }
}
