//! First-run demo asset generation.
//!
//! This module exists solely so users see something on screen the first time
//! they launch the app. Once they drop their own assets, the demo characters
//! can be deleted from config without losing anything.
//!
//! Not part of the runtime engine — only called from `main.rs` at startup.

mod ghost;
mod slime;

use std::path::Path;

/// Generate all demo assets if they don't already exist at the expected size.
/// Returns silently on any I/O error (demo is best-effort, not load-bearing).
pub fn generate_assets() {
    ghost::generate();
    slime::generate();
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
