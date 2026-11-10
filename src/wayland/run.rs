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
use crate::hotkeys::probe::HotkeyStatus;
use crate::input::selection::SelectionState;
use crate::keybindings::{Action, KeyChord};
use crate::monitor::{self, MonitorInfo, MonitorMode, WindowPlan};
use crate::outcomes::{self, OutcomeCtx, ShimejiImport};
use crate::pacing::FRAME_INTERVAL;
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
    scene: Scene,
    config: AppConfig,
    // Outside commands: the D-Bus activation service and the tray both send here.
    command_rx: Option<mpsc::Receiver<AnimaEvent>>,
    portal_rx: Option<mpsc::Receiver<crate::hotkeys::portal::PortalMsg>>,
) -> Result<()> {
    Native::new(scene, config, command_rx, portal_rx)?.run()
}

/// What a panel outcome may touch (`crate::outcomes`), from the `&mut`
/// bindings a phase of [`Native::iteration`] holds of those fields —
/// borrowed per call, so the phase keeps using them in between.
macro_rules! outcome_ctx {
    ($scene:ident, $selection:ident, $toasts:ident, $config_dirty:ident, $renderer:ident) => {
        OutcomeCtx {
            scene: &mut *$scene,
            selection: &mut *$selection,
            toasts: &mut *$toasts,
            config_dirty: &mut *$config_dirty,
            renderer: Some(&mut *$renderer),
        }
    };
}

/// The monitors as a turn found them ([`Native::outputs`]).
struct Outputs {
    /// Every output, as the compositor lists it.
    monitors_now: Vec<MonitorInfo>,
    /// The overlays they get (`MonitorMode`).
    plan: WindowPlan,
    /// The primary's monitor not known yet: nothing drawn for a moment.
    awaiting_primary: bool,
    /// Where the primary surface's origin sits in the scene's space.
    primary_origin: (f32, f32),
    /// The pointer over the primary, in the scene's space.
    cursor_global: Option<(f32, f32)>,
}

/// When the toasts change next, while the overlay is `shown`: one raised
/// after the last egui pass has asked egui for no frame.
fn toasts_at(
    toasts: &ToastQueue,
    config: &AppConfig,
    shown: bool,
    now: Instant,
) -> Option<Instant> {
    shown
        .then(|| toasts.next_change(now, config.global.reduced_motion))
        .flatten()
}

/// The turn's input for the overlays' eguis ([`Native::draw`]).
struct EguiInput {
    /// The primary's events: keys, pointer, a paste.
    events: Vec<egui::Event>,
    /// The modifiers held.
    modifiers: egui::Modifiers,
    /// The events for the right-click menu on another monitor's overlay.
    extra_menu_events: Vec<egui::Event>,
}

/// What one turn of the loop tells it ([`Native::iteration`]).
enum Flow {
    /// Round again.
    Next,
    /// Out of the loop: a quit, or the overlay gone for good.
    Exit,
}

/// The native loop's state, kept from one turn to the next ([`run_native`]).
///
/// The fields are in the reverse of the order they are made in, so they
/// drop as the locals they were did: every wgpu surface before the
/// `layer` whose wl_surface it draws into, on an error as on a quit.
struct Native {
    quit_requested: bool,
    reopens: Reopens,
    loop_started: Instant,
    wait: Duration,
    waker: Option<crate::wayland::waker::Waker>,
    last_monitor_mode: MonitorMode,
    primary_frame: Option<crate::app::windows::LastFrame>,
    extra_render_errors: HashMap<String, crate::renderer::ErrorOnce>,
    render_errors: crate::renderer::ErrorOnce,
    extra_frames: HashMap<String, crate::app::windows::LastFrame>,
    extra_eguis: HashMap<String, crate::ui::surface_egui::SurfaceEgui>,
    extra_surfaces: HashMap<String, SurfaceState>,
    library_scan: Option<mpsc::Receiver<crate::asset_library::LibraryIndex>>,
    audio_host: crate::audio::AudioHost,
    script_host: crate::scripting::ScriptHost,
    drag_widened: bool,
    scene_schedule: crate::schedule::Schedule,
    reminder_timers: crate::reminders::Timers,
    held_for_away: bool,
    on_battery: bool,
    aside: crate::fullscreen::StepAside,
    history: crate::undo::UndoHistory,
    frame_gate: FrameGate,
    palette_actions: Vec<Action>,
    pending_paste: Option<(mpsc::Receiver<crate::system_clipboard::Content>, (f32, f32))>,
    pending_text_paste: Option<mpsc::Receiver<crate::system_clipboard::Content>>,
    tap_gesture: crate::tap::TapGesture,
    tap_refreshed: Instant,
    moods: crate::moods::Moods,
    was_hidden: bool,
    welcome: crate::welcome::Welcome,
    hours_clock: crate::hours::Clock,
    bedtime_watch: crate::bedtime::Watch,
    pending_transfer: Option<crate::outcomes::SceneTransfer>,
    pending_file_chooser: Option<crate::outcomes::FileChooserAdd>,
    pending_shimeji: Option<ShimejiImport>,
    library_root: Option<std::path::PathBuf>,
    library: Option<crate::asset_library::LibraryIndex>,
    context_menu_state: Option<crate::app::ContextMenuState>,
    warnings: BTreeSet<Warning>,
    soak: Option<crate::soak::SoakRecorder>,
    perf_overlay_visible: bool,
    perf_sampler: crate::perf::PerfSampler,
    config_watch: crate::config_watch::ConfigWatcher,
    config_dirty: bool,
    toasts: ToastQueue,
    was_editing: bool,
    marquee: Option<crate::input::multi::Marquee>,
    drag: crate::input::drag::DragController,
    selection: SelectionState,
    egui_renderer: WaylandEguiRenderer,
    renderer: WgpuRenderer,
    layer: LayerWindow,
    hotkey_backend_status: HotkeyStatus,
    overlay_hidden: bool,
    portal_rx: Option<mpsc::Receiver<crate::hotkeys::portal::PortalMsg>>,
    command_rx: Option<mpsc::Receiver<AnimaEvent>>,
    config: AppConfig,
    scene: Scene,
}

impl Native {
    /// Everything up to the first turn: the overlay and its renderer, the
    /// library's scan, the other monitors' overlays, the textures.
    fn new(
        mut scene: Scene,
        config: AppConfig,
        // Outside commands: the D-Bus activation service and the tray both send here.
        command_rx: Option<mpsc::Receiver<AnimaEvent>>,
        portal_rx: Option<mpsc::Receiver<crate::hotkeys::portal::PortalMsg>>,
    ) -> Result<Self> {
        // Tracks the parity of portal HideOverlay toggles — the portal
        // delivers one *action*, the Hide/Show intent derives from state.
        let overlay_hidden = false;
        // Keybindings-tab backend status (T.4), updated by PortalMsg.
        let hotkey_backend_status = if portal_rx.is_some() {
            HotkeyStatus::PortalAwaiting
        } else {
            HotkeyStatus::CompositorBindings
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

        // The renderer builds its surface from the layer, for each instance
        // it tries. The wl_surface backing it stays alive inside
        // `layer.state.layer` for the rest of this scope; ordering guarantees
        // `renderer` is dropped before `layer` (Rust drops locals in reverse
        // declaration order).
        let mut renderer = WgpuRenderer::with_surface_from(
            &|instance| layer.wgpu_surface(instance),
            width,
            height,
        )?;
        // This loop paces itself (`pacing::FrameRate`), and behind the windows a
        // covered surface must not stall it waiting to present.
        renderer.prefer_mailbox();
        // The pointer is this client's to know over its own input region
        // only: in pass-through, nowhere.
        crate::ui::panels::cursor_seen_in_edit_mode_only();
        let egui_renderer = WaylandEguiRenderer::new(
            &renderer.shared.device,
            renderer.shared.surface_format,
            config.global.theme,
        );
        let selection = SelectionState::new();
        let drag = crate::input::drag::DragController::new();
        // A selection rectangle being dragged over empty space.
        let marquee: Option<crate::input::multi::Marquee> = None;
        // Edit mode as of the last iteration, to notice it ending.
        let was_editing = false;
        let toasts = ToastQueue::default();
        let config_dirty = false;
        let config_watch = crate::config_watch::ConfigWatcher::new();
        // Frame timings for the perf overlay (R27). This loop had no sampler
        // at all, so `TogglePerfOverlay` was one of five rebindable actions
        // that did nothing here — and the README's parity table called the
        // overlay stable on both backends.
        let perf_sampler = crate::perf::PerfSampler::default();
        let perf_overlay_visible = false;
        // Soak metrics (W.1). Previously wired only into the winit render
        // loop, so `ANIMA_SOAK_METRICS` was silently a no-op here: the
        // memory-regression harness covered one of the two backends and gave
        // no indication it was skipping the other.
        let soak = crate::soak::SoakRecorder::from_env();
        let mut warnings: BTreeSet<Warning> = BTreeSet::new();
        if crate::config::loaded_defaults_over_unreadable_config() {
            warnings.insert(Warning::ConfigUnreadable);
        }
        if config.set_aside.unread() {
            warnings.insert(Warning::ConfigPartlyUnread);
        }
        // Right-click context menu state, mirroring `app::ContextMenuState`
        // on the X11 path. Persists across frames while the menu is open.
        let context_menu_state: Option<crate::app::ContextMenuState> = None;
        tracing::info!("Native Wayland renderer initialized ({width}×{height})");

        // Discover + load + merge-scan the asset library — same sequence
        // as the X11 path's `handle_resumed` (src/app/lifecycle.rs).
        // Errors are logged but never fatal — an empty library is fine.
        let mut library: Option<crate::asset_library::LibraryIndex> = None;
        let mut library_root: Option<std::path::PathBuf> = None;
        // A Shimeji pack import running off the UI thread, if any.
        let pending_shimeji: Option<ShimejiImport> = None;
        // The desktop's file chooser, open after "Add file…" until it answers.
        let pending_file_chooser: Option<crate::outcomes::FileChooserAdd> = None;
        // A scene being shared or imported (`crate::scene_file`).
        let pending_transfer: Option<crate::outcomes::SceneTransfer> = None;
        // Bedtime against the clock (`crate::bedtime`).
        let bedtime_watch = crate::bedtime::Watch::default();
        let hours_clock = crate::hours::Clock::default();
        let welcome = crate::welcome::Welcome::default();
        let was_hidden = false;
        let moods = crate::moods::Moods::default();
        // Tapping in pass-through (`crate::tap`): when the input region last
        // followed the characters, and a press waiting for its release.
        let tap_refreshed = Instant::now();
        let tap_gesture = crate::tap::TapGesture::default();
        // Text a text field asked for with Ctrl+V, on its way.
        let pending_text_paste: Option<
            std::sync::mpsc::Receiver<crate::system_clipboard::Content>,
        > = None;
        // A paste waiting on the clipboard's answer, and where it lands.
        let pending_paste: Option<(
            std::sync::mpsc::Receiver<crate::system_clipboard::Content>,
            (f32, f32),
        )> = None;
        // Actions picked in the command palette, run next frame through the
        // same `match` as the ones bound to keys.
        let palette_actions: Vec<Action> = Vec::new();
        // Whether an iteration draws (`FrameGate`).
        let frame_gate = FrameGate::default();
        // Edits to the scene, for undo and redo (`crate::undo`).
        let history = crate::undo::UndoHistory::default();
        // Stepping aside while another window is full screen
        // (`crate::fullscreen`) — apart from the user's own hide and pause.
        let aside = crate::fullscreen::StepAside::default();
        // Running on battery, as the away watch last said, and whether the
        // scene is held still for someone being away (`crate::away`).
        let on_battery = false;
        let held_for_away = false;
        // When each reminder is next due (`crate::reminders`).
        let reminder_timers = crate::reminders::Timers::default();
        // Scenes by time of day: what the clock last showed (`crate::schedule`).
        let scene_schedule = crate::schedule::Schedule::default();
        // Whether the input region is currently widened for a drag
        // (`drag_region`).
        let drag_widened = false;
        // Rhai host for `Behavior::Script`, mirroring the winit path's
        // `App::script_host`. Scripts resolve against `library_root`.
        let script_host = crate::scripting::ScriptHost::new();
        let audio_host = crate::audio::AudioHost::new();
        // The scanned index arrives from its own thread (`asset_library::open`).
        let mut library_scan: Option<
            std::sync::mpsc::Receiver<crate::asset_library::LibraryIndex>,
        > = None;
        if let Some(root) = crate::asset_library::ensure_asset_root() {
            library_scan = Some(crate::asset_library::open(&root));
            library = Some(crate::asset_library::LibraryIndex::default());
            crate::sounds::set_root(&root);
            library_root = Some(root);
        } else {
            tracing::info!("No asset library root found; Library tab will show empty state.");
        }

        // Start in pass-through mode with the ⚙ button cutout — same default
        // as the X11 path.
        layer.set_input_region(Some(InputRect::toggle_button_corner(
            width,
            shown_button(&config.global),
        )))?;

        // Sprite-only extra surfaces for `MonitorMode::PerMonitor`, one
        // per non-primary output — the Wayland counterpart of the X11
        // path's `App.extra_windows`. Keyed by monitor name so it can be
        // diffed against `monitor::plan_windows`'s output on every
        // topology / mode change. Exercised against a real headless sway
        // session including output hotplug (see docs/wayland.md and the
        // module doc on `layer_window::mod`).
        let mut extra_surfaces: HashMap<String, SurfaceState> = HashMap::new();
        // Each other monitor's egui, for the speech bubbles of the characters
        // there (`ui::surface_egui`), made the first time one is needed.
        let extra_eguis: HashMap<String, crate::ui::surface_egui::SurfaceEgui> = HashMap::new();
        // Each extra surface's last frame: one that would look the same is not
        // drawn (`app::windows::needs_frame`).
        let extra_frames: HashMap<String, crate::app::windows::LastFrame> = HashMap::new();
        // Render errors said once each (`renderer::ErrorOnce`), the primary's
        // and each other monitor's apart, so two failing in turn don't take
        // turns at the log.
        let render_errors = crate::renderer::ErrorOnce::default();
        let extra_render_errors: HashMap<String, crate::renderer::ErrorOnce> = HashMap::new();
        // And the primary's, outside edit mode.
        let primary_frame: Option<crate::app::windows::LastFrame> = None;
        let last_monitor_mode = config.global.monitor_mode.clone();
        // Not on a guess of which monitor the primary is on: the loop makes
        // them once the compositor says (`awaiting_primary_output`).
        {
            let initial_monitors = layer.monitors();
            if !awaiting_primary_output(
                &config.global.monitor_mode,
                initial_monitors.len(),
                layer.state.primary_output_name.is_some(),
                std::time::Duration::ZERO,
            ) {
                let plan = monitor::plan_windows(&config.global.monitor_mode, &initial_monitors);
                rebuild_extra_surfaces(&mut layer, &renderer, &plan, &mut extra_surfaces);
            }
        }

        // Upload textures once for the initial scene.
        for entity in &scene.entities {
            renderer.ensure_texture(entity);
        }
        for entity in &mut scene.entities {
            entity.texture_dirty = false;
        }

        // ── Main loop ──
        // Waits for compositor events, or for another thread to wake it
        // (`waker`), at most until the next thing it has to do — a frame
        // interval while anything moves, else a sprite's next picture, egui's
        // repaint or the heartbeat (`pacing::next_wait`). See
        // `dispatch_with_timeout`.
        let waker = crate::wayland::waker::Waker::install();
        let wait = FRAME_INTERVAL;
        let loop_started = Instant::now();
        let reopens = Reopens::default();
        // Quit asked for: the tray, a signal, the shortcut (`Closing`).
        let quit_requested = false;
        Ok(Self {
            quit_requested,
            reopens,
            loop_started,
            wait,
            waker,
            last_monitor_mode,
            primary_frame,
            extra_render_errors,
            render_errors,
            extra_frames,
            extra_eguis,
            extra_surfaces,
            library_scan,
            audio_host,
            script_host,
            drag_widened,
            scene_schedule,
            reminder_timers,
            held_for_away,
            on_battery,
            aside,
            history,
            frame_gate,
            palette_actions,
            pending_paste,
            pending_text_paste,
            tap_gesture,
            tap_refreshed,
            moods,
            was_hidden,
            welcome,
            hours_clock,
            bedtime_watch,
            pending_transfer,
            pending_file_chooser,
            pending_shimeji,
            library_root,
            library,
            context_menu_state,
            warnings,
            soak,
            perf_overlay_visible,
            perf_sampler,
            config_watch,
            config_dirty,
            toasts,
            was_editing,
            marquee,
            drag,
            selection,
            egui_renderer,
            renderer,
            layer,
            hotkey_backend_status,
            overlay_hidden,
            portal_rx,
            command_rx,
            config,
            scene,
        })
    }

    /// The loop, and the save on the way out.
    fn run(mut self) -> Result<()> {
        while let Flow::Next = self.iteration()? {}

        // Renderer (and every extra surface's wgpu::Surface) is dropped
        // here before `layer` — each wgpu surface releases its handle
        // while the wl_surface it points into is still alive. `extra_surfaces`
        // holds the same kind of raw-handle surface the primary one does
        // (E.7), so it needs the identical ordering, not just `renderer`.
        // Persist any unsaved edits on a clean shutdown — Quit from the tray,
        // or a logout, Ctrl+C or closed terminal, which `quit_signal` turns
        // into that Quit — so the last toggle is not lost.
        if self.config_dirty {
            if let Err(e) = sync_and_save(&mut self.config, &self.scene) {
                tracing::warn!("Final config save failed: {e}");
            }
        }

        drop(self.renderer);
        drop(self.extra_surfaces);
        drop(self.layer);
        Ok(())
    }

    /// One turn of the loop: the compositor's events, then everything
    /// that follows from them and from the time, then a frame if there
    /// is a reason for one.
    fn iteration(&mut self) -> Result<Flow> {
        let frame_start = Instant::now();
        let mut activity = self.wake()?;
        // Capture-and-reset the previous frame's GPU op counters.
        let (gpu_uploads, gpu_draws) = self.renderer.shared.take_frame_gpu_counters();
        if let Some(flow) = self.overlay_closed(&mut activity) {
            return Ok(flow);
        }
        self.resized()?;
        self.results(&mut activity);
        let outputs = self.outputs(&mut activity);
        self.circumstances(&mut activity);
        self.input_regions(&outputs.monitors_now, outputs.primary_origin);
        self.drops(outputs.primary_origin, &mut activity);
        self.commands(&mut activity);
        let (events, modifiers) = self.keys(
            &outputs.plan,
            &outputs.monitors_now,
            outputs.primary_origin,
            &mut activity,
        );
        let extra_menu_events = self.pointer(
            &events,
            &outputs.plan,
            &outputs.monitors_now,
            outputs.primary_origin,
            &mut activity,
        );
        self.tick(&outputs.plan, outputs.cursor_global, &mut activity);
        // Draw only for a reason: it drew an unchanged scene sixty times a
        // second, 84% of a core in the rig with everything paused, where
        // the winit loop sleeps (`crate::pacing`).
        self.toasts.prune();
        activity |= self.egui_renderer.screen_reader_woke();
        let now = Instant::now();
        self.history.settle(
            &self.scene,
            self.egui_renderer.input_in_progress()
                || self.pending_file_chooser.is_some()
                || self.pending_shimeji.is_some(),
            now,
        );
        // Out of sight, the scene asks for no frames. As it was here, before
        // the frame, for the rest of the turn.
        let shown = !(self.overlay_hidden || self.aside.hidden);
        if !self.should_draw(now, shown, activity) {
            self.perf_sampler.end_frame();
            self.wait = self.next_wait(shown, outputs.awaiting_primary);
            return Ok(Flow::Next);
        }
        self.draw(
            &outputs,
            EguiInput {
                events,
                modifiers,
                extra_menu_events,
            },
            (gpu_uploads, gpu_draws),
            now,
            shown,
        )?;
        // Closed before the pacing sleep, so the overlay reports the work
        // the frame actually did rather than a flat ~16 ms for every
        // frame regardless of load — which would make the number useless
        // for the one thing it exists to answer.
        self.perf_sampler.end_frame();

        // Soft cap at ~60 Hz, or ~30 as set (`FrameRate`). The dispatch
        // above returns immediately when events were already queued, so
        // pace on the frame's actual elapsed time rather than sleeping
        // unconditionally — otherwise a busy compositor would spin faster
        // than the cap and an idle one would wait out the poll timeout
        // *and* a full sleep on top.
        let frame =
            (self.config.global.frame_rate).interval(self.on_battery, self.layer.state.edit_mode);
        if let Some(rest) = frame.checked_sub(frame_start.elapsed()) {
            std::thread::sleep(rest);
        }
        self.wait = self.next_wait(shown, outputs.awaiting_primary);
        Ok(Flow::Next)
    }

    /// How the scene asks to be drawn (`crate::pacing`): while it is
    /// `shown`; out of sight, not at all.
    fn scene_pacing(&self, shown: bool) -> crate::pacing::RedrawPacing {
        if shown {
            crate::pacing::redraw_pacing(&self.scene, self.perf_overlay_visible)
        } else {
            crate::pacing::RedrawPacing::Idle
        }
    }

    /// Whether this turn draws ([`FrameGate`]): for `activity`, a moving
    /// scene, an egui that asked for a frame, here or on another monitor,
    /// or a toast due to change.
    fn should_draw(&mut self, now: Instant, shown: bool, activity: bool) -> bool {
        let moving = matches!(
            self.scene_pacing(shown),
            crate::pacing::RedrawPacing::Continuous
        );
        let extras_repaint = self.extra_eguis.values().any(|e| e.repaint_due(now));
        let reason = activity
            || moving
            || self.egui_renderer.repaint_due(now)
            || extras_repaint
            || toasts_at(&self.toasts, &self.config, shown, now).is_some_and(|at| at <= now);
        self.frame_gate.should_draw(now, reason)
    }

    /// How long the next dispatch may sleep, from the state as it is
    /// when asked — after this turn's work.
    fn next_wait(&self, shown: bool, awaiting_primary: bool) -> Duration {
        let now = Instant::now();
        let egui_at = std::iter::once(self.egui_renderer.repaint_at())
            .chain(self.extra_eguis.values().map(|e| e.repaint_at()))
            .chain(std::iter::once(toasts_at(
                &self.toasts,
                &self.config,
                shown,
                now,
            )))
            .flatten()
            .min();
        // Results the loop looks for, which wake nothing when they come.
        let awaiting = self.pending_shimeji.is_some()
            || self.pending_file_chooser.is_some()
            || self.pending_transfer.is_some()
            || self.pending_paste.is_some()
            || self.pending_text_paste.is_some()
            || self.library_scan.is_some()
            || self.config_watch.waiting()
            || awaiting_primary;
        // And a reminder or the focus timer's turn on time, not at the
        // next heartbeat.
        let at = crate::pacing::soonest(egui_at, self.reminder_timers.next_due(now));
        crate::pacing::next_wait(
            crate::pacing::sooner(self.scene_pacing(shown), at, now),
            (self.config.global.frame_rate).interval(self.on_battery, self.layer.state.edit_mode),
            self.frame_gate.last_draw,
            awaiting,
            now,
        )
    }

    /// Waits for the compositor — at most `wait`, or until the waker
    /// rings — and takes what it sent: whether any of it is something a
    /// frame should show.
    fn wake(&mut self) -> Result<bool> {
        let Self {
            wait,
            waker,
            history,
            perf_sampler,
            selection,
            egui_renderer,
            layer,
            scene,
            ..
        } = self;
        // Opens the perf frame before the blocking dispatch, so the wait
        // for the compositor lands in `Idle` rather than vanishing — the
        // same placement the winit loop uses relative to its own wait.
        perf_sampler.begin_frame();
        let dispatched = dispatch_with_timeout(
            &mut layer.event_queue,
            &mut layer.state,
            waker.as_ref(),
            *wait,
        )?;
        // Whether anything happened this iteration that a frame should
        // show. Input and every other compositor event count; so does
        // whatever the loop picks up from its channels below.
        let activity = dispatched > 0;
        // A step to undo opens at the first thing someone does, before it
        // is applied (`crate::undo`).
        if layer.has_user_action() || egui_renderer.has_screen_reader_requests() {
            history.input(scene, selection, Instant::now());
        }

        Ok(activity)
    }

    /// The overlay closed by the compositor, or a quit asked for
    /// ([`Closing`]): what the loop does next, if not carry on.
    fn overlay_closed(&mut self, activity: &mut bool) -> Option<Flow> {
        let Self {
            quit_requested,
            reopens,
            wait,
            primary_frame,
            drag_widened,
            aside,
            renderer,
            layer,
            overlay_hidden,
            config,
            ..
        } = self;
        // The compositor closed the overlay: its monitor went — unplugged,
        // or Hyprland's stand-in for none replaced by a real one, which
        // ended an app started from `exec-once`. The session goes on, so it
        // is made again once a monitor is there; only a compositor that
        // keeps closing it ends the app, as every close did before
        // (`Reopens`). A quit is not a close (`Closing`).
        match closing(
            *quit_requested,
            layer.state.close_requested,
            !layer.monitors().is_empty(),
            &mut *reopens,
            Instant::now(),
        ) {
            Closing::Open => {}
            Closing::Quit => return Some(Flow::Exit),
            Closing::WaitForMonitor => {
                *wait = Duration::from_millis(500);
                return Some(Flow::Next);
            }
            Closing::GiveUp => {
                tracing::info!("Layer surface closed by compositor again and again — exiting.");
                return Some(Flow::Exit);
            }
            Closing::Reopen => match reopen_primary(&mut *layer, &mut *renderer) {
                Ok(()) => {
                    tracing::info!("Layer surface closed by compositor — made again");
                    let region = region_for_state(
                        *overlay_hidden || aside.hidden,
                        layer.state.edit_mode || *drag_widened,
                        renderer.primary.window_width,
                        renderer.primary.window_height,
                        shown_button(&config.global),
                    );
                    if let Err(e) = layer.set_input_region(region) {
                        tracing::warn!("input region after reopening: {e}");
                    }
                    *primary_frame = None;
                    *activity = true;
                }
                Err(e) => {
                    tracing::warn!(
                        "Layer surface closed by compositor, not made again ({e}) — exiting."
                    );
                    return Some(Flow::Exit);
                }
            },
        }

        None
    }

    /// A new size from the compositor: the buffers made again at it,
    /// and the input region from the live state.
    fn resized(&mut self) -> Result<()> {
        let Self {
            drag_widened,
            aside,
            renderer,
            layer,
            overlay_hidden,
            config,
            ..
        } = self;
        // Pick up any resize the compositor sent us.
        if let Some((new_w, new_h)) = layer.state.pending_size.take() {
            if new_w != renderer.primary.window_width || new_h != renderer.primary.window_height {
                renderer.resize(new_w, new_h);
                // Re-derive from the live state: this used to re-apply the
                // pass-through corner unconditionally, so a resize while
                // in edit mode silently reverted the surface to
                // click-through, and a resize while hidden made it
                // clickable again.
                let region = region_for_state(
                    *overlay_hidden || aside.hidden,
                    layer.state.edit_mode || *drag_widened,
                    new_w,
                    new_h,
                    shown_button(&config.global),
                );
                layer.set_input_region(region)?;
                tracing::info!("Layer surface resized to {new_w}×{new_h}");
            }
        }

        Ok(())
    }

    /// config.toml edited by hand (the hot reload), and the results the
    /// loop waits on: a Shimeji import, the file chooser's answer, a
    /// paste, a shared scene coming in or going out.
    fn results(&mut self, activity: &mut bool) {
        let Self {
            history,
            pending_paste,
            pending_transfer,
            pending_file_chooser,
            pending_shimeji,
            library_root,
            warnings,
            config_watch,
            config_dirty,
            toasts,
            marquee,
            drag,
            selection,
            renderer,
            layer,
            config,
            scene,
            ..
        } = self;
        // Hot-reload. The README lists it as a feature and its parity
        // table calls it stable on both backends; this loop never did it
        // at all, so editing config.toml with the overlay running on
        // Wayland did nothing (R28).
        let poll = config_watch.poll(*config_dirty);
        *activity |= !matches!(poll, crate::config_watch::Poll::Idle);
        let reloaded = matches!(poll, crate::config_watch::Poll::Ready(_));
        if reloaded {
            // The characters a drag or a rectangle holds are about to be
            // replaced: let go while they are still the ones it froze.
            crate::input::multi::cancel(&mut *scene, &mut *selection, &mut *drag, &mut *marquee);
        }
        crate::config_watch::handle(
            poll,
            &mut outcome_ctx!(scene, selection, toasts, config_dirty, renderer),
            &mut *config,
            &mut *warnings,
        );
        if reloaded {
            // The scene was replaced from the file: the steps no longer
            // describe it.
            history.clear();
        }
        if let Some(import) = &pending_shimeji {
            if import
                .poll(&mut outcome_ctx!(
                    scene,
                    selection,
                    toasts,
                    config_dirty,
                    renderer
                ))
                .is_some()
            {
                *pending_shimeji = None;
                *activity = true;
            }
        }
        if let Some(chooser) = &pending_file_chooser {
            if let Some(added) = chooser.poll(&mut outcome_ctx!(
                scene,
                selection,
                toasts,
                config_dirty,
                renderer
            )) {
                *pending_file_chooser = None;
                *activity = true;
                if added > 0 {
                    *config_dirty = true;
                }
            }
        }
        if let Some((answer, at)) = &pending_paste {
            let content = match answer.try_recv() {
                Ok(content) => Some(content),
                Err(std::sync::mpsc::TryRecvError::Empty) => None,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    Some(crate::system_clipboard::Content::Nothing)
                }
            };
            if let Some(content) = content {
                let at = *at;
                *pending_paste = None;
                *activity = true;
                // The key that asked may be a while ago: open a step if it
                // closed.
                history.input(scene, selection, Instant::now());
                let pasted = outcomes::apply_pasted(
                    content,
                    at,
                    &mut outcome_ctx!(scene, selection, toasts, config_dirty, renderer),
                );
                for path in pasted.to_import {
                    // A shared scene or a Shimeji pack copied in a file
                    // manager goes where a drop of it would.
                    if crate::scene_file::is_scene_file(&path) {
                        if pending_transfer.is_none() {
                            *pending_transfer =
                                Some(crate::outcomes::SceneTransfer::import(path, Some(at)));
                        }
                    } else if pending_shimeji.is_none() {
                        *pending_shimeji =
                            ShimejiImport::start(&path, library_root.as_deref(), at, &mut *toasts);
                    }
                }
            }
        }
        if let Some(transfer) = &mut *pending_transfer {
            match transfer.poll(&mut *toasts) {
                crate::outcomes::Transfer::Pending => {}
                crate::outcomes::Transfer::Done => {
                    *pending_transfer = None;
                    *activity = true;
                }
                crate::outcomes::Transfer::Imported(saved) => {
                    *pending_transfer = None;
                    *activity = true;
                    // Not input, so no undo step is open: open one, as
                    // "Next scene" does; and let go of any gesture.
                    history.input(scene, selection, Instant::now());
                    crate::input::multi::cancel(
                        &mut *scene,
                        &mut *selection,
                        &mut *drag,
                        &mut *marquee,
                    );
                    let bounds = monitor::covered_bounds(
                        &monitor::plan_windows(&config.global.monitor_mode, &layer.monitors()),
                        (
                            renderer.primary.window_width as f32,
                            renderer.primary.window_height as f32,
                        ),
                    );
                    outcomes::apply_imported(
                        saved,
                        &mut outcome_ctx!(scene, selection, toasts, config_dirty, renderer),
                        &mut config.global.active_scene,
                        bounds,
                    );
                }
            }
        }
    }

    /// The monitors as they are now: the other monitors' overlays made
    /// or closed to match them, every surface at its output's scale,
    /// and where the primary sits among them ([`Outputs`]).
    fn outputs(&mut self, activity: &mut bool) -> Outputs {
        let Self {
            loop_started,
            last_monitor_mode,
            extra_frames,
            extra_surfaces,
            renderer,
            layer,
            config,
            scene,
            ..
        } = self;
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
        // Which monitor the primary landed on is known once the compositor
        // says it entered one — after its first frame. Until then, for a
        // moment at most, no other monitor gets its overlay and nothing is
        // drawn: the plan guessed the first monitor listed, made an
        // overlay for the primary's own monitor when the guess was wrong,
        // and closed it again some 40 ms later.
        let awaiting_primary = awaiting_primary_output(
            &config.global.monitor_mode,
            monitors_now.len(),
            layer.state.primary_output_name.is_some(),
            (*loop_started).elapsed(),
        );
        if !awaiting_primary
            && (config.global.monitor_mode != *last_monitor_mode || plan_names != current_names)
        {
            rebuild_extra_surfaces(&mut *layer, renderer, &plan, &mut *extra_surfaces);
            // New surfaces, with nothing on them yet.
            extra_frames.clear();
            *last_monitor_mode = config.global.monitor_mode.clone();
        }
        for (name, new_w, new_h) in layer.drain_extra_resizes() {
            if let Some(surface) = extra_surfaces.get_mut(&name) {
                surface.resize(&renderer.shared, new_w, new_h);
            }
        }
        // Each surface at its output's scale (`layer_window::scale`),
        // as the compositor last said: its buffer made again at it.
        let primary_scale = layer.state.primary_scale.scale();
        if renderer.primary.pixel_scale() != primary_scale {
            renderer
                .primary
                .set_pixel_scale(&renderer.shared, primary_scale);
            tracing::info!("Overlay drawn at scale {primary_scale}");
            *activity = true;
        }
        for (name, surface) in extra_surfaces.iter_mut() {
            let scale = layer.extra_scale(name);
            if surface.pixel_scale() != scale {
                surface.set_pixel_scale(&renderer.shared, scale);
                tracing::info!("Overlay on {name} drawn at scale {scale}");
                *activity = true;
            }
        }
        // The pictures as sharp as the sharpest of them shows them
        // (`Scene::sharpen`): the scene was loaded before any said.
        let detail = (extra_surfaces.keys())
            .map(|name| layer.extra_scale(name))
            .fold(primary_scale, f32::max);
        crate::entity::set_detail_scale(detail);
        if scene.sharpen(crate::entity::picture_cap()) {
            *activity = true;
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

        Outputs {
            monitors_now,
            plan,
            awaiting_primary,
            primary_origin,
            cursor_global,
        }
    }

    /// What the desktop and the time ask of the overlay: in front of the
    /// windows or behind; stepping aside for a full-screen window, an app
    /// chosen in front, a snooze; held still for someone away, the battery
    /// or bedtime; the scene by the time of day; reminders and the focus
    /// timer; the pace by the hour; a greeting for someone back.
    fn circumstances(&mut self, activity: &mut bool) {
        let Self {
            library_scan,
            audio_host,
            drag_widened,
            scene_schedule,
            reminder_timers,
            held_for_away,
            on_battery,
            aside,
            history,
            moods,
            welcome,
            hours_clock,
            bedtime_watch,
            library,
            config_dirty,
            toasts,
            marquee,
            drag,
            selection,
            renderer,
            layer,
            overlay_hidden,
            config,
            scene,
            ..
        } = self;
        // Drain pointer + keyboard events. Until egui paint lands
        // (E.4) we don't have a UI consumer, but we already need to
        // detect the `Action::ToggleEditMode` chord so click-through
        // can flip in lock-step. Scan key-press events, match against
        // the user's bindings, dispatch the few actions that make
        // sense without a UI thread (just edit mode for now).
        // Step aside while another window is full screen, and come back
        // when it is not. Recomputed every iteration from the setting, the
        // window in front and edit mode, so nothing has to be undone.
        // In front of the windows or behind them; edit mode always in
        // front, so the panel is in reach.
        layer.set_on_top(config.global.always_on_top || layer.state.edit_mode);
        let chosen_in_front = layer
            .state
            .toplevels
            .chosen_in_front(&config.global.step_aside_apps);
        let wanted = crate::fullscreen::step_aside(
            config.global.on_fullscreen,
            layer.fullscreen_in_front(),
            layer.state.edit_mode,
        )
        // An app the user chose in front (`crate::chosen_apps`).
        .or_chosen_app(chosen_in_front, layer.state.edit_mode)
        // Snoozing (`crate::snooze`): out of sight and still until it ends.
        .or_snoozing(crate::snooze::active());
        // The open apps, for the picker, while it shows.
        if crate::chosen_apps::open_list_wanted() {
            crate::chosen_apps::set_open(layer.state.toplevels.open_apps());
        }
        // In the Flatpak, the Background portal's answer to "start at
        // login", once it comes (`crate::autostart`).
        if let Some(granted) = crate::autostart::take_portal_answer() {
            config.global.start_at_login = granted;
            *config_dirty = true;
        }
        // Held still while nobody is there, too (`crate::away`): idle from
        // the compositor, the battery from the away watch.
        let global = &config.global;
        // Watched for reminders even with pausing off.
        let idle_watch = crate::away::idle_watch_minutes(
            global.pause_when_idle_minutes,
            !config.reminders.is_empty() || global.welcome_back,
        );
        layer.set_idle_timeout(idle_watch);
        crate::away::configure(
            idle_watch,
            global.pause_on_battery || global.frame_rate.watches_battery(),
        );
        // Bedtime by the clock (`crate::bedtime`) — not while editing.
        let bedtime_now = !layer.state.edit_mode && bedtime_watch.now(&global.bedtime);
        // The library's fresh scan, once it is done (`asset_library::open`).
        if let Some(rx) = &library_scan {
            match rx.try_recv() {
                Ok(index) => {
                    *library = Some(index);
                    *library_scan = None;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => *library_scan = None,
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }
        // How loud the sounds are, if at all (`crate::audio`), before the
        // tick: scripts play from inside it.
        audio_host.set_level(
            global.sound_volume,
            global.sounds_muted,
            global.quiet_at_bedtime && bedtime_now,
        );
        // Characters with hours of their own (`crate::hours`) — everyone
        // while editing.
        let minute = if layer.state.edit_mode {
            None
        } else {
            hours_clock.minute()
        };
        if scene.set_clock(minute) {
            *activity = true;
        }
        let still = crate::away::stillness(
            global.pause_when_idle_minutes,
            layer.is_idle(),
            global.pause_on_battery,
            *on_battery,
        )
        .with_bedtime(bedtime_now)
        .watching(layer.fullscreen_in_front());
        if still.any() != *held_for_away {
            *held_for_away = still.any();
            *activity = true;
            tracing::info!(
                "Away: scene {}",
                if still.doze {
                    "dozing off"
                } else if still.freeze {
                    "held still"
                } else {
                    "back"
                }
            );
        }
        *activity |= scene.set_dozing(still.doze);
        scene.set_suspended(wanted.paused || still.freeze);
        // Scenes by time of day (`crate::schedule`), as on the winit path:
        // not in edit mode, the undo step opened here, gestures let go.
        if !layer.state.edit_mode {
            if let Some(name) = scene_schedule.due_now(&config.scene_schedule) {
                history.input(scene, selection, Instant::now());
                crate::input::multi::cancel(
                    &mut *scene,
                    &mut *selection,
                    &mut *drag,
                    &mut *marquee,
                );
                if crate::outcomes::apply_scheduled(
                    &name,
                    &mut outcome_ctx!(scene, selection, toasts, config_dirty, renderer),
                    &mut config.global.active_scene,
                ) {
                    *activity = true;
                }
            }
        }
        // Reminders due now (`crate::reminders`), said by any character:
        // every monitor draws its bubbles. Before the frame gate: a still
        // scene would never reach the drawing code.
        // While the focus timer runs (`crate::focus`), reminders wait out
        // the work, and its break counts as a break.
        let focus = crate::focus::phase();
        if crate::reminders::deliver(
            &mut *reminder_timers,
            &config.reminders,
            &config.global.reminder_sound,
            config.global.notify_reminders,
            &mut *scene,
            !(*overlay_hidden || wanted.hidden),
            // The night counts as a break too.
            crate::reminders::Away {
                on_a_break: layer.is_idle()
                    || bedtime_now
                    || focus == Some(crate::focus::Phase::Break),
                idle: layer.is_idle(),
                working: focus == Some(crate::focus::Phase::Work),
            },
            |_| true,
        ) {
            *activity = true;
        }
        // Each turn of the timer, said once it can be seen.
        if crate::focus::say_news(
            &mut *scene,
            !(*overlay_hidden || wanted.hidden),
            &config.global.reminder_sound,
            config.global.notify_reminders,
            |_| true,
        ) {
            *activity = true;
        }
        // The pace by the time of day, and a yawn before bedtime
        // (`crate::moods`).
        moods.step(
            &mut *scene,
            config.global.moods,
            &config.global.bedtime,
            !(*overlay_hidden || wanted.hidden),
        );
        // Back after being away (`crate::welcome`): greeted where it can be
        // seen, and not by sleepers.
        let minute = hours_clock.minute();
        if let Some(greeting) = welcome.check(layer.is_idle(), minute).filter(|_| {
            config.global.welcome_back && !(*overlay_hidden || wanted.hidden) && !bedtime_now
        }) {
            if let Some(i) = scene.frontmost_visible() {
                scene.entities[i].say(&greeting.line(), crate::welcome::SAY_SECONDS);
                *activity = true;
            }
        }
        if wanted != *aside {
            *aside = wanted;
            *activity = true;
            tracing::info!(
                "{}: overlay {}",
                if crate::snooze::active() {
                    "Snoozing"
                } else if chosen_in_front && !layer.state.edit_mode {
                    "An app chosen in Appearance in front"
                } else if layer.fullscreen_in_front() {
                    "Full-screen app in front"
                } else {
                    "Full-screen app gone"
                },
                if aside.hidden {
                    "hidden"
                } else if aside.paused {
                    "paused"
                } else {
                    "back"
                }
            );
            let region = region_for_state(
                *overlay_hidden || aside.hidden,
                layer.state.edit_mode || *drag_widened,
                renderer.primary.window_width,
                renderer.primary.window_height,
                shown_button(&config.global),
            );
            if let Err(e) = layer.set_input_region(region) {
                tracing::warn!("stepping aside: {e}");
            }
        }
    }

    /// Where the overlays take the pointer: wider while a drag is over
    /// the primary in pass-through, the characters' rectangles when they
    /// take taps, and the other monitors' overlays following the primary.
    fn input_regions(&mut self, monitors_now: &[MonitorInfo], primary_origin: (f32, f32)) {
        let Self {
            extra_surfaces,
            drag_widened,
            aside,
            tap_refreshed,
            egui_renderer,
            renderer,
            layer,
            overlay_hidden,
            config,
            scene,
            ..
        } = self;
        // Widen the input region while a drag is over us in pass-through,
        // and put it back when the drag ends.
        let widen = drag_region(
            *overlay_hidden || aside.hidden,
            layer.state.edit_mode,
            layer.state.drag_over,
        );
        if widen != *drag_widened {
            *drag_widened = widen;
            let region = region_for_state(
                *overlay_hidden || aside.hidden,
                layer.state.edit_mode || widen,
                renderer.primary.window_width,
                renderer.primary.window_height,
                shown_button(&config.global),
            );
            if let Err(e) = layer.set_input_region(region) {
                tracing::warn!("drag region: {e}");
            }
        }

        // Tapping (`crate::tap`): in pass-through, with the setting on,
        // the characters take clicks too — their rectangles join the ⚙
        // corner, a few times a second at most — and a click on one must
        // not take the keyboard. A click on the ⚙ still does, so edit mode
        // has the keys from its first moment: the pointer is over it before
        // the press.
        // Not over a full-screen app either, even one they only pause
        // for: a game's clicks are the game's.
        let tapping = config.global.tap_characters
            && !layer.state.edit_mode
            && !(*overlay_hidden || aside.hidden || aside.paused)
            && !widen;
        layer.set_keyboard_wanted(
            layer.state.edit_mode || !config.global.tap_characters || egui_renderer.owns_pointer(),
        );
        let tap_refresh = tapping && (*tap_refreshed).elapsed() >= crate::tap::REFRESH;
        if tap_refresh {
            *tap_refreshed = Instant::now();
            let base = region_for_state(
                false,
                false,
                renderer.primary.window_width,
                renderer.primary.window_height,
                shown_button(&config.global),
            );
            let rects = crate::tap::rects(
                scene,
                primary_origin,
                (
                    renderer.primary.window_width,
                    renderer.primary.window_height,
                ),
            );
            layer.sync_tap_region(base, rects);
        } else if !tapping && layer.tap_region_applied() {
            // Tapping just went off: the plain region again.
            let region = region_for_state(
                *overlay_hidden || aside.hidden,
                layer.state.edit_mode || widen,
                renderer.primary.window_width,
                renderer.primary.window_height,
                shown_button(&config.global),
            );
            if let Err(e) = layer.set_input_region(region) {
                tracing::warn!("tap region: {e}");
            }
        }

        // The per-monitor extra surfaces follow the primary: open to
        // input in edit mode, or while a drag is over the overlay, and
        // click-through otherwise — but for the characters, when tapping.
        // Their offsets into the primary's space come from the same output
        // snapshot as `primary_origin`, so a click, drag or drop on
        // another monitor lands in the right global place.
        layer.sync_extra_layers(
            extras_take_input(
                *overlay_hidden || aside.hidden,
                layer.state.edit_mode,
                widen,
            ),
            |name| {
                monitors_now
                    .iter()
                    .find(|m| m.name == name)
                    .map(|m| (m.x as f32 - primary_origin.0, m.y as f32 - primary_origin.1))
            },
            |name| {
                if !tapping {
                    return Some(Vec::new());
                }
                if !tap_refresh {
                    return None;
                }
                let mon = monitors_now.iter().find(|m| m.name == name)?;
                let surface = extra_surfaces.get(name)?;
                Some(crate::tap::rects(
                    scene,
                    (mon.x as f32, mon.y as f32),
                    (surface.window_width, surface.window_height),
                ))
            },
        );
    }

    /// Files dropped on an overlay: a Shimeji pack to the importer, a
    /// shared scene to its import, anything else added where it landed.
    fn drops(&mut self, primary_origin: (f32, f32), activity: &mut bool) {
        let Self {
            history,
            pending_transfer,
            pending_shimeji,
            library_root,
            config_watch,
            config_dirty,
            toasts,
            selection,
            renderer,
            layer,
            config,
            scene,
            ..
        } = self;
        // Files dropped on the overlay (E.3). Each drop carries the point
        // it landed on in the primary surface's space (shifted there from
        // an extra surface's); `primary_origin` turns that into the global
        // space the scene uses. Same as the winit path,
        // through the same code: a Shimeji pack folder goes to the
        // importer, anything else is validated, added, selected, toasted
        // and marked for saving.
        for dropped in layer.drain_dropped_files() {
            *activity = true;
            history.input(scene, selection, Instant::now());
            let at = (
                dropped.at.0 + primary_origin.0,
                dropped.at.1 + primary_origin.1,
            );
            let mut added = false;
            for path in &dropped.paths {
                if outcomes::is_shimeji_pack(path) {
                    if pending_shimeji.is_none() {
                        *pending_shimeji =
                            ShimejiImport::start(path, library_root.as_deref(), at, &mut *toasts);
                    }
                } else if crate::scene_file::is_scene_file(path) {
                    // A shared scene replaces this one.
                    if pending_transfer.is_none() {
                        *pending_transfer = Some(crate::outcomes::SceneTransfer::import(
                            path.clone(),
                            Some(at),
                        ));
                    }
                } else if outcomes::add_dropped_file(
                    path,
                    at,
                    &mut outcome_ctx!(scene, selection, toasts, config_dirty, renderer),
                )
                .is_some()
                {
                    added = true;
                }
            }
            // As on winit, a drop opens edit mode so the new character can
            // be moved into place straight away. It matters more here: in
            // pass-through the overlay only takes input — drops included —
            // on the ⚙ corner, so that is where a drop lands.
            if added && !layer.state.edit_mode {
                flip_edit_mode(
                    &mut *layer,
                    &mut *config,
                    scene,
                    &mut *config_dirty,
                    &mut *config_watch,
                    "Wayland, file drop",
                );
            }
        }
    }

    /// What came from outside: the D-Bus methods and the tray, a signal,
    /// a second start, the portal's global shortcuts.
    fn commands(&mut self, activity: &mut bool) {
        let Self {
            quit_requested,
            drag_widened,
            on_battery,
            aside,
            history,
            config_watch,
            config_dirty,
            toasts,
            marquee,
            drag,
            selection,
            egui_renderer,
            renderer,
            layer,
            hotkey_backend_status,
            overlay_hidden,
            portal_rx,
            command_rx,
            config,
            scene,
            ..
        } = self;
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
            let mut open_edit = false;
            let mut toggle_playback_xor = false;
            let mut last_visibility: Option<AnimaEvent> = None;
            let mut quit = false;
            let mut next_scene = false;
            let mut snoozing = false;
            let mut mute_changed = false;
            if let Some(rx) = &command_rx {
                while let Ok(ev) = rx.try_recv() {
                    *activity = true;
                    // A global shortcut: the recorder's while one is being
                    // recorded, else the event it stands for now.
                    let ev = match ev {
                        AnimaEvent::Shortcut(action, chord) => {
                            if egui_renderer.take_for_recording(chord) {
                                continue;
                            }
                            let visible = std::sync::atomic::AtomicBool::new(!*overlay_hidden);
                            match crate::hotkeys::action_to_event(action, &visible) {
                                Some(ev) => ev,
                                None => continue,
                            }
                        }
                        other => other,
                    };
                    match ev {
                        AnimaEvent::Shortcut(..) => {}
                        AnimaEvent::ToggleEditMode => toggle_edit_xor ^= true,
                        AnimaEvent::ToggleGlobalPlayback => toggle_playback_xor ^= true,
                        AnimaEvent::HideOverlay => last_visibility = Some(AnimaEvent::HideOverlay),
                        AnimaEvent::ShowOverlay => last_visibility = Some(AnimaEvent::ShowOverlay),
                        AnimaEvent::Quit => {
                            quit = true;
                            tracing::info!("Quit requested (tray, or a signal)");
                        }
                        // A second start from the menu: a layer surface has
                        // no focus to take, but one hidden from the tray (or
                        // snoozed) comes back, as on X11 — before, nothing
                        // happened and the start looked broken.
                        AnimaEvent::RaiseWindow => {
                            last_visibility = Some(AnimaEvent::ShowOverlay);
                            open_edit |= !config.global.show_toggle_button;
                            tracing::info!("Raised by second-instance handshake");
                        }
                        // Hotkey resolution events are winit-path UI; the
                        // native path logs the outcome where it resolves.
                        AnimaEvent::HotkeysUnavailable
                        | AnimaEvent::PortalShortcutsDenied
                        | AnimaEvent::PortalShortcutsActive => {}
                        // This loop learns it from the compositor's
                        // toplevel list instead.
                        AnimaEvent::FullscreenInFront(_) | AnimaEvent::ChosenAppInFront(_) => {}
                        AnimaEvent::OnBattery(now) => *on_battery = now,
                        // Idle comes from the compositor here.
                        AnimaEvent::Away(_) | AnimaEvent::IdleSource(_) => {}
                        AnimaEvent::NextScene => next_scene = true,
                        AnimaEvent::Snooze(what) => {
                            crate::snooze::snooze(what);
                            snoozing = true;
                        }
                        AnimaEvent::Wake => crate::snooze::wake(),
                        AnimaEvent::StartFocus(minutes) => {
                            crate::focus::start(crate::focus::lengths_for(
                                minutes,
                                config.global.focus,
                            ));
                            *activity = true;
                        }
                        AnimaEvent::StopFocus => crate::focus::stop(),
                        AnimaEvent::SetMuted(muted) => {
                            config.global.sounds_muted = muted;
                            *config_dirty = true;
                            mute_changed = true;
                        }
                        AnimaEvent::ToggleMute => {
                            config.global.sounds_muted = !config.global.sounds_muted;
                            *config_dirty = true;
                            mute_changed = true;
                        }
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
                    *activity = true;
                    match msg {
                        PortalMsg::Ready => {
                            tracing::info!("Portal shortcuts active (native path)");
                            *hotkey_backend_status = HotkeyStatus::Portal;
                        }
                        PortalMsg::Failed => {
                            tracing::warn!(
                                "Portal shortcuts unavailable — compositor \
                                 bindings via D-Bus remain the fallback"
                            );
                            toasts.warn(crate::i18n::t("portal-denied-native-toast"));
                            *hotkey_backend_status = HotkeyStatus::CompositorBindings;
                        }
                        // The desktop owns the keys: which chord, it does
                        // not say, so a recording takes nothing — but the
                        // action waits while one is under way.
                        PortalMsg::Activated(_) if egui_renderer.take_for_recording(None) => {}
                        PortalMsg::Activated(action) => match action {
                            KbAction::ToggleFocus | KbAction::ToggleSnooze => {
                                snoozing |= timer_shortcut(action, config.global.focus);
                            }
                            KbAction::ToggleMute => {
                                config.global.sounds_muted = !config.global.sounds_muted;
                                *config_dirty = true;
                                mute_changed = true;
                            }
                            KbAction::ToggleEditMode => toggle_edit_xor ^= true,
                            KbAction::PauseAll => toggle_playback_xor ^= true,
                            KbAction::HideOverlay => {
                                *overlay_hidden = !*overlay_hidden;
                                last_visibility = Some(if *overlay_hidden {
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
            // Snoozing out of edit mode: the panel goes with the rest.
            toggle_edit_xor ^= snoozing && layer.state.edit_mode;
            // Started again with the ⚙ button hidden: edit mode, the way
            // back every desktop has, with a tray and shortcuts or without.
            if open_edit {
                toggle_edit_xor = !layer.state.edit_mode;
            }
            // Muted or unmuted from the tray or a global shortcut: saved at
            // once outside edit mode. Nothing else would save it before the
            // app quits, and a config left dirty ignores the file's own
            // edits until then.
            if mute_changed && !layer.state.edit_mode {
                match sync_and_save(&mut *config, scene) {
                    Ok(()) => {
                        *config_dirty = false;
                        config_watch.note_saved();
                    }
                    Err(e) => tracing::warn!("Config save failed: {e}"),
                }
            }
            if toggle_edit_xor {
                // Through `flip_edit_mode`, like the keyboard and the ⚙
                // button. This path used to set the mode directly, so
                // leaving edit mode from the tray, a D-Bus call or a portal
                // shortcut skipped the save — R30 on a third entry point.
                flip_edit_mode(
                    &mut *layer,
                    &mut *config,
                    scene,
                    &mut *config_dirty,
                    &mut *config_watch,
                    "Wayland, tray / D-Bus / portal",
                );
            }
            if toggle_playback_xor {
                scene.toggle_global_playback();
                *config_dirty = true;
                // Said, as on the winit path: from the tray, a shortcut or
                // D-Bus there is nothing else to show it happened.
                toasts.info(crate::i18n::t(if scene.global_playing {
                    "toast-playback-resumed"
                } else {
                    "toast-playback-paused"
                }));
            }
            if let Some(vis) = last_visibility {
                // `overlay_hidden` is the single source of truth and it
                // gates rendering below. It used to be updated only by the
                // keybinding path, so a D-Bus Hide/Show left the flag and
                // the actual intent disagreeing.
                match vis {
                    AnimaEvent::HideOverlay => *overlay_hidden = true,
                    AnimaEvent::ShowOverlay => {
                        *overlay_hidden = false;
                        // That ends a snooze too (`crate::snooze`).
                        crate::snooze::wake();
                    }
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
                    *overlay_hidden || aside.hidden,
                    layer.state.edit_mode || *drag_widened,
                    renderer.primary.window_width,
                    renderer.primary.window_height,
                    shown_button(&config.global),
                );
                if let Err(e) = layer.set_input_region(region) {
                    tracing::warn!("visibility change: {e}");
                }
            }
            if next_scene {
                // Not input, so no undo step is open: open one, or a later
                // undo would take the switch back with the step before it.
                history.input(scene, selection, Instant::now());
                // A drag or a rectangle holds characters about to be
                // replaced: let go first (R56).
                crate::input::multi::cancel(
                    &mut *scene,
                    &mut *selection,
                    &mut *drag,
                    &mut *marquee,
                );
                crate::outcomes::next_scene(
                    &mut outcome_ctx!(scene, selection, toasts, config_dirty, renderer),
                    &mut config.global.active_scene,
                );
            }
            if quit {
                *quit_requested = true;
            }
        }
    }

    /// The keyboard: text pasted into a text field, then the actions
    /// of this turn's keys and of last turn's palette picks, through one
    /// `match`. Gives back the turn's egui events and modifiers.
    fn keys(
        &mut self,
        plan: &WindowPlan,
        monitors_now: &[MonitorInfo],
        primary_origin: (f32, f32),
        activity: &mut bool,
    ) -> (Vec<egui::Event>, egui::Modifiers) {
        let Self {
            quit_requested,
            drag_widened,
            history,
            palette_actions,
            pending_paste,
            pending_text_paste,
            pending_transfer,
            pending_shimeji,
            library_root,
            context_menu_state,
            perf_overlay_visible,
            config_watch,
            config_dirty,
            toasts,
            marquee,
            drag,
            selection,
            egui_renderer,
            renderer,
            layer,
            overlay_hidden,
            config,
            scene,
            ..
        } = self;
        let mut events = layer.drain_egui_events();
        // Text on its way from the clipboard into a text field, and Ctrl+V
        // in one now (`crate::system_clipboard`).
        if let Some(answer) = &pending_text_paste {
            match answer.try_recv() {
                Ok(content) => {
                    *pending_text_paste = None;
                    if let crate::system_clipboard::Content::Text(text) = content {
                        events.push(egui::Event::Paste(text));
                        *activity = true;
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => *pending_text_paste = None,
            }
        }
        if std::mem::take(&mut layer.state.text_paste_requested)
            && egui_renderer.wants_keyboard()
            && pending_text_paste.is_none()
        {
            match layer.read_clipboard(crate::system_clipboard::Want::Text) {
                crate::wayland::layer_window::ClipboardRead::Ready(
                    crate::system_clipboard::Content::Text(text),
                ) => events.push(egui::Event::Paste(text)),
                crate::wayland::layer_window::ClipboardRead::Ready(_) => {}
                crate::wayland::layer_window::ClipboardRead::Pending(answer) => {
                    *pending_text_paste = Some(answer);
                }
            }
        }
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
        let mut actions = std::mem::take(palette_actions);
        *activity |= !actions.is_empty();
        actions.extend(
            events
                .iter()
                .filter_map(|event| binding_chord(event, egui_owns_keyboard))
                .filter_map(|chord| config.keybindings.action_for_press(chord)),
        );
        // The Menu key, the keyboard's own key for the right-click menu:
        // not a binding, egui having no name for it to record or match.
        if std::mem::take(&mut layer.state.menu_key_pressed) && !egui_owns_keyboard {
            actions.push(Action::OpenContextMenu);
        }
        for action in actions {
            if action.interrupts_drag() {
                *config_dirty |= crate::input::multi::cancel(
                    &mut *scene,
                    &mut *selection,
                    &mut *drag,
                    &mut *marquee,
                );
            }
            match action {
                Action::ToggleEditMode => {
                    flip_edit_mode(
                        &mut *layer,
                        &mut *config,
                        scene,
                        &mut *config_dirty,
                        &mut *config_watch,
                        "Wayland",
                    );
                }
                // In edit mode the keys are the overlay's, so the global
                // chord comes as a key press rather than through the portal
                // or a compositor binding — and was dropped here, while on
                // X11 the grab took it in edit mode too. Hidden as from the
                // tray: nothing drawn, no input taken.
                Action::HideOverlay => {
                    *overlay_hidden = true;
                    let region = region_for_state(
                        true,
                        layer.state.edit_mode || *drag_widened,
                        renderer.primary.window_width,
                        renderer.primary.window_height,
                        shown_button(&config.global),
                    );
                    if let Err(e) = layer.set_input_region(region) {
                        tracing::warn!("hide: {e}");
                    }
                }
                // As the global shortcuts do; snoozing leaves edit mode.
                Action::ToggleMute => {
                    config.global.sounds_muted = !config.global.sounds_muted;
                    *config_dirty = true;
                }
                // The right-click menu, from the keyboard: on the panel's
                // surface, where the keys and a screen reader are, nearest
                // the character selected, its first item focused
                // (`input::multi::keyboard_menu_at`).
                Action::OpenContextMenu => {
                    if let Some(idx) = selection.selected_index() {
                        if let Some(e) = scene.entities.get(idx) {
                            let at = egui::pos2(
                                e.x + e.scaled_width() / 2.0,
                                e.y + e.scaled_height() / 2.0,
                            );
                            crate::input::multi::select_for_menu(scene, &mut *selection, idx);
                            *context_menu_state = Some(crate::app::ContextMenuState {
                                entity_idx: idx,
                                pos: crate::input::multi::keyboard_menu_at(
                                    at - egui::vec2(primary_origin.0, primary_origin.1),
                                    egui::vec2(
                                        renderer.primary.window_width as f32,
                                        renderer.primary.window_height as f32,
                                    ),
                                ),
                                surface: None,
                                armed: false,
                                keyboard: true,
                            });
                            *activity = true;
                        }
                    }
                }
                Action::ToggleFocus | Action::ToggleSnooze => {
                    if timer_shortcut(action, config.global.focus) && layer.state.edit_mode {
                        flip_edit_mode(
                            &mut *layer,
                            &mut *config,
                            scene,
                            &mut *config_dirty,
                            &mut *config_watch,
                            "Wayland",
                        );
                    }
                }
                // The next four are not shareable — each one reaches for
                // something only this loop owns (its config, its shutdown
                // flag, its renderer's texture cache) — but every piece
                // they need is already here, and the in-app help screen
                // lists all four. They did nothing at all on this backend
                // until now (R27).
                Action::SaveNow => match sync_and_save(&mut *config, scene) {
                    Ok(()) => {
                        *config_dirty = false;
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
                        args.set("error", e.said());
                        toasts.error(crate::i18n::t_args("toast-save-failed", &args));
                    }
                },
                Action::TogglePerfOverlay => {
                    *perf_overlay_visible = !*perf_overlay_visible;
                    tracing::debug!(
                        "Perf overlay {}",
                        if *perf_overlay_visible {
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
                    *quit_requested = true;
                }
                Action::DeleteSelected | Action::DuplicateSelected | Action::CutSelected => {
                    // The same functions the right-click menu and the winit
                    // path use (`crate::outcomes`), so none of the three can
                    // drift from the others. Every selected character.
                    let targets = selection.selected_indices();
                    let mut ctx = outcome_ctx!(scene, selection, toasts, config_dirty, renderer);
                    match action {
                        Action::DeleteSelected => {
                            outcomes::delete_entities(&targets, &mut ctx);
                        }
                        Action::CutSelected => {
                            outcomes::cut_entities(&targets, &mut ctx);
                        }
                        _ => {
                            outcomes::duplicate_entities(&targets, &mut ctx);
                        }
                    }
                }
                // The desktop's clipboard first: an image or files copied
                // elsewhere since the last copy here come in as characters
                // (`crate::system_clipboard`), at the pointer.
                Action::Paste => {
                    let at = layer
                        .state
                        .cursor_pos
                        .map(|(x, y)| (x + primary_origin.0, y + primary_origin.1))
                        .unwrap_or((
                            renderer.primary.window_width as f32 / 2.0 + primary_origin.0,
                            renderer.primary.window_height as f32 / 2.0 + primary_origin.1,
                        ));
                    match layer.read_clipboard(crate::system_clipboard::Want::Characters) {
                        crate::wayland::layer_window::ClipboardRead::Ready(content) => {
                            let pasted = outcomes::apply_pasted(
                                content,
                                at,
                                &mut outcome_ctx!(scene, selection, toasts, config_dirty, renderer),
                            );
                            for path in pasted.to_import {
                                // A shared scene or a Shimeji pack copied in a file
                                // manager goes where a drop of it would.
                                if crate::scene_file::is_scene_file(&path) {
                                    if pending_transfer.is_none() {
                                        *pending_transfer = Some(
                                            crate::outcomes::SceneTransfer::import(path, Some(at)),
                                        );
                                    }
                                } else if pending_shimeji.is_none() {
                                    *pending_shimeji = ShimejiImport::start(
                                        &path,
                                        library_root.as_deref(),
                                        at,
                                        &mut *toasts,
                                    );
                                }
                            }
                        }
                        crate::wayland::layer_window::ClipboardRead::Pending(answer) => {
                            pending_paste.get_or_insert((answer, at));
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
                        scene: &mut *scene,
                        selection: &mut *selection,
                        config_dirty: &mut *config_dirty,
                        shift_held: layer.state.last_modifiers.shift,
                        bounds: monitor::covered_bounds(
                            plan,
                            (
                                renderer.primary.window_width as f32,
                                renderer.primary.window_height as f32,
                            ),
                        ),
                        monitors: monitors_now,
                        toasts: &mut *toasts,
                        history: &mut *history,
                    };
                    crate::keybindings::shared::dispatch_shared(other, &mut ctx);
                }
            }
        }
        // A copy just made: its names on the desktop's clipboard, while the
        // press that made it is the latest (setting it needs its serial).
        if let Some(text) = crate::clipboard::take_claim() {
            layer.claim_clipboard(text, true);
        }

        (events, modifiers)
    }

    /// The pointer: the right-click menu, on whichever monitor it is;
    /// letting go of a drag when edit mode ends or the overlay hides; in
    /// edit mode, selecting, dragging, a rectangle over empty space; in
    /// pass-through, a tap. Gives back the events for the menu's monitor,
    /// for its overlay's egui.
    fn pointer(
        &mut self,
        events: &[egui::Event],
        plan: &WindowPlan,
        monitors_now: &[MonitorInfo],
        primary_origin: (f32, f32),
        activity: &mut bool,
    ) -> Vec<egui::Event> {
        let Self {
            extra_eguis,
            extra_surfaces,
            aside,
            history,
            tap_gesture,
            context_menu_state,
            config_dirty,
            was_editing,
            marquee,
            drag,
            selection,
            egui_renderer,
            renderer,
            layer,
            overlay_hidden,
            config,
            scene,
            ..
        } = self;
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
        // Edit mode just ended — by any of the four ways — so let go of
        // what the pointer held and drop the selection, as the winit path
        // does. The release of a drag in progress will not come: in
        // pass-through the surface no longer gets the pointer, and the
        // drag used to stay open until the next move in edit mode, which
        // made the character jump to the pointer.
        if *was_editing && !layer.state.edit_mode {
            *config_dirty |= crate::input::multi::cancel(
                &mut *scene,
                &mut *selection,
                &mut *drag,
                &mut *marquee,
            );
            selection.deselect();
            *context_menu_state = None;
        }
        *was_editing = layer.state.edit_mode;
        // Hidden, nothing is dragged on. Sway still delivers the release —
        // the button's implicit grab outlives the empty input region — but
        // on X11 an unmapped window gets none, and the character jumped to
        // the pointer at the first move once shown; let go here too rather
        // than count on every compositor.
        if *overlay_hidden || aside.hidden {
            *config_dirty |= crate::input::multi::cancel(
                &mut *scene,
                &mut *selection,
                &mut *drag,
                &mut *marquee,
            );
        }
        let egui_owns_pointer = egui_renderer.owns_pointer();
        // The right-click menu on another monitor's overlay (`surface`):
        // that overlay's egui hears the pointer while it is over that
        // monitor, and a press anywhere else closes the menu — the egui
        // there cannot count a click it never hears as outside. Escape
        // closes it too: the keys go to the panel's egui. Presses on the
        // menu stay out of the scene, as the panel's do.
        let mut extra_menu_events: Vec<egui::Event> = Vec::new();
        let menu_monitor = context_menu_state
            .as_ref()
            .and_then(|m| m.surface.as_ref())
            .filter(|name| extra_surfaces.contains_key(*name))
            .and_then(|name| monitors_now.iter().find(|m| &m.name == name))
            .cloned();
        // Its monitor went — unplugged, or the overlay down to one: nothing
        // could draw the menu, and it kept every key from the shortcuts.
        if menu_monitor.is_none()
            && context_menu_state
                .as_ref()
                .is_some_and(|m| m.surface.is_some())
        {
            *context_menu_state = None;
        }
        let global =
            |pos: &egui::Pos2| egui::pos2(pos.x + primary_origin.0, pos.y + primary_origin.1);
        if let Some(mon) = &menu_monitor {
            let origin = egui::vec2(mon.x as f32, mon.y as f32);
            for event in events {
                match event {
                    egui::Event::PointerMoved(pos) => {
                        let at = global(pos);
                        extra_menu_events.push(if mon.contains(at.x, at.y) {
                            egui::Event::PointerMoved(at - origin)
                        } else {
                            egui::Event::PointerGone
                        });
                    }
                    egui::Event::PointerButton {
                        pos,
                        button,
                        pressed,
                        modifiers,
                    } => {
                        let at = global(pos);
                        if mon.contains(at.x, at.y) {
                            extra_menu_events.push(egui::Event::PointerButton {
                                pos: at - origin,
                                button: *button,
                                pressed: *pressed,
                                modifiers: *modifiers,
                            });
                        } else if *pressed {
                            *context_menu_state = None;
                        }
                    }
                    egui::Event::PointerGone => extra_menu_events.push(egui::Event::PointerGone),
                    egui::Event::Key {
                        key: egui::Key::Escape,
                        pressed: true,
                        ..
                    } => *context_menu_state = None,
                    _ => {}
                }
            }
        }
        let extra_owns_pointer = menu_monitor
            .as_ref()
            .is_some_and(|m| extra_eguis.get(&m.name).is_some_and(|e| e.owns_pointer()));
        let owned_press = |pos: &egui::Pos2| {
            let at = global(pos);
            egui_owns_pointer
                || (extra_owns_pointer
                    && menu_monitor
                        .as_ref()
                        .is_some_and(|m| m.contains(at.x, at.y)))
        };
        if layer.state.edit_mode {
            for event in events {
                match event {
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Secondary,
                        pressed: true,
                        ..
                    } if !owned_press(pos) => {
                        let at = global(pos);
                        if let Some(idx) = scene.entity_at_point(at.x, at.y) {
                            // One of several selected keeps them all, so
                            // the menu acts on the whole selection;
                            // otherwise it takes the character with its
                            // group.
                            crate::input::multi::select_for_menu(scene, &mut *selection, idx);
                            // On the monitor clicked: another monitor's
                            // overlay draws it with its own egui, in its
                            // own points.
                            let elsewhere = monitors_now.iter().find(|m| {
                                extra_surfaces.contains_key(&m.name) && m.contains(at.x, at.y)
                            });
                            *context_menu_state = Some(crate::app::ContextMenuState {
                                entity_idx: idx,
                                pos: match elsewhere {
                                    Some(m) => at - egui::vec2(m.x as f32, m.y as f32),
                                    None => *pos,
                                },
                                surface: elsewhere.map(|m| m.name.clone()),
                                // Armed after the first showing — see ContextMenuState.
                                armed: false,
                                keyboard: false,
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
                        modifiers,
                        ..
                    } if !owned_press(pos) => {
                        let at = (pos.x + primary_origin.0, pos.y + primary_origin.1);
                        // Select (with its group, keeping the others selected,
                        // or toggled with Shift) and pick the selection up; on
                        // empty space, a selection rectangle
                        // (`crate::input::multi`).
                        match scene.entity_at_point(at.0, at.1) {
                            Some(idx) => {
                                crate::input::multi::press_on(
                                    &mut *scene,
                                    &mut *selection,
                                    &mut *drag,
                                    idx,
                                    at,
                                    modifiers.shift,
                                );
                            }
                            None => {
                                *marquee = Some(crate::input::multi::Marquee::begin(
                                    at,
                                    modifiers.shift,
                                    &mut *selection,
                                ));
                            }
                        }
                    }
                    egui::Event::PointerButton {
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        ..
                    } if marquee.is_some() => {
                        *marquee = None;
                    }
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers,
                        ..
                    } if drag.is_dragging() => {
                        let (gx, gy) = (pos.x + primary_origin.0, pos.y + primary_origin.1);
                        // A press/release that never moved is a *tap*, not a
                        // drag — poke the mascot instead of just dropping it.
                        let tapped = drag.was_tap(gx, gy, crate::constants::POKE_TAP_RADIUS);
                        if tapped {
                            // The hop that follows is play, not an edit.
                            history.finish(scene);
                        }
                        let poke_bounds = monitor::covered_bounds(
                            plan,
                            (
                                renderer.primary.window_width as f32,
                                renderer.primary.window_height as f32,
                            ),
                        );
                        if tapped {
                            if let Some(idx) = drag.dragging_entity() {
                                crate::tap::poke(&mut *scene, idx, gx, poke_bounds);
                            }
                        }
                        // Thrown: the flight is play, as a poke's hop is —
                        // the step closes where the character was let go.
                        crate::input::multi::end_drag(
                            &mut *scene,
                            &mut *selection,
                            &mut *drag,
                            tapped,
                            modifiers.shift,
                            |scene| {
                                history.finish(scene);
                            },
                        );
                        *config_dirty = true;
                    }
                    egui::Event::PointerMoved(pos) if marquee.is_some() => {
                        let at = (pos.x + primary_origin.0, pos.y + primary_origin.1);
                        if let Some(m) = marquee.as_mut() {
                            m.drag_to(at, scene, &mut *selection);
                        }
                    }
                    egui::Event::PointerMoved(pos) if drag.is_dragging() => {
                        let at = (pos.x + primary_origin.0, pos.y + primary_origin.1);
                        // Snapping, unless it is off or Alt is held.
                        let snap_to = (config.global.snap_while_dragging
                            && !layer.state.last_modifiers.alt)
                            .then(|| crate::input::arrange::monitor_rects(monitors_now));
                        crate::input::multi::drag_to(
                            &mut *scene,
                            selection,
                            &mut *drag,
                            at,
                            snap_to.as_deref(),
                        );
                    }
                    _ => {}
                }
            }
        } else if config.global.tap_characters && !(*overlay_hidden || aside.hidden || aside.paused)
        {
            // Pass-through with tapping on (`crate::tap`): a press and a
            // release close together on a character poke it. Presses on
            // the ⚙ button are egui's.
            for event in events {
                match event {
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        ..
                    } if !egui_owns_pointer => {
                        let at = global(pos);
                        tap_gesture.press(scene, at.x, at.y);
                    }
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        ..
                    } => {
                        let at = global(pos);
                        if let Some(idx) = tap_gesture.release(scene, at.x, at.y) {
                            let bounds = monitor::covered_bounds(
                                plan,
                                (
                                    renderer.primary.window_width as f32,
                                    renderer.primary.window_height as f32,
                                ),
                            );
                            crate::tap::poke(&mut *scene, idx, at.x, bounds);
                            *activity = true;
                        }
                    }
                    _ => {}
                }
            }
        }

        extra_menu_events
    }

    /// The scene moves on: the settings it follows, a tick of every
    /// character (scripts and sounds in it), the fades, the sounds it
    /// asked for, and the textures of the frames it moved to.
    fn tick(&mut self, plan: &WindowPlan, cursor_global: Option<(f32, f32)>, activity: &mut bool) {
        let Self {
            audio_host,
            script_host,
            aside,
            was_hidden,
            library_root,
            soak,
            perf_sampler,
            config_dirty,
            toasts,
            renderer,
            overlay_hidden,
            config,
            scene,
            ..
        } = self;
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
        scene.set_bump(config.global.characters_bump);
        scene.set_greet(config.global.characters_greet);
        scene.set_large_bubbles(config.global.large_bubbles);
        scene.set_little_effects(config.global.little_effects);
        {
            let _s = perf_sampler.scope(crate::perf::Category::SceneUpdate);
            scene.tick(
                crate::monitor::covered_bounds(
                    plan,
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
                        host: &mut *script_host,
                        audio: &mut *audio_host,
                        root,
                    }),
            );
        }
        // Back from hidden, everyone fades in; and the fades move on,
        // paused or not.
        let hidden_now = *overlay_hidden || aside.hidden;
        if *was_hidden && !hidden_now {
            scene.fade_all_in();
        }
        *was_hidden = hidden_now;
        scene.fade_step(Instant::now());
        // A character spoke up on its own (`crate::phrases`): show it.
        *activity |= scene.take_spoke();
        // A thrown one landed: save where.
        *config_dirty |= scene.take_landed();
        // Tap sounds (`crate::sounds`), panned across what the overlay covers.
        let sounds = scene.take_sounds();
        if let Some(root) = library_root.as_deref() {
            let bounds = crate::monitor::covered_bounds(
                plan,
                (
                    renderer.primary.window_width as f32,
                    renderer.primary.window_height as f32,
                ),
            );
            for (id, sound, x, volume) in sounds {
                audio_host.play(root, &sound, &id, x, bounds.min_x, bounds.max_x, volume);
            }
        }

        for (path, err) in audio_host.take_new_failures() {
            let mut args = fluent::FluentArgs::new();
            args.set("sound", path);
            args.set("error", err);
            toasts.error(crate::i18n::t_args("sound-failed-toast", &args));
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
                *activity |= !(*overlay_hidden || aside.hidden);
            }
        }
    }

    /// A frame: the primary's scene and egui — the panel, the palette,
    /// the menu, the toasts — and what they ask for, then every other
    /// monitor's overlay. Outside edit mode a surface draws only when it
    /// has something new to show (`app::windows::needs_frame`).
    fn draw(
        &mut self,
        outputs: &Outputs,
        input: EguiInput,
        gpu: (u32, u32),
        now: Instant,
        shown: bool,
    ) -> Result<()> {
        let Self {
            primary_frame,
            extra_render_errors,
            render_errors,
            extra_frames,
            extra_eguis,
            extra_surfaces,
            aside,
            palette_actions,
            pending_transfer,
            pending_file_chooser,
            pending_shimeji,
            library_root,
            library,
            context_menu_state,
            warnings,
            perf_overlay_visible,
            perf_sampler,
            config_watch,
            config_dirty,
            toasts,
            marquee,
            drag,
            selection,
            egui_renderer,
            renderer,
            layer,
            hotkey_backend_status,
            overlay_hidden,
            config,
            scene,
            ..
        } = self;
        let Outputs {
            monitors_now,
            plan,
            awaiting_primary,
            primary_origin,
            ..
        } = outputs;
        let (awaiting_primary, primary_origin) = (*awaiting_primary, *primary_origin);
        let EguiInput {
            events,
            modifiers,
            mut extra_menu_events,
        } = input;
        let (gpu_uploads, gpu_draws) = gpu;
        // Render the scene. Pass `selected_id` so the highlight ring
        // appears in edit mode for the entity the user clicked.
        // `monitors_now` (refreshed above) covers the inspector's
        // picker hot-plug needs too — no separate snapshot needed.
        let monitors = &monitors_now;
        let selected_owned = selection.selected_ids(scene);
        let selected_ids: Vec<&str> = selected_owned.iter().map(String::as_str).collect();
        let marks = crate::renderer::wgpu_renderer::EditMarks {
            selected: &selected_ids,
            marquee: marquee.as_ref().map(|m| m.rect()),
            guides: drag.guides(),
        };
        // Those fading out are drawn too (`Scene::fade_step`).
        let visible = scene.drawn_entities();
        // In PerMonitor mode the primary surface covers exactly its
        // own output: entities live in global coords (once extras
        // exist) and need filtering to just the primary's monitor,
        // same as the X11 path's `entity_on_monitor` gate (T.6/T.8).
        let primary_monitor_name: Option<String> = if !extra_surfaces.is_empty() {
            plan.primary.as_ref().map(|m| m.name.clone())
        } else {
            None
        };
        // Speech bubbles for the characters on the primary surface
        // (`crate::speech`), gathered while the scene is free to borrow.
        let bubbles = crate::speech::shown(scene, primary_origin, |e| {
            primary_monitor_name
                .as_ref()
                .is_none_or(|name| crate::app::windows::entity_on_monitor(monitors, e, name))
        });
        // And the hearts, dust and z's on it (`crate::effects`).
        let effects = if *overlay_hidden || aside.hidden {
            Vec::new()
        } else {
            scene.effects_shown(
                primary_origin,
                (
                    renderer.primary.window_width as f32,
                    renderer.primary.window_height as f32,
                ),
                now,
            )
        };
        let drawn: Vec<&Entity> = if *overlay_hidden || aside.hidden || awaiting_primary {
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
        // Outside edit mode the primary draws only when it has something
        // new to show (`app::windows::needs_frame`): a walker on another
        // monitor used to redraw it every frame. Busy — drawn every frame,
        // and once after — while its egui paints, has input or is asked
        // for anything. The other monitors and the outcomes below go on
        // either way.
        // The edit-mode bar is the overlay's too: hidden, it goes with the
        // rest — it stayed, a green line across the top of a hidden one.
        let edit_bar = layer.state.edit_mode && !(*overlay_hidden || aside.hidden);
        let primary_busy = layer.state.edit_mode
            || *perf_overlay_visible
            || !bubbles.is_empty()
            || !effects.is_empty()
            || !events.is_empty()
            || toasts_at(toasts, config, shown, now).is_some_and(|at| at <= now)
            || egui_renderer.repaint_due(now)
            || egui_renderer.has_screen_reader_requests();
        let primary_key = crate::app::windows::primary_key(renderer.primary.frame_key(
            &renderer.shared,
            &drawn,
            &scene.groups,
            edit_bar,
            marks,
            primary_origin,
        ));
        let primary =
            if crate::app::windows::needs_frame(*primary_frame, primary_key, primary_busy, now) {
                renderer
                    .render(&drawn, &scene.groups, edit_bar, marks, primary_origin)
                    .map(Some)
            } else {
                Ok(None)
            };
        match primary {
            Ok(output) => {
                if output.is_some() {
                    perf_sampler.add(crate::perf::Category::WgpuSubmit, submit_start.elapsed());
                    render_errors.drew();
                }
                let size = [
                    renderer.primary.window_width,
                    renderer.primary.window_height,
                ];
                // egui's points are the surface's units, the pointer's
                // and the input region's; its pixels are the buffer's, at
                // the surface's scale (`layer_window::scale`).
                let pixel_size = renderer.primary.pixel_size();
                let pixels_per_point = renderer.primary.pixel_scale();
                let edit_mode_snapshot = layer.state.edit_mode;
                let show_button_snapshot = config.global.show_toggle_button;
                let hidden_snapshot = *overlay_hidden || aside.hidden;
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
                let mut scene_request: Option<panels::SceneRequest> = None;
                // The menu on another monitor is drawn by that overlay's
                // egui, below.
                let menu_state = context_menu_state.clone().filter(|m| m.surface.is_none());
                let extra_menu = context_menu_state.clone().filter(|m| m.surface.is_some());
                let menu_offers = context_menu_state
                    .as_ref()
                    .map(|m| crate::outcomes::menu_offers(m.entity_idx, scene, selection))
                    .unwrap_or_default();
                // Disjoint mut borrows for the closure.
                let scene_mut = &mut *scene;
                let selection_mut = &mut *selection;
                let config_dirty_mut = &mut *config_dirty;
                let theme_mut = &mut config.global.theme;
                let locale_mut = &mut config.global.locale;
                let onboarding_mut = &mut config.global.onboarding;
                let monitor_mode_mut = &mut config.global.monitor_mode;
                let window_awareness_mut = &mut config.global.window_awareness;
                let snap_mut = &mut config.global.snap_while_dragging;
                let bump_mut = &mut config.global.characters_bump;
                let greet_mut = &mut config.global.characters_greet;
                let active_scene_mut = &mut config.global.active_scene;
                let reminders_mut = &mut config.reminders;
                let focus_mut = &mut config.global.focus;
                let reminder_sound_mut = &mut config.global.reminder_sound;
                let notify_mut = &mut config.global.notify_reminders;
                let scene_schedule_mut = &mut config.scene_schedule;
                let reduced_motion_mut = &mut config.global.reduced_motion;
                let large_bubbles_mut = &mut config.global.large_bubbles;
                let hover_startle_mut = &mut config.global.hover_startle;
                let on_fullscreen_mut = &mut config.global.on_fullscreen;
                let always_on_top_mut = &mut config.global.always_on_top;
                let step_aside_apps_mut = &mut config.global.step_aside_apps;
                let bedtime_mut = &mut config.global.bedtime;
                let moods_mut = &mut config.global.moods;
                let tap_characters_mut = &mut config.global.tap_characters;
                let show_toggle_button_mut = &mut config.global.show_toggle_button;
                let little_effects_mut = &mut config.global.little_effects;
                let idle_minutes_mut = &mut config.global.pause_when_idle_minutes;
                let on_battery_mut = &mut config.global.pause_on_battery;
                let frame_rate_mut = &mut config.global.frame_rate;
                let welcome_back_mut = &mut config.global.welcome_back;
                let sound_volume_mut = &mut config.global.sound_volume;
                let sounds_muted_mut = &mut config.global.sounds_muted;
                let quiet_at_bedtime_mut = &mut config.global.quiet_at_bedtime;
                let start_at_login_mut = &mut config.global.start_at_login;
                let idle_available = Some(layer.idle_available());
                let accesskit_mut = &mut config.global.accesskit_enabled;
                let keybindings_mut = &mut config.keybindings;
                let collapse_state_mut = &mut config.collapse_state;
                let last_seen_whats_new_mut = &mut config.global.last_seen_whats_new;
                let warnings_ref = &warnings;
                let hotkey_backend_ref = *hotkey_backend_status;
                let shimeji_import_ref = &mut shimeji_import;
                let scene_request_ref = &mut scene_request;
                let monitors_ref = monitors.as_slice();
                let toasts_ref = &toasts;
                let toggle_requested_ref = &mut toggle_requested;
                let palette_ref = &mut palette_outcome;
                let library_ref = &mut library_outcome;
                let menu_outcome_ref = &mut menu_outcome;
                let perf_sampler_ref = &perf_sampler;
                let perf_export_ref = &mut perf_export_request;
                let perf_visible_snapshot = *perf_overlay_visible;
                if let Some(output) = output {
                    let view = output.create_view();
                    let egui_start = Instant::now();
                    egui_renderer.set_accesskit_allowed(accesskit_snapshot);
                    egui_renderer.set_window_state(
                        layer.has_keyboard_focus(),
                        primary_origin,
                        size,
                    );
                    egui_renderer.render(
                        &renderer.shared.device,
                        &renderer.shared.queue,
                        &view,
                        pixel_size,
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
                            if (edit_mode_snapshot || show_button_snapshot)
                                && panels::toggle_button(ctx, edit_mode_snapshot)
                            {
                                *toggle_requested_ref = true;
                            }
                            // Positions here are already in points: pointer
                            // events map to them one to one.
                            crate::ui::effects::paint(ctx, &effects, 1.0);
                            crate::ui::speech::paint(ctx, &bubbles, 1.0);
                            // And to a screen reader (`crate::announce`), with
                            // the toasts.
                            crate::ui::speech::announce(ctx);
                            crate::ui::toasts::announce(ctx, toasts_ref);
                            if crate::ui::onboarding::coach_marks(
                                ctx,
                                onboarding_mut,
                                edit_mode_snapshot,
                                show_button_snapshot,
                                keybindings_mut,
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
                                    snap_mut,
                                    bump_mut,
                                    greet_mut,
                                    active_scene_mut,
                                    reminders_mut,
                                    scene_schedule_mut,
                                    focus_mut,
                                    reminder_sound_mut,
                                    notify_mut,
                                    // A layer surface belongs to one output, so no single
                                    // surface can span the desktop here.
                                    false,
                                    reduced_motion_mut,
                                    large_bubbles_mut,
                                    hover_startle_mut,
                                    on_fullscreen_mut,
                                    step_aside_apps_mut,
                                    always_on_top_mut,
                                    tap_characters_mut,
                                    show_toggle_button_mut,
                                    little_effects_mut,
                                    bedtime_mut,
                                    moods_mut,
                                    panels::AwayControls {
                                        idle_minutes: &mut *idle_minutes_mut,
                                        on_battery: &mut *on_battery_mut,
                                        frame_rate: &mut *frame_rate_mut,
                                        welcome_back: &mut *welcome_back_mut,
                                        idle_available,
                                    },
                                    panels::SoundControls {
                                        volume: &mut *sound_volume_mut,
                                        muted: &mut *sounds_muted_mut,
                                        quiet_at_bedtime: &mut *quiet_at_bedtime_mut,
                                    },
                                    start_at_login_mut,
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
                                    scene_request_ref,
                                );
                                if edit_mode_snapshot {
                                    if let Some(state) = &menu_state {
                                        *menu_outcome_ref =
                                            Some(panels::context_menu(ctx, state, menu_offers));
                                    }
                                    *palette_ref = panels::command_palette(
                                        ctx,
                                        keybindings_mut,
                                        selection_mut.selected_index().is_some(),
                                    );
                                }
                                // In both modes, as on the winit path.
                                panels::toasts(ctx, toasts_ref);
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
                    // Copied from a text field: onto the desktop's clipboard, as
                    // plain text.
                    if let Some(text) = egui_renderer.take_copied_text() {
                        layer.claim_clipboard(text, false);
                    }
                    if perf_export_request {
                        match crate::perf::export_snapshot(perf_sampler) {
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
                                toasts.error(crate::i18n::t_args(
                                    "toast-perf-snapshot-failed",
                                    &args,
                                ));
                            }
                        }
                    }
                    {
                        let _s = perf_sampler.scope(crate::perf::Category::Present);
                        renderer.present(output);
                    }
                    *primary_frame = Some(crate::app::windows::LastFrame {
                        key: primary_key,
                        overlay: primary_busy,
                        at: now,
                    });
                }
                // Extras: the entities pinned (or resolved by position) to
                // that monitor, translated by its own origin, and the speech
                // bubbles among them. Mirrors
                // `app::windows::render_extra_windows` on the X11 path.
                // Hidden, they clear like the primary: the X11 path unmaps
                // its extra windows, but these stay mapped, and they kept
                // showing the characters on the other monitors.
                if !extra_surfaces.is_empty() {
                    let extra_visible = if *overlay_hidden || aside.hidden {
                        Vec::new()
                    } else {
                        scene.drawn_entities()
                    };
                    for (name, surface) in extra_surfaces.iter_mut() {
                        let Some(mon) = monitors_now.iter().find(|m| &m.name == name) else {
                            continue;
                        };
                        let drawn: Vec<&Entity> = extra_visible
                            .iter()
                            .copied()
                            .filter(|e| {
                                crate::app::windows::entity_on_monitor(monitors_now, e, name)
                            })
                            .collect();
                        let origin = (mon.x as f32, mon.y as f32);
                        // Its own egui, run while there are bubbles here —
                        // and once more when one asked for the frame that
                        // clears it.
                        let bubbles = if *overlay_hidden || aside.hidden {
                            Vec::new()
                        } else {
                            crate::speech::shown(scene, origin, |e| {
                                crate::app::windows::entity_on_monitor(monitors_now, e, name)
                            })
                        };
                        let effects = if *overlay_hidden || aside.hidden {
                            Vec::new()
                        } else {
                            scene.effects_shown(
                                origin,
                                (surface.window_width as f32, surface.window_height as f32),
                                now,
                            )
                        };
                        let asked = extra_eguis.get(name).is_some_and(|e| e.repaint_due(now));
                        // Hidden means hidden, as on the panel's surface.
                        let menu_here = extra_menu.as_ref().filter(|m| {
                            m.surface.as_ref() == Some(name) && !(*overlay_hidden || aside.hidden)
                        });
                        let painted = !bubbles.is_empty()
                            || !effects.is_empty()
                            || asked
                            || menu_here.is_some();
                        // Not drawn when it would show what it shows
                        // already, as on X11 (`app::windows::needs_frame`):
                        // one monitor's walker no longer redraws the rest.
                        let key = surface.frame_key(
                            &renderer.shared,
                            &drawn,
                            &scene.groups,
                            edit_bar,
                            marks,
                            origin,
                        );
                        if !crate::app::windows::needs_frame(
                            extra_frames.get(name).copied(),
                            key,
                            painted,
                            now,
                        ) {
                            continue;
                        }
                        match surface.render(
                            &renderer.shared,
                            &drawn,
                            &scene.groups,
                            edit_bar,
                            marks,
                            origin,
                        ) {
                            Ok(extra_output) => {
                                if let Some(errors) = extra_render_errors.get_mut(name) {
                                    errors.drew();
                                }
                                if painted {
                                    let egui =
                                        extra_eguis.entry(name.clone()).or_insert_with(|| {
                                            crate::ui::surface_egui::SurfaceEgui::new(
                                                &renderer.shared.device,
                                                renderer.shared.surface_format,
                                                config.global.theme,
                                            )
                                        });
                                    egui.ensure_theme(config.global.theme);
                                    let view = extra_output.create_view();
                                    egui.render(
                                        &renderer.shared.device,
                                        &renderer.shared.queue,
                                        &view,
                                        surface.pixel_size(),
                                        surface.pixel_scale(),
                                        if menu_here.is_some() {
                                            std::mem::take(&mut extra_menu_events)
                                        } else {
                                            Vec::new()
                                        },
                                        egui::Modifiers::default(),
                                        |ctx| {
                                            // In points already, as on the primary.
                                            crate::ui::effects::paint(ctx, &effects, 1.0);
                                            crate::ui::speech::paint(ctx, &bubbles, 1.0);
                                            if let Some(state) = menu_here {
                                                menu_outcome = Some(panels::context_menu(
                                                    ctx,
                                                    state,
                                                    menu_offers,
                                                ));
                                            }
                                        },
                                    );
                                }
                                surface.present(&renderer.shared, extra_output);
                                extra_frames.insert(
                                    name.clone(),
                                    crate::app::windows::LastFrame {
                                        key,
                                        overlay: painted,
                                        at: now,
                                    },
                                );
                            }
                            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                                let (w, h) = (surface.window_width, surface.window_height);
                                surface.resize(&renderer.shared, w, h);
                                extra_frames.remove(name);
                            }
                            Err(wgpu::SurfaceError::Timeout) => {}
                            Err(e) => extra_render_errors
                                .entry(name.clone())
                                .or_default()
                                .failed(name, &e),
                        }
                    }
                }
                // Monitors gone, and their eguis and frames with them.
                extra_eguis.retain(|name, _| extra_surfaces.contains_key(name));
                extra_frames.retain(|name, _| extra_surfaces.contains_key(name));
                if toggle_requested {
                    flip_edit_mode(
                        &mut *layer,
                        &mut *config,
                        scene,
                        &mut *config_dirty,
                        &mut *config_watch,
                        "Wayland, toggle button",
                    );
                }
                // Palette / library outcomes apply outside the egui
                // closure where we can take &mut *renderer + &mut *toasts
                // without conflicting.
                match &scene_request {
                    // Pasted with the palette's picks, next frame.
                    Some(panels::SceneRequest::Paste) => palette_actions.push(Action::Paste),
                    Some(panels::SceneRequest::Share(path)) if pending_transfer.is_none() => {
                        *pending_transfer =
                            crate::outcomes::SceneTransfer::share(path, &mut *toasts);
                    }
                    Some(panels::SceneRequest::Import) if pending_transfer.is_none() => {
                        *pending_transfer = Some(crate::outcomes::SceneTransfer::import_chooser());
                    }
                    Some(panels::SceneRequest::BackUp) if pending_transfer.is_none() => {
                        *pending_transfer =
                            Some(crate::outcomes::SceneTransfer::back_up(config, scene));
                    }
                    Some(panels::SceneRequest::Restore) if pending_transfer.is_none() => {
                        *pending_transfer = Some(crate::outcomes::SceneTransfer::restore_chooser());
                    }
                    _ => {}
                }
                if scene_request == Some(panels::SceneRequest::AddFile)
                    && pending_file_chooser.is_none()
                {
                    *pending_file_chooser = Some(crate::outcomes::FileChooserAdd::start((
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
                        *pending_shimeji = ShimejiImport::start(
                            &crate::config::AppConfig::resolve_asset_path(&path),
                            library_root.as_deref(),
                            at,
                            &mut *toasts,
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
                            *context_menu_state = None;
                        }
                        // Asks where first, as sharing a scene does.
                        panels::ContextMenuOutcome::Action(panels::MenuAction::SaveGif(idx)) => {
                            if pending_transfer.is_none() {
                                *pending_transfer =
                                    crate::outcomes::SceneTransfer::save_gif(scene, idx);
                            }
                            *context_menu_state = None;
                        }
                        panels::ContextMenuOutcome::Action(panels::MenuAction::ShareCharacter(
                            idx,
                        )) => {
                            if pending_transfer.is_none() {
                                *pending_transfer =
                                    crate::outcomes::SceneTransfer::share_character(scene, idx);
                            }
                            *context_menu_state = None;
                        }
                        panels::ContextMenuOutcome::Action(action) => {
                            outcomes::apply_menu_action(
                                action,
                                &mut outcome_ctx!(scene, selection, toasts, config_dirty, renderer),
                            );
                            *context_menu_state = None;
                        }
                    }
                }
                match palette_outcome {
                    Some(panels::PaletteOutcome::RunAction(action)) => palette_actions.push(action),
                    Some(panels::PaletteOutcome::AddFile) => {
                        if pending_file_chooser.is_none() {
                            *pending_file_chooser = Some(crate::outcomes::FileChooserAdd::start((
                                renderer.primary.window_width as f32 / 2.0 + primary_origin.0,
                                renderer.primary.window_height as f32 / 2.0 + primary_origin.1,
                            )));
                        }
                    }
                    // As the tray's would: snoozing, out of edit mode.
                    Some(panels::PaletteOutcome::Snooze(what)) => {
                        crate::snooze::snooze(what);
                        if layer.state.edit_mode {
                            flip_edit_mode(
                                &mut *layer,
                                &mut *config,
                                scene,
                                &mut *config_dirty,
                                &mut *config_watch,
                                "Wayland, palette snooze",
                            );
                        }
                    }
                    Some(out) => outcomes::apply_palette_outcome(
                        out,
                        &mut outcome_ctx!(scene, selection, toasts, config_dirty, renderer),
                        &mut *config,
                    ),
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
                        &mut outcome_ctx!(scene, selection, toasts, config_dirty, renderer),
                        library_root.as_deref(),
                        &mut *library,
                        at,
                    );
                }
            }
            // Reconfigured, as on X11: an outdated surface was left as it
            // was, and failed again every frame.
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                renderer.resize(
                    renderer.primary.window_width,
                    renderer.primary.window_height,
                );
                *primary_frame = None;
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                return Err(AnimaError::other("GPU out of memory"));
            }
            // Transient, as on X11: this frame is dropped, the next tries.
            Err(wgpu::SurfaceError::Timeout) => {
                tracing::debug!("surface acquire timed out; skipping frame");
            }
            Err(e) => render_errors.failed("the overlay", &e),
        }

        Ok(())
    }
}

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

/// The longest the loop waits, at start, to learn which monitor the
/// primary surface landed on before making the other monitors' overlays
/// on a guess (`awaiting_primary_output`).
const PRIMARY_OUTPUT_WAIT: std::time::Duration = std::time::Duration::from_millis(500);

/// Whether the loop is still waiting to learn the primary's monitor: with
/// several monitors, each with an overlay of its own, until the compositor
/// names it or [`PRIMARY_OUTPUT_WAIT`] has gone by.
fn awaiting_primary_output(
    mode: &MonitorMode,
    monitors: usize,
    primary_known: bool,
    since_start: std::time::Duration,
) -> bool {
    matches!(mode, MonitorMode::PerMonitor)
        && monitors > 1
        && !primary_known
        && since_start < PRIMARY_OUTPUT_WAIT
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
        // Tab is the panel's, for its focus, as on X11 where the toolkit
        // takes every one: here the first Tab, before anything had the
        // focus, both moved it and fired a shortcut.
        egui::Event::Key {
            key: egui::Key::Tab,
            ..
        } => None,
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

/// The primary surface made again after the compositor closed it
/// (`LayerWindow::reopen_primary`), with a wgpu surface on its new
/// `wl_surface`. The old wgpu surface goes, with the state it is replaced
/// by, before the old layer and its `wl_surface` — the order
/// `rebuild_extra_surfaces` keeps for the extras.
fn reopen_primary(layer: &mut LayerWindow, renderer: &mut WgpuRenderer) -> Result<()> {
    let (old, wl_surface) = layer.reopen_primary()?;
    let (w, h) = layer.state.pending_size.take().unwrap_or((
        renderer.primary.window_width,
        renderer.primary.window_height,
    ));
    let wgpu_surface = layer_window::build_wgpu_surface(
        &renderer.shared.instance,
        &layer.connection,
        &wl_surface,
    )?;
    renderer.primary = SurfaceState::new(&renderer.shared, wgpu_surface, w, h);
    drop(old);
    Ok(())
}

/// What the loop does about the overlay going, at the top of an iteration.
#[derive(Debug, PartialEq, Eq)]
enum Closing {
    /// Nothing to do: it is open, and nobody asked to quit.
    Open,
    /// Quit was asked for — the tray, a signal, the shortcut.
    Quit,
    /// The compositor closed it with no monitor left: wait for one.
    WaitForMonitor,
    /// The compositor closed it: make it again.
    Reopen,
    /// The compositor closed it once too often ([`Reopens`]): exit.
    GiveUp,
}

/// [`Closing`], from whether quit was asked for, whether the compositor
/// closed the overlay, and whether any monitor is there. A quit is never
/// taken for a close: in 1.5.1 one flag stood for both, and Quit from the
/// tray or the session's SIGTERM at logout made the overlay again instead
/// of ending the app.
fn closing(
    quit_requested: bool,
    closed: bool,
    monitors: bool,
    reopens: &mut Reopens,
    now: Instant,
) -> Closing {
    if quit_requested {
        Closing::Quit
    } else if !closed {
        Closing::Open
    } else if !monitors {
        Closing::WaitForMonitor
    } else if !reopens.allow(now) {
        Closing::GiveUp
    } else {
        Closing::Reopen
    }
}

/// How often the compositor may close the overlay and see it made again:
/// past [`Reopens::MAX`] in [`Reopens::WINDOW`], one that will not hold it
/// ends the app rather than a loop of surfaces.
#[derive(Default)]
struct Reopens {
    at: Vec<Instant>,
}

impl Reopens {
    const MAX: usize = 5;
    const WINDOW: Duration = Duration::from_secs(60);

    /// Whether one more, at `now`, is allowed.
    fn allow(&mut self, now: Instant) -> bool {
        self.at
            .retain(|t| now.saturating_duration_since(*t) < Self::WINDOW);
        if self.at.len() >= Self::MAX {
            return false;
        }
        self.at.push(now);
        true
    }
}

/// The ⚙ corner's side outside edit mode: none when the button is hidden
/// (`GlobalConfig::show_toggle_button`), so the overlay passes every click.
fn shown_button(global: &crate::config::GlobalConfig) -> u32 {
    if global.show_toggle_button {
        TOGGLE_BUTTON_SIZE
    } else {
        0
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

/// Dispatch Wayland events, waiting at most `timeout` for the socket —
/// or for another thread's wake-up (`wayland::waker`), which ends the
/// wait the moment it comes.
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
/// flush, read, dispatch again) but polls the connection fd, and the
/// waker's beside it, with a deadline, so the loop sleeps in the kernel
/// when idle and never waits longer than `timeout`.
fn dispatch_with_timeout(
    queue: &mut EventQueue<WaylandState>,
    state: &mut WaylandState,
    waker: Option<&crate::wayland::waker::Waker>,
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
        // The Wayland socket, and the waker's beside it when there is one;
        // without it the loop waits no longer than a frame interval, and
        // looks at its channels that often, as it used to.
        let mut pfds = [
            libc::pollfd {
                fd: fd.as_raw_fd(),
                events: libc::POLLIN | libc::POLLERR,
                revents: 0,
            },
            libc::pollfd {
                fd: waker.map_or(-1, |w| w.fd().as_raw_fd()),
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        let timeout = if waker.is_some() {
            timeout
        } else {
            timeout.min(FRAME_INTERVAL)
        };
        // Rounded up, so a deadline 0.4 ms away is not a busy loop of
        // zero-length polls.
        let ms = timeout.as_micros().div_ceil(1000).min(i32::MAX as u128) as i32;
        // SAFETY: `pfds` is two initialised `pollfd`s we own for the whole
        // call, and `poll` touches only those; a negative fd is skipped.
        // The Wayland fd stays open because `guard` borrows the live
        // connection, the waker's because `waker` borrows it.
        let ready = unsafe { libc::poll(pfds.as_mut_ptr(), pfds.len() as libc::nfds_t, ms) };
        if ready > 0 && pfds[1].revents != 0 {
            if let Some(waker) = waker {
                waker.drain();
            }
        }
        if ready > 0 && pfds[0].revents != 0 {
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

/// A focus or snooze shortcut, as the event it stands for now
/// (`hotkeys::action_to_event`), done here. Returns whether it snoozed.
fn timer_shortcut(action: Action, focus: crate::focus::FocusConfig) -> bool {
    let visible = std::sync::atomic::AtomicBool::new(true);
    match crate::hotkeys::action_to_event(action, &visible) {
        Some(AnimaEvent::StartFocus(minutes)) => {
            crate::focus::start(crate::focus::lengths_for(minutes, focus));
        }
        Some(AnimaEvent::StopFocus) => crate::focus::stop(),
        Some(AnimaEvent::Snooze(what)) => {
            crate::snooze::snooze(what);
            return true;
        }
        Some(AnimaEvent::Wake) => crate::snooze::wake(),
        _ => {}
    }
    false
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
    config.take_scene(scene);
    config.save()
}

/// Pure half of `rebuild_extra_surfaces`'s decision: given what the
/// plan wants and what's currently tracked, which names need tearing
/// down and which planned monitors need a brand-new surface. Split
/// out so this decision has a unit test even though the actual
/// teardown/creation (real Wayland + GPU calls) doesn't.
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
        // Its monitor gone, a monitor mode without it, or the primary's
        // own: a compositor slower than `PRIMARY_OUTPUT_WAIT` to say where
        // the primary landed.
        tracing::info!("Closed the extra surface on {name}");
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
fn flip_edit_mode(
    layer: &mut LayerWindow,
    config: &mut AppConfig,
    scene: &Scene,
    config_dirty: &mut bool,
    watcher: &mut crate::config_watch::ConfigWatcher,
    via: &str,
) {
    let new_mode = !layer.state.edit_mode;
    if let Err(e) = layer.set_edit_mode(new_mode, shown_button(&config.global)) {
        tracing::warn!("Failed to flip input region on edit toggle: {e}");
        return;
    }
    // Editing wants them in view: it ends a snooze (`crate::snooze`), or
    // the panel would open hidden.
    if new_mode {
        crate::snooze::wake();
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
        // Tab is the panel's, never a shortcut's.
        assert!(binding_chord(&key_event(egui::Key::Tab, true), false).is_none());
        assert!(binding_chord(&egui::Event::Text("v".into()), false).is_none());
        assert!(binding_chord(&egui::Event::PointerGone, false).is_none());
    }

    /// Several monitors, each with an overlay: wait to learn the primary's,
    /// for a moment at most; otherwise there is nothing to wait for.
    #[test]
    fn the_other_overlays_wait_for_the_primarys_monitor() {
        use std::time::Duration;
        let per = MonitorMode::PerMonitor;
        let soon = Duration::from_millis(30);
        assert!(awaiting_primary_output(&per, 2, false, soon));
        assert!(!awaiting_primary_output(&per, 2, true, soon));
        assert!(!awaiting_primary_output(&per, 1, false, soon));
        assert!(!awaiting_primary_output(&MonitorMode::Span, 2, false, soon));
        assert!(!awaiting_primary_output(
            &per,
            2,
            false,
            PRIMARY_OUTPUT_WAIT
        ));
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
    fn a_closed_overlay_is_made_again_but_not_forever() {
        let t0 = Instant::now();
        let mut reopens = Reopens::default();
        for i in 0..Reopens::MAX {
            assert!(
                reopens.allow(t0 + Duration::from_secs(i as u64)),
                "close {i}"
            );
        }
        assert!(
            !reopens.allow(t0 + Duration::from_secs(10)),
            "a sixth within the minute"
        );
        assert!(
            reopens.allow(t0 + Duration::from_secs(61)),
            "a minute on, again"
        );
    }

    #[test]
    fn a_quit_ends_the_loop_and_is_not_taken_for_a_close() {
        let now = Instant::now();
        let mut reopens = Reopens::default();
        // Asked to quit: out, whatever the compositor did or did not do.
        for closed in [false, true] {
            for monitors in [false, true] {
                assert_eq!(
                    closing(true, closed, monitors, &mut reopens, now),
                    Closing::Quit
                );
            }
        }
        // None of those used up a reopen.
        assert!(reopens.at.is_empty());
        assert_eq!(
            closing(false, false, true, &mut reopens, now),
            Closing::Open
        );
        assert_eq!(
            closing(false, true, false, &mut reopens, now),
            Closing::WaitForMonitor
        );
        assert_eq!(
            closing(false, true, true, &mut reopens, now),
            Closing::Reopen
        );
        for _ in 1..Reopens::MAX {
            closing(false, true, true, &mut reopens, now);
        }
        assert_eq!(
            closing(false, true, true, &mut reopens, now),
            Closing::GiveUp
        );
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

        // The ⚙ button hidden (issue #1): no corner, so no click is taken
        // outside edit mode; edit mode still takes the whole surface.
        let mut global = crate::config::GlobalConfig::default();
        assert_eq!(shown_button(&global), TOGGLE_BUTTON_SIZE);
        global.show_toggle_button = false;
        let none = region_for_state(false, false, 1920, 1080, shown_button(&global)).unwrap();
        assert_eq!((none.w, none.h), (0, 0));
        let full = region_for_state(false, true, 1920, 1080, shown_button(&global)).unwrap();
        assert_eq!((full.w, full.h), (1920, 1080));
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
