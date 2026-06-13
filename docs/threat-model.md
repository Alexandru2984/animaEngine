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

### D-Bus interface — `com.animaengine.Anima`

The X11 / single-instance flavour exposes one method: `Activate()`,
which posts `AnimaEvent::RaiseWindow`.

The native Wayland flavour (E.6 in 0.5.0, hardened in F.4 in 0.5.1)
exposes four more — `ToggleEditMode`, `HideOverlay`, `ShowOverlay`,
`ToggleGlobalPlayback` — because Wayland has no `XGrabKey`
equivalent and these are the substitute that compositor bindings
call via `gdbus`. The pre-0.5.0 "single-method invariant" no longer
holds; this section documents the new surface and the mitigations
applied.

**What every method does:** flips an in-memory bool / forwards to
the scene's playback toggle. No file IO. No process spawn. No
clipboard read. Each call is one atomic state mutation per frame —
nothing more.

**Threats accepted by design:**

1. **Same-user processes can spam any method.** The session bus has
   no per-process ACL; any code running as the user can call these.
   We don't try to authenticate the caller (the right tool for that
   is xdg-portals, which is its own project). Mitigations bound the
   blast radius rather than block the call:
   - Idempotent toggles (`ToggleEditMode`, `ToggleGlobalPlayback`)
     are **coalesced per frame** in the Wayland run loop. A million
     `ToggleEditMode` calls between two frames apply the parity
     XOR once (i.e. either no flip or one flip), not a million
     flips.
   - The D-Bus → main-loop channel is a **bounded `sync_channel(64)`**;
     overflow is dropped at the sender with a warning rather than
     letting memory grow without bound between frames.
   - Visibility events (`HideOverlay` / `ShowOverlay`) keep only the
     last intent.
   - Asymmetry note: the **winit path** dispatches through
     `EventLoopProxy::send_event`, whose queue winit does not let us
     bound. Drain speed (a field write + coalesced redraw request per
     event) exceeds any realistic D-Bus delivery rate, and the caller
     is same-uid (out of scope) — accepted, but new event variants
     must stay cheap to handle for this to remain true.

2. **Activate is unchanged in semantics** — posts `RaiseWindow`. On
   Wayland this is a no-op (the layer surface is always present at
   the Overlay layer).

**Invariants going forward:**

- New methods need a threat-model review entry (PR template
  should ask). Every method is attack surface.
- Any new method must be idempotent or otherwise bounded — no
  unbounded queue-growth events.
- Rate-limit / coalesce in the consumer, not the publisher. The
  D-Bus service shouldn't drop calls silently from inside the
  service handler (that's the caller's signal).
- Don't add methods that take asset paths, exec strings, eval
  expressions, or other rich payloads. If a feature like that ever
  lands, design it as a separate object path with peer
  authentication (`org.freedesktop.DBus.GetConnectionUnixUser`)
  and a whitelist.

### Window-awareness — read-only X11 introspection (0.8.0)

`window_awareness` (off by default) connects a second X11 connection
and polls window geometry every ~300 ms to use window top edges as
physics platforms. The surface is deliberately minimal and
**read-only**:

- Only three request types are issued: `GetProperty`
  (`_NET_CLIENT_LIST`, `_NET_WM_WINDOW_TYPE`, `_NET_WM_STATE`,
  `_NET_FRAME_EXTENTS`), `GetGeometry`, and `TranslateCoordinates`.
  No `ChangeProperty`, no `SendEvent`, no input synthesis, no window
  manipulation.
- It reads only from the X server the app is already connected to —
  i.e. the user's own session, already in the trusted-display
  assumption (see Scope). It learns nothing a screenshot wouldn't.
- No data leaves the process: geometry is consumed into the physics
  floor calculation and discarded each poll. Nothing is logged above
  `trace`, nothing is persisted.
- Per-window errors are swallowed (windows race the list query); a
  torn read is corrected on the next poll. No error path writes.
- Wayland exposes no equivalent, so the feature is inert there — no
  fallback that reaches for anything more invasive.

The accepted risk is the same same-user one as everywhere else: a
process that can read window geometry could already do so directly.
New request types here need a threat-model entry; the read-only
invariant must hold.

### D-Bus accessibility tree (AT-SPI) — opt-out only

Since 0.2.0 (Phase A.9) we enable the `accesskit` feature on
`egui-winit`, which makes Linux screen readers like Orca work without
extra plumbing. The mechanism is `accesskit_unix` registering an
`org.a11y.atspi.*` object on the session bus. **Every egui widget
label, hover text, focus event, and `TextEdit` keystroke is broadcast
on that bus** so assistive technologies can read them.

What this widens, vs. pre-0.2.0:

- The Ctrl+K command-palette query is published character by character
  as the user types. A user who pastes a secret into the palette by
  mistake exposes it to any same-UID process subscribed to AT-SPI.
- Settings sidebar text (entity names, theme labels, scene list rows)
  appears in the AT-SPI tree.

What it does *not* widen:

- The `com.animaengine.Anima` surface stays minimal — five methods
  total post-0.5.0, all bounded and coalesced. AT-SPI is a
  separate, standards-required surface
  registered under `org.a11y.atspi.*` by AccessKit, not by us.
- Same-UID processes were already in the trust boundary (see "Trusted
  local user" below). AT-SPI does not extend access to a different UID.

**Operators who need to disable this** (e.g. a kiosk with no AT
requirements) can set `accesskit_enabled = false` under `[global]` —
the change applies live, no rebuild — or, for a binary that never links
the bridge at all, depend on `egui-winit` without the `accesskit`
feature and rebuild.

### No network

The binary never makes outbound network connections. No telemetry,
no crash reporting back home, no update check. The `--recover` flag
operates strictly on local files. Crash reports
(`~/.cache/animaEngine/crashes/`, newest five kept) are written and
read locally only — panic messages may contain asset paths, which is
acceptable because the file never leaves the machine unless the user
attaches it to an issue themselves. Pin this in your firewall if you
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

Exactly three actions are eligible for global registration —
`ToggleEditMode`, `HideOverlay`, `PauseAll` (`hotkeys::GLOBAL_ACTIONS`).
Their chords are **user-configurable** (defaults ship, rebindable in
Settings → Keybindings and `[keybindings.map]`), and the backend is
selectable via `[global].hotkey_backend` (`auto` probes the
GlobalShortcuts portal, then falls back to `XGrabKey` on X11). Every
other action (nudge, toggle-visible, etc.) is **in-app only** and never
globally grabbed — there's no useful global meaning without a selection.

**Privacy scope:** the global-hotkey integration registers *only* those
few chords — whichever the X server (or the portal) is told to watch
delivers *only those chords* to us, never the surrounding keystrokes.
animaEngine has no global keyboard capture, no event tap, and the
native Wayland path can't grab keys at all (compositor bindings call
our D-Bus methods instead). In-app keyboard input (edit mode, panels)
reaches us only while our window has focus, like any other application.

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
  (advisories + licenses + bans + sources). Both are **hard-fail**:
  a new advisory or a license drift in a transitive dep breaks the
  build. The exception policy is explicit:
  - Advisories without a fix yet land in `deny.toml`
    (`advisories.ignore`) with a comment naming the upstream tracking
    issue and the runtime-exposure rationale.
  - `cargo audit` also takes inline `--ignore RUSTSEC-XXXX-XXXX`
    flags in `.github/workflows/ci.yml`; same documentation rule.
  - Both exceptions are reviewed each minor release.
- `deny.toml` license allowlist is narrow on purpose — bumping a dep
  that pulls a new SPDX expression fails CI and forces a deliberate
  choice rather than silent license drift.
- `cargo fuzz` ships three targets (`keychord_parse`,
  `uri_list_parse`, `asset_type_detect`); see [docs/fuzzing.md](
  fuzzing.md). The decoders most worth covering next are
  `cache::deserialize_frames`, `video_loader::avcc_to_annex_b`, and
  the `image` crate's GIF/WebP paths.

## Build reproducibility

The `.deb` is reproducible: `cargo-deb` consumes the committed
`Cargo.lock` and a deterministic set of metadata fields, so two
maintainers building the same git tag on the same Rust toolchain
produce byte-identical packages.

The **AppImage** is *not* byte-reproducible by default — its bundled
`libxkbcommon-x11.so.0` is whatever the build host's `ldconfig` returns,
which differs across Ubuntu point releases and across distros. To
narrow this:

- A pinned build container ships at [`packaging/Dockerfile.appimage-builder`](
  ../packaging/Dockerfile.appimage-builder). Building inside that image
  ensures every maintainer produces the same artefact.
- The `SHA256SUMS-vX.Y.Z.txt` we publish in GitHub Releases is the
  hash *of the image-built AppImage*. Reproducing on a different host
  is expected to differ; reproducing inside the container should match.
- Release artefacts now carry **Sigstore build-provenance
  attestations** (`actions/attest-build-provenance`, keyless via GitHub
  OIDC) for both the `.deb` and the AppImage, verifiable with
  `gh attestation verify <file> --repo <owner>/<repo>`. This attests
  *where and how* the artefact was built (the provenance), tying it to
  the tagged commit and the release workflow. The `.deb`/AppImage also
  embed a dependency SBOM via `cargo-auditable`
  (`cargo audit bin <file>`). The `SHA256SUMS` file still pins the
  binary hash; the attestation covers the build chain on top of it.

If you're producing an AppImage for redistribution, please build inside
the container or document your build host clearly. A user who downloads
our SHA256SUMS file and reproduces locally with `make appimage` on a
different distro **will** get a different hash, and that's expected
behaviour, not tampering.

## Reporting

Use the public issue tracker. If you find something that's clearly
exploitable on a default install, open an issue first and tag it
`security`; we'll triage and ship a fix in the open. No private
security program — the project is too small for it to be meaningful.
