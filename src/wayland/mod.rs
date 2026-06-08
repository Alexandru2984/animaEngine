//! Native Wayland backend (in progress).
//!
//! At the moment this module only contains the capability probe — the app
//! still runs through winit + XWayland on every path. The full native
//! pipeline is being built in subsequent sub-phases of Phase 7:
//!
//! 1. ✅ Probe — detect WAYLAND_DISPLAY + `zwlr_layer_shell_v1` global.
//! 2. ⏳ Layer surface — create a fullscreen `Overlay` layer surface,
//!    bridge it to a wgpu surface.
//! 3. ⏳ Event translation — pointer / keyboard / frame callbacks → egui.
//! 4. ⏳ Click-through — `wl_surface::set_input_region(empty)` for the
//!    pass-through area, full region for the toggle button.
//! 5. ⏳ App integration — `run` picks the backend at startup based on
//!    `probe::detect`.

pub mod data_device;
pub mod egui_render;
pub mod keyboard;
pub mod layer_window;
pub mod probe;
pub mod run;

pub use layer_window::{InputRect, LayerWindow, WaylandState};
pub use probe::{detect, log_status, WaylandCapabilities};
pub use run::run_native;
