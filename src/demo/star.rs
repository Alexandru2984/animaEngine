//! The star: a golden five-pointed star rocking from side to side, with
//! sparkles twinkling around it; it blinks once a loop.

use super::paint::{self, rgb, Canvas};
use super::{FRAMES, SIZE};
use std::path::Path;

const NAME: &str = "star";

pub fn generate(base: &Path) {
    let Some(dir) = super::prepare(base, NAME, SIZE, FRAMES) else {
        return;
    };
    for f in 0..FRAMES {
        super::save_frame(&dir, f + 1, &frame(f, 1));
    }
    tracing::info!("Generated {FRAMES} star demo frames ({SIZE}×{SIZE})");
}

pub(super) fn frame(f: u32, scale: u32) -> image::RgbaImage {
    let phase = f as f32 / FRAMES as f32 * std::f32::consts::TAU;
    let rock = 0.16 * phase.sin();
    let swell = 1.0 + 0.03 * (phase * 2.0).sin();
    let (cx, cy) = (64.0, 64.0);
    let body = paint::star(cx, cy, 46.0 * swell, 0.5, rock, 5.0);

    let base = rgb(255, 204, 60);
    let ink = rgb(160, 100, 16);
    let mut c = Canvas::scaled(SIZE, scale);
    c.ground_shadow(cx, 118.0, 22.0, 4.0, 0.22);
    c.solid(paint::grow(&body, 2.0), ink, 1.0);
    c.fill(&body, |x, y| {
        (paint::lit(base, x, y, cx - 8.0, cy - 6.0, 40.0, 40.0), 1.0)
    });
    // The face turns with the star, a little.
    let fx = cx + rock * 14.0;
    let open = if f == 3 { 0.0 } else { 1.0 };
    paint::eyes(&mut c, fx, cy + 2.0, 18.0, 4.2, open, rgb(90, 50, 8));
    paint::smile(&mut c, fx, cy + 12.0, 6.0, rgb(90, 50, 8));
    paint::blush(&mut c, fx, cy + 9.0, 30.0, 4.0);
    for (k, (sx, sy)) in [(18.0_f32, 26.0_f32), (110.0, 30.0), (104.0, 100.0)]
        .iter()
        .enumerate()
    {
        let alpha = (0.5 + 0.5 * (phase + k as f32 * 2.1).sin()).powi(2);
        paint::sparkle(&mut c, *sx, *sy, 6.0, alpha);
    }
    c.into_image()
}
