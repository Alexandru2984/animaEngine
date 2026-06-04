#!/usr/bin/env bash
# Build a self-contained anima-engine.AppImage.
#
# Usage:
#   scripts/build-appimage.sh
#
# Outputs:
#   build/animaEngine-<version>-x86_64.AppImage
#
# What it does:
#   1. cargo build --release --locked
#   2. Stages the binary + .desktop + icon + metainfo into build/AppDir
#      (re-uses `make install DESTDIR=...` so the layout matches every
#      other packaging path).
#   3. Downloads linuxdeploy once into build/tools/.
#   4. Runs linuxdeploy to bundle library dependencies and pack the
#      AppImage.
#
# Glibc note: the resulting AppImage works on any distro whose glibc is
# at least as old as the one on the build machine. For broad reach build
# on Ubuntu 22.04 (glibc 2.35) or older.

set -euo pipefail

# Resolve repo root regardless of where the script is invoked from.
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO"

VERSION="$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)"
ARCH="x86_64"
BUILD_DIR="$REPO/build"
APPDIR="$BUILD_DIR/AppDir"
TOOLS_DIR="$BUILD_DIR/tools"
OUTPUT="$BUILD_DIR/animaEngine-${VERSION}-${ARCH}.AppImage"

LINUXDEPLOY="$TOOLS_DIR/linuxdeploy-${ARCH}.AppImage"
LINUXDEPLOY_URL="https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-${ARCH}.AppImage"

log() { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

# ── 1) Release build ─────────────────────────────────────────────────
log "Building release binary…"
cargo build --release --locked

if [[ ! -x target/release/anima_engine ]]; then
    die "release binary missing at target/release/anima_engine"
fi

# ── 2) Stage AppDir via the existing Makefile rules ───────────────────
log "Staging AppDir at $APPDIR…"
rm -rf "$APPDIR"
make install DESTDIR="$APPDIR" PREFIX=/usr >/dev/null

# linuxdeploy also expects top-level .desktop + icon as siblings of
# AppRun. We symlink them rather than copy so a single source of truth
# remains under usr/share/.
# 0.3.2 renamed the .desktop file to match the AppStream <id>
# (GNOME Shell 47+ icon-resolution requirement). The symlink target
# follows the new filename; linuxdeploy then uses it as the canonical
# AppImage .desktop entry.
ln -sf usr/share/applications/com.animaengine.Anima.desktop "$APPDIR/com.animaengine.Anima.desktop"
ln -sf usr/share/icons/hicolor/scalable/apps/anima-engine.svg "$APPDIR/anima-engine.svg"

# ── 3) Fetch linuxdeploy on demand ───────────────────────────────────
mkdir -p "$TOOLS_DIR"
if [[ ! -x "$LINUXDEPLOY" ]]; then
    log "Downloading linuxdeploy…"
    if ! curl -fL -o "$LINUXDEPLOY" "$LINUXDEPLOY_URL"; then
        die "Failed to fetch linuxdeploy. Re-run with network or place the
        AppImage manually at $LINUXDEPLOY."
    fi
    chmod +x "$LINUXDEPLOY"
fi

# ── 4) Pack ───────────────────────────────────────────────────────────
log "Packing AppImage with linuxdeploy…"
# `--output appimage` triggers the embedded appimagetool step.
# `--executable` lists every binary linuxdeploy should walk for deps.
#
# `--library` lists dlopen()'d libraries that linuxdeploy can't discover
# by walking the ELF NEEDED tags. accesskit_unix opens libxkbcommon-x11
# this way; we resolve a candidate path on the build host and bundle it
# so the AppImage runs on systems that don't ship the package.
XKB_X11_LIB="$(ldconfig -p | awk '/libxkbcommon-x11\.so\.0/ {print $NF; exit}')"
if [[ -z "$XKB_X11_LIB" || ! -f "$XKB_X11_LIB" ]]; then
    die "libxkbcommon-x11.so.0 not found on the build host (install libxkbcommon-x11-dev)."
fi
log "Bundling $XKB_X11_LIB"

OUTPUT_DIR="$BUILD_DIR" \
ARCH="$ARCH" \
VERSION="$VERSION" \
"$LINUXDEPLOY" \
    --appdir "$APPDIR" \
    --desktop-file "$APPDIR/com.animaengine.Anima.desktop" \
    --icon-file "$APPDIR/anima-engine.svg" \
    --executable "$APPDIR/usr/bin/anima-engine" \
    --library "$XKB_X11_LIB" \
    --output appimage

# linuxdeploy writes animaEngine-<version>-x86_64.AppImage into the cwd
# unless OUTPUT_DIR is honored. Move it explicitly to be safe.
shopt -s nullglob
for f in "$REPO"/animaEngine*"${ARCH}".AppImage; do
    mv "$f" "$OUTPUT"
done
shopt -u nullglob

if [[ ! -f "$OUTPUT" ]]; then
    die "AppImage build finished but no output produced. Check linuxdeploy logs above."
fi

log "Built $OUTPUT"
ls -lh "$OUTPUT"
