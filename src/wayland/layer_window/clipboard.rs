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
use std::io::{Read, Write};
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
    /// Put `text` — the names of the characters just copied — on the
    /// clipboard, under our own type as well as plain text.
    pub fn claim_clipboard(&mut self, text: String) {
        let Some(device) = self.state.data_device.as_ref() else {
            return;
        };
        let qh = self.event_queue.handle();
        let mut types = vec![sc::ANIMA_MIME];
        types.extend(sc::TEXT_MIMES);
        let source = self
            .state
            .data_device_manager
            .create_copy_paste_source(&qh, types);
        source.set_selection(device, self.state.last_serial);
        self.state.clipboard_claim = Some(Claim { source, text });
    }

    /// What the clipboard holds, as far as a paste goes.
    pub fn read_clipboard(&self) -> ClipboardRead {
        let Some(offer) = self
            .state
            .data_device
            .as_ref()
            .and_then(|d| d.data().selection_offer())
        else {
            return ClipboardRead::Ready(Content::Nothing);
        };
        let pick = offer.with_mime_types(sc::pick);
        let mime = match &pick {
            Pick::Ours => return ClipboardRead::Ready(Content::Ours),
            Pick::Nothing => return ClipboardRead::Ready(Content::Nothing),
            Pick::Image(mime, _) | Pick::Files(mime) => mime.clone(),
        };
        let pipe = match offer.receive(mime) {
            Ok(pipe) => pipe,
            Err(e) => return ClipboardRead::Ready(Content::Failed(e.to_string())),
        };
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("anima-clipboard-read".into())
            .spawn(move || {
                let cap = sc::cap_for(&pick);
                let mut bytes = Vec::new();
                let content = match pipe.take(cap + 1).read_to_end(&mut bytes) {
                    Ok(_) if bytes.len() as u64 > cap => {
                        Content::Failed(format!("over {} MB", cap / (1024 * 1024)))
                    }
                    Ok(_) => Content::from_bytes(&pick, bytes),
                    Err(e) => Content::Failed(e.to_string()),
                };
                let _ = tx.send(content);
            });
        if let Err(e) = spawned {
            return ClipboardRead::Ready(Content::Failed(e.to_string()));
        }
        ClipboardRead::Pending(rx)
    }
}

impl WaylandState {
    /// Another client asked for what we put on the clipboard.
    pub(super) fn send_clipboard(&mut self, source: &WlDataSource, mut fd: WritePipe) {
        if let Some(claim) = &self.clipboard_claim {
            if claim.source.inner() == source {
                if let Err(e) = fd.write_all(claim.text.as_bytes()) {
                    tracing::debug!("Clipboard: writing our text failed: {e}");
                }
            }
        }
        // Dropping the pipe closes it: the reader sees the end.
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
