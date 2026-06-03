//! Monitor enumeration and routing — the data layer beneath
//! `docs/engine-features.md` §1.
//!
//! This module is renderer-agnostic: it converts whatever winit hands
//! us at startup into a stable, serializable shape (`MonitorInfo`) and
//! provides pure functions that resolve "which monitor should this
//! entity render on?" without touching window or GPU state.
//!
//! Sub-phase C.1 (this commit) ships:
//! - the data types
//! - the centroid-based resolver
//! - logging at app startup
//!
//! C.2 wires the resolver into the rendering pipeline; C.3 spawns
//! one overlay window per monitor when `MonitorMode::PerMonitor` is
//! active. Keeping the data layer separate makes both straightforward.

use serde::{Deserialize, Serialize};

/// How animaEngine distributes its overlay across the user's
/// monitors. Persisted in `GlobalConfig.monitor_mode`.
///
/// The 0.2 behaviour was implicitly `Span` — one window stretched
/// across whatever winit reported as the primary monitor's size. The
/// 0.3 default flips to `PerMonitor`, which more closely matches what
/// users expect ("a character on each screen, please"). Existing
/// configs round-trip through the serde default so the change is
/// opt-in via upgrade rather than forced.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "kind")]
pub enum MonitorMode {
    /// One overlay surface per detected monitor. The default for
    /// fresh installs in 0.3.
    #[default]
    PerMonitor,
    /// One overlay surface stretching across whatever the primary
    /// monitor reports. 0.2 behaviour, kept for compatibility and
    /// kiosk-style setups where all characters belong on one
    /// surface.
    Span,
    /// A single named monitor carries the overlay; the rest stay
    /// untouched. Useful when streaming from one screen and not
    /// wanting characters on the broadcasted display.
    Single { name: String },
}

impl MonitorMode {
    /// Display label for UI pickers — kept ASCII so the same label
    /// works regardless of font availability.
    pub fn label(&self) -> String {
        match self {
            Self::PerMonitor => "Per monitor".to_string(),
            Self::Span => "Span all monitors".to_string(),
            Self::Single { name } => format!("Single ({name})"),
        }
    }
}

/// Stable, serializable snapshot of one monitor at app startup.
///
/// We deliberately don't store winit's `MonitorHandle` here — it
/// borrows the event loop, and we want to pass `MonitorInfo`
/// around freely (scene resolver, UI, persistence).
// `f64` precludes `Eq`; partial equality is enough for our use cases.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitorInfo {
    /// Identifier reported by winit's `MonitorHandle::name()`. On
    /// X11 this is the RandR output (`eDP-1`, `HDMI-A-1`); on
    /// Wayland it's the `wl_output` name. Falls back to `Display N`
    /// when winit returns `None`, so the field is always populated.
    pub name: String,
    /// Top-left corner of the monitor in the global desktop
    /// coordinate system, in physical pixels.
    pub x: i32,
    pub y: i32,
    /// Width and height in physical pixels (pre-scale).
    pub width: u32,
    pub height: u32,
    /// HiDPI scale factor reported by winit. We don't apply it here
    /// — the renderer does — but we snapshot it so the picker UI
    /// can display "2560×1440 @ 1.5×" without re-querying winit.
    pub scale_factor: f64,
    /// `true` for the monitor winit reports as primary. At most one
    /// `MonitorInfo` in the snapshot can have this set.
    pub is_primary: bool,
}

impl MonitorInfo {
    /// `true` when `(global_x, global_y)` falls inside this monitor's
    /// rectangle. Half-open at the right/bottom edges so a pixel on
    /// the boundary belongs to exactly one monitor.
    pub fn contains(&self, global_x: f32, global_y: f32) -> bool {
        let x = global_x as i32;
        let y = global_y as i32;
        x >= self.x
            && x < self.x + self.width as i32
            && y >= self.y
            && y < self.y + self.height as i32
    }

    /// Synthesise the entry used when winit returns no monitors at
    /// all (smoke tests on a headless Xvfb, broken display setup).
    pub fn fallback_single() -> Self {
        Self {
            name: "Display 0".to_string(),
            x: 0,
            y: 0,
            width: 1280,
            height: 720,
            scale_factor: 1.0,
            is_primary: true,
        }
    }
}

/// Resolve which monitor an entity should render on, given its
/// stored position and an explicit override.
///
/// Lookup order:
/// 1. If `requested` is `Some(name)` and that monitor exists in
///    `monitors`, use it.
/// 2. Otherwise pick whichever monitor contains the centroid.
/// 3. If no monitor contains the centroid (entity dragged off-screen,
///    or no monitors detected), fall back to the primary, then to
///    the first available.
///
/// Returns `None` only when `monitors` is empty — callers should
/// substitute `MonitorInfo::fallback_single()` in that case.
pub fn resolve_monitor_for_position<'a>(
    monitors: &'a [MonitorInfo],
    centroid_x: f32,
    centroid_y: f32,
    requested: Option<&str>,
) -> Option<&'a MonitorInfo> {
    if monitors.is_empty() {
        return None;
    }

    // 1) Explicit pin wins, when it points at something real.
    if let Some(name) = requested {
        if let Some(m) = monitors.iter().find(|m| m.name == name) {
            return Some(m);
        }
        // Pinned to a monitor that's no longer connected — log and
        // fall through. The caller observes this by getting a
        // different monitor back than they asked for; they should
        // optionally update the config to clear the stale name.
        tracing::warn!(
            "Entity pinned to monitor {:?} but it's no longer present; falling back to centroid resolution",
            name,
        );
    }

    // 2) Centroid hit-test.
    if let Some(m) = monitors.iter().find(|m| m.contains(centroid_x, centroid_y)) {
        return Some(m);
    }

    // 3) Primary / first fallback.
    monitors
        .iter()
        .find(|m| m.is_primary)
        .or_else(|| monitors.first())
}

/// Log the detected monitor topology at startup. Always runs once,
/// even if multi-monitor support is disabled at the config level —
/// the info is useful for bug reports either way.
pub fn log_topology(monitors: &[MonitorInfo]) {
    if monitors.is_empty() {
        tracing::warn!(
            "No monitors reported by winit; using fallback 1280x720 surface for the overlay",
        );
        return;
    }
    tracing::info!("Detected {} monitor(s):", monitors.len(),);
    for m in monitors {
        let marker = if m.is_primary { " (primary)" } else { "" };
        tracing::info!(
            "  {}{}: {}x{} @ ({}, {}) scale={:.2}",
            m.name,
            marker,
            m.width,
            m.height,
            m.x,
            m.y,
            m.scale_factor,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn primary_only() -> Vec<MonitorInfo> {
        vec![MonitorInfo {
            name: "eDP-1".into(),
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            scale_factor: 1.0,
            is_primary: true,
        }]
    }

    fn left_right_setup() -> Vec<MonitorInfo> {
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
    fn default_mode_is_per_monitor() {
        assert_eq!(MonitorMode::default(), MonitorMode::PerMonitor);
    }

    #[test]
    fn contains_is_half_open_on_right_edge() {
        let m = &primary_only()[0];
        assert!(m.contains(0.0, 0.0));
        assert!(m.contains(1919.5, 1079.5));
        // Exactly on the right / bottom edge belongs to the *next*
        // monitor in a tiled setup.
        assert!(!m.contains(1920.0, 0.0));
        assert!(!m.contains(0.0, 1080.0));
    }

    #[test]
    fn centroid_resolution_picks_correct_monitor() {
        let setup = left_right_setup();
        let left = resolve_monitor_for_position(&setup, 100.0, 100.0, None).unwrap();
        assert_eq!(left.name, "eDP-1");
        let right = resolve_monitor_for_position(&setup, 3000.0, 500.0, None).unwrap();
        assert_eq!(right.name, "HDMI-A-1");
    }

    #[test]
    fn explicit_pin_overrides_centroid() {
        let setup = left_right_setup();
        // Position falls on the left monitor, but we ask for the right.
        let m = resolve_monitor_for_position(&setup, 100.0, 100.0, Some("HDMI-A-1")).unwrap();
        assert_eq!(m.name, "HDMI-A-1");
    }

    #[test]
    fn stale_pin_falls_back_to_centroid() {
        let setup = left_right_setup();
        let m = resolve_monitor_for_position(&setup, 3000.0, 500.0, Some("missing")).unwrap();
        // We asked for a non-existent monitor; centroid puts us on the right.
        assert_eq!(m.name, "HDMI-A-1");
    }

    #[test]
    fn off_screen_centroid_falls_back_to_primary() {
        let setup = left_right_setup();
        // Position is far below both monitors.
        let m = resolve_monitor_for_position(&setup, 100.0, 5000.0, None).unwrap();
        assert!(m.is_primary, "expected primary fallback, got {:?}", m.name);
    }

    #[test]
    fn no_monitors_returns_none() {
        let empty: Vec<MonitorInfo> = vec![];
        assert!(resolve_monitor_for_position(&empty, 0.0, 0.0, None).is_none());
    }

    #[test]
    fn no_primary_falls_back_to_first() {
        // Pathological setup: two monitors, neither marked primary.
        let setup = vec![
            MonitorInfo {
                name: "A".into(),
                x: 0,
                y: 0,
                width: 100,
                height: 100,
                scale_factor: 1.0,
                is_primary: false,
            },
            MonitorInfo {
                name: "B".into(),
                x: 100,
                y: 0,
                width: 100,
                height: 100,
                scale_factor: 1.0,
                is_primary: false,
            },
        ];
        // Centroid off-screen.
        let m = resolve_monitor_for_position(&setup, -100.0, -100.0, None).unwrap();
        assert_eq!(m.name, "A");
    }

    #[test]
    fn mode_round_trips_through_toml() {
        for mode in [
            MonitorMode::PerMonitor,
            MonitorMode::Span,
            MonitorMode::Single {
                name: "DP-2".into(),
            },
        ] {
            #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
            struct Wrap {
                m: MonitorMode,
            }
            let toml_str = toml::to_string(&Wrap { m: mode.clone() }).unwrap();
            let back: Wrap = toml::from_str(&toml_str).unwrap();
            assert_eq!(back.m, mode);
        }
    }

    #[test]
    fn mode_labels_are_non_empty() {
        for mode in [
            MonitorMode::PerMonitor,
            MonitorMode::Span,
            MonitorMode::Single { name: "X".into() },
        ] {
            assert!(!mode.label().is_empty());
        }
    }

    #[test]
    fn fallback_single_is_marked_primary() {
        let f = MonitorInfo::fallback_single();
        assert!(f.is_primary);
        assert!(f.width > 0 && f.height > 0);
    }
}
