//! Library tab — read-only browser over the persisted `LibraryIndex`.
//! Extracted in I.5.
//!
//! Outcome flows back to `App` via the shared `&mut Option<LibraryOutcome>`
//! so the actual disk-touching load runs after the egui frame returns —
//! avoids the borrow checker hating us for grabbing &mut Scene and a
//! TextureManager at once.

use crate::asset_library::{LibraryAsset, LibraryIndex, LibraryKind};
use crate::i18n::t;
use crate::ui::icons;
use crate::ui::states;
use crate::ui::theme::{self, SPACE_L, SPACE_M, SPACE_S, SPACE_XS};

/// One-shot intent emitted by the Library tab. App resolves the asset
/// path to absolute and routes through `Scene::add_entity_from_path`
/// (which runs `pre_validate_dropped_file`, preserving audit L2).
pub struct LibraryOutcome {
    /// Asset id (12 hex chars) — for updating `last_used_at`.
    pub asset_id: String,
    /// Path relative to the asset root, as stored in `LibraryAsset.path`.
    pub relative_path: String,
    /// Human-friendly name for the toast.
    pub display_name: String,
}

pub(super) fn library_tab(
    ui: &mut egui::Ui,
    library: Option<&LibraryIndex>,
    outcome: &mut Option<LibraryOutcome>,
    import_request: &mut Option<String>,
) {
    // ── Shimeji pack import (U.4) ─────────────────────────────────────
    // Path-paste flow; the folder drag-drop equivalent lives in the
    // drop handler. No file-dialog dependency by design.
    ui.collapsing(t("library-import-shimeji-header"), |ui| {
        ui.label(
            egui::RichText::new(t("library-import-shimeji-hint"))
                .small()
                .weak(),
        );
        let path_id = egui::Id::new("anima.shimeji.import_path");
        let mut path: String = ui
            .ctx()
            .memory(|m| m.data.get_temp(path_id))
            .unwrap_or_default();
        ui.horizontal(|ui| {
            let edit = egui::TextEdit::singleline(&mut path)
                .hint_text("~/Downloads/MyMascot")
                .char_limit(512)
                .desired_width(200.0);
            ui.add(edit);
            if ui.button(t("library-import-shimeji-button")).clicked() && !path.trim().is_empty() {
                *import_request = Some(path.trim().to_string());
                path.clear();
            }
        });
        ui.ctx().memory_mut(|m| m.data.insert_temp(path_id, path));
    });
    ui.separator();

    let Some(library) = library else {
        // D.8: copy the documented path to the clipboard so the user
        // can paste it into a file manager / terminal without typing.
        if states::empty_with_action(
            ui,
            icons::LIBRARY,
            &t("library-empty-headline"),
            &t("library-no-asset-root"),
            Some(&t("library-empty-action-copy-path")),
        ) {
            let path = default_asset_path_hint();
            ui.ctx().copy_text(path);
        }
        return;
    };

    if library.assets.is_empty() {
        if states::empty_with_action(
            ui,
            icons::LIBRARY,
            &t("library-empty-headline"),
            &t("library-empty-hint"),
            Some(&t("library-empty-action-copy-path")),
        ) {
            let path = default_asset_path_hint();
            ui.ctx().copy_text(path);
        }
        return;
    }

    // Persisted across frames so the search box doesn't reset on every
    // repaint. Lives in egui::Memory rather than App to keep the
    // settings sidebar signature shorter.
    let query_id = egui::Id::new("anima.library.query");
    let mut query: String = ui.memory(|m| m.data.get_temp(query_id).unwrap_or_default());

    ui.horizontal(|ui| {
        ui.label(icons::SEARCH);
        // G.5 (0.5.3): cap the search query at 256 chars. Without it
        // a programmatic clipboard inject could grow the egui text
        // buffer without bound.
        ui.add(
            egui::TextEdit::singleline(&mut query)
                .hint_text(t("library-search-placeholder"))
                .desired_width(ui.available_width())
                .char_limit(256),
        );
    });
    ui.memory_mut(|m| m.data.insert_temp(query_id, query.clone()));
    ui.add_space(SPACE_S);

    // Filter once per frame; library sizes are small enough for this
    // to be cheap (couple hundred entries max in practice).
    let q = query.to_lowercase();
    let filtered: Vec<&LibraryAsset> = library
        .assets
        .iter()
        .filter(|a| {
            q.is_empty()
                || a.path.to_lowercase().contains(&q)
                || a.tags.iter().any(|t| t.to_lowercase().contains(&q))
        })
        .collect();

    if filtered.is_empty() {
        ui.add_space(SPACE_L);
        ui.label(
            egui::RichText::new(format!("0 / {} assets", library.assets.len()))
                .text_style(theme::caption())
                .weak(),
        );
        return;
    }

    // Footer count above the list (sticky-feel without nested ScrollArea).
    let mut args = fluent::FluentArgs::new();
    args.set("n", library.assets.len() as i64);
    ui.label(
        egui::RichText::new(crate::i18n::t_args("library-count", &args))
            .text_style(theme::caption())
            .weak(),
    );
    ui.add_space(SPACE_S);

    for asset in filtered {
        library_row(ui, asset, outcome);
        ui.add_space(SPACE_XS);
    }
}

/// Cap on cached thumbnail textures. On overflow the cache resets
/// wholesale — a rare event (it takes a 256-asset scroll session) and
/// strictly cheaper than tracking LRU order per frame.
const THUMB_TEXTURE_CAP: usize = 256;

/// Fetch (or lazily load) the thumbnail texture for an asset (U.6).
/// Returns `None` while the background generator hasn't produced the
/// file yet — callers fall back to the kind glyph, and the row picks
/// the texture up on a later frame without any signalling.
fn thumbnail_texture(ui: &egui::Ui, asset: &LibraryAsset) -> Option<egui::TextureHandle> {
    let tex_id = egui::Id::new(("anima.thumb", asset.id.as_str()));
    if let Some(handle) = ui
        .ctx()
        .memory(|m| m.data.get_temp::<egui::TextureHandle>(tex_id))
    {
        return Some(handle);
    }
    let path = crate::asset_library::thumbnail_cache_dir().join(asset.thumbnail_filename());
    // Cheap existence probe each frame until the generator catches up.
    let img = image::open(&path).ok()?;
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    let handle = ui
        .ctx()
        .load_texture(format!("thumb-{}", asset.id), color, Default::default());

    let count_id = egui::Id::new("anima.thumb.count");
    ui.ctx().memory_mut(|m| {
        let count: usize = m.data.get_temp(count_id).unwrap_or(0);
        if count >= THUMB_TEXTURE_CAP {
            // Wholesale reset; handles drop, egui frees the textures.
            m.data.clear();
        }
        m.data.insert_temp(tex_id, handle.clone());
        let count: usize = m.data.get_temp(count_id).unwrap_or(0);
        m.data.insert_temp(count_id, count + 1);
    });
    Some(handle)
}

/// Render one asset row: thumbnail (or kind glyph while it generates),
/// name + kind, "Add" button.
fn library_row(ui: &mut egui::Ui, asset: &LibraryAsset, outcome: &mut Option<LibraryOutcome>) {
    let (bg, accent, body_color) = {
        let v = ui.visuals();
        (v.faint_bg_color, v.hyperlink_color, v.text_color())
    };
    let kind_icon = match asset.kind {
        LibraryKind::Image => icons::KIND_IMAGE,
        LibraryKind::Animated => icons::KIND_ANIMATED,
        LibraryKind::Video => icons::KIND_VIDEO,
    };
    let kind_label = match asset.kind {
        LibraryKind::Image => t("library-kind-image"),
        LibraryKind::Animated => t("library-kind-animated"),
        LibraryKind::Video => t("library-kind-video"),
    };

    egui::Frame::new()
        .fill(bg)
        .corner_radius(theme::RADIUS_MD)
        .inner_margin(egui::Margin::symmetric(SPACE_M as i8, SPACE_S as i8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                match thumbnail_texture(ui, asset) {
                    Some(tex) => {
                        ui.add(
                            egui::Image::new(&tex)
                                .fit_to_exact_size(egui::vec2(32.0, 32.0))
                                .corner_radius(theme::RADIUS_SM),
                        );
                    }
                    None => {
                        ui.label(egui::RichText::new(kind_icon).size(18.0).color(accent));
                    }
                }
                ui.add_space(SPACE_S);
                ui.vertical(|ui| {
                    // Use the basename for the headline; path is a tooltip.
                    let basename = asset.path.rsplit('/').next().unwrap_or(asset.path.as_str());
                    ui.label(egui::RichText::new(basename).strong().color(body_color))
                        .on_hover_text(&asset.path);
                    ui.label(
                        egui::RichText::new(kind_label)
                            .text_style(theme::caption())
                            .weak(),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(format!("{}  {}", icons::ADD, t("library-add-to-scene")))
                        .clicked()
                        && outcome.is_none()
                    {
                        let display_name = asset
                            .path
                            .rsplit('/')
                            .next()
                            .unwrap_or(asset.path.as_str())
                            .to_string();
                        *outcome = Some(LibraryOutcome {
                            asset_id: asset.id.clone(),
                            relative_path: asset.path.clone(),
                            display_name,
                        });
                    }
                });
            });
        });
}

/// Documented asset-library path used by the empty-state CTA — kept
/// in sync with `library-no-asset-root` i18n and the doc in
/// `docs/config.md`. Lives here so the panels module can expose a
/// "Copy path" button without dragging the library module's path
/// resolution into the panel.
fn default_asset_path_hint() -> String {
    if let Ok(home) = std::env::var("HOME") {
        format!("{home}/.local/share/animaEngine/assets")
    } else {
        "~/.local/share/animaEngine/assets".into()
    }
}
