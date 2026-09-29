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
  ├─ command_palette.rs — Ctrl+K search over actions, themes + presets
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
  ├─ Bounce { amplitude_px, period_sec, axis } — sinusoidal bob
  │                                 around the rest position; gravity
  │                                 overrides it
  └─ Script { path, params }      — a Rhai file from the asset library

Scripted motion does NOT run inside `Behavior::tick`. It needs the asset
library root and a per-entity scope, so `Entity::tick_script` drives it
and the `tick` arm above is only the fallback: a script that is missing,
broken, or over its execution budget leaves the character where it was.

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
together); the settings panel lives on the primary window. The native
Wayland backend met this only for the primary surface until 1.3: its
extras now shift pointer and drop positions into the primary
surface's space at the edge (`WaylandState::surface_offset`), and one
per-frame sync keeps their input regions in step with the primary's
(`LayerWindow::sync_extra_layers`).

**Pacing:** `RedrawPacing` is computed per window — only entities
resolved to that window's monitor hold it awake; `request_redraw`
fans out only to windows whose content changed. The idle heartbeat
stays a single timer (hot-reload is window-independent). The decision
itself is `crate::pacing`, shared with the native Wayland loop, which
wakes every frame interval to drain its channels and tick the scene but
draws only when there is a reason to (`FrameGate` in `wayland/run.rs`):
input or another compositor event, a message from a channel, a sprite's
next animation frame, a repaint egui asked for, something moving every
frame — or the heartbeat.

**Hotplug (T.9):** monitor-set changes diff the registry — spawn
windows for new monitors, despawn for vanished ones, re-resolve
entity pins (stale pins fall back to centroid resolution with a
toast).

### Stepping aside (`src/fullscreen.rs`)

Each backend reports whether the window in front is full screen — the
winit path from a thread reading EWMH (`FullscreenWatch`, sending
`AnimaEvent::FullscreenInFront` on change), native Wayland from
`wlr-foreign-toplevel-management` (`layer_window/toplevels.rs`) — and
`step_aside` turns that, the setting and edit mode into `hidden` and
`paused`. Both are recomputed every frame and kept apart from the
user's hide (`overlay_hidden`) and playback switch
(`Scene::set_suspended`), so coming back needs no bookkeeping.

### Speech bubbles and reminders (`src/speech.rs`, `src/reminders.rs`)

An entity's `speech` is runtime-only: text (capped) and an expiry,
cleared by `Scene::tick` on the clock whether or not the scene runs.
Scripts set it through `say`, recorded like `play` and applied after the
run. Bubbles are gathered before the UI pass (`speech::shown`, the scene
being lent to the panels) and painted by egui on its background layer
(`ui::speech`), under the panel and click-through; egui runs on the
primary surface only, so that is where they show. Reminders are timers
keyed by the reminder itself, so editing the list leaves the others'
alone; the loops call `reminders::deliver` each frame — on Wayland before
the frame gate — and it restarts every timer while the user is away.

### Starting at login (`src/autostart.rs`)

Installed natively the switch *is* the XDG autostart entry: reading it
checks the file, flipping it writes or removes it (`Exec` is `$APPIMAGE`
or the current executable, quoted per the Desktop Entry spec, with the
native-Wayland opt-in carried over). In the Flatpak, detected by
`/.flatpak-info`, a thread asks the Background portal; its answer comes
back through an atomic the loops take each frame into
`start_at_login`, which is all the sandbox can know.

### Named scenes (`src/scenes.rs`)

A saved scene is a `SavedScene` — name, character configs, groups — in
its own TOML file under `scenes/` beside `config.toml`, found by the
`name` inside, not the file name (`slug` only picks a free one).
`shelf()` lists them at most every two seconds, or right after a save or
a delete, so the Scene tab and the palette can ask every frame. Loading
reads under the config's limits and goes through `Scene::apply_saved` →
`restore_configs`, the undo path, which also enforces `MAX_ENTITIES` and
the decode budget. The Scene tab and the palette load from input, so
undo records the switch; the tray's and D-Bus's `NextScene` is not
input, so its handler opens the undo step itself. `active_scene` in the
config remembers which one "Next scene" goes on from.

### Away and on battery (`src/away.rs`)

The scene also holds still (`Scene::set_suspended`, alongside stepping
aside) after `pause_when_idle_minutes` without input, and on battery if
asked. Native Wayland is told by the compositor (`ext-idle-notify-v1`,
`layer_window::idle`, the notification remade when the time changes).
Elsewhere a thread polls once a second: the MIT-SCREEN-SAVER extension
on a real X server, or Mutter's IdleMonitor under a Wayland session —
never XWayland's own counter, which misses input to native apps — and
UPower's `OnBattery` every ten seconds, at once when the setting turns
on. The loops hand it the settings through `away::configure`, and it
sends `AnimaEvent::Away` / `OnBattery` / `IdleSource` on change,
retrying a full channel rather than giving up.

### Several selected (`src/input/multi.rs`)

`SelectionState` keeps a *primary* — what the Inspector shows and what
single-entity code reads through `selected_index()` — and the others
selected with it. Press, drag and the selection rectangle go through
`input::multi` on both backends; the shared actions and the right-click
menu act on `selected_indices()`, toggles taking the primary's new
state. The renderer draws a highlight per selected entity and the
selection rectangle (`EditMarks`) — on every surface, cut to each by its
origin, so it shows on whichever monitors it crosses; the panel, which
drew it before, exists on the primary alone. `MAX_QUADS` is
`2 × MAX_ENTITIES + 8`: a sprite and a highlight each, then the
rectangle's fill and four edges, two snapping guides and the edit bar.

A gesture holds entity indices, so `multi::cancel` lets go of it
whenever those could change or the release could be lost: before an
action that adds, removes or reselects characters
(`Action::interrupts_drag`: undo, redo, cycle, delete, duplicate), when
edit mode ends, when the overlay hides or steps aside (R56), and before
the scene is replaced — a hot reload, a `NextScene` — while the
characters are still the ones it froze.

### Arranging (`src/input/arrange.rs`)

Align, distribute and snapping are geometry over `drawn_rect` — where
the renderer draws a character, its group's offset and scale included —
so what lines up is what is seen; each move shifts the stored position
by the same amount. `arrange` backs the row of buttons shared by the
Inspector and the right-click menu (`ui::panels::arrange`). `snap`
runs in `multi::drag_to` when the caller passes the monitors (the
setting is on and Alt is up): the selection's bounds, moved, meet the
nearest edge or centre of a monitor or of a visible unselected character
within `SNAP_DISTANCE` on each axis. The guides it returns live on the
`DragController`, cleared with the drag, and the renderer draws them
(`EditMarks::guides`), so no call site can leave one on screen.

### Groups (`src/group.rs`)

A group is data first (C.8): an id, a name, member ids, and an offset,
a scale and a visibility composed onto its members at draw time and in
hit-testing. Since 1.5 the app makes and edits them. `GroupSelected`
(Ctrl+G) and `UngroupSelected` (Ctrl+Shift+G) are shared actions, also
on the right-click menu (`outcomes::menu_group_offers` decides what it
offers); the Scene tab renames, hides, dissolves and selects. Making and
dissolving never changes what is on screen: a character leaving a group
takes the group's transform into its own (`leave_group`). An entity is
in at most one group — regrouping moves it, and a group emptied that way
goes, as does one emptied by deleting its members (a stub written empty
by hand stays). Duplicating exactly a group (`Scene::exact_group`)
groups the copies with the same transform. On the canvas a group acts as one: a press on a member selects the
whole group (`multi::select_with_its_group`, the right-click too), and a
second press on a member of the selection narrows it to that one on a
tap (`DragController::narrow_on_tap`). Undo covers it all through the
groups in its snapshots.

### Undo (`src/undo.rs`)

Edits reach the scene from dozens of places, so undo watches *gestures*
instead of hooking each one. The first input of a gesture — a press, a
key, a drop, a screen reader's request, not pointer motion — snapshots
the scene as character configs, with its sprite groups (a delete takes
a character out of its group), before the input is applied; the gesture
closes once no button is held, no text field or list has the keyboard,
no file chooser or import is running, and `SETTLE` (350 ms) has passed.
A snapshot that differs from the scene then is one step. Characters
that move by themselves (a behavior, physics) have their positions left
out of the comparison and out of the restore. A tap closes the gesture
before the poke's hop, which is play rather than an edit. Restoring
(`Scene::restore_configs`) sets properties back in place where a
character still shows the same asset and reloads only the rest. Both
loops feed it (`input`, `settle`); Undo and Redo are shared actions
(`keybindings::shared`). A config hot-reload clears the history.

## Threads

| Thread | Purpose | Communication |
|--------|---------|---------------|
| Main (winit event loop) | Render, input, scene tick | — |
| `anima-tray` | ksni async runtime + DBus | `EventLoopProxy<AnimaEvent>` |
| `anima-instance` | zbus connection holding `com.animaengine.Anima` | `EventLoopProxy<AnimaEvent>` |
| Hotkey global handler | `GlobalHotKeyEvent::set_event_handler` closure | `EventLoopProxy<AnimaEvent>` |
| Hot-reload worker | One-shot per mtime change: load + decode | `mpsc::Sender<config_watch::Reloaded>` (`src/config_watch.rs`, both backends) |

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
- Scripted behaviors: `src/scripting.rs` — the Rhai host, its sandbox and
  its execution limits. Design and rationale in
  [plans/v1.2-scripting.md](plans/v1.2-scripting.md); the script author's
  view is in CONTRIBUTING.
- Sound: `src/audio.rs` — one-shot playback panned by on-screen position,
  behind the optional `audio` feature. The module is always compiled; only
  the `rodio` parts are gated, so no caller's signature changes with it.
  The default output device is opened by the first sound and closed once
  the scene has been quiet for 30 s; a stream error closes it too, and the
  next sound reopens it.
- Machine load: `src/sysload.rs` — aggregate CPU / memory for scripts that
  react to it. Reads `/proc` directly rather than taking a dependency, so
  "we cannot enumerate processes" is structural rather than a promise.
- Render pass: `src/renderer/wgpu_renderer.rs::render`
- Event arm matrix: `src/app/mod.rs::user_event` (the AnimaEvent dispatch)
- Wayland scaffolding: read `src/wayland/mod.rs` first, then the
  sub-files in the order it lists
- **Security invariants**: `docs/threat-model.md` — what the codebase
  promises to keep safe and what it deliberately doesn't
