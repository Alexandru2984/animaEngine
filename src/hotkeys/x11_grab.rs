//! Global shortcuts on X11: grabbed on the root window (`XGrabKey`) and
//! waited for. The thread sleeps in the X server's socket until one is
//! pressed, the keyboard's layout changes, or the app's shortcuts do
//! ([`follow`]: a pipe wakes it, and it grabs them anew).
//!
//! The `global-hotkey` crate did this on unix before 1.5, and polled its
//! connection every 50 ms in between — twenty wake-ups a second, all day,
//! for keys pressed a few times an hour. It stays for Windows, where the
//! system delivers them.

use crate::keybindings::{Action, KeyChord, KeyCode, ModifierMask, NamedKey, SymbolKey};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::sync::{Mutex, OnceLock};
use x11rb::connection::{Connection, RequestConnection};
use x11rb::protocol::xproto::{
    ConnectionExt, GrabMode, KeyButMask, Keycode, Mapping, ModMask, Window,
};
use x11rb::protocol::{ErrorKind, Event};
use x11rb::rust_connection::RustConnection;

/// The way a change to the shortcuts reaches the grabber's thread: the
/// chords it was last told, the ones to take next, and the pipe's end that
/// wakes it. Set once a grabber serves; one grabber a process.
struct Follow {
    told: Mutex<Vec<(KeyChord, Action)>>,
    next: Mutex<Option<Vec<(KeyChord, Action)>>>,
    wake: OwnedFd,
}

static FOLLOW: OnceLock<Follow> = OnceLock::new();

/// Whether a grabber serves, so that [`follow`] has someone to tell.
pub fn serving() -> bool {
    FOLLOW.get().is_some()
}

/// Grab `chords` from now on instead, unless they are the ones the grabber
/// was last told: a change in the Keybindings tab, or a hot reload of the
/// config. They were grabbed once, at start, and a changed shortcut did
/// nothing until a restart (BUG-016). Cheap when nothing changed: the
/// winit loop asks every frame (`hotkeys::follow`).
pub fn follow(chords: Vec<(KeyChord, Action)>) {
    let Some(f) = FOLLOW.get() else {
        return;
    };
    let Ok(mut told) = f.told.lock() else {
        return;
    };
    if *told == chords {
        return;
    }
    told.clone_from(&chords);
    if let Ok(mut next) = f.next.lock() {
        *next = Some(chords);
    }
    // One byte wakes it; with the pipe full, it is awake already.
    // SAFETY: one byte from a live local, to the pipe's write end, which
    // `FOLLOW` owns for the life of the process.
    let _ = unsafe { libc::write(f.wake.as_raw_fd(), [1u8].as_ptr().cast(), 1) };
}

/// A pipe, both ends kept from children and the read end not blocking:
/// (read, write).
fn wake_pipe() -> Option<(OwnedFd, OwnedFd)> {
    let mut fds = [0; 2];
    // SAFETY: `pipe` fills the two-int array it is given.
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        return None;
    }
    // SAFETY: both descriptors were just made, and are owned from here.
    let (read, write) = unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };
    for fd in [&read, &write] {
        // SAFETY: fcntl on a descriptor we own.
        unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) };
    }
    // SAFETY: as above. The write end may block: a byte a change, each
    // read at once, never fills it.
    unsafe { libc::fcntl(read.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK) };
    Some((read, write))
}

/// The modifiers a chord is told apart by. X11 counts NumLock (`M2`) and
/// CapsLock (`LOCK`) as modifiers too, so each chord is grabbed with every
/// combination of those as well, and they are left out when reading a
/// press.
const CHORD_MODS: [ModMask; 4] = [ModMask::CONTROL, ModMask::SHIFT, ModMask::M1, ModMask::M4];

/// No lock, NumLock, CapsLock, both.
fn locks() -> [ModMask; 4] {
    [
        ModMask::default(),
        ModMask::M2,
        ModMask::LOCK,
        ModMask::M2 | ModMask::LOCK,
    ]
}

/// One chord as grabbed: which key, with which modifiers, and whether it
/// is down now — a held key fires once.
struct Grab {
    keycode: Keycode,
    mods: ModMask,
    action: Action,
    chord: KeyChord,
    down: bool,
}

/// The shortcuts grabbed, on their own connection.
pub struct Grabber {
    conn: RustConnection,
    root: Window,
    chords: Vec<(KeyChord, Action)>,
    grabs: Vec<Grab>,
}

impl Grabber {
    /// Connect and grab `chords`. `None` without an X server, or when not
    /// one could be grabbed — each taken by another program, or a key this
    /// keyboard does not have.
    pub fn new(chords: Vec<(KeyChord, Action)>) -> Option<Self> {
        let (conn, screen) = x11rb::connect(None).ok()?;
        let root = conn.setup().roots[screen].root;
        // A key held down repeats as presses alone, not press and release
        // pairs, so a held chord fires once. Without XKB it fires as the
        // key repeats; nothing else is lost.
        use_xkb(&conn);
        let mut grabber = Self {
            conn,
            root,
            chords,
            grabs: Vec::new(),
        };
        grabber.grab_all(true);
        (!grabber.grabs.is_empty()).then_some(grabber)
    }

    /// Grab every chord on the keys the keyboard has now — `first`, at
    /// start, said in the log; again for a new layout, which XKB tells
    /// several times over, only when debugging.
    fn grab_all(&mut self, first: bool) {
        let map = KeyMap::read(&self.conn);
        for &(chord, action) in &self.chords {
            let (Some(keysym), Some(map)) = (keysym(chord.key), map.as_ref()) else {
                tracing::debug!("Global shortcut {chord} has no X11 key; skipped");
                continue;
            };
            let Some(keycode) = map.keycode(keysym) else {
                tracing::warn!("Global shortcut {chord}: this keyboard has no such key");
                continue;
            };
            let mods = x11_mods(chord.mods);
            match self.grab(keycode, mods) {
                Ok(()) => {
                    if first {
                        tracing::info!("Registered global hotkey {chord} → {action:?}");
                    } else {
                        tracing::debug!("Grabbed {chord} → {action:?} again, on key {keycode}");
                    }
                    self.grabs.push(Grab {
                        keycode,
                        mods,
                        action,
                        chord,
                        down: false,
                    });
                }
                Err(why) => tracing::warn!("Failed to register {chord} → {action:?}: {why}"),
            }
        }
    }

    /// Grab one chord with each combination of the locks; undo it if one
    /// of them is taken.
    fn grab(&self, keycode: Keycode, mods: ModMask) -> Result<(), String> {
        for lock in locks() {
            let checked = self
                .conn
                .grab_key(
                    false,
                    self.root,
                    mods | lock,
                    keycode,
                    GrabMode::ASYNC,
                    GrabMode::ASYNC,
                )
                .map_err(|e| e.to_string())?
                .check();
            if let Err(e) = checked {
                self.ungrab(keycode, mods);
                return Err(match e {
                    x11rb::errors::ReplyError::X11Error(x) if x.error_kind == ErrorKind::Access => {
                        "another program has it".to_string()
                    }
                    other => other.to_string(),
                });
            }
        }
        Ok(())
    }

    fn ungrab(&self, keycode: Keycode, mods: ModMask) {
        for lock in locks() {
            if let Ok(cookie) = self.conn.ungrab_key(keycode, self.root, mods | lock) {
                cookie.ignore_error();
            }
        }
    }

    /// Let go of every grab and grab the chords again, on the keys the
    /// keyboard has now.
    fn regrab(&mut self) {
        for grab in std::mem::take(&mut self.grabs) {
            self.ungrab(grab.keycode, grab.mods);
        }
        self.grab_all(false);
        let _ = self.conn.flush();
    }

    /// Wait for the shortcuts, and hand each press of one to `pressed`;
    /// grab them anew when the app's change ([`follow`]). Returns when the
    /// X server goes away.
    pub fn serve(mut self, mut pressed: impl FnMut(Action, KeyChord)) {
        // Told of changes from now on — unless the pipe cannot be made,
        // or another grabber is told already: then the shortcuts stay as
        // they were at start.
        let wake = wake_pipe().and_then(|(read, write)| {
            let follow = Follow {
                told: Mutex::new(self.chords.clone()),
                next: Mutex::new(None),
                wake: write,
            };
            FOLLOW.set(follow).ok().map(|()| read)
        });
        loop {
            // Every event come already, the chords if they changed, then
            // sleep until there is more.
            loop {
                match self.conn.poll_for_event() {
                    Ok(Some(event)) => self.handle(event, &mut pressed),
                    Ok(None) => break,
                    Err(e) => {
                        tracing::warn!("Global shortcuts: the X connection is gone ({e})");
                        return;
                    }
                }
            }
            let next = wake
                .as_ref()
                .and_then(|_| FOLLOW.get()?.next.lock().ok()?.take());
            if let Some(chords) = next {
                tracing::info!("Global shortcuts changed: grabbing them anew");
                self.chords = chords;
                self.regrab();
                continue;
            }
            if !self.wait(wake.as_ref()) {
                tracing::warn!("Global shortcuts: the X connection is gone");
                return;
            }
        }
    }

    /// Sleep until the X server says something or [`follow`] wakes the
    /// thread; take what woke it from the pipe. Whether the connection is
    /// still there.
    fn wait(&self, wake: Option<&OwnedFd>) -> bool {
        let mut fds = [
            libc::pollfd {
                fd: self.conn.stream().as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: wake.map_or(-1, AsRawFd::as_raw_fd),
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        // SAFETY: two initialised `pollfd`s we own for the call; a
        // negative fd is skipped by poll(2).
        let n = unsafe { libc::poll(fds.as_mut_ptr(), 2, -1) };
        if n < 0 {
            // Interrupted: look again.
            return true;
        }
        if let Some(wake) = wake.filter(|_| fds[1].revents & libc::POLLIN != 0) {
            let mut sink = [0u8; 64];
            // SAFETY: reads into a local buffer of its length; the end is
            // non-blocking, so this stops when the pipe is empty.
            while unsafe { libc::read(wake.as_raw_fd(), sink.as_mut_ptr().cast(), sink.len()) } > 0
            {
            }
        }
        // Hung up with nothing left to read: the event poll would say so
        // too, but this is where the loop would spin.
        fds[0].revents & (libc::POLLHUP | libc::POLLERR) == 0 || fds[0].revents & libc::POLLIN != 0
    }

    /// One event from the X server.
    fn handle(&mut self, event: Event, pressed: &mut impl FnMut(Action, KeyChord)) {
        match event {
            Event::KeyPress(e) => {
                let mods = chord_mods(e.state);
                for grab in &mut self.grabs {
                    if grab.keycode == e.detail && grab.mods == mods && !grab.down {
                        grab.down = true;
                        pressed(grab.action, grab.chord);
                    }
                }
            }
            Event::KeyRelease(e) => {
                for grab in self.grabs.iter_mut().filter(|g| g.keycode == e.detail) {
                    grab.down = false;
                }
            }
            // Another layout: the same chords may be other keys now.
            // XKB says so to a client that uses it; the core protocol
            // to one that does not.
            Event::XkbMapNotify(_) | Event::XkbNewKeyboardNotify(_) => self.regrab(),
            Event::MappingNotify(e) if e.request == Mapping::KEYBOARD => self.regrab(),
            _ => {}
        }
    }
}

/// Ask XKB to report a held key's repeats as presses only, and to say
/// when the keyboard's layout changes: a client that uses XKB no longer
/// gets the core protocol's `MappingNotify` for that — the first try
/// kept its grabs on the old keys after `setxkbmap fr`.
fn use_xkb(conn: &RustConnection) {
    use x11rb::protocol::xkb::{self, ConnectionExt as _};
    let ok = conn
        .extension_information(xkb::X11_EXTENSION_NAME)
        .ok()
        .flatten()
        .is_some()
        && conn
            .xkb_use_extension(1, 0)
            .ok()
            .and_then(|c| c.reply().ok())
            .is_some_and(|r| r.supported)
        && conn
            .xkb_per_client_flags(
                xkb::ID::USE_CORE_KBD.into(),
                xkb::PerClientFlag::DETECTABLE_AUTO_REPEAT,
                xkb::PerClientFlag::DETECTABLE_AUTO_REPEAT,
                Default::default(),
                Default::default(),
                Default::default(),
            )
            .ok()
            .and_then(|c| c.reply().ok())
            .is_some();
    if !ok {
        tracing::debug!("Global shortcuts: no XKB, a held chord repeats");
        return;
    }
    let layouts = xkb::EventType::NEW_KEYBOARD_NOTIFY | xkb::EventType::MAP_NOTIFY;
    let selected = conn
        .xkb_select_events(
            xkb::ID::USE_CORE_KBD.into(),
            xkb::EventType::from(0u16),
            layouts,
            xkb::MapPart::from(0u16),
            xkb::MapPart::from(0u16),
            &xkb::SelectEventsAux::default(),
        )
        .map(|cookie| cookie.check());
    if !matches!(selected, Ok(Ok(()))) {
        tracing::debug!("Global shortcuts: layout changes go unseen");
    }
}

/// The modifiers of a press that tell chords apart: the locks left out.
fn chord_mods(state: KeyButMask) -> ModMask {
    CHORD_MODS
        .iter()
        .filter(|m| state.bits() & m.bits() != 0)
        .fold(ModMask::default(), |all, m| all | *m)
}

fn x11_mods(mods: ModifierMask) -> ModMask {
    let mut out = ModMask::default();
    if mods.ctrl() {
        out |= ModMask::CONTROL;
    }
    if mods.shift() {
        out |= ModMask::SHIFT;
    }
    if mods.alt() {
        out |= ModMask::M1;
    }
    if mods.sup() {
        out |= ModMask::M4;
    }
    out
}

/// The X keysym for a key, `None` for one X names otherwise per layout
/// (`+` is shifted `=` on some, its own key on others).
fn keysym(key: KeyCode) -> Option<u32> {
    Some(match key {
        KeyCode::Letter(c) if c.is_ascii_alphabetic() => u32::from(c.to_ascii_lowercase()),
        KeyCode::Letter(_) => return None,
        KeyCode::Digit(d) => 0x30 + u32::from(d),
        KeyCode::Named(n) => match n {
            NamedKey::Escape => 0xff1b,
            NamedKey::Space => 0x20,
            NamedKey::Tab => 0xff09,
            NamedKey::Enter => 0xff0d,
            NamedKey::Backspace => 0xff08,
            NamedKey::Delete => 0xffff,
            NamedKey::Home => 0xff50,
            NamedKey::End => 0xff57,
            NamedKey::PageUp => 0xff55,
            NamedKey::PageDown => 0xff56,
            NamedKey::ArrowUp => 0xff52,
            NamedKey::ArrowDown => 0xff54,
            NamedKey::ArrowLeft => 0xff51,
            NamedKey::ArrowRight => 0xff53,
            // XK_F1 is 0xffbe, and the rest follow.
            NamedKey::F(n @ 1..=12) => 0xffbe + u32::from(n) - 1,
            NamedKey::F(_) => return None,
        },
        KeyCode::Symbol(s) => match s {
            SymbolKey::Plus => return None,
            SymbolKey::Minus => 0x2d,
            SymbolKey::Equal => 0x3d,
            SymbolKey::BracketLeft => 0x5b,
            SymbolKey::BracketRight => 0x5d,
            SymbolKey::Backquote => 0x60,
        },
    })
}

/// The keyboard's keysyms, per keycode, as the server has them now.
struct KeyMap {
    first: Keycode,
    per_keycode: usize,
    keysyms: Vec<u32>,
}

impl KeyMap {
    fn read(conn: &RustConnection) -> Option<Self> {
        let setup = conn.setup();
        let (first, last) = (setup.min_keycode, setup.max_keycode);
        let reply = conn
            .get_keyboard_mapping(first, last.saturating_sub(first).saturating_add(1))
            .ok()?
            .reply()
            .ok()?;
        Some(Self {
            first,
            per_keycode: usize::from(reply.keysyms_per_keycode).max(1),
            keysyms: reply.keysyms,
        })
    }

    /// The first key that gives `keysym`, at any level.
    fn keycode(&self, keysym: u32) -> Option<Keycode> {
        self.keysyms
            .chunks(self.per_keycode)
            .position(|row| row.contains(&keysym))
            .and_then(|i| Keycode::try_from(i).ok())
            .and_then(|i| self.first.checked_add(i))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_name_their_keysyms() {
        assert_eq!(keysym(KeyCode::Letter('A')), Some(0x61), "lower case");
        assert_eq!(keysym(KeyCode::Digit(7)), Some(0x37));
        assert_eq!(keysym(KeyCode::Named(NamedKey::F(1))), Some(0xffbe));
        assert_eq!(keysym(KeyCode::Named(NamedKey::F(12))), Some(0xffc9));
        assert_eq!(keysym(KeyCode::Named(NamedKey::Escape)), Some(0xff1b));
        assert_eq!(keysym(KeyCode::Symbol(SymbolKey::Plus)), None);
    }

    #[test]
    fn the_locks_do_not_tell_chords_apart() {
        let ctrl_shift = x11_mods("Ctrl+Shift+A".parse::<KeyChord>().unwrap().mods);
        let with_numlock = KeyButMask::CONTROL | KeyButMask::SHIFT | KeyButMask::MOD2;
        assert_eq!(chord_mods(with_numlock), ctrl_shift);
        let with_capslock = KeyButMask::CONTROL | KeyButMask::SHIFT | KeyButMask::LOCK;
        assert_eq!(chord_mods(with_capslock), ctrl_shift);
        assert_ne!(chord_mods(KeyButMask::CONTROL), ctrl_shift);
    }

    #[test]
    fn a_keymap_finds_a_keysym_at_any_level() {
        let map = KeyMap {
            first: 8,
            per_keycode: 2,
            // keycode 8: a A; 9: = +; 10: F1
            keysyms: vec![0x61, 0x41, 0x3d, 0x2b, 0xffbe, 0],
        };
        assert_eq!(map.keycode(0x61), Some(8));
        assert_eq!(map.keycode(0x2b), Some(9));
        assert_eq!(map.keycode(0xffbe), Some(10));
        assert_eq!(map.keycode(0x7a), None);
    }
}
