//! Placeholders in what characters say (1.5): `{time}`, `{day}`, `{date}`
//! and `{name}` in a character's lines, in reminders and in a script's
//! `say` are filled in when it is said — "It's {time}, tea time!" — the
//! day and the date in the user's language, `{name}` the speaker's.
//!
//! Only those four, spelled so; anything else in braces is left as
//! written. Without a clock to read, the time, day and date are left as
//! written too.

use crate::schedule::LocalTime;

/// `text` with its placeholders filled in, said by `name` at local `now`.
pub fn fill(text: &str, name: &str, now: Option<LocalTime>) -> String {
    if !text.contains('{') {
        return text.to_string();
    }
    let mut out = text.to_string();
    if let Some(now) = now {
        if out.contains("{time}") {
            out = out.replace("{time}", &crate::schedule::format_hhmm(now.minute));
        }
        if out.contains("{day}") {
            out = out.replace("{day}", &weekday(now.weekday));
        }
        if out.contains("{date}") {
            out = out.replace("{date}", &date(now.day, now.month));
        }
    }
    // Last: a name with braces in it is not filled in itself.
    out.replace("{name}", name)
}

/// `text` filled in at the local time now.
pub fn fill_now(text: &str, name: &str) -> String {
    if !text.contains('{') {
        return text.to_string();
    }
    fill(text, name, crate::schedule::local_now())
}

/// The days of the week, Sunday first, and the months.
const WEEKDAYS: [&str; 7] = [
    "weekday-0",
    "weekday-1",
    "weekday-2",
    "weekday-3",
    "weekday-4",
    "weekday-5",
    "weekday-6",
];
const MONTHS: [&str; 12] = [
    "month-1", "month-2", "month-3", "month-4", "month-5", "month-6", "month-7", "month-8",
    "month-9", "month-10", "month-11", "month-12",
];

/// The name of the day of the week (0 = Sunday), in the user's language.
fn weekday(weekday: u8) -> String {
    crate::i18n::t(WEEKDAYS[usize::from(weekday.min(6))])
}

/// The day and month (1–12), as the user's language writes them.
fn date(day: u8, month: u8) -> String {
    let mut args = fluent::FluentArgs::new();
    args.set("day", day);
    args.set(
        "month",
        crate::i18n::t(MONTHS[usize::from(month.clamp(1, 12) - 1)]),
    );
    crate::i18n::t_args("placeholder-date", &args)
}

/// What the Inspector and the reminders say about the four.
pub fn hint() -> String {
    let mut args = fluent::FluentArgs::new();
    for token in ["time", "day", "date", "name"] {
        args.set(token, format!("{{{token}}}"));
    }
    crate::i18n::t_args("placeholders-hint", &args)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(minute: u32) -> Option<LocalTime> {
        Some(LocalTime {
            weekday: 1,
            minute,
            day: 5,
            month: 10,
        })
    }

    #[test]
    fn the_four_are_filled_in() {
        let said = fill(
            "{name}: it's {time} on {day}, {date}.",
            "Cat",
            at(14 * 60 + 5),
        );
        assert!(said.starts_with("Cat: it's 14:05 on "), "{said}");
        assert!(!said.contains('{'), "{said}");
        assert!(said.contains('5'), "the date: {said}");
    }

    #[test]
    fn anything_else_is_left_as_written() {
        assert_eq!(fill("{nope} and {Time}", "Cat", at(0)), "{nope} and {Time}");
        assert_eq!(fill("no braces", "Cat", None), "no braces");
        assert_eq!(fill("at {time}", "Cat", None), "at {time}", "no clock");
        assert_eq!(fill("I'm {name}", "Cat", None), "I'm Cat");
        assert_eq!(fill("I'm {name}", "{time}", at(0)), "I'm {time}");
    }
}
