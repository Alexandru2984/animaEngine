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
//! The winit path kept its own copy at first — it worked, and swapping a
//! healthy path is a different risk from giving a backend a feature it
//! never had. It now runs on this too, and [`handle`] is what both loops
//! do with a result, because the copies had already drifted: only winit
//! raised the banner when the worker died, and the toasts were English on
//! both.

use crate::config::AppConfig;
use crate::outcomes::OutcomeCtx;
use crate::scene::Scene;
use crate::ui::Warning;
use std::collections::{BTreeSet, HashSet};
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

/// Act on what [`ConfigWatcher::poll`] found — the same way on both
/// backends.
pub fn handle(
    poll: Poll,
    ctx: &mut OutcomeCtx<'_>,
    config: &mut AppConfig,
    warnings: &mut BTreeSet<Warning>,
) {
    match poll {
        Poll::Idle => {}
        Poll::Ready(result) => {
            install(*result, ctx, config);
            // A worker answered, so whatever made the last one vanish has
            // passed; the banner says it goes away when that happens.
            warnings.remove(&Warning::HotReloadDisconnected);
        }
        Poll::Discarded => {
            tracing::info!("Hot-reload discarded: the scene was edited while it was loading");
            ctx.toasts
                .warn(crate::i18n::t("toast-config-reload-discarded"));
            warnings.remove(&Warning::HotReloadDisconnected);
        }
        Poll::Failed(reason) => {
            // Nothing was written and the running scene is untouched —
            // `try_reload` never touches the file — but the user needs to
            // know their edit has not taken.
            tracing::warn!("Hot-reload skipped: {reason}; keeping current scene");
            ctx.toasts
                .warn(crate::i18n::t("toast-config-reload-failed"));
        }
        Poll::WorkerLost => {
            tracing::warn!("Hot-reload worker disconnected unexpectedly");
            // Without the banner the edit just silently does not apply,
            // and the user assumes the save took.
            warnings.insert(Warning::HotReloadDisconnected);
        }
    }
}

/// Swap in a finished reload.
///
/// Textures are diffed by entity id, so characters that survive the
/// reload keep their GPU memory instead of being re-uploaded, and ones
/// that are gone do not stay resident.
fn install(result: Reloaded, ctx: &mut OutcomeCtx<'_>, config: &mut AppConfig) {
    if let Some(renderer) = ctx.renderer.as_deref_mut() {
        let new_ids: HashSet<&str> = result
            .scene
            .entities
            .iter()
            .map(|e| e.id.as_str())
            .collect();
        renderer
            .shared
            .textures
            .retain(|id, _| new_ids.contains(id.as_str()));
    }

    *config = result.config;
    *ctx.scene = result.scene;
    // The reloaded scene is a different list; an index into the old one
    // means nothing against it.
    ctx.selection.deselect();

    // `ensure_texture` creates, updates in place (same size) or recreates.
    if let Some(renderer) = ctx.renderer.as_deref_mut() {
        for entity in &mut ctx.scene.entities {
            renderer.ensure_texture(entity);
            entity.texture_dirty = false;
        }
    }

    tracing::info!("Hot-reload applied: {} entities", ctx.scene.entities.len());
    ctx.toasts.info(crate::i18n::t("toast-config-reloaded"));
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

    fn watcher_with(result: Result<Reloaded, String>) -> ConfigWatcher {
        let (tx, rx) = mpsc::channel();
        tx.send(result).unwrap();
        ConfigWatcher {
            last_check: Instant::now(),
            mtime: None,
            rx: Some(rx),
        }
    }

    fn reloaded() -> Reloaded {
        let config = AppConfig::default();
        let scene = Scene::from_config(&config);
        Reloaded { config, scene }
    }

    #[test]
    fn a_clean_scene_accepts_a_finished_reload() {
        let mut w = watcher_with(Ok(reloaded()));
        assert!(matches!(w.poll(false), Poll::Ready(_)));
    }

    /// The case that lost work: the reload started while the scene was
    /// clean, the user nudged something while it ran, and the result
    /// landed on top of the edit.
    #[test]
    fn an_edit_made_while_loading_wins() {
        let mut w = watcher_with(Ok(reloaded()));
        assert!(matches!(w.poll(true), Poll::Discarded));
    }

    #[test]
    fn a_worker_that_vanishes_is_reported() {
        let (tx, rx) = mpsc::channel::<Result<Reloaded, String>>();
        drop(tx);
        let mut w = ConfigWatcher {
            last_check: Instant::now(),
            mtime: None,
            rx: Some(rx),
        };
        assert!(matches!(w.poll(false), Poll::WorkerLost));
    }

    /// The banner goes up when the worker is lost and comes down with the
    /// next answer — on both backends now; Wayland never raised it.
    #[test]
    fn the_disconnect_banner_follows_the_worker() {
        let mut scene = Scene::from_config(&AppConfig::default());
        let mut selection = crate::input::selection::SelectionState::default();
        let mut toasts = crate::ui::ToastQueue::default();
        let mut dirty = false;
        let mut config = AppConfig::default();
        let mut warnings = BTreeSet::new();
        let mut ctx = OutcomeCtx {
            scene: &mut scene,
            selection: &mut selection,
            toasts: &mut toasts,
            config_dirty: &mut dirty,
            renderer: None,
        };
        handle(Poll::WorkerLost, &mut ctx, &mut config, &mut warnings);
        assert!(warnings.contains(&Warning::HotReloadDisconnected));
        handle(
            Poll::Ready(Box::new(reloaded())),
            &mut ctx,
            &mut config,
            &mut warnings,
        );
        assert!(!warnings.contains(&Warning::HotReloadDisconnected));
    }

    /// A reload replaces the scene wholesale, so the selection cannot
    /// survive it.
    #[test]
    fn a_reload_clears_the_selection() {
        let mut scene = Scene::from_config(&AppConfig::default());
        let mut selection = crate::input::selection::SelectionState::default();
        selection.select(0);
        let mut toasts = crate::ui::ToastQueue::default();
        let mut dirty = false;
        let mut config = AppConfig::default();
        let mut ctx = OutcomeCtx {
            scene: &mut scene,
            selection: &mut selection,
            toasts: &mut toasts,
            config_dirty: &mut dirty,
            renderer: None,
        };
        handle(
            Poll::Ready(Box::new(reloaded())),
            &mut ctx,
            &mut config,
            &mut BTreeSet::new(),
        );
        assert_eq!(selection.selected_index(), None);
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
