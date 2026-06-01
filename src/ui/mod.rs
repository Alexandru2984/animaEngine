//! egui-based immediate-mode UI integrated with the wgpu sprite pipeline.
//!
//! `EguiRenderer` is the bridge between winit events, the egui context, and
//! the wgpu renderer. App constructs it once in `resumed()`, forwards window
//! events through `handle_event` before running its own input logic, and
//! invokes `render` after `WgpuRenderer::render` so the UI sits on top of
//! sprites.
//!
//! UI itself is defined in `panels` and friends — `EguiRenderer` does not
//! know what's being painted, only how to paint it.

mod egui_renderer;
pub mod panels;

pub use egui_renderer::EguiRenderer;
