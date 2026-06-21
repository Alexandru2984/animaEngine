# Native Wayland — running animaEngine on wlroots compositors

The default Linux path runs through **winit + X11** (XWayland on
Wayland systems) for stability and feature parity across desktops.
A second backend, opt-in via `ANIMA_USE_WAYLAND_NATIVE=1`, uses
**`wlr-layer-shell-unstable-v1`** directly and skips XWayland.

This doc covers when you'd want the native backend, how to enable it,
and how to wire shortcuts through your compositor since the X11-style
"global hotkeys" model isn't available on a native Wayland session.

## When to prefer native Wayland

- You don't want XWayland in your session at all.
- You're on **sway / Hyprland / river / Wayfire** (wlroots-based) and
  want true overlay layer semantics — animaEngine sits above normal
  windows without reserving an exclusive zone.
- You want crisper HiDPI rendering without XWayland's scale fallbacks.

Stay on the X11 path when:

- You're on **GNOME Mutter** or **KDE KWin** — neither advertises
  `zwlr_layer_shell_v1`, so the probe fails and animaEngine falls back
  silently.
- You rely on tray icon integrations that haven't moved off
  StatusNotifierItem yet (still works on most compositors, but Mutter
  needs an extension).

## Enabling

```bash
ANIMA_USE_WAYLAND_NATIVE=1 anima-engine
```

The first log lines tell you which path took:

```
INFO  ANIMA_USE_WAYLAND_NATIVE=1 set — trying native layer-shell path
INFO  Native Wayland renderer initialized (2560×1440)
```

If the probe doesn't find `zwlr_layer_shell_v1` the binary falls back
to the X11 path automatically with a warning — same as if you hadn't
set the env var.

## What's wired

| Feature | X11 path | Native Wayland (this doc) |
|---|---|---|
| Animated sprites | ✓ | ✓ |
| Click-through (toggle ⚙) | XShape | `wl_surface::set_input_region` |
| Keyboard shortcuts (in-app) | winit input | xkb decoded via sctk |
| Drag-drop files | winit `DroppedFile` | `wl_data_device` + `text/uri-list` |
| Settings panel + presets + Keybindings tab | ✓ | ✓ |
| Perf overlay (`Ctrl+Shift+\``) | ✓ | ✓ |
| Multi-monitor info | XRandR | `wl_output` enumeration |
| `MonitorMode::PerMonitor` distribution | extra `winit::Window` per monitor | extra layer-shell surface per `wl_output` (**untested** — see below) |
| Global hotkeys | XGrabKey (`Ctrl+Shift+A/H/P`) | **GlobalShortcuts portal** (preferred) or compositor bindings + D-Bus (see below) |
| Tray icon (StatusNotifierItem) | ✓ | ✓ |
| AccessKit / AT-SPI | ✓ | toggle still works; native screen-reader pickup compositor-dependent |

## XWayland caveat: fractional / mixed scaling and click-through

When the binary runs through **XWayland** (the default on GNOME/KDE
Wayland, where `zwlr_layer_shell_v1` isn't exposed) the click-through
cutout is an X11 `XShape` input region. XWayland presents X11 clients a
single unscaled coordinate space and scales surfaces behind their back,
so on **fractional scaling** (e.g. 125 %, 150 %) or a **mixed-DPI**
multi-monitor layout (one screen at 100 %, another at 200 %) the
`XShape` region can land offset from where the compositor actually
paints the overlay — clicks may pass through where the ⚙ button looks
like it is, or vice-versa.

This is an inherent XWayland limitation, not something an X11 client can
correct. animaEngine **detects the condition at startup and logs a
warning** (`XWayland + fractional/mixed display scaling detected …`) so
the behaviour isn't a mystery. To avoid it: run a native X11 session, or
use a uniform integer scale (100 % or 200 %) across monitors. The native
Wayland path (`wl_surface::set_input_region`, wlroots compositors) is not
affected — it works in the compositor's scaled coordinate space.

## Global shortcuts on Wayland — the portal path (preferred)

Wayland has no `XGrabKey` equivalent. The sanctioned mechanism is the
**`org.freedesktop.portal.GlobalShortcuts`** desktop portal: at
startup animaEngine creates a portal session and binds its three
global actions (`toggle-edit-mode`, `hide-overlay`, `pause-all`) with
the chords from your Keybindings tab as preferred triggers. The
desktop shows a one-time approval dialog (GNOME) or a shortcut editor
(KDE); after that, presses arrive as portal signals — no X server, no
compositor config, works inside the Flatpak sandbox.

Availability: GNOME ≥ 48, KDE Plasma ≥ 5.27, wlroots compositors with
a portal backend implementing GlobalShortcuts. On sessions without
the portal, animaEngine falls back automatically (XGrabKey when an X
server exists, otherwise the D-Bus path below) — the startup log line
`Hotkey strategy: …` shows which mechanism won. Pin one explicitly
with `hotkey_backend = "portal" | "x11" | "none"` under `[global]` in
config.toml.

If you decline the approval dialog, the overlay stays fully usable:
a toast explains the downgrade, and the tray menu, the ⚙ button and
the bindings below keep working.

## Global shortcuts on Wayland — the D-Bus fallback

On wlroots compositors without a GlobalShortcuts portal backend, use
compositor-level bindings invoking the **`org.animaengine.Anima`
D-Bus interface**.

animaEngine exposes these methods on `/com/animaengine/Anima` while
running:

| Method | Effect |
|---|---|
| `ToggleEditMode` | Switch between pass-through and edit mode |
| `HideOverlay` | Drop the surface into pass-through (toggle-corner only) |
| `ShowOverlay` | (No-op on wlroots — layer surface is always present) |
| `ToggleGlobalPlayback` | Pause / resume every animation |
| `Activate` | Single-instance handshake; the second launch invokes this |

### Calling them from a shell

```bash
gdbus call --session \
    --dest com.animaengine.Anima \
    --object-path /com/animaengine/Anima \
    --method org.animaengine.Anima.ToggleEditMode
```

### sway bindings example

Put these in `~/.config/sway/config`:

```
# Anima: edit mode toggle on Mod4+Shift+A
bindsym Mod4+Shift+a exec gdbus call --session \
    --dest com.animaengine.Anima \
    --object-path /com/animaengine/Anima \
    --method org.animaengine.Anima.ToggleEditMode

# Anima: hide overlay on Mod4+Shift+H
bindsym Mod4+Shift+h exec gdbus call --session \
    --dest com.animaengine.Anima \
    --object-path /com/animaengine/Anima \
    --method org.animaengine.Anima.HideOverlay

# Anima: pause / resume playback on Mod4+Shift+P
bindsym Mod4+Shift+p exec gdbus call --session \
    --dest com.animaengine.Anima \
    --object-path /com/animaengine/Anima \
    --method org.animaengine.Anima.ToggleGlobalPlayback
```

Reload sway (`Mod4+Shift+c`) to pick up the new bindings.

### Hyprland bindings example

`~/.config/hypr/hyprland.conf`:

```
bind = SUPER SHIFT, A, exec, gdbus call --session --dest com.animaengine.Anima --object-path /com/animaengine/Anima --method org.animaengine.Anima.ToggleEditMode
bind = SUPER SHIFT, H, exec, gdbus call --session --dest com.animaengine.Anima --object-path /com/animaengine/Anima --method org.animaengine.Anima.HideOverlay
bind = SUPER SHIFT, P, exec, gdbus call --session --dest com.animaengine.Anima --object-path /com/animaengine/Anima --method org.animaengine.Anima.ToggleGlobalPlayback
```

Reload with `hyprctl reload`.

### river bindings example

`~/.config/river/init`:

```
riverctl map normal Super+Shift A spawn 'gdbus call --session --dest com.animaengine.Anima --object-path /com/animaengine/Anima --method org.animaengine.Anima.ToggleEditMode'
riverctl map normal Super+Shift H spawn 'gdbus call --session --dest com.animaengine.Anima --object-path /com/animaengine/Anima --method org.animaengine.Anima.HideOverlay'
riverctl map normal Super+Shift P spawn 'gdbus call --session --dest com.animaengine.Anima --object-path /com/animaengine/Anima --method org.animaengine.Anima.ToggleGlobalPlayback'
```

## Compositor compatibility — tested matrix

The tested-matrix column is populated as users report back via
GitHub issues. The "Spec compliance" column reflects what the
protocol guarantees regardless of whether we've personally exercised
it.

| Compositor | `zwlr_layer_shell_v1` | `wl_data_device` | Status |
|---|---|---|---|
| **sway** (≥1.8) | ✓ | ✓ | Expected to work; primary development target |
| **Hyprland** (≥0.30) | ✓ | ✓ | Expected to work |
| **river** (≥0.3) | ✓ | ✓ | Expected to work |
| **Wayfire** | ✓ | ✓ | Expected to work |
| **GNOME Mutter** | ✗ | ✓ | Probe fails → X11 fallback |
| **KDE KWin** | ✗ | ✓ | Probe fails → X11 fallback (KWin gates `wlr-layer-shell` behind a KConfig flag) |

When you hit a compositor-specific quirk, file a GitHub issue tagged
`wayland` with:

- Compositor name + version (`sway --version`, `hyprctl version`, …)
- Output of `WAYLAND_DEBUG=client anima-engine` for the first frame
- Whether the X11 fallback path works on the same machine

## Multi-monitor (`MonitorMode::PerMonitor`) — untested

`PerMonitor` mode now spawns one sprite-only layer-shell surface per
non-primary `wl_output`, mirroring the X11 path's extra windows
exactly: entities resolve to a monitor (pin first, then centroid) and
each surface draws only its own monitor's entities, translated by
that output's logical position. Extras carry no input region (fully
click-through) and no egui — same as the X11 extras.

This was written against the published protocol docs and the sctk API
(no `zwlr_layer_shell_v1` compositor with more than one output was
available during development — see the note at the top of this doc).
If you run a multi-monitor sway / Hyprland / river / Wayfire session
and something looks wrong (entities on the wrong screen, a missing
extra surface, a crash on hotplug), please file an issue tagged
`wayland` with the compositor-compatibility report template above.

## What's not (yet) parity with X11

- **Native AccessKit on wlroots.** The runtime toggle still applies;
  whether AT-SPI actually picks the surface up is compositor-side.
  GNOME Mutter routes everything via the X11 path anyway.
- **IME composition popups.** xkbcommon composes dead-keys / latin
  diacritics correctly through `KeyEvent::utf8`; full text-input-v3
  IME popups (for CJK candidates etc.) aren't wired yet.
- **`FollowCursor` in pass-through mode.** X11 polls `XQueryPointer`
  to keep this behavior accurate even when the click-through region
  excludes the rest of the surface. No Wayland protocol hands a
  client the global pointer position outside its own input region by
  design, so `FollowCursor` only tracks the cursor in edit mode here.
- **Window-awareness physics.** The X11 path polls EWMH window
  properties to use other windows' top edges as physics platforms;
  there's no Wayland equivalent, so the config knob is inert here.

## Quick sanity check

After enabling the native path, the perf overlay
(`Ctrl+Shift+\``) reads `wgpu_submit` against the real GPU rather
than going through XWayland. A side-by-side test against the X11
path is the easiest way to see if the native backend is actually
helping latency on your hardware.
