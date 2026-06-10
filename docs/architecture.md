# animaEngine architecture

A map of the codebase organized around the data flow, not the file tree.
For the file tree, `ls src/` is honest.

## Boot sequence

```
main()
 ├─ init_tracing()                    src/main.rs
 ├─ single_instance::try_acquire()    src/single_instance.rs        (8.4)
 │    └─ either claim com.animaengine.Anima OR exit cleanly
 ├─ wayland::detect()                 src/wayland/probe.rs          (7.1)
 │    └─ check for zwlr_layer_shell_v1
 ├─ demo::generate_assets()           src/demo/                     (5.2)
 │    └─ generate sample sprites on first run
 ├─ AppConfig::load()                 src/config.rs
 ├─ Scene::from_config()              src/scene.rs
 └─ branch:
      ├─ ANIMA_USE_WAYLAND_NATIVE + layer_shell → wayland::run_native()
      └─ default → run_winit_path() with the full event loop
```

## Subsystems

### Rendering

```
WgpuRenderer (src/renderer/wgpu_renderer.rs)
  ├─ wgpu::Instance + Adapter + Device + Queue
  ├─ Surface (PreMultiplied alpha for transparency)
  ├─ Pipeline: sprite quads (src/shaders/sprite.wgsl)
  ├─ Dynamic vertex buffer reused per frame
  └─ Per-entity GpuTexture cache (src/renderer/texture.rs)

Two entry points:
  - new(window: Arc<winit::Window>)               winit path
  - from_instance_surface(instance, surface, w, h)  backend-agnostic
```

### UI overlay (egui)

```
EguiRenderer (src/ui/egui_renderer.rs)
  ├─ egui::Context + egui_winit::State + egui_wgpu::Renderer
  ├─ Installs Phosphor icon font + active theme on construction
  └─ Renders on top of the sprite pass via LoadOp::Load

Panels (src/ui/panels.rs)
  ├─ settings()         — tabbed sidebar (Inspector / Scene / Appearance)
  ├─ context_menu()     — right-click popup with 6 actions
  ├─ command_palette()  — Ctrl+K fuzzy search over themes + presets
  ├─ toasts()           — bottom-right notification stack
  └─ toggle_button()    — the ⚙ widget in pass-through mode

Token + helper modules (Phase A):
  ├─ theme.rs        — Palette, 4 themes (Dark/Light + HC pairs), apply()
  ├─ icons.rs        — Phosphor glyph constants by domain
  ├─ states.rs       — empty / error / spinner reusable cards
  ├─ anim.rs         — pure easing helpers (ease_in_quad, ease_out_quad)
  ├─ onboarding.rs   — OnboardingProgress + dismissible hint widget
  └─ keyboard.rs     — thin re-export of crate::keybindings::Action

Rebindable keyboard map (src/keybindings.rs, D.1):
  ├─ Action enum (27 variants) — single source of truth for dispatch
  ├─ KeyChord + KeyCode + ModifierMask — canonical serializable form
  ├─ KeyBindings (BTreeMap<Action, Vec<KeyChord>>) — user-overridable
  ├─ lookup() drives both global hotkeys and in-app dispatch
  └─ persisted under [keybindings.map] in config.toml

Toast queue (src/ui/toasts.rs)
  └─ FIFO with 8-entry cap, 4 severity levels (timing via created_at)

Localisation (src/i18n/)
  ├─ FluentBundle per locale, English fallback at every t() call
  ├─ 10 .ftl resources baked in via include_str!
  └─ Auto-detect from LANG / LC_ALL / LC_MESSAGES at startup

Curated content (src/presets.rs)
  └─ Six PresetIds with Apply{Replace, Append} modes for the Scene tab
```

### Behaviors

```
Behavior enum (src/behavior.rs)
  ├─ Idle                         — default, no motion
  ├─ WalkAround { speed }         — horizontal patrol with edge bounce
  ├─ FollowCursor { speed, comfort_distance }
  └─ BoundedWander { box, speed } — random walk inside a rect

Pattern: Behavior holds config (serialized to TOML), BehaviorState holds
runtime accumulators (direction, wander target, RNG seed). Entity::tick
applies behavior → physics → animation in that order.
```

### Animation pipeline

```
Asset on disk
   ↓
animation::loader::load_asset()           src/animation/loader.rs
   ├─ validate_image_dimensions()         decompression-bomb guard
   ├─ cache::try_load()                   on-disk RGBA cache, Phase 2.4
   └─ format dispatch:
         PngSequence   → png_sequence::load_png_sequence() (parallel, rayon)
         PngStatic     → png_sequence::load_single_png()
         Gif           → gif_loader
         WebpAnimated  → webp_loader
         Spritesheet   → spritesheet (row-stride memcpy, Phase 2.5)
         Video         → video_loader (mp4 + openh264, Phase 5.1)
   ↓
Vec<Frame> { rgba: Vec<u8>, width, height, delay_ms? }
   ↓
cache::try_save()                          best-effort RGBA cache write
```

### Event command bus

```
Three independent producers, one consumer:

  Tray (src/tray.rs, ksni async thread)
       ╲
  Global hotkeys (src/hotkeys.rs, XGrabKey via global-hotkey)
       ╲
  Single-instance Activate (src/single_instance.rs, zbus thread)
       ╲
        ╲
         AnimaEvent enum (src/event.rs)
              ↓
         EventLoopProxy<AnimaEvent>::send_event
              ↓
         winit user_event → App::user_event arm
              ↓
         {ToggleEditMode, ShowOverlay, HideOverlay,
          ToggleGlobalPlayback, RaiseWindow, Quit}
```

### Scene cache

```
Scene::visible_entities() uses a RefCell<VisibleCache>:
  - Indices sorted by z_index, refreshed only when invalidated
  - mark_visible_dirty() called from add/remove (auto)
                          + V / PageUp / PageDown keys (manual)
                          + inspector toggles (auto)
  - Saves ~3000 sort calls/sec at 60 fps with 50+ entities
```

## Click-through

The overlay window covers the whole screen but only one corner accepts
mouse input by default (the ⚙ button). Two equivalent implementations,
chosen at runtime:

### X11 (default)

`X11InputManager` (src/window/x11_input.rs) uses **XShape** extension:

```
pass-through mode → input shape = rect(width-64, 0, 64, 64)
edit mode         → input shape = full window
```

Re-applied on `Resized`, `Focused(true)`, `Occluded(false)` because
Mutter (and some others) clip the shape on certain transitions.

### Wayland native (opt-in)

`LayerWindow::set_input_region` (src/wayland/layer_window.rs) uses
**`wl_compositor::create_region` + `wl_surface::set_input_region`**:

```
let region = compositor.create_region(...);
region.add(rect);                        // empty for full click-through
surface.set_input_region(Some(&region));
surface.commit();
region.destroy();                        // compositor copied it
```

Behavior matches X11 exactly: button cutout in pass-through, full
region in edit mode.

## Native Wayland status

| Feature | X11 path | Wayland native (opt-in, beta) |
|---------|----------|------------------------------|
| Window creation | winit + EWMH hints | sctk + wlr-layer-shell |
| Click-through | XShape | set_input_region |
| Sprite rendering | ✅ | ✅ |
| Pointer events | ✅ via winit | ✅ via sctk |
| Keyboard events | ✅ via winit | ✅ via sctk + xkbcommon (E.1) |
| egui UI | ✅ | ✅ events routed to egui (E.5) |
| Drag-and-drop | ✅ | ✅ via wl_data_device (E.4) |
| Tray | ✅ | ✅ (D-Bus, independent of backend) |
| Global hotkeys | ✅ XGrabKey | ⚠ via `org.animaengine.Anima` D-Bus + compositor binding |
| Single instance | ✅ | ✅ (D-Bus) |

The X11/XWayland path remains the default and the recommended
daily-driver. The native path is opt-in via
`ANIMA_USE_WAYLAND_NATIVE=1` and limited to wlroots compositors
(sway, Hyprland, river, Wayfire). GNOME and KDE Wayland sessions
do not implement layer-shell and fall back to XWayland
automatically — no flag needed.

Global hotkeys on Wayland are intentionally compositor-gated:
Wayland refuses raw `XGrabKey`-style global grabs, so the native
backend exposes the same actions as D-Bus methods on
`org.animaengine.Anima` and ships sample sway / Hyprland / river
bindings in [docs/wayland.md](wayland.md) that call them through
`gdbus`.

## Threads

| Thread | Purpose | Communication |
|--------|---------|---------------|
| Main (winit event loop) | Render, input, scene tick | — |
| `anima-tray` | ksni async runtime + DBus | `EventLoopProxy<AnimaEvent>` |
| `anima-instance` | zbus connection holding `com.animaengine.Anima` | `EventLoopProxy<AnimaEvent>` |
| Hotkey global handler | `GlobalHotKeyEvent::set_event_handler` closure | `EventLoopProxy<AnimaEvent>` |
| Hot-reload worker | One-shot per mtime change: load + decode | `mpsc::Sender<HotReloadResult>` |

All cross-thread messages are typed (`AnimaEvent` / `HotReloadResult`).
No shared mutable state outside `mpsc` channels.

## Packaging

Three formats, all built from one source of truth (`make install` rules):

| Format | Builder | Output | Phase |
|--------|---------|--------|------|
| `.deb` | `cargo-deb` (reads `[package.metadata.deb]`) | ~5.4 MB | 8.3 |
| AppImage | `linuxdeploy` | ~7.2 MB | 8.2 |
| Flatpak | `flatpak-builder` + manifest | offline-only on Flathub | 8.4 |

Single source of truth for asset layout: the `install` target in the
top-level `Makefile`. AppImage and `.deb` both go through it.

## Where to look next

- Behavior deep-dive: `src/behavior.rs` is ~330 lines, mostly tests
- Render pass: `src/renderer/wgpu_renderer.rs::render` (~90 lines)
- Event arm matrix: `src/app.rs::user_event` (the AnimaEvent dispatch)
- Wayland scaffolding: read `src/wayland/mod.rs` first, then the
  sub-files in the order it lists
- **Security invariants**: `docs/threat-model.md` — what the codebase
  promises to keep safe and what it deliberately doesn't
