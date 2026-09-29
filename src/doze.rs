//! Dozing off while nobody is there (1.5): the characters stop walking,
//! whoever is in the air lands, each takes its idle pose and says Zzz…,
//! and then the scene holds still — as cheap as a freeze, which is what
//! pausing when away used to be, mid-stride and mid-air. The first input
//! wakes them, and the Zzz… goes.
//!
//! Only for being away (`crate::away`): held still on battery with
//! someone at the keyboard, the scene just freezes, with no bubbles.

use crate::scene::Scene;
use crate::speech::Speech;
use std::time::{Duration, Instant};

/// What a dozing character says.
pub const ZZZ: &str = "Zzz…";

/// Whatever still falls after this long is left where it is.
pub const SETTLE_LIMIT: Duration = Duration::from_secs(3);

/// How long one Zzz… lasts. `Scene::tick` puts another in its place
/// while the scene sleeps, so this only bounds a bubble's `until`.
const ZZZ_SECONDS: u64 = 3600;

/// A scene dozing off, or asleep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Doze {
    /// When it began.
    pub since: Instant,
    /// Everyone has landed: the scene holds still.
    pub settled: bool,
}

/// A Zzz… from `now`.
pub fn zzz(now: Instant) -> Speech {
    Speech {
        text: ZZZ.to_string(),
        until: now + Duration::from_secs(ZZZ_SECONDS),
    }
}

impl Scene {
    /// Doze off, or wake up. Cheap when nothing changes; the loops call
    /// it every frame. Returns whether it changed.
    pub fn set_dozing(&mut self, dozing: bool) -> bool {
        if dozing == self.doze.is_some() {
            return false;
        }
        if dozing {
            let now = Instant::now();
            self.doze = Some(Doze {
                since: now,
                settled: false,
            });
            // Someone mid-sentence finishes it first (`Scene::tick`).
            for entity in &mut self.entities {
                if entity.speech.as_ref().is_none_or(|s| !s.showing(now)) {
                    entity.speech = Some(zzz(now));
                }
            }
        } else {
            self.doze = None;
            for entity in &mut self.entities {
                if entity.speech.as_ref().is_some_and(|s| s.text == ZZZ) {
                    entity.speech = None;
                }
            }
        }
        true
    }

    /// Whether the characters doze, landing or asleep.
    pub fn is_dozing(&self) -> bool {
        self.doze.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, CharacterConfig};

    fn scene_of(n: usize) -> Scene {
        let mut scene = Scene::from_config(&AppConfig::default());
        let template = AppConfig::default().characters.remove(0);
        let configs: Vec<_> = (0..n)
            .map(|i| CharacterConfig {
                id: format!("c{i}"),
                name: format!("c{i}"),
                asset_path: String::new(),
                x: 100.0 * i as f32,
                y: 100.0,
                physics_enabled: false,
                behavior: crate::behavior::Behavior::WalkAround { speed: 80.0 },
                ..template.clone()
            })
            .collect();
        scene.restore_configs(&configs);
        scene
    }

    fn bounds() -> crate::monitor::DesktopBounds {
        crate::monitor::DesktopBounds::from_size(1600.0, 1000.0)
    }

    #[test]
    fn dozing_off_says_zzz_and_waking_takes_it_back() {
        let mut scene = scene_of(2);
        let now = Instant::now();
        scene.entities[1].speech = Speech::new("Time for a break!", 10.0, now);
        assert!(scene.set_dozing(true));
        assert!(!scene.set_dozing(true), "no change the second time");
        assert_eq!(scene.entities[0].speech.as_ref().unwrap().text, ZZZ);
        assert_eq!(
            scene.entities[1].speech.as_ref().unwrap().text,
            "Time for a break!",
            "a reminder is finished first"
        );
        assert!(scene.set_dozing(false));
        assert!(scene.entities[0].speech.is_none());
        assert!(scene.entities[1].speech.is_some(), "only the Zzz… goes");
    }

    #[test]
    fn nobody_walks_in_their_sleep() {
        let mut scene = scene_of(1);
        scene.set_dozing(true);
        let x = scene.entities[0].x;
        std::thread::sleep(Duration::from_millis(20));
        scene.tick(bounds(), None, None);
        assert_eq!(scene.entities[0].x, x);
        // Nobody in the air: asleep at the first step.
        assert!(!scene.is_running());
        assert!(scene.is_dozing());
        scene.set_dozing(false);
        assert!(scene.is_running());
    }

    #[test]
    fn someone_falling_lands_before_the_scene_sleeps() {
        let mut scene = scene_of(1);
        scene.entities[0].physics.enable();
        // 30 px above the floor: down in well under `SETTLE_LIMIT`.
        let start = 1000.0 - scene.entities[0].scaled_height() - 30.0;
        scene.entities[0].y = start;
        scene.set_dozing(true);
        let began = Instant::now();
        std::thread::sleep(Duration::from_millis(20));
        scene.tick(bounds(), None, None);
        assert!(scene.is_running(), "still falling");
        assert!(scene.entities[0].y > start, "and it fell");
        while scene.is_running() && began.elapsed() < SETTLE_LIMIT {
            std::thread::sleep(Duration::from_millis(10));
            scene.tick(bounds(), None, None);
        }
        assert!(
            began.elapsed() < SETTLE_LIMIT,
            "it landed, not ran out of time"
        );
        assert!(!scene.is_running(), "landed and asleep");
        assert!(scene.entities[0].physics.grounded);
    }
}
