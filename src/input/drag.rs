//! Mouse drag state machine for moving entities on screen.
//!
//! States: Idle → (mouse down on entity) → Dragging → (mouse up) → Idle

use std::time::{Duration, Instant};

/// Current drag state
#[derive(Debug, Default)]
pub enum DragState {
    /// No drag in progress
    #[default]
    Idle,
    /// Dragging entity at index, with mouse offset from entity origin
    Dragging {
        entity_index: usize,
        offset_x: f32,
        offset_y: f32,
        /// Cursor position when the press began — lets `was_tap`
        /// distinguish a tap (→ poke) from an actual move.
        press_x: f32,
        press_y: f32,
        /// Whether a tap — no move — narrows the selection to this entity:
        /// set when the press landed on one of several already selected
        /// (`crate::input::multi`).
        narrow_on_tap: bool,
    },
}

/// How far back the pointer's speed at release is measured.
const FLING_WINDOW: Duration = Duration::from_millis(100);

/// A pointer still for this long before the release threw nothing: it
/// was put down, not thrown.
const FLING_STILL: Duration = Duration::from_millis(50);

/// Slower than this (px/s) at release is a put-down, not a throw.
pub const FLING_MIN_SPEED: f32 = 300.0;

/// A throw is never faster than this (px/s).
pub const FLING_MAX_SPEED: f32 = 2000.0;

/// Drag controller
#[derive(Debug, Default)]
pub struct DragController {
    pub state: DragState,
    /// What the drag has snapped to, for the renderer to show
    /// (`crate::input::arrange`). Empty when it has not, and between drags.
    guides: Vec<crate::input::arrange::Guide>,
    /// Where the pointer was lately, and when — the last
    /// [`FLING_WINDOW`] of the drag, for [`DragController::fling`].
    trail: std::collections::VecDeque<(Instant, f32, f32)>,
}

impl DragController {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start dragging an entity.
    /// `offset_x/y` is the distance from mouse position to entity origin.
    pub fn start_drag(
        &mut self,
        entity_index: usize,
        offset_x: f32,
        offset_y: f32,
        press_x: f32,
        press_y: f32,
    ) {
        self.state = DragState::Dragging {
            entity_index,
            offset_x,
            offset_y,
            press_x,
            press_y,
            narrow_on_tap: false,
        };
        self.guides.clear();
        self.trail.clear();
        self.trail.push_back((Instant::now(), press_x, press_y));
        tracing::debug!("Started dragging entity at index {}", entity_index);
    }

    /// The pointer is at `(x, y)` at `now`, mid-drag.
    pub fn track(&mut self, (x, y): (f32, f32), now: Instant) {
        if !self.is_dragging() {
            return;
        }
        self.trail.push_back((now, x, y));
        while self
            .trail
            .front()
            .is_some_and(|&(t, ..)| now.saturating_duration_since(t) > FLING_WINDOW)
            && self.trail.len() > 2
        {
            self.trail.pop_front();
        }
    }

    /// How fast the pointer was going when let go at `now`, in px/s, if
    /// that was a throw: still moving then, and at least
    /// [`FLING_MIN_SPEED`] — capped at [`FLING_MAX_SPEED`].
    pub fn fling(&self, now: Instant) -> Option<(f32, f32)> {
        let &(last_t, last_x, last_y) = self.trail.back()?;
        if now.saturating_duration_since(last_t) > FLING_STILL {
            return None;
        }
        let &(first_t, first_x, first_y) = self
            .trail
            .iter()
            .find(|(t, ..)| last_t.saturating_duration_since(*t) <= FLING_WINDOW)?;
        let dt = last_t.saturating_duration_since(first_t).as_secs_f32();
        if dt < 0.01 {
            return None;
        }
        let (vx, vy) = ((last_x - first_x) / dt, (last_y - first_y) / dt);
        let speed = vx.hypot(vy);
        if !speed.is_finite() || speed < FLING_MIN_SPEED {
            return None;
        }
        let scale = (FLING_MAX_SPEED / speed).min(1.0);
        Some((vx * scale, vy * scale))
    }

    /// Update the dragged entity's position based on current mouse position.
    /// Returns Some((new_x, new_y)) if dragging, None otherwise.
    pub fn update(&self, mouse_x: f32, mouse_y: f32) -> Option<(usize, f32, f32)> {
        match &self.state {
            DragState::Dragging {
                entity_index,
                offset_x,
                offset_y,
                ..
            } => {
                let new_x = mouse_x - offset_x;
                let new_y = mouse_y - offset_y;
                Some((*entity_index, new_x, new_y))
            }
            DragState::Idle => None,
        }
    }

    /// Let a tap on this drag narrow the selection to its entity.
    pub fn narrow_on_tap(&mut self) {
        if let DragState::Dragging { narrow_on_tap, .. } = &mut self.state {
            *narrow_on_tap = true;
        }
    }

    /// Whether a tap narrows the selection ([`Self::narrow_on_tap`]).
    pub fn narrows_on_tap(&self) -> bool {
        matches!(
            self.state,
            DragState::Dragging {
                narrow_on_tap: true,
                ..
            }
        )
    }

    /// End the current drag
    pub fn end_drag(&mut self) {
        if matches!(self.state, DragState::Dragging { .. }) {
            tracing::debug!("Ended drag");
        }
        self.state = DragState::Idle;
        self.guides.clear();
        self.trail.clear();
    }

    /// The guides of the last move ([`Self::guides`]).
    pub fn set_guides(&mut self, guides: Vec<crate::input::arrange::Guide>) {
        self.guides = guides;
    }

    /// What the drag snapped to at its last move.
    pub fn guides(&self) -> &[crate::input::arrange::Guide] {
        &self.guides
    }

    /// Is a drag currently in progress?
    pub fn is_dragging(&self) -> bool {
        matches!(self.state, DragState::Dragging { .. })
    }

    /// Get the index of the entity currently being dragged, if any.
    pub fn dragging_entity(&self) -> Option<usize> {
        match &self.state {
            DragState::Dragging { entity_index, .. } => Some(*entity_index),
            DragState::Idle => None,
        }
    }

    /// Did the in-progress drag stay within `radius` of where it began —
    /// i.e. a tap (→ poke) rather than a move? False when idle.
    pub fn was_tap(&self, x: f32, y: f32, radius: f32) -> bool {
        match &self.state {
            DragState::Dragging {
                press_x, press_y, ..
            } => {
                let (dx, dy) = (x - press_x, y - press_y);
                dx * dx + dy * dy <= radius * radius
            }
            DragState::Idle => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A drag from (0, 0) pressed at `t0`, then the pointer at each
    /// `(ms after t0, x, y)`.
    fn dragged(t0: Instant, moves: &[(u64, f32, f32)]) -> DragController {
        let mut d = DragController::new();
        d.start_drag(0, 0.0, 0.0, 0.0, 0.0);
        // The press's own sample is "now"; put it at t0.
        d.trail.clear();
        d.trail.push_back((t0, 0.0, 0.0));
        for &(ms, x, y) in moves {
            d.track((x, y), t0 + Duration::from_millis(ms));
        }
        d
    }

    #[test]
    fn a_fast_release_throws_at_the_pointer_speed() {
        let t0 = Instant::now();
        let d = dragged(t0, &[(20, 20.0, 0.0), (40, 40.0, -10.0), (60, 60.0, -20.0)]);
        let (vx, vy) = d.fling(t0 + Duration::from_millis(65)).expect("thrown");
        assert!((vx - 1000.0).abs() < 1.0, "vx={vx}");
        assert!((vy + 333.3).abs() < 1.0, "vy={vy}");
    }

    #[test]
    fn a_pointer_stopped_before_release_puts_down() {
        let t0 = Instant::now();
        let d = dragged(t0, &[(20, 40.0, 0.0), (40, 80.0, 0.0)]);
        assert!(d.fling(t0 + Duration::from_millis(200)).is_none());
    }

    #[test]
    fn a_slow_drag_puts_down() {
        let t0 = Instant::now();
        // 100 px/s.
        let d = dragged(t0, &[(50, 5.0, 0.0), (100, 10.0, 0.0)]);
        assert!(d.fling(t0 + Duration::from_millis(100)).is_none());
    }

    #[test]
    fn only_the_last_moment_counts_and_speed_is_capped() {
        let t0 = Instant::now();
        // Slow for a long while, then a flick: the flick decides.
        let mut moves: Vec<(u64, f32, f32)> = (1..=20).map(|i| (i * 20, i as f32, 0.0)).collect();
        moves.push((420, 400.0, 0.0));
        let d = dragged(t0, &moves);
        let (vx, _) = d.fling(t0 + Duration::from_millis(420)).expect("thrown");
        assert!((vx - FLING_MAX_SPEED).abs() < 0.01, "vx={vx}");
        assert!(d.trail.len() <= 8, "old samples go: {}", d.trail.len());
    }

    #[test]
    fn idle_controller_reports_no_drag() {
        let c = DragController::new();
        assert!(!c.is_dragging());
        assert_eq!(c.dragging_entity(), None);
        assert_eq!(c.update(10.0, 10.0), None);
    }

    #[test]
    fn start_drag_sets_state() {
        let mut c = DragController::new();
        c.start_drag(3, 10.0, 20.0, 100.0, 100.0);
        assert!(c.is_dragging());
        assert_eq!(c.dragging_entity(), Some(3));
    }

    #[test]
    fn was_tap_is_true_only_within_radius() {
        let mut c = DragController::new();
        assert!(!c.was_tap(0.0, 0.0, 6.0), "idle is never a tap");
        c.start_drag(0, 0.0, 0.0, 100.0, 100.0);
        // Released ~where it was pressed → a tap.
        assert!(c.was_tap(103.0, 98.0, 6.0));
        // Released well away → a drag, not a tap.
        assert!(!c.was_tap(150.0, 100.0, 6.0));
    }

    #[test]
    fn update_subtracts_grab_offset_from_mouse() {
        // Grabbed entity index 1 at offset (10, 20) — i.e. the cursor sat
        // 10px right and 20px below the entity origin when the drag began.
        // Moving the cursor must keep that grab point under it: the origin
        // is always mouse - offset, never snapping to the cursor.
        let mut c = DragController::new();
        c.start_drag(1, 10.0, 20.0, 200.0, 200.0);
        assert_eq!(c.update(200.0, 200.0), Some((1, 190.0, 180.0)));
        assert_eq!(c.update(10.0, 20.0), Some((1, 0.0, 0.0)));
    }

    #[test]
    fn end_drag_returns_to_idle() {
        let mut c = DragController::new();
        c.start_drag(0, 1.0, 1.0, 0.0, 0.0);
        c.end_drag();
        assert!(!c.is_dragging());
        assert_eq!(c.dragging_entity(), None);
        assert_eq!(c.update(5.0, 5.0), None);
    }

    #[test]
    fn end_drag_when_idle_is_a_noop() {
        let mut c = DragController::new();
        c.end_drag(); // must not panic or change anything
        assert!(!c.is_dragging());
    }
}
