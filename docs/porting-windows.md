# Porting animaEngine off Linux (Windows first)

**Status: planned, post-1.0, help wanted.** The engine is Linux-first
today and does not compile on Windows yet. This is the map for bringing
it over — written in enough detail that a new contributor can pick it up
cold. macOS and BSD fall out of the same work; Windows is the first
target.

It's scoped for **after 1.0 ships** (see the freeze rules in
[CONTRIBUTING.md](../CONTRIBUTING.md)), but the plan lives here now so
the work is discoverable. If you want to take any step, open an issue
first so we don't double up.

## What already works everywhere

The large majority of the codebase doesn't care which OS it runs on: the
wgpu renderer, winit windowing, the egui UI, the asset loaders
(PNG / GIF / WebP / MP4), the scene / entity / behavior model, config,
and i18n. `winit`, `wgpu`, `egui`, `global-hotkey`, and `directories`
are all cross-platform crates and build on Windows unchanged.

## What's Linux-specific (the seam to abstract)

Only the **overlay integration layer** is OS-bound. The mapping to
Windows:

| Capability | Linux X11 | Linux Wayland | Windows |
|---|---|---|---|
| Click-through | `XShape` input region (`src/window/x11_input.rs`) | `wl_surface::set_input_region` (`src/wayland/`) | `WS_EX_LAYERED \| WS_EX_TRANSPARENT` via `SetWindowLongPtrW` |
| Always-on-top | EWMH `_NET_WM_STATE_ABOVE` | layer-shell | winit `WindowLevel::AlwaysOnTop` → `HWND_TOPMOST` (already works) |
| Global cursor (FollowCursor in pass-through) | `XQueryPointer` | not possible (protocol) | `GetCursorPos` |
| Tray icon | `ksni` (StatusNotifierItem / D-Bus) | same | `tray-icon` crate (`Shell_NotifyIcon`) |
| Single-instance | `zbus` D-Bus name (`src/single_instance.rs`) | same | named mutex (`CreateMutexW` + `ERROR_ALREADY_EXISTS`) |
| Global hotkeys | `XGrabKey` via `global-hotkey` | GlobalShortcuts portal | `global-hotkey` already does `RegisterHotKey` |

These are wired inline behind `#[cfg(target_os = "linux")]` today — only
nine gates, in `src/main.rs`, `src/app/windows.rs`, and
`src/app/lifecycle.rs`.

## The plan — three steps, in order

### 1. Extract an `OverlayPlatform` trait (the keystone)

Pull the seam above behind one trait so a Windows backend slots in
without touching the rest of the app:

```rust
trait OverlayPlatform {
    fn set_click_through(&mut self, region: Option<InputRect>) -> Result<()>;
    fn query_global_cursor(&self) -> Option<(f32, f32)>;
    fn acquire_single_instance() -> SingleInstance;
    fn register_tray(/* event proxy */) -> Option<TrayHandle>;
    fn register_global_hotkeys(/* bindings */) -> HotkeyStatus;
}
```

Refactor the existing X11 and Wayland code into `X11Backend` /
`WaylandBackend` impls. This is a **pure refactor — no behaviour
change** — and it's the bulk of the effort. Windows (and macOS) ride the
**existing winit run-loop** in `src/app/`; they do *not* need a new
event loop. The native Wayland path (`src/wayland/run.rs`) stays
Linux/BSD-only.

### 2. Gate the Linux-only dependencies

Every platform dep is unconditional in `Cargo.toml` today, which is why
the crate won't even compile on Windows. Move these under a target
section:

```toml
[target.'cfg(unix)'.dependencies]
x11rb = { version = "0.13", features = ["shape"] }
wayland-client = "0.31"
smithay-client-toolkit = { version = "0.19", default-features = false, features = ["xkbcommon"] }
ksni = { version = "0.3", default-features = false, features = ["async-io"] }
zbus = { version = "5", default-features = false, features = ["async-io"] }
```

and `#[cfg]`-gate the `src/wayland/` and `src/window/x11_*` modules.
`winit` / `wgpu` / `egui` / `global-hotkey` / `directories` stay
unconditional. After this, `cargo check --target x86_64-pc-windows-msvc`
gets past dependency resolution.

### 3. Implement the Windows backend

A `WindowsBackend: OverlayPlatform`, e.g. `src/platform/windows.rs`:

- **Click-through:** set `WS_EX_LAYERED | WS_EX_TRANSPARENT` on the HWND
  (`SetWindowLongPtrW(GWL_EXSTYLE, …)`). Edit mode clears
  `WS_EX_TRANSPARENT` so the window catches input; pass-through restores
  it. A per-region cutout for the ⚙ corner can use `SetWindowRgn` if the
  whole-window toggle isn't enough.
- **Always-on-top + transparency:** winit `with_transparent(true)` +
  `WindowLevel::AlwaysOnTop` already do the right thing through DWM.
- **Tray:** the `tray-icon` crate.
- **Single-instance:** a named mutex; for "second launch raises the
  first", a named pipe or a registered `WM_COPYDATA` message.
- **Global hotkeys:** `global-hotkey` works as-is (`RegisterHotKey`).
- **Paths:** `directories` already returns `%APPDATA%` /
  `%LOCALAPPDATA%`.

## Testing

This needs a real Windows machine or VM — it can't be meaningfully built
or run by cross-compiling from Linux (the C deps, notably `openh264`,
want a Windows toolchain). Smoke checklist: it launches, the ⚙ button
toggles edit mode, click-through reaches the desktop, sprites stay on
top, the tray icon appears, hotkeys register, and config lands under
`%APPDATA%`.

## macOS / BSD, for free

The same `OverlayPlatform` seam covers them:

- **macOS:** `ignoresMouseEvents` + an `NSWindow` window level; Metal
  comes free through wgpu.
- **BSD** (FreeBSD / NetBSD / OpenBSD): reuses the X11 and Wayland
  backends almost verbatim — mostly widening `cfg(unix)` and adding CI.

See [stability-policy.md](stability-policy.md) for the surfaces a port
must keep working, and [architecture.md](architecture.md) for the module
map.
