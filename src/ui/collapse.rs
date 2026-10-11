//! Persistent open/closed state for the inspector sections and the
//! Scene-tab preset gallery (D.2).
//!
//! egui's `CollapsingHeader` stores its open flag in `egui::Memory`
//! by default — that's only session-lifetime, so users had to
//! re-collapse the same sections after every restart. This struct
//! lifts the bools out of egui memory into `AppConfig` so they
//! survive across sessions; the inspector / scene tab read & write
//! the matching field every frame, and `config_dirty` flips when the
//! user toggles a section, picking the new state up through the
//! usual save-on-edit-mode-exit path.

use serde::{Deserialize, Serialize};

/// All persistable collapse flags. New sections add a field +
/// `#[serde(default)]` so older configs that don't carry the field
/// still decode cleanly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollapseState {
    /// Inspector → Position section. Default open.
    #[serde(default = "default_true")]
    pub inspector_position: bool,
    /// Inspector → Appearance section. Default open.
    #[serde(default = "default_true")]
    pub inspector_appearance: bool,
    /// Inspector → Animation section. Default open.
    #[serde(default = "default_true")]
    pub inspector_animation: bool,
    /// Inspector → Behavior section. Default closed — less common edit.
    #[serde(default)]
    pub inspector_behavior: bool,
    /// Inspector → Says section (`crate::phrases`). Default closed.
    #[serde(default)]
    pub inspector_talk: bool,
    /// Scene tab → preset gallery. Default open so fresh installs
    /// discover the curated scenes; closes on click and stays closed.
    #[serde(default = "default_true")]
    pub scene_presets: bool,
    /// Scene tab sections (1.5): open where most visits go — the
    /// characters, their groups, the saved scenes — and closed for the
    /// rest, whose headers still say what they hold.
    #[serde(default = "default_true")]
    pub scene_characters: bool,
    #[serde(default = "default_true")]
    pub scene_groups: bool,
    #[serde(default = "default_true")]
    pub scene_saved: bool,
    #[serde(default)]
    pub scene_reminders: bool,
    #[serde(default)]
    pub scene_focus: bool,
    #[serde(default)]
    pub scene_together: bool,
    #[serde(default)]
    pub scene_monitors: bool,
}

fn default_true() -> bool {
    true
}

impl Default for CollapseState {
    fn default() -> Self {
        Self {
            inspector_position: true,
            inspector_appearance: true,
            inspector_animation: true,
            inspector_behavior: false,
            inspector_talk: false,
            scene_presets: true,
            scene_characters: true,
            scene_groups: true,
            scene_saved: true,
            scene_reminders: false,
            scene_focus: false,
            scene_together: false,
            scene_monitors: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Defaults must match the pre-D.2 hard-coded `default_open` values
    /// — otherwise upgrading users would see their inspector sections
    /// reshuffle on first launch.
    #[test]
    fn defaults_match_pre_d2_open_states() {
        let d = CollapseState::default();
        assert!(d.inspector_position);
        assert!(d.inspector_appearance);
        assert!(d.inspector_animation);
        assert!(!d.inspector_behavior);
        assert!(d.scene_presets);
    }

    /// A pre-0.4 config without `[collapse_state]` must decode cleanly
    /// because the parent struct flags every field with `#[serde(default)]`.
    /// This test exercises the standalone default round-trip.
    #[test]
    fn empty_toml_round_trips_through_defaults() {
        let cs: CollapseState = toml::from_str("").unwrap();
        let defaults = CollapseState::default();
        assert_eq!(cs.inspector_position, defaults.inspector_position);
        assert_eq!(cs.inspector_appearance, defaults.inspector_appearance);
        assert_eq!(cs.inspector_animation, defaults.inspector_animation);
        assert_eq!(cs.inspector_behavior, defaults.inspector_behavior);
        assert_eq!(cs.scene_presets, defaults.scene_presets);
    }

    /// Round-trip: serialise current state, parse back, fields match.
    #[test]
    fn serde_round_trip_preserves_every_field() {
        let original = CollapseState {
            inspector_position: false,
            inspector_appearance: false,
            inspector_animation: true,
            inspector_behavior: true,
            inspector_talk: true,
            scene_presets: false,
            // The Scene tab's, each the other way from its default.
            scene_characters: false,
            scene_groups: false,
            scene_saved: false,
            scene_reminders: true,
            scene_focus: true,
            scene_together: true,
            scene_monitors: true,
        };
        let s = toml::to_string(&original).unwrap();
        let back: CollapseState = toml::from_str(&s).unwrap();
        assert_eq!(original, back);
    }
}
