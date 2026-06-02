//! On-disk RGBA cache for decoded animation frames.
//!
//! Decoding a 60-frame PNG sequence or a 5 MB GIF takes hundreds of
//! milliseconds even with rayon. After the first run we write the raw RGBA
//! pixels under `~/.cache/animaEngine/textures/<hash>.bin` so subsequent
//! starts are limited only by disk read speed.
//!
//! ## Cache key
//! `hash(canonical_path + latest_mtime_inside)` — any edit to the asset
//! produces a new key, so stale data is impossible without breaking the
//! filesystem semantics. Orphan files from previous keys leak on disk; a
//! sweep is left for the packaging phase.
//!
//! ## File format (little-endian)
//! ```text
//! u32 magic     = 0x414E494D ("ANIM")
//! u32 version   = 1
//! u32 n_frames
//! per frame:
//!   u32 width
//!   u32 height
//!   u32 delay_ms  (0 means "no per-frame delay")
//!   [u8; width*height*4]  raw RGBA
//! ```
//!
//! ## Disable
//! Set `ANIMA_NO_CACHE=1` to bypass both reads and writes — useful when
//! debugging asset loaders.

use crate::animation::frame::Frame;
use crate::constants::{MAX_ANIMATION_FRAMES, MAX_DECODED_ASSET_BYTES, MAX_IMAGE_DIM};
use crate::error::{AnimaError, Result};
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const MAGIC: u32 = 0x414E_494D;
const VERSION: u32 = 1;
const HEADER_BYTES: usize = 12; // magic + version + count
const PER_FRAME_HEADER: usize = 12; // width + height + delay

fn cache_disabled() -> bool {
    std::env::var_os("ANIMA_NO_CACHE").is_some()
}

fn cache_root() -> Option<PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", "animaEngine")?;
    Some(dirs.cache_dir().join("textures"))
}

/// Compute the cache file path for an asset, or `None` if we can't build
/// a stable key (path doesn't canonicalize, mtime unreadable, etc.).
fn cache_key(asset_path: &Path) -> Option<PathBuf> {
    let root = cache_root()?;
    let canon = asset_path.canonicalize().ok()?;
    let mtime = latest_mtime(&canon).ok()?;

    let mut hasher = DefaultHasher::new();
    canon.hash(&mut hasher);
    mtime.hash(&mut hasher);
    let h = hasher.finish();

    Some(root.join(format!("{h:016x}.bin")))
}

/// For files, the file's mtime. For directories, the latest mtime among
/// immediate children — so renaming a frame inside a PNG sequence
/// triggers a cache miss.
fn latest_mtime(path: &Path) -> Result<u64> {
    if path.is_dir() {
        let mut latest = 0u64;
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if let Ok(meta) = entry.metadata() {
                if let Ok(mt) = meta.modified() {
                    if let Ok(d) = mt.duration_since(SystemTime::UNIX_EPOCH) {
                        latest = latest.max(d.as_secs());
                    }
                }
            }
        }
        Ok(latest)
    } else {
        let mt = fs::metadata(path)?.modified()?;
        Ok(mt
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0))
    }
}

/// Try to load decoded frames from the cache. Returns `None` on any miss,
/// disable, or corruption — the caller falls back to full decode.
pub fn try_load(asset_path: &Path) -> Option<Vec<Frame>> {
    if cache_disabled() {
        return None;
    }
    let key = cache_key(asset_path)?;
    if !key.exists() {
        return None;
    }

    let bytes = fs::read(&key).ok()?;
    match deserialize_frames(&bytes) {
        Ok(frames) => {
            tracing::debug!(
                "Asset cache hit: {} → {}",
                asset_path.display(),
                key.display()
            );
            Some(frames)
        }
        Err(e) => {
            tracing::warn!("Asset cache corrupt at {}: {}", key.display(), e);
            // Best-effort cleanup so the next run regenerates a clean file.
            let _ = fs::remove_file(&key);
            None
        }
    }
}

/// Write decoded frames to the cache. Errors are reported as `Err` but
/// callers typically log and continue — caching is a perf optimization,
/// not a correctness requirement.
pub fn try_save(asset_path: &Path, frames: &[Frame]) -> Result<()> {
    if cache_disabled() {
        return Ok(());
    }
    let Some(key) = cache_key(asset_path) else {
        return Ok(());
    };

    let bytes = serialize_frames(frames);
    // Atomic write so a crash mid-write can't corrupt a cache file the
    // next launch would have to repair.
    crate::util::atomic_write_bytes(&key, &bytes)?;
    tracing::debug!(
        "Wrote asset cache: {} ({} bytes)",
        key.display(),
        bytes.len()
    );
    Ok(())
}

fn serialize_frames(frames: &[Frame]) -> Vec<u8> {
    // Pre-size to avoid reallocs on big sequences.
    let total = HEADER_BYTES
        + frames
            .iter()
            .map(|f| PER_FRAME_HEADER + f.rgba.len())
            .sum::<usize>();
    let mut out = Vec::with_capacity(total);

    out.extend_from_slice(&MAGIC.to_le_bytes());
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&(frames.len() as u32).to_le_bytes());

    for f in frames {
        out.extend_from_slice(&f.width.to_le_bytes());
        out.extend_from_slice(&f.height.to_le_bytes());
        out.extend_from_slice(&f.delay_ms.unwrap_or(0).to_le_bytes());
        out.extend_from_slice(&f.rgba);
    }
    out
}

fn deserialize_frames(bytes: &[u8]) -> Result<Vec<Frame>> {
    if bytes.len() < HEADER_BYTES {
        return Err(AnimaError::other("cache header too short"));
    }

    let magic = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    if magic != MAGIC {
        return Err(AnimaError::other("cache magic mismatch"));
    }

    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    if version != VERSION {
        return Err(AnimaError::other("cache version mismatch"));
    }

    // A malicious / corrupted cache file could claim a 100M-frame asset
    // and trick us into preallocating a huge Vec<Frame>. Reject up front
    // against the same caps the live decoders enforce.
    let count = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    if count > MAX_ANIMATION_FRAMES {
        return Err(AnimaError::other(format!(
            "cache claims {count} frames, max {MAX_ANIMATION_FRAMES}"
        )));
    }

    let mut frames = Vec::with_capacity(count);
    let mut cursor = HEADER_BYTES;
    let mut total_bytes: usize = 0;

    for _ in 0..count {
        if cursor + PER_FRAME_HEADER > bytes.len() {
            return Err(AnimaError::other("cache truncated mid-frame-header"));
        }
        let w = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
        let h = u32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().unwrap());
        let d = u32::from_le_bytes(bytes[cursor + 8..cursor + 12].try_into().unwrap());
        cursor += PER_FRAME_HEADER;

        // Reject frames whose dimensions exceed what we'd accept from a
        // fresh decode. Mirrors validate_image_dimensions.
        if w > MAX_IMAGE_DIM || h > MAX_IMAGE_DIM {
            return Err(AnimaError::ImageTooLarge {
                width: w,
                height: h,
                max: MAX_IMAGE_DIM,
            });
        }

        let pixel_len = (w as usize)
            .checked_mul(h as usize)
            .and_then(|n| n.checked_mul(4))
            .ok_or_else(|| AnimaError::other("frame pixel count overflow"))?;
        if cursor + pixel_len > bytes.len() {
            return Err(AnimaError::other("cache truncated mid-pixels"));
        }

        // Refuse before allocating if the cumulative pixel payload would
        // exceed our universal asset cap.
        total_bytes = total_bytes
            .checked_add(pixel_len)
            .ok_or_else(|| AnimaError::other("cumulative byte counter overflow"))?;
        if total_bytes > MAX_DECODED_ASSET_BYTES {
            return Err(AnimaError::other(format!(
                "cache exceeds MAX_DECODED_ASSET_BYTES = {} MB",
                MAX_DECODED_ASSET_BYTES / (1024 * 1024)
            )));
        }

        let rgba = bytes[cursor..cursor + pixel_len].to_vec();
        cursor += pixel_len;

        let frame = if d == 0 {
            Frame::new(rgba, w, h)
        } else {
            Frame::with_delay(rgba, w, h, d)
        };
        frames.push(frame);
    }

    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_frames() -> Vec<Frame> {
        vec![
            Frame::new(vec![10, 20, 30, 40, 50, 60, 70, 80], 2, 1),
            Frame::with_delay(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], 1, 3, 100),
        ]
    }

    #[test]
    fn roundtrip_preserves_frames() {
        let original = sample_frames();
        let bytes = serialize_frames(&original);
        let decoded = deserialize_frames(&bytes).unwrap();

        assert_eq!(decoded.len(), original.len());
        for (a, b) in decoded.iter().zip(&original) {
            assert_eq!(a.width, b.width);
            assert_eq!(a.height, b.height);
            assert_eq!(a.delay_ms, b.delay_ms);
            assert_eq!(a.rgba, b.rgba);
        }
    }

    #[test]
    fn bad_magic_rejected() {
        let mut bytes = serialize_frames(&sample_frames());
        bytes[0] = 0xAB;
        let err = deserialize_frames(&bytes).unwrap_err();
        assert!(err.to_string().contains("magic"));
    }

    #[test]
    fn bad_version_rejected() {
        let mut bytes = serialize_frames(&sample_frames());
        // Bump version field.
        bytes[4] = 99;
        let err = deserialize_frames(&bytes).unwrap_err();
        assert!(err.to_string().contains("version"));
    }

    #[test]
    fn truncated_payload_rejected() {
        let bytes = serialize_frames(&sample_frames());
        // Drop the last 5 bytes from the middle of frame 1's pixel data.
        let truncated = &bytes[..bytes.len() - 5];
        assert!(deserialize_frames(truncated).is_err());
    }

    #[test]
    fn empty_input_rejected() {
        assert!(deserialize_frames(&[]).is_err());
    }

    /// Build a header with an arbitrary `count` and no frames after it —
    /// used to test the count-cap without writing a real payload.
    fn header_with_count(count: u32) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&MAGIC.to_le_bytes());
        buf.extend_from_slice(&VERSION.to_le_bytes());
        buf.extend_from_slice(&count.to_le_bytes());
        buf
    }

    #[test]
    fn excessive_frame_count_rejected() {
        // Claim 1 million frames — well above MAX_ANIMATION_FRAMES.
        let bytes = header_with_count(1_000_000);
        let err = deserialize_frames(&bytes).unwrap_err();
        assert!(err.to_string().contains("max"), "got: {err}");
    }

    /// Build a one-frame cache with the given declared dimensions but a
    /// matching payload (so we get past the truncation check).
    fn one_frame_cache(w: u32, h: u32) -> Vec<u8> {
        let mut buf = header_with_count(1);
        buf.extend_from_slice(&w.to_le_bytes());
        buf.extend_from_slice(&h.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes()); // delay
        let pixel_len = (w as usize) * (h as usize) * 4;
        buf.extend(std::iter::repeat(0u8).take(pixel_len));
        buf
    }

    #[test]
    fn oversized_frame_dim_rejected() {
        // Declare a 5000×5000 frame. We allocate the corresponding payload
        // (100 MB of zeros) just so the truncation check passes; the
        // dimension check should trigger first.
        let bytes = one_frame_cache(MAX_IMAGE_DIM + 1, 32);
        let err = deserialize_frames(&bytes).unwrap_err();
        assert!(matches!(err, AnimaError::ImageTooLarge { .. }));
    }
}
