//! UI panels rendered through egui.
//!
//! Each function takes only the data it actually mutates plus `&egui::Context`.
//! This keeps `App` borrow-safe: the caller passes disjoint `&mut` references
//! to scene / selection / dirty flag instead of `&mut self`.

mod appearance;
mod command_palette;
mod context_menu;
mod inspector;
mod keybindings_tab;
mod library;
mod presets;
mod scene;
mod toasts;
mod toggle_button;

pub use command_palette::{command_palette, PaletteOutcome};
pub(crate) use context_menu::context_menu;
pub use library::LibraryOutcome;
pub use toasts::toasts;
pub use toggle_button::toggle_button;

use crate::asset_library::LibraryIndex;
use crate::i18n::t;
use crate::input::selection::SelectionState;
use crate::keybindings::KeyBindings;
use crate::monitor::{MonitorInfo, MonitorMode};
use crate::scene::Scene;
use crate::ui::banner::Warning;
use crate::ui::collapse::CollapseState;
use crate::ui::icons;
use crate::ui::onboarding::{self, OnboardingProgress};
use crate::ui::theme::{self, h2, Theme, SPACE_S, SPACE_XS};

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
                            inspector::inspector_tab(
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
pub(super) fn entity_monitor_picker(
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
