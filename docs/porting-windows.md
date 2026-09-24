# Porting animaEngine off Linux (Windows first)

**Status: builds, untested on real Windows.** The crate compiles for
`x86_64-pc-windows-gnu` — clippy-clean, docs included — and CI checks
that on every push (the `cross` job, alongside FreeBSD). The unit tests
pass on Windows under Wine (495 of 496; the one skipped scopes its data
directory through `XDG_DATA_HOME`, which Windows ignores). What has not
happened yet is a run of the overlay itself on a real Windows machine,
which is the step this page now exists for.

## What already works everywhere

The large majority of the codebase doesn't care which OS it runs on: the
wgpu renderer, winit windowing, the egui UI, the asset loaders
(PNG / GIF / WebP / MP4), the scene / entity / behavior model, config,
and i18n. `winit`, `wgpu`, `egui`, `global-hotkey`, and `directories`
are all cross-platform crates and build on Windows unchanged.

## The OS-specific layer, per platform

| Capability | Linux X11 | Linux Wayland | Windows |
|---|---|---|---|
| Click-through | `XShape` input region (`src/window/x11_input.rs`) | `wl_surface::set_input_region` (`src/wayland/`) | `WS_EX_LAYERED \| WS_EX_TRANSPARENT` (`src/window/win_overlay.rs`) |
| Transparent presentation | compositor honours premultiplied alpha | same | `UpdateLayeredWindow` from an offscreen render (`src/renderer/win_layered.rs`) — no Windows swapchain passes per-pixel alpha |
| Always-on-top | EWMH `_NET_WM_STATE_ABOVE` | layer-shell | `HWND_TOPMOST`, re-asserted by `win_overlay.rs` |
| Global cursor (FollowCursor in pass-through) | `XQueryPointer` | not possible (protocol) | `GetCursorPos` |
| Tray icon | `ksni` (StatusNotifierItem) | same | `Shell_NotifyIconW` (`src/win_tray.rs`) |
| Single instance | D-Bus name (`src/single_instance.rs`) | same | named mutex + a named event the second launch signals to raise the first (`src/win_instance.rs`) |
| Global hotkeys | `XGrabKey` via `global-hotkey` | GlobalShortcuts portal | `global-hotkey` (`RegisterHotKey`) |

Both trays render one menu, `src/tray_menu.rs`. The overlay operations go
through the `OverlayPlatform` trait (`src/window/overlay.rs`); Windows
rides the existing winit run loop in `src/app/`. The native Wayland loop
stays Linux/BSD-only.

The tray and the single instance use the `windows-sys` bindings winit
already links, not the `tray-icon` crate this page once suggested: one
icon and one popup menu did not justify a second copy of the Win32
bindings and a new dependency tree to audit.

## What is left

1. **Run it on Windows.** The smoke checklist below, on a real machine
   or VM. Nothing here has been seen working on screen yet except the
   layered presentation, which was measured on a VM when it was written.
2. **An MSVC build.** CI checks the GNU target from Linux; the MSVC one
   needs a Windows runner or machine.
3. **Video.** `openh264` compiles C, so the cross-check builds without
   the `video` feature. A native Windows build should build it as-is.

## Testing

From Linux, the compile and lint checks and the unit tests under Wine
are in [CONTRIBUTING.md](../CONTRIBUTING.md#windows-and-freebsd). Wine
has no GPU path worth testing the overlay on, so for everything else:

- it launches, and a second launch exits and raises the first;
- the ⚙ button toggles edit mode, and click-through reaches the desktop
  in pass-through;
- sprites stay on top, with the desktop visible around them;
- the tray icon appears; a left click toggles edit mode, a right click
  opens the menu, and Quit removes the icon (no dead icon left behind);
- restarting Explorer (`taskkill /f /im explorer.exe`, then start it)
  brings the icon back;
- the global hotkeys register;
- config lands under `%APPDATA%\animaEngine\config` and the asset
  library under `%APPDATA%\animaEngine\data\assets` (the paths the Wine
  run produced).

## macOS / BSD, for free

The same `OverlayPlatform` seam covers them:

- **macOS:** `ignoresMouseEvents` + an `NSWindow` window level; Metal
  comes free through wgpu.
- **BSD** (FreeBSD / NetBSD / OpenBSD): reuses the X11 and Wayland
  backends as they are. FreeBSD compiles today and is in the `cross` CI
  job; like Windows, it has not been run there yet.

See [stability-policy.md](stability-policy.md) for the surfaces a port
must keep working, and [architecture.md](architecture.md) for the module
map.
