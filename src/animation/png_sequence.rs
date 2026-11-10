use super::frame::Frame;
use crate::constants::{MAX_ANIMATION_FRAMES, MAX_DECODED_ASSET_BYTES, MAX_SEQUENCE_FILES};
use crate::error::{AnimaError, Result};
use rayon::prelude::*;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// At most this many threads decode a sequence. They are the load's own
/// and go when it is done: rayon's global pool would keep one per core
/// for the life of the app — 32, idle, on a 16-core machine — for work
/// that happens at start and on a drop.
const DECODE_THREADS: usize = 8;

/// Load a PNG sequence from a directory.
/// Files are sorted alphabetically — name them frame_001.png, frame_002.png, etc.
///
/// Frames are decoded in parallel via rayon while preserving the sorted order
/// (par_iter().map().collect() guarantees source-order output). A single
/// corrupt PNG is logged and skipped; the whole load only fails if every
/// frame fails.
///
/// Two safety caps:
/// - `MAX_SEQUENCE_FILES` limits the enumeration so a directory with 50 k
///   files doesn't pin the CPU before we even start decoding.
/// - `MAX_DECODED_ASSET_BYTES` truncates after decode so the same cap
///   used by GIF/WebP applies here too.
pub fn load_png_sequence(dir_path: &Path) -> Result<Vec<Frame>> {
    if !dir_path.is_dir() {
        return Err(AnimaError::NotADirectory(dir_path.to_path_buf()));
    }

    // Stream the directory and keep at most MAX_SEQUENCE_FILES + 1 regular
    // `.png` files. The old code collected *every* entry and sorted the
    // whole listing before truncating, so a directory with millions of
    // entries burned CPU + RAM before the cap ever applied. `file_type()`
    // comes from readdir (no symlink follow), and we require a regular
    // file — a FIFO/socket/device named `frame.png` can't slip through to
    // wedge the image probe.
    let mut entries: Vec<std::path::PathBuf> = Vec::new();
    let mut overflowed = false;
    for entry in fs::read_dir(dir_path)? {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("png") {
            continue;
        }
        match entry.file_type() {
            Ok(ft) if ft.is_file() => {}
            _ => continue, // symlink / FIFO / socket / dir / device
        }
        entries.push(path);
        if entries.len() > MAX_SEQUENCE_FILES {
            overflowed = true;
            break;
        }
    }

    entries.sort();

    if entries.is_empty() {
        return Err(AnimaError::EmptyAsset(dir_path.to_path_buf()));
    }

    let truncated_for_file_count = overflowed || entries.len() > MAX_SEQUENCE_FILES;
    if truncated_for_file_count {
        tracing::warn!(
            "PNG sequence {} has {} files; capping at MAX_SEQUENCE_FILES = {}",
            crate::drop_validate::redact_path(dir_path),
            entries.len(),
            MAX_SEQUENCE_FILES
        );
        entries.truncate(MAX_SEQUENCE_FILES);
    }

    // Cap retained frames at MAX_ANIMATION_FRAMES — the same limit the
    // on-disk cache parser enforces on read. MAX_SEQUENCE_FILES (the
    // enumeration cap, deliberately higher) would otherwise let a
    // 601–1000 file sequence decode more frames than the cache accepts,
    // so its cache entry was written and then rejected on every launch,
    // re-decoding the whole sequence each time.
    if entries.len() > MAX_ANIMATION_FRAMES {
        tracing::debug!(
            "PNG sequence {} has {} frames; capping at MAX_ANIMATION_FRAMES = {}",
            crate::drop_validate::redact_path(dir_path),
            entries.len(),
            MAX_ANIMATION_FRAMES
        );
        entries.truncate(MAX_ANIMATION_FRAMES);
    }

    tracing::info!(
        "Loading PNG sequence: {} frames from {} (parallel decode)",
        entries.len(),
        crate::drop_validate::redact_path(dir_path)
    );

    // G.4 (0.5.3): apply `MAX_DECODED_ASSET_BYTES` *during* the
    // parallel decode rather than after. The previous pass collected
    // every frame into memory and only then truncated, so a 1000-file
    // sequence of 4 K RGBA produced ~33 GB peak transient before the
    // post-hoc cap kicked in — a self-DoS path for a library scan.
    // Worker threads check the shared running total before pushing
    // their decoded frame and bail out once the cap would be crossed.
    let total_bytes = AtomicUsize::new(0);
    let truncated_for_bytes = AtomicBool::new(false);
    let decode = || -> Vec<Option<Frame>> {
        entries
            .par_iter()
            .map(|path| {
                if truncated_for_bytes.load(Ordering::Acquire) {
                    return None;
                }
                let frame = match load_single_png(path) {
                    Ok(f) => f,
                    Err(e) => {
                        tracing::warn!(
                            "Failed to load PNG {}: {}",
                            crate::drop_validate::redact_path(path),
                            e
                        );
                        return None;
                    }
                };
                let want = frame.rgba.len();
                // Reserve our slice of the budget atomically. If the
                // reservation crosses the cap, mark the run truncated and
                // discard this frame (preserving ordering of the kept
                // prefix — par_iter().collect() honours source order, so
                // the trailing `None`s end up after any kept frame).
                let prev = total_bytes.fetch_add(want, Ordering::AcqRel);
                if prev.saturating_add(want) > MAX_DECODED_ASSET_BYTES {
                    total_bytes.fetch_sub(want, Ordering::AcqRel);
                    truncated_for_bytes.store(true, Ordering::Release);
                    return None;
                }
                Some(frame)
            })
            .collect()
    };
    let threads = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(DECODE_THREADS)
        .min(entries.len().max(1));
    let decoded = match rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .thread_name(|i| format!("anima-decode-{i}"))
        .build()
    {
        Ok(pool) => pool.install(decode),
        // No threads to be had: rayon's own pool still decodes it.
        Err(e) => {
            tracing::debug!("PNG sequence: no pool of its own ({e})");
            decode()
        }
    };
    // Collapse the per-file results, preserving source order (rayon's
    // collect maps results back to input positions). `None`s are
    // dropped wherever they sit: a corrupt file mid-sequence is
    // *skipped* (deliberate — one bad frame shouldn't kill the whole
    // sequence), and budget-truncated frames usually form a suffix but
    // aren't guaranteed to — workers race for the byte reservation, so
    // a later frame can occasionally win over an earlier one near the
    // cap. Truncation is already a degraded state; we accept the
    // possible mid-sequence gap rather than serialise the decode.
    let frames: Vec<Frame> = decoded.into_iter().flatten().collect();
    if truncated_for_bytes.load(Ordering::Acquire) {
        tracing::warn!(
            "PNG sequence {} truncated at MAX_DECODED_ASSET_BYTES = {} MB ({} frames kept)",
            crate::drop_validate::redact_path(dir_path),
            MAX_DECODED_ASSET_BYTES / (1024 * 1024),
            frames.len()
        );
    }

    if frames.is_empty() {
        return Err(AnimaError::EmptyAsset(dir_path.to_path_buf()));
    }

    Ok(frames)
}

/// One picture's frames (`png_static`): every frame of an animated PNG,
/// each held its own delay as a GIF's are (1.5.1, issue #2), or the one
/// of any other picture.
pub fn load_picture(path: &Path) -> Result<Vec<Frame>> {
    if super::loader::content_format(path) == Some(image::ImageFormat::Png) {
        use image::{AnimationDecoder, ImageDecoder};
        let file = std::io::BufReader::new(fs::File::open(path)?);
        let mut decoder = image::codecs::png::PngDecoder::new(file)?;
        decoder.set_limits(super::loader::decode_limits())?;
        if decoder.is_apng()? {
            let frames = decoder.apng()?.into_frames();
            return super::animated::collect_frames(frames, "APNG", path);
        }
    }
    Ok(vec![load_single_png(path)?])
}

/// Whether `path` is an animated PNG: one whose `acTL` comes before its
/// first image data. Not an error when it cannot be read: it is then no
/// animation.
pub fn is_apng(path: &Path) -> bool {
    fs::File::open(path)
        .ok()
        .and_then(|f| image::codecs::png::PngDecoder::new(std::io::BufReader::new(f)).ok())
        .and_then(|d| d.is_apng().ok())
        .unwrap_or(false)
}

/// Load a single static PNG file
pub fn load_single_png(path: &Path) -> Result<Frame> {
    let img = super::loader::open_image(path)?.to_rgba8();
    let (width, height) = img.dimensions();
    let rgba = img.into_raw();
    Ok(Frame::new(rgba, width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("png_seq_tests")
            .join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Build a PNG with a unique solid color so we can verify frame ordering.
    fn write_colored_png(path: &Path, w: u32, h: u32, color: [u8; 4]) {
        let mut img = image::RgbaImage::new(w, h);
        for px in img.pixels_mut() {
            *px = image::Rgba(color);
        }
        img.save(path).unwrap();
    }

    /// One frame of an animated PNG for [`write_apng`]: where it goes on
    /// the canvas, how long it holds in tenths of a second, its pixels.
    struct ApngFrame {
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        tenths: u16,
        rgba: Vec<u8>,
    }

    /// An animated PNG, written chunk by chunk — no encoder for one is a
    /// dependency. The first frame is the whole `w` × `h` canvas.
    fn write_apng(path: &Path, w: u32, h: u32, frames: &[ApngFrame]) {
        fn crc32(bytes: &[u8]) -> u32 {
            let mut crc = 0xffff_ffffu32;
            for &b in bytes {
                crc ^= u32::from(b);
                for _ in 0..8 {
                    crc = if crc & 1 != 0 {
                        (crc >> 1) ^ 0xedb8_8320
                    } else {
                        crc >> 1
                    };
                }
            }
            !crc
        }
        fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
            out.extend((data.len() as u32).to_be_bytes());
            let mut body = kind.to_vec();
            body.extend(data);
            out.extend(&body);
            out.extend(crc32(&body).to_be_bytes());
        }
        // zlib holding one stored block: each row after filter byte 0.
        fn zlib_stored(w: u32, rgba: &[u8]) -> Vec<u8> {
            let mut raw = Vec::new();
            for row in rgba.chunks(w as usize * 4) {
                raw.push(0);
                raw.extend(row);
            }
            let len = raw.len() as u16;
            let mut z = vec![0x78, 0x01, 0x01];
            z.extend(len.to_le_bytes());
            z.extend((!len).to_le_bytes());
            z.extend(&raw);
            let (mut a, mut b) = (1u32, 0u32);
            for &x in &raw {
                a = (a + u32::from(x)) % 65521;
                b = (b + a) % 65521;
            }
            z.extend(((b << 16) | a).to_be_bytes());
            z
        }
        let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut ihdr = Vec::new();
        ihdr.extend(w.to_be_bytes());
        ihdr.extend(h.to_be_bytes());
        ihdr.extend([8, 6, 0, 0, 0]);
        chunk(&mut out, b"IHDR", &ihdr);
        let mut actl = (frames.len() as u32).to_be_bytes().to_vec();
        actl.extend(0u32.to_be_bytes());
        chunk(&mut out, b"acTL", &actl);
        let mut seq = 0u32;
        for (i, f) in frames.iter().enumerate() {
            let mut fctl = seq.to_be_bytes().to_vec();
            for v in [f.w, f.h, f.x, f.y] {
                fctl.extend(v.to_be_bytes());
            }
            fctl.extend(f.tenths.to_be_bytes());
            fctl.extend(10u16.to_be_bytes());
            fctl.extend([0, 0]);
            chunk(&mut out, b"fcTL", &fctl);
            seq += 1;
            let data = zlib_stored(f.w, &f.rgba);
            if i == 0 {
                chunk(&mut out, b"IDAT", &data);
            } else {
                let mut fdat = seq.to_be_bytes().to_vec();
                fdat.extend(data);
                chunk(&mut out, b"fdAT", &fdat);
                seq += 1;
            }
        }
        chunk(&mut out, b"IEND", &[]);
        std::fs::write(path, out).unwrap();
    }

    fn solid(w: u32, h: u32, px: [u8; 4]) -> Vec<u8> {
        px.repeat((w * h) as usize)
    }

    /// Issue #2: an animated PNG plays as a GIF does — every frame, each
    /// held its own delay, each the whole canvas, a smaller frame drawn
    /// over the one before at its place.
    #[test]
    fn an_animated_png_brings_every_frame_and_its_delay() {
        let dir = temp_dir("apng");
        let path = dir.join("wave.png");
        let (red, green, blue) = ([255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255]);
        write_apng(
            &path,
            2,
            2,
            &[
                ApngFrame {
                    x: 0,
                    y: 0,
                    w: 2,
                    h: 2,
                    tenths: 1,
                    rgba: solid(2, 2, red),
                },
                ApngFrame {
                    x: 0,
                    y: 0,
                    w: 2,
                    h: 2,
                    tenths: 2,
                    rgba: solid(2, 2, green),
                },
                ApngFrame {
                    x: 1,
                    y: 1,
                    w: 1,
                    h: 1,
                    tenths: 3,
                    rgba: solid(1, 1, blue),
                },
            ],
        );
        assert!(is_apng(&path));
        let frames = load_picture(&path).expect("decodes");
        assert_eq!(frames.len(), 3);
        let delays: Vec<_> = frames.iter().map(|f| f.delay_ms).collect();
        assert_eq!(delays, [Some(100), Some(200), Some(300)]);
        assert!(frames.iter().all(|f| (f.width, f.height) == (2, 2)));
        assert_eq!(frames[0].rgba, solid(2, 2, red));
        assert_eq!(frames[1].rgba, solid(2, 2, green));
        let mut last = solid(2, 2, green);
        last[12..16].copy_from_slice(&blue);
        assert_eq!(frames[2].rgba, last, "the small frame over the one before");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A plain PNG is one frame as before, held at the character's rate;
    /// what cannot be read is no animated PNG.
    #[test]
    fn a_plain_png_stays_one_frame() {
        let dir = temp_dir("plain_picture");
        let path = dir.join("still.png");
        write_colored_png(&path, 3, 2, [9, 8, 7, 255]);
        assert!(!is_apng(&path));
        let frames = load_picture(&path).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].delay_ms, None);
        assert_eq!((frames[0].width, frames[0].height), (3, 2));
        assert!(!is_apng(&dir.join("missing.png")));
        std::fs::write(dir.join("text.png"), b"not a picture").unwrap();
        assert!(!is_apng(&dir.join("text.png")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn caps_file_count_at_max_sequence_files() {
        // Synthesize MAX_SEQUENCE_FILES + 50 tiny PNGs and verify the
        // loader truncates instead of trying to decode all of them.
        let dir = temp_dir("oversized_seq");
        let total = crate::constants::MAX_SEQUENCE_FILES + 50;
        for i in 1..=total {
            let path = dir.join(format!("frame_{i:06}.png"));
            // Tiny 1×1 PNGs so the test runs fast.
            write_colored_png(&path, 1, 1, [255, 0, 0, 255]);
        }
        let frames = load_png_sequence(&dir).expect("load");
        assert!(frames.len() <= crate::constants::MAX_SEQUENCE_FILES);
    }

    #[test]
    fn parallel_decode_preserves_filename_order() {
        let dir = temp_dir("ordered_seq");
        // Each frame has a distinctive red channel matching its index.
        // After parallel decode the colors must come back in 0..N order.
        let n = 12u8;
        for i in 1..=n {
            let path = dir.join(format!("frame_{i:03}.png"));
            // Use distinct red values so we can detect any reordering.
            write_colored_png(&path, 2, 2, [i * 10, 0, 0, 255]);
        }

        let frames = load_png_sequence(&dir).expect("decode");
        assert_eq!(frames.len(), n as usize);
        for (i, frame) in frames.iter().enumerate() {
            // Read the red channel of pixel (0, 0).
            assert_eq!(
                frame.rgba[0],
                ((i + 1) as u8) * 10,
                "frame {i} arrived out of order"
            );
        }
    }
}
