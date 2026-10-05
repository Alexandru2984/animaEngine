//! A focus timer (1.5), the Pomodoro way: twenty-five minutes of work,
//! then a character says it is time for a break; five minutes later, that
//! it is time to get back to it; and round again until it is stopped.
//!
//! The tray and D-Bus start and stop it from their own threads, and the
//! loops read it every frame, so it is one process-wide timer, like
//! snoozing (`crate::snooze`), and it does not outlive the app. Each turn
//! of it is said once, by the frontmost character on screen; one that
//! comes while the overlay is out of sight waits for it. Reminders hold
//! their tongue while it runs (`crate::reminders`): during the work, they
//! wait; the break counts as a break.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// `[global.focus]`: how long the work and the break last, in minutes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusConfig {
    #[serde(default = "default_minutes")]
    pub minutes: u32,
    #[serde(default = "default_break")]
    pub break_minutes: u32,
}

fn default_minutes() -> u32 {
    25
}

fn default_break() -> u32 {
    5
}

impl Default for FocusConfig {
    fn default() -> Self {
        Self {
            minutes: default_minutes(),
            break_minutes: default_break(),
        }
    }
}

impl FocusConfig {
    /// Left out of the config while it is as it comes.
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Within what a person would mean: 1 to 180 minutes of work, 1 to 60
    /// of break.
    pub fn sanitized(self) -> Self {
        Self {
            minutes: self.minutes.clamp(1, 180),
            break_minutes: self.break_minutes.clamp(1, 60),
        }
    }
}

/// `config`'s lengths, with `minutes` of work instead when it is not 0 —
/// a start asked for over D-Bus may name its own.
pub fn lengths_for(minutes: u32, config: FocusConfig) -> FocusConfig {
    match minutes {
        0 => config,
        minutes => FocusConfig { minutes, ..config },
    }
}

/// Which part of the round it is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Work,
    Break,
}

/// What a character is to say about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum News {
    /// It started: work, until a time the line names.
    Started,
    /// The break began.
    Break,
    /// The break is over: back to work.
    BackToWork,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Timer {
    phase: Phase,
    /// When this part ends, in seconds since the epoch.
    until: u64,
    lengths: FocusConfig,
}

#[derive(Debug, Default)]
struct State {
    timer: Option<Timer>,
    /// Said next, once the overlay is on screen.
    news: Option<News>,
}

static STATE: Mutex<State> = Mutex::new(State {
    timer: None,
    news: None,
});

/// Tests that start the timer take turns: it is one for the process.
#[cfg(test)]
pub(crate) static TEST_LOCK: Mutex<()> = Mutex::new(());

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn with<T>(f: impl FnOnce(&mut State) -> T) -> Option<T> {
    STATE.lock().ok().map(|mut s| f(&mut s))
}

/// Start (or start over) with `lengths`, working from `now`.
fn start_at(state: &mut State, lengths: FocusConfig, now: u64) {
    let lengths = lengths.sanitized();
    state.timer = Some(Timer {
        phase: Phase::Work,
        until: now + u64::from(lengths.minutes) * 60,
        lengths,
    });
    state.news = Some(News::Started);
}

/// The timer at `now`: once a part is over, the next begins from `now` —
/// a machine that slept through several is not made to catch up — and
/// what to say about it.
fn advance_at(state: &mut State, now: u64) {
    let Some(timer) = state.timer.as_mut() else {
        return;
    };
    if now < timer.until {
        return;
    }
    let (phase, minutes, news) = match timer.phase {
        Phase::Work => (Phase::Break, timer.lengths.break_minutes, News::Break),
        Phase::Break => (Phase::Work, timer.lengths.minutes, News::BackToWork),
    };
    timer.phase = phase;
    timer.until = now + u64::from(minutes) * 60;
    state.news = Some(news);
}

/// Start focusing with `lengths`: the tray, D-Bus, the palette, the
/// Scene tab. Running already, it starts over.
pub fn start(lengths: FocusConfig) {
    with(|s| start_at(s, lengths, now_secs()));
    tracing::info!("Focus started: {} min", lengths.sanitized().minutes);
}

/// Stop it.
pub fn stop() {
    if with(|s| {
        s.news = None;
        s.timer.take().is_some()
    }) == Some(true)
    {
        tracing::info!("Focus stopped");
    }
}

/// Which part it is in, if it runs. Moves it on when a part is over.
pub fn phase() -> Option<Phase> {
    with(|s| {
        advance_at(s, now_secs());
        s.timer.map(|t| t.phase)
    })
    .flatten()
}

/// What to say now, if anything — taken only when `shown`, so news that
/// comes while the overlay is out of sight waits. The loops ask every
/// frame.
pub fn take_news(shown: bool) -> Option<News> {
    with(|s| {
        advance_at(s, now_secs());
        if shown {
            s.news.take()
        } else {
            None
        }
    })
    .flatten()
}

/// When the current part ends, as a local "HH:MM", and which part it is.
pub fn until_label() -> Option<(Phase, String)> {
    let (phase, until) = with(|s| {
        advance_at(s, now_secs());
        s.timer.map(|t| (t.phase, t.until))
    })
    .flatten()?;
    let left_minutes = until.saturating_sub(now_secs()).div_ceil(60);
    let now = crate::schedule::local_now()?.minute;
    let at = (u64::from(now) + left_minutes) % (24 * 60);
    Some((phase, crate::schedule::format_hhmm(at as u32)))
}

/// The line for `news`, in the user's language.
pub fn line(news: News) -> String {
    // A time, not a number of minutes: no plurals to get wrong.
    let mut args = fluent::FluentArgs::new();
    args.set("until", until_label().map(|(_, at)| at).unwrap_or_default());
    let key = match news {
        News::Started => "focus-said-started",
        News::Break => "focus-said-break",
        News::BackToWork => "focus-said-back",
    };
    crate::i18n::t_args(key, &args)
}

/// How long a turn of it is said for.
pub const SAY_SECONDS: f32 = 12.0;

/// Have what there is to say about the timer said, by the frontmost
/// visible character `on_surface` accepts — when the overlay is `shown`
/// and someone is there to say it; otherwise it waits. The loops call it
/// every frame. Returns whether someone spoke.
pub fn say_news(
    scene: &mut crate::scene::Scene,
    shown: bool,
    sound: &crate::reminders::ReminderSound,
    notify: bool,
    on_surface: impl Fn(&crate::entity::Entity) -> bool,
) -> bool {
    let speaker = (0..scene.entities.len())
        .filter(|&i| scene.effective_visible(i) && on_surface(&scene.entities[i]))
        .max_by_key(|&i| scene.entities[i].z_index)
        .filter(|_| shown);
    // Out of sight it waits — unless it can go to the desktop's
    // notifications (`crate::notify`).
    let Some(news) = take_news(speaker.is_some() || notify) else {
        return false;
    };
    let text = line(news);
    if notify {
        crate::notify::send(&crate::i18n::t("scene-focus-header"), &text);
    }
    let Some(i) = speaker else {
        return false;
    };
    tracing::info!("Focus said by {}: {text}", scene.entities[i].name);
    scene.entities[i].say(&text, SAY_SECONDS);
    sound.play_by(&mut scene.entities[i]);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lengths(minutes: u32, break_minutes: u32) -> FocusConfig {
        FocusConfig {
            minutes,
            break_minutes,
        }
    }

    #[test]
    fn work_then_a_break_then_work_again() {
        let mut s = State::default();
        start_at(&mut s, lengths(25, 5), 1000);
        assert_eq!(s.news.take(), Some(News::Started));
        advance_at(&mut s, 1000 + 25 * 60 - 1);
        assert_eq!(s.news, None, "not yet");
        advance_at(&mut s, 1000 + 25 * 60);
        assert_eq!(s.news.take(), Some(News::Break));
        assert_eq!(s.timer.unwrap().phase, Phase::Break);
        advance_at(&mut s, 1000 + 30 * 60);
        assert_eq!(s.news.take(), Some(News::BackToWork));
        assert_eq!(s.timer.unwrap().until, 1000 + 55 * 60);
    }

    #[test]
    fn a_long_sleep_moves_it_on_once() {
        let mut s = State::default();
        start_at(&mut s, lengths(25, 5), 0);
        advance_at(&mut s, 10 * 3600);
        assert_eq!(s.news, Some(News::Break), "one turn, not twenty");
        assert_eq!(s.timer.unwrap().until, 10 * 3600 + 5 * 60);
    }

    #[test]
    fn lengths_are_kept_sensible() {
        assert_eq!(lengths(0, 0).sanitized(), lengths(1, 1));
        assert_eq!(lengths(9999, 999).sanitized(), lengths(180, 60));
        assert!(FocusConfig::default().is_default());
    }

    #[test]
    fn news_waits_for_the_overlay_and_stopping_ends_it() {
        let _turn = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        start(FocusConfig::default());
        assert_eq!(phase(), Some(Phase::Work));
        assert_eq!(take_news(false), None, "out of sight: it waits");
        assert_eq!(take_news(true), Some(News::Started));
        assert_eq!(take_news(true), None, "said once");
        stop();
        assert_eq!(phase(), None);
        assert_eq!(until_label(), None);
    }
}
