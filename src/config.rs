use crate::behavior::Behavior;
use crate::constants::MAX_ENTITIES;
use crate::error::Result;
use crate::keybindings::KeyBindings;
use crate::monitor::MonitorMode;
use crate::ui::{CollapseState, OnboardingProgress, Theme};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

mod character;
mod global;
mod migrate;
mod set_aside;
#[cfg(test)]
mod tests;

pub use character::{
    default_fps, AssetType, CharacterConfig, StateSequenceConfig, Talk, WindowConfig,
};
pub(crate) use character::{finite_clamp, finite_place};
pub use global::GlobalConfig;
use migrate::*;
pub use set_aside::SetAside;

/// Current config schema version (W.0, 0.9). Bumped only when a change
/// is **not** purely additive — additive fields keep using serde
/// `default` and need no migration. `v1` is every config written before
/// 0.9 (no `version` key at all). See `migrate_table`.
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

/// serde default for [`AppConfig::schema_version`]: a config file with
/// no `version` key predates 0.9, i.e. schema v1.
fn schema_version_legacy() -> u32 {
    1
}

/// Full application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Schema version. **Must be the first field** so it serialises as a
    /// top-level scalar before any `[section]` (TOML forbids a bare key
    /// after a table). Absent in pre-0.9 files → treated as v1 and
    /// migrated on load; always re-written as [`CURRENT_SCHEMA_VERSION`].
    #[serde(rename = "version", default = "schema_version_legacy")]
    pub schema_version: u32,
    // Every table and key a person might leave out has a default. These
    // three and the `[global]` table itself used not to, so a hand-written
    // config naming only the settings it cared about failed to decode —
    // and at startup that means backed up and replaced by the demo scene.
    #[serde(default)]
    pub global: GlobalConfig,
    #[serde(rename = "characters", default)]
    pub characters: Vec<CharacterConfig>,
    /// Multi-window roster. Empty (or absent) means "legacy single
    /// overlay backed by `characters` above" — 0.2 configs decode
    /// identically with no migration. When non-empty, `characters`
    /// continues to work and is treated as the first implicit window
    /// so we never silently drop user entities.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub windows: Vec<WindowConfig>,
    /// Sprite groups (C.8). Empty for the legacy/0.2 path. Each
    /// `GroupConfig` carries a stable id and a list of member entity
    /// ids; composition rules (offset, scale, visibility) are
    /// applied at render / visibility-resolve time.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<crate::group::GroupConfig>,
    /// What characters say every so often (`crate::reminders`, 1.5).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reminders: Vec<crate::reminders::ReminderConfig>,
    /// Saved scenes to switch to at set times (`crate::schedule`, 1.5).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scene_schedule: Vec<crate::schedule::ScheduleRule>,
    /// Rebindable keyboard shortcuts (D.1). Defaults match the 0.3
    /// hard-coded set, so existing configs decode without losing any
    /// binding and pre-D configs without a `[keybindings]` section
    /// still behave exactly as before.
    #[serde(default)]
    pub keybindings: KeyBindings,
    /// Open/closed state of every persistable collapse section (D.2).
    /// Pre-0.4 configs without the `[collapse_state]` table fall back
    /// to the design-system defaults defined in
    /// `CollapseState::default()`.
    #[serde(default)]
    pub collapse_state: CollapseState,
    /// What of the file did not read, kept to be written back (1.5).
    #[serde(skip)]
    pub set_aside: SetAside,
    /// Forward-compatibility catch-all (W.0). Top-level `[section]`
    /// blocks this build doesn't model — e.g. a section added by a
    /// newer animaEngine — are captured here on load and written back
    /// verbatim on save, instead of being silently dropped. **Must be
    /// the last field** so unknown tables serialise after the known
    /// ones. Preserves unknown *sections*, not unknown bare keys at the
    /// document root (TOML ordering makes that unrepresentable here).
    #[serde(flatten, default)]
    pub extra: toml::Table,
}

impl AppConfig {
    /// Coerce every character's renderer-facing scalars into safe, finite
    /// ranges (see [`CharacterConfig::sanitize`]). Called once on load so
    /// a hand-edited or corrupt config can't feed `NaN`/`inf` to the GPU.
    pub(crate) fn sanitize(&mut self) {
        self.global.reminder_sound.sanitize();
        self.global.sound_volume = finite_clamp(self.global.sound_volume, 0.0, 1.0, 1.0);
        for c in &mut self.characters {
            c.sanitize();
        }
        for w in &mut self.windows {
            for c in &mut w.characters {
                c.sanitize();
            }
        }
        for g in &mut self.groups {
            g.sanitize();
        }
    }
}

/// One of the demo characters a first run starts with: the picture
/// sequence named after `id`, idle, at full size.
fn demo_character(
    id: &str,
    name: &str,
    (x, y): (f32, f32),
    opacity: f32,
    fps: f32,
    z_index: i32,
) -> CharacterConfig {
    CharacterConfig {
        id: id.to_string(),
        name: name.to_string(),
        asset_type: AssetType::PngSequence,
        asset_path: format!("assets/demo/{id}"),
        x,
        y,
        scale: 1.0,
        opacity,
        fps,
        visible: true,
        playing: true,
        z_index,
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

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            global: GlobalConfig::default(),
            characters: vec![
                demo_character("ghost", "Ghost Demo", (200.0, 300.0), 0.9, 10.0, 10),
                demo_character("slime", "Slime Demo", (600.0, 400.0), 1.0, 8.0, 20),
                demo_character("heart", "Heart Demo", (1000.0, 200.0), 1.0, 8.0, 30),
                demo_character("star", "Star Demo", (1300.0, 450.0), 1.0, 8.0, 40),
                demo_character("cat", "Cat Demo", (900.0, 600.0), 1.0, 8.0, 50),
            ],
            // 0.3 fresh installs use the legacy single-window shape;
            // multi-window is opt-in via UI / hand-edit (data layer ready
            // in C.3, render-side dispatch coming in 0.4).
            windows: vec![],
            groups: vec![],
            reminders: vec![],
            scene_schedule: vec![],
            keybindings: KeyBindings::default(),
            collapse_state: CollapseState::default(),
            schema_version: CURRENT_SCHEMA_VERSION,
            set_aside: SetAside::default(),
            extra: toml::Table::new(),
        }
    }
}

/// For `fuzz/fuzz_targets/config_round_trip.rs`: a config read from
/// `text`, whatever it holds, then written and read again reads the same
/// and writes the same — what was set aside or unknown comes back where
/// it was, every round, and nothing grows.
#[doc(hidden)]
pub fn fuzz_config_round_trip(text: &str) {
    let Ok((first, _)) = AppConfig::decode_config_str(text) else {
        return;
    };
    let once = first.to_toml_string().expect("a config read is written");
    let (second, _) = AppConfig::decode_config_str(&once)
        .unwrap_or_else(|e| panic!("what was written does not read: {e}\n{once}"));
    let twice = second.to_toml_string().expect("and written again");
    assert_eq!(once, twice, "a second round changed it");
}

/// Set when startup found a config it could not read and loaded the
/// defaults instead (the original is kept as `config.toml.bak-corrupt`).
static LOAD_FELL_BACK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Set when startup found no config at all: a first run.
static FIRST_RUN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Whether this run is the first: there was no config to load.
pub fn first_run() -> bool {
    FIRST_RUN.load(std::sync::atomic::Ordering::Relaxed)
}

/// Whether this run's config was unreadable and replaced by the defaults.
/// Each backend turns it into a banner at startup.
pub fn loaded_defaults_over_unreadable_config() -> bool {
    LOAD_FELL_BACK.load(std::sync::atomic::Ordering::Relaxed)
}

/// Copy an existing-but-unreadable `config.toml` aside to
/// `config.toml.bak-corrupt` so the default-config save that follows a
/// failed load can never destroy the user's only copy. Best-effort and
/// idempotent per launch (a repeat corruption overwrites the previous
/// corrupt backup — both were unreadable, and the good-config safety
/// nets are the migration `.bak-v<n>` and crash-recovery snapshots).
/// Extracted from `AppConfig::load` so the decision is unit-testable
/// without redirecting the real config path.
fn backup_unreadable_config(path: &Path) {
    let backup = path.with_extension("toml.bak-corrupt");
    match crate::util::copy_private(path, &backup) {
        Ok(_) => tracing::warn!(
            "Config was unreadable — backed the original up to {} before writing defaults",
            crate::drop_validate::redact_path(&backup)
        ),
        Err(e) => tracing::warn!("Could not back up unreadable config: {e}"),
    }
}

impl AppConfig {
    /// Get the config file path: ~/.config/animaengine/config.toml
    pub fn config_path() -> PathBuf {
        // A unit test never reaches the user's config, whatever it runs:
        // one that dispatched "Save now" (2026-11-17) wrote its scene over
        // the config of the machine it ran on. Each test process gets a
        // directory of its own; scenes and backups beside the config
        // follow it there.
        #[cfg(test)]
        {
            std::env::temp_dir()
                .join(format!("animaengine-unit-tests-{}", std::process::id()))
                .join("config.toml")
        }
        #[cfg(not(test))]
        Self::user_config_path()
    }

    /// Where the config lives for the app (not for its unit tests).
    #[cfg(not(test))]
    fn user_config_path() -> PathBuf {
        if let Some(proj_dirs) = directories::ProjectDirs::from("", "", "animaEngine") {
            return proj_dirs.config_dir().to_path_buf().join("config.toml");
        }
        // Fallback when XDG resolution fails (minimal containers, broken
        // env, etc.). `fallback_scoped_dir` prefers $XDG_RUNTIME_DIR
        // (0700 + uid-owned by spec) and only then a verified tmpdir —
        // a plain /tmp subdir could be pre-created by another local
        // user, landing our atomic writes in a directory they own.
        crate::util::fallback_scoped_dir("").join("config.toml")
    }

    /// Parse → migrate (in memory) → decode → cap → sanitise a config
    /// from a TOML string. **Pure**: no disk writes, no backups, no
    /// default fallback. Returns the decoded config and the on-disk
    /// schema version it came from (so `load` can decide whether to
    /// back up + persist a migration). `migrate_table` mutates the
    /// in-memory table only — the file on disk is never touched here,
    /// which is what lets both the startup `load` and the read-only
    /// `try_reload` share this without one leaking writes into the other.
    fn decode_config_str(contents: &str) -> std::result::Result<(Self, u32), String> {
        let table = contents
            .parse::<toml::Table>()
            .map_err(|e| format!("parse error: {e}"))?;
        Self::decode_config_table(table)
    }

    /// [`Self::decode_config_str`] for a table already read — a backup's
    /// config (`scene_file::backup`).
    pub(crate) fn decode_config_table(
        mut table: toml::Table,
    ) -> std::result::Result<(Self, u32), String> {
        let from = detect_schema_version(&table);
        migrate_table(&mut table);
        // Deserialise from the migrated Value, not a re-serialised
        // string: a top-level scalar (`version`) sorted among the section
        // keys would trip TOML's "value after table" rule on the way back
        // out. `try_into` doesn't care about order.
        let mut unknown = SetAside::default();
        unknown.note_unknown_keys(&table);
        let mut config: Self = match toml::Value::Table(table.clone()).try_into() {
            Ok(config) => config,
            // Set aside what does not read (`SetAside`), and try again
            // with the rest; when that does not read either, the file is
            // unreadable as before, and says why.
            Err(first) => {
                let aside = SetAside::take_unreadable(&mut table);
                if aside.is_empty() {
                    return Err(format!("decode error: {first}"));
                }
                let mut config: Self = toml::Value::Table(table)
                    .try_into()
                    .map_err(|_| format!("decode error: {first}"))?;
                config.set_aside = aside;
                config
            }
        };
        if config.characters.len() > MAX_ENTITIES {
            tracing::warn!(
                "Config has {} characters, capping at {} to prevent resource exhaustion",
                config.characters.len(),
                MAX_ENTITIES
            );
            config.characters.truncate(MAX_ENTITIES);
        }
        // Coerce NaN/inf/out-of-range scalars before anything reaches the
        // renderer or physics.
        config.sanitize();
        config.set_aside.unknown = unknown.unknown;
        let global = config.global.clone();
        config.set_aside.note_settings(&global);
        Ok((config, from))
    }

    /// This config as its file says it: what is set aside
    /// ([`SetAside`]) written back where it was.
    pub fn to_toml_string(&self) -> std::result::Result<String, toml::ser::Error> {
        if self.set_aside.is_empty() {
            return toml::to_string_pretty(self);
        }
        let mut table = toml::Table::try_from(self)?;
        self.set_aside.write_back(&mut table);
        toml::to_string_pretty(&table)
    }

    /// Read-only config load for **hot-reload**. Reads and decodes the
    /// config file without ever writing to disk — no default fallback,
    /// no backup, no migration persist. A transient partial write (an
    /// editor doing truncate-then-write) or a hand-edit typo therefore
    /// returns `Err` and leaves both the file and the running scene
    /// untouched, instead of the startup `load` behaviour of overwriting
    /// the user's config with defaults. The caller keeps the last valid
    /// scene and surfaces the error.
    pub fn try_reload() -> std::result::Result<Self, String> {
        let path = Self::config_path();
        let contents =
            crate::util::read_to_string_capped(&path, crate::constants::MAX_CONFIG_BYTES)
                .map_err(|e| format!("read error: {e}"))?;
        let (config, _from) = Self::decode_config_str(&contents)?;
        Ok(config)
    }

    /// Load config from disk, or create default if not found
    pub fn load() -> Self {
        let path = Self::config_path();
        tracing::info!("Config path: {}", crate::drop_validate::redact_path(&path));
        tracing::debug!("Config path (full): {}", path.display());

        if path.exists() {
            match crate::util::read_to_string_capped(&path, crate::constants::MAX_CONFIG_BYTES) {
                Ok(contents) => match Self::decode_config_str(&contents) {
                    Ok((config, from)) => {
                        // Behind the current schema: copy the file aside
                        // *before* the persist below overwrites it, so a
                        // migration bug can never be why a user loses
                        // their config, then write the migrated form so
                        // the next launch skips re-migrating.
                        if from < CURRENT_SCHEMA_VERSION {
                            let backup = path.with_extension(format!("toml.bak-v{from}"));
                            match crate::util::copy_private(&path, &backup) {
                                Ok(_) => tracing::info!(
                                    "Migrating config v{from} → v{CURRENT_SCHEMA_VERSION}; backed up original to {}",
                                    crate::drop_validate::redact_path(&backup)
                                ),
                                Err(e) => tracing::warn!(
                                    "Could not write migration backup {}: {e}",
                                    crate::drop_validate::redact_path(&backup)
                                ),
                            }
                            if let Err(e) = config.save() {
                                tracing::warn!("Could not persist migrated config: {e}");
                            }
                        }
                        tracing::info!(
                            "Loaded config with {} characters (schema v{})",
                            config.characters.len(),
                            config.schema_version
                        );
                        return config;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load config: {e}. Using defaults.");
                    }
                },
                Err(e) => {
                    tracing::warn!("Failed to read config: {}. Using defaults.", e);
                }
            }
        } else {
            tracing::info!("Config not found, creating default config");
        }

        // Reaching here with the file still on disk means it exists but
        // couldn't be read/parsed/decoded — copy it aside before the
        // default save below overwrites it. Same never-lose-the-user's-
        // file rule as the migration backup above: a hand-edit typo
        // (hot-reload invites hand edits) must not cost the whole scene.
        // A genuinely first run — no config file at all — has no previous
        // version to catch up on, so the "what's new" panel is meaningless
        // to it: the three-step onboarding tour is the right first-run
        // surface. Stamping the current anchor keeps that panel for
        // *upgraders*, who are the only ones it is written for. A config
        // that exists but failed to parse is not a fresh install, so it
        // still gets the panel.
        let fresh_install = !path.exists();
        if path.exists() {
            backup_unreadable_config(&path);
            // The UI says so on screen (Warning::ConfigUnreadable): the log
            // line alone left a user looking at the demo scene in place of
            // their own, with no idea why.
            LOAD_FELL_BACK.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        let mut config = AppConfig::default();
        if fresh_install {
            config.global.last_seen_whats_new =
                Some(crate::constants::WHATS_NEW_VERSION.to_string());
            FIRST_RUN.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        if let Err(e) = config.save() {
            tracing::warn!("Failed to save default config: {}", e);
        }
        config
    }

    /// Name the demo characters a first run starts with in the app's
    /// language — "Ghost Demo" and the rest were English in every one.
    /// Only those still under their English name: one renamed by hand is
    /// the user's. Whether any was renamed.
    pub fn name_demo_characters(&mut self) -> bool {
        let mut renamed = false;
        for c in &mut self.characters {
            let key = match (c.id.as_str(), c.name.as_str()) {
                ("ghost", "Ghost Demo") => "demo-ghost",
                ("slime", "Slime Demo") => "demo-slime",
                ("heart", "Heart Demo") => "demo-heart",
                ("star", "Star Demo") => "demo-star",
                ("cat", "Cat Demo") => "demo-cat",
                _ => continue,
            };
            let mut args = fluent::FluentArgs::new();
            args.set("name", crate::i18n::t(key));
            let name = crate::i18n::t_args("demo-name", &args);
            if !name.contains('?') && name != c.name {
                c.name = name;
                renamed = true;
            }
        }
        renamed
    }

    /// Take in what the running scene holds — its characters, its groups
    /// and the playback switch — before a save. Both loops call it. The
    /// groups were left out until 1.5, when the app began to make them:
    /// they had only ever changed by hand in this file.
    pub fn take_scene(&mut self, scene: &crate::scene::Scene) {
        self.characters = scene.to_character_configs();
        self.groups = scene.groups.clone();
        self.global.playback_enabled = scene.global_playing;
    }

    /// Save config to disk **atomically** — writes to a temp sibling
    /// then renames over the target. A crash mid-save can no longer
    /// leave a truncated `config.toml`.
    #[tracing::instrument(skip(self), fields(n_chars = self.characters.len()))]
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        let toml_string = self.to_toml_string()?;
        crate::util::atomic_write_bytes(&path, toml_string.as_bytes())?;
        tracing::info!(
            "Config saved to {}",
            crate::drop_validate::redact_path(&path)
        );
        tracing::debug!("Config saved (full path): {}", path.display());
        Ok(())
    }

    /// Resolve an asset path relative to the executable or current directory.
    /// Supports:
    /// - Absolute paths (returned as-is)
    /// - `~/` expansion to home directory (bare `~` too; `~user` syntax
    ///   is NOT supported and passes through untouched — expanding it
    ///   with our own `$HOME` would silently build a path inside the
    ///   wrong home directory)
    /// - Relative paths (checked against exe dir, then cwd)
    pub fn resolve_asset_path(asset_path: &str) -> PathBuf {
        let asset_path = if asset_path == "~" || asset_path.starts_with("~/") {
            if let Ok(home) = std::env::var("HOME") {
                asset_path.replacen('~', &home, 1)
            } else {
                asset_path.to_string()
            }
        } else {
            asset_path.to_string()
        };

        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf));
        resolve_asset_path_in(
            &asset_path,
            exe_dir.as_deref(),
            Path::new(""),
            &crate::demo::dir(),
        )
    }

    // Asset-type detection lives in animation::loader::detect_asset_type
    // — see comments there for the canonical extension → AssetType table
    // (it covers JPEG and MP4 / MOV / M4V too).
}

/// [`AppConfig::resolve_asset_path`] past `~`: a relative path beside the
/// program (`exe_dir`), then in `cwd`, then — a built-in character's —
/// among the ones made in `demo_dir` (`crate::demo`), as an installed app
/// has no `assets/` beside it and the current directory is the launcher's.
///
/// A built-in character found beside the program or in `cwd` is taken only
/// when drawn as this version draws it (`crate::demo::is_current`). 1.2.0
/// made its pictures in the folder it was started from — the home folder,
/// from a launcher — and those stood in front of the ones made since in
/// the data directory: an upgrade kept the old drawings, with no walk, fall
/// or sleep. They are left on disk, and taken only when nothing else is.
fn resolve_asset_path_in(
    asset_path: &str,
    exe_dir: Option<&Path>,
    cwd: &Path,
    demo_dir: &Path,
) -> PathBuf {
    let path = Path::new(asset_path);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    let builtin = asset_path
        .strip_prefix("assets/demo/")
        .and_then(|rest| rest.split('/').next())
        .filter(|name| crate::demo::CHARACTERS.contains(name));
    let drawn_now = |base: &Path| {
        builtin.is_none_or(|name| crate::demo::is_current(&base.join("assets/demo").join(name)))
    };
    let mut stale = None;
    for base in exe_dir.into_iter().chain(std::iter::once(cwd)) {
        let resolved = base.join(asset_path);
        if resolved.exists() {
            if drawn_now(base) {
                return resolved;
            }
            stale.get_or_insert(resolved);
        }
    }
    if let Some(rest) = asset_path.strip_prefix("assets/demo/") {
        let generated = demo_dir.join(rest);
        if generated.exists() {
            return generated;
        }
    }
    // As it is when nothing is there: the caller says it is missing.
    stale.unwrap_or_else(|| PathBuf::from(asset_path))
}
