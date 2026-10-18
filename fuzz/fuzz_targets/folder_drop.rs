//! Fuzz a dropped folder (1.5): built from the input, two bytes an
//! entry — PNG frames, broken ones, named pipes, links out and back,
//! folders named like pictures, other files, thousands of empty frames —
//! then checked, typed, measured and decoded as a sequence
//! (`loader::fuzz_folder_drop`). A pipe or a link to `/dev/zero` that is
//! opened hangs a run: run with `-timeout`. In a RAM-backed directory.

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
        let dir = base.join(format!("anima-fuzz-folder-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("fuzz dir");
        dir
    })
}

fuzz_target!(|data: &[u8]| {
    anima_engine::animation::loader::fuzz_folder_drop(dir(), data);
});
