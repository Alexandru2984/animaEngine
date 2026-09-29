//! Scenes by time of day (1.5): switch to a named scene (`crate::scenes`)
//! at set times — "Work" at 09:00 on weekdays, "Evening" at 18:00 every
//! day.
//!
//! A rule fires when its time is *crossed*, not for as long as it is past:
//! a scene picked by hand after 09:00 stays until the next rule's time.
//! The first look after a start catches up on the latest rule already
//! passed today, so a machine started at 10:00 shows the 09:00 scene. The
//! loops do not look while in edit mode — a rule crossed then fires once
//! it ends, instead of swapping the scene under the user's hands — and a
//! rule for the scene already active does nothing.

use serde::{Deserialize, Serialize};

/// Which days a rule holds on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Days {
    #[default]
    Every,
    Weekdays,
    Weekends,
}

impl Days {
    pub const ALL: [Self; 3] = [Self::Every, Self::Weekdays, Self::Weekends];

    /// Whether it holds on `weekday` (0 = Sunday).
    pub fn holds_on(self, weekday: u8) -> bool {
        match self {
            Self::Every => true,
            Self::Weekdays => (1..=5).contains(&weekday),
            Self::Weekends => weekday == 0 || weekday == 6,
        }
    }

    pub fn i18n_key(self) -> &'static str {
        match self {
            Self::Every => "scene-schedule-every-day",
            Self::Weekdays => "scene-schedule-weekdays",
            Self::Weekends => "scene-schedule-weekends",
        }
    }
}

/// One rule (`[[scene_schedule]]`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleRule {
    /// A saved scene's name.
    pub scene: String,
    /// "HH:MM", local time.
    pub at: String,
    #[serde(default)]
    pub days: Days,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl ScheduleRule {
    /// Its time as minutes since midnight; `None` for a malformed one,
    /// which then never fires.
    pub fn minute(&self) -> Option<u32> {
        parse_hhmm(&self.at)
    }
}

/// "HH:MM" to minutes since midnight.
pub fn parse_hhmm(text: &str) -> Option<u32> {
    let (h, m) = text.trim().split_once(':')?;
    let (h, m): (u32, u32) = (h.trim().parse().ok()?, m.trim().parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

/// Minutes since midnight to "HH:MM".
pub fn format_hhmm(minute: u32) -> String {
    format!("{:02}:{:02}", (minute / 60) % 24, minute % 60)
}

/// A moment of local time: the day of the week (0 = Sunday) and the
/// minute of that day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTime {
    pub weekday: u8,
    pub minute: u32,
}

/// Local time now, from the system's own time zone.
pub fn local_now() -> Option<LocalTime> {
    #[cfg(unix)]
    {
        let now: libc::time_t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs()
            .try_into()
            .ok()?;
        // SAFETY: `localtime_r` writes only into the `tm` it is given and
        // reads only `now`; both live for the call.
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        let ok = unsafe { !libc::localtime_r(&now, &mut tm).is_null() };
        ok.then(|| LocalTime {
            weekday: tm.tm_wday.clamp(0, 6) as u8,
            minute: (tm.tm_hour.clamp(0, 23) * 60 + tm.tm_min.clamp(0, 59)) as u32,
        })
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::SystemInformation::GetLocalTime;
        // SAFETY: `GetLocalTime` fills the SYSTEMTIME it is given.
        let mut st = unsafe { std::mem::zeroed() };
        unsafe { GetLocalTime(&mut st) };
        Some(LocalTime {
            weekday: (st.wDayOfWeek % 7) as u8,
            minute: u32::from(st.wHour.min(23)) * 60 + u32::from(st.wMinute.min(59)),
        })
    }
    #[cfg(not(any(unix, windows)))]
    {
        None
    }
}

/// What the schedule last saw.
#[derive(Debug, Default)]
pub struct Schedule {
    last: Option<LocalTime>,
    /// When the clock was last read: once a second is plenty for rules
    /// set to the minute.
    looked: Option<std::time::Instant>,
}

impl Schedule {
    /// [`Self::due`] against the local clock, read at most once a second.
    pub fn due_now(&mut self, rules: &[ScheduleRule]) -> Option<String> {
        let now = std::time::Instant::now();
        if self
            .looked
            .is_some_and(|t| now.duration_since(t) < std::time::Duration::from_secs(1))
        {
            return None;
        }
        self.looked = Some(now);
        self.due(local_now()?, rules)
    }

    /// The scene a rule asks for at `now`: the latest rule crossed since
    /// the last look — or, at the first look, the latest already passed
    /// today. `None` when none was.
    pub fn due(&mut self, now: LocalTime, rules: &[ScheduleRule]) -> Option<String> {
        let last = self.last.replace(now);
        // Each rule crossed, keyed by when: (0 = the last day, 1 = today;
        // minute), so the latest wins.
        let mut crossed: Vec<((u8, u32), &ScheduleRule)> = Vec::new();
        for rule in rules.iter().filter(|r| r.enabled) {
            let Some(m) = rule.minute() else {
                continue;
            };
            let today = rule.days.holds_on(now.weekday);
            match last {
                None if today && m <= now.minute => crossed.push(((1, m), rule)),
                None => {}
                Some(last) if last == now => {}
                Some(last) if last.weekday == now.weekday && last.minute <= now.minute => {
                    if today && m > last.minute && m <= now.minute {
                        crossed.push(((1, m), rule));
                    }
                }
                // Midnight passed (or the clock went back): what was left
                // of the last day, then today so far.
                Some(last) => {
                    if rule.days.holds_on(last.weekday) && m > last.minute {
                        crossed.push(((0, m), rule));
                    }
                    if today && m <= now.minute {
                        crossed.push(((1, m), rule));
                    }
                }
            }
        }
        crossed
            .into_iter()
            .max_by_key(|(when, _)| *when)
            .map(|(_, rule)| rule.scene.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(scene: &str, at: &str, days: Days) -> ScheduleRule {
        ScheduleRule {
            scene: scene.into(),
            at: at.into(),
            days,
            enabled: true,
        }
    }

    fn at(weekday: u8, hhmm: &str) -> LocalTime {
        LocalTime {
            weekday,
            minute: parse_hhmm(hhmm).unwrap(),
        }
    }

    #[test]
    fn times_read_and_write_as_hh_mm() {
        assert_eq!(parse_hhmm("09:05"), Some(545));
        assert_eq!(parse_hhmm(" 9:5 "), Some(545));
        assert_eq!(parse_hhmm("24:00"), None);
        assert_eq!(parse_hhmm("12:60"), None);
        assert_eq!(parse_hhmm("noon"), None);
        assert_eq!(format_hhmm(545), "09:05");
    }

    #[test]
    fn days_hold_as_named() {
        assert!(Days::Weekdays.holds_on(1) && Days::Weekdays.holds_on(5));
        assert!(!Days::Weekdays.holds_on(0) && !Days::Weekdays.holds_on(6));
        assert!(Days::Weekends.holds_on(0) && Days::Weekends.holds_on(6));
        assert!(!Days::Weekends.holds_on(3));
    }

    #[test]
    fn a_start_catches_up_on_the_latest_rule_passed_today() {
        let rules = [
            rule("Work", "09:00", Days::Weekdays),
            rule("Lunch", "12:00", Days::Every),
            rule("Evening", "18:00", Days::Every),
        ];
        let mut s = Schedule::default();
        assert_eq!(s.due(at(2, "13:30"), &rules).as_deref(), Some("Lunch"));
        // Then nothing until the next rule's time.
        assert_eq!(s.due(at(2, "13:31"), &rules), None);
        assert_eq!(s.due(at(2, "17:59"), &rules), None);
        assert_eq!(s.due(at(2, "18:00"), &rules).as_deref(), Some("Evening"));
        // A Saturday morning: Work does not hold.
        let mut s = Schedule::default();
        assert_eq!(s.due(at(6, "10:00"), &rules), None);
    }

    #[test]
    fn a_rule_fires_once_when_crossed_even_after_a_gap() {
        let rules = [rule("Work", "09:00", Days::Every)];
        let mut s = Schedule::default();
        s.due(at(1, "08:00"), &rules);
        // Not looked at for a while (edit mode): fires when looked at.
        assert_eq!(s.due(at(1, "09:40"), &rules).as_deref(), Some("Work"));
        assert_eq!(s.due(at(1, "09:41"), &rules), None);
    }

    #[test]
    fn midnight_is_crossed_too() {
        let rules = [
            rule("Late", "23:30", Days::Every),
            rule("Early", "00:10", Days::Every),
        ];
        let mut s = Schedule::default();
        s.due(at(1, "23:00"), &rules);
        assert_eq!(
            s.due(at(2, "00:20"), &rules).as_deref(),
            Some("Early"),
            "the latest of the two crossed"
        );
    }

    #[test]
    fn switched_off_and_malformed_rules_never_fire() {
        let mut off = rule("Off", "09:00", Days::Every);
        off.enabled = false;
        let bad = rule("Bad", "9 o'clock", Days::Every);
        let mut s = Schedule::default();
        assert_eq!(s.due(at(1, "10:00"), &[off, bad]), None);
    }

    #[test]
    fn the_local_clock_is_readable() {
        let now = local_now().expect("a local time");
        assert!(now.weekday <= 6 && now.minute < 24 * 60);
    }
}
