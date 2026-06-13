//! Fuzz the AVCC → Annex-B NALU walk (`video_loader::avcc_to_annex_b`).
//!
//! One MP4 sample's bytes are an untrusted sequence of length-prefixed
//! NALUs. The walk is a hand-written length/offset parser; a malformed
//! length must bail safely (the frame is skipped), never index out of
//! bounds or loop forever. Output buffer reused across calls like the
//! real loop.

#![no_main]

use anima_engine::animation::video_loader::avcc_to_annex_b;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut out = Vec::new();
    avcc_to_annex_b(data, &mut out);
});
