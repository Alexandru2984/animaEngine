//! Sharing a scene as one file (1.5): `name.animascene`, a zip holding
//! `scene.toml` — the scene as `crate::scenes` saves it — and the images
//! and videos its characters show, so it opens on another machine.
//!
//! What travels is pictures and settings, never code: a character with a
//! behavior script goes out standing still, and one coming in with a
//! script is stood still too. The pictures coming in are held to what a
//! dropped file is (`drop_validate`) — the formats a drop takes, its size
//! cap — and then go through the same decoders and memory budget as any
//! character. The zip itself is read by a reader that takes only what
//! this module writes (`zip`).
//!
//! Imported pictures go to a folder of their own under the data
//! directory (`imports_dir`), and the scene is saved to the shelf like any
//! other; its characters point at that folder.

mod zip;

use crate::config::{AppConfig, AssetType};
use crate::constants::{MAX_ASSET_FILE_BYTES, MAX_CONFIG_BYTES, MAX_ENTITIES, MAX_SEQUENCE_FILES};
use crate::drop_validate::DROP_EXTENSIONS;
use crate::error::{AnimaError, Result};
use crate::scenes::SavedScene;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A scene file's extension.
pub const EXTENSION: &str = "animascene";

/// The biggest scene file written or read.
pub const MAX_FILE_BYTES: u64 = 512 * 1024 * 1024;

/// At most this many files inside one: a thousand-frame sequence and
/// then some.
const MAX_ENTRIES: usize = 4096;

/// The scene inside, beside its `assets/`.
const MANIFEST: &str = "scene.toml";

/// This layout. A file with a higher number was made by a newer version.
const FORMAT: u32 = 1;

#[derive(Serialize, Deserialize)]
struct Manifest {
    format: u32,
    scene: SavedScene,
}

/// What an export wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exported {
    pub characters: usize,
    /// Characters whose behavior script stayed home.
    pub scripts_left_out: usize,
    pub bytes: u64,
}

/// Whether `path` names a scene file.
pub fn is_scene_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION))
}

/// Where imported scenes keep their pictures.
pub fn imports_dir() -> PathBuf {
    crate::asset_library::data_dir().join("imported-scenes")
}

/// A folder for one more import under `root`, named after `name`.
pub fn fresh_dir(root: &Path, name: &str) -> PathBuf {
    let stem = crate::scenes::slug(name);
    std::iter::once(root.join(&stem))
        .chain((2..).map(|n| root.join(format!("{stem}-{n}"))))
        .find(|p| !p.exists())
        .unwrap_or_else(|| root.join(stem))
}

/// Write `saved`, with the pictures its characters show, to `to`.
pub fn export(saved: &SavedScene, to: &Path) -> Result<Exported> {
    let mut scene = saved.clone();
    let mut packer = Packer::default();
    let mut scripts_left_out = 0;
    for c in &mut scene.characters {
        let why = |e: AnimaError| AnimaError::other(format!("{}: {e}", c.name));
        c.asset_path = packer.place(&c.asset_type, &c.asset_path).map_err(why)?;
        for state in c.animations.values_mut() {
            state.asset_path = packer
                .place(&state.asset_type, &state.asset_path)
                .map_err(|e| AnimaError::other(format!("{}: {e}", c.name)))?;
        }
        if matches!(c.behavior, crate::behavior::Behavior::Script { .. }) {
            c.behavior = crate::behavior::Behavior::Idle;
            scripts_left_out += 1;
        }
        // Monitor names belong to this machine; there, a character lands
        // by where it is.
        c.monitor = None;
    }
    let manifest = toml::to_string_pretty(&Manifest {
        format: FORMAT,
        scene,
    })?;

    let written = write_zip(to, manifest.as_bytes(), &packer.files);
    if written.is_err() {
        // Half a scene is no use to anyone.
        let _ = std::fs::remove_file(to);
    }
    Ok(Exported {
        characters: saved.characters.len(),
        scripts_left_out,
        bytes: written?,
    })
}

fn write_zip(to: &Path, manifest: &[u8], files: &[(String, PathBuf)]) -> Result<u64> {
    use std::io::Write;
    // Straight to `to`: in the Flatpak it is the one file the portal
    // opened to us, and a temporary beside it could not be made.
    let out = std::io::BufWriter::new(std::fs::File::create(to)?);
    let mut zip = zip::Writer::new(out);
    zip.add(MANIFEST, manifest)?;
    let mut total = manifest.len() as u64;
    for (name, source) in files {
        let data = read_asset(source)?;
        total += data.len() as u64;
        if total > MAX_FILE_BYTES {
            return Err(AnimaError::other(format!(
                "the scene is over {} MB",
                MAX_FILE_BYTES / (1024 * 1024)
            )));
        }
        zip.add(name, &data)?;
    }
    let mut out = zip.finish()?;
    out.flush()?;
    let file = out.into_inner().map_err(|e| e.into_error())?;
    file.sync_all()?;
    Ok(file.metadata()?.len())
}

/// A picture to pack: a regular file within the drop cap.
fn read_asset(path: &Path) -> Result<Vec<u8>> {
    let meta = std::fs::metadata(path)?;
    if !meta.is_file() || meta.len() > MAX_ASSET_FILE_BYTES {
        return Err(AnimaError::other(format!(
            "{} is not a picture that can be shared",
            crate::drop_validate::redact_path(path)
        )));
    }
    Ok(std::fs::read(path)?)
}

/// Gathers the pictures a scene shows, each once, under `assets/<n>/`.
#[derive(Default)]
struct Packer {
    /// Each picture's source and where it went in the file.
    placed: Vec<(PathBuf, String)>,
    /// (name in the file, source file).
    files: Vec<(String, PathBuf)>,
}

impl Packer {
    /// Pack the picture `asset_path` shows, if not yet, and say where it
    /// is in the file: `assets/<n>` for a sequence's folder,
    /// `assets/<n>/asset.<ext>` for one file.
    fn place(&mut self, asset_type: &AssetType, asset_path: &str) -> Result<String> {
        let source = AppConfig::resolve_asset_path(asset_path);
        if let Some((_, at)) = self.placed.iter().find(|(s, _)| *s == source) {
            return Ok(at.clone());
        }
        let dir = format!("assets/{}", self.placed.len());
        let at = if *asset_type == AssetType::PngSequence {
            let mut frames: Vec<PathBuf> = std::fs::read_dir(&source)?
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "png"))
                .take(MAX_SEQUENCE_FILES)
                .collect();
            if frames.is_empty() {
                return Err(AnimaError::EmptyAsset(source));
            }
            // Numbered in the order the loader plays them.
            frames.sort();
            for (i, frame) in frames.into_iter().enumerate() {
                self.files.push((format!("{dir}/{i:04}.png"), frame));
            }
            dir
        } else {
            let ext = source
                .extension()
                .and_then(|e| e.to_str())
                .map(str::to_ascii_lowercase)
                .filter(|e| DROP_EXTENSIONS.contains(&e.as_str()))
                .ok_or_else(|| {
                    AnimaError::other(format!(
                        "{} is not a picture that can be shared",
                        crate::drop_validate::redact_path(&source)
                    ))
                })?;
            let name = format!("{dir}/asset.{ext}");
            self.files.push((name.clone(), source.clone()));
            name
        };
        self.placed.push((source, at.clone()));
        Ok(at)
    }
}

/// Read the scene file `file`, and put its pictures in `into` — a folder
/// that does not exist yet. Returns the scene, its characters pointing
/// there. On failure nothing is left in `into`.
pub fn import(file: &Path, into: &Path) -> Result<SavedScene> {
    let meta = std::fs::metadata(file)?;
    if !meta.is_file() {
        return Err(AnimaError::other("not a regular file"));
    }
    if meta.len() > MAX_FILE_BYTES {
        return Err(AnimaError::other(format!(
            "the file is over {} MB",
            MAX_FILE_BYTES / (1024 * 1024)
        )));
    }
    let bytes = std::fs::read(file)?;
    let entries = zip::read(&bytes, MAX_ENTRIES).map_err(AnimaError::other)?;
    let manifest = entries
        .iter()
        .find(|e| e.name == MANIFEST)
        .ok_or_else(|| AnimaError::other("not a scene file: no scene.toml inside"))?;
    if manifest.data.len() as u64 > MAX_CONFIG_BYTES {
        return Err(AnimaError::other("its scene.toml is too big"));
    }
    let text = std::str::from_utf8(manifest.data)
        .map_err(|_| AnimaError::other("its scene.toml is not text"))?;
    let Manifest { format, mut scene } =
        toml::from_str(text).map_err(|e| AnimaError::other(format!("scene.toml: {e}")))?;
    if format > FORMAT {
        return Err(AnimaError::other(
            "made by a newer animaEngine; update to open it",
        ));
    }

    let mut assets = Vec::new();
    for entry in &entries {
        if entry.name == MANIFEST {
            continue;
        }
        if !is_asset_name(entry.name) {
            return Err(AnimaError::other(format!(
                "unexpected file inside: {}",
                crate::drop_validate::redact_path(Path::new(entry.name))
            )));
        }
        if entry.data.len() as u64 > MAX_ASSET_FILE_BYTES {
            return Err(AnimaError::other(format!(
                "{} is over the {} MB a picture may be",
                entry.name,
                MAX_ASSET_FILE_BYTES / (1024 * 1024)
            )));
        }
        assets.push(*entry);
    }

    tidy(&mut scene);
    for c in &mut scene.characters {
        c.asset_path =
            local_path(&c.asset_type, &c.asset_path, &assets, into).ok_or_else(|| {
                AnimaError::other(format!("{}: its picture is not in the file", c.name))
            })?;
        for state in c.animations.values_mut() {
            state.asset_path = local_path(&state.asset_type, &state.asset_path, &assets, into)
                .ok_or_else(|| {
                    AnimaError::other(format!("{}: its picture is not in the file", c.name))
                })?;
        }
    }

    if into.exists() {
        return Err(AnimaError::other("the folder for its pictures is taken"));
    }
    let unpacked = unpack(&assets, into);
    if unpacked.is_err() {
        let _ = std::fs::remove_dir_all(into);
    }
    unpacked?;
    Ok(scene)
}

/// Write the pictures into `into`, each then held to what a drop is.
fn unpack(assets: &[zip::Entry<'_>], into: &Path) -> Result<()> {
    std::fs::create_dir_all(into)?;
    for entry in assets {
        let path = into.join(entry.name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, entry.data)?;
        crate::drop_validate::pre_validate_dropped_file(&path)
            .map_err(|why| AnimaError::other(format!("{}: {why}", entry.name)))?;
    }
    Ok(())
}

/// `assets/<number>/<name>.<ext>`, nothing else: short plain names, one
/// folder deep, and an extension a drop takes.
fn is_asset_name(name: &str) -> bool {
    let mut parts = name.split('/');
    let (Some("assets"), Some(number), Some(file), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    let plain = |s: &str| {
        !s.is_empty()
            && s.len() <= 64
            && !s.starts_with('.')
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    };
    number.len() <= 6
        && !number.is_empty()
        && number.chars().all(|c| c.is_ascii_digit())
        && plain(file)
        && file
            .rsplit_once('.')
            .is_some_and(|(_, ext)| DROP_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
}

/// Where a character's picture `asset_path` — as the file names it — is
/// once unpacked into `into`; `None` when the file does not have it. A
/// sequence names its folder, which must hold frames.
fn local_path(
    asset_type: &AssetType,
    asset_path: &str,
    assets: &[zip::Entry<'_>],
    into: &Path,
) -> Option<String> {
    let found = if *asset_type == AssetType::PngSequence {
        let folder = format!("{asset_path}/");
        is_asset_name(&format!("{asset_path}/x.png"))
            && assets
                .iter()
                .any(|e| e.name.starts_with(&folder) && e.name.ends_with(".png"))
    } else {
        is_asset_name(asset_path) && assets.iter().any(|e| e.name == asset_path)
    };
    found.then(|| into.join(asset_path).to_string_lossy().into_owned())
}

/// What a scene from someone else may hold: at most `MAX_ENTITIES`
/// characters with finite, bounded numbers, distinct ids, no script and
/// no monitor of theirs; a plain name; groups of its own characters.
fn tidy(scene: &mut SavedScene) {
    let name: String = scene
        .name
        .chars()
        .filter(|c| !c.is_control())
        .take(60)
        .collect();
    scene.name = match name.trim() {
        "" => "Shared scene".to_string(),
        trimmed => trimmed.to_string(),
    };
    scene.characters.truncate(MAX_ENTITIES);
    let mut ids = std::collections::BTreeSet::new();
    for c in &mut scene.characters {
        c.sanitize();
        if matches!(c.behavior, crate::behavior::Behavior::Script { .. }) {
            c.behavior = crate::behavior::Behavior::Idle;
        }
        c.monitor = None;
        c.name = c
            .name
            .chars()
            .filter(|ch| !ch.is_control())
            .take(60)
            .collect();
        let mut id: String =
            c.id.chars()
                .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
                .take(40)
                .collect();
        if id.is_empty() {
            id = "character".into();
        }
        let base = id.clone();
        let mut n = 2;
        while !ids.insert(id.clone()) {
            id = format!("{base}-{n}");
            n += 1;
        }
        c.id = id;
    }
    let known: std::collections::BTreeSet<&str> =
        scene.characters.iter().map(|c| c.id.as_str()).collect();
    scene.groups.truncate(MAX_ENTITIES);
    for g in &mut scene.groups {
        g.member_ids.retain(|m| known.contains(m.as_str()));
        g.name = g
            .name
            .chars()
            .filter(|ch| !ch.is_control())
            .take(60)
            .collect();
        g.scale = if g.scale.is_finite() {
            g.scale.clamp(0.1, 5.0)
        } else {
            1.0
        };
        g.offset_x = if g.offset_x.is_finite() {
            g.offset_x
        } else {
            0.0
        };
        g.offset_y = if g.offset_y.is_finite() {
            g.offset_y
        } else {
            0.0
        };
    }
    scene.groups.retain(|g| !g.member_ids.is_empty());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CharacterConfig;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("anima-share-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// The demo scene: a sequence (the cat), still pictures, all shipped.
    fn demo() -> SavedScene {
        let config = AppConfig::default();
        SavedScene {
            name: "Demo".into(),
            characters: config.characters.clone(),
            groups: Vec::new(),
        }
    }

    #[test]
    fn a_scene_goes_out_and_comes_back() {
        let dir = tmp("round");
        let mut saved = demo();
        saved.characters[0].behavior = crate::behavior::Behavior::Script {
            path: "hello.rhai".into(),
            params: Default::default(),
        };
        saved.characters[1].monitor = Some("HDMI-A-1".into());
        let file = dir.join("demo.animascene");
        let exported = export(&saved, &file).unwrap();
        assert_eq!(exported.characters, saved.characters.len());
        assert_eq!(exported.scripts_left_out, 1);
        assert!(is_scene_file(&file));

        let into = dir.join("unpacked");
        let back = import(&file, &into).unwrap();
        assert_eq!(back.name, "Demo");
        assert_eq!(back.characters.len(), saved.characters.len());
        for (b, s) in back.characters.iter().zip(&saved.characters) {
            assert_eq!((b.id.as_str(), b.x, b.y), (s.id.as_str(), s.x, s.y));
            assert!(
                Path::new(&b.asset_path).starts_with(&into),
                "{}",
                b.asset_path
            );
            assert!(Path::new(&b.asset_path).exists());
            assert!(b.monitor.is_none());
        }
        assert_eq!(back.characters[0].behavior, crate::behavior::Behavior::Idle);
        // And the pictures load, not the stand-in for a broken one.
        let mut scene = crate::scene::Scene::from_config(&AppConfig::default());
        scene.entities.clear();
        for c in &back.characters {
            scene.append_character_config(c).unwrap();
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_folder_already_there_is_not_written_into() {
        let dir = tmp("taken");
        let file = dir.join("s.animascene");
        export(&demo(), &file).unwrap();
        let into = dir.join("taken");
        std::fs::create_dir_all(&into).unwrap();
        assert!(import(&file, &into).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_plain_asset_names_are_taken() {
        for good in [
            "assets/0/asset.png",
            "assets/12/0003.png",
            "assets/1/asset.MP4",
        ] {
            assert!(is_asset_name(good), "{good}");
        }
        for bad in [
            "assets/../etc/passwd.png",
            "/assets/0/a.png",
            "assets/0/../../x.png",
            "assets/0/a.exe",
            "assets/0/sub/a.png",
            "assets/x/a.png",
            "assets/0/.hidden.png",
            "assets/0/a b.png",
            "assets\\0\\a.png",
            "scripts/0/a.rhai",
            "assets/0/",
        ] {
            assert!(!is_asset_name(bad), "{bad}");
        }
    }

    #[test]
    fn a_hand_made_file_is_tidied() {
        let dir = tmp("tidy");
        let mut saved = demo();
        saved.name = "\u{1b}[31m   ".into();
        let dup = saved.characters[0].clone();
        saved.characters.push(dup);
        saved.characters[0].scale = f32::NAN;
        saved.groups.push(crate::group::GroupConfig {
            id: "g".into(),
            name: "G".into(),
            member_ids: vec!["nobody".into()],
            offset_x: f32::INFINITY,
            offset_y: 0.0,
            scale: 1.0,
            visible: true,
        });
        let file = dir.join("t.animascene");
        export(&saved, &file).unwrap();
        let back = import(&file, &dir.join("in")).unwrap();
        assert_eq!(back.name, "[31m");
        let ids: std::collections::BTreeSet<_> =
            back.characters.iter().map(|c| c.id.clone()).collect();
        assert_eq!(ids.len(), back.characters.len(), "ids made distinct");
        assert!(back.characters[0].scale.is_finite());
        assert!(back.groups.is_empty(), "a group of nobody goes");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_character_whose_picture_is_missing_fails_the_import() {
        let dir = tmp("missing");
        let manifest = toml::to_string_pretty(&Manifest {
            format: FORMAT,
            scene: SavedScene {
                name: "M".into(),
                characters: vec![CharacterConfig {
                    asset_path: "assets/0/asset.png".into(),
                    ..demo().characters[1].clone()
                }],
                groups: Vec::new(),
            },
        })
        .unwrap();
        let mut w = zip::Writer::new(Vec::new());
        w.add(MANIFEST, manifest.as_bytes()).unwrap();
        let file = dir.join("m.animascene");
        std::fs::write(&file, w.finish().unwrap()).unwrap();
        let into = dir.join("in");
        assert!(import(&file, &into).is_err());
        assert!(!into.exists(), "nothing left behind");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_newer_format_is_refused_with_a_reason() {
        let dir = tmp("newer");
        let text = "format = 99\n[scene]\nname = \"N\"\n";
        let mut w = zip::Writer::new(Vec::new());
        w.add(MANIFEST, text.as_bytes()).unwrap();
        let file = dir.join("n.animascene");
        std::fs::write(&file, w.finish().unwrap()).unwrap();
        let err = import(&file, &dir.join("in")).unwrap_err().to_string();
        assert!(err.contains("newer"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
