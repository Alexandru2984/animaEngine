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
}
