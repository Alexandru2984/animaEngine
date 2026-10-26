//! The star: a golden five-pointed star rocking from side to side, with
//! sparkles twinkling around it; it blinks once a loop.

use super::paint::{self, rgb, Canvas};
use super::{Look, SIZE};
use crate::animation::StateId;

pub(super) fn frame(state: StateId, f: u32, scale: u32) -> image::RgbaImage {
    let look = Look::of(state, f);
    let phase = look.phase;
    // It rocks as it stands and as it goes; held, it spins a little.
    let rock = match state {
        StateId::Drag => 0.35 * phase.sin(),
        _ => 0.16 * phase.sin(),
    };
    let swell = 1.0 + 0.03 * (phase * 2.0).sin();
    let (cx, cy) = (64.0, 64.0);
    let body = paint::star(cx, cy, 46.0 * swell, 0.5, rock, 5.0);

    let base = rgb(255, 204, 60);
    let ink = rgb(160, 100, 16);
    let mut c = Canvas::scaled(SIZE, scale);
    if look.grounded {
        c.ground_shadow(cx, 118.0, 22.0 - look.lift, 4.0, 0.22);
    }
    c.set_motion(look.motion(108.0, 18.0));
    c.solid(paint::grow(&body, 2.0), ink, 1.0);
    c.fill(&body, |x, y| {
        (paint::lit(base, x, y, cx - 8.0, cy - 6.0, 40.0, 40.0), 1.0)
    });
    // The face turns with the star, a little.
    let fx = cx + rock * 14.0 + look.face_dx;
    let open = if f == 3 { 0.0 } else { 1.0 };
    let face = rgb(90, 50, 8);
    paint::eyes_as(&mut c, look.eyes, open, fx, cy + 2.0, 18.0, 4.2, face);
    paint::mouth_as(&mut c, look.mouth, fx, cy + 12.0, 6.0, face, |c| {
        paint::smile(c, fx, cy + 12.0, 6.0, face);
    });
    paint::blush(&mut c, fx, cy + 9.0, 30.0, 4.0);
    c.set_motion(paint::Motion::STILL);
    for (k, (sx, sy)) in [(18.0_f32, 26.0_f32), (110.0, 30.0), (104.0, 100.0)]
        .iter()
        .enumerate()
    {
        let alpha = (0.5 + 0.5 * (phase + k as f32 * 2.1).sin()).powi(2);
        paint::sparkle(&mut c, *sx, *sy, 6.0, alpha);
    }
    c.into_image()
}
