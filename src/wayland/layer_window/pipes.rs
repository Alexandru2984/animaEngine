//! Reading and writing the pipes other clients hand us — a paste, a drop,
//! a request for what we copied — with a deadline (SEC-008).
//!
//! Each pipe's other end is another client's, and nothing said it would
//! read or write: a request for our text whose reader never read blocked
//! the main loop in `write` for good, and a drop or paste whose source
//! never wrote kept its reader thread waiting for good — four such drops
//! and no drop was taken again. Here the pipe is made non-blocking and
//! waited on with `poll`, so either end giving up costs a deadline, not a
//! thread or the app.

use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::time::{Duration, Instant};

/// How long a paste or a drop may take to arrive: X11's paste allows as
/// much (`x11_clipboard::INCR_WITHIN`).
pub(super) const READ_WITHIN: Duration = Duration::from_secs(10);

/// How long a client asking for our copy may take to take it.
pub(super) const WRITE_WITHIN: Duration = Duration::from_secs(5);

fn set_nonblocking(fd: i32) -> io::Result<()> {
    // SAFETY: `fcntl` on a descriptor the caller owns, reading then
    // setting its status flags; nothing is dereferenced.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: as above.
    if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Wait until `fd` is ready for `events`, or `deadline`.
fn wait(fd: i32, events: libc::c_short, deadline: Instant) -> io::Result<()> {
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "the other end took too long",
            ));
        }
        let mut p = libc::pollfd {
            fd,
            events,
            revents: 0,
        };
        let ms = i32::try_from(left.as_millis()).unwrap_or(i32::MAX).max(1);
        // SAFETY: one valid pollfd for the duration of the call.
        let n = unsafe { libc::poll(&mut p, 1, ms) };
        if n > 0 {
            return Ok(());
        }
        if n < 0 {
            let e = io::Error::last_os_error();
            if e.kind() != io::ErrorKind::Interrupted {
                return Err(e);
            }
        }
    }
}

/// Read `pipe` to its end, at most `cap + 1` bytes — one past the cap,
/// so the caller can tell too much from just enough — all within
/// `within`.
pub(super) fn read_within(
    pipe: &mut (impl Read + AsRawFd),
    cap: u64,
    within: Duration,
) -> io::Result<Vec<u8>> {
    let fd = pipe.as_raw_fd();
    set_nonblocking(fd)?;
    let deadline = Instant::now() + within;
    let limit = usize::try_from(cap.saturating_add(1)).unwrap_or(usize::MAX);
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 16 * 1024];
    while bytes.len() < limit {
        let want = chunk.len().min(limit - bytes.len());
        match pipe.read(&mut chunk[..want]) {
            Ok(0) => break,
            Ok(n) => bytes.extend_from_slice(&chunk[..n]),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => wait(fd, libc::POLLIN, deadline)?,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(bytes)
}

/// Write all of `bytes` to `pipe` within `within`.
pub(super) fn write_all_within(
    pipe: &mut (impl Write + AsRawFd),
    bytes: &[u8],
    within: Duration,
) -> io::Result<()> {
    let fd = pipe.as_raw_fd();
    set_nonblocking(fd)?;
    let deadline = Instant::now() + within;
    let mut left = bytes;
    while !left.is_empty() {
        match pipe.write(left) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "the pipe took nothing",
                ))
            }
            Ok(n) => left = &left[n..],
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => wait(fd, libc::POLLOUT, deadline)?,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::fd::FromRawFd;

    fn pipe() -> (std::fs::File, std::fs::File) {
        let mut fds = [0; 2];
        // SAFETY: `fds` has room for the two descriptors `pipe` writes.
        assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
        // SAFETY: both descriptors were just made and are owned by nobody
        // else.
        unsafe {
            (
                std::fs::File::from_raw_fd(fds[0]),
                std::fs::File::from_raw_fd(fds[1]),
            )
        }
    }

    /// A source that never writes and never closes: the read gives up at
    /// its deadline instead of waiting for good (SEC-008).
    #[test]
    fn a_read_nobody_writes_ends_at_its_deadline() {
        let (mut r, _w) = pipe();
        let started = Instant::now();
        let e = read_within(&mut r, 1024, Duration::from_millis(200)).unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    /// A reader that never reads: the write gives up at its deadline —
    /// it blocked the main loop for good (SEC-008).
    #[test]
    fn a_write_nobody_reads_ends_at_its_deadline() {
        let (_r, mut w) = pipe();
        let started = Instant::now();
        let big = vec![b'x'; 1024 * 1024];
        let e = write_all_within(&mut w, &big, Duration::from_millis(200)).unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn what_comes_whole_is_read_whole_and_capped() {
        let (mut r, mut w) = pipe();
        write_all_within(&mut w, b"file:///a.png\n", WRITE_WITHIN).unwrap();
        drop(w);
        assert_eq!(
            read_within(&mut r, 1024, READ_WITHIN).unwrap(),
            b"file:///a.png\n"
        );
        let (mut r, mut w) = pipe();
        write_all_within(&mut w, &[7u8; 100], WRITE_WITHIN).unwrap();
        drop(w);
        assert_eq!(
            read_within(&mut r, 10, READ_WITHIN).unwrap().len(),
            11,
            "one past the cap"
        );
    }
}
