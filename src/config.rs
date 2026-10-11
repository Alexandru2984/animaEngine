use crate::behavior::Behavior;
use crate::constants::MAX_ENTITIES;
use crate::error::Result;
use crate::keybindings::KeyBindings;
use crate::monitor::MonitorMode;
use crate::ui::{CollapseState, OnboardingProgress, Theme};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Global application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    #[serde(default = "default_true")]
    pub always_on_top: bool,
    #[serde(default = "default_true")]
    pub transparent: bool,
    #[serde(default = "default_true")]
    pub playback_enabled: bool,
    /// Window width (0 = auto-detect from monitor)
    #[serde(default = "default_window_width")]
    pub window_width: u32,
    /// Window height (0 = auto-detect from monitor)
    #[serde(default = "default_window_height")]
    pub window_height: u32,
    /// Active UI theme. Defaults to Dark; older configs without the
    /// field round-trip through `Theme::default()`.
    #[serde(default)]
    pub theme: Theme,
    /// Active locale code (e.g. "en", "ro", "pt-BR"). `None` means
    /// "detect from environment" — older configs round-trip through
    /// that, so upgrading users keep their OS-level language.
    #[serde(default)]
    pub locale: Option<String>,
    /// Which onboarding hints the user has already dismissed.
    /// `#[serde(default = ...)]` deliberately points at
    /// `OnboardingProgress::all_seen` so existing users (configs
    /// without this field) skip the new hints — only brand-new
    /// installs (`AppConfig::default()`) start with everything
    /// pending.
    #[serde(default = "OnboardingProgress::all_seen")]
    pub onboarding: OnboardingProgress,
    /// How the overlay is distributed across monitors. Defaults to
    /// `PerMonitor` for fresh installs in 0.3; existing 0.2 configs
    /// without the field round-trip through `MonitorMode::default()`
    /// which is also `PerMonitor`. Operators who relied on the
    /// implicit `Span` behaviour of 0.2 can set this explicitly.
    #[serde(default)]
    pub monitor_mode: MonitorMode,

    /// Reduced motion (a11y): disables UI transitions (panel slide,
    /// tab cross-fade, toast/palette animations) and decorative
    /// entity bobbing. Plain knob — no desktop portal exposes this
    /// preference on every DE, so we don't try to auto-detect.
    #[serde(default)]
    pub reduced_motion: bool,

    /// Hover-startle: mascots recoil from the mouse pointer when it comes
    /// within a small radius of them. Off by default so the overlay stays
    /// calm until asked. Cursor tracking is X11-only (same scope as
    /// FollowCursor), so on native Wayland this only reacts in edit mode.
    #[serde(default)]
    pub hover_startle: bool,

    /// What the overlay does while a full-screen app is in front — hide
    /// the characters (the default), pause them, or carry on. X11 and
    /// wlroots-based Wayland compositors; see `crate::fullscreen`.
    #[serde(default)]
    pub on_fullscreen: crate::fullscreen::OnFullscreen,

    /// Apps the characters hide for while one is in front
    /// (`crate::chosen_apps`), by id.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub step_aside_apps: Vec<String>,

    /// Characters take taps outside edit mode (`crate::tap`). Off by
    /// default: their rectangles then stop passing clicks through.
    #[serde(default)]
    pub tap_characters: bool,

    /// Hearts, dust and z's (`crate::effects`). On by default; none with
    /// Reduce motion either way.
    #[serde(default = "default_true")]
    pub little_effects: bool,

    /// Hold the scene still after this many minutes without input; 0
    /// never (`crate::away`). Ten by default: nobody is watching, and a
    /// still scene costs next to nothing.
    #[serde(default = "default_pause_when_idle_minutes")]
    pub pause_when_idle_minutes: u32,
    /// Hold the scene still while the machine runs on battery.
    #[serde(default)]
    pub pause_on_battery: bool,

    /// The saved scene last loaded or saved (`crate::scenes`), by name —
    /// what "Next scene" goes on from, and what the Scene tab can update.
    #[serde(default)]
    pub active_scene: Option<String>,

    /// In the Flatpak, whether the Background portal last agreed to start
    /// the app at login (`crate::autostart`). Installed natively the
    /// autostart entry itself is the truth and this is not read.
    #[serde(default)]
    pub start_at_login: bool,

    /// Hours when the characters doze off by the clock (`crate::bedtime`).
    #[serde(default, skip_serializing_if = "crate::bedtime::Bedtime::is_default")]
    pub bedtime: crate::bedtime::Bedtime,

    /// What sounds with a reminder or a turn of the focus timer
    /// (`crate::reminders::ReminderSound`).
    #[serde(
        default,
        skip_serializing_if = "crate::reminders::ReminderSound::is_none"
    )]
    pub reminder_sound: crate::reminders::ReminderSound,

    /// Reminders and the focus timer go to the desktop's notifications too
    /// (`crate::notify`). Off by default.
    #[serde(default)]
    pub notify_reminders: bool,

    /// Bigger text in every speech bubble (`crate::speech::LARGE`), for
    /// reading. Off by default.
    #[serde(default)]
    pub large_bubbles: bool,

    /// Livelier in the morning, slower at night, a yawn before bedtime
    /// (`crate::moods`). Off by default.
    #[serde(default)]
    pub moods: bool,

    /// How loud the sounds are — taps, reminders, the focus timer,
    /// scripts — from 0 to 1 (`crate::audio`). Full as it comes.
    #[serde(
        default = "default_sound_volume",
        skip_serializing_if = "is_full_volume"
    )]
    pub sound_volume: f32,
    /// No sounds at all; the device is not even opened. Off by default.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sounds_muted: bool,
    /// No sounds while bedtime lasts (`crate::bedtime`). Off by default.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub quiet_at_bedtime: bool,

    /// How long the focus timer's work and break last (`crate::focus`).
    #[serde(default, skip_serializing_if = "crate::focus::FocusConfig::is_default")]
    pub focus: crate::focus::FocusConfig,

    /// Window-awareness: desktop windows become physics platforms —
    /// mascots land on and walk along window top edges. X11 sessions
    /// only (Wayland exposes no global window geometry); silently
    /// inert elsewhere. Off by default like everything physics.
    #[serde(default)]
    pub window_awareness: bool,
    /// Snap a dragged selection's edges and centre to the monitors' and
    /// to other characters' (`crate::input::arrange`). On by default;
    /// Alt held during a drag places freely.
    #[serde(default = "default_true")]
    pub snap_while_dragging: bool,
    /// Characters land on each other's heads and walkers turn around when
    /// they meet (`crate::bump`). Off by default like everything physics.
    #[serde(default)]
    pub characters_bump: bool,

    /// Characters who meet say hello (`crate::greet`). Off by default.
    #[serde(default)]
    pub characters_greet: bool,

    /// A character greets you back after you were away (`crate::welcome`).
    /// On by default.
    #[serde(default = "default_true")]
    pub welcome_back: bool,
    /// Generate AccessKit tree updates (the AT-SPI bridge that drives
    /// screen readers like Orca). On by default — the overhead is
    /// negligible and we want screen-reader users to "just work" out
    /// of the box. Users on minimal setups who want a tighter footprint
    /// (or are bothered by the AT-SPI registration) can flip this off
    /// from Appearance; the change applies live without restart.
    #[serde(default = "default_true")]
    pub accesskit_enabled: bool,
    /// Which global-hotkey backend to use. `auto` (default) probes
    /// the GlobalShortcuts portal first, then falls back to XGrabKey
    /// on X11 sessions; explicit values pin a backend. See
    /// `hotkeys::probe::resolve` for the exact resolution table.
    #[serde(default)]
    pub hotkey_backend: crate::hotkeys::probe::HotkeyBackend,
    /// Last "What's new" version the user has dismissed (D.7). `None`
    /// on pre-0.4 configs and on brand-new installs — the panel fires
    /// once per minor-version bump after that.
    #[serde(default)]
    pub last_seen_whats_new: Option<String>,
}

fn default_sound_volume() -> f32 {
    1.0
}

fn is_full_volume(volume: &f32) -> bool {
    *volume >= 1.0
}

fn default_pause_when_idle_minutes() -> u32 {
    10
}

fn default_window_width() -> u32 {
    0 // 0 = auto-detect from monitor
}
fn default_window_height() -> u32 {
    0 // 0 = auto-detect from monitor
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            always_on_top: true,
            transparent: true,
            playback_enabled: true,
            window_width: 0,  // auto-detect
            window_height: 0, // auto-detect
            theme: Theme::default(),
            locale: None,
            // Brand-new install: nothing has been dismissed yet, so
            // every progressive hint will appear on the first run.
            onboarding: OnboardingProgress::default(),
            monitor_mode: MonitorMode::default(),
            reduced_motion: false,
            hover_startle: false,
            on_fullscreen: crate::fullscreen::OnFullscreen::default(),
            step_aside_apps: Vec::new(),
            tap_characters: false,
            little_effects: true,
            pause_when_idle_minutes: default_pause_when_idle_minutes(),
            pause_on_battery: false,
            active_scene: None,
            start_at_login: false,
            bedtime: Default::default(),
            focus: Default::default(),
            reminder_sound: Default::default(),
            notify_reminders: false,
            large_bubbles: false,
            moods: false,
            sound_volume: default_sound_volume(),
            sounds_muted: false,
            quiet_at_bedtime: false,
            window_awareness: false,
            snap_while_dragging: true,
            characters_bump: false,
            characters_greet: false,
            welcome_back: true,
            accesskit_enabled: true,
            hotkey_backend: crate::hotkeys::probe::HotkeyBackend::Auto,
            last_seen_whats_new: None,
        }
    }
}

/// Asset type for a character
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AssetType {
    PngStatic,
    PngSequence,
    Gif,
    WebpAnimated,
    WebpStatic,
    Spritesheet,
    /// MP4 container with an H.264 video track. Audio (if any) is ignored.
    Video,
}

/// Configuration for a single character/entity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterConfig {
    pub id: String,
    pub name: String,
    pub asset_type: AssetType,
    pub asset_path: String,
    pub x: f32,
    pub y: f32,
    #[serde(default = "default_scale")]
    pub scale: f32,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default = "default_fps")]
    pub fps: f32,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default = "default_true")]
    pub playing: bool,
    #[serde(default)]
    pub z_index: i32,
    /// Whether gravity is active for this entity. Default `false` — entities
    /// stay where the user places them. Toggle at runtime with the `G` key.
    #[serde(default)]
    pub physics_enabled: bool,
    /// Autonomous motion behavior (idle / walk-around / …). Default `Idle`.
    /// `Idle` is also skipped on serialize so the most common case
    /// produces a minimal TOML.
    #[serde(default, skip_serializing_if = "is_idle_behavior")]
    pub behavior: Behavior,
    /// Number of columns in spritesheet grid (only used for Spritesheet type)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spritesheet_columns: Option<u32>,
    /// Number of rows in spritesheet grid (only used for Spritesheet type)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spritesheet_rows: Option<u32>,
    /// Optional override for which monitor this entity belongs to.
    /// `None` (omitted in TOML) means "resolve via centroid hit-test
    /// against the live monitor topology"; `Some("eDP-1")` pins the
    /// entity to that monitor. Stale names fall back to centroid
    /// resolution at runtime with a warning. Backwards compat: every
    /// 0.2 config decodes as `None` and behaves exactly as before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monitor: Option<String>,
    /// Optional easing curve applied to per-frame timing. `None`
    /// (omitted) keeps the 0.2 behaviour (linear). Ignored when the
    /// underlying asset carries per-frame delays from GIF/WebP
    /// metadata — those are authoritative.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub easing: Option<crate::anim::EasingCurve>,
    /// Per-state animation sequences (U.1):
    /// `[characters.animations.walk]` tables keyed by
    /// [`StateId`](crate::animation::StateId). The legacy top-level
    /// `asset_path`/`asset_type` always define the `idle` state — an
    /// `idle` key in this map is ignored with a warning to keep one
    /// unambiguous source. Absent on every pre-0.7 config (and on
    /// every config that never uses states), so old files round-trip
    /// byte-identically.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub animations: std::collections::BTreeMap<crate::animation::StateId, StateSequenceConfig>,
    /// What it says (`crate::phrases`): `[characters.talk]`, absent when
    /// it says nothing.
    #[serde(default, skip_serializing_if = "Talk::is_empty")]
    pub talk: Talk,
    /// When it is there (`crate::hours`): absent for all day.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hours: Option<crate::hours::Hours>,
}

/// A character's own lines (`crate::phrases`): one said when it is poked,
/// and one about every `every_minutes` — 0, only when poked. And the sound
/// a tap makes (`crate::sounds`), a file in the asset library.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Talk {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub phrases: Vec<String>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub every_minutes: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sound: String,
    /// Its bubbles' colour, "#rrggbb" (`crate::speech`); white as it comes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bubble: Option<String>,
    /// Its bubbles' text size, in points; [`crate::speech::TEXT_SIZE`] as
    /// it comes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_size: Option<f32>,
    /// How loud its sounds are, 0 to 1, under the global volume — its tap,
    /// the reminders and focus turns it says, its script's (`crate::audio`).
    /// Full as it comes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<f32>,
}

impl Talk {
    pub fn is_empty(&self) -> bool {
        self.phrases.is_empty()
            && self.every_minutes == 0
            && self.sound.is_empty()
            && self.bubble.is_none()
            && self.text_size.is_none()
            && self.volume.is_none()
    }
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// One animation state's asset source (U.1). A miniature of the
/// legacy per-character asset fields; anything omitted inherits from
/// the character (fps) or the loader defaults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateSequenceConfig {
    pub asset_type: AssetType,
    pub asset_path: String,
    /// Falls back to the character's `fps` when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fps: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spritesheet_columns: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spritesheet_rows: Option<u32>,
}

fn is_idle_behavior(b: &Behavior) -> bool {
    matches!(b, Behavior::Idle)
}

fn default_scale() -> f32 {
    1.0
}
fn default_opacity() -> f32 {
    1.0
}
/// Default animation fps — also the double-click-reset target for the
/// inspector's FPS slider (V.3), hence `pub`.
pub fn default_fps() -> f32 {
    12.0
}
fn default_true() -> bool {
    true
}

/// Clamp a config float into `[min, max]`, substituting `default` for a
/// non-finite value (`NaN` / `±inf`). A hand-edited `config.toml` can
/// carry `scale = nan` or `opacity = inf`; left alone these reach the
/// renderer's transform matrices and the GPU chokes on the NaN.
pub(crate) fn finite_clamp(v: f32, min: f32, max: f32, default: f32) -> f32 {
    if v.is_finite() {
        v.clamp(min, max)
    } else {
        default
    }
}

/// Replace a non-finite coordinate with `default`; positions aren't
/// clamped to a range (entities legitimately sit off-screen) but `NaN` /
/// `inf` would still poison the render path.
fn finite_or(v: f32, default: f32) -> f32 {
    if v.is_finite() {
        v
    } else {
        default
    }
}

impl CharacterConfig {
    /// Coerce the renderer-facing scalars into safe, finite ranges. Run
    /// on load so a hand-edited or corrupt config can't push `NaN`/`inf`
    /// (or absurd magnitudes) into the transform math, physics, or the
    /// animation clock.
    pub(crate) fn sanitize(&mut self) {
        self.scale = finite_clamp(self.scale, 0.1, 5.0, default_scale());
        self.opacity = finite_clamp(self.opacity, 0.0, 1.0, default_opacity());
        self.fps = finite_clamp(self.fps, 0.1, 240.0, default_fps());
        self.x = finite_or(self.x, 0.0);
        self.y = finite_or(self.y, 0.0);
        // The behavior tunables feed the same physics and transform math
        // every tick, so they need the same coercion as the scalars above.
        self.behavior.sanitize();
        crate::phrases::tidy(&mut self.talk);
        if let Some(hours) = self.hours.as_mut() {
            hours.sanitize();
        }
    }
}

/// One overlay window's worth of state. Independent of monitor
/// distribution — a single window can still span monitors or pin to
/// one via the global `monitor_mode`. C.3 ships the data layer; the
/// actual multi-window event loop (one `winit::Window` per entry)
/// lands later. For 0.3 the renderer still uses one window backed by
/// the union of all entities (legacy + windowed) so existing setups
/// behave identically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Stable identifier used by `Activate()` cycling and tray menu
    /// entries. Required because window names can be edited by the
    /// user; ids stay constant. Conventionally lowercase-kebab.
    pub id: String,
    /// User-visible name shown in tray menus and the settings UI.
    pub name: String,
    /// Per-window override of the global monitor distribution. `None`
    /// inherits `GlobalConfig.monitor_mode`. Useful for "main on
    /// every monitor; companion always on the laptop screen" setups.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monitor_mode: Option<MonitorMode>,
    /// Characters belonging to this window. Independent from the
    /// top-level `characters` array (which 0.2 configs use).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub characters: Vec<CharacterConfig>,
}

/// Current config schema version (W.0, 0.9). Bumped only when a change
/// is **not** purely additive — additive fields keep using serde
/// `default` and need no migration. `v1` is every config written before
/// 0.9 (no `version` key at all). See `migrate_table`.
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

/// serde default for [`AppConfig::schema_version`]: a config file with
/// no `version` key predates 0.9, i.e. schema v1.
fn schema_version_legacy() -> u32 {
    1
}

/// Full application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Schema version. **Must be the first field** so it serialises as a
    /// top-level scalar before any `[section]` (TOML forbids a bare key
    /// after a table). Absent in pre-0.9 files → treated as v1 and
    /// migrated on load; always re-written as [`CURRENT_SCHEMA_VERSION`].
    #[serde(rename = "version", default = "schema_version_legacy")]
    pub schema_version: u32,
    // Every table and key a person might leave out has a default. These
    // three and the `[global]` table itself used not to, so a hand-written
    // config naming only the settings it cared about failed to decode —
    // and at startup that means backed up and replaced by the demo scene.
    #[serde(default)]
    pub global: GlobalConfig,
    #[serde(rename = "characters", default)]
    pub characters: Vec<CharacterConfig>,
    /// Multi-window roster. Empty (or absent) means "legacy single
    /// overlay backed by `characters` above" — 0.2 configs decode
    /// identically with no migration. When non-empty, `characters`
    /// continues to work and is treated as the first implicit window
    /// so we never silently drop user entities.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub windows: Vec<WindowConfig>,
    /// Sprite groups (C.8). Empty for the legacy/0.2 path. Each
    /// `GroupConfig` carries a stable id and a list of member entity
    /// ids; composition rules (offset, scale, visibility) are
    /// applied at render / visibility-resolve time.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<crate::group::GroupConfig>,
    /// What characters say every so often (`crate::reminders`, 1.5).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reminders: Vec<crate::reminders::ReminderConfig>,
    /// Saved scenes to switch to at set times (`crate::schedule`, 1.5).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scene_schedule: Vec<crate::schedule::ScheduleRule>,
    /// Rebindable keyboard shortcuts (D.1). Defaults match the 0.3
    /// hard-coded set, so existing configs decode without losing any
    /// binding and pre-D configs without a `[keybindings]` section
    /// still behave exactly as before.
    #[serde(default)]
    pub keybindings: KeyBindings,
    /// Open/closed state of every persistable collapse section (D.2).
    /// Pre-0.4 configs without the `[collapse_state]` table fall back
    /// to the design-system defaults defined in
    /// `CollapseState::default()`.
    #[serde(default)]
    pub collapse_state: CollapseState,
    /// What of the file did not read, kept to be written back (1.5).
    #[serde(skip)]
    pub set_aside: SetAside,
    /// Forward-compatibility catch-all (W.0). Top-level `[section]`
    /// blocks this build doesn't model — e.g. a section added by a
    /// newer animaEngine — are captured here on load and written back
    /// verbatim on save, instead of being silently dropped. **Must be
    /// the last field** so unknown tables serialise after the known
    /// ones. Preserves unknown *sections*, not unknown bare keys at the
    /// document root (TOML ordering makes that unrepresentable here).
    #[serde(flatten, default)]
    pub extra: toml::Table,
}

impl AppConfig {
    /// Returns the full list of windows, including the implicit legacy
    /// window backed by top-level `characters` when present.
    ///
    /// Lookup precedence:
    /// 1. If `windows` is non-empty, return clones of those entries.
    /// 2. Otherwise synthesise one window named "default" carrying the
    ///    top-level `characters`. This is the 0.2 compat path.
    ///
    /// Used by the tray submenu, the settings UI window picker, and
    /// the D-Bus `Activate()` cycle. The actual render path can still
    /// flatten this into a single scene until full multi-window
    /// dispatch lands.
    pub fn windows_normalised(&self) -> Vec<WindowConfig> {
        if !self.windows.is_empty() {
            return self.windows.clone();
        }
        vec![WindowConfig {
            id: "default".to_string(),
            name: "Main".to_string(),
            monitor_mode: None,
            characters: self.characters.clone(),
        }]
    }

    /// Coerce every character's renderer-facing scalars into safe, finite
    /// ranges (see [`CharacterConfig::sanitize`]). Called once on load so
    /// a hand-edited or corrupt config can't feed `NaN`/`inf` to the GPU.
    pub(crate) fn sanitize(&mut self) {
        self.global.reminder_sound.sanitize();
        self.global.sound_volume = finite_clamp(self.global.sound_volume, 0.0, 1.0, 1.0);
        for c in &mut self.characters {
            c.sanitize();
        }
        for w in &mut self.windows {
            for c in &mut w.characters {
                c.sanitize();
            }
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            global: GlobalConfig::default(),
            characters: vec![
                CharacterConfig {
                    id: "ghost".to_string(),
                    name: "Ghost Demo".to_string(),
                    asset_type: AssetType::PngSequence,
                    asset_path: "assets/demo/ghost".to_string(),
                    x: 200.0,
                    y: 300.0,
                    scale: 1.0,
                    opacity: 0.9,
                    fps: 10.0,
                    visible: true,
                    playing: true,
                    z_index: 10,
                    physics_enabled: false,
                    behavior: Behavior::Idle,
                    spritesheet_columns: None,
                    spritesheet_rows: None,
                    monitor: None,
                    easing: None,
                    animations: std::collections::BTreeMap::new(),
                    talk: Default::default(),
                    hours: None,
                },
                CharacterConfig {
                    id: "slime".to_string(),
                    name: "Slime Demo".to_string(),
                    asset_type: AssetType::PngSequence,
                    asset_path: "assets/demo/slime".to_string(),
                    x: 600.0,
                    y: 400.0,
                    scale: 1.0,
                    opacity: 1.0,
                    fps: 8.0,
                    visible: true,
                    playing: true,
                    z_index: 20,
                    physics_enabled: false,
                    behavior: Behavior::Idle,
                    spritesheet_columns: None,
                    spritesheet_rows: None,
                    monitor: None,
                    easing: None,
                    animations: std::collections::BTreeMap::new(),
                    talk: Default::default(),
                    hours: None,
                },
                CharacterConfig {
                    id: "heart".to_string(),
                    name: "Heart Demo".to_string(),
                    asset_type: AssetType::PngSequence,
                    asset_path: "assets/demo/heart".to_string(),
                    x: 1000.0,
                    y: 200.0,
                    scale: 1.0,
                    opacity: 1.0,
                    fps: 8.0,
                    visible: true,
                    playing: true,
                    z_index: 30,
                    physics_enabled: false,
                    behavior: Behavior::Idle,
                    spritesheet_columns: None,
                    spritesheet_rows: None,
                    monitor: None,
                    easing: None,
                    animations: std::collections::BTreeMap::new(),
                    talk: Default::default(),
                    hours: None,
                },
                CharacterConfig {
                    id: "star".to_string(),
                    name: "Star Demo".to_string(),
                    asset_type: AssetType::PngSequence,
                    asset_path: "assets/demo/star".to_string(),
                    x: 1300.0,
                    y: 450.0,
                    scale: 1.0,
                    opacity: 1.0,
                    fps: 8.0,
                    visible: true,
                    playing: true,
                    z_index: 40,
                    physics_enabled: false,
                    behavior: Behavior::Idle,
                    spritesheet_columns: None,
                    spritesheet_rows: None,
                    monitor: None,
                    easing: None,
                    animations: std::collections::BTreeMap::new(),
                    talk: Default::default(),
                    hours: None,
                },
                CharacterConfig {
                    id: "cat".to_string(),
                    name: "Cat Demo".to_string(),
                    asset_type: AssetType::PngSequence,
                    asset_path: "assets/demo/cat".to_string(),
                    x: 900.0,
                    y: 600.0,
                    scale: 1.0,
                    opacity: 1.0,
                    fps: 8.0,
                    visible: true,
                    playing: true,
                    z_index: 50,
                    physics_enabled: false,
                    behavior: Behavior::Idle,
                    spritesheet_columns: None,
                    spritesheet_rows: None,
                    monitor: None,
                    easing: None,
                    animations: std::collections::BTreeMap::new(),
                    talk: Default::default(),
                    hours: None,
                },
            ],
            // 0.3 fresh installs use the legacy single-window shape;
            // multi-window is opt-in via UI / hand-edit (data layer ready
            // in C.3, render-side dispatch coming in 0.4).
            windows: vec![],
            groups: vec![],
            reminders: vec![],
            scene_schedule: vec![],
            keybindings: KeyBindings::default(),
            collapse_state: CollapseState::default(),
            schema_version: CURRENT_SCHEMA_VERSION,
            set_aside: SetAside::default(),
            extra: toml::Table::new(),
        }
    }
}

/// What of `config.toml` this build could not read — written by a newer
/// animaEngine, or mistyped — kept to be written back as it was (1.5).
///
/// One character, group or setting that did not read used to cost the
/// whole file: the app started with the default scene. Every release
/// adds behaviors, settings and choices an older one does not know, so
/// opening a newer version's config in an older one did just that. Now
/// the app goes on without those parts — a character set aside is not
/// shown, a setting is at its default — and saves them back where they
/// were, for the version that wrote them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SetAside {
    /// `[global]` keys whose values did not read: the key, the value as
    /// written, and what the setting was at load — `None` when it is not
    /// written out — to tell whether it has been changed here since.
    global: Vec<(String, toml::Value, Option<toml::Value>)>,
    /// List entries that did not read: which list, and the entry.
    entries: Vec<(SetAsideList, toml::Value)>,
}

/// Where a [`SetAside`] entry came from.
#[derive(Debug, Clone, PartialEq)]
enum SetAsideList {
    /// One of [`SET_ASIDE_LISTS`].
    Top(&'static str),
    /// The characters of the window with this id.
    WindowCharacters(String),
}

/// The lists whose entries are read one by one when the whole file does
/// not read.
const SET_ASIDE_LISTS: [&str; 4] = ["characters", "groups", "reminders", "scene_schedule"];

/// Whether one entry of `list` reads, and if not, why.
fn entry_reads(list: &str, entry: &toml::Value) -> std::result::Result<(), String> {
    let entry = entry.clone();
    match list {
        "groups" => entry.try_into::<crate::group::GroupConfig>().map(drop),
        "reminders" => entry
            .try_into::<crate::reminders::ReminderConfig>()
            .map(drop),
        "scene_schedule" => entry.try_into::<crate::schedule::ScheduleRule>().map(drop),
        _ => entry.try_into::<CharacterConfig>().map(drop),
    }
    .map_err(|e| e.to_string().trim().replace('\n', " "))
}

/// A name for an entry in the log: its `name`, else its `id`.
fn entry_label(entry: &toml::Value) -> String {
    ["name", "id"]
        .iter()
        .find_map(|k| entry.get(k).and_then(toml::Value::as_str))
        .map(|s| format!("\"{s}\""))
        .unwrap_or_else(|| "one".to_string())
}

impl SetAside {
    /// Whether anything was set aside.
    pub fn is_empty(&self) -> bool {
        self.global.is_empty() && self.entries.is_empty()
    }

    /// Take out of `table` what does not read: list entries one by one,
    /// and `[global]` keys one by one. What is left is for the decode to
    /// try again.
    fn take_unreadable(table: &mut toml::Table) -> Self {
        let mut aside = Self::default();
        let mut sift = |list: SetAsideList, name: &str, items: &mut Vec<toml::Value>| {
            items.retain(|entry| match entry_reads(name, entry) {
                Ok(()) => true,
                Err(e) => {
                    tracing::warn!(
                        "config: {} in `{name}` does not read ({e}); set aside, and kept in the file",
                        entry_label(entry)
                    );
                    aside.entries.push((list.clone(), entry.clone()));
                    false
                }
            });
        };
        for name in SET_ASIDE_LISTS {
            if let Some(toml::Value::Array(items)) = table.get_mut(name) {
                sift(SetAsideList::Top(name), name, items);
            }
        }
        if let Some(toml::Value::Array(windows)) = table.get_mut("windows") {
            for window in windows.iter_mut().filter_map(toml::Value::as_table_mut) {
                let id = window
                    .get("id")
                    .and_then(toml::Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                if let Some(toml::Value::Array(items)) = window.get_mut("characters") {
                    sift(SetAsideList::WindowCharacters(id), "characters", items);
                }
            }
        }
        // Every `[global]` setting has a default, so each reads on its
        // own or does not.
        if let Some(toml::Value::Table(global)) = table.get_mut("global") {
            if toml::Value::Table(global.clone())
                .try_into::<GlobalConfig>()
                .is_err()
            {
                let keys: Vec<String> = global.keys().cloned().collect();
                for key in keys {
                    let alone = toml::Table::from_iter([(key.clone(), global[&key].clone())]);
                    if let Err(e) = toml::Value::Table(alone).try_into::<GlobalConfig>() {
                        let e = e.to_string().trim().replace('\n', " ");
                        tracing::warn!(
                            "config: `{key}` in [global] does not read ({e}); at its default, and kept in the file"
                        );
                        if let Some(value) = global.remove(&key) {
                            aside.global.push((key, value, None));
                        }
                    }
                }
            }
        }
        aside
    }

    /// Note what each setting set aside is now, to be compared on save.
    fn note_settings(&mut self, global: &GlobalConfig) {
        let now = toml::Value::try_from(global).ok();
        for (key, _, at_load) in &mut self.global {
            *at_load = now.as_ref().and_then(|t| t.get(key.as_str())).cloned();
        }
    }

    /// Put back into `table`, a config as about to be saved, what was set
    /// aside: entries at the end of their lists — a window's only if that
    /// window is still there — and settings unless changed here since.
    fn write_back(&self, table: &mut toml::Table) {
        for (list, entry) in &self.entries {
            let items = match list {
                SetAsideList::Top(name) => Some(
                    table
                        .entry(name.to_string())
                        .or_insert_with(|| toml::Value::Array(Vec::new())),
                ),
                SetAsideList::WindowCharacters(id) => table
                    .get_mut("windows")
                    .and_then(toml::Value::as_array_mut)
                    .and_then(|windows| {
                        windows.iter_mut().find(|w| {
                            w.get("id").and_then(toml::Value::as_str) == Some(id.as_str())
                        })
                    })
                    .and_then(toml::Value::as_table_mut)
                    .map(|window| {
                        window
                            .entry("characters")
                            .or_insert_with(|| toml::Value::Array(Vec::new()))
                    }),
            };
            if let Some(toml::Value::Array(items)) = items {
                items.push(entry.clone());
            }
        }
        if let Some(toml::Value::Table(global)) = table.get_mut("global") {
            for (key, written, at_load) in &self.global {
                if global.get(key) == at_load.as_ref() {
                    global.insert(key.clone(), written.clone());
                }
            }
        }
    }
}

/// Read the `version` key from a parsed config table.
///
/// - absent → `1` (every config written before 0.9 is schema v1);
/// - a positive integer → that version;
/// - anything else (string, float, negative, zero) → treated as
///   [`CURRENT_SCHEMA_VERSION`] with a warning. A malformed version
///   field must never be a reason to wipe a user's config — we'd
///   rather skip migration than run the wrong one.
fn detect_schema_version(table: &toml::Table) -> u32 {
    match table.get("version") {
        None => 1,
        // Bounded conversion. A value that doesn't fit u32 (e.g.
        // `version = 4294967296`) must never wrap to a bogus low
        // version and then drive migration off a cliff — the old
        // `*n as u32` turned 2^32 into 0 and `migrate_table` hit
        // `unreachable!`, so a hand-typed number could *panic* the
        // loader. `try_from` rejects overflow and negatives; the
        // `>= 1` guard rejects zero. All of them fall through to the
        // "treat as current, skip migration" contract below.
        Some(toml::Value::Integer(n)) => match u32::try_from(*n) {
            Ok(v) if v >= 1 => v,
            _ => {
                tracing::warn!(
                    "config `version` = {n} is out of range; treating as current ({CURRENT_SCHEMA_VERSION}) and skipping migration"
                );
                CURRENT_SCHEMA_VERSION
            }
        },
        Some(other) => {
            tracing::warn!(
                "config `version` is malformed ({other:?}); treating as current ({CURRENT_SCHEMA_VERSION}) and skipping migration"
            );
            CURRENT_SCHEMA_VERSION
        }
    }
}

/// Bring a parsed config table up to [`CURRENT_SCHEMA_VERSION`] in
/// place, then stamp the version. Pure: no IO, no deserialisation.
/// Returns the version it started from (for backup naming / logging).
///
/// The chain is ordered and idempotent: re-running it on an
/// already-current table is a no-op apart from re-stamping the same
/// version.
fn migrate_table(table: &mut toml::Table) -> u32 {
    let from = detect_schema_version(table);
    if from > CURRENT_SCHEMA_VERSION {
        // Config written by a newer build than this one. There is no
        // downgrade path, and re-stamping it to CURRENT would silently
        // strip fields this build can't model. Leave the table — version
        // marker and all — untouched and let `load` deserialise what it
        // understands without persisting, so the richer on-disk file is
        // never downgraded just by being opened by an older binary.
        tracing::warn!(
            "config schema v{from} is newer than this build (v{CURRENT_SCHEMA_VERSION}); \
             loading read-only and leaving the file untouched"
        );
        return from;
    }
    let mut v = from;
    while v < CURRENT_SCHEMA_VERSION {
        match v {
            1 => migrate_v1_v2(table),
            // Unreachable while CURRENT_SCHEMA_VERSION == 2; the arm is
            // here so adding a v2→v3 migration is a one-line change and
            // a missing arm is a loud panic, not a silent skip.
            other => unreachable!("no migration registered for schema v{other}"),
        }
        v += 1;
    }
    table.insert(
        "version".to_string(),
        toml::Value::Integer(CURRENT_SCHEMA_VERSION as i64),
    );
    from
}

/// v1 → v2: structurally identical. Every config change from 0.2
/// through 0.8 was additive and is handled by serde `default`s, so
/// there is nothing to rewrite — this migration only exists to stamp
/// the version and to prove the chain runs end to end. The first
/// genuinely non-additive change (a renamed/removed/retyped key) lands
/// here as real rewrite logic.
fn migrate_v1_v2(_table: &mut toml::Table) {}

/// Copy an existing-but-unreadable `config.toml` aside to
/// `config.toml.bak-corrupt` so the default-config save that follows a
/// failed load can never destroy the user's only copy. Best-effort and
/// idempotent per launch (a repeat corruption overwrites the previous
/// corrupt backup — both were unreadable, and the good-config safety
/// nets are the migration `.bak-v<n>` and crash-recovery snapshots).
/// Extracted from `AppConfig::load` so the decision is unit-testable
/// without redirecting the real config path.
/// Set when startup found a config it could not read and loaded the
/// defaults instead (the original is kept as `config.toml.bak-corrupt`).
static LOAD_FELL_BACK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Whether this run's config was unreadable and replaced by the defaults.
/// Each backend turns it into a banner at startup.
pub fn loaded_defaults_over_unreadable_config() -> bool {
    LOAD_FELL_BACK.load(std::sync::atomic::Ordering::Relaxed)
}

fn backup_unreadable_config(path: &Path) {
    let backup = path.with_extension("toml.bak-corrupt");
    match fs::copy(path, &backup) {
        Ok(_) => tracing::warn!(
            "Config was unreadable — backed the original up to {} before writing defaults",
            crate::drop_validate::redact_path(&backup)
        ),
        Err(e) => tracing::warn!("Could not back up unreadable config: {e}"),
    }
}

impl AppConfig {
    /// Get the config file path: ~/.config/animaengine/config.toml
    pub fn config_path() -> PathBuf {
        if let Some(proj_dirs) = directories::ProjectDirs::from("", "", "animaEngine") {
            return proj_dirs.config_dir().to_path_buf().join("config.toml");
        }
        // Fallback when XDG resolution fails (minimal containers, broken
        // env, etc.). `fallback_scoped_dir` prefers $XDG_RUNTIME_DIR
        // (0700 + uid-owned by spec) and only then a verified tmpdir —
        // a plain /tmp subdir could be pre-created by another local
        // user, landing our atomic writes in a directory they own.
        crate::util::fallback_scoped_dir("").join("config.toml")
    }

    /// Parse → migrate (in memory) → decode → cap → sanitise a config
    /// from a TOML string. **Pure**: no disk writes, no backups, no
    /// default fallback. Returns the decoded config and the on-disk
    /// schema version it came from (so `load` can decide whether to
    /// back up + persist a migration). `migrate_table` mutates the
    /// in-memory table only — the file on disk is never touched here,
    /// which is what lets both the startup `load` and the read-only
    /// `try_reload` share this without one leaking writes into the other.
    fn decode_config_str(contents: &str) -> std::result::Result<(Self, u32), String> {
        let mut table = contents
            .parse::<toml::Table>()
            .map_err(|e| format!("parse error: {e}"))?;
        let from = detect_schema_version(&table);
        migrate_table(&mut table);
        // Deserialise from the migrated Value, not a re-serialised
        // string: a top-level scalar (`version`) sorted among the section
        // keys would trip TOML's "value after table" rule on the way back
        // out. `try_into` doesn't care about order.
        let mut config: Self = match toml::Value::Table(table.clone()).try_into() {
            Ok(config) => config,
            // Set aside what does not read (`SetAside`), and try again
            // with the rest; when that does not read either, the file is
            // unreadable as before, and says why.
            Err(first) => {
                let aside = SetAside::take_unreadable(&mut table);
                if aside.is_empty() {
                    return Err(format!("decode error: {first}"));
                }
                let mut config: Self = toml::Value::Table(table)
                    .try_into()
                    .map_err(|_| format!("decode error: {first}"))?;
                config.set_aside = aside;
                config
            }
        };
        if config.characters.len() > MAX_ENTITIES {
            tracing::warn!(
                "Config has {} characters, capping at {} to prevent resource exhaustion",
                config.characters.len(),
                MAX_ENTITIES
            );
            config.characters.truncate(MAX_ENTITIES);
        }
        // Coerce NaN/inf/out-of-range scalars before anything reaches the
        // renderer or physics.
        config.sanitize();
        let global = config.global.clone();
        config.set_aside.note_settings(&global);
        Ok((config, from))
    }

    /// This config as its file says it: what is set aside
    /// ([`SetAside`]) written back where it was.
    pub fn to_toml_string(&self) -> std::result::Result<String, toml::ser::Error> {
        if self.set_aside.is_empty() {
            return toml::to_string_pretty(self);
        }
        let mut table = toml::Table::try_from(self)?;
        self.set_aside.write_back(&mut table);
        toml::to_string_pretty(&table)
    }

    /// Read-only config load for **hot-reload**. Reads and decodes the
    /// config file without ever writing to disk — no default fallback,
    /// no backup, no migration persist. A transient partial write (an
    /// editor doing truncate-then-write) or a hand-edit typo therefore
    /// returns `Err` and leaves both the file and the running scene
    /// untouched, instead of the startup `load` behaviour of overwriting
    /// the user's config with defaults. The caller keeps the last valid
    /// scene and surfaces the error.
    pub fn try_reload() -> std::result::Result<Self, String> {
        let path = Self::config_path();
        let contents =
            crate::util::read_to_string_capped(&path, crate::constants::MAX_CONFIG_BYTES)
                .map_err(|e| format!("read error: {e}"))?;
        let (config, _from) = Self::decode_config_str(&contents)?;
        Ok(config)
    }

    /// Load config from disk, or create default if not found
    pub fn load() -> Self {
        let path = Self::config_path();
        tracing::info!("Config path: {}", crate::drop_validate::redact_path(&path));
        tracing::debug!("Config path (full): {}", path.display());

        if path.exists() {
            match crate::util::read_to_string_capped(&path, crate::constants::MAX_CONFIG_BYTES) {
                Ok(contents) => match Self::decode_config_str(&contents) {
                    Ok((config, from)) => {
                        // Behind the current schema: copy the file aside
                        // *before* the persist below overwrites it, so a
                        // migration bug can never be why a user loses
                        // their config, then write the migrated form so
                        // the next launch skips re-migrating.
                        if from < CURRENT_SCHEMA_VERSION {
                            let backup = path.with_extension(format!("toml.bak-v{from}"));
                            match fs::copy(&path, &backup) {
                                Ok(_) => tracing::info!(
                                    "Migrating config v{from} → v{CURRENT_SCHEMA_VERSION}; backed up original to {}",
                                    crate::drop_validate::redact_path(&backup)
                                ),
                                Err(e) => tracing::warn!(
                                    "Could not write migration backup {}: {e}",
                                    crate::drop_validate::redact_path(&backup)
                                ),
                            }
                            if let Err(e) = config.save() {
                                tracing::warn!("Could not persist migrated config: {e}");
                            }
                        }
                        tracing::info!(
                            "Loaded config with {} characters (schema v{})",
                            config.characters.len(),
                            config.schema_version
                        );
                        return config;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load config: {e}. Using defaults.");
                    }
                },
                Err(e) => {
                    tracing::warn!("Failed to read config: {}. Using defaults.", e);
                }
            }
        } else {
            tracing::info!("Config not found, creating default config");
        }

        // Reaching here with the file still on disk means it exists but
        // couldn't be read/parsed/decoded — copy it aside before the
        // default save below overwrites it. Same never-lose-the-user's-
        // file rule as the migration backup above: a hand-edit typo
        // (hot-reload invites hand edits) must not cost the whole scene.
        // A genuinely first run — no config file at all — has no previous
        // version to catch up on, so the "what's new" panel is meaningless
        // to it: the three-step onboarding tour is the right first-run
        // surface. Stamping the current anchor keeps that panel for
        // *upgraders*, who are the only ones it is written for. A config
        // that exists but failed to parse is not a fresh install, so it
        // still gets the panel.
        let fresh_install = !path.exists();
        if path.exists() {
            backup_unreadable_config(&path);
            // The UI says so on screen (Warning::ConfigUnreadable): the log
            // line alone left a user looking at the demo scene in place of
            // their own, with no idea why.
            LOAD_FELL_BACK.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        let mut config = AppConfig::default();
        if fresh_install {
            config.global.last_seen_whats_new =
                Some(crate::constants::WHATS_NEW_VERSION.to_string());
        }
        if let Err(e) = config.save() {
            tracing::warn!("Failed to save default config: {}", e);
        }
        config
    }

    /// Take in what the running scene holds — its characters, its groups
    /// and the playback switch — before a save. Both loops call it. The
    /// groups were left out until 1.5, when the app began to make them:
    /// they had only ever changed by hand in this file.
    pub fn take_scene(&mut self, scene: &crate::scene::Scene) {
        self.characters = scene.to_character_configs();
        self.groups = scene.groups.clone();
        self.global.playback_enabled = scene.global_playing;
    }

    /// Save config to disk **atomically** — writes to a temp sibling
    /// then renames over the target. A crash mid-save can no longer
    /// leave a truncated `config.toml`.
    #[tracing::instrument(skip(self), fields(n_chars = self.characters.len()))]
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        let toml_string = self.to_toml_string()?;
        crate::util::atomic_write_bytes(&path, toml_string.as_bytes())?;
        tracing::info!(
            "Config saved to {}",
            crate::drop_validate::redact_path(&path)
        );
        tracing::debug!("Config saved (full path): {}", path.display());
        Ok(())
    }

    /// Resolve an asset path relative to the executable or current directory.
    /// Supports:
    /// - Absolute paths (returned as-is)
    /// - `~/` expansion to home directory (bare `~` too; `~user` syntax
    ///   is NOT supported and passes through untouched — expanding it
    ///   with our own `$HOME` would silently build a path inside the
    ///   wrong home directory)
    /// - Relative paths (checked against exe dir, then cwd)
    pub fn resolve_asset_path(asset_path: &str) -> PathBuf {
        let asset_path = if asset_path == "~" || asset_path.starts_with("~/") {
            if let Ok(home) = std::env::var("HOME") {
                asset_path.replacen('~', &home, 1)
            } else {
                asset_path.to_string()
            }
        } else {
            asset_path.to_string()
        };

        let path = Path::new(&asset_path);
        if path.is_absolute() {
            return path.to_path_buf();
        }

        // Try relative to current executable
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let resolved = exe_dir.join(&asset_path);
                if resolved.exists() {
                    return resolved;
                }
            }
        }

        // Try relative to current directory
        let cwd_path = PathBuf::from(&asset_path);
        if cwd_path.exists() {
            return cwd_path;
        }

        // Return as-is, let caller handle missing path
        PathBuf::from(&asset_path)
    }

    // Asset-type detection lives in animation::loader::detect_asset_type
    // — see comments there for the canonical extension → AssetType table
    // (it covers JPEG and MP4 / MOV / M4V too).
}

#[cfg(test)]
mod resolve_path_tests {
    use super::*;

    // `~` expansion reads `$HOME`, which unix guarantees and Windows does
    // not set at all — so both assertions below are unix-shaped. Off unix
    // `resolve_asset_path` takes its existing "no HOME" branch and passes
    // the path through untouched; whether Windows should expand `~`
    // against `%USERPROFILE%` instead is a C4 decision, not a C2b one.
    #[test]
    #[cfg(unix)]
    fn tilde_slash_expands_to_home() {
        let home = std::env::var("HOME").expect("HOME set in test env");
        let resolved = AppConfig::resolve_asset_path("~/assets/x.png");
        assert!(
            resolved.starts_with(&home),
            "expected {resolved:?} under {home}"
        );
    }

    #[test]
    #[cfg(unix)]
    fn tilde_user_syntax_passes_through_unexpanded() {
        // `~alex/x` must NOT become `$HOME + "alex/x"` — that silently
        // builds a path inside the wrong home. We don't support the
        // user-tilde form at all; it passes through as a literal.
        let home = std::env::var("HOME").expect("HOME set in test env");
        let resolved = AppConfig::resolve_asset_path("~nobody/assets/x.png");
        assert!(
            !resolved.starts_with(&home),
            "user-tilde must not expand against our HOME, got {resolved:?}"
        );
    }

    #[test]
    fn interior_tilde_untouched() {
        let resolved = AppConfig::resolve_asset_path("/data/backup~old/x.png");
        assert_eq!(resolved, PathBuf::from("/data/backup~old/x.png"));
    }
}

#[cfg(test)]
mod animations_schema_tests {
    use super::*;
    use crate::animation::StateId;

    #[test]
    fn legacy_character_decodes_with_empty_animations() {
        let toml_str = r#"
            id = "slime"
            name = "Slime"
            asset_type = "png_sequence"
            asset_path = "assets/demo/slime"
            x = 1.0
            y = 2.0
        "#;
        let cfg: CharacterConfig = toml::from_str(toml_str).unwrap();
        assert!(cfg.animations.is_empty());
        // Round-trip must not introduce the key.
        let out = toml::to_string(&cfg).unwrap();
        assert!(!out.contains("animations"), "got: {out}");
    }

    #[test]
    fn state_tables_round_trip() {
        let toml_str = r#"
            id = "shime"
            name = "Shime"
            asset_type = "png_sequence"
            asset_path = "imported/shime/idle"
            x = 0.0
            y = 0.0

            [animations.walk]
            asset_type = "png_sequence"
            asset_path = "imported/shime/walk"
            fps = 10.0

            [animations.fall]
            asset_type = "gif"
            asset_path = "imported/shime/fall.gif"
        "#;
        let cfg: CharacterConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.animations.len(), 2);
        assert_eq!(cfg.animations[&StateId::Walk].fps, Some(10.0));
        assert!(cfg.animations[&StateId::Fall].fps.is_none());

        let out = toml::to_string(&cfg).unwrap();
        let back: CharacterConfig = toml::from_str(&out).unwrap();
        assert_eq!(back.animations.len(), 2);
        assert_eq!(
            back.animations[&StateId::Walk].asset_path,
            "imported/shime/walk"
        );
    }

    #[test]
    fn unknown_state_key_is_rejected() {
        // Closed StateId enum: a typo'd state name must fail loudly at
        // parse, not silently vanish.
        let toml_str = r#"
            id = "x"
            name = "x"
            asset_type = "gif"
            asset_path = "a.gif"
            x = 0.0
            y = 0.0

            [animations.wlak]
            asset_type = "gif"
            asset_path = "w.gif"
        "#;
        assert!(toml::from_str::<CharacterConfig>(toml_str).is_err());
    }
}

#[cfg(test)]
mod windows_tests {
    use super::*;
    use crate::behavior::Behavior;

    fn empty_char(id: &str) -> CharacterConfig {
        CharacterConfig {
            id: id.into(),
            name: id.into(),
            asset_type: AssetType::PngStatic,
            asset_path: "/dev/null".into(),
            x: 0.0,
            y: 0.0,
            scale: 1.0,
            opacity: 1.0,
            fps: 1.0,
            visible: true,
            playing: true,
            z_index: 0,
            physics_enabled: false,
            behavior: Behavior::Idle,
            spritesheet_columns: None,
            spritesheet_rows: None,
            monitor: None,
            easing: None,
            animations: std::collections::BTreeMap::new(),
            talk: Default::default(),
            hours: None,
        }
    }

    #[test]
    fn legacy_config_synthesises_default_window() {
        let cfg = AppConfig {
            global: GlobalConfig::default(),
            characters: vec![empty_char("ghost")],
            windows: vec![],
            groups: vec![],
            reminders: vec![],
            scene_schedule: vec![],
            keybindings: KeyBindings::default(),
            collapse_state: CollapseState::default(),
            schema_version: CURRENT_SCHEMA_VERSION,
            set_aside: SetAside::default(),
            extra: toml::Table::new(),
        };
        let ws = cfg.windows_normalised();
        assert_eq!(ws.len(), 1);
        assert_eq!(ws[0].id, "default");
        assert_eq!(ws[0].name, "Main");
        assert_eq!(ws[0].characters.len(), 1);
        assert_eq!(ws[0].characters[0].id, "ghost");
    }

    #[test]
    fn explicit_windows_short_circuit_legacy_path() {
        let cfg = AppConfig {
            global: GlobalConfig::default(),
            characters: vec![empty_char("ghost")],
            groups: vec![],
            reminders: vec![],
            scene_schedule: vec![],
            windows: vec![
                WindowConfig {
                    id: "main".into(),
                    name: "Main".into(),
                    monitor_mode: None,
                    characters: vec![empty_char("slime")],
                },
                WindowConfig {
                    id: "side".into(),
                    name: "Companion".into(),
                    monitor_mode: None,
                    characters: vec![],
                },
            ],
            keybindings: KeyBindings::default(),
            collapse_state: CollapseState::default(),
            schema_version: CURRENT_SCHEMA_VERSION,
            set_aside: SetAside::default(),
            extra: toml::Table::new(),
        };
        let ws = cfg.windows_normalised();
        assert_eq!(ws.len(), 2);
        // Top-level `characters` ignored when explicit windows present
        // (preserving the user's deliberate distribution).
        assert!(ws
            .iter()
            .all(|w| w.characters.iter().all(|c| c.id != "ghost")));
        assert_eq!(ws[0].id, "main");
        assert_eq!(ws[1].id, "side");
    }

    #[test]
    fn empty_legacy_synthesises_window_with_no_characters() {
        let cfg = AppConfig {
            global: GlobalConfig::default(),
            characters: vec![],
            windows: vec![],
            groups: vec![],
            reminders: vec![],
            scene_schedule: vec![],
            keybindings: KeyBindings::default(),
            collapse_state: CollapseState::default(),
            schema_version: CURRENT_SCHEMA_VERSION,
            set_aside: SetAside::default(),
            extra: toml::Table::new(),
        };
        let ws = cfg.windows_normalised();
        assert_eq!(ws.len(), 1);
        assert!(ws[0].characters.is_empty());
    }

    #[test]
    fn window_config_round_trips_through_toml() {
        let w = WindowConfig {
            id: "side".into(),
            name: "Companion".into(),
            monitor_mode: Some(MonitorMode::Single {
                name: "HDMI-A-1".into(),
            }),
            characters: vec![empty_char("cat")],
        };
        let toml_str = toml::to_string(&w).expect("serialize");
        let back: WindowConfig = toml::from_str(&toml_str).expect("deserialize");
        assert_eq!(back.id, "side");
        assert_eq!(back.characters.len(), 1);
        matches!(
            back.monitor_mode,
            Some(MonitorMode::Single { ref name }) if name == "HDMI-A-1"
        );
    }

    /// A pre-0.3 TOML must decode cleanly into the new struct shape:
    /// no `windows` table, the field defaults to `vec![]`, and
    /// `windows_normalised` synthesises the legacy default.
    #[test]
    fn pre_0_3_toml_without_windows_field_decodes() {
        let toml_str = r#"
            [global]
            always_on_top = true
            transparent = true
            playback_enabled = true
            window_width = 0
            window_height = 0

            [[characters]]
            id = "ghost"
            name = "Ghost"
            asset_type = "png_static"
            asset_path = "/tmp/g.png"
            x = 0.0
            y = 0.0
        "#;
        let cfg: AppConfig = toml::from_str(toml_str).expect("decode 0.2 config");
        assert!(cfg.windows.is_empty());
        let ws = cfg.windows_normalised();
        assert_eq!(ws.len(), 1);
        assert_eq!(ws[0].characters.len(), 1);
    }
}

// ── Schema versioning + migration (W.0, 0.9) ─────────────────────────
#[cfg(test)]
mod migration_tests {
    use super::*;

    fn table(s: &str) -> toml::Table {
        s.parse().expect("parse toml")
    }

    #[test]
    fn detects_absent_version_as_v1() {
        assert_eq!(detect_schema_version(&table("a = 1")), 1);
    }

    #[test]
    fn detects_explicit_version() {
        assert_eq!(detect_schema_version(&table("version = 2")), 2);
    }

    #[test]
    fn malformed_version_is_treated_as_current_not_destroyed() {
        // A string, a float, zero and a negative all map to CURRENT so
        // we never run a migration off a garbage version field.
        for bad in [
            "version = \"two\"",
            "version = 1.5",
            "version = 0",
            "version = -3",
        ] {
            assert_eq!(
                detect_schema_version(&table(bad)),
                CURRENT_SCHEMA_VERSION,
                "input: {bad}"
            );
        }
    }

    #[test]
    fn migrate_stamps_current_version() {
        let mut t = table("a = 1");
        let from = migrate_table(&mut t);
        assert_eq!(from, 1, "started from implicit v1");
        assert_eq!(
            t.get("version"),
            Some(&toml::Value::Integer(CURRENT_SCHEMA_VERSION as i64))
        );
    }

    #[test]
    fn migrate_is_idempotent() {
        let mut t = table("a = 1");
        migrate_table(&mut t);
        let snapshot = t.clone();
        // Second run starts from CURRENT, does no work, re-stamps same.
        let from = migrate_table(&mut t);
        assert_eq!(from, CURRENT_SCHEMA_VERSION);
        assert_eq!(t, snapshot, "migrating twice equals migrating once");
    }

    #[test]
    fn out_of_range_version_does_not_panic_and_loads_as_current() {
        // Regression: `version = 4294967296` (u32::MAX + 1) used to wrap
        // to 0 via `as u32`, then `migrate_table` hit `unreachable!` and
        // *panicked* the config loader — contradicting the "a malformed
        // version must never be a reason to lose the config" contract.
        for v in ["4294967296", "9999999999999", "-1", "0"] {
            let src = format!("version = {v}\n[global]\n");
            assert_eq!(
                detect_schema_version(&table(&src)),
                CURRENT_SCHEMA_VERSION,
                "version = {v} must clamp to current, not wrap"
            );
            // The full path must not panic either.
            let mut t = table(&src);
            let from = migrate_table(&mut t);
            assert_eq!(from, CURRENT_SCHEMA_VERSION);
        }
    }

    #[test]
    fn future_version_is_loaded_read_only_not_downgraded() {
        // A config written by a newer build (v9) must not be silently
        // re-stamped to the current version — that would strip fields
        // this build doesn't understand on the next save. `migrate_table`
        // leaves the version marker and unknown keys intact.
        let mut t = table("version = 9\nfuture_only_key = 42\n");
        let from = migrate_table(&mut t);
        assert_eq!(from, 9, "reports the on-disk future version");
        assert_eq!(
            t.get("version"),
            Some(&toml::Value::Integer(9)),
            "future version marker is preserved, not downgraded"
        );
        assert_eq!(
            t.get("future_only_key"),
            Some(&toml::Value::Integer(42)),
            "unknown future keys survive the no-op migration"
        );
    }

    /// A hand-written config names only what it wants to change. Leaving
    /// out a `[global]` key — or the whole table, or `characters` — used to
    /// fail to decode, and at startup that means backed up and replaced by
    /// the demo scene.
    #[test]
    fn a_config_that_names_only_what_it_changes_loads() {
        let (cfg, _) =
            AppConfig::decode_config_str("version = 2\n[global]\nhover_startle = true\n")
                .expect("a partial [global] is a valid config");
        assert!(cfg.global.hover_startle);
        assert!(cfg.global.always_on_top && cfg.global.transparent && cfg.global.playback_enabled);
        assert!(
            cfg.characters.is_empty(),
            "no characters named, none invented"
        );

        assert!(
            AppConfig::decode_config_str("version = 2\n").is_ok(),
            "no [global] table"
        );
        assert!(AppConfig::decode_config_str("").is_ok(), "an empty file");
    }

    /// A character still needs what makes it a character — without it,
    /// it is set aside, not shown, and kept in the file; the rest reads.
    #[test]
    fn a_character_without_its_asset_is_set_aside() {
        let toml = "version = 2\n[[characters]]\nid = \"a\"\nname = \"A\"\n\
                    asset_type = \"png_static\"\nx = 0.0\ny = 0.0\n\
                    [[characters]]\nid = \"b\"\nname = \"B\"\n\
                    asset_type = \"png_static\"\nasset_path = \"b.png\"\nx = 0.0\ny = 0.0\n";
        let (cfg, _) = AppConfig::decode_config_str(toml).expect("the rest reads");
        assert_eq!(cfg.characters.len(), 1);
        assert_eq!(cfg.characters[0].id, "b");
        assert!(!cfg.set_aside.is_empty());
        let out = cfg.to_toml_string().unwrap();
        assert!(out.contains("id = \"a\""), "kept in the file: {out}");
    }

    /// What a newer version wrote — a behavior, an asset type, a choice
    /// in `[global]`, a character in a window — reads as far as this one
    /// can, and is written back as it was. Each used to cost the whole
    /// file, and the scene with it.
    #[test]
    fn what_a_newer_version_wrote_is_set_aside_and_written_back() {
        let newer = r#"
            version = 2
            [global]
            theme = "sepia"
            hover_startle = true
            on_fullscreen = "dim"

            [[characters]]
            id = "dancer"
            name = "Dancer"
            asset_type = "png_static"
            asset_path = "d.png"
            x = 1.0
            y = 2.0
            [characters.behavior]
            type = "breakdance"
            moves = 3

            [[characters]]
            id = "holo"
            name = "Holo"
            asset_type = "hologram"
            asset_path = "h.holo"
            x = 1.0
            y = 2.0

            [[characters]]
            id = "plain"
            name = "Plain"
            asset_type = "png_static"
            asset_path = "p.png"
            x = 5.0
            y = 6.0

            [[windows]]
            id = "side"
            name = "Side"
            [[windows.characters]]
            id = "w"
            name = "W"
            asset_type = "hologram"
            asset_path = "w.holo"
            x = 0.0
            y = 0.0

            [[groups]]
            id = "g"
            name = "G"
            member_ids = ["plain"]
            visible = "sometimes"
        "#;
        let (cfg, _) = AppConfig::decode_config_str(newer).expect("reads as far as it can");
        let ids: Vec<&str> = cfg.characters.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["plain"]);
        assert!(cfg.windows[0].characters.is_empty());
        assert!(cfg.groups.is_empty());
        assert!(cfg.global.hover_startle, "the settings that read, read");
        assert_eq!(cfg.global.theme, crate::ui::theme::Theme::default());

        // Saved as it is, everything comes back as written.
        let out = cfg.to_toml_string().unwrap();
        let back: toml::Table = out.parse().unwrap();
        let written: toml::Table = newer.parse().unwrap();
        assert_eq!(back["global"]["theme"].as_str(), Some("sepia"));
        assert_eq!(back["global"]["on_fullscreen"].as_str(), Some("dim"));
        let characters = back["characters"].as_array().unwrap();
        assert_eq!(characters.len(), 3);
        for original in written["characters"].as_array().unwrap() {
            let id = original["id"].as_str().unwrap();
            let kept = characters
                .iter()
                .find(|c| c["id"].as_str() == Some(id))
                .unwrap_or_else(|| panic!("{id} kept"));
            if id != "plain" {
                assert_eq!(kept, original, "{id} as written");
            }
        }
        assert_eq!(
            back["windows"][0]["characters"][0],
            written["windows"][0]["characters"][0]
        );
        assert_eq!(back["groups"][0], written["groups"][0]);
        // And it reads the same again.
        let (again, _) = AppConfig::decode_config_str(&out).unwrap();
        assert_eq!(again.set_aside, cfg.set_aside);

        // A setting changed here since is the user's word, not the file's.
        let mut changed = cfg.clone();
        changed.global.theme = crate::ui::theme::Theme::Light;
        let out: toml::Table = changed.to_toml_string().unwrap().parse().unwrap();
        assert_eq!(out["global"]["theme"].as_str(), Some("light"));
        assert_eq!(out["global"]["on_fullscreen"].as_str(), Some("dim"));
    }

    /// What does not read anywhere a list or a setting can be set aside is
    /// still an unreadable file.
    #[test]
    fn a_file_that_cannot_be_read_around_is_still_an_error() {
        assert!(AppConfig::decode_config_str("version = 2\ncollapse_state = 5\n").is_err());
        assert!(AppConfig::decode_config_str("version = 2\nglobal = 5\n").is_err());
        let (cfg, _) = AppConfig::decode_config_str("version = 2\n").unwrap();
        assert!(
            cfg.set_aside.is_empty(),
            "nothing set aside from a good file"
        );
    }

    #[test]
    fn decode_config_str_errors_instead_of_falling_back_to_defaults() {
        // The read-only reload path (`try_reload` → `decode_config_str`)
        // must surface a parse/decode failure as `Err`, never silently
        // substitute defaults the way startup `load` does — that's what
        // lets hot-reload keep the running scene on a mid-save read.
        assert!(
            AppConfig::decode_config_str("not [[[ valid toml").is_err(),
            "garbage must be an error, not a default config"
        );
        // A valid config still decodes cleanly through the same pure
        // path (no disk writes involved).
        let valid = toml::to_string(&AppConfig::default()).expect("serialise default");
        let (cfg, from) = AppConfig::decode_config_str(&valid).expect("valid config decodes");
        assert_eq!(from, CURRENT_SCHEMA_VERSION);
        assert_eq!(cfg.schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn unreadable_config_is_backed_up_verbatim_before_defaults() {
        // A corrupt config must be copied aside byte-for-byte so the
        // default-save that follows a failed load can't destroy the
        // user's only copy (the load-path clobber found post-rc1).
        let dir = std::env::temp_dir().join(format!("anima-corrupt-bak-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = dir.join("config.toml");
        let garbage = "version = 2 [[[ not toml !!!";
        std::fs::write(&cfg, garbage).unwrap();

        backup_unreadable_config(&cfg);

        let bak = dir.join("config.toml.bak-corrupt");
        assert_eq!(
            std::fs::read_to_string(&bak).expect("backup must exist"),
            garbage,
            "backup preserves the unreadable original verbatim"
        );
        // Original untouched by the backup itself.
        assert_eq!(std::fs::read_to_string(&cfg).unwrap(), garbage);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn v1_config_migrates_and_decodes() {
        // A minimal pre-0.9 config: no version key.
        let mut t = table(
            r#"
            characters = []

            [global]
            always_on_top = true
            transparent = true
            playback_enabled = true
            window_width = 0
            window_height = 0
            "#,
        );
        let from = migrate_table(&mut t);
        assert_eq!(from, 1);
        let cfg: AppConfig = toml::Value::Table(t).try_into().expect("decode migrated");
        assert_eq!(cfg.schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn default_config_is_current_version() {
        assert_eq!(AppConfig::default().schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn save_serialises_version_first() {
        // `version` must lead so it's a scalar before any [section];
        // otherwise TOML serialisation fails with "value after table".
        let s = toml::to_string_pretty(&AppConfig::default()).expect("serialise");
        let version_pos = s.find("version =").expect("version key present");
        let first_section = s.find("\n[").unwrap_or(s.len());
        assert!(
            version_pos < first_section,
            "version key must precede the first section:\n{s}"
        );
    }

    #[test]
    fn unknown_section_survives_round_trip() {
        // Forward-compat: a section a newer animaEngine added, that this
        // build doesn't model, must come back out unchanged.
        let src = format!(
            r#"
            version = {CURRENT_SCHEMA_VERSION}
            characters = []

            [global]
            always_on_top = true
            transparent = true
            playback_enabled = true
            window_width = 0
            window_height = 0

            [future_feature]
            shiny = true
            count = 7
            "#
        );
        let cfg: AppConfig = toml::from_str(&src).expect("decode with unknown section");
        assert!(
            cfg.extra.contains_key("future_feature"),
            "unknown section captured in extra: {:?}",
            cfg.extra
        );
        let out = toml::to_string_pretty(&cfg).expect("re-serialise");
        let back: AppConfig = toml::from_str(&out).expect("re-decode");
        let ff = back
            .extra
            .get("future_feature")
            .and_then(|v| v.as_table())
            .expect("future_feature preserved");
        assert_eq!(ff.get("shiny"), Some(&toml::Value::Boolean(true)));
        assert_eq!(ff.get("count"), Some(&toml::Value::Integer(7)));
    }
}

#[cfg(test)]
mod sanitize_tests {
    use super::*;

    #[test]
    fn finite_clamp_replaces_non_finite_and_clamps_range() {
        assert_eq!(finite_clamp(f32::NAN, 0.1, 5.0, 1.0), 1.0);
        assert_eq!(finite_clamp(f32::INFINITY, 0.1, 5.0, 1.0), 1.0);
        assert_eq!(finite_clamp(f32::NEG_INFINITY, 0.1, 5.0, 1.0), 1.0);
        assert_eq!(finite_clamp(100.0, 0.1, 5.0, 1.0), 5.0); // over-range clamps
        assert_eq!(finite_clamp(0.0, 0.1, 5.0, 1.0), 0.1); // under-range clamps
        assert_eq!(finite_clamp(2.5, 0.1, 5.0, 1.0), 2.5); // in-range kept
    }

    #[test]
    fn finite_or_replaces_non_finite_but_keeps_offscreen() {
        assert_eq!(finite_or(f32::NAN, 0.0), 0.0);
        assert_eq!(finite_or(f32::INFINITY, 0.0), 0.0);
        assert_eq!(finite_or(-500.0, 0.0), -500.0); // off-screen is legitimate
    }

    #[test]
    fn character_sanitize_fixes_hand_edited_nan_inf() {
        // TOML floats accept `nan` / `inf` literally — exactly the
        // hand-edited-config vector the sanitizer defends against.
        let toml_str = r#"
            id = "x"
            name = "x"
            asset_type = "gif"
            asset_path = "a.gif"
            x = nan
            y = inf
            scale = nan
            opacity = inf
            fps = nan
        "#;
        let mut c: CharacterConfig = toml::from_str(toml_str).unwrap();
        assert!(!c.scale.is_finite(), "precondition: parsed NaN scale");
        c.sanitize();
        assert!(c.x.is_finite() && c.y.is_finite());
        assert_eq!(c.scale, default_scale());
        assert_eq!(c.opacity, default_opacity());
        assert_eq!(c.fps, default_fps());
    }
}

/// Configs written by released versions (`tests/fixtures/configs`) read
/// in this one with every value they hold, and save back with it.
#[cfg(test)]
mod upgrade_tests {
    use super::*;
    use std::collections::BTreeMap;

    const WRITTEN_BY: [(&str, &str); 3] = [
        (
            "1.0.0, first run",
            include_str!("../tests/fixtures/configs/v1.0.0-default.toml"),
        ),
        (
            "1.1.0 to 1.4.0, first run",
            include_str!("../tests/fixtures/configs/v1.4.0-default.toml"),
        ),
        (
            "1.4.0, every setting changed",
            include_str!("../tests/fixtures/configs/v1.4.0-full.toml"),
        ),
    ];

    /// Every value in a table, by its path: `global.theme`,
    /// `characters[2].behavior.speed`.
    fn values(path: &str, value: &toml::Value, out: &mut BTreeMap<String, toml::Value>) {
        match value {
            toml::Value::Table(table) => {
                for (key, value) in table {
                    let path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    values(&path, value, out);
                }
            }
            toml::Value::Array(items) if items.iter().any(toml::Value::is_table) => {
                for (i, item) in items.iter().enumerate() {
                    values(&format!("{path}[{i}]"), item, out);
                }
            }
            other => {
                out.insert(path.to_string(), other.clone());
            }
        }
    }

    #[test]
    fn configs_from_earlier_versions_keep_every_value() {
        for (by, text) in WRITTEN_BY {
            let (config, _) = AppConfig::decode_config_str(text)
                .unwrap_or_else(|e| panic!("{by}: does not read: {e}"));
            assert!(config.set_aside.is_empty(), "{by}: {:?}", config.set_aside);
            assert!(
                config.extra.is_empty(),
                "{by}: unknown here: {:?}",
                config.extra
            );
            let saved = config.to_toml_string().unwrap();

            let mut before = BTreeMap::new();
            values("", &toml::Value::Table(text.parse().unwrap()), &mut before);
            let mut after = BTreeMap::new();
            values("", &toml::Value::Table(saved.parse().unwrap()), &mut after);
            let changed: Vec<String> = before
                .iter()
                .filter(|(path, value)| after.get(*path) != Some(value))
                .map(|(path, value)| format!("{path}: {value} → {:?}", after.get(path)))
                .collect();
            assert!(changed.is_empty(), "{by}: {changed:#?}");
        }
    }

    /// The shortcuts someone set in 1.4 still do what they did: none is
    /// taken by an action 1.5 added with that chord as its default.
    #[test]
    fn shortcuts_from_earlier_versions_still_do_what_they_did() {
        for (by, text) in WRITTEN_BY {
            let (config, _) = AppConfig::decode_config_str(text).unwrap();
            let bindings = &config.keybindings;
            for (action, chords) in &bindings.map {
                for chord in chords {
                    assert_eq!(
                        bindings.lookup(*chord),
                        Some(*action),
                        "{by}: {chord} is {action:?}'s"
                    );
                }
            }
        }
    }
}
