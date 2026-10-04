//! The desktop's clipboard, beside the app's own (`crate::clipboard`).
//!
//! Copying characters also puts their names on the desktop's clipboard,
//! under a type of our own as well as plain text. So whoever copied last
//! decides what Ctrl+V brings: while the clipboard is still ours, the
//! copied characters; once something else has copied — an image in a
//! browser, files in a file manager — that. An image becomes a character
//! saved under the data directory (`pasted/`), files are added as if
//! dropped; both go through the drop's checks (`drop_validate`).
//!
//! Each backend talks to its clipboard — native Wayland through
//! `wl_data_device` (`wayland::layer_window`), X11 through a selection
//! window of its own (`x11_clipboard`); this module is what they share.

use std::path::{Path, PathBuf};

/// Our own type on the clipboard: its presence means the clipboard still
/// holds the characters copied here.
pub const ANIMA_MIME: &str = "application/x-animaengine-characters";

/// Plain-text types the names are offered under.
pub const TEXT_MIMES: [&str; 3] = ["text/plain;charset=utf-8", "UTF8_STRING", "text/plain"];

/// Image types taken, most preferred first, with the extension a picture
/// of that type is saved under.
pub const IMAGE_MIMES: [(&str, &str); 4] = [
    ("image/png", "png"),
    ("image/webp", "webp"),
    ("image/gif", "gif"),
    ("image/jpeg", "jpg"),
];

/// Files copied in a file manager.
pub const URI_LIST_MIME: &str = "text/uri-list";

/// The largest image read off the clipboard. Screenshots and photos fit;
/// what does not was never going to be a character.
pub const MAX_IMAGE_BYTES: u64 = 64 * 1024 * 1024;

/// The largest list of files read.
pub const MAX_URI_LIST_BYTES: u64 = 64 * 1024;

/// What to ask the clipboard for, from the types it offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pick {
    /// It still holds what was copied here.
    Ours,
    /// An image, of this type, saved with this extension.
    Image(String, &'static str),
    /// Files.
    Files(String),
    /// Nothing a character can come from.
    Nothing,
}

/// Choose from `offered` (the clipboard's types, in any case).
pub fn pick(offered: &[String]) -> Pick {
    let has = |want: &str| {
        offered
            .iter()
            .find(|m| m.eq_ignore_ascii_case(want))
            .cloned()
    };
    if has(ANIMA_MIME).is_some() {
        return Pick::Ours;
    }
    for (mime, ext) in IMAGE_MIMES {
        if let Some(found) = has(mime) {
            return Pick::Image(found, ext);
        }
    }
    match has(URI_LIST_MIME) {
        Some(found) => Pick::Files(found),
        None => Pick::Nothing,
    }
}

/// What the clipboard gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    /// The characters copied here are still what is on it.
    Ours,
    /// An image's bytes, and the extension to save them under.
    Image(Vec<u8>, &'static str),
    /// Files, as `text/uri-list` named them.
    Files(Vec<PathBuf>),
    /// Nothing a character can come from — text, say, or an empty
    /// clipboard.
    Nothing,
    /// It could not be read; why, for the log.
    Failed(String),
}

impl Content {
    /// `bytes` read for `pick`, as content.
    pub fn from_bytes(pick: &Pick, bytes: Vec<u8>) -> Self {
        match pick {
            Pick::Ours => Self::Ours,
            Pick::Image(_, ext) if !bytes.is_empty() => Self::Image(bytes, ext),
            Pick::Files(_) => {
                // The same parser as a Wayland drop: `file://` only,
                // percent-decoded, capped.
                #[cfg(unix)]
                let paths = crate::wayland::data_device::parse_uri_list(&bytes);
                #[cfg(not(unix))]
                let paths: Vec<PathBuf> = Vec::new();
                if paths.is_empty() {
                    Self::Nothing
                } else {
                    Self::Files(paths)
                }
            }
            _ => Self::Nothing,
        }
    }
}

/// The largest read for `pick`.
pub fn cap_for(pick: &Pick) -> u64 {
    match pick {
        Pick::Files(_) => MAX_URI_LIST_BYTES,
        _ => MAX_IMAGE_BYTES,
    }
}

/// Where pasted images are kept: they are the characters' pictures.
pub fn pasted_dir() -> PathBuf {
    crate::asset_library::data_dir().join("pasted")
}

/// Save a pasted image in `dir` as `name.ext`, or `name 2.ext` and
/// upward — the file the drop path then takes, and whose name the new
/// character gets.
pub fn save_image_in(dir: &Path, name: &str, bytes: &[u8], ext: &str) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let name: String = name
        .chars()
        .filter(|c| !c.is_control() && !matches!(c, '/' | '\\' | ':'))
        .take(60)
        .collect();
    let name = match name.trim() {
        "" => "pasted",
        trimmed => trimmed,
    };
    let path = std::iter::once(dir.join(format!("{name}.{ext}")))
        .chain((2..).map(|n| dir.join(format!("{name} {n}.{ext}"))))
        .find(|p| !p.exists())
        .unwrap_or_else(|| dir.join(format!("{name}.{ext}")));
    crate::util::atomic_write_bytes(&path, bytes)?;
    Ok(path)
}

/// The names put on the clipboard for characters copied here.
pub fn names_text(copied: &crate::clipboard::Copied) -> String {
    copied
        .characters
        .iter()
        .map(|c| c.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offered(types: &[&str]) -> Vec<String> {
        types.iter().map(|t| t.to_string()).collect()
    }

    #[test]
    fn what_was_copied_here_wins_while_it_is_on_the_clipboard() {
        assert_eq!(pick(&offered(&[TEXT_MIMES[0], ANIMA_MIME])), Pick::Ours);
    }

    #[test]
    fn an_image_is_taken_in_the_best_type_offered() {
        let p = pick(&offered(&["text/html", "image/jpeg", "IMAGE/PNG"]));
        assert_eq!(p, Pick::Image("IMAGE/PNG".into(), "png"));
        assert_eq!(
            pick(&offered(&["image/gif"])),
            Pick::Image("image/gif".into(), "gif")
        );
    }

    #[test]
    fn files_then_nothing() {
        assert_eq!(
            pick(&offered(&["x-special/gnome-copied-files", URI_LIST_MIME])),
            Pick::Files(URI_LIST_MIME.into())
        );
        assert_eq!(pick(&offered(&["text/plain"])), Pick::Nothing);
        assert_eq!(pick(&[]), Pick::Nothing);
    }

    #[test]
    fn copied_files_are_read_as_paths() {
        let c = Content::from_bytes(
            &Pick::Files(URI_LIST_MIME.into()),
            b"file:///tmp/a%20b.png\r\nhttp://x/y.png\r\n".to_vec(),
        );
        assert_eq!(c, Content::Files(vec![PathBuf::from("/tmp/a b.png")]));
    }

    #[test]
    fn a_pasted_image_gets_a_file_of_its_own() {
        let dir = std::env::temp_dir().join(format!("anima-pasted-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let a = save_image_in(&dir, "Pasted image", b"one", "png").unwrap();
        let b = save_image_in(&dir, "Pasted image", b"two", "png").unwrap();
        assert_eq!(a.file_name().unwrap(), "Pasted image.png");
        assert_eq!(b.file_name().unwrap(), "Pasted image 2.png");
        assert_eq!(std::fs::read(&b).unwrap(), b"two");
        let odd = save_image_in(&dir, "../a/b\u{1b}", b"x", "gif").unwrap();
        assert_eq!(odd.parent().unwrap(), dir, "no way out of the folder");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
