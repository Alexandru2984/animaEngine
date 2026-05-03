use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Global application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub always_on_top: bool,
    pub transparent: bool,
    pub playback_enabled: bool,
    /// Window width (0 = auto/fullscreen)
    #[serde(default = "default_window_width")]
    pub window_width: u32,
    /// Window height (0 = auto/fullscreen)
    #[serde(default = "default_window_height")]
    pub window_height: u32,
}

fn default_window_width() -> u32 {
    1920
}
fn default_window_height() -> u32 {
    1080
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            always_on_top: true,
            transparent: true,
            playback_enabled: true,
            window_width: default_window_width(),
            window_height: default_window_height(),
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
                    scale: 1.5,
                    opacity: 0.9,
                    fps: 12.0,
                    visible: true,
                    playing: true,
                    z_index: 10,
                },
                CharacterConfig {
                    id: "slime".to_string(),
                    name: "Slime Demo".to_string(),
                    asset_type: AssetType::PngSequence,
                    asset_path: "assets/demo/slime".to_string(),
                    x: 600.0,
                    y: 400.0,
                    scale: 1.8,
                    opacity: 1.0,
                    fps: 8.0,
                    visible: true,
                    playing: true,
                    z_index: 20,
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
        log::info!("Config path: {}", path.display());

        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(contents) => match toml::from_str::<AppConfig>(&contents) {
                    Ok(config) => {
                        log::info!("Loaded config with {} characters", config.characters.len());
                        return config;
                    }
                    Err(e) => {
                        log::warn!("Failed to parse config: {}. Using defaults.", e);
                    }
                },
                Err(e) => {
                    log::warn!("Failed to read config: {}. Using defaults.", e);
                }
            }
        } else {
            log::info!("Config not found, creating default config");
        }

        let config = AppConfig::default();
        if let Err(e) = config.save() {
            log::warn!("Failed to save default config: {}", e);
        }
        config
    }

    /// Save config to disk
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path();

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let toml_string = toml::to_string_pretty(self)?;
        fs::write(&path, toml_string)?;
        log::info!("Config saved to {}", path.display());
        Ok(())
    }

    /// Resolve an asset path relative to the executable or current directory
    pub fn resolve_asset_path(asset_path: &str) -> PathBuf {
        let path = Path::new(asset_path);
        if path.is_absolute() {
            return path.to_path_buf();
        }

        // Try relative to current executable
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let resolved = exe_dir.join(asset_path);
                if resolved.exists() {
                    return resolved;
                }
            }
        }

        // Try relative to current directory
        let cwd_path = PathBuf::from(asset_path);
        if cwd_path.exists() {
            return cwd_path;
        }

        // Return as-is, let caller handle missing path
        PathBuf::from(asset_path)
    }
}
