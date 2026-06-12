//! Toast overlay (bottom-right stack). Extracted in I.1 so the
//! main panels module stops carrying its 70-line render path.
//!
//! `toasts()` is the entry point — it owns the egui Area and asks
//! egui to keep repainting while the queue is non-empty so cards
//! disappear at the exact moment they expire instead of waiting
//! for the next input event.

use crate::anim;
use crate::ui::icons;
use crate::ui::theme::{self, SPACE_L, SPACE_M, SPACE_S};
use crate::ui::toasts::{Toast, ToastKind, ToastQueue};

pub fn toasts(ctx: &egui::Context, queue: &ToastQueue) {
    if queue.is_empty() {
        return;
    }

    // While there are visible toasts, drive continuous repaints so they
    // disappear at the moment they expire (without waiting for the next
    // input event).
    ctx.request_repaint();

    egui::Area::new(egui::Id::new("anima_toasts"))
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-SPACE_L, -SPACE_L))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                for toast in queue.iter() {
                    toast_card(ui, toast);
                    ui.add_space(SPACE_S);
                }
            });
        });
}

fn toast_card(ui: &mut egui::Ui, toast: &Toast) {
    // ── Per design-system §6 micro-animation timings ─────────────────
    // - Slide-in fade:  200 ms, ease-out-quad
    // - Fade-out:       300 ms, ease-in-quad (trailing window before expiry)
    const SLIDE_IN: f32 = 0.200;
    const FADE_OUT: f32 = 0.300;
    /// Vertical travel during slide-in — enough to read as motion,
    /// small enough to never overlap the card above.
    const SLIDE_PX: f32 = 8.0;
    let reduced = crate::ui::motion::reduced(ui.ctx());
    let age = toast.age().as_secs_f32();
    let remaining = toast.remaining().as_secs_f32();
    let in_alpha = anim::ease_out_quad((age / SLIDE_IN).min(1.0));
    let out_alpha = if remaining < FADE_OUT {
        1.0 - anim::ease_in_quad(((FADE_OUT - remaining) / FADE_OUT).clamp(0.0, 1.0))
    } else {
        1.0
    };
    let alpha = if reduced {
        1.0
    } else {
        (in_alpha * out_alpha).clamp(0.0, 1.0)
    };
    if !reduced {
        // Slide up into place while fading in (bottom-up layout, so
        // leading space pushes the card down → it rises as it fades).
        ui.add_space(SLIDE_PX * (1.0 - in_alpha));
    }

    let visuals = ui.visuals();
    let bg = visuals.faint_bg_color; // bg.elevated per theme
    let body_fg = visuals.text_color(); // fg.primary
    let severity_fg = match toast.kind {
        ToastKind::Info => visuals.hyperlink_color, // info / accent tone
        ToastKind::Success => crate::ui::theme::palette_of(ui.ctx()).semantic_success,
        ToastKind::Warn => visuals.warn_fg_color,
        ToastKind::Error => visuals.error_fg_color,
    };
    let icon = match toast.kind {
        ToastKind::Info => icons::INFO,
        ToastKind::Success => icons::SUCCESS,
        ToastKind::Warn => icons::WARN,
        ToastKind::Error => icons::ERROR,
    };

    ui.scope(|ui| {
        ui.set_opacity(alpha);
        egui::Frame::new()
            .fill(bg)
            .corner_radius(theme::RADIUS_LG)
            .inner_margin(egui::Margin::symmetric(SPACE_L as i8, SPACE_M as i8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(icon).size(18.0).color(severity_fg));
                    ui.add_space(SPACE_S);
                    ui.colored_label(body_fg, &toast.message);
                });
            });
    });
}
