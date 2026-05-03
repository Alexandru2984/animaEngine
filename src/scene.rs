use crate::animation::loader::{generate_fallback_frame, load_asset};
use crate::animation::Animation;
use crate::config::{AppConfig, CharacterConfig};
use crate::entity::Entity;

/// The scene holds all active entities and global playback state
#[derive(Debug)]
pub struct Scene {
    /// All entities in the scene
    pub entities: Vec<Entity>,
    /// Global play/pause flag
    pub global_playing: bool,
}

impl Scene {
    /// Build a scene from app config
    pub fn from_config(config: &AppConfig) -> Self {
        let mut entities = Vec::new();

        for char_config in &config.characters {
            match Self::load_entity(char_config) {
                Ok(entity) => {
                    log::info!(
                        "Loaded entity '{}' ({} frames)",
                        entity.name,
                        entity.animation.frame_count()
                    );
                    entities.push(entity);
                }
                Err(e) => {
                    log::warn!(
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

        log::info!("Scene loaded with {} entities", entities.len());

        Self {
            entities,
            global_playing: config.global.playback_enabled,
        }
    }

    /// Load a single entity from config
    fn load_entity(config: &CharacterConfig) -> Result<Entity, Box<dyn std::error::Error>> {
        let resolved_path = AppConfig::resolve_asset_path(&config.asset_path);
        let frames = load_asset(&config.asset_type, &resolved_path)?;
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

    /// Tick all entity animations
    pub fn tick(&mut self) {
        if !self.global_playing {
            return;
        }

        for entity in &mut self.entities {
            entity.tick();
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
        log::info!(
            "Global playback: {}",
            if self.global_playing {
                "PLAYING"
            } else {
                "PAUSED"
            }
        );
    }

    /// Convert current scene state back to config for saving
    pub fn to_character_configs(&self) -> Vec<CharacterConfig> {
        self.entities.iter().map(|e| e.to_config()).collect()
    }
}
