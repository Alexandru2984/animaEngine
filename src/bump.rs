//! Characters bumping into each other (1.5, off by default): with it on,
//! the tops of other characters are floors — a falling one lands on
//! another's head, through the same platform rule window-awareness uses
//! (`crate::platforms`) — and a walker turns around when it meets one.
//!
//! Sprites carry transparent margins, so the sides count a little inside
//! the drawn rectangle; the top is the rectangle's. Characters being
//! dragged are nobody's floor and turn nobody back.

use crate::input::arrange::{drawn_rect, Rect};
use crate::platforms::PlatformRect;
use crate::scene::Scene;

/// How much of each side of a sprite is left out when two meet.
const SIDE_INSET: f32 = 0.15;
/// How much two must overlap vertically, as a share of the shorter, to
/// meet side by side — so one standing on another's head does not.
const MIN_VERTICAL_OVERLAP: f32 = 0.3;

/// Where each character is solid this tick: drawn, visible and not held.
pub fn solids(scene: &Scene) -> Vec<Option<Rect>> {
    (0..scene.entities.len())
        .map(|i| {
            let e = &scene.entities[i];
            (scene.effective_visible(i) && !e.dragging)
                .then(|| drawn_rect(scene, i))
                .flatten()
        })
        .collect()
}

/// The platforms for the character at `me`: the windows' and every other
/// solid character's top, in `me`'s own coordinates — the drawn ones less
/// its group's offset, the space its physics works in.
///
/// A character lands on what can be seen of the other — its topmost
/// visible row, not the top of a rectangle with empty rows above the
/// head — and with what can be seen of itself: the platform is lowered by
/// its own empty rows under the feet. Measured on the whole rectangles, a
/// heart dropped on the cat stopped a hand's width above its ears.
pub fn platforms_for(
    scene: &Scene,
    solids: &[Option<Rect>],
    me: usize,
    windows: &[PlatformRect],
) -> Vec<PlatformRect> {
    let Some(me_entity) = scene.entities.get(me) else {
        return windows.to_vec();
    };
    let (gx, gy, _) = crate::group::transform_for_member(&scene.groups, &me_entity.id);
    // Physics works in the entity's own scale, without its group's.
    let under_feet = margins(me_entity).1 * me_entity.scale;
    let mut out = windows.to_vec();
    for (i, rect) in solids.iter().enumerate() {
        let (true, Some((l, t, r, b))) = (i != me, rect) else {
            continue;
        };
        let other = &scene.entities[i];
        let (_, _, gscale) = crate::group::transform_for_member(&scene.groups, &other.id);
        let above_head = margins(other).0 * other.scale * gscale;
        let inset = (r - l) * SIDE_INSET;
        out.push(PlatformRect {
            x: l + inset - gx,
            y: t + above_head + under_feet - gy,
            w: (r - l) - 2.0 * inset,
            h: b - t,
        });
    }
    out
}

/// The empty rows at the top and the bottom of the entity's animation, in
/// frame pixels — steady over the loop, so the floor it makes holds still
/// (`Animation::steady_margins`).
fn margins(entity: &crate::entity::Entity) -> (f32, f32) {
    entity.animation().steady_margins()
}

/// Turn every walker heading into another solid character.
pub fn turn_walkers(scene: &mut Scene) {
    let solids = solids(scene);
    for i in 0..scene.entities.len() {
        if !matches!(
            scene.entities[i].behavior,
            crate::behavior::Behavior::WalkAround { .. }
        ) {
            continue;
        }
        let Some(me) = solids[i] else {
            continue;
        };
        let direction = scene.entities[i].behavior_state.walk_direction;
        let blocked = solids
            .iter()
            .enumerate()
            .any(|(j, other)| j != i && other.is_some_and(|other| meets(me, other, direction)));
        if blocked {
            scene.entities[i].behavior_state.walk_direction = -direction;
        }
    }
}

/// Whether a character at `me`, walking in `direction`, runs into one at
/// `other`: their insides overlap side by side, and `other` is ahead.
fn meets(me: Rect, other: Rect, direction: f32) -> bool {
    let inside = |(l, t, r, b): Rect| {
        let inset = (r - l) * SIDE_INSET;
        (l + inset, t, r - inset, b)
    };
    let (a, b) = (inside(me), inside(other));
    let across = a.2.min(b.2) - a.0.max(b.0);
    let down = a.3.min(b.3) - a.1.max(b.1);
    let shorter = (a.3 - a.1).min(b.3 - b.1);
    let ahead = if direction > 0.0 {
        b.0 + b.2 > a.0 + a.2
    } else {
        b.0 + b.2 < a.0 + a.2
    };
    across > 0.0 && down > shorter * MIN_VERTICAL_OVERLAP && ahead
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walkers_meet_side_by_side_and_only_ahead() {
        let me = (100.0, 100.0, 200.0, 200.0);
        // Overlapping by 40 px: 10 px more than the two margins.
        let right = (160.0, 110.0, 260.0, 210.0);
        assert!(meets(me, right, 1.0), "walking into it");
        assert!(!meets(me, right, -1.0), "walking away from it");
        // Just touching boxes: the transparent margins do not meet.
        assert!(!meets(me, (199.0, 100.0, 299.0, 200.0), 1.0));
        // One standing on the other's head is not in the way.
        assert!(!meets(me, (120.0, 195.0, 220.0, 295.0), 1.0));
    }

    #[test]
    fn another_characters_top_is_a_floor_but_its_own_is_not() {
        let mut scene = Scene::from_config(&crate::config::AppConfig::default());
        scene.entities.truncate(2);
        scene.entities[0].x = 100.0;
        scene.entities[0].y = 0.0;
        scene.entities[1].x = 100.0;
        scene.entities[1].y = 500.0;
        scene.mark_visible_dirty();
        let solids = solids(&scene);
        let floors = platforms_for(&scene, &solids, 0, &[]);
        assert_eq!(floors.len(), 1, "the other one, not itself");
        // Its top, lowered by the empty rows over its head and under the
        // lander's feet: never above the rectangle's top.
        let (_, t, _, b) = solids[1].unwrap();
        assert!(
            floors[0].y >= t && floors[0].y < b,
            "{} in {t}..{b}",
            floors[0].y
        );
        // Held by a drag: nobody's floor.
        scene.entities[1].dragging = true;
        assert!(platforms_for(&scene, &super::solids(&scene), 0, &[]).is_empty());
    }

    #[test]
    fn margins_count_the_empty_rows() {
        // 2×4: rows 0 and 3 empty, rows 1 and 2 with one seen pixel.
        let mut rgba = vec![0u8; 2 * 4 * 4];
        rgba[(2 + 1) * 4 + 3] = 255;
        rgba[(2 * 2) * 4 + 3] = 255;
        let frame = crate::animation::frame::Frame::new(rgba, 2, 4);
        let anim = crate::animation::Animation::new(vec![frame], 1.0, false);
        let config = crate::config::AppConfig::default().characters[0].clone();
        let entity = crate::entity::Entity::from_config(&config, anim);
        assert_eq!(margins(&entity), (1.0, 1.0));
    }

    #[test]
    fn a_walker_turns_when_it_walks_into_someone() {
        let mut scene = Scene::from_config(&crate::config::AppConfig::default());
        scene.entities.truncate(2);
        for e in &mut scene.entities {
            e.y = 300.0;
        }
        scene.entities[0].x = 100.0;
        scene.entities[1].x = 130.0;
        scene.entities[0].behavior = crate::behavior::Behavior::WalkAround { speed: 40.0 };
        scene.entities[0].behavior_state.walk_direction = 1.0;
        scene.mark_visible_dirty();
        turn_walkers(&mut scene);
        assert_eq!(scene.entities[0].behavior_state.walk_direction, -1.0);
        // Now heading away: stays turned.
        turn_walkers(&mut scene);
        assert_eq!(scene.entities[0].behavior_state.walk_direction, -1.0);
    }
}
