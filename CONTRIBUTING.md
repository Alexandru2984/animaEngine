# Contributing to animaEngine

Patches welcome — overlay engines have a lot of surface area and there's
always something to polish. This file covers the **how** (build, test,
style) and points at [docs/architecture.md](docs/architecture.md) for
the **what** (where each subsystem lives).

## What lands (post-1.0)

animaEngine is past 1.0 — the **stability freeze that ran from 0.9 to
1.0 is over**, and feature work is open again. What replaced it is
narrower and permanent: the promises in
[docs/stability-policy.md](docs/stability-policy.md).

**Guaranteed for the life of 1.x**, so a change that breaks any of them
waits for 2.0 rather than shipping in a minor:

- the config schema — a `config.toml` written by any 1.x release loads
  in every later one;
- the D-Bus surface (bus name, object path, interface, method names);
- the `--help` / `--recover` CLI flags;
- the accepted asset formats and the drag-drop extension allowlist;
- the XDG file locations.

Adding to those is fine — new config fields with serde defaults, new
D-Bus methods, new flags, new formats. Removing or repurposing is not.

**Always welcome, at any time:** crash fixes (panic, hang, OOM, GPU or
device failure), data-loss fixes (config corruption, lost user state, a
bad migration), security fixes (see [SECURITY.md](SECURITY.md) and
[docs/threat-model.md](docs/threat-model.md)), doc errors where the
docs and the code disagree, and fixes to strings in existing locales.

**Worth raising in an issue first**, not because they are unwelcome but
because they are easier to agree on before the code exists: new
dependencies, new locales, and architecture changes large enough to
touch several subsystems. A short issue saying what and why is enough.

No feature overrides the invariants in [House rules](#house-rules) below
— the zero-network rule in particular is load-bearing for the threat
model, not a preference.

**Branch policy:** work branches from `main` and merges back to `main`;
releases ship from `main`.

### Historical note

Through the 0.x series there was no stability guarantee, and from 0.9 to
1.0 the project ran a hard feature freeze — only crash, data-loss,
security, doc and translation fixes landed, with one granted exception
for X11/Wayland parity. That window is closed; it is recorded here
because the commit history and older issues refer to it.


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

### Or write it as a script instead

Most motion doesn't need a new enum variant. `Behavior::Script` runs a
[Rhai](https://rhai.rs) file from your asset library, so a new movement
is a text file rather than a patch:

```rhai
// behaviors/lazy_walk.rhai — walk right, wrap at the edge.
x += params.speed * dt;
if x > bounds_max_x - w {
    x = bounds_min_x;
}
```

Point an entity at it from the Inspector's Behavior section, or in
`config.toml`:

```toml
[characters.behavior]
type = "script"
path = "behaviors/lazy_walk.rhai"

[characters.behavior.params]
speed = 60.0
```

The script is **top-level statements**, not a function — Rhai passes map
arguments by value, so a `fn tick(e)` mutating `e.x` would move a copy and
lose the change silently.

Readable: `dt`, `w`, `h`, `bounds_min_x` / `bounds_min_y` / `bounds_max_x`
/ `bounds_max_y`, `cursor_x`, `cursor_y`, `has_cursor`, `elapsed`,
`reduced_motion`, and your own `params`. Writable: `x` and `y` — integers
are fine, `x = 100` works. For state that has to survive between frames,
use the `state` map; a top-level `let` will not, because Rhai unwinds the
scope when a run ends:

```rhai
if !state.contains("phase") { state.phase = 0.0; }
state.phase += dt;
y = state.phase.sin() * 20.0 + 400.0;
```

Constraints worth knowing before you debug something surprising:

- Scripts run **on the UI thread, once per entity per frame**, so
  execution is capped. A runaway script is aborted, not tolerated —
  the entity holds still and you get one toast.
- A failure is **sticky until the file changes.** Fix the file and it
  recompiles by itself; no restart.
- The sandbox is deny-by-default: no filesystem, no network, no process
  spawn, and `import` and `eval` are both switched off. There is no way
  to widen this from a script, by design.
- Paths resolve inside the asset library only.

Three worked examples live in
[docs/examples/behaviors/](docs/examples/behaviors/) — a walk, a bob that
accumulates through `state` and respects reduced motion, and one that
reacts to the cursor. Copy them into your asset library to try them. A
test compiles and runs every one of them, so they cannot drift from the
API without CI noticing.

Details and the reasoning are in
[docs/plans/v1.2-scripting.md](docs/plans/v1.2-scripting.md).

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
