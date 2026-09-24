//! Notification-area (system tray) icon on Windows.
//!
//! The Windows counterpart of `tray.rs`, with the same menu
//! (`tray_menu::MENU`) and the same shape: the icon lives on its own
//! thread and turns clicks into `AnimaEvent`s posted to the winit event
//! loop. Plain `Shell_NotifyIconW` on the `windows-sys` bindings the
//! overlay already links, rather than a tray crate: this is one icon and
//! one popup menu, and a crate would add a second copy of the Win32
//! bindings and a dependency tree to audit for it.
//!
//! The thread owns a hidden window, which is where the shell sends the
//! icon's mouse messages. Top-level rather than message-only: a
//! message-only window never sees broadcasts, and `TaskbarCreated` — the
//! one that says Explorer restarted and the icon is gone — is a
//! broadcast. Dropping the returned [`Tray`] closes the window, which
//! removes the icon; otherwise Windows leaves a dead icon in the tray
//! until the pointer happens to pass over it.

use crate::event::AnimaEvent;
use crate::tray_menu::{TrayItem, ACTIVATE, MENU, TITLE};
use std::cell::RefCell;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, GetCursorPos, GetMessageW, LoadIconW, PostMessageW, PostQuitMessage,
    RegisterClassW, RegisterWindowMessageW, SetForegroundWindow, TrackPopupMenu, TranslateMessage,
    IDI_APPLICATION, MF_SEPARATOR, MF_STRING, MSG, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    WM_APP, WM_CLOSE, WM_CONTEXTMENU, WM_DESTROY, WM_LBUTTONUP, WM_NULL, WM_RBUTTONUP, WNDCLASSW,
};
use winit::event_loop::EventLoopProxy;

/// The message the shell sends the window for icon activity.
const WM_TRAY: u32 = WM_APP + 1;
/// Our one icon's id within the window.
const ICON_ID: u32 = 1;
/// Menu command ids are the item's index in `MENU`, offset so that 0 —
/// what `TrackPopupMenu` returns when the menu is dismissed — is never
/// an item.
const FIRST_COMMAND: usize = 1;

thread_local! {
    /// The tray thread's state. The window procedure runs on this thread
    /// only (it is the thread that created the window and pumps it).
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

struct State {
    proxy: EventLoopProxy<AnimaEvent>,
    /// Explorer broadcasts this when it restarts; the icon has to be
    /// added again or it is simply gone until the app restarts.
    taskbar_created: u32,
}

/// The running tray. Dropping it removes the icon.
pub struct Tray {
    hwnd: HWND,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for Tray {
    fn drop(&mut self) {
        // SAFETY: posting to a window that may already be gone is safe;
        // the call just fails.
        unsafe {
            PostMessageW(self.hwnd, WM_CLOSE, 0, 0);
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Put the icon in the notification area. `None` if it could not be
/// created; the app remains usable from the ⚙ button and the keybinds.
pub fn spawn(proxy: EventLoopProxy<AnimaEvent>) -> Option<Tray> {
    let (tx, rx) = std::sync::mpsc::channel();
    let thread = std::thread::Builder::new()
        .name("anima-tray".into())
        .spawn(move || {
            // SAFETY: everything below runs on this thread, which creates
            // the window and then pumps its messages until WM_QUIT.
            unsafe { run(proxy, tx) }
        });
    let thread = match thread {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("Tray thread failed to start: {e}");
            return None;
        }
    };
    match rx.recv() {
        Ok(Some(hwnd)) => {
            tracing::info!("System tray registered (notification area)");
            Some(Tray {
                hwnd,
                thread: Some(thread),
            })
        }
        _ => {
            tracing::warn!("Tray unavailable. The app still works — use the ⚙ button or keybinds.");
            let _ = thread.join();
            None
        }
    }
}

/// Create the window and the icon, report the window back, pump messages.
unsafe fn run(proxy: EventLoopProxy<AnimaEvent>, ready: std::sync::mpsc::Sender<Option<HWND>>) {
    let class = wide("animaEngine.tray");
    let hinstance = GetModuleHandleW(std::ptr::null());
    let wc = WNDCLASSW {
        style: 0,
        lpfnWndProc: Some(wndproc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        hIcon: 0,
        hCursor: 0,
        hbrBackground: 0,
        lpszMenuName: std::ptr::null(),
        lpszClassName: class.as_ptr(),
    };
    RegisterClassW(&wc);

    let taskbar_created = RegisterWindowMessageW(wide("TaskbarCreated").as_ptr());
    STATE.with(|s| {
        *s.borrow_mut() = Some(State {
            proxy,
            taskbar_created,
        })
    });

    let title = wide(TITLE);
    let hwnd = CreateWindowExW(
        0,
        class.as_ptr(),
        title.as_ptr(),
        0,
        0,
        0,
        0,
        0,
        0, // no parent: top-level, and never shown
        0,
        hinstance,
        std::ptr::null(),
    );
    if hwnd == 0 || !add_icon(hwnd) {
        let _ = ready.send(None);
        if hwnd != 0 {
            DestroyWindow(hwnd);
        }
        return;
    }
    let _ = ready.send(Some(hwnd));

    let mut msg: MSG = std::mem::zeroed();
    while GetMessageW(&mut msg, 0, 0, 0) > 0 {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}

fn icon_data(hwnd: HWND) -> NOTIFYICONDATAW {
    // SAFETY: NOTIFYICONDATAW is plain data; all-zero is its documented
    // "nothing set" state, and the fields that matter are filled below.
    let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = ICON_ID;
    nid
}

unsafe fn add_icon(hwnd: HWND) -> bool {
    let mut nid = icon_data(hwnd);
    nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    nid.uCallbackMessage = WM_TRAY;
    // The stock application icon until the project has a raster brand
    // mark; the Linux tray uses a stock freedesktop icon for the same
    // reason.
    nid.hIcon = LoadIconW(0, IDI_APPLICATION);
    copy_truncated(&mut nid.szTip, TITLE);
    Shell_NotifyIconW(NIM_ADD, &nid) != 0
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_TRAY => {
            // Legacy (pre-NOTIFYICON_VERSION_4) callback: the mouse
            // message is the whole of lParam.
            match lparam as u32 {
                WM_LBUTTONUP => send(ACTIVATE),
                WM_RBUTTONUP | WM_CONTEXTMENU => show_menu(hwnd),
                _ => {}
            }
            0
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            Shell_NotifyIconW(NIM_DELETE, &icon_data(hwnd));
            PostQuitMessage(0);
            0
        }
        _ => {
            let taskbar_created = STATE.with(|s| s.borrow().as_ref().map(|s| s.taskbar_created));
            if taskbar_created == Some(msg) && msg != 0 {
                add_icon(hwnd);
                return 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
    }
}

unsafe fn show_menu(hwnd: HWND) {
    let menu = CreatePopupMenu();
    if menu == 0 {
        return;
    }
    for (i, item) in MENU.iter().enumerate() {
        match item {
            TrayItem::Action { label, .. } => {
                // AppendMenuW copies the string during the call.
                let label = wide(label);
                AppendMenuW(menu, MF_STRING, FIRST_COMMAND + i, label.as_ptr());
            }
            TrayItem::Separator => {
                AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
            }
        }
    }
    let mut at = POINT { x: 0, y: 0 };
    GetCursorPos(&mut at);
    // Without this the menu does not close when the user clicks elsewhere
    // (documented behaviour of TrackPopupMenu for notification icons).
    SetForegroundWindow(hwnd);
    let chosen = TrackPopupMenu(
        menu,
        TPM_RIGHTBUTTON | TPM_RETURNCMD | TPM_NONOTIFY,
        at.x,
        at.y,
        0,
        hwnd,
        std::ptr::null(),
    );
    PostMessageW(hwnd, WM_NULL, 0, 0);
    DestroyMenu(menu);
    if let Some(event) = command_event(chosen as usize) {
        send(event);
    }
}

/// The event a menu command id stands for.
fn command_event(command: usize) -> Option<AnimaEvent> {
    match MENU.get(command.checked_sub(FIRST_COMMAND)?)? {
        TrayItem::Action { event, .. } => Some(*event),
        TrayItem::Separator => None,
    }
}

fn send(event: AnimaEvent) {
    STATE.with(|s| {
        if let Some(state) = s.borrow().as_ref() {
            let _ = state.proxy.send_event(event);
        }
    });
}

/// UTF-16, NUL-terminated, for the `W` APIs.
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Copy `s` into a fixed UTF-16 buffer, truncated and NUL-terminated.
fn copy_truncated(buf: &mut [u16], s: &str) {
    let max = buf.len().saturating_sub(1);
    let mut n = 0;
    for (slot, unit) in buf.iter_mut().zip(s.encode_utf16().take(max)) {
        *slot = unit;
        n += 1;
    }
    if let Some(end) = buf.get_mut(n) {
        *end = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 0 is what TrackPopupMenu returns for "dismissed", so it must not
    /// map to an item — least of all to Quit.
    #[test]
    fn a_dismissed_menu_does_nothing() {
        assert!(command_event(0).is_none());
    }

    #[test]
    fn every_action_round_trips_through_its_command_id() {
        for (i, item) in MENU.iter().enumerate() {
            let got = command_event(FIRST_COMMAND + i);
            match item {
                TrayItem::Action { event, .. } => {
                    assert_eq!(format!("{got:?}"), format!("{:?}", Some(*event)));
                }
                TrayItem::Separator => assert!(got.is_none()),
            }
        }
        assert!(command_event(FIRST_COMMAND + MENU.len()).is_none());
    }

    #[test]
    fn the_tooltip_is_truncated_and_terminated() {
        let mut buf = [0xffffu16; 4];
        copy_truncated(&mut buf, "abcdef");
        assert_eq!(buf, [97, 98, 99, 0]);
    }
}
