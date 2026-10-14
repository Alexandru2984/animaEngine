# Fuzzing

animaEngine ships a small `cargo-fuzz` harness covering the parsers
that sit closest to untrusted input:

| Target | Surface | Origin of input |
|---|---|---|
| `keychord_parse` | `KeyChord::FromStr` | `[keybindings.map]` entries in user-edited `config.toml` |
| `uri_list_parse` | `wayland::data_device::parse_uri_list` | `text/uri-list` payloads from file-manager drags |
| `asset_type_detect` | `animation::loader::detect_asset_type` | dropped or library file paths |
| `cache_deserialize` | `animation::cache::deserialize_frames` | `~/.cache/animaengine/textures/*.bin` — our own binary format, corruptible by a crash mid-write or a tampered cache |
| `avcc_nalu_walk` | `animation::video_loader::avcc_to_annex_b` | one MP4 sample's length-prefixed NALU bytes (hand-written length/offset walk) |
| `shimeji_xml` | `shimeji::fuzz_parse_actions` | `actions.xml` inside downloaded third-party mascot packs |
| `scene_file_read` | `scene_file::fuzz_read` | `.animascene`, `.animachar` and `.animabackup` files (1.5): the strict zip reader, entry names, the manifests (`scene.toml`, `backup.toml`) parsed and tidied, the thumbnails decoded |
| `thumbnail_decode` | `scene_thumb::decode` | a scene's thumbnail PNG, inside a shared or restored file |
| `small_parsers` | `speech::fuzz_small_parsers` | 1.5's hand-written text parsers fed from `config.toml`: bubble colours, `HH:MM` times, `MM-DD` dates and when a reminder is due, sound names, own hours, `{placeholders}` |
| `config_round_trip` | `config::fuzz_config_round_trip` | `config.toml` itself, as a newer version or a hand edit leaves it: whatever reads, written and read again, must read and write the same — what is set aside and the keys this build does not know included (1.5) |
| `scene_save_over` | `scenes::fuzz_scene_save_over` | a named scene's file saved over itself (1.5): whatever reads must read the same after, leave the same out, keep every key a scene does not read as it was, and not change when saved again |

The invariant for every target is "**never panic**, return a typed
error or sensible default on adversarial input." `config_round_trip`
and `scene_save_over` ask more: a second round of writing and reading
changes nothing. The asset decoders
themselves (PNG / GIF / WebP / MP4) aren't fuzzed here because they
delegate to `image` and `mp4parse` upstream, both of which have their
own fuzz suites — but the **hand-written** parsers around them (our
cache codec, the NALU length-prefix walk, the Shimeji XML reader) are
exactly the bespoke code that warrants fuzzing, added in W.4 (0.9).
The readers of the files 1.5 opens and its small parsers followed; the
first run of `small_parsers` found an overflow in `HH:MM` parsing
(`"122412244:1"`, kept as a seed).

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
cargo +nightly fuzz run scene_file_read
cargo +nightly fuzz run config_round_trip
```

Corpora live under `fuzz/corpus/<target>/`; libfuzzer seeds itself
from anything it finds there and writes back useful inputs as it
makes progress. Crash reproducers land in `fuzz/artifacts/<target>/`.

### Long runs

Before 1.5 every target ran for half an hour or more, five at a time,
from its seeds (2026-10): no crash, leak, timeout or out-of-memory in
any. Runs per target: `keychord_parse` 1.1 billion, `asset_type_detect`
754 million, `avcc_nalu_walk` 594 million, `cache_deserialize` 578
million, `uri_list_parse` 245 million, `small_parsers` 169 million,
`shimeji_xml` 67 million, `scene_file_read` 50 million,
`thumbnail_decode` 36 million, `config_round_trip` 4.4 million (it
writes the config out and reads it back, each run). After a scene file
began to keep what it cannot read (`SetAside::of_scene`), the two
readers it touches ran again on that code: `scene_file_read` 117
million runs in half an hour, its coverage up from 4381 to 4493 edges,
`config_round_trip` 6.8 million — nothing found. A `slow-unit` left by
the first run was the machine's suspend counted against one input: it
runs in 10 ms. `scene_save_over`, added then, found within its first
half hour that a scene saved over its file rewrote a list it does not
read: two `[[windows]]` with one `id` had the second's keys written into
the first. Fixed (`SetAside::note_unknown_in_lists`), it ran another
half hour: 12 million runs, nothing. To repeat one:

```bash
cargo +nightly fuzz run scene_file_read -- -max_total_time=1800 -rss_limit_mb=2048
```

## CI integration

The fuzz package is not built by default `cargo test` or `cargo
build` runs — it lives in its own `fuzz/Cargo.toml`. Adding a CI job
that runs a short timeboxed fuzz batch on every push catches
regressions early without bloating the main build matrix:

The `fuzz` job in `.github/workflows/ci.yml` (nightly + cargo-fuzz)
fuzzes every target `cargo fuzz list` names — a new one without being
added to the workflow — each corpus seeded from `fuzz/seeds/`: 15 s a
target on every push and pull request, so a parser that panics is caught
the day it lands, and 60 s on the weekly canary and on demand
(**Run workflow**, `workflow_dispatch`). A panic or a sanitizer hit
fails the run, and the reproducer is uploaded as the `fuzz-artifacts`
artifact.

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
