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
    /// At a set time instead of every so often (`At`): `every_minutes`
    /// is then not used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<At>,
}

/// When a reminder at a set time is said (1.5): a time of day on some
/// days — `at = { time = "10:00", days = "weekdays" }` — or on a date each
/// year, whatever the day — `at = { time = "09:00", date = "10-12" }`
/// (month-day).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct At {
    /// "HH:MM", local time.
    pub time: String,
    #[serde(default)]
    pub days: crate::schedule::Days,
    /// "MM-DD".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

/// How late a reminder at a set time is still said: a start, or a wake
/// from sleep, a little after its time still brings it — not an evening
/// start the morning's stand-up. A date's lasts the rest of the day.
pub const LATE_BY_MINUTES: u32 = 120;

impl At {
    /// The (month, day) of its date, if it has one that reads.
    pub fn date(&self) -> Option<(u8, u8)> {
        let (m, d) = self.date.as_deref()?.trim().split_once('-')?;
        let (m, d): (u8, u8) = (m.parse().ok()?, d.parse().ok()?);
        ((1..=12).contains(&m) && (1..=31).contains(&d)).then_some((m, d))
    }

    /// Whether it is due at local `now`: on its day, from its time to
    /// [`LATE_BY_MINUTES`] after — or to midnight, for a date. Never if
    /// the time does not read.
    pub fn due_at(&self, now: crate::schedule::LocalTime) -> bool {
        let Some(time) = crate::schedule::parse_hhmm(&self.time) else {
            return false;
        };
        let (day, late) = match self.date() {
            Some(date) => (date == (now.month, now.day), 24 * 60),
            None if self.date.is_some() => return false,
            None => (self.days.holds_on(now.weekday), time + LATE_BY_MINUTES),
        };
        day && now.minute >= time && now.minute < late
    }
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

/// What sounds when a reminder — or a turn of the focus timer
/// (`crate::focus`) — is said (1.5): nothing, as it comes; the speaker's
/// own tap sound; or a file in the asset library (`crate::sounds`).
/// `reminder_sound = "speaker"`, or `reminder_sound = { file = "…" }`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderSound {
    #[default]
    None,
    Speaker,
    File(String),
}

impl ReminderSound {
    /// Left out of the config while it is as it comes.
    pub fn is_none(&self) -> bool {
        *self == Self::None
    }

    /// Have `entity`, which is saying a reminder, play it.
    pub fn play_by(&self, entity: &mut Entity) {
        match self {
            Self::None => {}
            Self::Speaker => entity.want_tap_sound(),
            Self::File(sound) => entity.want_sound(sound),
        }
    }

    /// A file kept as a sound path is (`sounds::tidy`); none, if it is no
    /// sound.
    pub fn sanitize(&mut self) {
        if let Self::File(sound) = self {
            let kept = crate::sounds::tidy(sound);
            *self = if kept.is_empty() {
                Self::None
            } else {
                Self::File(kept)
            };
        }
    }
}

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
    /// Reminders at a set time, and the (month, day) each was last said.
    said_on: Vec<(ReminderConfig, (u8, u8))>,
    /// The local clock, read at most every ten seconds.
    clock: Option<(Instant, Option<crate::schedule::LocalTime>)>,
}

impl Timers {
    /// The soonest reminder still to come after `now`, for a loop that
    /// sleeps until it has something to do. One already due and waiting —
    /// for the focus timer's work to end, the overlay to show — is not:
    /// what it waits for wakes the loop itself.
    pub fn next_due(&self, now: Instant) -> Option<Instant> {
        self.due
            .iter()
            .map(|(_, at)| *at)
            .filter(|at| *at > now)
            .min()
    }

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
        // Those at a set time are `poll_timed`'s.
        for r in reminders.iter().filter(|r| r.at.is_none()) {
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

impl Timers {
    /// The reminders at a set time due at local `clock`, each once a day.
    /// Away, or unable to be said, they wait — within their time.
    pub fn poll_timed(
        &mut self,
        clock: Option<crate::schedule::LocalTime>,
        reminders: &[ReminderConfig],
        away: bool,
        can_say: bool,
    ) -> Vec<ReminderConfig> {
        self.said_on.retain(|(r, _)| reminders.contains(r));
        let Some(now) = clock.filter(|_| !away && can_say) else {
            return Vec::new();
        };
        let today = (now.month, now.day);
        let mut fired = Vec::new();
        for r in reminders.iter().filter(|r| r.enabled) {
            let Some(at) = &r.at else {
                continue;
            };
            let said_today = self.said_on.iter().any(|(s, on)| s == r && *on == today);
            if at.due_at(now) && !said_today {
                self.said_on.retain(|(s, _)| s != r);
                self.said_on.push((r.clone(), today));
                fired.push(r.clone());
            }
        }
        fired
    }

    /// The local clock now, read at most every ten seconds.
    fn clock(&mut self, now: Instant) -> Option<crate::schedule::LocalTime> {
        match self.clock {
            Some((at, clock)) if now.duration_since(at) < Duration::from_secs(10) => clock,
            _ => {
                let clock = crate::schedule::local_now();
                self.clock = Some((now, clock));
                clock
            }
        }
    }
}

/// Where the user is, for reminders: `on_a_break` starts the ones every so
/// often over — time without input, bedtime, the focus timer's break; only
/// `idle`, no input, makes one at a set time wait: it was asked for that
/// time, bedtime or not.
#[derive(Debug, Clone, Copy, Default)]
pub struct Away {
    pub on_a_break: bool,
    pub idle: bool,
    /// The focus timer's work (`crate::focus`): every reminder waits it
    /// out — on screen and in the desktop's notifications alike.
    pub working: bool,
}

/// Have the due reminders said, by characters `on_surface` accepts (the
/// ones whose bubble shows). `shown` is whether the overlay is on screen at
/// all. Returns whether anything was said.
#[expect(clippy::too_many_arguments)]
pub fn deliver(
    timers: &mut Timers,
    reminders: &[ReminderConfig],
    sound: &ReminderSound,
    notify: bool,
    scene: &mut Scene,
    shown: bool,
    away: Away,
    on_surface: impl Fn(&Entity) -> bool,
) -> bool {
    let speakers: Vec<usize> = (0..scene.entities.len())
        .filter(|&i| scene.effective_visible(i) && on_surface(&scene.entities[i]))
        .collect();
    // Out of sight, a reminder waits for the characters — unless it can go
    // to the desktop's notifications (`crate::notify`). While the focus
    // timer's work lasts it waits either way: the notifications stand in
    // for characters out of sight, not for the quiet that focus asks.
    let can_say = shown && !speakers.is_empty();
    let can = !away.working && (can_say || notify);
    let now = Instant::now();
    let mut fired = timers.poll(now, reminders, away.on_a_break, can);
    let clock = timers.clock(now);
    fired.extend(timers.poll_timed(clock, reminders, away.idle, can));
    let mut used: Vec<usize> = Vec::new();
    for r in &fired {
        let speaker = if can_say {
            pick(scene, &speakers, r, &used)
        } else {
            None
        };
        if notify {
            // Under the name of who says it — or would, were they in
            // sight — so a `{name}` in it reads as it does in the bubble.
            let by_z = |e: &&Entity| e.z_index;
            let name = speaker
                .map(|i| &scene.entities[i])
                .or_else(|| {
                    r.character
                        .as_deref()
                        .and_then(|id| scene.entities.iter().find(|e| e.id == id))
                })
                .or_else(|| scene.entities.iter().max_by_key(by_z))
                .map_or_else(|| "animaEngine".to_string(), |e| e.name.clone());
            crate::notify::send(&name, &crate::placeholders::fill_now(&r.text, &name));
        }
        if let Some(i) = speaker {
            used.push(i);
            tracing::info!("Reminder said by {}: {}", scene.entities[i].name, r.text);
            scene.entities[i].say(&r.text, SAY_SECONDS);
            sound.play_by(&mut scene.entities[i]);
        }
    }
    !fired.is_empty()
}

/// Who of `speakers` says `r`: the character it names, if it is there, or
/// the frontmost free one — not yet `used`, and not in the middle of
/// saying something else. Several at once go to several, front to back,
/// where there are: one bubble would cover the other.
fn pick(scene: &Scene, speakers: &[usize], r: &ReminderConfig, used: &[usize]) -> Option<usize> {
    let chosen = r.character.as_deref().and_then(|id| {
        speakers
            .iter()
            .copied()
            .find(|&i| scene.entities[i].id == id)
    });
    let by_z = |&i: &usize| scene.entities[i].z_index;
    let now = Instant::now();
    let talking = |i: usize| {
        scene.entities[i]
            .speech
            .as_ref()
            .is_some_and(|s| s.showing(now))
    };
    let frontmost = |free: &dyn Fn(usize) -> bool| {
        speakers
            .iter()
            .copied()
            .filter(|&i| free(i))
            .max_by_key(by_z)
    };
    chosen
        .or_else(|| frontmost(&|i| !used.contains(&i) && !talking(i)))
        .or_else(|| frontmost(&|i| !used.contains(&i)))
        .or_else(|| frontmost(&|_| true))
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
            at: None,
        }
    }

    const MIN: Duration = Duration::from_secs(60);

    /// A sleeping loop wakes for the next reminder to come — and not for
    /// one already due and waiting, which would wake it at once, forever.
    #[test]
    fn the_next_due_is_still_to_come() {
        let now = Instant::now();
        let mut timers = Timers::default();
        assert_eq!(timers.next_due(now), None);
        timers.due = vec![
            (every("soon", 1), now + MIN),
            (every("later", 5), now + 5 * MIN),
            (every("waiting", 1), now - MIN),
        ];
        assert_eq!(timers.next_due(now), Some(now + MIN));
        assert_eq!(timers.next_due(now + 2 * MIN), Some(now + 5 * MIN));
        assert_eq!(timers.next_due(now + 6 * MIN), None);
    }

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
            &ReminderSound::None,
            false,
            &mut scene,
            true,
            Away::default(),
            |_| true
        ));
        assert!(scene.entities[0].speech.is_some());
        assert!(scene.take_sounds().is_empty(), "no sound as it comes");
        // Gone from the scene: the frontmost says it.
        let mut gone = every("hello", 1);
        gone.character = Some("nobody".into());
        t.due.push((gone.clone(), Instant::now() - MIN));
        deliver(
            &mut t,
            &[r, gone],
            &ReminderSound::None,
            false,
            &mut scene,
            true,
            Away::default(),
            |_| true,
        );
        let front = (0..scene.entities.len())
            .max_by_key(|&i| scene.entities[i].z_index)
            .unwrap();
        assert_eq!(scene.entities[front].speech.as_ref().unwrap().text, "hello");
    }

    #[test]
    fn a_reminder_sounds_as_asked() {
        let mut scene = Scene::from_config(&crate::config::AppConfig::default());
        let r = every("water", 1);
        let say = |scene: &mut Scene, sound: &ReminderSound| {
            let mut t = Timers::default();
            t.due.push((r.clone(), Instant::now() - MIN));
            deliver(
                &mut t,
                std::slice::from_ref(&r),
                sound,
                false,
                scene,
                true,
                Away::default(),
                |_| true,
            );
            scene.take_sounds()
        };
        let chime = ReminderSound::File("chime.ogg".into());
        let sounds = say(&mut scene, &chime);
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].1, "chime.ogg");
        // The speaker's own: none here until it has one.
        assert!(say(&mut scene, &ReminderSound::Speaker).is_empty());
        for e in &mut scene.entities {
            e.talk.sound = "meow.ogg".into();
        }
        assert_eq!(say(&mut scene, &ReminderSound::Speaker)[0].1, "meow.ogg");
    }

    #[test]
    fn out_of_sight_it_waits_unless_it_can_notify() {
        let mut scene = Scene::from_config(&crate::config::AppConfig::default());
        let r = every("water", 1);
        let mut t = Timers::default();
        t.due.push((r.clone(), Instant::now() - MIN));
        let list = std::slice::from_ref(&r);
        let none = ReminderSound::None;
        let here = Away::default();
        deliver(&mut t, list, &none, false, &mut scene, false, here, |_| {
            true
        });
        assert!(t.due[0].1 <= Instant::now(), "hidden: still due, waiting");
        deliver(&mut t, list, &none, true, &mut scene, false, here, |_| true);
        assert!(
            t.due[0].1 > Instant::now(),
            "sent as a notification instead"
        );
        assert!(
            scene.entities.iter().all(|e| e.speech.is_none()),
            "nobody said it"
        );
        let sent = crate::notify::SENT.lock().unwrap();
        assert!(sent.iter().any(|(_, body)| body == "water"), "{sent:?}");
    }

    fn at(time: &str, days: crate::schedule::Days, date: Option<&str>) -> ReminderConfig {
        ReminderConfig {
            at: Some(At {
                time: time.into(),
                days,
                date: date.map(Into::into),
            }),
            ..every("stand-up", 60)
        }
    }

    fn clock(weekday: u8, hhmm: &str, month: u8, day: u8) -> Option<crate::schedule::LocalTime> {
        Some(crate::schedule::LocalTime {
            weekday,
            minute: crate::schedule::parse_hhmm(hhmm).unwrap(),
            day,
            month,
        })
    }

    #[test]
    fn a_reminder_at_a_set_time_comes_once_on_its_days() {
        use crate::schedule::Days;
        let mut t = Timers::default();
        let list = [at("10:00", Days::Weekdays, None)];
        // Monday.
        assert!(t
            .poll_timed(clock(1, "09:59", 10, 5), &list, false, true)
            .is_empty());
        assert_eq!(
            t.poll_timed(clock(1, "10:00", 10, 5), &list, false, true)
                .len(),
            1
        );
        assert!(
            t.poll_timed(clock(1, "10:01", 10, 5), &list, false, true)
                .is_empty(),
            "once"
        );
        // Tuesday, started late: still within its two hours.
        assert_eq!(
            t.poll_timed(clock(2, "11:30", 10, 6), &list, false, true)
                .len(),
            1
        );
        // Wednesday evening: too late for the morning's.
        assert!(t
            .poll_timed(clock(3, "19:00", 10, 7), &list, false, true)
            .is_empty());
        // Saturday: not a weekday.
        assert!(t
            .poll_timed(clock(6, "10:00", 10, 10), &list, false, true)
            .is_empty());
        // The interval timers leave it alone.
        assert!(t
            .poll(Instant::now() + 3 * 60 * MIN, &list, false, true)
            .is_empty());
    }

    #[test]
    fn bedtime_does_not_hold_a_reminder_at_a_set_time() {
        use crate::schedule::Days;
        let mut scene = Scene::from_config(&crate::config::AppConfig::default());
        let now = crate::schedule::local_now().unwrap();
        let hhmm = crate::schedule::format_hhmm(now.minute);
        let list = [at(&hhmm, Days::Every, None)];
        let none = ReminderSound::None;
        let mut t = Timers::default();
        let bedtime = Away {
            on_a_break: true,
            ..Away::default()
        };
        deliver(
            &mut t,
            &list,
            &none,
            false,
            &mut scene,
            true,
            bedtime,
            |_| true,
        );
        assert!(
            scene.entities.iter().any(|e| e.speech.is_some()),
            "said at bedtime"
        );
        let mut scene = Scene::from_config(&crate::config::AppConfig::default());
        let mut t = Timers::default();
        let idle = Away {
            on_a_break: true,
            idle: true,
            ..Away::default()
        };
        deliver(&mut t, &list, &none, false, &mut scene, true, idle, |_| {
            true
        });
        assert!(
            scene.entities.iter().all(|e| e.speech.is_none()),
            "waits for you"
        );
    }

    #[test]
    fn focusing_holds_them_back_notifications_too() {
        let mut scene = Scene::from_config(&crate::config::AppConfig::default());
        let r = every("focus-held water", 1);
        let mut t = Timers::default();
        t.due.push((r.clone(), Instant::now() - MIN));
        let list = std::slice::from_ref(&r);
        let none = ReminderSound::None;
        let sent = |text: &str| {
            let sent = crate::notify::SENT.lock().unwrap();
            sent.iter().any(|(_, body)| body == text)
        };
        let working = Away {
            working: true,
            ..Away::default()
        };
        // Notifications on, in sight or not: nothing while the work lasts.
        for shown in [true, false] {
            assert!(!deliver(
                &mut t,
                list,
                &none,
                true,
                &mut scene,
                shown,
                working,
                |_| true
            ));
        }
        assert!(scene.entities.iter().all(|e| e.speech.is_none()));
        assert!(!sent("focus-held water"), "no notification either");
        assert!(t.due[0].1 <= Instant::now(), "still due, waiting");
        // The work over, it comes.
        assert!(deliver(
            &mut t,
            list,
            &none,
            true,
            &mut scene,
            true,
            Away::default(),
            |_| true
        ));
        assert!(scene.entities.iter().any(|e| e.speech.is_some()));
        assert!(sent("focus-held water"));
    }

    #[test]
    fn a_notification_goes_under_the_speakers_name() {
        let mut scene = Scene::from_config(&crate::config::AppConfig::default());
        let r = every("Water, {name}?", 1);
        let front = scene
            .entities
            .iter()
            .max_by_key(|e| e.z_index)
            .unwrap()
            .name
            .clone();
        let none = ReminderSound::None;
        // Said, and out of sight: the one who says it, or would.
        for shown in [true, false] {
            let mut t = Timers::default();
            t.due.push((r.clone(), Instant::now() - MIN));
            deliver(
                &mut t,
                std::slice::from_ref(&r),
                &none,
                true,
                &mut scene,
                shown,
                Away::default(),
                |_| true,
            );
            let sent = crate::notify::SENT.lock().unwrap();
            let body = format!("Water, {front}?");
            assert!(
                sent.iter().any(|(title, b)| *title == front && *b == body),
                "{sent:?}"
            );
        }
    }

    #[test]
    fn it_goes_to_one_not_already_talking() {
        let mut scene = Scene::from_config(&crate::config::AppConfig::default());
        let mut by_z: Vec<usize> = (0..scene.entities.len()).collect();
        by_z.sort_by_key(|&i| std::cmp::Reverse(scene.entities[i].z_index));
        scene.entities[by_z[0]].say("Break time!", 6.0);
        let r = every("water", 1);
        let mut t = Timers::default();
        t.due.push((r.clone(), Instant::now() - MIN));
        deliver(
            &mut t,
            std::slice::from_ref(&r),
            &ReminderSound::None,
            false,
            &mut scene,
            true,
            Away::default(),
            |_| true,
        );
        let text = |i: usize| scene.entities[i].speech.as_ref().map(|s| s.text.clone());
        assert_eq!(text(by_z[0]).as_deref(), Some("Break time!"), "not covered");
        assert_eq!(text(by_z[1]).as_deref(), Some("water"));
    }

    #[test]
    fn two_at_once_are_said_by_two() {
        let mut scene = Scene::from_config(&crate::config::AppConfig::default());
        let (a, b) = (every("water", 1), every("stretch", 1));
        let mut t = Timers::default();
        t.due.push((a.clone(), Instant::now() - MIN));
        t.due.push((b.clone(), Instant::now() - MIN));
        let none = ReminderSound::None;
        deliver(
            &mut t,
            &[a, b],
            &none,
            false,
            &mut scene,
            true,
            Away::default(),
            |_| true,
        );
        let said: Vec<_> = scene
            .entities
            .iter()
            .filter_map(|e| e.speech.as_ref().map(|s| s.text.clone()))
            .collect();
        assert_eq!(said.len(), 2, "{said:?}");
    }

    #[test]
    fn a_date_is_said_that_day_and_a_reminder_waits_for_you() {
        use crate::schedule::Days;
        let mut t = Timers::default();
        let list = [at("09:00", Days::Every, Some("10-12"))];
        assert!(t
            .poll_timed(clock(0, "09:30", 10, 11), &list, false, true)
            .is_empty());
        // Away at nine: it waits for the return, even the evening.
        assert!(t
            .poll_timed(clock(1, "09:30", 10, 12), &list, true, true)
            .is_empty());
        assert_eq!(
            t.poll_timed(clock(1, "18:00", 10, 12), &list, false, true)
                .len(),
            1
        );
        assert!(t
            .poll_timed(clock(1, "18:01", 10, 12), &list, false, true)
            .is_empty());
        // A date that does not read is never.
        let odd = [at("09:00", Days::Every, Some("13-40"))];
        assert!(t
            .poll_timed(clock(1, "09:00", 10, 12), &odd, false, true)
            .is_empty());
    }

    #[test]
    fn the_setting_reads_and_tidies() {
        #[derive(serde::Deserialize)]
        struct G {
            reminder_sound: ReminderSound,
        }
        let g: G = toml::from_str("reminder_sound = \"speaker\"").unwrap();
        assert_eq!(g.reminder_sound, ReminderSound::Speaker);
        let mut g: G = toml::from_str("reminder_sound = { file = \" a/chime.ogg \" }").unwrap();
        g.reminder_sound.sanitize();
        assert_eq!(g.reminder_sound, ReminderSound::File("a/chime.ogg".into()));
        let mut not_a_sound = ReminderSound::File("x.png".into());
        not_a_sound.sanitize();
        assert!(not_a_sound.is_none());
    }
}
