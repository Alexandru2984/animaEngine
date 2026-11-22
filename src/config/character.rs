//! `[[characters]]` and what each holds — its pictures, its behaviour,
//! what it says — and `[[windows]]`, the 0.3 per-window lists.

use super::*;

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

pub(super) fn is_zero(n: &u32) -> bool {
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

pub(super) fn is_idle_behavior(b: &Behavior) -> bool {
    matches!(b, Behavior::Idle)
}

pub(super) fn default_scale() -> f32 {
    1.0
}
pub(super) fn default_opacity() -> f32 {
    1.0
}
/// Default animation fps — also the double-click-reset target for the
/// inspector's FPS slider (V.3), hence `pub`.
pub fn default_fps() -> f32 {
    12.0
}
pub(super) fn default_true() -> bool {
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

/// How far from the origin a place read from a file may be, either way.
///
/// Characters sit off-screen legitimately, so places are not held to the
/// desktop; but only having to be finite let ±2e38 through, and the
/// sums made with it overflowed — to infinity in an imported scene's
/// fit (`inf × 0 = NaN`), out of `i32` in the input region's grid snap
/// (BUG-007). A million pixels is far past any desktop.
pub(crate) const MAX_COORD: f32 = 1_000_000.0;

/// A place read from a file: `default` for `NaN` / `inf`, which would
/// poison the render path, and held within [`MAX_COORD`].
pub(crate) fn finite_place(v: f32, default: f32) -> f32 {
    finite_clamp(v, -MAX_COORD, MAX_COORD, default)
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
        self.x = finite_place(self.x, 0.0);
        self.y = finite_place(self.y, 0.0);
        // A state's own fps, as the character's: `nan` made two states
        // never equal, an undo step for every gesture, and `inf` was past
        // the range (BUG-007). Not finite, it falls back to the
        // character's, as when it is left out.
        for state in self.animations.values_mut() {
            state.fps = state
                .fps
                .filter(|fps| fps.is_finite())
                .map(|fps| fps.clamp(0.1, 240.0));
        }
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
