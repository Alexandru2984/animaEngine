//! Fuzz the `text/uri-list` parser used by the native Wayland
//! drag-drop path.
//!
//! Drop payloads cross a process boundary (file manager → wayland
//! socket → our worker thread) so any panic on bad input would crash
//! the overlay on the first malformed drag. Invariants checked:
//!
//! 1. Never panics on any byte sequence.
//! 2. Output stays under `MAX_URI_LIST_PATHS` (F.5, 0.5.1). F.8
//!    upgraded this fuzz target from "panic-free only" to "panic-free
//!    AND bounded" — a million-line payload must not produce a
//!    million-PathBuf vector.

#![no_main]

use anima_engine::wayland::data_device::{parse_uri_list, MAX_URI_LIST_PATHS};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let paths = parse_uri_list(data);
    assert!(
        paths.len() <= MAX_URI_LIST_PATHS,
        "URI-list parser exceeded MAX_URI_LIST_PATHS ({} > {})",
        paths.len(),
        MAX_URI_LIST_PATHS,
    );
});
