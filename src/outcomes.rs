//! What the settings panels ask for, applied the same way on both backends.
//!
//! The context menu, the Library tab's "Add to scene" and the command
//! palette each hand the event loop an outcome to apply. Each backend used
//! to carry its own copy of the code that applies them, and the copies had
//! drifted the way every duplicated handler here eventually does (R19, R27,
//! R28, R30, R37):
//!
//! - applying a preset with *Replace* cleared the selection on winit but
//!   not on native Wayland, which kept an index into a scene that had just
//!   been rebuilt under it;
//! - the "preset loaded" toast was translated on Wayland and hard-coded
//!   English on winit;
//! - duplicating from the keyboard showed a toast on Wayland and none on
//!   winit, while duplicating from the menu showed one on both.
//!
//! What stays per-backend is only what genuinely differs: *when* the
//! config is written (the winit path saves at once after a delete or a
//! duplicate), and *where* a library asset lands (each backend knows its
//! own viewport). Both are the caller's, around these calls.

use std::path::Path;

use crate::asset_library::LibraryIndex;
use crate::config::AppConfig;
use crate::drop_validate::{pre_validate_dropped_file, redact_path, resolve_library_asset};
use crate::input::selection::SelectionState;
use crate::renderer::wgpu_renderer::WgpuRenderer;
use crate::scene::Scene;
use crate::ui::panels::{LibraryOutcome, MenuAction, PaletteOutcome};
use crate::ui::toasts::ToastQueue;

/// What applying an outcome is allowed to touch.
pub struct OutcomeCtx<'a> {
    pub scene: &'a mut Scene,
    pub selection: &'a mut SelectionState,
    pub toasts: &'a mut ToastQueue,
    pub config_dirty: &'a mut bool,
    /// The texture cache, keyed by entity id. `None` on the winit path
    /// before its first window exists — and in tests.
    pub renderer: Option<&'a mut WgpuRenderer>,
}

/// What an action on the character at `idx` covers: the whole selection
/// when `idx` is one of several selected — a right-click on any of them
/// acts on all — and `idx` alone otherwise.
fn covered(idx: usize, selection: &SelectionState) -> Vec<usize> {
    if selection.count() > 1 && selection.is_selected(idx) {
        selection.selected_indices()
    } else {
        vec![idx]
    }
}

/// Apply one context-menu action.
pub fn apply_menu_action(action: MenuAction, ctx: &mut OutcomeCtx<'_>) {
    match action {
        MenuAction::Duplicate(idx) => {
            let targets = covered(idx, ctx.selection);
            duplicate_entities(&targets, ctx);
        }
        MenuAction::Delete(idx) => {
            let targets = covered(idx, ctx.selection);
            delete_entities(&targets, ctx);
        }
        MenuAction::ResetTransform(idx) => {
            for i in covered(idx, ctx.selection) {
                if let Some(e) = ctx.scene.entities.get_mut(i) {
                    e.scale = 1.0;
                    e.opacity = 1.0;
                    *ctx.config_dirty = true;
                }
            }
        }
        MenuAction::ToggleGravity(idx) => {
            // The character clicked decides; the rest follow it.
            let Some(target) = ctx.scene.entities.get(idx).map(|e| !e.physics.enabled) else {
                return;
            };
            for i in covered(idx, ctx.selection) {
                if let Some(e) = ctx.scene.entities.get_mut(i) {
                    if target {
                        e.physics.enable();
                    } else {
                        e.physics.disable();
                    }
                    *ctx.config_dirty = true;
                }
            }
        }
        MenuAction::Group(idx) => {
            let targets = covered(idx, ctx.selection);
            *ctx.config_dirty |= group_entities(&targets, ctx.scene, ctx.toasts);
        }
        MenuAction::Ungroup(idx) => {
            let targets = covered(idx, ctx.selection);
            *ctx.config_dirty |= ungroup_entities(&targets, ctx.scene, ctx.toasts);
        }
        MenuAction::Arrange(idx, how) => {
            let targets = covered(idx, ctx.selection);
            *ctx.config_dirty |= crate::input::arrange::arrange(ctx.scene, &targets, how);
        }
        MenuAction::BringForward(idx) | MenuAction::SendBackward(idx) => {
            let step = if matches!(action, MenuAction::BringForward(_)) {
                10
            } else {
                -10
            };
            for i in covered(idx, ctx.selection) {
                if let Some(e) = ctx.scene.entities.get_mut(i) {
                    e.z_index += step;
                    *ctx.config_dirty = true;
                }
            }
            ctx.scene.mark_visible_dirty();
        }
    }
}

/// What the right-click menu on the character at `idx` offers about groups.
pub fn menu_offers(
    idx: usize,
    scene: &Scene,
    selection: &SelectionState,
) -> crate::ui::panels::MenuOffers {
    let targets = covered(idx, selection);
    crate::ui::panels::MenuOffers {
        count: targets.len(),
        group: targets.len() > 1 && !scene.already_a_group(&targets),
        ungroup: targets.iter().any(|&i| {
            scene
                .entities
                .get(i)
                .is_some_and(|e| crate::group::owning_group(&scene.groups, &e.id).is_some())
        }),
    }
}

/// Make a group of the characters at `targets`, numbered and named, and
/// say so. Returns whether one was made.
pub fn group_entities(targets: &[usize], scene: &mut Scene, toasts: &mut ToastQueue) -> bool {
    let name = scene.group_entities(targets, |number| {
        let mut args = fluent::FluentArgs::new();
        args.set("number", number);
        crate::i18n::t_args("group-default-name", &args)
    });
    let Some(name) = name else {
        return false;
    };
    let mut args = fluent::FluentArgs::new();
    args.set("name", name);
    toasts.info(crate::i18n::t_args("toast-grouped", &args));
    true
}

/// Dissolve the groups the characters at `targets` are in, and say so —
/// or that none is. Returns whether any went.
pub fn ungroup_entities(targets: &[usize], scene: &mut Scene, toasts: &mut ToastQueue) -> bool {
    let dissolved = scene.ungroup_entities(targets);
    if dissolved == 0 {
        toasts.info(crate::i18n::t("toast-nothing-to-ungroup"));
        return false;
    }
    let mut args = fluent::FluentArgs::new();
    args.set("count", dissolved);
    toasts.info(crate::i18n::t_args("toast-ungrouped", &args));
    true
}

/// Remove entity `idx`. Returns whether anything was removed.
///
/// The texture is dropped *before* the entity leaves the scene. Textures
/// are keyed by entity id, so a missed removal leaves an orphan resident
/// for the session — the leak `prune_stale_textures` exists to sweep up.
pub fn delete_entity(idx: usize, ctx: &mut OutcomeCtx<'_>) -> bool {
    delete_entities(&[idx], ctx) == 1
}

/// Remove the entities at `indices`, with one toast for them all.
/// Returns how many were removed. Highest index first, so the ones still
/// to go keep theirs.
pub fn delete_entities(indices: &[usize], ctx: &mut OutcomeCtx<'_>) -> usize {
    let mut order = indices.to_vec();
    order.sort_unstable();
    order.dedup();
    let mut names = Vec::new();
    for &idx in order.iter().rev() {
        let Some(entity) = ctx.scene.entities.get(idx) else {
            continue;
        };
        let name = entity.name.clone();
        if let Some(renderer) = ctx.renderer.as_deref_mut() {
            renderer.shared.textures.remove(&entity.id);
        }
        if let Some(removed_id) = ctx.scene.remove_entity(idx) {
            tracing::info!("Deleted entity: {removed_id}");
            names.push(name);
        }
    }
    if names.is_empty() {
        return 0;
    }
    ctx.selection.deselect();
    *ctx.config_dirty = true;
    let mut args = fluent::FluentArgs::new();
    let message = if let [name] = names.as_slice() {
        args.set("name", name.clone());
        crate::i18n::t_args("toast-deleted", &args)
    } else {
        args.set("count", names.len());
        crate::i18n::t_args("toast-deleted-many", &args)
    };
    ctx.toasts.info(message);
    names.len()
}

/// Copy entity `idx` 30 px down and right, keeping its scale and opacity,
/// and select the copy. Returns the copy's index.
pub fn duplicate_entity(idx: usize, ctx: &mut OutcomeCtx<'_>) -> Option<usize> {
    duplicate_entities(&[idx], ctx).first().copied()
}

/// Copy the entities at `indices`, each 30 px down and right of its
/// original, and select the copies — the primary's copy as the primary.
/// One toast for them all. Returns the copies' indices; the originals'
/// are unchanged, since copies are appended.
pub fn duplicate_entities(indices: &[usize], ctx: &mut OutcomeCtx<'_>) -> Vec<usize> {
    let mut copies = Vec::new();
    let mut names = Vec::new();
    for &idx in indices {
        match copy_entity(idx, ctx) {
            Some(Ok((new_idx, name))) => {
                copies.push(new_idx);
                names.push(name);
            }
            Some(Err(e)) => {
                tracing::error!("Duplicate failed: {e}");
                let mut args = fluent::FluentArgs::new();
                args.set("error", e);
                ctx.toasts
                    .error(crate::i18n::t_args("toast-duplicate-failed", &args));
                // One toast, not one per character: what stopped this one
                // — the entity limit, the memory budget — stops the rest.
                break;
            }
            None => {}
        }
    }
    if copies.is_empty() {
        return copies;
    }
    ctx.selection.select_all_of(&copies);
    *ctx.config_dirty = true;
    let mut args = fluent::FluentArgs::new();
    let message = if let [name] = names.as_slice() {
        args.set("name", name.clone());
        crate::i18n::t_args("toast-duplicated", &args)
    } else {
        args.set("count", names.len());
        crate::i18n::t_args("toast-duplicated-many", &args)
    };
    ctx.toasts.success(message);
    copies
}

/// Add the copy of `idx`; its index and the original's name. `None`
/// when there is no entity at `idx`.
fn copy_entity(idx: usize, ctx: &mut OutcomeCtx<'_>) -> Option<Result<(usize, String), String>> {
    let src = ctx.scene.entities.get(idx)?;
    // Everything read before the add: reading through `src` after the
    // scene's Vec has grown is the pattern `get`/`get_mut` retire.
    let src_name = src.name.clone();
    let src_path = std::path::PathBuf::from(&src.asset_path);
    let (new_x, new_y) = (src.x + 30.0, src.y + 30.0);
    let (scale, opacity) = (src.scale, src.opacity);

    match ctx.scene.add_entity_from_path(&src_path, new_x, new_y) {
        Ok(new_idx) => {
            if let Some(entity) = ctx.scene.entities.get_mut(new_idx) {
                entity.scale = scale;
                entity.opacity = opacity;
            }
            if let Some(renderer) = ctx.renderer.as_deref_mut() {
                if let Some(entity) = ctx.scene.entities.get(new_idx) {
                    renderer.ensure_texture(entity);
                }
                if let Some(entity) = ctx.scene.entities.get_mut(new_idx) {
                    entity.texture_dirty = false;
                }
            }
            tracing::info!("Duplicated '{src_name}' at ({new_x:.0}, {new_y:.0})");
            Some(Ok((new_idx, src_name)))
        }
        Err(e) => Some(Err(e.to_string())),
    }
}

/// Make the scene the one saved in `path` (`crate::scenes`) and say so;
/// it becomes the active one. Returns whether it loaded.
pub fn load_scene(
    path: &std::path::Path,
    ctx: &mut OutcomeCtx<'_>,
    active: &mut Option<String>,
) -> bool {
    match crate::scenes::load(path) {
        Ok(saved) => {
            ctx.scene.apply_saved(&saved);
            // Indices into the scene that was just replaced.
            ctx.selection.deselect();
            *ctx.config_dirty = true;
            tracing::info!("Scene loaded: {}", saved.name);
            let mut args = fluent::FluentArgs::new();
            args.set("name", saved.name.clone());
            ctx.toasts
                .info(crate::i18n::t_args("toast-scene-loaded", &args));
            *active = Some(saved.name);
            true
        }
        Err(e) => {
            tracing::warn!("Scene not loaded: {e}");
            let mut args = fluent::FluentArgs::new();
            args.set("error", e.to_string());
            ctx.toasts
                .warn(crate::i18n::t_args("toast-scene-failed", &args));
            false
        }
    }
}

/// The saved scene after the active one, by name, round to the first —
/// the tray's "Next scene". Says so when there are none.
pub fn next_scene(ctx: &mut OutcomeCtx<'_>, active: &mut Option<String>) -> bool {
    let shelf = crate::scenes::shelf();
    match crate::scenes::next_after(&shelf, active.as_deref()) {
        Some(entry) => {
            let path = entry.path.clone();
            load_scene(&path, ctx, active)
        }
        None => {
            ctx.toasts.info(crate::i18n::t("toast-no-scenes"));
            false
        }
    }
}

/// Apply a command-palette outcome.
pub fn apply_palette_outcome(
    outcome: PaletteOutcome,
    ctx: &mut OutcomeCtx<'_>,
    config: &mut AppConfig,
) {
    use crate::presets::{self, Preset};
    match outcome {
        PaletteOutcome::SwitchTheme(theme) => {
            config.global.theme = theme;
            let mut args = fluent::FluentArgs::new();
            args.set("theme", theme.label());
            ctx.toasts
                .success(crate::i18n::t_args("toast-theme-switched", &args));
        }
        // Each backend runs these itself before calling this — an action
        // through the same code as its shortcut, "Add file…" through its
        // own chooser — so nothing reaches this arm.
        PaletteOutcome::RunAction(_) | PaletteOutcome::AddFile => return,
        PaletteOutcome::LoadScene(path) => {
            load_scene(&path, ctx, &mut config.global.active_scene);
            return;
        }
        PaletteOutcome::ApplyPreset(id, mode) => {
            let preset = Preset::for_id(id);
            let existing = ctx.scene.to_character_configs();
            let new = presets::apply_to_scene(existing, &preset, mode);
            match mode {
                presets::ApplyMode::Replace => {
                    ctx.scene.reset_to_configs(&new);
                    // The selection is an index into the scene that was
                    // just rebuilt, so whatever it pointed at is gone.
                    ctx.selection.deselect();
                }
                presets::ApplyMode::Append => {
                    let already: std::collections::HashSet<String> =
                        ctx.scene.entities.iter().map(|e| e.id.clone()).collect();
                    for cfg in new.iter().filter(|c| !already.contains(&c.id)) {
                        if let Err(e) = ctx.scene.append_character_config(cfg) {
                            tracing::warn!("Palette preset append failed: {e}");
                            let mut args = fluent::FluentArgs::new();
                            args.set("error", e.to_string());
                            ctx.toasts
                                .warn(crate::i18n::t_args("toast-preset-entry-failed", &args));
                        }
                    }
                }
            }
            let mut args = fluent::FluentArgs::new();
            args.set("name", preset.name);
            ctx.toasts
                .success(crate::i18n::t_args("toast-preset-loaded", &args));
        }
    }
    *ctx.config_dirty = true;
}

/// Apply a Library "Add to scene" outcome, placing the asset at `at`.
pub fn apply_library_outcome(
    outcome: LibraryOutcome,
    ctx: &mut OutcomeCtx<'_>,
    library_root: Option<&Path>,
    library: &mut Option<LibraryIndex>,
    at: (f32, f32),
) {
    let Some(root) = library_root else {
        tracing::warn!("Library outcome received but no library_root is set; ignoring.");
        return;
    };
    // M2 hardening (0.5.2): a hand-edited `library.toml` could carry an
    // absolute path or a `../` segment that lifts the target out of the
    // asset root. `resolve_library_asset` canonicalises both sides and
    // rejects anything that escapes, before this reaches a decoder.
    let rel_path = Path::new(&outcome.relative_path);
    let abs_path = match resolve_library_asset(root, rel_path) {
        Ok(p) => p,
        Err(reason) => {
            // Redacted: `relative_path` comes from library.toml, which can
            // be hand-edited to carry Cf characters (RTL override,
            // zero-width, BOM) that would flip journald lines visually.
            tracing::warn!("Library asset {} rejected: {reason}", redact_path(rel_path));
            tracing::debug!("Rejected library relative path: {}", outcome.relative_path);
            reject(ctx.toasts, reason);
            return;
        }
    };
    // A path that stays inside the root can still be the wrong shape.
    if let Err(reason) = pre_validate_dropped_file(&abs_path) {
        tracing::warn!(
            "Library asset {} rejected: {reason}",
            redact_path(&abs_path)
        );
        tracing::debug!("Rejected library full path: {}", abs_path.display());
        reject(ctx.toasts, reason);
        return;
    }
    // `add_entity_from_path` runs the same asset-cap and format detection
    // as a drag-and-drop, so audit L2 holds for library adds too.
    match ctx.scene.add_entity_from_path(&abs_path, at.0, at.1) {
        Ok(_) => {
            let mut args = fluent::FluentArgs::new();
            args.set("name", outcome.display_name.clone());
            ctx.toasts
                .success(crate::i18n::t_args("library-asset-added-toast", &args));
            if let Some(library) = library.as_mut() {
                if let Some(asset) = library.assets.iter_mut().find(|a| a.id == outcome.asset_id) {
                    asset.last_used_at = Some(std::time::SystemTime::now());
                }
                // Best effort: losing a "last used" stamp is not worth an
                // error the user has to read.
                let _ = library.save(&LibraryIndex::default_path());
            }
            *ctx.config_dirty = true;
        }
        Err(e) => {
            tracing::warn!("Library add failed for {}: {e}", redact_path(rel_path));
            tracing::debug!("Failed relative path: {}", outcome.relative_path);
            let mut args = fluent::FluentArgs::new();
            args.set("name", outcome.display_name);
            ctx.toasts
                .error(crate::i18n::t_args("library-asset-add-failed-toast", &args));
        }
    }
}

/// Whether a dropped path is a Shimeji pack — a directory with `conf/` and
/// `img/` — rather than an asset (U.4).
pub fn is_shimeji_pack(path: &Path) -> bool {
    path.is_dir() && path.join("conf").is_dir() && path.join("img").is_dir()
}

/// Add a dropped file as a character at `at`, and select it.
///
/// The native Wayland loop used to do less than half of this: the new
/// character was not selected, nothing said whether the drop worked, and
/// the scene was never marked dirty — so a drop was only written to disk
/// if something else happened to be edited before quitting.
pub fn add_dropped_file(path: &Path, at: (f32, f32), ctx: &mut OutcomeCtx<'_>) -> Option<usize> {
    let label = redact_path(path);
    tracing::info!("File dropped: {label}");
    tracing::debug!("Dropped full path: {}", path.display());

    // The fast, clear refusal (extension, size, not a regular file) before
    // a decoder spins up and fails somewhere deeper.
    if let Err(reason) = pre_validate_dropped_file(path) {
        tracing::warn!("Rejecting dropped file {label}: {reason}");
        let mut args = fluent::FluentArgs::new();
        args.set("reason", reason);
        ctx.toasts
            .error(crate::i18n::t_args("toast-rejected", &args));
        return None;
    }

    match ctx.scene.add_entity_from_path(path, at.0, at.1) {
        Ok(idx) => {
            if let Some(renderer) = ctx.renderer.as_deref_mut() {
                if let Some(entity) = ctx.scene.entities.get(idx) {
                    renderer.ensure_texture(entity);
                }
                if let Some(entity) = ctx.scene.entities.get_mut(idx) {
                    entity.texture_dirty = false;
                }
            }
            ctx.selection.select(idx);
            *ctx.config_dirty = true;
            let name = ctx
                .scene
                .entities
                .get(idx)
                .map(|e| e.name.clone())
                .unwrap_or_default();
            tracing::info!("Added '{name}' at ({:.0}, {:.0})", at.0, at.1);
            let mut args = fluent::FluentArgs::new();
            args.set("name", name);
            ctx.toasts
                .success(crate::i18n::t_args("toast-added", &args));
            Some(idx)
        }
        Err(e) => {
            tracing::error!("Failed to load dropped file {label}: {e}");
            let mut args = fluent::FluentArgs::new();
            args.set("error", e.to_string());
            ctx.toasts
                .error(crate::i18n::t_args("toast-load-failed", &args));
            None
        }
    }
}

/// "Add file…" in progress: the desktop's file chooser is up, and its
/// answer is applied like a drop when it comes.
pub struct FileChooserAdd {
    chooser: crate::file_chooser::FileChooser,
    at: (f32, f32),
}

impl FileChooserAdd {
    /// Open the chooser; what is picked lands around `at`.
    pub fn start(at: (f32, f32)) -> Self {
        Self {
            chooser: crate::file_chooser::FileChooser::open(
                crate::i18n::t("file-chooser-title"),
                crate::i18n::t("file-chooser-filter"),
            ),
            at,
        }
    }

    /// Apply the answer if the dialog has closed: `None` while it is up,
    /// then `Some(n)` with the number of characters added, and the caller
    /// drops this. Each file goes through the drop path — validation,
    /// selection, toast — a little apart from the one before, so several
    /// files do not land on top of each other.
    pub fn poll(&self, ctx: &mut OutcomeCtx<'_>) -> Option<usize> {
        use crate::file_chooser::Chosen;
        Some(match self.chooser.poll()? {
            Chosen::Files(paths) => paths
                .iter()
                .enumerate()
                .filter(|(i, path)| {
                    let step = 48.0 * (*i % 8) as f32;
                    add_dropped_file(path, (self.at.0 + step, self.at.1 + step), ctx).is_some()
                })
                .count(),
            Chosen::Cancelled => 0,
            Chosen::Unavailable(reason) => {
                tracing::warn!("File chooser unavailable: {reason}");
                ctx.toasts
                    .error(crate::i18n::t("file-chooser-unavailable-toast"));
                0
            }
        })
    }
}

type ImportResult = Result<crate::shimeji::ImportReport, String>;

/// A Shimeji pack import running off the UI thread.
///
/// Copying a pack's sprites is the slow part, and a large (still capped)
/// pack must not freeze the overlay. The winit path did this on a worker;
/// the native Wayland loop imported synchronously on the UI thread and
/// dropped every character at a fixed (100, 100).
pub struct ShimejiImport {
    rx: std::sync::mpsc::Receiver<ImportResult>,
    at: (f32, f32),
}

impl ShimejiImport {
    /// Start importing `pack` into the library, to land at `at`.
    ///
    /// `None` when it could not start; the reason is already on screen.
    pub fn start(
        pack: &Path,
        library_root: Option<&Path>,
        at: (f32, f32),
        toasts: &mut ToastQueue,
    ) -> Option<Self> {
        let Some(library_root) = library_root.map(Path::to_path_buf) else {
            let mut args = fluent::FluentArgs::new();
            args.set(
                "path",
                crate::asset_library::asset_root_hint()
                    .display()
                    .to_string(),
            );
            toasts.error(crate::i18n::t_args("shimeji-no-library-toast", &args));
            return None;
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let pack = pack.to_path_buf();
        let spawned = std::thread::Builder::new()
            .name("anima-shimeji-import".into())
            .spawn(move || {
                let _ = tx.send(crate::shimeji::import_pack(&pack, &library_root));
            });
        match spawned {
            Ok(_) => Some(Self { rx, at }),
            Err(e) => {
                tracing::warn!("Shimeji import worker failed to spawn: {e}");
                let mut args = fluent::FluentArgs::new();
                args.set("reason", e.to_string());
                toasts.error(crate::i18n::t_args("shimeji-import-failed-toast", &args));
                None
            }
        }
    }

    /// Apply the import if it has finished.
    ///
    /// `None` while it is still running. Once it is done — imported,
    /// failed, or the worker vanished — `Some(n)` with the number of
    /// characters added, and the caller drops this.
    pub fn poll(&self, ctx: &mut OutcomeCtx<'_>) -> Option<usize> {
        let result = match self.rx.try_recv() {
            Ok(result) => result,
            Err(std::sync::mpsc::TryRecvError::Empty) => return None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                tracing::warn!("Shimeji import worker disconnected before delivering");
                return Some(0);
            }
        };
        Some(apply_shimeji_import(result, self.at, ctx))
    }
}

/// Add an import's characters at `at`. Returns how many were added.
fn apply_shimeji_import(result: ImportResult, at: (f32, f32), ctx: &mut OutcomeCtx<'_>) -> usize {
    let report = match result {
        Ok(report) => report,
        Err(reason) => {
            tracing::warn!("Shimeji import failed: {reason}");
            let mut args = fluent::FluentArgs::new();
            args.set("reason", reason);
            ctx.toasts
                .error(crate::i18n::t_args("shimeji-import-failed-toast", &args));
            return 0;
        }
    };
    let mut added = 0;
    for mut cfg in report.characters {
        cfg.x = at.0;
        cfg.y = at.1;
        // Importing the same pack twice must not collide on ids.
        cfg.id = ctx.scene.unique_id(&cfg.id);
        match ctx.scene.append_character_config(&cfg) {
            Ok(()) => added += 1,
            Err(e) => {
                tracing::warn!("Imported character '{}' rejected: {e}", cfg.name);
                let mut args = fluent::FluentArgs::new();
                args.set("name", cfg.name.clone());
                args.set("error", e.to_string());
                ctx.toasts
                    .error(crate::i18n::t_args("toast-entity-load-failed", &args));
            }
        }
    }
    for (what, why) in &report.skipped {
        tracing::info!("Shimeji import skip [{what}]: {why}");
    }
    if added > 0 {
        let mut args = fluent::FluentArgs::new();
        args.set("name", report.pack_name);
        args.set("n", report.skipped.len() as i64);
        ctx.toasts
            .success(crate::i18n::t_args("shimeji-imported-toast", &args));
        *ctx.config_dirty = true;
    }
    added
}

fn reject(toasts: &mut ToastQueue, reason: String) {
    let mut args = fluent::FluentArgs::new();
    args.set("reason", reason);
    toasts.warn(crate::i18n::t_args("toast-rejected", &args));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::{ApplyMode, PresetId};

    fn scene_with(n: usize) -> Scene {
        let mut scene = Scene::from_config(&AppConfig::default());
        scene.entities.clear();
        for i in 0..n {
            let cfg = crate::config::CharacterConfig {
                id: format!("e{i}"),
                name: format!("E{i}"),
                asset_type: crate::config::AssetType::PngStatic,
                asset_path: String::new(),
                x: 100.0,
                y: 200.0,
                scale: 2.0,
                opacity: 0.5,
                fps: 8.0,
                visible: true,
                playing: false,
                z_index: i as i32,
                physics_enabled: false,
                behavior: crate::behavior::Behavior::Idle,
                spritesheet_columns: None,
                spritesheet_rows: None,
                monitor: None,
                easing: None,
                animations: std::collections::BTreeMap::new(),
            };
            let frame = crate::animation::frame::Frame::new(vec![0u8; 4], 1, 1);
            let anim = crate::animation::Animation::new(vec![frame], 1.0, false);
            scene
                .entities
                .push(crate::entity::Entity::from_config(&cfg, anim));
        }
        scene
    }

    struct World {
        scene: Scene,
        selection: SelectionState,
        toasts: ToastQueue,
        dirty: bool,
    }

    impl World {
        fn new(n: usize) -> Self {
            Self {
                scene: scene_with(n),
                selection: SelectionState::default(),
                toasts: ToastQueue::default(),
                dirty: false,
            }
        }

        fn ctx(&mut self) -> OutcomeCtx<'_> {
            OutcomeCtx {
                scene: &mut self.scene,
                selection: &mut self.selection,
                toasts: &mut self.toasts,
                config_dirty: &mut self.dirty,
                renderer: None,
            }
        }

        fn messages(&self) -> Vec<String> {
            self.toasts.iter().map(|t| t.message.clone()).collect()
        }
    }

    /// The Wayland copy kept the selection across a Replace, i.e. an index
    /// into a scene that had just been rebuilt from different entities.
    #[test]
    fn replacing_the_scene_with_a_preset_clears_the_selection() {
        let mut w = World::new(3);
        w.selection.select(2);
        let mut config = AppConfig::default();
        apply_palette_outcome(
            PaletteOutcome::ApplyPreset(PresetId::ALL[0], ApplyMode::Replace),
            &mut w.ctx(),
            &mut config,
        );
        assert_eq!(w.selection.selected_index(), None);
        assert!(w.dirty);
    }

    #[test]
    fn switching_theme_marks_the_config_dirty() {
        let mut w = World::new(0);
        let mut config = AppConfig::default();
        let theme = crate::ui::Theme::Light;
        apply_palette_outcome(
            PaletteOutcome::SwitchTheme(theme),
            &mut w.ctx(),
            &mut config,
        );
        assert_eq!(config.global.theme, theme);
        assert!(w.dirty);
    }

    #[test]
    fn deleting_removes_the_entity_and_the_selection() {
        let mut w = World::new(2);
        w.selection.select(1);
        assert!(delete_entity(1, &mut w.ctx()));
        assert_eq!(w.scene.entities.len(), 1);
        assert_eq!(w.selection.selected_index(), None);
        assert!(w.dirty);
        assert_eq!(w.messages().len(), 1);
    }

    #[test]
    fn deleting_a_stale_index_does_nothing() {
        let mut w = World::new(1);
        assert!(!delete_entity(5, &mut w.ctx()));
        assert_eq!(w.scene.entities.len(), 1);
        assert!(!w.dirty);
        assert!(w.messages().is_empty());
    }

    #[test]
    fn menu_edits_touch_only_their_target() {
        let mut w = World::new(2);
        apply_menu_action(MenuAction::ResetTransform(1), &mut w.ctx());
        apply_menu_action(MenuAction::BringForward(1), &mut w.ctx());
        assert_eq!(
            (w.scene.entities[1].scale, w.scene.entities[1].opacity),
            (1.0, 1.0)
        );
        assert_eq!(w.scene.entities[1].z_index, 11);
        assert_eq!(
            (w.scene.entities[0].scale, w.scene.entities[0].z_index),
            (2.0, 0)
        );
        assert!(w.dirty);
    }

    /// Duplicating an entity whose asset cannot be loaded reports it rather
    /// than adding a broken copy.
    #[test]
    fn a_failed_duplicate_reports_and_changes_nothing() {
        let mut w = World::new(1);
        assert_eq!(duplicate_entity(0, &mut w.ctx()), None);
        assert_eq!(w.scene.entities.len(), 1);
        assert!(!w.dirty);
        assert_eq!(w.messages().len(), 1);
    }

    #[test]
    fn a_library_add_without_a_library_is_ignored() {
        let mut w = World::new(0);
        let outcome = LibraryOutcome {
            asset_id: "0123456789ab".into(),
            relative_path: "a.png".into(),
            display_name: "a".into(),
        };
        apply_library_outcome(outcome, &mut w.ctx(), None, &mut None, (0.0, 0.0));
        assert!(w.scene.entities.is_empty());
        assert!(!w.dirty);
    }

    /// The Wayland loop never marked a drop dirty, so a dropped character
    /// was only saved if something else was edited before quitting.
    #[test]
    fn a_rejected_drop_changes_nothing_and_says_so() {
        let mut w = World::new(0);
        let bogus = std::path::Path::new("/definitely/not/here.png");
        assert_eq!(add_dropped_file(bogus, (10.0, 10.0), &mut w.ctx()), None);
        assert!(w.scene.entities.is_empty());
        assert!(!w.dirty);
        assert_eq!(w.messages().len(), 1);
    }

    #[test]
    fn a_dropped_image_is_added_selected_and_marked_dirty() {
        let dir = std::env::temp_dir().join(format!("anima-drop-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let png = dir.join("dot.png");
        image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]))
            .save(&png)
            .unwrap();
        let mut w = World::new(1);
        let idx = add_dropped_file(&png, (40.0, 50.0), &mut w.ctx());
        let _ = std::fs::remove_dir_all(&dir);
        let idx = idx.expect("a valid PNG is added");
        assert_eq!(w.scene.entities.len(), 2);
        assert_eq!(
            (w.scene.entities[idx].x, w.scene.entities[idx].y),
            (40.0, 50.0)
        );
        assert_eq!(w.selection.selected_index(), Some(idx));
        assert!(w.dirty, "a drop has to reach the next save");
    }

    #[test]
    fn a_pack_needs_both_conf_and_img() {
        let dir = std::env::temp_dir().join(format!("anima-pack-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("img")).unwrap();
        let half = is_shimeji_pack(&dir);
        std::fs::create_dir_all(dir.join("conf")).unwrap();
        let whole = is_shimeji_pack(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(!half);
        assert!(whole);
    }

    #[test]
    fn an_import_lands_where_it_was_dropped_with_fresh_ids() {
        let dir = std::env::temp_dir().join(format!("anima-import-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let png = dir.join("sprite.png");
        image::RgbaImage::from_pixel(2, 2, image::Rgba([0, 255, 0, 255]))
            .save(&png)
            .unwrap();
        let mut w = World::new(1);
        let mut cfg = crate::config::AppConfig::default().characters[0].clone();
        cfg.id = "e0".into(); // already in the scene
        cfg.asset_type = crate::config::AssetType::PngStatic;
        cfg.asset_path = png.display().to_string();
        let report = crate::shimeji::ImportReport {
            pack_name: "pack".into(),
            characters: vec![cfg],
            skipped: vec![],
        };
        let added = apply_shimeji_import(Ok(report), (300.0, 400.0), &mut w.ctx());
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(added, 1, "{:?}", w.messages());
        let e = w.scene.entities.last().unwrap();
        assert_eq!((e.x, e.y), (300.0, 400.0));
        assert_ne!(e.id, "e0", "an id already in the scene is not reused");
        assert!(w.dirty);
    }

    #[test]
    fn a_failed_import_is_reported() {
        let mut w = World::new(0);
        assert_eq!(
            apply_shimeji_import(Err("nope".into()), (0.0, 0.0), &mut w.ctx()),
            0
        );
        assert_eq!(w.messages().len(), 1);
        assert!(!w.dirty);
    }

    /// The containment check runs before anything is decoded.
    #[test]
    fn a_library_path_escaping_the_root_is_rejected() {
        let root = std::env::temp_dir().join(format!("anima-outcomes-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let mut w = World::new(0);
        let outcome = LibraryOutcome {
            asset_id: "0123456789ab".into(),
            relative_path: "../../etc/passwd".into(),
            display_name: "passwd".into(),
        };
        apply_library_outcome(outcome, &mut w.ctx(), Some(&root), &mut None, (0.0, 0.0));
        let _ = std::fs::remove_dir_all(&root);
        assert!(w.scene.entities.is_empty());
        assert!(!w.dirty);
        assert_eq!(w.messages().len(), 1, "the rejection is reported");
    }
}
