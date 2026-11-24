//! Fuzz a sound as the app loads one (`audio::load_sound`, through its
//! fuzz entry point): the decoders behind `rodio` — WAV, Vorbis, FLAC,
//! MP3 — under the size, length and rate caps, a decoder's panic isolated
//! (TEST-001: never fuzzed — BUG-002's divide by zero was found by hand).
//! The first byte picks the file's name, the rest is the file, written to
//! a RAM-backed directory that stands for the library. A decoder's panic
//! the app isolates is a failed load here too; what aborts is a panic
//! outside it, a hang (`-timeout`) or runaway allocation.

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::path::PathBuf;
use std::sync::OnceLock;

const NAMES: [&str; 5] = ["s.wav", "s.ogg", "s.flac", "s.mp3", "s"];

fn dir() -> &'static PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let shm = std::path::Path::new("/dev/shm");
        let base = if shm.is_dir() {
            shm.to_path_buf()
        } else {
            std::env::temp_dir()
        };
        let dir = base.join(format!("anima-fuzz-audio-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("fuzz dir");
        dir
    })
}

/// libFuzzer's panic hook aborts on every panic; one the app isolates
/// (`crash::isolate`, around the decoders) is a sound failing to load, as
/// in the app, and is let go — any other still aborts.
fn let_isolated_panics_go() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let fuzzer = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if !anima_engine::crash::fuzz_isolated() {
                fuzzer(info);
            }
        }));
    });
}

fuzz_target!(|data: &[u8]| {
    let_isolated_panics_go();
    let Some((&pick, bytes)) = data.split_first() else {
        return;
    };
    let name = NAMES[usize::from(pick) % NAMES.len()];
    if std::fs::write(dir().join(name), bytes).is_err() {
        return;
    }
    let _ = anima_engine::audio::fuzz_load_sound(dir(), name);
});
