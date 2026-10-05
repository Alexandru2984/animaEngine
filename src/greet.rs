//! Greetings (1.5, off by default: `characters_greet`): two characters
//! that meet — walkers crossing, a follower reaching its leader, one
//! landing on another's head — stand still a moment and say hello, one
//! after the other, with a heart between them (`crate::effects`).
//!
//! They meet when what can be seen of them comes within [`NEAR`] pixels,
//! the sides a little inside their pictures as for bumping
//! (`crate::bump`). Only coming together counts: pairs already that close
//! when the scene starts or changes say nothing, nor do two who simply
//! stay side by side. Each pair greets at most every [`PAIR_EVERY`], the
//! scene at most every [`ANY_EVERY`], and nobody talks over someone
//! already talking — so a crowd does not chatter.

use crate::scene::Scene;
use std::collections::{BTreeSet, HashMap};
use std::time::{Duration, Instant};

/// How close two must come to meet, in pixels.
pub const NEAR: f32 = 24.0;
/// How much of each side of a picture is left out, as bumping does.
const SIDE_INSET: f32 = 0.15;
/// The least time between two greetings of one pair…
pub const PAIR_EVERY: Duration = Duration::from_secs(120);
/// …and between any two greetings at all.
pub const ANY_EVERY: Duration = Duration::from_secs(12);
/// How long the two stand still.
pub const STAND_STILL: Duration = Duration::from_millis(2500);
/// When the second answers.
const REPLY_AFTER: Duration = Duration::from_millis(900);

/// Who is near whom, and who greeted when.
#[derive(Debug, Default)]
pub struct Greetings {
    /// Whether `near` has been taken once: the pairs near each other then
    /// were not meeting, only there.
    primed: bool,
    near: BTreeSet<(String, String)>,
    last_pair: HashMap<(String, String), Instant>,
    last_any: Option<Instant>,
    /// Answers to come: who, when, and what they heard.
    replies: Vec<(String, Instant, Option<String>)>,
}

impl Greetings {
    /// Forget who was near whom: the scene changed under them.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// The pairs near each other now, by id, the smaller first.
fn near_pairs(scene: &Scene) -> BTreeSet<(String, String)> {
    let shapes: Vec<(usize, crate::input::arrange::Rect)> = (0..scene.entities.len())
        .filter(|&i| scene.effective_visible(i) && !scene.entities[i].dragging)
        .filter_map(|i| {
            let (l, t, r, b) = scene.seen_rect(i)?;
            let inset = (r - l) * SIDE_INSET;
            Some((
                i,
                (
                    l + inset - NEAR / 2.0,
                    t - NEAR / 2.0,
                    r - inset + NEAR / 2.0,
                    b + NEAR / 2.0,
                ),
            ))
        })
        .collect();
    let mut pairs = BTreeSet::new();
    for (n, &(i, a)) in shapes.iter().enumerate() {
        for &(j, b) in &shapes[n + 1..] {
            let overlap = a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3;
            if overlap {
                let (x, y) = (&scene.entities[i].id, &scene.entities[j].id);
                pairs.insert(if x < y {
                    (x.clone(), y.clone())
                } else {
                    (y.clone(), x.clone())
                });
            }
        }
    }
    pairs
}

/// One step: answers that are due, and greetings for pairs that just met.
/// `Scene::tick` calls it while the scene runs, with greetings on.
pub(crate) fn step(scene: &mut Scene, now: Instant) {
    let due: Vec<(String, Option<String>)> = {
        let replies = &mut scene.greetings.replies;
        let (due, later): (Vec<_>, Vec<_>) = replies.drain(..).partition(|(_, at, _)| *at <= now);
        *replies = later;
        due.into_iter().map(|(id, _, heard)| (id, heard)).collect()
    };
    for (id, heard) in due {
        if let Some(i) = scene.entities.iter().position(|e| e.id == id) {
            if scene.effective_visible(i) && !speaking(scene, i, now) {
                scene.entities[i].greet(now, heard.as_deref());
            }
        }
    }

    let near = near_pairs(scene);
    let met: Vec<(String, String)> = if scene.greetings.primed {
        near.difference(&scene.greetings.near).cloned().collect()
    } else {
        Vec::new()
    };
    scene.greetings.near = near;
    scene.greetings.primed = true;

    for pair in met {
        let g = &scene.greetings;
        if g.last_any
            .is_some_and(|at| now.duration_since(at) < ANY_EVERY)
            || g.last_pair
                .get(&pair)
                .is_some_and(|at| now.duration_since(*at) < PAIR_EVERY)
        {
            continue;
        }
        let find = |id: &str| scene.entities.iter().position(|e| e.id == id);
        let (Some(a), Some(b)) = (find(&pair.0), find(&pair.1)) else {
            continue;
        };
        if speaking(scene, a, now) || speaking(scene, b, now) {
            continue;
        }
        tracing::debug!(
            "{} and {} meet",
            scene.entities[a].name,
            scene.entities[b].name
        );
        let said = scene.entities[a].greet(now, None);
        for i in [a, b] {
            scene.entities[i].still_until = Some(now + STAND_STILL);
        }
        scene.heart_between(a, b);
        let g = &mut scene.greetings;
        g.replies.push((pair.1.clone(), now + REPLY_AFTER, said));
        g.last_pair.insert(pair, now);
        g.last_any = Some(now);
    }
}

fn speaking(scene: &Scene, i: usize, now: Instant) -> bool {
    scene.entities[i]
        .speech
        .as_ref()
        .is_some_and(|s| s.showing(now))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, CharacterConfig};

    fn scene_at(spots: &[(&str, f32)]) -> Scene {
        let mut scene = Scene::from_config(&AppConfig::default());
        let template = AppConfig::default().characters.remove(0);
        let configs: Vec<_> = spots
            .iter()
            .map(|&(id, x)| CharacterConfig {
                id: id.into(),
                name: id.into(),
                asset_path: String::new(),
                x,
                y: 500.0,
                physics_enabled: false,
                behavior: crate::behavior::Behavior::Idle,
                ..template.clone()
            })
            .collect();
        scene.restore_configs(&configs);
        scene
    }

    fn said(scene: &Scene, id: &str) -> bool {
        scene
            .entities
            .iter()
            .find(|e| e.id == id)
            .is_some_and(|e| e.speech.is_some())
    }

    #[test]
    fn two_who_come_together_say_hello_one_after_the_other() {
        let mut scene = scene_at(&[("a", 100.0), ("b", 600.0)]);
        let t0 = Instant::now();
        step(&mut scene, t0);
        assert!(!said(&scene, "a") && !said(&scene, "b"), "apart");
        scene.entities[1].x = 100.0 + scene.entities[0].scaled_width();
        step(&mut scene, t0 + Duration::from_millis(100));
        assert!(said(&scene, "a"), "the first says hello");
        assert!(!said(&scene, "b"), "the other answers in a moment");
        assert!(scene.entities.iter().all(|e| e.still_until.is_some()));
        step(&mut scene, t0 + Duration::from_secs(1));
        assert!(said(&scene, "b"), "and answers");
        let line = |i: usize| scene.entities[i].speech.as_ref().unwrap().text.clone();
        assert_ne!(line(0), line(1), "with a hello of its own");
    }

    #[test]
    fn those_already_together_and_those_staying_together_say_nothing() {
        let mut scene = scene_at(&[("a", 100.0), ("b", 140.0)]);
        let t0 = Instant::now();
        step(&mut scene, t0);
        step(&mut scene, t0 + Duration::from_secs(1));
        assert!(!said(&scene, "a") && !said(&scene, "b"));
    }

    #[test]
    fn they_stand_still_a_moment_then_go_on() {
        let mut scene = scene_at(&[("a", 100.0)]);
        scene.entities[0].behavior = crate::behavior::Behavior::WalkAround { speed: 200.0 };
        scene.entities[0].still_until = Some(Instant::now() + Duration::from_millis(200));
        let bounds = crate::monitor::DesktopBounds::from_size(1600.0, 1000.0);
        let x = scene.entities[0].x;
        std::thread::sleep(Duration::from_millis(20));
        scene.tick(bounds, None, None);
        assert_eq!(scene.entities[0].x, x, "standing still");
        std::thread::sleep(Duration::from_millis(220));
        scene.tick(bounds, None, None);
        assert_ne!(scene.entities[0].x, x, "walking again");
        assert!(scene.entities[0].still_until.is_none());
    }

    #[test]
    fn a_pair_does_not_greet_again_soon() {
        let mut scene = scene_at(&[("a", 100.0), ("b", 600.0)]);
        let t0 = Instant::now();
        step(&mut scene, t0);
        let close = 100.0 + scene.entities[0].scaled_width();
        scene.entities[1].x = close;
        step(&mut scene, t0);
        for e in &mut scene.entities {
            e.speech = None;
        }
        scene.greetings.replies.clear();
        // Apart, and together again a minute later.
        scene.entities[1].x = 600.0;
        step(&mut scene, t0 + Duration::from_secs(30));
        scene.entities[1].x = close;
        step(&mut scene, t0 + Duration::from_secs(60));
        assert!(!said(&scene, "a"), "too soon for this pair");
        scene.entities[1].x = 600.0;
        step(&mut scene, t0 + Duration::from_secs(100));
        scene.entities[1].x = close;
        step(&mut scene, t0 + Duration::from_secs(130));
        assert!(said(&scene, "a"), "later, again");
    }
}
