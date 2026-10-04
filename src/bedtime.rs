//! Bedtime (1.5): hours of the night when the characters doze off by the
//! clock — as they do when nobody is there (`crate::doze`) — whoever is at
//! the keyboard. Edit mode wakes them while it lasts, a full-screen film
//! keeps them to a plain pause (`away::Stillness::watching`), and the
//! night counts as a break for the reminders.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// `[global.bedtime]`: off by default, 23:00 to 07:00 once switched on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bedtime {
    #[serde(default)]
    pub enabled: bool,
    /// "HH:MM", local time.
    #[serde(default = "default_from")]
    pub from: String,
    #[serde(default = "default_to")]
    pub to: String,
}

fn default_from() -> String {
    "23:00".into()
}

fn default_to() -> String {
    "07:00".into()
}

impl Default for Bedtime {
    fn default() -> Self {
        Self {
            enabled: false,
            from: default_from(),
            to: default_to(),
        }
    }
}

impl Bedtime {
    /// Left out of the config while it is as it comes.
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Whether `minute` of the day falls in it — across midnight too.
    /// Off, or with a time that does not read, never; from 23:00 to 23:00
    /// is no time at all.
    pub fn asleep_at(&self, minute: u32) -> bool {
        if !self.enabled {
            return false;
        }
        let (Some(from), Some(to)) = (
            crate::schedule::parse_hhmm(&self.from),
            crate::schedule::parse_hhmm(&self.to),
        ) else {
            return false;
        };
        let minute = minute % (24 * 60);
        if from <= to {
            (from..to).contains(&minute)
        } else {
            minute >= from || minute < to
        }
    }
}

/// How often the clock is looked at: bedtime is set to the minute.
const LOOK_EVERY: Duration = Duration::from_secs(10);

/// Bedtime against the clock, read at most every ten seconds — the
/// loops ask every frame.
#[derive(Debug, Default)]
pub struct Watch {
    last: Option<(Instant, Bedtime, bool)>,
}

impl Watch {
    /// Whether it is bedtime now, by `bedtime`.
    pub fn now(&mut self, bedtime: &Bedtime) -> bool {
        if !bedtime.enabled {
            self.last = None;
            return false;
        }
        if let Some((at, seen, asleep)) = &self.last {
            if seen == bedtime && at.elapsed() < LOOK_EVERY {
                return *asleep;
            }
        }
        let asleep = crate::schedule::local_now().is_some_and(|t| bedtime.asleep_at(t.minute));
        self.last = Some((Instant::now(), bedtime.clone(), asleep));
        asleep
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bed(from: &str, to: &str) -> Bedtime {
        Bedtime {
            enabled: true,
            from: from.into(),
            to: to.into(),
        }
    }

    #[test]
    fn the_night_runs_across_midnight() {
        let b = bed("23:00", "07:00");
        assert!(b.asleep_at(23 * 60));
        assert!(b.asleep_at(2 * 60));
        assert!(b.asleep_at(6 * 60 + 59));
        assert!(!b.asleep_at(7 * 60));
        assert!(!b.asleep_at(12 * 60));
        assert!(!b.asleep_at(22 * 60 + 59));
    }

    #[test]
    fn a_nap_in_the_day_too() {
        let b = bed("13:00", "14:30");
        assert!(b.asleep_at(13 * 60 + 10));
        assert!(!b.asleep_at(14 * 60 + 30));
        assert!(!b.asleep_at(2 * 60));
    }

    #[test]
    fn off_or_unreadable_is_never() {
        assert!(!Bedtime::default().asleep_at(2 * 60), "off by default");
        assert!(!bed("25:00", "07:00").asleep_at(2 * 60));
        assert!(!bed("23:00", "23:00").asleep_at(23 * 60));
        assert!(!Watch::default().now(&Bedtime::default()));
    }

    #[test]
    fn as_it_comes_it_stays_out_of_the_config() {
        assert!(Bedtime::default().is_default());
        assert!(!bed("23:00", "07:00").is_default());
    }
}
