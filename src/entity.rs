use crate::animation::{Animation, AnimationSet, StateId};
use crate::behavior::{Behavior, BehaviorState, TickContext};
use crate::config::CharacterConfig;
use crate::physics::PhysicsState;

/// Below this horizontal speed (px per tick) an entity counts as
/// standing still — keeps float jitter from flapping Walk/Idle.
const FACING_EPSILON: f32 = 0.01;

/// Pick the animation state for this tick (U.2). Pure so the
/// priority table is unit-testable.
fn desired_state(dragging: bool, falling: bool, dx: f32) -> StateId {
    if dragging {
        StateId::Drag
    } else if falling {
        StateId::Fall
    } else if dx.abs() > FACING_EPSILON {
        StateId::Walk
    } else {
        StateId::Idle
    }
}

/// A single animated entity on screen.
/// Represents one character/asset with its position, appearance, and animation state.
#[derive(Debug)]
pub struct Entity {
    /// Unique identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Position on screen (pixels from top-left)
    pub x: f32,
    pub y: f32,
    /// Scale factor (1.0 = original size)
    pub scale: f32,
    /// Opacity (0.0 = invisible, 1.0 = fully opaque)
    pub opacity: f32,
    /// Z-ordering (higher = on top)
    pub z_index: i32,
    /// Whether this entity is visible
    pub visible: bool,
    /// Per-state animations with one active state (U.1). Single-state
    /// for everything that isn't a multi-state import; access the
    /// active sequence through [`Entity::animation`] /
    /// [`Entity::animation_mut`].
    pub animations: AnimationSet,
    /// Passthrough of the config's per-state sources so `to_config`
    /// round-trips them without the entity re-deriving anything.
    pub state_configs:
        std::collections::BTreeMap<crate::animation::StateId, crate::config::StateSequenceConfig>,
    /// Whether the GPU texture needs updating (frame changed)
    pub texture_dirty: bool,
    /// The original asset path (for config saving)
    pub asset_path: String,
    /// Asset type (for config saving)
    pub asset_type: crate::config::AssetType,
    /// Spritesheet columns (for config saving, only used for Spritesheet type)
    pub spritesheet_columns: Option<u32>,
    /// Spritesheet rows (for config saving, only used for Spritesheet type)
    pub spritesheet_rows: Option<u32>,
    /// Optional monitor pin — mirrors `CharacterConfig::monitor`.
    /// `None` means "resolve via centroid". The renderer reads this
    /// each frame; the inspector mutates it when the user explicitly
    /// chooses a monitor in the picker.
    pub monitor: Option<String>,
    /// Physics state (gravity, velocity, grounded)
    pub physics: PhysicsState,
    /// Autonomous motion configuration.
    pub behavior: Behavior,
    /// Behavior runtime accumulators (direction, timers). Not serialized.
    pub behavior_state: BehaviorState,
    /// `true` while the user drags this entity (set by the input
    /// layer). Drives the `Drag` animation state (U.2). Runtime-only.
    pub dragging: bool,
    /// Horizontal facing, updated from behavior motion. Persists
    /// across states so an entity that stops walking left keeps
    /// facing left. The renderer mirrors the sprite when `true`
    /// (art is assumed right-facing; importers can pre-flip).
    pub facing_left: bool,
}

impl Entity {
    /// Create an entity from config + the idle animation. Convenience
    /// wrapper over [`Entity::from_config_set`] for the single-state
    /// callers (drag-drop, presets, every pre-U.1 path).
    pub fn from_config(config: &CharacterConfig, animation: Animation) -> Self {
        Self::from_config_set(config, AnimationSet::single(animation))
    }

    /// Create an entity from config + a full animation set (U.1).
    pub fn from_config_set(config: &CharacterConfig, mut animations: AnimationSet) -> Self {
        // Seed the behavior RNG from the entity id so two BoundedWander
        // characters created from the same template don't trace identical
        // paths. Same-id reloads still get the same seed → deterministic.
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(&config.id, &mut hasher);
        let seed = std::hash::Hasher::finish(&hasher);

        // Easing applies to the active (idle) sequence — per-state
        // easing is not a thing; GIF/WebP per-frame delays in any
        // state stay authoritative regardless.
        animations.current_mut().easing = config.easing;

        Self {
            id: config.id.clone(),
            name: config.name.clone(),
            x: config.x,
            y: config.y,
            scale: config.scale,
            opacity: config.opacity,
            z_index: config.z_index,
            visible: config.visible,
            animations,
            state_configs: config.animations.clone(),
            texture_dirty: true, // Needs initial texture upload
            asset_path: config.asset_path.clone(),
            asset_type: config.asset_type.clone(),
            spritesheet_columns: config.spritesheet_columns,
            spritesheet_rows: config.spritesheet_rows,
            monitor: config.monitor.clone(),
            physics: PhysicsState::from_enabled(config.physics_enabled),
            behavior: config.behavior.clone(),
            behavior_state: BehaviorState::with_seed(seed),
            dragging: false,
            facing_left: false,
        }
    }

    /// The active state's animation.
    pub fn animation(&self) -> &Animation {
        self.animations.current()
    }

    /// Mutable access to the active state's animation.
    pub fn animation_mut(&mut self) -> &mut Animation {
        self.animations.current_mut()
    }

    /// Tick the entity: behavior + physics + animation.
    ///
    /// Order matters: behavior moves the entity (possibly in 2D), then
    /// physics resolves vertical motion if enabled (gravity wins on Y),
    /// then the animation advances. `cursor` is the screen-space mouse
    /// position if known — needed by `FollowCursor`.
    pub fn tick(
        &mut self,
        dt: f32,
        screen_width: f32,
        screen_height: f32,
        cursor: Option<(f32, f32)>,
        platforms: &[crate::platforms::PlatformRect],
        reduced_motion: bool,
    ) -> bool {
        let sprite_w = self.scaled_width();
        let sprite_h = self.scaled_height();
        let ctx = TickContext {
            sprite_width: sprite_w,
            sprite_height: sprite_h,
            screen_width,
            screen_height,
            cursor,
            dt,
            reduced_motion,
        };

        // Behavior — autonomous motion (can affect both X and Y).
        let x_before = self.x;
        self.behavior
            .tick(&mut self.behavior_state, &mut self.x, &mut self.y, &ctx);
        let dx = self.x - x_before;

        // Physics — gravity / bounce on the vertical axis. When enabled
        // this overrides whatever Y the behavior set. The floor is the
        // screen bottom or, with window-awareness on, the top of the
        // highest desktop window under the entity's feet. Tolerance
        // differs by state: grounded entities track their platform
        // (ride a slowly moved window), airborne ones only land on
        // tops at/below the feet — no mid-fall upward snapping.
        let tolerance = if self.physics.grounded {
            crate::platforms::RIDE_TOLERANCE
        } else {
            crate::platforms::LAND_TOLERANCE
        };
        let floor_feet = crate::platforms::effective_floor(
            platforms,
            self.x + sprite_w / 2.0,
            self.y + sprite_h,
            screen_height,
            tolerance,
        );
        let floor = floor_feet - sprite_h;
        self.physics.release_if_floor_dropped(self.y, floor);
        self.y = self.physics.tick(self.y, floor, dt);

        // Facing follows horizontal motion; standing still keeps the
        // last direction (U.2).
        if dx.abs() > FACING_EPSILON {
            self.facing_left = dx < 0.0;
        }

        // Animation state selection (U.2). Priority: a drag overrides
        // everything the entity does on its own; falling overrides
        // locomotion; horizontal motion plays Walk; otherwise Idle.
        // Missing states fall back to Idle inside the set, so this is
        // safe for single-state entities (the switch is then a no-op).
        let falling = self.physics.enabled && !self.physics.grounded;
        let desired = desired_state(self.dragging, falling, dx);
        if self.animations.switch(desired) {
            self.texture_dirty = true;
        }

        // Animation frame advance.
        if self.animations.current_mut().tick() {
            self.texture_dirty = true;
            return true;
        }
        false
    }

    /// Get the current frame dimensions (scaled)
    pub fn scaled_width(&self) -> f32 {
        self.animation()
            .current_frame_data()
            .map(|f| f.width as f32 * self.scale)
            .unwrap_or(64.0)
    }

    pub fn scaled_height(&self) -> f32 {
        self.animation()
            .current_frame_data()
            .map(|f| f.height as f32 * self.scale)
            .unwrap_or(64.0)
    }

    /// Check if a point (in screen coords) hits this entity.
    ///
    /// First does a fast AABB rejection, then samples the alpha of the
    /// underlying pixel in the current animation frame. Transparent pixels
    /// (alpha < ALPHA_HIT_THRESHOLD) do NOT count as a hit — clicking the
    /// transparent corner of a circular ghost sprite no longer selects it.
    ///
    /// Falls back to AABB when no frame data is available (e.g. immediately
    /// after construction before the first tick).
    pub fn contains_point(&self, px: f32, py: f32) -> bool {
        self.contains_point_composed(px, py, (0.0, 0.0), 1.0)
    }

    /// [`Entity::contains_point`] under a group transform (C.9): the
    /// drawn quad sits at `pos + offset` with `scale × scale_mul`, so
    /// the hit test must look where the renderer actually painted.
    pub fn contains_point_composed(
        &self,
        px: f32,
        py: f32,
        offset: (f32, f32),
        scale_mul: f32,
    ) -> bool {
        /// Pixels with alpha at or below this are treated as non-hittable.
        /// Small but non-zero so anti-aliased edges remain clickable.
        const ALPHA_HIT_THRESHOLD: u8 = 20;

        if !self.visible {
            return false;
        }
        let eff_x = self.x + offset.0;
        let eff_y = self.y + offset.1;
        let eff_scale = self.scale * scale_mul;
        let w = self.scaled_width() * scale_mul;
        let h = self.scaled_height() * scale_mul;

        // Fast AABB reject.
        if px < eff_x || px > eff_x + w || py < eff_y || py > eff_y + h {
            return false;
        }

        let Some(frame) = self.animation().current_frame_data() else {
            return true; // No pixel data yet — accept the AABB hit.
        };

        if eff_scale <= 0.0 || frame.width == 0 || frame.height == 0 {
            return true;
        }

        // Map screen-space → texture-space.
        let tex_x = ((px - eff_x) / eff_scale)
            .floor()
            .clamp(0.0, (frame.width - 1) as f32) as u32;
        let tex_y = ((py - eff_y) / eff_scale)
            .floor()
            .clamp(0.0, (frame.height - 1) as f32) as u32;

        let idx = (tex_y * frame.width + tex_x) as usize * 4;
        // Defensive: corrupted frame data shouldn't crash a click.
        let alpha = frame.rgba.get(idx + 3).copied().unwrap_or(0);
        alpha > ALPHA_HIT_THRESHOLD
    }

    /// Construct a minimal entity for tests — bypasses CharacterConfig.
    #[cfg(test)]
    fn for_test(x: f32, y: f32, animation: Animation) -> Self {
        Self {
            id: "t".into(),
            name: "t".into(),
            x,
            y,
            scale: 1.0,
            opacity: 1.0,
            z_index: 0,
            visible: true,
            animations: AnimationSet::single(animation),
            state_configs: std::collections::BTreeMap::new(),
            texture_dirty: false,
            asset_path: String::new(),
            asset_type: crate::config::AssetType::PngStatic,
            spritesheet_columns: None,
            spritesheet_rows: None,
            monitor: None,
            physics: PhysicsState::default(),
            behavior: Behavior::Idle,
            behavior_state: BehaviorState::default(),
            dragging: false,
            facing_left: false,
        }
    }

    /// Convert this entity's state back to a CharacterConfig for saving
    pub fn to_config(&self) -> CharacterConfig {
        CharacterConfig {
            id: self.id.clone(),
            name: self.name.clone(),
            asset_type: self.asset_type.clone(),
            asset_path: self.asset_path.clone(),
            x: self.x,
            y: self.y,
            scale: self.scale,
            opacity: self.opacity,
            fps: self.animation().fps,
            visible: self.visible,
            playing: self.animation().playing,
            z_index: self.z_index,
            physics_enabled: self.physics.enabled,
            behavior: self.behavior.clone(),
            spritesheet_columns: self.spritesheet_columns,
            spritesheet_rows: self.spritesheet_rows,
            monitor: self.monitor.clone(),
            easing: self.animation().easing,
            animations: self.state_configs.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::frame::Frame;

    /// 4×4 frame, only the inner 2×2 is opaque (alpha 255), border is transparent.
    fn checker_frame() -> Frame {
        // Layout (alpha only, R/G/B = 0):
        //   . . . .
        //   . X X .
        //   . X X .
        //   . . . .
        let mut rgba = vec![0u8; 4 * 4 * 4];
        for (x, y) in [(1u32, 1u32), (2, 1), (1, 2), (2, 2)] {
            let i = ((y * 4 + x) * 4) as usize;
            rgba[i + 3] = 255;
        }
        Frame::new(rgba, 4, 4)
    }

    fn entity_at(x: f32, y: f32) -> Entity {
        let anim = Animation::new(vec![checker_frame()], 1.0, false);
        Entity::for_test(x, y, anim)
    }

    #[test]
    fn state_priority_drag_beats_everything() {
        assert_eq!(desired_state(true, true, 5.0), StateId::Drag);
        assert_eq!(desired_state(true, false, 0.0), StateId::Drag);
    }

    #[test]
    fn state_priority_fall_beats_walk() {
        assert_eq!(desired_state(false, true, 5.0), StateId::Fall);
    }

    #[test]
    fn state_walk_requires_motion_above_epsilon() {
        assert_eq!(desired_state(false, false, 0.5), StateId::Walk);
        assert_eq!(desired_state(false, false, -0.5), StateId::Walk);
        assert_eq!(desired_state(false, false, 0.005), StateId::Idle);
        assert_eq!(desired_state(false, false, 0.0), StateId::Idle);
    }

    #[test]
    fn alpha_hit_inside_opaque_region() {
        let e = entity_at(0.0, 0.0);
        // Pixel (1,1) is opaque; click at (1.5, 1.5) → tex (1,1).
        assert!(e.contains_point(1.5, 1.5));
    }

    #[test]
    fn alpha_miss_in_transparent_corner() {
        let e = entity_at(0.0, 0.0);
        // Pixel (0,0) is fully transparent — this is the bug we just fixed.
        assert!(!e.contains_point(0.5, 0.5));
    }

    #[test]
    fn aabb_reject_outside_sprite() {
        let e = entity_at(10.0, 10.0);
        assert!(!e.contains_point(0.0, 0.0));
        assert!(!e.contains_point(100.0, 100.0));
    }

    #[test]
    fn invisible_entity_never_hits() {
        let mut e = entity_at(0.0, 0.0);
        e.visible = false;
        assert!(!e.contains_point(1.5, 1.5)); // would otherwise hit
    }

    #[test]
    fn scale_maps_screen_to_texture_coords() {
        let mut e = entity_at(0.0, 0.0);
        e.scale = 4.0; // sprite is now 16×16 on screen, source still 4×4.
                       // Screen (6, 6) → tex (6/4, 6/4) = (1, 1) → opaque.
        assert!(e.contains_point(6.0, 6.0));
        // Screen (2, 2) → tex (0, 0) → transparent.
        assert!(!e.contains_point(2.0, 2.0));
    }

    #[test]
    fn offset_position_still_maps_correctly() {
        let e = entity_at(100.0, 50.0);
        // World (101.5, 51.5) → local (1.5, 1.5) → tex (1, 1) → opaque.
        assert!(e.contains_point(101.5, 51.5));
        // World (100.5, 50.5) → tex (0, 0) → transparent.
        assert!(!e.contains_point(100.5, 50.5));
    }

    #[test]
    fn no_frame_data_falls_back_to_aabb() {
        // Empty animation → current_frame_data() returns None.
        let anim = Animation::new(vec![], 1.0, false);
        let mut e = Entity::for_test(0.0, 0.0, anim);
        // scaled_width/height default to 64.0 when no frame.
        e.scale = 1.0;
        assert!(e.contains_point(32.0, 32.0));
        assert!(!e.contains_point(100.0, 100.0));
    }
}
