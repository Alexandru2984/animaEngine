//! Smoke test: run `demo::generate_assets()` against a clean state and
//! verify every advertised demo character produces a frame_001.png with
//! the expected dimensions. This is the closest we can get to "I ran the
//! app and saw five characters appear" without spinning up a display.
//!
//! Both checks live in one test so cargo's parallel runner doesn't race
//! two `set_current_dir` calls against each other.

use std::path::Path;

const DEMO_DIRS: &[(&str, &str)] = &[
    ("assets/demo/ghost", "frame_001.png"),
    ("assets/demo/slime", "frame_001.png"),
    ("assets/demo/heart", "frame_001.png"),
    ("assets/demo/star", "frame_001.png"),
    ("assets/demo/cat", "frame_001.png"),
];

/// Sentinel size that all demo characters target. If a generator silently
/// drops to a smaller size the test catches it.
const EXPECTED_PX: u32 = 128;

#[test]
fn generate_assets_full_smoke() {
    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    std::env::set_current_dir(&workspace).unwrap();

    // ── Step 1: clean state → generate → expect five characters at 128 px.
    let _ = std::fs::remove_dir_all(workspace.join("assets/demo"));
    anima_engine::demo::generate_assets();

    for (dir, first_frame) in DEMO_DIRS {
        let frame_path = workspace.join(dir).join(first_frame);
        assert!(
            frame_path.exists(),
            "missing first frame for demo char: {}",
            frame_path.display()
        );
        let (w, h) = image::image_dimensions(&frame_path)
            .unwrap_or_else(|e| panic!("can't read dims of {}: {e}", frame_path.display()));
        assert_eq!(w, EXPECTED_PX, "{}: width", dir);
        assert_eq!(h, EXPECTED_PX, "{}: height", dir);
    }

    // ── Step 2: idempotency. A second call must not rewrite anything.
    let baseline: Vec<_> = DEMO_DIRS
        .iter()
        .map(|(dir, frame)| {
            std::fs::metadata(Path::new(dir).join(frame))
                .unwrap()
                .modified()
                .unwrap()
        })
        .collect();

    // Filesystem mtime resolution can be 1 s — sleep enough to guarantee
    // a rewrite would land on a fresh second.
    std::thread::sleep(std::time::Duration::from_millis(1200));

    anima_engine::demo::generate_assets();
    for ((dir, frame), prev) in DEMO_DIRS.iter().zip(baseline) {
        let now = std::fs::metadata(Path::new(dir).join(frame))
            .unwrap()
            .modified()
            .unwrap();
        assert_eq!(
            prev, now,
            "demo at {dir} was rewritten on a no-op generate_assets() call"
        );
    }
}
