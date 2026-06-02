# Changelog

All notable changes to animaEngine are recorded here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions
follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Audit hardening landed on top of the 0.1.0 series — see the Faza B
entries below.

## [0.1.0] — 2026-06-02

First development release. All twelve faze landed end-to-end; the
binary is daily-driver-usable on X11 (and XWayland) with an opt-in
native Wayland path.

### Added

**Foundation (Faza 0-1):**
- Typed `AnimaError` (`thiserror`) — no more `Box<dyn Error>`
- CI: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`,
  smoke test under Xvfb
- Procedural demo assets factored into `src/demo/`
- `tracing` everywhere (replaced `log`); spans on non-hot operations

**Bug fixes that became invariants:**
- **Physics opt-in per entity** (default off, `G` to toggle)
- **Alpha hit-testing** — clicks on transparent corners no longer
  select circular sprites
- **`Frame::resized` returns `Result`** — no more silent fallback to
  a zero-filled frame at the wrong dimensions
- **Async hot-reload** — UI thread never blocks on asset decoding
- **Recursive dimension validation** on PNG sequences
- **Input-shape re-apply** on focus regain and unocclude

**Performance (Faza 2):**
- Parallel PNG-sequence decode via rayon
- Cached visibility/z-order (filter + sort runs only on invalidation)
- On-disk RGBA cache at `~/.cache/animaEngine/textures/`
- Spritesheet slicing uses row-stride memcpy

**UI (Faza 3):**
- egui integration via `egui-wgpu`; ⚙ button is now an egui widget
- Right-side settings panel: inspector + scene list
- Right-click context menu (Duplicate / Reset / Gravity / Z-order /
  Delete)
- Toast notifications for save / add / delete / hot-reload

**Behaviors (Faza 4):**
- `Idle` (default), `WalkAround`, `FollowCursor`, `BoundedWander`
- `TickContext` struct, per-entity `BehaviorState`, deterministic
  per-id PRNG seed for bounded wander

**Asset ecosystem (Faza 5):**
- H.264 MP4 video loader (`mp4` + `openh264`, no system deps,
  capped at 20 s / 600 frames)
- Sample pack: ghost, slime, heart, star, cat (all procedural)

**System integration (Faza 6):**
- System tray via `ksni` (StatusNotifierItem, no libappindicator)
- Global hotkeys `Ctrl+Shift+A/H/P` via `global-hotkey`
- Single-instance D-Bus handshake (`com.animaengine.Anima`); a second
  launch raises the existing window

**Native Wayland (Faza 7, opt-in):**
- Compositor probe (logs whether `zwlr_layer_shell_v1` is available)
- `sctk`-driven layer surface + wgpu bridge
- Pointer events translated to `egui::Event` (keyboard deferred)
- `wl_surface::set_input_region` click-through, matching the X11 path
- `ANIMA_USE_WAYLAND_NATIVE=1` to opt in

**Packaging (Faza 8):**
- `.desktop` + scalable SVG icon + AppStream metainfo
- `make install` with `DESTDIR` / `PREFIX` support
- AppImage build via `linuxdeploy` (`make appimage` ≈ 7 MB output)
- `.deb` via `cargo-deb` (`make deb` ≈ 5 MB output)
- Flatpak manifest at `flatpak/com.animaengine.Anima.yml` plus
  `make flatpak`

**Docs + recovery (Faza 9):**
- README rewritten; `CONTRIBUTING.md`, `docs/architecture.md`,
  `docs/config.md` added
- Panic hook + `--recover` flag — last clean config snapshot survives
  a crash; restore copies it back over the live config

### Hardening (Faza B, audit response)

- **`MAX_ANIMATION_FRAMES`**, **`MAX_SEQUENCE_FILES`**,
  **`MAX_DECODED_ASSET_BYTES`**, **`MAX_ASSET_FILE_BYTES`** added to
  `src/constants.rs` and applied at every decoder boundary
- GIF / WebP / PNG sequence / video all truncate at the frame and
  total-bytes caps with explicit warn logs
- Video loader rejects openh264 frames whose dimensions exceed
  `MAX_IMAGE_DIM`; uses `checked_mul` for the pixel-count product
- `validate_single_file` is **fail-closed** on known image
  extensions (PNG / JPEG / GIF / WebP)
- `util::atomic_write_bytes` — `AppConfig::save` and `cache::try_save`
  now write via tmp + rename so a crash mid-save can't truncate
  either file
- Drag-drop pre-validation rejects unsupported extensions, oversized
  files, directories, and files without an extension before the
  decoder spins up
- `cache::deserialize_frames` enforces count / dimension / cumulative
  byte caps — a malicious cache file can't trick us into a huge
  preallocation

### Removed

- `AppConfig::detect_asset_type` — duplicated and inconsistent with
  `animation::loader::detect_asset_type` (which now covers JPEG +
  video too). Callers use the loader version.

### Telemetry

- **Zero.** No network calls, no analytics, no crash reporting back
  home. The `--recover` flag operates strictly on local files.

## Release process

Tags are `vMAJOR.MINOR.PATCH`. Each release should bump
`Cargo.toml::version`, add a new `[X.Y.Z] — YYYY-MM-DD` section
above with the changes since the last tag, then `git tag -a vX.Y.Z`.
