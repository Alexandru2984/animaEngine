use super::cache;
use super::frame::Frame;
use super::gif_loader;
use super::png_sequence;
use super::spritesheet;
#[cfg(feature = "video")]
use super::video_loader;
use super::webp_loader;
use crate::config::AssetType;
use crate::constants::{MAX_DECODED_ASSET_BYTES, MAX_IMAGE_DIM, MAX_SEQUENCE_FILES};
use crate::error::{AnimaError, Result};
use std::fs;
use std::path::Path;

/// Validate that loading the asset at `path` won't blow up memory.
///
/// - For a single file, checks the header dimensions against `MAX_IMAGE_DIM`.
/// - For a directory (PNG sequence), validates **every** `.png` file inside.
///   The first oversized frame triggers an `ImageTooLarge` error so we never
///   start decoding a 64-frame sequence only to OOM on frame 32.
///
/// Returns the dimensions of the file (or `(0, 0)` for directories — the
/// caller doesn't use the value for sequences anyway).
pub fn validate_image_dimensions(path: &Path) -> Result<(u32, u32)> {
    if path.is_dir() {
        validate_directory(path)?;
        return Ok((0, 0));
    }
    validate_single_file(path)
}

/// Extensions we know how to validate via the `image` crate's
/// header-only probe. For these we fail closed if the probe fails.
const IMAGE_PROBE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp"];

/// Header-only dimension check for a single image file.
///
/// **Fail-closed for known image extensions** (PNG / JPEG / GIF / WebP):
/// if the header can't be read, return `Err` instead of letting the
/// real decoder paper over it later. For other extensions (e.g. video,
/// where `image_dimensions` doesn't apply) we return `(0, 0)` so the
/// format-specific loader can do its own checks.
fn validate_single_file(path: &Path) -> Result<(u32, u32)> {
    if !path.exists() {
        return Err(AnimaError::AssetNotFound(path.to_path_buf()));
    }

    // Reject anything that isn't a regular file *before* opening it.
    // `image_dimensions` below opens the path, and opening a FIFO blocks
    // until a writer appears — a `.png` named pipe reached through a
    // config entry or a dropped directory would wedge this thread
    // forever. `stat` on a FIFO returns immediately, so checking the kind
    // first is what keeps the probe bounded.
    if !crate::util::resolves_to_regular_file(path) {
        return Err(AnimaError::other(format!(
            "not a regular file: {}",
            crate::drop_validate::redact_path(path)
        )));
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase());
    let is_image_ext = ext
        .as_deref()
        .map(|e| IMAGE_PROBE_EXTENSIONS.contains(&e))
        .unwrap_or(false);

    // By what the file holds: a picture saved under the wrong extension
    // (`content_format`) is measured as the picture it is.
    let dims = image::ImageReader::open(path)
        .and_then(|reader| reader.with_guessed_format())
        .map_err(image::ImageError::IoError)
        .and_then(|reader| reader.into_dimensions());
    match dims {
        Ok((w, h)) => {
            if w > MAX_IMAGE_DIM || h > MAX_IMAGE_DIM {
                Err(AnimaError::ImageTooLarge {
                    width: w,
                    height: h,
                    max: MAX_IMAGE_DIM,
                })
            } else {
                tracing::debug!("Image dimensions OK: {}×{}", w, h);
                Ok((w, h))
            }
        }
        Err(e) if is_image_ext => {
            // Known image extension but the header is unreadable —
            // refuse instead of trusting the decoder to fail later. The
            // decoder's words are for the log: the user was shown them,
            // in English, with the file's whole path and the decoder's
            // own structures ("ChunkType { … }").
            tracing::warn!(
                "Unreadable picture {}: {e}",
                crate::drop_validate::redact_path(path)
            );
            Err(AnimaError::said_as(
                crate::i18n::Msg::new("error-image-unreadable")
                    .arg("name", crate::drop_validate::redact_path(path)),
            ))
        }
        Err(e) => {
            // Unknown extension (video, …) — defer to the format-specific
            // loader.
            tracing::debug!(
                "Could not read image dimensions for {} (deferring): {}",
                path.display(),
                e
            );
            Ok((0, 0))
        }
    }
}

/// Validate every `.png` inside a sequence directory before any decode.
///
/// Mirrors the guards `png_sequence::load_png_sequence` applies — but
/// those run *after* this pass, so without the same limits here the
/// pre-decode validation is itself the unbounded step: a directory with
/// tens of thousands of entries, or one holding a `.png` FIFO, would
/// stall or hang before the loader's own caps ever applied.
fn validate_directory(dir: &Path) -> Result<()> {
    let mut checked = 0usize;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("png") {
            continue;
        }
        // Same dirent-level filter the sequence loader uses: skip
        // symlinks / FIFOs / sockets / dirs / devices without opening them.
        match entry.file_type() {
            Ok(ft) if ft.is_file() => {}
            _ => continue,
        }
        validate_single_file(&path)?;
        checked += 1;
        if checked >= MAX_SEQUENCE_FILES {
            tracing::debug!(
                "Validated MAX_SEQUENCE_FILES = {} frames in {}; the sequence \
                 loader caps the rest",
                MAX_SEQUENCE_FILES,
                crate::drop_validate::redact_path(dir)
            );
            break;
        }
    }
    Ok(())
}

/// Load animation frames based on asset type and path, each at most
/// `max_dim` a side (`entity::picture_cap`).
/// Returns a Vec of Frame on success.
#[tracing::instrument(
    skip(spritesheet_columns, spritesheet_rows),
    fields(path = %crate::drop_validate::redact_path(asset_path))
)]
pub fn load_asset(
    asset_type: &AssetType,
    asset_path: &Path,
    spritesheet_columns: Option<u32>,
    spritesheet_rows: Option<u32>,
    max_dim: u32,
) -> Result<Vec<Frame>> {
    // Reject decompression bombs up-front (works for files AND directories).
    validate_image_dimensions(asset_path)?;

    // Try the on-disk RGBA cache first — skips PNG/GIF/WebP decoding
    // entirely when the asset hasn't changed since last run.
    if let Some(frames) = cache::try_load(
        asset_path,
        asset_type,
        spritesheet_columns,
        spritesheet_rows,
        max_dim,
    ) {
        tracing::info!(
            "Asset cache hit ({}): {} frames",
            crate::drop_validate::redact_path(asset_path),
            frames.len()
        );
        return Ok(frames);
    }

    let frames = decode(
        asset_type,
        asset_path,
        spritesheet_columns,
        spritesheet_rows,
    )
    .map_err(|e| unreadable(asset_path, e))?;
    // At the size every caller keeps it (`Scene::cap_to`), before it is
    // cached: the cache kept the full size and every start resized it
    // again — a 1024² sequence of 24 frames was 96 MB on disk for 6 MB
    // used (cache format 2).
    let frames = super::frame::capped(frames, max_dim)?;

    // Best-effort cache write — never fails the load.
    if let Err(e) = cache::try_save(
        asset_path,
        asset_type,
        spritesheet_columns,
        spritesheet_rows,
        max_dim,
        &frames,
    ) {
        tracing::warn!(
            "Failed to write asset cache for {}: {}",
            crate::drop_validate::redact_path(asset_path),
            e
        );
    }

    Ok(frames)
}

/// Decode `asset_path` as `asset_type`, past the dimension check and the
/// cache.
fn decode(
    asset_type: &AssetType,
    asset_path: &Path,
    spritesheet_columns: Option<u32>,
    spritesheet_rows: Option<u32>,
) -> Result<Vec<Frame>> {
    Ok(match asset_type {
        AssetType::PngSequence => {
            tracing::info!(
                "Loading PNG sequence from: {}",
                crate::drop_validate::redact_path(asset_path)
            );
            png_sequence::load_png_sequence(asset_path)?
        }
        AssetType::PngStatic => {
            tracing::info!(
                "Loading static image from: {}",
                crate::drop_validate::redact_path(asset_path)
            );
            vec![png_sequence::load_single_png(asset_path)?]
        }
        AssetType::Gif => {
            tracing::info!(
                "Loading GIF from: {}",
                crate::drop_validate::redact_path(asset_path)
            );
            gif_loader::load_gif(asset_path)?
        }
        AssetType::WebpAnimated => {
            tracing::info!(
                "Loading animated WebP from: {}",
                crate::drop_validate::redact_path(asset_path)
            );
            webp_loader::load_webp(asset_path)?
        }
        AssetType::WebpStatic => {
            tracing::info!(
                "Loading static WebP from: {}",
                crate::drop_validate::redact_path(asset_path)
            );
            webp_loader::load_static_webp(asset_path)?
        }
        AssetType::Spritesheet => {
            let cols = spritesheet_columns.unwrap_or(4);
            let rows = spritesheet_rows.unwrap_or(1);
            tracing::info!(
                "Loading spritesheet from: {} ({}x{} grid)",
                crate::drop_validate::redact_path(asset_path),
                cols,
                rows
            );
            spritesheet::load_spritesheet(asset_path, cols, rows)?
        }
        AssetType::Video => {
            #[cfg(feature = "video")]
            {
                tracing::info!(
                    "Loading MP4 video from: {}",
                    crate::drop_validate::redact_path(asset_path)
                );
                video_loader::load_video(asset_path)?
            }
            #[cfg(not(feature = "video"))]
            {
                return Err(crate::error::AnimaError::VideoDecode(
                    "video support was not compiled into this build".into(),
                ));
            }
        }
    })
}

/// For the `content_sniff` fuzz target: `bytes` saved as `name` in `dir`,
/// then what a drop of it does up to the frames — the drop's checks, the
/// type by name and content, the dimension check, the decode. Not the
/// frame cache: a fuzz run would fill the real one. Never panics.
#[doc(hidden)]
pub fn fuzz_content_sniff(dir: &Path, name: &str, bytes: &[u8]) {
    let path = dir.join(name);
    if std::fs::write(&path, bytes).is_err() {
        return;
    }
    let _ = content_format(&path);
    let (asset_type, _) = detect_asset_type(&path);
    // A video goes to openh264, upstream's C, slow and fuzzed there.
    if crate::drop_validate::pre_validate_dropped_file(&path).is_ok()
        && asset_type != AssetType::Video
        && validate_image_dimensions(&path).is_ok()
    {
        let _ = decode(&asset_type, &path, Some(2), Some(2)).map_err(|e| unreadable(&path, e));
    }
    let _ = std::fs::remove_file(&path);
}

/// For the `folder_drop` fuzz target: a folder built in `dir` from `ops`,
/// two bytes each — PNG frames, broken ones, named pipes, links (to
/// `/dev/zero`, to a file outside, back up, to the folder itself),
/// folders named like pictures, other files, a couple of thousand empty
/// frames at once — then dropped: the drop's check, the type, the
/// dimension check over every frame, the sequence decoded. A pipe or a
/// link to `/dev/zero` that is opened hangs the run, which libFuzzer
/// reports. Not the frame cache. Never panics.
#[cfg(unix)]
#[doc(hidden)]
pub fn fuzz_folder_drop(dir: &Path, ops: &[u8]) {
    let folder = dir.join("drop");
    let _ = fs::remove_dir_all(&folder);
    if fs::create_dir_all(&folder).is_err() {
        return;
    }
    let png = |side: u32| {
        let mut bytes = Vec::new();
        let image = image::RgbaImage::from_pixel(side, side, image::Rgba([200, 40, 90, 255]));
        let _ = image.write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        );
        bytes
    };
    for &[kind, arg] in ops.as_chunks::<2>().0.iter().take(64) {
        let kind = kind % 8;
        let at = |name: String| folder.join(name);
        let _ = match kind {
            0 => fs::write(at(format!("f{arg:03}.png")), png(u32::from(arg % 8) + 1)),
            1 => fs::write(at(format!("g{arg:03}.png")), vec![0xAB; usize::from(arg)]),
            2 => {
                let Ok(c) = std::ffi::CString::new(
                    at(format!("p{arg:03}.png"))
                        .into_os_string()
                        .into_encoded_bytes(),
                ) else {
                    continue;
                };
                // SAFETY: `c` is a NUL-terminated path that outlives the call.
                unsafe { libc::mkfifo(c.as_ptr(), 0o600) };
                Ok(())
            }
            3 => {
                let target = match arg % 4 {
                    0 => Path::new("/dev/zero").to_path_buf(),
                    1 => Path::new("/etc/passwd").to_path_buf(),
                    2 => Path::new("..").to_path_buf(),
                    _ => Path::new("f000.png").to_path_buf(),
                };
                std::os::unix::fs::symlink(target, at(format!("l{arg:03}.png")))
            }
            4 => fs::create_dir_all(at(format!("d{arg:03}.png"))),
            5 => fs::write(at(format!("t{arg:03}.txt")), b"text"),
            6 => std::os::unix::fs::symlink(".", at(format!("s{arg:03}"))),
            _ => (0..usize::from(arg) * 8)
                .try_for_each(|n| fs::write(at(format!("e{n:04}.png")), b"")),
        };
    }
    let _ = crate::drop_validate::pre_validate_dropped_file(&folder);
    let (asset_type, _) = detect_asset_type(&folder);
    if validate_image_dimensions(&folder).is_ok() && asset_type == AssetType::PngSequence {
        let _ = decode(&asset_type, &folder, None, None).map_err(|e| unreadable(&folder, e));
    }
    let _ = fs::remove_dir_all(&folder);
}

/// A decoder's failure, as the user is told it: the picture or the video
/// cannot be read. The decoder's own words are for the log; they reached
/// the screen, in English — "image decode error: failed to fill whole
/// buffer", "MP4 header: file contains a box with a larger size than it"
/// — where a file passed the header check and failed further in.
fn unreadable(path: &Path, error: AnimaError) -> AnimaError {
    let key = match &error {
        AnimaError::ImageDecode(_) => "error-image-unreadable",
        // Without the feature, "not compiled into this build" says it.
        AnimaError::VideoDecode(_) if cfg!(feature = "video") => "error-video-unreadable",
        _ => return error,
    };
    let name = crate::drop_validate::redact_path(path);
    tracing::warn!("Unreadable {name}: {error}");
    AnimaError::said_as(crate::i18n::Msg::new(key).arg("name", name))
}

/// Detect the best AssetType from a file path's extension and properties.
/// Returns the detected type and a human-readable description.
pub fn detect_asset_type(path: &Path) -> (AssetType, &'static str) {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // What the file holds, where that and its name disagree. Only a
    // regular file is opened: a `.png` FIFO would block here.
    if crate::util::resolves_to_regular_file(path) {
        use image::ImageFormat as F;
        if let Some(format) = content_format(path) {
            let named_so = matches!(
                (format, ext.as_str()),
                (F::Png, "png") | (F::Jpeg, "jpg" | "jpeg") | (F::Gif, "gif") | (F::WebP, "webp")
            );
            if !named_so {
                return match format {
                    F::Gif => (AssetType::Gif, "GIF animation (by its content)"),
                    F::WebP => (AssetType::WebpAnimated, "WebP (by its content)"),
                    _ => (AssetType::PngStatic, "Picture (by its content)"),
                };
            }
        }
    }

    match ext.as_str() {
        "gif" => (AssetType::Gif, "GIF animation"),
        "webp" => (
            AssetType::WebpAnimated,
            "WebP (auto-detect animated/static)",
        ),
        "png" => {
            // If it's a directory, treat as PNG sequence
            if path.is_dir() {
                (AssetType::PngSequence, "PNG sequence (directory)")
            } else {
                (AssetType::PngStatic, "Static PNG")
            }
        }
        "jpg" | "jpeg" => (AssetType::PngStatic, "JPEG image"),
        "mp4" | "m4v" | "mov" => (AssetType::Video, "MP4 / H.264 video"),
        _ => {
            // Check if it's a directory (PNG sequence)
            if path.is_dir() {
                (AssetType::PngSequence, "PNG sequence (directory)")
            } else {
                // Default to static PNG for unknown extensions
                (
                    AssetType::PngStatic,
                    "Unknown format (trying as static image)",
                )
            }
        }
    }
}

/// The picture format a file's first bytes say it is — PNG, JPEG, GIF or
/// WebP, the ones this app decodes — whatever its name says. Pictures
/// saved from a browser often carry the wrong extension, a WebP or a
/// JPEG named `.png`, and the name alone sent them to the wrong decoder,
/// which refused them.
pub(crate) fn content_format(path: &Path) -> Option<image::ImageFormat> {
    use std::io::Read;
    let mut head = [0u8; 16];
    let mut file = std::fs::File::open(path).ok()?;
    let n = file.read(&mut head).ok()?;
    let head = &head[..n];
    if head.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(image::ImageFormat::Png)
    } else if head.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(image::ImageFormat::Jpeg)
    } else if head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a") {
        Some(image::ImageFormat::Gif)
    } else if head.len() >= 12 && &head[..4] == b"RIFF" && &head[8..12] == b"WEBP" {
        Some(image::ImageFormat::WebP)
    } else {
        None
    }
}

/// The limits every picture is decoded within: no frame over
/// `MAX_IMAGE_DIM` on a side, no buffer over `MAX_DECODED_ASSET_BYTES`.
/// The size in a file's header is checked first
/// (`validate_image_dimensions`), but a GIF's frames carry sizes of their
/// own: an 8×8 GIF whose one frame said 46003×46003 had 8.4 GB asked for
/// at once (found by the `content_sniff` fuzz target). A decoder made
/// with `new` has no limits, and `ImageReader`'s own allow 512 MiB at any
/// size; `image` checks these before it allocates.
pub(crate) fn decode_limits() -> image::Limits {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIM);
    limits.max_image_height = Some(MAX_IMAGE_DIM);
    limits.max_alloc = Some(MAX_DECODED_ASSET_BYTES as u64);
    limits
}

/// Open a picture by what it holds, not by its name (`content_format`);
/// only the formats this app is built to decode can come out, and within
/// [`decode_limits`].
pub(crate) fn open_image(path: &Path) -> image::ImageResult<image::DynamicImage> {
    let mut reader = image::ImageReader::open(path)?.with_guessed_format()?;
    reader.limits(decode_limits());
    reader.decode()
}

/// Generate a fallback frame — a simple colored rectangle
/// Used when asset loading fails
pub fn generate_fallback_frame(color: [u8; 4], size: u32) -> Frame {
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            // Create a simple shape with rounded corners
            let cx = size as f32 / 2.0;
            let cy = size as f32 / 2.0;
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let radius = size as f32 * 0.4;

            if dist < radius {
                rgba.extend_from_slice(&color);
            } else if dist < radius + 2.0 {
                // Anti-aliased edge
                let alpha = ((radius + 2.0 - dist) / 2.0 * color[3] as f32) as u8;
                rgba.extend_from_slice(&[color[0], color[1], color[2], alpha]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    Frame::new(rgba, size, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a temp dir under `target/` so we don't pollute the workspace.
    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("validate_tests")
            .join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A picture under the wrong name — a JPEG called `.png`, as browsers
    /// save them — is taken for what it holds, measured and loaded.
    #[test]
    fn a_picture_is_known_by_its_content_not_its_name() {
        let dir = temp_dir("by_content");
        let path = dir.join("saved-from-a-browser.png");
        image::DynamicImage::ImageRgb8(image::RgbImage::new(12, 8))
            .save_with_format(&path, image::ImageFormat::Jpeg)
            .unwrap();
        assert_eq!(content_format(&path), Some(image::ImageFormat::Jpeg));
        assert_eq!(detect_asset_type(&path).0, AssetType::PngStatic);
        assert_eq!(validate_single_file(&path).unwrap(), (12, 8));
        let frame = png_sequence::load_single_png(&path).unwrap();
        assert_eq!((frame.width, frame.height), (12, 8));
        // Named rightly, nothing changes.
        let named = dir.join("photo.jpg");
        std::fs::copy(&path, &named).unwrap();
        assert_eq!(detect_asset_type(&named).1, "JPEG image");
    }

    /// An unreadable picture is refused in the user's words, by its name —
    /// not with the decoder's, nor the whole path.
    #[test]
    fn an_unreadable_picture_is_refused_by_name() {
        let dir = temp_dir("unreadable");
        let path = dir.join("broken.png");
        std::fs::write(&path, b"\x89PNG\r\n\x1a\nnot really a picture").unwrap();
        crate::i18n::init(Some("en"));
        let said = validate_single_file(&path).unwrap_err().said();
        assert!(
            said.starts_with("broken.png cannot be read as a picture"),
            "{said}"
        );
        assert!(!said.contains(&*dir.to_string_lossy()), "{said}");
    }

    fn write_png(path: &Path, w: u32, h: u32) {
        let img = image::RgbaImage::new(w, h);
        img.save(path).unwrap();
    }

    #[test]
    fn validate_dir_with_safe_pngs_succeeds() {
        let dir = temp_dir("safe_seq");
        for i in 1..=3 {
            write_png(&dir.join(format!("frame_{i:03}.png")), 32, 32);
        }
        assert!(validate_image_dimensions(&dir).is_ok());
    }

    #[test]
    fn validate_dir_rejects_oversized_frame() {
        let dir = temp_dir("oversized_seq");
        // One safe frame, then an oversized one. Validation must catch the
        // big one regardless of file ordering inside the dir.
        write_png(&dir.join("frame_001.png"), 32, 32);
        write_png(&dir.join("frame_002.png"), MAX_IMAGE_DIM + 100, 32);

        let err = validate_image_dimensions(&dir).unwrap_err();
        match err {
            AnimaError::ImageTooLarge { width, .. } => {
                assert!(width > MAX_IMAGE_DIM);
            }
            other => panic!("expected ImageTooLarge, got {other:?}"),
        }
    }

    #[test]
    fn validate_dir_ignores_non_png_files() {
        let dir = temp_dir("mixed_seq");
        write_png(&dir.join("frame_001.png"), 32, 32);
        // A README that would fail dimension probing — must be ignored.
        std::fs::write(dir.join("README.txt"), b"not an image").unwrap();
        assert!(validate_image_dimensions(&dir).is_ok());
    }

    #[test]
    fn validate_single_oversized_file_errors() {
        let dir = temp_dir("oversized_file");
        let path = dir.join("big.png");
        write_png(&path, MAX_IMAGE_DIM + 50, MAX_IMAGE_DIM + 50);

        let err = validate_image_dimensions(&path).unwrap_err();
        assert!(matches!(err, AnimaError::ImageTooLarge { .. }));
    }

    #[test]
    fn validate_missing_file_errors() {
        let err = validate_image_dimensions(Path::new("/nonexistent/x.png")).unwrap_err();
        assert!(matches!(err, AnimaError::AssetNotFound(_)));
    }

    #[test]
    fn validate_corrupt_png_fails_closed() {
        // A file with a .png extension but no real PNG header. Old behavior
        // returned (0, 0) and deferred to the decoder; we now error out.
        let dir = temp_dir("corrupt_png");
        let path = dir.join("not_really.png");
        std::fs::write(&path, b"this is not a png").unwrap();
        let err = validate_image_dimensions(&path).unwrap_err();
        // We mostly care that it's NOT silently Ok((0, 0)) and NOT an
        // "asset not found" — the path exists, it just isn't a PNG — and
        // that it is said in the user's words (1.5).
        assert!(matches!(err, AnimaError::Said(_)));
    }

    #[test]
    fn frames_come_back_at_the_size_they_are_shown() {
        // So the frame cache keeps them so, not at full size (format 2).
        let dir = temp_dir("capped_frames");
        for i in 0..3 {
            image::RgbaImage::from_pixel(600, 300, image::Rgba([i * 40, 9, 9, 255]))
                .save(dir.join(format!("frame_{i:03}.png")))
                .unwrap();
        }
        let frames = load_asset(
            &AssetType::PngSequence,
            &dir,
            None,
            None,
            crate::constants::MAX_DROP_SIZE,
        )
        .unwrap();
        assert_eq!(frames.len(), 3);
        let max = crate::constants::MAX_DROP_SIZE;
        assert!(frames.iter().all(|f| (f.width, f.height) == (max, max / 2)));
    }

    #[test]
    fn a_frame_larger_than_its_picture_is_not_decoded() {
        // A GIF saved as `.png`: 8×8 by its header, so the dimension
        // check passes, and a frame of 11000×11000 — 484 MB, under the
        // 512 MiB `ImageReader` allows on its own.
        let dir = temp_dir("frame_larger");
        let path = dir.join("small.png");
        let mut gif = b"GIF89a\x08\x00\x08\x00\x80\x00\x00".to_vec();
        gif.extend_from_slice(&[0, 0, 0, 255, 255, 255]);
        gif.extend_from_slice(&[0x2C, 0, 0, 0, 0]);
        gif.extend_from_slice(&11000u16.to_le_bytes());
        gif.extend_from_slice(&11000u16.to_le_bytes());
        gif.extend_from_slice(&[0, 2, 2, 0x4C, 0x01, 0, 0x3B]);
        std::fs::write(&path, &gif).unwrap();
        assert!(validate_image_dimensions(&path).is_ok());
        let err = open_image(&path).unwrap_err();
        assert!(
            matches!(err, image::ImageError::Limits(_)),
            "refused for its size, not decoded: {err:?}"
        );
    }

    #[test]
    fn a_picture_that_fails_past_its_header_is_said_unreadable() {
        // Its header whole, so the dimension check passes, and the rest
        // cut off: the decoder's own words were what the user read.
        let dir = temp_dir("half_png");
        let path = dir.join("half.png");
        let mut bytes = Vec::new();
        image::RgbaImage::from_fn(64, 64, |x, y| {
            image::Rgba([x as u8 * 4, y as u8 * 4, 9, 255])
        })
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
        std::fs::write(&path, &bytes[..bytes.len() / 2]).unwrap();
        assert!(validate_image_dimensions(&path).is_ok());
        let err = load_asset(
            &AssetType::PngStatic,
            &path,
            None,
            None,
            crate::constants::MAX_DROP_SIZE,
        )
        .unwrap_err();
        assert!(matches!(err, AnimaError::Said(_)), "{err:?}");
    }

    #[cfg(unix)]
    #[test]
    fn validate_rejects_a_fifo_instead_of_blocking_on_open() {
        // A `.png` named pipe reachable from a config entry or a dropped
        // directory. `image_dimensions` would `open` it and block until a
        // writer appeared, wedging the loader thread; the kind check must
        // reject it immediately. If this test ever hangs, that guard is gone.
        use std::ffi::CString;
        let dir = temp_dir("fifo_png");
        let fifo = dir.join("evil.png");
        let c = CString::new(fifo.to_str().unwrap()).unwrap();
        assert_eq!(
            // SAFETY: `c` is a NUL-terminated path that outlives the call.
            unsafe { libc::mkfifo(c.as_ptr(), 0o644) },
            0,
            "mkfifo failed"
        );

        let err = validate_image_dimensions(&fifo).unwrap_err();
        assert!(
            err.to_string().contains("not a regular file"),
            "expected a regular-file rejection, got: {err}"
        );
        let _ = std::fs::remove_file(&fifo);
    }

    #[test]
    fn validate_unknown_extension_defers() {
        // A .xyz file the image crate can't probe — we still return Ok
        // because the format-specific loader may know what to do with it.
        let dir = temp_dir("unknown_ext");
        let path = dir.join("blob.xyz");
        std::fs::write(&path, b"opaque bytes").unwrap();
        let dims = validate_image_dimensions(&path).expect("should defer, not error");
        assert_eq!(dims, (0, 0));
    }
}
