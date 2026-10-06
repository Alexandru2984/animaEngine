//! Fuzz what a shared scene, a shared character or a backup goes through
//! on the way in (`scene_file::fuzz_read`): the hand-written stored-zip
//! reader, every entry name check, both kinds of manifest parsed and
//! tidied, and the scene pictures decoded under caps. These files come
//! from other people by design — the newest untrusted surface (1.5).

#![no_main]

use anima_engine::scene_file::fuzz_read;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_read(data);
});
