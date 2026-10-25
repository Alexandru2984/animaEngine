//! The bunny: a white rabbit hopping on the spot, ears flopping as it
//! lands; it blinks once a loop.

use super::paint::{self, blend, capsule, circle, ellipse, rgb, Canvas};
use super::{FRAMES, SIZE};
use std::path::Path;

const NAME: &str = "bunny";

pub fn generate(base: &Path) {
    let Some(dir) = super::prepare(base, NAME, SIZE, FRAMES) else {
        return;
    };
    for f in 0..FRAMES {
        super::save_frame(&dir, f + 1, &frame(f, 1));
    }
    tracing::info!("Generated {FRAMES} bunny demo frames ({SIZE}×{SIZE})");
}

pub(super) fn frame(f: u32, scale: u32) -> image::RgbaImage {
    let phase = f as f32 / FRAMES as f32 * std::f32::consts::TAU;
    // Two hops a loop: up in an arc, squashed a little on landing.
    let hop = (phase * 2.0).sin().max(0.0);
    let land = (-(phase * 2.0).sin()).max(0.0);
    let lift = 8.0 * hop;
    let squash = 1.0 - 0.08 * land;
    let cx = 64.0;
    let floor = 110.0;
    let body_y = floor - 22.0 * squash - lift;
    let head_y = body_y - 30.0 * squash;
    // The ears trail the hop: they lag, and droop on landing.
    let flop = 6.0 * land - 4.0 * hop;
    let ears = paint::union(
        capsule(
            cx - 9.0,
            head_y - 12.0,
            cx - 13.0 - flop * 0.4,
            head_y - 35.0 + flop,
            6.0,
        ),
        capsule(
            cx + 9.0,
            head_y - 12.0,
            cx + 15.0 + flop,
            head_y - 33.0 + flop * 1.4,
            6.0,
        ),
    );
    let body = ellipse(cx, body_y, 24.0 / squash.sqrt(), 22.0 * squash);
    let head = circle(cx, head_y, 19.0);
    let feet = paint::union(
        ellipse(cx - 13.0, body_y + 18.0 * squash, 9.0, 5.0),
        ellipse(cx + 13.0, body_y + 18.0 * squash, 9.0, 5.0),
    );
    let tail = circle(cx + 23.0, body_y + 6.0, 7.0);
    let shape = paint::union(
        paint::union(blend(body, head, 6.0), ears),
        paint::union(feet, tail),
    );

    let base = rgb(250, 246, 242);
    let ink = rgb(120, 96, 110);
    let pink = rgb(255, 170, 190);
    let mut c = Canvas::scaled(SIZE, scale);
    c.ground_shadow(cx, floor + 6.0, 26.0 - lift * 0.8, 4.5, 0.26);
    c.solid(paint::grow(&shape, 2.0), ink, 1.0);
    c.fill(&shape, |x, y| {
        (
            paint::lit(base, x, y, cx - 8.0, body_y - 14.0, 34.0, 48.0),
            1.0,
        )
    });
    c.solid(
        capsule(
            cx - 9.5,
            head_y - 16.0,
            cx - 12.6 - flop * 0.4,
            head_y - 32.0 + flop,
            2.6,
        ),
        pink,
        0.9,
    );
    c.solid(
        capsule(
            cx + 9.5,
            head_y - 16.0,
            cx + 14.2 + flop,
            head_y - 30.0 + flop * 1.4,
            2.6,
        ),
        pink,
        0.9,
    );
    let open = if f == 7 { 0.0 } else { 1.0 };
    paint::eyes(&mut c, cx, head_y - 1.0, 18.0, 4.0, open, rgb(50, 30, 50));
    c.solid(ellipse(cx, head_y + 6.0, 2.6, 2.0), rgb(240, 120, 150), 1.0);
    paint::smile(&mut c, cx - 2.4, head_y + 10.0, 3.0, ink);
    paint::smile(&mut c, cx + 2.4, head_y + 10.0, 3.0, ink);
    paint::blush(&mut c, cx, head_y + 6.0, 28.0, 4.0);
    c.into_image()
}
