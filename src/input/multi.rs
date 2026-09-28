//! Editing several characters at once with the pointer (1.5): Shift+click,
//! a selection rectangle dragged over empty space, and dragging the whole
//! selection. Shared by both backends, which feed it global desktop
//! coordinates, so the two cannot come to behave differently.
//!
//! The conventions are the usual ones. A press on a character selects it
//! alone — with the rest of its group, if it is in one (`crate::group`) —
//! unless it is one of several already selected: then they all stay
//! selected, so they can be dragged, and a tap without moving selects it
//! alone. So a click on a grouped character takes the group, and a second
//! click takes just that one. Shift+click adds or removes a character. A
//! drag over empty space selects what the rectangle touches — added to the
//! selection with Shift — and a click there deselects.

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
    let mut narrow_on_tap = false;
    if shift {
        selection.toggle(idx);
        if !selection.is_selected(idx) {
            return false;
        }
    } else if selection.count() > 1 && selection.is_selected(idx) {
        selection.make_primary(idx);
        narrow_on_tap = true;
    } else {
        select_with_its_group(scene, selection, idx);
    }
    let Some(entity) = scene.entities.get(idx) else {
        return false;
    };
    drag.start_drag(idx, x - entity.x, y - entity.y, x, y);
    if narrow_on_tap {
        drag.narrow_on_tap();
    }
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
///
/// Nothing moves once the pressed character has left the selection:
/// something else changed it mid-drag, and the rest would be moved by the
/// pressed one's whole offset again at every step, flying off.
pub fn drag_to(
    scene: &mut Scene,
    selection: &SelectionState,
    drag: &DragController,
    (x, y): (f32, f32),
) {
    let Some((idx, new_x, new_y)) = drag.update(x, y) else {
        return;
    };
    if !selection.is_selected(idx) {
        return;
    }
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
        if tapped && !shift && drag.narrows_on_tap() {
            selection.select(idx);
        }
    }
    drag.end_drag();
}

/// Select the character at `idx` with the rest of its group, it first.
pub fn select_with_its_group(scene: &Scene, selection: &mut SelectionState, idx: usize) {
    let mut picked = vec![idx];
    picked.extend(scene.with_its_group(idx).into_iter().filter(|&i| i != idx));
    selection.select_all_of(&picked);
}

/// A right-click on the character at `idx`: one of several selected keeps
/// them all, so the menu acts on every one; otherwise it is selected with
/// its group.
pub fn select_for_menu(scene: &Scene, selection: &mut SelectionState, idx: usize) {
    if !selection.is_selected(idx) {
        select_with_its_group(scene, selection, idx);
    }
}

/// Something ended or overtook a gesture with the button still down. Let
/// go of what the pointer held: every selected character unfrozen and out
/// of its Drag state, and any selection rectangle dropped. Returns whether
/// a drag was let go, having moved characters that need saving.
///
/// - Edit mode ended, or the overlay hid — Escape, the tray, a shortcut, a
///   full-screen app. The release will not come: the overlay no longer
///   receives the pointer, and a drag left open made the character jump
///   to the pointer on the next move.
/// - A shortcut is about to change which characters exist or are selected
///   ([`crate::keybindings::Action::interrupts_drag`]), and a gesture
///   holds indices into both.
pub fn cancel(
    scene: &mut Scene,
    selection: &mut SelectionState,
    drag: &mut DragController,
    marquee: &mut Option<Marquee>,
) -> bool {
    *marquee = None;
    if !drag.is_dragging() {
        return false;
    }
    end_drag(scene, selection, drag, false, false);
    true
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
    fn a_click_takes_the_group_and_a_second_click_the_one() {
        let mut s = scene();
        s.group_entities(&[0, 2], |n| format!("Group {n}"));
        let (mut sel, mut drag) = (SelectionState::default(), DragController::new());
        // First click on "c": the whole group, "c" leading — and the tap
        // keeps it.
        press_on(&mut s, &mut sel, &mut drag, 2, (201.0, 1.0), false);
        end_drag(&mut s, &mut sel, &mut drag, true, false);
        assert_eq!(ids(&s, &sel), ["c", "a"]);
        // Second click on "c": just "c".
        press_on(&mut s, &mut sel, &mut drag, 2, (201.0, 1.0), false);
        end_drag(&mut s, &mut sel, &mut drag, true, false);
        assert_eq!(ids(&s, &sel), ["c"]);
        // A drag from a fresh click moves the whole group.
        sel.deselect();
        press_on(&mut s, &mut sel, &mut drag, 0, (1.0, 1.0), false);
        drag_to(&mut s, &sel, &drag, (11.0, 1.0));
        end_drag(&mut s, &mut sel, &mut drag, false, false);
        assert_eq!((s.entities[0].x, s.entities[2].x), (10.0, 210.0));
        assert_eq!(s.entities[1].x, 100.0, "not in the group");
        // Shift still takes a single character.
        sel.deselect();
        press_on(&mut s, &mut sel, &mut drag, 0, (11.0, 1.0), true);
        end_drag(&mut s, &mut sel, &mut drag, true, true);
        assert_eq!(ids(&s, &sel), ["a"]);
    }

    #[test]
    fn a_right_click_takes_the_group_unless_already_selected() {
        let mut s = scene();
        s.group_entities(&[1, 2], |n| format!("Group {n}"));
        let mut sel = SelectionState::default();
        select_for_menu(&s, &mut sel, 1);
        assert_eq!(ids(&s, &sel), ["b", "c"]);
        sel.select_all_of(&[0, 2]);
        select_for_menu(&s, &mut sel, 2);
        assert_eq!(ids(&s, &sel), ["a", "c"], "kept as it was");
    }

    #[test]
    fn a_selection_changed_mid_drag_is_not_moved() {
        let mut s = scene();
        let (mut sel, mut drag) = (SelectionState::default(), DragController::new());
        press_on(&mut s, &mut sel, &mut drag, 0, (1.0, 1.0), false);
        drag_to(&mut s, &sel, &drag, (21.0, 1.0));
        assert_eq!(s.entities[0].x, 20.0);
        // What Duplicate does: the copy — here "c" — becomes the selection
        // while the pressed one stays where it is.
        sel.select(2);
        for step in 1..=3 {
            drag_to(&mut s, &sel, &drag, (21.0 + 10.0 * step as f32, 1.0));
        }
        assert_eq!(s.entities[2].x, 200.0, "not sent flying");
        assert_eq!(s.entities[0].x, 20.0);
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
