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
//! under caps. What the settings hold that this version cannot read and
//! tidy is left behind, and settings larger than a config are refused.
//! Restoring adds the scenes under free names — nothing saved
//! here is written over — and replaces the settings, keeping the old file
//! beside as `config.toml.before-restore`.

use super::{zip, Packer, MAX_ENTRIES, MAX_FILE_BYTES};
use crate::config::{AppConfig, CharacterConfig};
use crate::constants::MAX_CONFIG_BYTES;
use crate::error::{AnimaError, Result};
use crate::i18n::Msg;
use crate::scenes::SavedScene;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// A backup's extension.
pub const EXTENSION: &str = "animabackup";

/// The manifest inside.
const MANIFEST: &str = "backup.toml";

/// This layout.
const FORMAT: u32 = 1;

/// The manifest holds every scene: it may be several configs' worth.
const MAX_MANIFEST_BYTES: u64 = 4 * MAX_CONFIG_BYTES;

/// `backup.toml` as written: the config as its own file says it, what
/// it set aside included (`AppConfig::to_toml_string`).
#[derive(Serialize)]
struct Manifest<'a> {
    format: u32,
    config: toml::Table,
    scenes: &'a [SavedScene],
}

/// The text of `backup.toml` for `config` and `scenes`.
fn manifest_text(config: &AppConfig, scenes: &[SavedScene]) -> Result<String> {
    let config: toml::Table = config
        .to_toml_string()?
        .parse()
        .map_err(|e| AnimaError::other(format!("{e}")))?;
    Ok(toml::to_string_pretty(&Manifest {
        format: FORMAT,
        config,
        scenes,
    })?)
}

/// `backup.toml` as this version reads it: the format first, so a newer
/// one says so before anything in it fails to read; the config as
/// `config.toml` is read, what does not read set aside and kept
/// (`AppConfig::decode_config_table`); each scene as a scene file is,
/// a character or group that does not read left out
/// (`scenes::from_table`). One of them used to fail the whole restore.
fn read_manifest(text: &str) -> Result<(AppConfig, Vec<SavedScene>)> {
    let mut manifest: toml::Table = text
        .parse()
        .map_err(|e| super::damaged(format!("backup.toml: {e}")))?;
    let format = manifest
        .get("format")
        .and_then(toml::Value::as_integer)
        .and_then(|f| u32::try_from(f).ok())
        .ok_or_else(|| super::damaged("backup.toml: no format"))?;
    if format > FORMAT {
        return Err(AnimaError::said_as(Msg::new("error-file-newer")));
    }
    let Some(toml::Value::Table(config)) = manifest.remove("config") else {
        return Err(super::damaged("backup.toml: no config"));
    };
    let (config, _) = AppConfig::decode_config_table(config)
        .map_err(|e| super::damaged(format!("backup.toml: {e}")))?;
    let scenes = match manifest.remove("scenes") {
        None => Vec::new(),
        Some(toml::Value::Array(items)) => {
            let mut scenes = Vec::new();
            for item in items {
                let toml::Value::Table(table) = item else {
                    return Err(super::damaged("backup.toml: a scene is not a table"));
                };
                let (scene, left_out) = crate::scenes::from_table(table)
                    .map_err(|e| super::damaged(format!("backup.toml: {e}")))?;
                if left_out > 0 {
                    tracing::warn!(
                        "Backup scene {}: {left_out} left out, not read",
                        crate::drop_validate::escape_untrusted(&scene.name)
                    );
                }
                scenes.push(scene);
            }
            scenes
        }
        Some(_) => return Err(super::damaged("backup.toml: scenes are not a list")),
    };
    Ok((config, scenes))
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
    let manifest = manifest_text(&config, &saved)?;
    let written = write(to, manifest.as_bytes(), &thumbs, &packer.files)?;
    Ok(BackedUp {
        scenes: scenes.len(),
        bytes: written,
    })
}

fn write(
    to: &Path,
    manifest: &[u8],
    thumbs: &[(String, Vec<u8>)],
    files: &[(String, PathBuf)],
) -> Result<u64> {
    // Measured before `to` is opened, which empties it; refused on a
    // picture, the backup this one would replace used to be left empty,
    // then removed (SEC-010).
    let extra = manifest.len() as u64 + thumbs.iter().map(|(_, p)| p.len() as u64).sum::<u64>();
    super::files_fit(files, extra, MAX_FILE_BYTES, "error-backup-too-big")?;
    let file = create_owners(to)?;
    let written = write_into(file, manifest, thumbs, files);
    // Past the opening, a failure is a part-written backup: not kept.
    if written.is_err() {
        let _ = std::fs::remove_file(to);
    }
    written
}

/// `to`, created or emptied, the owner's alone: a backup holds the
/// settings and every scene, and was made with the umask's mode,
/// readable by every local account (SEC-010).
fn create_owners(to: &Path) -> std::io::Result<std::fs::File> {
    let mut open = std::fs::OpenOptions::new();
    open.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        open.mode(0o600);
        let file = open.open(to)?;
        // An existing file keeps its mode through `open`: set it too.
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        Ok(file)
    }
    #[cfg(not(unix))]
    open.open(to)
}

fn write_into(
    file: std::fs::File,
    manifest: &[u8],
    thumbs: &[(String, Vec<u8>)],
    files: &[(String, PathBuf)],
) -> Result<u64> {
    use std::io::Write;
    let out = std::io::BufWriter::new(file);
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
            return Err(AnimaError::said_as(
                Msg::new("error-backup-too-big").arg("max", MAX_FILE_BYTES / (1024 * 1024)),
            ));
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
    /// Where [`read`] unpacked the pictures.
    pub dir: PathBuf,
}

/// For `super::fuzz_read`: a backup's manifest parsed and each scene in
/// it tidied, as a restore would.
#[doc(hidden)]
pub fn fuzz_manifest(text: &str) {
    if let Ok((_config, scenes)) = read_manifest(text) {
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
        return Err(AnimaError::said_as(Msg::new("error-not-a-file")));
    }
    if meta.len() > MAX_FILE_BYTES {
        return Err(super::too_big_to_open());
    }
    // Held to the cap as it is read too (`scene_file::read_whole`).
    let bytes = super::read_whole(file)?;
    let entries = zip::read(&bytes, MAX_ENTRIES)
        .map_err(|e| super::not_ours_or_damaged(&e, "error-not-a-backup"))?;
    let manifest = entries
        .iter()
        .find(|e| e.name == MANIFEST)
        .ok_or_else(|| AnimaError::said_as(Msg::new("error-not-a-backup")))?;
    if manifest.data.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(super::damaged("its backup.toml is too big"));
    }
    let text = std::str::from_utf8(manifest.data)
        .map_err(|_| super::damaged("its backup.toml is not text"))?;
    let (mut config, scenes) = read_manifest(text)?;
    // A backup may come from anyone, so it is held to what this version
    // can read and tidy. What it cannot — a newer version's character or
    // setting, set aside verbatim, a key it does not know — never went
    // through `tidy`: a script behavior, a path outside the file, a sound
    // reached `config.toml` as written, for a version that reads them to
    // run (SEC-005). They stay behind; a local config keeps its own.
    let left_behind = config.set_aside.len() + config.extra.len();
    if left_behind > 0 {
        tracing::warn!(
            "Backup: {left_behind} setting(s) or entries this version cannot read left behind"
        );
    }
    config.set_aside = Default::default();
    config.extra.clear();
    // A sound file stays behind, as a character's does in `tidy`.
    if matches!(
        config.global.reminder_sound,
        crate::reminders::ReminderSound::File(_)
    ) {
        config.global.reminder_sound = crate::reminders::ReminderSound::None;
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
            return Err(super::damaged(format!(
                "unexpected file inside: {}",
                crate::drop_validate::redact_path(Path::new(entry.name))
            )));
        }
        if entry.data.len() as u64 > crate::constants::MAX_ASSET_FILE_BYTES {
            return Err(super::damaged(format!(
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
        c.asset_path = super::local_path(&c.asset_type, &c.asset_path, &assets, into)
            .ok_or_else(|| super::damaged(format!("{}: its picture is not in the file", c.name)))?;
        for state in c.animations.values_mut() {
            state.asset_path =
                super::local_path(&state.asset_type, &state.asset_path, &assets, into).ok_or_else(
                    || super::damaged(format!("{}: its picture is not in the file", c.name)),
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

    // Settings that would not load again are refused here, before
    // anything is written: `backup.toml` may hold four configs' worth for
    // its scenes, and a config past MAX_CONFIG_BYTES fell back to the
    // defaults on the next start.
    if config.to_toml_string()?.len() as u64 > MAX_CONFIG_BYTES {
        return Err(super::damaged(
            "its settings are larger than a config may be",
        ));
    }

    super::unpack(&assets, into)?;
    Ok(Restored {
        config,
        scenes: restored,
        dir: into.to_path_buf(),
    })
}

/// Put `restored` in place: each scene on the shelf under a free name,
/// with its picture, and the settings over this machine's — kept first as
/// `config.toml.before-restore`. The running app picks the settings up as
/// it does an edit of the file. Returns how many scenes came.
pub fn put_in_place(restored: Restored) -> Result<usize> {
    let pictures = restored.dir.clone();
    let mut count = 0;
    let done = put_each(restored, &mut count);
    // Stopped before its first scene, and the settings not written:
    // nothing points into the pictures' folder.
    if done.is_err() && count == 0 {
        super::forget_unpacked(Some(&pictures));
    }
    done.map(|()| count)
}

/// [`put_in_place`]'s work: `count` says how many scenes are shelved.
fn put_each(restored: Restored, count: &mut usize) -> Result<()> {
    let dir = crate::scenes::dir();
    for (mut scene, thumb) in restored.scenes {
        scene.name = crate::scenes::free_name(&crate::scenes::list_in(&dir), &scene.name);
        let path = crate::scenes::write_in(&dir, &scene)?;
        // On the shelf, pointing into the pictures' folder, from here.
        *count += 1;
        if let Some(image) = thumb {
            let png = crate::scene_thumb::encode(&image)?;
            crate::util::atomic_write_bytes(&crate::scene_thumb::path_for(&path), &png)?;
        }
    }
    let config_path = AppConfig::config_path();
    if config_path.exists() {
        // The user's alone, and never through a link (`util::copy_private`).
        crate::util::copy_private(
            &config_path,
            &config_path.with_extension("toml.before-restore"),
        )?;
    }
    restored.config.save()?;
    Ok(())
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

    /// A backup written over an older one that fails on its pictures —
    /// one gone, or not a file — leaves the older one as it was: the file
    /// was opened (truncated) first and then removed on the failure
    /// (SEC-010).
    #[test]
    fn a_failed_backup_leaves_the_last_one() {
        let dir = tmp("failed_over");
        let file = dir.join("all.animabackup");
        std::fs::write(&file, b"the backup made last week").unwrap();
        let files = vec![("assets/0/a.png".to_string(), dir.join("gone.png"))];
        assert!(write(&file, b"manifest", &[], &files).is_err());
        assert_eq!(std::fs::read(&file).unwrap(), b"the backup made last week");
    }

    /// A backup holds the settings and every scene: the owner's alone.
    #[cfg(unix)]
    #[test]
    fn a_backup_is_the_owners_alone() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tmp("mode");
        let file = dir.join("all.animabackup");
        write(&file, b"manifest", &[], &[]).unwrap();
        let mode = std::fs::metadata(&file).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "{mode:o}");
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

    /// What the config set aside — a newer version's character, its new
    /// setting — goes into a backup, and a restore reads past it instead
    /// of failing whole on it, as it once did. It does not come back: a
    /// backup may come from anyone, and what this version cannot read it
    /// cannot tidy (SEC-005). A newer version restoring the same backup
    /// reads those entries, and tidies them.
    #[test]
    fn what_the_config_set_aside_goes_out_but_does_not_come_back() {
        let dir = tmp("set_aside");
        let text = r#"
            version = 2
            [global]
            future_glow = 3
            theme = "sepia"
            [[characters]]
            id = "holo"
            name = "Holo"
            asset_type = "hologram"
            asset_path = "h.holo"
            x = 0.0
            y = 0.0
        "#;
        let (config, _) = AppConfig::decode_config_table(text.parse().unwrap()).unwrap();
        assert!(config.set_aside.unread());
        let file = dir.join("b.animabackup");
        back_up(&config, &[], &file).unwrap();
        let bytes = std::fs::read(&file).unwrap();
        let entries = zip::read(&bytes, MAX_ENTRIES).unwrap();
        let manifest = entries.iter().find(|e| e.name == MANIFEST).unwrap();
        let manifest = std::str::from_utf8(manifest.data).unwrap();
        assert!(manifest.contains("future_glow") && manifest.contains("holo"));

        let restored = read(&file, &dir.join("in")).expect("read past it");
        assert!(!restored.config.set_aside.unread(), "left behind");
        let back: toml::Table = restored.config.to_toml_string().unwrap().parse().unwrap();
        assert!(back["global"].get("future_glow").is_none());
        assert!(!restored.config.to_toml_string().unwrap().contains("holo"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A backup holding `manifest` as its `backup.toml`, and nothing else.
    fn backup_with(dir: &Path, manifest: &str) -> PathBuf {
        let mut w = zip::Writer::new(Vec::new());
        w.add(MANIFEST, manifest.as_bytes()).unwrap();
        let file = dir.join("b.animabackup");
        std::fs::write(&file, w.finish().unwrap()).unwrap();
        file
    }

    /// A backup may come from anyone. A character this version cannot
    /// read is set aside verbatim — and so it skipped `tidy`: a script
    /// behavior, a path outside the backup and a sound went into
    /// `config.toml` as written, for a version that can read it to run
    /// (SEC-005). What cannot be read and tidied stays behind now.
    #[test]
    fn what_a_backup_cannot_tidy_stays_behind() {
        let dir = tmp("smuggle");
        let manifest = r#"
            format = 1
            [config]
            version = 2
            future_key = "anything"
            [config.global]
            reminder_sound = { file = "chime.ogg" }
            [[config.characters]]
            id = "evil"
            name = "Evil"
            asset_type = "png_static"
            asset_path = "/etc/passwd"
            x = 0.0
            y = 0.0
            behavior = { type = "script", path = "evil.rhai" }
            easing = "future_easing_variant_v2"
            [config.characters.talk]
            sound = "evil.wav"
        "#;
        let file = backup_with(&dir, manifest);
        let restored = read(&file, &dir.join("in")).expect("the rest restores");
        let text = restored.config.to_toml_string().unwrap();
        for smuggled in [
            "evil.rhai",
            "/etc/passwd",
            "evil.wav",
            "future_key",
            "chime.ogg",
        ] {
            assert!(
                !text.contains(smuggled),
                "{smuggled} reached config.toml:\n{text}"
            );
        }
        assert!(!restored.config.set_aside.unread());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `backup.toml` may be four configs' worth, for its scenes; settings
    /// that come out larger than a config may be were written, and then
    /// did not load — the next start fell back to the defaults.
    #[test]
    fn settings_larger_than_a_config_are_refused_before_anything_is_written() {
        let dir = tmp("huge");
        let app = "x".repeat(4096);
        let apps: Vec<String> = (0..2200).map(|i| format!("\"{app}{i}\"")).collect();
        let manifest = format!(
            "format = 1\n[config]\nversion = 2\n[config.global]\nstep_aside_apps = [{}]\n",
            apps.join(",")
        );
        assert!(manifest.len() as u64 > MAX_CONFIG_BYTES);
        let file = backup_with(&dir, &manifest);
        let into = dir.join("in");
        assert!(
            read(&file, &into).is_err(),
            "a config too big to load was accepted"
        );
        assert!(!into.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn something_unexpected_inside_fails_the_restore() {
        let dir = tmp("odd");
        let manifest = manifest_text(
            &AppConfig {
                characters: Vec::new(),
                ..AppConfig::default()
            },
            &[],
        )
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
