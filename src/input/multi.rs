//! Editing several characters at once with the pointer (1.5): Shift+click,
//! a selection rectangle dragged over empty space, and dragging the whole
//! selection. Shared by both backends, which feed it global desktop
//! coordinates, so the two cannot come to behave differently.
//!
//! The conventions are the usual ones. A press on a character selects it
//! alone, unless it is one of several already selected: then the group
//! stays selected, so it can be dragged, and a tap without moving selects
//! it alone. Shift+click adds or removes a character. A drag over empty
//! space selects what the rectangle touches — added to the selection with
//! Shift — and a click there deselects.

use crate::input::drag::DragController;
use crate::input::selection::SelectionState;
use crate::scene::Scene;

/// A left press on the character at `idx`, at (`x`, `y`). Returns whether
/// a drag started — not when Shift took the character out.
pub fn press_on(
    scene: &mut Scene,
    selection: &mut SelectionState,
    drag: &mut DragController,
    idx: usize,
    (x, y): (f32, f32),
    shift: bool,
) -> bool {
    if shift {
        selection.toggle(idx);
        if !selection.is_selected(idx) {
            return false;
        }
    } else if selection.count() > 1 && selection.is_selected(idx) {
        selection.make_primary(idx);
    } else {
        selection.select(idx);
    }
    let Some(entity) = scene.entities.get(idx) else {
        return false;
    };
    drag.start_drag(idx, x - entity.x, y - entity.y, x, y);
    // Every selected character is picked up: physics lets go of it and
    // it shows its Drag state (U.2).
    for i in selection.selected_indices() {
        if let Some(e) = scene.entities.get_mut(i) {
            e.physics.freeze();
            e.dragging = true;
        }
    }
    true
}

/// The pointer moved during a drag: the pressed character follows it and
/// the rest of the selection keeps its place around it.
pub fn drag_to(
    scene: &mut Scene,
    selection: &SelectionState,
    drag: &DragController,
    (x, y): (f32, f32),
) {
    let Some((idx, new_x, new_y)) = drag.update(x, y) else {
        return;
    };
    let Some(entity) = scene.entities.get(idx) else {
        return;
    };
    let (dx, dy) = (new_x - entity.x, new_y - entity.y);
    for i in selection.selected_indices() {
        if let Some(e) = scene.entities.get_mut(i) {
            e.x += dx;
            e.y += dy;
            // Relocating invalidates any Bounce rest position, or the
            // sprite springs back to where it was picked up.
            e.behavior_state.bounce_invalidate();
        }
    }
}

/// The drag ended: physics takes the selected characters back. A tap —
/// press and release without moving — on one of several selects it alone.
/// The caller pokes the tapped character first, if it does (a poke is
/// per character and knows the bounds).
pub fn end_drag(
    scene: &mut Scene,
    selection: &mut SelectionState,
    drag: &mut DragController,
    tapped: bool,
    shift: bool,
) {
    let pressed = drag.dragging_entity();
    for i in selection.selected_indices().into_iter().chain(pressed) {
        if let Some(e) = scene.entities.get_mut(i) {
            e.physics.unfreeze();
            e.dragging = false;
        }
    }
    if let Some(idx) = pressed {
        if tapped && !shift && selection.count() > 1 {
            selection.select(idx);
        }
    }
    drag.end_drag();
}

/// Edit mode ended in the middle of a gesture — Escape, the tray, a
/// shortcut, with the button still down. Let go of what the pointer held:
/// every selected character unfrozen and out of its Drag state, and any
/// selection rectangle dropped. The release will not come: in pass-through
/// the overlay no longer receives the pointer, and a drag left open made
/// the character jump to the pointer on the next move in edit mode.
pub fn cancel(
    scene: &mut Scene,
    selection: &mut SelectionState,
    drag: &mut DragController,
    marquee: &mut Option<Marquee>,
) {
    if drag.is_dragging() {
        end_drag(scene, selection, drag, false, false);
    }
    *marquee = None;
}

/// A selection rectangle being dragged over empty space.
#[derive(Debug, Clone)]
pub struct Marquee {
    start: (f32, f32),
    end: (f32, f32),
    /// What was selected before, kept with Shift.
    kept: Vec<usize>,
}

/// Under this many pixels across, a marquee was a click on empty space.
const CLICK_SLOP: f32 = 4.0;

impl Marquee {
    /// Begin at a press on empty space. With `shift` the current selection
    /// stays and the rectangle adds to it; without, it starts empty.
    pub fn begin(at: (f32, f32), shift: bool, selection: &mut SelectionState) -> Self {
        let kept = if shift {
            selection.selected_indices()
        } else {
            selection.deselect();
            Vec::new()
        };
        Self {
            start: at,
            end: at,
            kept,
        }
    }

    /// The pointer moved: the selection follows the rectangle as it grows
    /// or shrinks.
    pub fn drag_to(&mut self, at: (f32, f32), scene: &Scene, selection: &mut SelectionState) {
        self.end = at;
        if self.is_click() {
            return;
        }
        let mut picked = self.kept.clone();
        for idx in touched(scene, self.rect()) {
            if !picked.contains(&idx) {
                picked.push(idx);
            }
        }
        selection.select_all_of(&picked);
    }

    /// Left, top, right, bottom, in global coordinates.
    pub fn rect(&self) -> (f32, f32, f32, f32) {
        (
            self.start.0.min(self.end.0),
            self.start.1.min(self.end.1),
            self.start.0.max(self.end.0),
            self.start.1.max(self.end.1),
        )
    }

    /// Whether it never grew past a click.
    pub fn is_click(&self) -> bool {
        let (l, t, r, b) = self.rect();
        r - l < CLICK_SLOP && b - t < CLICK_SLOP
    }
}

/// The visible characters the rectangle (left, top, right, bottom) touches,
/// where the renderer draws them — a group's offset and scale included.
pub fn touched(scene: &Scene, (l, t, r, b): (f32, f32, f32, f32)) -> Vec<usize> {
    (0..scene.entities.len())
        .filter(|&i| scene.effective_visible(i))
        .filter(|&i| {
            let e = &scene.entities[i];
            let (gx, gy, gscale) = crate::group::transform_for_member(&scene.groups, &e.id);
            let (x, y) = (e.x + gx, e.y + gy);
            let (w, h) = (e.scaled_width() * gscale, e.scaled_height() * gscale);
            x < r && x + w > l && y < b && y + h > t
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behavior::Behavior;

    /// Three characters with no asset — fallback circles about 70 px
    /// across — at x = 0, 100, 200.
    fn scene() -> Scene {
        let mut scene = Scene::from_config(&crate::config::AppConfig::default());
        let configs: Vec<_> = [("a", 0.0), ("b", 100.0), ("c", 200.0)]
            .into_iter()
            .map(|(id, x)| crate::config::CharacterConfig {
                id: id.into(),
                name: id.into(),
                asset_type: crate::config::AssetType::PngStatic,
                asset_path: String::new(),
                x,
                y: 0.0,
                scale: 1.0,
                opacity: 1.0,
                fps: 8.0,
                visible: true,
                playing: false,
                z_index: 0,
                physics_enabled: false,
                behavior: Behavior::Idle,
                spritesheet_columns: None,
                spritesheet_rows: None,
                monitor: None,
                easing: None,
                animations: std::collections::BTreeMap::new(),
            })
            .collect();
        scene.restore_configs(&configs);
        scene
    }

    fn ids(scene: &Scene, selection: &SelectionState) -> Vec<String> {
        selection
            .selected_indices()
            .into_iter()
            .map(|i| scene.entities[i].id.clone())
            .collect()
    }

    #[test]
    fn a_press_selects_alone_or_keeps_the_group_it_is_part_of() {
        let mut s = scene();
        let (mut sel, mut drag) = (SelectionState::default(), DragController::new());
        assert!(press_on(&mut s, &mut sel, &mut drag, 0, (1.0, 1.0), false));
        end_drag(&mut s, &mut sel, &mut drag, false, false);
        assert!(press_on(&mut s, &mut sel, &mut drag, 1, (101.0, 1.0), true));
        end_drag(&mut s, &mut sel, &mut drag, false, true);
        assert_eq!(ids(&s, &sel), ["b", "a"], "Shift adds, and leads");
        // A plain press on one of the group keeps the group.
        assert!(press_on(&mut s, &mut sel, &mut drag, 0, (1.0, 1.0), false));
        assert_eq!(sel.count(), 2);
        assert_eq!(sel.selected_index(), Some(0));
        // ...and a tap without moving selects it alone.
        end_drag(&mut s, &mut sel, &mut drag, true, false);
        assert_eq!(ids(&s, &sel), ["a"]);
        // A press on one not selected selects it alone.
        sel.select_all_of(&[0, 1]);
        press_on(&mut s, &mut sel, &mut drag, 2, (201.0, 1.0), false);
        assert_eq!(ids(&s, &sel), ["c"]);
    }

    #[test]
    fn shift_on_a_selected_one_takes_it_out_without_dragging() {
        let mut s = scene();
        let (mut sel, mut drag) = (SelectionState::default(), DragController::new());
        sel.select_all_of(&[0, 1]);
        assert!(!press_on(
            &mut s,
            &mut sel,
            &mut drag,
            1,
            (101.0, 1.0),
            true
        ));
        assert!(!drag.is_dragging());
        assert_eq!(ids(&s, &sel), ["a"]);
    }

    #[test]
    fn the_whole_selection_moves_with_the_pressed_one() {
        let mut s = scene();
        let (mut sel, mut drag) = (SelectionState::default(), DragController::new());
        sel.select_all_of(&[0, 2]);
        press_on(&mut s, &mut sel, &mut drag, 2, (205.0, 5.0), false);
        drag_to(&mut s, &sel, &drag, (225.0, 45.0));
        assert_eq!((s.entities[2].x, s.entities[2].y), (220.0, 40.0));
        assert_eq!(
            (s.entities[0].x, s.entities[0].y),
            (20.0, 40.0),
            "kept its place"
        );
        assert_eq!(s.entities[1].x, 100.0, "not selected, not moved");
        end_drag(&mut s, &mut sel, &mut drag, false, false);
        assert_eq!(sel.count(), 2, "a real drag keeps the group");
        assert!(s.entities.iter().all(|e| !e.dragging));
    }

    #[test]
    fn cancelling_mid_drag_lets_go_of_everything() {
        let mut s = scene();
        let (mut sel, mut drag) = (SelectionState::default(), DragController::new());
        sel.select_all_of(&[0, 1]);
        press_on(&mut s, &mut sel, &mut drag, 0, (1.0, 1.0), false);
        assert!(s.entities[1].dragging);
        let mut marquee = Some(Marquee::begin((0.0, 0.0), true, &mut sel));
        cancel(&mut s, &mut sel, &mut drag, &mut marquee);
        assert!(!drag.is_dragging() && marquee.is_none());
        assert!(s.entities.iter().all(|e| !e.dragging));
        // No drag left to move anything on the next pointer motion.
        drag_to(&mut s, &sel, &drag, (500.0, 500.0));
        assert_eq!(s.entities[0].x, 0.0);
    }

    #[test]
    fn a_rectangle_selects_what_it_touches() {
        let s = scene();
        let mut sel = SelectionState::default();
        let mut m = Marquee::begin((-5.0, -5.0), false, &mut sel);
        m.drag_to((105.0, 5.0), &s, &mut sel);
        assert_eq!(ids(&s, &sel), ["a", "b"]);
        // Shrinking it lets go of what it no longer touches.
        m.drag_to((50.0, 5.0), &s, &mut sel);
        assert_eq!(ids(&s, &sel), ["a"]);
    }

    #[test]
    fn with_shift_a_rectangle_adds_to_the_selection() {
        let s = scene();
        let mut sel = SelectionState::default();
        sel.select(2);
        let mut m = Marquee::begin((-5.0, -5.0), true, &mut sel);
        m.drag_to((5.0, 5.0), &s, &mut sel);
        assert_eq!(ids(&s, &sel), ["c", "a"]);
        // Without Shift the old selection goes at the press.
        let mut m = Marquee::begin((95.0, -5.0), false, &mut sel);
        assert_eq!(sel.count(), 0);
        m.drag_to((105.0, 5.0), &s, &mut sel);
        assert_eq!(ids(&s, &sel), ["b"]);
    }

    #[test]
    fn a_click_on_empty_space_is_not_a_rectangle() {
        let s = scene();
        let mut sel = SelectionState::default();
        sel.select(0);
        let mut m = Marquee::begin((500.0, 500.0), false, &mut sel);
        m.drag_to((502.0, 501.0), &s, &mut sel);
        assert!(m.is_click());
        assert_eq!(sel.count(), 0, "deselected by the press itself");
    }

    #[test]
    fn hidden_characters_are_not_touched() {
        let mut s = scene();
        s.entities[1].visible = false;
        s.mark_visible_dirty();
        assert_eq!(touched(&s, (-10.0, -10.0, 300.0, 20.0)), [0, 2]);
    }
}
