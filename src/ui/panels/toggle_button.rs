//! Top-right ⚙ toggle button overlay. Extracted in I.2.
//!
//! Drawn in BOTH pass-through and edit mode — it's the only egui
//! widget visible in pass-through and the way the user re-enters
//! edit mode. Returns `true` when clicked so the caller can flip
//! `App::edit_mode`.

use crate::constants::TOGGLE_BUTTON_SIZE;
use crate::ui::icons;

pub fn toggle_button(ctx: &egui::Context, edit_mode: bool) -> bool {
    let size = TOGGLE_BUTTON_SIZE as f32;
    let screen = ctx.screen_rect();
    let pos = egui::pos2(screen.right() - size, 0.0);

    let bg = if edit_mode {
        egui::Color32::from_rgb(40, 160, 60) // active = green
    } else {
        egui::Color32::from_rgba_unmultiplied(50, 50, 60, 200) // pass-through = dim
    };
    let tooltip = if edit_mode {
        "Exit edit mode"
    } else {
        "Enter edit mode"
    };

    let mut clicked = false;
    egui::Area::new(egui::Id::new("anima_toggle_button"))
        .fixed_pos(pos)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let response = ui
                .add_sized(
                    egui::vec2(size, size),
                    egui::Button::new(
                        egui::RichText::new(icons::SETTINGS)
                            .size(28.0)
                            .color(egui::Color32::WHITE),
                    )
                    .fill(bg)
                    .corner_radius(0.0),
                )
                .on_hover_text(tooltip);
            // Visible focus for keyboard users (F8): the custom fill
            // swallows egui's subtle default, so paint an explicit
            // ring with the theme's selection stroke (HC-aware).
            if response.has_focus() {
                let stroke = ui.visuals().selection.stroke;
                ui.painter().rect_stroke(
                    response.rect.shrink(1.5),
                    0.0,
                    egui::Stroke::new(2.0, stroke.color),
                    egui::StrokeKind::Inside,
                );
            }
            if response.clicked() {
                clicked = true;
            }
        });
    clicked
}
