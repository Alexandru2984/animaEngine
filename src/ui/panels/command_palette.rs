//! Ctrl+K command palette. Extracted in I.4.
//!
//! Quick-action overlay that filters across "Add file…", every action
//! that has a shortcut, themes and presets. Returns a `PaletteOutcome`
//! so the caller can mutate `App` state outside the egui frame closure —
//! same pattern as [`super::ContextMenuOutcome`].

use crate::keybindings::{Action, KeyBindings, ModifierNames};
use crate::presets::{ApplyMode, Preset, PresetId};
use crate::ui::icons;
use crate::ui::theme::{self, Theme, SPACE_XS};

/// One-shot intent emitted by the command palette so the caller can
/// apply it after `EguiRenderer::render` returns.
#[derive(Debug, Clone, PartialEq)]
pub enum PaletteOutcome {
    /// User picked a preset; apply with the given mode.
    ApplyPreset(PresetId, ApplyMode),
    /// User picked a theme.
    SwitchTheme(Theme),
    /// User picked an action; the caller runs it exactly as its shortcut
    /// would, through the same code.
    RunAction(Action),
    /// User picked "Add file…".
    AddFile,
    /// User picked a saved scene (`crate::scenes`).
    LoadScene(std::path::PathBuf),
}

/// Height of the scrolling result list. With every action listed the
/// palette is ~45 rows, several screens' worth unscrolled.
const LIST_HEIGHT: f32 = 360.0;

/// The palette's egui id; its open flag is stored under it.
fn palette_id() -> egui::Id {
    egui::Id::new("anima.palette")
}

/// Whether the palette is showing. While it is, every key belongs to it,
/// whether or not its search box still has the focus: Enter on a query
/// that matched nothing drops the focus, and the keys typed next used to
/// run as shortcuts — Escape closed the palette *and* left edit mode, and
/// a "q" would have quit.
pub fn command_palette_open(ctx: &egui::Context) -> bool {
    ctx.memory(|m| m.data.get_temp(palette_id()).unwrap_or(false))
}

/// Floating Ctrl+K command palette. Listens for `Ctrl+K` to toggle
/// itself, filters its rows by the query, returns the chosen intent so
/// the caller can mutate `App` state without holding a borrow across
/// egui's frame closure.
///
/// Actions show their first shortcut from `keybindings`, as the
/// Keybindings tab spells it; the ones that act on the selected
/// character are listed only when `has_selection`.
///
/// Only active in edit mode (pass-through mode has no other text
/// input either, so a popup wouldn't get the focus it needs).
pub fn command_palette(
    ctx: &egui::Context,
    keybindings: &KeyBindings,
    has_selection: bool,
) -> Option<PaletteOutcome> {
    // ── Open / close on Ctrl+K ────────────────────────────────────
    let toggle = ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::K));
    let id = palette_id();
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
            // Each open starts from the full list. The last query used to
            // stay, so reopening showed only what it had matched.
            ctx.memory_mut(|m| {
                m.data.insert_temp(id.with("query"), String::new());
                m.data.insert_temp(id.with("selected"), 0usize);
            });
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

    let rows = rows(&query, keybindings, has_selection);

    // ── Keyboard navigation state ─────────────────────────────────
    let sel_id = id.with("selected");
    let mut selected: usize = ctx.memory(|m| m.data.get_temp(sel_id).unwrap_or(0));
    // Set when the arrow keys move the selection, so the list scrolls to
    // keep it in view.
    let mut moved = false;
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
            moved = true;
        }
        if up {
            selected = selected.checked_sub(1).unwrap_or(rows.len() - 1);
            moved = true;
        }
        if enter {
            outcome = Some(rows[selected].outcome.clone());
            want_close = true;
        }
    }
    // Kept for the next frame. It never was: the arrow keys moved the
    // highlight for the one frame they were pressed in, and Enter — always
    // a later frame — read the old value and picked the first row.
    ctx.memory_mut(|m| m.data.insert_temp(sel_id, selected));

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
                    let placeholder = crate::i18n::t("palette-search-placeholder");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut query)
                            .hint_text(placeholder.as_str())
                            .desired_width(380.0)
                            .char_limit(256),
                    );
                    crate::ui::accessible::name_text_field(&response, &placeholder);
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

                let font = egui::TextStyle::Body.resolve(ui.style());
                let (strong, weak) = (ui.visuals().text_color(), ui.visuals().weak_text_color());
                egui::ScrollArea::vertical()
                    .max_height(LIST_HEIGHT)
                    .show(ui, |ui| {
                        for (i, row) in rows.iter().enumerate() {
                            let mut text = egui::text::LayoutJob::default();
                            let format = |color| egui::TextFormat {
                                font_id: font.clone(),
                                color,
                                ..Default::default()
                            };
                            text.append(
                                &format!("{}  {}", row.icon, row.label),
                                0.0,
                                format(strong),
                            );
                            if let Some(hint) = &row.hint {
                                text.append(&format!("    {hint}"), 0.0, format(weak));
                            }
                            let mut resp = ui.selectable_label(i == selected, text);
                            if i == selected && moved {
                                resp.scroll_to_me(Some(egui::Align::Center));
                            }
                            if let Some(hover) = row.hover {
                                resp = resp.on_hover_text(hover);
                            }
                            if resp.clicked() {
                                outcome = Some(row.outcome.clone());
                                want_close = true;
                            }
                        }
                    });

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

/// One Enter-able line of the palette.
struct Row {
    icon: &'static str,
    label: String,
    /// The action's shortcut, drawn dimmed after the label.
    hint: Option<String>,
    hover: Option<&'static str>,
    outcome: PaletteOutcome,
}

/// The rows matching `query`, in order: "Add file…", every action with a
/// shortcut, the themes, then per preset a Replace and an Append row — the
/// old two-buttons-per-row layout had no keyboard path to the second
/// button (F8).
fn rows(query: &str, keybindings: &KeyBindings, has_selection: bool) -> Vec<Row> {
    let q = query.to_lowercase();
    let matches_filter = |s: &str| q.is_empty() || s.to_lowercase().contains(&q);

    let mut rows: Vec<Row> = Vec::new();
    // Unix only, as the Scene tab's button: it needs the XDG portal.
    if cfg!(unix) {
        let label = crate::i18n::t("scene-add-file");
        if matches_filter(&label) {
            rows.push(Row {
                icon: icons::ADD,
                label,
                hint: None,
                hover: None,
                outcome: PaletteOutcome::AddFile,
            });
        }
    }
    let names = ModifierNames::localized();
    for &action in Action::ALL {
        if !action.in_palette() || (action.acts_on_selection() && !has_selection) {
            continue;
        }
        let label = crate::i18n::t(action.i18n_key());
        if !matches_filter(&label) {
            continue;
        }
        rows.push(Row {
            icon: icons::KEYBOARD,
            label,
            hint: keybindings
                .chords_for(action)
                .first()
                .map(|chord| chord.display_str(&names)),
            hover: None,
            outcome: PaletteOutcome::RunAction(action),
        });
    }
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
                icon,
                label,
                hint: None,
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
                icon: preset.icon,
                label: label_replace,
                hint: None,
                hover: Some(preset.description),
                outcome: PaletteOutcome::ApplyPreset(*pid, ApplyMode::Replace),
            });
        }
        if any || matches_filter(&label_append) {
            rows.push(Row {
                icon: preset.icon,
                label: label_append,
                hint: None,
                hover: Some(preset.description),
                outcome: PaletteOutcome::ApplyPreset(*pid, ApplyMode::Append),
            });
        }
    }
    for scene in crate::scenes::shelf() {
        let mut args = fluent::FluentArgs::new();
        args.set("name", scene.name.clone());
        let label = crate::i18n::t_args("palette-load-scene", &args);
        if matches_filter(&scene.name) || matches_filter(&label) {
            rows.push(Row {
                icon: icons::SCENE,
                label,
                hint: None,
                hover: None,
                outcome: PaletteOutcome::LoadScene(scene.path),
            });
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_actions(rows: &[Row]) -> Vec<Action> {
        rows.iter()
            .filter_map(|row| match row.outcome {
                PaletteOutcome::RunAction(action) => Some(action),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn selection_actions_are_listed_only_with_a_selection() {
        let keybindings = KeyBindings::default();
        let without = run_actions(&rows("", &keybindings, false));
        let with = run_actions(&rows("", &keybindings, true));
        for &action in Action::ALL {
            let expected = action.in_palette();
            assert_eq!(
                with.contains(&action),
                expected,
                "{action:?} with a selection"
            );
            assert_eq!(
                without.contains(&action),
                expected && !action.acts_on_selection(),
                "{action:?} without one"
            );
        }
    }

    #[test]
    fn an_action_row_shows_its_first_shortcut() {
        let keybindings = KeyBindings::default();
        let names = ModifierNames::localized();
        for row in rows("", &keybindings, true) {
            if let PaletteOutcome::RunAction(action) = row.outcome {
                let first = keybindings
                    .chords_for(action)
                    .first()
                    .map(|c| c.display_str(&names));
                assert_eq!(row.hint, first, "{action:?}");
            }
        }
    }

    #[test]
    fn a_query_that_matches_nothing_leaves_no_rows() {
        assert!(rows("zzzzzz", &KeyBindings::default(), true).is_empty());
    }

    fn key(key: egui::Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    /// A context styled as the app's, which the palette's caption needs.
    fn context() -> egui::Context {
        let ctx = egui::Context::default();
        theme::apply(&ctx, Theme::Dark);
        ctx
    }

    /// One frame of the palette with these events; its outcome.
    fn frame(ctx: &egui::Context, events: Vec<egui::Event>) -> Option<PaletteOutcome> {
        let modifiers = match events.first() {
            Some(egui::Event::Key { modifiers, .. }) => *modifiers,
            _ => egui::Modifiers::NONE,
        };
        let input = egui::RawInput {
            events,
            modifiers,
            ..Default::default()
        };
        let mut outcome = None;
        let _ = ctx.run(input, |ctx| {
            outcome = command_palette(ctx, &KeyBindings::default(), false);
        });
        outcome
    }

    fn toggle(ctx: &egui::Context) {
        let event = egui::Event::Key {
            key: egui::Key::K,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        };
        assert_eq!(frame(ctx, vec![event]), None);
    }

    #[test]
    fn enter_picks_the_row_the_arrows_moved_to() {
        let ctx = context();
        toggle(&ctx);
        // The search box takes the arrow keys from its second frame on;
        // an arrow in the first moves egui's focus to a row instead,
        // which Enter would then click. A person is never that fast.
        let _ = frame(&ctx, vec![]);
        assert!(command_palette_open(&ctx));
        // Each key in its own frame, as a person types them. The
        // highlight used to be forgotten after the frame that moved it,
        // so Enter always picked the first row.
        assert_eq!(frame(&ctx, vec![key(egui::Key::ArrowDown)]), None);
        assert_eq!(frame(&ctx, vec![key(egui::Key::ArrowDown)]), None);
        let expected = rows("", &KeyBindings::default(), false)
            .swap_remove(2)
            .outcome;
        assert_eq!(frame(&ctx, vec![key(egui::Key::Enter)]), Some(expected));
        assert!(!command_palette_open(&ctx), "picking closes the palette");
    }

    #[test]
    fn the_palette_holds_the_keyboard_until_it_closes() {
        let ctx = context();
        assert!(!command_palette_open(&ctx));
        toggle(&ctx);
        // Open, with nothing focused the next frame; still the palette's.
        let _ = frame(&ctx, vec![]);
        ctx.memory_mut(|m| m.stop_text_input());
        let _ = frame(&ctx, vec![]);
        assert!(command_palette_open(&ctx));
        assert_eq!(frame(&ctx, vec![key(egui::Key::Escape)]), None);
        assert!(!command_palette_open(&ctx));
    }

    #[test]
    fn reopening_starts_from_the_full_list() {
        let ctx = context();
        toggle(&ctx);
        let _ = frame(&ctx, vec![egui::Event::Text("zzzzzz".into())]);
        let query_id = palette_id().with("query");
        let query = || ctx.memory(|m| m.data.get_temp::<String>(query_id));
        assert_eq!(query().as_deref(), Some("zzzzzz"));
        toggle(&ctx);
        toggle(&ctx);
        assert_eq!(query().as_deref(), Some(""));
    }
}
