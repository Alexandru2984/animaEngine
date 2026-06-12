//! On-disk RGBA cache for decoded animation frames.
//!
//! Decoding a 60-frame PNG sequence or a 5 MB GIF takes hundreds of
//! milliseconds even with rayon. After the first run we write the raw RGBA
//! pixels under `~/.cache/animaEngine/textures/<hash>.bin` so subsequent
//! starts are limited only by disk read speed.
//!
//! ## Cache key
//! `hash(canonical_path + mtime_nanos + size + child_count)` — any edit
//! to the asset produces a new key, so stale data is impossible without
//! breaking the filesystem semantics. mtime is read in nanoseconds (not
//! seconds) so two saves within the same second still invalidate; size
//! and child count cover the corner case where the filesystem floors
//! mtime to the second on some media (FAT32 / SMB). Orphan files from
//! previous keys leak on disk; a sweep is left for the packaging phase.
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
    let fp = path_fingerprint(&canon).ok()?;

    Some(root.join(format!("{:016x}.bin", hash_fingerprint(&canon, &fp))))
}

fn hash_fingerprint(canon: &Path, fp: &PathFingerprint) -> u64 {
    let mut hasher = DefaultHasher::new();
    canon.hash(&mut hasher);
    fp.hash(&mut hasher);
    hasher.finish()
}

/// Stat-derived signature of an asset. Two assets with the same
/// canonical path and the same fingerprint are treated as identical for
/// caching; any field changing invalidates the cache.
#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy)]
struct PathFingerprint {
    /// Latest modification time, in nanoseconds since `UNIX_EPOCH`. We
    /// use nanoseconds (not seconds) so two saves within the same
    /// second still produce different keys when the filesystem records
    /// sub-second mtime (ext4 / btrfs / xfs / APFS). Falls back to `0`
    /// when the platform can't report a mtime.
    mtime_nanos: u128,
    /// File size for files; sum of immediate children sizes for
    /// directories. Disambiguates the rare case where two distinct
    /// asset versions share an mtime (FAT32 / SMB / restored backups
    /// that floor the timestamp).
    size: u64,
    /// `1` for files; number of immediate children for directories.
    /// Catches add/remove of a frame in a PNG sequence even when
    /// neither mtime nor total size moves measurably.
    count: u32,
}

/// For files: own mtime + size, count = 1. For directories: max(mtime)
/// over immediate children + sum(size) + child count. Mirrors how the
/// PNG-sequence loader walks the directory.
fn path_fingerprint(path: &Path) -> Result<PathFingerprint> {
    if path.is_dir() {
        let mut mtime_nanos: u128 = 0;
        let mut size: u64 = 0;
        let mut count: u32 = 0;
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if let Ok(meta) = entry.metadata() {
                size = size.saturating_add(meta.len());
                count = count.saturating_add(1);
                if let Ok(mt) = meta.modified() {
                    if let Ok(d) = mt.duration_since(SystemTime::UNIX_EPOCH) {
                        mtime_nanos = mtime_nanos.max(d.as_nanos());
                    }
                }
            }
        }
        Ok(PathFingerprint {
            mtime_nanos,
            size,
            count,
        })
    } else {
        let meta = fs::metadata(path)?;
        let mtime_nanos = meta
            .modified()
            .ok()
            .and_then(|mt| mt.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        Ok(PathFingerprint {
            mtime_nanos,
            size: meta.len(),
            count: 1,
        })
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
        buf.extend(std::iter::repeat_n(0u8, pixel_len));
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

    #[test]
    fn cache_key_distinguishes_size_at_same_mtime() {
        // Two assets identical in mtime but differing in size — second
        // saves on FAT32/SMB share the floored mtime; size catches them.
        let canon = PathBuf::from("/tmp/anima-test/asset");
        let fp_a = PathFingerprint {
            mtime_nanos: 1_700_000_000_000_000_000,
            size: 100,
            count: 1,
        };
        let fp_b = PathFingerprint { size: 200, ..fp_a };
        assert_ne!(
            hash_fingerprint(&canon, &fp_a),
            hash_fingerprint(&canon, &fp_b),
        );
    }

    #[test]
    fn cache_key_distinguishes_nanos_at_same_second() {
        // Two saves landing in the same wall-clock second — ext4/btrfs
        // record nanos, so this used to collide under the seconds-only
        // hash and now does not.
        let canon = PathBuf::from("/tmp/anima-test/asset");
        let fp_a = PathFingerprint {
            mtime_nanos: 1_700_000_000_000_000_000,
            size: 100,
            count: 1,
        };
        let fp_b = PathFingerprint {
            mtime_nanos: 1_700_000_000_500_000_000,
            ..fp_a
        };
        assert_ne!(
            hash_fingerprint(&canon, &fp_a),
            hash_fingerprint(&canon, &fp_b),
        );
    }

    #[test]
    fn cache_key_distinguishes_child_count() {
        // PNG sequence: adding a frame leaves the directory mtime moving
        // by a sub-second, but if the filesystem floors mtime to the
        // second and the added frame's size equals an existing one's,
        // size alone could still match. Child count catches that.
        let canon = PathBuf::from("/tmp/anima-test/sequence");
        let fp_a = PathFingerprint {
            mtime_nanos: 1_700_000_000_000_000_000,
            size: 4096,
            count: 8,
        };
        let fp_b = PathFingerprint { count: 9, ..fp_a };
        assert_ne!(
            hash_fingerprint(&canon, &fp_a),
            hash_fingerprint(&canon, &fp_b),
        );
    }

    #[test]
    fn cache_key_stable_for_identical_fingerprint() {
        // Same path + same fingerprint → same key. Cache hits depend on
        // this; regressing here means every cache lookup misses.
        let canon = PathBuf::from("/tmp/anima-test/asset");
        let fp = PathFingerprint {
            mtime_nanos: 1_700_000_000_000_000_000,
            size: 4096,
            count: 1,
        };
        assert_eq!(hash_fingerprint(&canon, &fp), hash_fingerprint(&canon, &fp),);
    }
}
