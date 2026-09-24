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

/// Apply one context-menu action.
pub fn apply_menu_action(action: MenuAction, ctx: &mut OutcomeCtx<'_>) {
    match action {
        MenuAction::Duplicate(idx) => {
            duplicate_entity(idx, ctx);
        }
        MenuAction::Delete(idx) => {
            delete_entity(idx, ctx);
        }
        MenuAction::ResetTransform(idx) => {
            if let Some(e) = ctx.scene.entities.get_mut(idx) {
                e.scale = 1.0;
                e.opacity = 1.0;
                *ctx.config_dirty = true;
            }
        }
        MenuAction::ToggleGravity(idx) => {
            if let Some(e) = ctx.scene.entities.get_mut(idx) {
                e.physics.toggle();
                *ctx.config_dirty = true;
            }
        }
        MenuAction::BringForward(idx) => {
            if let Some(e) = ctx.scene.entities.get_mut(idx) {
                e.z_index += 10;
                ctx.scene.mark_visible_dirty();
                *ctx.config_dirty = true;
            }
        }
        MenuAction::SendBackward(idx) => {
            if let Some(e) = ctx.scene.entities.get_mut(idx) {
                e.z_index -= 10;
                ctx.scene.mark_visible_dirty();
                *ctx.config_dirty = true;
            }
        }
    }
}

/// Remove entity `idx`. Returns whether anything was removed.
///
/// The texture is dropped *before* the entity leaves the scene. Textures
/// are keyed by entity id, so a missed removal leaves an orphan resident
/// for the session — the leak `prune_stale_textures` exists to sweep up.
pub fn delete_entity(idx: usize, ctx: &mut OutcomeCtx<'_>) -> bool {
    let Some(entity) = ctx.scene.entities.get(idx) else {
        return false;
    };
    let removed_name = entity.name.clone();
    if let Some(renderer) = ctx.renderer.as_deref_mut() {
        renderer.shared.textures.remove(&entity.id);
    }
    let Some(removed_id) = ctx.scene.remove_entity(idx) else {
        return false;
    };
    tracing::info!("Deleted entity: {removed_id}");
    ctx.selection.deselect();
    *ctx.config_dirty = true;
    let mut args = fluent::FluentArgs::new();
    args.set("name", removed_name);
    ctx.toasts.info(crate::i18n::t_args("toast-deleted", &args));
    true
}

/// Copy entity `idx` 30 px down and right, keeping its scale and opacity,
/// and select the copy. Returns the copy's index.
pub fn duplicate_entity(idx: usize, ctx: &mut OutcomeCtx<'_>) -> Option<usize> {
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
            ctx.selection.select(new_idx);
            *ctx.config_dirty = true;
            tracing::info!("Duplicated '{src_name}' at ({new_x:.0}, {new_y:.0})");
            let mut args = fluent::FluentArgs::new();
            args.set("name", src_name);
            ctx.toasts
                .success(crate::i18n::t_args("toast-duplicated", &args));
            Some(new_idx)
        }
        Err(e) => {
            tracing::error!("Duplicate failed: {e}");
            let mut args = fluent::FluentArgs::new();
            args.set("error", e.to_string());
            ctx.toasts
                .error(crate::i18n::t_args("toast-duplicate-failed", &args));
            None
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
