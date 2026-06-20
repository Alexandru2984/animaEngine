# Fuzzing

animaEngine ships a small `cargo-fuzz` harness covering the parsers
that sit closest to untrusted input:

| Target | Surface | Origin of input |
|---|---|---|
| `keychord_parse` | `KeyChord::FromStr` | `[keybindings.map]` entries in user-edited `config.toml` |
| `uri_list_parse` | `wayland::data_device::parse_uri_list` | `text/uri-list` payloads from file-manager drags |
| `asset_type_detect` | `animation::loader::detect_asset_type` | dropped or library file paths |
| `cache_deserialize` | `animation::cache::deserialize_frames` | `~/.cache/animaEngine/textures/*.bin` — our own binary format, corruptible by a crash mid-write or a tampered cache |
| `avcc_nalu_walk` | `animation::video_loader::avcc_to_annex_b` | one MP4 sample's length-prefixed NALU bytes (hand-written length/offset walk) |
| `shimeji_xml` | `shimeji::fuzz_parse_actions` | `actions.xml` inside downloaded third-party mascot packs |

The invariant for every target is "**never panic**, return a typed
error or sensible default on adversarial input." The asset decoders
themselves (PNG / GIF / WebP / MP4) aren't fuzzed here because they
delegate to `image` and `mp4parse` upstream, both of which have their
own fuzz suites — but the **hand-written** parsers around them (our
cache codec, the NALU length-prefix walk, the Shimeji XML reader) are
exactly the bespoke code that warrants fuzzing, added in W.4 (0.9).

Committed seed inputs live under `fuzz/seeds/<target>/` — **every**
target carries a handful: valid examples plus adversarial variants
(truncated cache headers, a NALU whose length overruns the buffer,
malformed / empty `actions.xml`, CRLF and comment-laden uri-lists,
extension-less paths, modified key chords). The corpus proper
(`fuzz/corpus/`) is gitignored and seeded from these, so a run starts
from real structure instead of random bytes.

## Running

`cargo-fuzz` requires a nightly toolchain.

```bash
rustup toolchain install nightly
cargo install cargo-fuzz --locked
cargo +nightly fuzz run keychord_parse        # ^C when you've had enough
cargo +nightly fuzz run uri_list_parse
cargo +nightly fuzz run asset_type_detect
```

Corpora live under `fuzz/corpus/<target>/`; libfuzzer seeds itself
from anything it finds there and writes back useful inputs as it
makes progress. Crash reproducers land in `fuzz/artifacts/<target>/`.

## CI integration

The fuzz package is not built by default `cargo test` or `cargo
build` runs — it lives in its own `fuzz/Cargo.toml`. Adding a CI job
that runs a short timeboxed fuzz batch on every push catches
regressions early without bloating the main build matrix:

A schedule-only `fuzz` job in `.github/workflows/ci.yml` (nightly +
cargo-fuzz) seeds each corpus from `fuzz/seeds/` and runs every target
for 60 s. It never blocks a push — it surfaces a regression as a red
weekly run and uploads any reproducer to the `fuzz-artifacts` artifact.
Trigger it on demand with **Run workflow** (`workflow_dispatch`).

## Adding a new target

1. Pick a function that ingests bytes / strings / paths from outside
   the program (file, network, IPC).
2. Add a binary entry to `fuzz/Cargo.toml` matching the target name.
3. Create `fuzz/fuzz_targets/<name>.rs` with `#![no_main]` and
   `fuzz_target!` calling the function under test.
4. Document the invariant in this file's table at the top.

Keep the harness tiny — call the function and discard the result.
Anything fancier (assertions, custom validation) belongs in the
function's own unit tests; fuzz targets exist solely to catch
panics, integer overflow, and pathological allocations.
