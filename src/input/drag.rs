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
    pub fn start_drag(&mut self, entity_index: usize, offset_x: f32, offset_y: f32) {
        self.state = DragState::Dragging {
            entity_index,
            offset_x,
            offset_y,
        };
        log::debug!("Started dragging entity at index {}", entity_index);
    }

    /// Update the dragged entity's position based on current mouse position.
    /// Returns Some((new_x, new_y)) if dragging, None otherwise.
    pub fn update(&self, mouse_x: f32, mouse_y: f32) -> Option<(usize, f32, f32)> {
        match &self.state {
            DragState::Dragging {
                entity_index,
                offset_x,
                offset_y,
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
            log::debug!("Ended drag");
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
}
