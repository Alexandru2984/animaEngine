# Contributing to animaEngine

Patches welcome — overlay engines have a lot of surface area and there's
always something to polish. This file covers the **how** (build, test,
style) and points at [docs/architecture.md](docs/architecture.md) for
the **what** (where each subsystem lives).

## Quick loop

```bash
git clone https://github.com/Alexandru2984/animaEngine
cd animaEngine

# System deps on Ubuntu/Debian
sudo apt install -y build-essential cmake \
    libvulkan-dev libx11-dev libxcb1-dev libxkbcommon-dev \
    libwayland-dev libxrandr-dev

# Sanity check
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check

# Run with debug logs
RUST_LOG=anima_engine=debug cargo run
```

CI (see `.github/workflows/ci.yml`) runs the same four checks against
Ubuntu 22.04 + 24.04. If they pass locally they pass in CI.

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
  asset caps, atomic writes, drag-drop pre-validation, single-method
  D-Bus surface. Read it before changing a loader or adding a D-Bus
  method.
- **Opt-in dangerous defaults**: physics, native Wayland, and the disk
  cache are all opt-in via field / env var. New "potentially surprising"
  features should follow the same pattern.
- **Constants**: cross-module magic numbers go in `src/constants.rs`.
  Module-local values stay local.
- **Tests**: every new feature gets at least one unit test. UI / event
  loop code is exempt (we'd need a display server) but the underlying
  pure logic isn't. `cargo test` currently runs 80 tests; please don't
  shrink that number.

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
- `src/wayland/` — native Wayland backend (opt-in, in development)
- `src/window/` — X11-side input shape + EWMH hints
- `src/app.rs` — the `ApplicationHandler` that ties it all together

## Adding a new behavior

Worked example for "add a behavior" — covers most of the patterns you'd
touch for a feature:

1. Add a variant to `Behavior` in `src/behavior.rs`, including
   serde defaults for every field.
2. Extend the `match self` in `Behavior::tick`. Read whatever you need
   from `TickContext` (sprite size, screen size, cursor, dt).
3. (Optional) Add accumulators to `BehaviorState` if you need runtime
   state separate from the config.
4. Wire the UI in `src/ui/panels.rs::behavior_picker` — add a
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
4. Open a PR. CI runs the same checks on Ubuntu 22.04 + 24.04.
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
