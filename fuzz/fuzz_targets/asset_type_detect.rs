//! Fuzz the asset-type detector that decides how to decode a dropped
//! or library file.
//!
//! Every dropped file path flows through `detect_asset_type` before
//! we touch its bytes. The function inspects the extension and
//! optionally peeks at magic bytes; either path on an adversarial
//! input must not panic — the validation gate downstream catches
//! anything we couldn't classify.

#![no_main]

use anima_engine::animation::loader::detect_asset_type;
use libfuzzer_sys::fuzz_target;
use std::path::Path;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };
    let _ = detect_asset_type(Path::new(s));
});
