//! Library tab — read-only browser over the persisted `LibraryIndex`.
//! Extracted in I.5.
//!
//! Outcome flows back to `App` via the shared `&mut Option<LibraryOutcome>`
//! so the actual disk-touching load runs after the egui frame returns —
//! avoids the borrow checker hating us for grabbing &mut Scene and a
//! TextureManager at once.

use crate::asset_library::{LibraryAsset, LibraryIndex, LibraryKind};
use crate::i18n::{t, t_args};
use crate::ui::accessible::AccessibleName;
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
            let field = ui.add(edit);
            crate::ui::accessible::name_text_field(&field, &t("library-import-shimeji-header"));
            if ui.button(t("library-import-shimeji-button")).clicked() && !path.trim().is_empty() {
                *import_request = Some(path.trim().to_string());
                path.clear();
            }
        });
        ui.ctx().memory_mut(|m| m.data.insert_temp(path_id, path));
    });
    ui.separator();

    // One resolved path for both the sentence and the clipboard, so the
    // button cannot hand over somewhere different from what the text just
    // named (R32). `library-no-asset-root` has always carried a `{ $path }`
    // placeholder and was rendered through `t()` with no argument to fill
    // it, which put the placeholder itself on screen.
    let root = crate::asset_library::asset_root_hint();
    let mut path_args = fluent::FluentArgs::new();
    path_args.set("path", root.display().to_string());

    let Some(library) = library else {
        // D.8: copy the path to the clipboard so the user can paste it
        // into a file manager / terminal without typing it.
        if states::empty_with_action(
            ui,
            icons::LIBRARY,
            &t("library-empty-headline"),
            &t_args("library-no-asset-root", &path_args),
            Some(&t("library-empty-action-copy-path")),
        ) {
            ui.ctx().copy_text(root.display().to_string());
        }
        return;
    };

    if library.assets.is_empty() {
        if states::empty_with_action(
            ui,
            icons::LIBRARY,
            &t("library-empty-headline"),
            &t_args("library-empty-hint", &path_args),
            Some(&t("library-empty-action-copy-path")),
        ) {
            ui.ctx().copy_text(root.display().to_string());
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
        let placeholder = t("library-search-placeholder");
        let field = ui.add(
            egui::TextEdit::singleline(&mut query)
                .hint_text(placeholder.as_str())
                .desired_width(ui.available_width())
                .char_limit(256),
        );
        crate::ui::accessible::name_text_field(&field, &placeholder);
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

    // Footer count above the list (sticky-feel without nested ScrollArea).
    let mut args = fluent::FluentArgs::new();
    args.set("n", library.assets.len() as i64);
    let count = crate::i18n::t_args("library-count", &args);
    if filtered.is_empty() {
        // It said "0 / 308 assets", in English in every language.
        ui.add_space(SPACE_L);
        ui.label(t("library-no-match"));
        ui.label(
            egui::RichText::new(count)
                .text_style(theme::caption())
                .weak(),
        );
        return;
    }
    ui.label(
        egui::RichText::new(count)
            .text_style(theme::caption())
            .weak(),
    );
    ui.add_space(SPACE_S);

    for asset in filtered {
        library_row(ui, asset, outcome);
        ui.add_space(SPACE_XS);
    }
}

/// Thumbnail textures kept at once: the rows on screen and those seen
/// most recently. Past it, the one seen longest ago goes.
const THUMB_TEXTURE_CAP: usize = 256;

/// How long a thumbnail that is not there yet goes unasked: the
/// generator writes them on its own thread, so a row looks again.
const THUMB_RETRY: std::time::Duration = std::time::Duration::from_secs(1);

/// The Library tab's thumbnail textures, kept in egui's memory under one
/// id. They used to be kept there one id each, beside everything else
/// the panels keep there, and past the cap the whole memory was cleared:
/// with more than 256 pictures in the library the tab could not be
/// opened — the active tab went with the rest, back to the Inspector —
/// and every frame decoded the thumbnails again (1.5).
#[derive(Default)]
struct Thumbs {
    /// By asset id: the texture, and the pass it was last drawn in.
    loaded: std::collections::HashMap<String, (egui::TextureHandle, u64)>,
    /// By asset id: when its thumbnail was last looked for and missing.
    missing: std::collections::HashMap<String, std::time::Instant>,
}

type SharedThumbs = std::sync::Arc<std::sync::Mutex<Thumbs>>;

/// The thumbnail texture for an asset (U.6), loaded the first time its
/// row is on screen. `None` while the background generator has not
/// written it — or never will: a video, a picture that does not decode —
/// and the row shows the kind's icon instead.
fn thumbnail_texture(ui: &egui::Ui, asset: &LibraryAsset) -> Option<egui::TextureHandle> {
    let ctx = ui.ctx();
    let pass = ctx.cumulative_pass_nr();
    let thumbs: SharedThumbs = ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<SharedThumbs>(egui::Id::new("anima.thumbs"))
            .clone()
    });
    let mut thumbs = thumbs.lock().ok()?;
    if let Some((handle, seen)) = thumbs.loaded.get_mut(&asset.id) {
        *seen = pass;
        return Some(handle.clone());
    }
    if thumbs
        .missing
        .get(&asset.id)
        .is_some_and(|at| at.elapsed() < THUMB_RETRY)
    {
        return None;
    }
    let path = crate::asset_library::thumbnail_cache_dir().join(asset.thumbnail_filename());
    // Ours are 64 px a side. Past 256 a file there is not one of them,
    // and it is not decoded at whatever size it says.
    let read = || -> image::ImageResult<image::DynamicImage> {
        let mut reader = image::ImageReader::open(&path)?.with_guessed_format()?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(256);
        limits.max_image_height = Some(256);
        reader.limits(limits);
        reader.decode()
    };
    let Ok(img) = read() else {
        thumbs
            .missing
            .insert(asset.id.clone(), std::time::Instant::now());
        return None;
    };
    thumbs.missing.remove(&asset.id);
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    let handle = ctx.load_texture(format!("thumb-{}", asset.id), color, Default::default());
    if thumbs.loaded.len() >= THUMB_TEXTURE_CAP {
        let oldest = thumbs
            .loaded
            .iter()
            .min_by_key(|(_, (_, seen))| *seen)
            .map(|(id, _)| id.clone());
        if let Some(oldest) = oldest {
            // Its handle dropped, egui frees the texture.
            thumbs.loaded.remove(&oldest);
        }
    }
    thumbs
        .loaded
        .insert(asset.id.clone(), (handle.clone(), pass));
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
                // Loaded only for a row on screen: every row of the
                // library is laid out each frame, and a thousand of them
                // would decode a thousand thumbnails.
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::hover());
                if ui.is_rect_visible(rect) {
                    match thumbnail_texture(ui, asset) {
                        Some(tex) => egui::Image::new(&tex)
                            .corner_radius(theme::RADIUS_SM)
                            .paint_at(ui, rect),
                        None => {
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                kind_icon,
                                egui::FontId::proportional(18.0),
                                accent,
                            );
                        }
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
                    // One per row: say which picture it adds.
                    let add = t("library-add-to-scene");
                    let basename = asset.path.rsplit('/').next().unwrap_or(asset.path.as_str());
                    if ui
                        .button(format!("{}  {add}", icons::ADD))
                        .named(format!("{add}: {basename}"))
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
