//! When the overlay needs another frame — shared by both backends.
//!
//! An overlay sits on screen all day, and most of that time nothing on it
//! moves. Drawing an unchanged scene sixty times a second costs a laptop
//! its battery for nothing. The winit loop has slept when nothing moves
//! since 0.5.5; the native Wayland loop drew every frame regardless — 84%
//! of a core in the rig with everything paused, against 0.9% on X11. Both
//! now ask this function what the scene needs.

use crate::scene::Scene;
use std::time::{Duration, Instant};

/// How soon the scene needs another frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedrawPacing {
    /// Something moves every frame: edit mode, a toast, the perf overlay,
    /// a behavior, physics. Draw at the display's rate.
    Continuous,
    /// Only animated sprites play; the soonest of them shows its next
    /// frame at this instant. An 8 fps sprite needs 8 frames a second,
    /// not 60.
    Deadline(Instant),
    /// Nothing moves.
    Idle,
}

/// The longest the overlay goes without a frame. Matches the config
/// hot-reload poll, so an edit to the file still shows while the scene
/// sits still.
pub const IDLE_HEARTBEAT: Duration = Duration::from_secs(2);

/// What `scene` needs next. `ui_animating` is true while something outside
/// the scene moves every frame: edit mode, a toast, the perf overlay.
pub fn redraw_pacing(scene: &Scene, ui_animating: bool) -> RedrawPacing {
    if ui_animating {
        return RedrawPacing::Continuous;
    }
    if !scene.global_playing {
        return RedrawPacing::Idle;
    }
    let mut deadline: Option<Instant> = None;
    for entity in scene.visible_entities() {
        if entity.physics.enabled || !matches!(entity.behavior, crate::behavior::Behavior::Idle) {
            return RedrawPacing::Continuous;
        }
        if entity.animation().playing && entity.animation().frame_count() > 1 {
            let due = entity.animation().next_frame_due();
            deadline = Some(deadline.map_or(due, |d| d.min(due)));
        }
    }
    match deadline {
        Some(due) => RedrawPacing::Deadline(due),
        None => RedrawPacing::Idle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behavior::Behavior;

    fn entity(frames: usize, playing: bool, behavior: Behavior) -> crate::entity::Entity {
        let cfg = crate::config::CharacterConfig {
            id: "e".into(),
            name: "E".into(),
            asset_type: crate::config::AssetType::PngStatic,
            asset_path: String::new(),
            x: 100.0,
            y: 200.0,
            scale: 1.0,
            opacity: 1.0,
            fps: 8.0,
            visible: true,
            playing,
            z_index: 0,
            physics_enabled: false,
            behavior,
            spritesheet_columns: None,
            spritesheet_rows: None,
            monitor: None,
            easing: None,
            animations: std::collections::BTreeMap::new(),
        };
        let frames = (0..frames)
            .map(|_| crate::animation::frame::Frame::new(vec![0u8; 4], 1, 1))
            .collect();
        let anim = crate::animation::Animation::new(frames, 8.0, playing);
        crate::entity::Entity::from_config(&cfg, anim)
    }

    fn scene(entities: Vec<crate::entity::Entity>) -> Scene {
        let mut scene = Scene::from_config(&crate::config::AppConfig::default());
        scene.entities = entities;
        scene
    }

    #[test]
    fn a_still_scene_idles() {
        let s = scene(vec![entity(1, false, Behavior::Idle)]);
        assert_eq!(redraw_pacing(&s, false), RedrawPacing::Idle);
        assert_eq!(redraw_pacing(&scene(Vec::new()), false), RedrawPacing::Idle);
    }

    #[test]
    fn the_ui_moving_needs_every_frame() {
        let s = scene(vec![entity(1, false, Behavior::Idle)]);
        assert_eq!(redraw_pacing(&s, true), RedrawPacing::Continuous);
    }

    #[test]
    fn a_behavior_needs_every_frame_until_paused() {
        let walker = Behavior::WalkAround { speed: 50.0 };
        let mut s = scene(vec![entity(1, false, walker)]);
        assert_eq!(redraw_pacing(&s, false), RedrawPacing::Continuous);
        s.global_playing = false;
        assert_eq!(redraw_pacing(&s, false), RedrawPacing::Idle);
    }

    #[test]
    fn an_animated_sprite_wakes_only_for_its_next_frame() {
        let s = scene(vec![entity(4, true, Behavior::Idle)]);
        assert!(matches!(
            redraw_pacing(&s, false),
            RedrawPacing::Deadline(_)
        ));
        // A single frame "playing" has nothing to show next.
        let s = scene(vec![entity(1, true, Behavior::Idle)]);
        assert_eq!(redraw_pacing(&s, false), RedrawPacing::Idle);
    }
}
