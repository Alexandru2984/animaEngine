//! Keybindings tab body. Extracted in I.7.
//!
//! Renders every action's live chord set, lets the user record /
//! remove / reset bindings, and surfaces conflict warnings inline
//! next to the conflicting chord.
//!
//! Recording state lives in `egui::Memory` so it survives the inevitable
//! re-builds of this widget tree without an extra field on `App`.

use crate::i18n::{t, t_args};
use crate::keybindings::{Action, KeyBindings, KeyChord};
use crate::ui::icons;
use crate::ui::theme::{self, SPACE_M, SPACE_S, SPACE_XS};

pub(super) fn keybindings_tab(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    bindings: &mut KeyBindings,
    config_dirty: &mut bool,
) {
    let recording_id = egui::Id::new("anima.kb.recording");
    let mut recording_for: Option<Action> = ctx.memory(|m| m.data.get_temp(recording_id));

    // While recording, intercept the first non-modifier key press as
    // the chord for the target action. Esc cancels. Repeat events are
    // ignored so holding a key doesn't keep firing captures.
    if let Some(action) = recording_for {
        let captured: Option<(egui::Key, egui::Modifiers)> = ctx.input(|i| {
            let mods = i.modifiers;
            i.events.iter().find_map(|e| {
                if let egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    ..
                } = e
                {
                    Some((*key, mods))
                } else {
                    None
                }
            })
        });
        if let Some((key, mods)) = captured {
            if key == egui::Key::Escape {
                recording_for = None;
            } else if let Some(chord) = KeyChord::from_egui(key, mods) {
                bindings.add_chord(action, chord);
                *config_dirty = true;
                recording_for = None;
            }
        }
    }
    // Persist (or clear) recording state for next frame.
    ctx.memory_mut(|m| match recording_for {
        Some(a) => m.data.insert_temp(recording_id, a),
        None => m.data.remove::<Action>(recording_id),
    });

    // ── Help blurb
    ui.label(
        egui::RichText::new(t("keybindings-help"))
            .text_style(theme::caption())
            .weak(),
    );
    ui.add_space(SPACE_S);

    // Pre-compute conflicts once per frame — the table queries it
    // per chord cell to colour the chip and surface a warning row.
    let conflicts = bindings.conflicts();

    // ── Per-action grid
    egui::Grid::new("anima.kb.grid")
        .num_columns(3)
        .spacing([SPACE_M, SPACE_S])
        .striped(true)
        .show(ui, |ui| {
            let (warn_color, caption_color) = {
                let v = ui.visuals();
                (egui::Color32::from_rgb(220, 180, 60), v.weak_text_color())
            };
            for &action in Action::ALL {
                // ── Column 1: action label (localized)
                ui.label(t(action.i18n_key()));

                // ── Column 2: chord chips + Record affordance
                ui.horizontal_wrapped(|ui| {
                    let chords = bindings.chords_for(action);
                    if chords.is_empty() {
                        ui.label(
                            egui::RichText::new(t("keybindings-unbound"))
                                .text_style(theme::caption())
                                .color(caption_color),
                        );
                    } else {
                        for chord in &chords {
                            let conflict = conflicts.iter().any(|(c, _)| c == chord);
                            let mut chip = egui::RichText::new(chord.display_str())
                                .text_style(egui::TextStyle::Monospace);
                            if conflict {
                                chip = chip.color(warn_color);
                            }
                            ui.label(chip);
                            if ui
                                .small_button(icons::CLOSE)
                                .on_hover_text("Remove this binding")
                                .clicked()
                            {
                                bindings.remove_chord(action, *chord);
                                *config_dirty = true;
                            }
                        }
                    }
                    if recording_for == Some(action) {
                        ui.label(
                            egui::RichText::new(t("keybindings-recording"))
                                .text_style(egui::TextStyle::Small)
                                .color(egui::Color32::from_rgb(100, 180, 220)),
                        );
                    } else if ui
                        .small_button(format!("{}  {}", icons::PLUS, t("keybindings-add")))
                        .clicked()
                    {
                        ctx.memory_mut(|m| m.data.insert_temp(recording_id, action));
                    }
                });

                // ── Column 3: per-row reset to defaults
                if ui
                    .small_button(icons::RESET)
                    .on_hover_text("Reset to default")
                    .clicked()
                {
                    bindings.reset_action(action);
                    *config_dirty = true;
                }

                ui.end_row();
            }
        });

    // ── Conflict summary banner
    if !conflicts.is_empty() {
        ui.add_space(SPACE_M);
        ui.separator();
        ui.add_space(SPACE_XS);
        for (chord, actions) in &conflicts {
            // Pick the first action as the "anchor" and list the rest
            // as the conflict source via t_args.
            let mut others = actions.iter().map(|a| t(a.i18n_key())).collect::<Vec<_>>();
            others.remove(0);
            let conflict_with = others.join(", ");
            let mut args = fluent::FluentArgs::new();
            args.set("action", conflict_with);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("{}  {}", icons::WARN, chord.display_str()))
                        .text_style(egui::TextStyle::Monospace)
                        .color(egui::Color32::from_rgb(220, 180, 60)),
                );
                ui.label(
                    egui::RichText::new(t_args("keybindings-conflict", &args))
                        .text_style(theme::caption()),
                );
            });
        }
    }

    // ── Footer: reset everything
    ui.add_space(SPACE_M);
    ui.separator();
    ui.add_space(SPACE_XS);
    if ui
        .button(format!("{}  {}", icons::RESET, t("keybindings-reset-all")))
        .clicked()
    {
        bindings.reset_all();
        *config_dirty = true;
    }
}
