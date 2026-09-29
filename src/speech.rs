//! Speech bubbles (1.5): a character saying something for a few seconds —
//! from its behavior script (`say("…")`) or from a reminder
//! (`crate::reminders`).
//!
//! The bubble is drawn by egui (`ui::speech`), over the character and
//! click-through like the rest of the overlay: by the panel's egui pass,
//! which runs in both modes, on the primary surface, and by an egui of
//! their own on every other monitor (`ui::surface_egui`). Until those
//! existed, a character on another monitor said nothing visible.

use std::time::{Duration, Instant};

/// The longest text a bubble holds, in characters.
pub const MAX_CHARS: usize = 140;
/// How long a bubble stays by default.
pub const DEFAULT_SECONDS: f32 = 4.0;
/// The bounds on how long one may stay.
pub const MIN_SECONDS: f32 = 1.0;
pub const MAX_SECONDS: f32 = 30.0;

/// What a character is saying, and until when.
#[derive(Debug, Clone, PartialEq)]
pub struct Speech {
    pub text: String,
    pub until: Instant,
}

impl Speech {
    /// `text` for `seconds` from `now`: trimmed, cut to [`MAX_CHARS`] (an
    /// ellipsis marking the cut), the time kept within its bounds. `None`
    /// for nothing to say.
    pub fn new(text: &str, seconds: f32, now: Instant) -> Option<Self> {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        let text = if text.chars().count() > MAX_CHARS {
            let cut: String = text.chars().take(MAX_CHARS - 1).collect();
            format!("{}…", cut.trim_end())
        } else {
            text.to_string()
        };
        let seconds = if seconds.is_finite() {
            seconds.clamp(MIN_SECONDS, MAX_SECONDS)
        } else {
            DEFAULT_SECONDS
        };
        Some(Self {
            text,
            until: now + Duration::from_secs_f32(seconds),
        })
    }

    /// Whether it is still being said at `now`.
    pub fn showing(&self, now: Instant) -> bool {
        now < self.until
    }
}

/// A bubble to draw this frame: what is said, where the character is on
/// the surface (left, top, right, bottom, relative to its origin), and
/// until when.
#[derive(Debug, Clone, PartialEq)]
pub struct Shown {
    pub text: String,
    pub character: (f32, f32, f32, f32),
    pub until: Instant,
}

/// The bubbles of every visible character `on_surface` accepts, placed
/// relative to that surface's `origin`. Gathered before the UI pass, which
/// has the scene lent to the panels.
pub fn shown(
    scene: &crate::scene::Scene,
    origin: (f32, f32),
    on_surface: impl Fn(&crate::entity::Entity) -> bool,
) -> Vec<Shown> {
    let now = Instant::now();
    (0..scene.entities.len())
        .filter(|&i| scene.effective_visible(i))
        .filter_map(|i| {
            let e = &scene.entities[i];
            let speech = e.speech.as_ref().filter(|s| s.showing(now))?;
            if !on_surface(e) {
                return None;
            }
            let (l, t, r, b) = crate::input::arrange::drawn_rect(scene, i)?;
            Some(Shown {
                text: speech.text.clone(),
                character: (l - origin.0, t - origin.1, r - origin.0, b - origin.1),
                until: speech.until,
            })
        })
        .collect()
}

/// Where a bubble of `size` goes for a character drawn at `character`
/// (left, top, right, bottom), inside `area` (the same four): centred
/// above it with room for the tail, below it when there is no room above,
/// and moved sideways to stay in the area. Returns its top-left, and
/// whether it hangs below the character.
pub fn place(
    character: (f32, f32, f32, f32),
    size: (f32, f32),
    area: (f32, f32, f32, f32),
    gap: f32,
) -> ((f32, f32), bool) {
    let (l, t, r, b) = character;
    let (w, h) = size;
    let centre = (l + r) / 2.0;
    let x = (centre - w / 2.0).clamp(area.0, (area.2 - w).max(area.0));
    let above = t - gap - h;
    if above >= area.1 {
        ((x, above), false)
    } else {
        ((x, (b + gap).min(area.3 - h)), true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_to_say_is_no_bubble() {
        let now = Instant::now();
        assert!(Speech::new("   ", 4.0, now).is_none());
        let s = Speech::new("  hi  ", 4.0, now).unwrap();
        assert_eq!(s.text, "hi");
        assert!(s.showing(now + Duration::from_secs(3)));
        assert!(!s.showing(now + Duration::from_secs(4)));
    }

    #[test]
    fn long_text_and_odd_times_are_held_to_their_bounds() {
        let now = Instant::now();
        let long = "ă".repeat(MAX_CHARS + 20);
        let s = Speech::new(&long, 999.0, now).unwrap();
        assert_eq!(s.text.chars().count(), MAX_CHARS);
        assert!(s.text.ends_with('…'));
        assert_eq!(s.until, now + Duration::from_secs_f32(MAX_SECONDS));
        let s = Speech::new("x", f32::NAN, now).unwrap();
        assert_eq!(s.until, now + Duration::from_secs_f32(DEFAULT_SECONDS));
        let s = Speech::new("x", 0.0, now).unwrap();
        assert_eq!(s.until, now + Duration::from_secs_f32(MIN_SECONDS));
    }

    #[test]
    fn a_bubble_sits_above_unless_there_is_no_room() {
        let area = (0.0, 0.0, 1000.0, 800.0);
        // Room above: centred over the character, tail's gap kept.
        let ((x, y), below) = place((400.0, 300.0, 500.0, 400.0), (120.0, 40.0), area, 10.0);
        assert_eq!((x, y, below), (390.0, 250.0, false));
        // At the top edge: below instead.
        let ((_, y), below) = place((400.0, 10.0, 500.0, 110.0), (120.0, 40.0), area, 10.0);
        assert_eq!((y, below), (120.0, true));
        // At the left edge: moved in, not cut off.
        let ((x, _), _) = place((0.0, 300.0, 40.0, 400.0), (120.0, 40.0), area, 10.0);
        assert_eq!(x, 0.0);
        // At the right edge too.
        let ((x, _), _) = place((960.0, 300.0, 1000.0, 400.0), (120.0, 40.0), area, 10.0);
        assert_eq!(x, 880.0);
    }
}
