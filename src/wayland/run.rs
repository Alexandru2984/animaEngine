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

use crate::config::AppConfig;
use crate::constants::TOGGLE_BUTTON_SIZE;
use crate::error::{AnimaError, Result};
use crate::event::AnimaEvent;
use crate::input::selection::SelectionState;
use crate::keybindings::{Action, KeyChord};
use crate::renderer::wgpu_renderer::WgpuRenderer;
use crate::scene::Scene;
use crate::ui::{panels, ToastQueue, Warning};
use crate::wayland::egui_render::WaylandEguiRenderer;
use crate::wayland::layer_window::{InputRect, LayerWindow};
use std::collections::BTreeSet;
use std::sync::mpsc;
use std::time::Duration;

/// Drive a native-Wayland session end-to-end.
///
/// Returns `Err` only when initialization fails (no compositor, missing
/// globals, wgpu surface creation refused, …). The caller falls back to
/// the X11 path on error. A successful return means the user closed
/// the layer surface (or the compositor disconnected).
#[tracing::instrument(skip(scene, config, dbus_rx))]
pub fn run_native(
    mut scene: Scene,
    mut config: AppConfig,
    dbus_rx: Option<mpsc::Receiver<AnimaEvent>>,
) -> Result<()> {
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
    let mut egui_renderer = WaylandEguiRenderer::new(
        &renderer.device,
        renderer.config.format,
        config.global.theme,
    );
    let mut selection = SelectionState::new();
    let mut toasts = ToastQueue::default();
    let mut config_dirty = false;
    let warnings: BTreeSet<Warning> = BTreeSet::new();
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

        // Drain any D-Bus actions arriving from compositor bindings
        // (E.6). Each event maps onto the same surface the X11 path's
        // global hotkeys produce, so a `gdbus call … ToggleEditMode`
        // invoked from sway is indistinguishable from clicking the ⚙
        // button.
        if let Some(rx) = &dbus_rx {
            while let Ok(ev) = rx.try_recv() {
                match ev {
                    AnimaEvent::ToggleEditMode => {
                        let new_mode = !layer.state.edit_mode;
                        if let Err(e) = layer.set_edit_mode(new_mode, TOGGLE_BUTTON_SIZE) {
                            tracing::warn!("dbus toggle: {e}");
                        }
                    }
                    AnimaEvent::HideOverlay => {
                        if let Err(e) =
                            layer.set_input_region(Some(InputRect::toggle_button_corner(
                                renderer.window_width,
                                TOGGLE_BUTTON_SIZE,
                            )))
                        {
                            tracing::warn!("dbus hide: {e}");
                        }
                    }
                    AnimaEvent::ShowOverlay => {
                        // No-op: the layer surface is always present
                        // on this path; the closest "show" is to drop
                        // back into pass-through which we already are
                        // unless edit-mode is on.
                    }
                    AnimaEvent::ToggleGlobalPlayback => {
                        scene.toggle_global_playback();
                        config_dirty = true;
                    }
                    AnimaEvent::RaiseWindow => {
                        // No raise concept on a layer surface — it's
                        // always at the Overlay layer.
                    }
                    AnimaEvent::Quit => {
                        layer.state.close_requested = true;
                    }
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
            if let Some(Action::ToggleEditMode) = config.keybindings.lookup(chord) {
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

        // Render the scene. Pass `selected_id` so the highlight ring
        // appears in edit mode for the entity the user clicked.
        // Refresh the monitor snapshot every frame so the inspector's
        // picker shows a hot-plug straight away.
        let monitors = layer.monitors();
        let selected_id = selection
            .selected_index()
            .and_then(|idx| scene.entities.get(idx).map(|e| e.id.clone()));
        let visible = scene.visible_entities();
        toasts.prune();
        egui_renderer.ensure_theme(config.global.theme);
        match renderer.render(&visible, layer.state.edit_mode, selected_id.as_deref()) {
            Ok(output) => {
                let view = output
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let size = [renderer.window_width, renderer.window_height];
                let edit_mode_snapshot = layer.state.edit_mode;
                // Snapshot the AccessKit flag BEFORE taking its mutable
                // borrow, same trick as the X11 path uses.
                let accesskit_snapshot = config.global.accesskit_enabled;
                let mut toggle_requested = false;
                let mut palette_outcome: Option<panels::PaletteOutcome> = None;
                let mut library_outcome: Option<panels::LibraryOutcome> = None;
                // Disjoint mut borrows for the closure.
                let scene_mut = &mut scene;
                let selection_mut = &mut selection;
                let config_dirty_mut = &mut config_dirty;
                let theme_mut = &mut config.global.theme;
                let locale_mut = &mut config.global.locale;
                let onboarding_mut = &mut config.global.onboarding;
                let monitor_mode_mut = &mut config.global.monitor_mode;
                let accesskit_mut = &mut config.global.accesskit_enabled;
                let keybindings_mut = &mut config.keybindings;
                let collapse_state_mut = &mut config.collapse_state;
                let last_seen_whats_new_mut = &mut config.global.last_seen_whats_new;
                let warnings_ref = &warnings;
                let monitors_ref = monitors.as_slice();
                let toasts_ref = &toasts;
                let toggle_requested_ref = &mut toggle_requested;
                let palette_ref = &mut palette_outcome;
                let library_ref = &mut library_outcome;
                egui_renderer.render(
                    &renderer.device,
                    &renderer.queue,
                    &view,
                    size,
                    events,
                    |ctx| {
                        if accesskit_snapshot {
                            ctx.enable_accesskit();
                        } else {
                            ctx.disable_accesskit();
                        }
                        if panels::toggle_button(ctx, edit_mode_snapshot) {
                            *toggle_requested_ref = true;
                        }
                        if edit_mode_snapshot {
                            panels::settings(
                                ctx,
                                scene_mut,
                                selection_mut,
                                config_dirty_mut,
                                theme_mut,
                                locale_mut,
                                onboarding_mut,
                                monitor_mode_mut,
                                monitors_ref,
                                None, // library index — not wired on Wayland yet
                                library_ref,
                                keybindings_mut,
                                collapse_state_mut,
                                accesskit_mut,
                                warnings_ref,
                                last_seen_whats_new_mut,
                            );
                            *palette_ref = panels::command_palette(ctx);
                            panels::toasts(ctx, toasts_ref);
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
                        // Exiting edit mode + dirty → persist now so
                        // hot-reload picks the fresh state up next
                        // session.
                        if !new_mode && config_dirty {
                            if let Err(e) = config.save() {
                                tracing::warn!("Config save failed: {e}");
                            } else {
                                config_dirty = false;
                            }
                        }
                    }
                }
                // Palette / library outcomes apply outside the egui
                // closure where we can take &mut renderer + &mut toasts
                // without conflicting.
                if let Some(out) = palette_outcome {
                    handle_palette_outcome(out, &mut scene, &mut config, &mut toasts);
                    config_dirty = true;
                }
                if let Some(out) = library_outcome {
                    // No asset library index on this path yet; surface
                    // a toast so the user knows the click was seen but
                    // not actionable.
                    let _ = out;
                    toasts.warn("Asset library not wired on the Wayland path yet");
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
    // Persist any unsaved edits on clean shutdown so a Ctrl+C / window
    // close doesn't lose the last toggle.
    if config_dirty {
        if let Err(e) = config.save() {
            tracing::warn!("Final config save failed: {e}");
        }
    }

    drop(renderer);
    drop(layer);
    Ok(())
}

/// Apply a command-palette outcome to scene + config + toast queue.
/// Mirrors the X11 path in `app.rs::handle_palette_outcome` but with
/// loose-typed handles since the Wayland loop doesn't go through `App`.
fn handle_palette_outcome(
    outcome: panels::PaletteOutcome,
    scene: &mut Scene,
    config: &mut AppConfig,
    toasts: &mut ToastQueue,
) {
    use crate::presets::{self, Preset};
    match outcome {
        panels::PaletteOutcome::SwitchTheme(theme) => {
            config.global.theme = theme;
            toasts.success(format!("Theme: {}", theme.label()));
        }
        panels::PaletteOutcome::ApplyPreset(id, mode) => {
            let preset = Preset::for_id(id);
            let existing = scene.to_character_configs();
            let new = presets::apply_to_scene(existing, &preset, mode);
            match mode {
                presets::ApplyMode::Replace => {
                    scene.reset_to_configs(&new);
                }
                presets::ApplyMode::Append => {
                    let already: std::collections::HashSet<String> =
                        scene.entities.iter().map(|e| e.id.clone()).collect();
                    for cfg in new.iter().filter(|c| !already.contains(&c.id)) {
                        if let Err(e) = scene.append_character_config(cfg) {
                            tracing::warn!("Palette preset append failed: {e}");
                            toasts.warn(format!("Couldn't add preset entry: {e}"));
                        }
                    }
                }
            }
            toasts.success(format!("Loaded preset: {}", preset.name));
        }
    }
}
