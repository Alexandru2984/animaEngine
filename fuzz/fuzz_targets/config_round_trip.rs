//! Fuzz `config.toml` reading and writing (1.5): whatever reads, written
//! and read again, reads and writes the same — set-aside entries and
//! settings, unknown keys and all (`config::fuzz_config_round_trip`).

#![no_main]

use anima_engine::config::fuzz_config_round_trip;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        fuzz_config_round_trip(s);
    }
});
