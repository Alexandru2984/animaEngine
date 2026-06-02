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

use crate::config::AppConfig;
use std::path::PathBuf;
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
/// the snapshot, then defers to the previous hook so default behavior
/// (printing the panic, backtrace, unwinding) is preserved.
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!("Panic — attempting crash-recovery snapshot. {info}");
        if let Err(e) = save_snapshot_to_disk() {
            tracing::error!("Crash snapshot save failed: {e}");
        }
        previous(info);
    }));
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
    std::fs::write(&path, toml).map_err(|e| format!("write: {e}"))?;
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
