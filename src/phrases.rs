//! Things a character says (1.5): a list of lines of its own
//! (`config::Talk`), one said in a bubble when it is poked, and — if the
//! user picks a time — one about that often while the scene runs.
//!
//! Lines are text, not code: a shared scene or a pasted character can
//! carry them, tidied like any config (`tidy`).

use std::time::{Duration, Instant};

/// At most this many lines per character.
pub const MAX_PHRASES: usize = 50;

/// How long a line stays up, in seconds.
pub const SAY_SECONDS: f32 = 4.0;

/// How often a character may speak up on its own, in minutes; 0 is only
/// when poked.
pub const EVERY_CHOICES: [u32; 5] = [0, 1, 5, 15, 60];

/// Lines as a person would have them: trimmed, without control
/// characters, no empty ones, at most [`MAX_PHRASES`] of at most
/// `speech::MAX_CHARS` each — and a time from [`EVERY_CHOICES`]'s range.
pub fn tidy(talk: &mut crate::config::Talk) {
    let lines = std::mem::take(&mut talk.phrases);
    talk.phrases = lines
        .into_iter()
        .map(|l| {
            l.chars()
                .filter(|c| !c.is_control())
                .take(crate::speech::MAX_CHARS)
                .collect::<String>()
                .trim()
                .to_string()
        })
        .filter(|l| !l.is_empty())
        .take(MAX_PHRASES)
        .collect();
    talk.every_minutes = talk.every_minutes.min(24 * 60);
    talk.sound = crate::sounds::tidy(&talk.sound);
    // A colour that does not read, or a size out of bounds, is as it comes.
    talk.bubble = talk
        .bubble
        .take()
        .filter(|c| crate::speech::parse_colour(c).is_some());
    talk.text_size = talk
        .text_size
        .filter(|s| s.is_finite())
        .map(|s| s.clamp(crate::speech::TEXT_SIZE_MIN, crate::speech::TEXT_SIZE_MAX));
    // Full is as it comes, and not written down.
    talk.volume = talk
        .volume
        .filter(|v| v.is_finite())
        .map(|v| v.clamp(0.0, 1.0))
        .filter(|v| *v < 1.0);
}

/// Which line to say: any of `count` but `last`, the one said before,
/// picked by `roll` in `0.0..1.0`. `None` with no lines.
pub fn pick(count: usize, last: Option<usize>, roll: f32) -> Option<usize> {
    match count {
        0 => None,
        1 => Some(0),
        _ => {
            let choices = count - usize::from(last.is_some_and(|l| l < count));
            let n = ((roll.clamp(0.0, 0.999_999) * choices as f32) as usize).min(choices - 1);
            // Skip over the last one said.
            Some(match last {
                Some(l) if n >= l => n + 1,
                _ => n,
            })
        }
    }
}

/// When to speak up next after `now`: about every `minutes` — a third
/// either way by `roll` in `0.0..1.0`, so several characters set to the
/// same time do not all talk at once. `None` for never.
pub fn next_after(now: Instant, minutes: u32, roll: f32) -> Option<Instant> {
    if minutes == 0 {
        return None;
    }
    let factor = 2.0 / 3.0 + roll.clamp(0.0, 1.0) * 2.0 / 3.0;
    now.checked_add(Duration::from_secs_f32(minutes as f32 * 60.0 * factor))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Talk;

    #[test]
    fn lines_are_tidied() {
        let mut talk = Talk {
            phrases: vec![
                "  Hi there!  ".into(),
                "".into(),
                "   ".into(),
                "line\u{7}bell".into(),
                "x".repeat(500),
            ],
            every_minutes: 99_999,
            sound: " cats/meow.ogg\n".into(),
            bubble: Some("not a colour".into()),
            text_size: Some(99.0),
            volume: Some(7.0),
        };
        tidy(&mut talk);
        assert_eq!(talk.volume, None, "past full is full, left out");
        assert_eq!(talk.bubble, None, "a colour that does not read");
        assert_eq!(talk.text_size, Some(crate::speech::TEXT_SIZE_MAX));
        assert_eq!(talk.sound, "cats/meow.ogg");
        assert_eq!(talk.phrases[0], "Hi there!");
        assert_eq!(talk.phrases[1], "linebell");
        assert_eq!(talk.phrases[2].chars().count(), crate::speech::MAX_CHARS);
        assert_eq!(talk.phrases.len(), 3);
        assert_eq!(talk.every_minutes, 1440);
        let mut many = Talk {
            phrases: (0..80).map(|i| i.to_string()).collect(),
            ..Default::default()
        };
        tidy(&mut many);
        assert_eq!(many.phrases.len(), MAX_PHRASES);
        for (given, kept) in [
            (Some(0.4), Some(0.4)),
            (Some(-1.0), Some(0.0)),
            (Some(f32::NAN), None),
        ] {
            let mut t = Talk {
                volume: given,
                ..Default::default()
            };
            tidy(&mut t);
            assert_eq!(t.volume, kept, "{given:?}");
        }
    }

    #[test]
    fn the_same_line_is_never_said_twice_in_a_row() {
        assert_eq!(pick(0, None, 0.5), None);
        assert_eq!(pick(1, Some(0), 0.5), Some(0), "one line: that one");
        for last in 0..3 {
            for i in 0..20 {
                let n = pick(3, Some(last), i as f32 / 20.0).unwrap();
                assert!(n < 3 && n != last, "{n} after {last}");
            }
        }
        // Every line comes up.
        let seen: std::collections::BTreeSet<_> = (0..20)
            .map(|i| pick(4, None, i as f32 / 20.0).unwrap())
            .collect();
        assert_eq!(seen.len(), 4);
    }

    #[test]
    fn speaking_up_comes_around_about_as_often_as_asked() {
        let now = Instant::now();
        assert_eq!(next_after(now, 0, 0.5), None);
        let soonest = next_after(now, 15, 0.0).unwrap() - now;
        let latest = next_after(now, 15, 1.0).unwrap() - now;
        assert_eq!(soonest.as_secs(), 600);
        assert_eq!(latest.as_secs(), 1200);
    }
}
