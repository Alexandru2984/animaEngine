//! Playing tag (1.5): a character whose behavior is `PlayTag` plays with
//! its partner — whoever is "it" runs after the other, which runs away a
//! little slower; a catch makes both stop for a breath, and they swap,
//! the new runner with a head start. After [`PLAY_FOR`] they calm down for
//! [`REST_FOR`], then play again.
//!
//! The partner needs no setting of its own: for a game it is taken from
//! what it was doing (`Entity::game_move` stands in for its behavior) and
//! given back between games. Each character is in one game at most; one
//! out of sight, or gone, ends it. Not to be confused with `crate::tap`,
//! which is about the user's taps.

use crate::scene::Scene;
use std::time::{Duration, Instant};

/// How long a game lasts…
pub const PLAY_FOR: Duration = Duration::from_secs(20);
/// …and the rest between two.
pub const REST_FOR: Duration = Duration::from_secs(90);
/// Both stand still this long after a catch…
const CAUGHT_PAUSE: Duration = Duration::from_millis(700);
/// …and the new runner gets this long besides before it can be caught.
const HEAD_START: Duration = Duration::from_millis(1500);
/// The runner's speed, as a share of the chaser's: it is caught, in the end.
const RUNNER: f32 = 0.8;
/// Caught when the middles come this close, as a share of their half
/// widths added.
const REACH: f32 = 0.6;

/// Where a character in a game goes this tick, instead of its behavior.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Move {
    /// Toward this point — or away from it.
    pub point: (f32, f32),
    pub away: bool,
    /// Pixels a second.
    pub speed: f32,
    /// For a runner that flies from a chaser held to the ground: the
    /// height (of its middle) to come down or up to as it runs, instead of
    /// fleeing up and down too — the chaser could never reach it there.
    pub level: Option<f32>,
}

impl Move {
    /// Move a `w`×`h` character at (`x`, `y`) for `dt` seconds within
    /// `bounds`; across only when physics holds it to the ground.
    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        &self,
        x: &mut f32,
        y: &mut f32,
        w: f32,
        h: f32,
        bounds: crate::monitor::DesktopBounds,
        grounded: bool,
        dt: f32,
    ) {
        let (cx, cy) = (*x + w / 2.0, *y + h / 2.0);
        let sign = if self.away { -1.0 } else { 1.0 };
        let dx = (self.point.0 - cx) * sign;
        let dy = match self.level {
            _ if grounded => 0.0,
            Some(level) => level - cy,
            None => (self.point.1 - cy) * sign,
        };
        let gap = (dx * dx + dy * dy).sqrt();
        if gap < 0.5 {
            return;
        }
        let step = self.speed * dt;
        let step = if self.away { step } else { step.min(gap) };
        *x = bounds.clamp_x(*x + dx / gap * step, w);
        if !grounded {
            // Not past the level it is making for.
            let rise = dy / gap * step;
            let rise = if self.level.is_some() {
                rise.clamp(-dy.abs(), dy.abs())
            } else {
                rise
            };
            *y = (*y + rise).clamp(bounds.min_y, (bounds.max_y - h).max(bounds.min_y));
        }
    }
}

#[derive(Debug)]
struct Game {
    player: String,
    partner: String,
    /// Whether the player is "it".
    player_chases: bool,
    playing: bool,
    /// When this game, or this rest, ends.
    until: Instant,
    /// No catch counts before this.
    catchable_from: Instant,
}

/// The games going on.
#[derive(Debug, Default)]
pub struct Games {
    games: Vec<Game>,
}

impl Games {
    /// Forget them: the characters were put where they are, not played.
    pub fn reset(&mut self) {
        self.games.clear();
    }

    /// Whether any is going on, playing or resting.
    pub fn any(&self) -> bool {
        !self.games.is_empty()
    }
}

/// One step of every game: who chases whom this tick (`Entity::game_move`).
/// `Scene::tick` calls it before the characters move.
pub(crate) fn step(scene: &mut Scene, now: Instant) {
    for e in &mut scene.entities {
        e.game_move = None;
    }
    // Who plays with whom: each in one game at most, first come first.
    let mut taken = std::collections::BTreeSet::new();
    let mut pairs = Vec::new();
    for i in 0..scene.entities.len() {
        let crate::behavior::Behavior::PlayTag { partner, speed } = &scene.entities[i].behavior
        else {
            continue;
        };
        let Some(j) = scene.entities.iter().position(|e| e.id == *partner) else {
            continue;
        };
        if i == j
            || !scene.effective_visible(i)
            || !scene.effective_visible(j)
            || taken.contains(&i)
            || taken.contains(&j)
        {
            continue;
        }
        taken.insert(i);
        taken.insert(j);
        pairs.push((i, j, *speed));
    }

    let mut old = std::mem::take(&mut scene.games.games);
    for (i, j, speed) in pairs {
        let (pid, qid) = (scene.entities[i].id.clone(), scene.entities[j].id.clone());
        let mut game = match old.iter().position(|g| g.player == pid && g.partner == qid) {
            Some(k) => old.swap_remove(k),
            None => Game {
                player: pid,
                partner: qid,
                player_chases: true,
                playing: true,
                until: now + PLAY_FOR,
                catchable_from: now,
            },
        };
        if now >= game.until {
            game.playing = !game.playing;
            game.until = now + if game.playing { PLAY_FOR } else { REST_FOR };
            game.catchable_from = now;
        }
        if game.playing {
            play(scene, &mut game, i, j, speed, now);
        }
        scene.games.games.push(game);
    }
}

/// The chase between the player at `i` and its partner at `j`.
fn play(scene: &mut Scene, game: &mut Game, i: usize, j: usize, speed: f32, now: Instant) {
    let middle = |scene: &Scene, k: usize| {
        let e = &scene.entities[k];
        (
            e.x + e.scaled_width() / 2.0,
            e.y + e.scaled_height() / 2.0,
            e.scaled_width() / 2.0,
        )
    };
    let (chaser, runner) = if game.player_chases { (i, j) } else { (j, i) };
    let (c, r) = (middle(scene, chaser), middle(scene, runner));
    let distance = ((c.0 - r.0).powi(2) + (c.1 - r.1).powi(2)).sqrt();
    if now >= game.catchable_from && distance < (c.2 + r.2) * REACH {
        // Caught: a breath, and they swap.
        game.player_chases = !game.player_chases;
        game.catchable_from = now + CAUGHT_PAUSE + HEAD_START;
        for k in [chaser, runner] {
            scene.entities[k].still_until = Some(now + CAUGHT_PAUSE);
        }
        tracing::debug!(
            "{} caught {}",
            scene.entities[chaser].name,
            scene.entities[runner].name
        );
        return;
    }
    scene.entities[chaser].game_move = Some(Move {
        point: (r.0, r.1),
        away: false,
        speed,
        level: None,
    });
    // A chaser held to the ground runs only across: a runner that flies
    // keeps to its height, or it would never be caught — and, "it" from
    // then on, game after game, the chaser would run under it for good.
    let held = |k: usize| scene.entities[k].physics.enabled;
    scene.entities[runner].game_move = Some(Move {
        point: (c.0, c.1),
        away: true,
        speed: speed * RUNNER,
        level: (held(chaser) && !held(runner)).then_some(c.1),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, CharacterConfig};

    fn players() -> Scene {
        let mut scene = Scene::from_config(&AppConfig::default());
        let template = AppConfig::default().characters.remove(0);
        let at = |id: &str, x: f32, behavior| CharacterConfig {
            id: id.into(),
            name: id.into(),
            asset_path: String::new(),
            x,
            y: 400.0,
            physics_enabled: false,
            behavior,
            ..template.clone()
        };
        scene.restore_configs(&[
            at(
                "cat",
                100.0,
                crate::behavior::Behavior::PlayTag {
                    partner: "slime".into(),
                    speed: 300.0,
                },
            ),
            at(
                "slime",
                900.0,
                crate::behavior::Behavior::WalkAround { speed: 60.0 },
            ),
        ]);
        scene
    }

    #[test]
    fn it_chases_the_other_runs() {
        let mut scene = players();
        let now = Instant::now();
        step(&mut scene, now);
        let chase = scene.entities[0].game_move.unwrap();
        let run = scene.entities[1].game_move.unwrap();
        assert!(!chase.away && run.away);
        assert!(run.speed < chase.speed, "the runner is caught in the end");
    }

    #[test]
    fn a_catch_swaps_them_after_a_breath() {
        let mut scene = players();
        let now = Instant::now();
        step(&mut scene, now);
        scene.entities[1].x = scene.entities[0].x + 5.0;
        step(&mut scene, now + Duration::from_millis(10));
        assert!(
            scene.entities.iter().all(|e| e.still_until.is_some()),
            "a breath"
        );
        assert!(!scene.games.games[0].player_chases, "the slime is it");
        // Right beside each other, the new runner cannot be caught at once.
        step(&mut scene, now + Duration::from_millis(900));
        assert!(!scene.games.games[0].player_chases);
        assert!(scene.entities[1].game_move.is_some_and(|m| !m.away));
    }

    #[test]
    fn they_calm_down_and_play_again() {
        let mut scene = players();
        let now = Instant::now();
        step(&mut scene, now);
        step(&mut scene, now + PLAY_FOR);
        assert!(
            scene.entities.iter().all(|e| e.game_move.is_none()),
            "resting"
        );
        step(&mut scene, now + PLAY_FOR + REST_FOR);
        assert!(scene.entities[0].game_move.is_some(), "playing again");
    }

    #[test]
    fn a_partner_out_of_sight_ends_the_game() {
        let mut scene = players();
        let now = Instant::now();
        step(&mut scene, now);
        scene.entities[1].visible = false;
        scene.mark_visible_dirty();
        step(&mut scene, now);
        assert!(scene.entities.iter().all(|e| e.game_move.is_none()));
        assert!(!scene.games.any());
    }

    #[test]
    fn it_held_to_the_ground_still_catches_one_that_flies() {
        let mut scene = players();
        let bounds = crate::monitor::DesktopBounds::from_size(1600.0, 1000.0);
        // The slime walks the floor; the cat flies, high up.
        scene.entities[1].physics.enabled = true;
        scene.entities[1].y = 800.0;
        scene.entities[0].y = 100.0;
        let t0 = Instant::now();
        step(&mut scene, t0);
        scene.games.games[0].player_chases = false;
        let dt = 1.0 / 60.0;
        let mut caught = false;
        for tick in 1..(PLAY_FOR.as_secs() as u32 * 60) {
            step(&mut scene, t0 + Duration::from_secs_f32(tick as f32 * dt));
            if scene.games.games[0].player_chases {
                caught = true;
                break;
            }
            for e in &mut scene.entities {
                let (w, h, grounded) = (e.scaled_width(), e.scaled_height(), e.physics.enabled);
                if let Some(m) = e.game_move {
                    m.apply(&mut e.x, &mut e.y, w, h, bounds, grounded, dt);
                }
            }
        }
        assert!(caught, "the cat came down to run, and was caught");
    }

    #[test]
    fn a_runner_moves_away_and_a_chaser_toward() {
        let bounds = crate::monitor::DesktopBounds::from_size(1600.0, 1000.0);
        let (mut x, mut y) = (500.0, 400.0);
        Move {
            point: (900.0, 450.0),
            away: false,
            speed: 100.0,
            level: None,
        }
        .apply(&mut x, &mut y, 100.0, 100.0, bounds, true, 0.5);
        assert!(
            (x - 550.0).abs() < 0.1 && y == 400.0,
            "across only, on the ground"
        );
        Move {
            point: (900.0, 450.0),
            away: true,
            speed: 100.0,
            level: None,
        }
        .apply(&mut x, &mut y, 100.0, 100.0, bounds, true, 0.5);
        assert!((x - 500.0).abs() < 0.1);
    }
}
