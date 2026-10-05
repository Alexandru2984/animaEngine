//! Tapping characters without edit mode (1.5, opt-in: `tap_characters`).
//!
//! In pass-through the overlay takes clicks only on the ⚙ corner. With
//! tapping on, it also takes them on the characters — each one's drawn
//! rectangle ([`rects`]) — and a tap there pokes it ([`poke`]): a hop, a
//! line of its own (`crate::phrases`). Every click anywhere else still
//! goes to the windows underneath, and the overlay does not take the
//! keyboard for it: native Wayland makes its surfaces non-interactive for
//! keys in pass-through, X11 tells the window manager not to focus them.
//!
//! The rectangles follow the characters as they move, at most a few times
//! a second and only when they changed (rounded outward to [`GRID`] px),
//! so a walker does not cost a new input region every frame.

use crate::scene::Scene;

/// The rectangles are rounded outward to this many pixels.
pub const GRID: i32 = 8;

/// How often, at most, the input region follows the characters.
pub const REFRESH: std::time::Duration = std::time::Duration::from_millis(100);

/// A rectangle on a surface: left, top, width, height, in its pixels.
pub type SurfaceRect = (i32, i32, u32, u32);

/// The visible characters' rectangles on a surface whose top-left is at
/// `origin` in the scene's coordinates and whose size is `size`: clipped to
/// it, rounded outward to [`GRID`].
pub fn rects(scene: &Scene, origin: (f32, f32), size: (u32, u32)) -> Vec<SurfaceRect> {
    let (w, h) = (size.0 as i32, size.1 as i32);
    (0..scene.entities.len())
        .filter(|&i| scene.effective_visible(i))
        .filter_map(|i| crate::input::arrange::drawn_rect(scene, i))
        .filter_map(|(l, t, r, b)| {
            let snap_down = |v: f32| (v.floor() as i32).div_euclid(GRID) * GRID;
            let snap_up = |v: f32| -(-(v.ceil() as i32)).div_euclid(GRID) * GRID;
            let l = snap_down(l - origin.0).max(0);
            let t = snap_down(t - origin.1).max(0);
            let r = snap_up(r - origin.0).min(w);
            let b = snap_up(b - origin.1).min(h);
            (r > l && b > t).then(|| (l, t, (r - l) as u32, (b - t) as u32))
        })
        .collect()
}

/// A press on a character in pass-through, waiting for its release.
#[derive(Debug, Default)]
pub struct TapGesture {
    pressed: Option<(String, f32, f32)>,
}

impl TapGesture {
    /// A press at `(x, y)` in the scene's coordinates.
    pub fn press(&mut self, scene: &Scene, x: f32, y: f32) {
        self.pressed = scene
            .entity_at_point(x, y)
            .map(|i| (scene.entities[i].id.clone(), x, y));
    }

    /// The release at `(x, y)`: the character to poke, if this was a tap
    /// on one — a release near where the press was, on the same one.
    pub fn release(&mut self, scene: &Scene, x: f32, y: f32) -> Option<usize> {
        let (id, px, py) = self.pressed.take()?;
        let radius = crate::constants::POKE_TAP_RADIUS * 2.0;
        if (x - px).hypot(y - py) > radius {
            return None;
        }
        scene.entities.iter().position(|e| e.id == id)
    }
}

/// Poke the character at `idx` from `from_x`: the hop, the line and the
/// hearts (`crate::effects`) every tap gets, in edit mode or out of it.
pub fn poke(scene: &mut Scene, idx: usize, from_x: f32, bounds: crate::monitor::DesktopBounds) {
    if let Some(entity) = scene.entities.get_mut(idx) {
        entity.poke(from_x, bounds);
        scene.hearts_for(idx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, CharacterConfig};

    fn scene_at(spots: &[(&str, f32, f32)]) -> Scene {
        let mut scene = Scene::from_config(&AppConfig::default());
        let template = AppConfig::default().characters.remove(0);
        let configs: Vec<_> = spots
            .iter()
            .map(|&(id, x, y)| CharacterConfig {
                id: id.into(),
                name: id.into(),
                asset_path: String::new(),
                x,
                y,
                ..template.clone()
            })
            .collect();
        scene.restore_configs(&configs);
        scene
    }

    #[test]
    fn rectangles_sit_on_the_surface_rounded_outward() {
        let scene = scene_at(&[("a", 1603.0, 205.0), ("b", 10.0, 10.0)]);
        // A second monitor at x = 1600.
        let on_second = rects(&scene, (1600.0, 0.0), (1600, 1000));
        assert_eq!(on_second.len(), 1, "b is not there: {on_second:?}");
        let (l, t, w, h) = on_second[0];
        assert_eq!((l, t), (0, 200), "3 → 0, 205 → 200");
        assert!(w as i32 % GRID == 0 && h as i32 % GRID == 0);
        assert!(l + w as i32 >= 3 + scene.entities[0].scaled_width() as i32);
        // On the first, b — and a is off its right edge.
        assert_eq!(rects(&scene, (0.0, 0.0), (1600, 1000)).len(), 1);
    }

    #[test]
    fn a_tap_is_a_press_and_a_release_close_together_on_one() {
        let scene = scene_at(&[("a", 100.0, 100.0)]);
        let (x, y) = (
            100.0 + scene.entities[0].scaled_width() / 2.0,
            100.0 + scene.entities[0].scaled_height() / 2.0,
        );
        let mut tap = TapGesture::default();
        tap.press(&scene, x, y);
        assert_eq!(tap.release(&scene, x + 3.0, y), Some(0));
        tap.press(&scene, x, y);
        assert_eq!(tap.release(&scene, x + 80.0, y), None, "dragged away");
        tap.press(&scene, 5.0, 5.0);
        assert_eq!(tap.release(&scene, 5.0, 5.0), None, "on nothing");
        assert_eq!(tap.release(&scene, x, y), None, "no press");
    }
}
