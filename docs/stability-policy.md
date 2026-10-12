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
  carries a `version` key for exactly this, and configs written by
  released versions themselves are kept as test fixtures
  (`tests/fixtures/configs`). Going the other way, from 1.5 on, an older
  1.x keeps what it cannot read of a newer config: unknown `[section]`
  tables, list entries (characters, groups, reminders, schedule rules),
  `[global]` settings and shortcuts that do not read are set aside and
  written back on save, and so are unknown keys inside the tables it
  knows (`[global]`, a character, its talk and hours, a group, a
  window). 1.4 and earlier cannot read a
  1.5 config at all; see [config.md](config.md#schema-version).
- **D-Bus surface.** The bus name `com.animaengine.Anima`, object path
  `/com/animaengine/Anima`, and interface `org.animaengine.Anima` are
  fixed. (The bus name and the interface are deliberately different
  namespaces: `com.…` matches the Flatpak app-id, the interface keeps
  the conventional reverse-DNS `org.…` — a `gdbus` binding needs both,
  e.g. `--dest com.animaengine.Anima … --method org.animaengine.Anima.ToggleEditMode`.)
  The method set — `Activate`, `ToggleEditMode`, `HideOverlay`,
  `ShowOverlay`, `ToggleGlobalPlayback`, and since 1.5 `NextScene`,
  `Snooze(u minutes)`, `Wake`, `StartFocus(u minutes)`, `StopFocus`,
  `SetMuted(b muted)` and `ToggleMute` — keeps its names and signatures. Methods may be *added*; existing
  ones aren't removed or repurposed.
- **CLI flags.** `--help` / `-h`, `--recover` / `-r` and `--version` /
  `-V` keep their meaning. New flags may be added; these
  don't change under you.
- **Asset formats accepted.** The `asset_type` values — `png_static`,
  `png_sequence`, `gif`, `webp_animated`, `webp_static`, `spritesheet`,
  `video` — and the drag-drop extension allowlist (`png`, `jpg`,
  `jpeg`, `gif`, `webp`, `mp4`, `m4v`, `mov`) stay accepted. Formats may
  be added; an accepted format won't be dropped.
- **XDG file locations.** Config at `~/.config/animaengine/config.toml`,
  cache under `~/.cache/animaengine/`, the asset library under
  `~/.local/share/animaengine/assets/` (all via the XDG base-directory
  spec, overridable by the standard `XDG_*` variables). These paths
  don't move within 1.x. All three are lower-case on Linux and the BSDs:
  the XDG lookup lower-cases the application name. Until 1.2 this page,
  the README and most of the docs spelled the first two `animaEngine`,
  which is not a directory the app has ever read — the paths themselves
  never changed. On Windows the folder is `%APPDATA%\animaEngine\`.

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
