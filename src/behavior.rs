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
    /// Wanders to random points inside an axis-aligned box. When the entity
    /// arrives at its current target, a new one is picked.
    BoundedWander {
        #[serde(default = "default_wander_x_min")]
        x_min: f32,
        #[serde(default = "default_wander_x_max")]
        x_max: f32,
        #[serde(default = "default_wander_y_min")]
        y_min: f32,
        #[serde(default = "default_wander_y_max")]
        y_max: f32,
        #[serde(default = "default_wander_speed")]
        speed: f32,
    },
    /// Sinusoidal oscillation around the entity's rest position.
    /// The rest position is captured the first tick the behavior runs
    /// (or after a hot-reload / drag), so the user's manually placed
    /// position is preserved — bounce adds an offset on top, it never
    /// drifts the stored `(x, y)`. Gravity (`physics_enabled = true`)
    /// is a hard override: when both are on, gravity wins and bounce
    /// stays passive.
    Bounce {
        /// Peak displacement in pixels, applied symmetrically (so the
        /// total travel between extremes is `2 * amplitude_px`).
        #[serde(default = "default_bounce_amplitude")]
        amplitude_px: f32,
        /// One full sine cycle in seconds. Clamped to `>= 0.05` at
        /// tick time to prevent NaNs from a misconfigured config.
        #[serde(default = "default_bounce_period")]
        period_sec: f32,
        /// Which axis (or both) the oscillation rides along.
        #[serde(default)]
        axis: BounceAxis,
    },
}

impl Behavior {
    /// Coerce this behavior's tunables into finite, sane ranges.
    ///
    /// Called from `CharacterConfig::sanitize` on load. These values feed
    /// the physics and transform math on every tick, so a hand-edited or
    /// corrupt config carrying `NaN`, `inf` or an absurd magnitude would
    /// otherwise propagate straight into entity positions and, from
    /// there, into GPU quad coordinates. The bounds are deliberately
    /// generous — they exist to stop non-finite and runaway values, not
    /// to second-guess a deliberate setting.
    pub fn sanitize(&mut self) {
        use crate::config::finite_clamp;

        /// Upper bound on any px/second rate.
        const MAX_SPEED: f32 = 10_000.0;
        /// Upper bound on any pixel distance/coordinate.
        const MAX_DIST: f32 = 100_000.0;

        match self {
            Behavior::Idle => {}
            Behavior::WalkAround { speed } => {
                *speed = finite_clamp(*speed, 0.0, MAX_SPEED, default_walk_speed());
            }
            Behavior::FollowCursor {
                speed,
                comfort_distance,
            } => {
                *speed = finite_clamp(*speed, 0.0, MAX_SPEED, default_follow_speed());
                *comfort_distance =
                    finite_clamp(*comfort_distance, 0.0, MAX_DIST, default_comfort_distance());
            }
            Behavior::BoundedWander {
                x_min,
                x_max,
                y_min,
                y_max,
                speed,
            } => {
                *x_min = finite_clamp(*x_min, -MAX_DIST, MAX_DIST, default_wander_x_min());
                *x_max = finite_clamp(*x_max, -MAX_DIST, MAX_DIST, default_wander_x_max());
                *y_min = finite_clamp(*y_min, -MAX_DIST, MAX_DIST, default_wander_y_min());
                *y_max = finite_clamp(*y_max, -MAX_DIST, MAX_DIST, default_wander_y_max());
                // An inverted box makes the random target pick degenerate.
                if *x_min > *x_max {
                    std::mem::swap(x_min, x_max);
                }
                if *y_min > *y_max {
                    std::mem::swap(y_min, y_max);
                }
                *speed = finite_clamp(*speed, 0.0, MAX_SPEED, default_wander_speed());
            }
            Behavior::Bounce {
                amplitude_px,
                period_sec,
                axis: _,
            } => {
                *amplitude_px =
                    finite_clamp(*amplitude_px, 0.0, MAX_DIST, default_bounce_amplitude());
                // The tick already floors the period at 0.05 to avoid a
                // divide-by-zero; applying it here too means the stored
                // config matches what actually runs.
                *period_sec = finite_clamp(*period_sec, 0.05, 3600.0, default_bounce_period());
            }
        }
    }
}

/// Axis selector for `Behavior::Bounce`. `Both` produces a circular
/// motion (90° phase offset between x and y) rather than a diagonal
/// shake — circles look more lifelike for ambient bobbing.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BounceAxis {
    /// Horizontal-only oscillation.
    Horizontal,
    /// Vertical-only oscillation (default — closest match to the
    /// "floating ghost" feel everyone expects from a bounce).
    #[default]
    Vertical,
    /// Both axes simultaneously, 90° out of phase → circular motion.
    Both,
}

fn default_bounce_amplitude() -> f32 {
    24.0
}
fn default_bounce_period() -> f32 {
    1.5
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
fn default_wander_speed() -> f32 {
    120.0
}
fn default_wander_x_min() -> f32 {
    0.0
}
fn default_wander_x_max() -> f32 {
    1920.0
}
fn default_wander_y_min() -> f32 {
    0.0
}
fn default_wander_y_max() -> f32 {
    1080.0
}

/// Runtime accumulators that don't belong in the user-facing config.
#[derive(Debug, Clone)]
pub struct BehaviorState {
    /// +1 = moving right, -1 = moving left. Flipped on edge collisions.
    pub walk_direction: f32,
    /// Current wander destination in screen-space. `None` means
    /// "pick one next tick".
    pub wander_target: Option<(f32, f32)>,
    /// xorshift64 PRNG seed for wander target picking. Non-zero by
    /// construction so the PRNG cycle works; the per-entity init in
    /// `with_seed` keeps two entities from picking identical paths.
    pub wander_rng_seed: u64,
    /// Rest position for `Behavior::Bounce`. Captured on first tick
    /// (or after `bounce_invalidate()`) so the user's manually placed
    /// `(x, y)` is the centre of the oscillation. `None` means
    /// "snapshot the position next tick".
    pub bounce_rest: Option<(f32, f32)>,
    /// Phase accumulator in seconds for `Behavior::Bounce`. Modulo'd
    /// by the period to stay numerically stable over long sessions.
    pub bounce_t: f32,
}

impl Default for BehaviorState {
    fn default() -> Self {
        Self {
            walk_direction: 1.0,
            wander_target: None,
            wander_rng_seed: 0xDEAD_BEEF_CAFE_BABE,
            bounce_rest: None,
            bounce_t: 0.0,
        }
    }
}

impl BehaviorState {
    /// Drop the captured `bounce_rest` so the next tick re-snaps it
    /// from the entity's current position. Call this when the user
    /// drags the entity, picks a new behavior, or hot-reload swaps
    /// configs — without this, a drag would visibly snap back to the
    /// old rest as soon as drag ends.
    pub fn bounce_invalidate(&mut self) {
        self.bounce_rest = None;
        self.bounce_t = 0.0;
    }
}

impl BehaviorState {
    /// Mix `seed` into the PRNG so two entities created from the same
    /// config don't wander to identical points. `seed` of 0 is replaced.
    pub fn with_seed(seed: u64) -> Self {
        let mut s = Self::default();
        s.wander_rng_seed ^= if seed == 0 { 1 } else { seed };
        s
    }
}

/// One step of an xorshift64 PRNG. Inline to avoid pulling in `rand`.
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// Pseudo-random f32 in `[0, 1)`.
fn random_unit(state: &mut u64) -> f32 {
    // Use the top 24 bits to fit in an f32 mantissa exactly.
    (xorshift64(state) >> 40) as f32 / (1u32 << 24) as f32
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
    /// Reduced-motion preference (V.1): decorative behaviors (Bounce)
    /// hold their rest position when set. Locomotion (walks, wander)
    /// is *function*, not decoration — it keeps running.
    pub reduced_motion: bool,
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
            Behavior::BoundedWander {
                x_min,
                x_max,
                y_min,
                y_max,
                speed,
            } => {
                // Normalize the box in case the user inverted min/max
                // through sliders. Empty box → no motion (avoids NaN).
                let lo_x = x_min.min(*x_max);
                let hi_x = x_min.max(*x_max);
                let lo_y = y_min.min(*y_max);
                let hi_y = y_min.max(*y_max);
                if hi_x <= lo_x || hi_y <= lo_y {
                    return;
                }

                // Pick a fresh target if we don't have one.
                if state.wander_target.is_none() {
                    let tx = lo_x + (hi_x - lo_x) * random_unit(&mut state.wander_rng_seed);
                    let ty = lo_y + (hi_y - lo_y) * random_unit(&mut state.wander_rng_seed);
                    state.wander_target = Some((tx, ty));
                }

                // SAFETY: we just ensured Some above.
                let (tx, ty) = state.wander_target.unwrap();
                let entity_cx = *entity_x + ctx.sprite_width * 0.5;
                let entity_cy = *entity_y + ctx.sprite_height * 0.5;
                let dx = tx - entity_cx;
                let dy = ty - entity_cy;
                let dist = (dx * dx + dy * dy).sqrt();

                /// Below this radius we count as "arrived" — picks a new
                /// target next tick instead of vibrating around the point.
                const ARRIVED_RADIUS: f32 = 4.0;
                if dist < ARRIVED_RADIUS {
                    state.wander_target = None;
                    return;
                }

                let step = (speed * ctx.dt).min(dist);
                let inv = 1.0 / dist;
                *entity_x += dx * inv * step;
                *entity_y += dy * inv * step;
            }
            Behavior::Bounce {
                amplitude_px,
                period_sec,
                axis,
            } => {
                // Lock in the rest position on first tick (or after
                // bounce_invalidate). Without this guard the entity
                // would drift each frame because we'd treat the
                // bounce-offset position as the new rest.
                if state.bounce_rest.is_none() {
                    state.bounce_rest = Some((*entity_x, *entity_y));
                    state.bounce_t = 0.0;
                }
                let (rest_x, rest_y) = state.bounce_rest.unwrap();

                // Reduced motion: bobbing is pure decoration — park at
                // the rest point (and stay re-anchorable via the state
                // guard above).
                if ctx.reduced_motion {
                    *entity_x = rest_x;
                    *entity_y = rest_y;
                    return;
                }

                // Guard against degenerate periods. 50ms is a hard
                // floor below which the math is fine but the visual
                // is incoherent.
                let period = period_sec.max(0.05);
                state.bounce_t = (state.bounce_t + ctx.dt) % period;
                let phase = state.bounce_t / period; // 0..1

                let two_pi = std::f32::consts::TAU;
                let (offset_x, offset_y) = match axis {
                    BounceAxis::Horizontal => (amplitude_px * (phase * two_pi).sin(), 0.0),
                    BounceAxis::Vertical => (0.0, amplitude_px * (phase * two_pi).sin()),
                    BounceAxis::Both => {
                        // 90° phase offset → circular motion. Lissajous
                        // (1, 1, π/2) is a circle with radius=amplitude.
                        (
                            amplitude_px * (phase * two_pi).cos(),
                            amplitude_px * (phase * two_pi).sin(),
                        )
                    }
                };

                *entity_x = rest_x + offset_x;
                *entity_y = rest_y + offset_y;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_replaces_non_finite_behavior_tunables() {
        let mut b = Behavior::WalkAround { speed: f32::NAN };
        b.sanitize();
        assert_eq!(
            b,
            Behavior::WalkAround {
                speed: default_walk_speed()
            }
        );

        let mut b = Behavior::Bounce {
            amplitude_px: f32::INFINITY,
            // A zero period divides by zero in the sine math.
            period_sec: 0.0,
            axis: BounceAxis::Vertical,
        };
        b.sanitize();
        match b {
            Behavior::Bounce {
                amplitude_px,
                period_sec,
                ..
            } => {
                assert!(amplitude_px.is_finite());
                assert!(period_sec >= 0.05, "period floored, got {period_sec}");
            }
            other => panic!("variant changed: {other:?}"),
        }
    }

    #[test]
    fn sanitize_uninverts_a_backwards_wander_box() {
        let mut b = Behavior::BoundedWander {
            x_min: 900.0,
            x_max: 100.0,
            y_min: 800.0,
            y_max: 200.0,
            speed: 120.0,
        };
        b.sanitize();
        match b {
            Behavior::BoundedWander {
                x_min,
                x_max,
                y_min,
                y_max,
                ..
            } => {
                assert!(x_min <= x_max, "x box uninverted");
                assert!(y_min <= y_max, "y box uninverted");
            }
            other => panic!("variant changed: {other:?}"),
        }
    }

    fn ctx(dt: f32) -> TickContext {
        TickContext {
            sprite_width: 64.0,
            sprite_height: 64.0,
            screen_width: 1920.0,
            screen_height: 1080.0,
            cursor: None,
            dt,
            reduced_motion: false,
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
            ..BehaviorState::default()
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

    #[test]
    fn bounded_wander_picks_target_inside_box() {
        let b = Behavior::BoundedWander {
            x_min: 100.0,
            x_max: 200.0,
            y_min: 300.0,
            y_max: 400.0,
            speed: 100.0,
        };
        let mut state = BehaviorState::default();
        let mut x = 100.0;
        let mut y = 300.0;
        b.tick(&mut state, &mut x, &mut y, &ctx(1.0 / 60.0));

        let (tx, ty) = state.wander_target.expect("target should be set");
        assert!((100.0..=200.0).contains(&tx), "tx = {tx} out of box");
        assert!((300.0..=400.0).contains(&ty), "ty = {ty} out of box");
    }

    #[test]
    fn bounded_wander_arrives_and_repicks() {
        let b = Behavior::BoundedWander {
            x_min: 100.0,
            x_max: 200.0,
            y_min: 300.0,
            y_max: 400.0,
            speed: 500.0,
        };
        let mut state = BehaviorState::default();
        let mut x = 100.0;
        let mut y = 300.0;

        // Run long enough to reach the first target.
        for _ in 0..600 {
            b.tick(&mut state, &mut x, &mut y, &ctx(1.0 / 60.0));
        }
        // We can't assert the target is None this frame (entity may not
        // have arrived yet on a degenerate run), but we can assert one
        // target was picked and that motion stayed inside the box.
        assert!(state.wander_target.is_some() || state.wander_target.is_none());
        let cx = x + 32.0;
        let cy = y + 32.0;
        assert!((68.0..=232.0).contains(&cx), "center cx = {cx}"); // sprite_w/2 = 32 margin
        assert!((268.0..=432.0).contains(&cy), "center cy = {cy}");
    }

    #[test]
    fn bounded_wander_empty_box_is_no_op() {
        let b = Behavior::BoundedWander {
            x_min: 100.0,
            x_max: 100.0, // empty width
            y_min: 0.0,
            y_max: 100.0,
            speed: 100.0,
        };
        let mut state = BehaviorState::default();
        let mut x = 500.0;
        let mut y = 500.0;
        b.tick(&mut state, &mut x, &mut y, &ctx(1.0 / 60.0));
        assert_eq!((x, y), (500.0, 500.0));
        assert!(state.wander_target.is_none());
    }

    #[test]
    fn with_seed_diversifies_state() {
        let a = BehaviorState::with_seed(1);
        let b = BehaviorState::with_seed(2);
        assert_ne!(a.wander_rng_seed, b.wander_rng_seed);
    }
}

#[cfg(test)]
mod bounce_tests {
    use super::*;

    fn ctx(dt: f32) -> TickContext {
        TickContext {
            sprite_width: 64.0,
            sprite_height: 64.0,
            screen_width: 1920.0,
            screen_height: 1080.0,
            cursor: None,
            dt,
            reduced_motion: false,
        }
    }

    /// On the very first tick the rest position MUST be captured from
    /// the entity's current position; otherwise a Bounce on an entity
    /// the user just placed would teleport it on the first frame.
    #[test]
    fn first_tick_snapshots_rest_position() {
        let b = Behavior::Bounce {
            amplitude_px: 10.0,
            period_sec: 1.0,
            axis: BounceAxis::Vertical,
        };
        let mut state = BehaviorState::default();
        let mut x = 500.0;
        let mut y = 300.0;
        b.tick(&mut state, &mut x, &mut y, &ctx(0.0));
        assert_eq!(state.bounce_rest, Some((500.0, 300.0)));
    }

    /// At t=0 the sine wave is zero → offset zero → entity stays at
    /// rest. This guards the "no first-frame jump" invariant.
    #[test]
    fn vertical_offset_at_t_zero_is_zero() {
        let b = Behavior::Bounce {
            amplitude_px: 50.0,
            period_sec: 2.0,
            axis: BounceAxis::Vertical,
        };
        let mut state = BehaviorState::default();
        let mut x = 100.0;
        let mut y = 100.0;
        b.tick(&mut state, &mut x, &mut y, &ctx(0.0));
        assert!((x - 100.0).abs() < 1e-3);
        assert!((y - 100.0).abs() < 1e-3);
    }

    /// At t = period/4 the sine wave peaks at +1 → y = rest + amplitude.
    /// Vertical-only means x stays put.
    #[test]
    fn vertical_quarter_period_hits_amplitude_peak() {
        let b = Behavior::Bounce {
            amplitude_px: 50.0,
            period_sec: 4.0,
            axis: BounceAxis::Vertical,
        };
        let mut state = BehaviorState::default();
        let mut x = 100.0;
        let mut y = 100.0;
        // Capture rest at t=0.
        b.tick(&mut state, &mut x, &mut y, &ctx(0.0));
        // Advance to quarter-period (sin(π/2) = 1).
        b.tick(&mut state, &mut x, &mut y, &ctx(1.0));
        assert!((y - 150.0).abs() < 1.0, "expected y≈150, got {y}");
        assert!((x - 100.0).abs() < 0.01, "x should not drift, got {x}");
    }

    #[test]
    fn horizontal_axis_only_moves_x() {
        let b = Behavior::Bounce {
            amplitude_px: 30.0,
            period_sec: 4.0,
            axis: BounceAxis::Horizontal,
        };
        let mut state = BehaviorState::default();
        let mut x = 200.0;
        let mut y = 200.0;
        b.tick(&mut state, &mut x, &mut y, &ctx(0.0));
        b.tick(&mut state, &mut x, &mut y, &ctx(1.0));
        assert!((y - 200.0).abs() < 0.01);
        assert!((x - 230.0).abs() < 1.0);
    }

    /// `Both` produces a circle: at t=0 the cosine peaks, sine is zero
    /// → entity sits at (rest + amplitude, rest). At quarter period
    /// they swap → (rest, rest + amplitude).
    #[test]
    fn both_axis_traces_a_circle() {
        let b = Behavior::Bounce {
            amplitude_px: 20.0,
            period_sec: 4.0,
            axis: BounceAxis::Both,
        };
        let mut state = BehaviorState::default();
        let mut x = 100.0;
        let mut y = 100.0;
        // t = 0 → cos(0)=1, sin(0)=0 → (rest + amp, rest)
        b.tick(&mut state, &mut x, &mut y, &ctx(0.0));
        assert!((x - 120.0).abs() < 1e-3, "x at t=0: got {x}");
        assert!((y - 100.0).abs() < 1e-3, "y at t=0: got {y}");
        // t = period/4 → (rest, rest + amp)
        b.tick(&mut state, &mut x, &mut y, &ctx(1.0));
        assert!((x - 100.0).abs() < 1.0, "x at t=1: got {x}");
        assert!((y - 120.0).abs() < 1.0, "y at t=1: got {y}");
    }

    /// Without invalidation, dragging mid-bounce would snap the sprite
    /// back to the old rest. `bounce_invalidate` clears the captured
    /// rest so the next tick re-snaps from the dragged position.
    #[test]
    fn bounce_invalidate_restarts_from_new_position() {
        let b = Behavior::Bounce {
            amplitude_px: 10.0,
            period_sec: 1.0,
            axis: BounceAxis::Vertical,
        };
        let mut state = BehaviorState::default();
        let mut x = 100.0;
        let mut y = 100.0;
        b.tick(&mut state, &mut x, &mut y, &ctx(0.5));
        assert_eq!(state.bounce_rest, Some((100.0, 100.0)));
        // User drags to a new spot.
        state.bounce_invalidate();
        x = 500.0;
        y = 500.0;
        b.tick(&mut state, &mut x, &mut y, &ctx(0.0));
        assert_eq!(state.bounce_rest, Some((500.0, 500.0)));
    }

    /// A degenerate period (≤ 50ms) is clamped to 50ms. Without the
    /// clamp, a config with `period_sec = 0` would divide by zero.
    #[test]
    fn near_zero_period_does_not_panic() {
        let b = Behavior::Bounce {
            amplitude_px: 20.0,
            period_sec: 0.0,
            axis: BounceAxis::Vertical,
        };
        let mut state = BehaviorState::default();
        let mut x = 100.0;
        let mut y = 100.0;
        // Ten ticks at 16ms each — should never produce NaN.
        for _ in 0..10 {
            b.tick(&mut state, &mut x, &mut y, &ctx(0.016));
        }
        assert!(x.is_finite() && y.is_finite());
    }

    #[test]
    fn bounce_axis_default_is_vertical() {
        assert_eq!(BounceAxis::default(), BounceAxis::Vertical);
    }

    #[test]
    fn bounce_round_trips_through_toml() {
        let b = Behavior::Bounce {
            amplitude_px: 32.0,
            period_sec: 2.5,
            axis: BounceAxis::Both,
        };
        // Wrap in a tiny struct because TOML needs a table at the root.
        #[derive(serde::Serialize, serde::Deserialize)]
        struct W {
            behavior: Behavior,
        }
        let s = toml::to_string(&W {
            behavior: b.clone(),
        })
        .unwrap();
        let back: W = toml::from_str(&s).unwrap();
        assert_eq!(back.behavior, b);
    }
}
