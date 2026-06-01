/// Entity selection — tracks which entity is currently selected via mouse click.

#[derive(Debug, Default)]
pub struct SelectionState {
    /// Index of the currently selected entity, if any
    pub selected_entity: Option<usize>,
}

impl SelectionState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Select an entity by index
    pub fn select(&mut self, index: usize) {
        self.selected_entity = Some(index);
        tracing::debug!("Selected entity at index {}", index);
    }

    /// Deselect all
    pub fn deselect(&mut self) {
        if self.selected_entity.is_some() {
            tracing::debug!("Deselected entity");
        }
        self.selected_entity = None;
    }

    /// Get the index of the currently selected entity
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_entity
    }

    /// Check if a specific entity is selected
    pub fn is_selected(&self, index: usize) -> bool {
        self.selected_entity == Some(index)
    }
}
