//! Off-thread config reload pipeline. Extracted in H.3 so the
//! main module's borrow-checker dance doesn't have to share a file
//! with the worker thread plumbing.
//!
//! Two-phase per tick:
//!
//! 1. Drain a finished worker (non-blocking `try_recv`) and apply
//!    the new config + scene through `apply_hot_reload`.
//! 2. If 2 s have passed since the last `stat` and the config file's
//!    mtime moved, spawn a background thread to re-decode the config
//!    + assets. The UI thread never blocks on asset IO.

use super::{App, HotReloadResult};
use crate::config::AppConfig;
use crate::scene::Scene;
use crate::ui::Warning;
use std::collections::HashSet;
use std::sync::mpsc;
use std::time::{Instant, SystemTime};

impl App {
    /// Get the modification time of the config file.
    pub(super) fn get_config_mtime() -> Option<SystemTime> {
        let path = AppConfig::config_path();
        std::fs::metadata(&path).ok()?.modified().ok()
    }

    /// Drive the hot-reload pipeline:
    /// 1. Apply any result already produced by a worker (non-blocking).
    /// 2. If the config file changed on disk, spawn a worker to load it.
    #[tracing::instrument(skip(self))]
    pub(super) fn check_hot_reload(&mut self) {
        // Phase 1: drain a finished worker, if any.
        if let Some(rx) = &self.hot_reload_rx {
            match rx.try_recv() {
                Ok(Ok(result)) => {
                    self.apply_hot_reload(result);
                    self.hot_reload_rx = None;
                }
                Ok(Err(reason)) => {
                    // The on-disk config couldn't be read/parsed/decoded
                    // (a partial write mid-save, or a hand-edit typo).
                    // try_reload never touches the file, so nothing was
                    // lost — keep the running scene and tell the user
                    // their edit hasn't taken yet.
                    tracing::warn!("Hot-reload skipped: {reason}; keeping current scene");
                    self.toasts
                        .warn("Config not reloaded (invalid or mid-save); keeping current scene");
                    self.hot_reload_rx = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    tracing::warn!("Hot-reload worker disconnected unexpectedly");
                    self.hot_reload_rx = None;
                    // Surface the silent crash to the user — without
                    // this banner the in-flight edit would just not
                    // apply and they'd assume the file save took.
                    self.warnings.insert(Warning::HotReloadDisconnected);
                }
                Err(mpsc::TryRecvError::Empty) => {} // still working
            }
        }

        // Phase 2: check if we should kick off a new worker. Cheap
        // syscall — OK to do every couple of seconds.
        if self.last_config_check.elapsed().as_secs() < 2 {
            return;
        }
        self.last_config_check = Instant::now();

        // Skip if there are unsaved local changes (we'd clobber them) or
        // a previous reload is still in flight.
        if self.config_dirty || self.hot_reload_rx.is_some() {
            return;
        }

        let new_mtime = Self::get_config_mtime();
        if new_mtime == self.config_mtime {
            return;
        }
        self.config_mtime = new_mtime;
        tracing::info!("Config file changed externally, spawning reload worker…");

        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("anima-hot-reload".into())
            .spawn(move || {
                // Read-only: unlike startup `load`, a bad or partially
                // written config must never make this worker rewrite the
                // user's file with defaults. On failure we forward the
                // reason and the UI keeps the current scene.
                let result = match AppConfig::try_reload() {
                    Ok(config) => {
                        let scene = Scene::from_config(&config);
                        Ok(HotReloadResult { config, scene })
                    }
                    Err(e) => Err(e),
                };
                // Receiver dropped (e.g. app exiting) → ignore send error.
                let _ = tx.send(result);
            });
        match spawned {
            Ok(_) => self.hot_reload_rx = Some(rx),
            Err(e) => tracing::warn!("Hot-reload worker failed to spawn: {e}"),
        }
    }

    /// Apply a finished hot-reload result on the UI thread.
    /// Diffs textures by entity ID so unchanged entities keep their GPU
    /// memory instead of being re-uploaded from scratch.
    fn apply_hot_reload(&mut self, result: HotReloadResult) {
        // Drop textures whose entity ID is no longer in the new scene.
        if let Some(renderer) = &mut self.renderer {
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

        self.config = result.config;
        self.scene = result.scene;
        self.selection.deselect();

        // For each entity: ensure_texture either creates new, updates in
        // place (same dimensions), or recreates (different dimensions).
        if let Some(renderer) = &mut self.renderer {
            for entity in &mut self.scene.entities {
                renderer.ensure_texture(entity);
                entity.texture_dirty = false;
            }
        }

        let n = self.scene.entities.len();
        tracing::info!("Hot-reload applied: {n} entities");
        self.toasts
            .info(format!("Reloaded {n} entities from config"));
    }
}
