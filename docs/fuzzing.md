# Fuzzing

animaEngine ships a small `cargo-fuzz` harness covering the parsers
that sit closest to untrusted input:

| Target | Surface | Origin of input |
|---|---|---|
| `keychord_parse` | `KeyChord::FromStr` | `[keybindings.map]` entries in user-edited `config.toml` |
| `uri_list_parse` | `wayland::data_device::parse_uri_list` | `text/uri-list` payloads from file-manager drags |
| `asset_type_detect` | `animation::loader::detect_asset_type` | dropped or library file paths |

The invariant for every target is "**never panic**, return a typed
error or sensible default on adversarial input." The asset decoders
themselves (PNG / GIF / WebP / MP4) aren't fuzzed here because they
delegate to `image` and `mp4parse` upstream, both of which have their
own fuzz suites.

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

```yaml
fuzz-smoke:
  name: fuzz smoke (60s per target)
  runs-on: ubuntu-24.04
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@nightly
    - run: cargo install cargo-fuzz --locked
    - run: cargo +nightly fuzz run keychord_parse -- -max_total_time=60
    - run: cargo +nightly fuzz run uri_list_parse -- -max_total_time=60
    - run: cargo +nightly fuzz run asset_type_detect -- -max_total_time=60
```

Optional — leave commented in `.github/workflows/ci.yml` until you've
got at least one corpus snapshot committed.

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
