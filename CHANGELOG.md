# Changelog

All notable changes to animaEngine are recorded here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions
follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1] — 2026-06-03

Patch release. Fixes a startup crash on systems that don't have
`libxkbcommon-x11` preinstalled — surfaced when the smoke test on a
minimal Xvfb container panicked while AccessKit tried to `dlopen` the
library. Affected anyone who downloaded the 0.2.0 AppImage on a
distro without the package, or installed the 0.2.0 `.deb` and didn't
already pull in `libxkbcommon-x11-0` transitively.

### Fixed

- **AppImage now bundles `libxkbcommon-x11.so.0` explicitly**.
  `linuxdeploy` walks ELF NEEDED tags, but the library is opened via
  `dlopen` from `accesskit_unix`, so it never showed up.
  `scripts/build-appimage.sh` now resolves a candidate path via
  `ldconfig` on the build host and passes it through `--library`.
- **`.deb` declares `libxkbcommon-x11-0` in `depends`**. Previously
  relied on `$auto`, which only inspects NEEDED tags and missed the
  dlopen.
- **CI installs `libxkbcommon-x11-dev`** across `clippy`, `test`,
  `build`, and `smoke` jobs so the smoke test under Xvfb actually
  reaches the event loop.

### Security hardening (from the post-Faza A audit)

- **AT-SPI exposure documented** in [`docs/threat-model.md`](
  docs/threat-model.md). The AccessKit bridge enabled in 0.2.0
  broadcasts every egui widget label and the Ctrl+K palette query
  on the session bus. Same-UID processes were already in the trust
  boundary; the note makes the *what* visible. Opt-out path is to
  depend on `egui-winit` without the `accesskit` feature and rebuild.
- **Locale rejection now logs** via `tracing::warn!` in
  `i18n::init` (when `config.toml` carries an unknown code) and
  `i18n::set_locale` (when a direct caller passes a non-SUPPORTED
  code). Silent fall-back masked tampering before.
- **SECURITY-tagged comment** added to `Scene::reset_to_configs`
  and `Scene::append_character_config` documenting that callers
  must pre-validate `asset_path` if it comes from outside the
  hardcoded preset set. No code change — preserves the
  drag-drop-validation invariant from 0.1.0's Faza B.
- **AppImage reproducibility envelope** documented and a pinned
  build container shipped at
  [`packaging/Dockerfile.appimage-builder`](
  packaging/Dockerfile.appimage-builder). Maintainers building
  inside the container produce byte-identical artefacts; building
  on a different host is expected to differ from our published
  SHA256SUMS.

### Notes for 0.2.0 downloaders

If your 0.2.0 install runs fine, you don't have to upgrade — your
distro had the library either preinstalled or pulled in by another
package. If it crashed at startup with `Library libxkbcommon-x11.so
could not be loaded`, either install `libxkbcommon-x11-0` manually or
download 0.2.1.

## [0.2.0] — 2026-06-02

UI/UX polish release. Faza A (A.0-A.11) landed on top of the 0.1.0 +
B series — twelve sub-phases focused on coherence, accessibility,
and localisation rather than new sprite features. The engine renders
the same content; the chrome around it is dramatically nicer.

### Added

**Design system (Faza A.0):**
- `docs/design-system.md` — single source of truth for colours,
  typography, spacing, radii, icons, motion, and component patterns
- Every panel now references token constants instead of hardcoded
  hex values / magic numbers

**Theme system (Faza A.1, A.9):**
- Four themes: Dark, Light, Dark · High contrast, Light · High contrast
- Theme persisted in `GlobalConfig.theme`
- HC variants clear WCAG AAA (≥ 7:1) on every text tier, thicken
  the focus ring to 3 px, and zero out animation time for
  motion-sensitive users
- CI enforces contrast thresholds via unit tests

**Iconography (Faza A.2):**
- `egui-phosphor` icon font wired through `src/ui/icons.rs` — every
  in-app glyph is a named constant for grep-ability
- New "Ghost Mascot" app icon ([data/anima-engine.svg](data/anima-engine.svg))
  selected from three exploratory variants in
  [packaging/icon-variants/](packaging/icon-variants/)

**Settings sidebar redesign (Faza A.3):**
- Three tabs (Inspector / Scene / Appearance) with sticky header,
  scrollable body, entity-count footer
- Inspector has collapsible sections (Position / Appearance /
  Animation / Behavior) and quick-toggle row for Visible/Gravity
- Tab selection persists through `egui::Memory`

**Empty / error / loading states (Faza A.4):**
- `src/ui/states.rs` with reusable `empty`, `error`, `spinner`
  helpers
- Toasts redesigned per design-system §7.8: bg.elevated surface,
  semantic-coloured severity icons, radius_lg, theme-aware

**Micro-animations (Faza A.5):**
- `src/ui/anim.rs` with `ease_in_quad` / `ease_out_quad` / `lerp`
- Toast slide-in (200 ms ease-out-quad) + fade-out
  (300 ms ease-in-quad) computed off `Toast::age()`
- Tab cross-fade (100 ms) via `ctx.animate_value_with_time`
- Hover and focus transitions ride egui's built-in animation_time

**Onboarding (Faza A.6):**
- Three dismissible inline hints for fresh users (tabs, V/G shortcuts,
  theme instant-apply)
- Existing users (configs pre-A.6) skip all hints via `#[serde(default = ...)]`

**Preset library (Faza A.7):**
- Six curated presets using only the shipped demo assets:
  Cozy Companion, Productivity Zen, Halloween Party,
  Birthday Confetti, Studio Session, Cursor Follower
- Append / Replace modes; Append suffixes IDs with `_a/_b/...` to
  avoid collisions

**Keyboard map + command palette (Faza A.8):**
- `src/ui/keyboard.rs` Action enum with label + description +
  default combo (21 actions)
- Read-only keyboard reference table in the Appearance tab
- **Ctrl+K command palette** — fuzzy search across themes and
  presets, executes via `PaletteOutcome` back into `App`

**Accessibility (Faza A.9):**
- AccessKit bridge enabled via egui-winit feature → AT-SPI events
  on Linux for every egui widget
- High-contrast theme variants (see Theme system above)
- `docs/accessibility.md` documenting commitments and non-goals

**Internationalisation (Faza A.10):**
- `fluent-rs` backend with `OnceLock<RwLock<State>>` for constant-time
  locale switching
- Ten locales shipped (`en`, `ro`, `es`, `de`, `fr`, `it`, `pt-BR`,
  `pl`, `nl`, `ja`) covering ~45 keys each
- Auto-detect from `LANG` / `LC_ALL` / `LC_MESSAGES` with sensible
  fallback chain (exact match → language-only → English)
- Language picker in Appearance tab with autonyms (Română, 日本語, …)
- Explicit selection persists in `GlobalConfig.locale`

**Polish (Faza A.11):**
- Selection pulse on the active scene-list row (2 s sine, low
  amplitude, accent-coloured left stripe)
- Hardcoded values from earlier sub-phases swept into token
  references where missed

### Changed

- `Theme::label()` returns `String` (was `&'static str`) — locale-aware
- `Toast::expires_at` field replaced by `Toast::age()` / `remaining()`
  helpers backed by `created_at + lifetime`
- `panels::settings()` signature now takes `&mut Theme`,
  `&mut Option<String>`, `&mut OnboardingProgress`
- Settings sidebar default width: 280 px → 320 px

### Tests

- **119 tests** passing (up from 90 in 0.1.0)
- WCAG contrast assertions on every theme variant
- FTL syntax + key coverage parity assertions across all 10 locales

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
