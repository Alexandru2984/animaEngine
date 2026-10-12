//! Global hotkeys — toggle the overlay from any focused window.
//!
//! On X11 (and XWayland) they are grabbed with `XGrabKey`
//! ([`x11_grab`]), which works regardless of which window currently has
//! focus; on Windows the `global-hotkey` crate registers them. On a native Wayland session XGrabKey is not exposed, so the
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

// The GlobalShortcuts portal is a D-Bus (zbus) client — unix-only. The
// `global-hotkey` path below stays portable: it is XGrabKey on X11 and
// `RegisterHotKey` on Windows.
#[cfg(unix)]
pub mod portal;
pub mod probe;
#[cfg(unix)]
pub mod x11_grab;

use crate::event::AnimaEvent;
use crate::keybindings::{Action, KeyBindings, KeyChord, ModifierMask};
#[cfg(windows)]
use crate::keybindings::{KeyCode, NamedKey, SymbolKey};
#[cfg(windows)]
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
#[cfg(windows)]
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(windows)]
use std::sync::Arc;
#[cfg(windows)]
use winit::event_loop::EventLoopProxy;

/// Owns the manager + the registered hotkeys (Windows). Dropping it
/// un-registers every binding, so callers must hold this alive for the
/// lifetime of the app.
pub struct HotkeyController {
    #[cfg(windows)]
    _manager: GlobalHotKeyManager,
}

/// Actions that may be registered as global hotkeys. Per-entity
/// shortcuts (NudgeUp, ToggleVisible, etc.) intentionally never make
/// it here — there's no useful global meaning without a selection.
const GLOBAL_ACTIONS: &[Action] = &[
    Action::ToggleEditMode,
    Action::HideOverlay,
    Action::PauseAll,
    Action::ToggleFocus,
    Action::ToggleSnooze,
    Action::ToggleMute,
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
        // Which way each goes is decided now, by what holds now.
        Action::ToggleFocus => {
            if crate::focus::phase().is_some() {
                AnimaEvent::StopFocus
            } else {
                AnimaEvent::StartFocus(0)
            }
        }
        Action::ToggleSnooze => {
            if crate::snooze::active() {
                AnimaEvent::Wake
            } else {
                AnimaEvent::Snooze(crate::snooze::SnoozeFor::Minutes(15))
            }
        }
        Action::ToggleMute => AnimaEvent::ToggleMute,
        _ => return None,
    })
}

/// The chords to register globally: those bound to `GLOBAL_ACTIONS`
/// with a modifier. Bare-letter chords are deliberately skipped — a grab
/// would steal the key from every other focused app.
pub fn global_chords(bindings: &KeyBindings) -> Vec<(KeyChord, Action)> {
    GLOBAL_ACTIONS
        .iter()
        .flat_map(|&action| {
            bindings
                .chords_for(action)
                .into_iter()
                .filter(|chord| chord.mods != ModifierMask::NONE)
                .map(move |chord| (chord, action))
        })
        .collect()
}

/// Try to register the user's globally-bound chords (Windows). Returns `None`
/// when no chord could be registered — callers treat that as a soft
/// failure: the app remains fully usable through tray + ⚙ button.
#[cfg(windows)]
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

    for (chord, action) in global_chords(bindings) {
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
#[cfg(windows)]
fn chord_to_global_hotkey(chord: KeyChord) -> Option<HotKey> {
    let mods = mods_to_global(chord.mods);
    let code = keycode_to_global_code(chord.key)?;
    Some(HotKey::new(Some(mods), code))
}

#[cfg(windows)]
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

#[cfg(windows)]
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
            NamedKey::F(n) => *[
                Code::F1,
                Code::F2,
                Code::F3,
                Code::F4,
                Code::F5,
                Code::F6,
                Code::F7,
                Code::F8,
                Code::F9,
                Code::F10,
                Code::F11,
                Code::F12,
            ]
            .get(usize::from(n).checked_sub(1)?)?,
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
