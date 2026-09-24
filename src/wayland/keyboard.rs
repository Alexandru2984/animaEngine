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
        Keysym::_0 => E::Num0,
        Keysym::_1 => E::Num1,
        Keysym::_2 => E::Num2,
        Keysym::_3 => E::Num3,
        Keysym::_4 => E::Num4,
        Keysym::_5 => E::Num5,
        Keysym::_6 => E::Num6,
        Keysym::_7 => E::Num7,
        Keysym::_8 => E::Num8,
        Keysym::_9 => E::Num9,
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
        // Unshifted forms only. The shifted keysyms (`asciitilde`,
        // `exclam`, `braceleft`…) were listed here for R29 and R35, and
        // that assumed a US layout: on AZERTY `ampersand` is the plain 1
        // key, and folding it onto 7 bound the wrong key. A keysym this
        // table does not know now falls back to the physical position
        // instead — see `egui_key_for` — which is right on every layout.
        Keysym::plus => E::Plus,
        Keysym::minus => E::Minus,
        Keysym::equal => E::Equals,
        Keysym::bracketleft => E::OpenBracket,
        Keysym::bracketright => E::CloseBracket,
        Keysym::grave => E::Backtick,
        _ => return None,
    })
}

/// The key at this physical position, named as on a US layout, from the
/// Linux evdev scancode `wl_keyboard` delivers (`KeyEvent::raw_code`).
///
/// Letters, the digit row and the punctuation we bind. Letters matter for
/// layouts with no Latin keysyms at all: on Cyrillic the C key produces
/// `Cyrillic_es`, which `keysym_to_egui_key` does not know, and without
/// the position Ctrl+C would never become a key event.
pub fn evdev_to_egui_key(code: u32) -> Option<egui::Key> {
    use egui::Key as E;
    Some(match code {
        2 => E::Num1,
        3 => E::Num2,
        4 => E::Num3,
        5 => E::Num4,
        6 => E::Num5,
        7 => E::Num6,
        8 => E::Num7,
        9 => E::Num8,
        10 => E::Num9,
        11 => E::Num0,
        12 => E::Minus,
        13 => E::Equals,
        16 => E::Q,
        17 => E::W,
        18 => E::E,
        19 => E::R,
        20 => E::T,
        21 => E::Y,
        22 => E::U,
        23 => E::I,
        24 => E::O,
        25 => E::P,
        26 => E::OpenBracket,
        27 => E::CloseBracket,
        30 => E::A,
        31 => E::S,
        32 => E::D,
        33 => E::F,
        34 => E::G,
        35 => E::H,
        36 => E::J,
        37 => E::K,
        38 => E::L,
        41 => E::Backtick,
        44 => E::Z,
        45 => E::X,
        46 => E::C,
        47 => E::V,
        48 => E::B,
        49 => E::N,
        50 => E::M,
        _ => return None,
    })
}

/// The egui key to report for a press, and its physical key.
///
/// The same shape egui-winit produces on the other backend — logical key
/// if known, else the position, with the position always attached — so
/// `KeyChord::from_egui_event` sees identical input from both. `None`
/// when neither is anything we bind.
pub fn egui_key_for(keysym: Keysym, raw_code: u32) -> Option<(egui::Key, Option<egui::Key>)> {
    let physical = evdev_to_egui_key(raw_code);
    let key = keysym_to_egui_key(keysym).or(physical)?;
    Some((key, physical))
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

    /// Shifted keysyms are no longer aliased; the position decides. On US
    /// Shift+` arrives as `asciitilde` at evdev 41 and still resolves to
    /// the backtick key, which is what the perf-overlay chord needs.
    #[test]
    fn an_unmapped_keysym_falls_back_to_its_position() {
        assert_eq!(
            egui_key_for(Keysym::asciitilde, 41),
            Some((egui::Key::Backtick, Some(egui::Key::Backtick)))
        );
        assert_eq!(
            egui_key_for(Keysym::exclam, 2),
            Some((egui::Key::Num1, Some(egui::Key::Num1)))
        );
    }

    /// The AZERTY case R35 got wrong: `ampersand` on the 1 key (evdev 2)
    /// must come out as 1, not the 7 a US-shift alias made of it.
    #[test]
    fn azerty_number_row_reports_its_position() {
        assert_eq!(
            egui_key_for(Keysym::ampersand, 2),
            Some((egui::Key::Num1, Some(egui::Key::Num1)))
        );
        assert_eq!(
            egui_key_for(Keysym::eacute, 3),
            Some((egui::Key::Num2, Some(egui::Key::Num2)))
        );
    }

    /// A letter keeps its label even where its position differs (AZERTY A
    /// sits at US Q), and carries the position along for the resolver.
    #[test]
    fn letters_keep_their_label() {
        assert_eq!(
            egui_key_for(Keysym::a, 16),
            Some((egui::Key::A, Some(egui::Key::Q)))
        );
    }

    /// A non-Latin letter has no keysym mapping at all; without the
    /// position there would be no key event, and no Ctrl+C.
    #[test]
    fn non_latin_letters_fall_back_to_the_position() {
        assert_eq!(
            egui_key_for(Keysym::Cyrillic_es, 46),
            Some((egui::Key::C, Some(egui::Key::C)))
        );
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
