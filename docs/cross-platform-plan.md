# Cross-platform plan — Linux → BSD → Windows → macOS

Master roadmap for taking animaEngine cross-platform. Grounded in an
audit of the current tree, not aspiration. Companion to
[porting-windows.md](porting-windows.md) (help-wanted detail for the
Windows backend once the seams exist).

## Principles

1. **Linux stays first-class.** No target regresses the Linux paths.
2. **Only the OS-specific *seams* get abstracted**, behind traits, with
   the existing Linux code as the first implementation. The portable
   core (below) is not touched.
3. **Trait-first, then a cheap second target (BSD), then the expensive
   ones (Windows, macOS).** BSD reuses the X11/Wayland backends, so it
   validates the trait boundary before we pay for Win32/Cocoa.
4. **Hard gate: no backend/refactor code lands until `v1.0.0` is
   tagged.** We are in RC bake; the port is post-1.0 by design. This doc
   is the exception — it is docs-only and freeze-safe.

## Portable core (unchanged by the port, ~90% of the code)

These already work on Windows/macOS as-is and are not part of the port:

- **winit** — window creation is already cross-OS (Win32/Cocoa/X11/Wayland).
- **wgpu** — DX12 / Metal / Vulkan; the renderer is backend-agnostic.
- **egui** + the whole `ui/` layer.
- Asset pipeline: `animation/` (png/gif/webp/jpeg/mp4), `image`, decode caps.
- `config` (paths via the `directories` crate — already per-OS), `scene`,
  `physics`, `behavior`, `entity`, `group`, `shimeji` import, `i18n`.
- Video: `openh264` builds from source — already cross-platform.

## The OS-specific surface (5 seams)

From the audit — small and well-isolated:

| # | Seam | Linux today | Files | Windows target | macOS target |
|---|------|-------------|-------|----------------|--------------|
| 1 | **Overlay window + input** (click-through, always-on-top, input shape, multi-output) | X11 XShape/EWMH (`X11InputManager`) + wlr-layer-shell | `window/`, `wayland/layer_window/` | `WS_EX_LAYERED\|WS_EX_TRANSPARENT`, `HWND_TOPMOST`, region/hit-test | `NSWindow` borderless + level + `ignoresMouseEvents` |
| 2 | Tray | ksni / StatusNotifierItem (D-Bus) | `tray.rs` | `Shell_NotifyIcon` | `NSStatusItem` |
| 3 | Global hotkeys | XGrabKey (`global-hotkey`) + Wayland portal | `hotkeys/` | `RegisterHotKey` | `RegisterEventHotKey` / `CGEventTap` |
| 4 | Single-instance | D-Bus bus name | `single_instance.rs` | named mutex + `WM_COPYDATA`/pipe | lock file + distributed notification |
| 5 | Private-dir perms | unix `0600`/`0700` | `util.rs` | ACL (or no-op + note) | unix perms (same as Linux) |

Seam 1 is the real work; 2–5 are small and mostly mechanical.

## Trait sketch (the B2 design — signatures will firm up during C1)

A `platform` module holding one core trait plus four small sibling
traits. Sketch, not final API:

```rust
// Seam 1 — the big one. Applied to a winit Window after creation.
pub trait OverlayPlatform {
    /// Make the window a click-through, always-on-top overlay:
    /// skip taskbar/pager, no focus steal, top layer.
    fn configure_overlay(&mut self, window: &Window) -> Result<()>;

    /// Set the interactive input region — the union of areas that
    /// should receive clicks (⚙ button, open panels, context menu).
    /// Everything else passes through to the desktop below. Empty =
    /// fully click-through. X11: input-shape rects; Wayland:
    /// wl_surface input_region; Win32: layered region / hit-test;
    /// macOS: ignoresMouseEvents toggle + content hit-test.
    fn set_input_regions(&mut self, window: &Window, regions: &[Rect]) -> Result<()>;

    /// Connected outputs in global desktop coordinates (multi-monitor
    /// placement + per-output surfaces).
    fn outputs(&self) -> Vec<OutputInfo>;
}

// Seam 2
pub trait TrayBackend {
    fn spawn(&mut self, model: TrayModel) -> Result<TrayHandle>;
    fn update(&mut self, handle: &TrayHandle, model: TrayModel) -> Result<()>;
}

// Seam 3
pub trait HotkeyBackend {
    fn register(&mut self, chords: &[Chord]) -> Result<()>;
    fn poll(&mut self) -> Vec<HotkeyEvent>;
}

// Seam 4 — formalises the existing AcquireOutcome shape.
pub trait SingleInstance {
    fn acquire_or_signal(&self, action: LaunchAction) -> AcquireOutcome;
}

// Seam 5 — a free fn, unix impl = chmod, Windows impl = ACL/no-op.
pub fn harden_private_dir(path: &Path) -> Result<()>;
```

The current seam already exists: `window/platform.rs::DisplayServer`
picks X11 vs native Wayland, and `X11InputManager` / `layer_window`
apply the overlay behaviours. C1 generalises that into `OverlayPlatform`.

## Roadmap

### Track A — Close 1.0 *(gating; maintainer + homelab; ~days)*
- **A1** Tag `v1.0.0-rc3` on HEAD (the glibc fix must be in the tagged commit).
- **A2** rc3 bake — short, focused on import + hot-reload (the churned paths).
- **A3** Run `install-verification-1.0.md` against the rc3 artifacts
  (Ubuntu 22.04, Debian 12, Fedora) and fill the log.
- **A4** At the final tag: bump Flathub `v0.9.0`→`1.0.0`, AUR
  `0.5.5`→`1.0.0`, add AppStream screenshots, verify each channel builds.
- **A5** Tag `v1.0.0`, drop prerelease.

### Track B — Freeze-safe groundwork *(now, parallel; docs only)*
- **B1** ✅ OS-surface audit (done — this table).
- **B2** ✅ This document + trait sketch.

### Track C — The port *(starts after the `v1.0.0` tag, trait-first)*
- **C1** 🔑 Extract `OverlayPlatform` from `window/platform.rs`; move X11
  (`X11InputManager`) and Wayland (layer-shell) behind it. **Pure
  refactor, Linux-only, all tests green.** Biggest de-risking step. *(~1 wk)*
- **C2** Abstract seams 2–5 (tray, hotkeys, single-instance, perms), Linux
  impl as the first implementor. *(~1 wk)*
- **C3** **BSD** backend — cheap validation (reuses X11/Wayland; mostly
  CI + deps). Proves the trait boundary before Win32/Cocoa. *(~days)*
- **C4** **Windows** backend: Win32 layered/transparent window +
  topmost; `Shell_NotifyIcon` tray; `RegisterHotKey`; named-mutex
  single-instance; wgpu DX12; MSI/portable packaging. *(~2–3 wk)*
- **C5** **macOS** backend: borderless `NSWindow` + level +
  `ignoresMouseEvents`; `NSStatusItem`; `RegisterEventHotKey`/
  `CGEventTap`; screen-recording permission; wgpu Metal; notarization +
  code signing in CI; `.app`/`.dmg`. Hardest. *(~3–4 wk)*
- **C6** Cross-OS CI matrix + per-OS release pipeline.

## Risks & notes

- winit already does Win32/Cocoa window creation, so that part is "free".
  The real work is the overlay *behaviours* (click-through, topmost,
  shape) via raw handles — exactly what `X11InputManager` does today.
  This is why C1 (the trait) is the linchpin of the whole port.
- macOS: the hard parts are `NSWindowLevel` + permissions + notarization
  (CI bureaucracy, not code) — not rendering (Metal via wgpu is fine).
- Window-awareness physics (`platforms.rs` + `window/x11_windows.rs`) is
  X11-only by nature (no global window geometry on Wayland). On
  Windows/macOS it degrades to screen-floor physics, same as native
  Wayland does today — not a blocker.
- Don't touch `openh264`/video decode; already cross-platform.

## Non-negotiable ordering

`v1.0.0` tag → C1 → C2 → C3 (BSD) → C4 (Windows) → C5 (macOS) → C6.
Nothing in Track C is a bug fix, so none of it is freeze-legal before the
1.0 tag.
