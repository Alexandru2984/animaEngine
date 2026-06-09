//! UI panels rendered through egui.
//!
//! Each function takes only the data it actually mutates plus `&egui::Context`.
//! This keeps `App` borrow-safe: the caller passes disjoint `&mut` references
//! to scene / selection / dirty flag instead of `&mut self`.

mod appearance;
mod command_palette;
mod context_menu;
mod keybindings_tab;
mod library;
mod scene;
mod toasts;
mod toggle_button;

pub use command_palette::{command_palette, PaletteOutcome};
pub(crate) use context_menu::context_menu;
pub use library::LibraryOutcome;
pub use toasts::toasts;
pub use toggle_button::toggle_button;

use crate::asset_library::LibraryIndex;
use crate::behavior::Behavior;
use crate::i18n::t;
use crate::input::selection::SelectionState;
use crate::keybindings::KeyBindings;
use crate::monitor::{MonitorInfo, MonitorMode};
use crate::presets::{self, ApplyMode, Preset, PresetId};
use crate::scene::Scene;
use crate::ui::banner::Warning;
use crate::ui::collapse::CollapseState;
use crate::ui::icons;
use crate::ui::onboarding::{self, OnboardingProgress};
use crate::ui::states;
use crate::ui::theme::{self, h2, Theme, SPACE_M, SPACE_S, SPACE_XS};

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
    Library,
    Appearance,
    Keybindings,
}

impl SettingsTab {
    const ALL: &'static [Self] = &[
        Self::Inspector,
        Self::Scene,
        Self::Library,
        Self::Appearance,
        Self::Keybindings,
    ];

    fn label(self) -> String {
        match self {
            Self::Inspector => t("settings-tab-inspector"),
            Self::Scene => t("settings-tab-scene"),
            Self::Library => t("settings-tab-library"),
            Self::Appearance => t("settings-tab-appearance"),
            Self::Keybindings => t("settings-tab-keybindings"),
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Inspector => icons::CURSOR,
            Self::Scene => icons::STACK,
            Self::Library => icons::LIBRARY,
            Self::Appearance => icons::PALETTE,
            Self::Keybindings => icons::KEYBOARD,
        }
    }
}

/// Right-side settings panel. Organized into three tabs (Inspector /
/// Scene / Appearance) with a sticky header and a scrollable body.
/// Mutations flow directly through the supplied mutable references;
/// `config_dirty` is set when anything changes so the save-on-exit-
/// edit-mode path picks them up.
#[allow(clippy::too_many_arguments)]
pub fn settings(
    ctx: &egui::Context,
    scene: &mut Scene,
    selection: &mut SelectionState,
    config_dirty: &mut bool,
    theme: &mut Theme,
    locale: &mut Option<String>,
    onboarding: &mut OnboardingProgress,
    monitor_mode: &mut MonitorMode,
    monitors: &[MonitorInfo],
    library: Option<&LibraryIndex>,
    library_outcome: &mut Option<LibraryOutcome>,
    keybindings: &mut KeyBindings,
    collapse_state: &mut CollapseState,
    accesskit_enabled: &mut bool,
    warnings: &std::collections::BTreeSet<Warning>,
    last_seen_whats_new: &mut Option<String>,
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
                    egui::RichText::new(format!("{}  {}", icons::GHOST, t("app-name")))
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

            // ── Banners (session-lifetime warnings) ──────────────────
            // Rendered between the tab switcher and the tab body so
            // the user notices them the moment the panel opens; they
            // never appear in pass-through mode (the whole panel is
            // hidden there). Auto-disappear when the underlying
            // condition clears (see App::clear_warning).
            if !warnings.is_empty() {
                for warning in warnings {
                    appearance::warning_banner(ui, *warning);
                }
                ui.add_space(SPACE_XS);
            }

            // ── What's new panel (D.7) ───────────────────────────────
            // One-shot per minor-version bump; dismissing stamps the
            // current WHATS_NEW_VERSION into the config so the next
            // session skips the panel.
            if crate::ui::whats_new::show(ui, last_seen_whats_new) {
                *config_dirty = true;
            }

            // First-run hint right under the tab switcher.
            if onboarding::hint(ui, &t("onboarding-tabs"), &mut onboarding.tabs) {
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
                            inspector_tab(
                                ui,
                                scene,
                                selection,
                                config_dirty,
                                onboarding,
                                monitors,
                                collapse_state,
                            );
                        }
                        SettingsTab::Scene => {
                            scene::scene_tab(
                                ui,
                                scene,
                                selection,
                                config_dirty,
                                monitor_mode,
                                monitors,
                                collapse_state,
                            );
                        }
                        SettingsTab::Library => {
                            library::library_tab(ui, library, library_outcome);
                        }
                        SettingsTab::Appearance => {
                            appearance::appearance_tab(
                                ui,
                                theme,
                                locale,
                                config_dirty,
                                onboarding,
                                accesskit_enabled,
                            );
                        }
                        SettingsTab::Keybindings => {
                            // First-time hint about the rebinding UX (D.7).
                            if onboarding::hint(
                                ui,
                                &t("onboarding-keybindings"),
                                &mut onboarding.keybindings_tab,
                            ) {
                                *config_dirty = true;
                            }
                            keybindings_tab::keybindings_tab(ctx, ui, keybindings, config_dirty);
                        }
                    }
                });

            // ── Sticky footer with entity count ───────────────────────
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                ui.add_space(SPACE_S);
                let count = scene.entities.len();
                let label = entity_count_label(count);
                ui.horizontal(|ui| {
                    ui.add_space(SPACE_XS);
                    ui.label(
                        egui::RichText::new(label)
                            .text_style(theme::caption())
                            .weak(),
                    );
                });
            });
        });
}

// ─── monitor pickers ────────────────────────────────────────────────

/// Scene-tab section that picks the global monitor distribution
/// (`PerMonitor` / `Span` / `Single { name }`). Returns nothing —
/// flips `config_dirty` directly on change.
pub(super) fn monitor_mode_picker(
    ui: &mut egui::Ui,
    mode: &mut MonitorMode,
    monitors: &[MonitorInfo],
    config_dirty: &mut bool,
) {
    ui.label(egui::RichText::new(t("monitor-section-header")).text_style(h2()));
    ui.add_space(SPACE_S);

    if monitors.is_empty() {
        ui.label(
            egui::RichText::new(t("monitor-no-monitors-detected"))
                .text_style(theme::caption())
                .weak(),
        );
        return;
    }

    ui.horizontal(|ui| {
        ui.label(t("monitor-mode-label"));
        egui::ComboBox::from_id_salt("anima.monitor.mode")
            .selected_text(monitor_mode_label_localised(mode))
            .show_ui(ui, |ui| {
                let mut new_mode = mode.clone();
                if ui
                    .selectable_label(
                        matches!(mode, MonitorMode::PerMonitor),
                        t("monitor-mode-per-monitor"),
                    )
                    .clicked()
                {
                    new_mode = MonitorMode::PerMonitor;
                }
                if ui
                    .selectable_label(matches!(mode, MonitorMode::Span), t("monitor-mode-span"))
                    .clicked()
                {
                    new_mode = MonitorMode::Span;
                }
                // Single-mode requires a named monitor; offer one entry
                // per monitor so the user picks both the mode and the
                // target in one click.
                for m in monitors {
                    let is_current =
                        matches!(mode, MonitorMode::Single { name } if name == &m.name);
                    let label = format!("{} — {}", t("monitor-mode-single"), m.name);
                    if ui.selectable_label(is_current, label).clicked() {
                        new_mode = MonitorMode::Single {
                            name: m.name.clone(),
                        };
                    }
                }
                if &new_mode != mode {
                    *mode = new_mode;
                    *config_dirty = true;
                }
            });
    });

    // Compact list of detected monitors for orientation.
    ui.add_space(SPACE_S);
    ui.label(
        egui::RichText::new(monitor_topology_summary(monitors))
            .text_style(theme::caption())
            .weak(),
    );
}

/// Inspector picker for the per-entity monitor pin. Returns `true`
/// when the user changed the selection.
fn entity_monitor_picker(
    ui: &mut egui::Ui,
    pin: &mut Option<String>,
    monitors: &[MonitorInfo],
) -> bool {
    if monitors.is_empty() {
        return false;
    }
    let mut changed = false;
    let active_label = match pin {
        None => t("monitor-pin-auto"),
        Some(name) => name.clone(),
    };
    ui.horizontal(|ui| {
        ui.label(t("monitor-pin-label"));
        egui::ComboBox::from_id_salt("anima.entity.monitor")
            .selected_text(active_label)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(pin.is_none(), t("monitor-pin-auto"))
                    .clicked()
                    && pin.is_some()
                {
                    *pin = None;
                    changed = true;
                }
                for m in monitors {
                    let is_current = pin.as_deref() == Some(m.name.as_str());
                    if ui.selectable_label(is_current, &m.name).clicked() && !is_current {
                        *pin = Some(m.name.clone());
                        changed = true;
                    }
                }
            });
    });
    changed
}

/// Cycle the entity's monitor pin in declaration order. Used by the
/// `Ctrl+M` hotkey. Returns the localised toast message describing
/// the new state, so the caller can dispatch it.
///
/// Cycle: `None` → first monitor → second → … → last → `None`.
pub fn cycle_entity_monitor(pin: &mut Option<String>, monitors: &[MonitorInfo]) -> String {
    if monitors.is_empty() {
        return t("monitor-no-monitors-detected");
    }
    let next = match pin.as_deref() {
        None => Some(monitors[0].name.clone()),
        Some(current) => match monitors.iter().position(|m| m.name == current) {
            // Currently pinned to a monitor that no longer exists →
            // restart the cycle from the first available.
            None => Some(monitors[0].name.clone()),
            Some(i) if i + 1 < monitors.len() => Some(monitors[i + 1].name.clone()),
            // Last monitor → wrap to auto.
            Some(_) => None,
        },
    };
    *pin = next.clone();
    match next {
        Some(n) => {
            let mut args = fluent::FluentArgs::new();
            args.set("name", n);
            crate::i18n::t_args("monitor-pinned-toast", &args)
        }
        None => t("monitor-pin-cleared-toast"),
    }
}

fn monitor_mode_label_localised(mode: &MonitorMode) -> String {
    match mode {
        MonitorMode::PerMonitor => t("monitor-mode-per-monitor"),
        MonitorMode::Span => t("monitor-mode-span"),
        MonitorMode::Single { name } => format!("{} — {name}", t("monitor-mode-single")),
    }
}

fn monitor_topology_summary(monitors: &[MonitorInfo]) -> String {
    monitors
        .iter()
        .map(|m| {
            let marker = if m.is_primary { " *" } else { "" };
            format!("{}{} ({}×{})", m.name, marker, m.width, m.height)
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

/// Subtle 2s-cycle pulse for the selected scene-list row. Range
/// `[0.45, 1.0]` so the stripe never disappears — it just *breathes*.
/// Matches design-system §6 "selection pulse: sine 2s cycle, low amplitude".
pub(super) fn pulse_alpha_at(t: f64) -> f32 {
    const PERIOD: f64 = 2.0;
    const MIN_ALPHA: f32 = 0.45;
    let phase = ((t / PERIOD).fract() * std::f64::consts::TAU).sin();
    let wave = 0.5 + 0.5 * (phase as f32);
    MIN_ALPHA + (1.0 - MIN_ALPHA) * wave
}

/// Localised footer label like "5 entities". Falls back to English
/// plural rules because we have no `{$n} ->` switches in the FTL files
/// yet — that's a future enhancement once we know which locales need
/// non-trivial plural tables.
fn entity_count_label(count: usize) -> String {
    use fluent::FluentArgs;
    let mut args = FluentArgs::new();
    args.set("n", count as i64);
    let key = match count {
        0 => "entity-count-zero",
        1 => "entity-count-singular",
        _ => "entity-count-plural",
    };
    crate::i18n::t_args(key, &args)
}

// ─── tabs ──────────────────────────────────────────────────────────────

fn inspector_tab(
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

/// Curated scene presets. Open by default on fresh installs so users
/// see the gallery; the flag persists across sessions through
/// [`CollapseState::scene_presets`] once the user toggles it.
pub(super) fn preset_gallery(
    ui: &mut egui::Ui,
    scene: &mut Scene,
    selection: &mut SelectionState,
    config_dirty: &mut bool,
    open: &mut bool,
) {
    let header = egui::RichText::new(format!("{}  {}", icons::SPARKLE, t("scene-presets-header")))
        .text_style(h2());
    let response = egui::CollapsingHeader::new(header)
        .id_salt("anima.scene.presets")
        .default_open(*open)
        .show(ui, |ui| {
            ui.add_space(SPACE_S);
            for id in PresetId::ALL {
                preset_card(ui, *id, scene, selection, config_dirty);
                ui.add_space(SPACE_S);
            }
        });
    let visually_open = response.openness > 0.5;
    if visually_open != *open {
        *open = visually_open;
        *config_dirty = true;
    }
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
                if ui.button(t("scene-preset-append")).clicked() {
                    apply_preset(scene, selection, &preset, ApplyMode::Append);
                    *config_dirty = true;
                }
                let error_color = ui.visuals().error_fg_color;
                if ui
                    .button(egui::RichText::new(t("scene-preset-replace")).color(error_color))
                    .on_hover_text(t("scene-preset-replace-tooltip"))
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

/// Behavior dropdown + variant-specific sliders. Returns `true` when the
/// user touched anything in this section.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn two_monitors() -> Vec<MonitorInfo> {
        vec![
            MonitorInfo {
                name: "eDP-1".into(),
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
                scale_factor: 1.0,
                is_primary: true,
            },
            MonitorInfo {
                name: "HDMI-A-1".into(),
                x: 1920,
                y: 0,
                width: 2560,
                height: 1440,
                scale_factor: 1.5,
                is_primary: false,
            },
        ]
    }

    #[test]
    fn cycle_from_none_picks_first_monitor() {
        let monitors = two_monitors();
        let mut pin = None;
        cycle_entity_monitor(&mut pin, &monitors);
        assert_eq!(pin.as_deref(), Some("eDP-1"));
    }

    #[test]
    fn cycle_walks_in_declaration_order() {
        let monitors = two_monitors();
        let mut pin = Some("eDP-1".to_string());
        cycle_entity_monitor(&mut pin, &monitors);
        assert_eq!(pin.as_deref(), Some("HDMI-A-1"));
    }

    #[test]
    fn cycle_wraps_from_last_to_none() {
        let monitors = two_monitors();
        let mut pin = Some("HDMI-A-1".to_string());
        cycle_entity_monitor(&mut pin, &monitors);
        assert!(pin.is_none(), "expected wrap to None, got {pin:?}");
    }

    #[test]
    fn cycle_on_stale_pin_restarts_from_first() {
        let monitors = two_monitors();
        let mut pin = Some("DP-99".to_string()); // not in monitors
        cycle_entity_monitor(&mut pin, &monitors);
        assert_eq!(pin.as_deref(), Some("eDP-1"));
    }

    #[test]
    fn cycle_with_no_monitors_keeps_pin_unchanged() {
        let empty: Vec<MonitorInfo> = vec![];
        let mut pin = Some("eDP-1".to_string());
        let toast = cycle_entity_monitor(&mut pin, &empty);
        assert_eq!(pin.as_deref(), Some("eDP-1"));
        // Toast should mention the no-monitors state (resolves via i18n
        // fallback if i18n hasn't been initialised in the test runner).
        assert!(!toast.is_empty());
    }

    #[test]
    fn topology_summary_marks_primary() {
        let monitors = two_monitors();
        let summary = monitor_topology_summary(&monitors);
        // Primary monitor gets an asterisk marker; the other one doesn't.
        assert!(summary.contains("eDP-1 *"));
        assert!(!summary.contains("HDMI-A-1 *"));
    }
}
