//! ApplicationHandler::resumed body. Extracted in H.5 so the main
//! module stops carrying ~180 lines of one-shot initialisation.
//!
//! Resumed runs once after the event loop starts (and again on
//! suspend/resume on platforms that emit it — desktop Linux doesn't).
//! It builds the monitor snapshot, scans the asset library,
//! creates the transparent always-on-top window, installs the X11
//! input shape, and brings up wgpu + egui.

use super::App;
use crate::renderer::wgpu_renderer::WgpuRenderer;
use crate::ui::EguiRenderer;
use std::sync::Arc;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowLevel};

#[cfg(target_os = "linux")]
use winit::platform::x11::{WindowAttributesExtX11, WindowType};

#[cfg(target_os = "linux")]
use winit::platform::wayland::WindowAttributesExtWayland;

impl App {
    pub(super) fn handle_resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return; // Already created
        }

        tracing::info!("Creating window...");

        // One-time notice if the previous session left a crash report
        // behind — launched from a desktop icon the panic text never
        // reaches a terminal, so this toast is the only breadcrumb.
        if let Some(report) = crate::crash::unnotified_report() {
            let mut args = fluent::FluentArgs::new();
            args.set("path", report.display().to_string());
            self.toasts
                .warn(crate::i18n::t_args("crash-report-found-toast", &args));
            crate::crash::mark_notified(&report);
        }

        // Snapshot the monitor topology once so the rest of the engine
        // can use the renderer-agnostic MonitorInfo instead of holding
        // a borrow on the event loop. The picker UI in C.2 will read
        // this list; for now we log it and keep the data ready.
        let monitors = super::windows::snapshot_monitors(event_loop);
        crate::monitor::log_topology(&monitors);
        // We force the X11 backend, so a Wayland session means XWayland —
        // where fractional/mixed scaling desyncs the XShape click-through
        // region. Warn once (handle_resumed runs once) rather than leave
        // misaligned clicks unexplained.
        let on_xwayland = matches!(
            crate::window::platform::detect_display_server(),
            crate::window::platform::DisplayServer::Wayland
        );
        crate::monitor::warn_xwayland_xshape_scaling(on_xwayland, &monitors);
        self.monitors = monitors;

        // Discover + load + merge-scan the asset library. Errors are
        // logged but never fatal — an empty library is fine.
        if let Some(root) = crate::asset_library::discover_asset_root() {
            let index_path = crate::asset_library::LibraryIndex::default_path();
            let mut idx = crate::asset_library::LibraryIndex::load(&index_path);
            let scanned = crate::asset_library::scan(&root);
            let scanned_count = scanned.len();
            idx.merge_scan(scanned);
            if let Err(e) = idx.save(&index_path) {
                tracing::warn!("Failed to persist library.toml: {e}");
            }
            tracing::info!(
                "Asset library at {}: {} indexed ({} from this scan)",
                crate::drop_validate::redact_path(&root),
                idx.assets.len(),
                scanned_count,
            );
            tracing::debug!("Asset library full root: {}", root.display());
            // U.5: fill the thumbnail cache off-thread; the grid
            // picks thumbs up from disk as they appear.
            {
                let root = root.clone();
                let index = idx.clone();
                let spawned = std::thread::Builder::new()
                    .name("anima-thumbs".into())
                    .spawn(move || {
                        crate::asset_library::generate_missing_thumbnails(&root, &index);
                    });
                if let Err(e) = spawned {
                    tracing::warn!("Thumbnail thread failed to spawn: {e}");
                }
            }
            self.library = Some(idx);
            self.library_root = Some(root);
        } else {
            tracing::info!("No asset library root found; Library tab will show empty state.");
        }

        // Auto-detected resolution, used when the config doesn't pin a
        // size and the plan doesn't name a monitor (i.e. Span).
        let detected = if let Some(monitor) = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next())
        {
            let size = monitor.size();
            tracing::info!(
                "Auto-detected monitor resolution: {}x{}",
                size.width,
                size.height
            );
            (size.width, size.height)
        } else {
            tracing::warn!("Could not detect monitor resolution, falling back to 1920x1080");
            (1920u32, 1080u32)
        };

        // Derive the primary window's geometry from the monitor plan, so
        // `Single`/`PerMonitor` actually put it on the monitor they name.
        // `self.monitors` was snapshotted above, before this point.
        let plan = crate::monitor::plan_windows(&self.config.global.monitor_mode, &self.monitors);
        let ((win_w, win_h), win_pos) = super::windows::primary_window_geometry(
            &plan,
            (
                self.config.global.window_width,
                self.config.global.window_height,
            ),
            detected,
        );

        // Build window attributes: transparent, borderless, always-on-top.
        // `win_w`/`win_h` are PHYSICAL pixels (from `monitor.size()`, or a
        // user-set config value meant to match a screen resolution), so the
        // inner size must be a `PhysicalSize`. Passing `LogicalSize` made
        // winit re-apply the monitor's scale factor: on any non-1.0 scale
        // (HiDPI laptops, fractional scaling, virtual displays like a
        // Proxmox console reporting scale≈1.08) the window came out scale×
        // too big and overhung the screen — pushing the top-right ⚙ toggle
        // button, the sole pass-through-mode affordance, off-screen.
        // Matches the extra-window path in `windows.rs`, which already
        // uses `PhysicalSize`.
        let window_attrs = Window::default_attributes()
            .with_title("animaEngine")
            .with_transparent(true)
            .with_decorations(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_inner_size(winit::dpi::PhysicalSize::new(win_w, win_h));
        // Place it on the planned monitor. Only the extra windows used to
        // be positioned, so picking a non-primary monitor in Single mode
        // left the overlay wherever the WM dropped it.
        let window_attrs = match win_pos {
            Some((x, y)) => {
                tracing::info!("Placing primary overlay at {x},{y} ({win_w}x{win_h})");
                window_attrs.with_position(winit::dpi::PhysicalPosition::new(x, y))
            }
            None => window_attrs,
        };

        // X11-specific: Use Normal type (NOT Dock).
        // Dock windows on XWayland/Mutter don't receive mouse events.
        // EWMH hints (ABOVE, SKIP_TASKBAR, etc.) are applied by X11InputManager.
        //
        // Plus: set X11 WM_CLASS *and* Wayland app_id to "animaEngine" so
        // the launcher entry (StartupWMClass=animaEngine in the .desktop)
        // matches the runtime window — without this, the dock shows a
        // separate "running" entry next to the pinned one with a
        // placeholder icon. Both APIs accept the same identifier; we
        // pass it identically to keep X11 and Wayland behaviour aligned.
        #[cfg(target_os = "linux")]
        let window_attrs = {
            // Disambiguated explicitly because both X11 and Wayland
            // extension traits provide a `with_name`-style API; using
            // UFCS keeps the intent clear and stops rustc from picking
            // the wrong one.
            let attrs = WindowAttributesExtX11::with_name(
                window_attrs.with_x11_window_type(vec![WindowType::Normal]),
                "animaEngine",
                "animaEngine",
            );
            WindowAttributesExtWayland::with_name(attrs, "animaEngine", "animaEngine")
        };

        match event_loop.create_window(window_attrs) {
            Ok(window) => {
                let window = Arc::new(window);
                tracing::info!(
                    "Window created: {:?} ({}x{})",
                    window.id(),
                    window.inner_size().width,
                    window.inner_size().height
                );

                // Create pooled X11 input manager (single connection)
                let mut x11_mgr = crate::window::overlay::for_window(&window);
                if let Some(ref mut mgr) = x11_mgr {
                    // Set initial input shape: click-through except toggle
                    // button. Physical pixels — the constant is in egui
                    // points, see `toggle_button_px`.
                    let button_px = super::toggle_button_px(window.scale_factor());
                    if let Err(e) = mgr.set_passthrough_with_button(button_px) {
                        tracing::warn!("Failed to set initial input shape: {}", e);
                        let _ = window.set_cursor_hittest(false);
                    }
                } else {
                    tracing::warn!(
                        "X11InputManager not available. Falling back to set_cursor_hittest."
                    );
                    let _ = window.set_cursor_hittest(false);
                }
                self.x11_input = x11_mgr;

                // Initialize wgpu renderer
                match WgpuRenderer::new(window.clone()) {
                    Ok(mut renderer) => {
                        // Create initial textures for all entities
                        for entity in &self.scene.entities {
                            renderer.ensure_texture(entity);
                        }
                        // Clear texture_dirty flags after initial upload
                        for entity in &mut self.scene.entities {
                            entity.texture_dirty = false;
                        }

                        // egui wraps the existing wgpu device + queue.
                        let ui = EguiRenderer::new(
                            &renderer.shared.device,
                            renderer.shared.surface_format,
                            window.clone(),
                            self.config.global.theme,
                        );
                        self.ui = Some(ui);
                        self.renderer = Some(renderer);
                        tracing::info!("wgpu + egui renderers initialized");
                    }
                    Err(e) => {
                        tracing::error!("Failed to initialize wgpu renderer: {}", e);
                        self.save_and_exit(event_loop);
                        return;
                    }
                }

                self.window = Some(window);
                // PerMonitor: spawn the extra overlay windows now that
                // the shared GPU state exists (T.6).
                self.rebuild_extra_windows(event_loop);
                // Kick the paced render loop. Most platforms emit an
                // initial RedrawRequested for a new window, but the
                // pacing chain must not depend on that courtesy.
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            Err(e) => {
                tracing::error!("Failed to create window: {}", e);
                self.save_and_exit(event_loop);
            }
        }
    }
}
