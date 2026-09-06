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
//!   `WaylandEguiRenderer` — the settings panel, command palette, and
//!   toast queue all render here, same as the X11 path.
//! - Keyboard events with xkbcommon-decoded keysyms + modifier
//!   tracking. UTF-8 text already composed via xkb's dead-key engine
//!   arrives as `egui::Event::Text` for widget input.
//! - File drops via `wl_data_device` (`text/uri-list`) — a worker
//!   thread drains the receive pipe and the main loop routes each
//!   path through the same `Scene::add_entity_from_path` validation
//!   the X11 path uses.
//! - Edit-mode toggle: bound chord (`Action::ToggleEditMode`) flips
//!   the click-through input region in lock-step.
//! - Asset library: discovered, scanned, indexed, and thumbnailed at
//!   startup exactly like the X11 path (`handle_resumed` in
//!   src/app/lifecycle.rs); "Add to scene" goes through the same
//!   `resolve_library_asset` containment check.
//! - Right-click context menu: same `ContextMenuState` /
//!   `MenuAction` types and the same six actions as the X11 path,
//!   detected straight off the egui pointer events this loop already
//!   drains (no new Wayland-protocol plumbing needed for it).
//! - `MonitorMode::PerMonitor`: one sprite-only `LayerWindow` extra
//!   surface per non-primary `wl_output`, mirroring the X11 path's
//!   `app::windows::WindowSlot`s. Exercised against a real headless
//!   sway session (2-3 fake outputs, hotplug both directions) — see
//!   the `layer_window` module doc and docs/wayland.md for what that
//!   covered and what's still unverified (real GPU output, the other
//!   three compositors in the support matrix).
//!
//! ## What doesn't (yet)
//!
//! - **`FollowCursor` in pass-through mode** — no Wayland protocol
//!   gives a client the global pointer position outside its own
//!   input region (unlike X11's `XQueryPointer`, by design), so this
//!   behavior stays edit-mode-only here. See docs/threat-model.md.
//! - **Window-awareness physics** — no Wayland equivalent of the EWMH
//!   properties this reads on X11; the config knob exists but the
//!   feature is inert.
//! - **`XGrabKey`-style global hotkeys** — Wayland has no client-side
//!   key grab; only the GlobalShortcuts portal (if present) or
//!   compositor-bound D-Bus methods (docs/wayland.md) work here.

use crate::config::AppConfig;
use crate::constants::TOGGLE_BUTTON_SIZE;
use crate::drop_validate::{pre_validate_dropped_file, redact_path};
use crate::entity::Entity;
use crate::error::{AnimaError, Result};
use crate::event::AnimaEvent;
use crate::input::selection::SelectionState;
use crate::keybindings::{Action, KeyChord};
use crate::monitor::{self, MonitorInfo, WindowPlan};
use crate::renderer::wgpu_renderer::{SurfaceState, WgpuRenderer};
use crate::scene::Scene;
use crate::ui::{panels, ToastQueue, Warning};
use crate::wayland::egui_render::WaylandEguiRenderer;
use crate::wayland::layer_window::{self, InputRect, LayerWindow, WaylandState};
use std::collections::{BTreeSet, HashMap};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use wayland_client::EventQueue;

/// Drive a native-Wayland session end-to-end.
///
/// Returns `Err` only when initialization fails (no compositor, missing
/// globals, wgpu surface creation refused, …). The caller falls back to
/// the X11 path on error. A successful return means the user closed
/// the layer surface (or the compositor disconnected).
#[tracing::instrument(skip(scene, config, dbus_rx, portal_rx))]
pub fn run_native(
    mut scene: Scene,
    mut config: AppConfig,
    dbus_rx: Option<mpsc::Receiver<AnimaEvent>>,
    portal_rx: Option<mpsc::Receiver<crate::hotkeys::portal::PortalMsg>>,
) -> Result<()> {
    // Tracks the parity of portal HideOverlay toggles — the portal
    // delivers one *action*, the Hide/Show intent derives from state.
    let mut overlay_hidden = false;
    // Keybindings-tab backend status (T.4), updated by PortalMsg.
    let mut hotkey_backend_status: String = if portal_rx.is_some() {
        "portal (awaiting approval)".into()
    } else {
        "none (compositor bindings + D-Bus)".into()
    };
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
        &renderer.shared.device,
        renderer.shared.surface_format,
        config.global.theme,
    );
    let mut selection = SelectionState::new();
    let mut toasts = ToastQueue::default();
    let mut config_dirty = false;
    let warnings: BTreeSet<Warning> = BTreeSet::new();
    // Right-click context menu state, mirroring `app::ContextMenuState`
    // on the X11 path. Persists across frames while the menu is open.
    let mut context_menu_state: Option<crate::app::ContextMenuState> = None;
    tracing::info!("Native Wayland renderer initialized ({width}×{height})");

    // Discover + load + merge-scan the asset library — same sequence
    // as the X11 path's `handle_resumed` (src/app/lifecycle.rs).
    // Errors are logged but never fatal — an empty library is fine.
    let mut library: Option<crate::asset_library::LibraryIndex> = None;
    let mut library_root: Option<std::path::PathBuf> = None;
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
            redact_path(&root),
            idx.assets.len(),
            scanned_count,
        );
        tracing::debug!("Asset library full root: {}", root.display());
        {
            let root_for_thumbs = root.clone();
            let index_for_thumbs = idx.clone();
            let spawned = std::thread::Builder::new()
                .name("anima-thumbs".into())
                .spawn(move || {
                    crate::asset_library::generate_missing_thumbnails(
                        &root_for_thumbs,
                        &index_for_thumbs,
                    );
                });
            if let Err(e) = spawned {
                tracing::warn!("Thumbnail thread failed to spawn: {e}");
            }
        }
        library = Some(idx);
        library_root = Some(root);
    } else {
        tracing::info!("No asset library root found; Library tab will show empty state.");
    }

    // Start in pass-through mode with the ⚙ button cutout — same default
    // as the X11 path.
    layer.set_input_region(Some(InputRect::toggle_button_corner(
        width,
        TOGGLE_BUTTON_SIZE,
    )))?;

    // Sprite-only extra surfaces for `MonitorMode::PerMonitor`, one
    // per non-primary output — the Wayland counterpart of the X11
    // path's `App.extra_windows`. Keyed by monitor name so it can be
    // diffed against `monitor::plan_windows`'s output on every
    // topology / mode change. Exercised against a real headless sway
    // session including output hotplug (see docs/wayland.md and the
    // module doc on `layer_window::mod`).
    let mut extra_surfaces: HashMap<String, SurfaceState> = HashMap::new();
    let mut last_monitor_mode = config.global.monitor_mode.clone();
    {
        let initial_monitors = layer.monitors();
        let plan = monitor::plan_windows(&config.global.monitor_mode, &initial_monitors);
        rebuild_extra_surfaces(&mut layer, &renderer, &plan, &mut extra_surfaces);
    }

    // Upload textures once for the initial scene.
    for entity in &scene.entities {
        renderer.ensure_texture(entity);
    }
    for entity in &mut scene.entities {
        entity.texture_dirty = false;
    }

    // ── Main loop ──
    // Waits for compositor events but never longer than one frame, so
    // animations keep ticking and channel-delivered commands are picked
    // up even when the compositor has nothing to say. See
    // `dispatch_with_timeout`.
    loop {
        let frame_start = Instant::now();
        dispatch_with_timeout(&mut layer.event_queue, &mut layer.state, FRAME_INTERVAL)?;

        if layer.state.close_requested {
            tracing::info!("Layer surface closed by compositor — exiting.");
            break;
        }

        // Pick up any resize the compositor sent us.
        if let Some((new_w, new_h)) = layer.state.pending_size.take() {
            if new_w != renderer.primary.window_width || new_h != renderer.primary.window_height {
                renderer.resize(new_w, new_h);
                layer.set_input_region(Some(InputRect::toggle_button_corner(
                    new_w,
                    TOGGLE_BUTTON_SIZE,
                )))?;
                tracing::info!("Layer surface resized to {new_w}×{new_h}");
            }
        }

        // Rebuild PerMonitor extras whenever the user switches mode or
        // the output topology changes (hotplug) — mirrors the X11
        // path's `rebuild_windows_if_mode_changed` + `check_monitor_topology`.
        // Computed early in the frame: file-drop landing coordinates
        // and the right-click hit test below both need `primary_origin`
        // translated into entity-space before they touch `scene`.
        let monitors_now = layer.monitors();
        let plan = monitor::plan_windows(&config.global.monitor_mode, &monitors_now);
        let plan_names: std::collections::HashSet<&str> =
            plan.extras.iter().map(|m| m.name.as_str()).collect();
        let current_names: std::collections::HashSet<&str> =
            extra_surfaces.keys().map(|s| s.as_str()).collect();
        if config.global.monitor_mode != last_monitor_mode || plan_names != current_names {
            rebuild_extra_surfaces(&mut layer, &renderer, &plan, &mut extra_surfaces);
            last_monitor_mode = config.global.monitor_mode.clone();
        }
        for (name, new_w, new_h) in layer.drain_extra_resizes() {
            if let Some(surface) = extra_surfaces.get_mut(&name) {
                surface.resize(&renderer.shared, new_w, new_h);
            }
        }

        // Entities live in window-local coordinates in single-output
        // modes (today's only well-exercised case) and in global
        // desktop coordinates once PerMonitor extras exist — same
        // duality as the X11 path's `App::primary_origin` (T.8).
        // `primary_output_name` is learned from `surface_enter`
        // (handlers.rs); it stays `None` (→ identity origin) if the
        // compositor never sends it, so the fallback is always the
        // already-shipped single-output behavior, never a wrong offset.
        let primary_origin: (f32, f32) = compute_primary_origin(
            !extra_surfaces.is_empty(),
            layer.state.primary_output_name.as_deref(),
            &monitors_now,
        );
        let cursor_global = layer
            .state
            .cursor_pos
            .map(|(x, y)| (x + primary_origin.0, y + primary_origin.1));

        // Drain pointer + keyboard events. Until egui paint lands
        // (E.4) we don't have a UI consumer, but we already need to
        // detect the `Action::ToggleEditMode` chord so click-through
        // can flip in lock-step. Scan key-press events, match against
        // the user's bindings, dispatch the few actions that make
        // sense without a UI thread (just edit mode for now).
        // Process any files dropped over the surface (E.3). Each path
        // routes through the same `add_entity_from_path` validation
        // gate as the X11 drag-drop path, so frame caps + extension
        // whitelist still apply. `drop_pos` is surface-local (same
        // space as `cursor_pos`); add `primary_origin` so the spawned
        // entity lands in the same coordinate space every other entity
        // uses once PerMonitor extras exist.
        let drop_pos = layer
            .last_drag_pos()
            .map(|(x, y)| (x + primary_origin.0, y + primary_origin.1));
        for path in layer.drain_dropped_files() {
            // F.1 fix: run the same pre-validate gate the X11 path
            // uses (size cap + extension whitelist + regular-file
            // check). Pre-0.5.1 this was skipped on the Wayland path,
            // so a `.png` of arbitrary size could reach the decoder.
            let label = redact_path(&path);
            if let Err(reason) = pre_validate_dropped_file(&path) {
                tracing::warn!("Drop rejected for {label}: {reason}");
                tracing::debug!("Rejected drop full path: {}", path.display());
                {
                    let mut args = fluent::FluentArgs::new();
                    args.set("reason", reason.clone());
                    toasts.warn(crate::i18n::t_args("toast-rejected", &args));
                }
                continue;
            }
            let (x, y) = drop_pos.unwrap_or((
                renderer.primary.window_width as f32 / 2.0 + primary_origin.0,
                renderer.primary.window_height as f32 / 2.0 + primary_origin.1,
            ));
            match scene.add_entity_from_path(&path, x, y) {
                Ok(idx) => {
                    renderer.ensure_texture(&scene.entities[idx]);
                    scene.entities[idx].texture_dirty = false;
                    tracing::info!("Spawned entity from drop: {label} at ({x:.0}, {y:.0})");
                    tracing::debug!("Drop full path: {}", path.display());
                }
                Err(e) => {
                    tracing::warn!("Drop rejected for {label}: {e}");
                    tracing::debug!("Rejected drop full path: {}", path.display());
                }
            }
        }

        // Drain any D-Bus actions arriving from compositor bindings
        // (E.6). Each event maps onto the same surface the X11 path's
        // global hotkeys produce, so a `gdbus call … ToggleEditMode`
        // invoked from sway is indistinguishable from clicking the ⚙
        // button.
        //
        // F.3: coalesce idempotent toggle events so a spammy caller
        // (a thousand `ToggleEditMode` calls between two frames =
        // odd parity → noop = even parity → toggle) doesn't bounce
        // the input region a thousand times — we apply each toggle
        // class at most once per frame. ShowOverlay / HideOverlay are
        // distinct because the user might want either intent.
        {
            let mut toggle_edit_xor = false;
            let mut toggle_playback_xor = false;
            let mut last_visibility: Option<AnimaEvent> = None;
            let mut quit = false;
            if let Some(rx) = &dbus_rx {
                while let Ok(ev) = rx.try_recv() {
                    match ev {
                        AnimaEvent::ToggleEditMode => toggle_edit_xor ^= true,
                        AnimaEvent::ToggleGlobalPlayback => toggle_playback_xor ^= true,
                        AnimaEvent::HideOverlay => last_visibility = Some(AnimaEvent::HideOverlay),
                        AnimaEvent::ShowOverlay => last_visibility = Some(AnimaEvent::ShowOverlay),
                        AnimaEvent::Quit => quit = true,
                        AnimaEvent::RaiseWindow => {}
                        // Hotkey resolution events are winit-path UI; the
                        // native path logs the outcome where it resolves.
                        AnimaEvent::HotkeysUnavailable | AnimaEvent::PortalShortcutsDenied => {}
                    }
                }
            }
            // Portal shortcut activations land in the same per-frame
            // accumulators as the D-Bus methods — a Ctrl+Shift+A from
            // the portal is indistinguishable from `gdbus call …
            // ToggleEditMode`. T.2: the portal is the first mechanism
            // that gives the *native* path real global hotkeys.
            if let Some(rx) = &portal_rx {
                use crate::hotkeys::portal::PortalMsg;
                use crate::keybindings::Action as KbAction;
                while let Ok(msg) = rx.try_recv() {
                    match msg {
                        PortalMsg::Ready => {
                            tracing::info!("Portal shortcuts active (native path)");
                            hotkey_backend_status = "portal (GlobalShortcuts)".into();
                        }
                        PortalMsg::Failed => {
                            tracing::warn!(
                                "Portal shortcuts unavailable — compositor \
                                 bindings via D-Bus remain the fallback"
                            );
                            toasts.warn(crate::i18n::t("portal-denied-native-toast"));
                            hotkey_backend_status = "none (compositor bindings + D-Bus)".into();
                        }
                        PortalMsg::Activated(action) => match action {
                            KbAction::ToggleEditMode => toggle_edit_xor ^= true,
                            KbAction::PauseAll => toggle_playback_xor ^= true,
                            KbAction::HideOverlay => {
                                overlay_hidden = !overlay_hidden;
                                last_visibility = Some(if overlay_hidden {
                                    AnimaEvent::HideOverlay
                                } else {
                                    AnimaEvent::ShowOverlay
                                });
                            }
                            _ => {}
                        },
                    }
                }
            }
            if toggle_edit_xor {
                let new_mode = !layer.state.edit_mode;
                if let Err(e) = layer.set_edit_mode(new_mode, TOGGLE_BUTTON_SIZE) {
                    tracing::warn!("dbus toggle: {e}");
                }
            }
            if toggle_playback_xor {
                scene.toggle_global_playback();
                config_dirty = true;
            }
            if let Some(vis) = last_visibility {
                // `overlay_hidden` is the single source of truth and it
                // gates rendering below. It used to be updated only by the
                // keybinding path, so a D-Bus Hide/Show left the flag and
                // the actual intent disagreeing.
                match vis {
                    AnimaEvent::HideOverlay => overlay_hidden = true,
                    AnimaEvent::ShowOverlay => overlay_hidden = false,
                    _ => {}
                }
                // A layer surface can't be unmapped the way winit's
                // `set_visible(false)` unmaps a window, so "hidden" is
                // expressed as: paint nothing (see the render gate) and
                // accept no input. Previously hide only narrowed the input
                // region, which left every sprite fully visible on screen —
                // the overlay was not hidden in any sense the user would
                // recognise.
                let region = if overlay_hidden {
                    None
                } else if layer.state.edit_mode {
                    Some(InputRect::full(
                        renderer.primary.window_width,
                        renderer.primary.window_height,
                    ))
                } else {
                    Some(InputRect::toggle_button_corner(
                        renderer.primary.window_width,
                        TOGGLE_BUTTON_SIZE,
                    ))
                };
                if let Err(e) = layer.set_input_region(region) {
                    tracing::warn!("visibility change: {e}");
                }
            }
            if quit {
                layer.state.close_requested = true;
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

        // Right-click on an entity opens the context menu and selects
        // it, same gating and behavior as the X11 path's
        // `handle_mouse_input` (src/app/input.rs). Entity-less right
        // clicks (empty space) are ignored.
        if layer.state.edit_mode {
            for event in &events {
                if let egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Secondary,
                    pressed: true,
                    ..
                } = event
                {
                    if let Some(idx) =
                        scene.entity_at_point(pos.x + primary_origin.0, pos.y + primary_origin.1)
                    {
                        selection.select(idx);
                        context_menu_state = Some(crate::app::ContextMenuState {
                            entity_idx: idx,
                            pos: *pos,
                        });
                    }
                }
            }
        }

        // Tick the simulation. screen_w / screen_h match the surface so
        // walk-around behaviors stay inside the visible area.
        scene.set_reduced_motion(config.global.reduced_motion);
        scene.set_hover_startle(config.global.hover_startle);
        scene.tick(
            renderer.primary.window_width as f32,
            renderer.primary.window_height as f32,
            // `cursor_pos` is tracked from every Motion/Enter pointer
            // event (pointer_handler.rs) in this surface's local
            // space; `cursor_global` adds `primary_origin` so it lands
            // in the same coordinate space entities use (identity
            // outside PerMonitor, T.8-equivalent otherwise). FollowCursor
            // sees it whenever the pointer is over the surface's active
            // input region — same X11 caveat applies: pass-through mode
            // still leaves it stale outside the toggle button, since
            // Wayland has no XQueryPointer equivalent (docs/threat-model.md).
            cursor_global,
        );

        // Update any dirty textures (animation frame advance).
        // The prune sweeps textures orphaned by preset Replace — same
        // rationale as the winit render loop.
        renderer.prune_stale_textures(&scene.entities);
        for entity in &mut scene.entities {
            if entity.texture_dirty {
                renderer.ensure_texture(entity);
                entity.texture_dirty = false;
            }
        }

        // Render the scene. Pass `selected_id` so the highlight ring
        // appears in edit mode for the entity the user clicked.
        // `monitors_now` (refreshed above) covers the inspector's
        // picker hot-plug needs too — no separate snapshot needed.
        let monitors = &monitors_now;
        let selected_id = selection
            .selected_index()
            .and_then(|idx| scene.entities.get(idx).map(|e| e.id.clone()));
        let visible = scene.visible_entities();
        // In PerMonitor mode the primary surface covers exactly its
        // own output: entities live in global coords (once extras
        // exist) and need filtering to just the primary's monitor,
        // same as the X11 path's `entity_on_monitor` gate (T.6/T.8).
        let primary_monitor_name: Option<String> = if !extra_surfaces.is_empty() {
            plan.primary.as_ref().map(|m| m.name.clone())
        } else {
            None
        };
        let drawn: Vec<&Entity> = if overlay_hidden {
            // Hidden: present a cleared (fully transparent) surface. The
            // layer surface stays mapped — that is what a layer shell
            // gives us — but nothing is painted into it, which is the
            // visible equivalent of the winit path unmapping its window.
            Vec::new()
        } else {
            match &primary_monitor_name {
                Some(name) => visible
                    .into_iter()
                    .filter(|e| crate::app::windows::entity_on_monitor(monitors, e, name))
                    .collect(),
                None => visible,
            }
        };
        toasts.prune();
        egui_renderer.ensure_theme(config.global.theme);
        match renderer.render(
            &drawn,
            &scene.groups,
            layer.state.edit_mode,
            selected_id.as_deref(),
            primary_origin,
        ) {
            Ok(output) => {
                let view = output.create_view();
                let size = [
                    renderer.primary.window_width,
                    renderer.primary.window_height,
                ];
                // Pick up the largest scale among all outputs the
                // surface might be on — undershooting blurs text on
                // HiDPI; overshooting just makes glyphs bigger than
                // necessary on standard DPI, which is the kinder
                // failure mode.
                let pixels_per_point = monitors
                    .iter()
                    .map(|m| m.scale_factor as f32)
                    .fold(1.0_f32, f32::max);
                let edit_mode_snapshot = layer.state.edit_mode;
                let hidden_snapshot = overlay_hidden;
                // Snapshot the AccessKit flag BEFORE taking its mutable
                // borrow, same trick as the X11 path uses.
                let accesskit_snapshot = config.global.accesskit_enabled;
                let mut toggle_requested = false;
                let mut palette_outcome: Option<panels::PaletteOutcome> = None;
                let mut library_outcome: Option<panels::LibraryOutcome> = None;
                let mut menu_outcome: Option<panels::ContextMenuOutcome> = None;
                let mut shimeji_import: Option<String> = None;
                let menu_state = context_menu_state.clone();
                // Disjoint mut borrows for the closure.
                let scene_mut = &mut scene;
                let selection_mut = &mut selection;
                let config_dirty_mut = &mut config_dirty;
                let theme_mut = &mut config.global.theme;
                let locale_mut = &mut config.global.locale;
                let onboarding_mut = &mut config.global.onboarding;
                let monitor_mode_mut = &mut config.global.monitor_mode;
                let window_awareness_mut = &mut config.global.window_awareness;
                let reduced_motion_mut = &mut config.global.reduced_motion;
                let hover_startle_mut = &mut config.global.hover_startle;
                let accesskit_mut = &mut config.global.accesskit_enabled;
                let keybindings_mut = &mut config.keybindings;
                let collapse_state_mut = &mut config.collapse_state;
                let last_seen_whats_new_mut = &mut config.global.last_seen_whats_new;
                let warnings_ref = &warnings;
                let hotkey_backend_ref = hotkey_backend_status.as_str();
                let shimeji_import_ref = &mut shimeji_import;
                let monitors_ref = monitors.as_slice();
                let toasts_ref = &toasts;
                let toggle_requested_ref = &mut toggle_requested;
                let palette_ref = &mut palette_outcome;
                let library_ref = &mut library_outcome;
                let menu_outcome_ref = &mut menu_outcome;
                egui_renderer.render(
                    &renderer.shared.device,
                    &renderer.shared.queue,
                    &view,
                    size,
                    pixels_per_point,
                    events,
                    |ctx| {
                        if accesskit_snapshot {
                            ctx.enable_accesskit();
                        } else {
                            ctx.disable_accesskit();
                        }
                        crate::ui::motion::set_reduced(ctx, *reduced_motion_mut);
                        if hidden_snapshot {
                            // Hidden means hidden: no sprites (the draw
                            // list is empty above) and no UI either, so
                            // the ⚙ button doesn't linger over a
                            // supposedly hidden overlay. Restore comes
                            // through the same D-Bus/keybinding channel
                            // that hid it, exactly as on the winit path.
                            return;
                        }
                        if panels::toggle_button(ctx, edit_mode_snapshot) {
                            *toggle_requested_ref = true;
                        }
                        if crate::ui::onboarding::coach_marks(
                            ctx,
                            onboarding_mut,
                            edit_mode_snapshot,
                        ) {
                            *config_dirty_mut = true;
                        }
                        {
                            panels::settings(
                                ctx,
                                edit_mode_snapshot,
                                scene_mut,
                                selection_mut,
                                config_dirty_mut,
                                theme_mut,
                                locale_mut,
                                onboarding_mut,
                                monitor_mode_mut,
                                window_awareness_mut,
                                reduced_motion_mut,
                                hover_startle_mut,
                                monitors_ref,
                                library.as_ref(),
                                library_ref,
                                keybindings_mut,
                                collapse_state_mut,
                                accesskit_mut,
                                warnings_ref,
                                last_seen_whats_new_mut,
                                hotkey_backend_ref,
                                shimeji_import_ref,
                            );
                            if edit_mode_snapshot {
                                if let Some(state) = &menu_state {
                                    *menu_outcome_ref = Some(panels::context_menu(ctx, state));
                                }
                                *palette_ref = panels::command_palette(ctx);
                                panels::toasts(ctx, toasts_ref);
                            }
                        }
                    },
                );
                renderer.present(output);
                // Sprite-only extras: no egui, no input — just the
                // entities pinned (or resolved by position) to that
                // monitor, translated by its own origin. Mirrors
                // `app::windows::render_extra_windows` on the X11 path.
                if !extra_surfaces.is_empty() {
                    let extra_visible = scene.visible_entities();
                    for (name, surface) in extra_surfaces.iter_mut() {
                        let Some(mon) = monitors_now.iter().find(|m| &m.name == name) else {
                            continue;
                        };
                        let drawn: Vec<&Entity> = extra_visible
                            .iter()
                            .copied()
                            .filter(|e| {
                                crate::app::windows::entity_on_monitor(&monitors_now, e, name)
                            })
                            .collect();
                        let origin = (mon.x as f32, mon.y as f32);
                        match surface.render(
                            &renderer.shared,
                            &drawn,
                            &scene.groups,
                            layer.state.edit_mode,
                            selected_id.as_deref(),
                            origin,
                        ) {
                            Ok(extra_output) => surface.present(&renderer.shared, extra_output),
                            Err(wgpu::SurfaceError::Lost) => {
                                let (w, h) = (surface.window_width, surface.window_height);
                                surface.resize(&renderer.shared, w, h);
                            }
                            Err(e) => {
                                tracing::warn!("Render error on extra surface {name}: {e:?}");
                            }
                        }
                    }
                }
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
                            if let Err(e) = sync_and_save(&mut config, &scene) {
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
                if let Some(path) = shimeji_import {
                    // Path-paste import on the native path: same
                    // importer, library root discovered fresh (the
                    // native loop doesn't hold one).
                    if let Some(root) = crate::asset_library::discover_asset_root() {
                        match crate::shimeji::import_pack(
                            &crate::config::AppConfig::resolve_asset_path(&path),
                            &root,
                        ) {
                            Ok(report) => {
                                let mut ok = 0usize;
                                for mut cfg in report.characters {
                                    cfg.x = 100.0;
                                    cfg.y = 100.0;
                                    cfg.id = scene.unique_id(&cfg.id);
                                    match scene.append_character_config(&cfg) {
                                        Ok(()) => ok += 1,
                                        Err(e) => {
                                            let mut args = fluent::FluentArgs::new();
                                            args.set("name", cfg.name.clone());
                                            args.set("error", e.to_string());
                                            toasts.error(crate::i18n::t_args(
                                                "toast-entity-load-failed",
                                                &args,
                                            ));
                                        }
                                    }
                                }
                                for (what, why) in &report.skipped {
                                    tracing::info!("Shimeji import skip [{what}]: {why}");
                                }
                                if ok > 0 {
                                    let mut args = fluent::FluentArgs::new();
                                    args.set("name", report.pack_name.clone());
                                    args.set("n", report.skipped.len() as i64);
                                    toasts.success(crate::i18n::t_args(
                                        "shimeji-imported-toast",
                                        &args,
                                    ));
                                    config_dirty = true;
                                }
                            }
                            Err(reason) => {
                                let mut args = fluent::FluentArgs::new();
                                args.set("reason", reason);
                                toasts.error(crate::i18n::t_args(
                                    "shimeji-import-failed-toast",
                                    &args,
                                ));
                            }
                        }
                    } else {
                        toasts.error(crate::i18n::t("shimeji-no-library-toast"));
                    }
                }
                if let Some(out) = menu_outcome {
                    match out {
                        panels::ContextMenuOutcome::Open => {}
                        panels::ContextMenuOutcome::Close => {
                            context_menu_state = None;
                        }
                        panels::ContextMenuOutcome::Action(action) => {
                            handle_menu_action(
                                action,
                                &mut scene,
                                &mut renderer,
                                &mut selection,
                                &mut toasts,
                                &mut config_dirty,
                            );
                            context_menu_state = None;
                        }
                    }
                }
                if let Some(out) = palette_outcome {
                    handle_palette_outcome(out, &mut scene, &mut config, &mut toasts);
                    config_dirty = true;
                }
                if let Some(out) = library_outcome {
                    handle_library_outcome(
                        out,
                        library_root.as_deref(),
                        &mut library,
                        &mut scene,
                        &mut toasts,
                        &mut config_dirty,
                        (
                            renderer.primary.window_width as f32 / 2.0 + primary_origin.0,
                            renderer.primary.window_height as f32 / 2.0 + primary_origin.1,
                        ),
                    );
                }
            }
            Err(wgpu::SurfaceError::Lost) => {
                renderer.resize(
                    renderer.primary.window_width,
                    renderer.primary.window_height,
                );
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                return Err(AnimaError::other("GPU out of memory"));
            }
            Err(e) => {
                tracing::warn!("Render error on Wayland path: {e:?}");
            }
        }

        // Soft cap at ~60 Hz. The dispatch above returns immediately when
        // events were already queued, so pace on the frame's actual
        // elapsed time rather than sleeping unconditionally — otherwise a
        // busy compositor would spin faster than the cap and an idle one
        // would wait out the poll timeout *and* a full sleep on top.
        if let Some(rest) = FRAME_INTERVAL.checked_sub(frame_start.elapsed()) {
            std::thread::sleep(rest);
        }
    }

    // Renderer (and every extra surface's wgpu::Surface) is dropped
    // here before `layer` — each wgpu surface releases its handle
    // while the wl_surface it points into is still alive. `extra_surfaces`
    // holds the same kind of raw-handle surface the primary one does
    // (E.7), so it needs the identical ordering, not just `renderer`.
    // Persist any unsaved edits on clean shutdown so a Ctrl+C / window
    // close doesn't lose the last toggle.
    if config_dirty {
        if let Err(e) = sync_and_save(&mut config, &scene) {
            tracing::warn!("Final config save failed: {e}");
        }
    }

    drop(renderer);
    drop(extra_surfaces);
    drop(layer);
    Ok(())
}

/// Pure half of `rebuild_extra_surfaces`'s decision: given what the
/// plan wants and what's currently tracked, which names need tearing
/// down and which planned monitors need a brand-new surface. Split
/// out so this decision has a unit test even though the actual
/// teardown/creation (real Wayland + GPU calls) doesn't.
/// Target frame interval for the native loop — a ~60 Hz soft cap.
const FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// Dispatch Wayland events, waiting at most `timeout` for the socket.
///
/// This replaces `EventQueue::blocking_dispatch`, which waits for a
/// *compositor* event with no upper bound. On a quiet desktop — pointer
/// still, nothing on screen changing — nothing ever arrives, so the loop
/// simply stopped: animations froze mid-sequence and D-Bus/portal
/// commands sat unread in their channels, because pushing into an mpsc
/// channel does not make the Wayland socket readable. The 16 ms sleep at
/// the bottom of the loop was documented as covering exactly this, but it
/// is only reached *after* a compositor event has already unblocked the
/// top.
///
/// Mirrors `blocking_dispatch`'s own steps (dispatch what's buffered,
/// flush, read, dispatch again) but polls the connection fd with a
/// deadline, so the loop still sleeps in the kernel when idle and never
/// waits longer than `timeout`.
fn dispatch_with_timeout(
    queue: &mut EventQueue<WaylandState>,
    state: &mut WaylandState,
    timeout: Duration,
) -> Result<()> {
    use std::os::fd::AsRawFd;

    let to_err =
        |e: wayland_client::DispatchError| AnimaError::other(format!("wayland dispatch: {e}"));

    // Already-buffered events first, exactly as blocking_dispatch does.
    if queue.dispatch_pending(state).map_err(to_err)? > 0 {
        return Ok(());
    }
    queue
        .flush()
        .map_err(|e| AnimaError::other(format!("wayland flush: {e}")))?;

    if let Some(guard) = queue.prepare_read() {
        let fd = guard.connection_fd();
        let mut pfd = libc::pollfd {
            fd: fd.as_raw_fd(),
            events: libc::POLLIN | libc::POLLERR,
            revents: 0,
        };
        let ms = timeout.as_millis().min(i32::MAX as u128) as i32;
        // SAFETY: `pfd` is one initialised `pollfd` we own for the whole
        // call and `poll` touches only that entry; the fd stays open
        // because `guard` borrows the live connection.
        let ready = unsafe { libc::poll(&mut pfd, 1, ms) };
        if ready > 0 {
            // A spurious wakeup returns WouldBlock — harmless, the next
            // iteration retries, same as wayland-client's own read path.
            let _ = guard.read();
        }
        // ready == 0 is the timeout (nothing to read) and ready < 0 is
        // EINTR or similar; in both cases dropping the guard cancels the
        // prepared read and we fall through to dispatch what we have.
    }

    queue.dispatch_pending(state).map_err(to_err)?;
    Ok(())
}

/// Mirror the live scene into `config`, then persist it.
///
/// The winit path does exactly this in `App::save_config_if_needed`. The
/// native loop used to call `config.save()` directly, which wrote back
/// whatever `characters` was parsed at startup — so dragging a sprite,
/// adding an entity or toggling playback set `config_dirty`, saved the
/// *stale* scene, and silently discarded the edit. Any save on this path
/// must go through here.
fn sync_and_save(config: &mut AppConfig, scene: &Scene) -> Result<()> {
    config.characters = scene.to_character_configs();
    config.global.playback_enabled = scene.global_playing;
    config.save()
}

fn diff_extra_plan<'a>(
    plan_extras: &'a [MonitorInfo],
    current_names: &[String],
) -> (Vec<String>, Vec<&'a MonitorInfo>) {
    let wanted: std::collections::HashSet<&str> =
        plan_extras.iter().map(|m| m.name.as_str()).collect();
    let stale: Vec<String> = current_names
        .iter()
        .filter(|name| !wanted.contains(name.as_str()))
        .cloned()
        .collect();
    let new: Vec<&MonitorInfo> = plan_extras
        .iter()
        .filter(|m| !current_names.iter().any(|n| n == &m.name))
        .collect();
    (stale, new)
}

/// Pure: the origin to translate primary-surface entity coordinates
/// and pointer positions by. Identity outside `PerMonitor` (no
/// extras) — the single-output behavior every compositor exercises
/// today; the primary output's logical position once extras exist
/// and `surface_enter` has told us which output that is.
fn compute_primary_origin(
    has_extras: bool,
    primary_output_name: Option<&str>,
    monitors: &[MonitorInfo],
) -> (f32, f32) {
    if !has_extras {
        return (0.0, 0.0);
    }
    primary_output_name
        .and_then(|name| monitors.iter().find(|m| m.name == name))
        .map(|m| (m.x as f32, m.y as f32))
        .unwrap_or((0.0, 0.0))
}

/// Rebuild the sprite-only extra (non-primary) surfaces to match
/// `plan`. Idempotent: tears down anything no longer in the plan,
/// creates anything new, leaves unchanged entries alone. Mirrors
/// `app::windows::rebuild_extra_windows` (X11 path) — both use the
/// same pure `monitor::plan_windows`, so the two backends can't
/// silently diverge on *which* monitors get an extra surface, only
/// on *how* the surface is created (layer-shell here, a winit
/// `Window` there).
///
/// Exercised against a real headless sway session: output binding,
/// adding an output mid-session, and removing one (including the one
/// an extra surface was bound to) all confirmed not to crash the
/// process — see the module doc on `wayland::layer_window` and
/// docs/wayland.md for what's still unverified.
fn rebuild_extra_surfaces(
    layer: &mut LayerWindow,
    renderer: &WgpuRenderer,
    plan: &WindowPlan,
    extra_surfaces: &mut HashMap<String, SurfaceState>,
) {
    let current_names: Vec<String> = extra_surfaces.keys().cloned().collect();
    let (stale, new) = diff_extra_plan(&plan.extras, &current_names);
    for name in stale {
        // Order matters: `SurfaceState::surface` is a `wgpu::Surface`
        // built from this layer's raw `wl_surface` pointer (E.7,
        // `build_wgpu_surface`'s safety comment) — it must be dropped
        // *before* the wl_surface it points into, never after, same
        // invariant `LayerWindow`'s own doc comment states for the
        // primary surface vs. `WgpuRenderer`.
        extra_surfaces.remove(&name);
        layer.destroy_extra_layer(&name);
    }

    for mon in new {
        let Some(output) = layer.output_by_name(&mon.name) else {
            tracing::warn!(
                "No wl_output found for monitor {} — skipping extra surface",
                mon.name
            );
            continue;
        };
        let wl_surface = match layer.create_extra_layer(&output, &mon.name) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Couldn't create extra layer for {}: {e}", mon.name);
                continue;
            }
        };
        // Wait for the compositor's first `configure` on this specific
        // layer surface before touching wgpu at all — the same
        // round-trip `LayerWindow::try_create` already does for the
        // primary surface, for the same reason (a layer-shell surface
        // committed to before its first configure is a protocol
        // error, and confused wgpu into a queue-family validation
        // panic on top of that when this went untested).
        if let Err(e) = layer.event_queue.roundtrip(&mut layer.state) {
            tracing::warn!(
                "Roundtrip after creating extra layer for {} failed: {e}",
                mon.name
            );
            layer.destroy_extra_layer(&mon.name);
            continue;
        }
        let (configured_w, configured_h) = layer
            .take_extra_configured_size(&mon.name)
            .unwrap_or((mon.width, mon.height));
        let wgpu_surface = match layer_window::build_wgpu_surface(
            &renderer.shared.instance,
            &layer.connection,
            &wl_surface,
        ) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Couldn't create wgpu surface for {}: {e}", mon.name);
                layer.destroy_extra_layer(&mon.name);
                continue;
            }
        };
        let surface = SurfaceState::new(&renderer.shared, wgpu_surface, configured_w, configured_h);
        tracing::info!(
            "Spawned Wayland extra surface on {} ({}x{} at {},{})",
            mon.name,
            configured_w,
            configured_h,
            mon.x,
            mon.y
        );
        extra_surfaces.insert(mon.name.clone(), surface);
    }
}

/// Apply a context-menu action to scene + renderer + selection +
/// toast queue. Mirrors `app/outcomes.rs::apply_menu_action` (X11
/// path) exactly, including the `.get`/`.get_mut` everywhere
/// hardening — a stale `entity_idx` (menu opened, then the entity
/// disappeared via hot-reload before the user picked an action)
/// degrades to a no-op rather than a panic. `renderer` isn't
/// `Option` here (unlike the X11 path's `self.renderer`) since the
/// native Wayland loop always has one by this point in the run.
fn handle_menu_action(
    action: panels::MenuAction,
    scene: &mut Scene,
    renderer: &mut WgpuRenderer,
    selection: &mut SelectionState,
    toasts: &mut ToastQueue,
    config_dirty: &mut bool,
) {
    match action {
        panels::MenuAction::Duplicate(idx) => {
            let Some(src) = scene.entities.get(idx) else {
                return;
            };
            let src_name = src.name.clone();
            let src_path = std::path::PathBuf::from(&src.asset_path);
            let new_x = src.x + 30.0;
            let new_y = src.y + 30.0;
            let orig_scale = src.scale;
            let orig_opacity = src.opacity;

            match scene.add_entity_from_path(&src_path, new_x, new_y) {
                Ok(new_idx) => {
                    if let Some(entity) = scene.entities.get_mut(new_idx) {
                        entity.scale = orig_scale;
                        entity.opacity = orig_opacity;
                    }
                    if let Some(entity) = scene.entities.get(new_idx) {
                        renderer.ensure_texture(entity);
                    }
                    if let Some(entity) = scene.entities.get_mut(new_idx) {
                        entity.texture_dirty = false;
                    }
                    selection.select(new_idx);
                    *config_dirty = true;
                    let mut args = fluent::FluentArgs::new();
                    args.set("name", src_name.clone());
                    toasts.success(crate::i18n::t_args("toast-duplicated", &args));
                }
                Err(e) => {
                    tracing::error!("Context menu duplicate failed: {}", e);
                    let mut args = fluent::FluentArgs::new();
                    args.set("error", e.to_string());
                    toasts.error(crate::i18n::t_args("toast-duplicate-failed", &args));
                }
            }
        }
        panels::MenuAction::Delete(idx) => {
            let removed_name = scene
                .entities
                .get(idx)
                .map(|e| e.name.clone())
                .unwrap_or_default();
            if let Some(entity) = scene.entities.get(idx) {
                renderer.shared.textures.remove(&entity.id);
            }
            if scene.remove_entity(idx).is_some() {
                selection.deselect();
                *config_dirty = true;
                let mut args = fluent::FluentArgs::new();
                args.set("name", removed_name.clone());
                toasts.info(crate::i18n::t_args("toast-deleted", &args));
            }
        }
        panels::MenuAction::ResetTransform(idx) => {
            if let Some(e) = scene.entities.get_mut(idx) {
                e.scale = 1.0;
                e.opacity = 1.0;
                *config_dirty = true;
            }
        }
        panels::MenuAction::ToggleGravity(idx) => {
            if let Some(e) = scene.entities.get_mut(idx) {
                e.physics.toggle();
                *config_dirty = true;
            }
        }
        panels::MenuAction::BringForward(idx) => {
            if let Some(e) = scene.entities.get_mut(idx) {
                e.z_index += 10;
                scene.mark_visible_dirty();
                *config_dirty = true;
            }
        }
        panels::MenuAction::SendBackward(idx) => {
            if let Some(e) = scene.entities.get_mut(idx) {
                e.z_index -= 10;
                scene.mark_visible_dirty();
                *config_dirty = true;
            }
        }
    }
}

/// Apply a library "Add to scene" outcome to scene + library index +
/// toast queue. Mirrors `app/outcomes.rs::handle_library_outcome`
/// (X11 path), adapted to free-function locals instead of `&mut self`
/// — there's no `App` on this run loop. `center` is the drop position
/// (window-local, since the native path has no multi-window origin to
/// translate by).
fn handle_library_outcome(
    outcome: panels::LibraryOutcome,
    library_root: Option<&std::path::Path>,
    library: &mut Option<crate::asset_library::LibraryIndex>,
    scene: &mut Scene,
    toasts: &mut ToastQueue,
    config_dirty: &mut bool,
    center: (f32, f32),
) {
    use crate::drop_validate::resolve_library_asset;

    let Some(root) = library_root else {
        tracing::warn!("Library outcome received but no library_root is set; ignoring.");
        return;
    };
    // Same M2 hardening as the X11 path: canonicalise both sides and
    // reject anything that escapes the asset root before this ever
    // reaches a decoder.
    let rel_path = std::path::Path::new(&outcome.relative_path);
    let abs_path = match resolve_library_asset(root, rel_path) {
        Ok(p) => p,
        Err(reason) => {
            tracing::warn!("Library asset {} rejected: {reason}", redact_path(rel_path));
            tracing::debug!("Rejected library relative path: {}", outcome.relative_path);
            let mut args = fluent::FluentArgs::new();
            args.set("reason", reason.clone());
            toasts.warn(crate::i18n::t_args("toast-rejected", &args));
            return;
        }
    };
    if let Err(reason) = pre_validate_dropped_file(&abs_path) {
        tracing::warn!(
            "Library asset {} rejected: {reason}",
            redact_path(&abs_path)
        );
        tracing::debug!("Rejected library full path: {}", abs_path.display());
        let mut args = fluent::FluentArgs::new();
        args.set("reason", reason.clone());
        toasts.warn(crate::i18n::t_args("toast-rejected", &args));
        return;
    }
    match scene.add_entity_from_path(&abs_path, center.0, center.1) {
        Ok(_) => {
            let mut args = fluent::FluentArgs::new();
            args.set("name", outcome.display_name.clone());
            toasts.success(crate::i18n::t_args("library-asset-added-toast", &args));
            if let Some(idx) = library.as_mut() {
                if let Some(asset) = idx.assets.iter_mut().find(|a| a.id == outcome.asset_id) {
                    asset.last_used_at = Some(std::time::SystemTime::now());
                }
                let _ = idx.save(&crate::asset_library::LibraryIndex::default_path());
            }
            *config_dirty = true;
        }
        Err(e) => {
            tracing::warn!(
                "Library add failed for {}: {e}",
                redact_path(std::path::Path::new(&outcome.relative_path))
            );
            tracing::debug!("Failed relative path: {}", outcome.relative_path);
            let mut args = fluent::FluentArgs::new();
            args.set("name", outcome.display_name);
            toasts.error(crate::i18n::t_args("library-asset-add-failed-toast", &args));
        }
    }
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
            {
                let mut args = fluent::FluentArgs::new();
                args.set("theme", theme.label());
                toasts.success(crate::i18n::t_args("toast-theme-switched", &args));
            }
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
                            {
                                let mut args = fluent::FluentArgs::new();
                                args.set("error", e.to_string());
                                toasts
                                    .warn(crate::i18n::t_args("toast-preset-entry-failed", &args));
                            }
                        }
                    }
                }
            }
            {
                let mut args = fluent::FluentArgs::new();
                args.set("name", preset.name);
                toasts.success(crate::i18n::t_args("toast-preset-loaded", &args));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(name: &str, x: i32, y: i32) -> MonitorInfo {
        MonitorInfo {
            name: name.to_string(),
            x,
            y,
            width: 1920,
            height: 1080,
            scale_factor: 1.0,
            is_primary: false,
        }
    }

    #[test]
    fn diff_extra_plan_creates_missing_and_removes_stale() {
        let plan = vec![monitor("right", 1920, 0), monitor("left", -1920, 0)];
        let current = vec!["right".to_string(), "gone".to_string()];
        let (stale, new) = diff_extra_plan(&plan, &current);
        assert_eq!(stale, vec!["gone".to_string()]);
        assert_eq!(new.len(), 1);
        assert_eq!(new[0].name, "left");
    }

    #[test]
    fn diff_extra_plan_no_changes_when_already_in_sync() {
        let plan = vec![monitor("right", 1920, 0)];
        let current = vec!["right".to_string()];
        let (stale, new) = diff_extra_plan(&plan, &current);
        assert!(stale.is_empty());
        assert!(new.is_empty());
    }

    #[test]
    fn diff_extra_plan_empty_plan_clears_everything() {
        let plan: Vec<MonitorInfo> = Vec::new();
        let current = vec!["right".to_string(), "left".to_string()];
        let (stale, new) = diff_extra_plan(&plan, &current);
        assert_eq!(stale.len(), 2);
        assert!(new.is_empty());
    }

    #[test]
    fn compute_primary_origin_is_identity_without_extras() {
        let monitors = vec![monitor("right", 1920, 0)];
        assert_eq!(
            compute_primary_origin(false, Some("right"), &monitors),
            (0.0, 0.0)
        );
    }

    #[test]
    fn compute_primary_origin_uses_known_output_position() {
        let monitors = vec![monitor("left", -1920, 0), monitor("right", 1920, 100)];
        assert_eq!(
            compute_primary_origin(true, Some("right"), &monitors),
            (1920.0, 100.0)
        );
    }

    #[test]
    fn compute_primary_origin_falls_back_to_identity_when_output_unknown() {
        let monitors = vec![monitor("right", 1920, 0)];
        assert_eq!(
            compute_primary_origin(true, Some("nonexistent"), &monitors),
            (0.0, 0.0)
        );
        assert_eq!(compute_primary_origin(true, None, &monitors), (0.0, 0.0));
    }
}
