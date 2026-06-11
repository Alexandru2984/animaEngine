//! Small cross-cutting utilities. Keep this module thin — anything that
//! grows past ~150 lines deserves its own file.

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Write `contents` to `path` atomically: data goes into a sibling
/// `<path>.anima.tmp`, gets `fsync`-ed, then `rename`d over the target.
/// On Linux `rename` within the same filesystem is atomic, so either the
/// reader sees the old contents or the new ones — never a half-written
/// file (which is what a plain `fs::write` can leave on a crash or
/// power loss).
///
/// On rare filesystems where rename crosses devices the `rename` call
/// returns `Err(ErrorKind::CrossesDevices)`; callers can decide to fall
/// back to `fs::write` if they care.
pub fn atomic_write_bytes(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let tmp = tmp_sibling(path);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    {
        // Scope so the file is closed before we rename.
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(contents)?;
        file.sync_all()?;
    }

    match std::fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            // Best-effort cleanup of the temp file on failure so we don't
            // leave litter behind.
            let _ = std::fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// `<path>.<pid>.anima.tmp` — kept verbose so we never collide with a
/// real asset called e.g. `config.tmp`, and include the process id so
/// two animaEngine instances racing through a missed single-instance
/// lock can't truncate each other's temp files mid-write (M5
/// hardening, 0.5.2). The rename target stays the unchanged final
/// path, so atomicity guarantees aren't affected.
fn tmp_sibling(path: &Path) -> PathBuf {
    let mut name: OsString = path.as_os_str().to_owned();
    name.push(format!(".{}.anima.tmp", std::process::id()));
    PathBuf::from(name)
}

/// uid-scoped fallback directory for when XDG resolution fails
/// (minimal containers, stripped env). Used by `config_path`,
/// `perf::snapshot_dir` and the asset-library dirs; never hit on a
/// normally configured desktop.
///
/// Resolution order:
///
/// 1. `$XDG_RUNTIME_DIR/animaEngine<suffix>` — the runtime dir is
///    `0700` and uid-owned **by spec** (pam_systemd creates it), so
///    another local user can't pre-create or swap entries inside it.
/// 2. `$TMPDIR/animaEngine-<uid><suffix>` — `/tmp` is world-writable
///    (sticky bit), so another local user *can* pre-create our entry
///    and own it; everything we write there would land in a directory
///    they control. Create with mode `0700` and verify (directory,
///    not a symlink, owned by our uid, no group/other bits). If the
///    check fails, retry once with a pid-suffixed sibling — turning
///    the attacker's cheap pre-creation into a mkdir race they have
///    to win live — and scream in the log if even that is compromised.
pub fn fallback_scoped_dir(suffix: &str) -> PathBuf {
    // SAFETY: libc::getuid is a POSIX syscall with no preconditions
    // and no failure modes.
    let uid = unsafe { libc::getuid() };

    if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") {
        let runtime = PathBuf::from(runtime);
        if runtime.is_absolute() && runtime.is_dir() {
            let dir = runtime.join(format!("animaEngine{suffix}"));
            // Trust but verify: the runtime dir is 0700 + uid-owned by
            // spec, but the value reaches us through an env var that a
            // wrapper script can point at any world-writable location
            // (`XDG_RUNTIME_DIR=/tmp anima-engine`) — the same redirect
            // class M3/H1 closed for $HOME. The create+verify gate
            // makes a pre-created entry owned by someone else fall
            // through to the tmp path below instead of being trusted.
            if create_and_verify_private_dir(&dir, uid) {
                return dir;
            }
            tracing::warn!(
                "XDG_RUNTIME_DIR subdir failed ownership/mode verification; \
                 falling back to a uid-scoped tmpdir"
            );
        }
    }

    let primary = std::env::temp_dir().join(format!("animaEngine-{uid}{suffix}"));
    if create_and_verify_private_dir(&primary, uid) {
        return primary;
    }
    let fallback = std::env::temp_dir().join(format!(
        "animaEngine-{uid}-{pid}{suffix}",
        pid = std::process::id()
    ));
    if !create_and_verify_private_dir(&fallback, uid) {
        tracing::error!(
            "Fallback dir {} failed ownership/mode verification — writes \
             there may be readable by other local users",
            fallback.display()
        );
    }
    fallback
}

/// mkdir(0700) + lstat verification: the path is a real directory (not
/// a symlink), owned by `uid`, with no group/other permission bits.
/// Returns `false` on any failure — the caller picks a different path.
fn create_and_verify_private_dir(path: &Path, uid: u32) -> bool {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};

    let mut builder = std::fs::DirBuilder::new();
    builder.mode(0o700);
    match builder.create(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(_) => return false,
    }
    // symlink_metadata (lstat) so a symlink planted at the path is seen
    // as a symlink instead of being followed to wherever it points.
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return false;
    };
    meta.is_dir() && meta.uid() == uid && meta.permissions().mode() & 0o077 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(name: &str) -> PathBuf {
        // Per-test subdir so cargo's parallel runner doesn't have us
        // racing on the same files.
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("util_tests")
            .join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn atomic_write_creates_file_and_cleans_tmp() {
        let dir = test_dir("creates_file");
        let path = dir.join("data.bin");
        atomic_write_bytes(&path, b"hello").unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"hello");
        // Tmp sibling must not survive a successful write.
        assert!(!tmp_sibling(&path).exists());
    }

    #[test]
    fn atomic_write_overwrites_existing() {
        let dir = test_dir("overwrites");
        let path = dir.join("data.bin");
        std::fs::write(&path, b"old contents").unwrap();
        atomic_write_bytes(&path, b"new").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
    }

    #[test]
    fn atomic_write_creates_parent_dirs() {
        let dir = test_dir("creates_parents").join("nested").join("deep");
        let path = dir.join("data.bin");
        atomic_write_bytes(&path, b"x").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"x");
    }

    fn current_uid() -> u32 {
        // SAFETY: getuid has no preconditions or failure modes.
        unsafe { libc::getuid() }
    }

    #[test]
    fn private_dir_created_fresh_passes_verification() {
        let dir = test_dir("private_fresh").join("scoped");
        assert!(create_and_verify_private_dir(&dir, current_uid()));
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&dir).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700);
    }

    #[test]
    fn private_dir_with_lax_mode_fails_verification() {
        // Simulates the pre-created-by-attacker case: the dir exists
        // but with group/other access bits set.
        let dir = test_dir("private_lax").join("scoped");
        std::fs::create_dir(&dir).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(!create_and_verify_private_dir(&dir, current_uid()));
    }

    #[test]
    fn private_dir_symlink_fails_verification() {
        // A symlink planted at the path must not be followed.
        let base = test_dir("private_symlink");
        let target = base.join("target");
        std::fs::create_dir(&target).unwrap();
        let link = base.join("scoped");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(!create_and_verify_private_dir(&link, current_uid()));
    }

    #[test]
    fn private_dir_wrong_owner_fails_verification() {
        // Can't chown without root, so probe the check from the other
        // side: verification against a uid that isn't the dir's owner.
        let dir = test_dir("private_owner").join("scoped");
        assert!(create_and_verify_private_dir(&dir, current_uid()));
        assert!(!create_and_verify_private_dir(
            &dir,
            current_uid().wrapping_add(1)
        ));
    }
}
