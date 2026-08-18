//! Mouse drag state machine for moving entities on screen.
//!
//! States: Idle → (mouse down on entity) → Dragging → (mouse up) → Idle

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
    },
}

/// Drag controller
#[derive(Debug, Default)]
pub struct DragController {
    pub state: DragState,
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
        };
        tracing::debug!("Started dragging entity at index {}", entity_index);
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

    /// End the current drag
    pub fn end_drag(&mut self) {
        if matches!(self.state, DragState::Dragging { .. }) {
            tracing::debug!("Ended drag");
        }
        self.state = DragState::Idle;
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
