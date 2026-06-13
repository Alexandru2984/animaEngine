//! Sprite groups — the data layer beneath
//! `docs/engine-features.md` §6.
//!
//! A `GroupConfig` carries a stable id, a display name, an explicit
//! list of member entity ids, and group-level transform overrides
//! (offset / scale / visibility). Composition rules:
//!
//! - **Position**: `effective = (member.x + group.offset_x,
//!   member.y + group.offset_y)` — composed at draw time via
//!   `transform_for_member`, used by both the renderer and
//!   `Scene::entity_at_point` so hit-testing matches the painted quad.
//! - **Scale**: `effective = member.scale * group.scale` (same path).
//! - **Visibility**: `effective = member.visible && group.visible`
//!   via `visible_for_member`, consumed by `Scene::visible_entities`.
//!
//! **No nesting in 0.3.** A future 0.4 may add parent groups; for now
//! the relationship is flat: each entity belongs to at most zero or
//! one group.
//!
//! Membership is stored on the group, not the entity. Two
//! consequences:
//! - Removing an entity must scrub its id from every group's
//!   `member_ids`. `cleanup_after_entity_removal` is the canonical
//!   helper for that.
//! - Adding the same entity to two groups is allowed by the data
//!   layer but the runtime composition resolves only the first; a
//!   warn-level log fires when a duplicate is observed.

use serde::{Deserialize, Serialize};

/// One sprite group. Persisted in `AppConfig.groups`. Empty
/// `member_ids` are valid (an empty group is a stub the user is
/// building).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupConfig {
    /// Stable identifier. Survives renames of `name`. Tray menus and
    /// the future "Activate this group" hotkey address groups by id.
    pub id: String,
    /// User-visible name; freely editable.
    pub name: String,
    /// Entity ids belonging to the group, in user-declared order.
    /// Order matters for the Inspector tree view but the renderer
    /// continues to draw by `z_index`.
    #[serde(default)]
    pub member_ids: Vec<String>,
    /// Pixels added to every member's stored x position when the
    /// renderer composes the group, via `transform_for_member`.
    #[serde(default)]
    pub offset_x: f32,
    #[serde(default)]
    pub offset_y: f32,
    /// Multiplier applied to every member's `scale`, composed on the
    /// same path as the offsets.
    #[serde(default = "default_scale")]
    pub scale: f32,
    /// Group-level visibility. `false` hides every member regardless
    /// of their individual `visible` flag.
    #[serde(default = "default_true")]
    pub visible: bool,
}

fn default_scale() -> f32 {
    1.0
}
fn default_true() -> bool {
    true
}

impl Default for GroupConfig {
    /// Visible + unit scale + no members. Mirrors the serde defaults
    /// for missing fields so a default-constructed group is
    /// indistinguishable from one decoded with only `id` + `name`
    /// set.
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            member_ids: Vec::new(),
            offset_x: 0.0,
            offset_y: 0.0,
            scale: default_scale(),
            visible: default_true(),
        }
    }
}

/// Resolve a member's *effective* visibility under group composition.
///
/// Returns `member_visible` when:
/// - no group claims the entity
/// - the entity's group has `visible = true`
///
/// Returns `false` when the entity's group has `visible = false`.
///
/// If multiple groups claim the entity (data layer allows it; UI
/// shouldn't), the first match wins and the duplicate is logged.
pub fn visible_for_member(groups: &[GroupConfig], entity_id: &str, member_visible: bool) -> bool {
    let mut owning: Option<&GroupConfig> = None;
    for g in groups {
        if g.member_ids.iter().any(|m| m == entity_id) {
            if owning.is_some() {
                tracing::warn!(
                    "Entity {:?} belongs to multiple groups; resolving via first match",
                    entity_id,
                );
                break;
            }
            owning = Some(g);
        }
    }
    match owning {
        Some(g) => member_visible && g.visible,
        None => member_visible,
    }
}

/// Remove `removed_id` from every group's `member_ids`. Cleans up
/// after `Scene::remove_entity` so groups never reference a missing
/// entity (which would render as a silent member-count drift in the
/// Inspector tree).
///
/// `cleanup_after_entity_removal` is called from `Scene::remove_entity`
/// after the entity has been popped; passing the live groups slice
/// keeps the data consistent without a second pass.
/// Composed visual transform contributed by the entity's owning
/// group: `(offset_x, offset_y, scale_multiplier)`. Identity when the
/// entity belongs to no group. First owning group wins — the same
/// tie-break rule as [`visible_for_member`], so visibility and
/// transform can't disagree about ownership (C.9).
pub fn transform_for_member(groups: &[GroupConfig], entity_id: &str) -> (f32, f32, f32) {
    for g in groups {
        if g.member_ids.iter().any(|m| m == entity_id) {
            return (g.offset_x, g.offset_y, g.scale);
        }
    }
    (0.0, 0.0, 1.0)
}

pub fn cleanup_after_entity_removal(groups: &mut [GroupConfig], removed_id: &str) {
    for g in groups {
        g.member_ids.retain(|m| m != removed_id);
    }
}

/// Validate that group ids are unique within the slice. Returns the
/// first duplicate id encountered, or `None` when all are unique.
/// Used by `AppConfig::load` to log (not fail) on a hand-edited
/// config that accidentally clones a group entry.
pub fn first_duplicate_id(groups: &[GroupConfig]) -> Option<String> {
    let mut seen = std::collections::HashSet::with_capacity(groups.len());
    for g in groups {
        if !seen.insert(g.id.as_str()) {
            return Some(g.id.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(id: &str, name: &str, members: &[&str], visible: bool) -> GroupConfig {
        GroupConfig {
            id: id.into(),
            name: name.into(),
            member_ids: members.iter().map(|s| (*s).to_string()).collect(),
            offset_x: 0.0,
            offset_y: 0.0,
            scale: 1.0,
            visible,
        }
    }

    #[test]
    fn transform_identity_for_non_member() {
        let groups = vec![g("a", "A", &["ghost"], true)];
        assert_eq!(transform_for_member(&groups, "cat"), (0.0, 0.0, 1.0));
        assert_eq!(transform_for_member(&[], "ghost"), (0.0, 0.0, 1.0));
    }

    #[test]
    fn transform_composes_from_owning_group() {
        let mut grp = g("a", "A", &["ghost"], true);
        grp.offset_x = 12.0;
        grp.offset_y = -30.0;
        grp.scale = 1.5;
        assert_eq!(transform_for_member(&[grp], "ghost"), (12.0, -30.0, 1.5));
    }

    #[test]
    fn transform_first_owning_group_wins() {
        let mut g1 = g("a", "A", &["ghost"], true);
        g1.offset_x = 1.0;
        let mut g2 = g("b", "B", &["ghost"], true);
        g2.offset_x = 99.0;
        assert_eq!(transform_for_member(&[g1, g2], "ghost").0, 1.0);
    }

    #[test]
    fn default_group_is_visible_with_unit_scale() {
        let d = GroupConfig::default();
        assert!(d.visible);
        assert_eq!(d.scale, 1.0);
        assert!(d.member_ids.is_empty());
    }

    #[test]
    fn member_visibility_ands_with_group_visibility() {
        let groups = vec![g("party", "Party", &["ghost"], false)];
        // Group invisible: hide the member even if it's visible.
        assert!(!visible_for_member(&groups, "ghost", true));
        // Group invisible + member already invisible: stays invisible.
        assert!(!visible_for_member(&groups, "ghost", false));
    }

    #[test]
    fn member_outside_any_group_keeps_own_visibility() {
        let groups = vec![g("party", "Party", &["ghost"], true)];
        // "cat" isn't in any group → returns its own flag.
        assert!(visible_for_member(&groups, "cat", true));
        assert!(!visible_for_member(&groups, "cat", false));
    }

    #[test]
    fn visible_group_passes_through_member_flag() {
        let groups = vec![g("party", "Party", &["ghost"], true)];
        assert!(visible_for_member(&groups, "ghost", true));
        assert!(!visible_for_member(&groups, "ghost", false));
    }

    #[test]
    fn cleanup_removes_id_from_every_group() {
        let mut groups = vec![
            g("a", "A", &["ghost", "slime"], true),
            g("b", "B", &["slime", "cat"], true),
        ];
        cleanup_after_entity_removal(&mut groups, "slime");
        assert_eq!(groups[0].member_ids, vec!["ghost".to_string()]);
        assert_eq!(groups[1].member_ids, vec!["cat".to_string()]);
    }

    #[test]
    fn cleanup_with_no_matches_is_no_op() {
        let mut groups = vec![g("a", "A", &["ghost"], true)];
        let before = groups.clone();
        cleanup_after_entity_removal(&mut groups, "missing");
        assert_eq!(groups, before);
    }

    #[test]
    fn duplicate_id_detection_returns_first_dup() {
        let groups = vec![
            g("a", "A", &[], true),
            g("b", "B", &[], true),
            g("a", "A2", &[], true),
        ];
        assert_eq!(first_duplicate_id(&groups).as_deref(), Some("a"));
    }

    #[test]
    fn no_duplicates_returns_none() {
        let groups = vec![g("a", "A", &[], true), g("b", "B", &[], true)];
        assert!(first_duplicate_id(&groups).is_none());
    }

    #[test]
    fn empty_groups_round_trip_through_toml() {
        let groups: Vec<GroupConfig> = vec![];
        #[derive(Serialize, Deserialize)]
        struct W {
            #[serde(default)]
            groups: Vec<GroupConfig>,
        }
        let s = toml::to_string(&W {
            groups: groups.clone(),
        })
        .unwrap();
        let back: W = toml::from_str(&s).unwrap();
        assert!(back.groups.is_empty());
    }

    #[test]
    fn group_with_members_round_trips_through_toml() {
        let g = GroupConfig {
            id: "halloween".into(),
            name: "Halloween Squad".into(),
            member_ids: vec!["g1".into(), "g2".into()],
            offset_x: 10.0,
            offset_y: -5.0,
            scale: 1.5,
            visible: true,
        };
        let s = toml::to_string(&g).unwrap();
        let back: GroupConfig = toml::from_str(&s).unwrap();
        assert_eq!(back, g);
    }

    /// Pre-0.3 configs decode cleanly: every field except id and name
    /// has a `#[serde(default)]`, so a minimal `[[groups]]` entry
    /// with just id + name is valid.
    #[test]
    fn minimal_group_toml_decodes() {
        let toml_str = r#"
            id = "tiny"
            name = "Tiny"
        "#;
        let g: GroupConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(g.id, "tiny");
        assert_eq!(g.scale, 1.0);
        assert!(g.visible);
        assert!(g.member_ids.is_empty());
    }

    /// Data layer allows duplicate membership but `visible_for_member`
    /// resolves the first owning group and logs the rest. This guards
    /// the contract: no panic on data we don't fully validate yet.
    #[test]
    fn duplicate_membership_resolves_via_first_match() {
        let groups = vec![
            g("a", "A", &["ghost"], true),
            g("b", "B", &["ghost"], false),
        ];
        // First match (visible=true) wins.
        assert!(visible_for_member(&groups, "ghost", true));
    }
}
