//! Fuzz the Shimeji `actions.xml` parser (`shimeji::fuzz_parse_actions`).
//!
//! `actions.xml` ships inside third-party mascot packs — the most
//! "downloaded a zip off the internet" input surface in the app. The
//! parser runs on quick-xml (no DTD/entity expansion by construction)
//! with depth/attribute caps on top; this enforces panic-freedom over
//! adversarial markup (deep nesting, giant attributes, truncated tags,
//! non-UTF-8 already rejected upstream so we feed valid UTF-8 here).

#![no_main]

use anima_engine::shimeji::fuzz_parse_actions;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        fuzz_parse_actions(s);
    }
});
