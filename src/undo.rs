//! Undo and redo for edits to the scene.
//!
//! Until 1.5 there was none: a Delete pressed by mistake, a character
//! dragged off by accident, "Replace scene" from the preset gallery — each
//! was final.
//!
//! Edits reach the scene from dozens of places: the Inspector's fields,
//! shortcuts, the mouse, drops, the palette, presets, the right-click
//! menu. Rather than hook each, this watches *gestures*. The first input
//! of one — a press, a key, a drop — takes a snapshot of the scene before
//! it is applied. The gesture closes once the input has gone quiet: no
//! button held, no text field being typed in, no file chooser open, and
//! [`SETTLE`] without input. If the scene then differs from the snapshot,
//! the snapshot is one step to undo. A slider dragged across a hundred
//! frames, or a burst of arrow-key nudges, is one step.
//!
//! Characters that move on their own — walking, falling, driven by a
//! script — change position every frame without anyone touching them.
//! Their positions are left out of the comparison, so that motion is never
//! mistaken for an edit, and undo leaves them where they have got to.

use crate::behavior::Behavior;
use crate::config::CharacterConfig;
use crate::scene::Scene;
use std::time::{Duration, Instant};

/// Quiet after the last input before a gesture counts as finished.
pub const SETTLE: Duration = Duration::from_millis(350);

/// Steps kept. Each is a list of character configs — a few kilobytes for
/// a full scene.
pub const MAX_STEPS: usize = 100;

/// The scene as its characters' configs, in scene order.
pub type Snapshot = Vec<CharacterConfig>;

/// The undo and redo stacks, and the gesture in progress.
#[derive(Default)]
pub struct UndoHistory {
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    /// The scene as it was when the open gesture began.
    before: Option<Snapshot>,
    last_input: Option<Instant>,
}

impl UndoHistory {
    /// User input arrived. Call it *before* the input is applied: the
    /// first call of a gesture keeps the scene as it was.
    pub fn input(&mut self, scene: &Scene, now: Instant) {
        if self.before.is_none() {
            self.before = Some(scene.to_character_configs());
        }
        self.last_input = Some(now);
    }

    /// Close the gesture if it has finished, recording a step if it
    /// changed the scene. `busy` holds it open: a pointer button down, a
    /// text field focused, a file chooser or import still running.
    /// Returns whether a step was recorded.
    pub fn settle(&mut self, scene: &Scene, busy: bool, now: Instant) -> bool {
        match self.last_input {
            Some(last) if !busy && now.saturating_duration_since(last) >= SETTLE => {
                self.commit(scene)
            }
            _ => false,
        }
    }

    /// Undo the last step. A gesture still open is recorded first, so it
    /// is the one undone. `false` when there is nothing to undo.
    pub fn undo(&mut self, scene: &mut Scene) -> bool {
        self.commit(scene);
        let Some(step) = self.undo.pop() else {
            return false;
        };
        self.redo.push(scene.to_character_configs());
        scene.restore_configs(&walkers_where_they_are(step, scene));
        true
    }

    /// Redo the last undone step. `false` when there is none — including
    /// after a new edit, which starts a new branch.
    pub fn redo(&mut self, scene: &mut Scene) -> bool {
        self.commit(scene);
        let Some(step) = self.redo.pop() else {
            return false;
        };
        self.undo.push(scene.to_character_configs());
        scene.restore_configs(&walkers_where_they_are(step, scene));
        true
    }

    /// Close the open gesture now, recording what it edited so far. For
    /// what follows that is play rather than an edit: a tap pokes the
    /// character and it hops aside, and undo must not start by putting it
    /// back. Returns whether a step was recorded.
    pub fn finish(&mut self, scene: &Scene) -> bool {
        self.commit(scene)
    }

    /// Forget everything: the scene was replaced from outside, by a
    /// reload of the config file.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Close the open gesture now, recording it if it edited anything.
    fn commit(&mut self, scene: &Scene) -> bool {
        self.last_input = None;
        let Some(before) = self.before.take() else {
            return false;
        };
        if !edited(&before, &scene.to_character_configs()) {
            return false;
        }
        self.undo.push(before);
        if self.undo.len() > MAX_STEPS {
            self.undo.remove(0);
        }
        self.redo.clear();
        true
    }
}

/// Whether the user changed anything between `before` and `after`, leaving
/// out where characters that move by themselves have moved to.
fn edited(before: &[CharacterConfig], after: &[CharacterConfig]) -> bool {
    before.len() != after.len()
        || before.iter().zip(after).any(|(b, a)| {
            if moves_by_itself(b) && moves_by_itself(a) {
                let placed = CharacterConfig {
                    x: b.x,
                    y: b.y,
                    ..a.clone()
                };
                *b != placed
            } else {
                b != a
            }
        })
}

/// Whether `event` is someone doing something — a press or release, a
/// key, typed text, a scroll, a screen reader's request — rather than the
/// pointer passing over. Only these open or extend a gesture: counting
/// motion would merge two slider drags with the pointer moving between
/// them into one step.
pub fn is_user_action(event: &egui::Event) -> bool {
    !matches!(
        event,
        egui::Event::PointerMoved(_)
            | egui::Event::MouseMoved(_)
            | egui::Event::PointerGone
            | egui::Event::WindowFocused(_)
    )
}

/// `step` with each character that moves by itself — in the step and in
/// the scene now — placed where it is now: undo sets edits back, and
/// where a walker has walked to is not one.
fn walkers_where_they_are(mut step: Snapshot, scene: &Scene) -> Snapshot {
    for config in &mut step {
        let now = scene.entities.iter().find(|e| e.id == config.id);
        if let Some(entity) = now {
            if moves_by_itself(config) && moves_by_itself(&entity.to_config()) {
                config.x = entity.x;
                config.y = entity.y;
            }
        }
    }
    step
}

/// Whether a character changes position without anyone touching it.
pub(crate) fn moves_by_itself(config: &CharacterConfig) -> bool {
    config.physics_enabled || !matches!(config.behavior, Behavior::Idle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn character(id: &str, x: f32) -> CharacterConfig {
        CharacterConfig {
            id: id.into(),
            name: id.into(),
            asset_type: crate::config::AssetType::PngStatic,
            asset_path: String::new(),
            x,
            y: 200.0,
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
        }
    }

    fn scene(characters: &[CharacterConfig]) -> Scene {
        let mut scene = Scene::from_config(&crate::config::AppConfig::default());
        scene.restore_configs(characters);
        scene
    }

    fn x_of(scene: &Scene, id: &str) -> f32 {
        scene.entities.iter().find(|e| e.id == id).unwrap().x
    }

    #[test]
    fn a_gesture_is_one_step_however_many_frames_it_takes() {
        let mut s = scene(&[character("a", 100.0)]);
        let mut h = UndoHistory::default();
        let t0 = Instant::now();
        // A drag: input every frame, the position following it.
        for i in 0..30 {
            let now = t0 + Duration::from_millis(16 * i);
            h.input(&s, now);
            s.entities[0].x = 100.0 + i as f32 * 10.0;
            assert!(!h.settle(&s, true, now), "button held: still open");
        }
        let released = t0 + Duration::from_millis(16 * 30);
        assert!(!h.settle(&s, false, released), "not quiet long enough yet");
        assert!(h.settle(&s, false, released + SETTLE), "one step");
        assert!(h.undo(&mut s));
        assert_eq!(x_of(&s, "a"), 100.0, "back where the drag began");
        assert!(!h.can_undo());
        assert!(h.redo(&mut s));
        assert_eq!(x_of(&s, "a"), 390.0);
    }

    #[test]
    fn a_gesture_that_changes_nothing_is_not_a_step() {
        let s = scene(&[character("a", 100.0)]);
        let mut h = UndoHistory::default();
        let t0 = Instant::now();
        h.input(&s, t0);
        assert!(!h.settle(&s, false, t0 + SETTLE));
        assert!(!h.can_undo());
    }

    #[test]
    fn walking_is_not_an_edit_and_undo_leaves_walkers_where_they_are() {
        let mut walker = character("w", 100.0);
        walker.behavior = Behavior::WalkAround { speed: 50.0 };
        let mut s = scene(&[walker, character("a", 10.0)]);
        let mut h = UndoHistory::default();
        let t0 = Instant::now();
        // Only the walker moves during a gesture: nothing to record.
        h.input(&s, t0);
        s.entities[0].x = 180.0;
        assert!(!h.settle(&s, false, t0 + SETTLE));
        // A real edit while it walks: one step, and undoing it keeps the
        // walker where it has walked to.
        h.input(&s, t0 + SETTLE * 2);
        s.entities[1].scale = 2.0;
        s.entities[0].x = 250.0;
        assert!(h.settle(&s, false, t0 + SETTLE * 4));
        s.entities[0].x = 300.0;
        assert!(h.undo(&mut s));
        assert_eq!(s.entities[1].scale, 1.0);
        assert_eq!(x_of(&s, "w"), 300.0, "the walker stays where it walked to");
        assert!(!h.can_undo());
    }

    #[test]
    fn a_deleted_character_comes_back_and_an_added_one_goes() {
        let mut s = scene(&[character("a", 1.0), character("b", 2.0)]);
        let mut h = UndoHistory::default();
        let t0 = Instant::now();
        h.input(&s, t0);
        s.entities.retain(|e| e.id != "a");
        assert!(h.settle(&s, false, t0 + SETTLE));
        h.input(&s, t0 + SETTLE * 2);
        s.restore_configs(&[character("b", 2.0), character("c", 3.0)]);
        assert!(h.settle(&s, false, t0 + SETTLE * 3));
        assert!(h.undo(&mut s));
        let ids: Vec<&str> = s.entities.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["b"]);
        assert!(h.undo(&mut s));
        let ids: Vec<&str> = s.entities.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["a", "b"]);
    }

    #[test]
    fn a_new_edit_after_undo_drops_the_redo() {
        let mut s = scene(&[character("a", 1.0)]);
        let mut h = UndoHistory::default();
        let t0 = Instant::now();
        h.input(&s, t0);
        s.entities[0].opacity = 0.5;
        assert!(h.settle(&s, false, t0 + SETTLE));
        assert!(h.undo(&mut s));
        assert!(h.can_redo());
        h.input(&s, t0 + SETTLE * 2);
        s.entities[0].scale = 3.0;
        assert!(h.settle(&s, false, t0 + SETTLE * 3));
        assert!(!h.can_redo());
    }

    #[test]
    fn undo_records_a_gesture_still_open_and_undoes_it() {
        let mut s = scene(&[character("a", 1.0)]);
        let mut h = UndoHistory::default();
        let t0 = Instant::now();
        h.input(&s, t0);
        s.entities[0].x = 50.0;
        // Ctrl+Z straight after, before the gesture settled.
        h.input(&s, t0 + Duration::from_millis(100));
        assert!(h.undo(&mut s));
        assert_eq!(x_of(&s, "a"), 1.0);
        assert!(!h.settle(&s, false, t0 + SETTLE * 2), "nothing left open");
    }

    #[test]
    fn a_poke_after_finish_is_not_a_step() {
        let mut s = scene(&[character("a", 100.0)]);
        let mut h = UndoHistory::default();
        let t0 = Instant::now();
        h.input(&s, t0);
        assert!(!h.finish(&s), "the tap itself changed nothing");
        s.entities[0].x = 140.0; // the hop
        assert!(!h.settle(&s, false, t0 + SETTLE));
        assert!(!h.can_undo());
    }

    #[test]
    fn the_history_is_bounded() {
        let mut s = scene(&[character("a", 0.0)]);
        let mut h = UndoHistory::default();
        let t0 = Instant::now();
        for i in 0..(MAX_STEPS + 20) {
            let now = t0 + SETTLE * (2 * i as u32);
            h.input(&s, now);
            s.entities[0].x = i as f32 + 1.0;
            assert!(h.settle(&s, false, now + SETTLE));
        }
        assert_eq!(h.undo.len(), MAX_STEPS);
    }
}
