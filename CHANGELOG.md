# Changelog

All notable changes to animaEngine are recorded here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions
follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **CI hardening** — five new gating jobs: rustdoc with `-D warnings`
  (broken intra-doc links fail the build), MSRV check on Rust 1.88
  (now declared as `rust-version` in Cargo.toml), cargo-machete
  (unused direct dependencies), desktop metadata validation
  (`desktop-file-validate` + `appstreamcli` — the icon-resolution
  regression class from 0.3.2 can no longer land silently), and a
  weekly beta-toolchain canary (schedule-only, never blocks a push).
  The test job now runs under cargo-nextest with `--retries 1`, so an
  intermittent failure is reported FLAKY *by name* instead of
  anonymously failing the run — this pulls the flake-naming harness
  planned for 0.9 (W.1/W.2) forward. The smoke test runs inside
  `dbus-run-session`, exercising the single-instance, tray and portal
  D-Bus paths instead of short-circuiting on "no session bus".
- **CI hardening, round 2** — workflow hygiene: read-only token
  (`permissions: contents: read`), `concurrency` with
  cancel-in-progress (stale runs stop costing minutes), no global
  `RUSTFLAGS: -D warnings` (it leaked into `cargo install` of CI
  tools), `--locked` on every cargo invocation,
  `persist-credentials: false` on every checkout, and an actionlint
  job validating the workflows themselves. The smoke test now runs
  the *release artifact* handed over from the build job instead of
  paying for a second debug build. Two new weekly canaries: headless
  Weston smoke — the first automated environment ever to run the
  pure-Wayland fallback path (field-unvalidated since 0.6) — and an
  lcov coverage artifact via cargo-llvm-cov.
- **Criterion benchmarks** — seven benches over the per-frame hot
  paths (`Scene::tick` at 10/50/100 entities, visible-list rebuild,
  cache codec, window planning, group transform), all synthetic and
  disk/GPU-free. Current numbers: `scene_tick/100` ≈ 8.5 µs against
  the 8 ms engine budget. CI compiles them on every push; a weekly
  canary runs them and archives the criterion history as an artifact.
- **Crash reports** — a panic now writes a local report
  (`~/.cache/animaEngine/crashes/`, version + message + location +
  backtrace, newest five kept) in addition to the existing config
  snapshot; the next launch shows a one-time toast pointing at the
  file. Reports never leave the machine (zero-telemetry policy
  unchanged) — without this, a crash of a desktop-launched instance
  left no trace at all.
- **Release automation** — pushing a `vX.Y.Z` tag now builds the
  .deb + AppImage in CI, generates checksums, attests build
  provenance (Sigstore via GitHub OIDC — verifiable with
  `gh attestation verify`), and opens a *draft* GitHub release with
  the notes from `docs/release-notes/`; publishing stays a human
  click. Both packaging scripts embed a full dependency SBOM in the
  binary when `cargo-auditable` is installed (always, in CI).
- **Video decode round-trip test** — the openh264 decoder FFI was the
  one component no test executed. A programmatic fixture (frames
  encoded with openh264's encoder, muxed by mp4's Mp4Writer — no
  binary lands in the repo) now drives the full `load_video` pipeline
  and asserts pixel colors after the YUV→RGBA conversion.

### Fixed

- **The recurring CI flake has a name and a fix** —
  `perf::tests::p95_returns_top_quantile` asserted p95 ≥ mean over
  100 wall-clock-timed near-zero frames; a single descheduled
  iteration made the mean exceed p95. The test now injects synthetic
  frame totals and asserts exact quantile values. (Named by the
  nextest retry harness on its first occurrence after installation.)
- **MP4 video loading for avcC-only files** — the SPS/PPS parameter
  sets extracted from the container were wiped by a buffer reuse
  before the decoder ever saw them, so any MP4 without in-band
  parameter sets (most of them — mainstream encoders don't repeat
  SPS/PPS in-stream) decoded zero frames and was rejected as empty.
  Present since the video loader first shipped; caught immediately by
  the new round-trip test.

### Changed

- Four unused direct dependencies removed (`wayland-protocols`,
  `wayland-protocols-wlr`, `calloop`, `calloop-wayland-source` — all
  consumed through smithay-client-toolkit's re-exports; sctk's
  `calloop` feature dropped likewise). Doc-comment link fixes
  surfaced by the new rustdoc gate.

## [0.7.0] — 2026-06-12

Content & feature completion (Faza U). The release's spine is the
multi-state animation engine; on top of it, Shimeji pack import gives
animaEngine access to an existing ecosystem of thousands of
community-made desktop mascots. The last two "lands later" promises
from 0.3 (library thumbnails, group composition) are gone — nothing
in the UI is stubbed anymore.

### Added

- **Multi-state animations** — entities carry per-state sequences
  (`idle` / `walk` / `fall` / `drag`) declared as additive
  `[characters.animations.<state>]` TOML tables; legacy
  single-sequence configs load byte-identically and never gain the
  key on save. State selection is behavior-driven with a strict
  priority (drag > falling > walking > idle); missing states fall
  back to idle; switches rewind the target sequence and re-upload
  the texture. Horizontal facing follows motion and persists when
  the entity stops; the renderer mirrors sprites via UV flip — no
  texture duplication.
- **Shimeji pack import** — drop a pack folder (`conf/` + `img/`)
  onto the overlay, or paste its path in the Library tab. The
  importer parses `actions.xml` (quick-xml: no DTD expansion by
  construction; size/depth/attribute caps; image paths
  canonicalised under the pack root), maps Stay/Move/Fall/Dragged
  onto the four states, derives walk speed from pose velocities and
  per-state fps from pose durations, and copies sprites into
  `<library>/imported/<slug>/` so deleting the source folder can't
  orphan the scene. Anything unmappable is skipped with a written
  reason (toast summary + log). Full spec: `docs/shimeji-import.md`.
- **Library thumbnails** (closes C.5) — generated on a background
  thread at startup (first frame, 64 px, mtime-stale-checked), shown
  in the asset rows with a capped texture cache; video assets keep
  the film glyph by design.
- **Group composition** (closes C.9) — a group's `offset_x/y` +
  `scale` now actually transform its members: in the renderer (all
  three paths), in the hit-test (clicks land where pixels are,
  including alpha sampling at effective scale), and in the inspector
  (effective-transform hint).

### Changed

- The library scanner skips the importer-managed `imported/`
  directory — individual frames of imported sequences don't flood
  the asset grid.
- The aggregate memory budget counts every state's frames, not just
  the active one.

## [0.6.0] — 2026-06-12

Platform completeness (Faza T). The two oldest platform gaps close:
global shortcuts on GNOME/KDE Wayland sessions via the desktop
portal, and the multi-monitor data model from 0.3 finally getting a
render-side implementation. Released without the portal/multi-monitor
field validations (no compatible test environment available) — see
"Known limitations" in the release notes; every unvalidated path
carries an automatic fallback to the previously tested behavior.

### Added

- **GlobalShortcuts portal backend** — on sessions exposing
  `org.freedesktop.portal.GlobalShortcuts` (GNOME ≥ 48, KDE Plasma ≥
  5.27), global hotkeys bind through the portal: works on native
  Wayland, no XWayland needed, survives the Flatpak sandbox. The
  handshake runs on a background thread (the permission dialog can't
  block startup); denial falls back to XGrabKey where an X server
  exists, with a toast explaining the downgrade. Strategy override:
  `hotkey_backend = "auto" | "portal" | "x11" | "none"` under
  `[global]`. The Keybindings tab shows the live backend. The native
  Wayland path gains real global hotkeys for the first time.
- **Window-per-monitor rendering** (closes C.3) — `MonitorMode::
  PerMonitor` (the default since 0.3) now spawns one overlay window
  per monitor: entities render on the monitor their position/pin
  resolves to, with global-desktop coordinates translated per
  window. Single-monitor machines degenerate to one window,
  behaviourally identical to before. `Span` keeps the exact pre-0.6
  single-window path.
- **Monitor hotplug** — topology changes are detected on the idle
  heartbeat; windows rebuild, pins naming vanished monitors fall
  back to position resolution, toasts summarise the change.
- **Suspend/resume + stall protocol** (`docs/soak-testing.md`) —
  including the scripted SIGSTOP/SIGCONT stall test and the
  documented fact that Linux suspend freezes `CLOCK_MONOTONIC`, so
  resume creates no animation backlog by construction.

### Changed

- Renderer split into process-wide shared GPU state (device,
  pipeline, entity texture cache) and per-window surface state — an
  entity visible on two monitors uploads its frames once.
- Mouse coordinates are tracked in global desktop space; input from
  any overlay window routes through one translation point.
- Extra (non-primary) windows are fully click-through in
  pass-through mode — the ⚙ toggle is a primary-window affordance.

## [0.5.5] — 2026-06-10

Docs refresh + hardening sweep prompted by two external reviews of the
post-0.5.4 tree, plus an engine-correctness pass that came out of
verifying their claims against the code. The first review flagged
stale wording in README / architecture docs (Wayland status claims
pre-dated the E.1–E.5 work), soft-fail supply-chain checks in CI, a
cache key with second-level granularity that could mask same-second
edits, and a missing *aggregate* memory budget on top of the per-asset
cap. The second caught two subtle animation-timing bugs (multi-frame
skip with per-frame delays; non-monotonic BounceOut producing negative
frame intervals) and the world-writable-`/tmp` weakness in the XDG
fallback dirs. Auditing those claims surfaced three more issues the
reviews missed: a quad-budget off-by-two that silently dropped sprites
from a full 64-entity scene, GPU textures leaking on preset Replace,
and the render loop redrawing at 60 Hz around the clock even for a
fully static overlay. None of these were exploitable on a cold start.

### Performance

- **Idle-aware frame pacing** — the render loop previously requested
  a new frame unconditionally, so the overlay re-rendered an
  unchanged scene at display refresh 24/7. Scheduling is now derived
  from live state each frame: display-refresh only while something
  animates per-tick (edit mode, toasts, autonomous behaviors,
  physics, perf overlay), deadline-based for playing sprite
  animations (an 8 fps sprite wakes the loop 8×/s instead of 60×/s),
  and a 2 s hot-reload heartbeat when the scene is fully static —
  zero GPU work between heartbeats. Input events (mouse, keyboard,
  drag-drop, tray actions, egui interactions) wake the loop
  immediately, and config hot-reload keeps applying while the
  overlay is hidden.

### Fixed

- **Frame skip ignored per-frame GIF/WebP delays** —
  `Animation::tick` computed how many frames to skip by dividing the
  whole elapsed span by the *current* frame's duration, then advanced
  the clock by that same duration × count. Correct for fixed-FPS
  assets; wrong for GIF/WebP per-frame delays (and easing-distorted
  intervals), where a stall could land on the wrong frame with a
  desynced clock. Skipped frames are now walked one at a time, each
  consuming its own duration, bounded at two full loops — beyond that
  (system suspend) the clock resyncs instead of replaying backlog.
- **BounceOut easing shortened the loop** — `bounce_out` is not
  monotonic (the ball descends between bounces), so raw boundary
  deltas in `frame_interval` went negative on descending segments and
  the animation layer clamped them to 1 ms, silently shortening the
  loop below `n / fps`. Intervals are now |delta| normalised by the
  loop's total |delta|: every interval non-negative, sum exactly
  `total`, monotonic curves bit-identical to before.
- **Full scene dropped sprites at the quad cap** — `MAX_QUADS` (64)
  equalled `MAX_ENTITIES`, but the renderer reserves two slots for UI
  overlays mid-loop, so a legal 64-entity scene silently dropped the
  last sprites and logged a warning **every frame** (60 lines/s in
  journald). `MAX_QUADS` is now `MAX_ENTITIES + 3` (worst case: 64
  sprites + selection ring + edit bar with the conservative in-loop
  reservation) and the overflow warning fires once per episode.
- **GPU textures leaked on preset Replace** — `reset_to_configs` (
  preset gallery Replace, palette Replace) swaps the entity list
  wholesale, but none of its three call sites dropped the old
  entities' GPU textures, so VRAM grew on every Replace. Both render
  loops now run `prune_stale_textures` — a two-integer-compare no-op
  in steady state that retains only textures whose entity still
  exists.
- **Stale-index panic trap in action dispatch** — selection-driven
  keyboard actions indexed `entities[idx]` directly; the
  deselect-on-removal invariant holds everywhere today, but any
  future removal path that forgets it would turn a keypress into a
  panic. All dispatch arms now go through `get`/`get_mut` and no-op
  on a stale index.
- **`/tmp` fallback dirs could be pre-created by another local
  user** — when XDG resolution fails, config/cache/data fell back to
  `$TMPDIR/animaEngine-<uid>`; `/tmp` being world-writable (sticky
  bit), another local user could pre-create that exact path and own
  it, landing every atomic write in a directory they control. The
  fallback now prefers `$XDG_RUNTIME_DIR` (0700 + uid-owned by spec)
  and otherwise creates the tmpdir with mode 0700 and verifies it
  (real directory, not a symlink, owned by our uid, no group/other
  bits) before trusting it, retrying once with a pid-suffixed
  sibling on failure.
- **Video loader logged full asset paths** — three `warn!`/`info!`
  lines in `video_loader.rs` missed the M4/G.1 redaction sweep; they
  now log the redacted filename with the full path at `debug!`.
- **Launcher icon rendered as a blank tile since 0.2.0** — the app
  SVG started with an XML prolog + a long design comment, and
  gdk-pixbuf (the pipeline GNOME Shell renders launcher icons
  through) detects format by sniffing the first bytes of the file:
  librsvg's loader signature is anchored at the start and never saw
  `<svg`. Every previous verification used `rsvg-convert` / Inkscape
  / image viewers — all of which parse the XML properly and rendered
  the icon fine, masking the bug. The SVG now starts with the literal
  bytes `<svg` (design notes moved inside the element), the packaged
  icon variants got the same treatment, and the constraint is
  documented in `packaging/icon-variants/README.md`.

### Documentation

- README now declares **Status: 0.5.4**, all `.deb` / AppImage example
  commands use the matching filenames, and the Wayland section ships
  a canonical Feature × Backend table marking the X11/XWayland path
  as stable and the native Wayland path as opt-in beta with explicit
  notes about wlroots coverage and XWayland fallback on GNOME/KDE.
- `docs/architecture.md` Native-Wayland status table now reflects
  E.1 (keyboard) / E.4 (drag-drop) / E.5 (egui events) shipping, and
  flips global hotkeys from `❌` to `⚠ via D-Bus + compositor binding`
  with the rationale (Wayland refuses raw `XGrabKey`-style grabs).
- New `SECURITY.md` documents the reporting channel (private GitHub
  advisory), the supported-version policy (latest minor only), the
  link to the threat model, and the disclosure timeline.
- New `ROADMAP.md` enumerates shipped releases by theme (0.1 →
  0.5.4), the in-flight 0.5.5 hardening, and the 0.6 candidates
  under consideration (renderer polish / portal hotkeys / plugin
  behaviors), plus an explicit non-goals list (macOS / FreeBSD,
  network features, bundled assets).

### Fixed

- **Cache key collisions on same-second edits** — the on-disk asset
  cache hashed `canonical_path + mtime_seconds`, which collapsed
  every sub-second mtime into the same key. A `cargo build && edit
  asset && cargo build` cycle inside one second could load stale
  RGBA on the second run. Key now hashes
  `canonical_path + mtime_nanos + size + child_count`: nanoseconds
  for ext4 / btrfs / xfs / APFS resolution; size to disambiguate
  FAT32 / SMB mtime flooring; child count to catch add / remove of
  a frame in a PNG sequence even when neither mtime nor total size
  moves measurably. Existing cache files become orphans on first
  upgrade and the next decode rebuilds them — no user action needed.
- **Supply-chain checks soft-failed in CI** — `audit` and `deny`
  jobs carried `continue-on-error: true` from the days when the
  exception policy was still being settled. Both now hard-fail; an
  explicit exception policy lives in `docs/threat-model.md`
  §"Supply chain" and `deny.toml`'s `advisories.ignore` list. The
  existing inline ignore for `RUSTSEC-2024-0436` (`paste`
  unmaintained, transitive through wgpu macOS metal + accesskit
  Windows AT bridge, no exposure on Linux X11/Wayland) stays in
  place with the rationale documented inline.

### Added

- **Global decoded-RGBA memory budget** — the per-asset cap
  (`MAX_DECODED_ASSET_BYTES = 512 MB`) multiplied by
  `MAX_ENTITIES = 64` allowed a 32 GB worst case when a hostile
  config (or accidental mass-drop) loaded the maximum on every
  slot. A new aggregate cap defaults to **1 GB total** across the
  scene; `Scene::add_entity_from_path` and
  `Scene::append_character_config` reject loads that would push
  the running total above the cap and surface an error toast.
  Override at startup with `ANIMA_MEMORY_BUDGET_MB=<int>` (for
  high-RAM machines that want headroom for many small assets); the
  variable is read once per push and validated for non-zero
  positive parses, falling back to the default on anything
  malformed. New helper `Scene::total_decoded_bytes()` reports the
  current running total for future perf-overlay surfacing.

## [0.5.4] — 2026-06-09

Follow-up patch after a third-round security audit that ran against
the codebase post-refactor (the `app.rs` / `panels.rs` /
`keybindings.rs` / `layer_window.rs` modules each became a directory
with focused sub-files). The refactor itself introduced no
regressions; the audit caught two HIGH and two MEDIUM gaps that had
been latent since earlier releases and that the prior passes missed,
plus two LOW items found during manual review. Take this update at
your leisure; nothing here is an exploitable hot patch.

### Fixed

- **H1** — `AppConfig::config_path()` had kept the original
  `HOME=.` fallback that the **M3** (0.5.2) pass closed in
  `perf.rs` and `asset_library/mod.rs` but missed in `config.rs`.
  A wrapper script like `HOME=/etc/cron.d anima-engine` could
  therefore still redirect `atomic_write_bytes` on `config.toml`
  into an attacker-chosen directory whenever
  `directories::ProjectDirs` returned `None` (rare but reachable
  in minimal containers / broken envs). Fallback now matches the
  rest of the codebase: `std::env::temp_dir().join("animaEngine-<uid>")`.
- **H2** — runtime entity push paths bypassed `MAX_ENTITIES`.
  The cap was only enforced in `AppConfig::load()`; drag-drop,
  library "Add to scene", duplicate, preset Append, and
  context-menu duplicate all reached `Scene::add_entity_from_path`
  / `Scene::append_character_config` directly. Both functions now
  gate the push against `MAX_ENTITIES` and return an error
  toast when the cap is hit, foreclosing a sustained drop-flood
  scenario from spawning arbitrary entities until OOM.
- **M1** — the **M4** (0.5.2) redaction sweep + **G.1** (0.5.3)
  follow-up missed three info-level traces newly visible after
  the refactor: `scene.rs::add_entity_from_path`,
  `app/lifecycle.rs` asset-library scan summary, and
  `app/render_loop.rs` perf-snapshot export. All three now go
  through `crate::drop_validate::redact_path` at `info!`; full
  paths remain at `debug!`. The perf-snapshot toast still shows
  the full path because the user just requested the export.
- **M2** — `i18n::t()` / `set_locale()` used `.expect("i18n state
  poisoned")` on the inner `RwLock`. `t()` runs on every frame
  from every panel, so a poisoned lock would take the render
  thread down on the next translation. Today no writer can
  poison the lock (only `set_locale` is a writer and it can't
  panic mid-write), but the risk was latent and the inner state
  is trivially recoverable — bundles are immutable `Arc`s. Both
  call sites now recover via `unwrap_or_else(|poisoned|
  poisoned.into_inner())`.
- **L2** — library "Add to scene" log lines used
  `outcome.relative_path` directly. A hand-edited `library.toml`
  with RTL-override or zero-width chars could plant visually
  reversed strings in journald. Same threat model as **G.3**
  but on the log side instead of egui input; redact + downgrade
  to `debug!` for the raw form.
- **L3** — `KeyCode::from_str` had two `.unwrap()` calls whose
  preconditions were enforced by surrounding `if` and match-arm
  guards. Sound today but a refactoring trap; replaced with
  `.expect("…")` strings that name the invariant explicitly.

### Tests

- 231 lib + 25 integration + 1 demo = 257 pass.

## [0.5.3] — 2026-06-08

Follow-up to the 0.5.2 release after a re-audit that scrutinised
the F.x / M.x fixes themselves and swept the parts of the codebase
that hadn't been touched recently. Re-audit verdict: zero
regressions introduced, zero new critical / high. Two medium and
four low findings; all addressed here.

### Fixed

- **G.1** — completes the M4 path-redaction sweep. The first pass
  in 0.5.2 covered the drop / library / Wayland paths but left
  11 more info-level traces leaking absolute paths (`config.rs`
  load + save, `main.rs` startup banner, `scene.rs` instrument
  macro, every `loader.rs` info!, `png_sequence.rs` warnings).
  All now go through `drop_validate::redact_path`; the full path
  is available at `RUST_LOG=anima_engine=debug`.
- **G.3** — extends the F.7 control-character filter on the
  Wayland `egui::Event::Text` path to also strip Unicode
  category Cf (zero-width chars U+200B–U+200F + U+FEFF, the
  bidi-override block U+202A–U+202E, soft hyphen U+00AD, and the
  invisible-format range U+2060–U+206F). Same threat model as the
  original F.7: prevents a process simulating keystrokes from
  storing display-spoofing or invisible characters in
  `TextEdit` widgets (preset names, library tags, search field).
- **G.4** — PNG sequence loader applies `MAX_DECODED_ASSET_BYTES`
  *during* the parallel decode (`AtomicUsize` running total +
  per-frame reservation) instead of after. Pre-fix, a hostile
  asset directory of 1 000 × 4 K PNGs could push transient
  decode memory to ~33 GB before the post-hoc cap kicked in.
- **G.5** — search-box `TextEdit` widgets (library tab, command
  palette) gain `char_limit(256)`. Caps an otherwise unbounded
  buffer if a clipboard inject pastes a huge string.
- **G.6** — D-Bus `try_send` overflow drops on the Wayland
  service path now log at `debug!` with the method name and the
  channel error, so a deluged operator sees in trace output why
  their `gdbus` calls aren't taking effect. Kept at `debug` (not
  `info` / `warn`) so a flood attacker can't amplify their volume
  into our own log.
- **G.7** — asset library scanner canonicalises both root and
  each candidate before recording `LibraryAsset.path`, and drops
  entries whose canonical form escapes the canonical root.
  Library tree containing a symlink to `/etc/` now ignores those
  files at scan time instead of listing them in the UI (the
  M2 "Add to scene" gate already rejected them at decode time).

### Hygiene

- **G.2** — `// SAFETY:` comments added to the three
  `unsafe { libc::getuid() }` blocks introduced in 0.5.2
  (`perf.rs`, `asset_library/mod.rs` ×2) so the codebase's
  unsafe-block convention is honoured uniformly.

### Tests

- 231 lib + 25 integration + 1 demo = 257 pass. Existing PNG
  sequence tests cover the new par-iter early-bail path.

## [0.5.2] — 2026-06-08

Follow-up to the 0.5.1 security patch. Closes the four medium-severity
audit findings that were filed as "accepted / deferred" in 0.5.1 plus
a documentation cleanup. All fixes target small information-disclosure
or race-window surfaces that didn't rise to the urgency of 0.5.1's
critical bundle.

### Fixed

- **M2** — `Path::join` in the asset-library "Add to scene" path
  could lift the resolved target out of `library_root` when a
  hand-edited `library.toml` carried an absolute path or a `../`
  segment. New `resolve_library_asset` helper canonicalises both
  sides and rejects anything that escapes; reachable from
  `app.rs::handle_library_outcome`.
- **M3** — fallback when `directories::ProjectDirs` is unreachable
  no longer honours `$HOME`. The new fallback is
  `std::env::temp_dir().join("animaEngine-<uid>")`, so a wrapper
  script like `HOME=/etc/cron.d anima-engine` can't redirect writes.
  Applies to perf-snapshot exports and the asset-library data/cache
  paths.
- **M4** — info-level traces for drag-drop, asset spawn, and
  library reject paths now log just the file name; the full absolute
  path is downgraded to `debug!` (off by default). Reduces home
  directory + private dir leakage into journald / syslog.
- **M5** — `util::tmp_sibling` now embeds `std::process::id()` so two
  animaEngine instances that race past a missed single-instance lock
  can't truncate each other's temp files mid-write.

### Changed

- Documentation sweep: "Faza X" replaced with "Phase X" in all
  English-language docs (CHANGELOG, release notes, threat model,
  architecture, AppStream metainfo). Stray "Claude" mentions in the
  D.4 locale-audit docs replaced with neutral "automated AI
  cross-check" / "the LLM" language; the audits' methodology is
  unchanged, just the framing.
- `Cargo.toml` promotes `libc` to a direct dependency. It was
  already transitive everywhere; the direct declaration makes the
  `getuid()` call in the M3 fallback honest.

### Tests

- 3 new unit tests in `drop_validate`: library-resolve accepts
  in-root, rejects absolute outside root, rejects `../` escape.
- 231 lib + 25 integration + 1 demo = 257 pass.

## [0.5.1] — 2026-06-08

Security patch following an audit of the 0.5.0 native Wayland
backend. Eight findings addressed — three critical, four high,
one informational. All issues are specific to the opt-in
`ANIMA_USE_WAYLAND_NATIVE=1` path; the default X11 backend is
unaffected.

Users on the default X11 backend can take this update at their
leisure. **Users running `ANIMA_USE_WAYLAND_NATIVE=1` should
upgrade promptly** — three of the fixes close DoS / unbounded-
allocation paths reachable by other local processes.

### Fixed

- **C1 (critical)** — Wayland drag-drop now runs the same
  `pre_validate_dropped_file` gate as the X11 path: 200 MB size
  cap, extension whitelist, regular-file check. Pre-0.5.1 the
  Wayland path skipped this gate, so an oversized file with a
  whitelisted extension could reach the decoder.
- **C2 (critical)** — drop-reader worker thread bounded: payload
  capped at 64 KiB via `Read::take`, at most 4 concurrent worker
  threads (enforced by an `AtomicUsize` counter with a RAII
  decrement guard), and the result channel switched from
  `mpsc::channel` (unbounded) to `mpsc::sync_channel(16)` with
  `try_send` so a stuck consumer can't grow memory.
- **C3 (critical)** — D-Bus `org.animaengine.Anima` queue on the
  Wayland path switched from unbounded to `sync_channel(64)`,
  with `try_send` at the publisher and idempotent-toggle
  coalescing at the consumer. A spammy session-bus peer
  (`gdbus call … ToggleEditMode` in a tight loop) drops overflow
  events instead of growing memory between frames.
- **H2 (high)** — URI-list parser caps paths at `MAX_URI_LIST_PATHS
  = 256`. Previously a payload of arbitrary-many `file://` lines
  produced an arbitrary-large `Vec<PathBuf>`.
- **H3 (high)** — `percent_decode` on the Wayland drag-drop path
  now reconstructs UTF-8 properly instead of folding each escaped
  byte into a Latin-1 codepoint. Files with diacritics in their
  names (`anima%C8%9Bie.png` → `animație.png`) now drop correctly;
  pre-fix they silently failed. Strict UTF-8 — malformed sequences
  are rejected rather than substituted.
- **L1 (informational)** — control characters from xkbcommon's
  `KeyEvent::utf8` are filtered out before `egui::Event::Text` so
  a `TextEdit` doesn't store `\x01` etc.

### Documentation

- **H1** — `docs/threat-model.md` updated for the expanded D-Bus
  surface. The pre-0.5.0 "single-method invariant" is replaced by
  an explicit accepted-threat section documenting that
  same-user processes can spam any method, the per-frame
  coalescing + bounded queue together bound the blast radius,
  and any future method needs a threat-model review entry.

### Tests

- New unit tests in `wayland::data_device`: cap-at-256, multi-byte
  UTF-8 round-trip, invalid-UTF-8 rejection.
- New unit tests in `drop_validate`: regular-file / extension /
  size / missing-extension cases (moved + expanded from app.rs).
- Fuzz harness upgraded — `uri_list_parse` now asserts the output
  bound; `keychord_parse` now asserts canonical-string round-trip.

Test suite: 228 lib + 25 integration + 1 demo = 254 pass.
`cargo clippy --all-targets -- -D warnings` clean.

## [0.5.0] — 2026-06-08

Phase E — platform reach (Linux-first half). The native Wayland
backend reaches feature parity with the X11 path on wlroots
compositors; a small `cargo-fuzz` harness covers the parsers that
sit closest to untrusted input. macOS and FreeBSD ports are
explicitly **out of scope for 0.5** — without hardware to verify
on, scaffolding those would be promising support we can't honour.

### Added — native Wayland backend (`ANIMA_USE_WAYLAND_NATIVE=1`)

- **Keyboard input** via sctk's `xkbcommon` feature — full keysym
  decoding, UTF-8 composition through xkb dead-key engine,
  modifier tracking. Letters, digits, named keys we dispatch on,
  and the five punctuation symbols animaEngine binds all map onto
  `egui::Key`. Bare-letter chords stay in-app (the X11
  `XGrabKey` global path doesn't exist on Wayland).
- **Pointer input** wired all the way through to egui with the
  cached modifier mask, so `Shift+Click` etc. work.
- **Click-through input region** flips in lock-step with edit-mode
  (`Action::ToggleEditMode`) — pass-through reveals only the ⚙
  corner, edit-mode opens the whole surface.
- **Drag-drop** via `wl_data_device` accepting `text/uri-list` from
  any GTK/Qt/Nautilus/Nemo source. A worker thread drains the
  receive pipe so the wayland event queue doesn't block; the parsed
  paths route through the same `Scene::add_entity_from_path`
  validation gate as the X11 path.
- **Egui paint integration** on top of the sprite layer. New
  `WaylandEguiRenderer` wires `egui::Context` + `egui_wgpu::Renderer`
  to the layer surface; settings panel, command palette, toasts,
  context menu — all painted.
- **Settings panel parity** with the X11 path: Inspector, Scene,
  Library (display only — index population deferred), Appearance,
  Keybindings tabs all available. The Ctrl+K command palette works,
  toasts render, banners appear.
- **D-Bus global-shortcut bridge** — `org.animaengine.Anima`
  exposes `ToggleEditMode`, `HideOverlay`, `ShowOverlay`,
  `ToggleGlobalPlayback`, `Activate`. Compositor bindings (sway /
  Hyprland / river) call these via `gdbus` to mimic X11 global
  hotkeys. See [docs/wayland.md](docs/wayland.md) for snippets.
- **Multi-monitor info** via `wl_output` enumeration (logical
  position / size / scale). The inspector's monitor picker shows
  the live list; per-monitor placement on the layer surface itself
  is queued for a later release.
- **HiDPI**: egui's `pixels_per_point` follows the largest
  compositor scale advertised among bound outputs.

### Added — infrastructure

- **`cargo-fuzz` harness** under [`fuzz/`](fuzz/) with three
  initial targets: chord-string parser, drag-drop URI list parser,
  asset-type detector. Invariant: never panic on adversarial input.
  Runtime requires nightly; see [docs/fuzzing.md](docs/fuzzing.md)
  for the CI snippet.

### Docs

- [docs/wayland.md](docs/wayland.md) — when to prefer the native
  backend, full feature matrix, compositor binding examples for
  sway / Hyprland / river, compatibility list.
- [docs/fuzzing.md](docs/fuzzing.md) — running and extending the
  fuzz harness.
- [docs/accessibility.md](docs/accessibility.md) §4 — AT-SPI gap
  on the native Wayland path (still flagged as something to revisit
  upstream of egui-winit).

### Explicit non-goals for 0.5

- **No macOS or FreeBSD port.** The maintainer has only Linux
  hardware; advertising those targets without verification would
  ship broken claims. PRs welcome from contributors with the
  matching boxes.

### Upgrade notes

`config.toml` files saved by 0.4 decode unchanged. The Wayland
backend stays **opt-in via `ANIMA_USE_WAYLAND_NATIVE=1`** —
defaults haven't moved. If the probe finds no `zwlr_layer_shell_v1`
(GNOME Mutter, KWin), the binary silently falls back to the X11
path, same as before.

```bash
# Debian / Ubuntu
sudo apt install ./anima-engine_0.5.0-1_amd64.deb
ANIMA_USE_WAYLAND_NATIVE=1 anima-engine     # opt-in native Wayland

# AppImage
chmod +x animaEngine-0.5.0-x86_64.AppImage
./animaEngine-0.5.0-x86_64.AppImage
```

## [0.4.0] — 2026-06-07

Phase D — UX completion. Ten sub-phases focused on giving every UX
surface a coherent story: rebindable shortcuts, persistent panel
state, runtime accessibility control, native-speaker review
pipeline, surfaced failure paths, live perf instrumentation,
onboarding polish, opinionated empty states, locale + tooltip
sweep.

### Added

- **Rebindable keyboard map** (D.1): every action animaEngine
  dispatches lives in `src/keybindings.rs` as one variant of `Action`
  (27 user-facing + 1 dev `TogglePerfOverlay`). The Keybindings tab
  in the settings sidebar shows the live chord table; clicking
  **Record** captures the next chord, conflicts colour the offending
  binding yellow, per-row + global "Reset to defaults" buttons. The
  bindings persist in `config.toml` under `[keybindings.map]` —
  hand-editing is supported, chord strings round-trip through
  `KeyChord::FromStr` (`"Ctrl+Shift+A"`, `"Esc"`, `"ArrowUp"`, …).
- **Persistent collapse state** (D.2): the four inspector sections
  (Position / Appearance / Animation / Behavior) and the Scene-tab
  preset gallery remember their open/closed flag across sessions.
  Stored under `[collapse_state]`; defaults match the pre-D.2
  open-state heuristics so upgrading users see no visual shuffle.
- **Runtime AccessKit toggle** (D.3): Appearance → Accessibility
  hosts a checkbox driving `Context::enable_accesskit()` /
  `disable_accesskit()` per frame. Persisted as
  `[global].accesskit_enabled` (default true). Users on minimal
  setups can shut down the AT-SPI tree-update generation without
  rebuilding from source.
- **Locale review pipeline** (D.4):
  [`docs/i18n-pipeline.md`](docs/i18n-pipeline.md) documents how
  new strings flow from `en.ftl` to the nine translated locales,
  including the placeholder-English convention used while a native
  speaker hasn't reviewed yet. Per-locale audits under
  [`docs/locale-audit/`](docs/locale-audit/) carry glossary
  anchors, suspect-issue lists, and AI-confidence labels.
  GitHub issue template `.github/ISSUE_TEMPLATE/locale-review.md`
  for structured review requests.
- **Error banners + toast wiring** (D.5): the new `Warning` enum
  (`GlobalHotkeysUnavailable`, `HotReloadDisconnected`) renders
  session-lifetime banners at the top of the settings panel — so
  the user notices when `XGrabKey` couldn't grab the global
  chords or the hot-reload worker crashed silently. Toast
  coverage filled in at previously silent failure paths:
  duplicate-via-keypress, palette preset append.
- **Live perf overlay** (D.6): toggleable via `Ctrl+Shift+\``
  (`Action::TogglePerfOverlay`, rebindable). Shows FPS, rolling
  avg + p95 frame time, 60-frame averages for five categories
  (`scene_update`, `egui_paint`, `wgpu_submit`, `present`,
  `idle`), RSS in MiB (Linux), and a 120-frame sparkline with a
  16.7 ms reference line. "Export snapshot" writes a
  chrome-tracing JSON file at
  `~/.cache/animaEngine/perf-<ts>.json` openable in
  `chrome://tracing` or Perfetto.
- **Onboarding 2.0** (D.7): retains the progressive-tooltip
  concept from Phase A; adds the "What's new in 0.4" highlight
  panel anchored by `WHATS_NEW_VERSION = "0.4.0"` (one-shot per
  minor bump), two new hint sites (Keybindings tab,
  perf overlay), and an Appearance → "Reset onboarding hints"
  button so users can retake the tour.
- **Empty-state CTAs** (D.8): the Scene-empty card now offers
  "Browse presets" (opens the preset gallery); the Library
  no-asset-root / empty-index cards offer "Copy path to
  clipboard" so users can paste the documented assets path
  straight into their file manager.

### Changed

- Inspector section headers route through `t()` against the
  `inspector-section-*` keys (D.9). Non-English locales see the
  section labels in their language for the first time.
- `src/ui/keyboard.rs` collapsed to a thin re-export of the new
  `crate::keybindings::Action`; the canonical Action enum + label /
  description / default chords now live in `src/keybindings.rs`.
- Toast for "Library asset add failed" and the global hotkeys
  startup outcome both wire through the new banner / toast paths.

### Removed

- Dead i18n keys `appearance-keyboard-header` and
  `appearance-keyboard-note` from all 10 locale files — the call
  sites went away when D.1.6 replaced the Appearance read-only
  table with the dedicated Keybindings tab.

### Stats

- Locale key count: 128 → 168 (D.1: +34, D.3: +3, D.5: +2, D.7: +8,
  D.8: +2, D.9: +1, −2 dead).
- New modules: `src/keybindings.rs`, `src/perf.rs`,
  `src/ui/banner.rs`, `src/ui/collapse.rs`,
  `src/ui/perf_overlay.rs`, `src/ui/whats_new.rs`.
- Test suite: 210 lib + 25 integration + 1 demo = 236 pass.

### Upgrade notes

Pre-0.4 `config.toml` files decode unchanged — every new field
carries `#[serde(default)]`. The first session after upgrade
shows the "What's new in 0.4" panel; dismissing it stamps the
version into `[global].last_seen_whats_new`. The previously
hard-coded chord set survives intact as the default
`[keybindings.map]`, so muscle memory is preserved.

## [0.3.2] — 2026-06-04

Patch release. 0.3.1 shipped the PNGs at every hicolor size but the
GNOME Shell launcher entry still rendered without an icon on
Ubuntu's Wayland session — the freedesktop-spec compliant theme
lookup found the file fine, but Mutter never wired it to the entry.
Two unrelated spec corners we'd both missed:

### Fixed

- **Rename `data/anima-engine.desktop` →
  `data/com.animaengine.Anima.desktop`** so the `.desktop` filename
  matches the AppStream `<id>` in the metainfo. GNOME Shell 47+ on
  Ubuntu uses that match as a primary key for resolving the icon in
  the dock and launcher — without it, even a correct `Icon=` field
  + correct theme lookup is silently ignored.
- **Set X11 WM_CLASS + Wayland `app_id` to `animaEngine`** explicitly
  in `src/app.rs` via `WindowAttributesExtX11::with_name` and
  `WindowAttributesExtWayland::with_name`. Now the runtime window's
  identity matches `StartupWMClass=animaEngine` in the `.desktop`
  entry, so the dock binds the running window to the pinned entry
  instead of treating them as two separate apps.

`Makefile` and `scripts/build-appimage.sh` both updated to use the
new filename; `make uninstall` also clears the legacy
`anima-engine.desktop` so an upgrading user doesn't end up with two
launcher entries.

### Notes for 0.3.1 downloaders

```bash
sudo apt install ./anima-engine_0.3.2-1_amd64.deb
```

dpkg sees the .desktop filename change and cleans the old one up
automatically on upgrade. After install, the launcher icon should
appear — **on the first session that starts fresh after the
install**. Logout/login is the cheapest path; reboot is the
guaranteed one. (Mutter's in-session app-icon cache is
session-lifetime; nothing we can touch from a post-install hook
invalidates it.)

## [0.3.1] — 2026-06-03

Patch release. The 0.3.0 `.deb` shipped only the scalable SVG at
`/usr/share/icons/hicolor/scalable/apps/`, which Yaru, Adwaita, and
most other freedesktop themes look at only **after** they've walked
their own discrete-size buckets. End result: an entry in the GNOME
app launcher with the right *name* but no icon.

### Fixed

- **Pre-rasterized icons land at 16 / 24 / 32 / 48 / 64 / 128 / 256
  px** under `/usr/share/icons/hicolor/<size>/apps/anima-engine.png`.
  Every freedesktop-spec resolver picks one up before falling back
  to the SVG; GNOME shows the icon in launcher / dock / Activities.
- **`scripts/render-icons.sh`** is the new canonical rasterizer
  (prefers `rsvg-convert`, falls back to Inkscape, then ImageMagick).
  `make icons` runs it explicitly; `make install`,
  `scripts/build-deb.sh`, and AppImage staging all depend on it
  implicitly so `.deb` / AppImage / `make install` ship the PNGs
  out of the box.
- **`make install` and `make uninstall` refresh
  `gtk-update-icon-cache`** so the icon shows up without a logout.

### Notes for 0.3.0 downloaders

If your 0.3.0 install already shows a proper icon in the launcher,
you don't need this update — your desktop happened to find the SVG
on its first lookup pass. If the launcher entry shows the app name
but no icon (the GNOME / Ubuntu default), grab the 0.3.1 `.deb`:

```bash
sudo apt install ./anima-engine_0.3.1-1_amd64.deb
```

`apt` upgrades 0.3.0 → 0.3.1 in place; your config at
`~/.config/animaEngine/config.toml` is untouched.

## [0.3.0] — 2026-06-03

Phase C — **engine polish**. Ten sub-phases across multi-monitor,
multi-window, asset library, behavior expansion, animation curves,
and sprite groups. The renderer pipeline still draws the same way
it did in 0.2 — the engine got a wider model around it instead of
a faster one.

### Added

**Multi-monitor (C.1, C.2):**
- `src/monitor.rs` data layer: `MonitorMode` (PerMonitor / Span /
  Single-by-name) persisted in `GlobalConfig.monitor_mode`
- `MonitorInfo` topology snapshot taken on first `resumed()` with
  HiDPI scale + primary marker
- `resolve_monitor_for_position` pure helper: explicit pin →
  centroid hit-test → primary fallback
- Per-entity `monitor: Option<String>` pin in `CharacterConfig`
  with stale-pin warn + auto-fallback
- Scene tab picker for the global mode, Inspector picker for the
  per-entity pin
- `Ctrl+M` cycles the selected entity through all monitors and
  emits a localised toast

**Multi-window data layer (C.3):**
- `WindowConfig` (id, name, optional per-window `monitor_mode`,
  characters)
- `AppConfig.windows: Vec<WindowConfig>` with serde defaults so
  every 0.2 / 0.2.1 config decodes cleanly
- `AppConfig::windows_normalised()` synthesises a default window
  from top-level `characters` when no explicit windows exist
- Render-side multi-window dispatch (one winit::Window per entry)
  deferred to 0.4 / Phase D

**Asset library (C.4, C.5):**
- `src/asset_library/` module: directory discovery (env override →
  XDG → exe-relative), recursive scan with symlink depth cap 4,
  FNV-1a stable ids (12 hex chars, zero new dep), atomic-written
  `library.toml`, mtime-based thumbnail freshness helpers
- Extension whitelist mirrors `app::DROP_EXTENSIONS` exactly — a
  unit test asserts the two stay aligned (audit invariant L2)
- New "Library" tab in the settings sidebar with search bar,
  per-row kind icon, basename + path-on-hover, "Add to scene"
  button that routes through `Scene::add_entity_from_path` (full
  pre-validation + asset caps preserved)
- Library merge-scan at startup preserves user tags + last_used_at
- Real thumbnail decoding remains a polish opportunity for later;
  rows show typed Phosphor icons for now

**Bounce behavior (C.6):**
- `Behavior::Bounce { amplitude_px, period_sec, axis: BounceAxis }`
  with `BounceAxis` = Horizontal / Vertical (default) / Both
  (cos+sin = circular Lissajous)
- `BehaviorState::bounce_invalidate()` called from drag, arrow-key
  nudge, and Home-center so the sprite never snaps back to a stale
  rest position
- Period clamped to ≥ 50 ms; gravity wins when both bounce and
  physics are on
- Spinner and Reactive variants from the original C.6 spec moved
  to 0.4 — rationale documented in `docs/engine-features.md` §4.1
  and §4.3

**Animation curves (C.7):**
- `src/anim.rs` relocated from `src/ui/anim.rs` (no longer
  UI-specific) with a new `EasingCurve` enum: Linear (default) /
  EaseInQuad / EaseOutQuad / EaseInOutQuad / Sine / BounceOut
- `Animation.easing: Option<EasingCurve>` distorts per-frame
  intervals while preserving total loop duration; GIF / WebP
  per-frame delays remain authoritative when present
- 6-choice picker in the Inspector's Animation section

**Sprite groups (C.8):**
- `src/group.rs` with `GroupConfig` (id, name, member_ids,
  offset_x, offset_y, scale, visible), pure composition helpers
  (`visible_for_member`, `cleanup_after_entity_removal`,
  `first_duplicate_id`), and a manual `Default` impl matching the
  serde defaults
- `Scene::visible_entities()` and `entity_at_point()` honour
  group visibility — members of a hidden group don't render *and*
  can't catch clicks
- `Scene::remove_entity` scrubs the removed id from every group's
  `member_ids` (dangling-membership invariant)
- Read-only Groups section in the Scene tab
- Offset / scale composition in the renderer + UI edit (add /
  remove / rename) deferred to 0.4

**Performance baseline (C.9):**
- `examples/perf_baseline.rs` measures `Scene::tick` +
  `visible_entities` at 10 / 25 / 50 / 100 entities (procedural
  fallback frames, no I/O). On the dev box at 100 entities the
  combined per-frame cost lands at ~0.004 ms — well under the
  engine target of 8 ms and the 60 fps budget of 16.6 ms

### Changed

- `src/anim.rs` graduated from `src/ui/anim.rs`; one import path
  change in `panels.rs`, no behavioural delta
- `CharacterConfig` carries new optional fields `monitor`,
  `easing` — both `#[serde(default, skip_serializing_if = "Option::is_none")]`
- `Animation` gained an `easing: Option<EasingCurve>` field
- `App` tracks `ctrl_held` (used by `Ctrl+M`) alongside
  `shift_held`
- Settings sidebar now has four tabs (Inspector / Scene / Library /
  Appearance) — previously three

### Tests

- **211 total** (up from 195 in 0.2.1) — +9 monitor, +6 panels
  (monitor cycle), +5 windows, +9 asset_library, +9 bounce, +10
  easing, +12 group, +14 documenting C.4/C.5/C.6/C.7/C.8 invariants
  across `tests/` integration
- New CI invariants: i18n key coverage parity (across all ten
  locales); library extension whitelist matches drag-drop
  whitelist; intervals-sum-to-total-duration under every easing
  curve

### i18n

- 10 new monitor keys across all ten locales
- 14 new library keys
- 5 new bounce keys
- 7 new easing keys (label + 6 curve labels)
- Test `every_locale_covers_every_en_key` still green

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

### Security hardening (from the post-Phase A audit)

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
  drag-drop-validation invariant from 0.1.0's Phase B.
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

UI/UX polish release. Phase A (A.0-A.11) landed on top of the 0.1.0 +
B series — twelve sub-phases focused on coherence, accessibility,
and localisation rather than new sprite features. The engine renders
the same content; the chrome around it is dramatically nicer.

### Added

**Design system (Phase A.0):**
- `docs/design-system.md` — single source of truth for colours,
  typography, spacing, radii, icons, motion, and component patterns
- Every panel now references token constants instead of hardcoded
  hex values / magic numbers

**Theme system (Phase A.1, A.9):**
- Four themes: Dark, Light, Dark · High contrast, Light · High contrast
- Theme persisted in `GlobalConfig.theme`
- HC variants clear WCAG AAA (≥ 7:1) on every text tier, thicken
  the focus ring to 3 px, and zero out animation time for
  motion-sensitive users
- CI enforces contrast thresholds via unit tests

**Iconography (Phase A.2):**
- `egui-phosphor` icon font wired through `src/ui/icons.rs` — every
  in-app glyph is a named constant for grep-ability
- New "Ghost Mascot" app icon ([data/anima-engine.svg](data/anima-engine.svg))
  selected from three exploratory variants in
  [packaging/icon-variants/](packaging/icon-variants/)

**Settings sidebar redesign (Phase A.3):**
- Three tabs (Inspector / Scene / Appearance) with sticky header,
  scrollable body, entity-count footer
- Inspector has collapsible sections (Position / Appearance /
  Animation / Behavior) and quick-toggle row for Visible/Gravity
- Tab selection persists through `egui::Memory`

**Empty / error / loading states (Phase A.4):**
- `src/ui/states.rs` with reusable `empty`, `error`, `spinner`
  helpers
- Toasts redesigned per design-system §7.8: bg.elevated surface,
  semantic-coloured severity icons, radius_lg, theme-aware

**Micro-animations (Phase A.5):**
- `src/ui/anim.rs` with `ease_in_quad` / `ease_out_quad` / `lerp`
- Toast slide-in (200 ms ease-out-quad) + fade-out
  (300 ms ease-in-quad) computed off `Toast::age()`
- Tab cross-fade (100 ms) via `ctx.animate_value_with_time`
- Hover and focus transitions ride egui's built-in animation_time

**Onboarding (Phase A.6):**
- Three dismissible inline hints for fresh users (tabs, V/G shortcuts,
  theme instant-apply)
- Existing users (configs pre-A.6) skip all hints via `#[serde(default = ...)]`

**Preset library (Phase A.7):**
- Six curated presets using only the shipped demo assets:
  Cozy Companion, Productivity Zen, Halloween Party,
  Birthday Confetti, Studio Session, Cursor Follower
- Append / Replace modes; Append suffixes IDs with `_a/_b/...` to
  avoid collisions

**Keyboard map + command palette (Phase A.8):**
- `src/ui/keyboard.rs` Action enum with label + description +
  default combo (21 actions)
- Read-only keyboard reference table in the Appearance tab
- **Ctrl+K command palette** — fuzzy search across themes and
  presets, executes via `PaletteOutcome` back into `App`

**Accessibility (Phase A.9):**
- AccessKit bridge enabled via egui-winit feature → AT-SPI events
  on Linux for every egui widget
- High-contrast theme variants (see Theme system above)
- `docs/accessibility.md` documenting commitments and non-goals

**Internationalisation (Phase A.10):**
- `fluent-rs` backend with `OnceLock<RwLock<State>>` for constant-time
  locale switching
- Ten locales shipped (`en`, `ro`, `es`, `de`, `fr`, `it`, `pt-BR`,
  `pl`, `nl`, `ja`) covering ~45 keys each
- Auto-detect from `LANG` / `LC_ALL` / `LC_MESSAGES` with sensible
  fallback chain (exact match → language-only → English)
- Language picker in Appearance tab with autonyms (Română, 日本語, …)
- Explicit selection persists in `GlobalConfig.locale`

**Polish (Phase A.11):**
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

**Foundation (Phase 0-1):**
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

**Performance (Phase 2):**
- Parallel PNG-sequence decode via rayon
- Cached visibility/z-order (filter + sort runs only on invalidation)
- On-disk RGBA cache at `~/.cache/animaEngine/textures/`
- Spritesheet slicing uses row-stride memcpy

**UI (Phase 3):**
- egui integration via `egui-wgpu`; ⚙ button is now an egui widget
- Right-side settings panel: inspector + scene list
- Right-click context menu (Duplicate / Reset / Gravity / Z-order /
  Delete)
- Toast notifications for save / add / delete / hot-reload

**Behaviors (Phase 4):**
- `Idle` (default), `WalkAround`, `FollowCursor`, `BoundedWander`
- `TickContext` struct, per-entity `BehaviorState`, deterministic
  per-id PRNG seed for bounded wander

**Asset ecosystem (Phase 5):**
- H.264 MP4 video loader (`mp4` + `openh264`, no system deps,
  capped at 20 s / 600 frames)
- Sample pack: ghost, slime, heart, star, cat (all procedural)

**System integration (Phase 6):**
- System tray via `ksni` (StatusNotifierItem, no libappindicator)
- Global hotkeys `Ctrl+Shift+A/H/P` via `global-hotkey`
- Single-instance D-Bus handshake (`com.animaengine.Anima`); a second
  launch raises the existing window

**Native Wayland (Phase 7, opt-in):**
- Compositor probe (logs whether `zwlr_layer_shell_v1` is available)
- `sctk`-driven layer surface + wgpu bridge
- Pointer events translated to `egui::Event` (keyboard deferred)
- `wl_surface::set_input_region` click-through, matching the X11 path
- `ANIMA_USE_WAYLAND_NATIVE=1` to opt in

**Packaging (Phase 8):**
- `.desktop` + scalable SVG icon + AppStream metainfo
- `make install` with `DESTDIR` / `PREFIX` support
- AppImage build via `linuxdeploy` (`make appimage` ≈ 7 MB output)
- `.deb` via `cargo-deb` (`make deb` ≈ 5 MB output)
- Flatpak manifest at `flatpak/com.animaengine.Anima.yml` plus
  `make flatpak`

**Docs + recovery (Phase 9):**
- README rewritten; `CONTRIBUTING.md`, `docs/architecture.md`,
  `docs/config.md` added
- Panic hook + `--recover` flag — last clean config snapshot survives
  a crash; restore copies it back over the live config

### Hardening (Phase B, audit response)

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
