//! Window-awareness: desktop windows as physics platforms (X11).
//!
//! When `window_awareness` is enabled, the top edges of regular
//! desktop windows act as floors for physics-enabled entities — a
//! mascot dropped over a window lands on its title bar and can walk
//! along it, falling off at the edges. The classic Shimeji trick.
//!
//! This module is the *pure* half: rectangle math, no X11. The
//! snapshot provider lives in `window::x11_windows` and feeds
//! [`crate::scene::Scene::set_window_platforms`] from the render
//! loop. On native
//! Wayland no provider exists (the protocol exposes no global window
//! geometry, by design) and the platform list stays empty — every
//! code path degrades to plain screen-floor physics.
//!
//! Coordinates are global desktop pixels, feet-space: a platform's
//! `top` is the y an entity's *feet* rest at while standing on it.

/// One window's frame rectangle, in global desktop coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlatformRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Tolerance while airborne: a platform top may be at most this many
/// pixels *above* the feet and still catch the entity. Small — a
/// near-miss fall past a window edge must not visibly yank the entity
/// upward.
pub const LAND_TOLERANCE: f32 = 2.0;

/// Tolerance while grounded: how far the platform under the feet may
/// rise between window polls (~300 ms) before the entity stops
/// tracking it. Generous so slowly dragging a window upward carries
/// the mascot along; fast drags drop it, which reads naturally.
pub const RIDE_TOLERANCE: f32 = 40.0;

/// Resolve the effective floor (feet-space y) for an entity whose
/// feet-center is at (`foot_x`, `feet_y`).
///
/// Candidates are platforms whose horizontal span contains `foot_x`
/// (half-open, so abutting windows hand over cleanly) and whose top
/// is no more than `tolerance` above the feet — anything higher is a
/// wall to fall past, not a floor. The result is the *highest*
/// candidate top, never below `screen_floor`.
pub fn effective_floor(
    platforms: &[PlatformRect],
    foot_x: f32,
    feet_y: f32,
    screen_floor: f32,
    tolerance: f32,
) -> f32 {
    platforms
        .iter()
        .filter(|p| foot_x >= p.x && foot_x < p.x + p.w)
        .map(|p| p.y)
        .filter(|&top| top >= feet_y - tolerance)
        .fold(screen_floor, f32::min)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn win(x: f32, y: f32, w: f32) -> PlatformRect {
        PlatformRect { x, y, w, h: 600.0 }
    }

    const SCREEN: f32 = 1080.0;

    #[test]
    fn no_platforms_means_screen_floor() {
        assert_eq!(
            effective_floor(&[], 500.0, 100.0, SCREEN, LAND_TOLERANCE),
            SCREEN
        );
    }

    #[test]
    fn lands_on_highest_window_below() {
        let wins = [win(400.0, 700.0, 300.0), win(450.0, 500.0, 300.0)];
        // Feet above both → the higher top (500) wins.
        assert_eq!(
            effective_floor(&wins, 500.0, 100.0, SCREEN, LAND_TOLERANCE),
            500.0
        );
    }

    #[test]
    fn window_above_feet_is_not_a_floor() {
        let wins = [win(400.0, 300.0, 300.0)];
        // Feet already below the top (400 > 300 + tolerance) → screen.
        assert_eq!(
            effective_floor(&wins, 500.0, 400.0, SCREEN, LAND_TOLERANCE),
            SCREEN
        );
    }

    #[test]
    fn walking_off_the_edge_loses_the_floor() {
        let wins = [win(400.0, 500.0, 300.0)];
        // Standing on it at x=699 …
        assert_eq!(
            effective_floor(&wins, 699.0, 500.0, SCREEN, RIDE_TOLERANCE),
            500.0
        );
        // … one step past the half-open right edge: gone.
        assert_eq!(
            effective_floor(&wins, 700.0, 500.0, SCREEN, RIDE_TOLERANCE),
            SCREEN
        );
    }

    #[test]
    fn ride_tolerance_tracks_a_rising_window() {
        let wins = [win(400.0, 470.0, 300.0)];
        // Window rose 30 px since the entity grounded at 500. Within
        // RIDE_TOLERANCE → still the floor (entity gets carried up).
        assert_eq!(
            effective_floor(&wins, 500.0, 500.0, SCREEN, RIDE_TOLERANCE),
            470.0
        );
        // While airborne the same gap is a near-miss, not a magnet.
        assert_eq!(
            effective_floor(&wins, 500.0, 500.0, SCREEN, LAND_TOLERANCE),
            SCREEN
        );
    }

    #[test]
    fn screen_floor_caps_low_platforms() {
        // A window hanging below the screen bottom never pulls the
        // floor down past the screen edge.
        let wins = [win(400.0, 2000.0, 300.0)];
        assert_eq!(
            effective_floor(&wins, 500.0, 100.0, SCREEN, LAND_TOLERANCE),
            SCREEN
        );
    }
}
