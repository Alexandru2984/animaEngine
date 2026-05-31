# 🎭 animaEngine

**Linux-first animated desktop overlay engine** — render multiple animated characters/sprites over your desktop using transparent, borderless, always-on-top windows with GPU acceleration.

Built in **Rust** with **wgpu** (Vulkan/OpenGL) for rendering and **winit** for windowing. No Electron. No Chromium. Minimal RAM footprint.

---

## ✨ Features

- 🖼️ **Transparent overlay** — borderless, always-on-top window
- 👆 **Click-through by default** — desktop is fully usable; characters float on top without blocking input
- ✏️ **Edit mode** — click the ⚙ toggle button to interact with characters; click again to return to pass-through
- 📂 **Drag & drop** — drop PNG, GIF, WebP, or JPEG files onto the overlay to add new characters
- 🎮 **Multiple characters** — render several animated entities simultaneously
- 🎬 **PNG sequence animation** — load frames from a folder
- 🎞️ **GIF support** — animated GIF loading with per-frame delays
- 🌐 **WebP support** — animated and static WebP images
- 📸 **JPEG support** — static JPEG/JPG images
- 🎨 **Spritesheet support** — texture atlas with configurable rows/columns
- 🖱️ **Drag to move** — click and drag characters to reposition them (edit mode)
- 🔍 **Scroll to resize** — mouse wheel scales the selected entity (edit mode)
- ↔️ **Arrow key nudge** — pixel-perfect positioning with arrow keys (10px steps)
- 🎚️ **Opacity control** — +/- keys to adjust transparency
- 👁️ **Visibility toggle** — V key to show/hide selected entity
- 🎯 **Click-to-select** — click on characters to select them with visual highlight
- 🔄 **Tab to cycle** — Tab key cycles through entities for quick selection
- ⏯️ **Play/pause** — global (Space) and per-entity (P) playback toggle
- 📋 **Duplicate** — D key to clone selected entity with its properties
- 📊 **Z-order control** — Page Up/Down to layer entities
- 🗑️ **Delete entities** — Delete/Backspace to remove selected character
- 💾 **Auto-save** — config automatically saved when exiting edit mode
- 🔄 **Hot-reload config** — edit `config.toml` externally and see changes live (polls every 2s)
- ⚙️ **Per-character config** — position, scale, opacity, FPS, visibility, z-index
- 💾 **Persistent config** — TOML configuration saved to `~/.config/animaEngine/config.toml`
- 🎨 **GPU-accelerated** — wgpu rendering with Vulkan/OpenGL backend, optimized vertex buffer reuse
- 🛡️ **Security hardened** — decompression bomb guard, entity cap, buffer overflow protection
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
| **Drag & drop file** | Add image/GIF as new character (auto-enters edit mode) |
| **Click + Drag** | Move a character *(edit mode)* |
| **Click** | Select a character *(edit mode)* |
| **Tab** | Cycle selection through entities *(edit mode)* |
| **Scroll wheel** | Resize selected entity *(edit mode)* |
| **Arrow keys** | Nudge selected entity 10px *(edit mode)* |
| **Shift + Arrow keys** | Fine nudge 1px *(edit mode)* |
| **Home** | Center selected entity on screen *(edit mode)* |
| **+/=** | Increase opacity *(edit mode)* |
| **-** | Decrease opacity *(edit mode)* |
| **R** | Reset scale & opacity to defaults *(edit mode)* |
| **P** | Toggle play/pause for selected entity *(edit mode)* |
| **V** | Toggle visibility of selected entity *(edit mode)* |
| **D** | Duplicate selected entity *(edit mode)* |
| **Page Up** | Bring selected entity forward (z-index +10) *(edit mode)* |
| **Page Down** | Send selected entity backward (z-index -10) *(edit mode)* |
| **Delete/Backspace** | Remove selected entity *(edit mode)* |
| **Space** | Toggle global play/pause *(edit mode)* |
| **S** | Save config *(edit mode)* |
| **Q** | Save and exit *(edit mode)* |
| **Escape** | Exit edit mode → pass-through (auto-saves) |

> **Default behavior:** The overlay starts in **pass-through mode** — all clicks go through to the desktop. Click the **⚙ button** in the top-right corner to enter edit mode. You can also drag & drop image files directly onto the overlay — it will auto-enter edit mode and add the character at the cursor position.

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

**Or simply drag & drop** any PNG, GIF, WebP, or JPEG file onto the overlay — it will be added instantly at the cursor position with auto-resize to 256px max.

### Asset Types

| Type | Value | Description |
|------|-------|-------------|
| PNG Sequence | `png_sequence` | Folder of sorted PNG files (frame_001.png, frame_002.png, ...) |
| Static PNG | `png_static` | Single PNG file |
| JPEG | `png_static` | JPEG/JPG image (uses same loader) |
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
├── physics.rs           # Physics: gravity, bounce, freeze/release
├── animation/
│   ├── mod.rs           # Animation state: frame cycling, delta-time FPS
│   ├── frame.rs         # Frame: raw RGBA pixel data + resize
│   ├── loader.rs        # Asset type router + dimension validation
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
│   └── x11_input.rs     # X11 Input Shape + EWMH overlay hints + reconnect
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
- ✅ Always-on-top (Normal window + EWMH `_NET_WM_STATE_ABOVE`)
- ✅ Click-through with toggle button (X11 Input Shape)
- ✅ Mouse drag, scroll resize, arrow key nudge
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
- [x] JPEG support
- [x] Drag & drop files to add characters
- [x] Auto-resize dropped assets (max 256px)
- [x] Scroll wheel resize + arrow key nudge
- [x] Opacity/visibility controls (+/-/V keys)
- [x] Delete entities (Delete/Backspace)
- [x] Visual selection highlight
- [x] Optimized GPU rendering (vertex buffer reuse)
- [x] Auto-detect monitor resolution
- [x] X11 connection pooling + reconnect
- [x] Security hardening (decompression bomb, entity cap, buffer overflow)
- [x] Delta-time animation fix
- [x] Physics engine (opt-in gravity with bounce)

### Next Steps
- [ ] System tray icon with controls
- [ ] Right-click context menu per character
- [x] Per-character play/pause toggle (P key)
- [x] Hot-reload config on file change (polls every 2s)

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
