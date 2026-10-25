//! The heart: a glossy red heart beating twice a loop — lub-dub — with
//! sparkles circling it; it blinks once a loop.

use super::paint::{self, rgb, Canvas};
use super::{FRAMES, SIZE};
use std::path::Path;

const NAME: &str = "heart";

pub fn generate(base: &Path) {
    let Some(dir) = super::prepare(base, NAME, SIZE, FRAMES) else {
        return;
    };
    for f in 0..FRAMES {
        super::save_frame(&dir, f + 1, &frame(f));
    }
    tracing::info!("Generated {FRAMES} heart demo frames ({SIZE}×{SIZE})");
}

fn frame(f: u32) -> image::RgbaImage {
    let t = f as f32 / FRAMES as f32;
    let phase = t * std::f32::consts::TAU;
    // Lub-dub: a beat, a smaller one just after, then rest.
    let pulse = |at: f32, width: f32| {
        let d = ((t - at + 0.5).rem_euclid(1.0) - 0.5) / width;
        (-d * d).exp()
    };
    let beat = 1.0 + 0.08 * pulse(0.0, 0.06) + 0.05 * pulse(0.17, 0.05);
    let cx = 64.0;
    let size = 54.0 * beat;
    let tip = 66.0 + size * 0.55;
    let body = paint::heart(cx, tip, size);
    let mid_y = tip - size * 0.62;

    let base = rgb(240, 72, 112);
    let ink = rgb(120, 18, 48);
    let mut c = Canvas::new(SIZE);
    c.ground_shadow(cx, 117.0, 20.0 * beat, 4.0, 0.26);
    c.solid(paint::grow(&body, 2.0), ink, 1.0);
    c.fill(&body, |x, y| {
        (
            paint::lit(base, x, y, cx - 8.0, mid_y - 6.0, 36.0, 34.0),
            1.0,
        )
    });
    c.solid(
        paint::ellipse(cx - 20.0 * beat, mid_y - 16.0 * beat, 7.0, 4.0),
        [1.0; 3],
        0.55,
    );
    let open = if f == 10 { 0.0 } else { 1.0 };
    paint::eyes(&mut c, cx, mid_y + 2.0, 22.0, 4.5, open, rgb(70, 10, 30));
    paint::smile(&mut c, cx, mid_y + 13.0, 7.0, rgb(70, 10, 30));
    paint::blush(&mut c, cx, mid_y + 10.0, 34.0, 4.5);
    // Two sparkles going round.
    for k in 0..2 {
        let a = phase + k as f32 * std::f32::consts::PI;
        let (sx, sy) = (cx + 50.0 * a.cos(), mid_y + 8.0 + 40.0 * a.sin());
        let alpha = 0.5 + 0.5 * (phase * 2.0 + k as f32).sin();
        paint::sparkle(&mut c, sx, sy, 5.0, alpha);
    }
    c.into_image()
}
