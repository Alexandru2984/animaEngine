//! Fuzz the rebindable-keyboard chord string parser.
//!
//! `KeyChord::FromStr` is invoked on every entry in the persisted
//! `[keybindings.map]` config table. Random user input (hand-edited
//! config.toml) flows directly into this parser. The invariant is
//! "parse returns `Ok` or a typed error; never panics" — anything
//! else here would be a config-load crash.

#![no_main]

use anima_engine::keybindings::KeyChord;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };
    let _ = s.parse::<KeyChord>();
});
