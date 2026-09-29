//! Reminders (1.5): something a character says every so often — a break, a
//! glass of water — in a speech bubble (`crate::speech`).
//!
//! Time counts only while someone is there: when the user is away
//! (`crate::away`) every timer starts over, since being away was the
//! break. One that comes due while the overlay is hidden, stepped aside for
//! a full-screen app, or has nobody to say it waits until it can be said.
//! The bubble goes to the chosen character, or — when that one is gone or
//! hidden — to the frontmost visible one, on whichever monitor.

use crate::entity::Entity;
use crate::scene::Scene;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// One reminder, as configured (`[[reminders]]`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReminderConfig {
    pub text: String,
    #[serde(default = "default_every")]
    pub every_minutes: u32,
    /// Who says it, by character id; the frontmost visible one when unset
    /// or not there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub character: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_every() -> u32 {
    60
}
fn default_true() -> bool {
    true
}

/// The bounds on how often, in minutes: at most once a minute, at least
/// once a day.
pub const MIN_MINUTES: u32 = 1;
pub const MAX_MINUTES: u32 = 24 * 60;
/// How long a reminder's bubble stays.
pub const SAY_SECONDS: f32 = 12.0;

impl ReminderConfig {
    fn every(&self) -> Duration {
        Duration::from_secs(u64::from(self.every_minutes.clamp(MIN_MINUTES, MAX_MINUTES)) * 60)
    }
}

/// When each reminder is next due. Kept by the reminder itself, not its
/// place in the list, so adding or deleting one leaves the others' timers
/// alone; an edited one starts over.
#[derive(Debug, Default)]
pub struct Timers {
    due: Vec<(ReminderConfig, Instant)>,
}

impl Timers {
    /// The reminders due at `now`, each rescheduled. With `away` every
    /// timer starts over and none is due; without `can_say` they wait.
    pub fn poll(
        &mut self,
        now: Instant,
        reminders: &[ReminderConfig],
        away: bool,
        can_say: bool,
    ) -> Vec<ReminderConfig> {
        // Match the list: keep a timer for each reminder still there, start
        // one for each new one.
        let mut old = std::mem::take(&mut self.due);
        for r in reminders {
            let due = match old.iter().position(|(o, _)| o == r) {
                Some(i) => old.swap_remove(i).1,
                None => now + r.every(),
            };
            self.due.push((r.clone(), due));
        }
        if away {
            for (r, due) in &mut self.due {
                *due = now + r.every();
            }
            return Vec::new();
        }
        if !can_say {
            return Vec::new();
        }
        let mut fired = Vec::new();
        for (r, due) in &mut self.due {
            if r.enabled && *due <= now {
                *due = now + r.every();
                fired.push(r.clone());
            }
        }
        fired
    }
}

/// Have the due reminders said, by characters `on_surface` accepts (the
/// ones whose bubble shows). `shown` is whether the overlay is on screen at
/// all. Returns whether anything was said.
pub fn deliver(
    timers: &mut Timers,
    reminders: &[ReminderConfig],
    scene: &mut Scene,
    shown: bool,
    away: bool,
    on_surface: impl Fn(&Entity) -> bool,
) -> bool {
    let speakers: Vec<usize> = (0..scene.entities.len())
        .filter(|&i| scene.effective_visible(i) && on_surface(&scene.entities[i]))
        .collect();
    let fired = timers.poll(
        Instant::now(),
        reminders,
        away,
        shown && !speakers.is_empty(),
    );
    for r in &fired {
        let chosen = r.character.as_deref().and_then(|id| {
            speakers
                .iter()
                .copied()
                .find(|&i| scene.entities[i].id == id)
        });
        let frontmost = || {
            speakers
                .iter()
                .copied()
                .max_by_key(|&i| scene.entities[i].z_index)
        };
        if let Some(i) = chosen.or_else(frontmost) {
            tracing::info!("Reminder said by {}: {}", scene.entities[i].name, r.text);
            scene.entities[i].say(&r.text, SAY_SECONDS);
        }
    }
    !fired.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn every(text: &str, minutes: u32) -> ReminderConfig {
        ReminderConfig {
            text: text.into(),
            every_minutes: minutes,
            character: None,
            enabled: true,
        }
    }

    const MIN: Duration = Duration::from_secs(60);

    #[test]
    fn a_reminder_comes_due_every_so_often() {
        let (mut t, t0) = (Timers::default(), Instant::now());
        let list = [every("water", 5)];
        assert!(
            t.poll(t0, &list, false, true).is_empty(),
            "not at the start"
        );
        assert!(t.poll(t0 + 4 * MIN, &list, false, true).is_empty());
        assert_eq!(t.poll(t0 + 5 * MIN, &list, false, true).len(), 1);
        assert!(
            t.poll(t0 + 6 * MIN, &list, false, true).is_empty(),
            "rescheduled"
        );
        assert_eq!(t.poll(t0 + 10 * MIN, &list, false, true).len(), 1);
    }

    #[test]
    fn being_away_starts_it_over() {
        let (mut t, t0) = (Timers::default(), Instant::now());
        let list = [every("break", 50)];
        t.poll(t0, &list, false, true);
        t.poll(t0 + 45 * MIN, &list, true, true);
        assert!(
            t.poll(t0 + 50 * MIN, &list, false, true).is_empty(),
            "the time away was the break"
        );
        assert_eq!(t.poll(t0 + 95 * MIN, &list, false, true).len(), 1);
    }

    #[test]
    fn one_that_cannot_be_said_waits() {
        let (mut t, t0) = (Timers::default(), Instant::now());
        let list = [every("stretch", 1)];
        t.poll(t0, &list, false, true);
        assert!(
            t.poll(t0 + 2 * MIN, &list, false, false).is_empty(),
            "hidden"
        );
        assert_eq!(
            t.poll(t0 + 3 * MIN, &list, false, true).len(),
            1,
            "then said"
        );
    }

    #[test]
    fn editing_one_leaves_the_others_timers() {
        let (mut t, t0) = (Timers::default(), Instant::now());
        t.poll(t0, &[every("a", 10), every("b", 10)], false, true);
        // At 8 min "a" is changed and a "c" added: "b" keeps its time.
        let edited = [every("a2", 10), every("b", 10), every("c", 10)];
        t.poll(t0 + 8 * MIN, &edited, false, true);
        let at_10: Vec<_> = t
            .poll(t0 + 10 * MIN, &edited, false, true)
            .into_iter()
            .map(|r| r.text)
            .collect();
        assert_eq!(at_10, ["b"]);
    }

    #[test]
    fn a_switched_off_reminder_says_nothing() {
        let (mut t, t0) = (Timers::default(), Instant::now());
        let mut r = every("off", 1);
        r.enabled = false;
        t.poll(t0, std::slice::from_ref(&r), false, true);
        assert!(t.poll(t0 + 5 * MIN, &[r], false, true).is_empty());
    }

    #[test]
    fn the_chosen_character_says_it_or_the_frontmost_does() {
        let mut scene = Scene::from_config(&crate::config::AppConfig::default());
        let mut t = Timers::default();
        let mut r = every("hi", 1);
        r.character = Some(scene.entities[0].id.clone());
        // Due at once: start the timer a minute back.
        t.due.push((r.clone(), Instant::now() - MIN));
        assert!(deliver(
            &mut t,
            std::slice::from_ref(&r),
            &mut scene,
            true,
            false,
            |_| true
        ));
        assert!(scene.entities[0].speech.is_some());
        // Gone from the scene: the frontmost says it.
        let mut gone = every("hello", 1);
        gone.character = Some("nobody".into());
        t.due.push((gone.clone(), Instant::now() - MIN));
        deliver(&mut t, &[r, gone], &mut scene, true, false, |_| true);
        let front = (0..scene.entities.len())
            .max_by_key(|&i| scene.entities[i].z_index)
            .unwrap();
        assert_eq!(scene.entities[front].speech.as_ref().unwrap().text, "hello");
    }
}
