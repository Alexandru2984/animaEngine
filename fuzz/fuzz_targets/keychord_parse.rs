//! Fuzz the rebindable-keyboard chord string parser.
//!
//! `KeyChord::FromStr` is invoked on every entry in the persisted
//! `[keybindings.map]` config table. Random user input (hand-edited
//! config.toml) flows directly into this parser. Invariants checked:
//!
//! 1. Never panics on any byte sequence.
//! 2. If parse succeeds, the chord round-trips through its display
//!    form back into an equivalent chord (F.8, 0.5.1 — catches
//!    asymmetries between `FromStr` and `Display`).

#![no_main]

use anima_engine::keybindings::KeyChord;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };
    if let Ok(parsed) = s.parse::<KeyChord>() {
        let serialised = parsed.canonical_str();
        let reparsed: KeyChord = serialised
            .parse()
            .expect("canonical_str output must round-trip");
        assert_eq!(parsed, reparsed, "round-trip diverged for input {s:?}");
    }
});
