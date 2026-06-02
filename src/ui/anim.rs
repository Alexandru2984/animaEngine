//! Micro-animation helpers — the live implementation of
//! [`docs/design-system.md`] §6.
//!
//! Two exports matter:
//!
//! - Easing curves: [`ease_out_quad`], [`ease_in_quad`] — pure
//!   functions, trivial to test.
//! - [`lerp`] — straight-alpha colour interpolation, used to fade
//!   panels in / out without pulling in a tween crate.
//!
//! Long-running animation state (per-widget hover progress, etc.) is
//! handled by `egui::Context::animate_*` directly at the call site;
//! this module deliberately stays state-free so the timing curves can
//! be unit-tested without an egui context.

use egui::Color32;

/// Quadratic ease-out: fast start, gentle finish. Range `[0, 1]`.
///
/// Used for entry transitions (toast slide-in, popup fade-in) where the
/// element should feel like it arrives, then settles.
pub fn ease_out_quad(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t) * (1.0 - t)
}

/// Quadratic ease-in: gentle start, accelerating finish. Range `[0, 1]`.
///
/// Used for exit transitions (toast fade-out) where the element should
/// feel like it accelerates away rather than disappearing abruptly.
pub fn ease_in_quad(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t
}

/// Linear interpolation of two colors in straight-alpha sRGB. `t` is
/// clamped to `[0, 1]`. Mirrors `theme::mix` but lives here so anim
/// callers don't have to depend on the theme module just for one
/// helper.
pub fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let lerp_u8 = |x: u8, y: u8| -> u8 {
        ((x as f32) * (1.0 - t) + (y as f32) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color32::from_rgba_unmultiplied(
        lerp_u8(a.r(), b.r()),
        lerp_u8(a.g(), b.g()),
        lerp_u8(a.b(), b.b()),
        lerp_u8(a.a(), b.a()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ease_out_quad_endpoints() {
        assert!((ease_out_quad(0.0) - 0.0).abs() < 1e-6);
        assert!((ease_out_quad(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn ease_out_quad_is_faster_at_start() {
        // At t = 0.25 a fast-start curve must already be past the
        // linear baseline of 0.25.
        assert!(ease_out_quad(0.25) > 0.25);
    }

    #[test]
    fn ease_in_quad_endpoints() {
        assert!((ease_in_quad(0.0) - 0.0).abs() < 1e-6);
        assert!((ease_in_quad(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn ease_in_quad_is_slower_at_start() {
        // At t = 0.25 a slow-start curve must be below the linear baseline.
        assert!(ease_in_quad(0.25) < 0.25);
    }

    #[test]
    fn clamp_out_of_range() {
        assert_eq!(ease_out_quad(-1.0), 0.0);
        assert_eq!(ease_out_quad(2.0), 1.0);
        assert_eq!(ease_in_quad(-1.0), 0.0);
        assert_eq!(ease_in_quad(2.0), 1.0);
    }

    #[test]
    fn lerp_endpoints() {
        let a = Color32::from_rgb(0, 0, 0);
        let b = Color32::from_rgb(255, 255, 255);
        assert_eq!(lerp(a, b, 0.0), a);
        assert_eq!(lerp(a, b, 1.0), b);
    }
}
