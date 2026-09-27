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
//! 1. open-or-create the *events*, then create the *mutex*;
//! 2. if the mutex already existed, another instance owns it: signal the
//!    raise event and wait for the running instance to acknowledge it;
//! 3. otherwise this is the instance. Once there is an event loop to post
//!    to, [`Instance::listen`] waits on the raise event, turns each signal
//!    into `AnimaEvent::RaiseWindow`, and acknowledges it.
//!
//! The events are opened first on purpose. `CreateEventW` on an existing
//! name returns a handle to the existing event, so a second launch always
//! has something to signal, even if it races the first one's startup.
//!
//! The acknowledgement is what makes a relaunch right after Quit work. An
//! exiting instance keeps the mutex alive until its process is gone, and
//! a launch that signalled and exited then left no overlay at all (R48 in
//! `docs/runtime-findings.md`, found on Linux, where the D-Bus handshake
//! had the same shape). Without an answer within `ACK_TIMEOUT`, the
//! launch lets go of its mutex handle and tries again, for up to
//! `HANDOFF_PATIENCE`: once the old process is gone, the mutex goes with
//! it, and the next attempt claims.

use crate::event::AnimaEvent;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, WAIT_OBJECT_0,
};
use windows_sys::Win32::System::Threading::{
    CreateEventW, CreateMutexW, ResetEvent, SetEvent, WaitForSingleObject, INFINITE,
};
use winit::event_loop::EventLoopProxy;

const MUTEX_NAME: &str = r"Local\com.animaengine.Anima";
const RAISE_EVENT_NAME: &str = r"Local\com.animaengine.Anima.raise";
const ACK_EVENT_NAME: &str = r"Local\com.animaengine.Anima.raised";

/// How long a launch keeps trying when the running instance does not
/// acknowledge — the same bound as the D-Bus handshake's.
const HANDOFF_PATIENCE: Duration = Duration::from_secs(5);
/// How long one raise gets to be acknowledged. A live instance answers
/// in milliseconds.
const ACK_TIMEOUT: Duration = Duration::from_secs(1);
/// Pause between attempts.
const RETRY: Duration = Duration::from_millis(100);

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
    ack: HANDLE,
}

// Kernel object handles are process-wide, not thread-affine.
unsafe impl Send for Instance {}

/// Claim the single instance, or hand off to the one already running.
///
/// If the kernel objects cannot be created at all, this process runs
/// anyway: a second overlay is a much smaller failure than none.
pub fn try_acquire() -> Option<Acquire> {
    try_acquire_named(
        &Names {
            mutex: MUTEX_NAME,
            raise: RAISE_EVENT_NAME,
            ack: ACK_EVENT_NAME,
        },
        HANDOFF_PATIENCE,
    )
}

/// The three kernel object names. Parameterised so tests never share
/// them with a running overlay or with each other.
struct Names<'a> {
    mutex: &'a str,
    raise: &'a str,
    ack: &'a str,
}

fn try_acquire_named(names: &Names<'_>, patience: Duration) -> Option<Acquire> {
    let mutex_name = wide(names.mutex);
    let raise_name = wide(names.raise);
    let ack_name = wide(names.ack);
    let deadline = Instant::now() + patience;
    // SAFETY: plain Win32 calls with valid, NUL-terminated names and null
    // security attributes; each returned handle is checked before use and
    // closed on every path that does not hand it to `Instance`.
    unsafe {
        // Auto-reset, initially clear: one signal wakes the listener once,
        // and one acknowledgement answers one launch.
        let raise = CreateEventW(std::ptr::null(), 0, 0, raise_name.as_ptr());
        if raise == 0 {
            tracing::warn!("Single-instance event unavailable ({})", GetLastError());
            return None;
        }
        let ack = CreateEventW(std::ptr::null(), 0, 0, ack_name.as_ptr());
        if ack == 0 {
            tracing::warn!("Single-instance event unavailable ({})", GetLastError());
            CloseHandle(raise);
            return None;
        }
        loop {
            let mutex = CreateMutexW(std::ptr::null(), 0, mutex_name.as_ptr());
            let already_running = GetLastError() == ERROR_ALREADY_EXISTS;
            if mutex == 0 {
                tracing::warn!("Single-instance mutex unavailable ({})", GetLastError());
                CloseHandle(raise);
                CloseHandle(ack);
                return None;
            }
            if !already_running {
                // A raise left signalled by a launch that gave up on the
                // previous owner is not meant for this instance.
                ResetEvent(raise);
                return Some(Acquire::Claimed(Instance { mutex, raise, ack }));
            }
            // A stale acknowledgement must not answer this launch.
            ResetEvent(ack);
            SetEvent(raise);
            let answered = WaitForSingleObject(ack, millis(ACK_TIMEOUT)) == WAIT_OBJECT_0;
            // Our handle keeps the mutex alive too: let go of it before
            // waiting, or an owner that exits never takes it away.
            CloseHandle(mutex);
            if answered {
                CloseHandle(raise);
                CloseHandle(ack);
                return Some(Acquire::HandedOff);
            }
            if Instant::now() >= deadline {
                // The owner never answered. Exit anyway: it is running,
                // and a second overlay on top of it is the worse outcome.
                tracing::warn!("The running instance did not answer; not starting a second one");
                CloseHandle(raise);
                CloseHandle(ack);
                return Some(Acquire::HandedOff);
            }
            std::thread::sleep(RETRY);
        }
    }
}

fn millis(d: Duration) -> u32 {
    u32::try_from(d.as_millis()).unwrap_or(u32::MAX)
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
                        // No acknowledgement, so the launch waits for the
                        // mutex instead of trusting a hand-off to nobody.
                        return;
                    }
                    instance.acknowledge();
                }
            });
        if let Err(e) = spawned {
            tracing::warn!("Single-instance listener failed to start: {e}");
        }
    }
}

impl Instance {
    /// Tell the launch that signalled that its raise was taken.
    fn acknowledge(&self) {
        // SAFETY: `ack` is a live event handle owned by `self`.
        unsafe {
            SetEvent(self.ack);
        }
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        // SAFETY: the handles were returned non-null by Create* and are
        // closed exactly once, here.
        unsafe {
            CloseHandle(self.ack);
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
    struct TestNames {
        mutex: String,
        raise: String,
        ack: String,
    }

    impl TestNames {
        fn new(tag: &str) -> Self {
            let base = format!(r"Local\anima-test-{}-{tag}", std::process::id());
            Self {
                raise: format!("{base}.raise"),
                ack: format!("{base}.raised"),
                mutex: base,
            }
        }

        fn names(&self) -> Names<'_> {
            Names {
                mutex: &self.mutex,
                raise: &self.raise,
                ack: &self.ack,
            }
        }
    }

    /// Stands in for `Instance::listen`, which needs a winit event loop:
    /// answers every raise, as the real listener does.
    fn answer_raises(instance: &Instance) -> impl Fn() -> bool + '_ {
        move || {
            // SAFETY: `raise` is live for as long as `instance` is.
            let woke = unsafe { WaitForSingleObject(instance.raise, 2_000) };
            if woke == WAIT_OBJECT_0 {
                instance.acknowledge();
            }
            woke == WAIT_OBJECT_0
        }
    }

    const QUICK: Duration = Duration::from_millis(300);

    #[test]
    fn the_first_launch_claims() {
        let n = TestNames::new("claim");
        assert!(matches!(
            try_acquire_named(&n.names(), QUICK),
            Some(Acquire::Claimed(_))
        ));
    }

    /// The ordinary second launch: the running instance answers, and the
    /// launch hands off at once.
    #[test]
    fn an_answered_launch_hands_off() {
        let n = TestNames::new("answered");
        let Some(Acquire::Claimed(first)) = try_acquire_named(&n.names(), QUICK) else {
            panic!("the first launch must claim");
        };
        std::thread::scope(|s| {
            let listener = s.spawn(|| answer_raises(&first)());
            let started = Instant::now();
            assert!(matches!(
                try_acquire_named(&n.names(), HANDOFF_PATIENCE),
                Some(Acquire::HandedOff)
            ));
            assert!(
                started.elapsed() < ACK_TIMEOUT,
                "an answered hand-off waited"
            );
            assert!(listener.join().unwrap(), "the raise was not signalled");
        });
    }

    /// An instance that holds the mutex but never answers — frozen, or
    /// exiting without a listener — is given up on, not doubled.
    #[test]
    fn an_unanswered_launch_gives_up_and_does_not_start() {
        let n = TestNames::new("silent");
        let Some(Acquire::Claimed(_first)) = try_acquire_named(&n.names(), QUICK) else {
            panic!("the first launch must claim");
        };
        assert!(matches!(
            try_acquire_named(&n.names(), QUICK),
            Some(Acquire::HandedOff)
        ));
    }

    /// R48: the instance a launch finds exits while it waits. The launch
    /// must take over instead of exiting with it.
    #[test]
    fn a_launch_takes_over_from_an_instance_that_exits() {
        let n = TestNames::new("exiting");
        let Some(Acquire::Claimed(first)) = try_acquire_named(&n.names(), QUICK) else {
            panic!("the first launch must claim");
        };
        std::thread::scope(|s| {
            s.spawn(move || {
                std::thread::sleep(Duration::from_millis(1_500));
                drop(first);
            });
            assert!(matches!(
                try_acquire_named(&n.names(), HANDOFF_PATIENCE),
                Some(Acquire::Claimed(_))
            ));
        });
    }

    /// Once the instance is gone, the next launch claims afresh.
    #[test]
    fn the_claim_ends_with_the_instance() {
        let n = TestNames::new("release");
        drop(try_acquire_named(&n.names(), QUICK));
        assert!(matches!(
            try_acquire_named(&n.names(), QUICK),
            Some(Acquire::Claimed(_))
        ));
    }

    #[test]
    fn names_are_nul_terminated() {
        assert_eq!(wide("ab"), vec![97, 98, 0]);
    }
}
