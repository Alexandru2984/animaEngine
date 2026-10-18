//! Named scenes (1.5): what is on screen — the characters and their
//! groups — saved under a name to come back to. One TOML file each in
//! `scenes/` beside `config.toml`, so a scene can be copied, shared or
//! edited by hand like the config.
//!
//! A scene file is trusted as far as `config.toml` is — it sits in the
//! same directory, the user's own — and is loaded under the same limits:
//! a capped read, at most `MAX_ENTITIES` characters, their values
//! sanitised, and the decode budget in `Scene::restore_configs`.
//! Loading goes through that same restore as undo does, so characters
//! showing the same asset keep their textures, and an undo brings back
//! what was there before.

use crate::config::{AppConfig, CharacterConfig, SetAside};
use crate::constants::{MAX_CONFIG_BYTES, MAX_ENTITIES};
use crate::error::{AnimaError, Result};
use crate::group::GroupConfig;
use crate::scene::Scene;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A scene as saved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedScene {
    pub name: String,
    #[serde(default)]
    pub characters: Vec<CharacterConfig>,
    #[serde(default)]
    pub groups: Vec<GroupConfig>,
}

/// A scene on disk: its name, and its file.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneEntry {
    pub name: String,
    pub path: PathBuf,
}

/// Where the scenes are.
pub fn dir() -> PathBuf {
    AppConfig::config_path().with_file_name("scenes")
}

/// At most this many scene files are looked at.
pub const MAX_SCENES: usize = 200;

/// Every scene saved in `dir`, by name. Unreadable files are left out.
pub fn list_in(dir: &Path) -> Vec<SceneEntry> {
    #[derive(Deserialize)]
    struct NameOnly {
        name: String,
    }
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut entries: Vec<SceneEntry> = read
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .take(MAX_SCENES)
        .filter_map(|path| {
            let text = crate::util::read_to_string_capped(&path, MAX_CONFIG_BYTES).ok()?;
            let NameOnly { name } = toml::from_str(&text).ok()?;
            let name = name.trim().to_string();
            (!name.is_empty()).then_some(SceneEntry { name, path })
        })
        .collect();
    entries.sort_by_key(|e| e.name.to_lowercase());
    entries
}

/// Set by a save or a delete: the next [`shelf`] lists again.
static SHELF_STALE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Every saved scene, by name — looked up at most every two seconds, or
/// after a save or a delete, so the Scene tab and the palette can ask
/// every frame.
pub fn shelf() -> Vec<SceneEntry> {
    use std::sync::atomic::Ordering;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    static CACHE: Mutex<Option<(Instant, Vec<SceneEntry>)>> = Mutex::new(None);
    let stale = SHELF_STALE.swap(false, Ordering::Relaxed);
    let Ok(mut cache) = CACHE.lock() else {
        return list_in(&dir());
    };
    if let Some((at, entries)) = cache.as_ref() {
        if !stale && at.elapsed() < Duration::from_secs(2) {
            return entries.clone();
        }
    }
    let entries = list_in(&dir());
    *cache = Some((Instant::now(), entries.clone()));
    entries
}

fn shelf_changed() {
    SHELF_STALE.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Save what `scene` holds under `name`, in `dir` — over the scene of
/// that name if there is one — with a picture of it beside
/// (`crate::scene_thumb`). Returns its file.
pub fn save_in(dir: &Path, name: &str, scene: &Scene) -> Result<PathBuf> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AnimaError::other("a scene needs a name"));
    }
    let path = write_in(
        dir,
        &SavedScene {
            name: name.to_string(),
            characters: scene.to_character_configs(),
            groups: scene.groups.clone(),
        },
    )?;
    crate::scene_thumb::save_for(&path, scene);
    Ok(path)
}

/// Save `saved` in `dir` under its own name — over the scene of that name
/// if there is one, keeping what that file held and this version does
/// not read: a newer version's characters, and its settings for the ones
/// this version does (`config::SetAside`). Returns its file.
pub fn write_in(dir: &Path, saved: &SavedScene) -> Result<PathBuf> {
    let existing = list_in(dir).into_iter().find(|e| e.name == saved.name);
    let (path, over) = match existing {
        Some(entry) => {
            let over = crate::util::read_to_string_capped(&entry.path, MAX_CONFIG_BYTES).ok();
            (entry.path, over)
        }
        None => (free_path(dir, &slug(&saved.name)), None),
    };
    let text = scene_text(saved, over.as_deref())?;
    std::fs::create_dir_all(dir)?;
    crate::util::atomic_write_bytes(&path, text.as_bytes())?;
    shelf_changed();
    Ok(path)
}

/// A scene file's text for `saved`, saved over the file whose text is
/// `over`, if any: with what that one held and this version does not
/// read written back (`config::SetAside`) — nothing kept from a file that
/// does not read as TOML at all.
pub(crate) fn scene_text(saved: &SavedScene, over: Option<&str>) -> Result<String> {
    let kept = over
        .and_then(|text| text.parse::<toml::Table>().ok())
        .map(|mut table| SetAside::of_scene(&mut table))
        .unwrap_or_default();
    let mut table = toml::Table::try_from(saved)?;
    kept.write_back(&mut table);
    Ok(toml::to_string_pretty(&table)?)
}

/// For `fuzz/fuzz_targets/scene_save_over.rs`: a scene read from `text`,
/// whatever it holds, then saved over that text, reads the same, leaves
/// the same out, keeps every key a scene does not read as it was, and
/// saved over again is not changed.
#[doc(hidden)]
pub fn fuzz_scene_save_over(text: &str) {
    let Ok(table) = text.parse::<toml::Table>() else {
        return;
    };
    let original = table.clone();
    let Ok((saved, left_out)) = from_table(table) else {
        return;
    };
    let once = scene_text(&saved, Some(text)).expect("a scene read is written");
    let written: toml::Table = once
        .parse()
        .unwrap_or_else(|e| panic!("what was written does not parse: {e}\n{once}"));
    // Each key a scene does not read, as it was — compared as text, so a
    // `nan` equals itself.
    for (key, value) in &original {
        if !["name", "characters", "groups"].contains(&key.as_str()) {
            assert_eq!(
                written.get(key).map(toml::Value::to_string),
                Some(value.to_string()),
                "`{key}` was not kept\n{once}"
            );
        }
    }
    let (again, left_again) = from_table(written)
        .unwrap_or_else(|e| panic!("what was written does not read: {e}\n{once}"));
    let as_text = |s: &SavedScene| toml::to_string(s).expect("a scene is written");
    assert_eq!(
        as_text(&again),
        as_text(&saved),
        "the scene changed\n{once}"
    );
    assert_eq!(left_again, left_out, "what was left out changed\n{once}");
    let twice = scene_text(&again, Some(&once)).expect("written again");
    assert_eq!(once, twice, "a second save over it changed it");
}

/// `name`, or "name (2)" and upward while `entries` has a scene so named.
pub fn free_name(entries: &[SceneEntry], name: &str) -> String {
    let taken = |n: &str| entries.iter().any(|e| e.name == n);
    if !taken(name) {
        return name.to_string();
    }
    (2..)
        .map(|n| format!("{name} ({n})"))
        .find(|n| !taken(n))
        .unwrap_or_else(|| name.to_string())
}

/// Read the scene in `path`, under the same limits as the config.
pub fn load(path: &Path) -> Result<SavedScene> {
    load_counting(path).map(|(saved, _)| saved)
}

/// [`load`], and how many characters and groups it left out: those that
/// do not read — a newer version's, or mistyped — as the config's are
/// (`config::SetAside`). The file keeps them, saved over too
/// ([`write_in`]); one of them used to make the whole scene fail to load.
pub fn load_counting(path: &Path) -> Result<(SavedScene, usize)> {
    let text = crate::util::read_to_string_capped(path, MAX_CONFIG_BYTES)?;
    let table: toml::Table = text
        .parse()
        .map_err(|e| AnimaError::other(format!("{e}")))?;
    let (mut saved, left_out) = from_table(table)?;
    if left_out > 0 {
        tracing::warn!("Scene {}: {left_out} left out, not read", saved.name);
    }
    saved.characters.truncate(MAX_ENTITIES);
    for c in &mut saved.characters {
        c.sanitize();
    }
    Ok((saved, left_out))
}

/// A scene from its TOML table, as far as this version reads it, and how
/// many characters and groups were left out.
pub(crate) fn from_table(mut table: toml::Table) -> Result<(SavedScene, usize)> {
    let left_out = SetAside::of_scene(&mut table).left_out();
    let saved = toml::Value::Table(table)
        .try_into::<SavedScene>()
        .map_err(|e| AnimaError::other(format!("{e}")))?;
    Ok((saved, left_out))
}

/// Delete the scene in `path`, and its picture. One already gone —
/// deleted in a file manager while the list still showed it — is as
/// deleted as it can be: the list lets it go, and nothing is said.
pub fn delete(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.into()),
        _ => {}
    }
    let _ = std::fs::remove_file(crate::scene_thumb::path_for(path));
    shelf_changed();
    Ok(())
}

/// The scene after the one named `current` in `entries`, by name, round
/// to the first — the tray's "Next scene".
pub fn next_after<'e>(entries: &'e [SceneEntry], current: Option<&str>) -> Option<&'e SceneEntry> {
    let at = current.and_then(|c| entries.iter().position(|e| e.name == c));
    match at {
        Some(i) => entries.get((i + 1) % entries.len()),
        None => entries.first(),
    }
}

/// A file name for `name`: letters and digits kept, lower-cased, the rest
/// one dash each.
pub(crate) fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let out: String = out.trim_matches('-').chars().take(60).collect();
    if out.is_empty() {
        "scene".into()
    } else {
        out
    }
}

/// `dir/stem.toml`, or `stem-2.toml` and upward if that is taken.
fn free_path(dir: &Path, stem: &str) -> PathBuf {
    let first = dir.join(format!("{stem}.toml"));
    if !first.exists() {
        return first;
    }
    (2..)
        .map(|n| dir.join(format!("{stem}-{n}.toml")))
        .find(|p| !p.exists())
        .unwrap_or(first)
}

impl Scene {
    /// Make the scene the saved one: its characters — kept where they
    /// show the same asset — and its groups.
    pub fn apply_saved(&mut self, saved: &SavedScene) {
        self.restore_configs(&saved.characters);
        self.groups = crate::scene::groups_in_one_each(saved.groups.clone());
        self.mark_visible_dirty();
        // Hearts go with a character by id, and another scene's may share
        // it.
        self.effects.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("anima-scenes-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn scene_of(ids: &[&str]) -> Scene {
        let mut scene = Scene::from_config(&AppConfig::default());
        let configs: Vec<_> = AppConfig::default()
            .characters
            .into_iter()
            .take(1)
            .flat_map(|template| {
                ids.iter()
                    .map(move |id| CharacterConfig {
                        id: (*id).into(),
                        name: (*id).into(),
                        asset_path: String::new(),
                        ..template.clone()
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        scene.restore_configs(&configs);
        scene
    }

    /// A scene with a character a newer version made: the rest loads, and
    /// it is counted as left out. It used to fail the whole scene.
    #[test]
    fn a_newer_versions_character_is_left_out_of_the_scene() {
        let text = r#"
            name = "Mixed"
            [[characters]]
            id = "cat"
            name = "Cat"
            asset_type = "png_static"
            asset_path = "c.png"
            x = 1.0
            y = 2.0
            [[characters]]
            id = "holo"
            name = "Holo"
            asset_type = "hologram"
            asset_path = "h.holo"
            x = 0.0
            y = 0.0
        "#;
        let (scene, left_out) = from_table(text.parse().unwrap()).unwrap();
        assert_eq!(left_out, 1);
        assert_eq!(scene.name, "Mixed");
        assert_eq!(scene.characters.len(), 1);
        assert_eq!(scene.characters[0].id, "cat");
        // Nothing to leave out, nothing left out; nothing readable, an error.
        assert_eq!(
            from_table("name = \"Empty\"".parse().unwrap()).unwrap().1,
            0
        );
        assert!(from_table("name = 5".parse().unwrap()).is_err());
    }

    /// Saved over, a scene keeps what a newer version wrote in it: the
    /// character this version cannot read, a key it does not know on one
    /// it can, and one of the scene's own. Saving dropped them all.
    #[test]
    fn saving_over_a_scene_keeps_what_a_newer_version_wrote() {
        let dir = tmp("keeps-newer");
        std::fs::create_dir_all(&dir).unwrap();
        let text = r#"
            name = "Mixed"
            backdrop = "dusk"
            [[characters]]
            id = "cat"
            name = "Cat"
            asset_type = "png_static"
            asset_path = "c.png"
            x = 1.0
            y = 2.0
            sparkle = true
            [[characters]]
            id = "holo"
            name = "Holo"
            asset_type = "hologram"
            asset_path = "h.holo"
            x = 0.0
            y = 0.0
        "#;
        let file = dir.join("mixed.toml");
        std::fs::write(&file, text).unwrap();
        let (mut saved, left_out) = load_counting(&file).unwrap();
        assert_eq!((saved.characters.len(), left_out), (1, 1));

        saved.characters[0].x = 40.0;
        assert_eq!(write_in(&dir, &saved).unwrap(), file);
        let table: toml::Table = std::fs::read_to_string(&file).unwrap().parse().unwrap();
        assert_eq!(table["backdrop"].as_str(), Some("dusk"));
        let characters = table["characters"].as_array().unwrap();
        assert_eq!(characters.len(), 2);
        assert_eq!(characters[0]["x"].as_float(), Some(40.0));
        assert_eq!(characters[0]["sparkle"].as_bool(), Some(true));
        assert_eq!(characters[1]["asset_type"].as_str(), Some("hologram"));
        // And it reads as it did, the change kept.
        let (again, left_out) = load_counting(&file).unwrap();
        assert_eq!((again.characters[0].x, left_out), (40.0, 1));

        // A scene saved under a new name has nothing to keep.
        saved.name = "Fresh".into();
        let fresh = write_in(&dir, &saved).unwrap();
        let table: toml::Table = std::fs::read_to_string(&fresh).unwrap().parse().unwrap();
        assert!(table.get("backdrop").is_none());
        assert_eq!(table["characters"].as_array().unwrap().len(), 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A list a scene does not read — a newer version's reminders in it —
    /// goes back whole when the scene is saved over: its readable entries
    /// were lost, the list made anew from the ones that did not read.
    #[test]
    fn a_list_a_scene_does_not_read_is_kept_whole() {
        let text = r#"
            name = "Day"
            [[characters]]
            id = "cat"
            name = "Cat"
            asset_type = "png_static"
            asset_path = "c.png"
            x = 1.0
            y = 2.0
            [[reminders]]
            text = "Water"
            every_minutes = 30
            [[reminders]]
            text = "Glow"
            every_minutes = "often"
        "#;
        let (saved, _) = from_table(text.parse().unwrap()).unwrap();
        let out: toml::Table = scene_text(&saved, Some(text)).unwrap().parse().unwrap();
        let reminders = out["reminders"].as_array().unwrap();
        assert_eq!(reminders.len(), 2, "{out}");
        assert_eq!(reminders[0]["text"].as_str(), Some("Water"));
        fuzz_scene_save_over(text);
    }

    /// Found by `scene_save_over`: two `[[windows]]` — a list a scene does
    /// not read — with one `id`. Their keys were noted by `id`, as a
    /// config's are, and the second's written into the first.
    #[test]
    fn a_list_a_scene_does_not_read_is_not_looked_into() {
        let text = r#"
            name = "Two"
            [[windows]]
            id = "w"
            a = 1
            [[windows]]
            id = "w"
            b = 2
        "#;
        let (saved, _) = from_table(text.parse().unwrap()).unwrap();
        let out: toml::Table = scene_text(&saved, Some(text)).unwrap().parse().unwrap();
        let original: toml::Table = text.parse().unwrap();
        assert_eq!(out["windows"], original["windows"]);
        fuzz_scene_save_over(text);
    }

    #[test]
    fn an_imported_name_steps_aside_for_one_on_the_shelf() {
        let entry = |name: &str| SceneEntry {
            name: name.into(),
            path: PathBuf::from(format!("{name}.toml")),
        };
        let shelf = [entry("Work"), entry("Work (2)")];
        assert_eq!(free_name(&shelf, "Stream"), "Stream");
        assert_eq!(free_name(&shelf, "Work"), "Work (3)");
    }

    #[test]
    fn names_become_file_names() {
        assert_eq!(slug("Work"), "work");
        assert_eq!(slug("  Late night / Stream!! "), "late-night-stream");
        assert_eq!(slug("Ședință"), "ședință");
        assert_eq!(slug("???"), "scene");
    }

    #[test]
    fn a_scene_saves_lists_loads_and_goes() {
        let dir = tmp("round-trip");
        let scene = scene_of(&["a", "b"]);
        let path = save_in(&dir, " Work ", &scene).unwrap();
        assert_eq!(path.file_name().unwrap(), "work.toml");
        let picture = crate::scene_thumb::path_for(&path);
        assert!(
            crate::scene_thumb::load(&picture).is_ok(),
            "its picture beside it"
        );
        save_in(&dir, "stream", &scene_of(&["c"])).unwrap();
        let names: Vec<_> = list_in(&dir).into_iter().map(|e| e.name).collect();
        assert_eq!(names, ["stream", "Work"], "by name, case aside");
        // Saving under a name that exists writes over it, not beside it.
        save_in(&dir, "Work", &scene_of(&["x"])).unwrap();
        assert_eq!(list_in(&dir).len(), 2);
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.characters.len(), 1);
        assert_eq!(loaded.characters[0].id, "x");
        delete(&path).unwrap();
        assert_eq!(list_in(&dir).len(), 1);
        assert!(!picture.exists(), "and it goes with it");
        delete(&path).expect("already gone is deleted");
        assert!(save_in(&dir, "  ", &scene).is_err(), "no name");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn two_names_with_one_file_name_both_keep_their_file() {
        let dir = tmp("collide");
        let a = save_in(&dir, "Work!", &scene_of(&["a"])).unwrap();
        let b = save_in(&dir, "Work?", &scene_of(&["b"])).unwrap();
        assert_ne!(a, b);
        assert_eq!(list_in(&dir).len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_hand_edited_scene_is_held_to_the_configs_limits() {
        let dir = tmp("limits");
        std::fs::create_dir_all(&dir).unwrap();
        let mut text = String::from("name = \"Big\"\n");
        for i in 0..(MAX_ENTITIES + 5) {
            text.push_str(&format!(
                "[[characters]]\nid = \"c{i}\"\nname = \"c{i}\"\nasset_type = \"png_static\"\nasset_path = \"\"\nx = 0.0\ny = 0.0\nscale = 99.0\n"
            ));
        }
        let path = dir.join("big.toml");
        std::fs::write(&path, text).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.characters.len(), MAX_ENTITIES);
        assert!(
            loaded.characters.iter().all(|c| c.scale <= 5.0),
            "sanitised"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn next_goes_round() {
        let entries: Vec<SceneEntry> = ["a", "b", "c"]
            .iter()
            .map(|n| SceneEntry {
                name: (*n).into(),
                path: PathBuf::from(format!("{n}.toml")),
            })
            .collect();
        assert_eq!(next_after(&entries, None).unwrap().name, "a");
        assert_eq!(next_after(&entries, Some("a")).unwrap().name, "b");
        assert_eq!(next_after(&entries, Some("c")).unwrap().name, "a");
        assert_eq!(next_after(&entries, Some("gone")).unwrap().name, "a");
        assert!(next_after(&[], Some("a")).is_none());
    }

    #[test]
    fn applying_a_saved_scene_brings_its_characters_and_groups() {
        let mut scene = scene_of(&["a", "b"]);
        let saved = SavedScene {
            name: "Other".into(),
            characters: scene_of(&["b", "c"]).to_character_configs(),
            groups: vec![GroupConfig {
                id: "g".into(),
                name: "G".into(),
                member_ids: vec!["c".into()],
                ..GroupConfig::default()
            }],
        };
        scene.apply_saved(&saved);
        let ids: Vec<_> = scene.entities.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["b", "c"]);
        assert_eq!(scene.groups, saved.groups);
    }
}
