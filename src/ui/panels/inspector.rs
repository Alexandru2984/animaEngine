//! Inspector tab — properties for the selected entity. Extracted in I.10.
//!
//! The tab consists of a sticky header, two quick-toggle checkboxes
//! (Visible / Gravity) and four CollapsingHeader sections: Position,
//! Appearance, Animation, Behavior. Each section's open-state is
//! persisted in `CollapseState` so the layout survives restarts.
//!
//! Behavior dropdown rebuilds each variant from sensible defaults on
//! switch so users can fiddle without first reading the data sheet.

use super::entity_monitor_picker;
use crate::behavior::Behavior;
use crate::i18n::t;
use crate::input::selection::SelectionState;
use crate::monitor::MonitorInfo;
use crate::scene::Scene;
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
    match selected_idx.and_then(|idx| scene.entities.get_mut(idx).map(|e| (idx, e))) {
        Some((_idx, entity)) => {
            // Hint about V / G shortcuts, sitting above the quick-toggle
            // row so the visual proximity makes the connection.
            if onboarding::hint(
                ui,
                &t("onboarding-quick-toggles"),
                &mut onboarding.quick_toggles,
            ) {
                *config_dirty = true;
            }
            let changed = entity_inspector(ui, entity, monitors, collapse_state, config_dirty);
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
    section(
        ui,
        &t("inspector-section-position"),
        &mut collapse_state.inspector_position,
        config_dirty,
        |ui| {
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
            // Monitor pin lives in the Position section because it's
            // conceptually a 3rd axis: x / y / which-screen.
            if entity_monitor_picker(ui, &mut entity.monitor, monitors) {
                change.any_field = true;
            }
        },
    );

    section(
        ui,
        &t("inspector-section-appearance"),
        &mut collapse_state.inspector_appearance,
        config_dirty,
        |ui| {
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
        },
    );

    section(
        ui,
        &t("inspector-section-animation"),
        &mut collapse_state.inspector_animation,
        config_dirty,
        |ui| {
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
            if easing_picker(ui, &mut entity.animation.easing) {
                change.any_field = true;
            }
        },
    );

    section(
        ui,
        &t("inspector-section-behavior"),
        &mut collapse_state.inspector_behavior,
        config_dirty,
        |ui| {
            if behavior_picker(ui, &mut entity.behavior) {
                change.any_field = true;
            }
        },
    );

    change
}

/// Collapsing inspector section with the design-system heading style.
///
/// `open` is the persisted flag from [`CollapseState`] — used as the
/// first-frame seed via `default_open` and synced back from the
/// widget's animated openness so a user click toggles config-dirty
/// exactly once (not per frame during the expand/collapse animation).
fn section(
    ui: &mut egui::Ui,
    title: &str,
    open: &mut bool,
    config_dirty: &mut bool,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let header = egui::RichText::new(title).text_style(h2());
    let response = egui::CollapsingHeader::new(header)
        .id_salt(("anima.inspector.section", title))
        .default_open(*open)
        .show(ui, |ui| {
            ui.add_space(SPACE_XS);
            add_contents(ui);
        });
    // `openness` animates 0.0..=1.0. Crossing 0.5 == user-visible
    // state flipped — write the new value back to the persisted bool
    // and flag dirty exactly once per toggle.
    let visually_open = response.openness > 0.5;
    if visually_open != *open {
        *open = visually_open;
        *config_dirty = true;
    }
    ui.add_space(SPACE_S);
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
        egui::ComboBox::from_id_salt("anima.animation.easing")
            .selected_text(active_label)
            .show_ui(ui, |ui| {
                let is_linear = easing.is_none() || matches!(easing, Some(EasingCurve::Linear));
                if ui.selectable_label(is_linear, t("easing-linear")).clicked() && !is_linear {
                    *easing = None;
                    changed = true;
                }
                for &c in EasingCurve::ALL {
                    if matches!(c, EasingCurve::Linear) {
                        continue;
                    }
                    let is_current = matches!(easing, Some(x) if *x == c);
                    if ui.selectable_label(is_current, t(c.i18n_key())).clicked() && !is_current {
                        *easing = Some(c);
                        changed = true;
                    }
                }
            });
    });
    changed
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
            ui.selectable_value(
                behavior,
                Behavior::Bounce {
                    amplitude_px: 24.0,
                    period_sec: 1.5,
                    axis: crate::behavior::BounceAxis::Vertical,
                },
                format!("{}  Bounce", icons::BEHAVIOR_BOUNCE),
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
        Behavior::Bounce {
            amplitude_px,
            period_sec,
            axis,
        } => {
            if ui
                .add(egui::Slider::new(amplitude_px, 1.0..=200.0).text("Amplitude (px)"))
                .changed()
            {
                changed = true;
            }
            if ui
                .add(egui::Slider::new(period_sec, 0.1..=10.0).text("Period (s)"))
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
    let (icon, name) = match b {
        Behavior::Idle => (icons::BEHAVIOR_IDLE, "Idle"),
        Behavior::WalkAround { .. } => (icons::BEHAVIOR_WALK, "Walk around"),
        Behavior::FollowCursor { .. } => (icons::BEHAVIOR_FOLLOW, "Follow cursor"),
        Behavior::BoundedWander { .. } => (icons::BEHAVIOR_WANDER, "Bounded wander"),
        Behavior::Bounce { .. } => (icons::BEHAVIOR_BOUNCE, "Bounce"),
    };
    format!("{icon}  {name}")
}
