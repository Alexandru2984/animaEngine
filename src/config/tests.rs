//! The config's tests, by topic.

#![allow(unused_imports)]

use super::character::*;
use super::global::*;
use super::migrate::*;
use super::set_aside::*;
use super::*;

#[cfg(test)]
mod config_path_tests {
    use super::AppConfig;

    #[test]
    fn a_unit_test_never_reaches_the_users_config() {
        let path = AppConfig::config_path();
        assert!(path.starts_with(std::env::temp_dir()), "{}", path.display());
        if let Some(dirs) = directories::ProjectDirs::from("", "", "animaEngine") {
            assert!(!path.starts_with(dirs.config_dir()), "{}", path.display());
        }
    }
}

#[cfg(test)]
mod resolve_path_tests {
    use super::*;

    /// A built-in character left by 1.2.0 in the folder it ran from no
    /// longer stands in front of the one this version made; one drawn now
    /// still does (a build run from the repository), and what is not a
    /// built-in character is found as before.
    #[test]
    fn an_old_drawing_of_a_built_in_character_gives_way() {
        let root = std::env::temp_dir().join(format!("anima_resolve_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (exe, cwd, demo) = (root.join("bin"), root.join("home"), root.join("data/demo"));
        for dir in [
            exe.clone(),
            cwd.join("assets/demo/ghost"),
            cwd.join("assets/demo/mine"),
            demo.join("ghost"),
        ] {
            std::fs::create_dir_all(&dir).unwrap();
        }
        std::fs::write(cwd.join("assets/demo/ghost/frame_001.png"), b"old").unwrap();
        std::fs::write(demo.join("ghost/frame_001.png"), b"new").unwrap();
        std::fs::write(demo.join("ghost/.art"), crate::demo::ART).unwrap();
        let resolve = |p: &str| resolve_asset_path_in(p, Some(&exe), &cwd, &demo);

        assert_eq!(resolve("assets/demo/ghost"), demo.join("ghost"));
        assert_eq!(
            resolve("assets/demo/ghost/frame_001.png"),
            demo.join("ghost/frame_001.png")
        );
        assert_eq!(resolve("assets/demo/mine"), cwd.join("assets/demo/mine"));

        std::fs::write(cwd.join("assets/demo/ghost/.art"), crate::demo::ART).unwrap();
        assert_eq!(resolve("assets/demo/ghost"), cwd.join("assets/demo/ghost"));

        // Nothing made yet: the old one is better than none.
        std::fs::remove_file(cwd.join("assets/demo/ghost/.art")).unwrap();
        std::fs::remove_dir_all(&demo).unwrap();
        assert_eq!(resolve("assets/demo/ghost"), cwd.join("assets/demo/ghost"));
        assert_eq!(
            resolve("assets/demo/star"),
            PathBuf::from("assets/demo/star")
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    // `~` expansion reads `$HOME`, which unix guarantees and Windows does
    // not set at all — so both assertions below are unix-shaped. Off unix
    // `resolve_asset_path` takes its existing "no HOME" branch and passes
    // the path through untouched; whether Windows should expand `~`
    // against `%USERPROFILE%` instead is a C4 decision, not a C2b one.
    #[test]
    #[cfg(unix)]
    fn tilde_slash_expands_to_home() {
        let home = std::env::var("HOME").expect("HOME set in test env");
        let resolved = AppConfig::resolve_asset_path("~/assets/x.png");
        assert!(
            resolved.starts_with(&home),
            "expected {resolved:?} under {home}"
        );
    }

    #[test]
    #[cfg(unix)]
    fn tilde_user_syntax_passes_through_unexpanded() {
        // `~alex/x` must NOT become `$HOME + "alex/x"` — that silently
        // builds a path inside the wrong home. We don't support the
        // user-tilde form at all; it passes through as a literal.
        let home = std::env::var("HOME").expect("HOME set in test env");
        let resolved = AppConfig::resolve_asset_path("~nobody/assets/x.png");
        assert!(
            !resolved.starts_with(&home),
            "user-tilde must not expand against our HOME, got {resolved:?}"
        );
    }

    #[test]
    fn interior_tilde_untouched() {
        let resolved = AppConfig::resolve_asset_path("/data/backup~old/x.png");
        assert_eq!(resolved, PathBuf::from("/data/backup~old/x.png"));
    }
}

#[cfg(test)]
mod toggle_button_tests {
    use super::*;

    /// The ⚙ button shows unless turned off: in a new config, and in one
    /// from before the setting, which has no key for it.
    #[test]
    fn the_gear_button_shows_unless_turned_off() {
        assert!(GlobalConfig::default().show_toggle_button);
        let before: GlobalConfig = toml::from_str("").expect("an empty [global] reads");
        assert!(before.show_toggle_button);
        let off: GlobalConfig = toml::from_str("show_toggle_button = false").unwrap();
        assert!(!off.show_toggle_button);
    }
}

#[cfg(test)]
mod animations_schema_tests {
    use super::*;
    use crate::animation::StateId;

    #[test]
    fn legacy_character_decodes_with_empty_animations() {
        let toml_str = r#"
            id = "slime"
            name = "Slime"
            asset_type = "png_sequence"
            asset_path = "assets/demo/slime"
            x = 1.0
            y = 2.0
        "#;
        let cfg: CharacterConfig = toml::from_str(toml_str).unwrap();
        assert!(cfg.animations.is_empty());
        // Round-trip must not introduce the key.
        let out = toml::to_string(&cfg).unwrap();
        assert!(!out.contains("animations"), "got: {out}");
    }

    #[test]
    fn state_tables_round_trip() {
        let toml_str = r#"
            id = "shime"
            name = "Shime"
            asset_type = "png_sequence"
            asset_path = "imported/shime/idle"
            x = 0.0
            y = 0.0

            [animations.walk]
            asset_type = "png_sequence"
            asset_path = "imported/shime/walk"
            fps = 10.0

            [animations.fall]
            asset_type = "gif"
            asset_path = "imported/shime/fall.gif"
        "#;
        let cfg: CharacterConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.animations.len(), 2);
        assert_eq!(cfg.animations[&StateId::Walk].fps, Some(10.0));
        assert!(cfg.animations[&StateId::Fall].fps.is_none());

        let out = toml::to_string(&cfg).unwrap();
        let back: CharacterConfig = toml::from_str(&out).unwrap();
        assert_eq!(back.animations.len(), 2);
        assert_eq!(
            back.animations[&StateId::Walk].asset_path,
            "imported/shime/walk"
        );
    }

    #[test]
    fn unknown_state_key_is_rejected() {
        // Closed StateId enum: a typo'd state name must fail loudly at
        // parse, not silently vanish.
        let toml_str = r#"
            id = "x"
            name = "x"
            asset_type = "gif"
            asset_path = "a.gif"
            x = 0.0
            y = 0.0

            [animations.wlak]
            asset_type = "gif"
            asset_path = "w.gif"
        "#;
        assert!(toml::from_str::<CharacterConfig>(toml_str).is_err());
    }
}

#[cfg(test)]
mod windows_tests {
    use super::*;
    use crate::behavior::Behavior;

    fn empty_char(id: &str) -> CharacterConfig {
        CharacterConfig {
            id: id.into(),
            name: id.into(),
            asset_type: AssetType::PngStatic,
            asset_path: "/dev/null".into(),
            x: 0.0,
            y: 0.0,
            scale: 1.0,
            opacity: 1.0,
            fps: 1.0,
            visible: true,
            playing: true,
            z_index: 0,
            physics_enabled: false,
            behavior: Behavior::Idle,
            spritesheet_columns: None,
            spritesheet_rows: None,
            monitor: None,
            easing: None,
            animations: std::collections::BTreeMap::new(),
            talk: Default::default(),
            hours: None,
        }
    }

    #[test]
    fn window_config_round_trips_through_toml() {
        let w = WindowConfig {
            id: "side".into(),
            name: "Companion".into(),
            monitor_mode: Some(MonitorMode::Single {
                name: "HDMI-A-1".into(),
            }),
            characters: vec![empty_char("cat")],
        };
        let toml_str = toml::to_string(&w).expect("serialize");
        let back: WindowConfig = toml::from_str(&toml_str).expect("deserialize");
        assert_eq!(back.id, "side");
        assert_eq!(back.characters.len(), 1);
        matches!(
            back.monitor_mode,
            Some(MonitorMode::Single { ref name }) if name == "HDMI-A-1"
        );
    }

    /// A pre-0.3 TOML must decode cleanly into the new struct shape:
    /// no `windows` table, the field defaults to `vec![]`, the characters
    /// where they were.
    #[test]
    fn pre_0_3_toml_without_windows_field_decodes() {
        let toml_str = r#"
            [global]
            always_on_top = true
            transparent = true
            playback_enabled = true
            window_width = 0
            window_height = 0

            [[characters]]
            id = "ghost"
            name = "Ghost"
            asset_type = "png_static"
            asset_path = "/tmp/g.png"
            x = 0.0
            y = 0.0
        "#;
        let cfg: AppConfig = toml::from_str(toml_str).expect("decode 0.2 config");
        assert!(cfg.windows.is_empty());
        assert_eq!(cfg.characters.len(), 1);
    }
}

// ── Schema versioning + migration (W.0, 0.9) ─────────────────────────
#[cfg(test)]
mod migration_tests {
    use super::*;

    fn table(s: &str) -> toml::Table {
        s.parse().expect("parse toml")
    }

    #[test]
    fn detects_absent_version_as_v1() {
        assert_eq!(detect_schema_version(&table("a = 1")), 1);
    }

    #[test]
    fn detects_explicit_version() {
        assert_eq!(detect_schema_version(&table("version = 2")), 2);
    }

    #[test]
    fn malformed_version_is_treated_as_current_not_destroyed() {
        // A string, a float, zero and a negative all map to CURRENT so
        // we never run a migration off a garbage version field.
        for bad in [
            "version = \"two\"",
            "version = 1.5",
            "version = 0",
            "version = -3",
        ] {
            assert_eq!(
                detect_schema_version(&table(bad)),
                CURRENT_SCHEMA_VERSION,
                "input: {bad}"
            );
        }
    }

    #[test]
    fn migrate_stamps_current_version() {
        let mut t = table("a = 1");
        let from = migrate_table(&mut t);
        assert_eq!(from, 1, "started from implicit v1");
        assert_eq!(
            t.get("version"),
            Some(&toml::Value::Integer(CURRENT_SCHEMA_VERSION as i64))
        );
    }

    #[test]
    fn migrate_is_idempotent() {
        let mut t = table("a = 1");
        migrate_table(&mut t);
        let snapshot = t.clone();
        // Second run starts from CURRENT, does no work, re-stamps same.
        let from = migrate_table(&mut t);
        assert_eq!(from, CURRENT_SCHEMA_VERSION);
        assert_eq!(t, snapshot, "migrating twice equals migrating once");
    }

    #[test]
    fn out_of_range_version_does_not_panic_and_loads_as_current() {
        // Regression: `version = 4294967296` (u32::MAX + 1) used to wrap
        // to 0 via `as u32`, then `migrate_table` hit `unreachable!` and
        // *panicked* the config loader — contradicting the "a malformed
        // version must never be a reason to lose the config" contract.
        for v in ["4294967296", "9999999999999", "-1", "0"] {
            let src = format!("version = {v}\n[global]\n");
            assert_eq!(
                detect_schema_version(&table(&src)),
                CURRENT_SCHEMA_VERSION,
                "version = {v} must clamp to current, not wrap"
            );
            // The full path must not panic either.
            let mut t = table(&src);
            let from = migrate_table(&mut t);
            assert_eq!(from, CURRENT_SCHEMA_VERSION);
        }
    }

    #[test]
    fn future_version_is_loaded_read_only_not_downgraded() {
        // A config written by a newer build (v9) must not be silently
        // re-stamped to the current version — that would strip fields
        // this build doesn't understand on the next save. `migrate_table`
        // leaves the version marker and unknown keys intact.
        let mut t = table("version = 9\nfuture_only_key = 42\n");
        let from = migrate_table(&mut t);
        assert_eq!(from, 9, "reports the on-disk future version");
        assert_eq!(
            t.get("version"),
            Some(&toml::Value::Integer(9)),
            "future version marker is preserved, not downgraded"
        );
        assert_eq!(
            t.get("future_only_key"),
            Some(&toml::Value::Integer(42)),
            "unknown future keys survive the no-op migration"
        );
    }

    /// A hand-written config names only what it wants to change. Leaving
    /// out a `[global]` key — or the whole table, or `characters` — used to
    /// fail to decode, and at startup that means backed up and replaced by
    /// the demo scene.
    #[test]
    fn a_config_that_names_only_what_it_changes_loads() {
        let (cfg, _) =
            AppConfig::decode_config_str("version = 2\n[global]\nhover_startle = true\n")
                .expect("a partial [global] is a valid config");
        assert!(cfg.global.hover_startle);
        assert!(cfg.global.always_on_top && cfg.global.transparent && cfg.global.playback_enabled);
        assert!(
            cfg.characters.is_empty(),
            "no characters named, none invented"
        );

        assert!(
            AppConfig::decode_config_str("version = 2\n").is_ok(),
            "no [global] table"
        );
        assert!(AppConfig::decode_config_str("").is_ok(), "an empty file");
    }

    /// A character still needs what makes it a character — without it,
    /// it is set aside, not shown, and kept in the file; the rest reads.
    #[test]
    fn a_character_without_its_asset_is_set_aside() {
        let toml = "version = 2\n[[characters]]\nid = \"a\"\nname = \"A\"\n\
                    asset_type = \"png_static\"\nx = 0.0\ny = 0.0\n\
                    [[characters]]\nid = \"b\"\nname = \"B\"\n\
                    asset_type = \"png_static\"\nasset_path = \"b.png\"\nx = 0.0\ny = 0.0\n";
        let (cfg, _) = AppConfig::decode_config_str(toml).expect("the rest reads");
        assert_eq!(cfg.characters.len(), 1);
        assert_eq!(cfg.characters[0].id, "b");
        assert!(!cfg.set_aside.is_empty());
        let out = cfg.to_toml_string().unwrap();
        assert!(out.contains("id = \"a\""), "kept in the file: {out}");
    }

    /// What a newer version wrote — a behavior, an asset type, a choice
    /// in `[global]`, a character in a window — reads as far as this one
    /// can, and is written back as it was. Each used to cost the whole
    /// file, and the scene with it.
    #[test]
    fn what_a_newer_version_wrote_is_set_aside_and_written_back() {
        let newer = r#"
            version = 2
            [global]
            theme = "sepia"
            hover_startle = true
            on_fullscreen = "dim"

            [[characters]]
            id = "dancer"
            name = "Dancer"
            asset_type = "png_static"
            asset_path = "d.png"
            x = 1.0
            y = 2.0
            [characters.behavior]
            type = "breakdance"
            moves = 3

            [[characters]]
            id = "holo"
            name = "Holo"
            asset_type = "hologram"
            asset_path = "h.holo"
            x = 1.0
            y = 2.0

            [[characters]]
            id = "plain"
            name = "Plain"
            asset_type = "png_static"
            asset_path = "p.png"
            x = 5.0
            y = 6.0

            [[windows]]
            id = "side"
            name = "Side"
            [[windows.characters]]
            id = "w"
            name = "W"
            asset_type = "hologram"
            asset_path = "w.holo"
            x = 0.0
            y = 0.0

            [[groups]]
            id = "g"
            name = "G"
            member_ids = ["plain"]
            visible = "sometimes"
        "#;
        let (cfg, _) = AppConfig::decode_config_str(newer).expect("reads as far as it can");
        let ids: Vec<&str> = cfg.characters.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["plain"]);
        assert!(cfg.windows[0].characters.is_empty());
        assert!(cfg.groups.is_empty());
        assert!(cfg.global.hover_startle, "the settings that read, read");
        assert_eq!(cfg.global.theme, crate::ui::theme::Theme::default());

        // Saved as it is, everything comes back as written.
        let out = cfg.to_toml_string().unwrap();
        let back: toml::Table = out.parse().unwrap();
        let written: toml::Table = newer.parse().unwrap();
        assert_eq!(back["global"]["theme"].as_str(), Some("sepia"));
        assert_eq!(back["global"]["on_fullscreen"].as_str(), Some("dim"));
        let characters = back["characters"].as_array().unwrap();
        assert_eq!(characters.len(), 3);
        for original in written["characters"].as_array().unwrap() {
            let id = original["id"].as_str().unwrap();
            let kept = characters
                .iter()
                .find(|c| c["id"].as_str() == Some(id))
                .unwrap_or_else(|| panic!("{id} kept"));
            if id != "plain" {
                assert_eq!(kept, original, "{id} as written");
            }
        }
        assert_eq!(
            back["windows"][0]["characters"][0],
            written["windows"][0]["characters"][0]
        );
        assert_eq!(back["groups"][0], written["groups"][0]);
        // And it reads the same again.
        let (again, _) = AppConfig::decode_config_str(&out).unwrap();
        assert_eq!(again.set_aside, cfg.set_aside);

        // A setting changed here since is the user's word, not the file's.
        let mut changed = cfg.clone();
        changed.global.theme = crate::ui::theme::Theme::Light;
        let out: toml::Table = changed.to_toml_string().unwrap().parse().unwrap();
        assert_eq!(out["global"]["theme"].as_str(), Some("light"));
        assert_eq!(out["global"]["on_fullscreen"].as_str(), Some("dim"));
    }

    /// An alias is a key the struct reads too: were it not listed, a
    /// config naming a field by it would have the key kept as unknown and
    /// written back beside the field — two of one, which does not read.
    #[test]
    fn an_alias_counts_as_a_key_read() {
        #[derive(serde::Deserialize)]
        #[allow(dead_code)]
        struct WithAlias {
            #[serde(alias = "colour")]
            color: String,
        }
        let fields = fields_of::<WithAlias>().unwrap();
        assert!(
            fields.contains(&"color") && fields.contains(&"colour"),
            "{fields:?}"
        );
    }

    /// A struct's keys, read off its `Deserialize`.
    #[test]
    fn a_struct_says_which_keys_it_reads() {
        let global = fields_of::<GlobalConfig>().expect("a plain struct");
        assert!(global.contains(&"always_on_top") && global.contains(&"sounds_muted"));
        let character = fields_of::<CharacterConfig>().unwrap();
        assert!(character.contains(&"id") && character.contains(&"talk"));
        assert!(!character.contains(&"future_wings"));
    }

    /// Keys a newer version added inside tables this one knows — a
    /// setting, a character's option, its talk's — are written back where
    /// they were. They used to be dropped at the first save, so a config
    /// went back to the newer version poorer.
    #[test]
    fn keys_a_newer_version_added_are_kept_where_they_were() {
        let newer = r#"
            version = 2
            [global]
            hover_startle = true
            future_glow = 3
            [global.focus]
            future_chime = "bell"

            [[characters]]
            id = "cat"
            name = "Cat"
            asset_type = "png_static"
            asset_path = "c.png"
            x = 1.0
            y = 2.0
            future_wings = true
            [characters.talk]
            phrases = ["Hi"]
            future_accent = "posh"

            [[characters]]
            id = "dog"
            name = "Dog"
            asset_type = "png_static"
            asset_path = "d.png"
            x = 3.0
            y = 4.0
            future_tail = 2

            [[groups]]
            id = "g"
            name = "G"
            member_ids = ["cat"]
            future_colour = "teal"
        "#;
        let (mut cfg, _) = AppConfig::decode_config_str(newer).unwrap();
        assert!(!cfg.set_aside.unread(), "nothing unread: no banner");
        assert!(!cfg.set_aside.is_empty(), "but kept to write back");
        let back: toml::Table = cfg.to_toml_string().unwrap().parse().unwrap();
        assert_eq!(back["global"]["future_glow"].as_integer(), Some(3));
        assert_eq!(
            back["global"]["focus"]["future_chime"].as_str(),
            Some("bell")
        );
        let cat = &back["characters"][0];
        assert_eq!(cat["id"].as_str(), Some("cat"));
        assert_eq!(cat["future_wings"].as_bool(), Some(true));
        assert_eq!(cat["talk"]["future_accent"].as_str(), Some("posh"));
        assert_eq!(back["characters"][1]["future_tail"].as_integer(), Some(2));
        assert_eq!(back["groups"][0]["future_colour"].as_str(), Some("teal"));
        // Read again, the same keys are kept again.
        let (again, _) = AppConfig::decode_config_str(&cfg.to_toml_string().unwrap()).unwrap();
        assert_eq!(again.set_aside, cfg.set_aside);

        // A character deleted here takes its keys with it.
        cfg.characters.retain(|c| c.id != "dog");
        let back: toml::Table = cfg.to_toml_string().unwrap().parse().unwrap();
        assert_eq!(back["characters"].as_array().unwrap().len(), 1);
        assert!(back["characters"][0].get("future_tail").is_none());
    }

    /// What does not read anywhere a list or a setting can be set aside is
    /// still an unreadable file.
    #[test]
    fn a_file_that_cannot_be_read_around_is_still_an_error() {
        assert!(AppConfig::decode_config_str("version = 2\ncollapse_state = 5\n").is_err());
        assert!(AppConfig::decode_config_str("version = 2\nglobal = 5\n").is_err());
        let (cfg, _) = AppConfig::decode_config_str("version = 2\n").unwrap();
        assert!(
            cfg.set_aside.is_empty(),
            "nothing set aside from a good file"
        );
    }

    #[test]
    fn decode_config_str_errors_instead_of_falling_back_to_defaults() {
        // The read-only reload path (`try_reload` → `decode_config_str`)
        // must surface a parse/decode failure as `Err`, never silently
        // substitute defaults the way startup `load` does — that's what
        // lets hot-reload keep the running scene on a mid-save read.
        assert!(
            AppConfig::decode_config_str("not [[[ valid toml").is_err(),
            "garbage must be an error, not a default config"
        );
        // A valid config still decodes cleanly through the same pure
        // path (no disk writes involved).
        let valid = toml::to_string(&AppConfig::default()).expect("serialise default");
        let (cfg, from) = AppConfig::decode_config_str(&valid).expect("valid config decodes");
        assert_eq!(from, CURRENT_SCHEMA_VERSION);
        assert_eq!(cfg.schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn unreadable_config_is_backed_up_verbatim_before_defaults() {
        // A corrupt config must be copied aside byte-for-byte so the
        // default-save that follows a failed load can't destroy the
        // user's only copy (the load-path clobber found post-rc1).
        let dir = std::env::temp_dir().join(format!("anima-corrupt-bak-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = dir.join("config.toml");
        let garbage = "version = 2 [[[ not toml !!!";
        std::fs::write(&cfg, garbage).unwrap();

        backup_unreadable_config(&cfg);

        let bak = dir.join("config.toml.bak-corrupt");
        assert_eq!(
            std::fs::read_to_string(&bak).expect("backup must exist"),
            garbage,
            "backup preserves the unreadable original verbatim"
        );
        // Original untouched by the backup itself.
        assert_eq!(std::fs::read_to_string(&cfg).unwrap(), garbage);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The copy aside is the user's alone, and goes where it is named:
    /// `fs::copy` made it with the source's mode (0644 from the umask,
    /// readable by every account) and wrote through a link left at its
    /// name (SEC-010).
    #[cfg(unix)]
    #[test]
    fn the_copy_aside_is_private_and_follows_no_link() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("anima-bak-private-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = dir.join("config.toml");
        std::fs::write(&cfg, "not [ toml").unwrap();
        std::fs::set_permissions(&cfg, std::fs::Permissions::from_mode(0o644)).unwrap();
        let elsewhere = dir.join("elsewhere");
        std::fs::write(&elsewhere, "someone else's").unwrap();
        let bak = dir.join("config.toml.bak-corrupt");
        std::os::unix::fs::symlink(&elsewhere, &bak).unwrap();

        backup_unreadable_config(&cfg);

        assert_eq!(
            std::fs::read_to_string(&elsewhere).unwrap(),
            "someone else's"
        );
        let meta = std::fs::symlink_metadata(&bak).unwrap();
        assert!(
            meta.file_type().is_file(),
            "a file of its own, not the link"
        );
        assert_eq!(meta.permissions().mode() & 0o777, 0o600);
        assert_eq!(std::fs::read_to_string(&bak).unwrap(), "not [ toml");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn v1_config_migrates_and_decodes() {
        // A minimal pre-0.9 config: no version key.
        let mut t = table(
            r#"
            characters = []

            [global]
            always_on_top = true
            transparent = true
            playback_enabled = true
            window_width = 0
            window_height = 0
            "#,
        );
        let from = migrate_table(&mut t);
        assert_eq!(from, 1);
        let cfg: AppConfig = toml::Value::Table(t).try_into().expect("decode migrated");
        assert_eq!(cfg.schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn default_config_is_current_version() {
        assert_eq!(AppConfig::default().schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn save_serialises_version_first() {
        // `version` must lead so it's a scalar before any [section];
        // otherwise TOML serialisation fails with "value after table".
        let s = toml::to_string_pretty(&AppConfig::default()).expect("serialise");
        let version_pos = s.find("version =").expect("version key present");
        let first_section = s.find("\n[").unwrap_or(s.len());
        assert!(
            version_pos < first_section,
            "version key must precede the first section:\n{s}"
        );
    }

    #[test]
    fn unknown_section_survives_round_trip() {
        // Forward-compat: a section a newer animaEngine added, that this
        // build doesn't model, must come back out unchanged.
        let src = format!(
            r#"
            version = {CURRENT_SCHEMA_VERSION}
            characters = []

            [global]
            always_on_top = true
            transparent = true
            playback_enabled = true
            window_width = 0
            window_height = 0

            [future_feature]
            shiny = true
            count = 7
            "#
        );
        let cfg: AppConfig = toml::from_str(&src).expect("decode with unknown section");
        assert!(
            cfg.extra.contains_key("future_feature"),
            "unknown section captured in extra: {:?}",
            cfg.extra
        );
        let out = toml::to_string_pretty(&cfg).expect("re-serialise");
        let back: AppConfig = toml::from_str(&out).expect("re-decode");
        let ff = back
            .extra
            .get("future_feature")
            .and_then(|v| v.as_table())
            .expect("future_feature preserved");
        assert_eq!(ff.get("shiny"), Some(&toml::Value::Boolean(true)));
        assert_eq!(ff.get("count"), Some(&toml::Value::Integer(7)));
    }
}

#[cfg(test)]
mod sanitize_tests {
    use super::*;

    #[test]
    fn finite_clamp_replaces_non_finite_and_clamps_range() {
        assert_eq!(finite_clamp(f32::NAN, 0.1, 5.0, 1.0), 1.0);
        assert_eq!(finite_clamp(f32::INFINITY, 0.1, 5.0, 1.0), 1.0);
        assert_eq!(finite_clamp(f32::NEG_INFINITY, 0.1, 5.0, 1.0), 1.0);
        assert_eq!(finite_clamp(100.0, 0.1, 5.0, 1.0), 5.0); // over-range clamps
        assert_eq!(finite_clamp(0.0, 0.1, 5.0, 1.0), 0.1); // under-range clamps
        assert_eq!(finite_clamp(2.5, 0.1, 5.0, 1.0), 2.5); // in-range kept
    }

    #[test]
    fn finite_place_replaces_non_finite_but_keeps_offscreen() {
        assert_eq!(finite_place(f32::NAN, 0.0), 0.0);
        assert_eq!(finite_place(f32::INFINITY, 0.0), 0.0);
        assert_eq!(finite_place(-500.0, 0.0), -500.0); // off-screen is legitimate
        assert_eq!(finite_place(-2.0e38, 0.0), -MAX_COORD); // but not without end
        assert_eq!(finite_place(3.0e9, 0.0), MAX_COORD);
    }

    #[test]
    fn character_sanitize_fixes_hand_edited_nan_inf() {
        // TOML floats accept `nan` / `inf` literally — exactly the
        // hand-edited-config vector the sanitizer defends against.
        let toml_str = r#"
            id = "x"
            name = "x"
            asset_type = "gif"
            asset_path = "a.gif"
            x = nan
            y = inf
            scale = nan
            opacity = inf
            fps = nan
        "#;
        let mut c: CharacterConfig = toml::from_str(toml_str).unwrap();
        assert!(!c.scale.is_finite(), "precondition: parsed NaN scale");
        c.sanitize();
        assert!(c.x.is_finite() && c.y.is_finite());
        assert_eq!(c.scale, default_scale());
        assert_eq!(c.opacity, default_opacity());
        assert_eq!(c.fps, default_fps());
    }
}

/// Configs written by released versions (`tests/fixtures/configs`) read
/// in this one with every value they hold, and save back with it.
#[cfg(test)]
mod upgrade_tests {
    use super::*;
    use std::collections::BTreeMap;

    const WRITTEN_BY: [(&str, &str); 3] = [
        (
            "1.0.0, first run",
            include_str!("../../tests/fixtures/configs/v1.0.0-default.toml"),
        ),
        (
            "1.1.0 to 1.4.0, first run",
            include_str!("../../tests/fixtures/configs/v1.4.0-default.toml"),
        ),
        (
            "1.4.0, every setting changed",
            include_str!("../../tests/fixtures/configs/v1.4.0-full.toml"),
        ),
    ];

    /// Every value in a table, by its path: `global.theme`,
    /// `characters[2].behavior.speed`.
    pub(super) fn values(path: &str, value: &toml::Value, out: &mut BTreeMap<String, toml::Value>) {
        match value {
            toml::Value::Table(table) => {
                for (key, value) in table {
                    let path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    values(&path, value, out);
                }
            }
            toml::Value::Array(items) if items.iter().any(toml::Value::is_table) => {
                for (i, item) in items.iter().enumerate() {
                    values(&format!("{path}[{i}]"), item, out);
                }
            }
            other => {
                out.insert(path.to_string(), other.clone());
            }
        }
    }

    /// The changes made on purpose, each with its reason: `path` going
    /// from `was` to `now`. Nothing else may change.
    fn meant(path: &str, was: &toml::Value, now: Option<&toml::Value>) -> bool {
        let strings = |v: &toml::Value| -> Option<Vec<String>> {
            v.as_array()?
                .iter()
                .map(|s| s.as_str().map(str::to_string))
                .collect()
        };
        match path {
            // Cycle character left Tab, the panel's, for N (1.5) — only
            // as it came; a Tab set by hand among others stays.
            "keybindings.map.cycle_entity" => {
                strings(was) == Some(vec!["Tab".into()])
                    && now.and_then(strings) == Some(vec!["N".into()])
            }
            _ => false,
        }
    }

    #[test]
    fn configs_from_earlier_versions_keep_every_value() {
        for (by, text) in WRITTEN_BY {
            let (config, _) = AppConfig::decode_config_str(text)
                .unwrap_or_else(|e| panic!("{by}: does not read: {e}"));
            assert!(config.set_aside.is_empty(), "{by}: {:?}", config.set_aside);
            assert!(
                config.extra.is_empty(),
                "{by}: unknown here: {:?}",
                config.extra
            );
            let saved = config.to_toml_string().unwrap();

            let mut before = BTreeMap::new();
            values("", &toml::Value::Table(text.parse().unwrap()), &mut before);
            let mut after = BTreeMap::new();
            values("", &toml::Value::Table(saved.parse().unwrap()), &mut after);
            let changed: Vec<String> = before
                .iter()
                .filter(|(path, value)| after.get(*path) != Some(value))
                .filter(|(path, value)| !meant(path, value, after.get(*path)))
                .map(|(path, value)| format!("{path}: {value} → {:?}", after.get(path)))
                .collect();
            assert!(changed.is_empty(), "{by}: {changed:#?}");
        }
    }

    /// The shortcuts someone set in 1.4 still do what they did: none is
    /// taken by an action 1.5 added with that chord as its default.
    #[test]
    fn shortcuts_from_earlier_versions_still_do_what_they_did() {
        for (by, text) in WRITTEN_BY {
            let (config, _) = AppConfig::decode_config_str(text).unwrap();
            let bindings = &config.keybindings;
            for (action, chords) in &bindings.map {
                for chord in chords {
                    assert_eq!(
                        bindings.lookup(*chord),
                        Some(*action),
                        "{by}: {chord} is {action:?}'s"
                    );
                }
            }
        }
    }
}

/// The config TOML the docs show reads here as written, every key known:
/// one that does not read is set aside, and whoever copied it gets a
/// banner instead of the setting (`monitor_mode` was a bare string in
/// docs/config.md until 1.5, and never read). Not the GNOME checklist's,
/// which is meant not to read.
#[cfg(test)]
mod doc_example_tests {
    use super::upgrade_tests::values;
    use super::*;
    use std::collections::BTreeMap;

    /// The character a `[characters.…]` table on its own belongs to.
    const CHARACTER: &str = "[[characters]]\nid = \"slime\"\nname = \"Slime\"\n\
        asset_type = \"png_sequence\"\nasset_path = \"assets/demo/slime\"\nx = 0.0\ny = 0.0\n";

    #[test]
    fn the_documented_config_reads_as_written() {
        for (doc, text) in [
            ("README.md", include_str!("../../README.md")),
            ("CONTRIBUTING.md", include_str!("../../CONTRIBUTING.md")),
            ("docs/config.md", include_str!("../../docs/config.md")),
            (
                "docs/engine-features.md",
                include_str!("../../docs/engine-features.md"),
            ),
        ] {
            for block in text.split("```toml\n").skip(1) {
                let block = block.split("```").next().unwrap_or_default();
                // A fragment, the rest of it left out (`…`, `# … other fields`).
                if block
                    .lines()
                    .any(|l| l.trim() == "…" || l.trim_start().starts_with("# …"))
                {
                    continue;
                }
                // The behaviors, listed one after another in one block.
                const BEHAVIOR: &str = "\n[characters.behavior]\n";
                let parts: Vec<String> = if block.matches(BEHAVIOR).count() > 1 {
                    block
                        .split(BEHAVIOR)
                        .skip(1)
                        .map(|part| format!("{BEHAVIOR}{part}"))
                        .collect()
                } else {
                    vec![block.to_string()]
                };
                for part in parts {
                    let table = part.lines().find(|l| l.starts_with('['));
                    if table.is_some_and(|l| l.starts_with("[characters.")) {
                        check(doc, &format!("{CHARACTER}{part}"));
                    } else {
                        check(doc, &part);
                    }
                }
            }
        }
    }

    fn check(doc: &str, toml: &str) {
        let (config, _) = AppConfig::decode_config_str(toml)
            .unwrap_or_else(|e| panic!("{doc}: does not read: {e}\n{toml}"));
        assert!(
            config.set_aside.is_empty() && config.extra.is_empty(),
            "{doc}: {:?} {:?}\n{toml}",
            config.set_aside,
            config.extra
        );
        // And every value is kept: a key a behavior does not have is
        // passed over without a word, so a misspelt one reads as its
        // default.
        let mut written = BTreeMap::new();
        values("", &toml::Value::Table(toml.parse().unwrap()), &mut written);
        let mut saved = BTreeMap::new();
        let out = config.to_toml_string().unwrap();
        values("", &toml::Value::Table(out.parse().unwrap()), &mut saved);
        let lost: Vec<String> = written
            .iter()
            .filter(|(path, value)| saved.get(*path) != Some(value))
            .filter(|(path, value)| !saved_otherwise(path, value, saved.get(*path)))
            .map(|(path, value)| format!("{path} = {value} → {:?}", saved.get(path)))
            .collect();
        assert!(lost.is_empty(), "{doc}: {lost:#?}\n{toml}");
    }

    /// A value kept, but saved another way: the focus timer's lengths at
    /// their defaults are not written, a chord is in its canonical
    /// spelling (`Up` → `ArrowUp`).
    fn saved_otherwise(path: &str, was: &toml::Value, now: Option<&toml::Value>) -> bool {
        let chords = |v: &toml::Value| -> Option<Vec<crate::keybindings::KeyChord>> {
            v.as_array()?
                .iter()
                .map(|c| c.as_str()?.parse().ok())
                .collect()
        };
        match path {
            "global.focus.minutes" | "global.focus.break_minutes" => now.is_none(),
            _ if path.starts_with("keybindings.map.") => {
                chords(was).is_some() && now.and_then(chords) == chords(was)
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod numbers_from_a_file_tests {
    use super::*;

    /// Groups were never sanitised, a state's fps neither, and a
    /// character's place only had to be finite: `nan` and `inf` read from
    /// config.toml went on into the quads, the hit test and the undo
    /// steps (`NaN != NaN`), and ±2e38 into sums that overflow (BUG-007).
    #[test]
    fn groups_states_and_places_read_from_a_file_are_held_in_range() {
        let mut config = AppConfig::default();
        let first = config.characters[0].id.clone();
        config.characters[0].x = 3.0e9;
        config.characters[0].y = -2.0e38;
        let mut state = config.characters[0]
            .animations
            .values()
            .next()
            .cloned()
            .unwrap_or(StateSequenceConfig {
                asset_type: config.characters[0].asset_type.clone(),
                asset_path: config.characters[0].asset_path.clone(),
                fps: None,
                spritesheet_columns: None,
                spritesheet_rows: None,
                unknown: toml::Table::new(),
            });
        state.fps = Some(f32::NAN);
        config.characters[0]
            .animations
            .insert(crate::animation::StateId::Walk, state.clone());
        state.fps = Some(f32::INFINITY);
        config.characters[0]
            .animations
            .insert(crate::animation::StateId::Fall, state);
        config.groups.push(crate::group::GroupConfig {
            id: "g".into(),
            name: "g".into(),
            member_ids: vec![first],
            offset_x: f32::NAN,
            offset_y: f32::INFINITY,
            scale: f32::NAN,
            visible: true,
        });
        let text = config.to_toml_string().unwrap();
        let (back, _) = AppConfig::decode_config_str(&text).unwrap();
        let g = &back.groups[0];
        assert!(g.offset_x.is_finite() && g.offset_y.is_finite(), "{g:?}");
        assert!((0.1..=5.0).contains(&g.scale), "{g:?}");
        let c = &back.characters[0];
        assert!(
            c.x.abs() <= MAX_COORD && c.y.abs() <= MAX_COORD,
            "{}, {}",
            c.x,
            c.y
        );
        for (id, s) in &c.animations {
            if let Some(fps) = s.fps {
                assert!((0.1..=240.0).contains(&fps), "{id:?}: {fps}");
            }
        }
    }
}

#[cfg(test)]
mod window_set_aside_tests {
    use super::*;

    /// A `[[windows]]` entry this version cannot read — a newer monitor
    /// mode — failed the whole config, every character with it; it is
    /// set aside, the rest read, and written back as it was (BUG-011).
    #[test]
    fn a_window_this_version_cannot_read_is_set_aside() {
        let mut text = AppConfig::default().to_toml_string().unwrap();
        text.push_str(
            "\n[[windows]]\nid = \"later\"\nname = \"Later\"\n\
             monitor_mode = { kind = \"hologram_wall\" }\n\
             \n[[windows]]\nid = \"now\"\nname = \"Now\"\n",
        );
        let (config, _) = AppConfig::decode_config_str(&text).expect("the rest reads");
        assert_eq!(
            config.characters.len(),
            AppConfig::default().characters.len()
        );
        assert_eq!(config.windows.len(), 1);
        assert_eq!(config.windows[0].id, "now");
        let written = config.to_toml_string().unwrap();
        assert!(
            written.contains("hologram_wall"),
            "not written back:\n{written}"
        );
    }
}

#[cfg(test)]
mod entry_keys_tests {
    use super::*;

    /// A newer version's keys inside a reminder, a schedule rule and a
    /// character's animation state are written back with the entry they
    /// came in — even when an entry before it is gone (BUG-011). They
    /// were dropped at load and lost at the next save.
    #[test]
    fn a_newer_versions_keys_stay_with_their_entry() {
        let mut text = AppConfig::default().to_toml_string().unwrap();
        text.push_str(
            "\n[[reminders]]\ntext = \"Stretch\"\nevery_minutes = 30\nchime = \"bell\"\n\
             \n[[reminders]]\ntext = \"Water\"\nevery_minutes = 45\nvolume_boost = 2\n\
             [reminders.at]\ntime = \"10:00\"\n\
             \n[[scene_schedule]]\nscene = \"Work\"\nat = \"09:00\"\nfade_secs = 3\n",
        );
        let (mut config, _) = AppConfig::decode_config_str(&text).expect("reads");
        assert_eq!(config.reminders.len(), 2);
        let written = config.to_toml_string().unwrap();
        for key in ["chime = \"bell\"", "volume_boost = 2", "fade_secs = 3"] {
            assert!(written.contains(key), "{key} not written back:\n{written}");
        }
        // The first reminder deleted: the second keeps its own key, and the
        // first's goes with it.
        config.reminders.remove(0);
        let written = config.to_toml_string().unwrap();
        let (back, _) = AppConfig::decode_config_str(&written).expect("reads again");
        assert_eq!(back.reminders.len(), 1);
        assert_eq!(back.reminders[0].text, "Water");
        assert!(written.contains("volume_boost = 2"), "{written}");
        assert!(!written.contains("chime"), "{written}");
    }

    /// A state's own newer keys go with the state: one deleted here does
    /// not come back as a part-entry holding only them.
    #[test]
    fn a_states_newer_keys_go_with_it() {
        let mut text = AppConfig::default().to_toml_string().unwrap();
        text.push_str(
            "\n[[characters]]\nid = \"walker\"\nname = \"Walker\"\nasset_type = \"png_sequence\"\n\
             asset_path = \"assets/demo/cat\"\nx = 1.0\ny = 1.0\n\
             [characters.animations.walk]\nasset_type = \"png_sequence\"\n\
             asset_path = \"assets/demo/cat/walk\"\nsway = 0.5\n",
        );
        let (mut config, _) = AppConfig::decode_config_str(&text).expect("reads");
        let walker = config
            .characters
            .iter()
            .position(|c| c.id == "walker")
            .unwrap();
        let written = config.to_toml_string().unwrap();
        assert!(written.contains("sway = 0.5"), "{written}");
        config.characters[walker]
            .animations
            .remove(&crate::animation::StateId::Walk);
        let written = config.to_toml_string().unwrap();
        assert!(
            !written.contains("sway"),
            "came back as a part-entry:\n{written}"
        );
        let (back, _) = AppConfig::decode_config_str(&written).expect("reads again");
        assert!(back.characters.iter().any(|c| c.id == "walker"));
    }
}

#[cfg(test)]
mod duplicate_id_tests {
    use super::*;

    /// Two characters with one id — a hand-edited file — each keep their
    /// own newer keys: the second's went to the first, found first by
    /// its id (BUG-011).
    #[test]
    fn two_characters_with_one_id_keep_their_own_keys() {
        let mut text = AppConfig::default().to_toml_string().unwrap();
        for (key, x) in [("first_only", 1.0), ("second_only", 2.0)] {
            text.push_str(&format!(
                "\n[[characters]]\nid = \"twin\"\nname = \"Twin\"\nasset_type = \"png_sequence\"\n\
                 asset_path = \"assets/demo/cat\"\nx = {x}\ny = 1.0\n{key} = true\n"
            ));
        }
        let (config, _) = AppConfig::decode_config_str(&text).expect("reads");
        let written: toml::Table = toml::from_str(&config.to_toml_string().unwrap()).unwrap();
        let twins: Vec<&toml::Table> = written["characters"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(toml::Value::as_table)
            .filter(|c| c.get("id").and_then(toml::Value::as_str) == Some("twin"))
            .collect();
        assert_eq!(twins.len(), 2);
        assert!(twins[0].contains_key("first_only") && !twins[0].contains_key("second_only"));
        assert!(twins[1].contains_key("second_only") && !twins[1].contains_key("first_only"));
    }
}
