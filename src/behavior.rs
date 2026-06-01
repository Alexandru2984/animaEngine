//! Per-entity autonomous behaviors.
//!
//! Behavior drives motion that the user didn't explicitly request. Each
//! entity carries one `Behavior` (the configuration) and one
//! `BehaviorState` (the runtime — current direction, accumulators, etc.).
//! Runtime state lives separately so it can default to sensible values
//! when an entity is loaded from a config that omits it.
//!
//! Adding a new behavior:
//! 1. Add a variant to `Behavior` with its config fields.
//! 2. Extend the `tick` match.
//! 3. (Optional) extend `BehaviorState` if you need runtime accumulators.

use serde::{Deserialize, Serialize};

/// Configuration of how an entity moves on its own.
///
/// Serialized as a TOML table with a `type` tag, e.g.
/// ```toml
/// [characters.behavior]
/// type = "walk_around"
/// speed = 80.0
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Behavior {
    /// Entity stays where the user put it (default).
    #[default]
    Idle,
    /// Walks horizontally across the screen, reversing at edges.
    WalkAround {
        /// Pixels per second, magnitude only — direction is in `BehaviorState`.
        #[serde(default = "default_walk_speed")]
        speed: f32,
    },
}

fn default_walk_speed() -> f32 {
    60.0
}

/// Runtime accumulators that don't belong in the user-facing config.
#[derive(Debug, Clone)]
pub struct BehaviorState {
    /// +1 = moving right, -1 = moving left. Flipped on edge collisions.
    pub walk_direction: f32,
}

impl Default for BehaviorState {
    fn default() -> Self {
        Self {
            walk_direction: 1.0,
        }
    }
}

impl Behavior {
    /// Advance the behavior by `dt` seconds. Mutates `entity_x` and the
    /// runtime `state`; reads `sprite_width` / `screen_width` to clamp
    /// movement inside the visible area.
    pub fn tick(
        &self,
        state: &mut BehaviorState,
        entity_x: &mut f32,
        sprite_width: f32,
        screen_width: f32,
        dt: f32,
    ) {
        match self {
            Behavior::Idle => {}
            Behavior::WalkAround { speed } => {
                *entity_x += state.walk_direction * speed * dt;

                // Bounce off the screen edges. We clamp position *and*
                // flip direction so a tick that overshoots the edge
                // immediately starts walking back the other way.
                if *entity_x <= 0.0 {
                    *entity_x = 0.0;
                    state.walk_direction = 1.0;
                } else if *entity_x + sprite_width >= screen_width {
                    *entity_x = screen_width - sprite_width;
                    state.walk_direction = -1.0;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_does_not_move() {
        let b = Behavior::Idle;
        let mut state = BehaviorState::default();
        let mut x = 100.0;
        b.tick(&mut state, &mut x, 64.0, 1920.0, 1.0 / 60.0);
        assert_eq!(x, 100.0);
    }

    #[test]
    fn walk_around_moves_in_direction() {
        let b = Behavior::WalkAround { speed: 60.0 };
        let mut state = BehaviorState::default(); // direction = +1
        let mut x = 100.0;
        // 1 second at 60 px/s → +60 px.
        for _ in 0..60 {
            b.tick(&mut state, &mut x, 64.0, 1920.0, 1.0 / 60.0);
        }
        assert!(x > 159.0 && x < 161.0, "x = {x}");
    }

    #[test]
    fn walk_around_bounces_at_right_edge() {
        let b = Behavior::WalkAround { speed: 200.0 };
        let mut state = BehaviorState::default();
        let mut x = 1900.0;
        let screen_w = 1920.0;
        let sprite_w = 64.0;

        // Drive long enough to hit the right wall.
        for _ in 0..30 {
            b.tick(&mut state, &mut x, sprite_w, screen_w, 1.0 / 60.0);
        }

        assert_eq!(state.walk_direction, -1.0, "should reverse on right edge");
        assert!(x + sprite_w <= screen_w, "should be clamped inside screen");
    }

    #[test]
    fn walk_around_bounces_at_left_edge() {
        let b = Behavior::WalkAround { speed: 200.0 };
        let mut state = BehaviorState {
            walk_direction: -1.0,
        };
        let mut x = 20.0;
        for _ in 0..30 {
            b.tick(&mut state, &mut x, 64.0, 1920.0, 1.0 / 60.0);
        }

        assert_eq!(state.walk_direction, 1.0);
        assert!(x >= 0.0);
    }

    #[test]
    fn default_is_idle() {
        let b = Behavior::default();
        assert!(matches!(b, Behavior::Idle));
    }
}
