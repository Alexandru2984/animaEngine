//! Multi-monitor pickers + topology helpers + selection-pulse curve.
//! Extracted in I.11 — last piece of Phase I.
//!
//! Two pickers live here:
//! - **Global `MonitorMode`** picker for the Scene tab (PerMonitor /
//!   Span / Single { name }).
//! - **Per-entity monitor pin** picker for the Inspector's Position
//!   section.
//!
//! Plus the keyboard-driven `cycle_entity_monitor` (Ctrl+M), the
//! design-system "selection pulse" curve, and a couple of label
//! helpers (`entity_count_label`, `monitor_topology_summary`).

use crate::i18n::t;
use crate::monitor::{MonitorInfo, MonitorMode};
use crate::ui::theme::{self, h2, SPACE_S};

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
pub(super) fn entity_count_label(count: usize) -> String {
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
