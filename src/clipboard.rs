//! Characters copied to be pasted later — into the same scene, or into
//! another one after switching (`crate::scenes`).
//!
//! The app's own clipboard, not the desktop's: a copy is each character's
//! whole config — its asset, animations, behavior, scale — which nothing
//! else could paste, so it is kept here for as long as the app runs.
//!
//! A paste lands where the copies were, or 30 px down and right of that
//! while the spot is taken. Pasted into the scene they came from, copies
//! step along as Duplicate's do; cut and pasted, or pasted into another
//! scene, they land where they were.

use crate::config::CharacterConfig;
use crate::group::GroupConfig;
use crate::scene::Scene;
use std::sync::Mutex;

/// How far each paste steps from a taken spot, as Duplicate does.
pub const STEP: f32 = 30.0;

/// At most this many steps are tried; a paste lands on the last one
/// even if that is taken too.
const MAX_STEPS: u32 = 20;

/// What was copied.
#[derive(Debug, Clone, PartialEq)]
pub struct Copied {
    /// Bottom of the stack first, so they paste in the same order.
    pub characters: Vec<CharacterConfig>,
    /// The group they were, when they were exactly one — the copies
    /// make a group like it.
    pub group: Option<GroupConfig>,
}

impl Copied {
    /// The characters at `indices` in `scene`; `None` when none of them
    /// is one.
    pub fn of(scene: &Scene, indices: &[usize]) -> Option<Self> {
        let mut unique = indices.to_vec();
        unique.sort_unstable();
        unique.dedup();
        let mut entities: Vec<_> = unique
            .iter()
            .filter_map(|&i| scene.entities.get(i))
            .collect();
        if entities.is_empty() {
            return None;
        }
        entities.sort_by_key(|e| e.z_index);
        Some(Self {
            characters: entities.iter().map(|e| e.to_config()).collect(),
            group: scene.exact_group(indices).cloned(),
        })
    }

    /// The copies as they go into `scene`: moved along from where they
    /// were until none sits exactly on a character already there. Ids
    /// and stacking are the paste's to settle, one at a time.
    pub fn placed_in(&self, scene: &Scene) -> Vec<CharacterConfig> {
        let taken = |dx: f32| {
            self.characters.iter().any(|c| {
                scene
                    .entities
                    .iter()
                    .any(|e| (e.x - (c.x + dx)).abs() < 1.0 && (e.y - (c.y + dx)).abs() < 1.0)
            })
        };
        let steps = (0..MAX_STEPS)
            .find(|&n| !taken(n as f32 * STEP))
            .unwrap_or(MAX_STEPS);
        let d = steps as f32 * STEP;
        self.characters
            .iter()
            .map(|c| CharacterConfig {
                x: c.x + d,
                y: c.y + d,
                ..c.clone()
            })
            .collect()
    }

    /// The name the toasts use: the character's, when there is one.
    pub fn single_name(&self) -> Option<&str> {
        match self.characters.as_slice() {
            [one] => Some(&one.name),
            _ => None,
        }
    }
}

static CLIPBOARD: Mutex<Option<Copied>> = Mutex::new(None);

/// Keep `copied` for the next paste, in place of what was there.
pub fn put(copied: Copied) {
    if let Ok(mut held) = CLIPBOARD.lock() {
        *held = Some(copied);
    }
}

/// What the next paste would add, if anything was copied.
pub fn get() -> Option<Copied> {
    CLIPBOARD.lock().ok().and_then(|held| held.clone())
}

/// How many characters a paste would add — 0 before any copy.
pub fn count() -> usize {
    CLIPBOARD
        .lock()
        .ok()
        .and_then(|held| held.as_ref().map(|c| c.characters.len()))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;

    fn scene_at(spots: &[(&str, f32, f32)]) -> Scene {
        let mut scene = Scene::from_config(&AppConfig::default());
        let template = AppConfig::default().characters.remove(0);
        let configs: Vec<_> = spots
            .iter()
            .enumerate()
            .map(|(z, &(id, x, y))| CharacterConfig {
                id: id.into(),
                name: id.into(),
                asset_path: String::new(),
                x,
                y,
                z_index: 10 * z as i32,
                ..template.clone()
            })
            .collect();
        scene.restore_configs(&configs);
        scene
    }

    #[test]
    fn nothing_selected_copies_nothing() {
        let scene = scene_at(&[("a", 0.0, 0.0)]);
        assert_eq!(Copied::of(&scene, &[]), None);
        assert_eq!(Copied::of(&scene, &[7]), None);
    }

    #[test]
    fn a_copy_keeps_the_stacking_order() {
        let mut scene = scene_at(&[("a", 0.0, 0.0), ("b", 50.0, 0.0)]);
        scene.entities[0].z_index = 90;
        let copied = Copied::of(&scene, &[0, 1, 0]).expect("two characters");
        let ids: Vec<_> = copied.characters.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["b", "a"], "bottom first, each once");
        assert_eq!(copied.group, None);
    }

    #[test]
    fn a_whole_group_is_copied_with_it() {
        let mut scene = scene_at(&[("a", 0.0, 0.0), ("b", 50.0, 0.0), ("c", 90.0, 0.0)]);
        scene.group_entities(&[0, 1], |n| format!("Group {n}"));
        assert!(Copied::of(&scene, &[0, 1]).unwrap().group.is_some());
        assert!(
            Copied::of(&scene, &[0]).unwrap().group.is_none(),
            "part of it"
        );
        assert!(
            Copied::of(&scene, &[0, 1, 2]).unwrap().group.is_none(),
            "more"
        );
    }

    #[test]
    fn a_paste_steps_off_the_originals_and_off_earlier_pastes() {
        let mut scene = scene_at(&[("a", 100.0, 200.0), ("b", 300.0, 200.0)]);
        let copied = Copied::of(&scene, &[0, 1]).unwrap();
        let first = copied.placed_in(&scene);
        assert_eq!((first[0].x, first[0].y), (130.0, 230.0));
        assert_eq!((first[1].x, first[1].y), (330.0, 230.0));
        // The first paste is in the scene now: the next steps past it.
        let mut with_paste = scene.to_character_configs();
        for (i, c) in first.iter().enumerate() {
            with_paste.push(CharacterConfig {
                id: format!("p{i}"),
                ..c.clone()
            });
        }
        scene.restore_configs(&with_paste);
        let second = copied.placed_in(&scene);
        assert_eq!((second[0].x, second[0].y), (160.0, 260.0));
    }

    #[test]
    fn where_the_spot_is_free_a_paste_lands_on_it() {
        // Cut and pasted back, or into another scene.
        let scene = scene_at(&[("a", 100.0, 200.0)]);
        let copied = Copied::of(&scene, &[0]).unwrap();
        let elsewhere = scene_at(&[("z", 500.0, 500.0)]);
        let placed = copied.placed_in(&elsewhere);
        assert_eq!((placed[0].x, placed[0].y), (100.0, 200.0));
        assert_eq!(copied.single_name(), Some("a"));
    }
}
