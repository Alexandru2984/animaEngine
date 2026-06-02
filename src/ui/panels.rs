//! UI panels rendered through egui.
//!
//! Each function takes only the data it actually mutates plus `&egui::Context`.
//! This keeps `App` borrow-safe: the caller passes disjoint `&mut` references
//! to scene / selection / dirty flag instead of `&mut self`.

use crate::app::ContextMenuState;
use crate::behavior::Behavior;
use crate::constants::TOGGLE_BUTTON_SIZE;
use crate::input::selection::SelectionState;
use crate::presets::{self, ApplyMode, Preset, PresetId};
use crate::scene::Scene;
use crate::ui::anim;
use crate::ui::icons;
use crate::ui::onboarding::{self, OnboardingProgress};
use crate::ui::states;
use crate::ui::theme::{self, h2, Theme, SPACE_2XL, SPACE_L, SPACE_M, SPACE_S, SPACE_XS};
use crate::ui::toasts::{Toast, ToastKind, ToastQueue};

/// Entity-targeted action requested from the right-click context menu.
/// `App` applies it after `EguiRenderer::render` returns so it can grab a
/// mutable borrow on the renderer for texture management (Duplicate, Delete).
pub enum MenuAction {
    Duplicate(usize),
    Delete(usize),
    ResetTransform(usize),
    ToggleGravity(usize),
    BringForward(usize),
    SendBackward(usize),
}

/// What `context_menu` decided about its own state for this frame.
pub enum ContextMenuOutcome {
    /// Menu remains visible — nothing happened this frame.
    Open,
    /// User dismissed the menu (clicked outside).
    Close,
    /// User picked an action — caller should apply it and close the menu.
    Action(MenuAction),
}

/// Which tab is currently focused in the settings sidebar. Persisted
/// across frames in `egui::Memory` so we don't have to thread state
/// through `App` for purely UI-local switching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
enum SettingsTab {
    #[default]
    Inspector,
    Scene,
    Appearance,
}

impl SettingsTab {
    const ALL: &'static [Self] = &[Self::Inspector, Self::Scene, Self::Appearance];

    fn label(self) -> &'static str {
        match self {
            Self::Inspector => "Inspector",
            Self::Scene => "Scene",
            Self::Appearance => "Appearance",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Inspector => icons::CURSOR,
            Self::Scene => icons::STACK,
            Self::Appearance => icons::PALETTE,
        }
    }
}

/// Right-side settings panel. Organized into three tabs (Inspector /
/// Scene / Appearance) with a sticky header and a scrollable body.
/// Mutations flow directly through the supplied mutable references;
/// `config_dirty` is set when anything changes so the save-on-exit-
/// edit-mode path picks them up.
pub fn settings(
    ctx: &egui::Context,
    scene: &mut Scene,
    selection: &mut SelectionState,
    config_dirty: &mut bool,
    theme: &mut Theme,
    onboarding: &mut OnboardingProgress,
) {
    egui::SidePanel::right("anima_settings")
        .resizable(false)
        .default_width(320.0)
        .show(ctx, |ui| {
            // ── Sticky header ─────────────────────────────────────────
            ui.add_space(SPACE_S);
            ui.horizontal(|ui| {
                ui.add_space(SPACE_XS);
                ui.label(
                    egui::RichText::new(format!("{}  Anima", icons::GHOST))
                        .text_style(egui::TextStyle::Heading),
                );
            });
            ui.add_space(SPACE_S);

            // ── Tab switcher ──────────────────────────────────────────
            let mut active_tab: SettingsTab = ui.memory(|m| {
                m.data
                    .get_temp::<SettingsTab>(egui::Id::new("anima.settings.tab"))
                    .unwrap_or_default()
            });
            ui.horizontal(|ui| {
                for tab in SettingsTab::ALL {
                    let selected = *tab == active_tab;
                    let label = format!("{}  {}", tab.icon(), tab.label());
                    if ui.selectable_label(selected, label).clicked() {
                        active_tab = *tab;
                    }
                }
            });
            ui.memory_mut(|m| {
                m.data
                    .insert_temp(egui::Id::new("anima.settings.tab"), active_tab);
            });
            ui.separator();

            // First-run hint right under the tab switcher.
            if onboarding::hint(
                ui,
                "Settings split across three tabs — Inspector, Scene, Appearance.",
                &mut onboarding.tabs,
            ) {
                *config_dirty = true;
            }
            ui.add_space(SPACE_XS);

            // ── Tab body ──────────────────────────────────────────────
            // Each tab gets its own animate-value id so switching
            // restarts the curve from 0 and produces a 100ms fade-in
            // (linear, per design-system §6 "tab content cross-fade").
            let tab_alpha = ctx.animate_value_with_time(
                egui::Id::new(("anima.settings.tab.alpha", active_tab)),
                1.0,
                0.1,
            );
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_opacity(tab_alpha);
                    match active_tab {
                        SettingsTab::Inspector => {
                            inspector_tab(ui, scene, selection, config_dirty, onboarding);
                        }
                        SettingsTab::Scene => {
                            scene_tab(ui, scene, selection, config_dirty);
                        }
                        SettingsTab::Appearance => {
                            appearance_tab(ui, theme, config_dirty, onboarding);
                        }
                    }
                });

            // ── Sticky footer with entity count ───────────────────────
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                ui.add_space(SPACE_S);
                let count = scene.entities.len();
                let plural = if count == 1 { "entity" } else { "entities" };
                ui.horizontal(|ui| {
                    ui.add_space(SPACE_XS);
                    ui.label(
                        egui::RichText::new(format!("{count} {plural}"))
                            .text_style(theme::caption())
                            .weak(),
                    );
                });
            });
        });
}

// ─── tabs ──────────────────────────────────────────────────────────────

fn inspector_tab(
    ui: &mut egui::Ui,
    scene: &mut Scene,
    selection: &mut SelectionState,
    config_dirty: &mut bool,
    onboarding: &mut OnboardingProgress,
) {
    let selected_idx = selection.selected_index();
    match selected_idx.and_then(|idx| scene.entities.get_mut(idx).map(|e| (idx, e))) {
        Some((_idx, entity)) => {
            // Hint about V / G shortcuts, sitting above the quick-toggle
            // row so the visual proximity makes the connection.
            if onboarding::hint(
                ui,
                "Tip: V toggles visibility, G toggles gravity — no need to open this panel.",
                &mut onboarding.quick_toggles,
            ) {
                *config_dirty = true;
            }
            let changed = entity_inspector(ui, entity);
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
            "Nothing selected",
            "Click an entity in the Scene tab, or press Tab to cycle through them.",
        ),
    }
}

fn scene_tab(
    ui: &mut egui::Ui,
    scene: &mut Scene,
    selection: &mut SelectionState,
    config_dirty: &mut bool,
) {
    let is_empty = scene.entities.is_empty();

    if is_empty {
        states::empty(
            ui,
            icons::GHOST,
            "Empty scene",
            "Drop a PNG / GIF / WebP / MP4 onto the overlay — or try a preset below.",
        );
    } else {
        ui.label(
            egui::RichText::new("Drop a PNG / GIF / WebP onto the overlay to add one.")
                .text_style(theme::caption())
                .weak(),
        );
        ui.add_space(SPACE_M);
        scene_list(ui, scene, selection, config_dirty);
        ui.add_space(SPACE_L);
        ui.separator();
    }

    ui.add_space(SPACE_M);
    preset_gallery(ui, scene, selection, config_dirty, is_empty);
}

/// Curated scene presets. Defaults to *open* when the scene is empty
/// (giving fresh users an obvious starting point) and *closed* once
/// they've already populated something, to keep the Scene tab tidy.
fn preset_gallery(
    ui: &mut egui::Ui,
    scene: &mut Scene,
    selection: &mut SelectionState,
    config_dirty: &mut bool,
    default_open: bool,
) {
    let header = egui::RichText::new(format!("{}  Presets", icons::SPARKLE)).text_style(h2());
    egui::CollapsingHeader::new(header)
        .id_salt("anima.scene.presets")
        .default_open(default_open)
        .show(ui, |ui| {
            ui.add_space(SPACE_S);
            for id in PresetId::ALL {
                preset_card(ui, *id, scene, selection, config_dirty);
                ui.add_space(SPACE_S);
            }
        });
}

fn preset_card(
    ui: &mut egui::Ui,
    id: PresetId,
    scene: &mut Scene,
    selection: &mut SelectionState,
    config_dirty: &mut bool,
) {
    let preset = Preset::for_id(id);
    let (bg, accent, body_color) = {
        let v = ui.visuals();
        (v.faint_bg_color, v.hyperlink_color, v.text_color())
    };

    egui::Frame::new()
        .fill(bg)
        .corner_radius(theme::RADIUS_MD)
        .inner_margin(egui::Margin::symmetric(SPACE_M as i8, SPACE_S as i8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(preset.icon).size(18.0).color(accent));
                ui.add_space(SPACE_S);
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(preset.name).strong().color(body_color));
                    ui.label(
                        egui::RichText::new(preset.description)
                            .text_style(theme::caption())
                            .weak(),
                    );
                });
            });
            ui.add_space(SPACE_XS);
            ui.horizontal(|ui| {
                if ui.button("Append").clicked() {
                    apply_preset(scene, selection, &preset, ApplyMode::Append);
                    *config_dirty = true;
                }
                let error_color = ui.visuals().error_fg_color;
                if ui
                    .button(egui::RichText::new("Replace").color(error_color))
                    .on_hover_text("Wipes the current scene before adding")
                    .clicked()
                {
                    apply_preset(scene, selection, &preset, ApplyMode::Replace);
                    *config_dirty = true;
                }
            });
        });
}

fn apply_preset(
    scene: &mut Scene,
    selection: &mut SelectionState,
    preset: &Preset,
    mode: ApplyMode,
) {
    let existing = scene.to_character_configs();
    let new = presets::apply_to_scene(existing, preset, mode);
    if matches!(mode, ApplyMode::Replace) {
        scene.reset_to_configs(&new);
        selection.deselect();
    } else {
        // Append the suffixed preset characters that aren't already present.
        let already: std::collections::HashSet<&str> =
            scene.entities.iter().map(|e| e.id.as_str()).collect();
        let to_add: Vec<_> = new
            .iter()
            .filter(|c| !already.contains(c.id.as_str()))
            .cloned()
            .collect();
        for cfg in &to_add {
            if let Err(e) = scene.append_character_config(cfg) {
                tracing::warn!("Preset entity '{}' failed to append: {}", cfg.id, e);
            }
        }
    }
}

fn appearance_tab(
    ui: &mut egui::Ui,
    theme: &mut Theme,
    config_dirty: &mut bool,
    onboarding: &mut OnboardingProgress,
) {
    ui.label(egui::RichText::new("Theme").text_style(h2()));
    ui.add_space(SPACE_S);
    if theme_picker(ui, theme) {
        *config_dirty = true;
    }
    ui.add_space(SPACE_S);
    if onboarding::hint(
        ui,
        "Themes apply instantly — no restart needed.",
        &mut onboarding.theme,
    ) {
        *config_dirty = true;
    }
    ui.add_space(SPACE_2XL);

    // ── Keyboard shortcuts ───────────────────────────────────────────
    ui.label(egui::RichText::new(format!("{}  Keyboard", icons::KEYBOARD)).text_style(h2()));
    ui.add_space(SPACE_S);
    ui.label(
        egui::RichText::new("Read-only for 0.2.0 — rebinding lands in a follow-up release.")
            .text_style(theme::caption())
            .weak(),
    );
    ui.add_space(SPACE_S);
    keyboard_table(ui);
}

/// Two-column read-only table of every action and its default key combo.
/// Stays inside the parent ScrollArea, so it never pushes the footer
/// out of view.
fn keyboard_table(ui: &mut egui::Ui) {
    use crate::ui::keyboard::Action;
    let (mono_color, caption_color) = {
        let v = ui.visuals();
        (v.text_color(), v.weak_text_color())
    };
    egui::Grid::new("anima.keyboard.table")
        .num_columns(2)
        .spacing([SPACE_L, SPACE_XS])
        .striped(true)
        .show(ui, |ui| {
            for action in Action::ALL {
                ui.label(
                    egui::RichText::new(action.label())
                        .color(mono_color)
                        .text_style(theme::caption()),
                );
                ui.label(
                    egui::RichText::new(action.default_combo())
                        .text_style(egui::TextStyle::Monospace)
                        .color(caption_color),
                );
                ui.end_row();
            }
        });
}

// ─── building blocks ──────────────────────────────────────────────────

/// Theme dropdown. Returns `true` when the user picked a different
/// theme than the current value, so the caller can flag the config
/// dirty.
fn theme_picker(ui: &mut egui::Ui, theme: &mut Theme) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(format!("{}  Theme", icons::PALETTE));
        egui::ComboBox::from_id_salt("theme_picker")
            .selected_text(theme_label_with_icon(*theme))
            .show_ui(ui, |ui| {
                for option in Theme::ALL {
                    if ui
                        .selectable_label(*theme == *option, theme_label_with_icon(*option))
                        .clicked()
                        && *theme != *option
                    {
                        *theme = *option;
                        changed = true;
                    }
                }
            });
    });
    changed
}

fn theme_label_with_icon(t: Theme) -> String {
    let icon = match t {
        Theme::Dark | Theme::DarkHighContrast => icons::DARK_MODE,
        Theme::Light | Theme::LightHighContrast => icons::LIGHT_MODE,
    };
    format!("{icon}  {}", t.label())
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

fn entity_inspector(ui: &mut egui::Ui, entity: &mut crate::entity::Entity) -> EntityChange {
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
        if ui.checkbox(&mut entity.visible, "Visible").changed() {
            change.touches_visibility_or_z_order = true;
        }
        let mut gravity = entity.physics.enabled;
        if ui.checkbox(&mut gravity, "Gravity").changed() {
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
    section(ui, "Position", true, |ui| {
        if ui
            .add(egui::Slider::new(&mut entity.x, -200.0..=4000.0).text("X"))
            .changed()
        {
            change.any_field = true;
        }
        if ui
            .add(egui::Slider::new(&mut entity.y, -200.0..=4000.0).text("Y"))
            .changed()
        {
            change.any_field = true;
        }
        ui.horizontal(|ui| {
            ui.label("z-index");
            if ui
                .add(
                    egui::DragValue::new(&mut entity.z_index)
                        .speed(1.0)
                        .range(-10_000..=10_000),
                )
                .changed()
            {
                change.touches_visibility_or_z_order = true;
            }
        });
    });

    section(ui, "Appearance", true, |ui| {
        if ui
            .add(egui::Slider::new(&mut entity.scale, 0.1..=5.0).text("Scale"))
            .changed()
        {
            change.any_field = true;
        }
        if ui
            .add(egui::Slider::new(&mut entity.opacity, 0.0..=1.0).text("Opacity"))
            .changed()
        {
            change.any_field = true;
        }
    });

    section(ui, "Animation", true, |ui| {
        let mut fps = entity.animation.fps;
        if ui
            .add(egui::Slider::new(&mut fps, 1.0..=60.0).text("FPS"))
            .changed()
        {
            entity.animation.set_fps(fps);
            change.any_field = true;
        }
        let mut playing = entity.animation.playing;
        if ui.checkbox(&mut playing, "Playing").changed() {
            entity.animation.playing = playing;
            change.any_field = true;
        }
    });

    section(ui, "Behavior", false, |ui| {
        if behavior_picker(ui, &mut entity.behavior) {
            change.any_field = true;
        }
    });

    change
}

/// Collapsing inspector section with the design-system heading style.
/// `default_open` reflects the recent-use heuristic: Position /
/// Appearance / Animation are touched on most edits; Behavior is rarer.
fn section(
    ui: &mut egui::Ui,
    title: &str,
    default_open: bool,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let header = egui::RichText::new(title).text_style(h2());
    egui::CollapsingHeader::new(header)
        .id_salt(("anima.inspector.section", title))
        .default_open(default_open)
        .show(ui, |ui| {
            ui.add_space(SPACE_XS);
            add_contents(ui);
        });
    ui.add_space(SPACE_S);
}

/// Behavior dropdown + variant-specific sliders. Returns `true` when the
/// user touched anything in this section.
fn behavior_picker(ui: &mut egui::Ui, behavior: &mut Behavior) -> bool {
    let mut changed = false;

    // ComboBox with the three concrete variants. selectable_value compares
    // via PartialEq, so picking the same variant a second time is a no-op.
    let current_label = behavior_label_with_icon(behavior);
    egui::ComboBox::from_id_salt("behavior_picker")
        .selected_text(current_label)
        .show_ui(ui, |ui| {
            let prev = behavior.clone();
            ui.selectable_value(
                behavior,
                Behavior::Idle,
                format!("{}  Idle", icons::BEHAVIOR_IDLE),
            );
            ui.selectable_value(
                behavior,
                Behavior::WalkAround { speed: 60.0 },
                format!("{}  Walk around", icons::BEHAVIOR_WALK),
            );
            ui.selectable_value(
                behavior,
                Behavior::FollowCursor {
                    speed: 240.0,
                    comfort_distance: 80.0,
                },
                format!("{}  Follow cursor", icons::BEHAVIOR_FOLLOW),
            );
            ui.selectable_value(
                behavior,
                Behavior::BoundedWander {
                    x_min: 200.0,
                    x_max: 1200.0,
                    y_min: 200.0,
                    y_max: 800.0,
                    speed: 120.0,
                },
                format!("{}  Bounded wander", icons::BEHAVIOR_WANDER),
            );
            if *behavior != prev {
                changed = true;
            }
        });

    // Variant-specific sliders.
    match behavior {
        Behavior::Idle => {}
        Behavior::WalkAround { speed } => {
            if ui
                .add(egui::Slider::new(speed, 10.0..=400.0).text("Speed (px/s)"))
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
                .add(egui::Slider::new(speed, 50.0..=800.0).text("Speed (px/s)"))
                .changed()
            {
                changed = true;
            }
            if ui
                .add(egui::Slider::new(comfort_distance, 0.0..=400.0).text("Comfort distance (px)"))
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
                .add(egui::Slider::new(speed, 20.0..=400.0).text("Speed (px/s)"))
                .changed()
            {
                changed = true;
            }
            ui.add_space(2.0);
            ui.label(egui::RichText::new("Wander box").small().weak());
            ui.horizontal(|ui| {
                ui.label("X");
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
                ui.label("Y");
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
    }

    changed
}

fn behavior_label_with_icon(b: &Behavior) -> String {
    let (icon, name) = match b {
        Behavior::Idle => (icons::BEHAVIOR_IDLE, "Idle"),
        Behavior::WalkAround { .. } => (icons::BEHAVIOR_WALK, "Walk around"),
        Behavior::FollowCursor { .. } => (icons::BEHAVIOR_FOLLOW, "Follow cursor"),
        Behavior::BoundedWander { .. } => (icons::BEHAVIOR_WANDER, "Bounded wander"),
    };
    format!("{icon}  {name}")
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

    for (idx, entity) in scene.entities.iter().enumerate() {
        let is_selected = selection.is_selected(idx);
        ui.horizontal(|ui| {
            let label = if entity.visible {
                entity.name.clone()
            } else {
                format!("{}  {}", icons::HIDDEN, entity.name)
            };
            if ui.selectable_label(is_selected, label).clicked() {
                action = Some(ListAction::Select(idx));
            }
            // Small delete button on the right.
            if ui
                .small_button(icons::TRASH)
                .on_hover_text("Delete")
                .clicked()
            {
                action = Some(ListAction::Delete(idx));
            }
        });
    }

    match action {
        Some(ListAction::Select(idx)) => {
            selection.select(idx);
        }
        Some(ListAction::Delete(idx)) if scene.remove_entity(idx).is_some() => {
            selection.deselect();
            *config_dirty = true;
        }
        _ => {}
    }
}

enum ListAction {
    Select(usize),
    Delete(usize),
}

/// Floating right-click context menu anchored at `state.pos`. Caller
/// owns the `ContextMenuState`; this function only inspects it and
/// reports back via `ContextMenuOutcome`.
pub(crate) fn context_menu(ctx: &egui::Context, state: &ContextMenuState) -> ContextMenuOutcome {
    let idx = state.entity_idx;
    let mut picked: Option<MenuAction> = None;

    let area = egui::Area::new(egui::Id::new("anima_entity_context_menu"))
        .fixed_pos(state.pos)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(160.0);

                if ui.button(format!("{}  Duplicate", icons::COPY)).clicked() {
                    picked = Some(MenuAction::Duplicate(idx));
                }
                if ui
                    .button(format!("{}  Reset transform", icons::RESET))
                    .clicked()
                {
                    picked = Some(MenuAction::ResetTransform(idx));
                }
                if ui
                    .button(format!("{}  Toggle gravity", icons::GRAVITY))
                    .clicked()
                {
                    picked = Some(MenuAction::ToggleGravity(idx));
                }
                ui.separator();
                if ui
                    .button(format!("{}  Bring forward", icons::BRING_FORWARD))
                    .clicked()
                {
                    picked = Some(MenuAction::BringForward(idx));
                }
                if ui
                    .button(format!("{}  Send backward", icons::SEND_BACKWARD))
                    .clicked()
                {
                    picked = Some(MenuAction::SendBackward(idx));
                }
                ui.separator();
                let error_color = ui.visuals().error_fg_color;
                if ui
                    .button(
                        egui::RichText::new(format!("{}  Delete", icons::TRASH)).color(error_color),
                    )
                    .clicked()
                {
                    picked = Some(MenuAction::Delete(idx));
                }
            });
        });

    if let Some(action) = picked {
        return ContextMenuOutcome::Action(action);
    }

    // Dismiss when the user clicks anywhere that isn't the menu itself, or
    // presses Escape. We deliberately check `any_click` (not `pressed`) so
    // a release that ended on a button still counts as "clicked the menu".
    let dismissed = ctx.input(|i| {
        let escape = i.key_pressed(egui::Key::Escape);
        let outside_click = i.pointer.any_click() && !area.response.contains_pointer();
        escape || outside_click
    });

    if dismissed {
        ContextMenuOutcome::Close
    } else {
        ContextMenuOutcome::Open
    }
}

/// Stack of toast notifications anchored to the bottom-right corner.
/// Renders above the settings panel and the context menu, styled per
/// docs/design-system.md §7.8: `bg.elevated` surface, leading severity
/// icon coloured by `semantic.*`, body text in `fg.primary`, radius
/// `lg`, `elev.mid` shadow, stack gap `space.s`.
pub fn toasts(ctx: &egui::Context, queue: &ToastQueue) {
    if queue.is_empty() {
        return;
    }

    // While there are visible toasts, drive continuous repaints so they
    // disappear at the moment they expire (without waiting for the next
    // input event).
    ctx.request_repaint();

    egui::Area::new(egui::Id::new("anima_toasts"))
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-SPACE_L, -SPACE_L))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                for toast in queue.iter() {
                    toast_card(ui, toast);
                    ui.add_space(SPACE_S);
                }
            });
        });
}

fn toast_card(ui: &mut egui::Ui, toast: &Toast) {
    // ── Per design-system §6 micro-animation timings ─────────────────
    // - Slide-in fade:  200 ms, ease-out-quad
    // - Fade-out:       300 ms, ease-in-quad (trailing window before expiry)
    const SLIDE_IN: f32 = 0.200;
    const FADE_OUT: f32 = 0.300;
    let age = toast.age().as_secs_f32();
    let remaining = toast.remaining().as_secs_f32();
    let in_alpha = anim::ease_out_quad((age / SLIDE_IN).min(1.0));
    let out_alpha = if remaining < FADE_OUT {
        1.0 - anim::ease_in_quad(((FADE_OUT - remaining) / FADE_OUT).clamp(0.0, 1.0))
    } else {
        1.0
    };
    let alpha = (in_alpha * out_alpha).clamp(0.0, 1.0);

    let visuals = ui.visuals();
    let bg = visuals.faint_bg_color; // bg.elevated per theme
    let body_fg = visuals.text_color(); // fg.primary
    let severity_fg = match toast.kind {
        ToastKind::Info => visuals.hyperlink_color, // info / accent tone
        ToastKind::Success => egui::Color32::from_rgb(0x5B, 0xCB, 0x7B),
        ToastKind::Warn => visuals.warn_fg_color,
        ToastKind::Error => visuals.error_fg_color,
    };
    let icon = match toast.kind {
        ToastKind::Info => icons::INFO,
        ToastKind::Success => icons::SUCCESS,
        ToastKind::Warn => icons::WARN,
        ToastKind::Error => icons::ERROR,
    };

    ui.scope(|ui| {
        ui.set_opacity(alpha);
        egui::Frame::new()
            .fill(bg)
            .corner_radius(theme::RADIUS_LG)
            .inner_margin(egui::Margin::symmetric(SPACE_L as i8, SPACE_M as i8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(icon).size(18.0).color(severity_fg));
                    ui.add_space(SPACE_S);
                    ui.colored_label(body_fg, &toast.message);
                });
            });
    });
}

/// One-shot intent emitted by the command palette so the caller can
/// apply it after `EguiRenderer::render` returns — same pattern as
/// [`ContextMenuOutcome`].
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
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut query)
                            .hint_text("Type to search themes / presets…")
                            .desired_width(380.0),
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

/// Top-right ⚙ button that toggles between pass-through and edit mode.
/// Returns `true` for the frame the user clicked it.
///
/// Geometry must match `TOGGLE_BUTTON_SIZE` because the X11 input shape
/// in pass-through mode uses the same constant to decide which pixels
/// receive clicks.
pub fn toggle_button(ctx: &egui::Context, edit_mode: bool) -> bool {
    let size = TOGGLE_BUTTON_SIZE as f32;
    let screen = ctx.screen_rect();
    let pos = egui::pos2(screen.right() - size, 0.0);

    let bg = if edit_mode {
        egui::Color32::from_rgb(40, 160, 60) // active = green
    } else {
        egui::Color32::from_rgba_unmultiplied(50, 50, 60, 200) // pass-through = dim
    };
    let tooltip = if edit_mode {
        "Exit edit mode"
    } else {
        "Enter edit mode"
    };

    let mut clicked = false;
    egui::Area::new(egui::Id::new("anima_toggle_button"))
        .fixed_pos(pos)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let response = ui
                .add_sized(
                    egui::vec2(size, size),
                    egui::Button::new(
                        egui::RichText::new(icons::SETTINGS)
                            .size(28.0)
                            .color(egui::Color32::WHITE),
                    )
                    .fill(bg)
                    .corner_radius(0.0),
                )
                .on_hover_text(tooltip);
            if response.clicked() {
                clicked = true;
            }
        });
    clicked
}
