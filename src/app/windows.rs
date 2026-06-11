//! Extra overlay windows for `MonitorMode::PerMonitor` (T.6).
//!
//! The *primary* window (created in `lifecycle.rs`) always exists and
//! hosts egui. In PerMonitor mode every other monitor gets an extra
//! sprite-only window, tracked here in a `WindowId → WindowSlot`
//! registry. The plan of which monitors get windows is the pure
//! `monitor::plan_windows`; this module is the imperative shell
//! around it.
//!
//! Entity coordinates are global desktop pixels; each window renders
//! the entities whose resolved monitor matches its own, translated by
//! the monitor's origin (see `SurfaceState::render`'s `origin`).

use super::App;
use crate::constants::TOGGLE_BUTTON_SIZE;
use crate::entity::Entity;
use crate::monitor::{self, MonitorInfo};
use crate::renderer::wgpu_renderer::SurfaceState;
use crate::window::x11_input::X11InputManager;
use std::sync::Arc;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId, WindowLevel};

#[cfg(target_os = "linux")]
use winit::platform::wayland::WindowAttributesExtWayland;
#[cfg(target_os = "linux")]
use winit::platform::x11::{WindowAttributesExtX11, WindowType};

/// `true` when the entity's resolved monitor (pin first, then
/// centroid, with the standard fallbacks) is `target`. The one filter
/// every per-monitor draw list goes through.
pub(super) fn entity_on_monitor(monitors: &[MonitorInfo], e: &Entity, target: &str) -> bool {
    let cx = e.x + e.scaled_width() / 2.0;
    let cy = e.y + e.scaled_height() / 2.0;
    monitor::resolve_monitor_for_position(monitors, cx, cy, e.monitor.as_deref())
        .map(|m| m.name == target)
        .unwrap_or(false)
}

/// One extra (non-primary) overlay window.
pub(super) struct WindowSlot {
    pub window: Arc<Window>,
    pub surface: SurfaceState,
    pub monitor: MonitorInfo,
    pub x11_input: Option<X11InputManager>,
}

/// Overlay window attributes shared by the primary and the extras.
/// Factored from `lifecycle.rs` so the two spawn sites can't drift.
pub(super) fn overlay_window_attrs(width: u32, height: u32) -> winit::window::WindowAttributes {
    let attrs = Window::default_attributes()
        .with_title("animaEngine")
        .with_transparent(true)
        .with_decorations(false)
        .with_window_level(WindowLevel::AlwaysOnTop)
        .with_inner_size(winit::dpi::PhysicalSize::new(width, height));

    // X11: Normal type (NOT Dock — Dock windows on XWayland/Mutter
    // don't receive mouse events). WM_CLASS + Wayland app_id both set
    // to "animaEngine" so the launcher entry matches the runtime
    // window (StartupWMClass in the .desktop).
    #[cfg(target_os = "linux")]
    let attrs = {
        let attrs = WindowAttributesExtX11::with_name(
            attrs.with_x11_window_type(vec![WindowType::Normal]),
            "animaEngine",
            "animaEngine",
        );
        WindowAttributesExtWayland::with_name(attrs, "animaEngine", "animaEngine")
    };
    attrs
}

impl App {
    /// `true` when PerMonitor extras are live — the render loop uses
    /// this to decide whether draw lists need per-monitor filtering.
    pub(super) fn has_extra_windows(&self) -> bool {
        !self.extra_windows.is_empty()
    }

    /// Global origin of the primary window. Identity for the
    /// single-window modes (pre-0.6 behaviour); the primary monitor's
    /// top-left in PerMonitor mode (entities live in global coords).
    pub(super) fn primary_origin(&self) -> (f32, f32) {
        if !self.has_extra_windows() {
            return (0.0, 0.0);
        }
        monitor::plan_windows(&self.config.global.monitor_mode, &self.monitors)
            .primary
            .map(|m| (m.x as f32, m.y as f32))
            .unwrap_or((0.0, 0.0))
    }

    /// Spawn (or re-spawn) the extra windows for the current mode +
    /// topology. Idempotent: clears the registry first, so it doubles
    /// as the mode-switch rebuild (and, in T.9, the hotplug rebuild).
    pub(super) fn rebuild_extra_windows(&mut self, event_loop: &ActiveEventLoop) {
        self.extra_windows.clear();
        self.last_monitor_mode = self.config.global.monitor_mode.clone();

        let Some(renderer) = &self.renderer else {
            return;
        };
        let plan = monitor::plan_windows(&self.config.global.monitor_mode, &self.monitors);
        for mon in plan.extras {
            let attrs = overlay_window_attrs(mon.width, mon.height)
                .with_position(winit::dpi::PhysicalPosition::new(mon.x, mon.y));
            let window = match event_loop.create_window(attrs) {
                Ok(w) => Arc::new(w),
                Err(e) => {
                    tracing::warn!("Couldn't create overlay window for {}: {e}", mon.name);
                    continue;
                }
            };
            let surface = match renderer.shared.instance.create_surface(window.clone()) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("Couldn't create surface for {}: {e}", mon.name);
                    continue;
                }
            };
            let surface = SurfaceState::new(&renderer.shared, surface, mon.width, mon.height);

            let mut x11_input = X11InputManager::new(&window);
            if let Some(mgr) = &mut x11_input {
                if let Err(e) = mgr.set_passthrough_with_button(TOGGLE_BUTTON_SIZE) {
                    tracing::warn!("Input shape on {} failed: {e}", mon.name);
                    let _ = window.set_cursor_hittest(false);
                }
            } else {
                let _ = window.set_cursor_hittest(false);
            }

            tracing::info!(
                "Spawned overlay window on {} ({}x{} at {},{})",
                mon.name,
                mon.width,
                mon.height,
                mon.x,
                mon.y
            );
            self.extra_windows.insert(
                window.id(),
                WindowSlot {
                    window,
                    surface,
                    monitor: mon,
                    x11_input,
                },
            );
        }
    }

    /// Rebuild when the user flipped the monitor mode in Appearance.
    /// Called from the redraw handler (it owns an `ActiveEventLoop`).
    pub(super) fn rebuild_windows_if_mode_changed(&mut self, event_loop: &ActiveEventLoop) {
        if self.last_monitor_mode != self.config.global.monitor_mode {
            tracing::info!(
                "Monitor mode changed {:?} → {:?}; rebuilding overlay windows",
                self.last_monitor_mode,
                self.config.global.monitor_mode
            );
            self.rebuild_extra_windows(event_loop);
            self.request_redraw_all();
        }
    }

    /// Render sprites to every extra window. Runs inside the primary
    /// redraw cycle (one pacing domain for T.6 — per-window pacing is
    /// noted as a follow-up in the architecture record).
    pub(super) fn render_extra_windows(&mut self) {
        if self.extra_windows.is_empty() {
            return;
        }
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        let selected_id = self
            .selection
            .selected_index()
            .and_then(|idx| self.scene.entities.get(idx))
            .map(|e| e.id.clone());

        let visible = self.scene.visible_entities();
        for slot in self.extra_windows.values_mut() {
            let drawn: Vec<&Entity> = visible
                .iter()
                .copied()
                .filter(|e| entity_on_monitor(&self.monitors, e, &slot.monitor.name))
                .collect();
            let origin = (slot.monitor.x as f32, slot.monitor.y as f32);
            match slot.surface.render(
                &renderer.shared,
                &drawn,
                self.edit_mode,
                selected_id.as_deref(),
                origin,
            ) {
                Ok(output) => output.present(),
                Err(wgpu::SurfaceError::Lost) => {
                    let (w, h) = (slot.surface.window_width, slot.surface.window_height);
                    slot.surface.resize(&renderer.shared, w, h);
                }
                Err(e) => {
                    tracing::warn!("Render error on {}: {e:?}", slot.monitor.name);
                }
            }
        }
    }

    /// Re-render one extra window (compositor expose / its own
    /// RedrawRequested). Sprite-only; no scene tick.
    pub(super) fn render_one_extra(&mut self, id: WindowId) {
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        let selected_id = self
            .selection
            .selected_index()
            .and_then(|idx| self.scene.entities.get(idx))
            .map(|e| e.id.clone());
        let visible = self.scene.visible_entities();
        let Some(slot) = self.extra_windows.get_mut(&id) else {
            return;
        };
        let drawn: Vec<&Entity> = visible
            .iter()
            .copied()
            .filter(|e| entity_on_monitor(&self.monitors, e, &slot.monitor.name))
            .collect();
        let origin = (slot.monitor.x as f32, slot.monitor.y as f32);
        if let Ok(output) = slot.surface.render(
            &renderer.shared,
            &drawn,
            self.edit_mode,
            selected_id.as_deref(),
            origin,
        ) {
            output.present();
        }
    }

    /// Wake every overlay window, not just the primary.
    pub(super) fn request_redraw_all(&self) {
        self.request_redraw();
        for slot in self.extra_windows.values() {
            slot.window.request_redraw();
        }
    }

    /// Re-apply XShape input regions on every extra window — called
    /// alongside the primary's `reapply_input_shape` on mode toggles.
    pub(super) fn reapply_extra_input_shapes(&mut self) {
        let edit = self.edit_mode;
        for slot in self.extra_windows.values_mut() {
            if let Some(mgr) = &mut slot.x11_input {
                let result = if edit {
                    mgr.set_full_input()
                } else {
                    mgr.set_passthrough_with_button(TOGGLE_BUTTON_SIZE)
                };
                if let Err(e) = result {
                    tracing::warn!("Input shape on {} failed: {e}", slot.monitor.name);
                    let _ = slot.window.set_cursor_hittest(edit);
                }
            } else {
                let _ = slot.window.set_cursor_hittest(edit);
            }
        }
    }
}
