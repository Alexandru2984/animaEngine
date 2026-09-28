//! Arranging characters (1.5): lining several up, spacing them evenly,
//! and snapping a drag to the monitors' edges and to other characters.
//!
//! All of it works on where the renderer draws each character — its
//! group's offset and scale included — so what lines up is what is seen.
//! Moving one moves its stored position by the same amount, which the
//! group transform leaves unchanged.

use crate::scene::Scene;

/// Left, top, right, bottom, in global coordinates.
pub type Rect = (f32, f32, f32, f32);

/// Where the entity at `idx` is drawn.
pub fn drawn_rect(scene: &Scene, idx: usize) -> Option<Rect> {
    let e = scene.entities.get(idx)?;
    let (gx, gy, gscale) = crate::group::transform_for_member(&scene.groups, &e.id);
    let (x, y) = (e.x + gx, e.y + gy);
    Some((
        x,
        y,
        x + e.scaled_width() * gscale,
        y + e.scaled_height() * gscale,
    ))
}

/// The rectangle around every entity at `indices`.
pub fn bounds(scene: &Scene, indices: &[usize]) -> Option<Rect> {
    indices
        .iter()
        .filter_map(|&i| drawn_rect(scene, i))
        .reduce(|(l, t, r, b), (l2, t2, r2, b2)| (l.min(l2), t.min(t2), r.max(r2), b.max(b2)))
}

/// Which edge or centre line of the selection to line characters up on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
    Top,
    Middle,
    Bottom,
}

/// A way to arrange several characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrange {
    Align(Align),
    /// Equal gaps side by side; the leftmost and rightmost stay.
    DistributeHorizontally,
    /// Equal gaps one above another; the top and bottom ones stay.
    DistributeVertically,
}

impl Arrange {
    pub const ALL: [Self; 8] = [
        Self::Align(Align::Left),
        Self::Align(Align::Center),
        Self::Align(Align::Right),
        Self::Align(Align::Top),
        Self::Align(Align::Middle),
        Self::Align(Align::Bottom),
        Self::DistributeHorizontally,
        Self::DistributeVertically,
    ];

    /// How many characters it takes to do anything.
    pub fn needs(self) -> usize {
        match self {
            Self::Align(_) => 2,
            Self::DistributeHorizontally | Self::DistributeVertically => 3,
        }
    }

    pub fn i18n_key(self) -> &'static str {
        match self {
            Self::Align(Align::Left) => "arrange-align-left",
            Self::Align(Align::Center) => "arrange-align-center",
            Self::Align(Align::Right) => "arrange-align-right",
            Self::Align(Align::Top) => "arrange-align-top",
            Self::Align(Align::Middle) => "arrange-align-middle",
            Self::Align(Align::Bottom) => "arrange-align-bottom",
            Self::DistributeHorizontally => "arrange-distribute-horizontally",
            Self::DistributeVertically => "arrange-distribute-vertically",
        }
    }
}

/// Arrange the entities at `indices`. Returns whether any moved.
pub fn arrange(scene: &mut Scene, indices: &[usize], how: Arrange) -> bool {
    let mut placed: Vec<(usize, Rect)> = Vec::new();
    for &i in indices {
        if placed.iter().all(|&(j, _)| j != i) {
            if let Some(rect) = drawn_rect(scene, i) {
                placed.push((i, rect));
            }
        }
    }
    if placed.len() < how.needs() {
        return false;
    }
    let moves = match how {
        Arrange::Align(align) => align_moves(&placed, align),
        Arrange::DistributeHorizontally => distribute_moves(placed, false),
        Arrange::DistributeVertically => distribute_moves(placed, true),
    };
    let mut moved = false;
    for (i, dx, dy) in moves {
        if dx == 0.0 && dy == 0.0 {
            continue;
        }
        if let Some(e) = scene.entities.get_mut(i) {
            e.x += dx;
            e.y += dy;
            e.behavior_state.bounce_invalidate();
            moved = true;
        }
    }
    moved
}

fn align_moves(placed: &[(usize, Rect)], align: Align) -> Vec<(usize, f32, f32)> {
    let rects: Vec<Rect> = placed.iter().map(|&(_, r)| r).collect();
    let (l, t, r, b) = rects
        .iter()
        .copied()
        .reduce(|(l, t, r, b), (l2, t2, r2, b2)| (l.min(l2), t.min(t2), r.max(r2), b.max(b2)))
        .unwrap_or_default();
    placed
        .iter()
        .map(|&(i, (rl, rt, rr, rb))| {
            let (dx, dy) = match align {
                Align::Left => (l - rl, 0.0),
                Align::Center => ((l + r) / 2.0 - (rl + rr) / 2.0, 0.0),
                Align::Right => (r - rr, 0.0),
                Align::Top => (0.0, t - rt),
                Align::Middle => (0.0, (t + b) / 2.0 - (rt + rb) / 2.0),
                Align::Bottom => (0.0, b - rb),
            };
            (i, dx, dy)
        })
        .collect()
}

/// Equal gaps between neighbours along one axis, in the order of their
/// centres; the first and last stay put.
fn distribute_moves(mut placed: Vec<(usize, Rect)>, vertical: bool) -> Vec<(usize, f32, f32)> {
    let span = |r: &Rect| if vertical { (r.1, r.3) } else { (r.0, r.2) };
    placed.sort_by(|a, b| {
        let (a0, a1) = span(&a.1);
        let (b0, b1) = span(&b.1);
        (a0 + a1).total_cmp(&(b0 + b1))
    });
    let (first, _) = span(&placed[0].1);
    let (_, last) = span(&placed[placed.len() - 1].1);
    let sizes: f32 = placed
        .iter()
        .map(|(_, r)| {
            let (lo, hi) = span(r);
            hi - lo
        })
        .sum();
    let gap = (last - first - sizes) / (placed.len() - 1) as f32;
    let mut at = first;
    let mut moves = Vec::with_capacity(placed.len());
    for (i, rect) in &placed {
        let (lo, hi) = span(rect);
        let delta = at - lo;
        moves.push(if vertical {
            (*i, 0.0, delta)
        } else {
            (*i, delta, 0.0)
        });
        at += hi - lo + gap;
    }
    moves
}

// ── Snapping ──────────────────────────────────────────────────────────

/// Within this many pixels an edge or a centre snaps.
pub const SNAP_DISTANCE: f32 = 8.0;

/// A line showing what a drag snapped to, from `from` to `to` along it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Guide {
    Vertical { x: f32, from: f32, to: f32 },
    Horizontal { y: f32, from: f32, to: f32 },
}

/// How far to shift `moving` so that one of its edges or its centre meets
/// the nearest edge or centre of a monitor or of another character, on
/// each axis, when one is within [`SNAP_DISTANCE`] — and the guides that
/// show it.
pub fn snap(moving: Rect, others: &[Rect], monitors: &[Rect]) -> (f32, f32, Vec<Guide>) {
    let lines = |r: &Rect, vertical: bool| {
        if vertical {
            [r.0, (r.0 + r.2) / 2.0, r.2]
        } else {
            [r.1, (r.1 + r.3) / 2.0, r.3]
        }
    };
    // The nearest (distance, shift, target) on one axis.
    let nearest = |vertical: bool| {
        let mut best: Option<(f32, f32, Rect)> = None;
        for target in others.iter().chain(monitors) {
            for line in lines(target, vertical) {
                for own in lines(&moving, vertical) {
                    let shift = line - own;
                    let distance = shift.abs();
                    if distance <= SNAP_DISTANCE && best.is_none_or(|(d, _, _)| distance < d) {
                        best = Some((distance, shift, *target));
                    }
                }
            }
        }
        best
    };
    let x = nearest(true);
    let y = nearest(false);
    let (dx, dy) = (x.map_or(0.0, |s| s.1), y.map_or(0.0, |s| s.1));
    let placed = (moving.0 + dx, moving.1 + dy, moving.2 + dx, moving.3 + dy);
    let mut guides = Vec::new();
    if let Some((_, _, target)) = x {
        // The line the snapped rectangle now shares with its target.
        let line = lines(&target, true)
            .into_iter()
            .find(|l| {
                lines(&placed, true)
                    .iter()
                    .any(|own| (own - l).abs() < 0.01)
            })
            .unwrap_or(placed.0);
        guides.push(Guide::Vertical {
            x: line,
            from: placed.1.min(target.1),
            to: placed.3.max(target.3),
        });
    }
    if let Some((_, _, target)) = y {
        let line = lines(&target, false)
            .into_iter()
            .find(|l| {
                lines(&placed, false)
                    .iter()
                    .any(|own| (own - l).abs() < 0.01)
            })
            .unwrap_or(placed.1);
        guides.push(Guide::Horizontal {
            y: line,
            from: placed.0.min(target.0),
            to: placed.2.max(target.2),
        });
    }
    (dx, dy, guides)
}

/// Every monitor as a rectangle, for [`snap`].
pub fn monitor_rects(monitors: &[crate::monitor::MonitorInfo]) -> Vec<Rect> {
    monitors
        .iter()
        .map(|m| {
            let (x, y) = (m.x as f32, m.y as f32);
            (x, y, x + m.width as f32, y + m.height as f32)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Characters "a", "b", "c" (no asset: fallback circles) at the given
    /// positions.
    fn scene(at: [(f32, f32); 3]) -> Scene {
        let mut scene = Scene::from_config(&crate::config::AppConfig::default());
        let configs: Vec<_> = ["a", "b", "c"]
            .into_iter()
            .zip(at)
            .map(|(id, (x, y))| crate::config::CharacterConfig {
                id: id.into(),
                name: id.into(),
                asset_type: crate::config::AssetType::PngStatic,
                asset_path: String::new(),
                x,
                y,
                scale: 1.0,
                opacity: 1.0,
                fps: 8.0,
                visible: true,
                playing: false,
                z_index: 0,
                physics_enabled: false,
                behavior: crate::behavior::Behavior::Idle,
                spritesheet_columns: None,
                spritesheet_rows: None,
                monitor: None,
                easing: None,
                animations: std::collections::BTreeMap::new(),
            })
            .collect();
        scene.restore_configs(&configs);
        scene
    }

    fn xs(s: &Scene) -> Vec<f32> {
        s.entities.iter().map(|e| e.x).collect()
    }
    fn ys(s: &Scene) -> Vec<f32> {
        s.entities.iter().map(|e| e.y).collect()
    }

    #[test]
    fn aligning_lines_up_on_the_selections_own_edges() {
        let mut s = scene([(0.0, 0.0), (100.0, 50.0), (300.0, 20.0)]);
        assert!(arrange(&mut s, &[0, 1], Arrange::Align(Align::Bottom)));
        assert_eq!(ys(&s), [50.0, 50.0, 20.0], "c not selected");
        assert!(arrange(&mut s, &[0, 1, 2], Arrange::Align(Align::Right)));
        assert_eq!(xs(&s), [300.0; 3]);
        assert!(
            !arrange(&mut s, &[0, 1, 2], Arrange::Align(Align::Right)),
            "already"
        );
        assert!(arrange(&mut s, &[0, 2], Arrange::Align(Align::Top)));
        assert_eq!(ys(&s), [20.0, 50.0, 20.0]);
    }

    #[test]
    fn aligning_centres_uses_the_middle_of_the_whole() {
        let mut s = scene([(0.0, 0.0), (200.0, 0.0), (0.0, 0.0)]);
        assert!(arrange(&mut s, &[0, 1], Arrange::Align(Align::Center)));
        assert_eq!(xs(&s)[..2], [100.0, 100.0]);
    }

    #[test]
    fn distributing_evens_the_gaps_and_keeps_the_ends() {
        let mut s = scene([(0.0, 0.0), (50.0, 0.0), (400.0, 0.0)]);
        let w = s.entities[0].scaled_width();
        assert!(arrange(&mut s, &[2, 0, 1], Arrange::DistributeHorizontally));
        let x = xs(&s);
        assert_eq!((x[0], x[2]), (0.0, 400.0), "the ends stay");
        let (gap1, gap2) = (x[1] - (x[0] + w), x[2] - (x[1] + w));
        assert!((gap1 - gap2).abs() < 0.01, "{gap1} vs {gap2}");
        // Two are not enough to distribute.
        assert!(!arrange(&mut s, &[0, 1], Arrange::DistributeVertically));
    }

    #[test]
    fn a_group_transform_counts_where_it_is_drawn() {
        let mut s = scene([(0.0, 0.0), (100.0, 0.0), (0.0, 0.0)]);
        s.group_entities(&[1], |n| format!("Group {n}"));
        s.groups[0].offset_x = 50.0;
        // b is drawn at 150: aligning left moves a to 0 — already there —
        // and b's stored x by −150, which draws it at 0.
        assert!(arrange(&mut s, &[0, 1], Arrange::Align(Align::Left)));
        assert_eq!(drawn_rect(&s, 1).unwrap().0, 0.0);
        assert_eq!(s.entities[1].x, -50.0);
    }

    #[test]
    fn a_drag_snaps_to_the_nearest_edge_or_centre_within_reach() {
        let monitor = (0.0, 0.0, 1600.0, 1000.0);
        let other = (500.0, 500.0, 600.0, 600.0);
        // Left edge 5 px from the monitor's: snaps left, and no y line near.
        let (dx, dy, guides) = snap((5.0, 300.0, 69.0, 364.0), &[other], &[monitor]);
        assert_eq!((dx, dy), (-5.0, 0.0));
        assert_eq!(
            guides,
            [Guide::Vertical {
                x: 0.0,
                from: 0.0,
                to: 1000.0
            }]
        );
        // Bottom 3 px above the other's top, and right edge 6 px from its
        // left: both axes snap to it.
        let (dx, dy, guides) = snap((430.0, 433.0, 494.0, 497.0), &[other], &[monitor]);
        assert_eq!((dx, dy), (6.0, 3.0));
        assert_eq!(guides.len(), 2);
        // Out of reach: nothing.
        let (dx, dy, guides) = snap((200.0, 200.0, 264.0, 264.0), &[other], &[monitor]);
        assert_eq!((dx, dy), (0.0, 0.0));
        assert!(guides.is_empty());
    }

    #[test]
    fn centres_snap_too() {
        let monitor = (0.0, 0.0, 1600.0, 1000.0);
        // Centre at 797, the monitor's at 800.
        let (dx, _, guides) = snap((765.0, 300.0, 829.0, 364.0), &[], &[monitor]);
        assert_eq!(dx, 3.0);
        assert!(matches!(guides[0], Guide::Vertical { x, .. } if x == 800.0));
    }
}
