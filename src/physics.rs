//! Simple 2D physics for entities: gravity, edge collision, and bounce.
//!
//! Physics is **opt-in per entity**. By default entities stay exactly where
//! the user places them — no force calculations run. Activating physics on an
//! entity (typically via the `G` keybind in edit mode) releases it: gravity
//! pulls it down until it hits the floor and bounces to rest.

/// Gravity acceleration in pixels per second squared.
const GRAVITY: f32 = 400.0;

/// Bounce damping factor — fraction of velocity retained on impact.
/// 0.3 = loses 70% of energy per bounce.
const BOUNCE_FACTOR: f32 = 0.3;

/// Velocity threshold (px/s) below which the entity is considered at rest.
const GROUNDED_THRESHOLD: f32 = 15.0;

/// Physics state for a single entity.
///
/// The combined effect of the four flags:
/// - `enabled = false` → entity is static, tick is a no-op (the default)
/// - `enabled = true,  grounded = false` → entity falls / bounces
/// - `enabled = true,  grounded = true`  → entity rests on the floor
/// - `frozen = true`                     → tick is a no-op regardless (used during drag)
#[derive(Debug, Clone)]
pub struct PhysicsState {
    /// Master switch — when false, `tick` returns `y` unchanged.
    pub enabled: bool,
    /// Vertical velocity in pixels per second (positive = downward).
    pub velocity_y: f32,
    /// Whether the entity is resting on a surface (no more bouncing).
    pub grounded: bool,
    /// Temporary freeze flag (e.g., while the user is dragging).
    pub frozen: bool,
}

impl Default for PhysicsState {
    fn default() -> Self {
        Self {
            enabled: false,
            velocity_y: 0.0,
            grounded: false,
            frozen: false,
        }
    }
}

impl PhysicsState {
    /// Construct from a `physics_enabled` config flag. When enabled at load
    /// time, the entity starts mid-air so it falls and settles on the floor.
    pub fn from_enabled(enabled: bool) -> Self {
        Self {
            enabled,
            ..Self::default()
        }
    }

    /// Update physics for one frame. Returns the new Y position.
    pub fn tick(&mut self, y: f32, sprite_height: f32, screen_height: f32, dt: f32) -> f32 {
        if !self.enabled || self.frozen {
            return y;
        }

        let floor = screen_height - sprite_height;

        if self.grounded {
            return y.min(floor);
        }

        self.velocity_y += GRAVITY * dt;
        let mut new_y = y + self.velocity_y * dt;

        if new_y >= floor {
            new_y = floor;
            self.velocity_y = -self.velocity_y * BOUNCE_FACTOR;

            if self.velocity_y.abs() < GROUNDED_THRESHOLD {
                self.velocity_y = 0.0;
                self.grounded = true;
            }
        }

        if new_y < 0.0 {
            new_y = 0.0;
            self.velocity_y = self.velocity_y.abs();
        }

        new_y
    }

    /// Turn physics on: entity starts falling from its current position.
    pub fn enable(&mut self) {
        self.enabled = true;
        self.grounded = false;
        self.velocity_y = 0.0;
        self.frozen = false;
    }

    /// Turn physics off: entity is pinned to its current position.
    pub fn disable(&mut self) {
        self.enabled = false;
        self.velocity_y = 0.0;
        self.grounded = false;
    }

    /// Toggle the master switch. Convenient for a single keybind.
    pub fn toggle(&mut self) {
        if self.enabled {
            self.disable();
        } else {
            self.enable();
        }
    }

    /// Temporarily suspend physics (e.g., while the user is dragging).
    pub fn freeze(&mut self) {
        self.frozen = true;
        self.velocity_y = 0.0;
    }

    /// Resume from a freeze. Does NOT change the master `enabled` flag —
    /// if physics was off before the freeze it stays off afterwards.
    pub fn unfreeze(&mut self) {
        self.frozen = false;
        if self.enabled {
            // Restart from rest so velocity doesn't snap from drag motion.
            self.velocity_y = 0.0;
            self.grounded = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_disabled_and_static() {
        let mut physics = PhysicsState::default();
        assert!(!physics.enabled);
        let y = 100.0;
        let new_y = physics.tick(y, 64.0, 1080.0, 1.0 / 60.0);
        assert_eq!(new_y, y, "Default (disabled) physics must not move entity");
    }

    #[test]
    fn enable_starts_falling() {
        let mut physics = PhysicsState::default();
        physics.enable();
        let screen_h = 1080.0;
        let sprite_h = 128.0;
        let mut y = 100.0;

        for _ in 0..60 {
            y = physics.tick(y, sprite_h, screen_h, 1.0 / 60.0);
        }

        assert!(y > 200.0, "Entity should fall once enabled: y={}", y);
        assert!(y <= screen_h - sprite_h, "Must not pass floor");
    }

    #[test]
    fn grounded_after_bouncing() {
        let mut physics = PhysicsState::default();
        physics.enable();
        let screen_h = 500.0;
        let sprite_h = 64.0;
        let mut y = 0.0;

        for _ in 0..600 {
            y = physics.tick(y, sprite_h, screen_h, 1.0 / 60.0);
        }

        assert!(physics.grounded, "Should be grounded after bouncing");
        assert!(
            (y - (screen_h - sprite_h)).abs() < 1.0,
            "Should rest at floor: y={}, floor={}",
            y,
            screen_h - sprite_h
        );
    }

    #[test]
    fn frozen_does_not_move_even_when_enabled() {
        let mut physics = PhysicsState::default();
        physics.enable();
        physics.freeze();
        let y = 100.0;
        let new_y = physics.tick(y, 64.0, 1080.0, 1.0 / 60.0);
        assert_eq!(y, new_y);
    }

    #[test]
    fn unfreeze_preserves_disabled_state() {
        let mut physics = PhysicsState::default();
        // Simulate drag on a disabled entity
        physics.freeze();
        physics.unfreeze();

        assert!(!physics.enabled, "unfreeze must not turn physics on");
        let y = 100.0;
        let new_y = physics.tick(y, 64.0, 1080.0, 1.0 / 60.0);
        assert_eq!(
            new_y, y,
            "Entity must stay put after drag if physics was off"
        );
    }

    #[test]
    fn toggle_round_trip() {
        let mut physics = PhysicsState::default();
        physics.toggle();
        assert!(physics.enabled);
        physics.toggle();
        assert!(!physics.enabled);
    }
}
