//! Fuzz the small readers of what a person types into the config (1.5):
//! colours, HH:MM times, MM-DD dates and the hours they make, sound
//! paths, and lines with placeholders (`speech::fuzz_small_parsers`).

#![no_main]

use anima_engine::speech::fuzz_small_parsers;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        fuzz_small_parsers(s);
    }
});
