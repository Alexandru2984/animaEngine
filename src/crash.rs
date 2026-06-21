//! Panic hook + crash-recovery snapshot.
//!
//! Lifecycle:
//!
//! 1. `install_panic_hook()` runs once at startup. It chains in front of
//!    whatever hook is currently installed (`std::panic::take_hook`) and
//!    dumps the last-known-good `AppConfig` to
//!    `~/.cache/animaEngine/crash-recovery.toml` before letting the
//!    default hook print the panic message and unwind.
//!
//! 2. Whenever the running scene reaches a clean, saveable state —
//!    `App::save_config_if_needed` immediately after a successful save —
//!    we call `record_known_good`. That keeps the in-memory snapshot
//!    matching what's on disk; if we panic later the snapshot is at
//!    worst the last good state.
//!
//! 3. On the next launch the user can pass `--recover` to restore the
//!    snapshot over the live config (the live config is backed up to
//!    `config.toml.bak` first). The snapshot is then deleted so a
//!    second `--recover` is a no-op.
//!
//! 4. The hook also writes a **crash report** —
//!    `~/.cache/animaEngine/crashes/crash-<ts>-<pid>.log` with version,
//!    panic message, location and backtrace. Launched from a desktop
//!    icon there is no terminal: without this file the panic text
//!    evaporates. The next launch shows a one-time toast pointing at
//!    the report (tracked via a `.last-notified` marker, newest five
//!    reports kept). Reports stay local — nothing is ever uploaded
//!    (zero-telemetry policy, docs/threat-model.md).

use crate::config::AppConfig;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// The single in-memory copy of the last config we saw clean. `OnceLock`
/// for free initialization, `Mutex` because the panic hook may fire from
/// any thread.
static LAST_GOOD_CONFIG: OnceLock<Mutex<Option<AppConfig>>> = OnceLock::new();

fn slot() -> &'static Mutex<Option<AppConfig>> {
    LAST_GOOD_CONFIG.get_or_init(|| Mutex::new(None))
}

/// Location of the crash-recovery snapshot on disk. `None` only when
/// XDG dirs are unavailable (extremely rare on a graphical Linux box).
pub fn recovery_path() -> Option<PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", "animaEngine")?;
    Some(dirs.cache_dir().join("crash-recovery.toml"))
}

/// Update the in-memory snapshot. Cheap (`AppConfig::clone`), safe to
/// call frequently.
pub fn record_known_good(config: &AppConfig) {
    if let Ok(mut guard) = slot().lock() {
        *guard = Some(config.clone());
    }
}

/// Chain a panic hook in front of the current one. The new hook writes
/// the crash report and the snapshot, then defers to the previous hook
/// so default behavior (printing the panic, backtrace, unwinding) is
/// preserved.
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!("Panic — writing crash report + recovery snapshot. {info}");
        match write_crash_report(info) {
            Ok(path) => tracing::error!("Crash report at {}", path.display()),
            Err(e) => tracing::error!("Crash report write failed: {e}"),
        }
        if let Err(e) = save_snapshot_to_disk() {
            tracing::error!("Crash snapshot save failed: {e}");
        }
        previous(info);
    }));
}

/// Directory holding crash reports + the notification marker.
pub fn reports_dir() -> Option<PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", "animaEngine")?;
    Some(dirs.cache_dir().join("crashes"))
}

/// Newest reports kept on disk; older ones are pruned at write time so
/// a crash loop can't fill the cache partition.
const KEEP_REPORTS: usize = 5;

/// Marker file remembering the newest report the user has been told
/// about. Plain filename content — comparison is lexicographic, which
/// works because report names embed a fixed-width unix timestamp.
const NOTIFIED_MARKER: &str = ".last-notified";

fn write_crash_report(info: &std::panic::PanicHookInfo<'_>) -> Result<PathBuf, String> {
    let dir = reports_dir().ok_or_else(|| "no XDG cache dir".to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("create crashes dir: {e}"))?;

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let message = panic_message(info);
    let location = info
        .location()
        .map(|l| l.to_string())
        .unwrap_or_else(|| "<unknown>".into());
    let thread = std::thread::current();
    let backtrace = std::backtrace::Backtrace::force_capture();
    let report = format_report(
        &message,
        &location,
        thread.name().unwrap_or("<unnamed>"),
        &backtrace.to_string(),
        ts,
    );

    let path = dir.join(format!("crash-{ts:010}-{}.log", std::process::id()));
    std::fs::write(&path, report).map_err(|e| format!("write report: {e}"))?;
    prune_reports(&dir, KEEP_REPORTS);
    Ok(path)
}

/// Panic payloads are almost always `&str` or `String`; anything else
/// gets a placeholder rather than a Debug dump of unknown size.
fn panic_message(info: &std::panic::PanicHookInfo<'_>) -> String {
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".into()
    }
}

fn format_report(message: &str, location: &str, thread: &str, backtrace: &str, ts: u64) -> String {
    format!(
        "animaEngine crash report\n\
         version:   {}\n\
         timestamp: {} (unix)\n\
         os:        {} / {}\n\
         thread:    {}\n\
         location:  {}\n\
         message:   {}\n\
         \n\
         backtrace:\n{}\n",
        env!("CARGO_PKG_VERSION"),
        ts,
        std::env::consts::OS,
        std::env::consts::ARCH,
        thread,
        location,
        message,
        backtrace,
    )
}

/// Delete all but the newest `keep` reports. Names sort by timestamp
/// because of the zero-padded `crash-<ts>-` prefix.
fn prune_reports(dir: &Path, keep: usize) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.starts_with("crash-") && n.ends_with(".log"))
        .collect();
    names.sort();
    if names.len() > keep {
        let surplus = names.len() - keep;
        for name in names.into_iter().take(surplus) {
            let _ = std::fs::remove_file(dir.join(name));
        }
    }
}

/// Report the user hasn't been notified about yet, if any. Called once
/// at startup; pair with [`mark_notified`] after showing the toast.
pub fn unnotified_report() -> Option<PathBuf> {
    let dir = reports_dir()?;
    let entries = std::fs::read_dir(&dir).ok()?;
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.starts_with("crash-") && n.ends_with(".log"))
        .collect();
    names.sort();
    let marker = std::fs::read_to_string(dir.join(NOTIFIED_MARKER)).ok();
    pick_unnotified(&names, marker.as_deref()).map(|name| dir.join(name))
}

/// Remember that the user has seen the toast for `report`.
pub fn mark_notified(report: &Path) {
    let Some(dir) = reports_dir() else { return };
    let Some(name) = report.file_name().and_then(|n| n.to_str()) else {
        return;
    };
    let _ = std::fs::write(dir.join(NOTIFIED_MARKER), name);
}

/// Pure core of [`unnotified_report`]: newest report name strictly
/// newer than the marker (lexicographic = chronological here).
fn pick_unnotified<'a>(sorted_names: &'a [String], marker: Option<&str>) -> Option<&'a String> {
    let newest = sorted_names.last()?;
    match marker {
        Some(seen) if newest.as_str() <= seen => None,
        _ => Some(newest),
    }
}

fn save_snapshot_to_disk() -> Result<(), String> {
    let path = recovery_path().ok_or_else(|| "no XDG cache dir".to_string())?;
    let guard = slot()
        .lock()
        .map_err(|e| format!("snapshot lock poisoned: {e}"))?;
    let Some(config) = guard.as_ref() else {
        // Nothing recorded yet — first-launch panic before any save.
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create cache dir: {e}"))?;
    }
    let toml = toml::to_string_pretty(config).map_err(|e| format!("serialize: {e}"))?;
    // Same atomic temp+rename helper every other stateful file on disk
    // uses — a second crash (or kill) mid-write must never leave a
    // truncated snapshot that `--recover` would later restore as-is.
    crate::util::atomic_write_bytes(&path, toml.as_bytes()).map_err(|e| format!("write: {e}"))?;
    tracing::error!("Crash snapshot saved to {}", path.display());
    Ok(())
}

/// Outcome of a `--recover` attempt — distinguishes "nothing to do"
/// from "found and restored" so the CLI can give honest feedback.
#[derive(Debug)]
pub enum RecoverOutcome {
    /// No snapshot file existed.
    NoSnapshot,
    /// Snapshot found, copied over `config.toml`. The previous live
    /// config is at `backup`.
    Restored { backup: Option<PathBuf> },
    /// Snapshot found but copying failed (permissions, disk full, …).
    Failed(String),
}

/// Apply the crash-recovery snapshot to the live config, backing up the
/// existing config to `config.toml.bak`. Deletes the snapshot on success.
pub fn try_recover() -> RecoverOutcome {
    let Some(snapshot) = recovery_path() else {
        return RecoverOutcome::NoSnapshot;
    };
    if !snapshot.exists() {
        return RecoverOutcome::NoSnapshot;
    }
    // Parse before touching anything on disk. A truncated or corrupt
    // snapshot (e.g. a second crash mid-write, pre-atomic-write fix)
    // must not be reported as "Restored" and then overwrite the live
    // config with garbage — fail closed and leave both files alone.
    match std::fs::read_to_string(&snapshot) {
        Ok(contents) => {
            if let Err(e) = validate_snapshot_contents(&contents) {
                return RecoverOutcome::Failed(format!("snapshot is not a valid config: {e}"));
            }
        }
        Err(e) => return RecoverOutcome::Failed(format!("read snapshot: {e}")),
    }
    let target = AppConfig::config_path();
    if let Some(parent) = target.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return RecoverOutcome::Failed(format!("create config dir: {e}"));
        }
    }

    // Back up the existing live config — even if it's the reason we
    // crashed, the user might still want to see it.
    let backup = if target.exists() {
        let path = target.with_extension("toml.bak");
        match std::fs::copy(&target, &path) {
            Ok(_) => Some(path),
            Err(e) => return RecoverOutcome::Failed(format!("backup live config: {e}")),
        }
    } else {
        None
    };

    if let Err(e) = std::fs::copy(&snapshot, &target) {
        return RecoverOutcome::Failed(format!("copy snapshot → config: {e}"));
    }
    let _ = std::fs::remove_file(&snapshot);
    RecoverOutcome::Restored { backup }
}

/// `try_recover`'s gate, split out so it's testable without touching
/// the real XDG paths `recovery_path`/`AppConfig::config_path` resolve
/// to.
fn validate_snapshot_contents(contents: &str) -> Result<(), toml::de::Error> {
    toml::from_str::<AppConfig>(contents).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_then_read_back() {
        let cfg = AppConfig::default();
        record_known_good(&cfg);
        let guard = slot().lock().unwrap();
        let stored = guard.as_ref().expect("snapshot must be recorded");
        assert_eq!(stored.characters.len(), cfg.characters.len());
    }

    #[test]
    fn validate_snapshot_accepts_real_config_round_trip() {
        let cfg = AppConfig::default();
        let toml = toml::to_string_pretty(&cfg).unwrap();
        assert!(validate_snapshot_contents(&toml).is_ok());
    }

    #[test]
    fn validate_snapshot_rejects_truncated_contents() {
        // Simulates a non-atomic write torn by a second crash mid-save.
        let cfg = AppConfig::default();
        let toml = toml::to_string_pretty(&cfg).unwrap();
        let truncated = &toml[..toml.len() / 2];
        assert!(validate_snapshot_contents(truncated).is_err());
    }

    #[test]
    fn report_contains_the_essentials() {
        let r = format_report("boom", "src/lib.rs:42:7", "main", "0: frame", 1_700_000_000);
        assert!(r.contains(env!("CARGO_PKG_VERSION")));
        assert!(r.contains("boom"));
        assert!(r.contains("src/lib.rs:42:7"));
        assert!(r.contains("0: frame"));
    }

    #[test]
    fn prune_keeps_only_newest() {
        let dir = std::env::temp_dir().join(format!("anima_crash_prune_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for ts in 1..=8u64 {
            std::fs::write(dir.join(format!("crash-{ts:010}-1.log")), "x").unwrap();
        }
        // A marker and a stray file must survive untouched.
        std::fs::write(dir.join(NOTIFIED_MARKER), "crash-0000000003-1.log").unwrap();
        prune_reports(&dir, 5);
        let mut left: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        left.sort();
        assert_eq!(
            left,
            vec![
                NOTIFIED_MARKER.to_string(),
                "crash-0000000004-1.log".into(),
                "crash-0000000005-1.log".into(),
                "crash-0000000006-1.log".into(),
                "crash-0000000007-1.log".into(),
                "crash-0000000008-1.log".into(),
            ]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pick_unnotified_logic() {
        let names = vec![
            "crash-0000000001-1.log".to_string(),
            "crash-0000000002-1.log".to_string(),
        ];
        // No marker → newest.
        assert_eq!(
            pick_unnotified(&names, None),
            Some(&"crash-0000000002-1.log".to_string())
        );
        // Marker at newest → nothing new.
        assert_eq!(
            pick_unnotified(&names, Some("crash-0000000002-1.log")),
            None
        );
        // Marker older → newest again.
        assert_eq!(
            pick_unnotified(&names, Some("crash-0000000001-1.log")),
            Some(&"crash-0000000002-1.log".to_string())
        );
        // Empty list → nothing, regardless of marker.
        assert_eq!(pick_unnotified(&[], None), None);
    }

    #[test]
    fn recover_outcome_when_no_snapshot() {
        // We can't reliably test the on-disk path without mucking with
        // $HOME, but the no-snapshot branch is hit when the file is
        // absent. Other paths exercise file IO and are covered by the
        // smoke install test (tests/demo_generation.rs).
        let path = recovery_path();
        if let Some(p) = path {
            if !p.exists() {
                assert!(matches!(try_recover(), RecoverOutcome::NoSnapshot));
            }
        }
    }
}
