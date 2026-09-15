//! Keysym → egui translation (E.1).
//!
//! The XKB keysym vocabulary is huge; the overlay only cares about
//! the keys it actually dispatches on (letters, digits, the arrow
//! cluster, the punctuation we bind, named keys like Escape / Tab /
//! Space / Enter / Backspace / Delete). Anything outside that set
//! returns `None` and the caller treats the press as "no UI event."
//!
//! Text input on the Wayland path comes from the `utf8` field of
//! sctk's `KeyEvent` (an already-composed UTF-8 string). The
//! caller pushes that as `egui::Event::Text` alongside the
//! `egui::Event::Key` produced here, so character composition with
//! dead keys / IME still works through xkbcommon's own engine.
//!
//! Keysym constants come from the `xkeysym` crate, which sctk
//! re-exports from `smithay_client_toolkit::seat::keyboard::Keysym`.

use smithay_client_toolkit::seat::keyboard::{Keysym, Modifiers as SctkModifiers};

/// Convert a key chord captured on the native Wayland path into an
/// `egui::Key`. Returns `None` for anything not in animaEngine's
/// dispatch table — the press is silently dropped on this path.
pub fn keysym_to_egui_key(keysym: Keysym) -> Option<egui::Key> {
    use egui::Key as E;
    Some(match keysym {
        // ── Letters (lower-case keysym; xkb lowercases by layout)
        Keysym::a | Keysym::A => E::A,
        Keysym::b | Keysym::B => E::B,
        Keysym::c | Keysym::C => E::C,
        Keysym::d | Keysym::D => E::D,
        Keysym::e | Keysym::E => E::E,
        Keysym::f | Keysym::F => E::F,
        Keysym::g | Keysym::G => E::G,
        Keysym::h | Keysym::H => E::H,
        Keysym::i | Keysym::I => E::I,
        Keysym::j | Keysym::J => E::J,
        Keysym::k | Keysym::K => E::K,
        Keysym::l | Keysym::L => E::L,
        Keysym::m | Keysym::M => E::M,
        Keysym::n | Keysym::N => E::N,
        Keysym::o | Keysym::O => E::O,
        Keysym::p | Keysym::P => E::P,
        Keysym::q | Keysym::Q => E::Q,
        Keysym::r | Keysym::R => E::R,
        Keysym::s | Keysym::S => E::S,
        Keysym::t | Keysym::T => E::T,
        Keysym::u | Keysym::U => E::U,
        Keysym::v | Keysym::V => E::V,
        Keysym::w | Keysym::W => E::W,
        Keysym::x | Keysym::X => E::X,
        Keysym::y | Keysym::Y => E::Y,
        Keysym::z | Keysym::Z => E::Z,
        // ── Digits
        // Each digit lists its shifted keysym too. `event.keysym` is
        // resolved against the modifier state, so Shift+1 arrives as
        // `exclam`, and with only the plain form listed the press was
        // never turned into an event at all (R35) — the same hole the
        // punctuation below had.
        Keysym::_0 | Keysym::parenright => E::Num0,
        Keysym::_1 | Keysym::exclam => E::Num1,
        Keysym::_2 | Keysym::at => E::Num2,
        Keysym::_3 | Keysym::numbersign => E::Num3,
        Keysym::_4 | Keysym::dollar => E::Num4,
        Keysym::_5 | Keysym::percent => E::Num5,
        Keysym::_6 | Keysym::asciicircum => E::Num6,
        Keysym::_7 | Keysym::ampersand => E::Num7,
        Keysym::_8 | Keysym::asterisk => E::Num8,
        Keysym::_9 | Keysym::parenleft => E::Num9,
        // ── Named control keys
        Keysym::Escape => E::Escape,
        Keysym::Tab => E::Tab,
        Keysym::Return | Keysym::KP_Enter => E::Enter,
        Keysym::BackSpace => E::Backspace,
        Keysym::Delete => E::Delete,
        Keysym::space => E::Space,
        Keysym::Home => E::Home,
        Keysym::End => E::End,
        Keysym::Page_Up => E::PageUp,
        Keysym::Page_Down => E::PageDown,
        Keysym::Up => E::ArrowUp,
        Keysym::Down => E::ArrowDown,
        Keysym::Left => E::ArrowLeft,
        Keysym::Right => E::ArrowRight,
        // ── Punctuation bound by animaEngine actions
        //
        // Both the plain and the shifted keysym map to the same key, the
        // way the letters above list `a | A`. `event.keysym` is *resolved*
        // against the modifier state, so holding Shift turns ` into ~ and
        // [ into { — and with only the plain form listed, the key stopped
        // being an egui event at all. That is why the default perf-overlay
        // chord, Ctrl+Shift+`, did nothing: not one event ever reached the
        // shortcut table (R29).
        Keysym::plus => E::Plus,
        Keysym::minus | Keysym::underscore => E::Minus,
        Keysym::equal => E::Equals,
        Keysym::bracketleft | Keysym::braceleft => E::OpenBracket,
        Keysym::bracketright | Keysym::braceright => E::CloseBracket,
        Keysym::grave | Keysym::asciitilde => E::Backtick,
        _ => return None,
    })
}

/// Project sctk's `Modifiers` struct onto egui's. `command` mirrors
/// `ctrl` on Linux, which is what egui itself does off macOS — widgets
/// test `command` for copy/paste and the command palette tests it for
/// Ctrl+K, so it has to be set.
///
/// It is deliberately *not* also reported as Super. `KeyChord::from_egui`
/// used to read `command` that way, which turned every Ctrl chord on this
/// backend into Ctrl+Super and matched nothing in the table (R26). Super
/// comes from `mac_cmd` alone; this path never sets it.
pub fn modifiers_to_egui(m: SctkModifiers) -> egui::Modifiers {
    egui::Modifiers {
        alt: m.alt,
        ctrl: m.ctrl,
        shift: m.shift,
        mac_cmd: false,
        command: m.ctrl,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_round_trip() {
        assert_eq!(keysym_to_egui_key(Keysym::a), Some(egui::Key::A));
        assert_eq!(keysym_to_egui_key(Keysym::A), Some(egui::Key::A));
        assert_eq!(keysym_to_egui_key(Keysym::z), Some(egui::Key::Z));
    }

    #[test]
    fn arrows_map_to_egui_arrows() {
        assert_eq!(keysym_to_egui_key(Keysym::Up), Some(egui::Key::ArrowUp));
        assert_eq!(keysym_to_egui_key(Keysym::Down), Some(egui::Key::ArrowDown));
        assert_eq!(keysym_to_egui_key(Keysym::Left), Some(egui::Key::ArrowLeft));
        assert_eq!(
            keysym_to_egui_key(Keysym::Right),
            Some(egui::Key::ArrowRight)
        );
    }

    #[test]
    fn named_control_keys() {
        assert_eq!(keysym_to_egui_key(Keysym::Escape), Some(egui::Key::Escape));
        assert_eq!(keysym_to_egui_key(Keysym::Return), Some(egui::Key::Enter));
        assert_eq!(
            keysym_to_egui_key(Keysym::KP_Enter),
            Some(egui::Key::Enter),
            "numpad enter folds into the same egui key"
        );
        assert_eq!(keysym_to_egui_key(Keysym::space), Some(egui::Key::Space));
    }

    #[test]
    fn punctuation_we_bind() {
        assert_eq!(keysym_to_egui_key(Keysym::grave), Some(egui::Key::Backtick));
    }

    /// Holding Shift *changes the keysym*, so the shifted form of every
    /// bound punctuation key has to resolve to the same egui key — the
    /// way `a | A` already did for letters. Without this the default
    /// Ctrl+Shift+` produced no event at all.
    #[test]
    fn shifted_punctuation_keeps_its_key_identity() {
        assert_eq!(
            keysym_to_egui_key(Keysym::asciitilde),
            Some(egui::Key::Backtick)
        );
        assert_eq!(
            keysym_to_egui_key(Keysym::braceleft),
            Some(egui::Key::OpenBracket)
        );
        assert_eq!(
            keysym_to_egui_key(Keysym::braceright),
            Some(egui::Key::CloseBracket)
        );
        assert_eq!(
            keysym_to_egui_key(Keysym::underscore),
            Some(egui::Key::Minus)
        );
        assert_eq!(
            keysym_to_egui_key(Keysym::bracketleft),
            Some(egui::Key::OpenBracket)
        );
    }

    /// Same hole as the punctuation, on the number row: Shift+1 resolves
    /// to `exclam`, which was unmapped, so the press never became an
    /// event at all.
    #[test]
    fn shifted_digits_keep_their_key_identity() {
        assert_eq!(keysym_to_egui_key(Keysym::exclam), Some(egui::Key::Num1));
        assert_eq!(keysym_to_egui_key(Keysym::at), Some(egui::Key::Num2));
        assert_eq!(
            keysym_to_egui_key(Keysym::parenright),
            Some(egui::Key::Num0)
        );
        assert_eq!(
            keysym_to_egui_key(Keysym::asciicircum),
            Some(egui::Key::Num6)
        );
        // The plain forms must still work.
        assert_eq!(keysym_to_egui_key(Keysym::_1), Some(egui::Key::Num1));
    }

    #[test]
    fn unmapped_keysym_returns_none() {
        // F1 isn't in our bind table — silent drop.
        assert_eq!(keysym_to_egui_key(Keysym::F1), None);
    }

    #[test]
    fn modifiers_project_cleanly() {
        let sctk = SctkModifiers {
            ctrl: true,
            shift: true,
            alt: false,
            logo: false,
            caps_lock: false,
            num_lock: false,
        };
        let e = modifiers_to_egui(sctk);
        assert!(e.ctrl);
        assert!(e.shift);
        assert!(!e.alt);
        assert!(e.command, "command mirrors ctrl on Linux");
    }
}
