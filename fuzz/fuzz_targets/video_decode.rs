//! Fuzz an MP4 as the app opens one (`video_loader::load_video`): the
//! `mp4` demuxer, the sample walk, the AVCC → Annex-B rewrite and the
//! H.264 decoder behind it, under the size and frame caps (TEST-001: the
//! one untrusted parser left unfuzzed — `content_sniff` skips video).
//! Written to a RAM-backed directory, one file at a time; nothing else is
//! touched. The decoder is C++, not instrumented by the sanitizer here:
//! what this finds is panics, hangs (`-timeout`) and runaway allocation.

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
        let dir = base.join(format!("anima-fuzz-video-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("fuzz dir");
        dir
    })
}

fuzz_target!(|data: &[u8]| {
    let path = dir().join("v.mp4");
    if std::fs::write(&path, data).is_err() {
        return;
    }
    let _ = anima_engine::animation::video_loader::load_video(&path);
});
