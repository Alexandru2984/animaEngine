//! The desktop's clipboard on native Wayland (`crate::system_clipboard`):
//! putting the names of characters copied here on it, and reading what
//! is on it for a paste.
//!
//! Only the client with keyboard focus sees the selection; the overlay
//! has it in edit mode, which is where copying and pasting happen. The
//! clipboard's content arrives through a pipe the source writes when the
//! compositor asks it to, so it is read on a thread of its own and capped.

use super::state::WaylandState;
use super::LayerWindow;
use crate::system_clipboard::{self as sc, Content, Pick};
use smithay_client_toolkit::data_device_manager::data_source::CopyPasteSource;
use smithay_client_toolkit::data_device_manager::WritePipe;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use wayland_client::protocol::wl_data_source::WlDataSource;

/// What asking the clipboard gave: at once, or from a reader thread.
pub enum ClipboardRead {
    Ready(Content),
    Pending(mpsc::Receiver<Content>),
}

/// Our side of the clipboard while it holds what was copied here.
pub struct Claim {
    source: CopyPasteSource,
    text: String,
}

impl LayerWindow {
    /// Put `text` on the clipboard: the names of the characters just
    /// copied (`ours`, so our own type goes with it), or text copied in
    /// a text field of the panel.
    pub fn claim_clipboard(&mut self, text: String, ours: bool) {
        let Some(device) = self.state.data_device.as_ref() else {
            return;
        };
        let qh = self.event_queue.handle();
        let mut types = Vec::new();
        if ours {
            types.push(sc::ANIMA_MIME);
        }
        types.extend(sc::TEXT_MIMES);
        let source = self
            .state
            .data_device_manager
            .create_copy_paste_source(&qh, types);
        source.set_selection(device, self.state.last_serial);
        self.state.clipboard_claim = Some(Claim { source, text });
    }

    /// What the clipboard holds, as far as a paste for `want` goes.
    pub fn read_clipboard(&self, want: sc::Want) -> ClipboardRead {
        let Some(offer) = self
            .state
            .data_device
            .as_ref()
            .and_then(|d| d.data().selection_offer())
        else {
            return ClipboardRead::Ready(Content::Nothing);
        };
        let pick = offer.with_mime_types(|offered| sc::pick_for(want, offered));
        let mime = match &pick {
            Pick::Ours => return ClipboardRead::Ready(Content::Ours),
            Pick::Nothing => return ClipboardRead::Ready(Content::Nothing),
            Pick::Image(mime, _) | Pick::Files(mime) | Pick::Text(mime) => mime.clone(),
        };
        // At most a few readers at once: a source that never wrote kept
        // its reader, and its pipe, for good, one more each paste
        // (SEC-008).
        if READERS.fetch_add(1, Ordering::AcqRel) >= MAX_READERS {
            READERS.fetch_sub(1, Ordering::AcqRel);
            return ClipboardRead::Ready(Content::Failed("a paste is still being read".into()));
        }
        let pipe = match offer.receive(mime) {
            Ok(pipe) => pipe,
            Err(e) => {
                READERS.fetch_sub(1, Ordering::AcqRel);
                return ClipboardRead::Ready(Content::Failed(e.to_string()));
            }
        };
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("anima-clipboard-read".into())
            .spawn(move || {
                let mut pipe = pipe;
                let cap = sc::cap_for(&pick);
                let content =
                    match super::pipes::read_within(&mut pipe, cap, super::pipes::READ_WITHIN) {
                        Ok(bytes) if bytes.len() as u64 > cap => {
                            Content::Failed(format!("over {}", said_size(cap)))
                        }
                        Ok(bytes) => Content::from_bytes(&pick, bytes),
                        Err(e) => Content::Failed(e.to_string()),
                    };
                READERS.fetch_sub(1, Ordering::AcqRel);
                let _ = tx.send(content);
                // The loop may be asleep; the paste is its to show now.
                crate::wayland::waker::wake();
            });
        if let Err(e) = spawned {
            READERS.fetch_sub(1, Ordering::AcqRel);
            return ClipboardRead::Ready(Content::Failed(e.to_string()));
        }
        ClipboardRead::Pending(rx)
    }
}

/// Pastes being read, and requests for our copy being written, at once.
static READERS: AtomicUsize = AtomicUsize::new(0);
static WRITERS: AtomicUsize = AtomicUsize::new(0);
const MAX_READERS: usize = 4;
const MAX_WRITERS: usize = 4;

/// `bytes` for a person: MB from one, KiB below — a list of files' 64 KiB
/// cap was "over 0 MB" (SEC-008).
fn said_size(bytes: u64) -> String {
    const MB: u64 = 1024 * 1024;
    if bytes >= MB {
        format!("{} MB", bytes / MB)
    } else {
        format!("{} KiB", bytes.div_ceil(1024))
    }
}

impl WaylandState {
    /// Another client asked for what we put on the clipboard.
    ///
    /// Written on a thread of its own, with a deadline: written here, a
    /// reader that never read kept the main loop in `write` for good
    /// (SEC-008). At most a few at once; past that the pipe is closed
    /// unwritten, which the reader sees as an empty answer.
    pub(super) fn send_clipboard(&mut self, source: &WlDataSource, fd: WritePipe) {
        let Some(claim) = self
            .clipboard_claim
            .as_ref()
            .filter(|c| c.source.inner() == source)
        else {
            return;
        };
        if WRITERS.fetch_add(1, Ordering::AcqRel) >= MAX_WRITERS {
            WRITERS.fetch_sub(1, Ordering::AcqRel);
            tracing::debug!("Clipboard: too many readers at once; one refused");
            return;
        }
        let text = claim.text.clone();
        let spawned = std::thread::Builder::new()
            .name("anima-clipboard-write".into())
            .spawn(move || {
                let mut fd = fd;
                if let Err(e) = super::pipes::write_all_within(
                    &mut fd,
                    text.as_bytes(),
                    super::pipes::WRITE_WITHIN,
                ) {
                    tracing::debug!("Clipboard: writing our text failed: {e}");
                }
                WRITERS.fetch_sub(1, Ordering::AcqRel);
                // Dropping the pipe closes it: the reader sees the end.
            });
        if let Err(e) = spawned {
            WRITERS.fetch_sub(1, Ordering::AcqRel);
            tracing::debug!("Clipboard: no writer thread: {e}");
        }
    }

    /// Something else was copied: the clipboard is no longer ours.
    pub(super) fn clipboard_cancelled(&mut self, source: &WlDataSource) {
        if self
            .clipboard_claim
            .as_ref()
            .is_some_and(|c| c.source.inner() == source)
        {
            self.clipboard_claim = None;
        }
    }
}
