//! Fuzz the `text/uri-list` parser used by the native Wayland
//! drag-drop path.
//!
//! Drop payloads cross a process boundary (file manager → wayland
//! socket → our worker thread) so any panic on bad input would crash
//! the overlay on the first malformed drag. Invariant: returns a
//! (possibly empty) `Vec<PathBuf>` without panicking, regardless of
//! the byte sequence.

#![no_main]

use anima_engine::wayland::data_device::parse_uri_list;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = parse_uri_list(data);
});
