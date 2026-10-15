//! The desktop's clipboard on the X11 path (`crate::system_clipboard`).
//!
//! winit gives the overlay's windows no say in selections, so the
//! clipboard gets a window of its own — unmapped, on a connection of its
//! own, served by one thread that starts at the first copy or paste.
//! After a copy here it owns CLIPBOARD and answers requests with the
//! characters' names; a paste asks the owner for its types (`TARGETS`),
//! then for the one chosen, taking a big answer in pieces (`INCR`), capped.
//! Under a Wayland session XWayland carries all of it across.

use crate::system_clipboard::{self as sc, Content, Pick};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use x11rb::connection::Connection as _;
use x11rb::protocol::xproto::{
    Atom, AtomEnum, ConnectionExt as _, CreateWindowAux, EventMask, PropMode, Property,
    SelectionNotifyEvent, SelectionRequestEvent, Window, WindowClass, SELECTION_NOTIFY_EVENT,
};
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

/// How long a clipboard owner gets to answer, each step of the way.
const ANSWER_WITHIN: Duration = Duration::from_secs(2);

/// How long an answer in pieces (`INCR`) may take, all of them.
const INCR_WITHIN: Duration = Duration::from_secs(10);

/// What the clipboard's owner answered.
enum Answer {
    /// What it sent, at most the image cap.
    Bytes(Vec<u8>),
    /// More than the image cap; none of it kept.
    TooBig,
    /// Refused, or not in time.
    Nothing,
}

enum Cmd {
    Claim(String, bool),
    Read(sc::Want, mpsc::Sender<Content>),
}

/// The clipboard thread, from the X11 path's side.
pub struct X11Clipboard {
    tx: mpsc::Sender<Cmd>,
    /// Written to after each command: the thread sleeps in `poll` on the
    /// X connection and the other end of this.
    wake: OwnedFd,
}

impl X11Clipboard {
    /// Connect and start the thread; `None` without an X server.
    pub fn start() -> Option<Self> {
        let (conn, screen) = match x11rb::connect(None) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Clipboard: no X connection: {e}");
                return None;
            }
        };
        let mut fds = [0; 2];
        // SAFETY: `fds` has room for the two descriptors pipe2 writes.
        if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) } != 0 {
            return None;
        }
        // SAFETY: pipe2 succeeded, so both are open descriptors we own.
        let (read_end, write_end) =
            unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("anima-clipboard".into())
            .spawn(move || match Selections::new(conn, screen) {
                Ok(mut s) => s.run(&rx, &read_end),
                Err(e) => tracing::warn!("Clipboard: setting up failed: {e}"),
            });
        if let Err(e) = spawned {
            tracing::warn!("Clipboard thread failed to start: {e}");
            return None;
        }
        Some(Self {
            tx,
            wake: write_end,
        })
    }

    /// Put `text` on the clipboard: the names of the characters just
    /// copied (`ours`, so our own type goes with it), or text copied in a
    /// text field of the panel.
    pub fn claim(&self, text: String, ours: bool) {
        self.send(Cmd::Claim(text, ours));
    }

    /// Ask what the clipboard holds for `want`; the answer comes on the
    /// receiver.
    pub fn read(&self, want: sc::Want) -> mpsc::Receiver<Content> {
        let (tx, rx) = mpsc::channel();
        self.send(Cmd::Read(want, tx));
        rx
    }

    fn send(&self, cmd: Cmd) {
        if self.tx.send(cmd).is_ok() {
            // SAFETY: writing one byte from a live buffer to our own pipe.
            unsafe {
                libc::write(self.wake.as_raw_fd(), [1u8].as_ptr().cast(), 1);
            }
        }
    }
}

type Failure = Box<dyn std::error::Error>;

struct Selections {
    conn: RustConnection,
    window: Window,
    atoms: Atoms,
    /// What we put on CLIPBOARD while we own it, and whether it is the
    /// names of characters copied here (our own type goes with them).
    claim: Option<(String, bool)>,
}

struct Atoms {
    clipboard: Atom,
    targets: Atom,
    incr: Atom,
    /// Where answers to our requests are put, on our window.
    landing: Atom,
    /// The types we answer with the names: ours first, then plain text.
    text: Vec<Atom>,
}

impl Selections {
    fn new(conn: RustConnection, screen: usize) -> Result<Self, Failure> {
        let root = conn.setup().roots[screen].root;
        let window = conn.generate_id()?;
        conn.create_window(
            x11rb::COPY_DEPTH_FROM_PARENT,
            window,
            root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::INPUT_OUTPUT,
            x11rb::COPY_FROM_PARENT,
            &CreateWindowAux::new().event_mask(EventMask::PROPERTY_CHANGE),
        )?;
        let intern = |name: &str| -> Result<Atom, Failure> {
            Ok(conn.intern_atom(false, name.as_bytes())?.reply()?.atom)
        };
        let utf8 = intern("UTF8_STRING")?;
        let ours = intern(sc::ANIMA_MIME)?;
        let mut text = vec![ours, utf8];
        for t in sc::TEXT_MIMES {
            let atom = intern(t)?;
            if !text.contains(&atom) {
                text.push(atom);
            }
        }
        text.push(AtomEnum::STRING.into());
        let atoms = Atoms {
            clipboard: intern("CLIPBOARD")?,
            targets: intern("TARGETS")?,
            incr: intern("INCR")?,
            landing: intern("ANIMA_CLIPBOARD")?,
            text,
        };
        conn.flush()?;
        Ok(Self {
            conn,
            window,
            atoms,
            claim: None,
        })
    }

    fn run(&mut self, rx: &mpsc::Receiver<Cmd>, wake: &OwnedFd) {
        loop {
            loop {
                match rx.try_recv() {
                    Ok(Cmd::Claim(text, ours)) => self.take_ownership(text, ours),
                    Ok(Cmd::Read(want, reply)) => {
                        let content = self
                            .read(want)
                            .unwrap_or_else(|e| Content::Failed(e.to_string()));
                        let _ = reply.send(content);
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => return,
                }
            }
            if let Err(e) = self.serve_pending() {
                tracing::warn!("Clipboard: X connection lost: {e}");
                return;
            }
            // Sleep until the server or a command wants us.
            let mut fds = [
                libc::pollfd {
                    fd: self.conn.stream().as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                },
                libc::pollfd {
                    fd: wake.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                },
            ];
            // SAFETY: two valid pollfd entries for the duration of the call.
            unsafe { libc::poll(fds.as_mut_ptr(), 2, -1) };
            let mut drain = [0u8; 64];
            // SAFETY: reading into a live buffer from our own non-blocking pipe.
            while unsafe { libc::read(wake.as_raw_fd(), drain.as_mut_ptr().cast(), drain.len()) }
                > 0
            {}
            if fds[1].revents & libc::POLLHUP != 0 && rx.try_recv().is_err() {
                // The app side is gone.
                return;
            }
        }
    }

    /// Handle every event already in, then flush.
    fn serve_pending(&mut self) -> Result<(), Failure> {
        while let Some(event) = self.conn.poll_for_event()? {
            self.handle(event)?;
        }
        self.conn.flush()?;
        Ok(())
    }

    fn handle(&mut self, event: Event) -> Result<(), Failure> {
        match event {
            Event::SelectionRequest(request) => self.answer(&request)?,
            Event::SelectionClear(clear) if clear.selection == self.atoms.clipboard => {
                // Copied elsewhere: no longer ours.
                self.claim = None;
            }
            _ => {}
        }
        Ok(())
    }

    fn take_ownership(&mut self, text: String, ours: bool) {
        let owned = self
            .conn
            .set_selection_owner(self.window, self.atoms.clipboard, x11rb::CURRENT_TIME)
            .ok()
            .and_then(|_| self.conn.get_selection_owner(self.atoms.clipboard).ok())
            .and_then(|c| c.reply().ok())
            .is_some_and(|r| r.owner == self.window);
        self.claim = owned.then_some((text, ours));
        if !owned {
            tracing::warn!("Clipboard: could not take CLIPBOARD");
        }
    }

    /// Answer a request for what we put on CLIPBOARD: our types, or the
    /// names; anything else is refused.
    fn answer(&self, request: &SelectionRequestEvent) -> Result<(), Failure> {
        // ICCCM: a request with no property is an old client's; use the
        // target as the property.
        let property = if request.property == x11rb::NONE {
            request.target
        } else {
            request.property
        };
        let mut answered = x11rb::NONE;
        if let (Some((text, ours)), true) = (&self.claim, request.selection == self.atoms.clipboard)
        {
            // Our own type (first in the list) only with characters copied
            // here.
            let offered: Vec<Atom> = self
                .atoms
                .text
                .iter()
                .copied()
                .enumerate()
                .filter(|&(i, _)| *ours || i > 0)
                .map(|(_, a)| a)
                .collect();
            if request.target == self.atoms.targets {
                let mut types = vec![self.atoms.targets];
                types.extend(&offered);
                self.conn.change_property32(
                    PropMode::REPLACE,
                    request.requestor,
                    property,
                    AtomEnum::ATOM,
                    &types,
                )?;
                answered = property;
            } else if offered.contains(&request.target) {
                self.conn.change_property8(
                    PropMode::REPLACE,
                    request.requestor,
                    property,
                    request.target,
                    text.as_bytes(),
                )?;
                answered = property;
            }
        }
        self.conn.send_event(
            false,
            request.requestor,
            EventMask::NO_EVENT,
            SelectionNotifyEvent {
                response_type: SELECTION_NOTIFY_EVENT,
                sequence: 0,
                time: request.time,
                requestor: request.requestor,
                selection: request.selection,
                target: request.target,
                property: answered,
            },
        )?;
        Ok(())
    }

    /// What CLIPBOARD holds, as far as a paste for `want` goes.
    fn read(&mut self, want: sc::Want) -> Result<Content, Failure> {
        let owner = self
            .conn
            .get_selection_owner(self.atoms.clipboard)?
            .reply()?
            .owner;
        if owner == x11rb::NONE {
            return Ok(Content::Nothing);
        }
        if owner == self.window {
            // Ours: no need to ask ourselves.
            return Ok(match (&self.claim, want) {
                (Some((text, _)), sc::Want::Text) => Content::Text(text.clone()),
                (Some((_, true)), sc::Want::Characters) => Content::Ours,
                _ => Content::Nothing,
            });
        }
        // Which types it offers — a list too long to be one is no answer.
        let Answer::Bytes(types) = self.convert(self.atoms.targets)? else {
            return Ok(Content::Nothing);
        };
        let atoms: Vec<Atom> = types
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| u32::from_ne_bytes(*b))
            .take(256)
            .collect();
        let cookies: Vec<_> = atoms
            .iter()
            .map(|&a| self.conn.get_atom_name(a))
            .collect::<Result<_, _>>()?;
        let mut named: Vec<(Atom, String)> = Vec::new();
        for (atom, cookie) in atoms.iter().zip(cookies) {
            if let Ok(reply) = cookie.reply() {
                named.push((*atom, String::from_utf8_lossy(&reply.name).into_owned()));
            }
        }
        let names: Vec<String> = named.iter().map(|(_, n)| n.clone()).collect();
        let pick = sc::pick_for(want, &names);
        let wanted = match &pick {
            Pick::Ours => return Ok(Content::Ours),
            Pick::Nothing => return Ok(Content::Nothing),
            Pick::Image(mime, _) | Pick::Files(mime) | Pick::Text(mime) => mime.clone(),
        };
        let Some(&(target, _)) = named.iter().find(|(_, n)| *n == wanted) else {
            return Ok(Content::Nothing);
        };
        let too_big = || Content::Failed(format!("over {} MB", sc::cap_for(&pick) / (1024 * 1024)));
        match self.convert(target)? {
            Answer::TooBig => Ok(too_big()),
            Answer::Bytes(bytes) if bytes.len() as u64 > sc::cap_for(&pick) => Ok(too_big()),
            Answer::Bytes(bytes) => Ok(Content::from_bytes(&pick, bytes)),
            Answer::Nothing => Ok(Content::Failed(
                "the clipboard's owner did not answer".into(),
            )),
        }
    }

    /// Ask CLIPBOARD's owner for `target` and read the answer off our
    /// window, in pieces if it comes that way. Capped at the image cap,
    /// and all of it within [`INCR_WITHIN`].
    fn convert(&mut self, target: Atom) -> Result<Answer, Failure> {
        self.conn.convert_selection(
            self.window,
            self.atoms.clipboard,
            target,
            self.atoms.landing,
            x11rb::CURRENT_TIME,
        )?;
        self.conn.flush()?;
        let deadline = Instant::now() + ANSWER_WITHIN;
        let window = self.window;
        let notified = self.wait_for(deadline, |e| {
            matches!(e, Event::SelectionNotify(n) if n.requestor == window && n.target == target)
        })?;
        let Some(Event::SelectionNotify(n)) = notified else {
            return Ok(Answer::Nothing);
        };
        if n.property == x11rb::NONE {
            return Ok(Answer::Nothing);
        }
        let cap = sc::MAX_IMAGE_BYTES;
        let words = u32::try_from(cap / 4 + 1).unwrap_or(u32::MAX);
        let first = self
            .conn
            .get_property(
                true,
                self.window,
                self.atoms.landing,
                AtomEnum::ANY,
                0,
                words,
            )?
            .reply()?;
        if first.type_ != self.atoms.incr {
            // More than the cap waits behind what was read: nothing is
            // kept. (It used to stand for that with a buffer of the cap's
            // size, 64 MB of zeros, made only to be measured.)
            if first.bytes_after > 0 {
                return Ok(Answer::TooBig);
            }
            return Ok(Answer::Bytes(first.value));
        }
        // INCR: the owner writes piece after piece, each once we have
        // deleted the last, and an empty piece at the end — all of them
        // within INCR_WITHIN: an owner dripping small pieces, each in
        // time, kept this thread reading for as long as it liked.
        self.conn.flush()?;
        let mut bytes = Vec::new();
        let all_by = Instant::now() + INCR_WITHIN;
        loop {
            let deadline = (Instant::now() + ANSWER_WITHIN).min(all_by);
            let landing = self.atoms.landing;
            let piece = self.wait_for(deadline, |e| {
                matches!(e, Event::PropertyNotify(p) if p.atom == landing && p.state == Property::NEW_VALUE)
            })?;
            if piece.is_none() {
                return Ok(Answer::Nothing);
            }
            let reply = self
                .conn
                .get_property(true, self.window, landing, AtomEnum::ANY, 0, words)?
                .reply()?;
            self.conn.flush()?;
            if reply.value.is_empty() {
                return Ok(Answer::Bytes(bytes));
            }
            if (bytes.len() + reply.value.len()) as u64 > cap {
                return Ok(Answer::TooBig);
            }
            bytes.extend_from_slice(&reply.value);
        }
    }

    /// The first event matching `wanted` before `deadline`, handling the
    /// others meanwhile — a request for what we own may come in between.
    fn wait_for(
        &mut self,
        deadline: Instant,
        wanted: impl Fn(&Event) -> bool,
    ) -> Result<Option<Event>, Failure> {
        loop {
            while let Some(event) = self.conn.poll_for_event()? {
                if wanted(&event) {
                    return Ok(Some(event));
                }
                self.handle(event)?;
            }
            self.conn.flush()?;
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Ok(None);
            }
            let mut fd = libc::pollfd {
                fd: self.conn.stream().as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            let ms = i32::try_from(left.as_millis()).unwrap_or(i32::MAX).max(1);
            // SAFETY: one valid pollfd for the duration of the call.
            unsafe { libc::poll(&mut fd, 1, ms) };
        }
    }
}
