//! Native Wayland run loop — opt-in via `ANIMA_USE_WAYLAND_NATIVE=1`.
//!
//! This is the proof-of-concept stitching everything in `src/wayland/`
//! together: layer surface (7.2), pointer translation (7.3), input region
//! (7.4), and a sprite-only render loop driven by `WgpuRenderer`.
//!
//! ## What works
//!
//! - Fullscreen overlay on wlroots compositors (sway, Hyprland, river, …).
//! - Animated sprite rendering for every entity in `Scene`.
//! - Pointer events translated and consumed by egui through the
//!   `WaylandEguiRenderer` (the ⚙ toggle button is the active UI
//!   surface here; the full settings panel parity lands next).
//! - Keyboard events with xkbcommon-decoded keysyms + modifier
//!   tracking. UTF-8 text already composed via xkb's dead-key engine
//!   arrives as `egui::Event::Text` for widget input.
//! - File drops via `wl_data_device` (`text/uri-list`) — a worker
//!   thread drains the receive pipe and the main loop routes each
//!   path through the same `Scene::add_entity_from_path` validation
//!   the X11 path uses.
//! - Edit-mode toggle: bound chord (`Action::ToggleEditMode`) flips
//!   the click-through input region in lock-step.
//!
//! ## What doesn't (yet)
//!
//! - **Settings panel + context menu + toasts** — only the ⚙ button is
//!   wired; the full UI surface ships in the next sub-phase.
//! - **Per-monitor placement** — the layer surface attaches to
//!   whichever output the compositor picks.

use crate::constants::TOGGLE_BUTTON_SIZE;
use crate::error::{AnimaError, Result};
use crate::keybindings::{Action, KeyBindings, KeyChord};
use crate::renderer::wgpu_renderer::WgpuRenderer;
use crate::scene::Scene;
use crate::ui::{panels, theme::Theme};
use crate::wayland::egui_render::WaylandEguiRenderer;
use crate::wayland::layer_window::{InputRect, LayerWindow};
use std::time::Duration;

/// Drive a native-Wayland session end-to-end.
///
/// Returns `Err` only when initialization fails (no compositor, missing
/// globals, wgpu surface creation refused, …). The caller falls back to
/// the X11 path on error. A successful return means the user closed
/// the layer surface (or the compositor disconnected).
#[tracing::instrument(skip(scene, keybindings))]
pub fn run_native(mut scene: Scene, keybindings: KeyBindings) -> Result<()> {
    let mut layer = LayerWindow::try_create()?;
    let (width, height) = layer
        .size
        .ok_or_else(|| AnimaError::other("compositor produced no initial size"))?;

    // Take the wgpu instance + surface out of the LayerWindow and hand
    // them to the renderer. The wl_surface backing the wgpu surface
    // stays alive inside `layer.state.layer` for the rest of this scope;
    // ordering guarantees `renderer` is dropped before `layer` (Rust
    // drops locals in reverse declaration order).
    let instance = layer
        .wgpu_instance
        .take()
        .ok_or_else(|| AnimaError::other("LayerWindow missing wgpu instance"))?;
    let surface = layer
        .wgpu_surface
        .take()
        .ok_or_else(|| AnimaError::other("LayerWindow missing wgpu surface"))?;
    let mut renderer = WgpuRenderer::from_instance_surface(instance, surface, width, height)?;
    let mut egui_renderer =
        WaylandEguiRenderer::new(&renderer.device, renderer.config.format, Theme::default());
    tracing::info!("Native Wayland renderer initialized ({width}×{height})");

    // Start in pass-through mode with the ⚙ button cutout — same default
    // as the X11 path.
    layer.set_input_region(Some(InputRect::toggle_button_corner(
        width,
        TOGGLE_BUTTON_SIZE,
    )))?;

    // Upload textures once for the initial scene.
    for entity in &scene.entities {
        renderer.ensure_texture(entity);
    }
    for entity in &mut scene.entities {
        entity.texture_dirty = false;
    }

    // ── Main loop ──
    // `blocking_dispatch` waits for compositor events; the 16-ms sleep
    // below ensures animations keep ticking even when the compositor
    // doesn't push events at us.
    loop {
        layer
            .event_queue
            .blocking_dispatch(&mut layer.state)
            .map_err(|e| AnimaError::other(format!("wayland dispatch: {e}")))?;

        if layer.state.close_requested {
            tracing::info!("Layer surface closed by compositor — exiting.");
            break;
        }

        // Pick up any resize the compositor sent us.
        if let Some((new_w, new_h)) = layer.state.pending_size.take() {
            if new_w != renderer.window_width || new_h != renderer.window_height {
                renderer.resize(new_w, new_h);
                layer.set_input_region(Some(InputRect::toggle_button_corner(
                    new_w,
                    TOGGLE_BUTTON_SIZE,
                )))?;
                tracing::info!("Layer surface resized to {new_w}×{new_h}");
            }
        }

        // Drain pointer + keyboard events. Until egui paint lands
        // (E.4) we don't have a UI consumer, but we already need to
        // detect the `Action::ToggleEditMode` chord so click-through
        // can flip in lock-step. Scan key-press events, match against
        // the user's bindings, dispatch the few actions that make
        // sense without a UI thread (just edit mode for now).
        // Process any files dropped over the surface (E.3). Each path
        // routes through the same `add_entity_from_path` validation
        // gate as the X11 drag-drop path, so frame caps + extension
        // whitelist still apply.
        let drop_pos = layer.last_drag_pos();
        for path in layer.drain_dropped_files() {
            let (x, y) = drop_pos.unwrap_or((
                renderer.window_width as f32 / 2.0,
                renderer.window_height as f32 / 2.0,
            ));
            match scene.add_entity_from_path(&path, x, y) {
                Ok(idx) => {
                    renderer.ensure_texture(&scene.entities[idx]);
                    scene.entities[idx].texture_dirty = false;
                    tracing::info!(
                        "Spawned entity from drop: {} at ({x:.0}, {y:.0})",
                        path.display()
                    );
                }
                Err(e) => {
                    tracing::warn!("Drop rejected for {}: {e}", path.display());
                }
            }
        }

        let events = layer.drain_egui_events();
        for event in &events {
            let egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = event
            else {
                continue;
            };
            let Some(chord) = KeyChord::from_egui(*key, *modifiers) else {
                continue;
            };
            if let Some(Action::ToggleEditMode) = keybindings.lookup(chord) {
                let new_mode = !layer.state.edit_mode;
                match layer.set_edit_mode(new_mode, TOGGLE_BUTTON_SIZE) {
                    Ok(()) => tracing::info!(
                        "Edit mode {} (Wayland)",
                        if new_mode { "on" } else { "off" }
                    ),
                    Err(e) => {
                        tracing::warn!("Failed to flip input region on edit toggle: {e}")
                    }
                }
            }
        }

        // Tick the simulation. screen_w / screen_h match the surface so
        // walk-around behaviors stay inside the visible area.
        scene.tick(
            renderer.window_width as f32,
            renderer.window_height as f32,
            None, // no cursor on this path until egui is wired in
        );

        // Update any dirty textures (animation frame advance).
        for entity in &mut scene.entities {
            if entity.texture_dirty {
                renderer.ensure_texture(entity);
                entity.texture_dirty = false;
            }
        }

        // Render the scene. We don't have a selection model on this
        // path yet — pass `selected_id = None` and let the sprite
        // pipeline run pass-through visuals.
        let visible = scene.visible_entities();
        match renderer.render(&visible, layer.state.edit_mode, None) {
            Ok(output) => {
                // Paint egui on top of the sprite layer. The toggle
                // button is the only UI surface here; the full settings
                // panel parity lands in E.5.
                let view = output
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let size = [renderer.window_width, renderer.window_height];
                let edit_mode_snapshot = layer.state.edit_mode;
                let mut toggle_requested = false;
                let toggle_requested_ref = &mut toggle_requested;
                egui_renderer.render(
                    &renderer.device,
                    &renderer.queue,
                    &view,
                    size,
                    events,
                    |ctx| {
                        if panels::toggle_button(ctx, edit_mode_snapshot) {
                            *toggle_requested_ref = true;
                        }
                    },
                );
                output.present();
                if toggle_requested {
                    let new_mode = !layer.state.edit_mode;
                    if let Err(e) = layer.set_edit_mode(new_mode, TOGGLE_BUTTON_SIZE) {
                        tracing::warn!("Failed to flip input region on toggle: {e}");
                    } else {
                        tracing::info!(
                            "Edit mode {} (Wayland, toggle button)",
                            if new_mode { "on" } else { "off" }
                        );
                    }
                }
            }
            Err(wgpu::SurfaceError::Lost) => {
                renderer.resize(renderer.window_width, renderer.window_height);
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                return Err(AnimaError::other("GPU out of memory"));
            }
            Err(e) => {
                tracing::warn!("Render error on Wayland path: {e:?}");
            }
        }

        // Soft cap at ~60 Hz when idle.
        std::thread::sleep(Duration::from_millis(16));
    }

    // Renderer is dropped here before `layer` — wgpu surface releases
    // its handle while the underlying wl_surface is still alive.
    drop(renderer);
    drop(layer);
    Ok(())
}
