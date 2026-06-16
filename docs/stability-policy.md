# Stability policy

What animaEngine promises to keep working across a major version, and
what it explicitly doesn't. This is the user-facing half of the 1.0
contract; the security half is [threat-model.md](threat-model.md).

**Effective with 1.0.0.** Through the 0.x series there was no stability
guarantee — config shapes, flags and interfaces moved between minors.
From 1.0 onward the guarantees below hold for the life of the 1.x line.

## Guaranteed across 1.x

These are the surfaces a user, a script, or a packager can build on. A
change to any of them is a breaking change and waits for 2.0 (see
*Deprecation* below).

- **Config schema.** A `config.toml` written by any 1.x release loads in
  every later 1.x release. New fields are additive with serde defaults;
  any non-additive change ships a migration that runs on load, after
  copying the original to `config.toml.bak-v<n>`. The on-disk schema
  carries a `version` key for exactly this. Unknown `[section]` tables
  are preserved through a load → save cycle, so a config touched by a
  newer 1.x and opened by an older one doesn't lose data.
- **D-Bus interface** `com.animaengine.Anima`. The method set —
  `Activate`, `ToggleEditMode`, `HideOverlay`, `ShowOverlay`,
  `ToggleGlobalPlayback` — keeps its names and (empty) signatures.
  Methods may be *added*; existing ones aren't removed or repurposed.
- **CLI flags.** `--help` / `-h` and `--recover` / `-r` keep their
  meaning. New flags may be added; these two don't change under you.
- **Asset formats accepted.** The `asset_type` values — `png_static`,
  `png_sequence`, `gif`, `webp_animated`, `webp_static`, `spritesheet`,
  `video` — and the drag-drop extension allowlist (`png`, `jpg`,
  `jpeg`, `gif`, `webp`, `mp4`, `m4v`, `mov`) stay accepted. Formats may
  be added; an accepted format won't be dropped.
- **XDG file locations.** Config at `~/.config/animaEngine/config.toml`,
  cache under `~/.cache/animaEngine/`, the asset library under
  `~/.local/share/animaEngine/assets/` (all via the XDG base-directory
  spec, overridable by the standard `XDG_*` variables). These paths
  don't move within 1.x.

## Not guaranteed

Building on these is fine, but they can change in any release, including
a patch — don't script against them.

- **Internal crate API.** `anima_engine` is an application, not a
  library. The Rust module structure, public items, and types are
  refactored freely; there is no semver contract on `cargo doc` output.
- **Log format and levels.** The text of `tracing` output, which events
  log at which level, and the span structure are diagnostic aids, not an
  interface. Parsing logs to drive automation will break.
- **Perf-overlay numbers.** The HUD's VRAM estimate, frame timings,
  upload/draw counters and their labels are for eyeballing, not
  measurement contracts; they shift as the renderer changes.
- **Undocumented environment variables.** Only the variables documented
  in [config.md](config.md) are stable. The `ANIMA_SOAK_*` knobs and any
  other internal/testing variables can change or vanish without notice.
- **The native Wayland path's internals.** The opt-in
  `ANIMA_USE_WAYLAND_NATIVE` backend's behaviour is covered by the same
  *user-facing* guarantees above (config, D-Bus, formats), but its
  compositor-specific quirks are not a frozen contract.

## Deprecation

When a guaranteed surface genuinely has to change, it goes through a
full minor-version runway, never a silent break:

1. **Announce** the deprecation in the release notes of the minor that
   introduces the replacement.
2. **Warn at runtime** for at least one subsequent minor — the old form
   keeps working but logs a deprecation warning pointing at the new one.
3. **Remove** no earlier than the next major (2.0). Within 1.x, the old
   form always keeps working.

Config keys are the most common case: a removed/renamed key is handled
by a migration that rewrites it on load, so an old `config.toml` upgrades
silently rather than erroring.

## Reporting a break

A 1.x release that breaks one of the guarantees above is a bug, not a
feature — report it the same way as any regression (see
[CONTRIBUTING.md](../CONTRIBUTING.md)). Include the config or command
that worked on the earlier version and the one it stopped working on.
