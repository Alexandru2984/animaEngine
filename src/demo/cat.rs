//! The cat: an orange tabby sitting up, breathing, swishing its tail and
//! twitching an ear; it blinks once a loop.

use super::paint::{self, blend, capsule, circle, ellipse, rgb, triangle, Canvas};
use super::{Look, SIZE};
use crate::animation::StateId;

pub(super) fn frame(state: StateId, f: u32, scale: u32) -> image::RgbaImage {
    let look = Look::of(state, f);
    let phase = look.phase;
    let breath = 1.0 + 0.025 * phase.sin();
    let sway = phase.sin();
    let cx = 60.0;
    let (head_x, head_y) = (cx, 48.0);
    let twitch = if state == StateId::Idle && (f == 4 || f == 5) {
        4.0
    } else {
        0.0
    };

    let body = ellipse(cx, 86.0, 25.0 * breath, 26.0 * breath);
    let head = circle(head_x, head_y, 22.0);
    let ears = paint::union(
        triangle(
            [
                (head_x - 20.0, head_y - 6.0),
                (head_x - 16.0, head_y - 30.0),
                (head_x - 3.0, head_y - 18.0),
            ],
            2.0,
        ),
        triangle(
            [
                (head_x + 20.0, head_y - 6.0),
                (head_x + 16.0 + twitch, head_y - 30.0),
                (head_x + 3.0, head_y - 18.0),
            ],
            2.0,
        ),
    );
    // Paws: on the ground; one up, then the other, walking; spread,
    // falling; hanging, held by the scruff.
    let (left_paw, right_paw) = match state {
        StateId::Idle => ((cx - 10.0, 108.0), (cx + 10.0, 108.0)),
        StateId::Walk => (
            (cx - 10.0, 108.0 - 4.0 * (-phase.sin()).max(0.0)),
            (cx + 10.0, 108.0 - 4.0 * phase.sin().max(0.0)),
        ),
        StateId::Fall => ((cx - 16.0, 106.0), (cx + 16.0, 106.0)),
        StateId::Drag => ((cx - 9.0, 112.0), (cx + 9.0, 112.0)),
    };
    let paws = paint::union(
        ellipse(left_paw.0, left_paw.1, 8.0, 6.0),
        ellipse(right_paw.0, right_paw.1, 8.0, 6.0),
    );
    // The tail: from behind the body, round and up, swishing — up and
    // puffed when it falls, low and curled when it is held.
    let tail_pts = match state {
        StateId::Fall => [
            (cx + 18.0, 100.0),
            (cx + 32.0, 92.0),
            (cx + 38.0 + 2.0 * sway, 76.0),
            (cx + 39.0 + 3.0 * sway, 60.0),
            (cx + 37.0 + 4.0 * sway, 46.0),
        ],
        StateId::Drag => [
            (cx + 18.0, 100.0),
            (cx + 28.0, 104.0),
            (cx + 34.0 + 2.0 * sway, 101.0),
            (cx + 37.0 + 3.0 * sway, 94.0),
            (cx + 38.0 + 4.0 * sway, 86.0),
        ],
        _ => [
            (cx + 18.0, 104.0),
            (cx + 34.0, 104.0),
            (cx + 44.0 + 3.0 * sway, 92.0),
            (cx + 46.0 + 7.0 * sway, 76.0),
            (cx + 42.0 + 11.0 * sway, 62.0),
        ],
    };
    let puff = if state == StateId::Fall { 2.0 } else { 0.0 };
    let tail = move |x: f32, y: f32| {
        (0..tail_pts.len() - 1)
            .map(|i| {
                let (a, b) = (tail_pts[i], tail_pts[i + 1]);
                capsule(a.0, a.1, b.0, b.1, 6.0 - i as f32 * 0.6 + puff)(x, y)
            })
            .fold(f32::MAX, f32::min)
    };
    let torso = blend(body, head, 6.0);
    let shape = paint::union(paint::union(torso, ears), paint::union(paws, tail));

    let base = rgb(245, 150, 62);
    let ink = rgb(110, 54, 18);
    let stripe = rgb(200, 100, 34);
    let cream = rgb(255, 238, 214);
    let mut c = Canvas::scaled(SIZE, scale);
    if look.grounded {
        c.ground_shadow(cx + 8.0, 116.0, 38.0 - look.lift, 5.0, 0.28);
    }
    c.set_motion(look.motion(114.0, 18.0));
    c.solid(paint::grow(&shape, 2.0), ink, 1.0);
    c.fill(&shape, |x, y| {
        (paint::lit(base, x, y, cx - 8.0, 64.0, 40.0, 56.0), 1.0)
    });
    // Inner ears.
    c.solid(
        triangle(
            [
                (head_x - 17.0, head_y - 9.0),
                (head_x - 15.0, head_y - 24.0),
                (head_x - 7.0, head_y - 17.0),
            ],
            1.0,
        ),
        rgb(255, 170, 170),
        0.9,
    );
    c.solid(
        triangle(
            [
                (head_x + 17.0, head_y - 9.0),
                (head_x + 15.0 + twitch * 0.8, head_y - 24.0),
                (head_x + 7.0, head_y - 17.0),
            ],
            1.0,
        ),
        rgb(255, 170, 170),
        0.9,
    );
    // Tabby stripes: on the forehead, the body's sides, the tail.
    for dx in [-6.0_f32, 0.0, 6.0] {
        c.solid(
            capsule(
                head_x + dx,
                head_y - 19.0,
                head_x + dx * 0.8,
                head_y - 11.0,
                1.6,
            ),
            stripe,
            0.85,
        );
    }
    for (y0, side) in [(76.0_f32, -1.0_f32), (86.0, -1.0), (76.0, 1.0), (86.0, 1.0)] {
        let x0 = cx + side * 24.0 * breath;
        c.solid(capsule(x0, y0, x0 - side * 7.0, y0 + 3.0, 1.8), stripe, 0.8);
    }
    for &(x, y) in &tail_pts[1..4] {
        c.solid(
            capsule(x - 5.0, y + 1.0, x + 5.0, y - 1.0, 1.6),
            stripe,
            0.8,
        );
    }
    // Belly and muzzle; the face turned toward where it goes.
    let fx = head_x + look.face_dx;
    c.solid(ellipse(cx, 92.0, 13.0 * breath, 15.0), cream, 0.9);
    c.solid(ellipse(fx, head_y + 9.0, 11.0, 7.5), cream, 0.95);
    // Eyes: green, slit pupils — round and wide when it falls.
    let ey = head_y - 1.0;
    match look.eyes {
        paint::Eyes::Happy => paint::happy_eyes(&mut c, fx, ey, 17.0, 4.2, ink),
        eyes => {
            for side in [-1.0_f32, 1.0] {
                let ex = fx + side * 8.5;
                if eyes == paint::Eyes::Own && f == 8 {
                    c.solid(capsule(ex - 4.5, ey, ex + 4.5, ey, 1.1), ink, 1.0);
                } else if eyes == paint::Eyes::Wide {
                    c.solid(circle(ex, ey, 5.8), rgb(110, 196, 80), 1.0);
                    c.solid(circle(ex, ey, 3.6), rgb(20, 30, 10), 1.0);
                    c.solid(circle(ex - 1.4, ey - 1.6, 1.3), [1.0; 3], 0.9);
                } else {
                    c.solid(ellipse(ex, ey, 4.6, 5.4), rgb(110, 196, 80), 1.0);
                    c.solid(
                        capsule(ex, ey - 3.6, ex, ey + 3.6, 1.2),
                        rgb(20, 30, 10),
                        1.0,
                    );
                    c.solid(circle(ex - 1.6, ey - 2.0, 1.3), [1.0; 3], 0.9);
                }
            }
        }
    }
    // Nose, mouth, whiskers.
    c.solid(
        triangle(
            [
                (fx - 3.0, head_y + 5.0),
                (fx + 3.0, head_y + 5.0),
                (fx, head_y + 8.0),
            ],
            0.5,
        ),
        rgb(230, 120, 140),
        1.0,
    );
    paint::mouth_as(&mut c, look.mouth, fx, head_y + 12.0, 5.0, ink, |c| {
        paint::smile(c, fx - 2.5, head_y + 11.0, 3.0, ink);
        paint::smile(c, fx + 2.5, head_y + 11.0, 3.0, ink);
    });
    for (dy, out) in [(8.0_f32, -2.0_f32), (11.0, 1.0)] {
        for side in [-1.0_f32, 1.0] {
            let x0 = fx + side * 9.0;
            c.solid(
                capsule(x0, head_y + dy, x0 + side * 15.0, head_y + dy + out, 0.45),
                ink,
                0.7,
            );
        }
    }
    paint::blush(&mut c, fx, head_y + 7.0, 30.0, 4.0);
    c.into_image()
}
