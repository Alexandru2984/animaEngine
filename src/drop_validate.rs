//! Drop / library pre-validation — the cheap gate that runs **before**
//! any decoder sees a candidate asset file.
//!
//! Extracted from `app.rs` in F.1 (0.5.1) after the security audit
//! flagged that the native Wayland drop path (`src/wayland/run.rs`)
//! was bypassing this check entirely. The X11 path (`app.rs:1599`)
//! has always called this; lifting the function up to a public
//! sibling module means both call sites — plus the asset-library
//! "Add to scene" outcome — share one source of truth.
//!
//! Invariants the validator enforces:
//!
//! - **Regular file**: directories, symlinks-to-directories, FIFOs,
//!   sockets are rejected. We follow symlinks to reach the file (so
//!   user symlinks into asset collections work) but the resolved
//!   target must `is_file()`.
//! - **Size cap**: `MAX_ASSET_FILE_BYTES` (200 MB at the time of
//!   writing). The decoder path enforces tighter caps per-format,
//!   but the cheap stat-based gate keeps us from ever touching a
//!   50 GB tarball labelled `.png`.
//! - **Extension whitelist**: the file's extension must match one of
//!   the supported decoders. `detect_asset_type` would otherwise
//!   pick the PngStatic default for any unknown extension and hand
//!   the bytes to the image crate.
//!
//! This is **not** a content-sniff. Magic-byte validation lives in
//! the decoders themselves (`validate_image_dimensions`, etc.). The
//! goal here is "obviously wrong inputs never reach decode."

use crate::constants::MAX_ASSET_FILE_BYTES;
use std::path::Path;

/// Extensions we know how to load. Matched against the path the user
/// dropped (or the library entry being added) so we reject obviously-
/// wrong types up front.
pub const DROP_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "mp4", "m4v", "mov"];

/// Sanity-check a candidate asset file before invoking the decoder.
///
/// Returns the reason string when the file should be rejected, or
/// `Ok(())` when it looks plausible. The decoders downstream still
/// run their own format-specific validation (dimensions, frame
/// count, codec checks); this gate only enforces the cheap filesystem-
/// level invariants.
pub fn pre_validate_dropped_file(path: &Path) -> Result<(), String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("can't stat file: {e}"))?;
    if !meta.is_file() {
        return Err("not a regular file".into());
    }
    if meta.len() > MAX_ASSET_FILE_BYTES {
        return Err(format!(
            "file is {} MB; cap is {} MB",
            meta.len() / (1024 * 1024),
            MAX_ASSET_FILE_BYTES / (1024 * 1024)
        ));
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase());
    match ext.as_deref() {
        Some(e) if DROP_EXTENSIONS.contains(&e) => Ok(()),
        Some(e) => Err(format!("unsupported file type: .{e}")),
        None => Err("file has no extension".into()),
    }
}

/// Resolve a library-relative asset path against its root and verify
/// the canonical form stays under that root (M2 hardening, 0.5.2).
///
/// `Path::join` returns the right-hand side as-is when it's absolute,
/// and lets `..` segments climb above the prefix. Neither shape is
/// what an honest library entry produces, but a hand-edited
/// `library.toml` could include them — without this gate a malformed
/// entry could point the engine at any file the user can read, with
/// the error reply itself doubling as a probe for which files exist.
///
/// Returns the canonical absolute path on success, or a descriptive
/// error when the resolved target escapes `root`, doesn't exist, or
/// the OS refuses to canonicalize either side.
pub fn resolve_library_asset(root: &Path, relative: &Path) -> Result<std::path::PathBuf, String> {
    let canonical_root = root
        .canonicalize()
        .map_err(|e| format!("library root unreachable: {e}"))?;
    let joined = canonical_root.join(relative);
    let canonical_target = joined
        .canonicalize()
        .map_err(|e| format!("asset path unreachable: {e}"))?;
    if !canonical_target.starts_with(&canonical_root) {
        return Err(format!(
            "asset path escapes library root ({})",
            canonical_root.display()
        ));
    }
    Ok(canonical_target)
}

#[cfg(test)]
mod tests {
    use super::{pre_validate_dropped_file, resolve_library_asset};
    use std::path::{Path, PathBuf};

    fn workspace_tmp(name: &str) -> PathBuf {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("drop_validate_tests")
            .join(name);
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn drop_rejects_unsupported_extension() {
        let dir = workspace_tmp("bad_ext");
        let path = dir.join("evil.exe");
        std::fs::write(&path, b"x").unwrap();
        let err = pre_validate_dropped_file(&path).unwrap_err();
        assert!(err.contains("unsupported"));
    }

    #[test]
    fn drop_accepts_small_image() {
        let dir = workspace_tmp("size_ok");
        let path = dir.join("tiny.png");
        std::fs::write(&path, b"x").unwrap();
        assert!(pre_validate_dropped_file(&path).is_ok());
    }

    #[test]
    fn drop_rejects_directory() {
        let dir = workspace_tmp("is_dir");
        let err = pre_validate_dropped_file(&dir).unwrap_err();
        assert!(err.contains("not a regular file"));
    }

    #[test]
    fn drop_rejects_missing_extension() {
        let dir = workspace_tmp("no_ext");
        let path = dir.join("noext");
        std::fs::write(&path, b"x").unwrap();
        let err = pre_validate_dropped_file(&path).unwrap_err();
        assert!(err.contains("no extension"));
    }

    #[test]
    fn library_resolve_accepts_in_root() {
        let dir = workspace_tmp("lib_in_root");
        let asset = dir.join("ghost.png");
        std::fs::write(&asset, b"x").unwrap();
        let resolved = resolve_library_asset(&dir, Path::new("ghost.png")).unwrap();
        assert_eq!(resolved, asset.canonicalize().unwrap());
    }

    #[test]
    fn library_resolve_rejects_absolute_outside_root() {
        let dir = workspace_tmp("lib_abs_escape");
        let escape = resolve_library_asset(&dir, Path::new("/etc/hostname"));
        let err = escape.unwrap_err();
        assert!(
            err.contains("escapes") || err.contains("unreachable"),
            "want escape/unreachable, got: {err}",
        );
    }

    #[test]
    fn library_resolve_rejects_dotdot_escape() {
        let parent = workspace_tmp("lib_dotdot");
        let root = parent.join("inner");
        std::fs::create_dir_all(&root).unwrap();
        let neighbour = parent.join("secret.png");
        std::fs::write(&neighbour, b"x").unwrap();
        let escape = resolve_library_asset(&root, Path::new("../secret.png"));
        let err = escape.unwrap_err();
        assert!(err.contains("escapes"), "want escape, got: {err}");
    }

    #[test]
    fn drop_rejects_missing_file() {
        let dir = workspace_tmp("missing");
        let path = dir.join("ghost.png");
        let err = pre_validate_dropped_file(&path).unwrap_err();
        assert!(err.contains("stat") || err.contains("file"));
    }
}
