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

/// True if this monitor set is prone to the XWayland click-through
/// misalignment: any fractional scale, or different scales across
/// monitors. XWayland presents X11 clients a single unscaled coordinate
/// space and scales surfaces behind their back, so an `XShape` input
/// region (our click-through cutout) can land offset from where the
/// compositor actually paints the window. Native X11 honours XShape
/// exactly, and a uniform integer scale is handled cleanly — only
/// fractional or mixed scaling desyncs it.
fn scaling_desyncs_xshape(scales: &[f64]) -> bool {
    let fractional = scales.iter().any(|s| s.fract().abs() > 1e-3);
    let mixed = scales.windows(2).any(|w| (w[0] - w[1]).abs() > 1e-3);
    fractional || mixed
}

/// Warn once at startup if we're on XWayland with a scaling setup that
/// desyncs the `XShape` click-through region. This is an inherent
/// XWayland limitation (see `docs/wayland.md`), not something we can fix
/// from an X11 client — the warning tells the user *why* clicks might
/// land in the wrong place rather than leaving it a mystery.
pub fn warn_xwayland_xshape_scaling(on_xwayland: bool, monitors: &[MonitorInfo]) {
    if !on_xwayland {
        return;
    }
    let scales: Vec<f64> = monitors.iter().map(|m| m.scale_factor).collect();
    if scaling_desyncs_xshape(&scales) {
        tracing::warn!(
            "XWayland + fractional/mixed display scaling detected ({scales:?}); \
             click-through (XShape) may be offset from the painted overlay. \
             This is an XWayland limitation — a native X11 session or a uniform \
             100%/200% scale avoids it. See docs/wayland.md."
        );
    }
}

/// Which windows a monitor mode wants, for a given topology (T.6).
///
/// The *primary* overlay window always exists (it hosts egui); the
/// plan says where it goes and which additional windows to spawn.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowPlan {
    /// Monitor the primary window should cover. `None` keeps the
    /// pre-0.6 behaviour: sized to the primary monitor, positioned
    /// wherever the WM puts it (effectively global origin).
    pub primary: Option<MonitorInfo>,
    /// Monitors that get an extra (non-egui) overlay window each.
    pub extras: Vec<MonitorInfo>,
}

/// The rectangle entities live in and are clamped to, in **global
/// desktop coordinates** (physical pixels).
///
/// A single monitor at the origin yields `(0, 0, w, h)` — exactly what
/// the old `screen_width` / `screen_height` pair meant — so the
/// single-monitor case is unchanged by construction. Several monitors
/// yield the union of their rectangles, whose origin can be **negative**
/// when a monitor sits left of or above the primary. That is precisely
/// why this is a rectangle and not a size: physics and behaviours used
/// to clamp to `[0, primary_window_width]`, so on a multi-monitor
/// desktop any sprite on another monitor was dragged back onto the
/// primary one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DesktopBounds {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl DesktopBounds {
    /// Origin-anchored bounds of the given size: the single-window case,
    /// and the fallback when no topology is known.
    pub fn from_size(width: f32, height: f32) -> Self {
        Self {
            min_x: 0.0,
            min_y: 0.0,
            max_x: width,
            max_y: height,
        }
    }

    /// Union of every listed monitor's rectangle. `None` for an empty
    /// list, so callers pick their own fallback.
    pub fn union_of(monitors: &[MonitorInfo]) -> Option<Self> {
        let mut iter = monitors.iter();
        let first = iter.next()?;
        let mut b = Self {
            min_x: first.x as f32,
            min_y: first.y as f32,
            max_x: (first.x as f32) + first.width as f32,
            max_y: (first.y as f32) + first.height as f32,
        };
        for m in iter {
            b.min_x = b.min_x.min(m.x as f32);
            b.min_y = b.min_y.min(m.y as f32);
            b.max_x = b.max_x.max((m.x as f32) + m.width as f32);
            b.max_y = b.max_y.max((m.y as f32) + m.height as f32);
        }
        Some(b)
    }

    pub fn width(&self) -> f32 {
        self.max_x - self.min_x
    }

    pub fn height(&self) -> f32 {
        self.max_y - self.min_y
    }

    /// Clamp a sprite's left edge so a `sprite_w`-wide sprite stays
    /// inside. Bounds narrower than the sprite pin it to `min_x` rather
    /// than producing an inverted range.
    pub fn clamp_x(&self, x: f32, sprite_w: f32) -> f32 {
        x.clamp(self.min_x, (self.max_x - sprite_w).max(self.min_x))
    }
}

/// The desktop region our overlay windows actually cover — the area an
/// entity may legitimately occupy.
///
/// Derived from the plan rather than from the raw topology, because the
/// two differ: `Span` draws **one** window sized to a single monitor, so
/// widening its bounds to the whole desktop would let sprites walk off
/// the window and vanish. `Single` covers exactly the named monitor, and
/// `PerMonitor` covers the union of primary plus extras.
///
/// `fallback` (the primary window's own size) is used when the plan
/// names nothing — `Span`, or an empty topology — which reproduces the
/// previous behaviour exactly.
pub fn covered_bounds(plan: &WindowPlan, fallback: (f32, f32)) -> DesktopBounds {
    let mut rects: Vec<MonitorInfo> = Vec::with_capacity(1 + plan.extras.len());
    if let Some(p) = &plan.primary {
        rects.push(p.clone());
    }
    rects.extend(plan.extras.iter().cloned());
    DesktopBounds::union_of(&rects)
        .unwrap_or_else(|| DesktopBounds::from_size(fallback.0, fallback.1))
}

/// Pure planning function — unit-testable without a display.
///
/// - `Span` / empty topology → single window, pre-0.6 behaviour.
/// - `Single { name }` → one window on the named monitor (falling
///   back to primary, then first, when the name is stale).
/// - `PerMonitor` → primary window on the primary monitor + one
///   extra per remaining monitor.
pub fn plan_windows(mode: &MonitorMode, monitors: &[MonitorInfo]) -> WindowPlan {
    if monitors.is_empty() {
        return WindowPlan {
            primary: None,
            extras: Vec::new(),
        };
    }
    let primary_monitor = monitors
        .iter()
        .find(|m| m.is_primary)
        .unwrap_or(&monitors[0])
        .clone();
    match mode {
        MonitorMode::Span => WindowPlan {
            primary: None,
            extras: Vec::new(),
        },
        MonitorMode::Single { name } => {
            let target = monitors
                .iter()
                .find(|m| &m.name == name)
                .cloned()
                .unwrap_or(primary_monitor);
            WindowPlan {
                primary: Some(target),
                extras: Vec::new(),
            }
        }
        MonitorMode::PerMonitor => {
            let extras = monitors
                .iter()
                .filter(|m| m.name != primary_monitor.name)
                .cloned()
                .collect();
            WindowPlan {
                primary: Some(primary_monitor),
                extras,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mon(name: &str, x: i32, y: i32, w: u32, h: u32) -> MonitorInfo {
        MonitorInfo {
            name: name.to_string(),
            x,
            y,
            width: w,
            height: h,
            scale_factor: 1.0,
            is_primary: false,
        }
    }

    // ── DesktopBounds ────────────────────────────────────────────────

    #[test]
    fn from_size_is_origin_anchored() {
        let b = DesktopBounds::from_size(1920.0, 1080.0);
        assert_eq!(
            (b.min_x, b.min_y, b.max_x, b.max_y),
            (0.0, 0.0, 1920.0, 1080.0)
        );
        assert_eq!((b.width(), b.height()), (1920.0, 1080.0));
    }

    #[test]
    fn union_of_empty_is_none() {
        assert_eq!(DesktopBounds::union_of(&[]), None);
    }

    #[test]
    fn union_spans_monitors_to_the_right() {
        let b = DesktopBounds::union_of(&[
            mon("primary", 0, 0, 1920, 1080),
            mon("right", 1920, 0, 2560, 1440),
        ])
        .unwrap();
        assert_eq!((b.min_x, b.max_x), (0.0, 4480.0));
        assert_eq!((b.min_y, b.max_y), (0.0, 1440.0));
    }

    /// The case a width/height pair fundamentally cannot express: a
    /// monitor placed left of and above the primary puts the desktop
    /// origin at negative coordinates.
    #[test]
    fn union_handles_negative_origins() {
        let b = DesktopBounds::union_of(&[
            mon("primary", 0, 0, 1920, 1080),
            mon("left", -1920, -200, 1920, 1080),
        ])
        .unwrap();
        assert_eq!((b.min_x, b.min_y), (-1920.0, -200.0));
        assert_eq!((b.max_x, b.max_y), (1920.0, 1080.0));
        assert_eq!(b.width(), 3840.0);
    }

    #[test]
    fn clamp_x_keeps_a_sprite_inside_including_negative_origin() {
        let b = DesktopBounds::union_of(&[mon("left", -1920, 0, 1920, 1080)]).unwrap();
        // Well inside → untouched.
        assert_eq!(b.clamp_x(-1000.0, 64.0), -1000.0);
        // Past the left edge → pinned to min_x, *not* to zero. Clamping
        // to 0 is exactly the old bug: it teleported the sprite onto the
        // primary monitor.
        assert_eq!(b.clamp_x(-5000.0, 64.0), -1920.0);
        // Past the right edge → last position that still fits.
        assert_eq!(b.clamp_x(500.0, 64.0), -64.0);
    }

    #[test]
    fn clamp_x_pins_when_bounds_are_narrower_than_the_sprite() {
        let b = DesktopBounds::from_size(50.0, 50.0);
        // max_x - sprite_w would be negative; must not invert the range.
        assert_eq!(b.clamp_x(30.0, 200.0), 0.0);
    }

    // ── covered_bounds ───────────────────────────────────────────────

    /// Span draws one window sized to a single monitor, so its bounds
    /// stay the primary window's own size — widening them to the whole
    /// desktop would let sprites walk off the window and vanish.
    #[test]
    fn covered_bounds_span_uses_the_window_size() {
        let plan = plan_windows(&MonitorMode::Span, &[mon("a", 0, 0, 1920, 1080)]);
        let b = covered_bounds(&plan, (1280.0, 720.0));
        assert_eq!(b, DesktopBounds::from_size(1280.0, 720.0));
    }

    #[test]
    fn covered_bounds_single_uses_that_monitor_only() {
        let monitors = vec![
            mon("primary", 0, 0, 1920, 1080),
            mon("right", 1920, 0, 2560, 1440),
        ];
        let plan = plan_windows(
            &MonitorMode::Single {
                name: "right".into(),
            },
            &monitors,
        );
        let b = covered_bounds(&plan, (1920.0, 1080.0));
        assert_eq!((b.min_x, b.max_x), (1920.0, 4480.0));
    }

    /// The regression this refactor exists for: with per-monitor
    /// overlays the covered region is the union, so an entity living on
    /// a secondary monitor is no longer clamped back onto the primary.
    #[test]
    fn covered_bounds_per_monitor_spans_every_overlay() {
        let monitors = vec![
            mon("primary", 0, 0, 1920, 1080),
            mon("right", 1920, 0, 1920, 1080),
        ];
        let mut monitors = monitors;
        monitors[0].is_primary = true;
        let plan = plan_windows(&MonitorMode::PerMonitor, &monitors);
        let b = covered_bounds(&plan, (1920.0, 1080.0));
        assert_eq!((b.min_x, b.max_x), (0.0, 3840.0));

        // A sprite at x=3000 sits on the second monitor. Under the old
        // primary-window bounds it was clamped to 1920-64; now it stays.
        assert_eq!(b.clamp_x(3000.0, 64.0), 3000.0);
        assert_eq!(
            DesktopBounds::from_size(1920.0, 1080.0).clamp_x(3000.0, 64.0),
            1856.0
        );
    }

    #[test]
    fn uniform_integer_scaling_is_safe() {
        assert!(!scaling_desyncs_xshape(&[1.0]));
        assert!(!scaling_desyncs_xshape(&[2.0, 2.0]));
        assert!(!scaling_desyncs_xshape(&[1.0, 1.0, 1.0]));
    }

    #[test]
    fn fractional_scaling_desyncs() {
        assert!(scaling_desyncs_xshape(&[1.5]));
        assert!(scaling_desyncs_xshape(&[1.25, 1.25]));
    }

    #[test]
    fn mixed_scaling_desyncs() {
        assert!(scaling_desyncs_xshape(&[1.0, 2.0]));
        assert!(scaling_desyncs_xshape(&[2.0, 1.0, 2.0]));
    }

    #[test]
    fn empty_or_single_uniform_does_not_desync() {
        assert!(!scaling_desyncs_xshape(&[]));
    }

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

    // ── plan_windows (T.6) ───────────────────────────────────────────

    #[test]
    fn plan_span_is_single_window() {
        let plan = plan_windows(&MonitorMode::Span, &left_right_setup());
        assert_eq!(plan.primary, None);
        assert!(plan.extras.is_empty());
    }

    #[test]
    fn plan_per_monitor_spawns_extras_for_non_primary() {
        let monitors = left_right_setup();
        let plan = plan_windows(&MonitorMode::PerMonitor, &monitors);
        assert_eq!(
            plan.primary.as_ref().map(|m| m.name.as_str()),
            Some("eDP-1")
        );
        assert_eq!(plan.extras.len(), 1);
        assert_eq!(plan.extras[0].name, "HDMI-A-1");
    }

    #[test]
    fn plan_per_monitor_single_monitor_has_no_extras() {
        let plan = plan_windows(&MonitorMode::PerMonitor, &primary_only());
        assert_eq!(
            plan.primary.as_ref().map(|m| m.name.as_str()),
            Some("eDP-1")
        );
        assert!(plan.extras.is_empty());
    }

    #[test]
    fn plan_single_targets_named_monitor() {
        let monitors = left_right_setup();
        let plan = plan_windows(
            &MonitorMode::Single {
                name: "HDMI-A-1".into(),
            },
            &monitors,
        );
        assert_eq!(
            plan.primary.as_ref().map(|m| m.name.as_str()),
            Some("HDMI-A-1")
        );
        assert!(plan.extras.is_empty());
    }

    #[test]
    fn plan_single_stale_name_falls_back_to_primary() {
        let plan = plan_windows(
            &MonitorMode::Single {
                name: "DP-9".into(),
            },
            &left_right_setup(),
        );
        assert_eq!(
            plan.primary.as_ref().map(|m| m.name.as_str()),
            Some("eDP-1")
        );
    }

    #[test]
    fn plan_empty_topology_degrades_to_single_window() {
        let plan = plan_windows(&MonitorMode::PerMonitor, &[]);
        assert_eq!(plan.primary, None);
        assert!(plan.extras.is_empty());
    }
}
