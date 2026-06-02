//! MP4 / H.264 video loader.
//!
//! Decodes the video track of an MP4 file to a `Vec<Frame>`, ignoring any
//! audio track. The output mirrors what GIF / WebP loaders produce: each
//! frame carries its own `delay_ms` derived from the source frame rate.
//!
//! Pipeline:
//! 1. `mp4::Mp4Reader` walks the container, locates the H.264 video track.
//! 2. For each sample we emit length-prefixed NALUs to `openh264::Decoder`,
//!    after re-formatting the AVCC headers into Annex-B start codes.
//! 3. Decoded YUV420 frames are converted to packed RGBA8.
//!
//! ## Limits
//! - **H.264 only.** WebM/VP9 and HEVC are explicitly unsupported.
//! - **Pre-decoded into memory.** Bounded by `constants::MAX_VIDEO_FRAMES`
//!   (≈20 s at 30 fps) so a misclick on a feature film doesn't OOM the app.
//! - **Audio dropped.** We don't even look at audio tracks.

use super::frame::Frame;
use crate::constants::{MAX_DECODED_ASSET_BYTES, MAX_IMAGE_DIM, MAX_VIDEO_FRAMES};
use crate::error::{AnimaError, Result};
use mp4::Mp4Reader;
use openh264::decoder::Decoder;
use openh264::formats::YUVSource;
use std::fs::File;
use std::io::{BufReader, Read, Seek};
use std::path::Path;

/// Annex-B start code used by openh264 between NALUs.
const ANNEX_B_START: &[u8] = &[0, 0, 0, 1];

/// Load an MP4 video and return its decoded RGBA frames.
pub fn load_video(path: &Path) -> Result<Vec<Frame>> {
    let file = File::open(path)?;
    let size = file.metadata()?.len();
    let reader = BufReader::new(file);
    let mut mp4 = Mp4Reader::read_header(reader, size)
        .map_err(|e| AnimaError::VideoDecode(format!("MP4 header: {e}")))?;

    // Find the first video track. We don't care which subtype it is — the
    // decoder will reject anything that isn't H.264.
    let (track_id, track) = mp4
        .tracks()
        .iter()
        .find(|(_, t)| matches!(t.media_type(), Ok(mp4::MediaType::H264)))
        .ok_or_else(|| AnimaError::VideoDecode("no H.264 video track found".into()))?;
    let track_id = *track_id;

    let sample_count = track.sample_count();
    let timescale = track.timescale();
    if sample_count == 0 {
        return Err(AnimaError::VideoDecode(
            "video track has zero samples".into(),
        ));
    }
    if timescale == 0 {
        return Err(AnimaError::VideoDecode(
            "video track has zero timescale".into(),
        ));
    }

    // Average fps lets us fill in `delay_ms` when the source has variable
    // sample durations. We still prefer per-sample timing when available.
    let total_duration = track.duration().as_secs_f64().max(1e-6);
    let avg_fps = sample_count as f64 / total_duration;
    let avg_delay_ms = (1000.0 / avg_fps).round() as u32;

    // openh264 wants Annex-B (`00 00 00 01` start codes). MP4 stores NALUs
    // length-prefixed (4-byte big-endian). The SPS/PPS live in the avcC box
    // and must be fed before the first IDR.
    let mut decoder =
        Decoder::new().map_err(|e| AnimaError::VideoDecode(format!("openh264 init: {e}")))?;

    let mut annex_b = Vec::with_capacity(64 * 1024);
    push_sps_pps_from_avcc(&mp4, track_id, &mut annex_b)?;

    let mut frames: Vec<Frame> = Vec::new();
    let mut total_bytes: usize = 0;

    for sample_id in 1..=sample_count {
        if frames.len() >= MAX_VIDEO_FRAMES {
            tracing::warn!(
                "Video {} truncated at {} frames (cap = {})",
                path.display(),
                frames.len(),
                MAX_VIDEO_FRAMES
            );
            break;
        }

        let Some(sample) = mp4
            .read_sample(track_id, sample_id)
            .map_err(|e| AnimaError::VideoDecode(format!("sample {sample_id}: {e}")))?
        else {
            continue;
        };

        annex_b.clear();
        avcc_to_annex_b(&sample.bytes, &mut annex_b);

        match decoder.decode(&annex_b) {
            Ok(Some(yuv)) => {
                let (w, h) = yuv.dimensions();
                // Sanity-check what openh264 just handed us. A pathological
                // stream could claim huge dimensions; refuse before alloc.
                if w as u32 > MAX_IMAGE_DIM || h as u32 > MAX_IMAGE_DIM {
                    return Err(AnimaError::ImageTooLarge {
                        width: w as u32,
                        height: h as u32,
                        max: MAX_IMAGE_DIM,
                    });
                }
                let frame_bytes = w
                    .checked_mul(h)
                    .and_then(|n| n.checked_mul(4))
                    .ok_or_else(|| AnimaError::VideoDecode("frame size overflow".into()))?;
                if total_bytes.saturating_add(frame_bytes) > MAX_DECODED_ASSET_BYTES {
                    tracing::warn!(
                        "Video {} truncated at MAX_DECODED_ASSET_BYTES = {} MB ({} frames kept)",
                        path.display(),
                        MAX_DECODED_ASSET_BYTES / (1024 * 1024),
                        frames.len()
                    );
                    break;
                }
                total_bytes += frame_bytes;

                let mut rgba = vec![0u8; frame_bytes];
                yuv_to_rgba(&yuv, &mut rgba);
                frames.push(Frame::with_delay(rgba, w as u32, h as u32, avg_delay_ms));
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!("openh264 decode error on sample {sample_id}: {e}");
            }
        }
    }

    if frames.is_empty() {
        return Err(AnimaError::EmptyAsset(path.to_path_buf()));
    }

    tracing::info!(
        "Loaded video {}: {} frames, {}×{}, ~{:.1} fps",
        path.display(),
        frames.len(),
        frames[0].width,
        frames[0].height,
        avg_fps
    );
    Ok(frames)
}

/// Convert one length-prefixed AVCC bitstream (the MP4 sample format) into
/// Annex-B (start-code prefixed) for openh264.
fn avcc_to_annex_b(input: &[u8], out: &mut Vec<u8>) {
    let mut cursor = 0;
    while cursor + 4 <= input.len() {
        let nal_len = u32::from_be_bytes([
            input[cursor],
            input[cursor + 1],
            input[cursor + 2],
            input[cursor + 3],
        ]) as usize;
        cursor += 4;
        if cursor + nal_len > input.len() {
            // Malformed sample — bail; the decoder will skip this frame.
            break;
        }
        out.extend_from_slice(ANNEX_B_START);
        out.extend_from_slice(&input[cursor..cursor + nal_len]);
        cursor += nal_len;
    }
}

/// Extract SPS / PPS parameter sets from the `avcC` configuration box and
/// emit them as Annex-B so the decoder is primed before the first IDR.
fn push_sps_pps_from_avcc<R: Read + Seek>(
    mp4: &Mp4Reader<R>,
    track_id: u32,
    out: &mut Vec<u8>,
) -> Result<()> {
    let track = mp4
        .tracks()
        .get(&track_id)
        .ok_or_else(|| AnimaError::VideoDecode(format!("track {track_id} vanished")))?;

    let trak = &track.trak;
    let avc1 = trak
        .mdia
        .minf
        .stbl
        .stsd
        .avc1
        .as_ref()
        .ok_or_else(|| AnimaError::VideoDecode("no avc1 sample entry".into()))?;
    let avcc = &avc1.avcc;

    for sps in &avcc.sequence_parameter_sets {
        out.extend_from_slice(ANNEX_B_START);
        out.extend_from_slice(&sps.bytes);
    }
    for pps in &avcc.picture_parameter_sets {
        out.extend_from_slice(ANNEX_B_START);
        out.extend_from_slice(&pps.bytes);
    }
    Ok(())
}

/// YUV420 → RGBA. BT.601 limited-range coefficients (the safe default for
/// SD/web video; HD content tagged BT.709 looks slightly desaturated but
/// not broken).
fn yuv_to_rgba(yuv: &impl YUVSource, out: &mut [u8]) {
    let (w, h) = yuv.dimensions();
    let (y_stride, u_stride, v_stride) = yuv.strides();
    let y_plane = yuv.y();
    let u_plane = yuv.u();
    let v_plane = yuv.v();

    for row in 0..h {
        for col in 0..w {
            let y = y_plane[row * y_stride + col] as f32;
            let u = u_plane[(row / 2) * u_stride + (col / 2)] as f32 - 128.0;
            let v = v_plane[(row / 2) * v_stride + (col / 2)] as f32 - 128.0;

            // BT.601 limited range.
            let r = (1.164 * (y - 16.0) + 1.596 * v).clamp(0.0, 255.0) as u8;
            let g = (1.164 * (y - 16.0) - 0.392 * u - 0.813 * v).clamp(0.0, 255.0) as u8;
            let b = (1.164 * (y - 16.0) + 2.017 * u).clamp(0.0, 255.0) as u8;

            let idx = (row * w + col) * 4;
            out[idx] = r;
            out[idx + 1] = g;
            out[idx + 2] = b;
            out[idx + 3] = 255;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_errors() {
        let err = load_video(Path::new("/nonexistent/no_such_file.mp4")).unwrap_err();
        assert!(
            matches!(err, AnimaError::Io(_)),
            "expected IO error, got {err:?}"
        );
    }

    #[test]
    fn avcc_to_annex_b_handles_two_nalus() {
        // Two minimal NALUs (length 2 + length 1) back-to-back.
        let input = [
            0, 0, 0, 2, 0xAA, 0xBB, // first NALU: 2 bytes payload
            0, 0, 0, 1, 0xCC, // second NALU: 1 byte payload
        ];
        let mut out = Vec::new();
        avcc_to_annex_b(&input, &mut out);
        assert_eq!(
            out,
            vec![0, 0, 0, 1, 0xAA, 0xBB, 0, 0, 0, 1, 0xCC],
            "expected two annex-b-prefixed NALUs"
        );
    }

    #[test]
    fn avcc_to_annex_b_truncated_input_bails_safely() {
        // Length says 10 but only 2 bytes follow → must not panic.
        let input = [0, 0, 0, 10, 0xAA, 0xBB];
        let mut out = Vec::new();
        avcc_to_annex_b(&input, &mut out);
        assert!(out.is_empty(), "truncated input should produce nothing");
    }
}
