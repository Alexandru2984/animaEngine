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

    /// React to a poke (a tap on this entity): recoil horizontally away
    /// from the poke point, and — if physics is on — hop. `from_x` is the
    /// poke's x in the same space as the entity; `bounds` clamps the
    /// recoil so a mascot at the edge can't be shoved off the desktop.
    pub fn poke(&mut self, from_x: f32, bounds: crate::monitor::DesktopBounds) {
        let center_x = self.x + self.scaled_width() * 0.5;
        let dir = if (center_x - from_x).abs() < 1e-3 {
            1.0
        } else {
            (center_x - from_x).signum()
        };
        self.x = bounds.clamp_x(
            self.x + dir * crate::constants::POKE_KICK,
            self.scaled_width(),
        );
        self.physics.poke_hop();
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
    // The per-frame entity update genuinely takes the frame's inputs (dt,
    // screen dims, cursor, physics platforms) plus the global interaction
    // toggles; grouping them into a struct would only move the argument
    // list around without making a call site clearer.
    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        &mut self,
        dt: f32,
        bounds: crate::monitor::DesktopBounds,
        cursor: Option<(f32, f32)>,
        platforms: &[crate::platforms::PlatformRect],
        reduced_motion: bool,
        hover_startle: bool,
    ) -> bool {
        let sprite_w = self.scaled_width();
        let sprite_h = self.scaled_height();
        let ctx = TickContext {
            sprite_width: sprite_w,
            sprite_height: sprite_h,
            bounds,
            cursor,
            dt,
            reduced_motion,
        };

        // Behavior — autonomous motion (can affect both X and Y).
        let x_before = self.x;
        self.behavior
            .tick(&mut self.behavior_state, &mut self.x, &mut self.y, &ctx);

        // Hover-startle: recoil from a cursor that comes too close.
        // Orthogonal to the base behavior (a walking mascot still
        // flinches) and radius-gated so it only fires near the pointer.
        // Applied before `dx` so facing follows the recoil; physics
        // re-governs Y for grounded entities, so the visible effect is a
        // scoot away. Suppressed under reduced motion.
        if hover_startle && !reduced_motion {
            if let Some((cx, cy)) = cursor {
                let (px, py) = hover_startle_push(
                    self.x + sprite_w * 0.5,
                    self.y + sprite_h * 0.5,
                    cx,
                    cy,
                    dt,
                );
                self.x = bounds.clamp_x(self.x + px, sprite_w);
                self.y += py;
            }
        }

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
            bounds.max_y,
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

        // The renderer mirrors the sprite horizontally when `facing_left`
        // (`make_quad_vertices` swaps the U coordinates), so the pixel
        // actually drawn at this screen column comes from the mirrored
        // texture column. Sampling the unmirrored column tests the
        // opposite side of the sprite — for an asymmetric one, its
        // transparent corners stop matching its clickable area.
        let tex_x = if self.facing_left {
            frame.width - 1 - tex_x
        } else {
            tex_x
        };

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

/// Recoil push for hover-startle: the (dx, dy) to add to an entity whose
/// centre is at (`ecx`, `ecy`) when the cursor is at (`cx`, `cy`). Zero
/// outside [`crate::constants::HOVER_STARTLE_RADIUS`]; inside, it points away from the
/// cursor and scales from `HOVER_STARTLE_SPEED · dt` at the centre down
/// to zero at the edge. A cursor exactly on the centre pushes right
/// (arbitrary, avoids a divide-by-zero).
fn hover_startle_push(ecx: f32, ecy: f32, cx: f32, cy: f32, dt: f32) -> (f32, f32) {
    use crate::constants::{HOVER_STARTLE_RADIUS, HOVER_STARTLE_SPEED};
    let (dx, dy) = (ecx - cx, ecy - cy);
    let dist = (dx * dx + dy * dy).sqrt();
    if dist >= HOVER_STARTLE_RADIUS {
        return (0.0, 0.0);
    }
    let strength = HOVER_STARTLE_SPEED * dt * (1.0 - dist / HOVER_STARTLE_RADIUS);
    if dist < 1e-3 {
        return (strength, 0.0); // cursor on centre → arbitrary push right
    }
    let inv = 1.0 / dist;
    (dx * inv * strength, dy * inv * strength)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::frame::Frame;

    #[test]
    fn hover_startle_pushes_away_within_radius_and_is_inert_outside() {
        use crate::constants::HOVER_STARTLE_RADIUS;
        let dt = 1.0 / 60.0;

        // Cursor to the LEFT of the entity centre → push is to the right
        // (positive x), away from the cursor, and vertically negligible.
        let (px, py) = hover_startle_push(100.0, 100.0, 100.0 - 20.0, 100.0, dt);
        assert!(px > 0.0, "should recoil right, got {px}");
        assert!(py.abs() < 1e-3, "no vertical push for a level cursor");

        // Cursor beyond the radius → no push at all.
        let (fx, fy) =
            hover_startle_push(100.0, 100.0, 100.0 + HOVER_STARTLE_RADIUS + 5.0, 100.0, dt);
        assert_eq!((fx, fy), (0.0, 0.0));

        // Closer cursor recoils harder than a farther one (linear falloff).
        let (near, _) = hover_startle_push(100.0, 100.0, 90.0, 100.0, dt);
        let (far, _) = hover_startle_push(
            100.0,
            100.0,
            100.0 - (HOVER_STARTLE_RADIUS - 5.0),
            100.0,
            dt,
        );
        assert!(
            near > far,
            "nearer cursor should push harder: {near} vs {far}"
        );

        // Cursor exactly on the centre → arbitrary rightward push, no NaN.
        let (cx, cy) = hover_startle_push(100.0, 100.0, 100.0, 100.0, dt);
        assert!(cx > 0.0 && cy == 0.0 && cx.is_finite());
    }

    /// 4×4 frame, only the inner 2×2 is opaque (alpha 255), border is transparent.
    /// Bounds wide enough that only the clamp under test can bite.
    fn wide_bounds() -> crate::monitor::DesktopBounds {
        crate::monitor::DesktopBounds::from_size(10_000.0, 10_000.0)
    }

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
    fn poke_recoils_away_from_the_poke_point_and_clamps() {
        // Poked from the left → recoils right.
        let mut e = entity_at(100.0, 100.0);
        e.poke(50.0, wide_bounds());
        assert!(e.x > 100.0, "poke from left pushes right, got {}", e.x);

        // Poked from the right → recoils left.
        let mut e2 = entity_at(100.0, 100.0);
        e2.poke(300.0, wide_bounds());
        assert!(e2.x < 100.0, "poke from right pushes left, got {}", e2.x);

        // At the left edge, a rightward poke can't shove it off-screen.
        let mut e3 = entity_at(0.0, 100.0);
        e3.poke(500.0, wide_bounds());
        assert!(e3.x >= 0.0, "clamped at the edge, got {}", e3.x);
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
    fn alpha_hit_follows_the_mirrored_sprite_when_facing_left() {
        // `checker_frame` is symmetric, so it can't catch a flip bug.
        // Build one opaque only at texture column 0, row 1.
        let (col, row) = (0u32, 1u32);
        let mut rgba = vec![0u8; 4 * 4 * 4];
        rgba[((row * 4 + col) * 4) as usize + 3] = 255;
        let anim = Animation::new(vec![Frame::new(rgba, 4, 4)], 1.0, false);
        let mut e = Entity::for_test(0.0, 0.0, anim);

        // Unflipped: that pixel is drawn at screen column 0.
        assert!(e.contains_point(0.5, 1.5));
        assert!(!e.contains_point(3.5, 1.5));

        // facing_left mirrors the sprite, so the same opaque pixel is
        // drawn at screen column 3 — the hit test has to follow it there.
        e.facing_left = true;
        assert!(
            e.contains_point(3.5, 1.5),
            "mirrored sprite must be clickable where it is drawn"
        );
        assert!(
            !e.contains_point(0.5, 1.5),
            "mirrored sprite must not be clickable where it is no longer drawn"
        );
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
