# shellcheck shell=bash
# Sourced by build-deb.sh and build-appimage.sh from the repository root:
# a release binary that holds nothing of the machine it was built on, and
# comes out byte for byte the same wherever a commit is built. The
# checkout's path and cargo's and rustup's homes are written into it as
# /build, /cargo and /rustup — a plain build carried some 800 paths under
# the builder's home, its user name among them, in panic locations — and
# SOURCE_DATE_EPOCH is the commit's time. Two builds of one commit from two
# directories were checked identical (docs/threat-model.md). Added to
# whatever RUSTFLAGS the caller set; both scripts set the same, so the
# second build is still a cache hit.
_repro_root="$(pwd)"
export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=${_repro_root}=/build --remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo --remap-path-prefix=${RUSTUP_HOME:-$HOME/.rustup}=/rustup"
export SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct 2>/dev/null || date +%s)}"
unset _repro_root
