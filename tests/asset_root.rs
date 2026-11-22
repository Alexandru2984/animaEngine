//! The asset library's root: made under XDG when there is none, never
//! where `ANIMA_ASSETS_DIR` points (`asset_library::ensure_asset_root`).
//!
//! An integration test, in a process of its own, because it sets
//! `XDG_DATA_HOME` and `ANIMA_ASSETS_DIR`: in the library's unit tests it
//! did so while the other tests ran on threads beside it, and setting the
//! environment while another thread reads it is undefined behaviour in
//! glibc (TEST-001). Alone in its file, nothing else runs in this process.
//!
//! Unix only, as before: the data directory is scoped through
//! `XDG_DATA_HOME`, which Windows and macOS ignore — there it would reach,
//! and create, the user's real application-data folder.
#![cfg(unix)]

use std::path::PathBuf;

/// Both checks live in one test because they mutate process-global
/// env vars and cargo runs tests in parallel threads within a single
/// process — the same reason `demo_generation` keeps its two steps
/// together. Split, they raced and one saw the other's XDG_DATA_HOME.
#[test]
// The data directory is scoped through XDG_DATA_HOME, which Windows and
// macOS ignore: there the test would reach, and create, the user's real
// application-data folder.
#[cfg_attr(
    any(windows, target_os = "macos"),
    ignore = "scopes the data dir through XDG_DATA_HOME, which this OS ignores"
)]
fn ensure_asset_root_creates_xdg_but_never_an_explicit_override() {
    fn scratch(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "anima-ensure-root-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    let prev_data = std::env::var_os("XDG_DATA_HOME");
    let prev_assets = std::env::var_os("ANIMA_ASSETS_DIR");

    let home = scratch("xdg");
    let missing = scratch("override");

    // ── The override case runs FIRST, while the XDG directory still
    // does not exist. Ordered the other way it passes for the wrong
    // reason: `discover_asset_root` falls back to XDG when the
    // override is missing, so a directory created earlier in this test
    // gets returned and hides whether the override was honoured.
    std::env::set_var("XDG_DATA_HOME", &home);
    std::env::set_var("ANIMA_ASSETS_DIR", &missing);

    // An explicit override pointing somewhere that doesn't exist is a
    // mistake worth surfacing, not one to paper over by creating it.
    assert_eq!(anima_engine::asset_library::ensure_asset_root(), None);
    assert!(!missing.exists(), "created the user's explicit override");
    assert!(!home.exists(), "created XDG despite an explicit override");

    // ── A fresh install has no asset directory, and everything that
    // resolves against the library — sprites, scripts, sounds — then
    // has nowhere to look. Scripts made that visible: a character
    // configured in the Inspector silently did nothing.
    std::env::remove_var("ANIMA_ASSETS_DIR");

    let root = anima_engine::asset_library::ensure_asset_root().expect("no root created");
    assert!(root.is_dir(), "returned a path that isn't a directory");
    assert!(
        root.starts_with(&home),
        "created outside the scoped XDG dir"
    );
    // A second call reuses it rather than failing on the existing dir.
    assert_eq!(
        anima_engine::asset_library::ensure_asset_root().as_deref(),
        Some(root.as_path())
    );

    // ── What the UI *tells* the user has to be the directory the app
    // actually reads. The empty state used to hard-code
    // `~/.local/share/animaEngine/assets`, which was wrong twice:
    // `directories` lower-cases the project name on Linux, so the real
    // directory is `animaengine` and following the instruction on a
    // case-sensitive filesystem creates one the app never looks in;
    // and it ignored both env vars below (R32). Asserted here rather
    // than in its own test because these are process-global.
    assert_eq!(
        anima_engine::asset_library::asset_root_hint(),
        root,
        "the hint must name the directory ensure_asset_root just made"
    );
    let name = root
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .expect("root has a named parent");
    assert_eq!(
        name,
        name.to_lowercase(),
        "the real directory name is lower-cased by `directories`; a docs \
         or UI string spelling it otherwise sends the user elsewhere"
    );

    // An explicit override wins, and — unlike `ensure_asset_root` — the
    // hint reports it even when it does not exist yet, because that is
    // exactly when the user needs to be told where to create it.
    std::env::set_var("ANIMA_ASSETS_DIR", &missing);
    assert_eq!(anima_engine::asset_library::asset_root_hint(), missing);
    std::env::remove_var("ANIMA_ASSETS_DIR");

    match prev_data {
        Some(v) => std::env::set_var("XDG_DATA_HOME", v),
        None => std::env::remove_var("XDG_DATA_HOME"),
    }
    match prev_assets {
        Some(v) => std::env::set_var("ANIMA_ASSETS_DIR", v),
        None => std::env::remove_var("ANIMA_ASSETS_DIR"),
    }
    let _ = std::fs::remove_dir_all(&home);
}
