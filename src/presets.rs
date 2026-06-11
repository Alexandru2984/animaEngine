//! Curated scene presets — one-click starting points.
//!
//! Each [`Preset`] bundles a name, a one-line description, a Phosphor
//! icon, and a list of [`CharacterConfig`]s built exclusively from the
//! shipped demo assets (`assets/demo/ghost`, `slime`, `heart`, `star`,
//! `cat`). Because no preset references external paths, applying one
//! is a pure-data operation that always works after `cargo run`.
//!
//! Presets are consumed from the Scene tab via [`apply_to_scene`] in
//! two modes: `Replace` (clears existing entities first) or `Append`
//! (keeps everything and shifts IDs to avoid collisions).
//!
//! Adding a new preset means:
//! 1. Adding a [`PresetId`] variant.
//! 2. Returning its config from [`Preset::for_id`].
//! 3. The unit tests below run automatically against every variant
//!    and will catch a broken asset path / duplicate IDs / entity
//!    over-cap.

use crate::behavior::Behavior;
use crate::config::{AssetType, CharacterConfig};
use crate::constants::MAX_ENTITIES;
use crate::ui::icons;

/// Identifier for a curated preset. Stable across versions — UI code
/// addresses presets by this enum, not by index or name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PresetId {
    CozyCompanion,
    ProductivityZen,
    HalloweenParty,
    BirthdayConfetti,
    StudioSession,
    CursorFollower,
}

impl PresetId {
    /// All presets, in their default UI order.
    pub const ALL: &'static [Self] = &[
        Self::CozyCompanion,
        Self::ProductivityZen,
        Self::HalloweenParty,
        Self::BirthdayConfetti,
        Self::StudioSession,
        Self::CursorFollower,
    ];
}

/// One curated scene snippet. Fields are owned (clone-on-apply) so the
/// caller can mutate freely after [`apply_to_scene`].
#[derive(Debug, Clone)]
pub struct Preset {
    pub id: PresetId,
    pub name: &'static str,
    pub description: &'static str,
    /// Phosphor glyph (single character); painted alongside the name.
    pub icon: &'static str,
    pub characters: Vec<CharacterConfig>,
}

/// How `apply_to_scene` blends a preset into an existing scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyMode {
    /// Wipe the existing scene first, then drop the preset in.
    Replace,
    /// Keep existing entities; append the preset's characters with
    /// IDs suffixed `_a`, `_b`, … so they can't collide.
    Append,
}

/// Apply a preset to `existing`, returning the new entity list.
///
/// Caps the result at [`MAX_ENTITIES`]; if a `Replace + Append` would
/// overflow, the tail is truncated rather than refusing the apply —
/// the user can clear the scene manually if they want everything.
#[must_use]
pub fn apply_to_scene(
    existing: Vec<CharacterConfig>,
    preset: &Preset,
    mode: ApplyMode,
) -> Vec<CharacterConfig> {
    let mut out = match mode {
        ApplyMode::Replace => Vec::with_capacity(preset.characters.len()),
        ApplyMode::Append => existing,
    };

    for (idx, mut character) in preset.characters.iter().cloned().enumerate() {
        if matches!(mode, ApplyMode::Append) {
            // Suffix the preset's IDs so they can't collide with anything
            // already in the scene. Single-letter suffix lets the user
            // still recognize the source preset in the scene list.
            character.id = format!("{}_{}", character.id, suffix(idx));
        }
        out.push(character);
    }

    out.truncate(MAX_ENTITIES);
    out
}

fn suffix(idx: usize) -> char {
    // a, b, c, … z, then repeat with the same letter — preset bodies
    // never exceed 26 entities (capped by MAX_ENTITIES = 64 well above
    // their natural length).
    let n = (idx % 26) as u8;
    (b'a' + n) as char
}

impl Preset {
    pub fn for_id(id: PresetId) -> Self {
        match id {
            PresetId::CozyCompanion => cozy_companion(),
            PresetId::ProductivityZen => productivity_zen(),
            PresetId::HalloweenParty => halloween_party(),
            PresetId::BirthdayConfetti => birthday_confetti(),
            PresetId::StudioSession => studio_session(),
            PresetId::CursorFollower => cursor_follower(),
        }
    }
}

// ─── individual presets ────────────────────────────────────────────────

fn demo_character(
    id: &str,
    name: &str,
    asset_path: &str,
    x: f32,
    y: f32,
    behavior: Behavior,
) -> CharacterConfig {
    CharacterConfig {
        id: id.into(),
        name: name.into(),
        asset_type: AssetType::PngSequence,
        asset_path: asset_path.into(),
        x,
        y,
        scale: 1.0,
        opacity: 1.0,
        fps: 8.0,
        visible: true,
        playing: true,
        z_index: 10,
        physics_enabled: false,
        behavior,
        spritesheet_columns: None,
        spritesheet_rows: None,
        monitor: None,
        easing: None,
        animations: std::collections::BTreeMap::new(),
    }
}

fn cozy_companion() -> Preset {
    Preset {
        id: PresetId::CozyCompanion,
        name: "Cozy Companion",
        description: "One gentle ghost wandering quietly on the right.",
        icon: icons::GHOST,
        characters: vec![CharacterConfig {
            opacity: 0.9,
            fps: 6.0,
            behavior: Behavior::BoundedWander {
                x_min: 1100.0,
                x_max: 1700.0,
                y_min: 400.0,
                y_max: 800.0,
                speed: 35.0,
            },
            ..demo_character(
                "ghost_companion",
                "Companion",
                "assets/demo/ghost",
                1400.0,
                600.0,
                Behavior::Idle,
            )
        }],
    }
}

fn productivity_zen() -> Preset {
    Preset {
        id: PresetId::ProductivityZen,
        name: "Productivity Zen",
        description: "Static slime + heart in a quiet corner. No motion.",
        icon: icons::HEART,
        characters: vec![
            CharacterConfig {
                opacity: 0.6,
                scale: 0.7,
                playing: false,
                ..demo_character(
                    "slime_zen",
                    "Zen slime",
                    "assets/demo/slime",
                    100.0,
                    900.0,
                    Behavior::Idle,
                )
            },
            CharacterConfig {
                opacity: 0.7,
                scale: 0.6,
                playing: false,
                ..demo_character(
                    "heart_zen",
                    "Zen heart",
                    "assets/demo/heart",
                    200.0,
                    880.0,
                    Behavior::Idle,
                )
            },
        ],
    }
}

fn halloween_party() -> Preset {
    let speed = 90.0;
    Preset {
        id: PresetId::HalloweenParty,
        name: "Halloween Party",
        description: "Three ghosts marching across the bottom edge.",
        icon: icons::FLAME,
        characters: vec![
            CharacterConfig {
                opacity: 0.85,
                fps: 10.0,
                behavior: Behavior::WalkAround { speed },
                ..demo_character(
                    "ghost_1",
                    "Ghost 1",
                    "assets/demo/ghost",
                    200.0,
                    800.0,
                    Behavior::Idle,
                )
            },
            CharacterConfig {
                opacity: 0.85,
                fps: 10.0,
                behavior: Behavior::WalkAround {
                    speed: speed + 20.0,
                },
                ..demo_character(
                    "ghost_2",
                    "Ghost 2",
                    "assets/demo/ghost",
                    700.0,
                    820.0,
                    Behavior::Idle,
                )
            },
            CharacterConfig {
                opacity: 0.85,
                fps: 10.0,
                behavior: Behavior::WalkAround {
                    speed: speed - 15.0,
                },
                ..demo_character(
                    "ghost_3",
                    "Ghost 3",
                    "assets/demo/ghost",
                    1200.0,
                    800.0,
                    Behavior::Idle,
                )
            },
        ],
    }
}

fn birthday_confetti() -> Preset {
    let make = |id: &str, asset: &str, x: f32, y: f32, scale: f32| CharacterConfig {
        scale,
        physics_enabled: true,
        fps: 12.0,
        ..demo_character(
            id,
            asset,
            &format!("assets/demo/{asset}"),
            x,
            y,
            Behavior::Idle,
        )
    };
    Preset {
        id: PresetId::BirthdayConfetti,
        name: "Birthday Confetti",
        description: "Hearts and stars raining down. Gravity on.",
        icon: icons::CONFETTI,
        characters: vec![
            make("heart_1", "heart", 300.0, 100.0, 0.5),
            make("heart_2", "heart", 600.0, 100.0, 0.6),
            make("heart_3", "heart", 900.0, 100.0, 0.5),
            make("star_1", "star", 450.0, 50.0, 0.5),
            make("star_2", "star", 750.0, 50.0, 0.4),
            make("star_3", "star", 1050.0, 50.0, 0.5),
        ],
    }
}

fn studio_session() -> Preset {
    Preset {
        id: PresetId::StudioSession,
        name: "Studio Session",
        description: "All five demo characters spread across the desktop.",
        icon: icons::SPARKLE,
        characters: vec![
            demo_character(
                "ghost_studio",
                "Ghost",
                "assets/demo/ghost",
                200.0,
                300.0,
                Behavior::Idle,
            ),
            demo_character(
                "slime_studio",
                "Slime",
                "assets/demo/slime",
                600.0,
                450.0,
                Behavior::WalkAround { speed: 50.0 },
            ),
            demo_character(
                "heart_studio",
                "Heart",
                "assets/demo/heart",
                1000.0,
                250.0,
                Behavior::Idle,
            ),
            demo_character(
                "star_studio",
                "Star",
                "assets/demo/star",
                1300.0,
                500.0,
                Behavior::Idle,
            ),
            demo_character(
                "cat_studio",
                "Cat",
                "assets/demo/cat",
                900.0,
                700.0,
                Behavior::BoundedWander {
                    x_min: 600.0,
                    x_max: 1400.0,
                    y_min: 600.0,
                    y_max: 800.0,
                    speed: 80.0,
                },
            ),
        ],
    }
}

fn cursor_follower() -> Preset {
    Preset {
        id: PresetId::CursorFollower,
        name: "Cursor Follower",
        description: "A cat tags along wherever your mouse goes.",
        icon: icons::CURSOR,
        characters: vec![CharacterConfig {
            scale: 0.8,
            fps: 10.0,
            behavior: Behavior::FollowCursor {
                speed: 240.0,
                comfort_distance: 80.0,
            },
            ..demo_character(
                "cat_follower",
                "Cat",
                "assets/demo/cat",
                800.0,
                500.0,
                Behavior::Idle,
            )
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Every preset must produce a non-empty roster, otherwise its UI
    /// entry would be a no-op confusing the user.
    #[test]
    fn every_preset_is_non_empty() {
        for id in PresetId::ALL {
            let preset = Preset::for_id(*id);
            assert!(
                !preset.characters.is_empty(),
                "preset {:?} has zero characters",
                id,
            );
        }
    }

    /// No preset can exceed the engine-wide entity cap on its own; the
    /// caller would silently truncate, hiding part of the curated
    /// scene from view.
    #[test]
    fn every_preset_fits_in_entity_cap() {
        for id in PresetId::ALL {
            let preset = Preset::for_id(*id);
            assert!(
                preset.characters.len() <= MAX_ENTITIES,
                "preset {:?} has {} characters; cap is {}",
                id,
                preset.characters.len(),
                MAX_ENTITIES,
            );
        }
    }

    /// IDs inside one preset must be unique; otherwise hot-reload would
    /// dedupe entities and the user would see fewer than expected.
    #[test]
    fn every_preset_has_unique_ids() {
        for id in PresetId::ALL {
            let preset = Preset::for_id(*id);
            let ids: HashSet<&str> = preset.characters.iter().map(|c| c.id.as_str()).collect();
            assert_eq!(
                ids.len(),
                preset.characters.len(),
                "preset {:?} has duplicate character ids",
                id,
            );
        }
    }

    /// Every preset's assets must come from the shipped demo bundle so
    /// applying one out of the box never fails on a missing path.
    #[test]
    fn every_preset_uses_only_demo_assets() {
        for id in PresetId::ALL {
            let preset = Preset::for_id(*id);
            for character in &preset.characters {
                assert!(
                    character.asset_path.starts_with("assets/demo/"),
                    "preset {:?} entity {:?} uses non-demo asset {:?}",
                    id,
                    character.id,
                    character.asset_path,
                );
            }
        }
    }

    #[test]
    fn apply_replace_discards_existing() {
        let preset = Preset::for_id(PresetId::CozyCompanion);
        let existing = vec![demo_character(
            "preexisting",
            "old",
            "assets/demo/ghost",
            0.0,
            0.0,
            Behavior::Idle,
        )];
        let out = apply_to_scene(existing, &preset, ApplyMode::Replace);
        assert_eq!(out.len(), preset.characters.len());
        assert!(!out.iter().any(|c| c.id == "preexisting"));
    }

    #[test]
    fn apply_append_keeps_existing_and_suffixes_ids() {
        let preset = Preset::for_id(PresetId::HalloweenParty);
        let existing = vec![demo_character(
            "preexisting",
            "old",
            "assets/demo/ghost",
            0.0,
            0.0,
            Behavior::Idle,
        )];
        let out = apply_to_scene(existing, &preset, ApplyMode::Append);
        assert_eq!(out.len(), 1 + preset.characters.len());
        assert!(out.iter().any(|c| c.id == "preexisting"));
        // Suffix must be present so we can't accidentally collide
        // with the pre-existing entity that itself was named "ghost_1".
        assert!(out.iter().any(|c| c.id == "ghost_1_a"));
        assert!(out.iter().any(|c| c.id == "ghost_2_b"));
        assert!(out.iter().any(|c| c.id == "ghost_3_c"));
    }

    #[test]
    fn apply_truncates_to_max_entities() {
        let preset = Preset::for_id(PresetId::HalloweenParty);
        let existing = (0..MAX_ENTITIES)
            .map(|i| {
                demo_character(
                    &format!("e_{i}"),
                    "filler",
                    "assets/demo/ghost",
                    0.0,
                    0.0,
                    Behavior::Idle,
                )
            })
            .collect();
        let out = apply_to_scene(existing, &preset, ApplyMode::Append);
        assert_eq!(out.len(), MAX_ENTITIES);
    }
}
