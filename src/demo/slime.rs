//! The slime: a glossy green jelly that squashes and stretches, with
//! bubbles rising inside; it blinks once a loop.

use super::paint::{self, blend, circle, ellipse, rgb, Canvas};
use super::{FRAMES, SIZE};
use std::path::Path;

const NAME: &str = "slime";

pub fn generate(base: &Path) {
    let Some(dir) = super::prepare(base, NAME, SIZE, FRAMES) else {
        return;
    };
    for f in 0..FRAMES {
        super::save_frame(&dir, f + 1, &frame(f, 1));
    }
    tracing::info!("Generated {FRAMES} slime demo frames ({SIZE}×{SIZE})");
}

pub(super) fn frame(f: u32, scale: u32) -> image::RgbaImage {
    let phase = f as f32 / FRAMES as f32 * std::f32::consts::TAU;
    let squash = phase.sin();
    let floor = 110.0;
    let h = 38.0 * (1.0 + 0.1 * squash);
    let w = 44.0 * (1.0 - 0.07 * squash);
    let cx = 64.0;
    let cy = floor - h * 0.6;
    let dome = ellipse(cx, cy, w, h * 0.8);
    let feet = paint::union(
        circle(cx - w + 7.0, floor - 5.0, 7.0),
        circle(cx + w - 7.0, floor - 5.0, 7.0),
    );
    let rounded = blend(dome, feet, 6.0);
    let body = move |x: f32, y: f32| rounded(x, y).max(y - floor);

    let base = rgb(96, 214, 112);
    let ink = rgb(28, 104, 48);
    let mut c = Canvas::scaled(SIZE, scale);
    c.ground_shadow(cx, floor + 3.0, w * 0.95, 5.0, 0.3);
    c.solid(paint::grow(&body, 2.0), ink, 1.0);
    c.fill(&body, |x, y| {
        (paint::lit(base, x, y, cx - 6.0, cy - 4.0, w, h * 0.9), 0.94)
    });
    // Bubbles rising inside, each at its own pace.
    for (i, (bx, speed, r)) in [
        (-18.0_f32, 1.0_f32, 3.0_f32),
        (14.0, 1.6, 2.2),
        (2.0, 0.7, 1.8),
    ]
    .iter()
    .enumerate()
    {
        let t = ((phase / std::f32::consts::TAU) * speed + i as f32 * 0.33).fract();
        let by = floor - 6.0 - t * h * 1.1;
        c.fill(circle(cx + bx, by, *r), |_, _| {
            (paint::shade(base, 1.6), 0.45 * (1.0 - t))
        });
    }
    // The gloss of jelly.
    c.solid(
        ellipse(cx - w * 0.42, cy - h * 0.42, 9.0, 4.5),
        [1.0; 3],
        0.55,
    );
    c.solid(circle(cx - w * 0.2, cy - h * 0.55, 2.2), [1.0; 3], 0.7);
    let open = if f == 6 { 0.0 } else { 1.0 };
    paint::eyes(&mut c, cx, cy + 4.0, 26.0, 5.0, open, rgb(20, 60, 30));
    paint::smile(&mut c, cx, cy + 16.0, 9.0, rgb(20, 60, 30));
    paint::blush(&mut c, cx, cy + 12.0, 40.0, 5.0);
    c.into_image()
}
