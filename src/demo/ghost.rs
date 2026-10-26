//! The ghost: a pale floating sheet with a wavy hem, bobbing and waving
//! its little arms; it blinks once a loop.

use super::paint::{self, blend, capsule, circle, rgb, Canvas};
use super::{Look, SIZE};
use crate::animation::StateId;

pub(super) fn frame(state: StateId, f: u32, scale: u32) -> image::RgbaImage {
    let look = Look::of(state, f);
    let phase = look.phase;
    // Asleep, it sinks a little and stops bobbing.
    let (bob, sink) = if state == StateId::Sleep {
        (0.0, 6.0)
    } else {
        (3.5 * phase.sin(), 0.0)
    };
    let cx = 64.0;
    let cy = 48.0 + bob + sink;
    let hem_y = 98.0 + bob + sink;
    // The hem ripples faster on the move.
    let ripple = if state == StateId::Idle { 2.0 } else { 3.0 };
    let wave = move |x: f32| hem_y + 4.0 * ((x - cx) * 0.3 + phase * ripple).sin();
    let head = circle(cx, cy, 32.0);
    let skirt = move |x: f32, y: f32| ((x - cx).abs() - 32.0).max(cy - y).max(y - wave(x));
    let wavy = move |x: f32, y: f32| head(x, y).min(skirt(x, y));
    // Its arms wave as it stands, swing as it goes, fly up as it falls
    // and hang as it is held.
    let arm = 4.0 * (phase * 2.0).sin();
    let (left, right) = match state {
        StateId::Idle => ((cx - 40.0, cy + 10.0 + arm), (cx + 40.0, cy + 10.0 - arm)),
        StateId::Walk => ((cx - 39.0, cy + 20.0 + arm), (cx + 42.0, cy + 8.0 - arm)),
        StateId::Fall => ((cx - 40.0, cy - 6.0 + arm), (cx + 40.0, cy - 6.0 - arm)),
        StateId::Drag | StateId::Sleep => ((cx - 35.0, cy + 31.0), (cx + 35.0, cy + 31.0)),
    };
    let arms = paint::union(
        capsule(cx - 30.0, cy + 18.0, left.0, left.1, 5.0),
        capsule(cx + 30.0, cy + 18.0, right.0, right.1, 5.0),
    );
    let body = blend(wavy, arms, 4.0);

    let base = rgb(236, 238, 255);
    let ink = rgb(70, 66, 120);
    let mut c = Canvas::scaled(SIZE, scale);
    if look.grounded {
        c.ground_shadow(cx, 117.0, 24.0 - bob - look.lift, 4.5, 0.28);
    }
    c.set_motion(look.motion(110.0, 16.0));
    c.solid(paint::grow(&body, 2.0), ink, 1.0);
    c.fill(&body, |x, y| {
        let lit = paint::lit(base, x, y, cx - 8.0, cy + 8.0, 44.0, 56.0);
        (lit, 0.96)
    });
    let fx = cx + look.face_dx;
    let open = if f == 9 { 0.0 } else { 1.0 };
    let eye_ink = rgb(40, 36, 72);
    paint::eyes_as(&mut c, look.eyes, open, fx, cy + 2.0, 22.0, 5.0, eye_ink);
    let mouth = rgb(60, 40, 80);
    paint::mouth_as(&mut c, look.mouth, fx, cy + 15.0, 8.0, mouth, |c| {
        c.solid(paint::ellipse(fx, cy + 15.0, 3.2, 4.2), mouth, 1.0);
    });
    paint::blush(&mut c, fx, cy + 11.0, 36.0, 5.0);
    c.into_image()
}
