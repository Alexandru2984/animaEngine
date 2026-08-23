//! Asset library — directory scan + persistent index, the data layer
//! beneath `docs/engine-features.md` §3.
//!
//! Phase C.4 (this commit) ships:
//! - directory discovery (env override → XDG_DATA_HOME → exe-relative)
//! - extension whitelist matching the drag-drop pre-validation in
//!   `crate::app::pre_validate_dropped_file` (audit invariant L2)
//! - stable short ids derived from the canonical path via FNV-1a
//! - serializable `LibraryIndex` (`library.toml`) with atomic writes
//! - thumbnail cache *paths* (the actual decode + write lands in C.5,
//!   which has the egui rendering context)
//!
//! Nothing here touches winit, wgpu, or egui — pure data and disk I/O.
//! The C.5 UI tab consumes a `&LibraryIndex` and surfaces it through
//! the existing settings panel.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::constants::MAX_IMAGE_DIM;
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Schema version stamped on every persisted `library.toml`. Bump when
/// we add a non-optional field (use `#[serde(default)]` first if the
/// field can carry a sensible zero value across migrations).
pub const SCHEMA_VERSION: u32 = 1;

/// Symlink resolution cap for the scanner. The XDG asset dir is the
/// only place we expect symlinks (users may symlink large libraries
/// from external drives). 4 is plenty in practice and stops loops
/// cold.
const MAX_SYMLINK_DEPTH: usize = 4;
/// Global caps so a symlink graph or a pathologically large tree can't
/// turn the (synchronous, pre-window) startup scan into an unbounded
/// CPU/RAM sink. The depth cap alone still allows exponential blow-up
/// through ancestor symlinks; these bound the total work.
const MAX_LIBRARY_ASSETS: usize = 10_000;
const MAX_SCAN_ENTRIES: usize = 50_000;

/// Whitelisted extensions, lowercase. Mirrors
/// `crate::app::DROP_EXTENSIONS` exactly — divergence here would let
/// the library index advertise asset paths that drag-drop refuses to
/// load, which is a confusing UX failure mode. A test asserts the two
/// stay in sync.
pub const LIBRARY_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "mp4", "mov", "m4v"];

/// One indexed asset. Persisted in `library.toml`; loaded back into
/// the UI as a `Vec<LibraryAsset>` and rendered as a thumbnail grid
/// in C.5.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryAsset {
    /// Stable 12-hex-char id derived from the *canonical* relative
    /// path (FNV-1a 64-bit, low 48 bits hex-formatted). Survives
    /// rescans and rename-restore. Used as the thumbnail cache
    /// filename, so it must be filesystem-safe.
    pub id: String,
    /// Path relative to the asset root that produced it. Lets the
    /// index round-trip through a backup / symlink swap without
    /// hardcoding an absolute path.
    pub path: String,
    /// Best-effort kind derived from the file extension. Not
    /// authoritative — the loader re-detects when the user
    /// drag-drops or clicks "Add to scene".
    pub kind: LibraryKind,
    /// Free-form tags assigned by the user from the library UI.
    #[serde(default)]
    pub tags: Vec<String>,
    /// When the file first appeared in a scan.
    pub added_at: SystemTime,
    /// Most recent successful "Add to scene" of this asset. Drives
    /// "Recent" sorting in C.5.
    #[serde(default)]
    pub last_used_at: Option<SystemTime>,
}

impl LibraryAsset {
    /// Returns the asset's thumbnail filename (`<id>.png`). Combine
    /// with [`thumbnail_cache_dir`] for an absolute path.
    pub fn thumbnail_filename(&self) -> String {
        format!("{}.png", self.id)
    }
}

/// The thumbnail's on-disk name is `<id>.png` joined into the cache
/// dir, so `id` must be a single, separator-free path component or the
/// join escapes the cache. Ids produced by [`stable_id`] are always 12
/// lowercase hex chars; only a hand-edited or foreign `library.toml`
/// could carry `../`, an absolute path, or control characters.
/// Restrict to ASCII alphanumerics — a strict superset of the hex ids
/// we generate — so `thumb_dir.join(<id>.png)` can never write outside
/// the cache directory.
fn is_safe_asset_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 128 && id.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// Coarse classification kept distinct from `config::AssetType` so a
/// future asset (e.g. APNG, AVIF) doesn't have to land in both enums
/// at once. The loader does the authoritative type-check at drop
/// time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryKind {
    /// PNG, JPEG, WebP-static — a single still frame.
    Image,
    /// GIF, animated WebP — a multi-frame still asset.
    Animated,
    /// MP4, MOV, M4V — H.264 video.
    Video,
}

impl LibraryKind {
    /// Pick a kind from a file extension (case-insensitive). `None`
    /// when the extension is unknown — the scanner uses this to
    /// silently skip files outside the whitelist.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "png" | "jpg" | "jpeg" => Some(Self::Image),
            "gif" | "webp" => Some(Self::Animated),
            "mp4" | "mov" | "m4v" => Some(Self::Video),
            _ => None,
        }
    }
}

/// Full persisted index. Serialised to `library.toml` via
/// `crate::util::atomic_write_bytes`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryIndex {
    pub schema_version: u32,
    #[serde(default)]
    pub assets: Vec<LibraryAsset>,
}

impl Default for LibraryIndex {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            assets: Vec::new(),
        }
    }
}

impl LibraryIndex {
    /// Default path under `XDG_DATA_HOME` (or `~/.local/share`).
    /// Atomic-write semantics carry over from `util::atomic_write_bytes`.
    pub fn default_path() -> PathBuf {
        xdg_data_dir().join("library.toml")
    }

    /// Load the index from disk, or return [`Self::default`] on any
    /// I/O / parse error. Errors are logged but never surfaced —
    /// rendering an empty library is preferable to crashing the
    /// settings panel.
    pub fn load(path: &Path) -> Self {
        match crate::util::read_to_string_capped(path, crate::constants::MAX_CONFIG_BYTES) {
            Ok(contents) => match toml::from_str::<LibraryIndex>(&contents) {
                Ok(index) => {
                    if index.schema_version != SCHEMA_VERSION {
                        tracing::warn!(
                            "library.toml schema_version={} differs from runtime {}; using as-is",
                            index.schema_version,
                            SCHEMA_VERSION,
                        );
                    }
                    index
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse {}: {}; starting empty",
                        crate::drop_validate::redact_path(path),
                        e
                    );
                    Self::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(e) => {
                tracing::warn!(
                    "Failed to read {}: {}; starting empty",
                    crate::drop_validate::redact_path(path),
                    e
                );
                Self::default()
            }
        }
    }

    /// Atomic-write the index to disk. The TOML is re-emitted in full
    /// every save — the index is small enough (a few hundred entries
    /// at most realistic library sizes) that incremental writes
    /// aren't worth the complexity.
    pub fn save(&self, path: &Path) -> Result<()> {
        let toml_str = toml::to_string_pretty(self)?;
        crate::util::atomic_write_bytes(path, toml_str.as_bytes())?;
        Ok(())
    }

    /// Look an asset up by id. `O(n)` — the UI does this once per
    /// frame for the currently selected asset; scaling it to a
    /// hashmap is premature for current library sizes.
    pub fn find(&self, id: &str) -> Option<&LibraryAsset> {
        self.assets.iter().find(|a| a.id == id)
    }

    /// Merge a freshly scanned roster into the existing index:
    /// - assets present in `scanned` but not in `self.assets` are
    ///   appended with their scan-time `added_at`
    /// - assets present in both keep the persisted `tags` and
    ///   `last_used_at` (the scan can't know either)
    /// - assets present in `self.assets` but missing from `scanned`
    ///   are kept too — the file might be on a temporarily
    ///   disconnected drive
    pub fn merge_scan(&mut self, scanned: Vec<LibraryAsset>) {
        // O(n) via a set of existing ids instead of a linear scan per
        // asset (the old `any()` made this O(n²) — painful once the
        // scanner can surface thousands of entries).
        let existing: std::collections::HashSet<&str> =
            self.assets.iter().map(|a| a.id.as_str()).collect();
        let fresh: Vec<LibraryAsset> = scanned
            .into_iter()
            .filter(|asset| !existing.contains(asset.id.as_str()))
            .collect();
        drop(existing);
        self.assets.extend(fresh);
    }
}

// ─── Discovery + scan ─────────────────────────────────────────────────

/// Pick the first asset directory that exists. Discovery order:
///
/// 1. `$ANIMA_ASSETS_DIR` — env override, useful in tests and CI
/// 2. `$XDG_DATA_HOME/animaEngine/assets/` (defaults to
///    `~/.local/share/animaEngine/assets/`)
/// 3. `assets/` next to the executable (development convenience)
///
/// Returns `None` when none of the candidates exist on disk. The
/// settings UI substitutes an empty index in that case so the user
/// sees "no assets yet" rather than a parse error.
pub fn discover_asset_root() -> Option<PathBuf> {
    if let Ok(env_dir) = std::env::var("ANIMA_ASSETS_DIR") {
        let path = PathBuf::from(env_dir);
        if path.is_dir() {
            return Some(path);
        }
    }
    let xdg = xdg_data_dir().join("assets");
    if xdg.is_dir() {
        return Some(xdg);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let dev = parent.join("assets");
            if dev.is_dir() {
                return Some(dev);
            }
        }
    }
    None
}

/// Walk `root` recursively and return a [`LibraryAsset`] for every
/// file whose extension is in [`LIBRARY_EXTENSIONS`]. Symlinks are
/// followed but capped at `MAX_SYMLINK_DEPTH` (4) to defuse loops.
///
/// Paths in the returned assets are relative to `root` and use `/`
/// separators on every platform so the index file round-trips across
/// host migrations.
///
/// Errors from individual files (permission denied, IO) are logged
/// and skipped — the scanner never aborts a whole walk for one bad
/// entry.
pub fn scan(root: &Path) -> Vec<LibraryAsset> {
    let mut out = Vec::new();
    // G.7 (0.5.3): canonicalise the root once up front so the
    // per-file containment check inside `walk` can compare against an
    // absolute, symlink-resolved path. A symlink under the library
    // tree pointing at `/etc` used to surface those files in the UI
    // (resolve_library_asset rejected them at "Add to scene", but the
    // listing leak itself was avoidable).
    let canonical_root = match root.canonicalize() {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(
                "Asset library root {} unreachable: {e}",
                crate::drop_validate::redact_path(root)
            );
            return out;
        }
    };
    let mut state = ScanState {
        visited: std::collections::HashSet::new(),
        entries_seen: 0,
    };
    walk(&canonical_root, &canonical_root, 0, &mut out, &mut state);
    out
}

/// Mutable scan bookkeeping: directories already entered (by identity)
/// and the running entry count, both shared across the recursion.
struct ScanState {
    visited: std::collections::HashSet<(u64, u64)>,
    entries_seen: usize,
}

/// A directory's `(device, inode)` identity for cycle detection. `None`
/// off-Unix, where the depth + entry-count caps do the bounding instead.
#[cfg(unix)]
fn dir_id(meta: &std::fs::Metadata) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    Some((meta.dev(), meta.ino()))
}
#[cfg(not(unix))]
fn dir_id(_meta: &std::fs::Metadata) -> Option<(u64, u64)> {
    None
}

fn walk(
    root: &Path,
    current: &Path,
    depth: usize,
    out: &mut Vec<LibraryAsset>,
    state: &mut ScanState,
) {
    if depth > MAX_SYMLINK_DEPTH {
        tracing::warn!(
            "Asset scan stopped at symlink depth {} under {}",
            depth,
            crate::drop_validate::redact_path(current),
        );
        return;
    }
    // Global caps — stop the whole walk once either is reached.
    if out.len() >= MAX_LIBRARY_ASSETS || state.entries_seen >= MAX_SCAN_ENTRIES {
        return;
    }
    // Cycle guard: never enter the same directory twice. A symlink back
    // to an ancestor (or a symlink graph) would otherwise be re-walked
    // up to the depth budget, exploding the work.
    if let Ok(meta) = std::fs::metadata(current) {
        if let Some(id) = dir_id(&meta) {
            if !state.visited.insert(id) {
                return;
            }
        }
    }
    let read_dir = match std::fs::read_dir(current) {
        Ok(rd) => rd,
        Err(e) => {
            tracing::warn!(
                "Skipping {}: {}",
                crate::drop_validate::redact_path(current),
                e
            );
            return;
        }
    };
    for entry in read_dir.flatten() {
        state.entries_seen += 1;
        if state.entries_seen >= MAX_SCAN_ENTRIES {
            tracing::warn!("Asset scan hit the {MAX_SCAN_ENTRIES}-entry cap; stopping early.");
            return;
        }
        if out.len() >= MAX_LIBRARY_ASSETS {
            return;
        }
        let path = entry.path();
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if metadata.is_dir() {
            // `imported/` at the library root is importer-managed
            // (U.4): it holds per-state frame sequences whose
            // individual PNGs would otherwise flood the library with
            // hundreds of meaningless single-frame entries. Imported
            // characters reach the scene through the import flow, not
            // the asset grid.
            if depth == 0 && current == root && path.file_name().is_some_and(|n| n == "imported") {
                continue;
            }
            // file_type().is_symlink() vs metadata().is_dir() interplay:
            // metadata() follows symlinks, so a symlinked dir reaches
            // this branch and counts toward the depth budget.
            let next_depth = if entry.file_type().map(|ft| ft.is_symlink()).unwrap_or(false) {
                depth + 1
            } else {
                depth
            };
            walk(root, &path, next_depth, out, state);
            continue;
        }
        if !metadata.is_file() {
            continue;
        }
        // G.7 (0.5.3): canonicalise the candidate and confirm it
        // resolves to a path inside the (already-canonical) root.
        // Drops entries reached through symlinks that point outside
        // the library tree, so the listing UI doesn't surface
        // unrelated files even if the "Add to scene" gate would
        // refuse them later anyway.
        let Ok(canonical) = path.canonicalize() else {
            continue;
        };
        if !canonical.starts_with(root) {
            continue;
        }
        let Some(rel) = canonical.strip_prefix(root).ok().and_then(|p| p.to_str()) else {
            continue;
        };
        // Normalize separators for cross-platform round-trip of the
        // index file (Windows-built indexes round-tripped to Linux
        // would otherwise carry backslashes).
        let rel = rel.replace(std::path::MAIN_SEPARATOR, "/");
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        let Some(kind) = LibraryKind::from_extension(ext) else {
            continue;
        };
        let added_at = metadata.created().or_else(|_| metadata.modified()).ok();
        let added_at = added_at.unwrap_or(SystemTime::UNIX_EPOCH);
        out.push(LibraryAsset {
            id: stable_id(&rel),
            path: rel,
            kind,
            tags: Vec::new(),
            added_at,
            last_used_at: None,
        });
    }
}

/// Where thumbnails live: `$XDG_CACHE_HOME/animaEngine/thumbs/`.
/// Created by C.5 when it first renders a thumbnail; this function
/// only returns the path so tests can mock it.
pub fn thumbnail_cache_dir() -> PathBuf {
    xdg_cache_dir().join("thumbs")
}

/// Maximum thumbnail edge, px. Grid cells render at half this on a
/// 1× display, so thumbs stay crisp on 2× without a regenerate.
pub const THUMB_EDGE: u32 = 64;

/// Generate the thumbnail for one asset if missing or stale (source
/// mtime newer than the thumb's). Returns the thumb path when one
/// exists after the call.
///
/// First frame only; `Video` assets are skipped — decoding H.264 for
/// a 64 px preview costs more than the glyph fallback is worth (the
/// UI shows a film icon instead). GIF/WebP decode to their first
/// frame through `image::open`, which is exactly what we want here.
pub fn ensure_thumbnail(root: &Path, asset: &LibraryAsset) -> Option<PathBuf> {
    ensure_thumbnail_at(root, asset, &thumbnail_cache_dir())
}

/// [`ensure_thumbnail`] with an explicit cache directory — split out
/// so tests don't write into the real user cache.
fn ensure_thumbnail_at(root: &Path, asset: &LibraryAsset, thumb_dir: &Path) -> Option<PathBuf> {
    if matches!(asset.kind, LibraryKind::Video) {
        return None;
    }
    // `asset.path` round-trips through library.toml, so a stale or
    // hand-edited entry (merge_scan keeps unmatched entries around for
    // disconnected drives) could carry a `../` escape or an absolute
    // path. Route through the same canonicalize-and-contain gate as
    // "Add to scene" instead of trusting it here too.
    let src = match crate::drop_validate::resolve_library_asset(root, Path::new(&asset.path)) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(
                "Skipping thumbnail for {}: {e}",
                crate::drop_validate::redact_path(Path::new(&asset.path))
            );
            return None;
        }
    };
    // The *write* target is `<id>.png` in the cache dir. Contain it: an
    // id carrying `../` or a separator (only possible from a hand-edited
    // index) would otherwise let `save` drop a PNG outside `thumb_dir`.
    if !is_safe_asset_id(&asset.id) {
        tracing::warn!("Refusing thumbnail: asset id is not a safe filename component");
        return None;
    }
    let thumb = thumb_dir.join(asset.thumbnail_filename());

    let src_mtime = std::fs::metadata(&src).and_then(|m| m.modified()).ok()?;
    if let Ok(tm) = std::fs::metadata(&thumb).and_then(|m| m.modified()) {
        if tm >= src_mtime {
            return Some(thumb); // fresh
        }
    }

    // Reject a decompression bomb (a small file declaring enormous
    // dimensions) at the header stage — the same MAX_IMAGE_DIM gate the
    // scene loader applies. `image::open` would otherwise allocate the
    // full decoded buffer before any cap is checked, OOMing this thread.
    match image::image_dimensions(&src) {
        Ok((w, h)) if w <= MAX_IMAGE_DIM && h <= MAX_IMAGE_DIM => {}
        Ok((w, h)) => {
            tracing::debug!(
                "Thumbnail source is {w}x{h}, over MAX_IMAGE_DIM {MAX_IMAGE_DIM}; skipping"
            );
            return None;
        }
        Err(e) => {
            tracing::debug!("Thumbnail dimension probe failed: {e}");
            return None;
        }
    }

    let img = match image::open(&src) {
        Ok(i) => i,
        Err(e) => {
            tracing::debug!(
                "Thumbnail decode failed for {}: {e}",
                crate::drop_validate::redact_path(&src)
            );
            return None;
        }
    };
    // `thumbnail` preserves aspect ratio within the bounding box and
    // uses a fast integer path for large downscales.
    let small = img.thumbnail(THUMB_EDGE, THUMB_EDGE);
    if let Some(parent) = thumb.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match small.save(&thumb) {
        Ok(()) => Some(thumb),
        Err(e) => {
            tracing::debug!("Thumbnail write failed: {e}");
            None
        }
    }
}

/// Sequentially generate every missing/stale thumbnail for the index.
/// Designed to run on one background thread right after the startup
/// scan — a typical library (≤ a few hundred entries) finishes in
/// well under a second, and the UI picks thumbs up from disk as they
/// appear (no channel needed).
pub fn generate_missing_thumbnails(root: &Path, index: &LibraryIndex) {
    let started = std::time::Instant::now();
    let mut made = 0usize;
    for asset in &index.assets {
        if ensure_thumbnail(root, asset).is_some() {
            made += 1;
        }
    }
    tracing::info!(
        "Thumbnails ready: {made}/{} in {:?}",
        index.assets.len(),
        started.elapsed()
    );
}

/// Decide whether the cached thumbnail at `cached` is still valid for
/// `source`. Returns `false` when the source's mtime is newer than the
/// cache, or when either path is missing. C.5 calls this before
/// re-encoding a thumbnail.
pub fn thumbnail_is_fresh(source: &Path, cached: &Path) -> bool {
    let Ok(src_meta) = std::fs::metadata(source) else {
        return false;
    };
    let Ok(cache_meta) = std::fs::metadata(cached) else {
        return false;
    };
    let Ok(src_mtime) = src_meta.modified() else {
        return false;
    };
    let Ok(cache_mtime) = cache_meta.modified() else {
        return false;
    };
    cache_mtime >= src_mtime
}

// ─── Helpers ──────────────────────────────────────────────────────────

// Fallbacks for stripped envs (no resolvable XDG dirs). Both go
// through `util::fallback_scoped_dir`, which prefers $XDG_RUNTIME_DIR
// and verifies tmpdir ownership/mode — a plain /tmp subdir could be
// pre-created (and thus owned) by another local user.
fn xdg_data_dir() -> PathBuf {
    if let Some(dirs) = directories::ProjectDirs::from("", "", "animaEngine") {
        return dirs.data_dir().to_path_buf();
    }
    crate::util::fallback_scoped_dir("")
}

fn xdg_cache_dir() -> PathBuf {
    if let Some(dirs) = directories::ProjectDirs::from("", "", "animaEngine") {
        return dirs.cache_dir().to_path_buf();
    }
    crate::util::fallback_scoped_dir("-cache")
}

/// FNV-1a 64-bit. Low 48 bits formatted as 12 hex chars give us
/// ~2.8e14 possible ids — far past any realistic library size for
/// collision worries. Stable across Rust release boundaries (FNV
/// is a published spec, not a Rust default hasher).
fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn stable_id(canonical_path: &str) -> String {
    let h = fnv1a_64(canonical_path.as_bytes()) & 0x0000_FFFF_FFFF_FFFF;
    format!("{:012x}", h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// The library scanner must whitelist exactly the extensions that
    /// drag-drop allows, otherwise the UI could surface assets the
    /// loader refuses. This test asserts the two lists stay aligned.
    ///
    /// `crate::app::DROP_EXTENSIONS` is private to that module; the
    /// canonical source is the literal here. If `app::DROP_EXTENSIONS`
    /// ever diverges, the audit invariant L2 is violated and this
    /// test must fail loudly.
    #[test]
    fn library_extensions_match_drag_drop_whitelist() {
        // Keep this list in sync with `app::DROP_EXTENSIONS`. If you
        // see this test fail because you added a new format,
        // update both.
        let expected = ["png", "jpg", "jpeg", "gif", "webp", "mp4", "mov", "m4v"];
        assert_eq!(LIBRARY_EXTENSIONS.len(), expected.len());
        for ext in expected.iter() {
            assert!(
                LIBRARY_EXTENSIONS.contains(ext),
                "missing library ext: {ext}"
            );
        }
    }

    #[test]
    fn kind_from_extension_is_case_insensitive() {
        assert_eq!(LibraryKind::from_extension("PNG"), Some(LibraryKind::Image));
        assert_eq!(
            LibraryKind::from_extension("Gif"),
            Some(LibraryKind::Animated)
        );
        assert_eq!(LibraryKind::from_extension("mp4"), Some(LibraryKind::Video));
        assert_eq!(LibraryKind::from_extension("exe"), None);
    }

    #[test]
    fn stable_id_is_deterministic_and_correct_length() {
        let a = stable_id("foo/bar.png");
        let b = stable_id("foo/bar.png");
        assert_eq!(a, b);
        assert_eq!(a.len(), 12);
        let c = stable_id("foo/baz.png");
        assert_ne!(a, c);
    }

    #[test]
    fn fnv1a_avalanches_single_byte_difference() {
        // Two strings differing by one byte should hash to substantially
        // different values — quick smoke-test that we wrote FNV-1a not
        // the lossy FNV-1.
        let a = fnv1a_64(b"hello");
        let b = fnv1a_64(b"hellp");
        assert_ne!(a, b);
        assert!(
            (a ^ b).count_ones() > 8,
            "expected at least 8 bits flipped, got {} ({:x} vs {:x})",
            (a ^ b).count_ones(),
            a,
            b,
        );
    }

    #[test]
    fn schema_default_matches_const() {
        assert_eq!(LibraryIndex::default().schema_version, SCHEMA_VERSION);
        assert!(LibraryIndex::default().assets.is_empty());
    }

    #[test]
    fn index_round_trips_through_toml() {
        let asset = LibraryAsset {
            id: "0123456789ab".into(),
            path: "ghost/idle.png".into(),
            kind: LibraryKind::Image,
            tags: vec!["mascot".into(), "ghost".into()],
            added_at: SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000),
            last_used_at: None,
        };
        let idx = LibraryIndex {
            schema_version: SCHEMA_VERSION,
            assets: vec![asset.clone()],
        };
        let toml_str = toml::to_string(&idx).unwrap();
        let back: LibraryIndex = toml::from_str(&toml_str).unwrap();
        assert_eq!(back, idx);
        assert_eq!(back.find("0123456789ab"), Some(&asset));
    }

    #[test]
    fn merge_scan_preserves_user_tags_and_drops_no_known_assets() {
        let mut idx = LibraryIndex::default();
        idx.assets.push(LibraryAsset {
            id: stable_id("foo.png"),
            path: "foo.png".into(),
            kind: LibraryKind::Image,
            tags: vec!["fav".into()],
            added_at: SystemTime::UNIX_EPOCH,
            last_used_at: None,
        });

        // Fresh scan re-discovers foo.png + a new bar.png.
        let scanned = vec![
            LibraryAsset {
                id: stable_id("foo.png"),
                path: "foo.png".into(),
                kind: LibraryKind::Image,
                tags: vec![],
                added_at: SystemTime::UNIX_EPOCH,
                last_used_at: None,
            },
            LibraryAsset {
                id: stable_id("bar.png"),
                path: "bar.png".into(),
                kind: LibraryKind::Image,
                tags: vec![],
                added_at: SystemTime::UNIX_EPOCH,
                last_used_at: None,
            },
        ];
        idx.merge_scan(scanned);

        assert_eq!(idx.assets.len(), 2);
        // Existing entry keeps its user-edited tags.
        let foo = idx.find(&stable_id("foo.png")).expect("foo present");
        assert_eq!(foo.tags, vec!["fav".to_string()]);
    }

    /// A real walk: build a tiny temp tree, scan it, verify we land
    /// the right kinds and skip non-whitelisted files.
    #[test]
    fn scan_visits_whitelisted_files_only() {
        let root = tempdir("scan_visits");
        fs::create_dir_all(root.join("ghost")).unwrap();
        fs::write(root.join("ghost/idle.png"), b"x").unwrap();
        fs::write(root.join("ghost/walk.gif"), b"x").unwrap();
        fs::write(root.join("ghost/notes.txt"), b"skip me").unwrap();
        fs::write(root.join("ghost/anim.MP4"), b"x").unwrap();
        fs::create_dir_all(root.join("ghost/sub")).unwrap();
        fs::write(root.join("ghost/sub/sprite.webp"), b"x").unwrap();

        let scanned = scan(&root);
        let paths: Vec<&str> = scanned.iter().map(|a| a.path.as_str()).collect();

        assert!(paths.contains(&"ghost/idle.png"));
        assert!(paths.contains(&"ghost/walk.gif"));
        assert!(paths.contains(&"ghost/anim.MP4"));
        assert!(paths.contains(&"ghost/sub/sprite.webp"));
        assert!(!paths.iter().any(|p| p.ends_with(".txt")));

        // Stable ids are unique within the same scan.
        let mut ids: Vec<&str> = scanned.iter().map(|a| a.id.as_str()).collect();
        ids.sort();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len(), "duplicate library ids in one scan");

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn thumbnail_is_fresh_handles_missing_files() {
        let root = tempdir("thumb_missing");
        let src = root.join("src.png");
        let cache = root.join("cache.png");
        // Neither exists.
        assert!(!thumbnail_is_fresh(&src, &cache));
        fs::write(&src, b"x").unwrap();
        assert!(!thumbnail_is_fresh(&src, &cache));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn thumbnail_generated_and_fits_bounding_box() {
        let dir = tempdir("thumb_gen");
        let root = dir.join("root");
        let thumbs = dir.join("thumbs");
        fs::create_dir_all(&root).unwrap();
        // 200×100 source → thumb must fit 64×64 preserving aspect.
        let img = image::RgbaImage::from_pixel(200, 100, image::Rgba([10, 200, 30, 255]));
        img.save(root.join("wide.png")).unwrap();

        let asset = LibraryAsset {
            id: "abc123def456".into(),
            path: "wide.png".into(),
            kind: LibraryKind::Image,
            tags: vec![],
            added_at: SystemTime::now(),
            last_used_at: None,
        };
        let thumb = ensure_thumbnail_at(&root, &asset, &thumbs).expect("thumb");
        let t = image::open(&thumb).unwrap();
        assert!(t.width() <= THUMB_EDGE && t.height() <= THUMB_EDGE);
        assert_eq!(t.width(), 64);
        assert_eq!(t.height(), 32, "aspect preserved");

        // Second call is a cache hit (same mtime) — file untouched.
        let m1 = fs::metadata(&thumb).unwrap().modified().unwrap();
        let again = ensure_thumbnail_at(&root, &asset, &thumbs).unwrap();
        assert_eq!(again, thumb);
        let m2 = fs::metadata(&thumb).unwrap().modified().unwrap();
        assert_eq!(m1, m2);
    }

    #[test]
    fn thumbnail_skips_video_kind() {
        let dir = tempdir("thumb_video");
        let root = dir.join("root");
        fs::create_dir_all(&root).unwrap();
        let asset = LibraryAsset {
            id: "video0000000".into(),
            path: "clip.mp4".into(),
            kind: LibraryKind::Video,
            tags: vec![],
            added_at: SystemTime::now(),
            last_used_at: None,
        };
        assert!(ensure_thumbnail_at(&root, &asset, &dir.join("thumbs")).is_none());
    }

    /// A stale or hand-edited `library.toml` entry can carry a `../`
    /// escape — `merge_scan` keeps entries the current scan didn't
    /// re-discover, so nothing re-validates `asset.path` between load
    /// and use. `ensure_thumbnail_at` must refuse to read outside
    /// `root` instead of handing `image::open` an escaped path.
    #[test]
    fn thumbnail_refuses_path_escaping_root() {
        let dir = tempdir("thumb_escape");
        let root = dir.join("root");
        fs::create_dir_all(&root).unwrap();

        // A file that exists outside `root`, reachable only via `../`.
        let secret = dir.join("secret.png");
        image::RgbaImage::from_pixel(4, 4, image::Rgba([1, 2, 3, 255]))
            .save(&secret)
            .unwrap();

        let asset = LibraryAsset {
            id: "escape000000".into(),
            path: "../secret.png".into(),
            kind: LibraryKind::Image,
            tags: vec![],
            added_at: SystemTime::now(),
            last_used_at: None,
        };
        assert!(ensure_thumbnail_at(&root, &asset, &dir.join("thumbs")).is_none());
        assert!(
            !dir.join("thumbs").exists(),
            "must not create a thumbnail for an escaped path"
        );
    }

    /// The read *source* being contained isn't enough — the *write*
    /// name `<id>.png` must be a safe path component too, or a
    /// hand-edited id like `../../escape` drops the thumbnail outside
    /// the cache dir even when `asset.path` is perfectly valid.
    #[test]
    fn thumbnail_refuses_unsafe_id_write_escape() {
        let dir = tempdir("thumb_id_escape");
        let root = dir.join("root");
        fs::create_dir_all(&root).unwrap();
        let src = root.join("ok.png");
        image::RgbaImage::from_pixel(4, 4, image::Rgba([1, 2, 3, 255]))
            .save(&src)
            .unwrap();

        let asset = LibraryAsset {
            id: "../../escape".into(),
            path: "ok.png".into(),
            kind: LibraryKind::Image,
            tags: vec![],
            added_at: SystemTime::now(),
            last_used_at: None,
        };
        let thumbs = dir.join("thumbs");
        assert!(ensure_thumbnail_at(&root, &asset, &thumbs).is_none());
        assert!(!thumbs.exists(), "unsafe id must write nothing at all");
    }

    #[test]
    fn is_safe_asset_id_accepts_hex_rejects_traversal() {
        assert!(is_safe_asset_id("0123456789ab")); // stable_id shape
        assert!(is_safe_asset_id("abc123DEF456"));
        assert!(!is_safe_asset_id(""));
        assert!(!is_safe_asset_id("../evil"));
        assert!(!is_safe_asset_id("a/b"));
        assert!(!is_safe_asset_id("a.b")); // no dots → no `..`
        assert!(!is_safe_asset_id("/abs"));
    }

    /// A small file that decodes to more than `MAX_IMAGE_DIM` on an axis
    /// must be refused at the header stage, before `image::open` commits
    /// to a full-resolution buffer.
    #[test]
    fn thumbnail_refuses_oversized_source() {
        let dir = tempdir("thumb_oversize");
        let root = dir.join("root");
        fs::create_dir_all(&root).unwrap();
        let src = root.join("big.png");
        image::RgbaImage::from_pixel(MAX_IMAGE_DIM + 1, 1, image::Rgba([9, 9, 9, 255]))
            .save(&src)
            .unwrap();
        let asset = LibraryAsset {
            id: "big000000000".into(),
            path: "big.png".into(),
            kind: LibraryKind::Image,
            tags: vec![],
            added_at: SystemTime::now(),
            last_used_at: None,
        };
        let thumbs = dir.join("thumbs");
        assert!(ensure_thumbnail_at(&root, &asset, &thumbs).is_none());
        assert!(!thumbs.join("big000000000.png").exists());
    }

    fn tempdir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "anima_library_test_{}_{}",
            name,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
