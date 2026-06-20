//! Native Wayland backend (wlroots / `zwlr_layer_shell_v1`).
//!
//! Opt-in via `ANIMA_USE_WAYLAND_NATIVE=1`, selected at startup by
//! `probe::detect`; everywhere else the app runs through winit +
//! XWayland. Brought to parity with the X11 path over the 0.5 *E*
//! phases — the full pipeline is wired:
//!
//! 1. ✅ Probe — detect `WAYLAND_DISPLAY` + `zwlr_layer_shell_v1`.
//! 2. ✅ Layer surface — a fullscreen `Overlay` layer surface bridged to
//!    a wgpu surface (`layer_window`).
//! 3. ✅ Event translation — pointer / keyboard (xkbcommon) / frame
//!    callbacks → egui (`keyboard`, `egui_render`).
//! 4. ✅ Click-through — `wl_surface::set_input_region` (empty for the
//!    pass-through area, full region for the toggle button).
//! 5. ✅ App integration — `run::run_native` drives the loop.
//!
//! File drops (`wl_data_device` / `text/uri-list`, `data_device`) pass
//! through the *same* `pre_validate_dropped_file` gate as the X11 path.
//! Remaining gaps are feature/UX, not correctness: the asset-library
//! index isn't surfaced here yet and per-monitor distribution is
//! single-surface.

pub mod data_device;
pub mod egui_render;
pub mod keyboard;
pub mod layer_window;
pub mod probe;
pub mod run;

pub use layer_window::{InputRect, LayerWindow, WaylandState};
pub use probe::{detect, log_status, WaylandCapabilities};
pub use run::run_native;
