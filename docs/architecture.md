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
  ├─ FollowCharacter { target, distance, speed } — keeps `distance`
  │                                 from another character, on its own
  │                                 side, level when physics is on
  └─ Script { path, params }      — a Rhai file from the asset library

A follower learns where its leader is from the scene each tick
(`Scene::find_leaders` sets `Entity::leader`; `TickContext::leader`):
the middle of a visible character other than itself, else none, and it
stands. It sets off only once the gap has grown past 16 px and then goes
all the way (`BehaviorState::following`), so a leader shuffling does not
make it shuffle. Who may be followed is `behavior::follow_candidates`:
not itself, and not any character that already leads back to it — the
Inspector offers only those, so no circle can be made there; a circle
written by hand converges and stops. A follower pasted together with
its leader follows the pasted leader (`outcomes::paste_entities`).

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

### Taps (`src/tap.rs`)

With `tap_characters` on, pass-through also takes clicks on the
characters: `tap::rects` gives each visible one's drawn rectangle on a
surface, rounded outward to 8 px, and the loops add them to the ⚙
cutout — `LayerWindow::sync_tap_region` and the extras'
`ExtraInput::Rects` on Wayland, `set_passthrough_with_rects` on X11 —
at most every `tap::REFRESH` and only when they changed, so a walker
does not cost a region a frame. Any other path that sets the region
(`set_input_region`, `reapply_input_shape`) clears what taps set, and
the next refresh puts it back. A press and a release close together on
one character (`TapGesture`) are a tap; `tap::poke` gives it the same
hop and line as a tap in edit mode. Taps are off while a full-screen app
is in front, even with `on_fullscreen = "pause"`, when the characters
stay on screen over it. The keyboard stays with the app in
front: Wayland's keyboard interactivity is none in pass-through
(`set_keyboard_wanted`), X11's `WM_HINTS` input is false
(`set_focusable`); both turn back while egui has the pointer — the ⚙ —
so its click still focuses edit mode.

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

Apps the user named (`src/chosen_apps.rs`, `step_aside_apps`) step it
aside too, hidden, whatever the full-screen setting
(`StepAside::or_chosen_app`; not in edit mode). The X11 watch reads the
active window's `WM_CLASS` beside its state and sends
`ChosenAppInFront`; native Wayland keeps each toplevel's app id and asks
the list every frame. The picker in Appearance lists the apps open now
(`chosen_apps::open`), filled by the same two only while it shows
(`want_open_list`): on X11 that is a round-trip per window.

### Snoozing (`src/snooze.rs`)

A snooze is a deadline in one process-wide atomic (seconds since the
epoch, 0 for none) that the tray, D-Bus and the palette set through
`AnimaEvent::Snooze` and the loops read every frame: while it holds,
`StepAside::or_snoozing` makes them step aside hidden and still, as for
a full-screen app set to hide, so nothing else needs to know. Snoozing
leaves edit mode; entering edit mode, or Show overlay, wakes. The X11 heartbeat runs `update_step_aside` too, or a
snooze ending with every window unmapped would wait for a frame that
never comes. The tray menu is `tray_menu::entries`, built on each
showing; ksni's thread looks at the deadline every few seconds and
asks for the menu again when it changed.

### The focus timer (`src/focus.rs`)

Work, break, work, … until stopped: one process-wide timer behind a
mutex, started and stopped through `AnimaEvent::StartFocus` (the tray,
D-Bus) and `StopFocus`, or directly by the palette and the Scene tab.
Reading it moves it on: a part that is over gives way to the next from
now — a machine that slept is not made to catch up — and leaves the news
to say. The loops call `focus::say_news` next to `reminders::deliver`
every frame: the frontmost visible character says it, and it waits
while the overlay is out of sight or nobody is there. Reminders take
the timer's part into account: during the work they cannot speak, and
the break counts as being away, so their timers start over. The tray's
block (`TrayItem::Focus`) shows until when, like the snooze's.

With `notify_reminders` on, `reminders::deliver` and `focus::say_news`
also hand what they say to `notify::send` — a queue to one thread on the
session bus (`Notify`, or the portal's `AddNotification` in a Flatpak) —
and no longer wait for the characters when they are out of sight. In
tests `send` only records (`notify::SENT`), so a test run never reaches
the desktop of whoever runs it.

### Behind the windows (`always_on_top`)

`always_on_top` off puts the overlay under other windows, on the
desktop: native Wayland moves every layer surface from the overlay
layer to the bottom one (`LayerWindow::set_on_top`, new extras made on
the current one); the winit path sets each window's level to
`AlwaysOnBottom` (`App::apply_stacking`, new extras too) and stops
re-asserting `_NET_WM_STATE_ABOVE` on focus. Edit mode is always in
front, so the panel is in reach. Both run every frame, cached. A
surface under a window that covers it is never shown, and a compositor
sends no frame callbacks to it — under `Fifo` the next present never
returned and the Wayland loop stopped there, so that loop, which paces
itself at `FRAME_INTERVAL`, presents with `Mailbox` where the driver has
it (`WgpuRenderer::prefer_mailbox`). The X11 loop leans on `Fifo` for
its pacing and keeps it; XWayland paces a hidden window on its own.

### Own hours (`src/hours.rs`)

A character's `hours` (config and entity) are a window of the day;
`hours::within` is the window test bedtime uses too. The loops read the
clock through an `hours::Clock` (every ten seconds) where they read
bedtime, and hand `Scene::set_clock` the minute — `None` in edit mode —
which sets each entity's runtime `off_hours` and marks the visibility
cache dirty when anyone came or went. `Scene::effective_visible` counts
`off_hours` as hidden, so drawing, hit tests, taps, bubbles, speakers
and effects all leave such a character out without knowing why.

### Moods (`src/moods.rs`)

With `moods` on, `Moods::step` — called by both loops every frame,
reading the clock every ten seconds — sets each entity's runtime `tempo`
from `tempo_at`, a smooth curve over the day (1.2 in the morning, 1 in
the day, 0.7 at night). `Entity::tick` scales a walker's or wanderer's
step by it (other behaviors keep their own pace) and hands it to the
animation, whose frame holds it divides (`Animation::tempo`). With
bedtime on, each visible character yawns once a night in the half hour
before it, at a moment its id picks. Off, the tempos go back to 1.

### Fading (`Scene::fade_step`)

Each entity carries a runtime `fade`, 0 to 1, that the renderer
multiplies into its opacity. `Scene::fade_step`, called every frame by
both loops whether the scene runs or not, moves it toward 1 while the
entity is effectively visible and toward 0 while not, over
`FADE_SECONDS` (at once with Reduce motion). Entities start at 1 as made
— tests and tools see them whole — and the scene sets 0 for those that
arrive: at startup, an add, the new ones of a restore. `fade_all_in`
does it for everyone when the overlay comes back into sight (X11:
`sync_window_visibility`; Wayland: the loop's own hidden flag).
`Scene::drawn_entities` is what the loops draw — the visible, and those
still fading out — while hit tests keep `effective_visible`; while
anything fades, `pacing` asks for frames.

### Tag (`src/tag.rs`)

`Behavior::PlayTag { partner, speed }` holds still on its own; the game
lives in the scene (`Scene::games`). Each tick before anyone moves,
`tag::step` pairs each player with its partner (one game per character,
both visible), keeps each pair's game — who is "it", playing or resting
until when, when a catch next counts — and gives both an
`Entity::game_move` for the tick: toward the runner, or away from the
chaser at `RUNNER` of the speed. `Entity::tick` follows that instead of
its behavior (across only with physics on). A catch (middles within
`REACH` of their half widths) sets both `still_until`, swaps who is it,
and holds the next catch off for a head start. Paste remaps a partner
pasted alongside; an import clears one not in the file; a restore
forgets the games.

### Greetings (`src/greet.rs`)

With `characters_greet` on, `Scene::tick` runs `greet::step` after the
walkers turn: the pairs whose seen shapes (`Scene::seen_rect`, sides
inset as bumping does) are within `greet::NEAR` of each other, by id,
against last tick's — a pair that was not near and is now has met. The
first set taken after a start, a scene switch or the setting coming on
only primes it, so those already together say nothing. A meeting has
the first say hello (`Entity::greet`: a line of its own, or one of three
plain ones), queues the other's answer a moment later — not what it just
heard — sets both entities' `still_until`, which skips their behavior
step while it lasts, and puts a heart between them
(`Scene::heart_between`). A pair waits `PAIR_EVERY`, the scene
`ANY_EVERY`, and nobody who is talking is made to. Only a running scene
ticks, so nobody greets while paused or asleep.

### Characters bumping into each other (`src/bump.rs`)

Optional (`characters_bump`). Before the entities tick, each gets its
own platform list: the window platforms plus every other visible,
undragged character's top, in its own coordinates, through the same
`platforms::effective_floor` rule — so two can never hold each other up,
a top above the feet being a wall. Tops and feet are what can be seen:
the rectangle less its empty rows, the fewest over the animation's
frames (`Animation::steady_margins`, worked out once), so the floor does
not twitch with an ear and drop a resting character through it. After
the tick, `turn_walkers` reverses a `WalkAround` heading into a solid
neighbour, sides inset for the margins, and only when the two overlap by
a third of the shorter's height — standing on a head is not meeting.

### Speech bubbles and reminders (`src/speech.rs`, `src/reminders.rs`)

An entity's `speech` is runtime-only: text (capped) and an expiry,
cleared by `Scene::tick` on the clock whether or not the scene runs.
Scripts set it through `say`, recorded like `play` and applied after the
run. Bubbles are gathered before the UI pass (`speech::shown`, the scene
being lent to the panels) and painted by egui on its background layer
(`ui::speech`), under the panel and click-through — by the panel's egui
on the primary surface, and by a `ui::surface_egui::SurfaceEgui` of
their own on every other monitor's (made when a bubble first lands
there, run while it has bubbles and once more for the frame that clears
one; the loops pace on its repaint requests too). The right-click menu
uses the same one when it opens on another monitor
(`ContextMenuState::surface` names it, and the panel's egui skips it):
that egui hears the pointer only while the menu is there and only over
its monitor, so a press on any other monitor closes the menu in the
loop, as does Escape, which reaches the panel's egui. A press on the
menu is kept from the scene, as the panel's egui keeps its own
(`SurfaceEgui::owns_pointer`). The native Wayland
panel is that same core plus a screen reader and an input method
(`wayland::egui_render`). Reminders are timers
keyed by the reminder itself, so editing the list leaves the others'
alone; the loops call `reminders::deliver` each frame — on Wayland before
the frame gate — and it restarts every timer while the user is away.
Reminders at a set time (`ReminderConfig::at`, `reminders::At`) are
`Timers::poll_timed`'s: once a day each, on its days or its date, from
its time to `LATE_BY_MINUTES` after (a date's to midnight), against a
local clock read every ten seconds; away (no input — not bedtime: a
time was asked for) or unable to be said, they wait within that time
rather than start over (`reminders::Away`). Several due in one frame
go to several characters, front to back. Idle is watched for them even with pausing off
(`away::idle_watch_minutes`: the pause's time, or five minutes); only the
pause setting decides whether the scene holds still.

A character's own lines are config (`CharacterConfig::talk`,
`[characters.talk]`, tidied by `phrases::tidy` on every load): a poke
says one (`Entity::say_a_phrase`, never the last one again), and
`Scene::tick` lets each visible one speak up about every `every_minutes`
(`Entity::speak_up`: a time a third either way, rescheduled when the
pace changes, skipped while it is mid-sentence). Only a running scene
ticks that far, so nobody speaks while paused or asleep;
`Scene::take_spoke` tells the loops to draw the frame. The Inspector
edits them as a text box, one line each. Whatever a character says goes
through `placeholders::fill_now` in `Entity::say` and `say_a_phrase`:
`{time}`, `{day}`, `{date}` and `{name}`, from `schedule::local_now`
(which reads the day and month too) and the locale's weekday and month
names and `placeholder-date` pattern.

A tap sound (`talk.sound`, `src/sounds.rs`) is a path into the asset
library, tidied with the lines. A poke sets the entity's `sound_wanted`
to it, as the Inspector's Play does — and a reminder or a focus turn to
`reminder_sound`'s choice (`ReminderSound::play_by`) — and the loops drain them right after the
tick (`Scene::take_sounds`) into `AudioHost::play` — the path a script's
`play` takes, so the same containment, caps, cooldown and lazily opened
device. The Inspector's list comes from `sounds::choices`: a bounded walk
of the library that passes links by, cached for a few seconds; the loops
hand it the root (`sounds::set_root`).

### Little effects (`src/effects.rs`)

Hearts, dust and z's are a list on the scene (`Scene::effects`), each
an origin, a start and a lifetime, drawn on the clock: `tap::poke` adds
hearts (`Scene::hearts_for`), `Scene::tick` and dozing off's settling
add dust where `PhysicsState::impact` — the speed of the last floor hit,
taken each tick — is past `effects::DUST_SPEED`, and the settling adds
z's once everyone is asleep. They start from what can be seen of a
character (its drawn rectangle less its empty rows), and hearts move
with theirs, so a poke's hop does not leave them at its feet. The loops
gather each surface's share with the bubbles (`Scene::effects_shown`)
and paint it under them (`ui::effects`; a heart is a fan mesh with a
one-pixel fringe). While one is alive `pacing::redraw_pacing` says
`Continuous`, a still or sleeping scene's too — a second or two — and
`Scene::tick` drops the finished ones. None are made with
`little_effects` off or Reduce motion on, and switching either clears
them.

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

`src/schedule.rs` switches between them by the local clock
(`localtime_r`, or `GetLocalTime` on Windows), read at most once a
second. `Schedule::due` fires a rule when its minute is crossed between
two looks — the latest one crossed, across midnight too — and, at the
first look, catches up on the latest passed today. The loops do not
look in edit mode, so a crossing then is found when it ends; the switch
(`outcomes::apply_scheduled`) skips the scene already active, opens the
undo step and lets go of any gesture, as `NextScene` does.

### Away and on battery (`src/away.rs`, `src/doze.rs`)

The scene also holds still after `pause_when_idle_minutes` without
input, and on battery if asked (`away::stillness`). Away, it dozes off
(`Scene::set_dozing`): every character gets a Zzz… (`doze::zzz`, put
back by `Scene::tick` when another bubble ends, taken away on waking),
and `tick` runs gravity alone (`Entity::fall`) — no behaviors, scripts
or animation — until everyone has landed or `doze::SETTLE_LIMIT` has
passed; then each takes its idle pose (`Entity::rest`) and
`is_running` turns false, which stops the frames as a pause does.
Bedtime (`src/bedtime.rs`, `[global.bedtime]`) dozes the same way by
the clock — read at most every ten seconds (`bedtime::Watch`), across
midnight, not in edit mode — through `Stillness::with_bedtime`, and is
handed to the reminders as time away, so the night is their break.
Characters arriving while it sleeps — a scheduled scene, a reload —
settle the same way (`Scene::settle_again`). With a full-screen app in
front, no input is someone watching (`Stillness::watching`): no Zzz…,
only the pause. On battery with someone there it is a plain
`Scene::set_suspended`, alongside stepping aside. Native Wayland is told by the compositor (`ext-idle-notify-v1`,
`layer_window::idle`, the notification remade when the time changes).
Elsewhere a thread polls once a second: the MIT-SCREEN-SAVER extension
on a real X server, or Mutter's IdleMonitor under a Wayland session —
never XWayland's own counter, which misses input to native apps — and
UPower's `OnBattery` every ten seconds, at once when the setting turns
on. The loops hand it the settings through `away::configure`, and it
sends `AnimaEvent::Away` / `OnBattery` / `IdleSource` on change,
retrying a full channel rather than giving up.


Coming back is greeted (`src/welcome.rs`): the loops hand
`welcome::Welcome::check` the away flag every frame, and the step from
away to back yields "Welcome back!" or, coming back in the morning after
the night (away four hours, or since before 04:00, or since the evening
before), "Good morning!" — said by `Scene::frontmost_visible` when the
overlay is on screen and it is not bedtime. With `welcome_back` on, idle
is watched even with pausing off (`away::idle_watch_minutes`'s
`needed`), as for reminders.
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
(`Action::interrupts_drag`: undo, redo, cycle, delete, duplicate, cut,
paste), when edit mode ends, when the overlay hides or steps aside
(R56), and before the scene is replaced — a hot reload, a `NextScene` —
while the characters are still the ones it froze.

Let go while the pointer still moves, the selection is thrown:
`DragController` keeps the last 100 ms of the pointer (`track`), and
`fling` at the release gives its speed if it was still going — at least
300 px/s, at most 2000, none with reduced motion, none for `cancel`.
`PhysicsState::throw` then flies each character with gravity whether
its physics is on or not (`thrown`, `velocity_x`, `tick_x`): off the
desktop's sides, onto the same floors as falling (windows, others with
bumping on), sliding to a stop, after which physics is what it was.
Nobody walks mid-air, a press catches one, and the pacing draws while
anything flies. Coming to rest marks the config for saving
(`Scene::take_landed` — the save at the release had it in mid-air), and
a character set somewhere else while flying — a scene switched in, a
reload — stays there (`Entity::set_properties`).

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

### Sharing a scene (`src/scene_file/`)

A saved scene goes out as one `.animascene` file: a zip holding
`scene.toml` (`format` and the `SavedScene`) and each picture its
characters show, once, under `assets/<n>/` — a sequence's frames
renumbered in play order, one file as `asset.<ext>` — with the paths in
`scene.toml` rewritten to those. `scene_file::zip` writes stored entries
and reads back only that shape (see the threat model). Import unpacks
into a fresh `imported-scenes/<name>/` under the data directory, points
the characters there, and tidies the scene as a file from someone else
(`tidy`); `outcomes::apply_imported` saves it to the shelf under a free
name (`scenes::free_name`) and puts it on screen, with an undo step the
loops open first — fitted into the area the overlay covers when it was
laid out for other screens (`scene_file::fit_onto`: per axis, kept,
moved as a block, or squeezed), and saved again so. Both directions run on a worker behind the portal's
chooser — `SaveFile` to share, falling back to Downloads where there is
no portal, and `OpenFile` or a drop to import — as one pending
`outcomes::SceneTransfer` each loop polls. Imported pictures stay when
their scene is deleted, like the library's.

One character goes out the same way (`scene_file::export_character`,
`.animachar`): a scene of one, `character = true` in `scene.toml`.
Everything that routes a `.animascene` routes it too
(`scene_file::is_scene_file`), through the same reader and `tidy`; the
import carries the flag (`Imported::character`) and where it was dropped
(`Imported::at`), and `outcomes::apply_imported` hands it to
`add_shared_characters`, which places it there — or mid-screen from the
chooser — and adds it as a paste does (`paste_with`: fresh ids, on top,
selected), instead of shelving and switching to a scene.

Everything goes out the same way too (`scene_file::backup`): the
settings — with the scene on screen — and every saved scene, packed by
the same `Packer` and `pack_character`, under `backup.toml`. Restoring
reads it with the same pieces (`tidy`, `local_path`, `unpack`), then
`put_in_place` adds the scenes under free names and saves the settings
over `config.toml`, kept first as `config.toml.before-restore`; the
running app takes them as it does an edit of the file, through
`config_watch`'s hot reload, on both backends. Both run on the
`SceneTransfer` worker behind the portal's chooser.

Each saved scene has a picture beside its TOML, under the same name
(`src/scene_thumb.rs`): `scenes::save_in` draws it on the CPU from the
frames already decoded — the characters framed, each twice its size,
each thumbnail pixel the average of what it covers — and
`scenes::delete` takes it along. The Scene tab shows it as a texture
read again when the file changes (looked at every two seconds). A share
puts it in the file as `thumbnail.png`, checked first; an import
decodes it under caps and writes what it decoded beside the new scene —
or draws one from the scene on screen when the file has none or the
scene was fitted onto this desktop.

### Saving a GIF (`src/gif_export.rs`)

"Save as GIF…" on the right-click menu is `MenuAction::SaveGif`, which
the loops catch before `apply_menu_action`, as they do a scene's Share:
`SceneTransfer::save_gif` copies the character's current animation into
a `GifJob` on the UI thread — scaled to its size on screen within
`MAX_SIDE`, turned the way it faces, each frame with its own delay — and
asks the portal's `SaveFile` where; a worker encodes it with `image`'s
GIF encoder (looping forever) straight into the chosen file. GIF delays
are in hundredths of a second, so they round.

### Copy and paste (`src/clipboard.rs`)

The app keeps its own clipboard, not the desktop's: a copy is the
characters' whole configs (`Entity::to_config`), bottom of the stack
first, and the group they were when they were exactly one. It is a
process-wide value, so it outlives a scene switch (`crate::scenes`) —
cutting from one scene and pasting into another is how a character
moves between them. `Copied::placed_in` puts a paste where the copies
were, stepping 30 px down and right while that spot holds a character,
so a paste into the same scene steps like Duplicate and one into
another scene lands in place. `outcomes::paste_entities` adds them
through `append_character_config` — the entity limit and memory budget
hold — with fresh ids, on top, selected, and regrouped when a whole
group was copied. Copy is a shared action; Cut and Paste add and remove
textures, so the backends run them beside Delete and Duplicate. The
Scene tab's Paste button reaches them through `panels::SceneRequest`.

The desktop's clipboard sits beside it (`src/system_clipboard.rs`). A
copy also puts the names there, under `application/x-animaengine-characters`
and plain text (`clipboard::take_claim`, handed on by the loops); a
paste asks the desktop first and goes by its types (`system_clipboard::pick`):
ours still → the characters copied here; an image → saved under
`pasted/` in the data directory and added like a drop; `text/uri-list` →
the files, like a drop — a shared scene or a Shimeji pack to its
importer (`outcomes::Pasted::to_import`); anything else → the
characters copied here. So
whoever copied last decides, without timestamps. Native Wayland goes
through `wl_data_device` (`layer_window::clipboard`: a copy-paste
source served from the event loop, the selection offer read on a thread
of its own); X11, whose winit windows have no say in selections, through
a window and connection of its own on one thread (`src/x11_clipboard.rs`),
answering `TARGETS` and text while it owns `CLIPBOARD`, and reading
others' answers in one piece or `INCR` pieces. Reads are capped (64 MB
for an image, 64 KiB for a file list) and land a frame or two later
(`pending_paste`), with an undo step opened then.

Text fields in the panel use the same two backends (`Want::Text`): the
copied text egui reports (`OutputCommand::CopyText`, taken by the
renderers) is claimed as plain text only, and Ctrl+V in a field asks
for text and hands it to egui as `Event::Paste` a frame later — on X11
because egui-winit is built without its clipboard, on native Wayland
because the keyboard handler feeds egui itself (it now adds
`Event::Copy` and `Event::Cut` too).

### Undo (`src/undo.rs`)

Edits reach the scene from dozens of places, so undo watches *gestures*
instead of hooking each one. The first input of a gesture — a press, a
key, a drop, a screen reader's request, not pointer motion — snapshots
the scene as character configs, with its sprite groups (a delete takes
a character out of its group), before the input is applied; the gesture
closes once no button is held, no text field or list has the keyboard,
no file chooser or import is running, and `SETTLE` (350 ms) has passed.
A snapshot that differs from the scene then is one step. Characters
that move by themselves (a behavior, physics, a throw still in flight —
`Snapshot::flying`) have their positions left out of the comparison and
out of the restore. A tap closes the gesture before the poke's hop, and
a throw where it was let go: both are play rather than an edit. Restoring
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
