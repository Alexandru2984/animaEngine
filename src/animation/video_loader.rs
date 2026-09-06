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
use crate::constants::{
    MAX_DECODED_ASSET_BYTES, MAX_IMAGE_DIM, MAX_VIDEO_FRAMES, MAX_VIDEO_SAMPLES,
};
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

    // Kept separate from the per-sample buffer: the loop below clears
    // that buffer every iteration, which used to wipe the primer before
    // the decoder ever saw it — avcC-only files (no in-band SPS/PPS,
    // i.e. most of them) decoded zero frames. Caught by
    // `decode_round_trip_through_real_decoder`.
    let mut primer = Vec::new();
    push_sps_pps_from_avcc(&mp4, track_id, &mut primer)?;
    // Prefix width the samples are actually encoded with — read once,
    // not assumed.
    let nal_len_size = nal_length_size(&mp4, track_id);

    let mut annex_b = Vec::with_capacity(64 * 1024);

    let mut frames: Vec<Frame> = Vec::new();
    let mut total_bytes: usize = 0;

    // Bound the *attempts*, not just the frames kept: a file with millions
    // of (mostly non-decodable) samples would otherwise feed the in-process
    // C decoder millions of times. Real clips finish well under this.
    let sample_limit = sample_count.min(MAX_VIDEO_SAMPLES);
    if sample_count > sample_limit {
        tracing::warn!(
            "Video has {sample_count} samples; only attempting the first {sample_limit}"
        );
    }
    for sample_id in 1..=sample_limit {
        if frames.len() >= MAX_VIDEO_FRAMES {
            // Redact at warn!, full path at debug! — matches the M4/G.1
            // log-redaction convention used by the other loaders.
            tracing::warn!(
                "Video {} truncated at {} frames (cap = {})",
                crate::drop_validate::redact_path(path),
                frames.len(),
                MAX_VIDEO_FRAMES
            );
            tracing::debug!("Truncated video full path: {}", path.display());
            break;
        }

        let Some(sample) = mp4
            .read_sample(track_id, sample_id)
            .map_err(|e| AnimaError::VideoDecode(format!("sample {sample_id}: {e}")))?
        else {
            continue;
        };

        annex_b.clear();
        if sample_id == 1 {
            annex_b.extend_from_slice(&primer);
        }
        avcc_to_annex_b(&sample.bytes, nal_len_size, &mut annex_b);

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
                        crate::drop_validate::redact_path(path),
                        MAX_DECODED_ASSET_BYTES / (1024 * 1024),
                        frames.len()
                    );
                    tracing::debug!("Truncated video full path: {}", path.display());
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
        crate::drop_validate::redact_path(path),
        frames.len(),
        frames[0].width,
        frames[0].height,
        avg_fps
    );
    tracing::debug!("Loaded video full path: {}", path.display());
    Ok(frames)
}

/// Convert one length-prefixed AVCC bitstream (the MP4 sample format) into
/// Annex-B (start-code prefixed) for openh264.
///
/// `nal_length_size` is the prefix width in bytes, from the avcC box's
/// `lengthSizeMinusOne + 1` — legally 1, 2 or 4. This used to be hard-
/// coded to 4; a file using a narrower prefix (rare, but valid — every
/// mainstream encoder emits 4) read its lengths as garbage, failed the
/// bounds check below and lost every frame. Anything outside {1, 2, 4}
/// is not a legal avcC value, so it falls back to 4.
///
/// `pub` (hidden) for the W.4 fuzz target — the NALU walk is a hand-
/// written length-prefix parser over untrusted MP4 sample bytes.
#[doc(hidden)]
pub fn avcc_to_annex_b(input: &[u8], nal_length_size: u8, out: &mut Vec<u8>) {
    let size = match nal_length_size {
        1 | 2 | 4 => nal_length_size as usize,
        _ => 4,
    };
    let mut cursor = 0;
    while cursor + size <= input.len() {
        // Big-endian over `size` bytes. Accumulating avoids a separate
        // branch per width and can't overflow: size ≤ 4, so at most a
        // u32's worth lands in a usize.
        let mut nal_len: usize = 0;
        for b in &input[cursor..cursor + size] {
            nal_len = (nal_len << 8) | *b as usize;
        }
        cursor += size;
        if nal_len == 0 || cursor + nal_len > input.len() {
            // Malformed sample — bail; the decoder will skip this frame.
            break;
        }
        out.extend_from_slice(ANNEX_B_START);
        out.extend_from_slice(&input[cursor..cursor + nal_len]);
        cursor += nal_len;
    }
}

/// NALU length prefix width for `track_id`, from its avcC box.
///
/// Falls back to 4 when the box can't be read — the near-universal value,
/// and the one this walker assumed unconditionally before.
fn nal_length_size<R: Read + Seek>(mp4: &Mp4Reader<R>, track_id: u32) -> u8 {
    mp4.tracks()
        .get(&track_id)
        .and_then(|t| t.trak.mdia.minf.stbl.stsd.avc1.as_ref())
        .map(|avc1| avc1.avcc.length_size_minus_one + 1)
        .unwrap_or(4)
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
        avcc_to_annex_b(&input, 4, &mut out);
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
        avcc_to_annex_b(&input, 4, &mut out);
        assert!(out.is_empty(), "truncated input should produce nothing");
    }

    #[test]
    fn avcc_to_annex_b_honours_narrow_length_prefixes() {
        // avcC legally allows 1- and 2-byte prefixes. Hard-coding 4 read
        // these as garbage lengths and dropped every frame.
        let one = [2u8, 0xAA, 0xBB, 1, 0xCC];
        let mut out = Vec::new();
        avcc_to_annex_b(&one, 1, &mut out);
        assert_eq!(out, vec![0, 0, 0, 1, 0xAA, 0xBB, 0, 0, 0, 1, 0xCC]);

        let two = [0u8, 2, 0xAA, 0xBB, 0, 1, 0xCC];
        out.clear();
        avcc_to_annex_b(&two, 2, &mut out);
        assert_eq!(out, vec![0, 0, 0, 1, 0xAA, 0xBB, 0, 0, 0, 1, 0xCC]);

        // The same bytes read as 4-byte prefixes are nonsense — this is
        // exactly the frame loss the old hard-coding caused.
        out.clear();
        avcc_to_annex_b(&two, 4, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn avcc_to_annex_b_rejects_illegal_prefix_width() {
        // 3 is not a legal lengthSizeMinusOne+1; fall back to 4 rather
        // than trusting a corrupt container field.
        let input = [0, 0, 0, 2, 0xAA, 0xBB];
        let mut out = Vec::new();
        avcc_to_annex_b(&input, 3, &mut out);
        assert_eq!(out, vec![0, 0, 0, 1, 0xAA, 0xBB]);
    }

    /// Split an Annex-B stream into NALUs (3- and 4-byte start codes).
    fn split_annex_b(stream: &[u8]) -> Vec<&[u8]> {
        let mut payload_starts = Vec::new();
        let mut i = 0;
        while i < stream.len() {
            if stream[i..].starts_with(&[0, 0, 0, 1]) {
                payload_starts.push(i + 4);
                i += 4;
            } else if stream[i..].starts_with(&[0, 0, 1]) {
                payload_starts.push(i + 3);
                i += 3;
            } else {
                i += 1;
            }
        }
        let mut nalus = Vec::with_capacity(payload_starts.len());
        for (idx, &start) in payload_starts.iter().enumerate() {
            let end = payload_starts
                .get(idx + 1)
                .map(|&next| {
                    next - if stream[next - 4..].starts_with(&[0, 0, 0, 1]) {
                        4
                    } else {
                        3
                    }
                })
                .unwrap_or(stream.len());
            nalus.push(&stream[start..end]);
        }
        nalus
    }

    /// The one component nothing else exercises: the openh264 *decoder*
    /// FFI itself. The fixture is built programmatically (repo policy:
    /// no committed binaries) — solid-color frames through openh264's
    /// encoder, muxed into a real MP4 by mp4::Mp4Writer, then loaded
    /// back through the full `load_video` pipeline.
    #[test]
    fn decode_round_trip_through_real_decoder() {
        use mp4::{
            AvcConfig, MediaConfig, Mp4Config, Mp4Sample, Mp4Writer, TrackConfig, TrackType,
        };
        use openh264::encoder::Encoder;
        use openh264::formats::YUVBuffer;

        const W: usize = 32;
        const H: usize = 32;
        // Limited-range BT.601 triples for pure red / green / blue —
        // the exact inverse of the constants in `yuv_to_rgba`.
        const YUV_COLORS: [(u8, u8, u8); 3] = [(81, 90, 240), (145, 54, 34), (41, 240, 110)];

        let mut encoder = Encoder::new().expect("openh264 encoder init");
        let mut sps: Option<Vec<u8>> = None;
        let mut pps: Option<Vec<u8>> = None;
        // (avcc_bytes, contains_idr) per encoded frame.
        let mut samples: Vec<(Vec<u8>, bool)> = Vec::new();

        for &(y, u, v) in &YUV_COLORS {
            let mut planes = vec![y; W * H];
            planes.extend(std::iter::repeat_n(u, W * H / 4));
            planes.extend(std::iter::repeat_n(v, W * H / 4));
            let bitstream = encoder
                .encode(&YUVBuffer::from_vec(planes, W, H))
                .expect("encode frame")
                .to_vec();

            let mut avcc = Vec::new();
            let mut is_sync = false;
            for nalu in split_annex_b(&bitstream) {
                match nalu.first().map(|b| b & 0x1F) {
                    Some(7) => sps = Some(nalu.to_vec()),
                    Some(8) => pps = Some(nalu.to_vec()),
                    other => {
                        if other == Some(5) {
                            is_sync = true;
                        }
                        avcc.extend_from_slice(&(nalu.len() as u32).to_be_bytes());
                        avcc.extend_from_slice(nalu);
                    }
                }
            }
            samples.push((avcc, is_sync));
        }

        let dir =
            std::env::temp_dir().join(format!("anima_video_roundtrip_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("roundtrip.mp4");

        let file = std::fs::File::create(&path).unwrap();
        let mut writer = Mp4Writer::write_start(
            file,
            &Mp4Config {
                major_brand: "isom".parse().unwrap(),
                minor_version: 512,
                compatible_brands: vec!["isom".parse().unwrap(), "avc1".parse().unwrap()],
                timescale: 1000,
            },
        )
        .unwrap();
        writer
            .add_track(&TrackConfig {
                track_type: TrackType::Video,
                timescale: 1000,
                language: "und".into(),
                media_conf: MediaConfig::AvcConfig(AvcConfig {
                    width: W as u16,
                    height: H as u16,
                    seq_param_set: sps.expect("encoder emitted SPS"),
                    pic_param_set: pps.expect("encoder emitted PPS"),
                }),
            })
            .unwrap();
        for (i, (bytes, is_sync)) in samples.into_iter().enumerate() {
            writer
                .write_sample(
                    1,
                    &Mp4Sample {
                        start_time: i as u64 * 100,
                        duration: 100,
                        rendering_offset: 0,
                        is_sync,
                        bytes: mp4::Bytes::from(bytes),
                    },
                )
                .unwrap();
        }
        writer.write_end().unwrap();

        let frames = load_video(&path).expect("round-trip decode");
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(frames.len(), 3, "three frames in, three frames out");
        // Solid-color H.264 at 4:2:0 decodes near-exactly; the dominant
        // channel must dominate by a wide margin at the center pixel.
        let expected_dominant = [0usize, 1, 2]; // R, G, B per frame
        for (frame, &dom) in frames.iter().zip(&expected_dominant) {
            assert_eq!((frame.width, frame.height), (W as u32, H as u32));
            let center = ((H / 2) * W + W / 2) * 4;
            let px = &frame.rgba[center..center + 4];
            assert!(px[dom] > 180, "dominant channel {dom} too weak: {px:?}");
            for ch in 0..3 {
                if ch != dom {
                    assert!(px[ch] < 80, "channel {ch} should be near zero: {px:?}");
                }
            }
            assert_eq!(px[3], 255, "alpha must be opaque");
        }
    }
}
