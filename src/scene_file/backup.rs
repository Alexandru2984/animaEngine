//! Backing everything up in one file (1.5): `name.animabackup` holds the
//! settings, every saved scene with its picture, and the pictures they
//! all show, each once — to move to another machine, or to keep.
//!
//! It is the scene file's zip (`super::zip`) with a manifest of its own,
//! `backup.toml`, and it is read back with the same suspicion as a scene
//! file from someone else: the same reader and limits, each scene and the
//! settings' characters tidied as `super::tidy` does (scripts stood still,
//! sounds left behind, no monitors), the pictures unpacked into a folder
//! that did not exist and held to the drop checks, scene pictures decoded
//! under caps. Restoring adds the scenes under free names — nothing saved
//! here is written over — and replaces the settings, keeping the old file
//! beside as `config.toml.before-restore`.

use super::{zip, Packer, MAX_ENTRIES, MAX_FILE_BYTES};
use crate::config::{AppConfig, CharacterConfig};
use crate::constants::MAX_CONFIG_BYTES;
use crate::error::{AnimaError, Result};
use crate::scenes::SavedScene;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A backup's extension.
pub const EXTENSION: &str = "animabackup";

/// The manifest inside.
const MANIFEST: &str = "backup.toml";

/// This layout.
const FORMAT: u32 = 1;

/// The manifest holds every scene: it may be several configs' worth.
const MAX_MANIFEST_BYTES: u64 = 4 * MAX_CONFIG_BYTES;

#[derive(Serialize, Deserialize)]
struct Manifest {
    format: u32,
    config: AppConfig,
    #[serde(default)]
    scenes: Vec<SavedScene>,
}

/// Whether `path` names a backup.
pub fn is_backup(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION))
}

/// What a backup wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackedUp {
    pub scenes: usize,
    pub bytes: u64,
}

/// Every character of `config`: the scene on screen's, and the older
/// per-window lists'.
fn characters_mut(config: &mut AppConfig) -> impl Iterator<Item = &mut CharacterConfig> {
    config.characters.iter_mut().chain(
        config
            .windows
            .iter_mut()
            .flat_map(|w| w.characters.iter_mut()),
    )
}

/// Write `config` and the saved `scenes` — each with its picture, if it
/// has one — and every picture they show, to `to`.
pub fn back_up(
    config: &AppConfig,
    scenes: &[(SavedScene, Option<Vec<u8>>)],
    to: &Path,
) -> Result<BackedUp> {
    let mut packer = Packer::default();
    let mut config = config.clone();
    for c in characters_mut(&mut config) {
        super::pack_character(c, &mut packer)?;
    }
    let mut saved = Vec::new();
    let mut thumbs = Vec::new();
    for (i, (scene, thumb)) in scenes.iter().enumerate() {
        let mut scene = scene.clone();
        for c in &mut scene.characters {
            super::pack_character(c, &mut packer)?;
        }
        saved.push(scene);
        if let Some(png) = thumb
            .as_ref()
            .filter(|png| crate::scene_thumb::decode(png).is_ok())
        {
            thumbs.push((format!("thumbs/{i}.png"), png.clone()));
        }
    }
    let manifest = toml::to_string_pretty(&Manifest {
        format: FORMAT,
        config,
        scenes: saved,
    })?;
    let written = write(to, manifest.as_bytes(), &thumbs, &packer.files);
    if written.is_err() {
        let _ = std::fs::remove_file(to);
    }
    Ok(BackedUp {
        scenes: scenes.len(),
        bytes: written?,
    })
}

fn write(
    to: &Path,
    manifest: &[u8],
    thumbs: &[(String, Vec<u8>)],
    files: &[(String, PathBuf)],
) -> Result<u64> {
    use std::io::Write;
    let out = std::io::BufWriter::new(std::fs::File::create(to)?);
    let mut zip = zip::Writer::new(out);
    zip.add(MANIFEST, manifest)?;
    let mut total = manifest.len() as u64;
    for (name, png) in thumbs {
        zip.add(name, png)?;
        total += png.len() as u64;
    }
    for (name, source) in files {
        let data = super::read_asset(source)?;
        total += data.len() as u64;
        if total > MAX_FILE_BYTES {
            return Err(AnimaError::other(format!(
                "the backup is over {} MB",
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

/// What a backup held, read and checked, its pictures unpacked.
#[derive(Debug)]
pub struct Restored {
    pub config: AppConfig,
    /// Each scene, and its picture if it had one.
    pub scenes: Vec<(SavedScene, Option<image::RgbaImage>)>,
}

/// For `super::fuzz_read`: a backup's manifest parsed and each scene in
/// it tidied, as a restore would.
#[doc(hidden)]
pub fn fuzz_manifest(text: &str) {
    if let Ok(Manifest {
        mut config, scenes, ..
    }) = toml::from_str::<Manifest>(text)
    {
        config.sanitize();
        for mut scene in scenes {
            super::tidy(&mut scene);
        }
    }
}

/// For `super::fuzz_read`.
#[doc(hidden)]
pub fn fuzz_thumb_index(name: &str) -> Option<usize> {
    thumb_index(name)
}

/// `thumbs/<number>.png`, the number of a scene in the manifest.
fn thumb_index(name: &str) -> Option<usize> {
    let n = name.strip_prefix("thumbs/")?.strip_suffix(".png")?;
    (!n.is_empty() && n.len() <= 4 && n.chars().all(|c| c.is_ascii_digit()))
        .then(|| n.parse().ok())
        .flatten()
}

/// Read the backup `file`, and put its pictures in `into` — a folder that
/// does not exist yet. On failure nothing is left in `into`.
pub fn read(file: &Path, into: &Path) -> Result<Restored> {
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
        .ok_or_else(|| AnimaError::other("not a backup: no backup.toml inside"))?;
    if manifest.data.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(AnimaError::other("its backup.toml is too big"));
    }
    let text = std::str::from_utf8(manifest.data)
        .map_err(|_| AnimaError::other("its backup.toml is not text"))?;
    let Manifest {
        format,
        mut config,
        scenes,
    } = toml::from_str(text).map_err(|e| AnimaError::other(format!("backup.toml: {e}")))?;
    if format > FORMAT {
        return Err(AnimaError::other(
            "made by a newer animaEngine; update to open it",
        ));
    }

    let mut assets = Vec::new();
    let mut thumbs: Vec<(usize, image::RgbaImage)> = Vec::new();
    for entry in &entries {
        if entry.name == MANIFEST {
            continue;
        }
        if let Some(i) = thumb_index(entry.name) {
            thumbs.push((i, crate::scene_thumb::decode(entry.data)?));
            continue;
        }
        if !super::is_asset_name(entry.name) {
            return Err(AnimaError::other(format!(
                "unexpected file inside: {}",
                crate::drop_validate::redact_path(Path::new(entry.name))
            )));
        }
        if entry.data.len() as u64 > crate::constants::MAX_ASSET_FILE_BYTES {
            return Err(AnimaError::other(format!(
                "{} is over the size a picture may be",
                entry.name
            )));
        }
        assets.push(*entry);
    }

    // The settings' own scene, checked as any scene is.
    let mut own = SavedScene {
        name: "backup".into(),
        characters: std::mem::take(&mut config.characters),
        groups: std::mem::take(&mut config.groups),
    };
    super::tidy(&mut own);
    config.characters = own.characters;
    config.groups = own.groups;
    for w in &mut config.windows {
        let mut scene = SavedScene {
            name: "backup".into(),
            characters: std::mem::take(&mut w.characters),
            groups: Vec::new(),
        };
        super::tidy(&mut scene);
        w.characters = scene.characters;
    }
    let point = |c: &mut CharacterConfig| -> Result<()> {
        c.asset_path =
            super::local_path(&c.asset_type, &c.asset_path, &assets, into).ok_or_else(|| {
                AnimaError::other(format!("{}: its picture is not in the file", c.name))
            })?;
        for state in c.animations.values_mut() {
            state.asset_path =
                super::local_path(&state.asset_type, &state.asset_path, &assets, into).ok_or_else(
                    || AnimaError::other(format!("{}: its picture is not in the file", c.name)),
                )?;
        }
        Ok(())
    };
    for c in characters_mut(&mut config) {
        point(c)?;
    }
    config.sanitize();

    let mut restored = Vec::new();
    for (i, mut scene) in scenes
        .into_iter()
        .take(crate::scenes::MAX_SCENES)
        .enumerate()
    {
        super::tidy(&mut scene);
        for c in &mut scene.characters {
            point(c)?;
        }
        let thumb = thumbs
            .iter()
            .position(|(n, _)| *n == i)
            .map(|k| thumbs.swap_remove(k).1);
        restored.push((scene, thumb));
    }

    if into.exists() {
        return Err(AnimaError::other("the folder for its pictures is taken"));
    }
    let unpacked = super::unpack(&assets, into);
    if unpacked.is_err() {
        let _ = std::fs::remove_dir_all(into);
    }
    unpacked?;
    Ok(Restored {
        config,
        scenes: restored,
    })
}

/// Put `restored` in place: each scene on the shelf under a free name,
/// with its picture, and the settings over this machine's — kept first as
/// `config.toml.before-restore`. The running app picks the settings up as
/// it does an edit of the file. Returns how many scenes came.
pub fn put_in_place(restored: Restored) -> Result<usize> {
    let dir = crate::scenes::dir();
    let mut count = 0;
    for (mut scene, thumb) in restored.scenes {
        scene.name = crate::scenes::free_name(&crate::scenes::list_in(&dir), &scene.name);
        let path = crate::scenes::write_in(&dir, &scene)?;
        if let Some(image) = thumb {
            let png = crate::scene_thumb::encode(&image)?;
            crate::util::atomic_write_bytes(&crate::scene_thumb::path_for(&path), &png)?;
        }
        count += 1;
    }
    let config_path = AppConfig::config_path();
    if config_path.exists() {
        std::fs::copy(
            &config_path,
            config_path.with_extension("toml.before-restore"),
        )?;
    }
    restored.config.save()?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("backup_tests")
            .join(tag);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn settings_and_scenes_go_out_and_come_back() {
        let dir = tmp("round");
        let mut config = AppConfig::default();
        config.global.theme = crate::ui::Theme::Light;
        let scene = SavedScene {
            name: "Work".into(),
            characters: config.characters[..2].to_vec(),
            groups: Vec::new(),
        };
        let thumb = image::RgbaImage::from_pixel(160, 100, image::Rgba([1, 2, 3, 255]));
        let png = crate::scene_thumb::encode(&thumb).unwrap();
        let file = dir.join("all.animabackup");
        let out = back_up(&config, &[(scene, Some(png))], &file).unwrap();
        assert_eq!(out.scenes, 1);
        assert!(is_backup(&file));

        let into = dir.join("in");
        let back = read(&file, &into).unwrap();
        assert_eq!(back.config.global.theme, crate::ui::Theme::Light);
        assert_eq!(back.config.characters.len(), config.characters.len());
        for c in &back.config.characters {
            assert!(
                Path::new(&c.asset_path).starts_with(&into),
                "{}",
                c.asset_path
            );
        }
        assert_eq!(back.scenes.len(), 1);
        assert_eq!(back.scenes[0].0.name, "Work");
        assert_eq!(back.scenes[0].1.as_ref(), Some(&thumb));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn something_unexpected_inside_fails_the_restore() {
        let dir = tmp("odd");
        let manifest = toml::to_string_pretty(&Manifest {
            format: FORMAT,
            config: AppConfig {
                characters: Vec::new(),
                ..AppConfig::default()
            },
            scenes: Vec::new(),
        })
        .unwrap();
        let mut w = zip::Writer::new(Vec::new());
        w.add(MANIFEST, manifest.as_bytes()).unwrap();
        w.add("../../etc/evil", b"x").unwrap();
        let file = dir.join("odd.animabackup");
        std::fs::write(&file, w.finish().unwrap()).unwrap();
        let into = dir.join("in");
        assert!(read(&file, &into).is_err());
        assert!(!into.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scene_pictures_are_named_by_number_only() {
        assert_eq!(thumb_index("thumbs/3.png"), Some(3));
        assert_eq!(thumb_index("thumbs/x.png"), None);
        assert_eq!(thumb_index("thumbs/.png"), None);
        assert_eq!(thumb_index("thumbs/3.png.exe"), None);
    }
}
