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
//! - Entity selection and drag: left-click selects and picks a
//!   character up, motion moves it, release drops it (a press/release
//!   that never moved pokes instead). Same `DragController` and the
//!   same tap/poke semantics as `App::handle_mouse_input`.
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
use crate::drop_validate::redact_path;
use crate::entity::Entity;
use crate::error::{AnimaError, Result};
use crate::event::AnimaEvent;
use crate::input::selection::SelectionState;
use crate::keybindings::{Action, KeyChord};
use crate::monitor::{self, MonitorInfo, WindowPlan};
use crate::outcomes::{self, OutcomeCtx, ShimejiImport};
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
#[tracing::instrument(skip(scene, config, command_rx, portal_rx))]
pub fn run_native(
    mut scene: Scene,
    mut config: AppConfig,
    // Outside commands: the D-Bus activation service and the tray both send here.
    command_rx: Option<mpsc::Receiver<AnimaEvent>>,
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
    // `Single` names the monitor the overlay belongs on; the other modes
    // let the compositor choose.
    let preferred_output = match &config.global.monitor_mode {
        crate::monitor::MonitorMode::Single { name } => Some(name.clone()),
        _ => None,
    };
    let mut layer = LayerWindow::try_create(preferred_output.as_deref())?;
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
    let mut drag = crate::input::drag::DragController::new();
    let mut toasts = ToastQueue::default();
    let mut config_dirty = false;
    // What a panel outcome may touch (`crate::outcomes`), borrowed per
    // call so the loop keeps using these locals in between.
    macro_rules! outcome_ctx {
        () => {
            OutcomeCtx {
                scene: &mut scene,
                selection: &mut selection,
                toasts: &mut toasts,
                config_dirty: &mut config_dirty,
                renderer: Some(&mut renderer),
            }
        };
    }
    let mut config_watch = crate::config_watch::ConfigWatcher::new();
    // Frame timings for the perf overlay (R27). This loop had no sampler
    // at all, so `TogglePerfOverlay` was one of five rebindable actions
    // that did nothing here — and the README's parity table called the
    // overlay stable on both backends.
    let mut perf_sampler = crate::perf::PerfSampler::default();
    let mut perf_overlay_visible = false;
    // Soak metrics (W.1). Previously wired only into the winit render
    // loop, so `ANIMA_SOAK_METRICS` was silently a no-op here: the
    // memory-regression harness covered one of the two backends and gave
    // no indication it was skipping the other.
    let mut soak = crate::soak::SoakRecorder::from_env();
    let mut warnings: BTreeSet<Warning> = BTreeSet::new();
    if crate::config::loaded_defaults_over_unreadable_config() {
        warnings.insert(Warning::ConfigUnreadable);
    }
    // Right-click context menu state, mirroring `app::ContextMenuState`
    // on the X11 path. Persists across frames while the menu is open.
    let mut context_menu_state: Option<crate::app::ContextMenuState> = None;
    tracing::info!("Native Wayland renderer initialized ({width}×{height})");

    // Discover + load + merge-scan the asset library — same sequence
    // as the X11 path's `handle_resumed` (src/app/lifecycle.rs).
    // Errors are logged but never fatal — an empty library is fine.
    let mut library: Option<crate::asset_library::LibraryIndex> = None;
    let mut library_root: Option<std::path::PathBuf> = None;
    // A Shimeji pack import running off the UI thread, if any.
    let mut pending_shimeji: Option<ShimejiImport> = None;
    // The desktop's file chooser, open after "Add file…" until it answers.
    let mut pending_file_chooser: Option<crate::outcomes::FileChooserAdd> = None;
    // Actions picked in the command palette, run next frame through the
    // same `match` as the ones bound to keys.
    let mut palette_actions: Vec<Action> = Vec::new();
    // Whether an iteration draws (`FrameGate`).
    let mut frame_gate = FrameGate::default();
    // Edits to the scene, for undo and redo (`crate::undo`).
    let mut history = crate::undo::UndoHistory::default();
    // Whether the input region is currently widened for a drag
    // (`drag_region`).
    let mut drag_widened = false;
    // Rhai host for `Behavior::Script`, mirroring the winit path's
    // `App::script_host`. Scripts resolve against `library_root`.
    let mut script_host = crate::scripting::ScriptHost::new();
    let mut audio_host = crate::audio::AudioHost::new();
    if let Some(root) = crate::asset_library::ensure_asset_root() {
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
        toggle_button_units(&layer.monitors()),
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
        // Opens the perf frame before the blocking dispatch, so the wait
        // for the compositor lands in `Idle` rather than vanishing — the
        // same placement the winit loop uses relative to its own wait.
        perf_sampler.begin_frame();
        let dispatched =
            dispatch_with_timeout(&mut layer.event_queue, &mut layer.state, FRAME_INTERVAL)?;
        // Whether anything happened this iteration that a frame should
        // show. Input and every other compositor event count; so does
        // whatever the loop picks up from its channels below.
        let mut activity = dispatched > 0;
        // A step to undo opens at the first thing someone does, before it
        // is applied (`crate::undo`).
        if layer.has_user_action() || egui_renderer.has_screen_reader_requests() {
            history.input(&scene, Instant::now());
        }

        // Capture-and-reset the previous frame's GPU op counters. The
        // counters are `Cell`s, so this is a shared borrow and does not
        // conflict with the `&mut renderer` uses below.
        let (gpu_uploads, gpu_draws) = renderer.shared.take_frame_gpu_counters();

        if layer.state.close_requested {
            tracing::info!("Layer surface closed by compositor — exiting.");
            break;
        }

        // Pick up any resize the compositor sent us.
        if let Some((new_w, new_h)) = layer.state.pending_size.take() {
            if new_w != renderer.primary.window_width || new_h != renderer.primary.window_height {
                renderer.resize(new_w, new_h);
                // Re-derive from the live state: this used to re-apply the
                // pass-through corner unconditionally, so a resize while
                // in edit mode silently reverted the surface to
                // click-through, and a resize while hidden made it
                // clickable again.
                let button = toggle_button_units(&layer.monitors());
                let region = region_for_state(
                    overlay_hidden,
                    layer.state.edit_mode || drag_widened,
                    new_w,
                    new_h,
                    button,
                );
                layer.set_input_region(region)?;
                tracing::info!("Layer surface resized to {new_w}×{new_h}");
            }
        }

        // Hot-reload. The README lists it as a feature and its parity
        // table calls it stable on both backends; this loop never did it
        // at all, so editing config.toml with the overlay running on
        // Wayland did nothing (R28).
        let poll = config_watch.poll(config_dirty);
        activity |= !matches!(poll, crate::config_watch::Poll::Idle);
        let reloaded = matches!(poll, crate::config_watch::Poll::Ready(_));
        crate::config_watch::handle(poll, &mut outcome_ctx!(), &mut config, &mut warnings);
        if reloaded {
            // The scene was replaced from the file: the steps no longer
            // describe it.
            history.clear();
        }
        if let Some(import) = &pending_shimeji {
            if import.poll(&mut outcome_ctx!()).is_some() {
                pending_shimeji = None;
                activity = true;
            }
        }
        if let Some(chooser) = &pending_file_chooser {
            if let Some(added) = chooser.poll(&mut outcome_ctx!()) {
                pending_file_chooser = None;
                activity = true;
                if added > 0 {
                    config_dirty = true;
                }
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
        // Widen the input region while a drag is over us in pass-through,
        // and put it back when the drag ends.
        let widen = drag_region(overlay_hidden, layer.state.edit_mode, layer.state.drag_over);
        if widen != drag_widened {
            drag_widened = widen;
            let region = region_for_state(
                overlay_hidden,
                layer.state.edit_mode || widen,
                renderer.primary.window_width,
                renderer.primary.window_height,
                toggle_button_units(&monitors_now),
            );
            if let Err(e) = layer.set_input_region(region) {
                tracing::warn!("drag region: {e}");
            }
        }

        // The per-monitor extra surfaces follow the primary: open to
        // input in edit mode, or while a drag is over the overlay, and
        // click-through otherwise. Their offsets into the primary's
        // space come from the same output snapshot as `primary_origin`,
        // so a click, drag or drop on another monitor lands in the
        // right global place.
        layer.sync_extra_layers(
            extras_take_input(overlay_hidden, layer.state.edit_mode, widen),
            |name| {
                monitors_now
                    .iter()
                    .find(|m| m.name == name)
                    .map(|m| (m.x as f32 - primary_origin.0, m.y as f32 - primary_origin.1))
            },
        );

        // Files dropped on the overlay (E.3). Each drop carries the point
        // it landed on in the primary surface's space (shifted there from
        // an extra surface's); `primary_origin` turns that into the global
        // space the scene uses. Same as the winit path,
        // through the same code: a Shimeji pack folder goes to the
        // importer, anything else is validated, added, selected, toasted
        // and marked for saving.
        for dropped in layer.drain_dropped_files() {
            activity = true;
            history.input(&scene, Instant::now());
            let at = (
                dropped.at.0 + primary_origin.0,
                dropped.at.1 + primary_origin.1,
            );
            let mut added = false;
            for path in &dropped.paths {
                if outcomes::is_shimeji_pack(path) {
                    if pending_shimeji.is_none() {
                        pending_shimeji =
                            ShimejiImport::start(path, library_root.as_deref(), at, &mut toasts);
                    }
                } else if outcomes::add_dropped_file(path, at, &mut outcome_ctx!()).is_some() {
                    added = true;
                }
            }
            // As on winit, a drop opens edit mode so the new character can
            // be moved into place straight away. It matters more here: in
            // pass-through the overlay only takes input — drops included —
            // on the ⚙ corner, so that is where a drop lands.
            if added && !layer.state.edit_mode {
                flip_edit_mode(
                    &mut layer,
                    &monitors_now,
                    &mut config,
                    &scene,
                    &mut config_dirty,
                    &mut config_watch,
                    "Wayland, file drop",
                );
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
            if let Some(rx) = &command_rx {
                while let Ok(ev) = rx.try_recv() {
                    activity = true;
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
                    activity = true;
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
                // Through `flip_edit_mode`, like the keyboard and the ⚙
                // button. This path used to set the mode directly, so
                // leaving edit mode from the tray, a D-Bus call or a portal
                // shortcut skipped the save — R30 on a third entry point.
                flip_edit_mode(
                    &mut layer,
                    &monitors_now,
                    &mut config,
                    &scene,
                    &mut config_dirty,
                    &mut config_watch,
                    "Wayland, tray / D-Bus / portal",
                );
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
                let region = region_for_state(
                    overlay_hidden,
                    layer.state.edit_mode || drag_widened,
                    renderer.primary.window_width,
                    renderer.primary.window_height,
                    toggle_button_units(&monitors_now),
                );
                if let Err(e) = layer.set_input_region(region) {
                    tracing::warn!("visibility change: {e}");
                }
            }
            if quit {
                layer.state.close_requested = true;
            }
        }

        let events = layer.drain_egui_events();
        let modifiers = layer.modifiers();
        // A focused text field owns the keyboard: the palette's search box,
        // a numeric field being typed into, the keybinding capture widget.
        // Without this gate every character double-fires as a global
        // shortcut — typing "vi" into the command palette also toggled the
        // selection's visibility and dumped its info to the log (R25).
        // `egui_winit` reports key events as consumed on exactly this
        // condition, which is why the winit path never had the bug. The
        // open palette, an open list and the right-click menu own it too:
        // Escape there closed them and also left edit mode.
        let egui_owns_keyboard = egui_renderer.wants_keyboard() || context_menu_state.is_some();
        // Last frame's palette picks first, then this frame's keys — one
        // path for both, so an action cannot behave differently depending
        // on how it was asked for.
        let mut actions = std::mem::take(&mut palette_actions);
        activity |= !actions.is_empty();
        actions.extend(
            events
                .iter()
                .filter_map(|event| binding_chord(event, egui_owns_keyboard))
                .filter_map(|chord| config.keybindings.lookup(chord)),
        );
        for action in actions {
            match action {
                Action::ToggleEditMode => {
                    flip_edit_mode(
                        &mut layer,
                        &monitors_now,
                        &mut config,
                        &scene,
                        &mut config_dirty,
                        &mut config_watch,
                        "Wayland",
                    );
                }
                // The next four are not shareable — each one reaches for
                // something only this loop owns (its config, its shutdown
                // flag, its renderer's texture cache) — but every piece
                // they need is already here, and the in-app help screen
                // lists all four. They did nothing at all on this backend
                // until now (R27).
                Action::SaveNow => match sync_and_save(&mut config, &scene) {
                    Ok(()) => {
                        config_dirty = false;
                        // Our own write changes the file's mtime and would
                        // otherwise look exactly like somebody else's edit
                        // to the watcher above, costing a pointless full
                        // reload — selection and every texture included.
                        config_watch.note_saved();
                        tracing::info!("Config saved manually");
                        toasts.success(crate::i18n::t("toast-config-saved"));
                    }
                    Err(e) => {
                        // `config_dirty` deliberately stays set: a
                        // transient failure must let the next edit retry
                        // rather than silently drop the scene, which is
                        // the same rule `save_config_if_needed` follows
                        // on the winit path.
                        tracing::warn!("Failed to save config: {e}");
                        let mut args = fluent::FluentArgs::new();
                        args.set("error", e.to_string());
                        toasts.error(crate::i18n::t_args("toast-save-failed", &args));
                    }
                },
                Action::TogglePerfOverlay => {
                    perf_overlay_visible = !perf_overlay_visible;
                    tracing::debug!(
                        "Perf overlay {}",
                        if perf_overlay_visible {
                            "shown"
                        } else {
                            "hidden"
                        }
                    );
                }
                Action::QuitWithSave => {
                    // The shutdown path at the end of `run` persists
                    // `config_dirty` on its way out, so this needs no save
                    // of its own — and doing one here would write the
                    // scene twice on every quit.
                    tracing::info!("Quit action — saving and exiting");
                    layer.state.close_requested = true;
                }
                Action::DeleteSelected | Action::DuplicateSelected => {
                    if let Some(idx) = selection.selected_index() {
                        // The same functions the right-click menu and the
                        // winit path use (`crate::outcomes`), so none of
                        // the three can drift from the others.
                        let mut ctx = outcome_ctx!();
                        if action == Action::DeleteSelected {
                            outcomes::delete_entity(idx, &mut ctx);
                        } else {
                            outcomes::duplicate_entity(idx, &mut ctx);
                        }
                    }
                }
                // Everything that only touches the scene and the selection
                // is shared with the winit path. Before this, the table was
                // consulted here and matched ToggleEditMode alone, so the
                // other twenty-seven rebindable actions were inert on this
                // backend — the user could rebind them and see the config
                // persist while nothing happened (R19).
                other => {
                    let mut ctx = crate::keybindings::shared::ActionCtx {
                        scene: &mut scene,
                        selection: &mut selection,
                        config_dirty: &mut config_dirty,
                        shift_held: layer.state.last_modifiers.shift,
                        bounds: monitor::covered_bounds(
                            &plan,
                            (
                                renderer.primary.window_width as f32,
                                renderer.primary.window_height as f32,
                            ),
                        ),
                        monitors: &monitors_now,
                        toasts: &mut toasts,
                        history: &mut history,
                    };
                    crate::keybindings::shared::dispatch_shared(other, &mut ctx);
                }
            }
        }

        // Right-click on an entity opens the context menu and selects
        // it, same gating and behavior as the X11 path's
        // `handle_mouse_input` (src/app/input.rs). Entity-less right
        // clicks (empty space) are ignored.
        //
        // Presses that egui owns are skipped. The winit path gets this for
        // free — `App::window_event` forwards to egui first and returns
        // early when the event is consumed — and without the equivalent
        // here, a left click anywhere on the settings panel's background
        // fell through to `entity_at_point`, found nothing, and cleared
        // the selection. So clicking blank space in the panel threw away
        // the very entity whose Inspector you were reading.
        //
        // Only the *press* arms are gated. Motion and release stay live so
        // a drag that began on a sprite still tracks and still finishes if
        // the pointer crosses the panel on the way.
        let egui_owns_pointer = egui_renderer.owns_pointer();
        if layer.state.edit_mode {
            for event in &events {
                match event {
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Secondary,
                        pressed: true,
                        ..
                    } if !egui_owns_pointer => {
                        if let Some(idx) = scene
                            .entity_at_point(pos.x + primary_origin.0, pos.y + primary_origin.1)
                        {
                            selection.select(idx);
                            context_menu_state = Some(crate::app::ContextMenuState {
                                entity_idx: idx,
                                pos: *pos,
                                // Armed after the first showing — see ContextMenuState.
                                armed: false,
                            });
                        }
                    }
                    // Left press: select and begin a drag, mirroring
                    // `App::handle_mouse_input` on the winit path. Both were
                    // missing here — selection was right-click only, and
                    // dragging a character (the core interaction of a
                    // desktop-pet overlay) was not implemented on this
                    // backend at all.
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        ..
                    } if !egui_owns_pointer => {
                        let (gx, gy) = (pos.x + primary_origin.0, pos.y + primary_origin.1);
                        match scene.entity_at_point(gx, gy) {
                            Some(idx) => {
                                selection.select(idx);
                                if let Some(entity) = scene.entities.get_mut(idx) {
                                    entity.physics.freeze();
                                    entity.dragging = true;
                                    drag.start_drag(idx, gx - entity.x, gy - entity.y, gx, gy);
                                }
                            }
                            None => selection.deselect(),
                        }
                    }
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        ..
                    } if drag.is_dragging() => {
                        let (gx, gy) = (pos.x + primary_origin.0, pos.y + primary_origin.1);
                        // A press/release that never moved is a *tap*, not a
                        // drag — poke the mascot instead of just dropping it.
                        let tapped = drag.was_tap(gx, gy, crate::constants::POKE_TAP_RADIUS);
                        if tapped {
                            // The hop that follows is play, not an edit.
                            history.finish(&scene);
                        }
                        let poke_bounds = monitor::covered_bounds(
                            &plan,
                            (
                                renderer.primary.window_width as f32,
                                renderer.primary.window_height as f32,
                            ),
                        );
                        if let Some(idx) = drag.dragging_entity() {
                            if let Some(entity) = scene.entities.get_mut(idx) {
                                entity.physics.unfreeze();
                                entity.dragging = false;
                                if tapped {
                                    entity.poke(gx, poke_bounds);
                                }
                            }
                        }
                        drag.end_drag();
                        config_dirty = true;
                    }
                    egui::Event::PointerMoved(pos) if drag.is_dragging() => {
                        let (gx, gy) = (pos.x + primary_origin.0, pos.y + primary_origin.1);
                        if let Some((idx, nx, ny)) = drag.update(gx, gy) {
                            if let Some(entity) = scene.entities.get_mut(idx) {
                                entity.x = nx;
                                entity.y = ny;
                                // Relocating invalidates any Bounce rest
                                // position, or the sprite springs back to
                                // where it was picked up.
                                entity.behavior_state.bounce_invalidate();
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Tick the simulation. screen_w / screen_h match the surface so
        // walk-around behaviors stay inside the visible area.
        if let Some(soak) = soak.as_mut() {
            soak.maybe_sample(
                crate::perf::read_rss_kib(),
                scene.total_decoded_bytes(),
                renderer.shared.textures.len(),
                None,
            );
        }
        scene.set_reduced_motion(config.global.reduced_motion);
        scene.set_hover_startle(config.global.hover_startle);
        {
            let _s = perf_sampler.scope(crate::perf::Category::SceneUpdate);
            scene.tick(
                crate::monitor::covered_bounds(
                    &plan,
                    (
                        renderer.primary.window_width as f32,
                        renderer.primary.window_height as f32,
                    ),
                ),
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
                library_root
                    .as_deref()
                    .map(|root| crate::scripting::ScriptContext {
                        host: &mut script_host,
                        audio: &mut audio_host,
                        root,
                    }),
            );
        }

        for (path, err) in audio_host.take_new_failures() {
            let mut args = fluent::FluentArgs::new();
            args.set("script", path);
            args.set("error", err);
            toasts.error(crate::i18n::t_args("script-failed-toast", &args));
        }

        // Update any dirty textures (animation frame advance).
        // The prune sweeps textures orphaned by preset Replace — same
        // rationale as the winit render loop.
        renderer.prune_stale_textures(&scene.entities);
        for entity in &mut scene.entities {
            if entity.texture_dirty {
                renderer.ensure_texture(entity);
                entity.texture_dirty = false;
                // An animation showing its next frame: this is what wakes
                // a still scene with playing sprites, at their own rate —
                // unless the overlay is hidden and shows nothing.
                activity |= !overlay_hidden;
            }
        }

        // Draw only for a reason. The loop still wakes every frame
        // interval — draining its channels and ticking the scene is cheap
        // — but it drew an unchanged scene sixty times a second, 84% of a
        // core in the rig with everything paused, where the winit loop
        // sleeps (`crate::pacing`).
        toasts.prune();
        activity |= egui_renderer.screen_reader_woke();
        let now = Instant::now();
        history.settle(
            &scene,
            egui_renderer.input_in_progress()
                || pending_file_chooser.is_some()
                || pending_shimeji.is_some(),
            now,
        );
        let moving = !overlay_hidden
            && matches!(
                crate::pacing::redraw_pacing(
                    &scene,
                    layer.state.edit_mode || perf_overlay_visible || !toasts.is_empty(),
                ),
                crate::pacing::RedrawPacing::Continuous
            );
        if !frame_gate.should_draw(now, activity || moving || egui_renderer.repaint_due(now)) {
            perf_sampler.end_frame();
            if let Some(rest) = FRAME_INTERVAL.checked_sub(frame_start.elapsed()) {
                std::thread::sleep(rest);
            }
            continue;
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
        egui_renderer.ensure_theme(config.global.theme);
        // Timed with an explicit instant rather than a `scope` guard:
        // the guard would have to be dropped in both match arms, and
        // holding it across the arm keeps `perf_sampler` mutably borrowed
        // to the end of the frame.
        let submit_start = Instant::now();
        match renderer.render(
            &drawn,
            &scene.groups,
            layer.state.edit_mode,
            selected_id.as_deref(),
            primary_origin,
        ) {
            Ok(output) => {
                perf_sampler.add(crate::perf::Category::WgpuSubmit, submit_start.elapsed());
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
                // Same scale the input region is sized with, so the
                // drawn button and its clickable area agree.
                let pixels_per_point = ui_pixels_per_point(monitors);
                let edit_mode_snapshot = layer.state.edit_mode;
                let hidden_snapshot = overlay_hidden;
                // Snapshot the AccessKit flag BEFORE taking its mutable
                // borrow, same trick as the X11 path uses; the renderer
                // gates egui's tree on it.
                let accesskit_snapshot = config.global.accesskit_enabled;
                let mut toggle_requested = false;
                let mut palette_outcome: Option<panels::PaletteOutcome> = None;
                let mut library_outcome: Option<panels::LibraryOutcome> = None;
                let mut menu_outcome: Option<panels::ContextMenuOutcome> = None;
                let mut perf_export_request = false;
                // Built here, while `scene` and `renderer` are still
                // freely readable — after this point the closure takes
                // disjoint `&mut` borrows of both.
                let gpu_stats = crate::ui::perf_overlay::GpuStats {
                    decoded_bytes: scene.total_decoded_bytes(),
                    texture_bytes: renderer.shared.texture_bytes(),
                    texture_count: renderer.shared.textures.len(),
                    uploads_last_frame: gpu_uploads,
                    draws_last_frame: gpu_draws,
                };
                let mut shimeji_import: Option<String> = None;
                let mut add_file_requested = false;
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
                let add_file_requested_ref = &mut add_file_requested;
                let monitors_ref = monitors.as_slice();
                let toasts_ref = &toasts;
                let toggle_requested_ref = &mut toggle_requested;
                let palette_ref = &mut palette_outcome;
                let library_ref = &mut library_outcome;
                let menu_outcome_ref = &mut menu_outcome;
                let perf_sampler_ref = &perf_sampler;
                let perf_export_ref = &mut perf_export_request;
                let perf_visible_snapshot = perf_overlay_visible;
                let egui_start = Instant::now();
                egui_renderer.set_accesskit_allowed(accesskit_snapshot);
                egui_renderer.set_window_state(layer.has_keyboard_focus(), primary_origin, size);
                egui_renderer.render(
                    &renderer.shared.device,
                    &renderer.shared.queue,
                    &view,
                    size,
                    pixels_per_point,
                    events,
                    modifiers,
                    |ctx| {
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
                                // Native Wayland exposes no window positions.
                                false,
                                // A layer surface belongs to one output, so no single
                                // surface can span the desktop here.
                                false,
                                reduced_motion_mut,
                                hover_startle_mut,
                                monitors_ref,
                                library.as_ref(),
                                library_ref,
                                keybindings_mut,
                                collapse_state_mut,
                                accesskit_mut,
                                // `crate::a11y` carries the AT-SPI bridge.
                                true,
                                warnings_ref,
                                last_seen_whats_new_mut,
                                hotkey_backend_ref,
                                shimeji_import_ref,
                                add_file_requested_ref,
                            );
                            if edit_mode_snapshot {
                                if let Some(state) = &menu_state {
                                    *menu_outcome_ref = Some(panels::context_menu(ctx, state));
                                }
                                *palette_ref = panels::command_palette(
                                    ctx,
                                    keybindings_mut,
                                    selection_mut.selected_index().is_some(),
                                );
                                panels::toasts(ctx, toasts_ref);
                            }
                            // Above every panel, so someone chasing a
                            // stutter doesn't have to hunt for it behind
                            // one — same placement as the winit path.
                            if perf_visible_snapshot
                                && crate::ui::perf_overlay::show(
                                    ctx,
                                    perf_sampler_ref,
                                    crate::perf::read_rss_kib(),
                                    gpu_stats,
                                )
                                .is_some()
                            {
                                *perf_export_ref = true;
                            }
                        }
                    },
                );
                perf_sampler.add(crate::perf::Category::EguiPaint, egui_start.elapsed());
                // A focused text field turns the input method on, at its
                // caret; losing it turns it off.
                layer.sync_ime(egui_renderer.ime_caret());
                if perf_export_request {
                    match crate::perf::export_snapshot(&perf_sampler) {
                        Ok(path) => {
                            // Toast shows the full path — the user asked
                            // for the export and wants to find the file.
                            // The log redacts, so journald doesn't carry
                            // their home directory.
                            tracing::info!("Perf snapshot written: {}", redact_path(&path));
                            tracing::debug!("Perf snapshot full path: {}", path.display());
                            let mut args = fluent::FluentArgs::new();
                            args.set("path", path.display().to_string());
                            toasts.success(crate::i18n::t_args("toast-perf-snapshot", &args));
                        }
                        Err(e) => {
                            tracing::error!("Perf snapshot failed: {e}");
                            let mut args = fluent::FluentArgs::new();
                            args.set("error", e.to_string());
                            toasts.error(crate::i18n::t_args("toast-perf-snapshot-failed", &args));
                        }
                    }
                }
                {
                    let _s = perf_sampler.scope(crate::perf::Category::Present);
                    renderer.present(output);
                }
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
                    flip_edit_mode(
                        &mut layer,
                        &monitors_now,
                        &mut config,
                        &scene,
                        &mut config_dirty,
                        &mut config_watch,
                        "Wayland, toggle button",
                    );
                }
                // Palette / library outcomes apply outside the egui
                // closure where we can take &mut renderer + &mut toasts
                // without conflicting.
                if add_file_requested && pending_file_chooser.is_none() {
                    pending_file_chooser = Some(crate::outcomes::FileChooserAdd::start((
                        renderer.primary.window_width as f32 / 2.0 + primary_origin.0,
                        renderer.primary.window_height as f32 / 2.0 + primary_origin.1,
                    )));
                }
                if let Some(path) = shimeji_import {
                    // Off the UI thread, like a dropped pack. This used to
                    // import synchronously right here, freezing the overlay
                    // for as long as the sprite copy took.
                    if pending_shimeji.is_none() {
                        let at = (
                            renderer.primary.window_width as f32 / 2.0 + primary_origin.0,
                            renderer.primary.window_height as f32 / 2.0 + primary_origin.1,
                        );
                        pending_shimeji = ShimejiImport::start(
                            &crate::config::AppConfig::resolve_asset_path(&path),
                            library_root.as_deref(),
                            at,
                            &mut toasts,
                        );
                    }
                }
                if let Some(out) = menu_outcome {
                    match out {
                        panels::ContextMenuOutcome::Open { settled } => {
                            // Showing, and the click that opened it is
                            // over: a *subsequent* click may now dismiss it.
                            if let Some(state) = context_menu_state.as_mut() {
                                state.armed |= settled;
                            }
                        }
                        panels::ContextMenuOutcome::Close => {
                            context_menu_state = None;
                        }
                        panels::ContextMenuOutcome::Action(action) => {
                            outcomes::apply_menu_action(action, &mut outcome_ctx!());
                            context_menu_state = None;
                        }
                    }
                }
                match palette_outcome {
                    Some(panels::PaletteOutcome::RunAction(action)) => palette_actions.push(action),
                    Some(panels::PaletteOutcome::AddFile) => {
                        if pending_file_chooser.is_none() {
                            pending_file_chooser = Some(crate::outcomes::FileChooserAdd::start((
                                renderer.primary.window_width as f32 / 2.0 + primary_origin.0,
                                renderer.primary.window_height as f32 / 2.0 + primary_origin.1,
                            )));
                        }
                    }
                    Some(out) => {
                        outcomes::apply_palette_outcome(out, &mut outcome_ctx!(), &mut config)
                    }
                    None => {}
                }
                if let Some(out) = library_outcome {
                    // The middle of the primary output, in the global
                    // coordinates the scene uses.
                    let at = (
                        renderer.primary.window_width as f32 / 2.0 + primary_origin.0,
                        renderer.primary.window_height as f32 / 2.0 + primary_origin.1,
                    );
                    outcomes::apply_library_outcome(
                        out,
                        &mut outcome_ctx!(),
                        library_root.as_deref(),
                        &mut library,
                        at,
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

        // Closed before the pacing sleep, so the overlay reports the work
        // the frame actually did rather than a flat ~16 ms for every
        // frame regardless of load — which would make the number useless
        // for the one thing it exists to answer.
        perf_sampler.end_frame();

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

/// Whether a loop iteration draws a frame: when there is a reason to, and
/// otherwise once per [`crate::pacing::IDLE_HEARTBEAT`], so a change the
/// loop did not notice still shows within it.
#[derive(Default)]
struct FrameGate {
    last_draw: Option<Instant>,
}

impl FrameGate {
    fn should_draw(&mut self, now: Instant, reason: bool) -> bool {
        let draw = reason
            || self.last_draw.is_none_or(|last| {
                now.saturating_duration_since(last) >= crate::pacing::IDLE_HEARTBEAT
            });
        if draw {
            self.last_draw = Some(now);
        }
        draw
    }
}

/// egui's points-per-buffer-pixel for the native surface.
///
/// The layer surface leaves `wl_surface.set_buffer_scale` at 1, so buffer
/// pixels and surface-local units are the same thing. Taking the largest
/// scale among the outputs the surface may sit on keeps text crisp —
/// undershooting blurs it, overshooting only makes glyphs bigger than
/// necessary. True per-surface buffer scaling is the deferred hi-DPI work
/// noted in `CompositorHandler::scale_factor_changed`.
fn ui_pixels_per_point(monitors: &[MonitorInfo]) -> f32 {
    monitors
        .iter()
        .map(|m| m.scale_factor as f32)
        .fold(1.0_f32, f32::max)
}

/// The ⚙ toggle button's size in the surface-local units the input
/// region is specified in.
///
/// egui lays the button out in *points* and paints it at
/// [`ui_pixels_per_point`], so it covers `size × scale` buffer pixels —
/// which, at buffer scale 1, are surface-local units. Passing the raw
/// constant to `set_input_region` therefore left the clickable corner a
/// fraction of the drawn button on any HiDPI output: exactly the
/// points-vs-pixels mismatch the X11 path had.
fn toggle_button_units(monitors: &[MonitorInfo]) -> u32 {
    let px = TOGGLE_BUTTON_SIZE as f32 * ui_pixels_per_point(monitors);
    if !px.is_finite() {
        return TOGGLE_BUTTON_SIZE;
    }
    px.round().clamp(1.0, u32::MAX as f32) as u32
}

/// The chord a drained egui event should be looked up as, if any.
///
/// `egui_owns_keyboard` is `Context::wants_keyboard_input()`: a focused
/// text field, the palette's search box, the keybinding capture widget.
/// While that is true the key belongs to the widget and must not also fire
/// a global shortcut — typing "vi" into the command palette used to toggle
/// the selection's visibility and dump its info as well (R25).
///
/// Split out of the event loop so the gate is testable: the loop needs a
/// compositor, a renderer and a scene, but this decision is one boolean
/// and one event. The winit path gets the same answer for free, from
/// `egui_winit` reporting key events as consumed on exactly this condition.
fn binding_chord(event: &egui::Event, egui_owns_keyboard: bool) -> Option<KeyChord> {
    if egui_owns_keyboard {
        return None;
    }
    match event {
        egui::Event::Key {
            key,
            physical_key,
            pressed: true,
            modifiers,
            ..
        } => KeyChord::from_egui_event(*key, *physical_key, *modifiers),
        _ => None,
    }
}

/// The input region matching the current overlay state.
///
/// Centralised because the region has to be re-derived from *all* of
/// hidden/edit-mode/size on every event that can change any of them. The
/// resize path in particular used to re-apply the pass-through corner
/// unconditionally, silently dropping edit mode's full-surface region on
/// any compositor-driven resize.
fn region_for_state(
    hidden: bool,
    edit_mode: bool,
    surface_w: u32,
    surface_h: u32,
    button: u32,
) -> Option<InputRect> {
    if hidden {
        None
    } else if edit_mode {
        Some(InputRect::full(surface_w, surface_h))
    } else {
        Some(InputRect::toggle_button_corner(surface_w, button))
    }
}

/// Whether a drag should widen the input region to the whole surface.
///
/// In pass-through the overlay takes input — drops included — only on the
/// ⚙ corner, because the compositor offers a drop only to a surface whose
/// input region is under the pointer. So a file could be dropped there and
/// nowhere else, and landed under the button. Once a drag reaches the
/// corner, the whole surface becomes the target for the rest of that drag:
/// drop it where the character should be. The region goes back when the
/// drag ends, dropped or not.
///
/// In edit mode the region is already the whole surface, and a hidden
/// overlay takes no input at all, so neither needs it.
fn drag_region(hidden: bool, edit_mode: bool, drag_over: bool) -> bool {
    drag_over && !edit_mode && !hidden
}

/// Whether the per-monitor extra surfaces take input: whenever the
/// primary takes it over its whole surface — edit mode, or a drag that
/// has widened the pass-through region — and the overlay is showing.
fn extras_take_input(hidden: bool, edit_mode: bool, drag_widened: bool) -> bool {
    !hidden && (edit_mode || drag_widened)
}

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
) -> Result<usize> {
    use std::os::fd::AsRawFd;

    let to_err =
        |e: wayland_client::DispatchError| AnimaError::other(format!("wayland dispatch: {e}"));

    // Already-buffered events first, exactly as blocking_dispatch does.
    let buffered = queue.dispatch_pending(state).map_err(to_err)?;
    if buffered > 0 {
        return Ok(buffered);
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

    queue.dispatch_pending(state).map_err(to_err)
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

/// Flip edit mode, persisting on the way out.
///
/// Every way in comes through here — the keyboard shortcut, the ⚙ button,
/// a file drop, and the tray / D-Bus / portal channel. The
/// save used to live in the button's handler alone, so leaving edit mode
/// with the keyboard kept every change in memory only — rebind a
/// shortcut, press Escape, and the binding was gone (R30). The winit path
/// avoids this by construction: its save sits inside
/// `App::toggle_edit_mode`, which both of its entry points call.
///
/// `via` only colours the log line, and it earns its place: the two
/// spellings are what made the asymmetry visible in the first place.
#[allow(clippy::too_many_arguments)]
fn flip_edit_mode(
    layer: &mut LayerWindow,
    monitors: &[MonitorInfo],
    config: &mut AppConfig,
    scene: &Scene,
    config_dirty: &mut bool,
    watcher: &mut crate::config_watch::ConfigWatcher,
    via: &str,
) {
    let new_mode = !layer.state.edit_mode;
    if let Err(e) = layer.set_edit_mode(new_mode, toggle_button_units(monitors)) {
        tracing::warn!("Failed to flip input region on edit toggle: {e}");
        return;
    }
    tracing::info!("Edit mode {} ({via})", if new_mode { "on" } else { "off" });

    // Leaving edit mode with unsaved work → persist now. Two reasons, and
    // the second only became true once this backend learned to hot-reload
    // (R28): a dirty scene also *blocks* reloading, so a session left
    // dirty ignores every external edit to config.toml until something
    // else happens to save.
    if should_persist_on_exit(new_mode, *config_dirty) {
        match sync_and_save(config, scene) {
            Ok(()) => {
                *config_dirty = false;
                watcher.note_saved();
            }
            Err(e) => tracing::warn!("Config save failed: {e}"),
        }
    }
}

/// Whether flipping edit mode to `new_mode` should write the config.
///
/// Entering edit mode never saves — nothing has changed yet. Leaving it
/// saves only if something did. Split out so the rule is testable: the
/// function around it needs a layer surface and a compositor, and this
/// is two booleans.
fn should_persist_on_exit(new_mode: bool, config_dirty: bool) -> bool {
    !new_mode && config_dirty
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_is_drawn_for_a_reason_or_on_the_heartbeat() {
        use crate::pacing::IDLE_HEARTBEAT;
        let mut gate = FrameGate::default();
        let t0 = Instant::now();
        let ms = Duration::from_millis;
        assert!(gate.should_draw(t0, false), "the first iteration draws");
        assert!(!gate.should_draw(t0 + ms(16), false), "nothing to show");
        assert!(gate.should_draw(t0 + ms(32), true), "a reason");
        let last = t0 + ms(32);
        assert!(!gate.should_draw(last + IDLE_HEARTBEAT - ms(1), false));
        assert!(
            gate.should_draw(last + IDLE_HEARTBEAT, false),
            "the heartbeat"
        );
    }

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

    fn scaled(name: &str, scale: f64) -> MonitorInfo {
        MonitorInfo {
            scale_factor: scale,
            ..monitor(name, 0, 0)
        }
    }

    // ── edit-mode persistence (R30) ──────────────────────────────────

    #[test]
    fn leaving_edit_mode_with_changes_saves() {
        assert!(should_persist_on_exit(false, true));
    }

    #[test]
    fn leaving_edit_mode_with_nothing_to_save_does_not_write() {
        assert!(!should_persist_on_exit(false, false));
    }

    /// Entering edit mode must never write: nothing has changed yet, and
    /// a write here would also stamp the file's mtime for no reason.
    #[test]
    fn entering_edit_mode_never_saves() {
        assert!(!should_persist_on_exit(true, true));
        assert!(!should_persist_on_exit(true, false));
    }

    // ── keybinding gate (R25) ────────────────────────────────────────

    fn key_event(key: egui::Key, pressed: bool) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    #[test]
    fn a_press_becomes_a_chord_when_egui_is_not_typing() {
        let ev = key_event(egui::Key::V, true);
        assert_eq!(
            binding_chord(&ev, false),
            KeyChord::from_egui(egui::Key::V, egui::Modifiers::NONE)
        );
    }

    /// The whole point of R25: while a text field has focus the keystroke
    /// belongs to the widget, not to the global shortcut table.
    #[test]
    fn a_press_is_swallowed_while_egui_wants_the_keyboard() {
        let ev = key_event(egui::Key::V, true);
        assert!(
            binding_chord(&ev, true).is_none(),
            "typing into the palette must not toggle visibility as well"
        );
    }

    /// Releases and non-key events were already ignored; keep it that way,
    /// or every shortcut fires twice.
    #[test]
    fn releases_and_other_events_are_never_chords() {
        assert!(binding_chord(&key_event(egui::Key::V, false), false).is_none());
        assert!(binding_chord(&egui::Event::Text("v".into()), false).is_none());
        assert!(binding_chord(&egui::Event::PointerGone, false).is_none());
    }

    /// egui paints the button at `points × pixels_per_point`, and at
    /// buffer scale 1 those are the same units the input region uses —
    /// so the region has to be scaled too, or the clickable corner is a
    /// fraction of the drawn button on HiDPI.
    #[test]
    fn toggle_button_units_track_the_ui_scale() {
        assert_eq!(toggle_button_units(&[scaled("a", 1.0)]), TOGGLE_BUTTON_SIZE);
        assert_eq!(
            toggle_button_units(&[scaled("a", 2.0)]),
            TOGGLE_BUTTON_SIZE * 2
        );
        // Mixed outputs: the largest scale wins, matching what egui is
        // told to paint at.
        assert_eq!(
            toggle_button_units(&[scaled("a", 1.0), scaled("b", 2.0)]),
            TOGGLE_BUTTON_SIZE * 2
        );
        // Fractional scaling rounds to whole units.
        assert_eq!(toggle_button_units(&[scaled("a", 1.5)]), 96);
        // No outputs yet → the unscaled constant, never zero.
        assert_eq!(toggle_button_units(&[]), TOGGLE_BUTTON_SIZE);
    }

    #[test]
    fn toggle_button_units_survive_a_nonsense_scale() {
        assert_eq!(
            toggle_button_units(&[scaled("a", f64::NAN)]),
            TOGGLE_BUTTON_SIZE
        );
        assert!(toggle_button_units(&[scaled("a", 0.0)]) >= 1);
    }

    /// The resize path used to re-apply the pass-through corner
    /// unconditionally, so resizing in edit mode reverted the surface to
    /// click-through and resizing while hidden made it clickable again.
    /// A drag in pass-through widens the region; nothing else does.
    #[test]
    fn only_a_pass_through_drag_widens_the_region() {
        assert!(drag_region(false, false, true));
        assert!(!drag_region(false, false, false), "no drag, no widening");
        assert!(!drag_region(false, true, true), "edit mode is already full");
        assert!(!drag_region(true, false, true), "hidden takes no input");
    }

    #[test]
    fn region_follows_hidden_and_edit_state() {
        // Hidden wins over everything: no region at all.
        assert!(region_for_state(true, false, 1920, 1080, 64).is_none());
        assert!(region_for_state(true, true, 1920, 1080, 64).is_none());

        // Edit mode takes the whole surface.
        let full = region_for_state(false, true, 1920, 1080, 64).unwrap();
        assert_eq!((full.x, full.y, full.w, full.h), (0, 0, 1920, 1080));

        // Pass-through keeps only the scaled corner, anchored top-right.
        let corner = region_for_state(false, false, 1920, 1080, 128).unwrap();
        assert_eq!(
            (corner.x, corner.y, corner.w, corner.h),
            (1792, 0, 128, 128)
        );
    }

    /// A second monitor takes clicks exactly when the primary takes them
    /// everywhere: edit mode, or a drag in progress — never while hidden,
    /// and never in plain pass-through, where it must not swallow clicks
    /// meant for the desktop.
    #[test]
    fn extras_take_input_only_when_the_primary_is_fully_open() {
        assert!(extras_take_input(false, true, false));
        assert!(extras_take_input(false, false, true));
        assert!(!extras_take_input(false, false, false));
        assert!(!extras_take_input(true, true, false));
        assert!(!extras_take_input(true, false, true));
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
