# animaEngine

Linux-first animated desktop overlay engine. Render multiple animated
characters or sprites on top of your desktop using transparent,
always-on-top windows with GPU acceleration. Built in Rust with **wgpu**
+ **winit** + **egui** — no Electron, no Chromium, minimal RAM use.

> Status: 0.1 — production-ready packaging (`.deb` / AppImage / Flatpak),
> X11 + opt-in native Wayland. See [Architecture](docs/architecture.md)
> for a deeper map; [CONTRIBUTING.md](CONTRIBUTING.md) for how to hack on it.

## What it does

- **Drop any image or short MP4** onto the overlay → it becomes an
  animated character. PNG / GIF / WebP / JPEG / MP4 (H.264).
- **Click-through by default**: clicks pass straight to your desktop;
  the only widget that catches input in pass-through mode is the ⚙
  toggle button in the top-right.
- **Edit mode** exposes a settings panel (egui), right-click context
  menus, scene list, sliders for every field, drag-and-drop placement.
- **Autonomous behaviors** per entity: `Idle` (default), `WalkAround`,
  `FollowCursor`, `BoundedWander` — wired through the UI.
- **System integration**: tray icon (StatusNotifierItem),
  `Ctrl+Shift+A/H/P` global hotkeys, single-instance D-Bus handshake.
- **Hot-reload**: edit `~/.config/animaEngine/config.toml` while the app
  runs; changes are decoded off the UI thread and applied seamlessly.

## Install

### Pre-built packages

If you have one of the artifacts under [releases](
https://github.com/Alexandru2984/animaEngine/releases) (or built locally
via `make appimage` / `make deb` / `make flatpak`):

```bash
# Debian / Ubuntu (.deb)
sudo apt install ./anima-engine_0.1.0-1_amd64.deb

# AppImage (any distro)
chmod +x animaEngine-0.1.0-x86_64.AppImage
./animaEngine-0.1.0-x86_64.AppImage

# Flatpak
flatpak install --user com.animaengine.Anima.flatpak
flatpak run com.animaengine.Anima
```

### From source

```bash
# System deps (Ubuntu/Debian)
sudo apt install -y build-essential cmake \
    libvulkan-dev libx11-dev libxcb1-dev libxkbcommon-dev \
    libwayland-dev libxrandr-dev

# Build & run
cargo build --release
./target/release/anima_engine
```

## Daily use

| Action | How |
|--------|-----|
| Enter edit mode | Click ⚙ (top-right), or `Ctrl+Shift+A` from anywhere |
| Add a character | Drag a PNG / GIF / WebP / JPEG / MP4 onto the overlay |
| Move a character | Drag it (edit mode) or use the X/Y sliders |
| Adjust scale / opacity / FPS | Sliders in the settings panel |
| Toggle visibility / playback | `V` / `P` keys, or checkboxes |
| Set behavior | Dropdown in panel (Idle / Walk / Follow / Bounded) |
| Delete | `Delete`, right-click → Delete, or the `×` button in the list |
| Hide overlay | `Ctrl+Shift+H` (global) or tray menu |
| Pause animations | `Space` (edit mode), `Ctrl+Shift+P` (global), or tray |
| Save & quit | `Q` (edit mode), tray → Quit, or close the window |

Full keyboard reference: press `H` in edit mode.

## Configuration

A TOML config lives at `~/.config/animaEngine/config.toml`. Hand-editing
works — the app polls every 2 s and reloads off-thread.

```toml
[global]
always_on_top = true
transparent = true
playback_enabled = true
window_width = 0    # 0 = auto from monitor
window_height = 0

[[characters]]
id = "slime"
name = "Slime Demo"
asset_type = "png_sequence"
asset_path = "assets/demo/slime"
x = 600.0
y = 400.0
scale = 1.0
opacity = 1.0
fps = 8.0
visible = true
playing = true
z_index = 20
physics_enabled = false      # G key in edit mode

[characters.behavior]
type = "walk_around"
speed = 80.0
```

See [docs/config.md](docs/config.md) for every field.

## Supported assets

| Type | Extensions | Notes |
|------|-----------|-------|
| Static image | `.png`, `.jpg`, `.jpeg` | Single frame |
| Animated GIF | `.gif` | Per-frame delays preserved |
| Animated WebP | `.webp` | Animated and static |
| PNG sequence | folder of `frame_*.png` | Decoded in parallel (rayon) |
| Spritesheet | `.png` + `columns` × `rows` | Grid auto-sliced |
| Video | `.mp4`, `.m4v`, `.mov` | H.264 only, audio ignored, capped at ~20 s |

Decoded RGBA frames are cached on disk under `~/.cache/animaEngine/`
so subsequent starts are limited by disk read speed. Set
`ANIMA_NO_CACHE=1` to skip both reads and writes.

## Wayland

The default code path uses **winit + X11** (XWayland on Wayland systems)
— stable, supports every Linux desktop. An **opt-in native Wayland
backend** with `wlr-layer-shell-unstable-v1` exists for wlroots
compositors (sway / Hyprland / river):

```bash
ANIMA_USE_WAYLAND_NATIVE=1 anima-engine
```

The native path currently renders sprites only — no egui UI, keyboard,
or drag-drop yet. Use the tray + `Ctrl+Shift+A` to control it. See
[docs/architecture.md](docs/architecture.md#native-wayland-status) for
the full status matrix.

## Security & trust

Single-user desktop overlay, designed to run as your unprivileged user.
Asset loaders enforce frame / dimension / byte caps so a malicious file
can't OOM you; config + cache writes are atomic so a crash can't
corrupt either. Full invariants in [docs/threat-model.md](
docs/threat-model.md). Zero network calls. Don't run it as root.

## License

MIT. See [LICENSE](LICENSE).
