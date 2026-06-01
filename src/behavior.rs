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
    /// Chases the cursor with simple ease-in; stops within `comfort_distance`.
    FollowCursor {
        /// Maximum movement per second toward the cursor (px/s).
        #[serde(default = "default_follow_speed")]
        speed: f32,
        /// Radius around the cursor at which the entity stops chasing.
        /// Prevents jitter when the entity reaches the target.
        #[serde(default = "default_comfort_distance")]
        comfort_distance: f32,
    },
}

fn default_walk_speed() -> f32 {
    60.0
}
fn default_follow_speed() -> f32 {
    240.0
}
fn default_comfort_distance() -> f32 {
    80.0
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

/// Per-frame inputs each behavior may read. Bundled into a struct so adding
/// new behaviors doesn't blow up `tick`'s signature.
#[derive(Debug, Clone, Copy)]
pub struct TickContext {
    pub sprite_width: f32,
    pub sprite_height: f32,
    pub screen_width: f32,
    pub screen_height: f32,
    /// Mouse position in screen space, if known. `None` when the cursor
    /// isn't being tracked (e.g. window unfocused) — behaviors that need
    /// it should no-op.
    pub cursor: Option<(f32, f32)>,
    pub dt: f32,
}

impl Behavior {
    /// Advance the behavior by `ctx.dt` seconds, mutating the entity's
    /// position and the runtime `state`.
    pub fn tick(
        &self,
        state: &mut BehaviorState,
        entity_x: &mut f32,
        entity_y: &mut f32,
        ctx: &TickContext,
    ) {
        match self {
            Behavior::Idle => {}
            Behavior::WalkAround { speed } => {
                *entity_x += state.walk_direction * speed * ctx.dt;

                // Bounce off the screen edges. We clamp position *and*
                // flip direction so a tick that overshoots the edge
                // immediately starts walking back the other way.
                if *entity_x <= 0.0 {
                    *entity_x = 0.0;
                    state.walk_direction = 1.0;
                } else if *entity_x + ctx.sprite_width >= ctx.screen_width {
                    *entity_x = ctx.screen_width - ctx.sprite_width;
                    state.walk_direction = -1.0;
                }
            }
            Behavior::FollowCursor {
                speed,
                comfort_distance,
            } => {
                let Some((cx, cy)) = ctx.cursor else {
                    return;
                };
                // Aim for cursor → entity *center*, not corner, so the
                // sprite ends up visually centered on the mouse.
                let entity_cx = *entity_x + ctx.sprite_width * 0.5;
                let entity_cy = *entity_y + ctx.sprite_height * 0.5;
                let dx = cx - entity_cx;
                let dy = cy - entity_cy;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist <= *comfort_distance {
                    return; // Inside personal space — stop chasing.
                }

                // Move at most the gap-to-comfort-zone this frame so we
                // never overshoot into the comfort radius.
                let step = (speed * ctx.dt).min(dist - comfort_distance);
                let inv = 1.0 / dist;
                *entity_x += dx * inv * step;
                *entity_y += dy * inv * step;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dt: f32) -> TickContext {
        TickContext {
            sprite_width: 64.0,
            sprite_height: 64.0,
            screen_width: 1920.0,
            screen_height: 1080.0,
            cursor: None,
            dt,
        }
    }

    #[test]
    fn idle_does_not_move() {
        let b = Behavior::Idle;
        let mut state = BehaviorState::default();
        let mut x = 100.0;
        let mut y = 200.0;
        b.tick(&mut state, &mut x, &mut y, &ctx(1.0 / 60.0));
        assert_eq!(x, 100.0);
        assert_eq!(y, 200.0);
    }

    #[test]
    fn walk_around_moves_in_direction() {
        let b = Behavior::WalkAround { speed: 60.0 };
        let mut state = BehaviorState::default(); // direction = +1
        let mut x = 100.0;
        let mut y = 0.0;
        for _ in 0..60 {
            b.tick(&mut state, &mut x, &mut y, &ctx(1.0 / 60.0));
        }
        assert!(x > 159.0 && x < 161.0, "x = {x}");
    }

    #[test]
    fn walk_around_bounces_at_right_edge() {
        let b = Behavior::WalkAround { speed: 200.0 };
        let mut state = BehaviorState::default();
        let mut x = 1900.0;
        let mut y = 0.0;
        for _ in 0..30 {
            b.tick(&mut state, &mut x, &mut y, &ctx(1.0 / 60.0));
        }
        assert_eq!(state.walk_direction, -1.0);
        assert!(x + 64.0 <= 1920.0);
    }

    #[test]
    fn walk_around_bounces_at_left_edge() {
        let b = Behavior::WalkAround { speed: 200.0 };
        let mut state = BehaviorState {
            walk_direction: -1.0,
        };
        let mut x = 20.0;
        let mut y = 0.0;
        for _ in 0..30 {
            b.tick(&mut state, &mut x, &mut y, &ctx(1.0 / 60.0));
        }
        assert_eq!(state.walk_direction, 1.0);
        assert!(x >= 0.0);
    }

    #[test]
    fn follow_cursor_no_op_when_cursor_unknown() {
        let b = Behavior::FollowCursor {
            speed: 200.0,
            comfort_distance: 50.0,
        };
        let mut state = BehaviorState::default();
        let mut x = 100.0;
        let mut y = 100.0;
        // Default ctx has cursor = None.
        b.tick(&mut state, &mut x, &mut y, &ctx(1.0 / 60.0));
        assert_eq!((x, y), (100.0, 100.0));
    }

    #[test]
    fn follow_cursor_moves_toward_target() {
        let b = Behavior::FollowCursor {
            speed: 200.0,
            comfort_distance: 0.0,
        };
        let mut state = BehaviorState::default();
        let mut x = 100.0;
        let mut y = 100.0;
        let mut c = ctx(1.0 / 60.0);
        // Cursor far to the bottom-right of entity.
        c.cursor = Some((500.0, 500.0));

        for _ in 0..120 {
            b.tick(&mut state, &mut x, &mut y, &c);
        }
        // Should have moved noticeably toward (500, 500).
        assert!(x > 150.0, "x = {x}");
        assert!(y > 150.0, "y = {y}");
    }

    #[test]
    fn follow_cursor_stops_inside_comfort_radius() {
        let b = Behavior::FollowCursor {
            speed: 200.0,
            comfort_distance: 50.0,
        };
        let mut state = BehaviorState::default();
        // Entity center will already be at cursor.
        let mut x = 500.0 - 32.0; // center = 500
        let mut y = 500.0 - 32.0;
        let mut c = ctx(1.0 / 60.0);
        c.cursor = Some((500.0, 500.0));

        let before = (x, y);
        b.tick(&mut state, &mut x, &mut y, &c);
        assert_eq!((x, y), before, "inside comfort radius → no motion");
    }

    #[test]
    fn default_is_idle() {
        assert!(matches!(Behavior::default(), Behavior::Idle));
    }
}
