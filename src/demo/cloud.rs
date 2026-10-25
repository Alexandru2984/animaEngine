//! The cloud: a soft white cloud drifting up and down, puffing gently,
//! a few drops of rain falling under it; it blinks once a loop.

use super::paint::{self, blend, capsule, circle, rgb, Canvas};
use super::{FRAMES, SIZE};
use std::path::Path;

const NAME: &str = "cloud";

pub fn generate(base: &Path) {
    let Some(dir) = super::prepare(base, NAME, SIZE, FRAMES) else {
        return;
    };
    for f in 0..FRAMES {
        super::save_frame(&dir, f + 1, &frame(f, 1));
    }
    tracing::info!("Generated {FRAMES} cloud demo frames ({SIZE}×{SIZE})");
}

pub(super) fn frame(f: u32, scale: u32) -> image::RgbaImage {
    let t = f as f32 / FRAMES as f32;
    let phase = t * std::f32::consts::TAU;
    let drift = 3.0 * phase.sin();
    let puff = 1.0 + 0.04 * (phase * 2.0).sin();
    let cx = 64.0;
    let cy = 56.0 + drift;
    let puffs = blend(
        blend(
            circle(cx - 22.0, cy + 6.0, 18.0 * puff),
            circle(cx, cy - 6.0, 24.0 * puff),
            6.0,
        ),
        blend(
            circle(cx + 23.0, cy + 4.0, 19.0 * puff),
            circle(cx + 4.0, cy + 12.0, 20.0 * puff),
            6.0,
        ),
        6.0,
    );
    // A flatter underside.
    let bottom = cy + 22.0;
    let shape = move |x: f32, y: f32| puffs(x, y).max(y - bottom - 2.0 * ((x - cx) * 0.2).sin());

    let base = rgb(246, 249, 255);
    let ink = rgb(108, 128, 172);
    let mut c = Canvas::scaled(SIZE, scale);
    // Rain: three drops falling, each at its own time.
    for (k, dx) in [-18.0_f32, 2.0, 20.0].iter().enumerate() {
        let fall = (t + k as f32 * 0.37).fract();
        let y = bottom + 6.0 + fall * 28.0;
        let alpha = 0.85 * (1.0 - fall);
        c.solid(
            capsule(cx + dx, y, cx + dx, y + 6.0, 2.2),
            rgb(120, 170, 240),
            alpha,
        );
    }
    c.solid(paint::grow(&shape, 2.0), ink, 1.0);
    c.fill(&shape, |x, y| {
        (
            paint::lit(base, x, y, cx - 10.0, cy - 6.0, 46.0, 30.0),
            0.98,
        )
    });
    let open = if f == 4 { 0.0 } else { 1.0 };
    paint::eyes(
        &mut c,
        cx + 2.0,
        cy + 4.0,
        22.0,
        4.0,
        open,
        rgb(60, 70, 110),
    );
    paint::smile(&mut c, cx + 2.0, cy + 14.0, 6.0, rgb(60, 70, 110));
    paint::blush(&mut c, cx + 2.0, cy + 11.0, 34.0, 4.5);
    c.into_image()
}
