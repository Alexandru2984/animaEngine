//! Fuzz the on-disk decoded-frame cache reader (`cache::deserialize_frames`).
//!
//! These bytes come straight off disk from
//! `~/.cache/animaEngine/textures/<hash>.bin`. The cache dir is the
//! user's own, but a corrupt or truncated file (crash mid-write, a
//! tampered cache, a format skew across versions) must yield a typed
//! error, never a panic or an out-of-bounds read. The reader has its
//! own length/dimension caps; this enforces panic-freedom over the
//! whole hand-rolled binary format.

#![no_main]

use anima_engine::animation::cache::deserialize_frames;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = deserialize_frames(data);
});
