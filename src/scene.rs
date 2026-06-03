use crate::animation::loader::{generate_fallback_frame, load_asset};
use crate::animation::Animation;
use crate::config::{AppConfig, CharacterConfig};
use crate::constants::MAX_DROP_SIZE;
use crate::entity::Entity;
use crate::error::Result;
use std::cell::RefCell;
use std::time::Instant;

/// Cached visibility/z-order index list. Rebuilt only when invalidated.
#[derive(Debug, Default)]
struct VisibleCache {
    /// Indices into `Scene::entities`, sorted by z_index ascending.
    /// Contains only visible entities.
    indices: Vec<usize>,
    /// `true` if `indices` matches the current scene state.
    valid: bool,
}

/// The scene holds all active entities and global playback state
#[derive(Debug)]
pub struct Scene {
    /// All entities in the scene
    pub entities: Vec<Entity>,
    /// Global play/pause flag
    pub global_playing: bool,
    /// Last tick time for delta time calculation
    last_tick: Instant,
    /// Cached visibility/z-order. `RefCell` lets `visible_entities(&self)`
    /// refresh the cache lazily without taking `&mut self`.
    visible_cache: RefCell<VisibleCache>,
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
            visible_cache: RefCell::default(),
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

    /// Tick all entities: behavior + physics + animation.
    /// Screen dimensions bound autonomous motion (walk-around) and gravity.
    /// `cursor` is forwarded to behaviors that track the mouse (FollowCursor);
    /// pass `None` when the position is stale or unknown.
    pub fn tick(&mut self, screen_width: f32, screen_height: f32, cursor: Option<(f32, f32)>) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f32();
        self.last_tick = now;

        // Clamp dt to prevent physics / behavior explosion after long pauses.
        let dt = dt.min(0.1);

        if !self.global_playing {
            return;
        }

        for entity in &mut self.entities {
            entity.tick(dt, screen_width, screen_height, cursor);
        }
    }

    /// Mark the visible/z-order cache dirty. Call this from a parent module
    /// any time an entity's `visible` or `z_index` field changes directly.
    /// (Mutations through `add_entity_from_path` / `remove_entity` invalidate
    /// the cache automatically.)
    pub fn mark_visible_dirty(&mut self) {
        self.visible_cache.borrow_mut().valid = false;
    }

    /// Get entities sorted by z_index for rendering (back to front).
    ///
    /// Uses a lazily-refreshed cache so the per-frame cost is just a Vec build
    /// from cached indices, not a full filter + sort. The cache is invalidated
    /// on add/remove and via `mark_visible_dirty` for direct field changes.
    pub fn visible_entities(&self) -> Vec<&Entity> {
        {
            let mut cache = self.visible_cache.borrow_mut();
            if !cache.valid {
                cache.indices.clear();
                cache
                    .indices
                    .extend((0..self.entities.len()).filter(|&i| self.entities[i].visible));
                cache.indices.sort_by_key(|&i| self.entities[i].z_index);
                cache.valid = true;
            }
        }
        let cache = self.visible_cache.borrow();
        cache.indices.iter().map(|&i| &self.entities[i]).collect()
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
            behavior: crate::behavior::Behavior::Idle,
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
        // `?` propagates a corrupt-buffer error instead of silently producing
        // a frame with mismatched dimensions.
        let frames: Vec<_> = frames
            .into_iter()
            .map(|f| f.resized(MAX_DROP_SIZE))
            .collect::<Result<Vec<_>>>()?;

        let animation = Animation::new(frames, char_config.fps, char_config.playing);
        let entity = Entity::from_config(&char_config, animation);
        tracing::info!(
            "Entity '{}' loaded: {} frames (max {}px)",
            entity.id,
            entity.animation.frame_count(),
            MAX_DROP_SIZE
        );

        self.entities.push(entity);
        self.mark_visible_dirty();
        let idx = self.entities.len() - 1;
        Ok(idx)
    }

    /// Remove an entity by index. Returns the removed entity's ID.
    pub fn remove_entity(&mut self, index: usize) -> Option<String> {
        if index < self.entities.len() {
            let entity = self.entities.remove(index);
            tracing::info!("Removed entity '{}' ({})", entity.name, entity.id);
            self.mark_visible_dirty();
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

    /// Replace every entity with the given character configs. Used by
    /// the preset gallery (`presets::apply_to_scene`) when the user
    /// picks "Replace". Failed loads fall back to a placeholder so a
    /// missing asset can't strand the preset application midway.
    ///
    /// SECURITY: this entry point assumes `configs` come from a trusted
    /// in-binary source (hardcoded `Preset::for_id` rosters). Each
    /// `CharacterConfig::asset_path` is fed to `load_asset` without
    /// going through `app::pre_validate_dropped_file`. If you wire a
    /// future "import scene from URL / external file" path into this
    /// method, you MUST run the same drag-drop validation (extension
    /// whitelist, byte cap, frame cap) on every `asset_path` first, or
    /// route it through `add_entity_from_path` per character instead.
    pub fn reset_to_configs(&mut self, configs: &[CharacterConfig]) {
        self.entities.clear();
        for cfg in configs {
            let entity = Self::load_entity(cfg).unwrap_or_else(|err| {
                tracing::warn!(
                    "Preset entity '{}' failed to load: {}; using fallback",
                    cfg.id,
                    err,
                );
                Self::create_fallback_entity(cfg)
            });
            self.entities.push(entity);
        }
        self.entities.sort_by_key(|e| e.z_index);
        self.mark_visible_dirty();
    }

    /// Append one character config to the scene. Mirrors the success
    /// path of `add_entity_from_path` but skips the path-resolution +
    /// type-detection dance because the caller already has a finished
    /// `CharacterConfig` (a preset, a hot-reload result, etc.).
    ///
    /// SECURITY: same trust assumption as `reset_to_configs` above —
    /// `cfg.asset_path` is loaded as-is, with no whitelist or size
    /// pre-check. Today every caller passes a hardcoded preset config.
    pub fn append_character_config(&mut self, cfg: &CharacterConfig) -> Result<()> {
        let entity = Self::load_entity(cfg)?;
        self.entities.push(entity);
        self.mark_visible_dirty();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::frame::Frame;
    use crate::config::{AssetType, GlobalConfig};

    fn make_entity(id: &str, z: i32, visible: bool) -> Entity {
        let frame = Frame::new(vec![0u8; 4], 1, 1);
        let anim = Animation::new(vec![frame], 1.0, false);
        let cfg = CharacterConfig {
            id: id.into(),
            name: id.into(),
            asset_type: AssetType::PngStatic,
            asset_path: String::new(),
            x: 0.0,
            y: 0.0,
            scale: 1.0,
            opacity: 1.0,
            fps: 1.0,
            visible,
            playing: false,
            z_index: z,
            physics_enabled: false,
            behavior: crate::behavior::Behavior::Idle,
            spritesheet_columns: None,
            spritesheet_rows: None,
        };
        Entity::from_config(&cfg, anim)
    }

    fn empty_scene() -> Scene {
        let config = AppConfig {
            global: GlobalConfig::default(),
            characters: vec![],
        };
        Scene::from_config(&config)
    }

    #[test]
    fn visible_entities_filters_and_sorts_by_z_index() {
        let mut scene = empty_scene();
        scene.entities.push(make_entity("a", 30, true));
        scene.entities.push(make_entity("b", 10, true));
        scene.entities.push(make_entity("c", 20, false)); // hidden — must be skipped
        scene.entities.push(make_entity("d", 20, true));
        scene.mark_visible_dirty();

        let ids: Vec<&str> = scene
            .visible_entities()
            .iter()
            .map(|e| e.id.as_str())
            .collect();
        assert_eq!(ids, vec!["b", "d", "a"]);
    }

    #[test]
    fn cache_is_reused_when_valid() {
        let mut scene = empty_scene();
        scene.entities.push(make_entity("a", 0, true));
        scene.mark_visible_dirty();
        // First call populates the cache.
        let _ = scene.visible_entities();
        assert!(scene.visible_cache.borrow().valid);
        // Without invalidation, second call leaves it valid.
        let _ = scene.visible_entities();
        assert!(scene.visible_cache.borrow().valid);
    }

    #[test]
    fn add_entity_invalidates_cache() {
        let mut scene = empty_scene();
        scene.entities.push(make_entity("a", 0, true));
        scene.mark_visible_dirty();
        let _ = scene.visible_entities();
        assert!(scene.visible_cache.borrow().valid);

        // add_entity_from_path requires real I/O, so simulate the
        // invariant: any mutation that adds/removes must invalidate.
        scene.entities.push(make_entity("b", 5, true));
        scene.mark_visible_dirty();
        assert!(!scene.visible_cache.borrow().valid);
    }

    #[test]
    fn remove_entity_invalidates_cache() {
        let mut scene = empty_scene();
        scene.entities.push(make_entity("a", 0, true));
        scene.entities.push(make_entity("b", 10, true));
        scene.mark_visible_dirty();
        let _ = scene.visible_entities();

        scene.remove_entity(0);
        assert!(!scene.visible_cache.borrow().valid);

        let ids: Vec<&str> = scene
            .visible_entities()
            .iter()
            .map(|e| e.id.as_str())
            .collect();
        assert_eq!(ids, vec!["b"]);
    }
}
