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
use crate::entity::Entity;
use crate::monitor::{self, MonitorInfo};
use crate::renderer::wgpu_renderer::SurfaceState;
use crate::window::overlay::OverlayPlatform;
use std::sync::Arc;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId, WindowLevel};

#[cfg(target_os = "linux")]
use winit::platform::wayland::WindowAttributesExtWayland;
#[cfg(target_os = "linux")]
use winit::platform::x11::{WindowAttributesExtX11, WindowType};

/// Snapshot the monitor topology from winit. Shared by startup
/// (`lifecycle.rs`) and the hotplug check so the two can't drift.
pub(super) fn snapshot_monitors(event_loop: &ActiveEventLoop) -> Vec<MonitorInfo> {
    let primary = event_loop.primary_monitor();
    event_loop
        .available_monitors()
        .enumerate()
        .map(|(i, m)| {
            let size = m.size();
            let pos = m.position();
            let is_primary = primary
                .as_ref()
                .is_some_and(|p| p.name() == m.name() && p.size() == size);
            let name = m.name().unwrap_or_else(|| format!("Display {i}"));
            MonitorInfo {
                name,
                x: pos.x,
                y: pos.y,
                width: size.width,
                height: size.height,
                scale_factor: m.scale_factor(),
                is_primary,
            }
        })
        .collect()
}

/// `true` when the entity's resolved monitor (pin first, then
/// centroid, with the standard fallbacks) is `target`. The one filter
/// every per-monitor draw list goes through.
pub(crate) fn entity_on_monitor(monitors: &[MonitorInfo], e: &Entity, target: &str) -> bool {
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
    pub x11_input: Option<Box<dyn OverlayPlatform>>,
}

/// The `HWND` behind a winit window, for the layered presentation path.
#[cfg(windows)]
fn win_hwnd(window: &Window) -> Option<isize> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match window.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(handle) => Some(handle.hwnd.get()),
        _ => None,
    }
}

/// Size (physical px) and optional desktop position for the **primary**
/// overlay window, derived from the monitor plan.
///
/// `plan.primary` names the monitor the window should cover: `Single`
/// picks it explicitly, `PerMonitor` pins it to the OS primary. `Span`
/// leaves it `None` and keeps the whole-desktop behaviour, so `fallback`
/// (the auto-detected resolution) applies.
///
/// The primary window was previously never positioned at all — only the
/// extras were — so selecting a non-primary monitor in `Single` mode
/// silently did nothing: the overlay stayed wherever the window manager
/// happened to drop it. An explicit non-zero size in the config still
/// wins over the monitor's own, since that is a deliberate user override.
pub(super) fn primary_window_geometry(
    plan: &monitor::WindowPlan,
    configured: (u32, u32),
    fallback: (u32, u32),
) -> ((u32, u32), Option<(i32, i32)>) {
    let size = if configured.0 != 0 && configured.1 != 0 {
        configured
    } else {
        match &plan.primary {
            Some(m) => (m.width, m.height),
            None => fallback,
        }
    };
    (size, plan.primary.as_ref().map(|m| (m.x, m.y)))
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

    /// Global origin of the primary window — the top-left of whichever
    /// monitor the plan puts it on, since entities live in global
    /// desktop coordinates. `Span` has no single monitor (`primary:
    /// None`) and keeps the identity origin.
    ///
    /// This used to short-circuit to `(0, 0)` whenever no extra windows
    /// existed, which silently mis-translated `Single` mode: that mode
    /// names one monitor and creates no extras, so a monitor sitting at
    /// a non-zero desktop offset got entity coordinates that were off by
    /// exactly that offset. It has to agree with where
    /// [`primary_window_geometry`] actually places the window.
    pub(super) fn primary_origin(&self) -> (f32, f32) {
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
            // Same split as the primary window: Windows renders offscreen
            // and blits, everywhere else gets a swapchain.
            #[cfg(windows)]
            let surface = {
                let Some(hwnd) = win_hwnd(&window) else {
                    tracing::warn!("Extra window on {} is not a Win32 window", mon.name);
                    continue;
                };
                SurfaceState::new_layered(&renderer.shared, hwnd, mon.width, mon.height)
            };
            #[cfg(not(windows))]
            let surface = {
                let surface = match renderer.shared.instance.create_surface(window.clone()) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::warn!("Couldn't create surface for {}: {e}", mon.name);
                        continue;
                    }
                };
                SurfaceState::new(&renderer.shared, surface, mon.width, mon.height)
            };

            let mut x11_input = crate::window::overlay::for_window(&window);
            if let Some(mgr) = &mut x11_input {
                // Fully click-through — the ⚙ toggle is a primary-
                // window affordance; extras reserve no corner (T.8).
                if let Err(e) = mgr.set_passthrough_total() {
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
            // A rebuild can happen while the overlay is hidden (a monitor
            // hotplug, or the user switching monitor mode). Without this
            // the fresh windows would appear on screen despite Hide being
            // in effect.
            if self.overlay_hidden {
                window.set_visible(false);
            }
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

    /// Hotplug detection (T.9). winit has no monitor-change event on
    /// X11, so the redraw cycle re-enumerates on the idle-heartbeat
    /// cadence and diffs. On change: update the snapshot, clear pins
    /// naming vanished monitors (they fall back to centroid
    /// resolution), rebuild the extra windows, toast the summary.
    pub(super) fn check_monitor_topology(&mut self, event_loop: &ActiveEventLoop) {
        if self.last_monitor_check.elapsed() < std::time::Duration::from_secs(2) {
            return;
        }
        self.last_monitor_check = std::time::Instant::now();

        let fresh = snapshot_monitors(event_loop);
        if fresh == self.monitors {
            return;
        }
        let old_names: std::collections::BTreeSet<&str> =
            self.monitors.iter().map(|m| m.name.as_str()).collect();
        let new_names: std::collections::BTreeSet<&str> =
            fresh.iter().map(|m| m.name.as_str()).collect();

        for gone in old_names.difference(&new_names) {
            let mut cleared = 0usize;
            for e in &mut self.scene.entities {
                if e.monitor.as_deref() == Some(*gone) {
                    e.monitor = None;
                    cleared += 1;
                }
            }
            if cleared > 0 {
                self.config_dirty = true;
            }
            let mut args = fluent::FluentArgs::new();
            args.set("name", gone.to_string());
            args.set("n", cleared as i64);
            self.toasts
                .warn(crate::i18n::t_args("monitor-unplugged-toast", &args));
            tracing::info!("Monitor {gone} disconnected; {cleared} pins cleared");
        }
        for added in new_names.difference(&old_names) {
            let mut args = fluent::FluentArgs::new();
            args.set("name", added.to_string());
            self.toasts
                .info(crate::i18n::t_args("monitor-plugged-toast", &args));
            tracing::info!("Monitor {added} connected");
        }

        crate::monitor::log_topology(&fresh);
        self.monitors = fresh;
        self.rebuild_extra_windows(event_loop);
        self.request_redraw_all();
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
                &self.scene.groups,
                self.edit_mode,
                selected_id.as_deref(),
                origin,
            ) {
                Ok(output) => slot.surface.present(&renderer.shared, output),
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
            &self.scene.groups,
            self.edit_mode,
            selected_id.as_deref(),
            origin,
        ) {
            slot.surface.present(&renderer.shared, output);
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
                    mgr.set_passthrough_total()
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

    /// Window-awareness poll (~300 ms): refresh the desktop-window
    /// platform set the physics floor resolution reads. Lazy-connects
    /// the EWMH watcher on first need; on sessions without an X server
    /// the single failed probe disables further attempts. Disabling
    /// the config knob clears the platforms exactly once.
    pub(super) fn poll_window_platforms(&mut self) {
        const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(300);

        if !self.config.global.window_awareness {
            if self.window_platforms_active {
                self.scene.set_window_platforms(Vec::new());
                self.window_platforms_active = false;
            }
            return;
        }

        if self.last_window_poll.elapsed() < POLL_INTERVAL && self.window_platforms_active {
            return;
        }
        self.last_window_poll = std::time::Instant::now();

        // EWMH gives global window geometry only on X11. Off unix (and on
        // native Wayland) there is no such query, so the feature stays
        // inert and physics resolve against the screen floor.
        #[cfg(unix)]
        {
            if self.window_watcher.is_none() {
                if self.window_watcher_probe_done {
                    return;
                }
                self.window_watcher_probe_done = true;
                self.window_watcher = crate::window::x11_windows::WindowWatcher::new();
                if self.window_watcher.is_none() {
                    tracing::info!(
                        "window_awareness: no X server reachable — feature inert this session"
                    );
                    return;
                }
            }

            if let Some(watcher) = &self.window_watcher {
                let platforms = watcher.snapshot();
                tracing::trace!("window_awareness: {} platform(s)", platforms.len());
                self.scene.set_window_platforms(platforms);
                self.window_platforms_active = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::primary_window_geometry;
    use crate::monitor::{MonitorInfo, WindowPlan};

    fn mon(name: &str, x: i32, y: i32, w: u32, h: u32) -> MonitorInfo {
        MonitorInfo {
            name: name.to_string(),
            x,
            y,
            width: w,
            height: h,
            scale_factor: 1.0,
            is_primary: false,
        }
    }

    /// Span names no monitor, so the auto-detected resolution applies and
    /// the window stays unpositioned — the pre-existing behaviour.
    #[test]
    fn span_keeps_detected_size_and_no_position() {
        let plan = WindowPlan {
            primary: None,
            extras: Vec::new(),
        };
        let (size, pos) = primary_window_geometry(&plan, (0, 0), (1920, 1080));
        assert_eq!(size, (1920, 1080));
        assert_eq!(pos, None);
    }

    /// The regression this fixes: Single names a monitor at a non-zero
    /// desktop offset, and the primary window has to actually go there.
    #[test]
    fn single_places_the_window_on_the_named_monitor() {
        let plan = WindowPlan {
            primary: Some(mon("HDMI-A-1", 1920, 0, 2560, 1440)),
            extras: Vec::new(),
        };
        let (size, pos) = primary_window_geometry(&plan, (0, 0), (1920, 1080));
        assert_eq!(size, (2560, 1440), "sized to the chosen monitor");
        assert_eq!(pos, Some((1920, 0)), "and placed at its desktop origin");
    }

    /// An explicit config size is a deliberate override and still wins,
    /// but the plan continues to decide *where* the window goes.
    #[test]
    fn configured_size_overrides_the_monitor_but_not_the_position() {
        let plan = WindowPlan {
            primary: Some(mon("HDMI-A-1", 1920, 0, 2560, 1440)),
            extras: Vec::new(),
        };
        let (size, pos) = primary_window_geometry(&plan, (800, 600), (1920, 1080));
        assert_eq!(size, (800, 600));
        assert_eq!(pos, Some((1920, 0)));
    }

    /// A half-specified config size (one axis zero) is not a valid
    /// override and must fall through to the monitor's own size.
    #[test]
    fn partial_configured_size_falls_through() {
        let plan = WindowPlan {
            primary: Some(mon("eDP-1", 0, 0, 1366, 768)),
            extras: Vec::new(),
        };
        let (size, _) = primary_window_geometry(&plan, (800, 0), (1920, 1080));
        assert_eq!(size, (1366, 768));
    }
}
