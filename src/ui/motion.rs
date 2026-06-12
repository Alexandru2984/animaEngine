//! Reduced-motion plumbing (V.1, audit F9).
//!
//! The flag is stored per-frame in egui temp memory so every panel,
//! toast and overlay can consult it without parameter threading. The
//! app sets it at the top of each egui pass from
//! `config.global.reduced_motion`; helpers below collapse animation
//! durations to zero when it's on. egui's *built-in* animations
//! (collapsing headers, panel slide) are covered separately by
//! zeroing `style.animation_time` in [`set_reduced`].

use egui::Id;

fn flag_id() -> Id {
    Id::new("anima.reduced_motion")
}

/// Record the reduced-motion preference for this frame and align
/// egui's built-in animation clock with it. Call once per egui pass,
/// before any panel paints.
pub fn set_reduced(ctx: &egui::Context, reduced: bool) {
    ctx.data_mut(|d| d.insert_temp(flag_id(), reduced));
    let target = if reduced { 0.0 } else { 0.1 };
    if ctx.style().animation_time != target {
        ctx.style_mut(|s| s.animation_time = target);
    }
}

/// Whether reduced motion is on this frame.
pub fn reduced(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp(flag_id()).unwrap_or(false))
}

/// An animation duration honoring the preference: `base` seconds
/// normally, `0.0` (instant) under reduced motion.
pub fn time(ctx: &egui::Context, base: f32) -> f32 {
    if reduced(ctx) {
        0.0
    } else {
        base
    }
}
