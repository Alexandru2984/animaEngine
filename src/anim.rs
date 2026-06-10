//! Easing curves and small interpolation helpers — pure functions,
//! zero side effects, no egui or wgpu dependencies.
//!
//! This module serves two callers:
//!
//! - **UI micro-animations** (`crate::ui::panels`) — toast slide-in,
//!   tab fade, command-palette pop-in. The original home for these
//!   helpers was `crate::ui::anim`; in 0.3 they got promoted because
//!   the per-frame animation pipeline (`crate::animation::Animation`)
//!   wants the same curves for its own easing.
//! - **Per-frame easing on sprite animations** — `EasingCurve` is a
//!   serializable enum that pulls or pushes the frame timing along
//!   one of six standard curves while preserving the loop's total
//!   duration. See `crate::animation::Animation::easing`.
//!
//! Long-running animation state (per-widget hover progress, etc.) is
//! still handled by `egui::Context::animate_*` at the call site; this
//! module deliberately stays state-free so the timing curves can be
//! unit-tested without an egui context.

use egui::Color32;
use serde::{Deserialize, Serialize};

// ─── UI animation helpers (pre-existing exports) ─────────────────────

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

// ─── Per-frame easing curves ─────────────────────────────────────────

/// Serializable selection of an easing curve, stored on
/// `Animation::easing`. Each variant carries the same contract: a
/// pure function `f: [0, 1] → [0, 1]` with `f(0) = 0` and `f(1) = 1`,
/// so an animation under any curve still loops at exactly its
/// configured total duration.
///
/// The defaults are tuned for cartoon-style sprite work — `Linear`
/// matches the 0.2 behaviour exactly (every frame held for the same
/// `1 / fps` seconds), and `EaseOutQuad` is the recommended starting
/// point for hand-drawn cycles where the action should hit early and
/// settle.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EasingCurve {
    /// Identity: every frame held for the same duration. Matches the
    /// pre-0.3 behaviour exactly.
    #[default]
    Linear,
    /// Slow start, fast finish. `f(t) = t²`.
    EaseInQuad,
    /// Fast start, slow finish. `f(t) = 1 - (1 - t)²`.
    EaseOutQuad,
    /// Slow start, fast middle, slow finish — the gentlest of the
    /// three quads.
    EaseInOutQuad,
    /// Sinusoidal ease. `f(t) = (1 - cos(πt)) / 2`. Softer than the
    /// quadratics on both ends; popular for ambient idle loops.
    Sine,
    /// Bouncing finish — overshoots and settles, hitting `f(1) = 1`
    /// exactly. Standard formula from easings.net.
    BounceOut,
}

impl EasingCurve {
    /// The six variants in their canonical UI display order.
    pub const ALL: &'static [Self] = &[
        Self::Linear,
        Self::EaseInQuad,
        Self::EaseOutQuad,
        Self::EaseInOutQuad,
        Self::Sine,
        Self::BounceOut,
    ];

    /// Stable i18n key per variant; the UI looks up the localised
    /// label via this slug.
    pub fn i18n_key(self) -> &'static str {
        match self {
            Self::Linear => "easing-linear",
            Self::EaseInQuad => "easing-ease-in-quad",
            Self::EaseOutQuad => "easing-ease-out-quad",
            Self::EaseInOutQuad => "easing-ease-in-out-quad",
            Self::Sine => "easing-sine",
            Self::BounceOut => "easing-bounce-out",
        }
    }

    /// Evaluate the curve at `t`, clamped to `[0, 1]`. Always returns
    /// a value in `[0, 1]` for the standard curves; `BounceOut`
    /// undershoots transiently but recovers to `f(1) = 1`.
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::EaseInQuad => t * t,
            Self::EaseOutQuad => 1.0 - (1.0 - t) * (1.0 - t),
            Self::EaseInOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            Self::Sine => 0.5 * (1.0 - (t * std::f32::consts::PI).cos()),
            Self::BounceOut => bounce_out(t),
        }
    }

    /// Compute the duration of one frame `(i, n)` under this curve,
    /// scaled so the sum across all `n` intervals equals
    /// `total_duration`. This is the key operation for
    /// `Animation::easing` — it distorts the *interval* between
    /// frames without changing the loop period.
    ///
    /// Math:
    /// - Linear baseline: each interval is `total_duration / n`.
    /// - Under curve `f`, the boundary times are
    ///   `f(0), f(1/n), f(2/n), …, f(n/n) = 1`, scaled by
    ///   `total_duration`.
    /// - The `i`-th interval is `|f((i+1)/n) - f(i/n)|`, normalised by
    ///   the sum of all `n` absolute deltas, times `total_duration`.
    ///
    /// The absolute value + normalisation matter for `BounceOut`: the
    /// curve is **not monotonic** (the ball drops between bounces), so
    /// raw deltas go negative on the descending segments. Without the
    /// normalisation those intervals would clamp to ~0 downstream and
    /// the loop would run shorter than `total_duration`. For monotonic
    /// curves the deltas are already positive and sum to exactly
    /// `f(1) - f(0) = 1`, so this reduces to the raw-delta formula.
    pub fn frame_interval(self, frame_index: usize, frame_count: usize, total: f32) -> f32 {
        if frame_count == 0 {
            return total;
        }
        let n = frame_count as f32;
        let i = frame_index as f32;
        let t0 = (i / n).clamp(0.0, 1.0);
        let t1 = ((i + 1.0) / n).clamp(0.0, 1.0);
        let delta = (self.apply(t1) - self.apply(t0)).abs();

        let abs_sum: f32 = (0..frame_count)
            .map(|k| {
                let a = (k as f32 / n).clamp(0.0, 1.0);
                let b = ((k as f32 + 1.0) / n).clamp(0.0, 1.0);
                (self.apply(b) - self.apply(a)).abs()
            })
            .sum();
        // Degenerate flat curve — split the loop evenly rather than
        // dividing by zero.
        if abs_sum <= f32::EPSILON {
            return total / n;
        }
        delta / abs_sum * total
    }
}

/// Classic bouncing easing — overshoots once near the end, then
/// settles to 1. Lifted directly from easings.net so behaviour
/// matches what users see in tween libraries elsewhere.
fn bounce_out(t: f32) -> f32 {
    const N1: f32 = 7.5625;
    const D1: f32 = 2.75;
    if t < 1.0 / D1 {
        N1 * t * t
    } else if t < 2.0 / D1 {
        let t = t - 1.5 / D1;
        N1 * t * t + 0.75
    } else if t < 2.5 / D1 {
        let t = t - 2.25 / D1;
        N1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / D1;
        N1 * t * t + 0.984375
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── UI helpers (round-trip from the old ui::anim module) ────────

    #[test]
    fn ease_out_quad_endpoints() {
        assert!((ease_out_quad(0.0) - 0.0).abs() < 1e-6);
        assert!((ease_out_quad(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn ease_out_quad_is_faster_at_start() {
        assert!(ease_out_quad(0.25) > 0.25);
    }

    #[test]
    fn ease_in_quad_endpoints() {
        assert!((ease_in_quad(0.0) - 0.0).abs() < 1e-6);
        assert!((ease_in_quad(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn ease_in_quad_is_slower_at_start() {
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

    // ── EasingCurve contract ────────────────────────────────────────

    #[test]
    fn default_curve_is_linear() {
        assert_eq!(EasingCurve::default(), EasingCurve::Linear);
    }

    /// Every curve must hit the canonical endpoints. If a future
    /// variant doesn't, total animation duration drifts and loops
    /// stop being seamless.
    #[test]
    fn every_curve_hits_zero_and_one() {
        for &c in EasingCurve::ALL {
            assert!(
                (c.apply(0.0) - 0.0).abs() < 1e-5,
                "{c:?} doesn't hit 0 at t=0",
            );
            assert!(
                (c.apply(1.0) - 1.0).abs() < 1e-5,
                "{c:?} doesn't hit 1 at t=1",
            );
        }
    }

    #[test]
    fn linear_curve_is_identity() {
        for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
            assert!((EasingCurve::Linear.apply(t) - t).abs() < 1e-6);
        }
    }

    #[test]
    fn ease_in_quad_curve_matches_helper() {
        for t in [0.0, 0.3, 0.7, 1.0] {
            let from_helper = ease_in_quad(t);
            let from_curve = EasingCurve::EaseInQuad.apply(t);
            assert!((from_helper - from_curve).abs() < 1e-6);
        }
    }

    #[test]
    fn ease_in_out_quad_is_symmetric() {
        // f(t) + f(1-t) should equal 1 for the in-out family.
        for t in [0.1, 0.25, 0.4] {
            let lhs = EasingCurve::EaseInOutQuad.apply(t);
            let rhs = EasingCurve::EaseInOutQuad.apply(1.0 - t);
            assert!((lhs + rhs - 1.0).abs() < 1e-5, "symmetry broken at t={t}");
        }
    }

    #[test]
    fn bounce_out_overshoots_then_settles() {
        // easings.net bounce-out splits the domain into four segments;
        // by t=0.95 we're in the last segment and the value is near 1.
        let near_end = EasingCurve::BounceOut.apply(0.95);
        assert!(
            near_end > 0.95,
            "bounce should be near 1 close to t=1, got {near_end}",
        );
        // Final value is exactly 1.
        assert!((EasingCurve::BounceOut.apply(1.0) - 1.0).abs() < 1e-6);
        // Mid-segment hits values that visibly "settle" — by t=0.4 the
        // curve has crossed half-progress and we're past the first
        // bounce inflection.
        let early = EasingCurve::BounceOut.apply(0.4);
        assert!(
            early > 0.3,
            "BounceOut should accelerate past 0.3 by t=0.4, got {early}",
        );
    }

    // ── Frame-interval distortion ───────────────────────────────────

    /// Linear curve gives uniform intervals.
    #[test]
    fn linear_frame_intervals_are_uniform() {
        let total = 1.2;
        let n = 8;
        for i in 0..n {
            let dt = EasingCurve::Linear.frame_interval(i, n, total);
            assert!((dt - total / n as f32).abs() < 1e-5);
        }
    }

    /// Intervals always sum to `total` under any curve — this is the
    /// "preserves loop duration" invariant.
    #[test]
    fn intervals_sum_to_total_duration_for_every_curve() {
        let total = 2.5;
        let n = 12;
        for &c in EasingCurve::ALL {
            let sum: f32 = (0..n).map(|i| c.frame_interval(i, n, total)).sum();
            assert!(
                (sum - total).abs() < 1e-3,
                "{c:?}: intervals sum {sum}, expected {total}",
            );
        }
    }

    /// EaseOut starts fast: the first interval must be longer than
    /// the last interval (under a curve that arrives quickly, the
    /// earlier frames hold longer in real time? No — read the math
    /// carefully). The mapping is `time → progress`, so an EaseOut
    /// progress function means early time covers more progress, so
    /// successive frames advance faster — i.e. **earlier intervals
    /// are shorter** in real time. This guards against an inversion
    /// when refactoring.
    #[test]
    fn ease_out_frame_intervals_are_front_loaded_in_progress() {
        let total = 1.0;
        let n = 10;
        let first = EasingCurve::EaseOutQuad.frame_interval(0, n, total);
        let last = EasingCurve::EaseOutQuad.frame_interval(n - 1, n, total);
        assert!(
            first > last,
            "EaseOut: expected first ({first}) > last ({last}) — under \
             progress = ease_out(time), early time means more progress \
             per second, so a frame's time slice shrinks toward the end.",
        );
    }

    #[test]
    fn frame_interval_handles_zero_frame_count() {
        let dt = EasingCurve::Linear.frame_interval(0, 0, 0.5);
        // Defensive: return `total` so the caller can fall back.
        assert!((dt - 0.5).abs() < 1e-6);
    }

    /// Every interval is non-negative under every curve. BounceOut is
    /// the curve this guards: it is not monotonic (the ball descends
    /// between bounces), so raw boundary deltas go negative there —
    /// the |delta| normalisation must absorb that without breaking
    /// the sum-to-total invariant (checked separately above).
    #[test]
    fn frame_intervals_are_non_negative_for_every_curve() {
        let total = 2.0;
        for &c in EasingCurve::ALL {
            // Small frame counts stress the bounce segments hardest.
            for n in [2usize, 3, 5, 12, 60] {
                for i in 0..n {
                    let dt = c.frame_interval(i, n, total);
                    assert!(dt >= 0.0, "{c:?} n={n} i={i}: negative interval {dt}",);
                }
            }
        }
    }

    /// BounceOut specifically: the descending bounce segments used to
    /// produce negative intervals that the animation layer clamped to
    /// 1 ms, silently shortening the loop. With |delta| normalisation
    /// the sum must stay exact even at frame counts that straddle the
    /// bounce inflection points.
    #[test]
    fn bounce_out_sum_stays_total_at_awkward_frame_counts() {
        let total = 1.0;
        for n in [2usize, 3, 4, 7, 11] {
            let sum: f32 = (0..n)
                .map(|i| EasingCurve::BounceOut.frame_interval(i, n, total))
                .sum();
            assert!(
                (sum - total).abs() < 1e-3,
                "BounceOut n={n}: sum {sum} != {total}",
            );
        }
    }
}
