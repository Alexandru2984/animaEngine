//! First-run demo asset generation.
//!
//! This module exists solely so users see something on screen the first time
//! they launch the app. Once they drop their own assets, the demo characters
//! can be deleted from config without losing anything.
//!
//! The pictures are made in the app's data directory ([`dir`]), and a
//! config's `assets/demo/<name>` finds them there when there is no
//! `assets/` beside the binary or in the current directory
//! (`AppConfig::resolve_asset_path`). They used to be written to
//! `assets/demo/` under the current directory: a package has no `assets/`
//! of its own, so started from the desktop's menu the app made an `assets`
//! folder in its user's home, and where that was not writable its first
//! characters had no pictures.
//!
//! Not part of the runtime engine — only called from `main.rs` at startup.

mod bunny;
mod cat;
mod cloud;
mod frog;
mod ghost;
mod heart;
mod mushroom;
mod paint;
mod robot;
mod slime;
mod star;

/// The characters the app draws itself, by folder: the first five are the
/// first run's scene; the rest come from Library -> Built-in characters
/// and the Meadow friends preset. Each one's name is `demo-<folder>`.
pub const CHARACTERS: [&str; 10] = [
    "ghost", "slime", "heart", "star", "cat", "bunny", "frog", "robot", "cloud", "mushroom",
];

/// Every demo character's side, in pixels.
pub(crate) const SIZE: u32 = 128;

/// The frames in each one's loop, in every state.
const FRAMES: u32 = 12;

/// The drawing's version, kept in each folder (`.art`): pictures made by
/// an earlier one are made again. 4 (1.5): asleep too, in `sleep/`; 3: a
/// walk, a fall and being held, in `walk/`, `fall/` and `drag/`; 2:
/// anti-aliased, outlined, lit, twelve frames; 1 drew hard-edged flat
/// shapes in six or eight.
const ART: &str = "4";

/// The frames drawn of `state`: a loop's, but one asleep — a sleeping
/// scene holds still, so its first frame is all that shows.
pub(crate) fn frames_of(state: StateId) -> u32 {
    if state == StateId::Sleep {
        1
    } else {
        FRAMES
    }
}

use crate::animation::StateId;
use std::path::{Path, PathBuf};

/// Where the demo characters' pictures are made: `demo/` in the app's data
/// directory (`~/.local/share/animaengine/demo`).
pub fn dir() -> PathBuf {
    match directories::ProjectDirs::from("", "", "animaEngine") {
        Some(dirs) => dirs.data_dir().join("demo"),
        None => crate::util::fallback_scoped_dir("").join("demo"),
    }
}

/// Generate all demo assets in [`dir`] if they don't already exist at the
/// expected size. Returns silently on any I/O error (demo is best-effort,
/// not load-bearing).
pub fn generate_assets() {
    generate_assets_in(&dir());
}

/// Frame `frame` of the built-in character `name` standing, drawn `scale`
/// times as large as the app's — for pictures of them, a store page's;
/// `None` for a name not in [`CHARACTERS`].
pub fn picture(name: &str, frame: u32, scale: u32) -> Option<image::RgbaImage> {
    picture_in(name, StateId::Idle, frame, scale)
}

/// [`picture`] in `state`: walking, falling or held as well as standing.
pub fn picture_in(name: &str, state: StateId, frame: u32, scale: u32) -> Option<image::RgbaImage> {
    let f = frame % FRAMES;
    Some(match name {
        "ghost" => ghost::frame(state, f, scale),
        "slime" => slime::frame(state, f, scale),
        "heart" => heart::frame(state, f, scale),
        "star" => star::frame(state, f, scale),
        "cat" => cat::frame(state, f, scale),
        "bunny" => bunny::frame(state, f, scale),
        "frog" => frog::frame(state, f, scale),
        "robot" => robot::frame(state, f, scale),
        "cloud" => cloud::frame(state, f, scale),
        "mushroom" => mushroom::frame(state, f, scale),
        _ => return None,
    })
}

/// [`generate_assets`] into `base`: `base/ghost`, `base/slime` and the rest.
pub fn generate_assets_in(base: &Path) {
    for name in CHARACTERS {
        generate(base, name);
    }
}

/// Every state of demo `name` into `base/name`: standing in the folder,
/// the rest in the folders the loader looks for beside it
/// (`StateId::FOLDERS`). The frames are drawn in parallel; at the first
/// start, before the window, this is what it waits for.
fn generate(base: &Path, name: &str) {
    use rayon::prelude::*;
    let Some(dir) = prepare(base, name, SIZE) else {
        return;
    };
    let states = std::iter::once((StateId::Idle, dir.clone())).chain(
        StateId::FOLDERS
            .iter()
            .map(|(state, folder)| (*state, dir.join(folder))),
    );
    let mut saved = true;
    for (state, into) in states {
        if let Err(e) = std::fs::create_dir_all(&into) {
            tracing::warn!("Failed to create demo directory {}: {e}", into.display());
            return;
        }
        saved &= (0..frames_of(state))
            .into_par_iter()
            .map(|f| match picture_in(name, state, f, 1) {
                Some(image) => save_frame(&into, f + 1, &image),
                None => false,
            })
            .reduce(|| true, |a, b| a && b);
    }
    // Last: a run cut short, or a frame that could not be saved, leaves no
    // mark, and the next start makes them all again.
    if saved {
        let _ = std::fs::write(dir.join(".art"), ART);
        tracing::info!("Generated {name}'s pictures: standing, walking, falling, held and asleep");
    }
}

/// How a built-in character looks in `state` at frame `f` of the loop,
/// as far as every one of them shares it: the whole body's motion, the
/// face, whether it is on the ground. Each draws itself standing, from
/// this, and moves its own arms, legs, ears or tail by `state`.
///
/// The art faces right, as the app expects: walking left, it is mirrored.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Look {
    pub state: StateId,
    /// The loop's angle, 0 to TAU.
    pub phase: f32,
    /// Walking, how far into a step: 0 on the ground, 1 at its top — two
    /// steps a loop. 0 in the other states.
    pub step: f32,
    /// Units off the ground at the top of a step.
    pub lift: f32,
    pub eyes: paint::Eyes,
    pub mouth: paint::Mouth,
    /// The face, turned toward where it goes.
    pub face_dx: f32,
    /// On the ground, and so with a shadow: not falling or held up.
    pub grounded: bool,
}

impl Look {
    pub fn of(state: StateId, f: u32) -> Self {
        let phase = f as f32 / FRAMES as f32 * std::f32::consts::TAU;
        let mut look = Self {
            state,
            phase,
            step: 0.0,
            lift: 0.0,
            eyes: paint::Eyes::Own,
            mouth: paint::Mouth::Own,
            face_dx: 0.0,
            grounded: true,
        };
        match state {
            StateId::Idle => {}
            StateId::Walk => {
                look.step = phase.sin().abs();
                look.lift = 5.0 * look.step;
                look.face_dx = 3.0;
            }
            StateId::Fall => {
                look.eyes = paint::Eyes::Wide;
                look.mouth = paint::Mouth::O;
                look.grounded = false;
            }
            StateId::Drag => {
                look.eyes = paint::Eyes::Happy;
                look.mouth = paint::Mouth::Laugh;
                look.grounded = false;
            }
            StateId::Sleep => {
                look.eyes = paint::Eyes::Asleep;
                look.mouth = paint::Mouth::O;
            }
        }
        look
    }

    /// The body's motion for a character standing on `floor` whose top is
    /// at `top`: walking, a step up and a lean forward, squashed a little
    /// as it lands; falling, stretched and wobbling; held, swinging from
    /// its top, where the pointer has it.
    pub fn motion(&self, floor: f32, top: f32) -> paint::Motion {
        let centre = SIZE as f32 / 2.0;
        let still = paint::Motion::STILL;
        match self.state {
            StateId::Idle => still,
            StateId::Walk => {
                let sy = 0.95 + 0.07 * self.step;
                paint::Motion {
                    pivot: (centre, floor),
                    sx: 1.0 / sy.sqrt(),
                    sy,
                    lean: 0.08,
                    dy: -self.lift,
                    ..still
                }
            }
            StateId::Fall => paint::Motion {
                pivot: (centre, (floor + top) / 2.0),
                sx: 0.95,
                sy: 1.07,
                angle: 0.05 * (self.phase * 2.0).sin(),
                ..still
            },
            StateId::Drag => paint::Motion {
                pivot: (centre, top),
                sx: 0.98,
                sy: 1.03,
                angle: 0.13 * self.phase.sin(),
                dy: -2.0,
                ..still
            },
            // Settled: a little lower and wider, as a body at rest is.
            StateId::Sleep => paint::Motion {
                pivot: (centre, floor),
                sx: 1.03,
                sy: 0.95,
                ..still
            },
        }
    }
}

/// The folder demo `name`'s frames go in, emptied and made ready — or
/// `None` when they are already there, of this drawing, at `size` or
/// larger, or it cannot be made.
fn prepare(base: &Path, name: &str, size: u32) -> Option<std::path::PathBuf> {
    let dir = base.join(name);
    if dir.exists() {
        let same_art = std::fs::read_to_string(dir.join(".art")).is_ok_and(|v| v.trim() == ART);
        if same_art && assets_already_at_size(&dir, size) {
            tracing::debug!(
                "Demo {name} assets already exist at {size}px: {}",
                dir.display()
            );
            return None;
        }
        tracing::info!("Regenerating {name} demo assets at {size}×{size}…");
        let _ = std::fs::remove_dir_all(&dir);
    }
    tracing::info!("Generating {name} demo assets ({size}×{size})…");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        tracing::warn!("Failed to create demo directory {}: {e}", dir.display());
        return None;
    }
    Some(dir)
}

/// Save frame `index` (from 1) of a demo into `dir`; whether it was.
fn save_frame(dir: &Path, index: u32, image: &image::RgbaImage) -> bool {
    let path = dir.join(format!("frame_{index:03}.png"));
    match image.save(&path) {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!("Failed to save demo frame {}: {e}", path.display());
            false
        }
    }
}

/// Check whether a sprite directory already has frames at or above `min_size`.
/// Used to skip regeneration when the user already has demo assets.
fn assets_already_at_size(dir: &Path, min_size: u32) -> bool {
    let first_frame = dir.join("frame_001.png");
    match image::image_dimensions(&first_frame) {
        Ok((w, _)) => w >= min_size,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every character in every state, every frame: nothing drawn past
    /// the picture's edge, where it would be cut off — a step's lift, a
    /// swing, a stretch, a tail hanging down.
    #[test]
    fn nothing_is_cut_off_at_the_edges() {
        use rayon::prelude::*;
        let states = [
            StateId::Idle,
            StateId::Walk,
            StateId::Fall,
            StateId::Drag,
            StateId::Sleep,
        ];
        let cut: Vec<String> = CHARACTERS
            .par_iter()
            .flat_map(|name| {
                states
                    .iter()
                    .flat_map(move |state| (0..FRAMES).map(move |f| (*name, *state, f)))
                    .collect::<Vec<_>>()
            })
            .filter_map(|(name, state, f)| {
                let image = picture_in(name, state, f, 1).expect("a built-in character");
                let n = image.width() - 1;
                let edge = (0..=n)
                    .flat_map(|i| [(i, 0), (i, n), (0, i), (n, i)])
                    .map(|(x, y)| image.get_pixel(x, y)[3])
                    .max()
                    .unwrap_or(0);
                (edge > 8).then(|| format!("{name} {state:?} frame {f}: alpha {edge} at the edge"))
            })
            .collect();
        assert!(cut.is_empty(), "{cut:#?}");
    }

    /// Each state is a drawing of its own, not the standing one again.
    #[test]
    fn every_state_is_drawn() {
        for name in CHARACTERS {
            let idle = picture_in(name, StateId::Idle, 3, 1).unwrap();
            for state in [StateId::Walk, StateId::Fall, StateId::Drag, StateId::Sleep] {
                let other = picture_in(name, state, 3, 1).unwrap();
                assert_ne!(idle.as_raw(), other.as_raw(), "{name} {state:?}");
            }
        }
    }
}
