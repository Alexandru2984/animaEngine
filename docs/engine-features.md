# Engine features spec — Faza C / 0.3

Single source of truth for the engine work in 0.3. Sub-phases C.1
through C.9 implement what's described here; if you find yourself
inventing a new data shape mid-implementation, update this doc
first, then the code.

The goal isn't novelty — it's giving the existing engine more reach
without breaking the invariants Faza B locked in (asset caps, atomic
writes, drag-drop validation, single-method D-Bus).

---

## 1. Multi-monitor

### 1.1 Model

A user with two monitors today gets one overlay window that's
positioned wherever the WM dropped it. In 0.3 they get either:

- **One overlay per monitor** (`MonitorMode::PerMonitor`, default) —
  each monitor has its own surface, its own scene subset, and its
  own X11 shape / Wayland layer
- **One overlay spanning all monitors** (`MonitorMode::Span`) —
  current 0.2 behaviour, kept for compatibility
- **One overlay on a single chosen monitor** (`MonitorMode::Single
  { name }`) — useful for kiosk / streaming setups where only one
  display should carry the characters

The mode lives in `GlobalConfig.monitor_mode` and switches at
runtime (no restart). Switching `Span → PerMonitor` distributes
existing entities by their current pixel position into the right
monitor's scene.

### 1.2 Entity ↔ monitor binding

Each `CharacterConfig` gains an **optional**
`monitor: Option<String>` field (`#[serde(default)]`). Values:

- `None` — appears on whatever monitor the entity's `(x, y)`
  centroid falls into. This is the implicit default and keeps
  existing configs working.
- `Some("eDP-1")` / `Some("DP-2")` — pinned to a specific monitor.
  Coordinates are relative to that monitor's origin.

Monitor names match what `winit`'s `MonitorHandle::name()` returns
(`XRandR` output names on X11, `wl_output` names on Wayland). If
the named monitor disappears at runtime, the entity falls back to
the primary; a `tracing::warn!` records the fallback.

### 1.3 Coordinate system

To keep behaviors / hot-reload / drag-drop simple, **entity x/y
remains in monitor-local coordinates**. The renderer translates
into desktop-global coordinates only when handing pixels to the GPU.
Implication: an entity dragged across a monitor boundary updates its
`monitor` field as the centroid crosses the gap.

### 1.4 Edge cases

- **No monitors detected**: fall back to a single 1280×720 logical
  surface. Already what 0.2 does when winit returns an empty list;
  no change.
- **Monitor scaling** (HiDPI): each monitor's `scale_factor` is
  read from winit and applied per-surface. Entity sizes specified
  in logical pixels stay constant across monitors.
- **Wayland multi-output**: `wlr-layer-shell-unstable-v1` allows
  binding to a specific `wl_output`. The native backend
  (`ANIMA_USE_WAYLAND_NATIVE=1`) iterates outputs and creates one
  layer surface per monitor in `PerMonitor` mode.

### 1.5 Tests

- `monitor_mode_default_is_per_monitor`
- `entity_without_monitor_resolves_via_centroid`
- `entity_with_named_monitor_uses_that_one`
- `unknown_monitor_falls_back_to_primary`
- `span_mode_preserves_pre_0_3_behaviour`

---

## 2. Multi-window

### 2.1 Model

Independent overlay windows on the same monitor. Use case: one
window for chat-friendly characters (FollowCursor cat),
another for ambient ghost wandering on the same screen. Each
window has its own scene and its own settings panel.

### 2.2 Relationship with multi-monitor

Multi-window is **orthogonal** to multi-monitor. A user can have:

- 2 monitors × 1 window each (the `PerMonitor` baseline)
- 2 monitors × 2 windows on each (4 overlays total)
- 1 monitor × 3 windows (three scenes on one screen)

Window list lives in `AppConfig.windows: Vec<WindowConfig>`. The
default config carries one window (matches 0.2 behaviour).

### 2.3 `WindowConfig`

```toml
[[windows]]
id = "default"
name = "Main"
monitor_mode = "per_monitor"  # overrides global for this window

[[windows.characters]]
id = "ghost"
…
```

Each window's `characters` array is independent. The top-level
`characters` array remains as a fallback when `windows` is empty
(backwards compat).

### 2.4 Single-instance D-Bus

The `Activate()` D-Bus method currently raises the one overlay
window. With multi-window, it cycles through them in declaration
order. A future method `ActivateWindow(name)` would be richer, but
**adding it would extend the single-method invariant** (threat-model
§D-Bus single-instance handshake), so we skip it for 0.3 — the
tray menu offers per-window control instead.

### 2.5 Tray menu extension

The existing tray entries (Toggle visibility, Pause all, Quit)
remain global. New entries per window: `Windows ▸ Main ▸ {Show /
Hide / Edit}`. Implemented in `src/tray.rs` by reading
`config.windows` at construction time and emitting one submenu
per window.

### 2.6 Tests

- `default_config_has_one_window`
- `legacy_config_without_windows_loads_as_single_window`
- `activate_cycles_through_windows_in_order`

---

## 3. Asset library

### 3.1 Model

Today the only way to add an entity is drag-drop a file onto the
overlay. The asset library makes drop-targets discoverable: a
sidebar tab that lists everything in known asset directories with
thumbnails, tags, and a search bar.

### 3.2 Locations scanned

In order, until a path is found / created:

1. `$ANIMA_ASSETS_DIR` (env override, useful for testing)
2. `~/.local/share/animaEngine/assets/` (XDG_DATA_HOME default)
3. `assets/` next to the executable (development convenience)

Symlinks are followed but resolution depth is capped at 4 to
prevent loops. Files matching the drag-drop extension whitelist
(PNG / JPG / JPEG / GIF / WebP / MP4 / MOV / M4V) are indexed;
everything else is skipped silently.

### 3.3 Library index

```
~/.local/share/animaEngine/library.toml
```

Atomic-write managed (reusing `util::atomic_write_bytes`). Layout:

```toml
schema_version = 1

[[assets]]
id = "9c4f1a"           # short stable hash of canonical path
path = "ghost/idle.png"  # relative to assets root
kind = "png_sequence"
tags = ["mascot", "ghost"]
added_at = 2026-06-15T12:00:00Z
last_used_at = 2026-06-20T08:30:00Z

[[assets]]
…
```

`tags` are user-editable from the library UI. `added_at` /
`last_used_at` enable sorting by recency / frequency. The library
**caches no decoded frame data** — that stays in the existing
`~/.cache/animaEngine/` per-frame RGBA cache.

### 3.4 Thumbnails

64 × 64 PNG thumbnails generated lazily on first display in the
library UI. Cached at:

```
~/.cache/animaEngine/thumbs/<asset_id>.png
```

Cache invalidation: mtime of the source file > mtime of the thumbnail.
Atomic write applies.

### 3.5 Asset-library + drag-drop interaction

Pre-validation (`pre_validate_dropped_file`) still gates *what* a
file can become. The library cannot bypass it; the library's "Add
to scene" button routes through the same path with the resolved
asset's full path as input.

This preserves the audit invariant from L2 (asset_path entering
Scene must always be pre-validated). Library entries aren't
trusted just because they're in the index — re-validation happens
on every "Add to scene" click.

### 3.6 Tests

- `library_index_round_trip_through_toml`
- `library_index_atomic_write_uses_temp_sibling`
- `scan_skips_non_whitelisted_extensions`
- `scan_caps_symlink_depth_at_4`
- `thumbnail_cache_invalidates_on_source_mtime`
- `add_to_scene_routes_through_pre_validation`

---

## 4. Behavior expansion

Three new variants added to `Behavior`. Existing behaviors
(`Idle`, `WalkAround`, `FollowCursor`, `BoundedWander`) unchanged.

### 4.1 `Spinner` (deferred to 0.4)

Originally scoped for C.6. Implementing sprite rotation requires a
per-quad transform matrix in the vertex shader and a `rotation: f32`
field on `Entity`; both are clean changes but they cross the
behavior / renderer boundary cleanly enough that they deserve their
own sub-phase. Moved to 0.4 / Faza D where the AccessKit-runtime-
toggle and rebindable-keymap refactors also touch wider surfaces.
The shape stays as originally specified:

```rust
Behavior::Spinner { rps: f32 }
```

### 4.2 `Bounce`

```rust
Behavior::Bounce {
    amplitude_px: f32,    // peak displacement from rest position
    period_sec: f32,      // one full sine cycle
    axis: BounceAxis,     // X / Y / both (figure-8)
}
```

Oscillates around the entity's stored `(x, y)`. The stored
position is the rest position; bounce adds an offset frame-by-frame.
At `t = 0` the offset is zero, so `Bounce` composes cleanly with
hot-reload and drag — the user's manually placed position is
preserved.

Combinable mentally with gravity: a `Bounce` entity with
`physics_enabled = true` is undefined behaviour for 0.3; we keep
gravity as a hard override (gravity wins, bounce inactive).

### 4.3 `Reactive` (deferred to 0.4)

Originally scoped for C.6. Triggers require hooks in the click
dispatch and the mouse-enter detection that the X11 input shape
mechanism doesn't surface today; we'd be plumbing a "behavior gets
told about UI events" channel through the event loop. That's
exactly the kind of cross-cut better tackled with the multi-window
event-loop refactor in 0.4. Spec stays as originally written:

```rust
Behavior::Reactive {
    trigger: ReactiveTrigger,   // ClickOnSelf / CursorEnter / GlobalHotkey(...)
    effect: ReactiveEffect,     // Bounce(...) / Spinner(...) / SwapFrameRange(...)
    cooldown_sec: f32,
}
```

### 4.4 UI

Each new behavior gets a picker entry, an icon, and parameter
sliders following the existing pattern in
`src/ui/panels.rs::behavior_picker`. No new icon module work
needed beyond looking up Phosphor glyphs (suggestions:
`arrows-clockwise` for Spinner, `wave-sine` for Bounce,
`lightning` for Reactive).

### 4.5 Tests

- `spinner_rps_clamps_to_range`
- `bounce_offset_is_zero_at_t_zero`
- `bounce_with_gravity_is_overridden_by_gravity`
- `reactive_cooldown_blocks_retriggers`
- `swap_frame_range_returns_to_base_after_duration`

---

## 5. Animation curves

### 5.1 Model

Today FPS is a flat scalar — every frame holds for `1.0 / fps`
seconds. For ambient idle animations this is fine; for
hand-animated character sprites it looks robotic. 0.3 adds an
optional `easing` field on `Animation` that distorts the per-frame
hold time along a curve.

### 5.2 Curve set

```rust
#[derive(Serialize, Deserialize)]
pub enum EasingCurve {
    Linear,        // current behaviour, kept as default
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    Sine,
    BounceOut,     // overshoot then settle
}
```

All six are pure functions over `[0, 1]`. Implementations reuse
the helpers in `src/ui/anim.rs` (which becomes a more general
`src/anim.rs` since it's no longer UI-specific — relocate in C.7).

### 5.3 Per-frame timing

```rust
// at frame i of n, with base interval dt = 1.0 / fps:
let t = (i as f32) / (n as f32);     // 0..1
let eased = curve.apply(t);          // 0..1
let frame_dt = dt * curve_factor(i, n, curve);
```

`curve_factor` distorts the *interval* between frames while
preserving total animation duration — so an `EaseOut` curve makes
the first frames quick and the last frames slow without changing
the loop period.

### 5.4 Hot-reload

Changing `easing` at runtime takes effect on the next frame
boundary, not mid-frame, to avoid visible jumps. Implementation:
`Animation::tick` reads `self.curve` afresh on every advance,
no cached state.

### 5.5 UI

Five preset chips in the Animation collapsible section of the
Inspector (Linear / EaseIn / EaseOut / EaseInOut / Sine /
BounceOut). Plus a tiny preview rectangle that animates a single
test sprite at the chosen curve so the user can see the difference
before committing.

### 5.6 Tests

- `linear_curve_keeps_existing_behaviour`
- `easeout_quad_front_loads_intervals`
- `sine_curve_is_symmetric`
- `curve_changes_apply_at_next_frame_boundary`

---

## 6. Sprite groups

### 6.1 Model

A `Group` is a named container of `entity_id`s plus a group-level
transform. Moving the group moves every member by the same vector;
scaling the group multiplies every member's scale; toggling
visibility hides everyone. **Groups don't nest in 0.3** — flat
group → member relationship only. Nesting can land in 0.4 if it
proves needed.

### 6.2 `GroupConfig`

```toml
[[groups]]
id = "halloween_squad"
name = "Halloween Squad"
member_ids = ["ghost_1", "ghost_2", "ghost_3"]
offset_x = 0.0      # added to every member's x
offset_y = 0.0
scale = 1.0         # multiplies every member's scale
visible = true
```

Stored on `AppConfig.groups: Vec<GroupConfig>`. Empty for
single-entity scenes — backwards compatible.

### 6.3 Composition rules

Member transform = `(member.x + group.offset_x, member.y +
group.offset_y)`. Member scale = `member.scale * group.scale`.
Member visibility = `member.visible && group.visible`.

Behaviors compose normally on each member — a group of three
`WalkAround` ghosts still walks independently, but the group's
offset applies on top. Useful for parade-style movement.

### 6.4 UI

Scene tab gets a tree view: groups at top level, members nested
under each group. Drag-and-drop within the tree to reparent
(`member_ids` mutates). Group-level controls (offset / scale /
visibility) inline at the group header.

### 6.5 Tests

- `empty_groups_round_trip_through_toml`
- `member_offset_is_added_not_assigned`
- `group_visibility_anding_with_member_visibility`
- `removing_entity_removes_it_from_groups`
- `group_id_uniqueness_enforced_on_load`

---

## 7. Performance pass

### 7.1 Targets

Profile scenes at:

- 10 entities (baseline, 0.2 reality)
- 25 entities (modest power-user)
- 50 entities (preset stress test)
- 100 entities (synthetic max — half of MAX_ENTITIES)

At 60 fps target frame budget is 16.6 ms. Goal: 100 entities under
8 ms per frame, leaving room for compositor / shader work.

### 7.2 Likely hot spots (informed guesses)

- `Scene::visible_entities()` cache rebuild — already optimized in
  0.1; verify the invalidation rules still hold under
  multi-monitor / groups
- Per-entity texture upload — `GpuTexture` cache should hit ≥ 98 %
  during steady state; measure
- egui repaint cost when palette / tabs are idle — `set_opacity`
  + `animate_value_with_time` ride a fast path but worth verifying
- Hot-reload polling — fs::metadata every 2 s on the config file;
  if multi-window adds N config files, N grows; consider a single
  watcher thread

### 7.3 Tooling

- `tracing` spans on the render loop, gated behind a feature flag
  so the cost isn't paid in release
- Per-frame metrics overlay (lands in 0.4 / G.6); for 0.3 we lean
  on `tracing::Span::record` + offline log analysis
- `cargo-flamegraph` runs in the dev loop for local profiling;
  not a CI gate

### 7.4 Acceptance

Promote to "performance pass complete" when:

- 50-entity preset (Studio Session × multiple monitors) sustains
  60 fps on the dev box
- No regression vs 0.2 baseline on 10-entity scene
- New `bench/` directory carries criterion microbenchmarks for
  the three known hot paths (scene tick, frame composition,
  texture upload)

---

## 8. Implementation map

How this doc lands in code, sub-phase by sub-phase:

| Doc section | Lands in | Sub-phase |
|-------------|----------|-----------|
| §1 Multi-monitor | `src/renderer/`, `src/scene.rs`, `src/config.rs::GlobalConfig` | C.1 |
| §1 multi-monitor UI | `src/ui/panels.rs::scene_tab`, new `MonitorPicker` | C.2 |
| §2 Multi-window | `src/app.rs` (loop), `src/config.rs::AppConfig`, `src/tray.rs` | C.3 |
| §3 Asset library data | new `src/asset_library/` | C.4 |
| §3 Asset library UI | `src/ui/panels.rs::library_tab`, new tab variant | C.5 |
| §4 Behaviors | `src/behavior.rs`, `src/ui/panels.rs::behavior_picker` | C.6 |
| §5 Curves | `src/anim.rs` (relocated), `src/entity::Animation` | C.7 |
| §6 Groups | new `src/group.rs`, `src/config.rs`, `src/ui/panels.rs::scene_tab` | C.8 |
| §7 Performance | `bench/`, tracing spans, fixes per measurement | C.9 |

After C.9 this document is the canonical reference for engine
behaviour. Adding a new behavior or a new monitor mode means
amending this file first.
