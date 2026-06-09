//! Small sctk handler impls bundled together. Extracted in K.5.
//!
//! These five handlers (`LayerShellHandler`, `CompositorHandler`,
//! `OutputHandler`, `SeatHandler`, `ProvidesRegistryState`) are each
//! short — most methods are no-ops because the overlay doesn't react
//! to scale changes / transform / surface-leave / output topology
//! deltas at the protocol layer. They live together to avoid one
//! 30-line file per trait.
//!
//! The two non-trivial bits:
//! - `LayerShellHandler::configure` caches the size so the next
//!   frame can resize wgpu.
//! - `SeatHandler::new_capability` binds the pointer / keyboard the
//!   first time a seat advertises them; pointer events route to
//!   `pointer_handler`, keyboard to `keyboard_handler`.

use super::state::WaylandState;
use smithay_client_toolkit::{
    compositor::CompositorHandler,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{pointer::PointerData, Capability, SeatHandler, SeatState},
    shell::wlr_layer::{LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
};
use wayland_client::{
    protocol::{wl_output, wl_seat, wl_surface},
    Connection, QueueHandle,
};

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

impl ProvidesRegistryState for WaylandState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}
