//! Stepping aside for apps the user names (1.5): while one of them is the
//! window in front — a video call, a stream, a game — the characters hide,
//! as they do for a full-screen app (`crate::fullscreen`).
//!
//! Apps go by their id: `WM_CLASS` on X11, the `app_id` on native Wayland,
//! compared without case. Each backend finds the app in front — the X11
//! full-screen watch, the Wayland toplevel list — and the Appearance tab
//! offers the apps open now to pick from ([`open`]), so nobody has to know
//! an id. Ids are held in memory for that and nothing else: never logged,
//! saved (but the ones the user picks) or sent anywhere. Window titles are
//! not read at all.
//!
//! Like the full-screen rule, X11 sees only X11 clients: under a Wayland
//! session running the X11 path (GNOME, KDE), apps that run on Wayland
//! natively are not seen.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Whether `app` is one of `chosen` — without case, and ignoring empties.
pub fn is_chosen(chosen: &[String], app: &str) -> bool {
    let app = app.trim();
    !app.is_empty() && chosen.iter().any(|c| c.trim().eq_ignore_ascii_case(app))
}

/// The chosen ids, for a backend's watch thread (the loops set them).
static CHOSEN: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// The apps open now, for the picker — filled while it is wanted.
static OPEN: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// When the picker last asked for [`OPEN`], in seconds since the epoch.
static WANTED_AT: AtomicU64 = AtomicU64::new(0);

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// The loops hand over the setting, every frame. Cheap.
pub fn configure(chosen: &[String]) {
    if let Ok(mut held) = CHOSEN.lock() {
        if held.as_slice() != chosen {
            *held = chosen.to_vec();
        }
    }
}

/// Whether `app` is chosen, by the setting handed over last.
pub fn chosen_now(app: &str) -> bool {
    CHOSEN.lock().is_ok_and(|c| is_chosen(&c, app))
}

/// The picker is showing: keep [`open`] fresh for a couple of seconds.
pub fn want_open_list() {
    WANTED_AT.store(now_secs(), Ordering::Relaxed);
}

/// Whether the open apps are wanted: the picker showed in the last few
/// seconds. Listing them costs a round-trip per window on X11.
pub fn open_list_wanted() -> bool {
    now_secs().saturating_sub(WANTED_AT.load(Ordering::Relaxed)) <= 3
}

/// The apps open now, sorted, each once; ours left out.
pub fn set_open(mut apps: Vec<String>) {
    apps.retain(|a| {
        let a = a.trim();
        !a.is_empty()
            && !a.eq_ignore_ascii_case("animaengine")
            && !a.eq_ignore_ascii_case("anima_engine")
    });
    apps.sort_by_key(|a| a.to_lowercase());
    apps.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    if let Ok(mut held) = OPEN.lock() {
        *held = apps;
    }
}

/// The apps open now, as last listed.
pub fn open() -> Vec<String> {
    OPEN.lock().map(|o| o.clone()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apps_match_without_case() {
        let chosen = vec!["zoom".to_string(), " OBS ".to_string()];
        assert!(is_chosen(&chosen, "Zoom"));
        assert!(is_chosen(&chosen, "obs"));
        assert!(!is_chosen(&chosen, "zoom-helper"));
        assert!(!is_chosen(&chosen, ""));
        assert!(!is_chosen(&[], "zoom"));
    }

    #[test]
    fn the_open_list_is_tidy_and_leaves_us_out() {
        set_open(vec![
            "firefox".into(),
            "Zoom".into(),
            "zoom".into(),
            "animaEngine".into(),
            " ".into(),
            "Alacritty".into(),
        ]);
        assert_eq!(open(), ["Alacritty", "firefox", "Zoom"]);
    }
}
