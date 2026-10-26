//! The bunny: a white rabbit hopping on the spot, ears flopping as it
//! lands; it blinks once a loop.

use super::paint::{self, blend, capsule, circle, ellipse, rgb, Canvas};
use super::{Look, SIZE};
use crate::animation::StateId;

pub(super) fn frame(state: StateId, f: u32, scale: u32) -> image::RgbaImage {
    let mut look = Look::of(state, f);
    let phase = look.phase;
    // Two hops a loop: up in an arc, squashed a little on landing — on
    // the spot as it stands, higher as it goes, where its own hop is the
    // step (the shared motion only leans it forward).
    let hopping = matches!(state, StateId::Idle | StateId::Walk);
    let hop = if hopping {
        (phase * 2.0).sin().max(0.0)
    } else {
        0.0
    };
    let land = if hopping {
        (-(phase * 2.0).sin()).max(0.0)
    } else {
        0.0
    };
    let lift = 8.0 * hop;
    if state == StateId::Walk {
        look.step = 1.0;
        look.lift = 0.0;
    }
    let squash = 1.0 - 0.08 * land;
    let cx = 64.0;
    let floor = 110.0;
    let body_y = floor - 22.0 * squash - lift;
    let head_y = body_y - 30.0 * squash;
    // The ears trail the hop: they lag, and droop on landing; straight up
    // in a fall, drooping when held.
    let flop = match state {
        StateId::Fall => -3.0,
        StateId::Drag => 7.0 + 2.0 * phase.sin(),
        StateId::Sleep => 9.0,
        _ => 6.0 * land - 4.0 * hop,
    };
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
    // Feet under it; apart when it falls, hanging when it is held.
    let (feet_dx, feet_dy) = match state {
        StateId::Fall => (17.0, 18.0),
        StateId::Drag => (11.0, 22.0),
        _ => (13.0, 18.0 * squash),
    };
    let feet = paint::union(
        ellipse(cx - feet_dx, body_y + feet_dy, 9.0, 5.0),
        ellipse(cx + feet_dx, body_y + feet_dy, 9.0, 5.0),
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
    if look.grounded {
        c.ground_shadow(cx, floor + 6.0, 26.0 - lift * 0.8, 4.5, 0.26);
    }
    c.set_motion(look.motion(floor, head_y - 30.0));
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
    let fx = cx + look.face_dx;
    let open = if f == 7 { 0.0 } else { 1.0 };
    let face = rgb(50, 30, 50);
    paint::eyes_as(&mut c, look.eyes, open, fx, head_y - 1.0, 18.0, 4.0, face);
    c.solid(ellipse(fx, head_y + 6.0, 2.6, 2.0), rgb(240, 120, 150), 1.0);
    paint::mouth_as(&mut c, look.mouth, fx, head_y + 11.0, 5.0, ink, |c| {
        paint::smile(c, fx - 2.4, head_y + 10.0, 3.0, ink);
        paint::smile(c, fx + 2.4, head_y + 10.0, 3.0, ink);
    });
    paint::blush(&mut c, fx, head_y + 6.0, 28.0, 4.0);
    c.into_image()
}
