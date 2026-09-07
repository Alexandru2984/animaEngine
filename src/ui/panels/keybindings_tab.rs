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
    hotkey_backend: &str,
) {
    // ── Global-shortcut backend status (T.4) ──────────────────────────
    // Which mechanism delivers the *global* chords this session —
    // resolved at startup, updated live if the deferred portal
    // fallback fires. In-app (edit-mode) chords are unaffected.
    ui.add_space(SPACE_XS);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(t("keybindings-backend-label"))
                .small()
                .weak(),
        );
        ui.label(egui::RichText::new(hotkey_backend).small().strong())
            .on_hover_text(t("keybindings-backend-tooltip"));
    });
    if hotkey_backend.contains("portal") {
        ui.label(
            egui::RichText::new(t("keybindings-portal-restart-hint"))
                .small()
                .weak(),
        );
    }
    ui.add_space(SPACE_XS);

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

    // ── Per-action rows
    //
    // Deliberately NOT an `egui::Grid`. The chord column wraps, and a
    // `horizontal_wrapped` inside a grid cell reserved height for fewer
    // lines than it actually painted — so the next row's stripe was drawn
    // straight over the tail of the previous row. In practice the "+ Add"
    // button of any action with two chords was completely covered, i.e.
    // invisible and unclickable, in every locale.
    //
    // One `Frame` per action fixes it by construction: a frame reserves its
    // background shape and fills it in *after* laying out its contents, so
    // the stripe can never be smaller than what it sits behind.
    let (warn_color, caption_color, stripe) = {
        let v = ui.visuals();
        (
            crate::ui::theme::palette_of(ui.ctx()).semantic_warn,
            v.weak_text_color(),
            v.faint_bg_color,
        )
    };
    // Label column width. Wrapping keeps long localized labels — German's
    // "Bearbeitungsmodus umschalten" is the worst case — inside the column
    // instead of pushing the chords off the panel.
    let label_w = (ui.available_width() * 0.45).clamp(110.0, 240.0);

    for (i, &action) in Action::ALL.iter().enumerate() {
        let fill = if i % 2 == 1 {
            stripe
        } else {
            egui::Color32::TRANSPARENT
        };
        egui::Frame::new()
            .fill(fill)
            .corner_radius(theme::RADIUS_SM)
            .inner_margin(egui::Margin::symmetric(SPACE_S as i8, SPACE_XS as i8))
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    // ── Column 1: action label (localized)
                    ui.allocate_ui_with_layout(
                        egui::vec2(label_w, 0.0),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            ui.add(egui::Label::new(t(action.i18n_key())).wrap());
                        },
                    );
                    ui.add_space(SPACE_M);

                    // ── Column 3 first: the reset button pins to the right
                    // edge, and the chord column then wraps inside whatever
                    // width is left.
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        if ui
                            .small_button(icons::RESET)
                            .on_hover_text("Reset to default")
                            .clicked()
                        {
                            bindings.reset_action(action);
                            *config_dirty = true;
                        }

                        ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                            chord_cell(
                                ui,
                                ctx,
                                bindings,
                                config_dirty,
                                action,
                                recording_for,
                                recording_id,
                                &conflicts,
                                warn_color,
                                caption_color,
                            );
                        });
                    });
                });
            });
    }

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
                        .color(crate::ui::theme::palette_of(ui.ctx()).semantic_warn),
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

/// The chord column for one action: a chip per bound chord, each with its
/// own remove button, followed by the record affordance.
///
/// Every item is `TextWrapMode::Extend`. This column is narrow — the panel
/// starts at 320px and the label and reset button take their share — so
/// egui otherwise breaks text *inside* items, and chords came out as
/// "Ctrl+ / Shift / +H" with the add button's own label as "+ / Ad / d".
/// `Extend` stops the intra-item break so `horizontal_wrapped` wraps
/// between chips instead.
#[allow(clippy::too_many_arguments)]
fn chord_cell(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    bindings: &mut KeyBindings,
    config_dirty: &mut bool,
    action: Action,
    recording_for: Option<Action>,
    recording_id: egui::Id,
    conflicts: &[(KeyChord, Vec<Action>)],
    warn_color: egui::Color32,
    caption_color: egui::Color32,
) {
    ui.horizontal_wrapped(|ui| {
        let chords = bindings.chords_for(action);
        if chords.is_empty() {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(t("keybindings-unbound"))
                        .text_style(theme::caption())
                        .color(caption_color),
                )
                .wrap_mode(egui::TextWrapMode::Extend),
            );
        } else {
            for chord in &chords {
                let conflict = conflicts.iter().any(|(c, _)| c == chord);
                let mut chip =
                    egui::RichText::new(chord.display_str()).text_style(egui::TextStyle::Monospace);
                if conflict {
                    chip = chip.color(warn_color);
                }
                ui.add(egui::Label::new(chip).wrap_mode(egui::TextWrapMode::Extend));
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
            ui.add(
                egui::Label::new(
                    egui::RichText::new(t("keybindings-recording"))
                        .text_style(egui::TextStyle::Small)
                        .color(crate::ui::theme::palette_of(ui.ctx()).semantic_info),
                )
                .wrap_mode(egui::TextWrapMode::Extend),
            );
        } else if ui
            .add(
                egui::Button::new(format!("{}  {}", icons::PLUS, t("keybindings-add")))
                    .small()
                    .wrap_mode(egui::TextWrapMode::Extend),
            )
            .clicked()
        {
            ctx.memory_mut(|m| m.data.insert_temp(recording_id, action));
        }
    });
}
