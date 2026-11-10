//! Native Wayland run loop — opt-in via `ANIMA_USE_WAYLAND_NATIVE=1`.
//!
//! This is the proof-of-concept stitching everything in `src/wayland/`
//! together: layer surface (7.2), pointer translation (7.3), input region
//! (7.4), and a sprite-only render loop driven by `WgpuRenderer`.
//!
//! Its state is a `Native`; one turn of the loop, `Native::iteration`,
//! is a list of phases, each a method in a file of `run/` (`wake`,
//! `circumstances`, `commands`, `input`, `frame`). docs/architecture.md,
//! "The native loop", has the table.
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

// The phases of a turn, each an `impl Native` of its own; after
// `outcome_ctx!`, which they use.
mod circumstances;
mod commands;
mod frame;
mod input;
mod wake;

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
        let asked = self.draw(
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
        if let Some(asked) = asked {
            self.apply_asked(asked, outputs.primary_origin);
        }
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
