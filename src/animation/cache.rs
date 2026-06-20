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
//! mtime to the second on some media (FAT32 / SMB). Editing an asset
//! orphans its previous cache file; [`sweep`] (run once at startup)
//! evicts the oldest `.bin` files when the directory exceeds
//! `CACHE_DIR_CAP_BYTES`, so the orphans can't grow without bound.
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

/// Largest a *valid* cache file can be: the per-asset decode cap plus
/// room for the file/frame headers (`MAX_ANIMATION_FRAMES` × 12 + 12,
/// ~7 KiB; 1 MiB is generous slack). We stat against this before reading
/// a cache file whole into memory, so a corrupt or planted oversized
/// file can't OOM us *before* `deserialize_frames`' own caps even run.
const MAX_CACHE_FILE_BYTES: u64 = MAX_DECODED_ASSET_BYTES as u64 + 1024 * 1024;

fn cache_disabled() -> bool {
    std::env::var_os("ANIMA_NO_CACHE").is_some()
}

fn cache_root() -> Option<PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", "animaEngine")?;
    Some(dirs.cache_dir().join("textures"))
}

/// On-disk cap for the decoded-frame cache. Beyond it, [`sweep`] evicts
/// the oldest `.bin` files. Sized to hold a couple of max-size assets
/// (each up to `MAX_DECODED_ASSET_BYTES` = 512 MiB) plus a working set
/// of smaller ones, so ordinary use never trims — only a long history
/// of edited/large assets does.
const CACHE_DIR_CAP_BYTES: u64 = 1024 * 1024 * 1024; // 1 GiB

/// What a [`sweep`] did. `kept_bytes` is the on-disk total afterwards.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SweepReport {
    pub removed: usize,
    pub freed_bytes: u64,
    pub kept_bytes: u64,
}

/// Evict the oldest cache files until the texture cache directory is
/// under `CACHE_DIR_CAP_BYTES` (W.2 — bounds the orphan-file growth
/// the keying scheme creates). Best-effort and side-effect-only:
/// disabled by `ANIMA_NO_CACHE`, a no-op when the dir is missing or
/// already small. Run once at startup, off the hot path — never per
/// frame.
pub fn sweep() -> SweepReport {
    if cache_disabled() {
        return SweepReport::default();
    }
    match cache_root() {
        Some(root) => sweep_dir(&root, CACHE_DIR_CAP_BYTES),
        None => SweepReport::default(),
    }
}

/// IO half of [`sweep`]: stat every `.bin`, and if the total exceeds
/// `cap`, delete oldest-first (by mtime) until it fits. The decision of
/// *how many* to drop is the pure [`evictions_needed`].
fn sweep_dir(root: &Path, cap: u64) -> SweepReport {
    let Ok(read) = fs::read_dir(root) else {
        return SweepReport::default();
    };
    // (path, size, mtime) for every cache file.
    let mut files: Vec<(PathBuf, u64, SystemTime)> = Vec::new();
    for entry in read.flatten() {
        let path = entry.path();
        if path.extension().and_then(|x| x.to_str()) != Some("bin") {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_file() {
            continue;
        }
        let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        files.push((path, meta.len(), mtime));
    }

    files.sort_by_key(|(_, _, mtime)| *mtime); // oldest first
    let sizes: Vec<u64> = files.iter().map(|(_, len, _)| *len).collect();
    let total: u64 = sizes.iter().sum();
    let drop_count = evictions_needed(&sizes, cap);

    let mut freed = 0;
    let mut removed = 0;
    for (path, len, _) in files.into_iter().take(drop_count) {
        if fs::remove_file(&path).is_ok() {
            freed += len;
            removed += 1;
        }
    }
    if removed > 0 {
        tracing::info!(
            "Texture cache sweep: removed {removed} file(s), freed {} MiB (cap {} MiB)",
            freed / (1024 * 1024),
            cap / (1024 * 1024)
        );
    }
    SweepReport {
        removed,
        freed_bytes: freed,
        kept_bytes: total.saturating_sub(freed),
    }
}

/// Pure eviction planner: given file sizes ordered **oldest-first** and
/// a cap, return how many leading (oldest) files to remove so the rest
/// fit under `cap`. Returns 0 when already under cap.
fn evictions_needed(sizes_oldest_first: &[u64], cap: u64) -> usize {
    let mut remaining: u64 = sizes_oldest_first.iter().sum();
    let mut count = 0;
    for &size in sizes_oldest_first {
        if remaining <= cap {
            break;
        }
        remaining = remaining.saturating_sub(size);
        count += 1;
    }
    count
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

    // Stat before slurp: a valid cache is bounded, so an oversized file
    // is corrupt/planted and must not be read whole into RAM (local DoS).
    let meta = fs::metadata(&key).ok()?;
    if meta.len() > MAX_CACHE_FILE_BYTES {
        tracing::warn!(
            "Asset cache at {} is {} bytes (> {} cap); ignoring and regenerating",
            key.display(),
            meta.len(),
            MAX_CACHE_FILE_BYTES,
        );
        let _ = fs::remove_file(&key);
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

/// `pub` (hidden) for the criterion benches and the W.4 fuzz target —
/// not part of any supported API.
#[doc(hidden)]
pub fn serialize_frames(frames: &[Frame]) -> Vec<u8> {
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

/// See [`serialize_frames`] on why this is `pub`.
#[doc(hidden)]
pub fn deserialize_frames(bytes: &[u8]) -> Result<Vec<Frame>> {
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

    #[test]
    fn cache_file_cap_admits_the_largest_valid_cache() {
        // The biggest a legitimate cache can be: the full per-asset pixel
        // budget + every frame header + the file header. The stat guard
        // must never reject that, only files bigger than physically
        // possible for a valid cache.
        let max_valid = MAX_DECODED_ASSET_BYTES as u64
            + (MAX_ANIMATION_FRAMES as u64 * PER_FRAME_HEADER as u64)
            + HEADER_BYTES as u64;
        assert!(
            MAX_CACHE_FILE_BYTES >= max_valid,
            "cap {MAX_CACHE_FILE_BYTES} would reject a legitimate {max_valid}-byte cache",
        );
    }

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

    // ── Cache sweep (W.2) ────────────────────────────────────────────

    #[test]
    fn evictions_needed_under_cap_drops_nothing() {
        assert_eq!(evictions_needed(&[10, 20, 30], 100), 0);
        assert_eq!(evictions_needed(&[], 100), 0);
    }

    #[test]
    fn evictions_needed_drops_oldest_until_under_cap() {
        // total 100, cap 50 → drop oldest (10, 20, 30 = 60 freed)
        // leaving 40 ≤ 50. That's 3 leading files.
        assert_eq!(evictions_needed(&[10, 20, 30, 40], 50), 3);
        // cap 0 → everything goes.
        assert_eq!(evictions_needed(&[5, 5, 5], 0), 3);
        // exactly at cap → nothing.
        assert_eq!(evictions_needed(&[50, 50], 100), 0);
        // one over → drop just the oldest.
        assert_eq!(evictions_needed(&[50, 50, 1], 100), 1);
    }

    #[test]
    fn sweep_dir_trims_to_cap_and_ignores_non_bin() {
        let dir = std::env::temp_dir().join(format!("anima_sweep_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        // Five 100-byte .bin files (500 total) + one unrelated file the
        // sweep must never touch.
        for i in 0..5 {
            fs::write(dir.join(format!("{i:016x}.bin")), vec![0u8; 100]).unwrap();
        }
        fs::write(dir.join("library.toml"), b"keep me").unwrap();

        let report = sweep_dir(&dir, 250);
        // 500 > 250 → must drop the three oldest (300 freed, 200 kept).
        assert_eq!(report.removed, 3, "{report:?}");
        assert!(report.kept_bytes <= 250, "kept {} > cap", report.kept_bytes);
        assert_eq!(report.freed_bytes, 300);
        // The non-.bin file survives.
        assert!(dir.join("library.toml").exists());
        let bins = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("bin"))
            .count();
        assert_eq!(bins, 2);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn sweep_dir_missing_dir_is_noop() {
        let dir = std::env::temp_dir().join("anima_sweep_does_not_exist_xyz");
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(sweep_dir(&dir, 100), SweepReport::default());
    }
}
