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

**Surface-loss recovery** (`app/render_loop.rs`). `get_current_texture`
returns `SurfaceError`, matched exhaustively — there is no `unwrap`, so
a lost surface never panics. `Lost`/`Outdated` reconfigure the surface
in place (the common transient: resize race, occlusion, the first frame
or two after S3 resume) and bump a consecutive-loss streak; `Timeout`
drops the frame; `OutOfMemory` saves and exits. If the streak passes
`SURFACE_LOSS_REBUILD_THRESHOLD` the surface isn't coming back by
reconfigure (driver reset, GPU hot-unplug, device lost across suspend),
so the renderer is **rebuilt wholesale** from the retained `Arc<Window>`
— the same `WgpuRenderer::new` path as startup — and every entity is
re-marked dirty to re-upload its texture. A failed rebuild means the GPU
is unusable: exit cleanly (config already persisted) so the session
restarts us, rather than spin on a dead device. The escalation policy is
unit-tested (`next_surface_loss_state`); the device-loss *trigger* can't
be simulated without GPU hardware, so the rebuild path is validated by
construction (it reuses the tested init), not by a forced loss.

### UI overlay (egui)

```
EguiRenderer (src/ui/egui_renderer.rs)
  ├─ egui::Context + egui_winit::State + egui_wgpu::Renderer
  ├─ Installs Phosphor icon font + active theme on construction
  └─ Renders on top of the sprite pass via LoadOp::Load

Panels (src/ui/panels/ — one file per tab/widget)
  ├─ scene.rs / inspector.rs / appearance.rs — tabbed sidebar
  ├─ context_menu.rs    — right-click popup
  ├─ command_palette.rs — Ctrl+K fuzzy search over themes + presets
  ├─ toasts.rs          — bottom-right notification stack
  ├─ library.rs / monitor.rs / presets.rs / keybindings_tab.rs
  └─ toggle_button.rs   — the ⚙ widget in pass-through mode

Token + helper modules (Phase A), src/ui/:
  ├─ theme.rs        — Palette, 4 themes (Dark/Light + HC pairs), apply()
  ├─ icons.rs        — Phosphor glyph constants by domain
  ├─ states.rs       — empty / error / spinner reusable cards
  ├─ motion.rs       — UI transition helpers (crate::anim holds the
  │                    pure easing curves: ease_in_quad, ease_out_quad)
  ├─ onboarding.rs   — OnboardingProgress + dismissible hint widget
  └─ keyboard.rs     — thin re-export of crate::keybindings::Action

Rebindable keyboard map (src/keybindings/, D.1):
  ├─ Action enum (28 variants) — single source of truth for dispatch
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
  ├─ BoundedWander { box, speed } — random walk inside a rect
  └─ Bounce { amplitude_px, period_sec, axis } — sinusoidal bob
                                    around the rest position; gravity
                                    overrides it

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
  Global hotkeys (src/hotkeys/, portal preferred, XGrabKey fallback)
       ╲
  Single-instance Activate (src/single_instance.rs, zbus thread)
       ╲
        ╲
         AnimaEvent enum (src/event.rs)
              ↓
         EventLoopProxy<AnimaEvent>::send_event
              ↓
         winit user_event → App::user_event arm (src/app/mod.rs)
              ↓
         {ToggleEditMode, ToggleGlobalPlayback, HideOverlay,
          ShowOverlay, RaiseWindow, Quit, HotkeysUnavailable,
          PortalShortcutsDenied}
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

`LayerWindow::set_input_region` (src/wayland/layer_window/mod.rs) uses
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
| Global hotkeys | ✅ XGrabKey (portal preferred when present) | ✅ GlobalShortcuts portal; D-Bus + compositor binding fallback |
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

## Multi-window rendering (decision record, T.5)

Decided before the 0.6 implementation (T.6–T.8); recorded so the
constraints survive the refactor.

**Shape:** one shared `wgpu::Instance` + `Device` + `Queue` +
pipeline + bind-group layouts + **entity texture cache**, and one
`Surface` + `SurfaceConfiguration` + dynamic vertex buffer per
overlay window. `App` owns a `WindowId → WindowSlot` registry
(`WindowSlot { window, surface_state, monitor: MonitorInfo,
x11_input }`).

**Why one device, many surfaces:**

- Entity textures are window-agnostic — an entity moving between
  monitors (or visible on two in a future Span-across-windows mode)
  must not re-upload its frames.
- One device = one device-loss domain; recovery handles every
  window the same way.
- The egui renderer binds to a single device, and egui runs only on
  the **primary** window (settings panel, palette, toasts). Other
  windows render sprites + the ⚙ toggle button sprite only.
- `prune_stale_textures` stays a single sweep over the shared cache.

**Mode mapping** (`MonitorMode`, unchanged in config; the default
is `PerMonitor` — corrected from an earlier draft of this record
that claimed Span):

- `Span` — exactly the pre-0.6 single-window path: one window sized
  to the primary monitor, identity origin.
- `PerMonitor` (default) — one window per `MonitorInfo`; entities
  render in the window whose monitor resolves from their
  position/pin; coordinates translate global → window-local at
  draw-list build. On a single-monitor machine this degenerates to
  one window, behaviourally identical to Span — which is why the
  default changing paths is safe for the typical install. On
  multi-monitor setups this is the C.3 fix: entities resolved to a
  secondary monitor were previously distributed by the data layer
  but never rendered.
- `Single { name }` — one window, on the named monitor (stale names
  fall back to primary).

**Input:** every window forwards events tagged by `WindowId`;
cursor coordinates translate window-local → global before
hit-testing. Edit mode is global (all windows flip input regions
together); the settings panel lives on the primary window.

**Pacing:** `RedrawPacing` is computed per window — only entities
resolved to that window's monitor hold it awake; `request_redraw`
fans out only to windows whose content changed. The idle heartbeat
stays a single timer (hot-reload is window-independent).

**Hotplug (T.9):** monitor-set changes diff the registry — spawn
windows for new monitors, despawn for vanished ones, re-resolve
entity pins (stale pins fall back to centroid resolution with a
toast).

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

- Behavior deep-dive: `src/behavior.rs` (~740 lines, mostly tests)
- Render pass: `src/renderer/wgpu_renderer.rs::render`
- Event arm matrix: `src/app/mod.rs::user_event` (the AnimaEvent dispatch)
- Wayland scaffolding: read `src/wayland/mod.rs` first, then the
  sub-files in the order it lists
- **Security invariants**: `docs/threat-model.md` — what the codebase
  promises to keep safe and what it deliberately doesn't
