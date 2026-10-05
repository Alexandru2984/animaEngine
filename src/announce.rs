//! What the characters say, for screen readers (1.5): every line a
//! character starts saying — its own, a reminder, the focus timer, a
//! greeting, a script's `say` — is announced through the accessibility
//! bridge, so Orca reads it out (`ui::speech::announce`). The Zzz… of a
//! sleeping scene is not news and is left out.
//!
//! Lines are said from wherever the scene is ticked, and announced from
//! the panel's egui pass, so they wait here in between: one process-wide
//! slot holding the latest, numbered, since a screen reader is told of a
//! line by a node it has not seen before. With the bridge off nothing
//! reads the slot, which only ever holds one.

use std::sync::Mutex;

/// The latest line: its number, and who said what.
static LATEST: Mutex<Option<(u64, String)>> = Mutex::new(None);

/// `name` started saying `text`.
pub fn said(name: &str, text: &str) {
    if let Ok(mut latest) = LATEST.lock() {
        let n = latest.as_ref().map_or(1, |(n, _)| n + 1);
        *latest = Some((n, format!("{name}: {text}")));
    }
}

/// The latest line, and its number.
pub fn latest() -> Option<(u64, String)> {
    LATEST.lock().ok().and_then(|l| l.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_line_is_new_even_when_the_words_are_not() {
        said("Cat", "Time for a break!");
        let (first, text) = latest().unwrap();
        assert_eq!(text, "Cat: Time for a break!");
        said("Cat", "Time for a break!");
        let (second, _) = latest().unwrap();
        assert!(second > first, "a new number, so it is read again");
    }
}
