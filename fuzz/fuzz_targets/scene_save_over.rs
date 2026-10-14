//! Fuzz a named scene saved over its own file (1.5): whatever reads,
//! saved over the text it came from, reads the same, leaves the same out,
//! keeps every key a scene does not read, and saved again does not change
//! (`scenes::fuzz_scene_save_over`).

#![no_main]

use anima_engine::scenes::fuzz_scene_save_over;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        fuzz_scene_save_over(s);
    }
});
