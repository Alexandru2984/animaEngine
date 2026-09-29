//! Drawing speech bubbles (`crate::speech`): a white rounded box over the
//! character with a tail pointing at it, whatever the theme — a bubble
//! sits on the desktop, not on the panel, and dark text on white reads on
//! any wallpaper. On egui's background layer, so the settings panel
//! covers it as it covers the characters, and click-through like them.

use crate::speech::Shown;

/// The widest a bubble grows before wrapping, in points.
const MAX_WIDTH: f32 = 220.0;
const PADDING: egui::Vec2 = egui::vec2(10.0, 7.0);
const TAIL: f32 = 9.0;
const CORNER: f32 = 10.0;

/// Draw `bubbles`, whose rectangles are in surface pixels; `pixels_per_point`
/// turns them into egui's points. Asks for the frame that clears each one.
pub fn paint(ctx: &egui::Context, bubbles: &[Shown], pixels_per_point: f32) {
    if bubbles.is_empty() {
        return;
    }
    // A reminder in Chinese, Japanese or Korean under a Latin locale would
    // otherwise read as empty boxes (R33); loading the face is guarded.
    if bubbles
        .iter()
        .any(|b| crate::ui::icons::text_needs_cjk(&b.text))
    {
        crate::ui::icons::install_with_cjk(ctx);
    }
    let painter = ctx.layer_painter(egui::LayerId::background());
    let screen = ctx.screen_rect();
    let area = (screen.left(), screen.top(), screen.right(), screen.bottom());
    let fill = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 245);
    let ink = egui::Color32::from_rgb(28, 28, 32);
    let edge = egui::Stroke::new(1.0_f32, egui::Color32::from_gray(150));
    let now = std::time::Instant::now();
    for bubble in bubbles {
        let (l, t, r, b) = bubble.character;
        let ppp = pixels_per_point.max(0.1);
        let character = (l / ppp, t / ppp, r / ppp, b / ppp);
        let galley = painter.layout(
            bubble.text.clone(),
            egui::FontId::proportional(14.0),
            ink,
            MAX_WIDTH,
        );
        let size = galley.size() + PADDING * 2.0;
        let ((x, y), below) = crate::speech::place(character, (size.x, size.y), area, TAIL);
        let rect = egui::Rect::from_min_size(egui::pos2(x, y), size);
        painter.rect(rect, CORNER, fill, edge, egui::StrokeKind::Inside);

        // The tail, from the edge nearest the character towards its
        // middle, kept clear of the rounded corners.
        let cx = ((character.0 + character.2) / 2.0)
            .clamp(rect.left() + CORNER + 6.0, rect.right() - CORNER - 6.0);
        let (base, tip) = if below {
            (rect.top() + 1.0, rect.top() - TAIL)
        } else {
            (rect.bottom() - 1.0, rect.bottom() + TAIL)
        };
        let left = egui::pos2(cx - 7.0, base);
        let right = egui::pos2(cx + 7.0, base);
        let point = egui::pos2(cx, tip);
        painter.add(egui::Shape::convex_polygon(
            vec![left, right, point],
            fill,
            egui::Stroke::NONE,
        ));
        painter.line_segment([left, point], edge);
        painter.line_segment([right, point], edge);

        painter.galley(rect.min + PADDING, galley, ink);
        ctx.request_repaint_after(bubble.until.saturating_duration_since(now));
    }
}
