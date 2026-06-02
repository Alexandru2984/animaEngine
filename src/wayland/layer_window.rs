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

use crate::error::{AnimaError, Result};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_registry,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        WaylandSurface,
    },
};
use std::sync::Arc;
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_output, wl_surface},
    Connection, EventQueue, QueueHandle,
};

/// What `try_create` produces. The caller stores it and drives the event
/// loop manually (we don't spawn anything internally).
pub struct LayerWindow {
    pub connection: Connection,
    pub event_queue: EventQueue<WaylandState>,
    pub state: WaylandState,
    pub wgpu_surface: wgpu::Surface<'static>,
    pub wgpu_instance: wgpu::Instance,
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
        let output_state = OutputState::new(&globals, &qh);
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
            _compositor: compositor,
            _layer_shell: layer_shell,
            layer,
            pending_size: None,
            close_requested: false,
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
            wgpu_surface,
            wgpu_instance,
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

/// State threaded through every sctk callback. `delegate_*` macros below
/// wire each protocol's handler to the matching `impl` block.
pub struct WaylandState {
    pub registry_state: RegistryState,
    pub output_state: OutputState,
    _compositor: CompositorState,
    _layer_shell: LayerShell,
    pub layer: LayerSurface,
    /// Last size we got from a `configure` event — drives wgpu resize.
    pub pending_size: Option<(u32, u32)>,
    /// True after a layer-surface `closed` event. Caller polls this to
    /// know when to tear down.
    pub close_requested: bool,
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

impl ProvidesRegistryState for WaylandState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}

delegate_compositor!(WaylandState);
delegate_layer!(WaylandState);
delegate_output!(WaylandState);
delegate_registry!(WaylandState);

// Type-side hint to keep the `Arc` import live — once 7.3 adds pointer /
// keyboard handlers, those structures will be shared between threads.
#[allow(dead_code)]
fn _arc_used(_x: Arc<()>) {}
