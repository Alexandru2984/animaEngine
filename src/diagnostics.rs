//! The diagnostic report Appearance → About copies to the clipboard, for a
//! bug report: the version, the system and session, what the log said
//! about the graphics card, the backend and the shortcuts, and the last
//! warnings and errors. Nothing leaves the machine — the user pastes it
//! where they ask for help — and nothing in it names their files: the
//! log already gives a file's name without its folder at this level, and
//! the home directory is written `~` besides.
//!
//! What it knows comes from the log itself, through [`Layer`]: a line
//! that starts as one of `FACTS` is kept, the latest of each, and every
//! warning and error, the last `PROBLEMS` (30) of them. So no subsystem
//! has to hand its state to the panel.

use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;
use std::time::Instant;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};

/// The info lines kept, by how they start: the latest of each.
const FACTS: &[&str] = &[
    "Event loop created",
    "Native Wayland renderer initialized",
    "GPU adapter:",
    "Backend:",
    "Presenting with",
    "Using alpha mode",
    "Overlay drawn at scale",
    "Pictures kept sharp",
    "Hotkey strategy",
    "Portal shortcuts",
    "Global shortcuts",
    "Idle time from",
    "System tray",
    "Scene loaded with",
];

/// How many of the latest warnings and errors are kept.
const PROBLEMS: usize = 30;

/// The longest a kept line may be.
const LINE: usize = 300;

struct Kept {
    started: Instant,
    facts: BTreeMap<usize, String>,
    problems: VecDeque<String>,
}

static KEPT: Mutex<Option<Kept>> = Mutex::new(None);

/// Keeps what the report needs from the log. Installed beside the
/// formatter in `main`.
pub struct Layer;

impl<S: Subscriber> tracing_subscriber::Layer<S> for Layer {
    fn on_event(&self, event: &Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        let level = *event.metadata().level();
        if level > Level::INFO {
            return;
        }
        let mut message = Message(String::new());
        event.record(&mut message);
        let text = message.0;
        let fact = (level == Level::INFO)
            .then(|| FACTS.iter().position(|f| text.starts_with(f)))
            .flatten();
        if fact.is_none() && level == Level::INFO {
            return;
        }
        let Ok(mut kept) = KEPT.lock() else {
            return;
        };
        let kept = kept.get_or_insert_with(|| Kept {
            started: Instant::now(),
            facts: BTreeMap::new(),
            problems: VecDeque::new(),
        });
        let text = clip(&text);
        match fact {
            Some(index) => {
                kept.facts.insert(index, text);
            }
            None => {
                let at = kept.started.elapsed().as_secs_f32();
                if kept.problems.len() == PROBLEMS {
                    kept.problems.pop_front();
                }
                kept.problems
                    .push_back(format!("[{at:7.1} s] {level:5} {text}"));
            }
        }
    }
}

struct Message(String);

impl Visit for Message {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = format!("{value:?}");
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.0 = value.to_string();
        }
    }
}

/// One line, at most [`LINE`] characters.
fn clip(text: &str) -> String {
    let line = text.lines().next().unwrap_or_default();
    match line.char_indices().nth(LINE) {
        Some((cut, _)) => format!("{}…", &line[..cut]),
        None => line.to_string(),
    }
}

/// `text` with the home directory written `~`.
fn private(text: &str) -> String {
    match std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        Ok(home) if home.len() > 1 => text.replace(&home, "~"),
        _ => text.to_string(),
    }
}

/// The system's name, as its os-release says on Linux and the BSDs.
fn system() -> String {
    let pretty = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|release| {
            release.lines().find_map(|l| {
                l.strip_prefix("PRETTY_NAME=")
                    .map(|v| v.trim_matches('"').to_string())
            })
        });
    let family = format!("{}, {}", std::env::consts::OS, std::env::consts::ARCH);
    match pretty {
        Some(name) => format!("{name} ({family})"),
        None => family,
    }
}

/// The report: plain text, a few dozen lines, for the user to paste.
pub fn report() -> String {
    let mut out = String::from("animaEngine diagnostic report\n\n");
    let mut line = |label: &str, value: String| {
        out.push_str(&format!("{label}: {value}\n"));
    };
    line("Version", env!("CARGO_PKG_VERSION").to_string());
    line("System", system());
    let env = |name: &str| std::env::var(name).unwrap_or_else(|_| "-".into());
    line(
        "Session",
        format!(
            "{}, desktop {}",
            env("XDG_SESSION_TYPE"),
            env("XDG_CURRENT_DESKTOP")
        ),
    );
    let packaging = if crate::autostart::in_flatpak() {
        "Flatpak"
    } else if std::env::var_os("APPIMAGE").is_some() {
        "AppImage"
    } else {
        "native"
    };
    line("Installed as", packaging.to_string());
    if std::env::var("ANIMA_USE_WAYLAND_NATIVE").as_deref() == Ok("1") {
        line("Native Wayland", "asked for".to_string());
    }
    line("Language", crate::i18n::current_locale());
    let kept = KEPT.lock().ok();
    let kept = kept.as_ref().and_then(|k| k.as_ref());
    if let Some(kept) = kept {
        let minutes = kept.started.elapsed().as_secs() / 60;
        out.push_str(&format!(
            "Running for: {}h {:02}m\n",
            minutes / 60,
            minutes % 60
        ));
        out.push_str("\nFrom the log:\n");
        for fact in kept.facts.values() {
            out.push_str(&format!("  {fact}\n"));
        }
        out.push_str(&format!(
            "\nLast warnings and errors ({}):\n",
            kept.problems.len()
        ));
        for problem in &kept.problems {
            out.push_str(&format!("  {problem}\n"));
        }
    }
    private(&out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::prelude::*;

    /// What the log said reaches the report — a fact kept as its latest, a
    /// warning in order — and an info line that is neither does not; the
    /// home directory is `~`.
    #[test]
    fn the_report_keeps_the_facts_and_the_problems() {
        let subscriber = tracing_subscriber::registry().with(Layer);
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/someone".into());
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!("GPU adapter: Old");
            tracing::info!("GPU adapter: Test GPU");
            tracing::info!("Loaded entity 'secret name'");
            tracing::warn!("Asset cache corrupt at {home}/pictures/a.png");
        });
        let report = report();
        assert!(report.contains("Version: "), "{report}");
        assert!(report.contains("  GPU adapter: Test GPU"), "{report}");
        assert!(!report.contains("GPU adapter: Old"), "{report}");
        assert!(!report.contains("secret name"), "{report}");
        assert!(
            report.contains("WARN  Asset cache corrupt at ~/pictures/a.png"),
            "{report}"
        );
        assert!(!report.contains(&home) || home.len() <= 1, "{report}");
    }

    #[test]
    fn a_long_line_is_cut() {
        assert_eq!(clip(&"x".repeat(LINE + 10)).chars().count(), LINE + 1);
        assert_eq!(clip("one\ntwo"), "one");
    }
}
