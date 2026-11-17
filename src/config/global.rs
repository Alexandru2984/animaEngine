//! `[global]`: the settings that are not a character's — what the
//! overlay does, how it looks and sounds, when it steps aside or rests.

use super::character::default_true;
use super::*;

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

    /// The ⚙ button outside edit mode (1.5.1, issue #1). Off, nothing of
    /// the overlay takes a click outside edit mode but the characters
    /// that take taps; edit mode comes from the tray, the global shortcut,
    /// a compositor binding over D-Bus, or starting the app again, which
    /// opens it then. In edit mode the button shows either way: it is the
    /// way out.
    #[serde(default = "default_true")]
    pub show_toggle_button: bool,

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
    /// How often moving characters are drawn: about sixty a second, or
    /// thirty, always or on battery (`crate::pacing::FrameRate`).
    #[serde(default)]
    pub frame_rate: crate::pacing::FrameRate,

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

pub(super) fn default_sound_volume() -> f32 {
    1.0
}

pub(super) fn is_full_volume(volume: &f32) -> bool {
    *volume >= 1.0
}

pub(super) fn default_pause_when_idle_minutes() -> u32 {
    10
}

pub(super) fn default_window_width() -> u32 {
    0 // 0 = auto-detect from monitor
}
pub(super) fn default_window_height() -> u32 {
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
            show_toggle_button: true,
            little_effects: true,
            pause_when_idle_minutes: default_pause_when_idle_minutes(),
            pause_on_battery: false,
            frame_rate: Default::default(),
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
