use crate::animation::loader::{generate_fallback_frame, load_asset};
use crate::animation::Animation;
use crate::config::{AppConfig, CharacterConfig};
use crate::constants::MAX_DROP_SIZE;
use crate::entity::Entity;
use crate::error::Result;
use std::time::Instant;

/// The scene holds all active entities and global playback state
#[derive(Debug)]
pub struct Scene {
    /// All entities in the scene
    pub entities: Vec<Entity>,
    /// Global play/pause flag
    pub global_playing: bool,
    /// Last tick time for delta time calculation
    last_tick: Instant,
}

impl Scene {
    /// Build a scene from app config
    #[tracing::instrument(skip(config), fields(n_chars = config.characters.len()))]
    pub fn from_config(config: &AppConfig) -> Self {
        let mut entities = Vec::new();

        for char_config in &config.characters {
            match Self::load_entity(char_config) {
                Ok(entity) => {
                    tracing::info!(
                        "Loaded entity '{}' ({} frames, per-frame delays: {})",
                        entity.name,
                        entity.animation.frame_count(),
                        entity.animation.has_per_frame_delays
                    );
                    entities.push(entity);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to load entity '{}': {}. Using fallback.",
                        char_config.id,
                        e
                    );
                    // Create entity with fallback frame
                    let fallback = Self::create_fallback_entity(char_config);
                    entities.push(fallback);
                }
            }
        }

        // Sort by z_index for correct draw order
        entities.sort_by_key(|e| e.z_index);

        tracing::info!("Scene loaded with {} entities", entities.len());

        Self {
            entities,
            global_playing: config.global.playback_enabled,
            last_tick: Instant::now(),
        }
    }

    /// Load a single entity from config
    fn load_entity(config: &CharacterConfig) -> Result<Entity> {
        let resolved_path = AppConfig::resolve_asset_path(&config.asset_path);
        let frames = load_asset(
            &config.asset_type,
            &resolved_path,
            config.spritesheet_columns,
            config.spritesheet_rows,
        )?;
        let animation = Animation::new(frames, config.fps, config.playing);
        Ok(Entity::from_config(config, animation))
    }

    /// Create a fallback entity with procedurally generated frame
    fn create_fallback_entity(config: &CharacterConfig) -> Entity {
        // Generate a colored circle based on entity id
        let color = match config.id.as_str() {
            "ghost" => [200, 200, 255, 180], // Light blue, semi-transparent
            "slime" => [100, 220, 100, 230], // Green
            _ => [255, 200, 100, 200],       // Orange
        };

        // Generate 3 frames with slight size variation for animation
        let frames: Vec<_> = (0..3)
            .map(|i| {
                let size = 64 + (i as u32) * 4;
                generate_fallback_frame(color, size)
            })
            .collect();

        let animation = Animation::new(frames, config.fps, config.playing);
        Entity::from_config(config, animation)
    }

    /// Tick all entity animations and physics.
    /// `screen_height` is needed for floor collision.
    pub fn tick(&mut self, screen_height: f32) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f32();
        self.last_tick = now;

        // Clamp dt to prevent physics explosion after long pauses
        let dt = dt.min(0.1);

        if !self.global_playing {
            return;
        }

        for entity in &mut self.entities {
            entity.tick(dt, screen_height);
        }
    }

    /// Get entities sorted by z_index for rendering (back to front)
    pub fn visible_entities(&self) -> Vec<&Entity> {
        let mut visible: Vec<&Entity> = self.entities.iter().filter(|e| e.visible).collect();
        visible.sort_by_key(|e| e.z_index);
        visible
    }

    /// Find the topmost entity at a screen position (reverse z-order)
    pub fn entity_at_point(&self, x: f32, y: f32) -> Option<usize> {
        // Check in reverse z-order (topmost first)
        let mut indices: Vec<usize> = (0..self.entities.len())
            .filter(|&i| self.entities[i].visible)
            .collect();
        indices.sort_by(|&a, &b| self.entities[b].z_index.cmp(&self.entities[a].z_index));

        indices
            .into_iter()
            .find(|&idx| self.entities[idx].contains_point(x, y))
    }

    /// Toggle global playback
    pub fn toggle_global_playback(&mut self) {
        self.global_playing = !self.global_playing;
        tracing::info!(
            "Global playback: {}",
            if self.global_playing {
                "PLAYING"
            } else {
                "PAUSED"
            }
        );
    }

    /// Add a new entity by loading an asset from a file path.
    /// Auto-detects the asset type from the extension.
    /// Returns the index of the new entity, or an error if loading fails.
    #[tracing::instrument(skip(self), fields(path = %path.display()))]
    pub fn add_entity_from_path(
        &mut self,
        path: &std::path::Path,
        x: f32,
        y: f32,
    ) -> Result<usize> {
        use crate::animation::loader::detect_asset_type;

        let (asset_type, type_desc) = detect_asset_type(path);

        // Generate a unique ID from the filename
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("entity");
        let id = format!("{}_{}", stem, self.entities.len());
        let name = stem.to_string();

        // Use absolute path for reliable loading
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_default().join(path)
        };
        let asset_path_str = abs_path.to_string_lossy().to_string();

        tracing::info!(
            "Adding entity '{}' from {} ({})",
            name,
            asset_path_str,
            type_desc
        );

        // Build a config for this entity
        let char_config = CharacterConfig {
            id: id.clone(),
            name,
            asset_type: asset_type.clone(),
            asset_path: asset_path_str,
            x,
            y,
            scale: 1.0,
            opacity: 1.0,
            fps: 12.0,
            visible: true,
            playing: true,
            z_index: self.next_z_index(),
            physics_enabled: false,
            spritesheet_columns: None,
            spritesheet_rows: None,
        };

        // Load frames and resize to overlay-friendly dimensions
        let resolved = AppConfig::resolve_asset_path(&char_config.asset_path);
        let frames = load_asset(
            &char_config.asset_type,
            &resolved,
            char_config.spritesheet_columns,
            char_config.spritesheet_rows,
        )?;

        // Cap frames at MAX_DROP_SIZE for overlay-friendly sprites.
        let frames: Vec<_> = frames
            .into_iter()
            .map(|f| f.resized(MAX_DROP_SIZE))
            .collect();

        let animation = Animation::new(frames, char_config.fps, char_config.playing);
        let entity = Entity::from_config(&char_config, animation);
        tracing::info!(
            "Entity '{}' loaded: {} frames (max {}px)",
            entity.id,
            entity.animation.frame_count(),
            MAX_DROP_SIZE
        );

        self.entities.push(entity);
        let idx = self.entities.len() - 1;
        Ok(idx)
    }

    /// Remove an entity by index. Returns the removed entity's ID.
    pub fn remove_entity(&mut self, index: usize) -> Option<String> {
        if index < self.entities.len() {
            let entity = self.entities.remove(index);
            tracing::info!("Removed entity '{}' ({})", entity.name, entity.id);
            Some(entity.id)
        } else {
            None
        }
    }

    /// Get the next z_index value (one above the current maximum)
    fn next_z_index(&self) -> i32 {
        self.entities.iter().map(|e| e.z_index).max().unwrap_or(0) + 10
    }

    /// Convert current scene state back to config for saving
    pub fn to_character_configs(&self) -> Vec<CharacterConfig> {
        self.entities.iter().map(|e| e.to_config()).collect()
    }
}
