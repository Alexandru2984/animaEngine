# Contributing to animaEngine

Patches welcome — overlay engines have a lot of surface area and there's
always something to polish. This file covers the **how** (build, test,
style) and points at [docs/architecture.md](docs/architecture.md) for
the **what** (where each subsystem lives).

## Stability freeze (0.9 → 1.0)

animaEngine is in a **stability freeze** from 0.9 until 1.0. Feature
work stopped when 0.9 opened; everything between here and 1.0 is
measurement, hardening, and paperwork — proving the 1.0 contract before
promising it. If you're contributing during this window, read this
section first: it decides whether a change can land at all.

**A change qualifies for the freeze only if it is one of:**

- a **crash** fix — panic, hang, OOM, GPU/device failure;
- a **data-loss** fix — config corruption, lost user state, a bad
  migration;
- a **regression** fix — something that worked in an earlier release
  and no longer does;
- a **security** fix — see [SECURITY.md](SECURITY.md) and
  [docs/threat-model.md](docs/threat-model.md);
- a **doc error** — the docs describe something the code doesn't do, or
  the reverse;
- a **translation** fix — a wrong or corrupted string in an *existing*
  locale.

**These wait until after 1.0, no matter how good:**

- new features, new behaviors, new config fields, new UI;
- refactors, renames, or architecture changes not required by one of
  the fixes above (the "god-object `App` split" and a native-Wayland
  device-loss rework are the standing examples — both deferred on
  purpose);
- new dependencies or non-security version bumps;
- new locales — the string set is frozen; fixes to shipped locales are
  welcome, a brand-new language is not.

**Exceptions are decided in the open, before the PR.** Open an issue
describing the change and why it can't wait for 1.0, and get an explicit
"yes, in scope" from the maintainer first. A PR that expands scope
without that decision will be asked to wait — the whole point of the
freeze is that *nothing* grows the surface we're trying to stabilize.

**Active exception: X11/Wayland parity (granted 2026-06-21).** The
native Wayland path (`ANIMA_USE_WAYLAND_NATIVE=1`) is a documented,
supported target (see [docs/wayland.md](docs/wayland.md)), not an
experiment — closing a capability gap between it and the X11 path
(asset library, context menu, per-monitor distribution, cursor
tracking wherever the Wayland protocol actually allows it) is treated
as evening out the 1.0 contract across both backends, not as adding a
new feature. This does **not** reopen the freeze generally: a
capability neither path has today is still out of scope, and anything
landing under this exception still needs the same crash/security/test
discipline as everything else in this list.

**Branch policy:** fixes branch from `main` and merge back to `main`;
1.0 ships from `main`. No feature branches are in flight during the
freeze — anything feature-shaped lives in an issue until the 1.0 tag is
cut, then development reopens.

## Quick loop

```bash
git clone https://github.com/Alexandru2984/animaEngine
cd animaEngine

# System deps on Ubuntu/Debian.
# pkg-config is required (smithay-client-toolkit's build script probes
# xkbcommon through it); build-essential does NOT pull it in, and the
# GitHub CI runners ship it preinstalled, which is why its absence only
# bites on a clean Debian/Kali box.
sudo apt install -y build-essential cmake pkg-config \
    libvulkan-dev libx11-dev libxcb1-dev libxkbcommon-dev \
    libxkbcommon-x11-dev libwayland-dev libxrandr-dev

# Sanity check
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check

# Run with debug logs
RUST_LOG=anima_engine=debug cargo run
```

CI (see `.github/workflows/ci.yml`) runs these four on Ubuntu 24.04,
plus rustdoc, MSRV, cargo-machete, desktop-metadata and actionlint
gates. If the four above pass locally they pass in CI; the rest rarely
trip on a focused change.

### Alpine / musl

On Alpine (or any musl target) the deps are:

```bash
apk add build-base cmake pkgconf nasm vulkan-loader-dev \
    libx11-dev libxcb-dev libxkbcommon-dev wayland-dev libxrandr-dev \
    mesa-vulkan-swrast
```

and the build needs **dynamic CRT linking**, because the Rust musl
target defaults to fully-static linking but Alpine only ships the
shared `libxkbcommon.so` (no static `.a`), so the link fails with
`cannot find -lxkbcommon`:

```bash
RUSTFLAGS="-C target-feature=-crt-static" cargo build --release
```

`nasm` is required there too — `openh264` builds its assembly from
source on musl. The binary itself builds and links clean this way;
running it still needs a transparent-capable surface (a compositor +
a GPU/WSI that exposes a non-opaque alpha mode), which a bare
software-rendered VM may not provide — the app refuses with a clear
message rather than painting the desktop black.

## House rules

These are conventions the codebase already follows; matching them keeps
review fast:

- **Error handling**: `thiserror` everywhere. Add a variant to
  `AnimaError` rather than introducing `anyhow` or stringly-typed
  errors. Internal functions return `Result<T>` (the crate alias).
- **Logging**: `tracing` only — never `log::` or `println!`. Spans live
  on non-hot operations (`Scene::from_config`, `load_asset`,
  `WgpuRenderer::new`). The hot path (`render`, `tick`) stays clean.
- **No telemetry**: zero network calls, no Sentry, no analytics, no
  ping-on-launch. Decision baked into the project.
- **Security invariants** in [docs/threat-model.md](docs/threat-model.md):
  asset caps, atomic writes, drag-drop pre-validation, the bounded
  five-method D-Bus surface. Read it before changing a loader or
  adding a D-Bus method.
- **Opt-in dangerous defaults**: physics, native Wayland, and the disk
  cache are all opt-in via field / env var. New "potentially surprising"
  features should follow the same pattern.
- **Constants**: cross-module magic numbers go in `src/constants.rs`.
  Module-local values stay local.
- **Tests**: every fix gets at least one unit test that would have
  caught it. UI / event-loop code is exempt (we'd need a display
  server), but the underlying pure logic isn't — extract the decision
  into a testable function and test that, the way `next_surface_loss_state`
  and `scaling_desyncs_xshape` were. Coverage only goes up; don't delete
  tests to make a change fit.

## Style

- `cargo fmt` is the rule. `rustfmt.toml` is committed.
- Clippy is `-D warnings`. If a lint genuinely doesn't fit, gate it
  inline with `#[allow]` and a one-line comment explaining why.
- **Comments**: explain **why**, not **what**. Identifiers do the
  "what" already.
- **Doc comments** on `pub` items in library modules. `#![deny(missing_docs)]`
  isn't enforced yet but will be once the API stabilizes.

## Code organization

See [docs/architecture.md](docs/architecture.md) for a module-level map.
Briefly:

- `src/animation/` — loaders (PNG seq, GIF, WebP, MP4, spritesheet) +
  per-frame cache + the `Frame` type
- `src/behavior.rs` — per-entity motion behaviors
- `src/renderer/` — wgpu pipeline (sprite shader, batched quads)
- `src/ui/` — egui integration (settings panel, context menu, toasts)
- `src/wayland/` — native Wayland backend (opt-in, wlroots only)
- `src/window/` — X11-side input shape + EWMH hints
- `src/app/` — the `ApplicationHandler` that ties it all together
  (`mod.rs` + `render_loop.rs`, `lifecycle.rs`, `dispatch.rs`, …)

Almost everything above is OS-agnostic; only `src/wayland/` and
`src/window/` are Linux-bound. **Porting to Windows / macOS / BSD** is a
scoped, post-1.0, help-wanted epic — see
[docs/porting-windows.md](docs/porting-windows.md) for the full map.

## Adding a new behavior

Worked example for "add a behavior" — covers most of the patterns you'd
touch for a feature:

1. Add a variant to `Behavior` in `src/behavior.rs`, including
   serde defaults for every field.
2. Extend the `match self` in `Behavior::tick`. Read whatever you need
   from `TickContext` (sprite size, screen size, cursor, dt).
3. (Optional) Add accumulators to `BehaviorState` if you need runtime
   state separate from the config.
4. Wire the UI in the behavior picker under `src/ui/panels/` — add a
   `selectable_value` entry and a `match` arm with sliders.
5. Write 2-3 unit tests in the `tests` module at the bottom of
   `behavior.rs`.

That's it — the rest of the engine (config save / load, hot-reload,
toasts, the inspector) picks it up automatically because everything
flows through `Behavior` and `BehaviorState`.

## Submitting a change

1. Fork the repo, branch from `main`.
2. Make your changes; keep commits focused (one logical change per
   commit, ideally).
3. Run the four checks above (`build` / `test` / `clippy` / `fmt`).
4. Open a PR. CI runs them (and the extra gates) on Ubuntu 24.04.
5. PR description should include:
   - What it does
   - Why (linked issue if applicable)
   - How you tested

## Reporting bugs

Open an issue with:

- Distro + version (`/etc/os-release` is fine)
- Compositor / desktop environment (`echo $XDG_CURRENT_DESKTOP`)
- Output of `RUST_LOG=anima_engine=debug anima-engine` up to where the
  bug manifests
- Steps to reproduce

For overlay-specific weirdness (window not transparent, click-through
broken, sprite ghosting), include a screenshot — they're surprisingly
diagnostic.
