//! Moods by the time of day (1.5, off by default: `moods`): the characters
//! are livelier in the morning and slower in the evening — they walk and
//! play their animations faster or slower by the clock — and, with
//! bedtime set, each yawns once in the half hour before it.
//!
//! The pace is a smooth curve over the day ([`tempo_at`]); each entity's
//! runtime `tempo` takes it (`Entity::tick` scales walking and the
//! animation's frame rate by it). The clock is read every ten seconds.

use crate::scene::Scene;
use std::time::{Duration, Instant};

/// The pace through the day: (minute of the day, tempo), straight lines
/// between, round past midnight.
const CURVE: [(u32, f32); 7] = [
    (5 * 60, 0.75),
    (8 * 60, 1.2),
    (11 * 60, 1.2),
    (13 * 60, 1.0),
    (18 * 60, 1.0),
    (21 * 60, 0.85),
    (23 * 60, 0.7),
];

/// How long before bedtime the yawning starts.
const YAWN_BEFORE: u32 = 30;

/// How long a yawn is shown.
const YAWN_SECONDS: f32 = 4.0;

/// The pace at `minute` of the day: about 1.2 in the morning, 1 in the
/// day, down to 0.7 at night.
pub fn tempo_at(minute: u32) -> f32 {
    let minute = minute % (24 * 60);
    let day = 24 * 60;
    for i in 0..CURVE.len() {
        let (a, ta) = CURVE[i];
        let (b, tb) = CURVE[(i + 1) % CURVE.len()];
        let b = if b <= a { b + day } else { b };
        let m = if minute < a { minute + day } else { minute };
        if (a..b).contains(&m) {
            let t = (m - a) as f32 / (b - a) as f32;
            return ta + (tb - ta) * t;
        }
    }
    1.0
}

/// When, in minutes past the start of the yawning, the character `id`
/// yawns: its own moment, the same every night.
fn yawn_offset(id: &str) -> u32 {
    let hash = id.bytes().fold(2166136261u32, |h, b| {
        (h ^ u32::from(b)).wrapping_mul(16777619)
    });
    hash % (YAWN_BEFORE - 5)
}

/// The moods' state: the clock, and who yawned on which night.
#[derive(Debug, Default)]
pub struct Moods {
    clock: Option<(Instant, Option<crate::schedule::LocalTime>)>,
    yawned: Vec<(String, (u8, u8))>,
    /// Whether the tempos are other than 1 (so turning off resets them once).
    applied: bool,
}

impl Moods {
    /// One step, every frame: set the pace by the clock, and the yawns.
    /// `shown`: whether a yawn can be seen.
    pub fn step(
        &mut self,
        scene: &mut Scene,
        enabled: bool,
        bedtime: &crate::bedtime::Bedtime,
        shown: bool,
    ) {
        if !enabled {
            if self.applied {
                for e in &mut scene.entities {
                    e.tempo = 1.0;
                }
                self.applied = false;
            }
            return;
        }
        let now = Instant::now();
        let clock = match self.clock {
            Some((at, clock)) if now.duration_since(at) < Duration::from_secs(10) => clock,
            _ => {
                let clock = crate::schedule::local_now();
                self.clock = Some((now, clock));
                clock
            }
        };
        let Some(clock) = clock else {
            return;
        };
        let tempo = tempo_at(clock.minute);
        for e in &mut scene.entities {
            e.tempo = tempo;
        }
        self.applied = true;
        if shown && bedtime.enabled && !scene.is_dozing() {
            self.yawn(scene, clock, bedtime);
        }
    }

    /// Each visible character yawns once in the half hour before bedtime,
    /// at its own moment.
    fn yawn(
        &mut self,
        scene: &mut Scene,
        clock: crate::schedule::LocalTime,
        bedtime: &crate::bedtime::Bedtime,
    ) {
        let Some(from) = crate::schedule::parse_hhmm(&bedtime.from) else {
            return;
        };
        let day = 24 * 60;
        let start = (from + day - YAWN_BEFORE) % day;
        // Minutes since the yawning started, if it has and bedtime has not.
        let since = (clock.minute + day - start) % day;
        if since >= YAWN_BEFORE {
            return;
        }
        let night = (clock.month, clock.day);
        self.yawned.retain(|(_, on)| *on == night);
        for i in 0..scene.entities.len() {
            if !scene.effective_visible(i) {
                continue;
            }
            let id = scene.entities[i].id.clone();
            if since < yawn_offset(&id) || self.yawned.iter().any(|(who, _)| *who == id) {
                continue;
            }
            if scene.entities[i]
                .speech
                .as_ref()
                .is_some_and(|s| s.showing(Instant::now()))
            {
                continue;
            }
            scene.entities[i].say(&crate::i18n::t("mood-yawn"), YAWN_SECONDS);
            self.yawned.push((id, night));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn livelier_in_the_morning_slower_at_night() {
        assert!((tempo_at(9 * 60) - 1.2).abs() < 0.001);
        assert!((tempo_at(15 * 60) - 1.0).abs() < 0.001);
        assert!(tempo_at(22 * 60) < 0.85 && tempo_at(22 * 60) > 0.7);
        assert!(
            (tempo_at(2 * 60) - 0.725).abs() < 0.01,
            "{}",
            tempo_at(2 * 60)
        );
        // No jumps: a minute on, nearly the same.
        for m in 0..24 * 60 {
            assert!((tempo_at(m) - tempo_at(m + 1)).abs() < 0.01, "at {m}");
        }
    }

    #[test]
    fn each_yawns_once_before_bedtime() {
        let mut scene = Scene::from_config(&crate::config::AppConfig::default());
        let bedtime = crate::bedtime::Bedtime {
            enabled: true,
            from: "23:00".into(),
            to: "07:00".into(),
        };
        let mut moods = Moods::default();
        let at = |minute| crate::schedule::LocalTime {
            weekday: 1,
            minute,
            day: 6,
            month: 10,
        };
        // Before the half hour: nobody.
        moods.yawn(&mut scene, at(22 * 60 + 20), &bedtime);
        assert!(scene.entities.iter().all(|e| e.speech.is_none()));
        // Through it: everyone, once.
        for m in 22 * 60 + 30..23 * 60 {
            moods.yawn(&mut scene, at(m), &bedtime);
        }
        assert_eq!(moods.yawned.len(), scene.entities.len());
        for e in &mut scene.entities {
            e.speech = None;
        }
        moods.yawn(&mut scene, at(22 * 60 + 59), &bedtime);
        assert!(
            scene.entities.iter().all(|e| e.speech.is_none()),
            "once a night"
        );
    }

    #[test]
    fn off_puts_the_pace_back() {
        let mut scene = Scene::from_config(&crate::config::AppConfig::default());
        let mut moods = Moods::default();
        let bedtime = crate::bedtime::Bedtime::default();
        moods.step(&mut scene, true, &bedtime, true);
        if crate::schedule::local_now().is_some() {
            assert!(moods.applied);
        }
        moods.step(&mut scene, false, &bedtime, true);
        assert!(scene.entities.iter().all(|e| e.tempo == 1.0));
    }
}
