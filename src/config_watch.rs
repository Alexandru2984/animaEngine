//! Watch `config.toml` and rebuild the scene off the UI thread.
//!
//! The winit path has done this since early on, through
//! `App::check_hot_reload`. The native Wayland loop never did — there is
//! not one reference to hot-reload anywhere under `src/wayland/` — while
//! the README lists the feature plainly and its backend-parity table
//! calls it `stable` on both. Editing the config with the overlay running
//! on Wayland simply did nothing.
//!
//! Nothing about reloading is Wayland- or X11-specific: read the file,
//! build a `Scene`, hand it back. So the mechanism lives here, owned by
//! neither backend, rather than being copied a second time into a loop
//! that already duplicates enough of the other one.
//!
//! Deliberately *not* retrofitted onto the winit path in the same change.
//! That path works today and its version is entangled with `App`'s
//! warning banners and toasts; swapping it out is a refactor of something
//! healthy, which is a different risk from giving a second backend a
//! feature it never had.

use crate::config::AppConfig;
use crate::scene::Scene;
use std::sync::mpsc;
use std::time::{Instant, SystemTime};

/// How often the file is stat-ed. One `stat` every couple of seconds is
/// far below the cost of the frame it runs in.
const CHECK_INTERVAL_SECS: u64 = 2;

/// A finished reload, ready for the caller to install.
pub struct Reloaded {
    pub config: AppConfig,
    pub scene: Scene,
}

/// What `ConfigWatcher::poll` found this frame.
pub enum Poll {
    /// Nothing to do — no change, or a worker still running.
    Idle,
    /// A reload finished and may be applied.
    Ready(Box<Reloaded>),
    /// The file changed but could not be read or parsed. Nothing was
    /// written and the running scene is untouched; the user wants to know
    /// their edit has not taken.
    Failed(String),
    /// A reload finished, but the scene was edited while it ran, so it was
    /// dropped rather than overwriting that edit.
    Discarded,
    /// The worker vanished without answering.
    WorkerLost,
}

pub struct ConfigWatcher {
    last_check: Instant,
    mtime: Option<SystemTime>,
    rx: Option<mpsc::Receiver<Result<Reloaded, String>>>,
}

impl Default for ConfigWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigWatcher {
    pub fn new() -> Self {
        Self {
            last_check: Instant::now(),
            mtime: current_mtime(),
            rx: None,
        }
    }

    /// Call after writing the config ourselves.
    ///
    /// Without it our own save looks exactly like somebody else's edit and
    /// the next poll reloads the file we just wrote — throwing away the
    /// selection and every texture for no reason.
    pub fn note_saved(&mut self) {
        self.mtime = current_mtime();
    }

    /// Drive one step: collect a finished worker, then decide whether to
    /// start one.
    ///
    /// `config_dirty` is the caller's unsaved-edit flag, and it is checked
    /// *twice* on purpose — once before starting, and again when the
    /// result arrives. The worker decodes every asset in the scene, so it
    /// is not instant, and an edit made while it ran used to be silently
    /// overwritten on the winit path.
    pub fn poll(&mut self, config_dirty: bool) -> Poll {
        if let Some(rx) = &self.rx {
            match rx.try_recv() {
                Ok(Ok(result)) => {
                    self.rx = None;
                    if config_dirty {
                        return Poll::Discarded;
                    }
                    return Poll::Ready(Box::new(result));
                }
                Ok(Err(reason)) => {
                    self.rx = None;
                    return Poll::Failed(reason);
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.rx = None;
                    return Poll::WorkerLost;
                }
                Err(mpsc::TryRecvError::Empty) => return Poll::Idle,
            }
        }

        if !should_check(self.last_check.elapsed().as_secs(), config_dirty) {
            return Poll::Idle;
        }
        self.last_check = Instant::now();

        let new_mtime = current_mtime();
        if new_mtime == self.mtime {
            return Poll::Idle;
        }
        // Recorded before the worker runs, so a file edited twice in quick
        // succession is not re-read forever.
        self.mtime = new_mtime;
        tracing::info!("Config file changed externally, spawning reload worker…");

        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("anima-hot-reload".into())
            .spawn(move || {
                // Read-only: unlike startup `load`, a bad or partially
                // written config must never make this worker rewrite the
                // user's file with defaults.
                let result = match AppConfig::try_reload() {
                    Ok(config) => {
                        let scene = Scene::from_config(&config);
                        Ok(Reloaded { config, scene })
                    }
                    Err(e) => Err(e),
                };
                // Receiver dropped (app exiting) → ignore the send error.
                let _ = tx.send(result);
            });
        match spawned {
            Ok(_) => self.rx = Some(rx),
            Err(e) => tracing::warn!("Hot-reload worker failed to spawn: {e}"),
        }
        Poll::Idle
    }
}

fn current_mtime() -> Option<SystemTime> {
    std::fs::metadata(AppConfig::config_path())
        .ok()?
        .modified()
        .ok()
}

/// Whether this frame should stat the config file.
///
/// Split out because it is the whole throttling policy and needs no
/// filesystem: a stat every frame would be sixty syscalls a second for a
/// file that changes when a human saves it, and there is no point looking
/// at all while the scene has unsaved edits, because a change found then
/// cannot be applied anyway.
fn should_check(since_last_check_secs: u64, config_dirty: bool) -> bool {
    !config_dirty && since_last_check_secs >= CHECK_INTERVAL_SECS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_are_throttled_to_the_interval() {
        assert!(!should_check(0, false));
        assert!(!should_check(CHECK_INTERVAL_SECS - 1, false));
        assert!(should_check(CHECK_INTERVAL_SECS, false));
        assert!(should_check(CHECK_INTERVAL_SECS + 10, false));
    }

    /// A dirty scene cannot accept a reload, so there is no reason to go
    /// looking for one.
    #[test]
    fn a_dirty_scene_is_never_checked() {
        assert!(!should_check(CHECK_INTERVAL_SECS, true));
        assert!(!should_check(u64::MAX, true));
    }

    /// A fresh watcher must not fire immediately: it records the file's
    /// mtime at construction, so the config the app just loaded does not
    /// count as a change.
    #[test]
    fn a_new_watcher_starts_idle() {
        let mut w = ConfigWatcher::new();
        assert!(matches!(w.poll(false), Poll::Idle));
    }
}
