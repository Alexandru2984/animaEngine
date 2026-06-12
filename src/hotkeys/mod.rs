//! Global hotkeys — toggle the overlay from any focused window.
//!
//! Backed by the `global-hotkey` crate. On X11 (and XWayland) it uses
//! `XGrabKey`, which works regardless of which window currently has
//! focus. On a native Wayland session XGrabKey is not exposed, so the
//! hotkeys silently no-op — the tray menu and `⚙` button still work.
//!
//! `probe` (T.0) detects whether the session offers the
//! `GlobalShortcuts` desktop portal — the mechanism that will replace
//! XGrabKey as the preferred backend on GNOME/KDE Wayland (T.1/T.2).
//!
//! The set of globally-registered chords is derived from
//! [`KeyBindings`] at startup:
//! whichever chord the user has bound to `ToggleEditMode`,
//! `HideOverlay`, or `PauseAll` *and* that carries at least one
//! modifier (Ctrl / Alt / Super) gets registered. Bare-letter chords
//! are skipped on purpose — `XGrabKey`-ing a plain `Q` would steal
//! the key from every focused app.

pub mod portal;
pub mod probe;

use crate::event::AnimaEvent;
use crate::keybindings::{
    Action, KeyBindings, KeyChord, KeyCode, ModifierMask, NamedKey, SymbolKey,
};
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use winit::event_loop::EventLoopProxy;

/// Owns the manager + the registered hotkeys. Dropping it un-registers
/// every binding, so callers must hold this alive for the lifetime of
/// the app.
pub struct HotkeyController {
    _manager: GlobalHotKeyManager,
}

/// Actions that may be registered as global hotkeys. Per-entity
/// shortcuts (NudgeUp, ToggleVisible, etc.) intentionally never make
/// it here — there's no useful global meaning without a selection.
const GLOBAL_ACTIONS: &[Action] = &[
    Action::ToggleEditMode,
    Action::HideOverlay,
    Action::PauseAll,
];

/// Map a globally-triggered action onto the event the main loop
/// consumes. Shared by both backends (XGrabKey handler and the
/// portal bridge) so HideOverlay's toggle semantics — flip a shared
/// visibility bit, emit Hide or Show accordingly — stay identical no
/// matter which mechanism fired.
pub fn action_to_event(action: Action, visible: &AtomicBool) -> Option<AnimaEvent> {
    Some(match action {
        Action::ToggleEditMode => AnimaEvent::ToggleEditMode,
        Action::HideOverlay => {
            let was_visible = visible.fetch_xor(true, Ordering::SeqCst);
            if was_visible {
                AnimaEvent::HideOverlay
            } else {
                AnimaEvent::ShowOverlay
            }
        }
        Action::PauseAll => AnimaEvent::ToggleGlobalPlayback,
        _ => return None,
    })
}

/// Try to register the user's globally-bound chords. Returns `None`
/// when no chord could be registered — callers treat that as a soft
/// failure: the app remains fully usable through tray + ⚙ button.
pub fn register(
    proxy: EventLoopProxy<AnimaEvent>,
    bindings: &KeyBindings,
) -> Option<HotkeyController> {
    let manager = match GlobalHotKeyManager::new() {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("Global hotkeys unavailable: {e}");
            return None;
        }
    };

    // Track visibility state for HideOverlay's hard toggle. Stored as
    // Arc<AtomicBool> so the closure stays `Send + 'static`.
    let visible = Arc::new(AtomicBool::new(true));

    let mut id_to_action: HashMap<u32, Action> = HashMap::new();
    let mut registered_any = false;

    for &action in GLOBAL_ACTIONS {
        for chord in bindings.chords_for(action) {
            // Bare-letter chords are deliberately skipped — XGrabKey
            // would steal the key from every other focused app.
            if chord.mods == ModifierMask::NONE {
                continue;
            }
            let Some(hk) = chord_to_global_hotkey(chord) else {
                tracing::debug!(
                    "Chord {} bound to {:?} has no global-hotkey equivalent, skipping",
                    chord.canonical_str(),
                    action,
                );
                continue;
            };
            match manager.register(hk) {
                Ok(()) => {
                    tracing::info!(
                        "Registered global hotkey {} → {:?}",
                        chord.canonical_str(),
                        action,
                    );
                    id_to_action.insert(hk.id(), action);
                    registered_any = true;
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to register {} → {:?}: {e}",
                        chord.canonical_str(),
                        action,
                    );
                }
            }
        }
    }

    if !registered_any {
        return None;
    }

    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if event.state != HotKeyState::Pressed {
            return;
        }
        let Some(action) = id_to_action.get(&event.id).copied() else {
            return;
        };
        let Some(outgoing) = action_to_event(action, &visible) else {
            return;
        };
        if proxy.send_event(outgoing).is_err() {
            // Event loop is closed — nothing left to do.
        }
    }));

    Some(HotkeyController { _manager: manager })
}

/// Convert a `KeyChord` into the `global-hotkey` crate's
/// `(Modifiers, Code)` pair. Returns `None` for keys that aren't
/// representable in the global-hotkey vocabulary (numeric digits, IME,
/// etc.) — the registration path skips those.
fn chord_to_global_hotkey(chord: KeyChord) -> Option<HotKey> {
    let mods = mods_to_global(chord.mods);
    let code = keycode_to_global_code(chord.key)?;
    Some(HotKey::new(Some(mods), code))
}

fn mods_to_global(mask: ModifierMask) -> Modifiers {
    let mut m = Modifiers::empty();
    if mask.ctrl() {
        m |= Modifiers::CONTROL;
    }
    if mask.shift() {
        m |= Modifiers::SHIFT;
    }
    if mask.alt() {
        m |= Modifiers::ALT;
    }
    if mask.sup() {
        m |= Modifiers::SUPER;
    }
    m
}

fn keycode_to_global_code(key: KeyCode) -> Option<Code> {
    Some(match key {
        KeyCode::Letter(c) => match c {
            'A' => Code::KeyA,
            'B' => Code::KeyB,
            'C' => Code::KeyC,
            'D' => Code::KeyD,
            'E' => Code::KeyE,
            'F' => Code::KeyF,
            'G' => Code::KeyG,
            'H' => Code::KeyH,
            'I' => Code::KeyI,
            'J' => Code::KeyJ,
            'K' => Code::KeyK,
            'L' => Code::KeyL,
            'M' => Code::KeyM,
            'N' => Code::KeyN,
            'O' => Code::KeyO,
            'P' => Code::KeyP,
            'Q' => Code::KeyQ,
            'R' => Code::KeyR,
            'S' => Code::KeyS,
            'T' => Code::KeyT,
            'U' => Code::KeyU,
            'V' => Code::KeyV,
            'W' => Code::KeyW,
            'X' => Code::KeyX,
            'Y' => Code::KeyY,
            'Z' => Code::KeyZ,
            _ => return None,
        },
        // Digits would need Digit0..=Digit9 (not currently bound to any
        // action — skip rather than match an unused branch).
        KeyCode::Digit(_) => return None,
        KeyCode::Named(n) => match n {
            NamedKey::Escape => Code::Escape,
            NamedKey::Space => Code::Space,
            NamedKey::Tab => Code::Tab,
            NamedKey::Enter => Code::Enter,
            NamedKey::Backspace => Code::Backspace,
            NamedKey::Delete => Code::Delete,
            NamedKey::Home => Code::Home,
            NamedKey::End => Code::End,
            NamedKey::PageUp => Code::PageUp,
            NamedKey::PageDown => Code::PageDown,
            NamedKey::ArrowUp => Code::ArrowUp,
            NamedKey::ArrowDown => Code::ArrowDown,
            NamedKey::ArrowLeft => Code::ArrowLeft,
            NamedKey::ArrowRight => Code::ArrowRight,
        },
        KeyCode::Symbol(s) => match s {
            SymbolKey::Plus => return None, // No matching keyboard-types Code
            SymbolKey::Minus => Code::Minus,
            SymbolKey::Equal => Code::Equal,
            SymbolKey::BracketLeft => Code::BracketLeft,
            SymbolKey::BracketRight => Code::BracketRight,
            SymbolKey::Backquote => Code::Backquote,
        },
    })
}
