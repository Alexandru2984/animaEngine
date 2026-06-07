//! Empty / error / loading state helpers — the live implementation of
//! `docs/design-system.md` §8.
//!
//! Three exports:
//!
//! - [`empty`]   — centered icon + headline + hint, for panels with no
//!   data yet ("nothing selected", "empty scene")
//! - [`error`]   — same shape but the icon is the design-system error
//!   tone and an optional action button sits at the bottom (retry, etc.)
//! - [`spinner`] — three-dot pulsing indicator with staggered alpha.
//!   Local to a panel — never a full-screen overlay.
//!
//! Keeping these in a single module means every "we have nothing to
//! show" branch in the UI ends up looking the same; readers can
//! `grep states::` to find every such branch.

use crate::ui::theme::{self, h2, SPACE_2XL, SPACE_M, SPACE_S, SPACE_XS};

/// Empty-state card. Renders centered in whatever container `ui` is.
///
/// Use for "nothing selected" / "no entities" / "no results" — places
/// where the absence of data is *expected* and the user just needs
/// guidance toward the next action.
pub fn empty(ui: &mut egui::Ui, icon: &str, headline: &str, hint: &str) {
    let _ = empty_with_action(ui, icon, headline, hint, None);
}

/// Same as [`empty`] but with an optional CTA button at the bottom.
/// Returns `true` for the frame the user clicked the button so the
/// caller can route the action (insert a demo preset, mkdir the
/// asset root, etc.). When `action_label` is `None`, the behaviour
/// is identical to `empty`.
#[must_use]
pub fn empty_with_action(
    ui: &mut egui::Ui,
    icon: &str,
    headline: &str,
    hint: &str,
    action_label: Option<&str>,
) -> bool {
    ui.add_space(SPACE_2XL);
    let mut clicked = false;
    ui.vertical_centered(|ui| {
        ui.label(
            egui::RichText::new(icon)
                .size(40.0)
                .color(ui.visuals().weak_text_color()),
        );
        ui.add_space(SPACE_M);
        ui.label(egui::RichText::new(headline).text_style(h2()));
        ui.add_space(SPACE_XS);
        ui.label(
            egui::RichText::new(hint)
                .text_style(theme::caption())
                .weak(),
        );
        if let Some(label) = action_label {
            ui.add_space(SPACE_M);
            if ui.button(label).clicked() {
                clicked = true;
            }
        }
    });
    clicked
}

/// Error-state card. Same shape as [`empty`] but tinted with the active
/// theme's error tone and optionally accompanied by an action button.
///
/// Returns `true` for the frame the user clicked the action button —
/// `false` (or always `false` when `action_label` is `None`).
///
/// Use for asset-load failures, decode errors, "couldn't reach disk
/// cache" — places where the user needs to know *something went wrong*
/// and how to recover.
#[must_use]
pub fn error(
    ui: &mut egui::Ui,
    icon: &str,
    headline: &str,
    detail: &str,
    action_label: Option<&str>,
) -> bool {
    ui.add_space(SPACE_2XL);
    let mut clicked = false;
    ui.vertical_centered(|ui| {
        let error_color = ui.visuals().error_fg_color;
        ui.label(egui::RichText::new(icon).size(40.0).color(error_color));
        ui.add_space(SPACE_M);
        ui.label(
            egui::RichText::new(headline)
                .text_style(h2())
                .color(error_color),
        );
        ui.add_space(SPACE_XS);
        ui.label(
            egui::RichText::new(detail)
                .text_style(theme::caption())
                .weak(),
        );
        if let Some(label) = action_label {
            ui.add_space(SPACE_M);
            if ui.button(label).clicked() {
                clicked = true;
            }
        }
    });
    clicked
}

/// Three-dot loading indicator. Painted inline at the current cursor.
///
/// Each dot pulses on a `1.2s` cycle with `0.4s` phase between dots, so
/// the eye reads a "left-to-right" wave. Alpha modulation keeps the
/// indicator quiet — never a flashy hero. Calls `ctx.request_repaint`
/// internally so animation continues without an input event.
///
/// Use for hot-reload in progress / disk decode pending / waiting on
/// any task ≤ a few seconds. For longer waits, prefer an explicit
/// progress bar.
pub fn spinner(ui: &mut egui::Ui, label: Option<&str>) {
    let ctx = ui.ctx().clone();
    ctx.request_repaint();
    let now = ctx.input(|i| i.time);

    let accent = {
        // Resolve from the active palette via egui's selection stroke,
        // which `theme::apply` sets to accent_base. Doing it this way
        // keeps the spinner colour theme-aware without a second lookup.
        ui.visuals().selection.stroke.color
    };

    ui.horizontal(|ui| {
        // Allocate room for three dots so the layout doesn't jitter.
        let dot_radius: f32 = 4.0;
        let gap: f32 = 8.0;
        let total_w = (dot_radius * 2.0) * 3.0 + gap * 2.0;
        let (rect, _resp) = ui.allocate_exact_size(
            egui::vec2(total_w, dot_radius * 2.0 + SPACE_S),
            egui::Sense::hover(),
        );
        let painter = ui.painter_at(rect);
        let y = rect.center().y;
        let x0 = rect.left() + dot_radius;
        for i in 0..3 {
            let cx = x0 + (i as f32) * (dot_radius * 2.0 + gap);
            let alpha = dot_alpha(now, i);
            painter.circle_filled(egui::pos2(cx, y), dot_radius, accent.gamma_multiply(alpha));
        }

        if let Some(label) = label {
            ui.add_space(SPACE_S);
            ui.label(
                egui::RichText::new(label)
                    .text_style(theme::caption())
                    .weak(),
            );
        }
    });
}

/// Alpha for the `i`-th dot at absolute time `t` (seconds).
///
/// Split out so the timing curve has a pure-function test. Range
/// `[MIN_ALPHA, 1.0]` — never fully transparent, so the indicator
/// remains a stable three-dot shape (just modulating, not blinking).
fn dot_alpha(t: f64, i: usize) -> f32 {
    const PERIOD: f64 = 1.2;
    const PHASE: f64 = 0.4;
    const MIN_ALPHA: f32 = 0.30;
    let phase = ((t - (i as f64) * PHASE) / PERIOD).fract();
    // Cosine wave normalized to [0, 1] then lifted to [MIN_ALPHA, 1.0].
    let wave = 0.5 - 0.5 * (phase * std::f64::consts::TAU).cos();
    MIN_ALPHA + (1.0 - MIN_ALPHA) * wave as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dot_alpha_stays_in_range() {
        // Sweep over two periods and check every dot's alpha is bounded.
        for n in 0..2400 {
            let t = (n as f64) * 0.001; // 1 ms steps
            for i in 0..3 {
                let a = dot_alpha(t, i);
                assert!(
                    (0.29..=1.01).contains(&a),
                    "dot {i} at t={t:.3}s gave alpha {a}",
                );
            }
        }
    }

    #[test]
    fn dots_are_staggered() {
        // At t = 0 the three dots are 0.4s apart in phase; their alphas
        // should not all collide on the same value.
        let a0 = dot_alpha(0.0, 0);
        let a1 = dot_alpha(0.0, 1);
        let a2 = dot_alpha(0.0, 2);
        assert!(
            (a0 - a1).abs() > 0.01 || (a1 - a2).abs() > 0.01,
            "all three dots collapsed to the same alpha ({a0}, {a1}, {a2})",
        );
    }

    #[test]
    fn dot_alpha_repeats_with_period() {
        // At t = 0 vs t = PERIOD the same dot should be at the same alpha.
        let baseline = dot_alpha(0.5, 0);
        let after = dot_alpha(0.5 + 1.2, 0);
        assert!((baseline - after).abs() < 1e-3);
    }
}
