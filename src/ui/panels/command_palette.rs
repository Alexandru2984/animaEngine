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
#[derive(Clone)]
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
    let gen_id = id.with("open_gen");
    if toggle {
        open = !open;
        let mut new_gen: Option<u32> = None;
        ctx.memory_mut(|m| {
            m.data.insert_temp(id, open);
            if open {
                // New open → new animation key, so the pop replays on
                // every open instead of only the first.
                let g = m.data.get_temp::<u32>(gen_id).unwrap_or(0).wrapping_add(1);
                m.data.insert_temp(gen_id, g);
                new_gen = Some(g);
            }
        });
        if let Some(g) = new_gen {
            // Seed the fresh key at 0 — egui returns the target as-is
            // the first time it sees an id, which would skip the fade.
            ctx.animate_value_with_time(id.with(("pop", g)), 0.0, 0.0);
            // One-shot focus grab for the query field. Grabbing every
            // frame (the old behavior) made Tab navigation impossible
            // — focus snapped back to the text field each frame (F8).
            ctx.memory_mut(|m| m.data.insert_temp(id.with("focus_pending"), true));
        }
    }
    if !open {
        return None;
    }
    // Pop-in: quick fade driven by the per-open generation key. The
    // duration goes through `motion::time`, so reduced motion makes it
    // instant.
    let open_gen: u32 = ctx.memory(|m| m.data.get_temp(gen_id).unwrap_or(0));
    let pop = ctx.animate_value_with_time(
        id.with(("pop", open_gen)),
        1.0,
        crate::ui::motion::time(ctx, 0.12),
    );

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

    // ── Action rows (flattened, filterable) ───────────────────────
    // One row = one Enter-able action. Themes first, then per preset
    // a Replace and an Append row — the old two-buttons-per-row
    // layout had no keyboard path to the second button (F8).
    let q = query.to_lowercase();
    let matches_filter = |s: &str| q.is_empty() || s.to_lowercase().contains(&q);

    struct Row {
        text: String,
        hover: Option<&'static str>,
        outcome: PaletteOutcome,
    }
    let mut rows: Vec<Row> = Vec::new();
    for theme in Theme::ALL {
        let mut args = fluent::FluentArgs::new();
        args.set("theme", theme.label());
        let label = crate::i18n::t_args("palette-switch-theme", &args);
        if matches_filter(&label) {
            let icon = match theme {
                Theme::Dark | Theme::DarkHighContrast => icons::DARK_MODE,
                Theme::Light | Theme::LightHighContrast => icons::LIGHT_MODE,
            };
            rows.push(Row {
                text: format!("{icon}  {label}"),
                hover: None,
                outcome: PaletteOutcome::SwitchTheme(*theme),
            });
        }
    }
    for pid in PresetId::ALL {
        let preset = Preset::for_id(*pid);
        let mut args = fluent::FluentArgs::new();
        args.set("preset", preset.name);
        let label_replace = crate::i18n::t_args("palette-replace-row", &args);
        let label_append = crate::i18n::t_args("palette-append-row", &args);
        let any = matches_filter(preset.name) || matches_filter(preset.description);
        if any || matches_filter(&label_replace) {
            rows.push(Row {
                text: format!("{}  {label_replace}", preset.icon),
                hover: Some(preset.description),
                outcome: PaletteOutcome::ApplyPreset(*pid, ApplyMode::Replace),
            });
        }
        if any || matches_filter(&label_append) {
            rows.push(Row {
                text: format!("{}  {label_append}", preset.icon),
                hover: Some(preset.description),
                outcome: PaletteOutcome::ApplyPreset(*pid, ApplyMode::Append),
            });
        }
    }

    // ── Keyboard navigation state ─────────────────────────────────
    let sel_id = id.with("selected");
    let mut selected: usize = ctx.memory(|m| m.data.get_temp(sel_id).unwrap_or(0));
    if !rows.is_empty() {
        selected = selected.min(rows.len() - 1);
        let (down, up, enter) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowDown),
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::Enter),
            )
        });
        if down {
            selected = (selected + 1) % rows.len();
        }
        if up {
            selected = selected.checked_sub(1).unwrap_or(rows.len() - 1);
        }
        if enter {
            outcome = Some(rows[selected].outcome.clone());
            want_close = true;
        }
    }

    egui::Area::new(id.with("area"))
        .order(egui::Order::Foreground)
        .fixed_pos(center - egui::vec2(220.0, 0.0))
        .show(ctx, |ui| {
            ui.set_opacity(pop);
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(440.0);

                ui.horizontal(|ui| {
                    ui.label(icons::SETTINGS);
                    // G.5 (0.5.3): same cap as the library search box.
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut query)
                            .hint_text(crate::i18n::t("palette-search-placeholder"))
                            .desired_width(380.0)
                            .char_limit(256),
                    );
                    let focus_id = id.with("focus_pending");
                    if ctx.memory(|m| m.data.get_temp(focus_id).unwrap_or(false)) {
                        response.request_focus();
                        ctx.memory_mut(|m| m.data.insert_temp(focus_id, false));
                    }
                    if response.changed() {
                        // New filter → selection back to the top.
                        ctx.memory_mut(|m| m.data.insert_temp(sel_id, 0usize));
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        want_close = true;
                    }
                });
                ui.separator();

                for (i, row) in rows.iter().enumerate() {
                    let mut resp = ui.selectable_label(i == selected, &row.text);
                    if let Some(hover) = row.hover {
                        resp = resp.on_hover_text(hover);
                    }
                    if resp.clicked() {
                        outcome = Some(row.outcome.clone());
                        want_close = true;
                    }
                }

                ui.add_space(SPACE_XS);
                ui.label(
                    egui::RichText::new(crate::i18n::t("palette-footer-hint"))
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
