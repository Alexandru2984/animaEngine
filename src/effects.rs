//! Little effects (1.5): a few hearts rising from a character that was
//! tapped, a puff of dust where one lands hard, and z's floating up as the
//! characters fall asleep (`crate::doze`). Drawn with the speech bubbles
//! (`crate::ui::effects`) and click-through like them; none with Reduce
//! motion, or with `little_effects` off.
//!
//! Hearts go along with the character they rise from — a tap makes it
//! hop, and they would be left at its feet — the rest stay where they
//! began, in the scene's coordinates. All run on the clock: the loops
//! keep drawing while one is alive (`crate::pacing`) — a still or
//! sleeping scene too — and stop after.

use std::time::{Duration, Instant};

/// At most this many at once: a burst of taps drops the oldest.
const MAX: usize = 96;

/// Landings slower than this raise no dust: a poke's hop comes down
/// softly, a fall from a few heads up does not.
pub const DUST_SPEED: f32 = 260.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Heart,
    Dust,
    Zzz,
}

#[derive(Debug, Clone)]
struct Effect {
    kind: Kind,
    /// Where it began, in the scene's coordinates.
    at: (f32, f32),
    /// The character it goes along with, by id, and the middle of its top
    /// edge then.
    with: Option<(String, (f32, f32))>,
    start: Instant,
    life: Duration,
    /// Which way it drifts, -1 (left) to 1 (right).
    drift: f32,
    /// How big, about 0.5 to 1.
    strength: f32,
}

/// One effect as drawn on a surface this frame, in its pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drawn {
    pub kind: Kind,
    pub at: (f32, f32),
    /// A heart's half width, a puff's radius, a z's height.
    pub size: f32,
    /// 0 to 1.
    pub alpha: f32,
}

/// The effects running now.
#[derive(Debug, Default)]
pub struct Effects {
    list: Vec<Effect>,
}

impl Effects {
    fn add(&mut self, effect: Effect) {
        if self.list.len() >= MAX {
            self.list.remove(0);
        }
        self.list.push(effect);
    }

    /// Three hearts rising from `top`, the middle of the top edge of the
    /// character `id`, and going along with it.
    pub fn hearts(&mut self, id: &str, top: (f32, f32), now: Instant) {
        for (i, drift) in [0.0, -1.0, 1.0].into_iter().enumerate() {
            self.add(Effect {
                kind: Kind::Heart,
                at: (top.0 + drift * 10.0, top.1),
                with: Some((id.to_string(), top)),
                start: now + Duration::from_millis(110 * i as u64),
                life: Duration::from_millis(1100),
                drift,
                strength: if i == 0 { 1.0 } else { 0.8 },
            });
        }
    }

    /// A puff of dust either side of `feet`, the middle of a character's
    /// bottom edge, for a landing at `speed` pixels a second.
    pub fn dust(&mut self, feet: (f32, f32), speed: f32, now: Instant) {
        let strength = (speed / 600.0).clamp(0.5, 1.0);
        for drift in [-1.0, -0.6, -0.25, 0.25, 0.6, 1.0] {
            self.add(Effect {
                kind: Kind::Dust,
                at: feet,
                with: None,
                start: now,
                life: Duration::from_millis(600),
                drift,
                strength,
            });
        }
    }

    /// Three z's floating up, one after another, from `head`.
    pub fn zzz(&mut self, head: (f32, f32), now: Instant) {
        for i in 0..3u32 {
            self.add(Effect {
                kind: Kind::Zzz,
                at: head,
                with: None,
                start: now + Duration::from_millis(600 * u64::from(i)),
                life: Duration::from_millis(2200),
                drift: 1.0,
                strength: 0.7 + 0.15 * i as f32,
            });
        }
    }

    /// Whether any is still to be drawn at `now`, or yet to begin.
    pub fn alive(&self, now: Instant) -> bool {
        self.list.iter().any(|e| now < e.start + e.life)
    }

    /// Forget the ones that are over.
    pub fn prune(&mut self, now: Instant) {
        self.list.retain(|e| now < e.start + e.life);
    }

    /// Forget them all: the setting went off, the scene was replaced.
    pub fn clear(&mut self) {
        self.list.clear();
    }

    /// The effects on a surface whose top-left is at `origin` in the
    /// scene's coordinates and whose size is `size`, as drawn at `now`, in
    /// its pixels. `top_of` gives the middle of a character's top edge now,
    /// by id: what goes along with one moves as it has; with it gone, it
    /// stays where it was.
    pub fn shown(
        &self,
        origin: (f32, f32),
        size: (f32, f32),
        now: Instant,
        top_of: impl Fn(&str) -> Option<(f32, f32)>,
    ) -> Vec<Drawn> {
        const MARGIN: f32 = 40.0;
        self.list
            .iter()
            .filter_map(|e| {
                let p = now.checked_duration_since(e.start)?.as_secs_f32()
                    / e.life.as_secs_f32().max(0.001);
                if p >= 1.0 {
                    return None;
                }
                let moved = e
                    .with
                    .as_ref()
                    .and_then(|(id, then)| top_of(id).map(|t| (t.0 - then.0, t.1 - then.1)))
                    .unwrap_or((0.0, 0.0));
                let mut drawn = e.drawn(p);
                drawn.at = (drawn.at.0 + moved.0, drawn.at.1 + moved.1);
                let at = (drawn.at.0 - origin.0, drawn.at.1 - origin.1);
                let on = at.0 > -MARGIN
                    && at.1 > -MARGIN
                    && at.0 < size.0 + MARGIN
                    && at.1 < size.1 + MARGIN;
                on.then_some(Drawn { at, ..drawn })
            })
            .collect()
    }
}

fn ease_out(p: f32) -> f32 {
    1.0 - (1.0 - p) * (1.0 - p)
}

/// Fully there until `from`, then fading to nothing at the end.
fn fade_after(p: f32, from: f32) -> f32 {
    if p < from {
        1.0
    } else {
        ((1.0 - p) / (1.0 - from)).clamp(0.0, 1.0)
    }
}

impl Effect {
    /// Where it is, how big and how clear, `p` of the way through.
    fn drawn(&self, p: f32) -> Drawn {
        let (x, y) = self.at;
        match self.kind {
            Kind::Heart => {
                let sway =
                    (p * std::f32::consts::TAU).sin() * 4.0 + self.drift * 16.0 * ease_out(p);
                // Pops to full size in the first moments.
                let grow = 0.5 + 0.5 * (p / 0.15).min(1.0);
                Drawn {
                    kind: self.kind,
                    at: (x + sway, y - 6.0 - 48.0 * ease_out(p)),
                    size: 7.0 * self.strength * grow,
                    alpha: fade_after(p, 0.6),
                }
            }
            Kind::Dust => {
                let out = ease_out(p);
                let size = (3.0 + 6.0 * out) * self.strength;
                Drawn {
                    kind: self.kind,
                    // Sitting on the floor as it spreads, and lifting a little.
                    at: (
                        x + self.drift * 34.0 * self.strength * out,
                        y - size - 6.0 * out * (1.0 - 0.5 * self.drift.abs()),
                    ),
                    size,
                    alpha: 0.55 * (1.0 - p),
                }
            }
            Kind::Zzz => Drawn {
                kind: self.kind,
                at: (
                    x + (p * 3.0).sin() * 6.0 + self.drift * 14.0 * p,
                    y - 40.0 * p,
                ),
                size: (11.0 + 8.0 * p) * self.strength,
                alpha: (p / 0.15).min(1.0) * fade_after(p, 0.6),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hearts_rise_and_are_gone_in_a_moment() {
        let now = Instant::now();
        let mut fx = Effects::default();
        fx.hearts("a", (500.0, 300.0), now);
        let at = |ms: u64| {
            fx.shown(
                (0.0, 0.0),
                (1000.0, 1000.0),
                now + Duration::from_millis(ms),
                |_| None,
            )
        };
        assert_eq!(at(0).len(), 1, "the others start a little later");
        let first = at(50)[0];
        let later = at(600)[0];
        assert!(later.at.1 < first.at.1, "rising: {first:?} → {later:?}");
        assert_eq!(at(400).len(), 3);
        assert!(fx.alive(now + Duration::from_millis(1200)));
        assert!(!fx.alive(now + Duration::from_secs(2)));
        fx.prune(now + Duration::from_secs(2));
        assert!(fx.list.is_empty());
    }

    #[test]
    fn a_surface_shows_its_own_in_its_pixels() {
        let now = Instant::now();
        let mut fx = Effects::default();
        fx.dust((1700.0, 900.0), 500.0, now);
        let soon = now + Duration::from_millis(100);
        let none = |_: &str| None;
        assert!(
            fx.shown((0.0, 0.0), (1600.0, 1000.0), soon, none)
                .is_empty(),
            "on the other monitor"
        );
        let there = fx.shown((1600.0, 0.0), (1600.0, 1000.0), soon, none);
        assert_eq!(there.len(), 6);
        assert!(there
            .iter()
            .all(|d| (d.at.0 - 100.0).abs() < 40.0 && d.alpha > 0.0));
    }

    fn one() -> crate::scene::Scene {
        use crate::config::{AppConfig, CharacterConfig};
        let mut scene = crate::scene::Scene::from_config(&AppConfig::default());
        let template = AppConfig::default().characters.remove(0);
        scene.restore_configs(&[CharacterConfig {
            id: "a".into(),
            name: "a".into(),
            asset_path: String::new(),
            x: 100.0,
            y: 100.0,
            physics_enabled: false,
            behavior: crate::behavior::Behavior::Idle,
            ..template
        }]);
        scene
    }

    fn step(scene: &mut crate::scene::Scene) {
        std::thread::sleep(Duration::from_millis(20));
        let bounds = crate::monitor::DesktopBounds::from_size(1600.0, 1000.0);
        scene.tick(bounds, None, None);
    }

    /// The character an instant above the floor, coming down at `speed`.
    fn about_to_land(scene: &mut crate::scene::Scene, speed: f32) {
        let floor = 1000.0 - scene.entities[0].scaled_height();
        let e = &mut scene.entities[0];
        e.physics.enabled = true;
        e.physics.grounded = false;
        e.physics.velocity_y = speed;
        e.y = floor - 1.0;
    }

    #[test]
    fn a_hard_landing_raises_dust_and_a_soft_one_none() {
        let mut scene = one();
        about_to_land(&mut scene, 500.0);
        step(&mut scene);
        assert!(scene.effects.alive(Instant::now()), "dust");
        scene.effects.clear();
        about_to_land(&mut scene, 100.0);
        step(&mut scene);
        assert!(!scene.effects.alive(Instant::now()), "a hop's landing");
        scene.set_reduced_motion(true);
        about_to_land(&mut scene, 500.0);
        step(&mut scene);
        assert!(!scene.effects.alive(Instant::now()), "Reduce motion");
    }

    #[test]
    fn z_s_as_they_fall_asleep_and_hearts_on_a_tap() {
        let mut scene = one();
        scene.set_dozing(true);
        step(&mut scene);
        assert!(!scene.is_running(), "asleep");
        assert!(scene.effects.alive(Instant::now()), "z's");
        scene.set_little_effects(false);
        assert!(
            !scene.effects.alive(Instant::now()),
            "gone with the setting"
        );
        scene.hearts_for(0);
        assert!(!scene.effects.alive(Instant::now()));
        scene.set_little_effects(true);
        scene.hearts_for(0);
        assert!(scene.effects.alive(Instant::now()), "hearts");
    }

    #[test]
    fn hearts_go_along_with_their_character() {
        let now = Instant::now();
        let mut fx = Effects::default();
        fx.hearts("a", (500.0, 300.0), now);
        let soon = now + Duration::from_millis(50);
        let still = fx.shown((0.0, 0.0), (1000.0, 1000.0), soon, |_| Some((500.0, 300.0)))[0];
        let hopped = fx.shown((0.0, 0.0), (1000.0, 1000.0), soon, |_| Some((500.0, 220.0)))[0];
        assert!(
            (still.at.1 - hopped.at.1 - 80.0).abs() < 0.01,
            "{still:?} {hopped:?}"
        );
        let gone = fx.shown((0.0, 0.0), (1000.0, 1000.0), soon, |_| None)[0];
        assert_eq!(gone.at, still.at, "left where it was");
    }

    #[test]
    fn a_burst_keeps_the_newest() {
        let now = Instant::now();
        let mut fx = Effects::default();
        for _ in 0..100 {
            fx.dust((0.0, 0.0), 300.0, now);
        }
        assert_eq!(fx.list.len(), MAX);
    }
}
