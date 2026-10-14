//! Quitting on the signals a session sends (1.5): SIGTERM at logout or
//! shutdown, SIGINT from a terminal's Ctrl+C, SIGHUP when the terminal
//! goes. Each used to end the process where it stood, so whatever had not
//! been saved yet — edits still in edit mode, the tour just finished,
//! where the walkers had got to, a pause from the tray — was lost. Now the
//! first one asks the loop to quit as the tray's Quit does, which saves;
//! a second ends the process at once, for a loop that does not answer.
//!
//! The signals are blocked in every thread — [`start`] runs first thing
//! in `main`, before any other thread is made, so all inherit the mask —
//! and taken by one thread of its own with `sigwait`: nothing runs in a
//! signal handler, so nothing has to be async-signal-safe. A child the app
//! starts gets an empty mask from `std::process::Command`. Until the loop
//! is there to ask ([`connect`]), a signal ends the process as before.

use crate::event::{AnimaEvent, EventSink};
use std::sync::OnceLock;

/// The signals taken.
const SIGNALS: [libc::c_int; 3] = [libc::SIGTERM, libc::SIGINT, libc::SIGHUP];

/// Where a signal's Quit goes, once the loop exists.
static SINK: OnceLock<EventSink> = OnceLock::new();

fn signal_set() -> libc::sigset_t {
    // SAFETY: `sigemptyset` initialises the set before `sigaddset` reads
    // it; both only write the set they are given.
    unsafe {
        let mut set: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut set);
        for signal in SIGNALS {
            libc::sigaddset(&mut set, signal);
        }
        set
    }
}

/// Block the signals in this thread, and so in every thread made after,
/// and start the thread that takes them. Call before anything else in
/// `main`.
pub fn start() {
    let set = signal_set();
    // SAFETY: `set` is initialised; the old mask is not asked for.
    let blocked = unsafe { libc::pthread_sigmask(libc::SIG_BLOCK, &set, std::ptr::null_mut()) };
    if blocked != 0 {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("anima-signals".into())
        .spawn(move || wait(set));
    if spawned.is_err() {
        // No one to take them: let them do what they did before.
        // SAFETY: as above.
        unsafe { libc::pthread_sigmask(libc::SIG_UNBLOCK, &set, std::ptr::null_mut()) };
    }
}

/// From now on a signal asks the loop behind `sink` to quit.
pub fn connect(sink: EventSink) {
    let _ = SINK.set(sink);
}

fn wait(set: libc::sigset_t) {
    let mut asked = false;
    loop {
        let mut signal: libc::c_int = 0;
        // SAFETY: `set` is initialised and `signal` a valid place to write.
        if unsafe { libc::sigwait(&set, &mut signal) } != 0 {
            continue;
        }
        if asked {
            tracing::warn!("Signal {signal} again: exiting now, without saving");
            std::process::exit(128 + signal);
        }
        match SINK.get() {
            Some(sink) if sink.send(AnimaEvent::Quit) => {
                asked = true;
                tracing::info!("Signal {signal}: quitting, saving first");
            }
            // Before the loop is there, or with it gone: as before.
            _ => std::process::exit(128 + signal),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_session_signals_are_the_ones_taken() {
        let set = signal_set();
        for signal in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP] {
            // SAFETY: `set` is initialised.
            assert_eq!(unsafe { libc::sigismember(&set, signal) }, 1);
        }
        // SAFETY: as above.
        assert_eq!(unsafe { libc::sigismember(&set, libc::SIGKILL) }, 0);
    }
}
