//! Fuzz what a dropped file's bytes decide (1.5): the type known by its
//! first bytes where its name says otherwise (`loader::content_format`),
//! the drop's checks, the dimension check, and the decode, failures said
//! in the user's words. The first byte picks the file's name — a
//! picture's extension, a video's, an upper-case one, none — the rest is
//! the file. Written to a RAM-backed directory, one file at a time; the
//! frame cache, which would be the real one, is never reached.

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::path::PathBuf;
use std::sync::OnceLock;

const NAMES: [&str; 8] = [
    "a.png", "a.jpg", "a.jpeg", "a.gif", "a.webp", "a.mp4", "a", "a.PNG",
];

fn dir() -> &'static PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let shm = std::path::Path::new("/dev/shm");
        let base = if shm.is_dir() {
            shm.to_path_buf()
        } else {
            std::env::temp_dir()
        };
        let dir = base.join(format!("anima-fuzz-sniff-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("fuzz dir");
        dir
    })
}

fuzz_target!(|data: &[u8]| {
    let Some((&pick, bytes)) = data.split_first() else {
        return;
    };
    let name = NAMES[usize::from(pick) % NAMES.len()];
    anima_engine::animation::loader::fuzz_content_sniff(dir(), name, bytes);
});
