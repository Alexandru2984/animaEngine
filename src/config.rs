use crate::constants::MAX_ENTITIES;
use crate::error::Result;
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
    /// Number of columns in spritesheet grid (only used for Spritesheet type)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spritesheet_columns: Option<u32>,
    /// Number of rows in spritesheet grid (only used for Spritesheet type)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spritesheet_rows: Option<u32>,
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

/// Full application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub global: GlobalConfig,
    #[serde(rename = "characters")]
    pub characters: Vec<CharacterConfig>,
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
                    spritesheet_columns: None,
                    spritesheet_rows: None,
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
                    spritesheet_columns: None,
                    spritesheet_rows: None,
                },
            ],
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

    /// Save config to disk
    #[tracing::instrument(skip(self), fields(n_chars = self.characters.len()))]
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let toml_string = toml::to_string_pretty(self)?;
        fs::write(&path, toml_string)?;
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

    /// Detect the AssetType from a file extension
    pub fn detect_asset_type(path: &str) -> AssetType {
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "gif" => AssetType::Gif,
            "webp" => AssetType::WebpAnimated,
            "png" => {
                let p = Path::new(path);
                if p.is_dir() {
                    AssetType::PngSequence
                } else {
                    AssetType::PngStatic
                }
            }
            _ => AssetType::PngStatic,
        }
    }
}
