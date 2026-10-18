//! Inspector tab — properties for the selected entity. Extracted in I.10.
//!
//! The tab consists of a sticky header, two quick-toggle checkboxes
//! (Visible / Gravity) and four CollapsingHeader sections: Position,
//! Appearance, Animation, Behavior. Each section's open-state is
//! persisted in `CollapseState` so the layout survives restarts.
//!
//! Behavior dropdown rebuilds each variant from sensible defaults on
//! switch so users can fiddle without first reading the data sheet.

use super::monitor::entity_monitor_picker;
use crate::behavior::Behavior;
use crate::i18n::{t, t_args};
use crate::input::selection::SelectionState;
use crate::monitor::MonitorInfo;
use crate::scene::Scene;
use crate::ui::accessible::{AccessibleName, ComboOption};
use crate::ui::collapse::CollapseState;
use crate::ui::icons;
use crate::ui::onboarding::{self, OnboardingProgress};
use crate::ui::states;
use crate::ui::theme::{self, h2, SPACE_M, SPACE_S, SPACE_XS};

pub(super) fn inspector_tab(
    ui: &mut egui::Ui,
    scene: &mut Scene,
    selection: &mut SelectionState,
    config_dirty: &mut bool,
    onboarding: &mut OnboardingProgress,
    monitors: &[MonitorInfo],
    collapse_state: &mut CollapseState,
) {
    let selected_idx = selection.selected_index();
    // C.9: effective-transform hint, computed before the mutable
    // entity borrow below (both run through `scene`).
    let group_hint: Option<String> = selected_idx
        .and_then(|idx| scene.entities.get(idx))
        .and_then(|e| {
            scene
                .groups
                .iter()
                .find(|g| g.member_ids.iter().any(|m| m == &e.id))
        })
        .filter(|g| g.offset_x != 0.0 || g.offset_y != 0.0 || g.scale != 1.0)
        .map(|g| {
            let mut args = fluent::FluentArgs::new();
            args.set("group", g.name.clone());
            args.set(
                "transform",
                format!("{:+.0},{:+.0} ×{:.2}", g.offset_x, g.offset_y, g.scale),
            );
            crate::i18n::t_args("inspector-group-hint", &args)
        });
    // Several selected: say which one this panel edits, and that the rest
    // of the editing reaches them all.
    let also_selected = selection.count().saturating_sub(1);
    // Whom the selected one may follow, with their names — before the
    // mutable borrow below (`behavior::follow_candidates`).
    let leaders: Vec<(String, String)> = selected_idx
        .and_then(|idx| scene.entities.get(idx))
        .map(|selected| {
            let chains: Vec<(String, Option<String>)> = scene
                .entities
                .iter()
                .map(|e| {
                    let follows = match &e.behavior {
                        Behavior::FollowCharacter { target, .. } => Some(target.clone()),
                        _ => None,
                    };
                    (e.id.clone(), follows)
                })
                .collect();
            crate::behavior::follow_candidates(&chains, &selected.id)
                .into_iter()
                .filter_map(|id| {
                    let name = scene.entities.iter().find(|e| e.id == id)?.name.clone();
                    Some((id, name))
                })
                .collect()
        })
        .unwrap_or_default();
    // Whom it may play tag with: anyone else (`crate::tag`).
    let partners: Vec<(String, String)> = selected_idx
        .and_then(|idx| scene.entities.get(idx))
        .map(|selected| {
            scene
                .entities
                .iter()
                .filter(|e| e.id != selected.id)
                .map(|e| (e.id.clone(), e.name.clone()))
                .collect()
        })
        .unwrap_or_default();
    let mut arrange_picked = None;
    match selected_idx.and_then(|idx| scene.entities.get_mut(idx).map(|e| (idx, e))) {
        Some((_idx, entity)) => {
            if also_selected > 0 {
                let mut args = fluent::FluentArgs::new();
                args.set("count", also_selected + 1);
                args.set("name", entity.name.clone());
                ui.label(
                    egui::RichText::new(crate::i18n::t_args("inspector-multi-selected", &args))
                        .text_style(theme::caption())
                        .color(ui.visuals().weak_text_color()),
                );
                ui.add_space(SPACE_XS);
                arrange_picked = super::arrange::arrange_buttons(ui, also_selected + 1);
                ui.add_space(SPACE_XS);
            }
            // Hint about V / G shortcuts, sitting above the quick-toggle
            // row so the visual proximity makes the connection.
            if onboarding::hint(
                ui,
                &t("onboarding-quick-toggles"),
                &mut onboarding.quick_toggles,
            ) {
                *config_dirty = true;
            }
            let changed = entity_inspector(
                ui,
                entity,
                monitors,
                collapse_state,
                config_dirty,
                group_hint.as_deref(),
                Others {
                    leaders: &leaders,
                    partners: &partners,
                },
            );
            if changed.any() {
                *config_dirty = true;
            }
            if changed.touches_visibility_or_z_order {
                scene.mark_visible_dirty();
            }
        }
        None => states::empty(
            ui,
            icons::CURSOR,
            &t("inspector-nothing-selected-headline"),
            &t("inspector-nothing-selected-hint"),
        ),
    }
    if let Some(how) = arrange_picked {
        *config_dirty |= crate::input::arrange::arrange(scene, &selection.selected_indices(), how);
    }
}

/// Tracks which fields of an entity were modified, so the caller can mark
/// the right caches dirty without scanning the entity afterwards.
#[derive(Default)]
struct EntityChange {
    any_field: bool,
    touches_visibility_or_z_order: bool,
}

impl EntityChange {
    fn any(&self) -> bool {
        self.any_field || self.touches_visibility_or_z_order
    }
}

fn entity_inspector(
    ui: &mut egui::Ui,
    entity: &mut crate::entity::Entity,
    monitors: &[MonitorInfo],
    collapse_state: &mut CollapseState,
    config_dirty: &mut bool,
    group_hint: Option<&str>,
    others: Others<'_>,
) -> EntityChange {
    let mut change = EntityChange::default();

    // ── Header: entity name + id ──────────────────────────────────────
    ui.label(egui::RichText::new(&entity.name).text_style(h2()));
    ui.label(
        egui::RichText::new(format!("id: {}", entity.id))
            .text_style(egui::TextStyle::Monospace)
            .text_style(theme::caption())
            .weak(),
    );
    ui.add_space(SPACE_M);

    // Quick-toggle row: Visible + Gravity. These are the two
    // booleans users flip most often, so they stay at the top — the
    // collapsibles below host the slider-heavy detail.
    ui.horizontal(|ui| {
        if ui
            .checkbox(&mut entity.visible, t("inspector-visible"))
            .changed()
        {
            change.touches_visibility_or_z_order = true;
        }
        let mut gravity = entity.physics.enabled;
        if ui.checkbox(&mut gravity, t("inspector-gravity")).changed() {
            if gravity {
                entity.physics.enable();
            } else {
                entity.physics.disable();
            }
            change.any_field = true;
        }
    });
    ui.add_space(SPACE_S);

    // ── Collapsibles ──────────────────────────────────────────────────
    super::section(
        ui,
        "inspector_position",
        &t("inspector-section-position"),
        &mut collapse_state.inspector_position,
        config_dirty,
        |ui| {
            if ui
                .add(
                    egui::Slider::new(&mut entity.x, -200.0..=4000.0)
                        .text(t("inspector-x"))
                        .suffix(" px"),
                )
                .changed()
            {
                change.any_field = true;
            }
            if ui
                .add(
                    egui::Slider::new(&mut entity.y, -200.0..=4000.0)
                        .text(t("inspector-y"))
                        .suffix(" px"),
                )
                .changed()
            {
                change.any_field = true;
            }
            ui.horizontal(|ui| {
                let label = ui.label(t("inspector-z-index"));
                if ui
                    .add(
                        egui::DragValue::new(&mut entity.z_index)
                            .speed(1.0)
                            .range(-10_000..=10_000),
                    )
                    .labelled_by(label.id)
                    .changed()
                {
                    change.touches_visibility_or_z_order = true;
                }
            });
            // Monitor pin lives in the Position section because it's
            // conceptually a 3rd axis: x / y / which-screen.
            let previous = entity.monitor.clone();
            if entity_monitor_picker(ui, &mut entity.monitor, monitors) {
                super::monitor::move_to_pinned_monitor(entity, previous.as_deref(), monitors);
                change.any_field = true;
            }
            if let Some(hint) = group_hint {
                ui.label(egui::RichText::new(hint).small().weak());
            }
        },
    );

    super::section(
        ui,
        "inspector_appearance",
        &t("inspector-section-appearance"),
        &mut collapse_state.inspector_appearance,
        config_dirty,
        |ui| {
            // Each label over its slider and value box (`labelled_slider`).
            let caption = ui.label(t("inspector-scale"));
            let scale_resp =
                labelled_slider(ui, caption.id, &mut entity.scale, 0.1..=5.0, |field| {
                    field.speed(0.01).suffix("\u{d7}")
                })
                .on_hover_text(t("inspector-double-click-reset-hint"));
            if scale_resp.double_clicked() {
                entity.scale = 1.0;
                change.any_field = true;
            } else if scale_resp.changed() {
                change.any_field = true;
            }
            // Opacity reads as a percentage; storage stays 0..1.
            let caption = ui.label(t("inspector-opacity"));
            let opacity_resp =
                labelled_slider(ui, caption.id, &mut entity.opacity, 0.0..=1.0, percent)
                    .on_hover_text(t("inspector-double-click-reset-hint"));
            if opacity_resp.double_clicked() {
                entity.opacity = 1.0;
                change.any_field = true;
            } else if opacity_resp.changed() {
                change.any_field = true;
            }
            if hours_controls(ui, entity) {
                change.any_field = true;
            }
        },
    );

    super::section(
        ui,
        "inspector_animation",
        &t("inspector-section-animation"),
        &mut collapse_state.inspector_animation,
        config_dirty,
        |ui| {
            let mut fps = entity.animation().fps;
            let fps_resp = ui
                .add(
                    egui::Slider::new(&mut fps, 1.0..=60.0)
                        .text(t("inspector-fps"))
                        .suffix(" fps"),
                )
                .on_hover_text(t("inspector-double-click-reset-hint"));
            if fps_resp.double_clicked() {
                entity.animation_mut().set_fps(crate::config::default_fps());
                change.any_field = true;
            } else if fps_resp.changed() {
                entity.animation_mut().set_fps(fps);
                change.any_field = true;
            }
            let mut playing = entity.animation().playing;
            if ui.checkbox(&mut playing, t("inspector-playing")).changed() {
                entity.animation_mut().playing = playing;
                change.any_field = true;
            }
            if easing_picker(ui, &mut entity.animation_mut().easing) {
                change.any_field = true;
            }
        },
    );

    super::section(
        ui,
        "inspector_behavior",
        &t("inspector-section-behavior"),
        &mut collapse_state.inspector_behavior,
        config_dirty,
        |ui| {
            if behavior_picker(ui, &mut entity.behavior, others) {
                change.any_field = true;
            }
        },
    );

    super::section(
        ui,
        "inspector_talk",
        &t("inspector-section-talk"),
        &mut collapse_state.inspector_talk,
        config_dirty,
        |ui| {
            if talk_controls(ui, entity) {
                change.any_field = true;
            }
        },
    );

    change
}

/// The lines a character says (`crate::phrases`): one per line of a text
/// box, how often it speaks up on its own, and a button to hear one.
/// Returns whether the lines or the pace changed.
fn talk_controls(ui: &mut egui::Ui, entity: &mut crate::entity::Entity) -> bool {
    let mut changed = false;
    ui.add(
        egui::Label::new(
            egui::RichText::new(t("inspector-talk-hint"))
                .text_style(theme::caption())
                .weak(),
        )
        .wrap(),
    );
    ui.add_space(SPACE_XS);
    // What is typed is kept as typed — a blank line on the way to the next
    // one, say — and the lines are taken from it, tidied.
    let key = egui::Id::new(("anima.inspector.talk", entity.id.as_str()));
    let tidied = |text: &str| {
        let mut talk = crate::config::Talk {
            phrases: text.lines().map(str::to_string).collect(),
            ..Default::default()
        };
        crate::phrases::tidy(&mut talk);
        talk.phrases
    };
    let mut text: String = ui
        .data(|d| d.get_temp::<String>(key))
        // Changed elsewhere — an undo, a reload: start from the lines.
        .filter(|kept| tidied(kept) == entity.talk.phrases)
        .unwrap_or_else(|| entity.talk.phrases.join("\n"));
    let field = ui.add(
        egui::TextEdit::multiline(&mut text)
            .hint_text(t("inspector-talk-placeholder"))
            .desired_rows(4)
            .desired_width(f32::INFINITY),
    );
    crate::ui::accessible::name_text_field(&field, &t("inspector-section-talk"));
    // {time}, {day}, {date}, {name} (`crate::placeholders`).
    ui.add(
        egui::Label::new(
            egui::RichText::new(crate::placeholders::hint())
                .text_style(theme::caption())
                .weak(),
        )
        .wrap(),
    );
    if field.changed() {
        let phrases = tidied(&text);
        if phrases != entity.talk.phrases {
            entity.talk.phrases = phrases;
            entity.chatter.last = None;
            changed = true;
        }
    }
    ui.data_mut(|d| d.insert_temp(key, text));

    let label = |minutes: u32| {
        if minutes == 0 {
            t("inspector-talk-only-poked")
        } else {
            let mut args = fluent::FluentArgs::new();
            args.set("minutes", minutes);
            crate::i18n::t_args("inspector-talk-every", &args)
        }
    };
    ui.horizontal(|ui| {
        let caption = t("inspector-talk-pace");
        ui.label(&caption);
        let combo =
            egui::ComboBox::from_id_salt(("anima.inspector.talk-every", entity.id.as_str()))
                .selected_text(label(entity.talk.every_minutes))
                .show_ui(ui, |ui| {
                    for minutes in crate::phrases::EVERY_CHOICES {
                        let option = ui.selectable_value(
                            &mut entity.talk.every_minutes,
                            minutes,
                            label(minutes),
                        );
                        if option.picked() && option.changed() {
                            changed = true;
                        }
                    }
                });
        // Named for a screen reader: `labelled_by` does not reach AT-SPI.
        crate::ui::accessible::name_combo(
            &combo.response,
            &caption,
            &label(entity.talk.every_minutes),
        );
    });
    if ui
        .add_enabled(
            !entity.talk.phrases.is_empty(),
            egui::Button::new(t("inspector-talk-say-now")),
        )
        .clicked()
    {
        entity.say_a_phrase(std::time::Instant::now());
    }
    ui.add_space(SPACE_S);
    if sound_controls(ui, entity) {
        changed = true;
    }
    ui.add_space(SPACE_S);
    if bubble_controls(ui, entity) {
        changed = true;
    }
    changed
}

/// How its bubbles look (`crate::speech`): their colour and text size,
/// and a way back to how they come. Returns whether either changed.
fn bubble_controls(ui: &mut egui::Ui, entity: &mut crate::entity::Entity) -> bool {
    use crate::speech::{
        colour_text, parse_colour, TEXT_SIZE, TEXT_SIZE_MAX, TEXT_SIZE_MIN, WHITE,
    };
    let mut changed = false;
    ui.horizontal(|ui| {
        let caption = ui.label(t("inspector-bubble-colour"));
        let mut rgb = entity
            .talk
            .bubble
            .as_deref()
            .and_then(parse_colour)
            .unwrap_or(WHITE);
        if egui::color_picker::color_edit_button_srgb(ui, &mut rgb)
            .labelled_by(caption.id)
            .changed()
        {
            entity.talk.bubble = (rgb != WHITE).then(|| colour_text(rgb));
            changed = true;
        }
        let custom = entity.talk.bubble.is_some() || entity.talk.text_size.is_some();
        if ui
            .add_enabled(
                custom,
                egui::Button::new(t("inspector-bubble-reset")).small(),
            )
            .clicked()
        {
            entity.talk.bubble = None;
            entity.talk.text_size = None;
            changed = true;
        }
    });
    // The label over the slider: to its right, in longer languages, it
    // made the row wider than the panel, which then grew to fit it.
    let caption = ui.label(t("inspector-bubble-size"));
    let mut size = entity.talk.text_size.unwrap_or(TEXT_SIZE);
    if labelled_slider(
        ui,
        caption.id,
        &mut size,
        TEXT_SIZE_MIN..=TEXT_SIZE_MAX,
        |v| v.speed(0.1).suffix(" pt"),
    )
    .changed()
    {
        entity.talk.text_size = ((size - TEXT_SIZE).abs() > 0.01).then_some(size);
        changed = true;
    }
    changed
}

/// The sound a tap makes (`crate::sounds`): none, or a file from the
/// asset library — picking one plays it — and a button to hear it again.
/// Returns whether it changed.
fn sound_controls(ui: &mut egui::Ui, entity: &mut crate::entity::Entity) -> bool {
    let mut changed = false;
    let choices = crate::sounds::choices();
    let none = t("inspector-sound-none");
    // Wrapped, not one row: in longer languages a row wider than the
    // panel made the panel grow to fit it.
    ui.horizontal_wrapped(|ui| {
        let caption = t("inspector-sound-label");
        ui.label(&caption);
        let current = if entity.talk.sound.is_empty() {
            none.clone()
        } else {
            entity.talk.sound.clone()
        };
        let combo = egui::ComboBox::from_id_salt(("anima.inspector.sound", entity.id.as_str()))
            .selected_text(current.clone())
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(entity.talk.sound.is_empty(), none.as_str())
                    .picked()
                    && !entity.talk.sound.is_empty()
                {
                    entity.talk.sound.clear();
                    changed = true;
                }
                for sound in &choices {
                    if ui
                        .selectable_label(entity.talk.sound == *sound, sound.as_str())
                        .picked()
                        && entity.talk.sound != *sound
                    {
                        entity.talk.sound = sound.clone();
                        entity.want_tap_sound();
                        changed = true;
                    }
                }
            });
        crate::ui::accessible::name_combo(&combo.response, &caption, &current);
        if ui
            .add_enabled(
                !entity.talk.sound.is_empty(),
                egui::Button::new(t("inspector-sound-play")),
            )
            .clicked()
        {
            entity.want_tap_sound();
        }
    });
    if choices.is_empty() {
        let mut args = fluent::FluentArgs::new();
        args.set(
            "path",
            crate::sounds::root()
                .map(|r| r.display().to_string())
                .unwrap_or_default(),
        );
        ui.add(
            egui::Label::new(
                egui::RichText::new(crate::i18n::t_args("inspector-sound-hint", &args))
                    .text_style(theme::caption())
                    .weak(),
            )
            .wrap(),
        );
    }
    // How loud this one is, under the global volume (`Talk::volume`):
    // its tap, what it says for the reminders and the focus timer, its
    // script's sounds.
    let caption = ui.label(t("appearance-sounds-volume"));
    let mut volume = entity.talk.volume.unwrap_or(1.0);
    if labelled_slider(ui, caption.id, &mut volume, 0.0..=1.0, percent).changed() {
        entity.talk.volume = (volume < 0.995).then_some(volume);
        changed = true;
    }
    changed
}

/// Its own hours (`crate::hours`): a switch, and from when to when.
/// Returns whether they changed.
fn hours_controls(ui: &mut egui::Ui, entity: &mut crate::entity::Entity) -> bool {
    let mut on = entity.hours.is_some();
    let mut changed = false;
    if ui
        .checkbox(&mut on, t("inspector-hours-label"))
        .on_hover_text(t("inspector-hours-hint"))
        .changed()
    {
        entity.hours = on.then(crate::hours::Hours::default);
        changed = true;
    }
    if let Some(hours) = entity.hours.as_mut() {
        ui.horizontal(|ui| {
            changed |=
                super::appearance::hhmm_field(ui, &mut hours.from, &t("inspector-hours-from"));
            ui.add_space(SPACE_XS);
            changed |= super::appearance::hhmm_field(ui, &mut hours.to, &t("inspector-hours-to"));
        });
    }
    changed
}

/// A slider and its value box on one line, both named after the label
/// `caption` set over them, as one response.
///
/// A slider's own text beside it made the row wider than the panel in
/// longer languages, and the panel grew to fit; moved over it, the value
/// box egui draws had no name a screen reader could read. So the box is
/// drawn here, named like the slider.
/// A 0..1 number box read and typed as a percentage: "90%", or "90".
fn percent(field: egui::DragValue<'_>) -> egui::DragValue<'_> {
    field
        .speed(0.01)
        .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
        .custom_parser(|s| {
            s.trim_end_matches('%')
                .trim()
                .parse::<f64>()
                .ok()
                .map(|p| p / 100.0)
        })
}

fn labelled_slider(
    ui: &mut egui::Ui,
    caption: egui::Id,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    value_box: impl FnOnce(egui::DragValue<'_>) -> egui::DragValue<'_>,
) -> egui::Response {
    ui.horizontal(|ui| {
        let slider = ui
            .add(egui::Slider::new(value, range.clone()).show_value(false))
            .labelled_by(caption);
        let number = ui
            .add(value_box(egui::DragValue::new(value).range(range)))
            .labelled_by(caption);
        slider | number
    })
    .inner
}

/// Easing-curve dropdown for `Animation::easing`. `None` represents
/// "Linear (default)" — the 0.2 behaviour. Returns `true` when the
/// selection changed so the caller can flag the config dirty.
fn easing_picker(ui: &mut egui::Ui, easing: &mut Option<crate::anim::EasingCurve>) -> bool {
    use crate::anim::EasingCurve;
    let mut changed = false;
    let active_label = match easing {
        None => t("easing-linear"),
        Some(c) => t(c.i18n_key()),
    };
    ui.horizontal(|ui| {
        ui.label(t("animation-easing-label"));
        let combo_value = active_label.clone();
        let combo = egui::ComboBox::from_id_salt("anima.animation().easing")
            .selected_text(active_label)
            .show_ui(ui, |ui| {
                let is_linear = easing.is_none() || matches!(easing, Some(EasingCurve::Linear));
                if ui.selectable_label(is_linear, t("easing-linear")).picked() && !is_linear {
                    *easing = None;
                    changed = true;
                }
                for &c in EasingCurve::ALL {
                    if matches!(c, EasingCurve::Linear) {
                        continue;
                    }
                    let is_current = matches!(easing, Some(x) if *x == c);
                    if ui.selectable_label(is_current, t(c.i18n_key())).picked() && !is_current {
                        *easing = Some(c);
                        changed = true;
                    }
                }
            });
        crate::ui::accessible::name_combo(
            &combo.response,
            &t("animation-easing-label"),
            &combo_value,
        );
    });
    changed
}

/// Behavior dropdown + variant-specific sliders. Returns `true` when the
/// user touched anything in this section.
/// Controls for `Behavior::Script`: where the script lives, and the
/// author-defined tunables passed to it.
///
/// The path is typed rather than picked from a list. The asset library
/// indexes media, not `.rhai` files, so there is nothing to enumerate yet;
/// a picker wants a script index first, and inventing one here would put
/// discovery logic in a UI panel.
///
/// Failures are not shown here either — they arrive as a toast, from
/// `ScriptHost::take_new_failures`. Surfacing them inline would mean
/// threading the host through `panels::settings`, which already carries
/// about twenty parameters and is called from both backends, for a message
/// the user has already been given.
fn script_controls(
    ui: &mut egui::Ui,
    path: &mut String,
    params: &mut std::collections::BTreeMap<String, f64>,
) -> bool {
    let mut changed = false;

    ui.horizontal(|ui| {
        let label = ui.label(t("behavior-script-path-label"));
        if ui
            .text_edit_singleline(path)
            .labelled_by(label.id)
            .changed()
        {
            changed = true;
        }
    });
    // The hint ends in the library path, so it needs the argument —
    // rendered through `t()` it printed a literal `{ $path }` under the
    // field, which is worse than saying nothing (R34).
    let mut path_args = fluent::FluentArgs::new();
    path_args.set(
        "path",
        crate::asset_library::asset_root_hint()
            .display()
            .to_string(),
    );
    ui.label(
        egui::RichText::new(t_args("behavior-script-path-hint", &path_args))
            .text_style(theme::caption())
            .color(ui.visuals().weak_text_color()),
    );

    ui.add_space(SPACE_S);
    let params_label = ui.label(
        egui::RichText::new(t("behavior-script-params"))
            .text_style(theme::caption())
            .weak(),
    );

    // Collected and applied after the loop: the rows borrow `params`.
    let mut remove: Option<String> = None;
    for (name, value) in params.iter_mut() {
        ui.horizontal(|ui| {
            let remove_label = t("behavior-script-remove-param");
            if ui
                .small_button(icons::TRASH)
                .named(format!("{remove_label}: {name}"))
                .on_hover_text(remove_label)
                .clicked()
            {
                remove = Some(name.clone());
            }
            let label = ui.add(
                egui::Label::new(
                    egui::RichText::new(name.as_str()).text_style(egui::TextStyle::Monospace),
                )
                .wrap_mode(egui::TextWrapMode::Extend),
            );
            if ui
                .add(egui::DragValue::new(value).speed(1.0))
                .labelled_by(label.id)
                .changed()
            {
                changed = true;
            }
        });
    }
    if let Some(name) = remove {
        params.remove(&name);
        changed = true;
    }

    // New-parameter row. The name lives in egui memory rather than on the
    // entity, so a half-typed name isn't written into the config.
    let draft_id = ui.id().with("script_param_draft");
    let mut draft: String = ui.memory(|m| m.data.get_temp(draft_id).unwrap_or_default());
    ui.horizontal(|ui| {
        // Named after the "Parameters" caption: the hint inside says
        // "name", but a hint is not a name to a screen reader.
        ui.add(
            egui::TextEdit::singleline(&mut draft)
                .hint_text(t("behavior-script-param-name"))
                .desired_width(110.0),
        )
        .labelled_by(params_label.id);
        let can_add = !draft.trim().is_empty() && !params.contains_key(draft.trim());
        if ui
            .add_enabled(
                can_add,
                egui::Button::new(format!(
                    "{}  {}",
                    icons::PLUS,
                    t("behavior-script-add-param")
                ))
                .small()
                .wrap_mode(egui::TextWrapMode::Extend),
            )
            .clicked()
        {
            params.insert(draft.trim().to_string(), 0.0);
            draft.clear();
            changed = true;
        }
    });
    ui.memory_mut(|m| m.data.insert_temp(draft_id, draft));

    changed
}

/// Who else is in the scene, by id and name, for the behaviors that name
/// another character.
#[derive(Clone, Copy)]
struct Others<'a> {
    /// Whom it may follow (`behavior::follow_candidates`).
    leaders: &'a [(String, String)],
    /// Whom it may play tag with (`crate::tag`).
    partners: &'a [(String, String)],
}

/// `others`: whom this character may follow or play with.
fn behavior_picker(ui: &mut egui::Ui, behavior: &mut Behavior, others: Others<'_>) -> bool {
    let Others { leaders, partners } = others;
    let mut changed = false;

    // ComboBox with the three concrete variants. selectable_value compares
    // via PartialEq, so picking the same variant a second time is a no-op.
    let current_label = behavior_label_with_icon(behavior);
    let combo_value = current_label.clone();
    let combo = egui::ComboBox::from_id_salt("behavior_picker")
        .selected_text(current_label)
        .show_ui(ui, |ui| {
            let prev = behavior.clone();
            // The defaults a freshly-picked variant starts from. Looping
            // over them keeps each option's label going through the same
            // `behavior_label_with_icon` as the closed combo's selected
            // text, so the two can't drift — the dropdown used to build
            // its own hardcoded English strings while the selected text
            // came from the helper.
            for option in [
                Behavior::Idle,
                Behavior::WalkAround { speed: 60.0 },
                Behavior::FollowCursor {
                    speed: 240.0,
                    comfort_distance: 80.0,
                },
                Behavior::BoundedWander {
                    x_min: 200.0,
                    x_max: 1200.0,
                    y_min: 200.0,
                    y_max: 800.0,
                    speed: 120.0,
                },
                Behavior::Bounce {
                    amplitude_px: 24.0,
                    period_sec: 1.5,
                    axis: crate::behavior::BounceAxis::Vertical,
                },
                // Keeps its settings when already following; otherwise
                // starts after the first one it may follow.
                match &prev {
                    Behavior::FollowCharacter { .. } => prev.clone(),
                    _ => Behavior::FollowCharacter {
                        target: leaders
                            .first()
                            .map(|(id, _)| id.clone())
                            .unwrap_or_default(),
                        distance: 90.0,
                        speed: 140.0,
                    },
                },
                // Likewise: with the first one it may play with.
                match &prev {
                    Behavior::PlayTag { .. } => prev.clone(),
                    _ => Behavior::PlayTag {
                        partner: partners
                            .first()
                            .map(|(id, _)| id.clone())
                            .unwrap_or_default(),
                        speed: 220.0,
                    },
                },
                // Starts empty: an entity gets a script only once the
                // user names one, and an empty path is the documented
                // "hold still" state rather than an error.
                Behavior::Script {
                    path: String::new(),
                    params: Default::default(),
                },
            ] {
                let label = behavior_label_with_icon(&option);
                ui.selectable_value(behavior, option, label).picked();
            }
            if *behavior != prev {
                changed = true;
            }
        });
    crate::ui::accessible::name_combo(
        &combo.response,
        &t("inspector-section-behavior"),
        &combo_value,
    );

    // Variant-specific sliders.
    match behavior {
        Behavior::Idle => {}
        Behavior::FollowCharacter {
            target,
            distance,
            speed,
        } => {
            let nobody = t("inspector-follow-nobody");
            let current = leaders
                .iter()
                .find(|(id, _)| id == target)
                .map_or(nobody.clone(), |(_, name)| name.clone());
            ui.horizontal(|ui| {
                let caption = ui.label(t("inspector-follow-target"));
                let combo = egui::ComboBox::from_id_salt("behavior_follow_target")
                    .selected_text(current.clone())
                    .show_ui(ui, |ui| {
                        if leaders.is_empty() {
                            ui.label(
                                egui::RichText::new(t("inspector-follow-none-free"))
                                    .text_style(theme::caption())
                                    .weak(),
                            );
                        }
                        for (id, name) in leaders {
                            if ui.selectable_label(target == id, name).picked() && target != id {
                                *target = id.clone();
                                changed = true;
                            }
                        }
                    });
                crate::ui::accessible::name_combo(
                    &combo.response.labelled_by(caption.id),
                    &t("inspector-follow-target"),
                    &current,
                );
            });
            if ui
                .add(
                    egui::Slider::new(distance, 0.0..=400.0)
                        .text(t("inspector-follow-distance"))
                        .suffix(" px"),
                )
                .changed()
            {
                changed = true;
            }
            if ui
                .add(
                    egui::Slider::new(speed, 20.0..=400.0)
                        .text(t("inspector-behavior-speed"))
                        .suffix(" px/s"),
                )
                .changed()
            {
                changed = true;
            }
        }
        Behavior::PlayTag { partner, speed } => {
            let nobody = t("inspector-follow-nobody");
            let current = partners
                .iter()
                .find(|(id, _)| id == partner)
                .map_or(nobody.clone(), |(_, name)| name.clone());
            ui.horizontal(|ui| {
                let caption = ui.label(t("inspector-tag-partner"));
                let combo = egui::ComboBox::from_id_salt("behavior_tag_partner")
                    .selected_text(current.clone())
                    .show_ui(ui, |ui| {
                        for (id, name) in partners {
                            if ui.selectable_label(partner == id, name).picked() && partner != id {
                                *partner = id.clone();
                                changed = true;
                            }
                        }
                    });
                crate::ui::accessible::name_combo(
                    &combo.response.labelled_by(caption.id),
                    &t("inspector-tag-partner"),
                    &current,
                );
            });
            ui.add(
                egui::Label::new(
                    egui::RichText::new(t("inspector-tag-hint"))
                        .text_style(theme::caption())
                        .weak(),
                )
                .wrap(),
            );
            if ui
                .add(
                    egui::Slider::new(speed, 40.0..=500.0)
                        .text(t("inspector-behavior-speed"))
                        .suffix(" px/s"),
                )
                .changed()
            {
                changed = true;
            }
        }
        Behavior::Script { path, params } => {
            if script_controls(ui, path, params) {
                changed = true;
            }
        }
        Behavior::WalkAround { speed } => {
            if ui
                .add(
                    egui::Slider::new(speed, 10.0..=400.0)
                        .text(t("inspector-behavior-speed"))
                        .suffix(" px/s"),
                )
                .changed()
            {
                changed = true;
            }
        }
        Behavior::FollowCursor {
            speed,
            comfort_distance,
        } => {
            if ui
                .add(
                    egui::Slider::new(speed, 50.0..=800.0)
                        .text(t("inspector-behavior-speed"))
                        .suffix(" px/s"),
                )
                .changed()
            {
                changed = true;
            }
            if ui
                .add(
                    egui::Slider::new(comfort_distance, 0.0..=400.0)
                        .text(t("inspector-behavior-comfort"))
                        .suffix(" px"),
                )
                .changed()
            {
                changed = true;
            }
        }
        Behavior::BoundedWander {
            x_min,
            x_max,
            y_min,
            y_max,
            speed,
        } => {
            if ui
                .add(
                    egui::Slider::new(speed, 20.0..=400.0)
                        .text(t("inspector-behavior-speed"))
                        .suffix(" px/s"),
                )
                .changed()
            {
                changed = true;
            }
            ui.add_space(SPACE_XS);
            ui.label(
                egui::RichText::new(t("inspector-wander-box"))
                    .small()
                    .weak(),
            );
            ui.horizontal(|ui| {
                ui.label(t("inspector-x"));
                if ui
                    .add(egui::DragValue::new(x_min).speed(1.0).prefix("min "))
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .add(egui::DragValue::new(x_max).speed(1.0).prefix("max "))
                    .changed()
                {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(t("inspector-y"));
                if ui
                    .add(egui::DragValue::new(y_min).speed(1.0).prefix("min "))
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .add(egui::DragValue::new(y_max).speed(1.0).prefix("max "))
                    .changed()
                {
                    changed = true;
                }
            });
        }
        Behavior::Bounce {
            amplitude_px,
            period_sec,
            axis,
        } => {
            if ui
                .add(
                    egui::Slider::new(amplitude_px, 1.0..=200.0)
                        .text(t("inspector-behavior-amplitude"))
                        .suffix(" px"),
                )
                .changed()
            {
                changed = true;
            }
            if ui
                .add(
                    egui::Slider::new(period_sec, 0.1..=10.0)
                        .text(t("inspector-behavior-period"))
                        .suffix(" s"),
                )
                .changed()
            {
                changed = true;
            }
            ui.horizontal(|ui| {
                ui.label(t("behavior-bounce-axis"));
                let prev_axis = *axis;
                ui.selectable_value(
                    axis,
                    crate::behavior::BounceAxis::Horizontal,
                    t("behavior-bounce-horizontal"),
                );
                ui.selectable_value(
                    axis,
                    crate::behavior::BounceAxis::Vertical,
                    t("behavior-bounce-vertical"),
                );
                ui.selectable_value(
                    axis,
                    crate::behavior::BounceAxis::Both,
                    t("behavior-bounce-both"),
                );
                if *axis != prev_axis {
                    changed = true;
                }
            });
        }
    }

    changed
}

fn behavior_label_with_icon(b: &Behavior) -> String {
    let (icon, key) = match b {
        Behavior::Idle => (icons::BEHAVIOR_IDLE, "behavior-idle"),
        Behavior::WalkAround { .. } => (icons::BEHAVIOR_WALK, "behavior-walk"),
        Behavior::FollowCursor { .. } => (icons::BEHAVIOR_FOLLOW, "behavior-follow"),
        Behavior::FollowCharacter { .. } => (
            icons::BEHAVIOR_FOLLOW_CHARACTER,
            "behavior-follow-character",
        ),
        Behavior::BoundedWander { .. } => (icons::BEHAVIOR_WANDER, "behavior-wander"),
        Behavior::Bounce { .. } => (icons::BEHAVIOR_BOUNCE, "behavior-bounce"),
        Behavior::PlayTag { .. } => (icons::BEHAVIOR_PLAY_TAG, "behavior-play-tag"),
        Behavior::Script { .. } => (icons::BEHAVIOR_SCRIPT, "behavior-script"),
    };
    format!("{icon}  {}", t(key))
}
