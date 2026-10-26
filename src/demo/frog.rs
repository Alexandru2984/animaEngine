//! The frog: a green frog sitting, its throat puffing in and out, eyes
//! on top of its head; it blinks once a loop.

use super::paint::{self, blend, circle, ellipse, rgb, Canvas};
use super::{Look, SIZE};
use crate::animation::StateId;

pub(super) fn frame(state: StateId, f: u32, scale: u32) -> image::RgbaImage {
    let mut look = Look::of(state, f);
    let phase = look.phase;
    // A frog walks in hops: higher than a step.
    look.lift *= 1.6;
    let breath = 1.0 + 0.03 * phase.sin();
    let puff = if state == StateId::Idle {
        (phase * 2.0).sin().max(0.0)
    } else {
        0.0
    };
    let cx = 64.0;
    let floor = 110.0;
    let body_y = 88.0;
    let body = ellipse(cx, body_y, 36.0 * breath, 24.0);
    let eyes_y = body_y - 24.0;
    let bumps = paint::union(
        circle(cx - 17.0, eyes_y, 12.0),
        circle(cx + 17.0, eyes_y, 12.0),
    );
    // Back legs folded under it — kicked out at the top of a hop, spread
    // in a fall, dangling long when held.
    let legs = match state {
        StateId::Fall => paint::union(
            ellipse(cx - 36.0, floor - 2.0, 12.0, 7.0),
            ellipse(cx + 36.0, floor - 2.0, 12.0, 7.0),
        ),
        StateId::Drag => paint::union(
            ellipse(cx - 20.0, floor - 1.0, 7.0, 11.0),
            ellipse(cx + 20.0, floor - 1.0, 7.0, 11.0),
        ),
        _ => {
            let kick = look.step;
            paint::union(
                ellipse(
                    cx - 30.0 - 4.0 * kick,
                    floor - 6.0 + 4.0 * kick,
                    13.0 + 3.0 * kick,
                    7.0,
                ),
                ellipse(
                    cx + 30.0 + 4.0 * kick,
                    floor - 6.0 + 4.0 * kick,
                    13.0 + 3.0 * kick,
                    7.0,
                ),
            )
        }
    };
    let hands = paint::union(
        ellipse(cx - 12.0, floor - 3.0, 7.0, 4.5),
        ellipse(cx + 12.0, floor - 3.0, 7.0, 4.5),
    );
    let shape = paint::union(blend(body, bumps, 8.0), paint::union(legs, hands));

    let base = rgb(112, 200, 92);
    let ink = rgb(34, 92, 36);
    let mut c = Canvas::scaled(SIZE, scale);
    if look.grounded {
        c.ground_shadow(cx, floor + 4.0, 42.0 - look.lift, 5.0, 0.28);
    }
    c.set_motion(look.motion(floor, eyes_y - 12.0));
    c.solid(paint::grow(&shape, 2.0), ink, 1.0);
    c.fill(&shape, |x, y| {
        (
            paint::lit(base, x, y, cx - 10.0, body_y - 12.0, 46.0, 36.0),
            1.0,
        )
    });
    // A pale belly and the throat sac, puffing.
    c.solid(
        ellipse(cx, body_y + 9.0, 22.0 * breath, 12.0),
        rgb(236, 240, 170),
        0.9,
    );
    if puff > 0.05 {
        c.solid(
            ellipse(cx, body_y + 5.0, 9.0 + 7.0 * puff, 5.0 + 5.0 * puff),
            rgb(244, 236, 190),
            0.9,
        );
    }
    // Spots.
    for (sx, sy, r) in [
        (-24.0_f32, -4.0_f32, 3.0_f32),
        (26.0, -2.0, 2.5),
        (18.0, 8.0, 2.0),
    ] {
        c.solid(
            circle(cx + sx, body_y + sy, r),
            paint::shade(base, 0.72),
            0.8,
        );
    }
    // Eyes on the bumps: white, a big pupil that looks where it goes —
    // small in a fall, shut in a smile when held.
    let look_dx = look.face_dx * 0.5;
    for side in [-1.0_f32, 1.0] {
        let ex = cx + side * 17.0;
        match look.eyes {
            paint::Eyes::Own if f == 9 => {
                c.solid(
                    paint::capsule(ex - 7.0, eyes_y + 1.0, ex + 7.0, eyes_y + 1.0, 1.4),
                    ink,
                    1.0,
                );
            }
            paint::Eyes::Happy => {
                c.solid(circle(ex, eyes_y, 8.5), [1.0; 3], 1.0);
                paint::happy_eyes(&mut c, ex, eyes_y + 1.0, 0.0, 4.5, ink);
            }
            eyes => {
                let pupil = if eyes == paint::Eyes::Wide { 3.0 } else { 4.8 };
                c.solid(circle(ex, eyes_y, 8.5), [1.0; 3], 1.0);
                c.solid(
                    circle(ex + side * 1.0 + look_dx, eyes_y + 1.0, pupil),
                    rgb(24, 30, 20),
                    1.0,
                );
                c.solid(
                    circle(ex - 1.0 + look_dx, eyes_y - 2.0, 1.8),
                    [1.0; 3],
                    0.95,
                );
            }
        }
    }
    let fx = cx + look.face_dx;
    paint::mouth_as(&mut c, look.mouth, fx, body_y - 4.0, 12.0, ink, |c| {
        paint::smile(c, fx, body_y - 4.0, 18.0, ink);
    });
    paint::blush(&mut c, fx, body_y - 4.0, 46.0, 5.0);
    c.into_image()
}
