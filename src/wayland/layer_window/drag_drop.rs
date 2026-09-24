//! Drag-and-drop handlers + the F.2 audit hardening. Extracted in K.4.
//!
//! The Wayland drop flow:
//!
//! 1. `DataDeviceHandler::enter` — compositor advertises the drag entry;
//!    we accept the `text/uri-list` mime type and set `DndAction::Copy`
//!    as our preferred action.
//! 2. `drop_performed` — take the drop point and the receive-pipe from
//!    the drag offer, and hand both to a worker thread that reads and
//!    parses the list, sends [`DroppedFiles`] back over `drop_tx`, and
//!    finishes the offer. The main loop drains `drop_rx` each frame.
//!
//! The drop point travels *with* the paths. It used to be read from a
//! "last drag position" the main loop looked up when the paths arrived —
//! by which time the `leave` that follows every drop had cleared it, so
//! each drop landed at the fallback, the middle of the screen.
//!
//! `finish` matters as much as the read: under `wl_data_offer` v3 the
//! source only learns the drop succeeded through it. Without it the
//! source never receives `dnd_finished`.
//!
//! F.2 (0.5.1) hardening:
//!
//! - `MAX_URI_LIST_BYTES` (64 KiB cap on the receive pipe) — was an
//!   unbounded `read_to_end` previously.
//! - `MAX_CONCURRENT_DROPS` (4 worker threads max) — refuses new
//!   readers when the cap is hit so a hostile source can't fork-bomb.
//! - `DROP_RESULT_QUEUE_CAP` (16 batches max in the bounded channel)
//!   — `try_send` drops new batches if the consumer is stuck rather
//!   than growing memory.
//! - `DropCounterGuard` RAII — keeps the in-flight worker count
//!   honest even when a reader panics during `read_to_end`.
//!
//! `DataSourceHandler` is a no-op set; the overlay never starts
//! outgoing drags. The trait is still required by `delegate_data_device!`.

use super::state::WaylandState;
use crate::wayland::data_device::{parse_uri_list, URI_LIST_MIME};
use smithay_client_toolkit::data_device_manager::{
    data_device::DataDeviceHandler,
    data_offer::{DataOfferHandler, DragOffer},
    data_source::DataSourceHandler,
    WritePipe,
};
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use wayland_client::{
    protocol::{wl_data_device_manager::DndAction, wl_surface},
    Connection, QueueHandle,
};

/// Cap on the size of one `text/uri-list` payload (F.2). 64 KiB is
/// well past any legitimate drag — a hundred 600-byte paths fits
/// comfortably — and forecloses the previous unbounded `read_to_end`
/// against a malicious source that streams gigabytes through the
/// pipe.
const MAX_URI_LIST_BYTES: u64 = 64 * 1024;

/// Cap on concurrent drop-reader worker threads (F.2). A
/// drag-and-drop is a brief user gesture; if we see more than this
/// in flight we're being attacked and refuse to start a new one.
const MAX_CONCURRENT_DROPS: usize = 4;

/// Cap on the in-memory drop-result queue (F.2). The wayland event
/// loop drains it once per frame; under sustained pressure the
/// bounded sender just drops the newest extra batches with a
/// warning rather than letting the channel grow without bound.
pub(super) const DROP_RESULT_QUEUE_CAP: usize = 16;

/// RAII guard decrementing the `active_drop_workers` counter on
/// thread exit. Centralised so panic-induced unwinds still keep the
/// counter accurate.
struct DropCounterGuard {
    counter: Arc<AtomicUsize>,
}

impl Drop for DropCounterGuard {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::AcqRel);
    }
}

/// One drop: the files, and the surface-local point they were dropped at.
#[derive(Debug)]
pub struct DroppedFiles {
    pub paths: Vec<PathBuf>,
    pub at: (f32, f32),
}

impl DataDeviceHandler for WaylandState {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _data_device: &wayland_client::protocol::wl_data_device::WlDataDevice,
        _x: f64,
        _y: f64,
        _surface: &wl_surface::WlSurface,
    ) {
        self.drag_over = true;
        let Some(device) = self.data_device.as_ref() else {
            return;
        };
        let Some(offer) = device.data().drag_offer() else {
            return;
        };
        // Tell the source we'll accept its files as a copy. set_actions
        // is required before the source decides whether to even
        // transmit the payload.
        offer.set_actions(DndAction::Copy, DndAction::Copy);
        offer.accept_mime_type(offer.serial, Some(URI_LIST_MIME.to_string()));
    }

    // The toolkit records the position on the offer itself, which is where
    // `drop_performed` reads it from.
    fn motion(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _data_device: &wayland_client::protocol::wl_data_device::WlDataDevice,
        _x: f64,
        _y: f64,
    ) {
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _data_device: &wayland_client::protocol::wl_data_device::WlDataDevice,
    ) {
        self.drag_over = false;
    }

    fn selection(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _data_device: &wayland_client::protocol::wl_data_device::WlDataDevice,
    ) {
        // Clipboard selection — not a drop. Overlay doesn't consume
        // clipboard, so ignored.
    }

    fn drop_performed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _data_device: &wayland_client::protocol::wl_data_device::WlDataDevice,
    ) {
        // The drag is over whether or not the read below works.
        self.drag_over = false;
        let Some(device) = self.data_device.as_ref() else {
            return;
        };
        let Some(offer) = device.data().drag_offer() else {
            return;
        };
        // F.2: refuse new readers when MAX_CONCURRENT_DROPS workers
        // are already in flight. A malicious source can't spawn an
        // unbounded number of background threads on us this way.
        let prev = self.active_drop_workers.fetch_add(1, Ordering::AcqRel);
        if prev >= MAX_CONCURRENT_DROPS {
            self.active_drop_workers.fetch_sub(1, Ordering::AcqRel);
            tracing::warn!(
                "Drop refused: {prev} drop workers already in flight (cap {})",
                MAX_CONCURRENT_DROPS
            );
            return;
        }
        let pipe = match offer.receive(URI_LIST_MIME.to_string()) {
            Ok(p) => p,
            Err(e) => {
                self.active_drop_workers.fetch_sub(1, Ordering::AcqRel);
                tracing::warn!("Drop: receive(text/uri-list) failed: {e}");
                return;
            }
        };
        let at = (offer.x as f32, offer.y as f32);
        let tx = self.drop_tx.clone();
        let guard = DropCounterGuard {
            counter: self.active_drop_workers.clone(),
        };
        let spawned = std::thread::Builder::new()
            .name("anima-drop-read".into())
            .spawn(move || {
                // Hold the guard for the whole closure so the counter
                // decrements regardless of how we exit (success, error,
                // or panic during read_to_end).
                let _guard = guard;
                // F.2: cap the payload via `Read::take` so the previous
                // unbounded `read_to_end` can't be exploited by a source
                // that streams gigabytes. 64 KiB covers every legitimate
                // drag (hundreds of paths fit comfortably).
                let mut capped = pipe.take(MAX_URI_LIST_BYTES);
                let mut buf = Vec::with_capacity(512);
                let read = capped.read_to_end(&mut buf);
                // Tell the source it is over either way; wayland-client
                // proxies may be used from any thread, and the main loop
                // flushes the requests on its next pass.
                offer.finish();
                offer.destroy();
                if let Err(e) = read {
                    tracing::warn!("Drop: read pipe failed: {e}");
                    return;
                }
                let paths = parse_uri_list(&buf);
                if !paths.is_empty() {
                    // `try_send` so a stuck consumer (main loop) can't
                    // make us block here; bounded channel = bounded
                    // memory.
                    if let Err(e) = tx.try_send(DroppedFiles { paths, at }) {
                        tracing::warn!("Drop: result queue full, dropping batch: {e}");
                    }
                }
            });
        // Spawn failure drops the closure (and `guard` with it), which
        // decrements active_drop_workers via DropCounterGuard — the
        // counter stays correct even on this rare path.
        if let Err(e) = spawned {
            tracing::warn!("Drop: read worker failed to spawn: {e}");
        }
    }
}

/// We never start outgoing drags from the overlay (no
/// `create_drag_and_drop_source` call site). The trait is required by
/// `delegate_data_device!` since the manager handles WlDataSource
/// dispatch too — every method here is a no-op.
impl DataSourceHandler for WaylandState {
    fn accept_mime(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &wayland_client::protocol::wl_data_source::WlDataSource,
        _mime: Option<String>,
    ) {
    }

    fn send_request(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &wayland_client::protocol::wl_data_source::WlDataSource,
        _mime: String,
        _fd: WritePipe,
    ) {
    }

    fn cancelled(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &wayland_client::protocol::wl_data_source::WlDataSource,
    ) {
    }

    fn dnd_dropped(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &wayland_client::protocol::wl_data_source::WlDataSource,
    ) {
    }

    fn dnd_finished(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &wayland_client::protocol::wl_data_source::WlDataSource,
    ) {
    }

    fn action(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &wayland_client::protocol::wl_data_source::WlDataSource,
        _action: DndAction,
    ) {
    }
}

impl DataOfferHandler for WaylandState {
    fn source_actions(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        offer: &mut DragOffer,
        _actions: DndAction,
    ) {
        // Always prefer Copy — overlay doesn't move source files.
        offer.set_actions(DndAction::Copy, DndAction::Copy);
    }

    fn selected_action(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _offer: &mut DragOffer,
        _actions: DndAction,
    ) {
        // The compositor picked an action; we don't need to react —
        // the source will send the payload on receive().
    }
}
