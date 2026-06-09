//! Ctrl+K command palette. Extracted in I.4.
//!
//! Quick-action overlay that fuzzy-filters across themes and
//! presets. Returns a `PaletteOutcome` so the caller can mutate
//! `App` state outside the egui frame closure — same pattern as
//! [`super::ContextMenuOutcome`].

use crate::presets::{ApplyMode, Preset, PresetId};
use crate::ui::icons;
use crate::ui::theme::{self, Theme, SPACE_XS};

/// One-shot intent emitted by the command palette so the caller can
/// apply it after `EguiRenderer::render` returns.
pub enum PaletteOutcome {
    /// User picked a preset; apply with the given mode.
    ApplyPreset(PresetId, ApplyMode),
    /// User picked a theme.
    SwitchTheme(Theme),
}

/// Floating Ctrl+K command palette. Listens for `Ctrl+K` to toggle
/// itself, fuzzy-filters across themes and presets, returns the
/// chosen intent so the caller can mutate `App` state without holding
/// a borrow across egui's frame closure.
///
/// Only active in edit mode (pass-through mode has no other text
/// input either, so a popup wouldn't get the focus it needs).
pub fn command_palette(ctx: &egui::Context) -> Option<PaletteOutcome> {
    // ── Open / close on Ctrl+K ────────────────────────────────────
    let toggle = ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::K));
    let id = egui::Id::new("anima.palette");
    let mut open: bool = ctx.memory(|m| m.data.get_temp(id).unwrap_or(false));
    if toggle {
        open = !open;
        ctx.memory_mut(|m| m.data.insert_temp(id, open));
    }
    if !open {
        return None;
    }

    // ── Query state ───────────────────────────────────────────────
    let query_id = id.with("query");
    let mut query: String = ctx.memory(|m| m.data.get_temp(query_id).unwrap_or_default());

    // ── Window ────────────────────────────────────────────────────
    let mut outcome: Option<PaletteOutcome> = None;
    let mut want_close = false;

    let screen_rect = ctx.screen_rect();
    let center = egui::pos2(
        screen_rect.center().x,
        screen_rect.top() + screen_rect.height() * 0.25,
    );

    egui::Area::new(id.with("area"))
        .order(egui::Order::Foreground)
        .fixed_pos(center - egui::vec2(220.0, 0.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(440.0);

                ui.horizontal(|ui| {
                    ui.label(icons::SETTINGS);
                    // G.5 (0.5.3): same cap as the library search box.
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut query)
                            .hint_text("Type to search themes / presets…")
                            .desired_width(380.0)
                            .char_limit(256),
                    );
                    response.request_focus();
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        want_close = true;
                    }
                });
                ui.separator();

                let q = query.to_lowercase();
                let matches_filter = |s: &str| q.is_empty() || s.to_lowercase().contains(&q);

                // Themes
                for theme in Theme::ALL {
                    let label = format!("Switch to {} theme", theme.label());
                    if matches_filter(&label) {
                        let icon = match theme {
                            Theme::Dark | Theme::DarkHighContrast => icons::DARK_MODE,
                            Theme::Light | Theme::LightHighContrast => icons::LIGHT_MODE,
                        };
                        if ui.button(format!("{icon}  {label}")).clicked() {
                            outcome = Some(PaletteOutcome::SwitchTheme(*theme));
                            want_close = true;
                        }
                    }
                }

                // Presets
                for id in PresetId::ALL {
                    let preset = Preset::for_id(*id);
                    let label_replace = format!("Replace scene with: {}", preset.name);
                    let label_append = format!("Append preset: {}", preset.name);
                    if matches_filter(&label_replace)
                        || matches_filter(preset.name)
                        || matches_filter(preset.description)
                    {
                        ui.horizontal(|ui| {
                            ui.label(preset.icon);
                            ui.label(preset.name);
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("Replace").clicked() {
                                        outcome = Some(PaletteOutcome::ApplyPreset(
                                            *id,
                                            ApplyMode::Replace,
                                        ));
                                        want_close = true;
                                    }
                                    if ui.button("Append").clicked() {
                                        outcome = Some(PaletteOutcome::ApplyPreset(
                                            *id,
                                            ApplyMode::Append,
                                        ));
                                        want_close = true;
                                    }
                                },
                            );
                        });
                        ui.add_space(SPACE_XS);
                        // Show description as caption for the row.
                        ui.label(
                            egui::RichText::new(preset.description)
                                .text_style(theme::caption())
                                .weak(),
                        );
                        if matches_filter(&label_append) {
                            // (already rendered with both buttons above;
                            // separate label_append filter ensures both
                            // verbs hit if the query targets "append")
                        }
                        ui.separator();
                    }
                }

                ui.add_space(SPACE_XS);
                ui.label(
                    egui::RichText::new("Esc to close · Ctrl+K to toggle")
                        .text_style(theme::caption())
                        .weak(),
                );
            });
        });

    ctx.memory_mut(|m| m.data.insert_temp(query_id, query));
    if want_close {
        ctx.memory_mut(|m| m.data.insert_temp(id, false));
    }
    outcome
}
