//! Waking the native Wayland loop from another thread.
//!
//! The loop sleeps in `poll` on the Wayland socket until the next thing it
//! has to do — a frame, a sprite's next picture, the heartbeat. What other
//! threads hand it — a D-Bus call, the tray, a portal shortcut, a screen
//! reader — goes through channels a `poll` cannot see, so each sender
//! also writes a byte here, and the loop polls this socket beside
//! Wayland's. It used to wake every 16 ms to look instead: sixty times a
//! second, all day, with nothing on screen to draw.

use std::io::{Read, Write};
use std::os::fd::{AsFd, BorrowedFd};
use std::os::unix::net::UnixStream;
use std::sync::OnceLock;

/// The sending end, for [`wake`] from any thread.
static SENDER: OnceLock<UnixStream> = OnceLock::new();

/// The loop's end: readable once someone has woken it.
pub struct Waker {
    rx: UnixStream,
}

impl Waker {
    /// Make the pair and keep the sending end for [`wake`]. `None` when no
    /// socket can be made, or one already was — there is one loop to a
    /// process; the loop then looks at its channels as often as it wakes
    /// anyway.
    pub fn install() -> Option<Self> {
        let (tx, rx) = UnixStream::pair().ok()?;
        tx.set_nonblocking(true).ok()?;
        rx.set_nonblocking(true).ok()?;
        SENDER.set(tx).ok()?;
        Some(Self { rx })
    }

    /// What to poll.
    pub fn fd(&self) -> BorrowedFd<'_> {
        self.rx.as_fd()
    }

    /// Take what woke the loop, so the next poll sleeps again.
    pub fn drain(&self) {
        let mut buf = [0u8; 64];
        while matches!((&self.rx).read(&mut buf), Ok(n) if n > 0) {}
    }
}

/// Wake the loop, if there is one. Never blocks: a socket too full to
/// take another byte has the loop's attention already.
pub fn wake() {
    if let Some(tx) = SENDER.get() {
        let _ = (&*tx).write(&[1]);
    }
}
