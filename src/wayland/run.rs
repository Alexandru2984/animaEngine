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
//! - Pointer events are collected (`drain_egui_events` returns them).
//! - **Keyboard events** with xkbcommon-decoded keysyms + modifier
//!   tracking (E.1, 0.5). UTF-8 text already composed via xkb's dead-key
//!   engine arrives as `egui::Event::Text` for widget input later.
//! - `Ctrl+Shift+A/H/P` global hotkeys and the tray still work — they
//!   don't depend on winit.
//!
//! ## What doesn't (yet)
//!
//! - **No egui UI** — settings panel, context menu, toasts, the ⚙ button.
//!   Edit mode toggling is currently only accessible through the tray /
//!   `Ctrl+Shift+A`. Pointer + keyboard events are buffered but discarded
//!   until the egui paint integration lands (E.4).
//! ## Drag-and-drop (E.3)
//!
//! Files dragged onto the overlay from a file manager are accepted
//! via `wl_data_device` + `wl_data_offer`. The `text/uri-list` mime
//! type is the canonical "here are file paths" payload across
//! GTK/Qt/Nautilus/Nemo/etc. A worker thread drains the receive-pipe
//! and pushes parsed `PathBuf`s back to the main loop, which routes
//! each through `Scene::add_entity_from_path` — the same validation
//! gate the X11 path uses.

use crate::constants::TOGGLE_BUTTON_SIZE;
use crate::error::{AnimaError, Result};
use crate::keybindings::{Action, KeyBindings, KeyChord};
use crate::renderer::wgpu_renderer::WgpuRenderer;
use crate::scene::Scene;
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

        // Render the scene. We don't have an edit mode toggle on this
        // path yet so always render in pass-through visuals.
        let visible = scene.visible_entities();
        match renderer.render(&visible, false, None) {
            Ok(output) => output.present(),
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
