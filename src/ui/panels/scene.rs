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
use crate::ui::accessible::{AccessibleName, ComboOption};
use crate::ui::collapse::CollapseState;
use crate::ui::icons;
use crate::ui::states;
use crate::ui::theme::{self, h2, SPACE_L, SPACE_M, SPACE_S};

// UI plumbing fans out one settings struct into per-tab params — same
// allow as `panels::settings` itself.
#[allow(clippy::too_many_arguments)]
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
    active_scene: &mut Option<String>,
    reminders: &mut Vec<crate::reminders::ReminderConfig>,
    scene_schedule: &mut Vec<crate::schedule::ScheduleRule>,
    // Kept separate from window-awareness even though both are false on
    // the same backend today: they are different capabilities, and a
    // future backend could have one without the other.
    span_supported: bool,
    monitors: &[MonitorInfo],
    collapse_state: &mut CollapseState,
    add_file_requested: &mut bool,
) {
    // ── Monitor distribution section ─────────────────────────────────
    monitor_mode_picker(ui, monitor_mode, monitors, span_supported, config_dirty);

    // ── Window awareness (X11) ────────────────────────────────────────
    //
    // Greyed out where it cannot work. The control used to stay live on
    // every backend with only a tooltip to explain it, so on native
    // Wayland a user could tick it, see nothing happen, and have no way
    // to tell whether the feature or their setup was at fault. The
    // display server is the wrong thing to test — a Wayland session
    // running us through XWayland reads EWMH just fine — so the caller
    // passes what it actually has.
    ui.add_space(SPACE_S);
    ui.add_enabled_ui(window_awareness_supported, |ui| {
        if ui
            .checkbox(window_awareness, t("scene-window-awareness"))
            .on_hover_text(t("scene-window-awareness-tooltip"))
            .changed()
        {
            *config_dirty = true;
        }
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
    // Snapping while dragging (1.5), on both backends.
    ui.add_space(SPACE_S);
    if ui
        .checkbox(snap_while_dragging, t("scene-snap"))
        .on_hover_text(t("scene-snap-tooltip"))
        .changed()
    {
        *config_dirty = true;
    }
    // Characters bumping into each other (1.5), on both backends.
    if ui
        .checkbox(characters_bump, t("scene-bump"))
        .on_hover_text(t("scene-bump-tooltip"))
        .changed()
    {
        *config_dirty = true;
    }
    ui.add_space(SPACE_L);
    ui.separator();
    ui.add_space(SPACE_M);

    // The way in that needs no dragging — and, in the Flatpak, the one
    // that reaches files outside Pictures and Downloads. Unix only: it
    // goes through the XDG desktop portal.
    if cfg!(unix)
        && ui
            .button(format!("{}  {}", icons::ADD, t("scene-add-file")))
            .on_hover_text(t("scene-add-file-tooltip"))
            .clicked()
    {
        *add_file_requested = true;
    }
    ui.add_space(SPACE_M);

    let is_empty = scene.entities.is_empty();

    if is_empty {
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
            *config_dirty = true;
        }
    } else {
        ui.label(
            egui::RichText::new(t("scene-drop-hint"))
                .text_style(theme::caption())
                .weak(),
        );
        ui.add_space(SPACE_M);
        scene_list(ui, scene, selection, config_dirty);
        // Right under the characters they are made of — with a hint on
        // how, until there are any. Below the preset gallery it was off
        // the bottom of the panel.
        ui.add_space(SPACE_L);
        groups_section(ui, scene, selection, config_dirty);
    }
    // Saved scenes, even with none on screen: one may be loaded.
    ui.add_space(SPACE_L);
    scenes_section(ui, scene, selection, config_dirty, active_scene);
    schedule_section(ui, scene_schedule, config_dirty);
    ui.add_space(SPACE_L);
    reminders_section(ui, scene, reminders, config_dirty);
    if !is_empty {
        ui.add_space(SPACE_L);
        ui.separator();
    }

    ui.add_space(SPACE_M);
    preset_gallery(
        ui,
        scene,
        selection,
        config_dirty,
        &mut collapse_state.scene_presets,
    );
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

/// The reminder being written below the list, kept between frames.
#[derive(Clone)]
struct ReminderDraft {
    text: String,
    minutes: u32,
    character: Option<String>,
}

impl Default for ReminderDraft {
    fn default() -> Self {
        Self {
            text: String::new(),
            minutes: 60,
            character: None,
        }
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
    ui.label(
        egui::RichText::new(format!(
            "{}  {}",
            icons::REMINDER,
            t("scene-reminders-header")
        ))
        .text_style(h2()),
    );
    ui.add_space(SPACE_S);
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
                    every(r.every_minutes),
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
    let mut add = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    ui.horizontal(|ui| {
        let label = ui.label(t("scene-reminder-minutes"));
        ui.add(
            egui::DragValue::new(&mut draft.minutes)
                .range(MIN_MINUTES..=MAX_MINUTES)
                .speed(1.0),
        )
        .labelled_by(label.id);
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
    });
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
        ui.add(egui::DragValue::new(&mut draft.hour).range(0..=23))
            .labelled_by(hour.id);
        ui.label(":");
        let minute = ui.add(egui::DragValue::new(&mut draft.minute).range(0..=59));
        crate::ui::accessible::name_text_field(&minute, &t("scene-schedule-minute"));
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
) {
    ui.label(
        egui::RichText::new(format!("{}  {}", icons::SCENE, t("scene-scenes-header")))
            .text_style(h2()),
    );
    ui.add_space(SPACE_S);
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
                }
            });
        });
    }

    ui.add_space(SPACE_S);
    let mut name: String = ui.data(|d| d.get_temp(name_key)).unwrap_or_default();
    ui.horizontal(|ui| {
        let field = ui.add(
            egui::TextEdit::singleline(&mut name)
                .hint_text(t("scene-scene-name-hint"))
                .desired_width(150.0),
        );
        crate::ui::accessible::name_text_field(&field, &t("scene-scene-name-hint"));
        let entered = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let clicked = ui
            .add_enabled(
                !name.trim().is_empty(),
                egui::Button::new(format!("{}  {}", icons::SAVE, t("scene-scene-save-new"))),
            )
            .clicked();
        if (entered || clicked) && !name.trim().is_empty() {
            action = Some(SceneAction::SaveNew(name.trim().to_string()));
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
        Some(SceneAction::Load(path)) => match crate::scenes::load(&path) {
            Ok(saved) => {
                tracing::info!("Scene loaded: {}", saved.name);
                scene.apply_saved(&saved);
                // Indices into the scene that was just replaced.
                selection.deselect();
                *active = Some(saved.name);
                *config_dirty = true;
            }
            Err(e) => {
                let mut args = fluent::FluentArgs::new();
                args.set("error", e.to_string());
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
                error = Some(e.to_string());
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
            args.set("error", e.to_string());
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
    ui.label(
        egui::RichText::new(format!("{}  {}", icons::STACK, t("scene-groups-header")))
            .text_style(h2()),
    );
    ui.add_space(SPACE_S);
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
                        r.focused = true;
                    }
                    if field.changed() {
                        *config_dirty = true;
                    }
                    if field.lost_focus() {
                        if group.name.trim().is_empty() {
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
            let label = if entity.visible {
                entity.name.clone()
            } else {
                format!("{}  {}", icons::HIDDEN, entity.name)
            };
            if ui.selectable_label(is_selected, label).clicked() {
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
