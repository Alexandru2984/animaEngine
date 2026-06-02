# Threat model

This document is the short version of "what does animaEngine promise
to keep safe, what does it deliberately not, and what should change if
you want to harden it further." It's deliberately short — the codebase
is small enough that the actual security surface is mostly the file
loaders and a couple of D-Bus interfaces.

If you're reading this because you found a bug, the right place to
report it is the public issue tracker. We don't run a private security
program.

## Scope

animaEngine is a single-user desktop overlay. It assumes:

- The binary runs as the user who launched it (no setuid, no sudo).
  **Don't run it as root.** Nothing in the codebase guards against
  privilege misuse — if you launch it as root you give it access to
  every user file on the system.
- The local D-Bus session bus is trusted (every user gets their own
  session bus; only processes owned by the same user can talk on it).
- The X11 / Wayland display server is trusted. Other processes on the
  same display can already trivially read your screen and inject input;
  the overlay engine doesn't change that.
- The on-disk config at `~/.config/animaEngine/config.toml` and the
  cache at `~/.cache/animaEngine/` are user-writable. They're not
  considered hostile inputs, but the loaders below still validate
  them because programs misbehave.

## What it does try to prevent

These are explicit invariants enforced by the code; regressions in
them are bugs, not features.

### Resource exhaustion from asset files

Caps in `src/constants.rs`:

| Limit | Value | Enforced by |
|-------|-------|-------------|
| `MAX_IMAGE_DIM` | 4096 px | `validate_image_dimensions`, `cache::deserialize_frames`, `video_loader` |
| `MAX_ANIMATION_FRAMES` | 600 | gif/webp/video/cache loaders |
| `MAX_VIDEO_FRAMES` | 600 | video loader |
| `MAX_SEQUENCE_FILES` | 1000 | png_sequence loader (enumeration) |
| `MAX_DECODED_ASSET_BYTES` | 512 MB | gif/webp/png_sequence/video/cache |
| `MAX_ASSET_FILE_BYTES` | 200 MB | drag-drop pre-validation |
| `MAX_ENTITIES` | 64 | `AppConfig::load` truncation |
| `MAX_DROP_SIZE` | 256 px | resize on drag-drop |
| `MAX_QUADS` | 64 | renderer batch |

A pathological GIF / WebP / video / PNG sequence cannot run us out of
memory at parse or decode time. Truncation is logged at `warn` level.

### Drag-drop rejection at the boundary

`app::pre_validate_dropped_file` runs **before** the decoder for every
`WindowEvent::DroppedFile`:

- Refuses directories
- Refuses anything larger than `MAX_ASSET_FILE_BYTES`
- Refuses extensions outside `[png, jpg, jpeg, gif, webp, mp4, m4v, mov]`
- Refuses files without an extension at all

The user sees a clear `Rejected: …` toast on failure.

### Atomic writes for stateful files

`util::atomic_write_bytes` is used by `AppConfig::save` and
`cache::try_save`. A crash or power loss mid-write leaves the old
file intact — never a half-written `config.toml` or cache.

### Fail-closed dimension probe

`validate_single_file` fails closed when the extension is a known
image format (`png`, `jpg`, `jpeg`, `gif`, `webp`) but the header
probe can't read dimensions. Unknown extensions defer to the
format-specific loader.

### Cache file deserialization

`cache::deserialize_frames` enforces `MAX_ANIMATION_FRAMES`,
`MAX_IMAGE_DIM`, and `MAX_DECODED_ASSET_BYTES` *before* allocating.
A malicious cache file claiming `count = 1_000_000` or a 100 000 ×
100 000 frame is rejected up front, not after a `Vec::with_capacity`
or a `to_vec()`.

### Decompression-bomb guard

Image header probing happens before decode. Any PNG / JPEG / GIF /
WebP with a header claiming dimensions larger than `MAX_IMAGE_DIM`
is rejected without allocating the pixel buffer. Same check applied
to video frames coming out of openh264.

### D-Bus single-instance handshake

The single-instance service exposes **exactly one** method:
`Activate()`, which posts `AnimaEvent::RaiseWindow` to the event loop.
That's it. No load-asset, no eval-config, no quit-remote.

**Invariant (intentional):** new methods on `com.animaengine.Anima`
must not be added without explicit threat-model review. Every method
is a piece of attack surface the way `Activate` is small. If something
needs to be richer, design it as a separate object path with
authentication.

### No network

The binary never makes outbound network connections. No telemetry,
no crash reporting back home, no update check. The `--recover` flag
operates strictly on local files. Pin this in your firewall if you
care.

## What it does NOT try to prevent

### Trusted local user

Any process running as the same user can:

- Read or write `~/.config/animaEngine/config.toml` and the
  `~/.cache/animaEngine/` tree.
- Call `Activate()` on our D-Bus name to make us focus / show our
  window.
- Send arbitrary input via the X11 display (or the Wayland portal,
  if granted).

Defending against a co-resident attacker is outside scope — they
already have the user's privileges.

### Path traversal in config

`AppConfig.resolve_asset_path` accepts absolute paths, `~`-expansion,
and paths relative to the executable or `cwd`. A malicious hand-edited
`config.toml` can therefore make us try to open arbitrary paths the
user can read. This is **deliberate** — users routinely want to point
at assets under `~/Pictures/`, network mounts, etc. We don't currently
chroot or restrict to an asset directory.

**Mitigation that's planned:** an "asset library" model where config
loads from a registered asset dir by default and absolute paths
require an explicit per-character opt-in. Not in 0.1.0; see audit
finding #6 in the project notes.

### Global hotkeys

`Ctrl+Shift+A/H/P` are hardcoded. They can conflict with other apps'
bindings and there's no UI to rebind them. Configurable hotkeys are
planned for the UI/UX polish phase.

### Side-channel / display attacks

If another process on the user's display can see our window contents
or read pixel data, that's a property of the display server, not us.
Use a screen lock and don't share your display with untrusted users.

### Wayland native path

The `ANIMA_USE_WAYLAND_NATIVE=1` code path is **experimental** and
deliberately less hardened:

- Keyboard events aren't translated (sctk requires `libxkbcommon-dev`,
  which we don't bundle).
- egui UI isn't wired in.
- Pointer events are buffered but discarded.

Use the X11 path for daily-driver work; the Wayland path is a
correctness preview, not a hardened production target.

## Supply chain

- `Cargo.lock` is **committed** so reproducible builds match what CI
  runs.
- CI runs `cargo audit` (RustSec advisory DB) and `cargo deny check`
  (advisories + licenses + bans + sources). Both are
  `continue-on-error: true` for now so a freshly-disclosed advisory
  on a transitive dep doesn't break PRs that touched nothing related;
  flip that to `false` once we settle on an exception policy.
- `deny.toml` license allowlist is narrow on purpose — bumping a dep
  that pulls a new SPDX expression fails CI and forces a deliberate
  choice rather than silent license drift.
- We don't run `cargo fuzz` yet. The decoders most worth fuzzing are
  `cache::deserialize_frames`, `video_loader::avcc_to_annex_b`, and
  the `image` crate's GIF/WebP paths. Targets are tracked as a
  post-0.1.0 task.

## Reporting

Use the public issue tracker. If you find something that's clearly
exploitable on a default install, open an issue first and tag it
`security`; we'll triage and ship a fix in the open. No private
security program — the project is too small for it to be meaningful.
