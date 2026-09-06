use crate::animation::loader::{generate_fallback_frame, load_asset};
use crate::animation::Animation;
use crate::config::{AppConfig, CharacterConfig};
use crate::constants::{max_total_decoded_bytes, MAX_DROP_SIZE, MAX_ENTITIES};
use crate::entity::Entity;
use crate::error::{AnimaError, Result};
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
    /// Sprite groups. Empty when the user hasn't created any. Mirrors
    /// `AppConfig.groups`; kept on Scene so visibility folding (C.8),
    /// the composed hit-test and the renderer's offset/scale
    /// composition (C.9, 0.7) all read one source of truth.
    pub groups: Vec<crate::group::GroupConfig>,
    /// Desktop-window platforms for window-awareness physics. Fed by
    /// the X11 watcher from the render loop (~300 ms cadence); empty
    /// when the feature is off or no provider exists (native
    /// Wayland). Entities treat the rect tops as floors — see
    /// `crate::platforms`.
    window_platforms: Vec<crate::platforms::PlatformRect>,
    /// Reduced-motion preference (a11y, V.1): forwarded into every
    /// entity tick so decorative behaviors (Bounce bobbing) idle at
    /// their rest position instead of animating.
    reduced_motion: bool,
    hover_startle: bool,
}

impl Scene {
    /// Build a scene from app config
    #[tracing::instrument(skip(config), fields(n_chars = config.characters.len()))]
    pub fn from_config(config: &AppConfig) -> Self {
        let mut entities = Vec::new();

        // Enforce the aggregate decode budget on the STARTUP path too —
        // not only on runtime adds. A hand-edited config with many large
        // assets (or many per-state animation sets) would otherwise decode
        // up to the per-asset-cap × MAX_ENTITIES worst case the budget
        // exists to prevent. We track the running total of *kept* entities;
        // an entity that would push us over loads (one transient decode,
        // itself capped) but is dropped for a fallback instead of retained.
        let budget = max_total_decoded_bytes();
        let mut decoded_total: usize = 0;

        for char_config in &config.characters {
            match Self::load_entity(char_config) {
                Ok(entity) => {
                    let incoming = entity.animations.decoded_bytes();
                    if check_budget(decoded_total, incoming, budget).is_err() {
                        tracing::warn!(
                            "Entity '{}' would exceed the {} MB decode budget at load; using fallback",
                            char_config.id,
                            budget / (1024 * 1024),
                        );
                        entities.push(Self::create_fallback_entity(char_config));
                        continue;
                    }
                    decoded_total = decoded_total.saturating_add(incoming);
                    tracing::info!(
                        "Loaded entity '{}' ({} frames, per-frame delays: {})",
                        entity.name,
                        entity.animation().frame_count(),
                        entity.animation().has_per_frame_delays
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

        // Group id-uniqueness is a soft constraint: we log on
        // duplicates but keep the data so a hand-edited config that
        // copy-pasted a `[[groups]]` entry doesn't crash on load.
        if let Some(dup) = crate::group::first_duplicate_id(&config.groups) {
            tracing::warn!(
                "AppConfig.groups carries duplicate id {:?}; first occurrence wins for visibility resolution",
                dup,
            );
        }

        Self {
            entities,
            global_playing: config.global.playback_enabled,
            last_tick: Instant::now(),
            visible_cache: RefCell::default(),
            groups: config.groups.clone(),
            window_platforms: Vec::new(),
            reduced_motion: false,
            hover_startle: false,
        }
    }

    /// Cap every frame at `MAX_DROP_SIZE`, the same overlay-friendly size
    /// `add_entity_from_path` applies when an asset is first dropped.
    /// Startup load must apply it too, or a sprite decoded from disk comes
    /// back at its original resolution: a 1024² asset shown at 256² during
    /// the drop session reloaded at 1024² after a restart, shifting its
    /// placement and inflating RAM/VRAM. `?`-propagates a corrupt buffer
    /// rather than fabricating a mismatched frame.
    fn cap_to_drop_size(
        frames: Vec<crate::animation::frame::Frame>,
    ) -> Result<Vec<crate::animation::frame::Frame>> {
        frames
            .into_iter()
            .map(|f| f.resized(MAX_DROP_SIZE))
            .collect()
    }

    /// Load a single entity from config. The legacy top-level asset
    /// fields define the `Idle` state; `[characters.animations.*]`
    /// tables add further states (U.1). A state that fails to load is
    /// skipped with a warning — the `Idle` fallback covers it — so a
    /// missing walk sequence can't take the whole entity down.
    fn load_entity(config: &CharacterConfig) -> Result<Entity> {
        let resolved_path = AppConfig::resolve_asset_path(&config.asset_path);
        let frames = load_asset(
            &config.asset_type,
            &resolved_path,
            config.spritesheet_columns,
            config.spritesheet_rows,
        )?;
        let frames = Self::cap_to_drop_size(frames)?;
        let idle = Animation::new(frames, config.fps, config.playing);
        let mut set = crate::animation::AnimationSet::single(idle);

        for (state, scfg) in &config.animations {
            if *state == crate::animation::StateId::Idle {
                // The legacy fields are the one canonical Idle source.
                tracing::warn!(
                    "Entity '{}': `animations.idle` is ignored — the top-level \
                     asset fields define the idle state",
                    config.id
                );
                continue;
            }
            let resolved = AppConfig::resolve_asset_path(&scfg.asset_path);
            let loaded = load_asset(
                &scfg.asset_type,
                &resolved,
                scfg.spritesheet_columns,
                scfg.spritesheet_rows,
            )
            .and_then(Self::cap_to_drop_size);
            match loaded {
                Ok(frames) => {
                    let fps = scfg.fps.unwrap_or(config.fps);
                    set.insert(*state, Animation::new(frames, fps, config.playing));
                }
                Err(e) => {
                    tracing::warn!(
                        "Entity '{}': state {:?} failed to load ({e}); falling back to idle",
                        config.id,
                        state
                    );
                }
            }
        }
        Ok(Entity::from_config_set(config, set))
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

    /// Update the reduced-motion preference (cheap, called per frame).
    pub fn set_reduced_motion(&mut self, reduced: bool) {
        self.reduced_motion = reduced;
    }

    pub fn set_hover_startle(&mut self, enabled: bool) {
        self.hover_startle = enabled;
    }

    /// Replace the desktop-window platform set (window-awareness).
    /// Called from the render loop after each X11 window poll; pass
    /// an empty vec to turn the feature's effect off instantly.
    pub fn set_window_platforms(&mut self, platforms: Vec<crate::platforms::PlatformRect>) {
        self.window_platforms = platforms;
    }

    /// Whether any entity is currently running `Behavior::FollowCursor`.
    /// Lets the render loop skip the extra X11 `XQueryPointer` round
    /// trip on every frame when nothing needs a live cursor position.
    pub fn has_cursor_follower(&self) -> bool {
        self.entities
            .iter()
            .any(|e| matches!(e.behavior, crate::behavior::Behavior::FollowCursor { .. }))
    }

    /// Tick all entities: behavior + physics + animation.
    /// Screen dimensions bound autonomous motion (walk-around) and gravity.
    /// `cursor` is forwarded to behaviors that track the mouse (FollowCursor);
    /// pass `None` when the position is stale or unknown.
    pub fn tick(&mut self, bounds: crate::monitor::DesktopBounds, cursor: Option<(f32, f32)>) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f32();
        self.last_tick = now;

        // Clamp dt to prevent physics / behavior explosion after long pauses.
        let dt = dt.min(0.1);

        if !self.global_playing {
            return;
        }

        for entity in &mut self.entities {
            entity.tick(
                dt,
                bounds,
                cursor,
                &self.window_platforms,
                self.reduced_motion,
                self.hover_startle,
            );
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
                    .extend((0..self.entities.len()).filter(|&i| self.effective_visible(i)));
                cache.indices.sort_by_key(|&i| self.entities[i].z_index);
                cache.valid = true;
            }
        }
        let cache = self.visible_cache.borrow();
        cache.indices.iter().map(|&i| &self.entities[i]).collect()
    }

    /// Resolve effective visibility for the entity at `idx`, folding
    /// the entity's own `visible` flag with any owning group's
    /// `visible` flag (`crate::group::visible_for_member`). Used by
    /// the visibility cache and the hit-test below so a group hidden
    /// by the user doesn't catch clicks either.
    fn effective_visible(&self, idx: usize) -> bool {
        let entity = &self.entities[idx];
        if !entity.visible {
            return false;
        }
        if self.groups.is_empty() {
            return true;
        }
        crate::group::visible_for_member(&self.groups, &entity.id, entity.visible)
    }

    /// Find the topmost entity at a screen position (reverse z-order).
    /// Honours group visibility — an entity in a hidden group can't
    /// be clicked.
    pub fn entity_at_point(&self, x: f32, y: f32) -> Option<usize> {
        // Check in reverse z-order (topmost first)
        let mut indices: Vec<usize> = (0..self.entities.len())
            .filter(|&i| self.effective_visible(i))
            .collect();
        indices.sort_by(|&a, &b| self.entities[b].z_index.cmp(&self.entities[a].z_index));

        indices.into_iter().find(|&idx| {
            let e = &self.entities[idx];
            // C.9: hit-test where the renderer paints — the owning
            // group's offset/scale shift the visible quad.
            let (gx, gy, gscale) = crate::group::transform_for_member(&self.groups, &e.id);
            e.contains_point_composed(x, y, (gx, gy), gscale)
        })
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

    /// A fresh `{stem}_{n}` id no current entity holds, probing `n` upward
    /// from the entity count. Entity GPU textures are keyed by id
    /// (`GpuShared::textures`), so two entities sharing an id alias each
    /// other's texture — one upload clobbers the other, and pruning one
    /// frees the survivor's. `entities.len()` alone is not collision-free
    /// across deletes (add a,b,c → _0,_1,_2; delete _1 → len 2; add → _2,
    /// aliasing the existing _2), so mint fresh ids only through here.
    fn fresh_entity_id(&self, stem: &str) -> String {
        let mut n = self.entities.len();
        loop {
            let candidate = format!("{stem}_{n}");
            if self.entities.iter().all(|e| e.id != candidate) {
                return candidate;
            }
            n += 1;
        }
    }

    /// Make `desired` unique among current entity ids: returned unchanged
    /// when free, otherwise suffixed `-2`, `-3`, … until it finds a gap.
    /// Used by the duplicate/import paths, which start from an existing
    /// id. Entity GPU textures are keyed by id, so a duplicate id makes
    /// two entities alias one texture. The old `-{len}` suffix those
    /// sites used was itself not verified unique; this probes until it
    /// actually is.
    pub fn unique_id(&self, desired: &str) -> String {
        if self.entities.iter().all(|e| e.id != desired) {
            return desired.to_string();
        }
        let mut n = 2u32;
        loop {
            let candidate = format!("{desired}-{n}");
            if self.entities.iter().all(|e| e.id != candidate) {
                return candidate;
            }
            n += 1;
        }
    }

    /// Add a new entity by loading an asset from a file path.
    /// Auto-detects the asset type from the extension.
    /// Returns the index of the new entity, or an error if loading fails.
    #[tracing::instrument(skip(self), fields(path = %crate::drop_validate::redact_path(path)))]
    pub fn add_entity_from_path(
        &mut self,
        path: &std::path::Path,
        x: f32,
        y: f32,
    ) -> Result<usize> {
        use crate::animation::loader::detect_asset_type;

        // Cap total entities at MAX_ENTITIES at every runtime push site —
        // `AppConfig::load` enforces this on disk-load but drag-drop,
        // library add, and duplicate ultimately reach here and used to
        // bypass the cap. A drag-flood from a hostile compositor could
        // otherwise spawn arbitrary entities until OOM.
        if self.entities.len() >= MAX_ENTITIES {
            return Err(AnimaError::other(format!(
                "entity limit reached ({MAX_ENTITIES}); remove some before adding more"
            )));
        }

        let (asset_type, type_desc) = detect_asset_type(path);

        // Generate a unique ID from the filename
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("entity");
        let id = self.fresh_entity_id(stem);
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
            crate::drop_validate::redact_path(&abs_path),
            type_desc
        );
        tracing::debug!("Entity asset full path: {asset_path_str}");

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
            monitor: None,
            easing: None,
            animations: std::collections::BTreeMap::new(),
        };

        // Load frames and resize to overlay-friendly dimensions
        let resolved = AppConfig::resolve_asset_path(&char_config.asset_path);
        let frames = load_asset(
            &char_config.asset_type,
            &resolved,
            char_config.spritesheet_columns,
            char_config.spritesheet_rows,
        )?;

        // Cap frames at MAX_DROP_SIZE for overlay-friendly sprites — the
        // same cap `load_entity` applies on startup so the sprite is the
        // same size across restarts. `?` propagates a corrupt-buffer error
        // instead of silently producing a frame with mismatched dimensions.
        let frames = Self::cap_to_drop_size(frames)?;

        let animation = Animation::new(frames, char_config.fps, char_config.playing);
        let entity = Entity::from_config(&char_config, animation);

        // Aggregate memory budget — per-asset cap × MAX_ENTITIES is a
        // 32 GB worst case, so we also gate the total here. Decoded
        // frames go out of scope on early-return and free their RGBA
        // buffers; no leak.
        check_budget(
            self.total_decoded_bytes(),
            entity.animations.decoded_bytes(),
            max_total_decoded_bytes(),
        )?;

        tracing::info!(
            "Entity '{}' loaded: {} frames (max {}px)",
            entity.id,
            entity.animation().frame_count(),
            MAX_DROP_SIZE
        );

        self.entities.push(entity);
        self.mark_visible_dirty();
        let idx = self.entities.len() - 1;
        Ok(idx)
    }

    /// Sum of decoded-RGBA bytes across every loaded entity. Used by the
    /// aggregate memory-budget check on every runtime push.
    pub fn total_decoded_bytes(&self) -> usize {
        self.entities
            .iter()
            .map(|e| e.animations.decoded_bytes())
            .fold(0usize, |acc, n| acc.saturating_add(n))
    }

    /// Remove an entity by index. Returns the removed entity's ID.
    /// Also scrubs the entity's id from every group's `member_ids`
    /// so dangling references can't survive — the C.8 invariant.
    pub fn remove_entity(&mut self, index: usize) -> Option<String> {
        if index < self.entities.len() {
            let entity = self.entities.remove(index);
            tracing::info!("Removed entity '{}' ({})", entity.name, entity.id);
            crate::group::cleanup_after_entity_removal(&mut self.groups, &entity.id);
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
        if self.entities.len() >= MAX_ENTITIES {
            return Err(AnimaError::other(format!(
                "entity limit reached ({MAX_ENTITIES}); remove some before adding more"
            )));
        }
        let entity = Self::load_entity(cfg)?;

        check_budget(
            self.total_decoded_bytes(),
            entity.animations.decoded_bytes(),
            max_total_decoded_bytes(),
        )?;

        self.entities.push(entity);
        self.mark_visible_dirty();
        Ok(())
    }
}

/// Enforce the aggregate decoded-RGBA budget. Pure function so call
/// sites can pass a synthetic budget in tests without touching the
/// `ANIMA_MEMORY_BUDGET_MB` env var.
fn check_budget(current: usize, incoming: usize, budget: usize) -> Result<()> {
    if current.saturating_add(incoming) > budget {
        return Err(AnimaError::other(format!(
            "memory budget exceeded ({} MB used + {} MB incoming > {} MB cap); \
             set ANIMA_MEMORY_BUDGET_MB to raise",
            current / (1024 * 1024),
            incoming / (1024 * 1024),
            budget / (1024 * 1024),
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::frame::Frame;
    use crate::config::{AssetType, GlobalConfig};

    pub(super) fn make_entity(id: &str, z: i32, visible: bool) -> Entity {
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
            monitor: None,
            easing: None,
            animations: std::collections::BTreeMap::new(),
        };
        Entity::from_config(&cfg, anim)
    }

    pub(super) fn empty_scene() -> Scene {
        let config = AppConfig {
            global: GlobalConfig::default(),
            characters: vec![],
            windows: vec![],
            groups: vec![],
            keybindings: crate::keybindings::KeyBindings::default(),
            collapse_state: crate::ui::CollapseState::default(),
            schema_version: crate::config::CURRENT_SCHEMA_VERSION,
            extra: toml::Table::new(),
        };
        Scene::from_config(&config)
    }

    #[test]
    fn has_cursor_follower_detects_only_follow_cursor_behavior() {
        let mut scene = empty_scene();
        assert!(!scene.has_cursor_follower(), "empty scene has no follower");

        scene.entities.push(make_entity("idle", 0, true));
        assert!(!scene.has_cursor_follower(), "Idle isn't a follower");

        let mut follower = make_entity("follower", 0, true);
        follower.behavior = crate::behavior::Behavior::FollowCursor {
            speed: 100.0,
            comfort_distance: 80.0,
        };
        scene.entities.push(follower);
        assert!(scene.has_cursor_follower());
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
    fn fresh_entity_id_skips_ids_left_by_a_middle_delete() {
        // add cat_0, cat_1, cat_2 then delete the middle → cat_0, cat_2.
        // len is now 2, so the old `{stem}_{len}` scheme would mint cat_2
        // again and alias the survivor's texture. Probing must skip it.
        let mut scene = empty_scene();
        scene.entities.push(make_entity("cat_0", 0, true));
        scene.entities.push(make_entity("cat_2", 0, true));
        assert_eq!(scene.fresh_entity_id("cat"), "cat_3");
    }

    #[test]
    fn fresh_entity_id_uses_count_when_no_collision() {
        let mut scene = empty_scene();
        scene.entities.push(make_entity("cat_0", 0, true));
        scene.entities.push(make_entity("cat_1", 0, true));
        assert_eq!(scene.fresh_entity_id("cat"), "cat_2");
    }

    #[test]
    fn unique_id_suffixes_until_free() {
        let mut scene = empty_scene();
        scene.entities.push(make_entity("cat_0", 0, true));
        scene.entities.push(make_entity("cat_0-2", 0, true));
        // Original taken, -2 taken → -3. The old `-{len}` (len 2 → -2)
        // would have collided with the existing duplicate.
        assert_eq!(scene.unique_id("cat_0"), "cat_0-3");
        // A free id is returned unchanged.
        assert_eq!(scene.unique_id("dog_0"), "dog_0");
    }

    #[test]
    fn total_decoded_bytes_empty_scene_is_zero() {
        let scene = empty_scene();
        assert_eq!(scene.total_decoded_bytes(), 0);
    }

    #[test]
    fn total_decoded_bytes_sums_across_entities() {
        // make_entity ships one 1×1 RGBA frame = 4 bytes.
        let mut scene = empty_scene();
        scene.entities.push(make_entity("a", 0, true));
        scene.entities.push(make_entity("b", 0, true));
        scene.entities.push(make_entity("c", 0, true));
        assert_eq!(scene.total_decoded_bytes(), 12);
    }

    #[test]
    fn check_budget_accepts_under_cap() {
        assert!(check_budget(100, 200, 1000).is_ok());
    }

    #[test]
    fn check_budget_accepts_exact_cap() {
        // Boundary: hitting the cap exactly is fine; only strict
        // overflow is rejected.
        assert!(check_budget(600, 400, 1000).is_ok());
    }

    #[test]
    fn check_budget_rejects_over_cap() {
        let err = check_budget(700, 400, 1000).unwrap_err();
        assert!(err.to_string().contains("memory budget"), "got: {err}",);
    }

    #[test]
    fn check_budget_handles_overflow_safely() {
        // Saturating add must not wrap to 0 and let this slide; the
        // strict > check then catches it.
        let err = check_budget(usize::MAX - 10, 20, 1000).unwrap_err();
        assert!(err.to_string().contains("memory budget"));
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

#[cfg(test)]
mod groups_tests {
    use super::tests::{empty_scene, make_entity};
    use crate::group::GroupConfig;

    fn make_group(id: &str, members: &[&str], visible: bool) -> GroupConfig {
        GroupConfig {
            id: id.into(),
            name: id.into(),
            member_ids: members.iter().map(|s| (*s).to_string()).collect(),
            offset_x: 0.0,
            offset_y: 0.0,
            scale: 1.0,
            visible,
        }
    }

    /// Member of a hidden group is filtered out of `visible_entities`,
    /// even though its own `visible` flag is true.
    #[test]
    fn hidden_group_hides_its_members() {
        let mut scene = empty_scene();
        scene.entities.push(make_entity("ghost", 10, true));
        scene.entities.push(make_entity("cat", 20, true));
        scene.groups.push(make_group("party", &["ghost"], false));
        scene.mark_visible_dirty();

        let ids: Vec<&str> = scene
            .visible_entities()
            .iter()
            .map(|e| e.id.as_str())
            .collect();
        assert_eq!(ids, vec!["cat"]);
    }

    /// Removing an entity must scrub its id from every group's
    /// `member_ids` so dangling references can't survive.
    #[test]
    fn remove_entity_cleans_up_group_membership() {
        let mut scene = empty_scene();
        scene.entities.push(make_entity("ghost", 0, true));
        scene.entities.push(make_entity("cat", 0, true));
        scene
            .groups
            .push(make_group("party", &["ghost", "cat"], true));

        // remove "ghost" (index 0).
        scene.remove_entity(0);
        assert_eq!(scene.groups[0].member_ids, vec!["cat".to_string()]);
    }

    /// A click on a hidden-by-group entity must NOT select it —
    /// otherwise the user could pick up sprites they can't see.
    #[test]
    fn entity_at_point_skips_hidden_group_members() {
        let mut scene = empty_scene();
        // Make ghost large enough to be clickable at (50, 50).
        let mut ghost = make_entity("ghost", 0, true);
        // for_test uses x=0, y=0 + a 1-frame Animation. Reposition
        // by mutating the public field directly.
        ghost.x = 0.0;
        ghost.y = 0.0;
        scene.entities.push(ghost);
        scene.groups.push(make_group("party", &["ghost"], false));

        // entity_at_point should now skip the ghost even though its
        // own `visible` is true.
        assert!(scene.entity_at_point(0.0, 0.0).is_none());
    }

    /// Empty groups are valid (user just made one but hasn't added
    /// members yet) — scene loads them without complaint.
    #[test]
    fn empty_group_is_no_op_on_visibility() {
        let mut scene = empty_scene();
        scene.entities.push(make_entity("ghost", 0, true));
        scene.groups.push(make_group("empty_party", &[], true));
        scene.mark_visible_dirty();

        let ids: Vec<&str> = scene
            .visible_entities()
            .iter()
            .map(|e| e.id.as_str())
            .collect();
        assert_eq!(ids, vec!["ghost"]);
    }
}
