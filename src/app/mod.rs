/// The fields of `App` a panel outcome or a reload may touch, borrowed
/// one by one: a method returning this would borrow all of `self`, and the
/// callers need `config`, `library` or `warnings` alongside it.
macro_rules! outcome_ctx {
    ($app:expr) => {
        crate::outcomes::OutcomeCtx {
            scene: &mut $app.scene,
            selection: &mut $app.selection,
            toasts: &mut $app.toasts,
            config_dirty: &mut $app.config_dirty,
            renderer: $app.renderer.as_mut(),
        }
    };
}

mod dispatch;
mod events;
mod hot_reload;
mod input;
mod lifecycle;
mod outcomes;
mod overlay;
mod render_loop;
pub(crate) mod windows;

use crate::config::AppConfig;
use crate::constants::TOGGLE_BUTTON_SIZE;
use crate::event::AnimaEvent;
use crate::input::drag::DragController;
use crate::input::selection::SelectionState;
use crate::renderer::wgpu_renderer::WgpuRenderer;
use crate::scene::Scene;
use crate::ui::Warning;
use crate::ui::{EguiRenderer, ToastQueue};
use crate::window::overlay::OverlayPlatform;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{Window, WindowId, WindowLevel};

/// The ⚙ toggle button's size in **physical pixels**, for a window whose
/// scale factor is `scale_factor`.
///
/// [`TOGGLE_BUTTON_SIZE`] is expressed in egui points (logical units) and
/// egui paints at the window's scale factor, so the button occupies
/// `size × scale` physical pixels on screen. The X11 input region,
/// however, is specified in physical pixels. Passing the raw constant to
/// both — which is what the code did — meant that on a HiDPI display
/// (scale 2) the button was drawn 128px wide while only a 64px corner
/// accepted clicks, leaving three quarters of the visible button dead.
pub(crate) fn toggle_button_px(scale_factor: f64) -> u32 {
    let px = (TOGGLE_BUTTON_SIZE as f64) * scale_factor;
    // A non-finite or absurd scale must not produce a zero-sized (or
    // wrapped) region — that would make the overlay unreachable.
    if !px.is_finite() {
        return TOGGLE_BUTTON_SIZE;
    }
    px.round().clamp(1.0, u32::MAX as f64) as u32
}

/// Main application state — implements winit's ApplicationHandler.
///
/// The overlay operates in two modes:
/// - **Pass-through mode** (default): clicks go through to the desktop,
///   except for a small toggle button in the top-right corner.
/// - **Edit mode** (click toggle button): the full overlay
///   receives input. You can drag characters, select them, use keyboard shortcuts.
pub struct App {
    /// The winit window (created on resume)
    window: Option<Arc<Window>>,
    /// GPU renderer
    renderer: Option<WgpuRenderer>,
    /// The scene with all entities
    scene: Scene,
    /// Application config
    config: AppConfig,
    /// Drag controller
    drag: DragController,
    /// A selection rectangle being dragged over empty space.
    marquee: Option<crate::input::multi::Marquee>,
    /// Selection state
    selection: SelectionState,
    /// Current mouse position
    mouse_x: f32,
    mouse_y: f32,
    /// Whether config needs saving (dirty flag)
    config_dirty: bool,
    /// Whether the overlay is in "edit mode" (interactive) or "pass-through" mode
    /// Default: false (pass-through — clicks go to desktop)
    edit_mode: bool,
    /// Whether Shift key is currently held (for fine control)
    shift_held: bool,
    /// Whether Ctrl key is currently held (used for Ctrl-modified chords).
    ctrl_held: bool,
    /// Whether Alt is currently held — tracked so user-bound chords
    /// involving Alt resolve correctly through `KeyBindings::lookup`.
    alt_held: bool,
    /// Whether Super (Win/Cmd/Meta) is currently held — same reason.
    super_held: bool,
    /// Pooled X11 input manager (holds a single X11 connection)
    x11_input: Option<Box<dyn OverlayPlatform>>,
    /// Watches config.toml and reloads it off the UI thread — the same
    /// watcher the native Wayland loop uses.
    config_watch: crate::config_watch::ConfigWatcher,
    /// In-flight off-thread Shimeji import. `Some` while a worker copies a
    /// pack's sprites (the slow part) so a large pack can't freeze the UI;
    /// the result is applied on the UI thread at the captured drop point.
    pending_shimeji: Option<crate::outcomes::ShimejiImport>,
    /// The desktop's file chooser, open after "Add file…" until it answers.
    pending_file_chooser: Option<crate::outcomes::FileChooserAdd>,
    /// A scene being shared or imported (`crate::scene_file`).
    pending_transfer: Option<crate::outcomes::SceneTransfer>,
    /// The desktop's clipboard (`crate::x11_clipboard`): `None` until the
    /// first copy or paste, then what starting it gave.
    #[cfg(unix)]
    x11_clipboard: Option<Option<crate::x11_clipboard::X11Clipboard>>,
    /// A paste waiting on the clipboard's answer, and where it lands.
    pending_paste: Option<(
        std::sync::mpsc::Receiver<crate::system_clipboard::Content>,
        (f32, f32),
    )>,
    /// Text a text field asked for with Ctrl+V, on its way.
    pending_text_paste: Option<std::sync::mpsc::Receiver<crate::system_clipboard::Content>>,
    /// egui integration. Paints in BOTH modes — the ⚙ toggle button, the
    /// bubbles and the toasts live in pass-through too. The rest (settings
    /// panel, context menu, palette) is gated to edit mode inside the build
    /// closure.
    ui: Option<EguiRenderer>,
    /// The primary window's last frame, for its pacing outside edit mode
    /// (`windows::needs_frame`); `None` draws the next.
    primary_frame: Option<windows::LastFrame>,
    /// Ephemeral UI state (currently just the context menu) kept separate
    /// from the egui renderer so it survives across resumed/suspended cycles.
    ui_state: UiState,
    /// Toast notification queue. Persistent across edit/pass-through
    /// transitions but only painted when in edit mode (no UI otherwise).
    toasts: ToastQueue,
    /// Edits to the scene, for undo and redo (`crate::undo`).
    history: crate::undo::UndoHistory,
    /// Whether the window in front is full screen, as the full-screen
    /// watch last reported (`crate::fullscreen`).
    fullscreen_in_front: bool,
    /// What stepping aside for it currently asks — apart from the user's
    /// own hide (`overlay_hidden`) and pause.
    stepped_aside: crate::fullscreen::StepAside,
    /// Whether the windows were last put above the others (`true`) or
    /// under them; `None` before the first time (`apply_stacking`).
    stacked_on_top: Option<bool>,
    /// The primary's render errors, said once each (`renderer::ErrorOnce`).
    render_errors: crate::renderer::ErrorOnce,
    /// Bedtime against the clock (`crate::bedtime`), and whether it is.
    bedtime_watch: crate::bedtime::Watch,
    /// The clock for characters' own hours (`crate::hours`).
    hours_clock: crate::hours::Clock,
    /// Watching for the user's return (`crate::welcome`).
    welcome: crate::welcome::Welcome,
    /// The pace by the time of day, and the yawns (`crate::moods`).
    moods: crate::moods::Moods,
    /// Whether the overlay was out of sight last frame: back, everyone
    /// fades in (`Scene::fade_all_in`).
    was_hidden: bool,
    bedtime_now: bool,
    /// An app the user chose is the window in front (`crate::chosen_apps`).
    chosen_app_in_front: bool,
    /// Tapping in pass-through (`crate::tap`): the characters' rectangles
    /// on the primary window, when they last followed them, whether the
    /// windows may take the keyboard, and a press waiting for its release.
    tap_rects: Vec<crate::tap::SurfaceRect>,
    tap_refreshed: Instant,
    focusable: Option<bool>,
    tap_gesture: crate::tap::TapGesture,
    /// Idle past the chosen time, as the away watch last reported
    /// (`crate::away`).
    away: bool,
    /// Running on battery, as the away watch last reported.
    on_battery: bool,
    /// Whether this session has an idle time to read; `None` until the
    /// watch has looked.
    idle_source: Option<bool>,
    /// When each reminder is next due (`crate::reminders`).
    reminder_timers: crate::reminders::Timers,
    /// The monitor whose overlay the pointer was last over, `None` for the
    /// panel's window: where a right-click menu opens.
    pointer_monitor: Option<String>,
    /// Scenes by time of day: what the clock last showed (`crate::schedule`).
    schedule: crate::schedule::Schedule,
    /// Session-lifetime warnings rendered as a banner at the top of
    /// the settings panel (D.5). Distinct from toasts: these persist
    /// until the underlying condition clears or the user dismisses
    /// the banner. Stored as `BTreeSet` so insertion is idempotent
    /// (the same warning fired twice doesn't duplicate the banner)
    /// and display order is deterministic.
    warnings: std::collections::BTreeSet<Warning>,
    /// Per-system frame-time + total sampler (D.6). Always populated;
    /// the overlay widget is what's actually toggled. Keeping the
    /// sampler always-on costs ~5 µs/frame which is below any
    /// perceivable noise, and lets the overlay show meaningful
    /// averages the moment it opens.
    perf_sampler: crate::perf::PerfSampler,
    /// Whether the perf overlay widget is currently visible. Toggled
    /// via `Action::TogglePerfOverlay` (`Ctrl+Shift+\`` by default).
    perf_overlay_visible: bool,
    /// Cached resident-set size (KiB) shown in the perf overlay. Updated
    /// every `RSS_REFRESH_FRAMES` frames so the proc-fs read doesn't
    /// land in the per-frame budget. `None` until the first read or on
    /// non-Linux platforms.
    perf_last_rss_kib: Option<u64>,
    /// Frame counter for the RSS refresh cadence.
    perf_frame_counter: u32,
    /// Soak-test metrics emitter (W.1). `None` unless
    /// `ANIMA_SOAK_METRICS` is set; ticked once per frame.
    soak: Option<crate::soak::SoakRecorder>,
    /// Previous frame's GPU texture uploads / draw calls, for the perf
    /// HUD (W.3). Captured-and-reset at each frame's start.
    gpu_uploads: u32,
    gpu_draws: u32,
    /// Consecutive frames the primary surface has come back `Lost` /
    /// `Outdated`. Reset on any successful present. A short streak is a
    /// transient blip the per-frame `surface.configure()` clears (resize,
    /// occlusion, the first frame or two after S3 resume); a streak past
    /// `render_loop::SURFACE_LOSS_REBUILD_THRESHOLD` means the surface
    /// reconfigure isn't recovering it (driver reset, GPU hot-unplug,
    /// device lost
    /// across suspend) and we rebuild the renderer wholesale.
    surface_loss_streak: u32,
    /// Snapshot of the monitor topology taken on the first `resumed()`
    /// — empty until then. Used by the picker UI (C.2) and the
    /// per-monitor render path (C.3); the data layer (this commit /
    /// C.1) only populates and logs it.
    monitors: Vec<crate::monitor::MonitorInfo>,
    /// EWMH window watcher for window-awareness. `None` until first
    /// use or when no X server exists; `watcher_probe_done` stops us
    /// re-attempting the connection every poll on Wayland. X11-only by
    /// nature — off unix the physics fall back to the screen floor, the
    /// same as a native Wayland session does today.
    #[cfg(unix)]
    window_watcher: Option<crate::window::x11_windows::WindowWatcher>,
    #[cfg(unix)]
    window_watcher_probe_done: bool,
    /// Last desktop-window poll — 300 ms cadence, see
    /// `poll_window_platforms`.
    last_window_poll: Instant,
    /// Whether the previous poll pushed a non-empty platform set into
    /// the scene — lets the disable path clear it exactly once.
    window_platforms_active: bool,
    /// Asset library index. `None` when no asset root was discovered
    /// at startup (env var unset, XDG dir missing, no exe-relative
    /// fallback). The UI shows an empty state in that case rather
    /// than failing.
    library: Option<crate::asset_library::LibraryIndex>,
    /// Asset root path used at startup. Kept so the "Add to scene"
    /// path can resolve relative asset paths to absolute without
    /// re-scanning.
    library_root: Option<std::path::PathBuf>,
    /// The library's fresh scan, until it arrives (`asset_library::open`).
    library_scan: Option<std::sync::mpsc::Receiver<crate::asset_library::LibraryIndex>>,
    /// Rhai host for `Behavior::Script`. Lives here rather than on
    /// `Scene`, which must stay `Send` for the hot-reload worker.
    script_host: crate::scripting::ScriptHost,
    /// Sound playback for scripts. Opens a device once at startup and
    /// falls back to silence when there isn't one.
    audio_host: crate::audio::AudioHost,
    /// The live global-hotkey backend, shown in the Keybindings tab
    /// (T.4). Set from `main` after strategy resolution; updated when
    /// the deferred portal fallback fires.
    hotkey_backend_status: crate::hotkeys::probe::HotkeyStatus,
    /// Extra overlay windows for `MonitorMode::PerMonitor` (T.6) —
    /// one per non-primary monitor, sprite-only (egui stays on the
    /// primary). Keyed by `WindowId` for event routing.
    extra_windows: std::collections::HashMap<WindowId, windows::WindowSlot>,
    /// Whether `HideOverlay` is currently in effect. Tracked so a
    /// monitor hotplug (which rebuilds `extra_windows`) doesn't put
    /// freshly created overlays on screen while the user has the whole
    /// overlay hidden.
    overlay_hidden: bool,
    /// Mode snapshot from the last window (re)build — the redraw
    /// handler compares it against config to catch Appearance-tab
    /// switches.
    last_monitor_mode: crate::monitor::MonitorMode,
    /// Last hotplug poll — winit has no monitor-change event on X11,
    /// so the redraw cycle re-enumerates on this cadence (T.9).
    last_monitor_check: Instant,
    /// Last time we re-applied the click-through input shape as a
    /// self-heal. GNOME/XWayland Mutter resets the XShape region behind
    /// our back when it restacks the overlay for always-on-top, which
    /// can leave pass-through mode swallowing every click. Re-applying
    /// on a slow cadence guarantees click-through recovers within this
    /// window no matter what the compositor did. Pass-through only.
    last_shape_refresh: Instant,
}

/// Transient UI state owned by `App` (vs the persistent settings panel
/// which is stateless and rebuilt from `Scene` every frame).
#[derive(Default)]
pub(crate) struct UiState {
    pub context_menu: Option<ContextMenuState>,
}

impl ContextMenuState {
    /// Whether the character it was opened on is still where it points.
    pub(crate) fn still_for(&self, scene: &crate::scene::Scene) -> bool {
        scene
            .entities
            .get(self.entity_idx)
            .is_some_and(|e| e.id == self.entity_id)
    }
}

#[derive(Clone)]
pub(crate) struct ContextMenuState {
    pub entity_idx: usize,
    /// Who it was opened on. The index outlives a scene replaced under it
    /// — a reload, a scene switched in, an import, an undo — and then
    /// named whoever came to sit there: the menu acted on another
    /// character (BUG-011). See [`Self::still_for`].
    pub entity_id: String,
    /// Anchor for the floating menu, in the points of the surface it is on.
    pub pos: egui::Pos2,
    /// The monitor whose overlay shows it — the one clicked on — or `None`
    /// for the panel's surface. It used to open on the panel's surface
    /// always, at its edge nearest a click on another monitor (R51).
    pub surface: Option<String>,
    /// Whether the menu may be dismissed by a click yet.
    ///
    /// The menu opens on the right *press*, and the matching *release*
    /// completes a click a few frames later. Without this guard that
    /// release dismissed the menu it had just opened, so a normal
    /// right-click made the menu flash and vanish. The caller arms the
    /// state once the menu is showing *and* no button is held, so only a
    /// click that begins afterwards can close it.
    ///
    /// Arming after the first frame alone was not enough. It only held
    /// while the release landed inside the menu's hit area, which the
    /// menu's 2 px offset from the click point happened to guarantee —
    /// until a right-click on another monitor put the menu at the edge of
    /// the primary, far from the pointer, and the late release closed it
    /// before it could be used.
    pub armed: bool,
    /// Opened from the keyboard (`Action::OpenContextMenu`): its first
    /// item takes the focus, so Tab and the arrows move through it and
    /// Enter picks.
    pub keyboard: bool,
}

/// However the app ends — its event loop returning, or an exit that does
/// not go through `save_and_exit` — the GPU state goes in the order
/// `release_gpu` keeps, not in field order, which put the window and the
/// renderer before egui and the other monitors' windows.
impl Drop for App {
    fn drop(&mut self) {
        // Before the window, which the fields drop first: on Windows its
        // cursor tracker is joined there, and must be gone before the
        // HWND is (`WinOverlay`, BUG-005).
        self.x11_input = None;
        self.release_gpu();
    }
}

impl App {
    pub fn new(config: AppConfig, scene: Scene) -> Self {
        let last_monitor_mode = config.global.monitor_mode.clone();
        Self {
            window: None,
            renderer: None,
            scene,
            config,
            drag: DragController::new(),
            marquee: None,
            selection: SelectionState::new(),
            mouse_x: 0.0,
            mouse_y: 0.0,
            config_dirty: false,
            edit_mode: false,
            shift_held: false,
            ctrl_held: false,
            alt_held: false,
            super_held: false,
            x11_input: None,
            config_watch: crate::config_watch::ConfigWatcher::new(),
            pending_shimeji: None,
            pending_file_chooser: None,
            pending_transfer: None,
            #[cfg(unix)]
            x11_clipboard: None,
            pending_paste: None,
            pending_text_paste: None,
            ui: None,
            primary_frame: None,
            ui_state: UiState::default(),
            toasts: ToastQueue::default(),
            history: crate::undo::UndoHistory::default(),
            fullscreen_in_front: false,
            stepped_aside: crate::fullscreen::StepAside::default(),
            stacked_on_top: None,
            render_errors: crate::renderer::ErrorOnce::default(),
            bedtime_watch: Default::default(),
            hours_clock: Default::default(),
            welcome: Default::default(),
            moods: Default::default(),
            was_hidden: false,
            bedtime_now: false,
            chosen_app_in_front: false,
            tap_rects: Vec::new(),
            tap_refreshed: Instant::now(),
            focusable: None,
            tap_gesture: Default::default(),
            away: false,
            on_battery: false,
            idle_source: None,
            reminder_timers: crate::reminders::Timers::default(),
            pointer_monitor: None,
            schedule: crate::schedule::Schedule::default(),
            warnings: std::collections::BTreeSet::new(),
            perf_sampler: crate::perf::PerfSampler::default(),
            perf_overlay_visible: false,
            perf_last_rss_kib: None,
            perf_frame_counter: 0,
            soak: crate::soak::SoakRecorder::from_env(),
            gpu_uploads: 0,
            gpu_draws: 0,
            surface_loss_streak: 0,
            monitors: Vec::new(),
            #[cfg(unix)]
            window_watcher: None,
            #[cfg(unix)]
            window_watcher_probe_done: false,
            last_window_poll: Instant::now(),
            window_platforms_active: false,
            library: None,
            library_root: None,
            library_scan: None,
            script_host: crate::scripting::ScriptHost::new(),
            audio_host: crate::audio::AudioHost::new(),
            hotkey_backend_status: crate::hotkeys::probe::HotkeyStatus::TrayOnly,
            extra_windows: std::collections::HashMap::new(),
            overlay_hidden: false,
            last_monitor_mode,
            last_monitor_check: Instant::now(),
            last_shape_refresh: Instant::now(),
        }
    }

    /// Record which global-hotkey backend won resolution — purely
    /// informational, rendered in the Keybindings tab.
    pub fn set_hotkey_backend_status(&mut self, status: crate::hotkeys::probe::HotkeyStatus) {
        self.hotkey_backend_status = status;
    }

    /// Mark a session-lifetime warning. Idempotent — setting the same
    /// variant twice does not duplicate the banner. Called by
    /// `main.rs` for startup-time conditions (global hotkeys
    /// unavailable) and from inside `App` for runtime conditions
    /// (hot-reload worker disconnected).
    pub fn push_warning(&mut self, w: Warning) {
        self.warnings.insert(w);
    }

    // `dispatch_action` lives in `src/app/dispatch.rs` (H.1) — same
    // `impl App` block, split across files so this module stays
    // focused on lifecycle / event-loop wiring.

    // Hot-reload (`check_hot_reload`) lives in `src/app/hot_reload.rs`,
    // outcome handlers (`handle_{menu,library,palette}_outcome`) in
    // `src/app/outcomes.rs`; both apply through the modules the Wayland
    // loop shares, `crate::config_watch` and `crate::outcomes`.

    /// The single shutdown path: persist any pending edits, then tear
    /// down GPU resources in the required order and stop the loop.
    ///
    /// Every exit routes through here — including the abnormal ones (GPU
    /// out of memory, a renderer rebuild that failed). Those used to call
    /// `event_loop.exit()` directly, which silently discarded whatever
    /// the user had changed since the last save; losing the GPU is not a
    /// reason to also lose their scene.
    pub(super) fn save_and_exit(&mut self, event_loop: &ActiveEventLoop) {
        self.save_config_if_needed();
        self.release_gpu();
        self.x11_input = None;
        event_loop.exit();
    }

    /// The GPU state, children before the device they were made on:
    /// egui, then the other monitors' windows (their surfaces and egui),
    /// then the renderer. The monitors' windows were left to the end
    /// before, after the renderer and its device (BUG-006).
    fn release_gpu(&mut self) {
        self.ui = None;
        self.extra_windows.clear();
        self.primary_frame = None;
        self.renderer = None;
    }

    /// Save config if dirty
    fn save_config_if_needed(&mut self) {
        if self.config_dirty {
            // The crash snapshot and the watcher learn of it there.
            match self.config_watch.save(&mut self.config, &self.scene) {
                Ok(()) => {
                    self.toasts.success(crate::i18n::t("toast-config-saved"));
                    // Clear the dirty flag only on a successful write. A
                    // transient failure (disk full, permissions) must keep
                    // the scene dirty so the next save-triggering edit
                    // retries instead of silently dropping the change.
                    // Saves fire on discrete edits, not per frame, so a
                    // persistent failure re-toasts per edit, not per frame.
                    self.config_dirty = false;
                }
                Err(e) => {
                    tracing::warn!("Failed to save config: {}", e);
                    let mut args = fluent::FluentArgs::new();
                    args.set("error", e.said());
                    self.toasts
                        .error(crate::i18n::t_args("toast-save-failed", &args));
                }
            }
        }
    }
}

impl App {
    /// Ask winit for a frame. Cheap and idempotent (winit coalesces);
    /// called from every input/user-event path so the paced render
    /// loop wakes up when state changed outside `RedrawRequested`.
    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl ApplicationHandler<AnimaEvent> for App {
    /// Timer wake-ups from the paced render loop (`ControlFlow::WaitUntil`).
    ///
    /// Re-arms the heartbeat *before* requesting the redraw so the
    /// chain survives even when the compositor suppresses redraw
    /// delivery for a hidden window; `check_hot_reload` runs here
    /// directly for the same reason (config edits must apply while
    /// the overlay is hidden). Both are cheap: the hot-reload check
    /// self-gates on a 2 s mtime poll.
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if matches!(cause, StartCause::ResumeTimeReached { .. }) {
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + crate::pacing::IDLE_HEARTBEAT,
            ));
            self.check_hot_reload();
            self.check_shimeji_import();
            self.check_file_chooser();
            self.check_scene_transfer();
            self.check_clipboard();
            // A snooze ending while every window is hidden: no frame would
            // notice, so the heartbeat looks.
            self.update_step_aside();
            self.request_redraw();
        }
    }

    /// Handle tray / global-hotkey commands routed through the event loop
    /// (`app::events`).
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AnimaEvent) {
        self.on_event(event_loop, event);
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Init body extracted to src/app/lifecycle.rs (H.5).
        self.handle_resumed(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        // A step to undo opens at the first thing someone does, on any
        // window, before it is applied (`crate::undo`). Pointer motion
        // does not count: it would merge the edits either side of it.
        if matches!(
            event,
            WindowEvent::MouseInput { .. }
                | WindowEvent::KeyboardInput { .. }
                | WindowEvent::MouseWheel { .. }
                | WindowEvent::DroppedFile(_)
                | WindowEvent::Ime(_)
        ) {
            self.history
                .input(&self.scene, &self.selection, std::time::Instant::now());
        }
        // Events from extra (PerMonitor) windows get a narrow handler:
        // sprites repaint on expose, surfaces resize, shapes re-apply.
        // Pointer/keyboard routing for extras (T.8, below) only
        // matters in edit mode — pass-through extras are fully
        // click-through, so the compositor never delivers them input.
        let is_primary = self.window.as_ref().is_some_and(|w| w.id() == window_id);
        if !is_primary {
            self.extra_window_event(event_loop, window_id, event);
            return;
        }

        // Let egui peek at the event first. The toggle ⚙ button is an egui
        // widget that lives in BOTH modes, so we always forward; a consumed
        // event short-circuits our own handlers (so e.g. clicking the button
        // doesn't also try to drag an entity).
        if let (Some(ui), Some(window)) = (self.ui.as_mut(), self.window.as_ref()) {
            if ui.handle_event(window, &event) {
                // egui reacted (hover state, button press…) — make sure
                // a frame shows it even when the paced loop is asleep.
                window.request_redraw();
                return;
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                tracing::info!("Close requested — saving config and exiting");
                self.save_and_exit(event_loop);
            }

            // The screen's scale changed (`Xft.dpi` reloaded): the
            // characters follow it, as the panel does.
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                crate::entity::set_display_scale(scale_factor as f32);
                crate::entity::set_detail_scale(scale_factor as f32);
                self.scene.sharpen(crate::entity::picture_cap());
                self.request_redraw_all();
            }

            WindowEvent::Resized(physical_size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(physical_size.width, physical_size.height);
                }
                // The input shape mask is sized to the old window dimensions;
                // re-apply for the new size in whatever mode we're in.
                self.reapply_input_shape();
                self.request_redraw();
            }

            // Every focus / occlusion transition. Mutter (GNOME on
            // XWayland) both sinks our always-on-top *and* can reset the
            // XShape click-through region when it processes the resulting
            // window-state change. So re-assert ABOVE first, then re-apply
            // the input shape **last** — order is load-bearing: doing the
            // shape first (or not at all on focus loss, the earlier bug)
            // let the ABOVE re-assert clobber it, leaving the overlay
            // swallowing every click on Arch's newer Mutter (the user
            // couldn't reach windows under the sprites). Shape-last keeps
            // click-through and stay-on-top from desyncing.
            WindowEvent::Focused(_) | WindowEvent::Occluded(_) => {
                self.reassert_always_on_top();
                self.reapply_input_shape();
                // Drawn whatever it shows: the compositor may have let go
                // of its contents.
                self.primary_frame = None;
                self.request_redraw();
            }

            WindowEvent::RedrawRequested => {
                // The whole render pipeline lives in
                // `src/app/render_loop.rs` (H.4a) so this match arm
                // stays a one-line delegate.
                self.handle_redraw_requested(event_loop);
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.handle_cursor_moved(position);
                self.request_redraw();
            }

            WindowEvent::MouseInput { state, button, .. } => {
                self.handle_mouse_input(state, button);
                self.request_redraw();
            }

            // Edit-mode keyboard dispatch goes through the rebindable
            // `KeyBindings::lookup` table. Conversion failures (function
            // keys, IME, etc.) and unbound chords are silent no-ops —
            // every other path stays inside `dispatch_action`.
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        state: ElementState::Pressed,
                        ref logical_key,
                        physical_key,
                        ..
                    },
                ..
            } if self.edit_mode => {
                self.dispatch_key(logical_key.as_ref(), physical_key, event_loop, false);
            }

            WindowEvent::MouseWheel { delta, .. } => {
                self.handle_mouse_wheel(delta);
                self.request_redraw();
            }

            WindowEvent::DroppedFile(path) => {
                self.handle_dropped_file(path);
                self.request_redraw();
            }

            WindowEvent::HoveredFile(path) => {
                self.handle_hovered_file(path);
                self.request_redraw();
            }

            WindowEvent::ModifiersChanged(modifiers) => {
                self.handle_modifiers_changed(modifiers);
            }

            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The constant is in egui points but the X11 input region is in
    /// physical pixels. Handing the raw value to both left three quarters
    /// of the drawn button unclickable at scale 2.
    #[test]
    fn toggle_button_px_scales_logical_points_to_physical() {
        assert_eq!(toggle_button_px(1.0), TOGGLE_BUTTON_SIZE);
        assert_eq!(toggle_button_px(2.0), TOGGLE_BUTTON_SIZE * 2);
        // Fractional scaling (GNOME 125% / 150%) rounds to whole pixels.
        assert_eq!(toggle_button_px(1.25), 80);
        assert_eq!(toggle_button_px(1.5), 96);
    }

    /// A bad scale must never yield a zero-sized region — that would make
    /// the overlay permanently unreachable in pass-through mode.
    #[test]
    fn toggle_button_px_never_degenerates_to_zero() {
        assert!(toggle_button_px(0.0) >= 1);
        assert!(toggle_button_px(-1.0) >= 1);
        assert_eq!(toggle_button_px(f64::NAN), TOGGLE_BUTTON_SIZE);
        assert_eq!(toggle_button_px(f64::INFINITY), TOGGLE_BUTTON_SIZE);
    }
}

#[cfg(test)]
mod menu_tests {
    use super::ContextMenuState;

    /// A menu stays for the character it was opened on: once the scene
    /// under it is replaced and another sits at its index, it is not that
    /// one's (BUG-011).
    #[test]
    fn a_menu_is_for_its_own_character() {
        let config = crate::config::AppConfig::default();
        let mut scene = crate::scene::Scene::from_config(&config);
        let template = config.characters[0].clone();
        let at = |ids: &[&str]| -> Vec<_> {
            ids.iter()
                .map(|id| crate::config::CharacterConfig {
                    id: (*id).into(),
                    asset_path: String::new(),
                    ..template.clone()
                })
                .collect()
        };
        scene.restore_configs(&at(&["a", "b"]));
        let menu = ContextMenuState {
            entity_idx: 0,
            entity_id: "a".into(),
            pos: egui::Pos2::ZERO,
            surface: None,
            armed: false,
            keyboard: false,
        };
        assert!(menu.still_for(&scene));
        scene.restore_configs(&at(&["b", "a"]));
        assert!(!menu.still_for(&scene), "the menu now named b");
        scene.restore_configs(&at(&[]));
        assert!(!menu.still_for(&scene));
    }
}
