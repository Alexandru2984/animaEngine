//! Simple 2D physics for entities: gravity, edge collision, and bounce.
//!
//! Each entity has a `PhysicsState` that tracks vertical velocity.
//! On each tick, gravity accelerates the entity downward. When the entity
//! hits the bottom edge of the screen, it bounces with damping.

/// Gravity acceleration in pixels per second squared.
const GRAVITY: f32 = 400.0;

/// Bounce damping factor — entity retains this fraction of velocity on impact.
/// 0.3 = loses 70% of energy per bounce.
const BOUNCE_FACTOR: f32 = 0.3;

/// Velocity threshold below which the entity is considered "grounded" (stops bouncing).
const GROUNDED_THRESHOLD: f32 = 15.0;

/// Physics state for a single entity.
#[derive(Debug, Clone)]
pub struct PhysicsState {
    /// Vertical velocity in pixels per second (positive = downward).
    pub velocity_y: f32,
    /// Whether the entity is resting on a surface (no more bouncing).
    pub grounded: bool,
    /// Whether physics is currently frozen (e.g., during drag).
    pub frozen: bool,
}

impl Default for PhysicsState {
    fn default() -> Self {
        Self {
            velocity_y: 0.0,
            grounded: true, // Entities stay where placed by default
            frozen: false,
        }
    }
}

impl PhysicsState {
    /// Update physics for one frame.
    ///
    /// - `y`: current entity Y position (top of sprite)
    /// - `sprite_height`: height of the entity in pixels (scaled)
    /// - `screen_height`: height of the screen in pixels
    /// - `dt`: delta time in seconds
    ///
    /// Returns the new Y position after physics update.
    pub fn tick(&mut self, y: f32, sprite_height: f32, screen_height: f32, dt: f32) -> f32 {
        if self.frozen {
            return y;
        }

        // Floor is the bottom edge of the screen
        let floor = screen_height - sprite_height;

        if self.grounded {
            // Already resting on the floor — clamp position
            return y.min(floor);
        }

        // Apply gravity
        self.velocity_y += GRAVITY * dt;

        // Update position
        let mut new_y = y + self.velocity_y * dt;

        // Check collision with floor
        if new_y >= floor {
            new_y = floor;

            // Bounce: reverse velocity with damping
            self.velocity_y = -self.velocity_y * BOUNCE_FACTOR;

            // Check if velocity is low enough to stop
            if self.velocity_y.abs() < GROUNDED_THRESHOLD {
                self.velocity_y = 0.0;
                self.grounded = true;
            }
        }

        // Clamp to screen top
        if new_y < 0.0 {
            new_y = 0.0;
            self.velocity_y = self.velocity_y.abs(); // Bounce off top
        }

        new_y
    }

    /// Unground the entity (e.g., after drag release).
    /// Entity will start falling from its current position.
    pub fn release(&mut self) {
        self.grounded = false;
        self.velocity_y = 0.0;
        self.frozen = false;
    }

    /// Freeze physics (e.g., during drag).
    pub fn freeze(&mut self) {
        self.frozen = true;
        self.velocity_y = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grounded_stays_put() {
        let mut physics = PhysicsState::default();
        let y = 100.0;
        let new_y = physics.tick(y, 64.0, 1080.0, 1.0 / 60.0);

        // Default is grounded — entity should not move (stays where placed)
        assert!(physics.grounded);
        assert_eq!(new_y, y, "Grounded entity should stay in place");
    }

    #[test]
    fn test_gravity_fall_after_release() {
        let mut physics = PhysicsState::default();
        physics.release(); // Explicitly activate gravity
        let screen_h = 1080.0;
        let sprite_h = 128.0;
        let mut y = 100.0;

        // Simulate 1 second of falling
        for _ in 0..60 {
            y = physics.tick(y, sprite_h, screen_h, 1.0 / 60.0);
        }

        // Should have moved down significantly
        assert!(y > 200.0, "Entity should fall: y={}", y);
        assert!(y <= screen_h - sprite_h, "Entity should not pass floor");
    }

    #[test]
    fn test_grounded_after_bouncing() {
        let mut physics = PhysicsState::default();
        physics.release(); // Activate gravity first
        let screen_h = 500.0;
        let sprite_h = 64.0;
        let mut y = 0.0; // Start at top

        // Simulate several seconds — entity should eventually come to rest
        for _ in 0..600 {
            y = physics.tick(y, sprite_h, screen_h, 1.0 / 60.0);
        }

        assert!(physics.grounded, "Entity should be grounded after bouncing");
        assert!(
            (y - (screen_h - sprite_h)).abs() < 1.0,
            "Entity should rest at floor: y={}, floor={}",
            y,
            screen_h - sprite_h
        );
    }

    #[test]
    fn test_frozen_physics() {
        let mut physics = PhysicsState::default();
        physics.freeze();

        let y = 100.0;
        let new_y = physics.tick(y, 64.0, 1080.0, 1.0 / 60.0);

        assert_eq!(y, new_y, "Frozen physics should not change position");
    }

    #[test]
    fn test_release_after_drag() {
        let mut physics = PhysicsState::default();
        physics.release();

        assert!(!physics.grounded);
        assert!(!physics.frozen);
        assert_eq!(physics.velocity_y, 0.0);
    }
}
