//! Which characters are selected.
//!
//! One of them is the *primary*: the one the Inspector shows, the last
//! one picked. Since 1.5 others can be selected with it — Shift+click, or
//! a rectangle dragged over empty space — and shortcuts, a drag, Delete
//! and the right-click menu act on all of them. Code that only ever dealt
//! with one character keeps using [`SelectionState::selected_index`],
//! which is the primary.
//!
//! Selections are entity indices, so anything that removes entities
//! either clears the selection or tells it ([`SelectionState::removed`]).

#[derive(Debug, Default)]
pub struct SelectionState {
    /// Index of the primary selected entity, if any.
    pub selected_entity: Option<usize>,
    /// The others selected along with it, in the order they were added.
    /// Never contains the primary; empty without one.
    others: Vec<usize>,
}

impl SelectionState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Select this entity alone.
    pub fn select(&mut self, index: usize) {
        self.selected_entity = Some(index);
        self.others.clear();
        tracing::debug!("Selected entity at index {}", index);
    }

    /// Select these entities, the first as the primary. An empty list
    /// deselects.
    pub fn select_all_of(&mut self, indices: &[usize]) {
        self.deselect();
        let mut seen = Vec::with_capacity(indices.len());
        for &i in indices {
            if !seen.contains(&i) {
                seen.push(i);
            }
        }
        let mut it = seen.into_iter();
        self.selected_entity = it.next();
        self.others = it.collect();
    }

    /// Shift+click: take `index` out of the selection if it is in it,
    /// otherwise add it as the new primary.
    pub fn toggle(&mut self, index: usize) {
        if self.selected_entity == Some(index) {
            self.selected_entity = self.others.pop();
        } else if let Some(pos) = self.others.iter().position(|&i| i == index) {
            self.others.remove(pos);
        } else {
            if let Some(previous) = self.selected_entity.replace(index) {
                self.others.push(previous);
            }
        }
    }

    /// Make `index`, already selected, the primary — a press on one of
    /// several keeps them all and leads with it. Selects it alone if it
    /// was not selected.
    pub fn make_primary(&mut self, index: usize) {
        if self.selected_entity == Some(index) {
            return;
        }
        match self.others.iter().position(|&i| i == index) {
            Some(pos) => {
                self.others.remove(pos);
                if let Some(previous) = self.selected_entity.replace(index) {
                    self.others.push(previous);
                }
            }
            None => self.select(index),
        }
    }

    /// Deselect all
    pub fn deselect(&mut self) {
        if self.selected_entity.is_some() {
            tracing::debug!("Deselected entity");
        }
        self.selected_entity = None;
        self.others.clear();
    }

    /// Get the index of the primary selected entity
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_entity
    }

    /// Every selected entity, the primary first.
    pub fn selected_indices(&self) -> Vec<usize> {
        self.selected_entity
            .into_iter()
            .chain(self.others.iter().copied())
            .collect()
    }

    /// The ids of every selected entity, the primary first — owned, so the
    /// scene can be borrowed again while they are in use.
    pub fn selected_ids(&self, scene: &crate::scene::Scene) -> Vec<String> {
        self.selected_indices()
            .into_iter()
            .filter_map(|i| scene.entities.get(i))
            .map(|e| e.id.clone())
            .collect()
    }

    /// How many entities are selected.
    pub fn count(&self) -> usize {
        usize::from(self.selected_entity.is_some()) + self.others.len()
    }

    /// Check if a specific entity is selected
    pub fn is_selected(&self, index: usize) -> bool {
        self.selected_entity == Some(index) || self.others.contains(&index)
    }

    /// The entity at `index` was removed from the scene: forget it, and
    /// shift the indices after it down by one.
    pub fn removed(&mut self, index: usize) {
        let shift = |i: usize| if i > index { i - 1 } else { i };
        if self.selected_entity == Some(index) {
            self.selected_entity = self.others.pop();
        }
        self.others.retain(|&i| i != index);
        self.selected_entity = self.selected_entity.map(shift);
        for i in &mut self.others {
            *i = shift(*i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_select_replaces_the_whole_selection() {
        let mut s = SelectionState::default();
        s.select_all_of(&[1, 2, 3]);
        s.select(5);
        assert_eq!(s.selected_indices(), [5]);
        assert_eq!(s.count(), 1);
    }

    #[test]
    fn toggling_adds_as_primary_and_removes() {
        let mut s = SelectionState::default();
        s.select(1);
        s.toggle(4);
        assert_eq!(s.selected_index(), Some(4), "the one just picked leads");
        assert_eq!(s.selected_indices(), [4, 1]);
        s.toggle(1);
        assert_eq!(s.selected_indices(), [4]);
        s.toggle(4);
        assert_eq!(s.selected_index(), None);
        assert_eq!(s.count(), 0);
        s.toggle(7);
        assert_eq!(s.selected_indices(), [7], "toggling from nothing selects");
    }

    #[test]
    fn removing_the_primary_promotes_another() {
        let mut s = SelectionState::default();
        s.select_all_of(&[2, 5, 8]);
        s.toggle(2);
        assert_eq!(s.selected_index(), Some(8));
        assert!(s.is_selected(5) && !s.is_selected(2));
    }

    #[test]
    fn a_removed_entity_leaves_and_later_indices_shift() {
        let mut s = SelectionState::default();
        s.select_all_of(&[1, 3, 6]);
        s.removed(3);
        assert_eq!(s.selected_indices(), [1, 5]);
        s.removed(1);
        assert_eq!(
            s.selected_indices(),
            [4],
            "the primary went; the next leads"
        );
        s.removed(0);
        assert_eq!(s.selected_indices(), [3]);
    }

    #[test]
    fn making_a_selected_one_primary_keeps_the_rest() {
        let mut s = SelectionState::default();
        s.select_all_of(&[1, 2, 3]);
        s.make_primary(3);
        assert_eq!(s.selected_index(), Some(3));
        assert_eq!(s.count(), 3);
        s.make_primary(9);
        assert_eq!(s.selected_indices(), [9], "not selected: selected alone");
    }

    #[test]
    fn duplicates_in_a_list_select_once() {
        let mut s = SelectionState::default();
        s.select_all_of(&[2, 2, 3, 2]);
        assert_eq!(s.selected_indices(), [2, 3]);
        s.select_all_of(&[]);
        assert_eq!(s.count(), 0);
    }
}
