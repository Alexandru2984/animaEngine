//! Drawing the little effects (`crate::effects`): hearts, puffs of dust
//! and z's. On egui's background layer like the speech bubbles, under
//! them, so the panel covers them as it covers the characters.

use crate::effects::{Drawn, Kind};

/// Points around a heart's outline: more look no smoother at this size.
const HEART_POINTS: usize = 28;

/// Draw `effects`, whose positions are in surface pixels; `pixels_per_point`
/// turns them into egui's points. Asks for the next frame while any shows.
pub fn paint(ctx: &egui::Context, effects: &[Drawn], pixels_per_point: f32) {
    if effects.is_empty() {
        return;
    }
    let painter = ctx.layer_painter(egui::LayerId::background());
    let ppp = pixels_per_point.max(0.1);
    for effect in effects {
        let at = egui::pos2(effect.at.0 / ppp, effect.at.1 / ppp);
        let size = effect.size / ppp;
        let alpha = (effect.alpha.clamp(0.0, 1.0) * 255.0) as u8;
        match effect.kind {
            Kind::Heart => heart(&painter, at, size, alpha, ppp),
            Kind::Dust => {
                let dust = egui::Color32::from_rgba_unmultiplied(205, 195, 180, alpha);
                painter.circle_filled(at, size, dust);
            }
            Kind::Zzz => {
                // Light with a dark shadow, so it reads on any wallpaper.
                let font = egui::FontId::proportional(size);
                let shadow = egui::Color32::from_rgba_unmultiplied(30, 30, 40, alpha / 2);
                let ink = egui::Color32::from_rgba_unmultiplied(235, 240, 255, alpha);
                let align = egui::Align2::CENTER_CENTER;
                painter.text(at + egui::vec2(1.0, 1.0), align, "z", font.clone(), shadow);
                painter.text(at, align, "z", font, ink);
            }
        }
    }
    ctx.request_repaint();
}

/// A heart of half-width `size` centred on `at`: a fan of triangles from
/// its middle, which sees all of its outline, and a one-pixel fringe
/// fading out around it for a smooth edge.
fn heart(painter: &egui::Painter, at: egui::Pos2, size: f32, alpha: u8, ppp: f32) {
    let fill = egui::Color32::from_rgba_unmultiplied(240, 84, 128, alpha);
    let scale = size / 16.0;
    // The classic curve; y grows downwards here, and its middle sits a
    // little below the dip between the lobes.
    let outline: Vec<egui::Pos2> = (0..HEART_POINTS)
        .map(|i| {
            let t = i as f32 / HEART_POINTS as f32 * std::f32::consts::TAU;
            let x = 16.0 * t.sin().powi(3);
            let y =
                13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
            egui::pos2(at.x + x * scale, at.y - (y + 2.0) * scale)
        })
        .collect();
    let feather = 1.0 / ppp;
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(at, fill);
    for &p in &outline {
        let out = (p - at).normalized() * feather;
        mesh.colored_vertex(p, fill);
        mesh.colored_vertex(p + out, egui::Color32::TRANSPARENT);
    }
    let n = HEART_POINTS as u32;
    for i in 0..n {
        let j = (i + 1) % n;
        let (inner_i, outer_i) = (1 + 2 * i, 2 + 2 * i);
        let (inner_j, outer_j) = (1 + 2 * j, 2 + 2 * j);
        mesh.add_triangle(0, inner_i, inner_j);
        mesh.add_triangle(inner_i, outer_i, outer_j);
        mesh.add_triangle(inner_i, outer_j, inner_j);
    }
    painter.add(egui::Shape::mesh(mesh));
}
