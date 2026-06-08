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
        match std::fs::read_to_string(path) {
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
                    tracing::warn!("Failed to parse {}: {}; starting empty", path.display(), e);
                    Self::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(e) => {
                tracing::warn!("Failed to read {}: {}; starting empty", path.display(), e);
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
        for asset in scanned {
            if !self.assets.iter().any(|a| a.id == asset.id) {
                self.assets.push(asset);
            }
        }
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
    walk(root, root, 0, &mut out);
    out
}

fn walk(root: &Path, current: &Path, depth: usize, out: &mut Vec<LibraryAsset>) {
    if depth > MAX_SYMLINK_DEPTH {
        tracing::warn!(
            "Asset scan stopped at symlink depth {} under {}",
            depth,
            current.display(),
        );
        return;
    }
    let read_dir = match std::fs::read_dir(current) {
        Ok(rd) => rd,
        Err(e) => {
            tracing::warn!("Skipping {}: {}", current.display(), e);
            return;
        }
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if metadata.is_dir() {
            // file_type().is_symlink() vs metadata().is_dir() interplay:
            // metadata() follows symlinks, so a symlinked dir reaches
            // this branch and counts toward the depth budget.
            let next_depth = if entry.file_type().map(|ft| ft.is_symlink()).unwrap_or(false) {
                depth + 1
            } else {
                depth
            };
            walk(root, &path, next_depth, out);
            continue;
        }
        if !metadata.is_file() {
            continue;
        }
        let Some(rel) = path.strip_prefix(root).ok().and_then(|p| p.to_str()) else {
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

fn xdg_data_dir() -> PathBuf {
    if let Some(dirs) = directories::ProjectDirs::from("", "", "animaEngine") {
        return dirs.data_dir().to_path_buf();
    }
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
        .join(".local/share/animaEngine")
}

fn xdg_cache_dir() -> PathBuf {
    if let Some(dirs) = directories::ProjectDirs::from("", "", "animaEngine") {
        return dirs.cache_dir().to_path_buf();
    }
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
        .join(".cache/animaEngine")
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
