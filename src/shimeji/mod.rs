//! Shimeji pack importer (U.3). Spec: `docs/shimeji-import.md`.
//!
//! `import_pack(pack_root, dest_root)` parses `conf/actions.xml`,
//! maps the supported action subset onto our animation states, copies
//! the referenced sprites into `dest_root/imported/<slug>/<state>/`
//! as ordered PNG sequences, and returns the `CharacterConfig`s plus
//! a skip report. Best-effort per pack: anything malformed skips the
//! affected piece with a written reason, never aborts the import.
//!
//! Security posture (enforced here, fuzzed in W.4):
//! - quick-xml has **no DTD/entity expansion at all** — XML bombs are
//!   dead by construction; size/depth/attribute caps below bound the
//!   rest.
//! - Every `Image` attribute resolves via canonicalize-and-prefix-
//!   check against the pack root, the same pattern as
//!   `drop_validate::resolve_library_asset`.
//! - Stat-based pack caps run before any pixel is decoded.

use crate::behavior::Behavior;
use crate::config::{AssetType, CharacterConfig, StateSequenceConfig};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Caps, per docs/shimeji-import.md.
const MAX_XML_BYTES: u64 = 2 * 1024 * 1024;
const MAX_XML_DEPTH: usize = 32;
const MAX_ATTR_BYTES: usize = 4096;
const MAX_PACK_SPRITES: usize = 512;
const MAX_PACK_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ACTIONS: usize = 256;
const MAX_POSES_PER_ACTION: usize = 256;

/// One shimeji engine tick, in milliseconds (shimeji-ee default).
const TICK_MS: u32 = 40;
/// Per-frame delay clamp, ms.
const MIN_DELAY_MS: u32 = 20;
const MAX_DELAY_MS: u32 = 10_000;

/// Import outcome: zero or more characters plus the skip ledger.
#[derive(Debug)]
pub struct ImportReport {
    pub pack_name: String,
    pub characters: Vec<CharacterConfig>,
    /// (what was skipped, why) — surfaced verbatim in the UI report.
    pub skipped: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionKind {
    Stay,
    Move,
    Animate,
    Other,
}

#[derive(Debug)]
struct ParsedPose {
    image: String,
    velocity_x: f32,
    duration_ticks: u32,
}

#[derive(Debug)]
struct ParsedAction {
    name: String,
    kind: ActionKind,
    poses: Vec<ParsedPose>,
}

/// Import one pack directory. `dest_root` is the asset-library root;
/// sprites land under `dest_root/imported/<slug>/`.
pub fn import_pack(pack_root: &Path, dest_root: &Path) -> Result<ImportReport, String> {
    let pack_root = pack_root
        .canonicalize()
        .map_err(|e| format!("pack root unreachable: {e}"))?;
    let pack_name = pack_root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "pack".into());
    let slug = slugify(&pack_name);

    let mut report = ImportReport {
        pack_name: pack_name.clone(),
        characters: Vec::new(),
        skipped: Vec::new(),
    };

    // ── Stat-based pack caps, before any decode ─────────────────────
    let (sprite_count, pack_bytes) = pack_stats(&pack_root)?;
    if sprite_count > MAX_PACK_SPRITES {
        return Err(format!(
            "pack has {sprite_count} sprites; cap is {MAX_PACK_SPRITES}"
        ));
    }
    if pack_bytes > MAX_PACK_BYTES {
        return Err(format!(
            "pack is {} MiB on disk; cap is {} MiB",
            pack_bytes / (1024 * 1024),
            MAX_PACK_BYTES / (1024 * 1024)
        ));
    }

    // ── Locate + parse actions.xml ──────────────────────────────────
    let actions_xml = find_actions_xml(&pack_root)?;
    let xml_text = read_capped_utf8(&actions_xml)?;
    let actions = parse_actions(&xml_text)?;

    // ── Map actions → states ────────────────────────────────────────
    let idle = actions
        .iter()
        .find(|a| a.kind == ActionKind::Stay && !a.poses.is_empty());
    let Some(idle) = idle else {
        return Err("no usable Stay action — a pack needs at least an idle pose".into());
    };
    let walk = actions
        .iter()
        .filter(|a| a.kind == ActionKind::Move && !a.poses.is_empty())
        .find(|a| a.name.to_ascii_lowercase().contains("walk"))
        .or_else(|| {
            actions.iter().find(|a| {
                a.kind == ActionKind::Move && a.poses.iter().any(|p| p.velocity_x.abs() > 0.0)
            })
        });
    let fall = actions
        .iter()
        .find(|a| a.name.to_ascii_lowercase().contains("fall") && !a.poses.is_empty());
    let drag = actions.iter().find(|a| {
        let n = a.name.to_ascii_lowercase();
        (n.contains("dragged") || n.contains("pinched")) && !a.poses.is_empty()
    });

    // ── Copy sprites + build per-state sequence configs ─────────────
    let import_dir = dest_root.join("imported").join(&slug);

    let idle_dir = copy_state_sequence(&pack_root, &import_dir, "idle", &idle.poses, &mut report)?;
    let idle_fps = fps_from_poses(&idle.poses);

    let mut animations: BTreeMap<crate::animation::StateId, StateSequenceConfig> = BTreeMap::new();
    for (state, action, label) in [
        (crate::animation::StateId::Walk, walk, "walk"),
        (crate::animation::StateId::Fall, fall, "fall"),
        (crate::animation::StateId::Drag, drag, "drag"),
    ] {
        let Some(action) = action else {
            report.skipped.push((
                label.to_string(),
                "no matching action in pack; idle will be used".into(),
            ));
            continue;
        };
        match copy_state_sequence(&pack_root, &import_dir, label, &action.poses, &mut report) {
            Ok(dir) => {
                animations.insert(
                    state,
                    StateSequenceConfig {
                        asset_type: AssetType::PngSequence,
                        asset_path: dir.to_string_lossy().into_owned(),
                        fps: Some(fps_from_poses(&action.poses)),
                        spritesheet_columns: None,
                        spritesheet_rows: None,
                    },
                );
            }
            Err(e) => report.skipped.push((label.to_string(), e)),
        }
    }

    // Walk speed from pose velocities: |vx| px/tick × 25 ticks/s.
    let behavior = match walk {
        Some(w) => {
            let speeds: Vec<f32> = w
                .poses
                .iter()
                .map(|p| p.velocity_x.abs())
                .filter(|v| *v > 0.0)
                .collect();
            if speeds.is_empty() {
                Behavior::Idle
            } else {
                let avg = speeds.iter().sum::<f32>() / speeds.len() as f32;
                Behavior::WalkAround {
                    speed: (avg * 1000.0 / TICK_MS as f32).clamp(10.0, 400.0),
                }
            }
        }
        None => Behavior::Idle,
    };

    report.characters.push(CharacterConfig {
        id: slug.clone(),
        name: pack_name,
        asset_type: AssetType::PngSequence,
        asset_path: idle_dir.to_string_lossy().into_owned(),
        x: 100.0,
        y: 100.0,
        scale: 1.0,
        opacity: 1.0,
        fps: idle_fps,
        visible: true,
        playing: true,
        z_index: 0,
        physics_enabled: false,
        behavior,
        spritesheet_columns: None,
        spritesheet_rows: None,
        monitor: None,
        easing: None,
        animations,
    });

    Ok(report)
}

/// `My Mascot (v2)!` → `my-mascot-v2`
fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_dash = true;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "pack".into()
    } else {
        trimmed.into()
    }
}

/// Count PNG sprites + total byte size of the pack (recursive, depth-
/// capped walk; symlinks not followed).
fn pack_stats(root: &Path) -> Result<(usize, u64), String> {
    fn walk(dir: &Path, depth: usize, count: &mut usize, bytes: &mut u64) -> Result<(), String> {
        if depth > 8 {
            return Err("pack directory nesting too deep".into());
        }
        let entries = std::fs::read_dir(dir).map_err(|e| format!("read_dir: {e}"))?;
        for entry in entries.flatten() {
            let meta = entry
                .metadata()
                .map_err(|e| format!("stat {}: {e}", entry.path().display()))?;
            if meta.file_type().is_symlink() {
                continue; // never follow symlinks inside a pack
            }
            if meta.is_dir() {
                walk(&entry.path(), depth + 1, count, bytes)?;
            } else {
                *bytes = bytes.saturating_add(meta.len());
                let p = entry.path();
                if p.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("png"))
                {
                    *count += 1;
                }
            }
        }
        Ok(())
    }
    let mut count = 0;
    let mut bytes = 0;
    walk(root, 0, &mut count, &mut bytes)?;
    Ok((count, bytes))
}

/// `conf/actions.xml` by convention; otherwise the first `conf/*.xml`
/// whose root element local-name is `Mascot`.
fn find_actions_xml(root: &Path) -> Result<PathBuf, String> {
    let conventional = root.join("conf").join("actions.xml");
    if conventional.is_file() {
        return Ok(conventional);
    }
    let conf = root.join("conf");
    let entries =
        std::fs::read_dir(&conf).map_err(|_| "pack has no conf/ directory".to_string())?;
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) != Some("xml") {
            continue;
        }
        if let Ok(text) = read_capped_utf8(&p) {
            let mut reader = Reader::from_str(&text);
            loop {
                match reader.read_event() {
                    Ok(Event::Start(e)) => {
                        let is_mascot = e.local_name().as_ref() == b"Mascot";
                        if is_mascot {
                            return Ok(p);
                        }
                        break;
                    }
                    Ok(Event::Eof) | Err(_) => break,
                    _ => continue,
                }
            }
        }
    }
    Err("no actions.xml found (and no conf/*.xml with a <Mascot> root)".into())
}

/// Read a file with the XML size cap, requiring valid UTF-8.
fn read_capped_utf8(path: &Path) -> Result<String, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("stat: {e}"))?;
    if meta.len() > MAX_XML_BYTES {
        return Err(format!(
            "XML is {} KiB; cap is {} KiB",
            meta.len() / 1024,
            MAX_XML_BYTES / 1024
        ));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("read: {e}"))?;
    String::from_utf8(bytes)
        .map_err(|_| "XML is not valid UTF-8 (Shift-JIS packs are not supported)".into())
}

/// Stream-parse the `<ActionList>` into our intermediate shape.
fn parse_actions(xml: &str) -> Result<Vec<ParsedAction>, String> {
    let mut reader = Reader::from_str(xml);
    let mut actions: Vec<ParsedAction> = Vec::new();
    let mut depth = 0usize;
    let mut current: Option<ParsedAction> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                depth += 1;
                if depth > MAX_XML_DEPTH {
                    return Err("XML nesting exceeds depth cap".into());
                }
                if e.local_name().as_ref() == b"Action" {
                    if actions.len() >= MAX_ACTIONS {
                        return Err("too many actions in pack".into());
                    }
                    current = Some(action_from_attrs(&e)?);
                }
            }
            Ok(Event::Empty(e)) => {
                // Empty elements don't change depth.
                if e.local_name().as_ref() == b"Pose" {
                    if let Some(action) = current.as_mut() {
                        if action.poses.len() >= MAX_POSES_PER_ACTION {
                            return Err(format!("action '{}' has too many poses", action.name));
                        }
                        if let Some(pose) = pose_from_attrs(&e)? {
                            action.poses.push(pose);
                        }
                    }
                } else if e.local_name().as_ref() == b"Action" {
                    // Pose-less self-closed action (references etc.) —
                    // record it so name-based lookups see it as empty.
                    if actions.len() >= MAX_ACTIONS {
                        return Err("too many actions in pack".into());
                    }
                    actions.push(action_from_attrs(&e)?);
                }
            }
            Ok(Event::End(e)) => {
                depth = depth.saturating_sub(1);
                if e.local_name().as_ref() == b"Action" {
                    if let Some(action) = current.take() {
                        actions.push(action);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {e}")),
            _ => {}
        }
    }
    Ok(actions)
}

fn attr_value(e: &quick_xml::events::BytesStart, name: &[u8]) -> Result<Option<String>, String> {
    for attr in e.attributes() {
        let attr = attr.map_err(|e| format!("bad attribute: {e}"))?;
        if attr.key.local_name().as_ref() == name {
            if attr.value.len() > MAX_ATTR_BYTES {
                return Err("attribute exceeds length cap".into());
            }
            let v = attr
                .unescape_value()
                .map_err(|e| format!("attribute decode: {e}"))?;
            return Ok(Some(v.into_owned()));
        }
    }
    Ok(None)
}

fn action_from_attrs(e: &quick_xml::events::BytesStart) -> Result<ParsedAction, String> {
    let name = attr_value(e, b"Name")?.unwrap_or_default();
    let kind = match attr_value(e, b"Type")?.as_deref() {
        Some("Stay") => ActionKind::Stay,
        Some("Move") => ActionKind::Move,
        Some("Animate") => ActionKind::Animate,
        _ => ActionKind::Other,
    };
    Ok(ParsedAction {
        name,
        kind,
        poses: Vec::new(),
    })
}

fn pose_from_attrs(e: &quick_xml::events::BytesStart) -> Result<Option<ParsedPose>, String> {
    let Some(image) = attr_value(e, b"Image")? else {
        return Ok(None); // pose without an image carries nothing for us
    };
    let velocity_x = attr_value(e, b"Velocity")?
        .and_then(|v| v.split(',').next().map(|s| s.trim().to_string()))
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(0.0);
    let duration_ticks = attr_value(e, b"Duration")?
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(6);
    Ok(Some(ParsedPose {
        image,
        velocity_x,
        duration_ticks,
    }))
}

/// Average pose duration → fps, clamped to [1, 60].
fn fps_from_poses(poses: &[ParsedPose]) -> f32 {
    if poses.is_empty() {
        return 8.0;
    }
    let total_ticks: u64 = poses.iter().map(|p| p.duration_ticks as u64).sum();
    let avg_ms = ((total_ticks * TICK_MS as u64) / poses.len() as u64)
        .clamp(MIN_DELAY_MS as u64, MAX_DELAY_MS as u64);
    (1000.0 / avg_ms as f32).clamp(1.0, 60.0)
}

/// Resolve one `Image` attribute against the pack root, enforcing
/// containment — the same canonicalize-and-prefix-check pattern as
/// `drop_validate::resolve_library_asset`.
fn resolve_pack_image(pack_root: &Path, image: &str) -> Result<PathBuf, String> {
    // Shimeji image refs start with `/` meaning "relative to img/".
    let rel = image.trim_start_matches('/');
    let candidate = pack_root.join("img").join(rel);
    let canonical = candidate
        .canonicalize()
        .map_err(|e| format!("sprite {image:?} unreachable: {e}"))?;
    if !canonical.starts_with(pack_root) {
        return Err(format!("sprite {image:?} escapes the pack root"));
    }
    if canonical
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| !e.eq_ignore_ascii_case("png"))
        .unwrap_or(true)
    {
        return Err(format!("sprite {image:?} is not a PNG"));
    }
    Ok(canonical)
}

/// Copy a pose sequence into `<import_dir>/<state>/frame_NNN.png`,
/// preserving pose order (the PNG-sequence loader sorts by name).
/// Returns the sequence directory.
fn copy_state_sequence(
    pack_root: &Path,
    import_dir: &Path,
    state: &str,
    poses: &[ParsedPose],
    report: &mut ImportReport,
) -> Result<PathBuf, String> {
    let dir = import_dir.join(state);
    std::fs::create_dir_all(&dir).map_err(|e| format!("create {state} dir: {e}"))?;

    let mut copied = 0usize;
    for (i, pose) in poses.iter().enumerate() {
        let src = match resolve_pack_image(pack_root, &pose.image) {
            Ok(p) => p,
            Err(why) => {
                report.skipped.push((format!("{state} pose {i}"), why));
                continue;
            }
        };
        let dest = dir.join(format!("frame_{i:03}.png"));
        std::fs::copy(&src, &dest).map_err(|e| format!("copy frame {i}: {e}"))?;
        copied += 1;
    }
    if copied == 0 {
        let _ = std::fs::remove_dir(&dir);
        return Err(format!("{state}: no usable sprites"));
    }
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::StateId;

    const ACTIONS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Mascot>
  <ActionList>
    <Action Name="Stand" Type="Stay" BorderType="Floor">
      <Animation>
        <Pose Image="/shime1.png" Velocity="0,0" Duration="250" />
      </Animation>
    </Action>
    <Action Name="Walk" Type="Move" BorderType="Floor">
      <Animation>
        <Pose Image="/shime1.png" Velocity="-2,0" Duration="6" />
        <Pose Image="/shime2.png" Velocity="-2,0" Duration="6" />
      </Animation>
    </Action>
    <Action Name="Fall" Type="Move">
      <Animation>
        <Pose Image="/shime3.png" Velocity="0,4" Duration="2" />
      </Animation>
    </Action>
    <Action Name="Dragged" Type="Embedded" Class="com.example.Dragged">
      <Animation>
        <Pose Image="/shime4.png" Velocity="0,0" Duration="4" />
      </Animation>
    </Action>
    <Action Name="ClimbWall" Type="Embedded" Class="com.example.Climb" />
  </ActionList>
</Mascot>
"#;

    fn write_png(path: &Path, shade: u8) {
        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([shade, 0, 0, 255]));
        img.save(path).unwrap();
    }

    fn build_pack(name: &str) -> (PathBuf, PathBuf) {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("shimeji_tests")
            .join(name);
        let _ = std::fs::remove_dir_all(&base);
        let pack = base.join("TestMascot");
        let dest = base.join("library");
        std::fs::create_dir_all(pack.join("img")).unwrap();
        std::fs::create_dir_all(pack.join("conf")).unwrap();
        std::fs::create_dir_all(&dest).unwrap();
        for (i, shade) in [(1u8, 10u8), (2, 20), (3, 30), (4, 40)] {
            write_png(&pack.join("img").join(format!("shime{i}.png")), shade);
        }
        std::fs::write(pack.join("conf").join("actions.xml"), ACTIONS_XML).unwrap();
        (pack, dest)
    }

    #[test]
    fn golden_pack_imports_all_four_states() {
        let (pack, dest) = build_pack("golden");
        let report = import_pack(&pack, &dest).unwrap();

        assert_eq!(report.characters.len(), 1);
        let c = &report.characters[0];
        assert_eq!(c.id, "testmascot");
        assert!(matches!(c.asset_type, AssetType::PngSequence));
        // Idle: 250 ticks × 40ms = 10s clamps the fps to 1.
        assert!((c.fps - 1.0).abs() < f32::EPSILON, "idle fps {}", c.fps);
        // Walk: 6 ticks × 40ms = 240ms → ~4.17 fps; speed 2 px/tick → 50 px/s.
        let walk = &c.animations[&StateId::Walk];
        assert!((walk.fps.unwrap() - (1000.0 / 240.0)).abs() < 0.05);
        assert!(matches!(
            c.behavior,
            Behavior::WalkAround { speed } if (speed - 50.0).abs() < 0.5
        ));
        assert!(c.animations.contains_key(&StateId::Fall));
        assert!(c.animations.contains_key(&StateId::Drag));

        // Frames copied in pose order.
        let walk_dir = PathBuf::from(&walk.asset_path);
        assert!(walk_dir.join("frame_000.png").is_file());
        assert!(walk_dir.join("frame_001.png").is_file());
        // ClimbWall (Embedded, no usable mapping) didn't produce a state
        // and the report stayed clean of hard errors.
        assert_eq!(c.animations.len(), 3);
    }

    #[test]
    fn traversal_in_image_attr_is_skipped() {
        let (pack, dest) = build_pack("traversal");
        let evil = ACTIONS_XML.replace("/shime3.png", "/../../outside.png");
        std::fs::write(pack.join("conf").join("actions.xml"), evil).unwrap();
        // Plant the escape target so canonicalize succeeds — the
        // prefix check must still reject it.
        write_png(&pack.parent().unwrap().join("outside.png"), 99);

        let report = import_pack(&pack, &dest).unwrap();
        // Fall state died (its only pose escaped); the rest imported.
        let c = &report.characters[0];
        assert!(!c.animations.contains_key(&StateId::Fall));
        assert!(report
            .skipped
            .iter()
            .any(|(_, why)| why.contains("escapes") || why.contains("no usable")));
        // And nothing got copied out of thin air.
        assert!(!dest.join("imported/testmascot/fall/frame_000.png").exists());
    }

    #[test]
    fn missing_stay_action_fails_with_reason() {
        let (pack, dest) = build_pack("no_stay");
        let no_stay = ACTIONS_XML.replace("Type=\"Stay\"", "Type=\"Animate\"");
        std::fs::write(pack.join("conf").join("actions.xml"), no_stay).unwrap();
        let err = import_pack(&pack, &dest).unwrap_err();
        assert!(err.contains("Stay"), "got: {err}");
    }

    #[test]
    fn depth_bomb_rejected() {
        let (pack, dest) = build_pack("depth");
        let mut bomb = String::from("<Mascot>");
        for _ in 0..64 {
            bomb.push_str("<a>");
        }
        std::fs::write(pack.join("conf").join("actions.xml"), bomb).unwrap();
        let err = import_pack(&pack, &dest).unwrap_err();
        assert!(err.contains("depth"), "got: {err}");
    }

    #[test]
    fn non_utf8_xml_rejected() {
        let (pack, dest) = build_pack("sjis");
        std::fs::write(
            pack.join("conf").join("actions.xml"),
            [0x83u8, 0x7B, 0x83, 0x70, 0x83, 0x93],
        )
        .unwrap();
        let err = import_pack(&pack, &dest).unwrap_err();
        assert!(err.contains("UTF-8"), "got: {err}");
    }

    #[test]
    fn slugify_normalises() {
        assert_eq!(slugify("My Mascot (v2)!"), "my-mascot-v2");
        assert_eq!(slugify("__"), "pack");
        assert_eq!(slugify("Shimeji"), "shimeji");
    }
}
