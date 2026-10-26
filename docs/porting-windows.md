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
| Global hotkeys | `XGrabKey` (`src/hotkeys/x11_grab.rs`) | GlobalShortcuts portal | `global-hotkey` (`RegisterHotKey`) |

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
   or VM. Under Wine the release build installs, starts, draws its
   characters through the layered window, registers its tray icon,
   raises the first copy on a second start, and uninstalls clean — but
   Wine is not Windows.
2. **Signing.** The `.exe` and the installer are not signed, so
   SmartScreen warns on every download until enough people have run
   them. The ways to sign are listed under "Releasing" below.
3. **An MSVC build.** CI builds the GNU target from Linux; the MSVC one
   needs a Windows runner or machine. `build.rs` already compiles the
   resources with `rc.exe` there.

## Releasing

`scripts/build-windows.sh` builds, from Linux, what a release ships for
Windows:

- `animaEngine-<version>-windows-x86_64.zip`: the portable app,
  `anima-engine.exe` with `LICENSE.txt`, `THIRD-PARTY-LICENSES.txt` and
  a `README.txt`. Reproducible: two builds of a commit give the same
  bytes.
- `animaEngine-<version>-windows-x86_64-setup.exe`: an NSIS installer
  (`packaging/windows/installer.nsi`). It installs per user into
  `%LOCALAPPDATA%\Programs\animaEngine`, without asking for
  administrator rights. It adds a Start-menu entry and an uninstaller
  that Settings -> Apps lists, and speaks the app's ten languages. A
  running copy is asked to close first, and saves as it would on Quit.
  Uninstalling keeps the settings and characters, and removes "Start at
  login".

The `.exe` is cross-compiled with mingw-w64. libstdc++ (the C++ runtime
of the H.264 decoder) is linked in statically, from the win32 thread
model's, so the `.exe` needs only DLLs every Windows has; the script
fails if it would need another. `build.rs` gives it the icon (also the
tray's) and the version details its Properties show. The release
workflow runs the script, checksums both files with the Linux packages,
attests them and attaches them to the draft.

"Start at login" is the `animaEngine` value of
`HKCU\Software\Microsoft\Windows\CurrentVersion\Run` — what Task
Manager's Startup tab lists too. Its test writes the registry, so it is
`#[ignore]`d: run it under Wine in a throwaway prefix with `-- --ignored`.

**Signing**, for when the app is distributed — each with its own cost and
checks, to compare at the time:

- an OV or EV code-signing certificate from a certificate authority
  (`osslsigncode` signs from Linux);
- Azure Trusted Signing;
- for open-source projects, SignPath Foundation;
- the Microsoft Store, which signs what it distributes (it needs an MSIX
  package).

A signature does not end SmartScreen's warnings at once: they stop as
the signed file builds up a reputation (an EV certificate used to skip
that, but no longer does). Sign the installer and the `.exe` inside it.

## Testing

From Linux, the compile and lint checks and the unit tests under Wine
are in [CONTRIBUTING.md](../CONTRIBUTING.md#windows-and-freebsd). The
installer and the app run under Wine too, in a throwaway prefix
(`WINEPREFIX`, `HOME` and a private `Xvfb` of their own, and
`winemenubuilder.exe=d` so the Start-menu entry does not reach your own
desktop's menu). On a real Windows machine,
[`packaging/windows/test-kit/Test-AnimaEngine.ps1`](../packaging/windows/test-kit/README.md)
goes through most of this list — installing, running, asking what only
a person can see, uninstalling — and leaves a report to send back; a
second launch, restarting Explorer and a real sign-in are still by hand:

- the installer runs without asking for administrator rights, in the
  language Windows is set to; the Start-menu entry starts the app;
  Settings -> Apps lists it and uninstalls it;
- SmartScreen's warning, while the files are unsigned;

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
  run produced);
- "Start at login" (Appearance) starts it at the next sign-in, and Task
  Manager's Startup tab shows it;
- a 125 % or 150 % display scale, and two monitors.

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
