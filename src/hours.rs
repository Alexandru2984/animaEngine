//! A character's own hours (1.5): one that comes out only at night, or
//! only in the daytime — the ghost from 22:00 to 06:00, the sun the other
//! way round. Outside them it is not there: not drawn, not tapped, saying
//! nothing (`Scene::effective_visible`). In edit mode everyone is, to be
//! edited. The clock is read every ten seconds, as for bedtime
//! (`crate::bedtime`), which keeps its hours the same way.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// `hours = { from = "22:00", to = "06:00" }` on a character: when it is
/// there, local time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hours {
    pub from: String,
    pub to: String,
}

impl Default for Hours {
    /// What the Inspector offers first: the night.
    fn default() -> Self {
        Self {
            from: "22:00".into(),
            to: "06:00".into(),
        }
    }
}

impl Hours {
    /// Whether `minute` of the day falls in them. Times that do not read,
    /// or the same time twice, are all day: a character is never lost to
    /// a typo.
    pub fn contains(&self, minute: u32) -> bool {
        if self.from.trim() == self.to.trim() {
            return true;
        }
        within(&self.from, &self.to, minute).unwrap_or(true)
    }

    /// Kept to two short times.
    pub fn sanitize(&mut self) {
        for t in [&mut self.from, &mut self.to] {
            *t = t.trim().chars().take(5).collect();
        }
    }
}

/// Whether `minute` of the day falls from `from` up to `to` ("HH:MM"),
/// across midnight when `to` comes first; `None` when either does not
/// read. From a time to itself is no time at all.
pub fn within(from: &str, to: &str, minute: u32) -> Option<bool> {
    let from = crate::schedule::parse_hhmm(from)?;
    let to = crate::schedule::parse_hhmm(to)?;
    let minute = minute % (24 * 60);
    Some(if from <= to {
        (from..to).contains(&minute)
    } else {
        minute >= from || minute < to
    })
}

/// How often the clock is looked at: hours are set to the minute.
const LOOK_EVERY: Duration = Duration::from_secs(10);

/// The local minute of the day, read at most every ten seconds — the
/// loops ask every frame.
#[derive(Debug, Default)]
pub struct Clock {
    last: Option<(Instant, Option<u32>)>,
}

impl Clock {
    /// The minute of the day now, if the clock reads.
    pub fn minute(&mut self) -> Option<u32> {
        if let Some((at, minute)) = self.last {
            if at.elapsed() < LOOK_EVERY {
                return minute;
            }
        }
        let minute = crate::schedule::local_now().map(|t| t.minute);
        self.last = Some((Instant::now(), minute));
        minute
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hours(from: &str, to: &str) -> Hours {
        Hours {
            from: from.into(),
            to: to.into(),
        }
    }

    #[test]
    fn night_hours_run_across_midnight() {
        let night = hours("22:00", "06:00");
        assert!(night.contains(23 * 60));
        assert!(night.contains(3 * 60));
        assert!(!night.contains(6 * 60));
        assert!(!night.contains(12 * 60));
        let day = hours("08:00", "20:00");
        assert!(day.contains(12 * 60));
        assert!(!day.contains(21 * 60));
    }

    #[test]
    fn a_typo_or_an_empty_span_keeps_it_there() {
        assert!(hours("25:00", "06:00").contains(12 * 60));
        assert!(hours("", "").contains(12 * 60));
        assert!(hours("10:00", "10:00").contains(3 * 60));
    }

    #[test]
    fn outside_its_hours_a_character_is_not_there_but_in_edit_mode() {
        let mut scene = crate::scene::Scene::from_config(&crate::config::AppConfig::default());
        scene.entities[0].hours = Some(hours("22:00", "06:00"));
        let n = scene.visible_entities().len();
        assert!(scene.set_clock(Some(12 * 60)), "noon: the night one goes");
        assert!(!scene.effective_visible(0));
        assert_eq!(scene.visible_entities().len(), n - 1);
        assert!(!scene.set_clock(Some(12 * 60 + 1)), "nothing new");
        assert!(scene.set_clock(Some(23 * 60)), "night: back");
        assert!(scene.effective_visible(0));
        scene.set_clock(Some(12 * 60));
        assert!(scene.set_clock(None), "edit mode: everyone");
        assert_eq!(scene.visible_entities().len(), n);
    }

    #[test]
    fn hours_go_into_the_config_and_back() {
        let mut c = crate::config::AppConfig::default().characters[0].clone();
        c.hours = Some(hours("22:00", "06:00"));
        let text = toml::to_string(&c).unwrap();
        let back: crate::config::CharacterConfig = toml::from_str(&text).unwrap();
        assert_eq!(back.hours, c.hours);
        c.hours = None;
        assert!(!toml::to_string(&c).unwrap().contains("hours"));
    }

    #[test]
    fn hours_are_kept_short() {
        let mut h = hours("  22:00:00:00 ", "06:00");
        h.sanitize();
        assert_eq!(h.from, "22:00");
    }
}
