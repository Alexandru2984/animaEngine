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

mod cat;
mod ghost;
mod heart;
mod slime;
mod star;

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

/// [`generate_assets`] into `base`: `base/ghost`, `base/slime` and the rest.
pub fn generate_assets_in(base: &Path) {
    ghost::generate(base);
    slime::generate(base);
    heart::generate(base);
    star::generate(base);
    cat::generate(base);
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
