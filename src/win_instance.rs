//! Single instance on Windows: a named mutex, and a named event to ask the
//! running instance to raise itself.
//!
//! The Windows counterpart of `single_instance.rs`, which does the same
//! over the D-Bus session bus. Both objects live in the `Local\`
//! namespace — per logon session, as the session bus is — so two users
//! on one machine each get their own overlay.
//!
//! The handshake:
//!
//! 1. open-or-create the *event*, then create the *mutex*;
//! 2. if the mutex already existed, another instance owns it: signal the
//!    event and exit;
//! 3. otherwise this is the instance. Once there is an event loop to post
//!    to, [`Instance::listen`] waits on the event and turns each signal
//!    into `AnimaEvent::RaiseWindow`.
//!
//! The event is opened first on purpose. `CreateEventW` on an existing
//! name returns a handle to the existing event, so a second launch always
//! has something to signal, even if it races the first one's startup.

use crate::event::AnimaEvent;
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, WAIT_OBJECT_0,
};
use windows_sys::Win32::System::Threading::{
    CreateEventW, CreateMutexW, SetEvent, WaitForSingleObject, INFINITE,
};
use winit::event_loop::EventLoopProxy;

const MUTEX_NAME: &str = r"Local\com.animaengine.Anima";
const RAISE_EVENT_NAME: &str = r"Local\com.animaengine.Anima.raise";

/// What [`try_acquire`] found.
pub enum Acquire {
    /// This process is the instance.
    Claimed(Instance),
    /// Another instance is running and has been asked to raise itself.
    HandedOff,
}

/// Ownership of the single-instance mutex, and the event a later launch
/// signals. Keep it for the life of the process.
pub struct Instance {
    mutex: HANDLE,
    raise: HANDLE,
}

// Kernel object handles are process-wide, not thread-affine.
unsafe impl Send for Instance {}

/// Claim the single instance, or hand off to the one already running.
///
/// If the kernel objects cannot be created at all, this process runs
/// anyway: a second overlay is a much smaller failure than none.
pub fn try_acquire() -> Option<Acquire> {
    try_acquire_named(MUTEX_NAME, RAISE_EVENT_NAME)
}

fn try_acquire_named(mutex_name: &str, event_name: &str) -> Option<Acquire> {
    let event_name = wide(event_name);
    let mutex_name = wide(mutex_name);
    // SAFETY: plain Win32 calls with valid, NUL-terminated names and null
    // security attributes; each returned handle is checked before use.
    unsafe {
        // Auto-reset, initially clear: one signal wakes the listener once.
        let raise = CreateEventW(std::ptr::null(), 0, 0, event_name.as_ptr());
        if raise == 0 {
            tracing::warn!("Single-instance event unavailable ({})", GetLastError());
            return None;
        }
        let mutex = CreateMutexW(std::ptr::null(), 0, mutex_name.as_ptr());
        let already_running = GetLastError() == ERROR_ALREADY_EXISTS;
        if mutex == 0 {
            tracing::warn!("Single-instance mutex unavailable ({})", GetLastError());
            CloseHandle(raise);
            return None;
        }
        if already_running {
            SetEvent(raise);
            CloseHandle(mutex);
            CloseHandle(raise);
            return Some(Acquire::HandedOff);
        }
        Some(Acquire::Claimed(Instance { mutex, raise }))
    }
}

impl Instance {
    /// Raise the overlay whenever a later launch signals.
    ///
    /// Consumes the handle: the listener thread keeps the mutex (and so
    /// the claim) until the process exits.
    pub fn listen(self, proxy: EventLoopProxy<AnimaEvent>) {
        let spawned = std::thread::Builder::new()
            .name("anima-instance".into())
            .spawn(move || {
                let instance = self;
                loop {
                    // SAFETY: `raise` is a live event handle owned by
                    // `instance`, which this thread holds.
                    let woke = unsafe { WaitForSingleObject(instance.raise, INFINITE) };
                    if woke != WAIT_OBJECT_0 {
                        tracing::warn!("Single-instance listener stopped ({woke})");
                        return;
                    }
                    if proxy.send_event(AnimaEvent::RaiseWindow).is_err() {
                        // The event loop is gone; the process is exiting.
                        return;
                    }
                }
            });
        if let Err(e) = spawned {
            tracing::warn!("Single-instance listener failed to start: {e}");
        }
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        // SAFETY: both handles were returned non-null by Create* and are
        // closed exactly once, here.
        unsafe {
            CloseHandle(self.raise);
            CloseHandle(self.mutex);
        }
    }
}

/// UTF-16, NUL-terminated, for the `W` APIs.
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unique per test run, so parallel tests and a running overlay never
    /// share a name.
    fn names(tag: &str) -> (String, String) {
        let base = format!(r"Local\anima-test-{}-{tag}", std::process::id());
        (base.clone(), format!("{base}.raise"))
    }

    #[test]
    fn the_first_launch_claims_and_the_second_hands_off() {
        let (m, e) = names("claim");
        let first = try_acquire_named(&m, &e).expect("kernel objects");
        assert!(matches!(first, Acquire::Claimed(_)));
        let second = try_acquire_named(&m, &e).expect("kernel objects");
        assert!(matches!(second, Acquire::HandedOff));
    }

    /// The hand-off signals the event the first instance waits on.
    #[test]
    fn a_second_launch_signals_the_first() {
        let (m, e) = names("signal");
        let Some(Acquire::Claimed(first)) = try_acquire_named(&m, &e) else {
            panic!("the first launch must claim");
        };
        assert!(matches!(
            try_acquire_named(&m, &e),
            Some(Acquire::HandedOff)
        ));
        // SAFETY: `first.raise` is live for the duration of the test.
        let woke = unsafe { WaitForSingleObject(first.raise, 0) };
        assert_eq!(woke, WAIT_OBJECT_0, "the event was not signalled");
    }

    /// Once the instance is gone, the next launch claims afresh.
    #[test]
    fn the_claim_ends_with_the_instance() {
        let (m, e) = names("release");
        let first = try_acquire_named(&m, &e);
        drop(first);
        assert!(matches!(
            try_acquire_named(&m, &e),
            Some(Acquire::Claimed(_))
        ));
    }

    #[test]
    fn names_are_nul_terminated() {
        assert_eq!(wide("ab"), vec![97, 98, 0]);
    }
}
