//! The welcome (1.5): on a first run, the first time edit mode opens, a
//! window to start from — which of the built-in characters are on the
//! desktop, or a ready-made scene instead; the language; whether the app
//! starts at login. Continue, or its ✕, and the tour goes on
//! (`onboarding::coach_marks`, whose steps in edit mode wait for it).
//!
//! Not a modal: the panel beside it works, and nothing in it has to be
//! chosen. Shown once — `OnboardingProgress::getting_started`, which an
//! upgrade reads as seen — and again after "Reset onboarding hints".

use super::library::{builtin_texture, LibraryOutcome};
use crate::i18n::t;
use crate::input::selection::SelectionState;
use crate::presets::{ApplyMode, Preset, PresetId};
use crate::scene::Scene;
use crate::ui::onboarding::OnboardingProgress;
use crate::ui::theme::{self, h2, RADIUS_MD, SPACE_L, SPACE_M, SPACE_S, SPACE_XS};

/// The side panel's width, which the window keeps clear of.
const PANEL_WIDTH: f32 = 320.0;

/// A character's tile: its picture over its name.
const TILE: egui::Vec2 = egui::vec2(76.0, 92.0);

/// The welcome, while it has not been seen. Characters toggled on are
/// added the Library's way (`library_outcome`); toggled off, every one
/// of them goes from the scene.
#[expect(clippy::too_many_arguments)]
pub(super) fn window(
    ctx: &egui::Context,
    progress: &mut OnboardingProgress,
    scene: &mut Scene,
    selection: &mut SelectionState,
    locale: &mut Option<String>,
    start_at_login: &mut bool,
    library_outcome: &mut Option<LibraryOutcome>,
    config_dirty: &mut bool,
) {
    if progress.getting_started {
        return;
    }
    let focused = egui::Id::new("anima.first-run.focused");
    let mut open = true;
    let mut done = false;
    // At the top, clear of the panel: the characters it adds come in
    // below the middle of the screen (`outcomes::add_builtin_character`).
    egui::Window::new(t("first-run-title"))
        .id(egui::Id::new("anima.first-run"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(
            egui::Align2::CENTER_TOP,
            egui::vec2(-PANEL_WIDTH / 2.0, SPACE_L),
        )
        .show(ctx, |ui| {
            ui.set_max_width(TILE.x * 5.0 + SPACE_XS * 4.0);
            ui.add(egui::Label::new(t("first-run-intro")).wrap());
            ui.add_space(SPACE_M);

            ui.label(egui::RichText::new(t("first-run-characters")).text_style(h2()));
            ui.label(
                egui::RichText::new(t("first-run-characters-hint"))
                    .text_style(theme::caption())
                    .weak(),
            );
            ui.add_space(SPACE_S);
            let mut first = None;
            for row in crate::demo::CHARACTERS.chunks(5) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = SPACE_XS;
                    for name in row {
                        let on = scene
                            .entities
                            .iter()
                            .any(|e| is_builtin(&e.asset_path, name));
                        let response = tile(ui, name, on);
                        first.get_or_insert(response.id);
                        if response.clicked() {
                            if on {
                                take_off(scene, selection, name);
                                *config_dirty = true;
                            } else if library_outcome.is_none() {
                                *library_outcome = Some(LibraryOutcome {
                                    asset_id: String::new(),
                                    relative_path: name.to_string(),
                                    display_name: t(&format!("demo-{name}")),
                                    builtin: true,
                                });
                            }
                        }
                    }
                });
            }
            // Into the window as it opens, so the keyboard — and a screen
            // reader — start where it does, not in the panel behind.
            if let Some(first) = first {
                if !ctx.data(|d| d.get_temp::<bool>(focused).unwrap_or(false)) {
                    ctx.memory_mut(|m| m.request_focus(first));
                    ctx.data_mut(|d| d.insert_temp(focused, true));
                }
            }
            ui.add_space(SPACE_S);
            if preset_picker(ui, scene, selection) {
                *config_dirty = true;
            }
            ui.add_space(SPACE_M);

            ui.label(egui::RichText::new(t("appearance-language-header")).text_style(h2()));
            ui.add_space(SPACE_XS);
            if super::appearance::language_picker(ui, locale) {
                *config_dirty = true;
            }
            ui.add_space(SPACE_S);
            if super::appearance::autostart_switch(ui, start_at_login) {
                *config_dirty = true;
            }
            ui.add_space(SPACE_M);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // The one way on: a primary button (design system §7.2),
                // whose colours are held to contrast in every theme.
                let palette = theme::palette_of(ui.ctx());
                let button = egui::Button::new(
                    egui::RichText::new(t("first-run-continue"))
                        .strong()
                        .color(palette.fg_inverse),
                )
                .fill(palette.accent_base)
                .min_size(egui::vec2(96.0, 28.0));
                if ui.add(button).clicked() {
                    done = true;
                }
            });
        });
    if done || !open {
        progress.getting_started = true;
        *config_dirty = true;
        // "Reset onboarding hints" brings the window back, focus and all.
        ctx.data_mut(|d| d.remove::<bool>(focused));
    }
}

/// Whether a character's pictures are the built-in `name`'s: as a first
/// run, a preset and the Library write it, or as the path they are at.
fn is_builtin(asset_path: &str, name: &str) -> bool {
    asset_path
        .strip_prefix("assets/demo/")
        .is_some_and(|rest| rest == name)
        || std::path::Path::new(asset_path) == crate::demo::dir().join(name)
}

/// Every one of the built-in `name` out of the scene, the selection kept
/// on what stays.
fn take_off(scene: &mut Scene, selection: &mut SelectionState, name: &str) {
    for idx in (0..scene.entities.len()).rev() {
        if is_builtin(&scene.entities[idx].asset_path, name) && scene.remove_entity(idx).is_some() {
            selection.removed(idx);
        }
    }
}

/// A character as a toggle: its picture over its name, pressed while it
/// is on the desktop. Read out as a toggle button with the name.
fn tile(ui: &mut egui::Ui, name: &str, on: bool) -> egui::Response {
    let label = t(&format!("demo-{name}"));
    let (rect, response) = ui.allocate_exact_size(TILE, egui::Sense::click());
    let enabled = ui.is_enabled();
    response
        .widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, enabled, on, &label));
    let response = response.on_hover_text(&label);
    if !ui.is_rect_visible(rect) {
        return response;
    }
    // On, as a selected row is (design system §7.7): the accent's tint
    // and a border in it, the name in the primary text colour. egui's
    // selected colours put the accent's text on its fill — 3.1:1 in the
    // dark high-contrast theme.
    let palette = theme::palette_of(ui.ctx());
    let (fill, stroke, colour) = if on {
        (
            palette.accent_subtle,
            egui::Stroke::new(2.0_f32, palette.accent_base),
            palette.fg_primary,
        )
    } else {
        let border = if response.hovered() {
            palette.border_focus
        } else {
            palette.border_strong
        };
        (
            palette.bg_elevated,
            egui::Stroke::new(1.0_f32, border),
            palette.fg_secondary,
        )
    };
    let painter = ui.painter();
    painter.rect(rect, RADIUS_MD, fill, stroke, egui::StrokeKind::Inside);
    if response.has_focus() {
        painter.rect_stroke(
            rect.expand(2.0),
            RADIUS_MD,
            egui::Stroke::new(2.0_f32, palette.border_focus),
            egui::StrokeKind::Outside,
        );
    }
    let picture = egui::Rect::from_center_size(
        egui::pos2(rect.center().x, rect.top() + 6.0 + 30.0),
        egui::vec2(60.0, 60.0),
    );
    if let Some(texture) = builtin_texture(ui.ctx(), name) {
        // Off, faded: what is on the desktop stands out.
        let tint = if on {
            egui::Color32::WHITE
        } else {
            egui::Color32::from_white_alpha(110)
        };
        painter.image(
            texture.id(),
            picture,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            tint,
        );
    }
    if on {
        painter.text(
            rect.right_top() + egui::vec2(-6.0, 4.0),
            egui::Align2::RIGHT_TOP,
            crate::ui::icons::SUCCESS,
            egui::FontId::proportional(14.0),
            palette.accent_base,
        );
    }
    let mut font = ui
        .style()
        .text_styles
        .get(&theme::caption())
        .cloned()
        .unwrap_or_else(|| egui::FontId::proportional(12.0));
    let mut name = painter.layout_no_wrap(label.clone(), font.clone(), colour);
    // "Paddenstoel", "Coniglietto": a long name smaller, not cut.
    let room = TILE.x - 6.0;
    if name.size().x > room {
        font.size *= room / name.size().x;
        name = painter.layout_no_wrap(label, font, colour);
    }
    painter.galley(
        egui::pos2(
            rect.center().x - name.size().x / 2.0,
            rect.bottom() - 6.0 - name.size().y,
        ),
        name,
        colour,
    );
    response
}

/// "Or start from a scene": a preset in place of the characters there
/// are. Returns whether one was picked.
fn preset_picker(ui: &mut egui::Ui, scene: &mut Scene, selection: &mut SelectionState) -> bool {
    let mut picked = None;
    ui.horizontal_wrapped(|ui| {
        ui.label(t("first-run-preset"));
        let placeholder = t("first-run-preset-pick");
        let combo = egui::ComboBox::from_id_salt("anima.first-run.preset")
            .selected_text(&placeholder)
            .show_ui(ui, |ui| {
                for id in PresetId::ALL {
                    let preset = Preset::for_id(*id);
                    let row = format!("{}  {}", preset.icon, preset.name);
                    if crate::ui::accessible::ComboOption::picked(
                        &ui.selectable_label(false, row)
                            .on_hover_text(&preset.description),
                    ) {
                        picked = Some(preset);
                    }
                }
            });
        crate::ui::accessible::name_combo(&combo.response, &t("first-run-preset"), &placeholder);
    });
    match picked {
        Some(preset) => {
            super::presets::apply_preset(scene, selection, &preset, ApplyMode::Replace);
            true
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::is_builtin;

    #[test]
    fn a_built_in_character_is_known_by_its_path() {
        assert!(is_builtin("assets/demo/bunny", "bunny"));
        assert!(!is_builtin("assets/demo/bunnyx", "bunny"));
        assert!(!is_builtin("assets/demo/bunny", "frog"));
        let made = crate::demo::dir().join("frog");
        assert!(is_builtin(&made.to_string_lossy(), "frog"));
        // A picture of your own that happens to share the name is not one.
        assert!(!is_builtin("/home/someone/pictures/frog", "frog"));
    }
}
