# Configuration reference

The config file lives at `~/.config/animaEngine/config.toml`. It's
auto-created on first run and hot-reloaded every 2 seconds. Decoding
runs on a worker thread, so even large GIF changes don't freeze the UI.

Almost everything here is also reachable from the in-app UI, which
writes the same file. A handful of sections (`[collapse_state]`,
`onboarding`, `last_seen_whats_new`) are **app-managed** UI bookkeeping
— they round-trip safely but aren't meant for hand-editing; they're
called out where they appear.

## Schema version

```toml
version = 2                 # config schema version (0.9+)
```

A top-level `version` key records the config schema generation. Files
written before 0.9 have no `version` key and are treated as schema v1;
on load they're migrated to the current version and re-saved. **Before
any migration runs, the original is copied to
`config.toml.bak-v<n>`** — a migration bug can never be the reason you
lose a config. A malformed `version` (not a positive integer) is
treated as current and skips migration rather than risk running the
wrong one.

Sections this build doesn't recognise (e.g. written by a newer
animaEngine) are preserved verbatim through a load → save cycle rather
than dropped, so downgrading doesn't silently strip newer settings.
(This applies to whole `[section]` tables; unknown bare keys at the
very top of the file are not preserved.)

## `[global]`

```toml
[global]
always_on_top = true
transparent = true
playback_enabled = true     # Space key inverts this
window_width = 0            # 0 = autodetect from primary monitor
window_height = 0           # 0 = autodetect from primary monitor
theme = "dark"              # dark | light | dark_high_contrast |
                            # light_high_contrast
locale = "ro"              # language code (en, ro, pt-BR, …); omit
                            # the key entirely to detect from the OS
monitor_mode = "per_monitor" # per_monitor | span | single
accesskit_enabled = true    # AT-SPI / screen-reader bridge (Orca)
hotkey_backend = "auto"     # auto | portal | x11 | none
reduced_motion = false      # a11y: skip UI transitions + decorative
                            # entity bobbing; state-conveying
                            # animations still play
window_awareness = false    # X11 only: physics characters land on
                            # window top edges and walk along them.
                            # No effect on Wayland (the protocol
                            # exposes no global window geometry).
```

| Key | Values | Notes |
|-----|--------|-------|
| `theme` | `dark` (default), `light`, `dark_high_contrast`, `light_high_contrast` | |
| `locale` | any shipped language code | Omit to auto-detect from the environment; an unknown code falls back to English. |
| `monitor_mode` | `per_monitor` (default), `span`, `single` | `single` pins the overlay to one output — see below. |
| `accesskit_enabled` | `true` (default) / `false` | Toggling applies live; off trims the AT-SPI registration. |
| `hotkey_backend` | `auto` (default), `portal`, `x11`, `none` | `auto` probes the GlobalShortcuts portal, then XGrabKey on X11. `none` = tray + D-Bus only. |

Pinning the overlay to a single output uses the struct form:

```toml
[global.monitor_mode.single]
name = "eDP-1"              # connector name from `xrandr` / `wlr-randr`
```

Two more keys are written here by the app and are not meant for hand
editing: `onboarding` (which first-run hints you've dismissed) and
`last_seen_whats_new` (the last release whose "What's new" panel you
saw). Both are safe to delete — the app reseeds them.

## `[[characters]]`

Each `[[characters]]` block is one entity on screen. Add as many as you
need (capped at 64 to prevent runaway).

```toml
[[characters]]
id = "slime"                # Stable identifier; used for PRNG seeding
name = "Slime Demo"         # Shown in the inspector
asset_type = "png_sequence" # See "Asset types" below
asset_path = "assets/demo/slime"
x = 600.0                   # Top-left position, pixels
y = 400.0
scale = 1.0                 # 0.1..=5.0
opacity = 1.0               # 0.0..=1.0
fps = 8.0                   # Animation rate (ignored for GIF/WebP
                            # frames that carry their own delays)
visible = true
playing = true
z_index = 20                # Higher = on top
physics_enabled = false     # G key toggles at runtime
monitor = "eDP-1"          # Optional: pin this entity to one output.
                            # Omit to resolve by position against the
                            # live monitor topology. A stale name falls
                            # back to that resolution with a warning.
easing = "ease_out_quad"   # Optional per-frame timing curve. Omit for
                            # linear. Ignored when the asset carries its
                            # own GIF/WebP frame delays.
```

`easing` accepts `linear` (default), `ease_in_quad`, `ease_out_quad`,
`ease_in_out_quad`, `sine`, `bounce_out`.

### Asset types

| `asset_type` value | What it expects |
|--------------------|-----------------|
| `png_static`       | single PNG / JPEG file |
| `png_sequence`     | folder of `frame_*.png` |
| `gif`              | animated GIF |
| `webp_animated`    | animated WebP |
| `webp_static`      | static WebP |
| `spritesheet`      | single PNG + `spritesheet_columns` / `_rows` keys |
| `video`            | MP4 / M4V / MOV with H.264 video (audio dropped) |

Asset paths are tried relative to the binary, then to the working
directory. Use absolute paths to avoid ambiguity. `~/` expands to your
home directory; `~user` is **not** expanded (it passes through as a
literal rather than risk building a path inside the wrong home).

### Spritesheet example

```toml
[[characters]]
id = "knight"
name = "Knight"
asset_type = "spritesheet"
asset_path = "assets/knight.png"
spritesheet_columns = 8
spritesheet_rows = 2
# … other fields …
```

### Per-state animation sets

A character can carry distinct assets for distinct states (idle, walk,
fall, drag) — this is what the Shimeji importer produces. Each state is
its own sub-table:

```toml
[[characters]]
id = "shime"
name = "Shimeji"
asset_type = "png_sequence"   # this pair always defines the `idle` state
asset_path = "imported/shime/idle"
x = 400.0
y = 300.0
fps = 12.0

[characters.animations.walk]
asset_type = "png_sequence"
asset_path = "imported/shime/walk"
fps = 10.0                    # optional; inherits the character `fps`

[characters.animations.fall]
asset_type = "gif"
asset_path = "imported/shime/fall.gif"

[characters.animations.drag]
asset_type = "png_sequence"
asset_path = "imported/shime/drag"
```

State keys are `idle`, `walk`, `fall`, `drag`. The top-level
`asset_type` / `asset_path` **always** define `idle`, so an explicit
`[characters.animations.idle]` is ignored with a warning (one
unambiguous source). A misspelled state name is a hard parse error, not
a silent drop. Each state table mirrors the character's asset fields
(`asset_type`, `asset_path`, optional `fps`, `spritesheet_columns`,
`spritesheet_rows`); anything omitted inherits from the character.
Characters that never use states omit the block entirely and round-trip
byte-identically with pre-0.7 configs.

### Behavior

Optional autonomous motion. Skipped on serialize when the entity is
`Idle` (the default), so plain configs stay compact. One `[characters.
behavior]` table per character, tagged by `type`:

```toml
# Stays where placed (default — never written to disk).
# type = "idle"

# Walks left/right with edge bounce.
[characters.behavior]
type = "walk_around"
speed = 80.0                # pixels per second

# Chases the cursor with ease-in; stops at comfort_distance.
[characters.behavior]
type = "follow_cursor"
speed = 240.0
comfort_distance = 80.0

# Random walk inside a user-defined box.
[characters.behavior]
type = "bounded_wander"
x_min = 200.0
x_max = 1200.0
y_min = 700.0
y_max = 800.0
speed = 120.0

# Sinusoidal bob around the rest position. Gravity overrides it.
[characters.behavior]
type = "bounce"
amplitude_px = 24.0         # peak displacement, applied symmetrically
period_sec = 1.5            # one full cycle; clamped to >= 0.05
axis = "vertical"           # vertical (default) | horizontal | both
                            # ("both" = 90°-offset circular motion)
```

## `[[windows]]`

Optional multi-window roster. When absent (or empty), the top-level
`characters` array is the single legacy overlay — every pre-0.3 config
behaves exactly as before. When you declare windows, each gets its own
character set and an optional per-window monitor distribution:

```toml
[[windows]]
id = "main"                 # stable id (lowercase-kebab by convention)
name = "Main"               # shown in tray menus and the window picker
# monitor_mode omitted → inherits [global].monitor_mode

[[windows.characters]]
id = "ghost"
name = "Ghost"
asset_type = "png_sequence"
asset_path = "assets/demo/ghost"
x = 200.0
y = 300.0

[[windows]]
id = "side"
name = "Companion"
monitor_mode = "single"     # this window only; struct form for a pin:
# [windows.monitor_mode.single]
# name = "HDMI-A-1"
```

If both top-level `characters` and explicit `windows` are present, the
explicit windows win and the top-level array is ignored (your
deliberate distribution is preserved, nothing is silently merged).

## `[[groups]]`

Sprite groups bind several entities so they move, scale, and hide
together. Membership is by entity `id`:

```toml
[[groups]]
id = "duo"
name = "Cat & Mouse"
member_ids = ["cat", "mouse"]
offset_x = 0.0              # pixels added to every member's x
offset_y = 0.0
scale = 1.0                # multiplier on every member's scale
visible = true             # false hides all members regardless of
                           # their own `visible` flag
```

`offset_*`, `scale`, and `visible` are composed onto each member at
render time. If an entity belongs to more than one group, the first
group that lists it wins.

## `[keybindings.map]`

Every shortcut is rebindable. Missing entries fall back to the built-in
defaults at lookup time, so you only list the ones you override and a
new release's new action can't be silently disabled for you. Keys are
snake_case action names; values are lists of chord strings (an action
can have several):

```toml
[keybindings.map]
toggle_edit_mode = ["F2"]
open_command_palette = ["Ctrl+Shift+P"]
toggle_perf_overlay = ["Ctrl+Shift+`"]
quit_with_save = ["Ctrl+Q"]
nudge_up = ["Up", "K"]
```

Chord syntax is `Modifier+Modifier+Key` (`Ctrl`, `Shift`, `Alt`,
`Super`). The Settings → Keybindings UI is the easiest way to discover
action names and rebind without typos; it also flags conflicts. A
chord bound to two actions is reported there rather than silently
shadowed.

## Environment variables

| Variable | Effect |
|----------|--------|
| `RUST_LOG=anima_engine=debug` | Verbose logs (default is `info`) |
| `ANIMA_NO_CACHE=1` | Bypass the on-disk RGBA cache (and its startup sweep) |
| `ANIMA_USE_WAYLAND_NATIVE=1` | Try the native wlr-layer-shell path |
| `ANIMA_ASSETS_DIR=<path>` | Override the asset-library root (default `~/.local/share/animaEngine/assets/`) |
| `ANIMA_MEMORY_BUDGET_MB=<int>` | Raise the aggregate decoded-RGBA budget (default 1024) for high-RAM machines |

Two more are read only by the soak-test harness (`scripts/soak.sh`),
not in normal use: `ANIMA_SOAK_METRICS=<path>` enables one CSV metrics
row per interval, and `ANIMA_SOAK_INTERVAL_SECS=<int>` sets that
interval (default 60). See [soak-testing.md](soak-testing.md).

## Limits

| Knob | Default | Why |
|------|---------|-----|
| `MAX_ENTITIES` | 64 | Caps characters; prevents runaway from a malicious / runaway config |
| `MAX_IMAGE_DIM` | 4096 | Decompression-bomb guard at the header level |
| `MAX_DROP_SIZE` | 256 | Auto-resize dropped assets to overlay-friendly dims |
| `MAX_VIDEO_FRAMES` | 600 | ~20 s at 30 fps; protects RAM on long videos |
| `MAX_ANIMATION_FRAMES` | 600 | Same cap for GIF / WebP / PNG sequences |
| `MAX_SEQUENCE_FILES` | 1000 | Upper bound on files scanned for a `png_sequence` |
| Aggregate decoded budget | 1024 MB | Sum of all decoded RGBA; raise with `ANIMA_MEMORY_BUDGET_MB` |

These are compile-time constants (`src/constants.rs`) except the
aggregate budget, which is the one runtime-tunable limit.
