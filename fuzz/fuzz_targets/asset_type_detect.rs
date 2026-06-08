//! Fuzz the asset-type detector + drop pre-validator that decides
//! whether a dropped or library file even reaches a decoder.
//!
//! Every dropped file path flows through `detect_asset_type` and
//! then `pre_validate_dropped_file` (F.1, 0.5.1) before we touch
//! the bytes. F.8 expanded this target to cover both functions so
//! the pre-validator's invariant (always returns `Ok` or `Err`,
//! never panics) is enforced under fuzz.

#![no_main]

use anima_engine::animation::loader::detect_asset_type;
use anima_engine::drop_validate::pre_validate_dropped_file;
use libfuzzer_sys::fuzz_target;
use std::path::Path;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };
    let path = Path::new(s);
    let _ = detect_asset_type(path);
    // pre_validate_dropped_file calls fs::metadata, so the
    // overwhelming majority of fuzz inputs return `Err` quickly.
    // What we want to assert is panic-freedom on path shapes the
    // syscall might hand back — embedded NUL, very long paths,
    // multi-byte sequences cut mid-codepoint, etc.
    let _ = pre_validate_dropped_file(path);
});
