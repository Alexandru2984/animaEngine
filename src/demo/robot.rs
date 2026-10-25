//! The robot: a small round-cornered robot, its antenna light pulsing,
//! arms swinging and head tilting; its screen eyes blink once a loop.

use super::paint::{self, capsule, circle, rgb, round_rect, Canvas};
use super::{FRAMES, SIZE};
use std::path::Path;

const NAME: &str = "robot";

pub fn generate(base: &Path) {
    let Some(dir) = super::prepare(base, NAME, SIZE, FRAMES) else {
        return;
    };
    for f in 0..FRAMES {
        super::save_frame(&dir, f + 1, &frame(f, 1));
    }
    tracing::info!("Generated {FRAMES} robot demo frames ({SIZE}×{SIZE})");
}

pub(super) fn frame(f: u32, scale: u32) -> image::RgbaImage {
    let phase = f as f32 / FRAMES as f32 * std::f32::consts::TAU;
    let swing = 8.0 * phase.sin();
    let tilt = 2.5 * (phase + 0.6).sin();
    let cx = 64.0;
    let floor = 112.0;
    let body_y = 88.0;
    let head_y = 52.0;
    let head = round_rect(cx + tilt, head_y, 24.0, 17.0, 8.0);
    let neck = round_rect(cx, head_y + 20.0, 6.0, 5.0, 2.0);
    let body = round_rect(cx, body_y, 20.0, 16.0, 6.0);
    let arms = paint::union(
        capsule(
            cx - 21.0,
            body_y - 8.0,
            cx - 30.0 - swing * 0.3,
            body_y + 8.0 + swing * 0.4,
            4.0,
        ),
        capsule(
            cx + 21.0,
            body_y - 8.0,
            cx + 30.0 - swing * 0.3,
            body_y + 8.0 - swing * 0.4,
            4.0,
        ),
    );
    let legs = paint::union(
        round_rect(cx - 10.0, body_y + 21.0, 6.0, 6.0, 2.5),
        round_rect(cx + 10.0, body_y + 21.0, 6.0, 6.0, 2.5),
    );
    let antenna = capsule(
        cx + tilt,
        head_y - 17.0,
        cx + tilt * 1.4,
        head_y - 30.0,
        1.6,
    );
    let bulb = circle(cx + tilt * 1.5, head_y - 32.0, 4.5);
    let shape = paint::union(
        paint::union(paint::union(head, neck), paint::union(body, arms)),
        paint::union(legs, paint::union(antenna, bulb)),
    );

    let base = rgb(150, 178, 212);
    let ink = rgb(44, 58, 90);
    let mut c = Canvas::scaled(SIZE, scale);
    c.ground_shadow(cx, floor + 3.0, 30.0, 4.5, 0.28);
    c.solid(paint::grow(&shape, 2.0), ink, 1.0);
    c.fill(&shape, |x, y| {
        (paint::lit(base, x, y, cx - 10.0, 64.0, 34.0, 52.0), 1.0)
    });
    // The antenna's light, pulsing.
    let glow = 0.5 + 0.5 * (phase * 2.0).sin();
    c.solid(
        circle(cx + tilt * 1.5, head_y - 32.0, 7.0 + 2.0 * glow),
        rgb(255, 120, 110),
        0.25 * glow,
    );
    c.solid(
        circle(cx + tilt * 1.5, head_y - 32.0, 3.6),
        paint::mix(rgb(200, 60, 60), rgb(255, 150, 140), glow),
        1.0,
    );
    // The face: a dark screen, two glowing eyes, a smile.
    c.solid(
        round_rect(cx + tilt, head_y + 1.0, 17.0, 10.0, 5.0),
        rgb(30, 40, 62),
        1.0,
    );
    let eye = rgb(120, 236, 255);
    for side in [-1.0_f32, 1.0] {
        let ex = cx + tilt + side * 8.0;
        if f == 5 {
            c.solid(capsule(ex - 3.5, head_y, ex + 3.5, head_y, 1.1), eye, 1.0);
        } else {
            c.solid(round_rect(ex, head_y - 1.0, 3.2, 4.2, 1.6), eye, 1.0);
        }
    }
    paint::smile(&mut c, cx + tilt, head_y + 6.0, 6.0, eye);
    // Lights on the chest.
    for (k, color) in [rgb(255, 210, 80), rgb(120, 230, 140), rgb(255, 120, 110)]
        .into_iter()
        .enumerate()
    {
        let on = ((f as usize + k * 4) % FRAMES as usize) < 8;
        c.solid(
            circle(cx - 8.0 + k as f32 * 8.0, body_y - 3.0, 2.6),
            color,
            if on { 1.0 } else { 0.35 },
        );
    }
    c.solid(
        round_rect(cx, body_y + 7.0, 10.0, 2.0, 1.0),
        paint::shade(base, 0.7),
        1.0,
    );
    c.into_image()
}
