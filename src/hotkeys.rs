//! Global hotkeys — toggle the overlay from any focused window.
//!
//! Backed by the `global-hotkey` crate. On X11 (and XWayland) it uses
//! `XGrabKey`, which works regardless of which window currently has focus.
//! On a native Wayland session XGrabKey is not exposed, so the hotkeys
//! silently no-op — the tray menu and `⚙` button still work.
//!
//! Hotkeys are intentionally hardcoded for now. Making them user-rebindable
//! is part of the dedicated UI/UX phase.

use crate::event::AnimaEvent;
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use winit::event_loop::EventLoopProxy;

/// Owns the manager + the registered hotkeys. Dropping it un-registers
/// every binding, so callers must hold this alive for the lifetime of
/// the app.
pub struct HotkeyController {
    _manager: GlobalHotKeyManager,
}

/// Try to register the standard set of global hotkeys. Returns `None` on
/// platforms / sessions that don't expose key grabbing (e.g. native
/// Wayland without an XWayland fallback) — the caller treats this as a
/// soft failure: the app remains fully usable through tray + ⚙ button.
pub fn register(proxy: EventLoopProxy<AnimaEvent>) -> Option<HotkeyController> {
    let manager = match GlobalHotKeyManager::new() {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("Global hotkeys unavailable: {e}");
            return None;
        }
    };

    // Ctrl+Shift+A → edit mode toggle. "A" for animation.
    let edit = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyA);
    // Ctrl+Shift+H → hide/show overlay.
    let visibility = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyH);
    // Ctrl+Shift+P → play/pause global animation.
    let playback = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyP);

    // Track visibility state in the closure so the same hotkey toggles
    // between Show and Hide. Storing it as an Arc<AtomicBool> lets the
    // handler stay 'Send + 'static.
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    let visible = Arc::new(AtomicBool::new(true));

    let mut registered_any = false;
    for (hk, label) in [
        (edit, "Ctrl+Shift+A → edit mode"),
        (visibility, "Ctrl+Shift+H → show/hide"),
        (playback, "Ctrl+Shift+P → playback"),
    ] {
        match manager.register(hk) {
            Ok(()) => {
                tracing::info!("Registered hotkey {label}");
                registered_any = true;
            }
            Err(e) => {
                tracing::warn!("Failed to register {label}: {e}");
            }
        }
    }

    if !registered_any {
        return None;
    }

    let edit_id = edit.id();
    let vis_id = visibility.id();
    let play_id = playback.id();

    // Install the global handler. Trigger only on `Pressed` — `Released`
    // events would fire a second time on key-up.
    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if event.state != HotKeyState::Pressed {
            return;
        }

        let outgoing = if event.id == edit_id {
            AnimaEvent::ToggleEditMode
        } else if event.id == vis_id {
            // Hard toggle — local copy of intent for next press.
            let was_visible = visible.fetch_xor(true, Ordering::SeqCst);
            if was_visible {
                AnimaEvent::HideOverlay
            } else {
                AnimaEvent::ShowOverlay
            }
        } else if event.id == play_id {
            AnimaEvent::ToggleGlobalPlayback
        } else {
            return;
        };

        if proxy.send_event(outgoing).is_err() {
            // Event loop is closed — nothing left to do.
        }
    }));

    Some(HotkeyController { _manager: manager })
}
