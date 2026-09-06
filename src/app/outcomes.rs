//! UI outcome handlers — the bridges between egui panels and `App`.
//!
//! Each handler runs once per frame after `panels::settings` returns,
//! consuming the `Option<…Outcome>` the panel emitted. Extracted in
//! H.2 to keep the main module focused on event-loop wiring.

use super::App;
use crate::drop_validate::{pre_validate_dropped_file, redact_path, resolve_library_asset};
use crate::ui::panels;

impl App {
    pub(super) fn handle_menu_outcome(&mut self, outcome: panels::ContextMenuOutcome) {
        match outcome {
            panels::ContextMenuOutcome::Open => {}
            panels::ContextMenuOutcome::Close => {
                self.ui_state.context_menu = None;
            }
            panels::ContextMenuOutcome::Action(action) => {
                self.apply_menu_action(action);
                self.ui_state.context_menu = None;
            }
        }
    }

    pub(super) fn handle_library_outcome(&mut self, outcome: panels::LibraryOutcome) {
        let Some(root) = self.library_root.as_ref() else {
            tracing::warn!("Library outcome received but no library_root is set; ignoring.");
            return;
        };
        // M2 hardening (0.5.2): a hand-edited `library.toml` could
        // carry an absolute path or `../` segment that lifts the
        // resolved target out of the asset root. `resolve_library_asset`
        // canonicalises both sides and rejects anything that escapes.
        let rel_path = std::path::Path::new(&outcome.relative_path);
        let abs_path = match resolve_library_asset(root, rel_path) {
            Ok(p) => p,
            Err(reason) => {
                // Redact: `relative_path` comes from library.toml which
                // a determined user can hand-edit with Cf chars (RTL
                // override, zero-width, BOM) that would flip journald
                // entries visually.
                tracing::warn!("Library asset {} rejected: {reason}", redact_path(rel_path));
                tracing::debug!("Rejected library relative path: {}", outcome.relative_path);
                {
                    let mut args = fluent::FluentArgs::new();
                    args.set("reason", reason.clone());
                    self.toasts
                        .warn(crate::i18n::t_args("toast-rejected", &args));
                }
                return;
            }
        };
        // The shared stat/whitelist gate still applies — a path that
        // stays inside the root can still be the wrong shape.
        if let Err(reason) = pre_validate_dropped_file(&abs_path) {
            tracing::warn!(
                "Library asset {} rejected: {reason}",
                redact_path(&abs_path)
            );
            tracing::debug!("Rejected library full path: {}", abs_path.display());
            {
                let mut args = fluent::FluentArgs::new();
                args.set("reason", reason.clone());
                self.toasts
                    .warn(crate::i18n::t_args("toast-rejected", &args));
            }
            return;
        }
        // Drop in the middle of the visible viewport, falling back to
        // a sensible default when the window isn't fully wired yet.
        let (x, y) = self
            .window
            .as_ref()
            .map(|w| {
                let size = w.inner_size();
                (size.width as f32 / 2.0, size.height as f32 / 2.0)
            })
            .unwrap_or((400.0, 300.0));
        // `add_entity_from_path` runs the full asset-cap + extension
        // detection pipeline — same path as drag-drop — so audit L2
        // is preserved even though the asset came from the library
        // index instead of a user drop.
        match self.scene.add_entity_from_path(&abs_path, x, y) {
            Ok(_) => {
                let mut args = fluent::FluentArgs::new();
                args.set("name", outcome.display_name.clone());
                self.toasts
                    .success(crate::i18n::t_args("library-asset-added-toast", &args));
                // Bump last_used_at so the asset surfaces in the future
                // "Recent" sort introduced in C.9 polish.
                if let Some(library) = self.library.as_mut() {
                    if let Some(asset) =
                        library.assets.iter_mut().find(|a| a.id == outcome.asset_id)
                    {
                        asset.last_used_at = Some(std::time::SystemTime::now());
                    }
                    // Best-effort persist; failure is non-fatal.
                    let _ = library.save(&crate::asset_library::LibraryIndex::default_path());
                }
                self.config_dirty = true;
            }
            Err(e) => {
                tracing::warn!(
                    "Library add failed for {}: {e}",
                    redact_path(std::path::Path::new(&outcome.relative_path))
                );
                tracing::debug!("Failed relative path: {}", outcome.relative_path);
                let mut args = fluent::FluentArgs::new();
                args.set("name", outcome.display_name);
                self.toasts
                    .error(crate::i18n::t_args("library-asset-add-failed-toast", &args));
            }
        }
    }

    pub(super) fn handle_palette_outcome(&mut self, outcome: panels::PaletteOutcome) {
        use crate::presets::{self, Preset};
        match outcome {
            panels::PaletteOutcome::SwitchTheme(theme) => {
                self.config.global.theme = theme;
                self.config_dirty = true;
                {
                    let mut args = fluent::FluentArgs::new();
                    args.set("theme", theme.label());
                    self.toasts
                        .success(crate::i18n::t_args("toast-theme-switched", &args));
                }
            }
            panels::PaletteOutcome::ApplyPreset(id, mode) => {
                let preset = Preset::for_id(id);
                let existing = self.scene.to_character_configs();
                let new = presets::apply_to_scene(existing, &preset, mode);
                match mode {
                    presets::ApplyMode::Replace => {
                        self.scene.reset_to_configs(&new);
                        self.selection.deselect();
                    }
                    presets::ApplyMode::Append => {
                        let already: std::collections::HashSet<String> =
                            self.scene.entities.iter().map(|e| e.id.clone()).collect();
                        for cfg in new.iter().filter(|c| !already.contains(&c.id)) {
                            if let Err(e) = self.scene.append_character_config(cfg) {
                                tracing::warn!("Palette preset append failed: {e}");
                                {
                                    let mut args = fluent::FluentArgs::new();
                                    args.set("error", e.to_string());
                                    self.toasts.warn(crate::i18n::t_args(
                                        "toast-preset-entry-failed",
                                        &args,
                                    ));
                                }
                            }
                        }
                    }
                }
                self.config_dirty = true;
                self.toasts
                    .success(format!("Loaded preset: {}", preset.name));
            }
        }
    }

    fn apply_menu_action(&mut self, action: panels::MenuAction) {
        match action {
            panels::MenuAction::Duplicate(idx) => {
                let Some(src) = self.scene.entities.get(idx) else {
                    return;
                };
                let src_name = src.name.clone();
                let src_path = std::path::PathBuf::from(&src.asset_path);
                let new_x = src.x + 30.0;
                let new_y = src.y + 30.0;
                let orig_scale = src.scale;
                let orig_opacity = src.opacity;

                match self.scene.add_entity_from_path(&src_path, new_x, new_y) {
                    Ok(new_idx) => {
                        if let Some(entity) = self.scene.entities.get_mut(new_idx) {
                            entity.scale = orig_scale;
                            entity.opacity = orig_opacity;
                        }
                        if let Some(renderer) = &mut self.renderer {
                            if let Some(entity) = self.scene.entities.get(new_idx) {
                                renderer.ensure_texture(entity);
                            }
                            if let Some(entity) = self.scene.entities.get_mut(new_idx) {
                                entity.texture_dirty = false;
                            }
                        }
                        self.selection.select(new_idx);
                        self.config_dirty = true;
                        {
                            let mut args = fluent::FluentArgs::new();
                            args.set("name", src_name.clone());
                            self.toasts
                                .success(crate::i18n::t_args("toast-duplicated", &args));
                        }
                        self.save_config_if_needed();
                    }
                    Err(e) => {
                        tracing::error!("Context menu duplicate failed: {}", e);
                        {
                            let mut args = fluent::FluentArgs::new();
                            args.set("error", e.to_string());
                            self.toasts
                                .error(crate::i18n::t_args("toast-duplicate-failed", &args));
                        }
                    }
                }
            }
            panels::MenuAction::Delete(idx) => self.delete_entity(idx),
            panels::MenuAction::ResetTransform(idx) => {
                if let Some(e) = self.scene.entities.get_mut(idx) {
                    e.scale = 1.0;
                    e.opacity = 1.0;
                    self.config_dirty = true;
                }
            }
            panels::MenuAction::ToggleGravity(idx) => {
                if let Some(e) = self.scene.entities.get_mut(idx) {
                    e.physics.toggle();
                    self.config_dirty = true;
                }
            }
            panels::MenuAction::BringForward(idx) => {
                if let Some(e) = self.scene.entities.get_mut(idx) {
                    e.z_index += 10;
                    self.scene.mark_visible_dirty();
                    self.config_dirty = true;
                }
            }
            panels::MenuAction::SendBackward(idx) => {
                if let Some(e) = self.scene.entities.get_mut(idx) {
                    e.z_index -= 10;
                    self.scene.mark_visible_dirty();
                    self.config_dirty = true;
                }
            }
        }
    }
}
