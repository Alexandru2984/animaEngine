use crate::behavior::Behavior;
use crate::constants::MAX_ENTITIES;
use crate::error::Result;
use crate::monitor::MonitorMode;
use crate::ui::{OnboardingProgress, Theme};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Global application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub always_on_top: bool,
    pub transparent: bool,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
fn default_fps() -> f32 {
    12.0
}
fn default_true() -> bool {
    true
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

/// Full application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub global: GlobalConfig,
    #[serde(rename = "characters")]
    pub characters: Vec<CharacterConfig>,
    /// Multi-window roster. Empty (or absent) means "legacy single
    /// overlay backed by `characters` above" — 0.2 configs decode
    /// identically with no migration. When non-empty, `characters`
    /// continues to work and is treated as the first implicit window
    /// so we never silently drop user entities.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub windows: Vec<WindowConfig>,
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
                },
            ],
            // 0.3 fresh installs use the legacy single-window shape;
            // multi-window is opt-in via UI / hand-edit (data layer ready
            // in C.3, render-side dispatch coming in 0.4).
            windows: vec![],
        }
    }
}

impl AppConfig {
    /// Get the config file path: ~/.config/animaEngine/config.toml
    pub fn config_path() -> PathBuf {
        if let Some(proj_dirs) = directories::ProjectDirs::from("", "", "animaEngine") {
            proj_dirs.config_dir().to_path_buf().join("config.toml")
        } else {
            // Fallback if XDG dirs not available
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home)
                .join(".config")
                .join("animaEngine")
                .join("config.toml")
        }
    }

    /// Load config from disk, or create default if not found
    pub fn load() -> Self {
        let path = Self::config_path();
        tracing::info!("Config path: {}", path.display());

        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(contents) => match toml::from_str::<AppConfig>(&contents) {
                    Ok(mut config) => {
                        if config.characters.len() > MAX_ENTITIES {
                            tracing::warn!(
                                "Config has {} characters, capping at {} to prevent resource exhaustion",
                                config.characters.len(),
                                MAX_ENTITIES
                            );
                            config.characters.truncate(MAX_ENTITIES);
                        }
                        tracing::info!("Loaded config with {} characters", config.characters.len());
                        return config;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse config: {}. Using defaults.", e);
                    }
                },
                Err(e) => {
                    tracing::warn!("Failed to read config: {}. Using defaults.", e);
                }
            }
        } else {
            tracing::info!("Config not found, creating default config");
        }

        let config = AppConfig::default();
        if let Err(e) = config.save() {
            tracing::warn!("Failed to save default config: {}", e);
        }
        config
    }

    /// Save config to disk **atomically** — writes to a temp sibling
    /// then renames over the target. A crash mid-save can no longer
    /// leave a truncated `config.toml`.
    #[tracing::instrument(skip(self), fields(n_chars = self.characters.len()))]
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        let toml_string = toml::to_string_pretty(self)?;
        crate::util::atomic_write_bytes(&path, toml_string.as_bytes())?;
        tracing::info!("Config saved to {}", path.display());
        Ok(())
    }

    /// Resolve an asset path relative to the executable or current directory.
    /// Supports:
    /// - Absolute paths (returned as-is)
    /// - `~` expansion to home directory
    /// - Relative paths (checked against exe dir, then cwd)
    pub fn resolve_asset_path(asset_path: &str) -> PathBuf {
        let asset_path = if asset_path.starts_with('~') {
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
        }
    }

    #[test]
    fn legacy_config_synthesises_default_window() {
        let cfg = AppConfig {
            global: GlobalConfig::default(),
            characters: vec![empty_char("ghost")],
            windows: vec![],
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
