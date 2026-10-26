//! The mushroom: a red-capped toadstool with white spots, swaying on its
//! stem and bouncing its cap; it blinks once a loop.

use super::paint::{self, blend, ellipse, rgb, round_rect, Canvas};
use super::{Look, SIZE};
use crate::animation::StateId;

pub(super) fn frame(state: StateId, f: u32, scale: u32) -> image::RgbaImage {
    let look = Look::of(state, f);
    let phase = look.phase;
    // Its cap sways as it stands, and leans back as it hurries.
    let sway = match state {
        StateId::Walk => -2.0 + 2.0 * phase.sin(),
        _ => 3.0 * phase.sin(),
    };
    let bounce = 1.0 + 0.05 * (phase * 2.0).sin();
    let cx = 64.0;
    let floor = 112.0;
    let stem = round_rect(cx, 92.0, 17.0, 20.0, 9.0);
    let cap_y = 66.0;
    let cap_cx = cx + sway;
    let dome = ellipse(cap_cx, cap_y, 42.0 * bounce, 32.0 / bounce);
    let cap = move |x: f32, y: f32| dome(x, y).max(y - (cap_y + 6.0));
    let rim = ellipse(cap_cx, cap_y + 5.0, 40.0 * bounce, 6.0);
    let cap_shape = blend(cap, rim, 4.0);

    let red = rgb(226, 62, 62);
    let cream = rgb(250, 236, 212);
    let ink = rgb(110, 32, 30);
    let mut c = Canvas::scaled(SIZE, scale);
    if look.grounded {
        c.ground_shadow(cx, floor + 2.0, 28.0 - look.lift, 4.5, 0.28);
    }
    c.set_motion(look.motion(floor, cap_y - 32.0));
    // The stem under the cap, outlined, then the cap over it.
    c.solid(paint::grow(&stem, 2.0), ink, 1.0);
    c.fill(&stem, |x, y| {
        (paint::lit(cream, x, y, cx - 6.0, 88.0, 20.0, 24.0), 1.0)
    });
    c.solid(paint::grow(&cap_shape, 2.0), ink, 1.0);
    c.fill(&cap_shape, |x, y| {
        (
            paint::lit(red, x, y, cap_cx - 12.0, cap_y - 10.0, 46.0, 34.0),
            1.0,
        )
    });
    for (sx, sy, r) in [
        (-22.0_f32, -8.0_f32, 6.0_f32),
        (2.0, -20.0, 7.0),
        (24.0, -6.0, 5.5),
        (-6.0, -2.0, 3.5),
        (14.0, -24.0, 3.0),
    ] {
        c.solid(
            ellipse(cap_cx + sx * bounce, cap_y + sy / bounce, r, r * 0.85),
            rgb(255, 250, 244),
            0.95,
        );
    }
    let face_y = 92.0;
    let fx = cx + look.face_dx;
    let open = if f == 2 { 0.0 } else { 1.0 };
    let face = rgb(70, 40, 30);
    paint::eyes_as(&mut c, look.eyes, open, fx, face_y, 14.0, 3.6, face);
    paint::mouth_as(&mut c, look.mouth, fx, face_y + 9.0, 5.0, face, |c| {
        paint::smile(c, fx, face_y + 9.0, 5.0, face);
    });
    paint::blush(&mut c, fx, face_y + 6.0, 24.0, 3.5);
    c.into_image()
}
