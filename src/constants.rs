//! Cross-module constants. Module-local values stay in their modules.

/// Maximum image dimension (width or height) accepted by loaders.
/// A 4096×4096 RGBA image is 64 MB in RAM — generous for an overlay app
/// while still rejecting decompression bombs.
pub const MAX_IMAGE_DIM: u32 = 4096;

/// Maximum dimension (px) to which dropped assets are resized.
/// Keeps overlay-friendly sprites and bounds GPU texture allocations.
pub const MAX_DROP_SIZE: u32 = 256;

/// Hard cap on entities loaded from a config file. Prevents resource
/// exhaustion from a malicious or runaway config.
pub const MAX_ENTITIES: usize = 64;

/// Maximum number of quads the renderer can batch in one frame.
/// Sized so a full scene of [`MAX_ENTITIES`] sprites still draws every
/// entity plus both UI overlays (selection highlight + edit bar).
/// The renderer's in-loop cap check conservatively reserves 2 slots
/// *before* knowing whether the selection quad was already emitted, so
/// the worst case (all 64 entities drawn, one selected, edit mode on)
/// needs `MAX_ENTITIES + 3` slots for the check to pass on the last
/// entity. Anything smaller silently drops legal entities.
pub const MAX_QUADS: usize = MAX_ENTITIES + 3;

/// Size (px) of the clickable toggle button in the top-right corner.
/// In pass-through mode this is the only area that receives mouse input.
pub const TOGGLE_BUTTON_SIZE: u32 = 64;

/// Hover-startle: a mascot recoils from the pointer when the cursor comes
/// within this radius (px, in global/desktop space) of its centre.
pub const HOVER_STARTLE_RADIUS: f32 = 130.0;

/// Peak recoil speed (px/s) at the centre of the startle radius; the push
/// scales down linearly to zero at the radius edge.
pub const HOVER_STARTLE_SPEED: f32 = 340.0;

/// Poke: horizontal recoil (px) a mascot jumps when tapped, away from the
/// poke point.
pub const POKE_KICK: f32 = 42.0;

/// Poke: upward launch speed (px/s) for a physics-enabled mascot — gravity
/// brings it back down for a little hop. Physics-off mascots just recoil.
pub const POKE_HOP_SPEED: f32 = 430.0;

/// Poke: a press-then-release counts as a tap (→ poke) only if the cursor
/// stayed within this radius (px); moving further is a drag, not a poke.
pub const POKE_TAP_RADIUS: f32 = 6.0;

/// Maximum number of frames extracted from a video. ~20 seconds at 30 fps.
/// Caps memory at roughly MAX_VIDEO_FRAMES × MAX_DROP_SIZE² × 4 bytes
/// (≈150 MB for a 256-px square, much less for typical overlay sprites).
pub const MAX_VIDEO_FRAMES: usize = 600;

/// Cap on the number of frames we'll keep from any animated asset
/// (GIF / WebP / PNG sequence / spritesheet). A pathological 10 000-frame
/// GIF at 256 px would otherwise eat ~2.5 GB of RAM after decode.
pub const MAX_ANIMATION_FRAMES: usize = 600;

/// Cap on the number of PNG files we'll honour inside a sequence
/// directory. The frames-count cap above already protects against decode
/// blowup; this one cuts off the *enumeration* before we even try to
/// open files (handles directories with tens of thousands of entries).
pub const MAX_SEQUENCE_FILES: usize = 1_000;

/// Hard cap on the total decoded-RGBA size we'll hold in memory for a
/// single asset (after resize / sequence). 512 MB matches what a high-end
/// integrated GPU can host without thrashing; loaders that exceed it
/// truncate and log a warning.
pub const MAX_DECODED_ASSET_BYTES: usize = 512 * 1024 * 1024;

/// Cap on the on-disk size we'll accept for a single drag-dropped asset.
/// 200 MB — plenty for any reasonable GIF or short MP4, while still
/// keeping a misclick on a multi-GB video from running OOM on parse.
pub const MAX_ASSET_FILE_BYTES: u64 = 200 * 1024 * 1024;

/// Hard cap on the *aggregate* decoded-RGBA size across all entities
/// loaded into a scene. The per-asset cap [`MAX_DECODED_ASSET_BYTES`]
/// alone allows a worst case of 64 × 512 MB = 32 GB — fine for one
/// hostile asset, catastrophic for a hostile config full of them. The
/// runtime budget defaults to 1 GB, overridable at startup with
/// `ANIMA_MEMORY_BUDGET_MB=<int>` so high-RAM machines can opt in.
/// Resolved lazily from [`max_total_decoded_bytes`].
pub const DEFAULT_MAX_TOTAL_DECODED_BYTES: usize = 1024 * 1024 * 1024;

/// Resolve the runtime aggregate-memory budget.
///
/// Reads `ANIMA_MEMORY_BUDGET_MB` once per call and falls back to
/// [`DEFAULT_MAX_TOTAL_DECODED_BYTES`] when the variable is missing,
/// unparseable, zero, or would saturate `usize`.
pub fn max_total_decoded_bytes() -> usize {
    std::env::var("ANIMA_MEMORY_BUDGET_MB")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|&mb| mb > 0)
        .and_then(|mb| mb.checked_mul(1024 * 1024))
        .unwrap_or(DEFAULT_MAX_TOTAL_DECODED_BYTES)
}
