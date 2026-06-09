//! Native Wayland overlay surface backed by `wlr-layer-shell-unstable-v1`.
//!
//! This module is **scaffolding for the native Wayland backend**. It owns
//! the compositor connection, the registry bind, the layer surface, and
//! the wgpu surface attached to it. Event translation (pointer / keyboard)
//! and rendering integration live in sibling modules — added in sub-phases
//! 7.3 / 7.4 of Phase 7.
//!
//! ## Untested on the development machine
//!
//! GNOME Mutter (the host this code is being written on) does not advertise
//! `zwlr_layer_shell_v1`, so the code below has been validated only against
//! the published Wayland protocol spec and the sctk API docs. It will
//! receive real exercise the first time someone runs the binary under
//! sway / Hyprland / river.
//!
//! ## Lifecycle
//!
//! 1. `try_create()` connects to the compositor and binds all required
//!    globals. Fails fast with a descriptive error when anything is
//!    missing (no `zwlr_layer_shell_v1`, no `wl_compositor`, etc.) so the
//!    caller can fall back to the X11 path.
//! 2. The returned `LayerWindow` exposes the wgpu surface + the sctk event
//!    queue. The caller drives both per frame.
//! 3. Dropping `LayerWindow` releases the layer surface and disconnects.

mod keyboard_handler;
mod state;

pub use state::{InputRect, WaylandState};

use crate::error::{AnimaError, Result};
use crate::wayland::data_device::{parse_uri_list, URI_LIST_MIME};
use crate::wayland::keyboard::modifiers_to_egui;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    data_device_manager::{
        data_device::DataDeviceHandler,
        data_offer::{DataOfferHandler, DragOffer},
        data_source::DataSourceHandler,
        DataDeviceManagerState, WritePipe,
    },
    delegate_compositor, delegate_data_device, delegate_keyboard, delegate_layer, delegate_output,
    delegate_pointer, delegate_registry, delegate_seat,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::Modifiers as SctkModifiers,
        pointer::{PointerData, PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        WaylandSurface,
    },
};
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_data_device_manager::DndAction, wl_output, wl_pointer, wl_seat, wl_surface},
    Connection, EventQueue, QueueHandle,
};

/// What `try_create` produces. The caller stores it and drives the event
/// loop manually (we don't spawn anything internally).
///
/// `wgpu_instance` / `wgpu_surface` are `Option` so the run-loop can
/// `.take()` them and hand them to `WgpuRenderer::from_instance_surface`
/// without copying or transmuting. The `LayerWindow` keeps the underlying
/// `wl_surface` alive afterward — the raw handle baked into the wgpu
/// surface remains valid until the `LayerWindow` is dropped, so the
/// caller must drop `WgpuRenderer` first.
pub struct LayerWindow {
    pub connection: Connection,
    pub event_queue: EventQueue<WaylandState>,
    pub state: WaylandState,
    pub wgpu_surface: Option<wgpu::Surface<'static>>,
    pub wgpu_instance: Option<wgpu::Instance>,
    /// Logical dimensions reported by the compositor's first `configure`.
    /// Stays `None` until the round-trip after surface commit completes.
    pub size: Option<(u32, u32)>,
}

impl LayerWindow {
    /// Best-effort layer surface creation. Returns `Err` with a clear
    /// reason when the compositor lacks any required global — callers
    /// drop down to the X11 path.
    pub fn try_create() -> Result<Self> {
        let connection = Connection::connect_to_env()
            .map_err(|e| AnimaError::other(format!("wayland connect: {e}")))?;

        let (globals, mut event_queue) = registry_queue_init::<WaylandState>(&connection)
            .map_err(|e| AnimaError::other(format!("wayland registry init: {e}")))?;
        let qh = event_queue.handle();

        // Bind required globals up front so we know we can succeed before
        // we touch wgpu.
        let registry_state = RegistryState::new(&globals);
        // Bounded channel — `sync_channel(DROP_RESULT_QUEUE_CAP)` —
        // so a malicious source spamming drops can't grow memory
        // without bound between two frames of the event loop. F.2
        // (0.5.1) hardening.
        let (drop_tx, drop_rx) = mpsc::sync_channel::<Vec<PathBuf>>(DROP_RESULT_QUEUE_CAP);
        let output_state = OutputState::new(&globals, &qh);
        let seat_state = SeatState::new(&globals, &qh);
        let data_device_manager = DataDeviceManagerState::bind(&globals, &qh).map_err(|e| {
            AnimaError::other(format!(
                "wl_data_device_manager missing — drag-drop disabled: {e}"
            ))
        })?;
        let compositor = CompositorState::bind(&globals, &qh)
            .map_err(|e| AnimaError::other(format!("no wl_compositor: {e}")))?;
        let layer_shell = LayerShell::bind(&globals, &qh)
            .map_err(|e| AnimaError::other(format!("no zwlr_layer_shell_v1: {e}")))?;

        // Create the wl_surface that the layer is built on.
        let wl_surface = compositor.create_surface(&qh);

        // Wrap it as a layer surface anchored to all four edges so it
        // covers the entire output. Layer::Overlay sits above normal
        // windows; `KeyboardInteractivity::OnDemand` means the surface
        // only grabs keyboard focus when we explicitly ask (e.g. an
        // egui text box accepting input).
        let layer = layer_shell.create_layer_surface(
            &qh,
            wl_surface.clone(),
            Layer::Overlay,
            Some("anima_engine"),
            None, // any output — we'll pick later, multi-monitor in 7.x
        );
        layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
        // exclusive_zone(-1) → tell the compositor we *don't* reserve any
        // edge space (so docks etc. keep their layout) but still draw
        // over them.
        layer.set_exclusive_zone(-1);
        layer.commit();

        // Build the wgpu instance and surface from the raw Wayland
        // handles. The compositor's first configure round-trip happens
        // after this; we'll learn the real size in `dispatch_until_sized`.
        let wgpu_instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::GL,
            ..Default::default()
        });
        let wgpu_surface = build_wgpu_surface(&wgpu_instance, &connection, layer.wl_surface())?;

        let mut state = WaylandState {
            registry_state,
            output_state,
            seat_state,
            compositor,
            _layer_shell: layer_shell,
            layer,
            pointer: None,
            keyboard: None,
            last_modifiers: SctkModifiers::default(),
            pending_size: None,
            cursor_pos: None,
            pending_egui_events: Vec::new(),
            close_requested: false,
            edit_mode: false,
            data_device_manager,
            data_device: None,
            last_drag_pos: None,
            drop_tx,
            drop_rx,
            active_drop_workers: Arc::new(AtomicUsize::new(0)),
        };

        // Round-trip so the compositor sends us its first `configure`
        // event and we learn the size. This also surfaces protocol errors
        // (bad anchor combination, etc.) early.
        event_queue
            .roundtrip(&mut state)
            .map_err(|e| AnimaError::other(format!("initial roundtrip: {e}")))?;

        let size = state.pending_size;
        Ok(Self {
            connection,
            event_queue,
            state,
            wgpu_surface: Some(wgpu_surface),
            wgpu_instance: Some(wgpu_instance),
            size,
        })
    }
}

/// Wrap the wayland connection + layer wl_surface in a wgpu-compatible
/// raw-window-handle pair and ask wgpu for a Surface.
///
/// `wgpu::SurfaceTargetUnsafe::RawHandle` requires `'static` references
/// internally; we satisfy that by leaking small `Arc`-wrapped handle
/// values for the lifetime of the process. The leak is fixed-size and
/// bounded to one surface, so it's a non-issue in practice.
fn build_wgpu_surface(
    instance: &wgpu::Instance,
    connection: &Connection,
    surface: &wl_surface::WlSurface,
) -> Result<wgpu::Surface<'static>> {
    use raw_window_handle::{
        RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
    };
    use std::ptr::NonNull;
    use wayland_client::Proxy;

    let display_ptr = connection.backend().display_ptr() as *mut std::ffi::c_void;
    let surface_ptr = surface.id().as_ptr() as *mut std::ffi::c_void;

    let display =
        NonNull::new(display_ptr).ok_or_else(|| AnimaError::other("null wl_display ptr"))?;
    let surface =
        NonNull::new(surface_ptr).ok_or_else(|| AnimaError::other("null wl_surface ptr"))?;

    let display_handle = RawDisplayHandle::Wayland(WaylandDisplayHandle::new(display));
    let window_handle = RawWindowHandle::Wayland(WaylandWindowHandle::new(surface));

    // SAFETY: both pointers stay valid for the lifetime of the
    // `LayerWindow` (the connection + wl_surface live inside the struct).
    // We never reach this code on platforms that don't support Wayland.
    let surface = unsafe {
        instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
            raw_display_handle: display_handle,
            raw_window_handle: window_handle,
        })
    }
    .map_err(|e| AnimaError::other(format!("wgpu surface from wayland handle: {e}")))?;

    Ok(surface)
}

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
const DROP_RESULT_QUEUE_CAP: usize = 16;

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

impl LayerWindow {
    /// Drain pointer events accumulated since the previous call. Frame
    /// callbacks invoke this and feed the result into egui.
    pub fn drain_egui_events(&mut self) -> Vec<egui::Event> {
        std::mem::take(&mut self.state.pending_egui_events)
    }

    /// Drain any file paths the drag-drop worker thread parsed since
    /// the last call (E.3). Each call returns ownership of the paths
    /// alongside the last drag position, so the caller can spawn
    /// entities at the cursor's landing coordinate.
    pub fn drain_dropped_files(&mut self) -> Vec<PathBuf> {
        let mut out = Vec::new();
        while let Ok(batch) = self.state.drop_rx.try_recv() {
            out.extend(batch);
        }
        out
    }

    /// Last surface-local position the drag cursor reported. Returns
    /// `None` once the drag leaves or completes; call this right after
    /// `drain_dropped_files` to anchor newly-spawned entities.
    pub fn last_drag_pos(&self) -> Option<(f32, f32)> {
        self.state.last_drag_pos
    }

    /// Snapshot every `wl_output` the compositor has advertised so far
    /// (E.7), projected onto the engine's neutral
    /// [`MonitorInfo`](crate::monitor::MonitorInfo) shape. Cheap to call
    /// once per frame — OutputState already caches the per-output state
    /// and we copy a few fields per output. Empty when nothing has been
    /// bound yet (the compositor sends globals before the first
    /// configure round-trip).
    pub fn monitors(&self) -> Vec<crate::monitor::MonitorInfo> {
        let mut out = Vec::new();
        for output in self.state.output_state.outputs() {
            let Some(info) = self.state.output_state.info(&output) else {
                continue;
            };
            let name = info
                .name
                .clone()
                .unwrap_or_else(|| format!("wl_output #{}", info.id));
            // Prefer the logical position/size when the compositor
            // supplies them (xdg-output extension); fall back to the
            // physical mode otherwise so single-DPI users still get a
            // sensible monitor entry.
            let (x, y) = info.logical_position.unwrap_or(info.location);
            let (w, h) = info.logical_size.unwrap_or_else(|| {
                info.modes
                    .iter()
                    .find(|m| m.current)
                    .map(|m| m.dimensions)
                    .unwrap_or((0, 0))
            });
            out.push(crate::monitor::MonitorInfo {
                name,
                x,
                y,
                width: w.max(0) as u32,
                height: h.max(0) as u32,
                scale_factor: info.scale_factor as f64,
                // wl_output protocol has no "primary" notion; the
                // compositor decides placement. Mark none — picker UI
                // shows them in advertised order.
                is_primary: false,
            });
        }
        out
    }

    /// Swap the click-through region in lock-step with the edit-mode
    /// flag: edit-mode on → whole surface receives clicks; off → only
    /// the top-right ⚙ button corner does. Idempotent — calling with
    /// the current value is a no-op so the dispatch loop can call this
    /// every time it suspects a flip happened without spamming the
    /// compositor.
    pub fn set_edit_mode(&mut self, on: bool, toggle_size: u32) -> Result<()> {
        if self.state.edit_mode == on {
            return Ok(());
        }
        let Some((w, h)) = self.size else {
            // No configure yet — defer until first frame.
            self.state.edit_mode = on;
            return Ok(());
        };
        let rect = if on {
            InputRect::full(w, h)
        } else {
            InputRect::toggle_button_corner(w, toggle_size)
        };
        self.set_input_region(Some(rect))?;
        self.state.edit_mode = on;
        Ok(())
    }

    /// Configure the click-through input region.
    ///
    /// - `cutout = None` → empty region → **every** pixel is click-through
    ///   (no point in this state for our overlay, but a useful primitive).
    /// - `cutout = Some(rect)` → only `rect` receives input; the rest is
    ///   click-through. Equivalent to the X11 input shape used in
    ///   pass-through mode for the ⚙ toggle button.
    /// - Pass `cutout = Some(InputRect::full(w, h))` to make the entire
    ///   surface receive input — used for edit mode.
    pub fn set_input_region(&mut self, cutout: Option<InputRect>) -> Result<()> {
        let qh = self.event_queue.handle();
        // `wl_compositor::create_region` is a constructor — the returned
        // proxy is fresh state owned by the client. We hand it back to
        // the compositor via `set_input_region` (which copies it) and
        // then destroy it immediately; the surface keeps the region.
        let region = self.state.compositor.wl_compositor().create_region(&qh, ());
        if let Some(rect) = cutout {
            region.add(rect.x, rect.y, rect.w as i32, rect.h as i32);
        }
        self.state
            .layer
            .wl_surface()
            .set_input_region(Some(&region));
        self.state.layer.commit();
        region.destroy();
        Ok(())
    }
}

// ──────────────────────────────────────────────────────────────────────
//  sctk handler impls. Most of these are pure plumbing — sctk requires
//  every protocol it manages to have a handler even when we don't react
//  to its events.
// ──────────────────────────────────────────────────────────────────────

impl LayerShellHandler for WaylandState {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        self.close_requested = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let (w, h) = configure.new_size;
        if w > 0 && h > 0 {
            self.pending_size = Some((w, h));
        }
    }
}

impl CompositorHandler for WaylandState {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
        // Hi-DPI handling lands in a later sub-phase.
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        // Frame callbacks drive the render loop in 7.3.
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for WaylandState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl SeatHandler for WaylandState {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, seat: wl_seat::WlSeat) {
        // One DataDevice per seat; overlay is single-seat in practice so
        // we keep just the first. The handle stays alive for the seat's
        // lifetime — drop happens in `remove_seat`.
        if self.data_device.is_none() {
            self.data_device = Some(self.data_device_manager.get_data_device(qh, &seat));
        }
    }

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer && self.pointer.is_none() {
            match self
                .seat_state
                .get_pointer_with_data(qh, &seat, PointerData::new(seat.clone()))
            {
                Ok(p) => self.pointer = Some(p),
                Err(e) => tracing::warn!("Failed to bind wl_pointer: {e}"),
            }
        }
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            // No repeat info: we let xkbcommon's update_modifiers drive
            // chord matching directly; per-press semantics are enough
            // for shortcut dispatch and text widgets compose their own
            // repeat via egui's input layer once it's wired in E.4.
            match self.seat_state.get_keyboard(qh, &seat, None) {
                Ok(k) => self.keyboard = Some(k),
                Err(e) => tracing::warn!("Failed to bind wl_keyboard: {e}"),
            }
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer {
            if let Some(p) = self.pointer.take() {
                p.release();
            }
        }
        if capability == Capability::Keyboard {
            if let Some(k) = self.keyboard.take() {
                k.release();
            }
        }
    }

    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {
        // Drop the DataDevice — its `Drop` impl releases the wayland
        // resource. Single-seat overlay means there's nothing else to
        // attach to.
        self.data_device = None;
    }
}

impl PointerHandler for WaylandState {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            match &event.kind {
                PointerEventKind::Enter { .. } => {
                    self.cursor_pos = Some((event.position.0 as f32, event.position.1 as f32));
                    self.pending_egui_events
                        .push(egui::Event::PointerMoved(egui::pos2(
                            event.position.0 as f32,
                            event.position.1 as f32,
                        )));
                }
                PointerEventKind::Leave { .. } => {
                    self.cursor_pos = None;
                    self.pending_egui_events.push(egui::Event::PointerGone);
                }
                PointerEventKind::Motion { .. } => {
                    self.cursor_pos = Some((event.position.0 as f32, event.position.1 as f32));
                    self.pending_egui_events
                        .push(egui::Event::PointerMoved(egui::pos2(
                            event.position.0 as f32,
                            event.position.1 as f32,
                        )));
                }
                PointerEventKind::Press { button, .. } => {
                    if let Some(b) = linux_button_to_egui(*button) {
                        if let Some((x, y)) = self.cursor_pos {
                            self.pending_egui_events.push(egui::Event::PointerButton {
                                pos: egui::pos2(x, y),
                                button: b,
                                pressed: true,
                                modifiers: modifiers_to_egui(self.last_modifiers),
                            });
                        }
                    }
                }
                PointerEventKind::Release { button, .. } => {
                    if let Some(b) = linux_button_to_egui(*button) {
                        if let Some((x, y)) = self.cursor_pos {
                            self.pending_egui_events.push(egui::Event::PointerButton {
                                pos: egui::pos2(x, y),
                                button: b,
                                pressed: false,
                                modifiers: modifiers_to_egui(self.last_modifiers),
                            });
                        }
                    }
                }
                PointerEventKind::Axis {
                    horizontal,
                    vertical,
                    ..
                } => {
                    // Wayland reports scroll in "discrete steps" (mouse
                    // wheel) and "absolute" (touchpad). Use `absolute`
                    // for fidelity; egui expects pixels per second-ish.
                    let dx = -horizontal.absolute as f32;
                    let dy = -vertical.absolute as f32;
                    if dx != 0.0 || dy != 0.0 {
                        self.pending_egui_events.push(egui::Event::MouseWheel {
                            unit: egui::MouseWheelUnit::Point,
                            delta: egui::vec2(dx, dy),
                            modifiers: egui::Modifiers::NONE,
                        });
                    }
                }
            }
        }
    }
}

/// Linux input event button codes (from `linux/input-event-codes.h`).
/// We only translate the three buttons egui actually handles.
fn linux_button_to_egui(code: u32) -> Option<egui::PointerButton> {
    match code {
        0x110 => Some(egui::PointerButton::Primary), // BTN_LEFT
        0x111 => Some(egui::PointerButton::Secondary), // BTN_RIGHT
        0x112 => Some(egui::PointerButton::Middle),  // BTN_MIDDLE
        _ => None,
    }
}

impl ProvidesRegistryState for WaylandState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

/// Accept dropped files via `wl_data_device`. The flow is:
///
/// 1. `enter`: compositor advertises the drag entry; we accept the
///    `text/uri-list` mime type so the source knows we'll consume it,
///    and set `DndAction::Copy` as our preferred action (overlay
///    doesn't move source files — it spawns sprites from copies).
/// 2. `motion`: cache the latest surface-local position so the drop
///    coordinate matches what the user sees.
/// 3. `drop_performed`: pull the receive-pipe out of the drag offer,
///    hand it off to a worker thread that reads + parses, and pushes
///    the resulting `Vec<PathBuf>` back over `drop_tx`. The main loop
///    drains `drop_rx` each frame.
impl DataDeviceHandler for WaylandState {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _data_device: &wayland_client::protocol::wl_data_device::WlDataDevice,
        x: f64,
        y: f64,
        _surface: &wl_surface::WlSurface,
    ) {
        self.last_drag_pos = Some((x as f32, y as f32));
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

    fn motion(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _data_device: &wayland_client::protocol::wl_data_device::WlDataDevice,
        x: f64,
        y: f64,
    ) {
        self.last_drag_pos = Some((x as f32, y as f32));
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _data_device: &wayland_client::protocol::wl_data_device::WlDataDevice,
    ) {
        self.last_drag_pos = None;
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
        let tx = self.drop_tx.clone();
        let guard = DropCounterGuard {
            counter: self.active_drop_workers.clone(),
        };
        std::thread::spawn(move || {
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
            if let Err(e) = capped.read_to_end(&mut buf) {
                tracing::warn!("Drop: read pipe failed: {e}");
                return;
            }
            let paths = parse_uri_list(&buf);
            if !paths.is_empty() {
                // `try_send` so a stuck consumer (main loop) can't
                // make us block here; bounded channel = bounded
                // memory.
                if let Err(e) = tx.try_send(paths) {
                    tracing::warn!("Drop: result queue full, dropping batch: {e}");
                }
            }
        });
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

delegate_compositor!(WaylandState);
delegate_data_device!(WaylandState);
delegate_keyboard!(WaylandState);
delegate_layer!(WaylandState);
delegate_output!(WaylandState);
delegate_pointer!(WaylandState);
delegate_registry!(WaylandState);
delegate_seat!(WaylandState);

// Type-side hint to keep the `Arc` import live — once a sub-phase moves
// state across threads, this becomes meaningful.
#[allow(dead_code)]
fn _arc_used(_x: Arc<()>) {}
