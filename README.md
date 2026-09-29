# animaEngine

[![CI](https://github.com/Alexandru2984/animaEngine/actions/workflows/ci.yml/badge.svg)](https://github.com/Alexandru2984/animaEngine/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Alexandru2984/animaEngine)](https://github.com/Alexandru2984/animaEngine/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Linux-first animated desktop overlay engine. Render multiple animated
characters or sprites on top of your desktop using transparent,
always-on-top windows with GPU acceleration. Built in Rust with **wgpu**
+ **winit** + **egui** — no Electron, no Chromium, minimal RAM use.

<!-- TODO (X.3, against the RC build): demo GIF, top-fold. Record a
     pass-through → edit-mode → drop-an-asset loop and drop it in:
     ![animaEngine in action](docs/media/demo.gif) -->

> Status: 1.4.0 — stable. Production-ready packaging
> (`.deb` / AppImage / Flatpak), stable X11/XWayland backend, opt-in
> native Wayland backend (wlroots), ten UI locales, multi-monitor
> distribution, asset library, sprite groups. See
> [Architecture](docs/architecture.md) for a deeper map, the
> [stability policy](docs/stability-policy.md) for what stays put across
> releases, and [CONTRIBUTING.md](CONTRIBUTING.md) for how to hack on it.

## What it does

- **Drop any image or short MP4** onto the overlay → it becomes an
  animated character. PNG / GIF / WebP / JPEG / MP4 (H.264). Or pick it
  with **Add file…** in the Scene tab, through the desktop's file chooser.
- **Click-through by default**: clicks pass straight to your desktop;
  the only widget that catches input in pass-through mode is the ⚙
  toggle button in the top-right.
- **Edit mode** exposes a tabbed settings panel (Inspector / Scene /
  Library / Appearance / Keybindings), right-click context menus, collapsible
  inspector sections, sliders for every field, drag-and-drop placement.
- **Autonomous behaviors** per entity: `Idle` (default), `WalkAround`,
  `FollowCursor`, `BoundedWander`, `Bounce` — wired through the UI.
- **Scripted behaviors**: a character's motion can be a small
  [Rhai](https://rhai.rs) script from your asset library, sandboxed and
  bounded (no files, no network, capped operations). Scripts can play
  **sounds**, panned to where the character is on screen, and react to
  **how busy the machine is** — overall CPU and memory load only, never
  what is running. Examples in
  [docs/examples/behaviors/](docs/examples/behaviors/).
- **Animation curves**: six easing options (Linear / Ease in / Ease out
  / Ease in-out / Sine / Bounce out) distort per-frame timing while
  preserving the loop's total duration.
- **Multi-monitor distribution**: `PerMonitor` (default) / `Span` /
  `Single { name }`, plus a per-entity pin and `Ctrl+M` to cycle the
  selected entity through monitors. Characters on every monitor can be
  selected and dragged, across monitors too.
- **Asset library**: scans `~/.local/share/animaengine/assets/` (or
  `$ANIMA_ASSETS_DIR`) and surfaces a search-filtered grid; "Add to
  scene" routes through the same drag-drop validation path so asset
  caps and the extension whitelist still apply.
- **Sprite groups**: `Ctrl+G` binds the selected characters into one
  named group that a click selects and a drag moves as one; the Scene
  tab renames, hides and dissolves groups. A hidden group hides every
  member and blocks their click hit-test.
- **Themes**: Dark and Light plus high-contrast siblings for both,
  switchable instantly without restart. HC variants clear WCAG AAA.
- **Bundled presets**: six curated one-click scenes (Cozy Companion,
  Productivity Zen, Halloween Party, Birthday Confetti, Studio
  Session, Cursor Follower) — Append or Replace.
- **Command palette** (`Ctrl+K`): search every action, theme and
  preset, and run it in one keystroke.
- **Several at once**: Shift+click or drag a rectangle to select several
  characters, then move, delete, duplicate or change them together —
  or line them up and space them evenly. Drags snap to screen edges and
  to other characters (hold Alt to place freely).
- **Undo / redo** (`Ctrl+Z` / `Ctrl+Shift+Z`): every edit in edit mode —
  a move, a delete, a changed property, a preset's Replace.
- **Steps aside for full-screen apps**: the characters hide (or pause)
  while a game, a video or a presentation fills the screen — and hold
  still while you are away, or on battery if you like.
- **Ten UI languages**: English, Română, Español, Deutsch, Français,
  Italiano, Português (BR), Polski, Nederlands, 日本語 — auto-detected
  from `LANG`, switchable in Appearance.
- **Rebindable shortcuts** that work on any keyboard layout — AZERTY,
  QWERTZ, Cyrillic — and are shown the way your keyboard labels them.
- **System integration**: tray icon (StatusNotifierItem),
  `Ctrl+Shift+A/H/P` global hotkeys, single-instance D-Bus handshake.
- **Hot-reload**: edit `~/.config/animaengine/config.toml` while the app
  runs; changes are decoded off the UI thread and applied seamlessly.
- **Accessibility**: screen readers (Orca and others, over AT-SPI) on
  both backends, visible focus rings, full keyboard reference table. See
  [docs/accessibility.md](docs/accessibility.md).

## Install

### Pre-built packages

If you have one of the artifacts under [releases](
https://github.com/Alexandru2984/animaEngine/releases) (or built locally
via `make appimage` / `make deb` / `make flatpak`):

```bash
# Debian / Ubuntu (.deb)
sudo apt install ./anima-engine_1.4.0-1_amd64.deb

# AppImage (any distro)
chmod +x animaEngine-1.4.0-x86_64.AppImage
./animaEngine-1.4.0-x86_64.AppImage

# Flatpak
flatpak install --user com.animaengine.Anima.flatpak
flatpak run com.animaengine.Anima
```

### From source

```bash
# System deps (Ubuntu/Debian) — pkg-config is required and is NOT
# part of build-essential; without it the xkbcommon build script fails.
sudo apt install -y build-essential cmake pkg-config \
    libvulkan-dev libx11-dev libxcb1-dev libxkbcommon-dev \
    libxkbcommon-x11-dev libwayland-dev libxrandr-dev

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
| Set behavior | Dropdown in the Inspector (Idle / Walk / Follow / Bounded / Bounce / Script) |
| Delete | `Delete`, right-click → Delete, or the trash button in the Scene list |
| Group / ungroup | `Ctrl+G` / `Ctrl+Shift+G` on the selection, or right-click → Group |
| Hide overlay | `Ctrl+Shift+H` (global) or tray menu |
| Pause animations | `Space` (edit mode), `Ctrl+Shift+P` (global), or tray |
| Save & quit | `Q` (edit mode), tray → Quit, or close the window |

Full keyboard reference: press `H` in edit mode.

## Configuration

A TOML config lives at `~/.config/animaengine/config.toml`. Hand-editing
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

See [docs/config.md](docs/config.md) for every field. What stays stable
across releases — config, D-Bus, CLI, asset formats, file locations — is
the [stability policy](docs/stability-policy.md).

## Supported assets

| Type | Extensions | Notes |
|------|-----------|-------|
| Static image | `.png`, `.jpg`, `.jpeg` | Single frame |
| Animated GIF | `.gif` | Per-frame delays preserved |
| Animated WebP | `.webp` | Animated and static |
| PNG sequence | folder of `frame_*.png` | Decoded in parallel (rayon) |
| Spritesheet | `.png` + `columns` × `rows` | Grid auto-sliced |
| Video | `.mp4`, `.m4v`, `.mov` | H.264 only, audio ignored, capped at ~20 s |

Decoded RGBA frames are cached on disk under `~/.cache/animaengine/`
so subsequent starts are limited by disk read speed. Set
`ANIMA_NO_CACHE=1` to skip both reads and writes.

## Wayland

The default code path uses **winit + X11** (XWayland on Wayland
sessions) — stable, supports every Linux desktop. An **opt-in native
Wayland backend** built on `wlr-layer-shell-unstable-v1` adds true
overlay support for wlroots compositors (sway / Hyprland / river /
Wayfire). It reached feature parity over the 0.5 *E* phases — keyboard
(xkbcommon), pointer, egui and drag-drop all go through the same
validation as X11 — but it stays **opt-in and wlroots-only**, not the
default:

```bash
ANIMA_USE_WAYLAND_NATIVE=1 anima-engine
```

GNOME and KDE Wayland sessions do not implement layer-shell, so the
overlay falls back to the XWayland code path automatically — no opt-in
flag needed there.

### Feature matrix

| Feature | X11 / XWayland (default) | Wayland native (opt-in, wlroots) |
|---------|-------------------------|------------------------------|
| Click-through overlay | stable | stable (wlroots) |
| Tray icon | stable | stable |
| Drag-and-drop assets | stable | stable (in pass-through, drag over the ⚙ corner first) |
| Keyboard input | stable (winit) | stable (sctk + xkbcommon) |
| Input methods (CJK) | stable (winit) | stable (text-input-v3) |
| egui settings panel | stable | stable |
| Multi-monitor distribution | stable | stable |
| Perf overlay | stable | stable |
| Global hotkeys | stable (XGrabKey) | via D-Bus + compositor binding |
| Hot-reload | stable | stable |

Global shortcuts on the native path go through `org.animaengine.Anima`
D-Bus methods that compositor bindings call via `gdbus` — see
[docs/wayland.md](docs/wayland.md) for sway / Hyprland / river config
snippets.

## Security & trust

Single-user desktop overlay, designed to run as your unprivileged user.
Asset loaders enforce frame / dimension / byte caps so a malicious file
can't OOM you; config + cache writes are atomic so a crash can't
corrupt either. Full invariants in [docs/threat-model.md](
docs/threat-model.md). Zero network calls. Don't run it as root.

## License

MIT. See [LICENSE](LICENSE).
