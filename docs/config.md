# Configuration reference

The config file lives at `~/.config/animaEngine/config.toml`. It's
auto-created on first run and hot-reloaded every 2 seconds. Decoding
runs on a worker thread, so even large GIF changes don't freeze the UI.

## `[global]`

```toml
[global]
always_on_top = true
transparent = true
playback_enabled = true     # Space key inverts this
window_width = 0            # 0 = autodetect from primary monitor
window_height = 0           # 0 = autodetect from primary monitor
window_awareness = false    # X11 only: physics characters land on
                            # window top edges and walk along them.
                            # No effect on Wayland (the protocol
                            # exposes no global window geometry).
```

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
```

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
directory. Use absolute paths to avoid ambiguity.

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

### Behavior

Optional. Skipped on serialize when the entity is `Idle`, so plain
configs stay compact.

```toml
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
```

## Environment variables

| Variable | Effect |
|----------|--------|
| `RUST_LOG=anima_engine=debug` | Verbose logs (default is `info`) |
| `ANIMA_NO_CACHE=1` | Bypass the on-disk RGBA cache |
| `ANIMA_USE_WAYLAND_NATIVE=1` | Try the native wlr-layer-shell path |

## Limits

| Knob | Default | Why |
|------|---------|-----|
| `MAX_ENTITIES` | 64 | Prevents runaway from a malicious / runaway config |
| `MAX_IMAGE_DIM` | 4096 | Decompression-bomb guard at the header level |
| `MAX_DROP_SIZE` | 256 | Auto-resize dropped assets to overlay-friendly dims |
| `MAX_VIDEO_FRAMES` | 600 | ~20 s at 30 fps; protects RAM on long videos |
