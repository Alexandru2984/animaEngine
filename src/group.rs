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

/// Composed visual transform contributed by the entity's owning
/// group: `(offset_x, offset_y, scale_multiplier)`. Identity when the
/// entity belongs to no group. First owning group wins — the same
/// tie-break rule as [`visible_for_member`], so visibility and
/// transform can't disagree about ownership (C.9).
pub fn transform_for_member(groups: &[GroupConfig], entity_id: &str) -> (f32, f32, f32) {
    match owning_group(groups, entity_id) {
        Some(g) => (g.offset_x, g.offset_y, g.scale),
        None => (0.0, 0.0, 1.0),
    }
}

/// The group whose transform applies to the entity: the first that
/// lists it.
pub fn owning_group<'g>(groups: &'g [GroupConfig], entity_id: &str) -> Option<&'g GroupConfig> {
    groups
        .iter()
        .find(|g| g.member_ids.iter().any(|m| m == entity_id))
}

/// Remove `removed_id` from every group's `member_ids`. Cleans up
/// after `Scene::remove_entity` so groups never reference a missing
/// entity (which would render as a silent member-count drift in the
/// Inspector tree).
///
/// `cleanup_after_entity_removal` is called from `Scene::remove_entity`
/// after the entity has been popped; passing the live groups slice
/// keeps the data consistent without a second pass.
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

// ── Grouping characters (1.5) ────────────────────────────────────────
//
// Until 1.5 groups could only be written into config.toml by hand. These
// make and dissolve them from a selection. None of them changes what is on
// screen: a character that leaves a group takes the group's offset, scale
// and visibility into its own.

impl crate::scene::Scene {
    /// The entity at `entity_idx` and every other member of its group, in
    /// scene order — what a press on a grouped character selects. Just
    /// the entity when it is in no group.
    pub fn with_its_group(&self, entity_idx: usize) -> Vec<usize> {
        let Some(entity) = self.entities.get(entity_idx) else {
            return Vec::new();
        };
        match owning_group(&self.groups, &entity.id) {
            Some(group) => self
                .entities
                .iter()
                .enumerate()
                .filter(|(_, e)| group.member_ids.contains(&e.id))
                .map(|(i, _)| i)
                .collect(),
            None => vec![entity_idx],
        }
    }

    /// Whether the entities at `indices` are exactly the members of one
    /// group — grouping them again would only rename it.
    pub fn already_a_group(&self, indices: &[usize]) -> bool {
        let ids = self.distinct_ids(indices);
        !ids.is_empty()
            && self.groups.iter().any(|g| {
                g.member_ids.len() == ids.len() && ids.iter().all(|id| g.member_ids.contains(id))
            })
    }

    /// Put the entities at `indices` in a new group, named by `name` from
    /// its number. Each leaves the group it was in first, and a group
    /// emptied that way goes. Returns the new group's name; `None` when
    /// no index is an entity, or when they already are one group.
    pub fn group_entities(
        &mut self,
        indices: &[usize],
        name: impl FnOnce(usize) -> String,
    ) -> Option<String> {
        let member_ids = self.distinct_ids(indices);
        if member_ids.is_empty() || self.already_a_group(indices) {
            return None;
        }
        let left: Vec<String> = self
            .groups
            .iter()
            .filter(|g| g.member_ids.iter().any(|m| member_ids.contains(m)))
            .map(|g| g.id.clone())
            .collect();
        for &idx in indices {
            self.leave_group(idx);
        }
        self.groups
            .retain(|g| !(left.contains(&g.id) && g.member_ids.is_empty()));
        let number = (1..)
            .find(|n| {
                let id = format!("group_{n}");
                !self.groups.iter().any(|g| g.id == id)
            })
            .unwrap_or(1);
        let name = name(number);
        self.groups.push(GroupConfig {
            id: format!("group_{number}"),
            name: name.clone(),
            member_ids,
            ..GroupConfig::default()
        });
        self.mark_visible_dirty();
        Some(name)
    }

    /// Dissolve every group one of the entities at `indices` is in.
    /// Returns how many went.
    pub fn ungroup_entities(&mut self, indices: &[usize]) -> usize {
        let mut doomed: Vec<String> = Vec::new();
        for entity in indices.iter().filter_map(|&i| self.entities.get(i)) {
            if let Some(group) = owning_group(&self.groups, &entity.id) {
                if !doomed.contains(&group.id) {
                    doomed.push(group.id.clone());
                }
            }
        }
        for id in &doomed {
            self.dissolve_group(id);
        }
        doomed.len()
    }

    /// Dissolve the group `id`: its members stay as they look, and the
    /// group goes. Returns whether there was one.
    pub fn dissolve_group(&mut self, id: &str) -> bool {
        let Some(group) = self.groups.iter().find(|g| g.id == id) else {
            return false;
        };
        for member in group.member_ids.clone() {
            if let Some(idx) = self.entities.iter().position(|e| e.id == member) {
                self.leave_group(idx);
            }
        }
        self.groups.retain(|g| g.id != id);
        self.mark_visible_dirty();
        true
    }

    /// The ids of the entities at `indices`, each once, in that order.
    fn distinct_ids(&self, indices: &[usize]) -> Vec<String> {
        let mut ids: Vec<String> = Vec::new();
        for entity in indices.iter().filter_map(|&i| self.entities.get(i)) {
            if !ids.contains(&entity.id) {
                ids.push(entity.id.clone());
            }
        }
        ids
    }

    /// Take the entity at `idx` out of every group, folding the one that
    /// applied to it into its own position, scale and visibility.
    fn leave_group(&mut self, idx: usize) {
        let Some(entity) = self.entities.get_mut(idx) else {
            return;
        };
        if let Some(group) = owning_group(&self.groups, &entity.id) {
            entity.x += group.offset_x;
            entity.y += group.offset_y;
            entity.scale *= group.scale;
            entity.visible &= group.visible;
            if group.offset_x != 0.0 || group.offset_y != 0.0 {
                entity.behavior_state.bounce_invalidate();
            }
        }
        for group in &mut self.groups {
            group.member_ids.retain(|m| *m != entity.id);
        }
    }
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

    /// Characters "a", "b", "c" (no asset: fallback circles) at x = 0,
    /// 100, 200.
    fn scene() -> crate::scene::Scene {
        let mut scene = crate::scene::Scene::from_config(&crate::config::AppConfig::default());
        let configs: Vec<_> = [("a", 0.0), ("b", 100.0), ("c", 200.0)]
            .into_iter()
            .map(|(id, x)| crate::config::CharacterConfig {
                id: id.into(),
                name: id.into(),
                asset_type: crate::config::AssetType::PngStatic,
                asset_path: String::new(),
                x,
                y: 0.0,
                scale: 1.0,
                opacity: 1.0,
                fps: 8.0,
                visible: true,
                playing: false,
                z_index: 0,
                physics_enabled: false,
                behavior: crate::behavior::Behavior::Idle,
                spritesheet_columns: None,
                spritesheet_rows: None,
                monitor: None,
                easing: None,
                animations: std::collections::BTreeMap::new(),
            })
            .collect();
        scene.restore_configs(&configs);
        scene
    }

    fn number(n: usize) -> String {
        format!("Group {n}")
    }

    #[test]
    fn grouping_makes_a_numbered_group_of_the_selection() {
        let mut s = scene();
        assert_eq!(
            s.group_entities(&[2, 0, 2], number).as_deref(),
            Some("Group 1")
        );
        assert_eq!(s.groups.len(), 1);
        assert_eq!(s.groups[0].id, "group_1");
        assert_eq!(s.groups[0].member_ids, ["c", "a"], "each once");
        assert_eq!(s.with_its_group(2), [0, 2], "in scene order");
        assert_eq!(s.with_its_group(1), [1], "not grouped: alone");
        assert_eq!(s.group_entities(&[1], number).as_deref(), Some("Group 2"));
        assert!(s.group_entities(&[9], number).is_none(), "nothing to group");
        // Already exactly a group: left as it is, name and all.
        s.groups[0].name = "Party".into();
        assert!(s.already_a_group(&[0, 2]));
        assert!(s.group_entities(&[0, 2], number).is_none());
        assert_eq!(s.groups[0].name, "Party");
        assert!(!s.already_a_group(&[0, 1, 2]), "more than the group");
    }

    #[test]
    fn regrouping_takes_members_out_of_their_old_group() {
        let mut s = scene();
        s.group_entities(&[0, 1], number);
        s.group_entities(&[1, 2], number);
        let members: Vec<_> = s.groups.iter().map(|g| g.member_ids.clone()).collect();
        assert_eq!(members, [vec!["a"], vec!["b", "c"]]);
        // Emptied by a regroup, a group goes; its number is free again.
        s.group_entities(&[0, 1], number);
        let members: Vec<_> = s.groups.iter().map(|g| g.member_ids.clone()).collect();
        assert_eq!(members, [vec!["c"], vec!["a", "b"]]);
        assert_eq!(s.groups[1].id, "group_1");
    }

    #[test]
    fn leaving_a_group_keeps_how_a_character_looks() {
        let mut s = scene();
        s.group_entities(&[0, 1], number);
        s.groups[0].offset_x = 30.0;
        s.groups[0].offset_y = -10.0;
        s.groups[0].scale = 2.0;
        s.groups[0].visible = false;
        assert_eq!(s.ungroup_entities(&[1]), 1);
        assert!(s.groups.is_empty());
        let a = &s.entities[0];
        assert_eq!((a.x, a.y, a.scale, a.visible), (30.0, -10.0, 2.0, false));
        assert_eq!(s.entities[2].x, 200.0, "not a member: untouched");
    }

    #[test]
    fn a_save_takes_the_groups_along() {
        let mut s = scene();
        s.group_entities(&[0, 1], number);
        let mut config = crate::config::AppConfig::default();
        config.take_scene(&s);
        assert_eq!(config.groups, s.groups);
    }

    #[test]
    fn ungrouping_dissolves_each_group_once() {
        let mut s = scene();
        s.group_entities(&[0, 1], number);
        s.group_entities(&[2], number);
        assert_eq!(s.ungroup_entities(&[0, 1, 2]), 2);
        assert!(s.groups.is_empty());
        assert_eq!(s.ungroup_entities(&[0]), 0, "none left");
        assert!(!s.dissolve_group("group_1"));
    }
}
