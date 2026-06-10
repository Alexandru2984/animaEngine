# Roadmap

animaEngine ships in **thematic minor releases**: each 0.x.0 has one
clear focus area, and patch releases (0.x.y) close anything the audit
or the field uncovers afterwards. Versions land sequentially — no
parallel feature tracks.

## Shipped

| Release | Theme | Highlights |
|---------|-------|------------|
| 0.1.0   | Bootstrap | wgpu sprite quads, transparent X11 window, click-through, drag-drop |
| 0.2.0   | UI / a11y foundation | egui settings panel, themes, i18n, AccessKit, presets, command palette |
| 0.3.0   | Engine polish | multi-monitor, asset library, sprite groups, animation curves, behaviors |
| 0.3.1 / 0.3.2 | Packaging | icon resolution chain on GNOME / Ubuntu (.deb) |
| 0.4.0   | UX completion | rebindable keymap, collapse-state persistence, perf overlay, native-speaker locale review, onboarding 2.0 |
| 0.5.0   | Platform reach | native Wayland parity (wlroots), `cargo-fuzz` harness |
| 0.5.1 – 0.5.4 | Hardening | audit follow-ups: per-asset caps, redaction sweep, path canonicalisation, refactor split, MAX_ENTITIES enforcement, AT-SPI disclosure |

## In flight

| Release | Theme | Status |
|---------|-------|--------|
| 0.5.5 (current) | Post-audit doc + hardening + engine correctness | README/architecture refresh, cache key hardening (size + nanos), global memory budget, CI audit/deny hard-fail, idle-aware frame pacing, animation-timing fixes (per-frame delay walk, BounceOut), quad-budget + GPU-texture-leak fixes, tmpdir fallback ownership verification |

## Planned

| Release | Theme | Scope sketch |
|---------|-------|-------------|
| 0.6.0   | TBD — candidates below | Pick one focus area, do it well |

Candidates being weighed for 0.6:

- **Renderer polish** — texture atlas, batched draw calls, GPU-time
  metrics in the perf overlay, frame pacing. Useful only if scenes
  grow beyond the current MAX_ENTITIES = 64 cap.
- **Wayland GNOME/KDE story** — explore portal-based global hotkeys
  (xdg-desktop-portal `GlobalShortcuts`) so the native path is a
  first-class citizen on every desktop, not just wlroots.
- **Plugin behaviors** — sandboxed scripting (WASM or Lua) so users
  can write their own per-entity behaviors without rebuilding.

## Non-goals (for now)

- **macOS / FreeBSD ports** — the engine assumes Linux idioms (XDG
  paths, AT-SPI, D-Bus, wl_compositor). A port is welcome as a
  community PR; the maintainer has no hardware to test on, so it is
  not scheduled.
- **Network features** — telemetry, asset CDN, auto-update, remote
  control. animaEngine makes zero network calls and intends to keep
  it that way; see [docs/threat-model.md](docs/threat-model.md).
- **Bundled assets** — the project ships a single demo slime sequence
  for smoke-testing. Curated asset packs are out of scope; users
  point `ANIMA_ASSETS_DIR` at their own collection.

## How releases are cut

1. The theme is picked when the previous release closes its audit.
2. Sub-phases are sized so each is one self-contained commit (CI
   green, fmt + clippy + tests). 8–10 sub-phases per minor is the
   sweet spot.
3. After the last sub-phase: bump `Cargo.toml`, write the CHANGELOG
   entry, add `docs/release-notes/v<version>.md`, rebuild `.deb` /
   AppImage / Flatpak with refreshed SHA256SUMS.
4. A security audit runs against the unreleased tree before tagging.
   Findings either land in the same release or schedule the next
   patch series.

See [CHANGELOG.md](CHANGELOG.md) for the per-release detail.
