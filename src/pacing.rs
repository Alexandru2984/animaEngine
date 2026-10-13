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
    /// Something moves every frame: the perf overlay,
    /// a behavior, physics. Draw at the display's rate.
    Continuous,
    /// Only animated sprites play; the soonest of them shows its next
    /// frame at this instant. An 8 fps sprite needs 8 frames a second,
    /// not 60.
    Deadline(Instant),
    /// Nothing moves.
    Idle,
}

/// The shortest time between two frames: about sixty a second. A
/// present that waits for the display (`Fifo`) paces most drivers by
/// itself, but not every one, nor a window that is not mapped.
pub const FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// The longest the overlay goes without a frame. Matches the config
/// hot-reload poll, so an edit to the file still shows while the scene
/// sits still.
pub const IDLE_HEARTBEAT: Duration = Duration::from_secs(2);

/// What `scene` needs next. `ui_animating` is true while something outside
/// the scene moves every frame: the perf overlay. A toast is not — the
/// loops ask it when it next changes (`ToastQueue::next_change`) — and
/// edit mode is not one of them: egui asks for the frames the panel
/// needs ([`sooner`]).
pub fn redraw_pacing(scene: &Scene, ui_animating: bool) -> RedrawPacing {
    if ui_animating {
        return RedrawPacing::Continuous;
    }
    // Hearts, dust and z's run on the clock, a sleeping scene's too
    // (`crate::effects`); they are over in a moment or two.
    if scene.effects.alive(Instant::now()) || scene.is_fading() {
        return RedrawPacing::Continuous;
    }
    if !scene.is_running() {
        return RedrawPacing::Idle;
    }
    let mut deadline: Option<Instant> = None;
    for entity in scene.visible_entities() {
        if entity.physics.enabled
            || entity.physics.thrown
            || !matches!(entity.behavior, crate::behavior::Behavior::Idle)
        {
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

/// The soonest of the times the scene's clocks come due — a reminder, the
/// focus timer's turn — and `at`, egui's: what a loop that sleeps should
/// wake for besides frames.
pub fn soonest(at: Option<Instant>, reminders: Option<Instant>) -> Option<Instant> {
    [at, reminders, crate::focus::next_turn()]
        .into_iter()
        .flatten()
        .min()
}

/// How often a loop that sleeps until woken looks at a result it waits
/// for that wakes nothing — a file chooser, a clipboard read, a reload.
pub const AWAIT_POLL: Duration = Duration::from_millis(100);

/// How long a loop may sleep before it next has something to do: a frame
/// interval while anything moves; else until `pacing`'s deadline, at most
/// the heartbeat after the last frame (`last_draw`), and [`AWAIT_POLL`]
/// while `awaiting` a result.
pub fn next_wait(
    pacing: RedrawPacing,
    last_draw: Option<Instant>,
    awaiting: bool,
    now: Instant,
) -> Duration {
    let mut until = match pacing {
        RedrawPacing::Continuous => return FRAME_INTERVAL,
        RedrawPacing::Deadline(due) => due,
        RedrawPacing::Idle => now + IDLE_HEARTBEAT,
    };
    until = until.min(last_draw.map_or(now, |last| last + IDLE_HEARTBEAT));
    if awaiting {
        until = until.min(now + AWAIT_POLL);
    }
    until.saturating_duration_since(now)
}

/// `pacing`, or sooner if something outside the scene needs a frame by
/// `at` — egui's next repaint (a tooltip's delay, a caret's blink, an
/// animation), a result waiting to be polled.
pub fn sooner(pacing: RedrawPacing, at: Option<Instant>, now: Instant) -> RedrawPacing {
    let Some(at) = at else {
        return pacing;
    };
    if at <= now {
        return RedrawPacing::Continuous;
    }
    match pacing {
        RedrawPacing::Continuous => RedrawPacing::Continuous,
        RedrawPacing::Deadline(due) => RedrawPacing::Deadline(due.min(at)),
        RedrawPacing::Idle => RedrawPacing::Deadline(at),
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
            talk: Default::default(),
            hours: None,
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
    fn an_effect_is_drawn_through_to_its_end_on_a_still_scene() {
        let mut s = scene(vec![entity(1, false, Behavior::Idle)]);
        s.global_playing = false;
        assert_eq!(redraw_pacing(&s, false), RedrawPacing::Idle);
        s.effects.hearts("e", (100.0, 100.0), Instant::now());
        assert_eq!(redraw_pacing(&s, false), RedrawPacing::Continuous);
        s.effects.clear();
        assert_eq!(redraw_pacing(&s, false), RedrawPacing::Idle);
    }

    #[test]
    fn a_repaint_asked_for_comes_first() {
        let now = Instant::now();
        let soon = now + Duration::from_millis(500);
        let later = now + Duration::from_secs(1);
        assert_eq!(sooner(RedrawPacing::Idle, None, now), RedrawPacing::Idle);
        assert_eq!(
            sooner(RedrawPacing::Idle, Some(soon), now),
            RedrawPacing::Deadline(soon)
        );
        assert_eq!(
            sooner(RedrawPacing::Deadline(later), Some(soon), now),
            RedrawPacing::Deadline(soon)
        );
        assert_eq!(
            sooner(RedrawPacing::Deadline(soon), Some(later), now),
            RedrawPacing::Deadline(soon)
        );
        assert_eq!(
            sooner(RedrawPacing::Idle, Some(now), now),
            RedrawPacing::Continuous
        );
        assert_eq!(
            sooner(RedrawPacing::Continuous, Some(later), now),
            RedrawPacing::Continuous
        );
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

    /// A loop that sleeps until woken sleeps as long as it can: a frame
    /// while anything moves, else to the next deadline, never past the
    /// heartbeat, and a tenth of a second while a result is due.
    #[test]
    fn a_still_scene_lets_the_loop_sleep() {
        let now = Instant::now();
        let ms = Duration::from_millis;
        let drew = Some(now);
        assert_eq!(
            next_wait(RedrawPacing::Continuous, drew, false, now),
            FRAME_INTERVAL
        );
        assert_eq!(
            next_wait(RedrawPacing::Idle, drew, false, now),
            IDLE_HEARTBEAT
        );
        assert_eq!(
            next_wait(RedrawPacing::Deadline(now + ms(125)), drew, false, now),
            ms(125),
            "an 8 fps sprite's next picture"
        );
        assert_eq!(
            next_wait(RedrawPacing::Deadline(now + ms(9000)), drew, false, now),
            IDLE_HEARTBEAT
        );
        assert_eq!(
            next_wait(RedrawPacing::Idle, Some(now - ms(1500)), false, now),
            ms(500),
            "the heartbeat counts from the last frame"
        );
        assert_eq!(next_wait(RedrawPacing::Idle, None, false, now), ms(0));
        assert_eq!(next_wait(RedrawPacing::Idle, drew, true, now), AWAIT_POLL);
        assert_eq!(
            next_wait(RedrawPacing::Deadline(now - ms(5)), drew, false, now),
            ms(0),
            "overdue"
        );
    }
}
