//! A sound on a tap (1.5): each character can have one — a file from the
//! asset library, played when it is tapped, in edit mode or out of it,
//! panned by where it is (`crate::audio`). None by default.
//!
//! The Inspector offers the sound files the library holds ([`choices`]),
//! listed when it asks and at most every few seconds: a bounded walk that
//! does not follow links. Playing goes through `AudioHost::play`, which
//! resolves the path inside the library again and caps the read and the
//! decode, as it does for a script's `play`. The file stays in the
//! library: a shared scene carries no sounds (`crate::scene_file`).

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// The kinds offered, lowercase.
pub const EXTENSIONS: &[&str] = &["wav", "ogg", "oga", "flac", "mp3"];

/// The longest path kept in the config.
pub const MAX_PATH_CHARS: usize = 256;

/// At most this many offered…
const MAX_LISTED: usize = 200;
/// …from at most this many entries looked at…
const MAX_SEEN: usize = 4000;
/// …this many folders down.
const MAX_DEPTH: usize = 4;

/// How long a listing is good for.
const FRESH_FOR: Duration = Duration::from_secs(3);

static ROOT: OnceLock<PathBuf> = OnceLock::new();
static LISTED: Mutex<Option<(Instant, Vec<String>)>> = Mutex::new(None);

/// Where the asset library is: the loops say, once they found it.
pub fn set_root(root: &Path) {
    let _ = ROOT.set(root.to_path_buf());
}

/// The asset library, if there is one.
pub fn root() -> Option<&'static Path> {
    ROOT.get().map(PathBuf::as_path)
}

/// The sound files in the asset library, relative to it, sorted.
pub fn choices() -> Vec<String> {
    let Some(root) = root() else {
        return Vec::new();
    };
    let Ok(mut listed) = LISTED.lock() else {
        return Vec::new();
    };
    if let Some((at, list)) = listed.as_ref() {
        if at.elapsed() < FRESH_FOR {
            return list.clone();
        }
    }
    let list = list(root);
    *listed = Some((Instant::now(), list.clone()));
    list
}

/// The sound files under `root`, relative to it with `/` between
/// folders, sorted without case. Hidden entries and links are passed by.
pub fn list(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = 0;
    walk(root, "", 0, &mut out, &mut seen);
    out.sort_by_key(|s| s.to_lowercase());
    out
}

fn walk(dir: &Path, rel: &str, depth: usize, out: &mut Vec<String>, seen: &mut usize) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        *seen += 1;
        if *seen > MAX_SEEN || out.len() >= MAX_LISTED {
            return;
        }
        // The entry's own kind: a link is neither a folder nor a file here.
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let path = if rel.is_empty() {
            name.to_string()
        } else {
            format!("{rel}/{name}")
        };
        if kind.is_dir() {
            if depth < MAX_DEPTH {
                walk(&entry.path(), &path, depth + 1, out, seen);
            }
        } else if kind.is_file() && is_sound(name) {
            out.push(path);
        }
    }
}

/// Whether `name` ends in one of [`EXTENSIONS`].
pub fn is_sound(name: &str) -> bool {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

/// A sound path as kept: without control characters, trimmed, capped; one
/// that is not a sound file is no sound.
pub fn tidy(path: &str) -> String {
    let kept: String = path
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_PATH_CHARS)
        .collect();
    let kept = kept.trim();
    if is_sound(kept) {
        kept.to_string()
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("sounds_tests")
            .join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_library_s_sounds_are_listed_and_nothing_else() {
        let dir = scratch("listed");
        let root = dir.as_path();
        std::fs::create_dir_all(root.join("cats/more")).unwrap();
        std::fs::create_dir_all(root.join(".hidden")).unwrap();
        for f in [
            "meow.OGG",
            "cats/purr.wav",
            "cats/more/hiss.mp3",
            "cats/cat.png",
            ".hidden/secret.ogg",
            "notes.txt",
        ] {
            std::fs::write(root.join(f), b"x").unwrap();
        }
        assert_eq!(
            list(root),
            ["cats/more/hiss.mp3", "cats/purr.wav", "meow.OGG"]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn links_are_not_followed() {
        let outside = scratch("outside");
        std::fs::write(outside.join("elsewhere.ogg"), b"x").unwrap();
        let dir = scratch("links");
        std::os::unix::fs::symlink(&outside, dir.join("out")).unwrap();
        std::os::unix::fs::symlink(outside.join("elsewhere.ogg"), dir.join("link.ogg")).unwrap();
        assert!(list(&dir).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn a_poke_asks_for_its_sound_once() {
        use crate::config::AppConfig;
        let mut scene = crate::scene::Scene::from_config(&AppConfig::default());
        let bounds = crate::monitor::DesktopBounds::from_size(1600.0, 1000.0);
        scene.entities[0].poke(0.0, bounds);
        assert!(scene.take_sounds().is_empty(), "no sound, nothing to play");
        scene.entities[1].talk.sound = "cats/purr.wav".into();
        scene.entities[1].poke(0.0, bounds);
        let sounds = scene.take_sounds();
        assert_eq!(sounds.len(), 1);
        let (id, sound, x, volume) = &sounds[0];
        assert_eq!(*volume, 1.0, "full as it comes");
        assert_eq!(
            (id.as_str(), sound.as_str()),
            (scene.entities[1].id.as_str(), "cats/purr.wav")
        );
        let e = &scene.entities[1];
        assert!((x - (e.x + e.scaled_width() / 2.0)).abs() < 0.01);
        assert!(scene.take_sounds().is_empty(), "once");
    }

    #[test]
    fn a_kept_path_is_a_sound_and_tidy() {
        assert_eq!(tidy("  cats/purr.wav \n"), "cats/purr.wav");
        assert_eq!(tidy("cats/cat.png"), "");
        assert_eq!(tidy(""), "");
        let long = format!("{}.ogg", "a".repeat(400));
        assert_eq!(tidy(&long), "", "cut short, it is no sound file");
    }
}
