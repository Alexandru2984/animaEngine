//! Scene tab — monitor distribution, entity list, preset gallery,
//! groups. Extracted in I.8.
//!
//! The `scene_list` selection pulse and the groups list live here too
//! since they're only used by this tab.

use super::monitor::{monitor_mode_picker, pulse_alpha_at};
use super::presets::preset_gallery;
use crate::i18n::t;
use crate::input::selection::SelectionState;
use crate::monitor::{MonitorInfo, MonitorMode};
use crate::scene::Scene;
use crate::ui::accessible::{whole, AccessibleName, ComboOption};
use crate::ui::collapse::CollapseState;
use crate::ui::icons;
use crate::ui::states;
use crate::ui::theme::{self, SPACE_M, SPACE_S};

// UI plumbing fans out one settings struct into per-tab params — same
// allow as `panels::settings` itself.
#[expect(clippy::too_many_arguments)]
pub(super) fn scene_tab(
    ui: &mut egui::Ui,
    scene: &mut Scene,
    selection: &mut SelectionState,
    config_dirty: &mut bool,
    monitor_mode: &mut MonitorMode,
    window_awareness: &mut bool,
    // Whether this backend can actually provide window positions.
    window_awareness_supported: bool,
    snap_while_dragging: &mut bool,
    characters_bump: &mut bool,
    characters_greet: &mut bool,
    active_scene: &mut Option<String>,
    reminders: &mut Vec<crate::reminders::ReminderConfig>,
    scene_schedule: &mut Vec<crate::schedule::ScheduleRule>,
    focus: &mut crate::focus::FocusConfig,
    reminder_sound: &mut crate::reminders::ReminderSound,
    notify_reminders: &mut bool,
    // Kept separate from window-awareness even though both are false on
    // the same backend today: they are different capabilities, and a
    // future backend could have one without the other.
    span_supported: bool,
    monitors: &[MonitorInfo],
    collapse_state: &mut CollapseState,
    request: &mut Option<super::SceneRequest>,
) {
    // Sections that fold away, their state kept across restarts
    // (`CollapseState`): the characters and what is made of them first,
    // then the time of day, then how they behave together and where.
    // The way in that needs no dragging comes before all of it.
    ui.horizontal(|ui| {
        // In the Flatpak, the one that reaches files outside Pictures and
        // Downloads. Unix only: it goes through the XDG desktop portal.
        if cfg!(unix)
            && ui
                .button(format!("{}  {}", icons::ADD, t("scene-add-file")))
                .on_hover_text(t("scene-add-file-tooltip"))
                .clicked()
        {
            *request = Some(super::SceneRequest::AddFile);
        }
        // Where characters copied in another scene come in, for someone
        // not using the keyboard (`crate::clipboard`) — or an image copied
        // elsewhere (`crate::system_clipboard`), so never greyed out.
        let copied = crate::clipboard::count();
        let hint = if copied == 0 {
            t("scene-paste-empty")
        } else {
            let mut args = fluent::FluentArgs::new();
            args.set("count", copied);
            crate::i18n::t_args("scene-paste-tooltip", &args)
        };
        if ui
            .button(format!("{}  {}", icons::PASTE, t("scene-paste")))
            .on_hover_text(&hint)
            .clicked()
        {
            *request = Some(super::SceneRequest::Paste);
        }
    });
    ui.add_space(SPACE_M);

    let mut changed = false;
    if scene.entities.is_empty() {
        // D.8: zero-config CTA — open the preset gallery so a fresh
        // install can land in a curated scene with one click.
        if states::empty_with_action(
            ui,
            icons::GHOST,
            &t("scene-empty-headline"),
            &t("scene-empty-hint"),
            Some(&t("scene-empty-action-browse-presets")),
        ) {
            collapse_state.scene_presets = true;
            changed = true;
        }
        ui.add_space(SPACE_M);
    } else {
        let title = titled(
            icons::GHOST,
            "scene-characters-header",
            scene.entities.len(),
        );
        super::section(
            ui,
            "scene_characters",
            &title,
            &mut collapse_state.scene_characters,
            config_dirty,
            |ui| {
                ui.label(
                    egui::RichText::new(t("scene-drop-hint"))
                        .text_style(theme::caption())
                        .weak(),
                );
                ui.add_space(SPACE_S);
                scene_list(ui, scene, selection, &mut changed);
            },
        );
        // Right under the characters they are made of — with a hint on
        // how, until there are any.
        let title = titled(icons::STACK, "scene-groups-header", scene.groups.len());
        super::section(
            ui,
            "scene_groups",
            &title,
            &mut collapse_state.scene_groups,
            config_dirty,
            |ui| groups_section(ui, scene, selection, &mut changed),
        );
    }
    // Saved scenes, even with none on screen: one may be loaded.
    let title = titled(
        icons::SCENE,
        "scene-scenes-header",
        crate::scenes::shelf().len(),
    );
    super::section(
        ui,
        "scene_saved",
        &title,
        &mut collapse_state.scene_saved,
        config_dirty,
        |ui| {
            scenes_section(ui, scene, selection, &mut changed, active_scene, request);
            schedule_section(ui, scene_schedule, &mut changed);
        },
    );
    // Reminders, with what plays and where they go — both shared with
    // the focus timer below, as their hints say.
    let title = titled(icons::REMINDER, "scene-reminders-header", reminders.len());
    super::section(
        ui,
        "scene_reminders",
        &title,
        &mut collapse_state.scene_reminders,
        config_dirty,
        |ui| {
            reminders_section(ui, scene, reminders, &mut changed);
            ui.add_space(SPACE_S);
            changed |= reminder_sound_row(ui, reminder_sound, scene);
            // To the desktop's notifications too (`crate::notify`).
            changed |= ui
                .checkbox(notify_reminders, t("reminder-notify-label"))
                .on_hover_text(t("reminder-notify-hint"))
                .changed();
        },
    );
    super::section(
        ui,
        "scene_focus",
        &titled(icons::FOCUS, "scene-focus-header", 0),
        &mut collapse_state.scene_focus,
        config_dirty,
        |ui| focus_section(ui, focus, &mut changed),
    );
    // How the characters behave around each other and a drag.
    super::section(
        ui,
        "scene_together",
        &titled(icons::HEART, "scene-together-header", 0),
        &mut collapse_state.scene_together,
        config_dirty,
        |ui| {
            // Snapping while dragging (1.5), on both backends.
            changed |= ui
                .checkbox(snap_while_dragging, t("scene-snap"))
                .on_hover_text(t("scene-snap-tooltip"))
                .changed();
            // Characters bumping into each other (1.5), on both backends.
            changed |= ui
                .checkbox(characters_bump, t("scene-bump"))
                .on_hover_text(t("scene-bump-tooltip"))
                .changed();
            // And saying hello when they meet (`crate::greet`).
            changed |= ui
                .checkbox(characters_greet, t("scene-greet"))
                .on_hover_text(t("scene-greet-tooltip"))
                .changed();
        },
    );
    super::section(
        ui,
        "scene_monitors",
        &titled(icons::MONITOR, "monitor-section-header", 0),
        &mut collapse_state.scene_monitors,
        config_dirty,
        |ui| {
            monitor_mode_picker(ui, monitor_mode, monitors, span_supported, &mut changed);
            // Window awareness (X11), greyed out where it cannot work. The
            // control used to stay live on every backend with only a
            // tooltip to explain it, so on native Wayland a user could
            // tick it, see nothing happen, and have no way to tell whether
            // the feature or their setup was at fault. The display server
            // is the wrong thing to test — a Wayland session running us
            // through XWayland reads EWMH just fine — so the caller passes
            // what it actually has.
            ui.add_space(SPACE_S);
            ui.add_enabled_ui(window_awareness_supported, |ui| {
                changed |= ui
                    .checkbox(window_awareness, t("scene-window-awareness"))
                    .on_hover_text(t("scene-window-awareness-tooltip"))
                    .changed();
            });
            if !window_awareness_supported {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(t("scene-window-awareness-unavailable"))
                            .text_style(crate::ui::theme::caption())
                            .color(ui.visuals().weak_text_color()),
                    )
                    .wrap(),
                );
            }
        },
    );
    *config_dirty |= changed;

    preset_gallery(
        ui,
        scene,
        selection,
        config_dirty,
        &mut collapse_state.scene_presets,
    );
}

/// A section's title: its icon, its name, and how many it holds when it
/// holds any — so a folded section still says what is in it.
fn titled(icon: &str, key: &str, count: usize) -> String {
    if count > 0 {
        format!("{icon}  {} ({count})", t(key))
    } else {
        format!("{icon}  {}", t(key))
    }
}

/// A group being renamed: which, the name it had, and whether its field
/// has been given the keyboard yet.
#[derive(Clone)]
struct Renaming {
    group_id: String,
    original: String,
    focused: bool,
}

enum GroupAction {
    Select(String),
    Dissolve(String),
}

/// The focus timer (1.5, `crate::focus`): how long the work and the break
/// last, where it is now, and a start or a stop.
fn focus_section(
    ui: &mut egui::Ui,
    focus: &mut crate::focus::FocusConfig,
    config_dirty: &mut bool,
) {
    ui.add(
        egui::Label::new(
            egui::RichText::new(t("scene-focus-hint"))
                .text_style(theme::caption())
                .weak(),
        )
        .wrap(),
    );
    ui.add_space(SPACE_S);
    ui.horizontal(|ui| {
        let work = ui.label(t("scene-focus-work"));
        let changed_work = ui
            .add(
                whole(egui::DragValue::new(&mut focus.minutes))
                    .range(1..=180)
                    .suffix(" min"),
            )
            .labelled_by(work.id)
            .changed();
        let rest = ui.label(t("scene-focus-break"));
        let changed_rest = ui
            .add(
                whole(egui::DragValue::new(&mut focus.break_minutes))
                    .range(1..=60)
                    .suffix(" min"),
            )
            .labelled_by(rest.id)
            .changed();
        if changed_work || changed_rest {
            *config_dirty = true;
        }
    });
    ui.add_space(SPACE_S);
    match crate::focus::until_label() {
        Some((phase, until)) => {
            let mut args = fluent::FluentArgs::new();
            args.set("until", until);
            let key = match phase {
                crate::focus::Phase::Work => "scene-focus-working",
                crate::focus::Phase::Break => "scene-focus-on-break",
            };
            ui.horizontal(|ui| {
                ui.label(crate::i18n::t_args(key, &args));
                if ui.button(t("scene-focus-stop")).clicked() {
                    crate::focus::stop();
                }
            });
        }
        None => {
            if ui
                .button(format!("{}  {}", icons::FOCUS, t("scene-focus-start")))
                .clicked()
            {
                crate::focus::start(*focus);
            }
        }
    }
}

/// What sounds with a reminder or a turn of the focus timer
/// (`crate::reminders::ReminderSound`): nothing, the speaker's own tap
/// sound, or a file from the asset library — picking one plays it, by the
/// character in front. Returns whether it changed.
fn reminder_sound_row(
    ui: &mut egui::Ui,
    sound: &mut crate::reminders::ReminderSound,
    scene: &mut Scene,
) -> bool {
    use crate::reminders::ReminderSound;
    let mut changed = false;
    let choices = crate::sounds::choices();
    let label = |s: &ReminderSound| match s {
        ReminderSound::None => t("reminder-sound-none"),
        ReminderSound::Speaker => t("reminder-sound-speaker"),
        ReminderSound::File(file) => file.clone(),
    };
    // The label over the choice: side by side they were wider than the
    // panel, which then grew to fit them.
    ui.label(t("reminder-sound-label"));
    {
        let combo = egui::ComboBox::from_id_salt("anima.reminder-sound")
            .selected_text(label(sound))
            .show_ui(ui, |ui| {
                let options = [ReminderSound::None, ReminderSound::Speaker]
                    .into_iter()
                    .chain(choices.iter().cloned().map(ReminderSound::File));
                for option in options {
                    if ui
                        .selectable_label(*sound == option, label(&option))
                        .picked()
                        && *sound != option
                    {
                        // Heard once, from whoever is in front.
                        if let Some(i) = scene.frontmost_visible() {
                            option.play_by(&mut scene.entities[i]);
                        }
                        *sound = option;
                        changed = true;
                    }
                }
            });
        // Named for a screen reader: `labelled_by` does not reach AT-SPI.
        crate::ui::accessible::name_combo(
            &combo.response,
            &t("reminder-sound-label"),
            &label(sound),
        );
    }
    changed
}

/// The reminder being written below the list, kept between frames.
#[derive(Clone)]
struct ReminderDraft {
    text: String,
    minutes: u32,
    character: Option<String>,
    /// At a set time, rather than every so often.
    timed: bool,
    time: String,
    days: crate::schedule::Days,
    /// On a date each year, rather than on `days`.
    on_date: bool,
    day: u8,
    month: u8,
}

impl Default for ReminderDraft {
    fn default() -> Self {
        // A date starts as today's.
        let today = crate::schedule::local_now();
        Self {
            text: String::new(),
            minutes: 60,
            character: None,
            timed: false,
            time: "09:00".into(),
            days: crate::schedule::Days::Every,
            on_date: false,
            day: today.map_or(1, |t| t.day),
            month: today.map_or(1, |t| t.month),
        }
    }
}

/// When a reminder at a set time comes, as the list says it: "10:00 ·
/// weekdays", or "12 October · 09:00".
fn at_label(at: &crate::reminders::At) -> String {
    match at.date() {
        Some((month, day)) => format!("{} · {}", crate::placeholders::date(day, month), at.time),
        None => format!("{} · {}", at.time, t(at.days.i18n_key())),
    }
}

/// Reminders (1.5, `crate::reminders`): each one's text switches it on and
/// off, with how often and who says it beside, and a delete. Below, a new
/// one: what to say, every how many minutes, and who — any character by
/// default. With none yet, three ready to add.
fn reminders_section(
    ui: &mut egui::Ui,
    scene: &Scene,
    reminders: &mut Vec<crate::reminders::ReminderConfig>,
    config_dirty: &mut bool,
) {
    use crate::reminders::{ReminderConfig, MAX_MINUTES, MIN_MINUTES};
    let weak = ui.visuals().weak_text_color();
    let every = |minutes: u32| {
        let mut args = fluent::FluentArgs::new();
        args.set("minutes", minutes);
        crate::i18n::t_args("scene-reminder-every", &args)
    };
    let who = |id: &Option<String>| match id {
        Some(id) => scene
            .entities
            .iter()
            .find(|e| &e.id == id)
            .map(|e| e.name.clone())
            .unwrap_or_else(|| t("scene-reminder-anyone")),
        None => t("scene-reminder-anyone"),
    };

    if reminders.is_empty() {
        ui.add(
            egui::Label::new(
                egui::RichText::new(t("scene-reminders-hint"))
                    .text_style(theme::caption())
                    .color(weak),
            )
            .wrap(),
        );
        ui.add_space(SPACE_S);
        // Ready-made, one click each.
        for (key, minutes) in [
            ("reminder-break", 50),
            ("reminder-water", 60),
            ("reminder-stretch", 30),
        ] {
            let text = t(key);
            if ui
                .button(format!("{}  {} · {}", icons::PLUS, text, every(minutes)))
                .clicked()
            {
                reminders.push(ReminderConfig {
                    text,
                    every_minutes: minutes,
                    character: None,
                    enabled: true,
                    at: None,
                });
                *config_dirty = true;
            }
        }
    }

    let delete_label = t("scene-reminder-delete");
    let mut remove: Option<usize> = None;
    for (i, r) in reminders.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            if ui.checkbox(&mut r.enabled, &r.text).changed() {
                *config_dirty = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button(icons::TRASH)
                    .named(format!("{delete_label}: {}", r.text))
                    .on_hover_text(&delete_label)
                    .clicked()
                {
                    remove = Some(i);
                }
            });
        });
        // How often and who, on a line of its own: beside the text it ran
        // into a longer reminder. Tucked up under the text it belongs to.
        ui.add_space(-ui.spacing().item_spacing.y / 2.0);
        ui.horizontal(|ui| {
            ui.add_space(ui.spacing().icon_width + ui.spacing().icon_spacing);
            ui.label(
                egui::RichText::new(format!(
                    "{} · {}",
                    r.at.as_ref()
                        .map_or_else(|| every(r.every_minutes), at_label),
                    who(&r.character)
                ))
                .text_style(theme::caption())
                .color(weak),
            );
        });
    }
    if let Some(i) = remove {
        reminders.remove(i);
        *config_dirty = true;
    }

    // A new one.
    ui.add_space(SPACE_S);
    let draft_key = egui::Id::new("anima.reminder-draft");
    let mut draft: ReminderDraft = ui.data(|d| d.get_temp(draft_key)).unwrap_or_default();
    let field = ui.add(
        egui::TextEdit::singleline(&mut draft.text)
            .hint_text(t("scene-reminder-text-hint"))
            .desired_width(f32::INFINITY),
    );
    crate::ui::accessible::name_text_field(&field, &t("scene-reminder-text-hint"));
    // {time}, {day}, {date}, {name} (`crate::placeholders`).
    ui.add(
        egui::Label::new(
            egui::RichText::new(crate::placeholders::hint())
                .text_style(theme::caption())
                .weak(),
        )
        .wrap(),
    );
    let mut add = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    // Every so often, or at a set time (`reminders::At`).
    ui.checkbox(&mut draft.timed, t("scene-reminder-timed"));
    if draft.timed {
        ui.horizontal(|ui| {
            super::appearance::hhmm_field(ui, &mut draft.time, &t("scene-reminder-at"));
            let days_label = if draft.on_date {
                t("scene-reminder-on-date")
            } else {
                t(draft.days.i18n_key())
            };
            let combo = egui::ComboBox::from_id_salt("anima.reminder-days")
                .selected_text(days_label.clone())
                .show_ui(ui, |ui| {
                    for option in crate::schedule::Days::ALL {
                        if ui
                            .selectable_label(
                                !draft.on_date && draft.days == option,
                                t(option.i18n_key()),
                            )
                            .picked()
                        {
                            draft.days = option;
                            draft.on_date = false;
                        }
                    }
                    if ui
                        .selectable_label(draft.on_date, t("scene-reminder-on-date"))
                        .picked()
                    {
                        draft.on_date = true;
                    }
                });
            crate::ui::accessible::name_combo(
                &combo.response,
                &t("scene-reminder-days"),
                &days_label,
            );
        });
        if draft.on_date {
            ui.horizontal(|ui| {
                let label = ui.label(t("scene-reminder-date"));
                ui.add(whole(egui::DragValue::new(&mut draft.day)).range(1..=31))
                    .labelled_by(label.id);
                let month = crate::placeholders::month_name(draft.month);
                let combo = egui::ComboBox::from_id_salt("anima.reminder-month")
                    .selected_text(month.clone())
                    .show_ui(ui, |ui| {
                        for m in 1..=12u8 {
                            ui.selectable_value(
                                &mut draft.month,
                                m,
                                crate::placeholders::month_name(m),
                            )
                            .picked();
                        }
                    });
                crate::ui::accessible::name_combo(
                    &combo.response,
                    &t("scene-reminder-date"),
                    &month,
                );
            });
        }
    }
    // How often, then who, on lines of their own: on one, a longer
    // language's "any character" made the row wider than the panel.
    if !draft.timed {
        ui.horizontal(|ui| {
            let label = ui.label(t("scene-reminder-minutes"));
            ui.add(
                whole(egui::DragValue::new(&mut draft.minutes)).range(MIN_MINUTES..=MAX_MINUTES),
            )
            .labelled_by(label.id);
        });
    }
    {
        let chosen = who(&draft.character);
        let combo = egui::ComboBox::from_id_salt("anima.reminder-character")
            .selected_text(chosen.clone())
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(draft.character.is_none(), t("scene-reminder-anyone"))
                    .picked()
                {
                    draft.character = None;
                }
                for e in &scene.entities {
                    if ui
                        .selectable_label(
                            draft.character.as_deref() == Some(e.id.as_str()),
                            &e.name,
                        )
                        .picked()
                    {
                        draft.character = Some(e.id.clone());
                    }
                }
            });
        crate::ui::accessible::name_combo(&combo.response, &t("scene-reminder-who"), &chosen);
    }
    add |= ui
        .add_enabled(
            !draft.text.trim().is_empty(),
            egui::Button::new(format!("{}  {}", icons::PLUS, t("scene-reminder-add"))),
        )
        .clicked();
    if add && !draft.text.trim().is_empty() {
        reminders.push(ReminderConfig {
            text: draft.text.trim().to_string(),
            every_minutes: draft.minutes.clamp(MIN_MINUTES, MAX_MINUTES),
            character: draft.character.clone(),
            enabled: true,
            at: draft.timed.then(|| crate::reminders::At {
                time: draft.time.clone(),
                days: draft.days,
                date: draft
                    .on_date
                    .then(|| format!("{:02}-{:02}", draft.month, draft.day)),
            }),
        });
        *config_dirty = true;
        draft = ReminderDraft::default();
    }
    ui.data_mut(|d| d.insert_temp(draft_key, draft));
}

/// The rule being written below the schedule, kept between frames.
#[derive(Clone)]
struct RuleDraft {
    hour: u32,
    minute: u32,
    days: crate::schedule::Days,
    scene: Option<String>,
}

impl Default for RuleDraft {
    fn default() -> Self {
        Self {
            hour: 9,
            minute: 0,
            days: crate::schedule::Days::Weekdays,
            scene: None,
        }
    }
}

/// Scenes by time of day (1.5, `crate::schedule`), under the saved scenes:
/// each rule as "09:00 · weekdays → Work", switched on and off by a click,
/// with a delete; below, a new one. Needs a saved scene to name.
fn schedule_section(
    ui: &mut egui::Ui,
    rules: &mut Vec<crate::schedule::ScheduleRule>,
    config_dirty: &mut bool,
) {
    use crate::schedule::{format_hhmm, Days, ScheduleRule};
    ui.add_space(SPACE_S);
    ui.label(egui::RichText::new(t("scene-schedule-header")).strong());
    let weak = ui.visuals().weak_text_color();
    let caption = |ui: &mut egui::Ui, key: &str| {
        ui.add(
            egui::Label::new(
                egui::RichText::new(t(key))
                    .text_style(theme::caption())
                    .color(weak),
            )
            .wrap(),
        );
    };
    let shelf = crate::scenes::shelf();
    if rules.is_empty() {
        caption(
            ui,
            if shelf.is_empty() {
                "scene-schedule-needs-scene"
            } else {
                "scene-schedule-hint"
            },
        );
    }
    let row_label = |r: &ScheduleRule| {
        let mut args = fluent::FluentArgs::new();
        args.set("time", r.at.clone());
        args.set("days", t(r.days.i18n_key()));
        args.set("scene", r.scene.clone());
        crate::i18n::t_args("scene-schedule-row", &args)
    };
    let delete_label = t("scene-schedule-delete");
    let mut remove = None;
    for (i, r) in rules.iter_mut().enumerate() {
        let label = row_label(r);
        ui.horizontal(|ui| {
            if ui.checkbox(&mut r.enabled, &label).changed() {
                *config_dirty = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button(icons::TRASH)
                    .named(format!("{delete_label}: {label}"))
                    .on_hover_text(&delete_label)
                    .clicked()
                {
                    remove = Some(i);
                }
            });
        });
    }
    if let Some(i) = remove {
        rules.remove(i);
        *config_dirty = true;
    }
    if shelf.is_empty() {
        return;
    }

    let draft_key = egui::Id::new("anima.schedule-draft");
    let mut draft: RuleDraft = ui.data(|d| d.get_temp(draft_key)).unwrap_or_default();
    ui.horizontal(|ui| {
        let hour = ui.label(t("scene-schedule-at"));
        ui.add(whole(egui::DragValue::new(&mut draft.hour)).range(0..=23))
            .labelled_by(hour.id);
        ui.label(":");
        let minute = ui.add(whole(egui::DragValue::new(&mut draft.minute)).range(0..=59));
        crate::ui::accessible::name_number_field(
            &minute,
            &t("scene-schedule-minute"),
            f64::from(draft.minute),
        );
        let days = t(draft.days.i18n_key());
        let combo = egui::ComboBox::from_id_salt("anima.schedule-days")
            .selected_text(days.clone())
            .show_ui(ui, |ui| {
                for option in Days::ALL {
                    if ui
                        .selectable_label(draft.days == option, t(option.i18n_key()))
                        .picked()
                    {
                        draft.days = option;
                    }
                }
            });
        crate::ui::accessible::name_combo(&combo.response, &t("scene-schedule-days"), &days);
    });
    ui.horizontal(|ui| {
        let chosen = draft
            .scene
            .clone()
            .filter(|s| shelf.iter().any(|e| &e.name == s))
            .unwrap_or_else(|| shelf[0].name.clone());
        let combo = egui::ComboBox::from_id_salt("anima.schedule-scene")
            .selected_text(chosen.clone())
            .show_ui(ui, |ui| {
                for entry in &shelf {
                    if ui
                        .selectable_label(chosen == entry.name, &entry.name)
                        .picked()
                    {
                        draft.scene = Some(entry.name.clone());
                    }
                }
            });
        crate::ui::accessible::name_combo(&combo.response, &t("scene-schedule-scene"), &chosen);
        if ui
            .button(format!("{}  {}", icons::PLUS, t("scene-schedule-add")))
            .clicked()
        {
            rules.push(ScheduleRule {
                scene: chosen,
                at: format_hhmm(draft.hour * 60 + draft.minute),
                days: draft.days,
                enabled: true,
            });
            *config_dirty = true;
        }
    });
    ui.data_mut(|d| d.insert_temp(draft_key, draft));
}

enum SceneAction {
    Load(std::path::PathBuf),
    SaveOver(String),
    SaveNew(String),
    Delete(std::path::PathBuf),
}

/// Saved scenes (1.5, `crate::scenes`): a click on one switches to it —
/// Undo switches back — and beside it, save what is on screen over it,
/// or delete it (asked twice: a file, and undo does not reach files).
/// Below, a name and a button to save what is on screen as a new one.
fn scenes_section(
    ui: &mut egui::Ui,
    scene: &mut Scene,
    selection: &mut SelectionState,
    config_dirty: &mut bool,
    active: &mut Option<String>,
    request: &mut Option<super::SceneRequest>,
) {
    let weak = ui.visuals().weak_text_color();
    let shelf = crate::scenes::shelf();
    if shelf.is_empty() {
        ui.add(
            egui::Label::new(
                egui::RichText::new(t("scene-scenes-hint"))
                    .text_style(theme::caption())
                    .color(weak),
            )
            .wrap(),
        );
        ui.add_space(SPACE_S);
    }

    let deleting_key = egui::Id::new("anima.scene-deleting");
    let error_key = egui::Id::new("anima.scene-error");
    let name_key = egui::Id::new("anima.scene-new-name");
    let mut deleting: Option<std::path::PathBuf> = ui.data(|d| d.get_temp(deleting_key));
    let mut action: Option<SceneAction> = None;
    for entry in &shelf {
        ui.horizontal(|ui| {
            // Its picture, or an empty frame where there is none yet.
            let (rect, _) = ui.allocate_exact_size(egui::vec2(80.0, 50.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(rect, theme::RADIUS_SM, ui.visuals().faint_bg_color);
            if let Some(texture) = scene_thumbnail(ui, &entry.path) {
                egui::Image::new(&texture)
                    .corner_radius(theme::RADIUS_SM)
                    .paint_at(ui, rect);
            }
            let is_active = active.as_deref() == Some(entry.name.as_str());
            if ui
                .selectable_label(is_active, &entry.name)
                .on_hover_text(t("scene-scene-load-tooltip"))
                .clicked()
            {
                action = Some(SceneAction::Load(entry.path.clone()));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if deleting.as_ref() == Some(&entry.path) {
                    let error_color = ui.visuals().error_fg_color;
                    if ui
                        .small_button(
                            egui::RichText::new(t("scene-scene-delete-confirm")).color(error_color),
                        )
                        .named(format!("{}: {}", t("scene-scene-delete"), entry.name))
                        .clicked()
                    {
                        action = Some(SceneAction::Delete(entry.path.clone()));
                    }
                    if ui
                        .small_button(icons::CLOSE)
                        .on_hover_name(t("scene-scene-delete-cancel"))
                        .clicked()
                    {
                        deleting = None;
                    }
                } else {
                    if ui
                        .small_button(icons::TRASH)
                        .named(format!("{}: {}", t("scene-scene-delete"), entry.name))
                        .on_hover_text(t("scene-scene-delete"))
                        .clicked()
                    {
                        deleting = Some(entry.path.clone());
                    }
                    let mut args = fluent::FluentArgs::new();
                    args.set("name", entry.name.clone());
                    let save_over = crate::i18n::t_args("scene-scene-save-over", &args);
                    if ui
                        .small_button(icons::SAVE)
                        .on_hover_name(save_over)
                        .clicked()
                    {
                        action = Some(SceneAction::SaveOver(entry.name.clone()));
                    }
                    // As one file, pictures and all (`crate::scene_file`).
                    let share = crate::i18n::t_args("scene-scene-share", &args);
                    if ui.small_button(icons::SHARE).on_hover_name(share).clicked() {
                        *request = Some(super::SceneRequest::Share(entry.path.clone()));
                    }
                }
            });
        });
    }

    ui.add_space(SPACE_S);
    let mut name: String = ui.data(|d| d.get_temp(name_key)).unwrap_or_default();
    // The name over the button, the field as wide as the panel: side by
    // side, a fixed-width field pushed the row past the panel where the
    // button's words are long, and a side panel widens to fit its widest
    // row — and a field sized from what the button left grew the panel
    // without end, a pixel of misjudged button at a time.
    let field = ui.add(
        egui::TextEdit::singleline(&mut name)
            .hint_text(t("scene-scene-name-hint"))
            .desired_width(f32::INFINITY),
    );
    crate::ui::accessible::name_text_field(&field, &t("scene-scene-name-hint"));
    let entered = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    ui.horizontal_wrapped(|ui| {
        let clicked = ui
            .add_enabled(
                !name.trim().is_empty(),
                egui::Button::new(format!("{}  {}", icons::SAVE, t("scene-scene-save-new"))),
            )
            .clicked();
        if (entered || clicked) && !name.trim().is_empty() {
            action = Some(SceneAction::SaveNew(name.trim().to_string()));
        }
        // A scene someone shared — through the desktop's chooser, so Unix
        // only, like "Add file…"; dropping the file works everywhere.
        if cfg!(unix)
            && ui
                .button(format!("{}  {}", icons::IMPORT, t("scene-import")))
                .on_hover_text(t("scene-import-tooltip"))
                .clicked()
        {
            *request = Some(super::SceneRequest::Import);
        }
    });
    if let Some(error) = ui.data(|d| d.get_temp::<String>(error_key)) {
        ui.add(
            egui::Label::new(
                egui::RichText::new(error)
                    .text_style(theme::caption())
                    .color(ui.visuals().error_fg_color),
            )
            .wrap(),
        );
    }

    // An error stays up until the next thing done here.
    let acted = action.is_some();
    let mut error: Option<String> = None;
    match action {
        Some(SceneAction::Load(path)) => match crate::scenes::load_counting(&path) {
            Ok((saved, left_out)) => {
                tracing::info!("Scene loaded: {}", saved.name);
                scene.apply_saved(&saved);
                // Indices into the scene that was just replaced.
                selection.deselect();
                *active = Some(saved.name);
                *config_dirty = true;
                if left_out > 0 {
                    error = Some(crate::i18n::t("toast-scene-partly-read"));
                }
            }
            Err(e) => {
                let mut args = fluent::FluentArgs::new();
                args.set("error", e.said());
                error = Some(crate::i18n::t_args("toast-scene-failed", &args));
            }
        },
        Some(SceneAction::SaveOver(target)) => {
            error = save_scene_as(scene, target, active, config_dirty).err();
        }
        Some(SceneAction::SaveNew(target)) => {
            match save_scene_as(scene, target, active, config_dirty) {
                Ok(()) => name.clear(),
                Err(e) => error = Some(e),
            }
        }
        Some(SceneAction::Delete(path)) => {
            deleting = None;
            if let Err(e) = crate::scenes::delete(&path) {
                let mut args = fluent::FluentArgs::new();
                args.set("error", e.said());
                error = Some(crate::i18n::t_args("scene-scene-delete-failed", &args));
            }
        }
        None => {}
    }
    ui.data_mut(|d| {
        d.insert_temp(name_key, name);
        match &deleting {
            Some(path) => d.insert_temp(deleting_key, path.clone()),
            None => d.remove::<std::path::PathBuf>(deleting_key),
        }
        if acted {
            match error {
                Some(e) => d.insert_temp(error_key, e),
                None => d.remove::<String>(error_key),
            }
        }
    });
}

/// The picture of the scene saved in `scene_file` (`crate::scene_thumb`),
/// as a texture: read again when its file changes, which is looked at
/// every couple of seconds.
fn scene_thumbnail(ui: &egui::Ui, scene_file: &std::path::Path) -> Option<egui::TextureHandle> {
    use std::time::{Duration, Instant, SystemTime};
    #[derive(Clone)]
    struct Held {
        checked: Instant,
        modified: Option<SystemTime>,
        texture: Option<egui::TextureHandle>,
    }
    let png = crate::scene_thumb::path_for(scene_file);
    let id = egui::Id::new(("anima.scene-thumb", png.as_path()));
    let held = ui.ctx().memory(|m| m.data.get_temp::<Held>(id));
    if let Some(held) = &held {
        if held.checked.elapsed() < Duration::from_secs(2) {
            return held.texture.clone();
        }
    }
    let modified = std::fs::metadata(&png).and_then(|m| m.modified()).ok();
    let texture = match held {
        Some(held) if held.modified == modified => held.texture,
        _ => modified
            .and_then(|_| crate::scene_thumb::load(&png).ok())
            .map(|image| {
                let size = [image.width() as usize, image.height() as usize];
                let color = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
                ui.ctx().load_texture(
                    format!("scene-thumb-{}", png.display()),
                    color,
                    egui::TextureOptions::LINEAR,
                )
            }),
    };
    ui.ctx().memory_mut(|m| {
        m.data.insert_temp(
            id,
            Held {
                checked: Instant::now(),
                modified,
                texture: texture.clone(),
            },
        )
    });
    texture
}

/// Save what is on screen as the scene `name`, which becomes the active
/// one; the error, worded for the panel, if it could not.
fn save_scene_as(
    scene: &Scene,
    name: String,
    active: &mut Option<String>,
    config_dirty: &mut bool,
) -> Result<(), String> {
    match crate::scenes::save_in(&crate::scenes::dir(), &name, scene) {
        Ok(_) => {
            *active = Some(name);
            *config_dirty = true;
            Ok(())
        }
        Err(e) => {
            let mut args = fluent::FluentArgs::new();
            args.set("error", e.said());
            Err(crate::i18n::t_args("scene-scene-save-failed", &args))
        }
    }
}

/// Sprite groups (C.8; editable since 1.5). A group's name selects its
/// characters; beside it, rename, show / hide, and ungroup. Groups are
/// made from the selection (`crate::group`). This used to be a read-only
/// list — groups could only be written into config.toml by hand — with
/// its counts in untranslated English.
fn groups_section(
    ui: &mut egui::Ui,
    scene: &mut Scene,
    selection: &mut SelectionState,
    config_dirty: &mut bool,
) {
    let weak = ui.visuals().weak_text_color();
    if scene.groups.is_empty() {
        ui.add(
            egui::Label::new(
                egui::RichText::new(t("scene-groups-empty-hint"))
                    .text_style(theme::caption())
                    .color(weak),
            )
            .wrap(),
        );
        return;
    }

    let renaming_key = egui::Id::new("anima.group-renaming");
    let mut renaming: Option<Renaming> = ui.data(|d| d.get_temp(renaming_key));
    let selected = selection.selected_ids(scene);
    let mut action: Option<GroupAction> = None;
    let mut visibility_changed = false;
    for group in &mut scene.groups {
        ui.horizontal(|ui| {
            match renaming.as_mut().filter(|r| r.group_id == group.id) {
                Some(r) => {
                    let field =
                        ui.add(egui::TextEdit::singleline(&mut group.name).desired_width(150.0));
                    crate::ui::accessible::name_text_field(&field, &t("scene-group-rename"));
                    if !r.focused {
                        field.request_focus();
                        // The name selected, as a rename has it, so what is
                        // typed replaces it. The field kept its cursor from
                        // the last rename instead — at the start after one
                        // emptied — and typing went in before the name.
                        let mut state = egui::text_edit::TextEditState::load(ui.ctx(), field.id)
                            .unwrap_or_default();
                        state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::two(
                                egui::text::CCursor::new(0),
                                egui::text::CCursor::new(group.name.chars().count()),
                            )));
                        state.store(ui.ctx(), field.id);
                        r.focused = true;
                    }
                    if field.changed() {
                        *config_dirty = true;
                    }
                    if field.lost_focus() {
                        // Escape takes the rename back; Enter, Tab or a click
                        // elsewhere keeps it.
                        if ui.input(|i| i.key_pressed(egui::Key::Escape))
                            || group.name.trim().is_empty()
                        {
                            group.name = r.original.clone();
                        }
                        renaming = None;
                    }
                }
                None => {
                    let all_selected = !group.member_ids.is_empty()
                        && group.member_ids.iter().all(|m| selected.contains(m));
                    if ui
                        .selectable_label(all_selected, &group.name)
                        .on_hover_text(t("scene-group-select-tooltip"))
                        .clicked()
                    {
                        action = Some(GroupAction::Select(group.id.clone()));
                    }
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button(icons::UNGROUP)
                    .named(format!("{}: {}", t("scene-group-ungroup"), group.name))
                    .on_hover_text(t("scene-group-ungroup"))
                    .clicked()
                {
                    action = Some(GroupAction::Dissolve(group.id.clone()));
                }
                let (icon, label) = if group.visible {
                    (icons::VISIBLE, t("scene-group-hide"))
                } else {
                    (icons::HIDDEN, t("scene-group-show"))
                };
                if ui
                    .small_button(icon)
                    .named(format!("{label}: {}", group.name))
                    .on_hover_text(&label)
                    .clicked()
                {
                    group.visible = !group.visible;
                    visibility_changed = true;
                }
                if ui
                    .small_button(icons::RENAME)
                    .named(format!("{}: {}", t("scene-group-rename"), group.name))
                    .on_hover_text(t("scene-group-rename"))
                    .clicked()
                {
                    renaming = Some(Renaming {
                        group_id: group.id.clone(),
                        original: group.name.clone(),
                        focused: false,
                    });
                }
                let mut args = fluent::FluentArgs::new();
                args.set("count", group.member_ids.len());
                ui.label(
                    egui::RichText::new(crate::i18n::t_args("scene-group-members", &args))
                        .text_style(theme::caption())
                        .color(weak),
                );
            });
        });
    }
    ui.data_mut(|d| match &renaming {
        Some(r) => d.insert_temp(renaming_key, r.clone()),
        None => d.remove::<Renaming>(renaming_key),
    });

    if visibility_changed {
        scene.mark_visible_dirty();
        *config_dirty = true;
    }
    match action {
        Some(GroupAction::Select(id)) => {
            let members: Vec<usize> = scene
                .groups
                .iter()
                .find(|g| g.id == id)
                .map(|g| {
                    scene
                        .entities
                        .iter()
                        .enumerate()
                        .filter(|(_, e)| g.member_ids.contains(&e.id))
                        .map(|(i, _)| i)
                        .collect()
                })
                .unwrap_or_default();
            selection.select_all_of(&members);
        }
        Some(GroupAction::Dissolve(id)) if scene.dissolve_group(&id) => {
            *config_dirty = true;
        }
        _ => {}
    }
}

fn scene_list(
    ui: &mut egui::Ui,
    scene: &mut Scene,
    selection: &mut SelectionState,
    config_dirty: &mut bool,
) {
    // Gather actions to apply *after* the loop so we don't hold a borrow
    // of scene.entities while we mutate the scene.
    let mut action: Option<ListAction> = None;

    // Selection pulse — design-system §6, sine 2s cycle, low amplitude.
    // We paint a subtle accent stripe at the left of the selected row
    // after the row itself has been laid out, so a keyboard-only user
    // can spot which row Tab landed on without scanning opacity / weight
    // differences.
    // Reduced motion holds it still. Otherwise a slow, faint wave needs
    // no more than ten frames a second; asking for every frame kept the
    // whole overlay drawing at the display's rate while a row was
    // selected.
    let reduced = crate::ui::motion::reduced(ui.ctx());
    let now = ui.ctx().input(|i| i.time);
    let pulse_alpha = if reduced { 1.0 } else { pulse_alpha_at(now) };
    if selection.selected_index().is_some() && !reduced {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(100));
    }
    let accent = ui.visuals().selection.stroke.color;
    let delete_tooltip = t("menu-delete");

    for (idx, entity) in scene.entities.iter().enumerate() {
        let is_selected = selection.is_selected(idx);
        let row_response = ui.horizontal(|ui| {
            // Hidden by its group too: hiding one left its rows as if
            // nothing had changed, its characters gone from the screen.
            // And said in words, as a reader leaves the icon out.
            let row = if scene.visible_any_hour(idx) {
                ui.selectable_label(is_selected, &entity.name)
            } else {
                let mut args = fluent::FluentArgs::new();
                args.set("name", entity.name.clone());
                let name = crate::i18n::t_args("scene-character-hidden", &args);
                let row =
                    ui.selectable_label(is_selected, format!("{}  {}", icons::HIDDEN, entity.name));
                let enabled = row.enabled();
                row.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::SelectableLabel,
                        enabled,
                        is_selected,
                        &name,
                    )
                });
                row
            };
            if row.clicked() {
                action = Some(ListAction::Select(idx));
            }
            if ui
                .small_button(icons::TRASH)
                .named(format!("{delete_tooltip}: {}", entity.name))
                .on_hover_text(&delete_tooltip)
                .clicked()
            {
                action = Some(ListAction::Delete(idx));
            }
        });
        if is_selected {
            let rect = row_response.response.rect;
            let stripe = egui::Rect::from_min_max(
                rect.left_top(),
                egui::pos2(rect.left() + 3.0, rect.bottom()),
            );
            ui.painter()
                .rect_filled(stripe, 1.5, accent.gamma_multiply(pulse_alpha));
        }
    }

    match action {
        // Shift+click adds or removes a row, as on the overlay.
        Some(ListAction::Select(idx)) if ui.input(|i| i.modifiers.shift) => {
            selection.toggle(idx);
        }
        Some(ListAction::Select(idx)) => {
            selection.select(idx);
        }
        Some(ListAction::Delete(idx)) if scene.remove_entity(idx).is_some() => {
            // The rest of the selection stays selected.
            selection.removed(idx);
            *config_dirty = true;
        }
        _ => {}
    }
}

enum ListAction {
    Select(usize),
    Delete(usize),
}
