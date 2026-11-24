//! Fuzz a shared scene, character or backup all the way in, as far as it
//! goes before anything of the user's is touched: `scene_file::import`
//! and `backup::read`, which also unpack the pictures into a folder of
//! their own and check each as a drop (TEST-001: `scene_file_read` stops
//! short of writing). Never `put_in_place`, which writes the user's
//! scenes and settings. A RAM-backed directory; the folder unpacked into
//! is taken away after each input.

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::path::PathBuf;
use std::sync::OnceLock;

fn dir() -> &'static PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let shm = std::path::Path::new("/dev/shm");
        let base = if shm.is_dir() {
            shm.to_path_buf()
        } else {
            std::env::temp_dir()
        };
        let dir = base.join(format!("anima-fuzz-import-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("fuzz dir");
        dir
    })
}

fuzz_target!(|data: &[u8]| {
    let file = dir().join("in.animascene");
    if std::fs::write(&file, data).is_err() {
        return;
    }
    let into = dir().join("unpacked");
    let _ = std::fs::remove_dir_all(&into);
    let _ = anima_engine::scene_file::import(&file, &into);
    let _ = std::fs::remove_dir_all(&into);
    let _ = anima_engine::scene_file::backup::read(&file, &into);
    let _ = std::fs::remove_dir_all(&into);
});
