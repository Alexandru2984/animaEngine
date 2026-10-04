//! Snoozing (1.5): the characters out of sight for a while — a quarter of
//! an hour, an hour, or until the morning — and back on their own.
//!
//! The tray and D-Bus set it from their own threads and the loops read it
//! every frame, so the deadline is one process-wide atomic: seconds since
//! the epoch, 0 for none. While it holds, the loops step aside as they do
//! for a full-screen app set to hide (`crate::fullscreen`): hidden and
//! still, reminders waiting. It does not outlive the app.

use std::sync::atomic::{AtomicU64, Ordering};

/// When "until the morning" ends: 07:00, local time.
pub const MORNING_MINUTE: u32 = 7 * 60;

/// How long to snooze.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnoozeFor {
    Minutes(u32),
    /// Until [`MORNING_MINUTE`] next comes round.
    Morning,
}

/// The deadline, in seconds since the epoch; 0 when not snoozing.
static UNTIL: AtomicU64 = AtomicU64::new(0);

/// Tests that snooze take turns: the deadline is one for the process.
#[cfg(test)]
pub(crate) static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// How many seconds `what` lasts from the local minute of the day `minute`
/// (`None` when the clock cannot be read: the morning is then eight hours
/// off). At most a day.
pub fn seconds_for(what: SnoozeFor, minute: Option<u32>) -> u64 {
    let minutes = match what {
        SnoozeFor::Minutes(m) => m.clamp(1, 24 * 60),
        SnoozeFor::Morning => match minute {
            Some(now) => {
                let now = now % (24 * 60);
                if now < MORNING_MINUTE {
                    MORNING_MINUTE - now
                } else {
                    24 * 60 - now + MORNING_MINUTE
                }
            }
            None => 8 * 60,
        },
    };
    u64::from(minutes) * 60
}

/// Snooze for `what`, from now.
pub fn snooze(what: SnoozeFor) {
    let minute = crate::schedule::local_now().map(|t| t.minute);
    let secs = seconds_for(what, minute);
    UNTIL.store(now_secs() + secs, Ordering::Relaxed);
    tracing::info!("Snoozing for {} min", secs / 60);
}

/// Wake up now.
pub fn wake() {
    if UNTIL.swap(0, Ordering::Relaxed) != 0 {
        tracing::info!("Awake again");
    }
}

/// Whether the characters are snoozing now. Once the time is up this says
/// no, and the deadline is cleared.
pub fn active() -> bool {
    let until = UNTIL.load(Ordering::Relaxed);
    if until == 0 {
        return false;
    }
    if now_secs() < until {
        return true;
    }
    let _ = UNTIL.compare_exchange(until, 0, Ordering::Relaxed, Ordering::Relaxed);
    false
}

/// The deadline as a local "HH:MM", while snoozing.
pub fn until_label() -> Option<String> {
    if !active() {
        return None;
    }
    let left_minutes = UNTIL
        .load(Ordering::Relaxed)
        .saturating_sub(now_secs())
        .div_ceil(60);
    let now = crate::schedule::local_now()?.minute;
    let at = (u64::from(now) + left_minutes) % (24 * 60);
    Some(crate::schedule::format_hhmm(at as u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minutes_are_minutes_within_a_day() {
        assert_eq!(seconds_for(SnoozeFor::Minutes(15), Some(600)), 900);
        assert_eq!(seconds_for(SnoozeFor::Minutes(0), None), 60);
        assert_eq!(seconds_for(SnoozeFor::Minutes(99_999), None), 24 * 3600);
    }

    #[test]
    fn the_morning_is_the_next_seven_o_clock() {
        // 23:00 → 8 h; 01:30 → 5 h 30; 07:00 exactly → a day.
        assert_eq!(seconds_for(SnoozeFor::Morning, Some(23 * 60)), 8 * 3600);
        assert_eq!(seconds_for(SnoozeFor::Morning, Some(90)), 330 * 60);
        assert_eq!(seconds_for(SnoozeFor::Morning, Some(7 * 60)), 24 * 3600);
        assert_eq!(seconds_for(SnoozeFor::Morning, None), 8 * 3600);
    }

    #[test]
    fn snoozing_ends_when_woken() {
        let _turn = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        snooze(SnoozeFor::Minutes(5));
        assert!(active());
        assert!(until_label().is_some() || crate::schedule::local_now().is_none());
        wake();
        assert!(!active());
        assert_eq!(until_label(), None);
    }
}
