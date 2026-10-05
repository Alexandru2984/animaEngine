//! Welcome back (1.5, on by default: `welcome_back`): after you have been
//! away — as long without input as the idle watch counts (`crate::away`) —
//! the frontmost character greets you as you return: "Welcome back!", or
//! "Good morning!" when you come back in the morning after the night.
//!
//! Only a return is greeted, never the start: an app coming up at login
//! has nobody to welcome back yet. Nothing is said out of sight — hidden,
//! snoozed, stepped aside — or at bedtime, when they sleep.

use std::time::{Duration, Instant};

/// The morning, in minutes of the day: from 04:00 to noon.
const MORNING: std::ops::Range<u32> = 4 * 60..12 * 60;

/// Away this long, coming back in the morning is a new day whatever the
/// clock said when you left.
const A_NIGHT: Duration = Duration::from_secs(4 * 3600);

/// How long a greeting stays up.
pub const SAY_SECONDS: f32 = 6.0;

/// What to say on a return.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Greeting {
    WelcomeBack,
    GoodMorning,
}

impl Greeting {
    /// In the user's language.
    pub fn line(self) -> String {
        crate::i18n::t(match self {
            Greeting::WelcomeBack => "welcome-back",
            Greeting::GoodMorning => "welcome-morning",
        })
    }
}

/// Watches for the return.
#[derive(Debug, Default)]
pub struct Welcome {
    /// Since when away, and the minute of the day it was then.
    since: Option<(Instant, Option<u32>)>,
}

impl Welcome {
    /// Whether the user is `away` now, at local `minute` of the day: what
    /// to say if they just came back. The loops call it every frame.
    pub fn check(&mut self, away: bool, minute: Option<u32>) -> Option<Greeting> {
        if away {
            self.since.get_or_insert((Instant::now(), minute));
            return None;
        }
        let (at, then) = self.since.take()?;
        Some(greeting_for(at.elapsed(), then, minute))
    }
}

/// "Good morning!" for a return in the morning after the night — away for
/// hours, or since before the morning, or since the evening before;
/// "Welcome back!" otherwise.
pub fn greeting_for(away_for: Duration, then: Option<u32>, now: Option<u32>) -> Greeting {
    let Some(now) = now else {
        return Greeting::WelcomeBack;
    };
    let overnight =
        away_for >= A_NIGHT || then.is_some_and(|then| then < MORNING.start || then > now);
    if MORNING.contains(&now) && overnight {
        Greeting::GoodMorning
    } else {
        Greeting::WelcomeBack
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: Duration = Duration::from_secs(60);

    #[test]
    fn a_return_is_greeted_once() {
        let mut w = Welcome::default();
        assert_eq!(w.check(false, Some(600)), None, "never away: nothing");
        assert_eq!(w.check(true, Some(600)), None);
        assert_eq!(w.check(true, Some(610)), None);
        assert_eq!(w.check(false, Some(615)), Some(Greeting::WelcomeBack));
        assert_eq!(w.check(false, Some(616)), None, "once");
    }

    #[test]
    fn the_morning_after_the_night_is_good_morning() {
        // Left at 23:00, back at 08:00.
        assert_eq!(
            greeting_for(9 * 60 * MIN, Some(23 * 60), Some(8 * 60)),
            Greeting::GoodMorning
        );
        // Left at 02:00, back at 07:00.
        assert_eq!(
            greeting_for(5 * 60 * MIN, Some(2 * 60), Some(7 * 60)),
            Greeting::GoodMorning
        );
        // A coffee at 09:00.
        assert_eq!(
            greeting_for(15 * MIN, Some(9 * 60), Some(9 * 60 + 15)),
            Greeting::WelcomeBack
        );
        // Lunch, back at 13:00.
        assert_eq!(
            greeting_for(60 * MIN, Some(12 * 60), Some(13 * 60)),
            Greeting::WelcomeBack
        );
        // No clock: plainly back.
        assert_eq!(
            greeting_for(9 * 60 * MIN, None, None),
            Greeting::WelcomeBack
        );
    }
}
