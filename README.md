# 🎭 animaEngine

**Linux-first animated desktop overlay engine** — render multiple animated characters/sprites over your desktop using transparent, borderless, always-on-top windows with GPU acceleration.

Built in **Rust** with **wgpu** (Vulkan/OpenGL) for rendering and **winit** for windowing. No Electron. No Chromium. Minimal RAM footprint.

---

## ✨ Features

- 🖼️ **Transparent overlay** — borderless, always-on-top window
- 👆 **Click-through by default** — desktop is fully usable; characters float on top without blocking input
- ✏️ **Edit mode** — click the ⚙ toggle button to interact with characters (drag, select); click again to return to pass-through
- 🎮 **Multiple characters** — render several animated entities simultaneously
- 🎬 **PNG sequence animation** — load frames from a folder
- 🎞️ **GIF support** — animated GIF loading with per-frame delays
- 🌐 **WebP support** — animated and static WebP images
- 🎨 **Spritesheet support** — texture atlas with configurable rows/columns
- 🖱️ **Drag & drop** — click and drag characters to reposition them (in edit mode)
- 🎯 **Click-to-select** — click on characters to select them with visual highlight (in edit mode)
- ⏯️ **Play/pause** — global playback toggle (Space key)
- ⚙️ **Per-character config** — position, scale, opacity, FPS, visibility, z-index
- 💾 **Persistent config** — TOML configuration saved to `~/.config/animaEngine/config.toml`
- 🎨 **GPU-accelerated** — wgpu rendering with Vulkan/OpenGL backend, optimized vertex buffer reuse
- 📦 **Demo included** — starts with 2 cute demo characters (ghost + slime) generated procedurally

---

## 🚀 Quick Start

### Prerequisites

**Ubuntu/Debian:**
```bash
# System dependencies
sudo apt install -y \
  build-essential \
  cmake \
  libvulkan-dev \
  libx11-dev \
  libxcb1-dev \
  libxkbcommon-dev \
  libwayland-dev \
  libxrandr-dev

# Install Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Build & Run

```bash
cd animaEngine

# Build
cargo build

# Run
cargo run

# Run with debug logging
RUST_LOG=debug cargo run
```

### Controls

| Key/Action | Effect |
|-----------|--------|
| **⚙ button** (top-right) | Toggle edit mode ↔ pass-through mode |
| **Click + Drag** | Move a character *(edit mode only)* |
| **Click** | Select a character with highlight *(edit mode only)* |
| **Escape** | Exit edit mode → pass-through |
| **Space** | Toggle play/pause (edit mode) |
| **S** | Save config (edit mode) |
| **Q** | Save and exit (edit mode) |

> **Default behavior:** The overlay starts in **pass-through mode** — all clicks go through to the desktop. Click the **⚙ button** in the top-right corner to enter edit mode when you want to move characters. Selected entities are highlighted with a cyan glow border.

---

## ⚙️ Configuration

Config is stored at `~/.config/animaEngine/config.toml` and auto-created on first run.

### Example Config

```toml
[global]
always_on_top = true
transparent = true
playback_enabled = true
window_width = 0    # 0 = auto-detect from monitor
window_height = 0   # 0 = auto-detect from monitor

[[characters]]
id = "ghost"
name = "Ghost Demo"
asset_type = "png_sequence"
asset_path = "assets/demo/ghost"
x = 200.0
y = 300.0
scale = 1.0
opacity = 0.9
fps = 10.0
visible = true
playing = true
z_index = 10

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
```

### Adding a New Character

1. Create a folder with your PNG frames:
   ```
   assets/my_character/
     frame_001.png
     frame_002.png
     frame_003.png
   ```

2. Add an entry to `~/.config/animaEngine/config.toml`:
   ```toml
   [[characters]]
   id = "my_character"
   name = "My Character"
   asset_type = "png_sequence"
   asset_path = "assets/my_character"
   x = 400.0
   y = 200.0
   scale = 1.0
   opacity = 1.0
   fps = 12.0
   visible = true
   playing = true
   z_index = 30
   ```

3. Restart the application.

### Asset Types

| Type | Value | Description |
|------|-------|-------------|
| PNG Sequence | `png_sequence` | Folder of sorted PNG files (frame_001.png, frame_002.png, ...) |
| Static PNG | `png_static` | Single PNG file |
| GIF | `gif` | Animated GIF file (supports per-frame delays) |
| WebP (animated) | `webp_animated` | Animated WebP file |
| WebP (static) | `webp_static` | Static WebP image |
| Spritesheet | `spritesheet` | Texture atlas with `spritesheet_columns` and `spritesheet_rows` |

---

## 🏗️ Architecture

```
src/
├── main.rs              # Entry point, logging, demo asset generation
├── app.rs               # winit ApplicationHandler — event loop, input handling
├── config.rs            # TOML config with serde serialization
├── scene.rs             # Scene: collection of entities, global controls
├── entity.rs            # Entity: animated character with transform
├── animation/
│   ├── mod.rs           # Animation state: frame cycling, FPS, play/pause
│   ├── frame.rs         # Frame: raw RGBA pixel data
│   ├── loader.rs        # Asset type router + fallback generator
│   ├── png_sequence.rs  # PNG directory loader
│   ├── gif_loader.rs    # GIF frame decoder (per-frame delays)
│   ├── webp_loader.rs   # WebP animated/static loader
│   └── spritesheet.rs   # Spritesheet grid slicer
├── renderer/
│   ├── mod.rs           # Module exports
│   ├── wgpu_renderer.rs # wgpu device, pipeline, optimized batch rendering
│   ├── texture.rs       # GPU texture management
│   └── sprite.rs        # Vertex data, quad generation, projection
├── window/
│   ├── mod.rs
│   ├── platform.rs      # X11/Wayland detection
│   ├── linux.rs         # Compositor detection
│   └── x11_input.rs     # X11 Input Shape (click-through) + connection pooling
├── input/
│   ├── mod.rs
│   ├── drag.rs          # Mouse drag state machine
│   └── selection.rs     # Click-to-select entity
└── shaders/
    └── sprite.wgsl      # WGSL vertex + fragment shader
```

---

## 🐧 Platform Notes

### X11 (Recommended)

Full support for:
- ✅ Transparent window
- ✅ Borderless/undecorated
- ✅ Always-on-top (DOCK window type)
- ✅ Click-through with toggle button (X11 Input Shape)
- ✅ Mouse drag
- ✅ Auto-detect monitor resolution

### Wayland (via XWayland)

The application forces X11 backend on Wayland systems (via XWayland), which is available on virtually all Wayland-based desktops. This ensures:
- ✅ All X11 features work reliably
- ✅ No compositor-specific limitations
- ⚠️ Minor latency overhead from XWayland translation

---

## 🔧 Troubleshooting

### Window is not transparent / has black background

1. **Check compositor**: Transparency requires a running compositor
   - GNOME/Mutter: compositing is built-in ✅
   - KDE/KWin: compositing is built-in ✅
   - Bare X11: install and run `picom` or `compton`

2. **Check GPU drivers**: wgpu requires Vulkan or OpenGL support
   ```bash
   vulkaninfo | head -5
   ```

3. **Force OpenGL backend** (if Vulkan has issues):
   ```bash
   WGPU_BACKEND=gl cargo run
   ```

### Always-on-top not working

- Some window managers don't respect `_NET_WM_STATE_ABOVE`
- Try a different WM or use X11 mode

### Missing Vulkan

```bash
# Install Vulkan drivers (NVIDIA)
sudo apt install nvidia-driver-XXX

# Install Vulkan drivers (AMD)
sudo apt install mesa-vulkan-drivers

# Install Vulkan drivers (Intel)
sudo apt install mesa-vulkan-drivers intel-media-va-driver
```

### Asset paths not found

- Paths in config are relative to the working directory
- Run `cargo run` from the project root
- Or use absolute paths in config

---

## 🗺️ Roadmap

### Completed
- [x] Click-through mode (X11 Input Shape)
- [x] Sprite sheet support (texture atlas)
- [x] WebP support (animated + static)
- [x] Visual selection highlight
- [x] Optimized GPU rendering (vertex buffer reuse)
- [x] Auto-detect monitor resolution
- [x] X11 connection pooling

### Next Steps
- [ ] System tray icon with controls
- [ ] Right-click context menu per character
- [ ] Per-character play/pause toggle
- [ ] Hot-reload config on file change

### Future Vision
- [ ] Visual editor for character placement
- [ ] Asset pack system (.animapack)
- [ ] Physics-based idle animations
- [ ] Audio-reactive animations
- [ ] AI background remover for sprites
- [ ] AppImage/deb packaging
- [ ] Full Wayland support (layer-shell protocol)
- [ ] Windows/macOS support
- [ ] MP4/video overlay support

---

## 📝 License

MIT

---

## 🤝 Contributing

Contributions welcome! Key areas:
- Wayland layer-shell integration
- Better animation formats (APNG, Lottie)
- UI/settings panel (egui integration)
- Performance optimizations
- Asset pack management
