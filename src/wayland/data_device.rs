//! Drag-and-drop on the native Wayland path (E.3).
//!
//! This module owns the `text/uri-list` parsing — turning the CRLF
//! separated payload that flows through `wl_data_offer::receive` into
//! a `Vec<PathBuf>` the scene can ingest. The `wl_data_device` /
//! `wl_data_offer` handlers themselves live in
//! [`crate::wayland::layer_window`] alongside `WaylandState` so they
//! can mutate the shared event-loop state directly; this module is
//! the pure-function half.
//!
//! Reference: <https://www.iana.org/assignments/media-types/text/uri-list>
//! and the freedesktop "dragndrop" spec.

use std::path::PathBuf;

/// Mime type we accept on incoming drag offers — the only one that
/// reliably points at on-disk files across GTK/Qt/file managers.
pub const URI_LIST_MIME: &str = "text/uri-list";

/// Cap on how many paths a single drag may produce (F.5, 0.5.1).
/// Real drag-and-drop sessions are a handful of files — the
/// upstream X11 path already short-circuits any drag with > a few
/// dozen entries. A million-line uri-list is either a corrupt or an
/// adversarial payload; capping at 256 forecloses the unbounded
/// `Vec<PathBuf>` allocation the pre-fix parser would have produced.
pub const MAX_URI_LIST_PATHS: usize = 256;

/// Parse the `text/uri-list` payload received over `wl_data_offer`
/// and return the file paths it carries. Non-`file://` URIs (http://,
/// data:, etc.) are silently ignored — the overlay only consumes
/// local files. Comment lines (`#`-prefixed) are skipped per the
/// IANA spec.
pub fn parse_uri_list(payload: &[u8]) -> Vec<PathBuf> {
    let Ok(text) = std::str::from_utf8(payload) else {
        // Non-UTF-8 payloads are pathological for text/uri-list;
        // returning an empty vec is consistent with "no file dropped".
        return Vec::new();
    };
    text.lines()
        .map(|line| line.trim_end_matches('\r').trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(file_uri_to_path)
        .take(MAX_URI_LIST_PATHS)
        .collect()
}

/// Decode a single `file://...` URI into a filesystem path. Returns
/// `None` for any other scheme so the caller can skip without
/// special-casing.
fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
    let path_with_host = uri.strip_prefix("file://")?;
    // `file:///abs/path` → host is empty, path starts after the third
    // slash. `file://host/abs/path` is rare on Linux but valid; we
    // ignore the host and take the absolute path that follows.
    let path = match path_with_host.find('/') {
        Some(idx) => &path_with_host[idx..],
        None => path_with_host,
    };
    Some(PathBuf::from(percent_decode(path)))
}

/// Minimal `%XX` percent-decoder — file URIs carry spaces as `%20`
/// and other punctuation similarly. We avoid pulling `percent-encoding`
/// just for this one call site; the inputs are short (file paths) and
/// only the standard ASCII-printable escapes appear in practice.
fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = hex_digit(bytes[i + 1]);
            let lo = hex_digit(bytes[i + 2]);
            if let (Some(h), Some(l)) = (hi, lo) {
                out.push((h * 16 + l) as char);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

const fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_file_uri_decodes() {
        let payload = b"file:///tmp/cat.png\r\n";
        let paths = parse_uri_list(payload);
        assert_eq!(paths, vec![PathBuf::from("/tmp/cat.png")]);
    }

    #[test]
    fn multiple_files_split_on_crlf() {
        let payload = b"file:///a/one.gif\r\nfile:///b/two.webp\r\n";
        let paths = parse_uri_list(payload);
        assert_eq!(
            paths,
            vec![PathBuf::from("/a/one.gif"), PathBuf::from("/b/two.webp")]
        );
    }

    #[test]
    fn percent_encoded_path_decodes() {
        let payload = b"file:///home/user/My%20Pictures/ghost%20(1).png\r\n";
        let paths = parse_uri_list(payload);
        assert_eq!(
            paths,
            vec![PathBuf::from("/home/user/My Pictures/ghost (1).png")]
        );
    }

    #[test]
    fn comment_lines_ignored() {
        let payload = b"# dragged from nautilus\r\nfile:///tmp/a.png\r\n";
        let paths = parse_uri_list(payload);
        assert_eq!(paths, vec![PathBuf::from("/tmp/a.png")]);
    }

    #[test]
    fn non_file_uris_skipped() {
        let payload = b"http://example.com/a.png\r\nfile:///tmp/a.png\r\n";
        let paths = parse_uri_list(payload);
        assert_eq!(paths, vec![PathBuf::from("/tmp/a.png")]);
    }

    #[test]
    fn lf_only_newlines_also_accepted() {
        // Some compositors omit the CR. The spec wants CRLF but real
        // sources are loose; tolerate it.
        let payload = b"file:///tmp/a.png\nfile:///tmp/b.png\n";
        let paths = parse_uri_list(payload);
        assert_eq!(
            paths,
            vec![PathBuf::from("/tmp/a.png"), PathBuf::from("/tmp/b.png")]
        );
    }

    #[test]
    fn payload_with_excess_paths_is_capped() {
        let mut payload = String::new();
        for i in 0..(MAX_URI_LIST_PATHS * 4) {
            payload.push_str(&format!("file:///tmp/x{i}.png\r\n"));
        }
        let paths = parse_uri_list(payload.as_bytes());
        assert_eq!(paths.len(), MAX_URI_LIST_PATHS);
    }

    #[test]
    fn non_utf8_payload_returns_empty() {
        let payload = b"\xff\xfe\xfd";
        let paths = parse_uri_list(payload);
        assert!(paths.is_empty());
    }

    #[test]
    fn file_uri_with_host_strips_host() {
        // RFC 8089 form: file://localhost/path. Rare but legal.
        let payload = b"file://localhost/tmp/a.png\r\n";
        let paths = parse_uri_list(payload);
        assert_eq!(paths, vec![PathBuf::from("/tmp/a.png")]);
    }
}
