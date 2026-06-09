//! `KeyboardHandler` impl + the text-input safety predicate.
//! Extracted in K.2.
//!
//! `is_unsafe_text_char` is the F.7 + G.3 audit response: filter the
//! C0 / C1 control range AND the Unicode `Format` (Cf) category before
//! handing characters to egui `TextEdit`. Without it, a hostile
//! keysym source could plant zero-width or RTL-override characters in
//! preset names / library tags.
//!
//! Modifier tracking is split off into `update_modifiers` per sctk's
//! protocol — we cache the latest snapshot on `WaylandState` so every
//! `press_key` / `release_key` already carries the active modifier
//! mask when egui processes it.

use super::state::WaylandState;
use crate::wayland::keyboard::{keysym_to_egui_key, modifiers_to_egui};
use smithay_client_toolkit::seat::keyboard::{
    KeyEvent, KeyboardHandler, Keymap, Keysym, Modifiers as SctkModifiers,
};
use wayland_client::{
    protocol::{wl_keyboard, wl_surface},
    Connection, QueueHandle,
};

/// Predicate matching characters we refuse to deliver to egui
/// `TextEdit` widgets via `egui::Event::Text`. Covers:
///
/// - C0 / C1 / DEL controls (`is_control()` → catches Ctrl+key
///   keysyms like `\x01` that xkbcommon emits when composition stays
///   active).
/// - Unicode "Format" category Cf members the audit (G.3) flagged
///   as dangerous to store: zero-width chars, RTL override, BOM,
///   soft hyphen, invisible separators (U+2060-U+206F).
fn is_unsafe_text_char(c: char) -> bool {
    if c.is_control() {
        return true;
    }
    matches!(
        c,
        '\u{00AD}'                  // soft hyphen
        | '\u{200B}'..='\u{200F}'   // zero-width joiners + LRM/RLM
        | '\u{202A}'..='\u{202E}'   // bidi embedding / override
        | '\u{2060}'..='\u{206F}'   // invisible operators + format codes
        | '\u{FEFF}'                // BOM / ZWNBSP
    )
}

impl KeyboardHandler for WaylandState {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[Keysym],
    ) {
        // Pre-pressed keys at focus enter are deliberately ignored —
        // synthesising press events for them would fire shortcuts the
        // user didn't intend (e.g. holding Tab while alt-tabbing into
        // the overlay would cycle entities).
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _serial: u32,
    ) {
        // Reset modifiers so a key released elsewhere doesn't leave us
        // thinking Ctrl is still down on the next focus.
        self.last_modifiers = SctkModifiers::default();
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        if let Some(key) = keysym_to_egui_key(event.keysym) {
            let modifiers = modifiers_to_egui(self.last_modifiers);
            self.pending_egui_events.push(egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            });
        }
        // UTF-8 text — already composed by xkbcommon. Push as a
        // separate Text event so text widgets get the character; chord
        // dispatch above already fired for shortcut-key combos.
        //
        // F.7 (0.5.1) + G.3 (0.5.3): strip C0 control characters and
        // the Unicode "Format" category (Cf) before pushing. The
        // initial F.7 pass only caught `is_control()` which covers
        // Cc (C0 / C1 / DEL) — xkbcommon producing `\x01` for Ctrl+A.
        // The audit re-check flagged that Cf characters (zero-width
        // joiners U+200B-U+200F, RTL-override U+202E, BOM U+FEFF,
        // soft hyphen U+00AD, U+2060-U+206F) sneak through and let
        // a user store invisible / display-reversing strings in
        // TextEdit widgets (preset names, library tags).
        if let Some(s) = event.utf8 {
            if !s.is_empty()
                && !self.last_modifiers.ctrl
                && !self.last_modifiers.alt
                && !self.last_modifiers.logo
            {
                let filtered: String = s.chars().filter(|c| !is_unsafe_text_char(*c)).collect();
                if !filtered.is_empty() {
                    self.pending_egui_events.push(egui::Event::Text(filtered));
                }
            }
        }
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        if let Some(key) = keysym_to_egui_key(event.keysym) {
            let modifiers = modifiers_to_egui(self.last_modifiers);
            self.pending_egui_events.push(egui::Event::Key {
                key,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers,
            });
        }
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        modifiers: SctkModifiers,
        _layout: u32,
    ) {
        self.last_modifiers = modifiers;
    }

    fn update_keymap(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _keymap: Keymap<'_>,
    ) {
        // sctk's default handler already builds the in-memory keymap;
        // we only ever read decoded keysyms from `KeyEvent`, so this
        // hook is a no-op placeholder for future locale-switch logic.
    }
}
