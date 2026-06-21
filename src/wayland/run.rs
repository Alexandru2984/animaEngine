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
//!
//! ## What doesn't (yet)
//!
//! - **Right-click context menu** — not wired on this path; the
//!   settings panel, palette, and library cover entity management.
//! - **Per-monitor placement** — the layer surface attaches to
//!   whichever output the compositor picks; no `PerMonitor`-style
//!   multi-surface support yet.
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
        renderer.primary.config.format,
        config.global.theme,
    );
    let mut selection = SelectionState::new();
    let mut toasts = ToastQueue::default();
    let mut config_dirty = false;
    let warnings: BTreeSet<Warning> = BTreeSet::new();
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
            if new_w != renderer.primary.window_width || new_h != renderer.primary.window_height {
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
                renderer.primary.window_width as f32 / 2.0,
                renderer.primary.window_height as f32 / 2.0,
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
                match vis {
                    AnimaEvent::HideOverlay => {
                        if let Err(e) =
                            layer.set_input_region(Some(InputRect::toggle_button_corner(
                                renderer.primary.window_width,
                                TOGGLE_BUTTON_SIZE,
                            )))
                        {
                            tracing::warn!("dbus hide: {e}");
                        }
                    }
                    AnimaEvent::ShowOverlay => {
                        // No-op on a layer surface (always present).
                    }
                    _ => {}
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

        // Tick the simulation. screen_w / screen_h match the surface so
        // walk-around behaviors stay inside the visible area.
        scene.set_reduced_motion(config.global.reduced_motion);
        scene.tick(
            renderer.primary.window_width as f32,
            renderer.primary.window_height as f32,
            None, // no cursor on this path until egui is wired in
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
        // Refresh the monitor snapshot every frame so the inspector's
        // picker shows a hot-plug straight away.
        let monitors = layer.monitors();
        let selected_id = selection
            .selected_index()
            .and_then(|idx| scene.entities.get(idx).map(|e| e.id.clone()));
        let visible = scene.visible_entities();
        toasts.prune();
        egui_renderer.ensure_theme(config.global.theme);
        match renderer.render(
            &visible,
            &scene.groups,
            layer.state.edit_mode,
            selected_id.as_deref(),
            (0.0, 0.0),
        ) {
            Ok(output) => {
                let view = output
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
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
                // Snapshot the AccessKit flag BEFORE taking its mutable
                // borrow, same trick as the X11 path uses.
                let accesskit_snapshot = config.global.accesskit_enabled;
                let mut toggle_requested = false;
                let mut palette_outcome: Option<panels::PaletteOutcome> = None;
                let mut library_outcome: Option<panels::LibraryOutcome> = None;
                let mut shimeji_import: Option<String> = None;
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
                                *palette_ref = panels::command_palette(ctx);
                                panels::toasts(ctx, toasts_ref);
                            }
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
                                    if scene.entities.iter().any(|e| e.id == cfg.id) {
                                        cfg.id = format!("{}-{}", cfg.id, scene.entities.len());
                                    }
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
                            renderer.primary.window_width as f32 / 2.0,
                            renderer.primary.window_height as f32 / 2.0,
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
