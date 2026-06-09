//! Central Wayland event-loop state. Extracted in K.1.
//!
//! SCTK funnels every protocol event into a single object via the
//! `delegate_*!` macros, so `WaylandState` owns the registry, the
//! seat / pointer / keyboard handles, the layer surface, the drag-drop
//! plumbing, and the per-frame egui event buffer.
//!
//! Sub-files (`keyboard_handler.rs`, `pointer_handler.rs`, etc.)
//! provide handler trait impls on this struct. The struct itself
//! stays in this file so future field additions don't have to thread
//! through every handler module.
//!
//! `InputRect` and the no-op `wl_region` dispatch live here too —
//! both are tightly coupled to the surface and small enough that a
//! separate file would just add noise.

use smithay_client_toolkit::{
    compositor::CompositorState,
    data_device_manager::{data_device::DataDevice, DataDeviceManagerState},
    output::OutputState,
    registry::RegistryState,
    seat::{keyboard::Modifiers as SctkModifiers, SeatState},
    shell::wlr_layer::{LayerShell, LayerSurface},
};
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::mpsc;
use std::sync::Arc;
use wayland_client::{
    protocol::{wl_keyboard, wl_pointer, wl_region},
    Connection, Dispatch, QueueHandle,
};

/// The single sctk event-loop state. Every `delegate_*!` macro in
/// `mod.rs` resolves the corresponding trait impl on this struct, so
/// the struct definition stays here and the handlers live in sibling
/// files (`keyboard_handler.rs`, `pointer_handler.rs`, `drag_drop.rs`,
/// `handlers.rs`).
pub struct WaylandState {
    pub registry_state: RegistryState,
    pub output_state: OutputState,
    pub seat_state: SeatState,
    pub compositor: CompositorState,
    pub(super) _layer_shell: LayerShell,
    pub layer: LayerSurface,
    /// Active pointer once a seat advertises the capability. We keep one
    /// reference even on multi-seat setups (overlays don't usually need
    /// per-seat cursors).
    pub pointer: Option<wl_pointer::WlPointer>,
    /// Active keyboard once a seat advertises the capability. Same
    /// rationale as the pointer — multi-seat is rare for an overlay.
    /// Decoded via sctk's `xkbcommon` feature (E.1).
    pub keyboard: Option<wl_keyboard::WlKeyboard>,
    /// Most-recent modifier state delivered by `KeyboardHandler::
    /// update_modifiers`. Attached to every synthesised egui::Event::Key
    /// so chord lookup in `KeyBindings::lookup` matches what the user
    /// pressed.
    pub last_modifiers: SctkModifiers,
    /// Last size we got from a `configure` event — drives wgpu resize.
    pub pending_size: Option<(u32, u32)>,
    /// Current cursor position in surface-local logical pixels.
    pub cursor_pos: Option<(f32, f32)>,
    /// Pointer + keyboard events translated into egui's vocabulary,
    /// buffered until `LayerWindow::drain_egui_events` is called once
    /// per frame. Text events (`egui::Event::Text`) come from sctk's
    /// `KeyEvent::utf8` — xkbcommon has already composed dead-keys /
    /// IME by then, so widget text input is correct without extra work.
    pub pending_egui_events: Vec<egui::Event>,
    /// True after a layer-surface `closed` event. Caller polls this to
    /// know when to tear down.
    pub close_requested: bool,
    /// Whether the overlay is currently interactive (full-surface input
    /// region) or pass-through (only the ⚙ corner takes clicks). Owned
    /// by `LayerWindow::set_edit_mode`; the caller never mutates it
    /// directly so the input-region commit stays in lock-step with the
    /// flag.
    pub edit_mode: bool,
    /// Drag-and-drop wiring (E.3). `data_device_manager` is the global
    /// once bound; `data_device` is the per-seat handle we set actions /
    /// accept mime on. `last_drag_pos` follows the latest motion event
    /// during a drag so the drop landing coordinate matches what the
    /// user sees. `drop_rx` carries parsed file paths from the worker
    /// thread that drains the receive-pipe back to the main loop.
    pub data_device_manager: DataDeviceManagerState,
    pub data_device: Option<DataDevice>,
    pub last_drag_pos: Option<(f32, f32)>,
    pub drop_tx: mpsc::SyncSender<Vec<PathBuf>>,
    pub drop_rx: mpsc::Receiver<Vec<PathBuf>>,
    /// Number of drop-reader worker threads currently in flight (F.2,
    /// 0.5.1). Bounds the per-drop `std::thread::spawn` so a tight
    /// loop of drag events from a hostile source can't exhaust
    /// RLIMIT_NPROC. Decremented by each worker on exit through a
    /// `DropCounterGuard` so the counter stays honest even if the
    /// worker panics mid-read.
    pub active_drop_workers: Arc<AtomicUsize>,
}

/// Rectangular cutout used by `LayerWindow::set_input_region`.
#[derive(Debug, Clone, Copy)]
pub struct InputRect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl InputRect {
    /// Whole-surface input region — every pixel receives clicks. Used
    /// when entering edit mode.
    pub fn full(w: u32, h: u32) -> Self {
        Self { x: 0, y: 0, w, h }
    }

    /// Top-right corner of size `size` × `size` inside an `w` × `h`
    /// surface — matches the ⚙ button geometry from the X11 path.
    pub fn toggle_button_corner(surface_w: u32, size: u32) -> Self {
        Self {
            x: surface_w as i32 - size as i32,
            y: 0,
            w: size,
            h: size,
        }
    }
}

/// `wl_compositor::create_region` returns a `wl_region` proxy that we
/// never receive events on (it's purely a constructor handle). sctk
/// requires a `Dispatch` impl for every protocol object the queue
/// touches; a no-op covers it.
impl Dispatch<wl_region::WlRegion, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_region::WlRegion,
        _event: <wl_region::WlRegion as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}
