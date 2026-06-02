#!/usr/bin/env bash
# Build a Debian (.deb) package from the [package.metadata.deb] block in
# Cargo.toml (set up in Etapa 8.1).
#
# Usage:
#   scripts/build-deb.sh
#
# Outputs:
#   build/anima-engine_<version>_amd64.deb
#
# Prereq: `cargo install cargo-deb` (once per machine).

set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO"

log() { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

if ! command -v cargo-deb >/dev/null 2>&1; then
    die "cargo-deb not found. Install with: cargo install cargo-deb"
fi

log "Building Debian package…"
mkdir -p build

# `cargo deb` rebuilds in release mode and reads
# [package.metadata.deb] for everything else. Output ends up at
# target/debian/. We move it to build/ for parity with the AppImage path.
cargo deb --no-strip

# Find the freshly built .deb (versioned filename) and copy into build/.
DEB="$(ls -t target/debian/*.deb 2>/dev/null | head -1)"
[[ -n "$DEB" ]] || die "cargo deb produced no output"

DEST="build/$(basename "$DEB")"
cp -f "$DEB" "$DEST"
log "Wrote $DEST"
ls -lh "$DEST"

# Inspect contents (sanity-check that assets land at the documented
# XDG paths). dpkg-deb is part of dpkg, always present on Debian/Ubuntu.
if command -v dpkg-deb >/dev/null 2>&1; then
    log "Package contents:"
    dpkg-deb -c "$DEST" | awk '{print "    " $6}'
fi

# Lint with `lintian` if present. Non-fatal — packagers run it; users
# building locally just see the report.
if command -v lintian >/dev/null 2>&1; then
    log "Running lintian…"
    lintian "$DEST" || true
fi
