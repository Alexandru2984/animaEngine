//! The ghost: a pale floating sheet with a wavy hem, bobbing and waving
//! its little arms; it blinks once a loop.

use super::paint::{self, blend, capsule, circle, rgb, Canvas};
use super::{FRAMES, SIZE};
use std::path::Path;

const NAME: &str = "ghost";

pub fn generate(base: &Path) {
    let Some(dir) = super::prepare(base, NAME, SIZE, FRAMES) else {
        return;
    };
    for f in 0..FRAMES {
        super::save_frame(&dir, f + 1, &frame(f));
    }
    tracing::info!("Generated {FRAMES} ghost demo frames ({SIZE}×{SIZE})");
}

fn frame(f: u32) -> image::RgbaImage {
    let phase = f as f32 / FRAMES as f32 * std::f32::consts::TAU;
    let bob = 3.5 * phase.sin();
    let cx = 64.0;
    let cy = 48.0 + bob;
    let hem_y = 98.0 + bob;
    let wave = move |x: f32| hem_y + 4.0 * ((x - cx) * 0.3 + phase * 2.0).sin();
    let head = circle(cx, cy, 32.0);
    let skirt = move |x: f32, y: f32| ((x - cx).abs() - 32.0).max(cy - y).max(y - wave(x));
    let wavy = move |x: f32, y: f32| head(x, y).min(skirt(x, y));
    let arm = 4.0 * (phase * 2.0).sin();
    let arms = paint::union(
        capsule(cx - 30.0, cy + 18.0, cx - 40.0, cy + 10.0 + arm, 5.0),
        capsule(cx + 30.0, cy + 18.0, cx + 40.0, cy + 10.0 - arm, 5.0),
    );
    let body = blend(wavy, arms, 4.0);

    let base = rgb(236, 238, 255);
    let ink = rgb(70, 66, 120);
    let mut c = Canvas::new(SIZE);
    c.ground_shadow(cx, 117.0, 24.0 - bob, 4.5, 0.28);
    c.solid(paint::grow(&body, 2.0), ink, 1.0);
    c.fill(&body, |x, y| {
        let lit = paint::lit(base, x, y, cx - 8.0, cy + 8.0, 44.0, 56.0);
        (lit, 0.96)
    });
    let open = if f == 9 { 0.0 } else { 1.0 };
    paint::eyes(&mut c, cx, cy + 2.0, 22.0, 5.0, open, rgb(40, 36, 72));
    c.solid(
        paint::ellipse(cx, cy + 15.0, 3.2, 4.2),
        rgb(60, 40, 80),
        1.0,
    );
    paint::blush(&mut c, cx, cy + 11.0, 36.0, 5.0);
    c.into_image()
}
