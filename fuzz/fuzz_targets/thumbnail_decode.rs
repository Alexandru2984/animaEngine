//! Fuzz the scene-picture decoder (`scene_thumb::decode`): a PNG from a
//! shared file, refused past its byte cap or a side over 512 before any
//! pixel is decoded. The PNG decoder is upstream's; the caps around it
//! are ours.

#![no_main]

use anima_engine::scene_thumb::decode;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = decode(data);
});
