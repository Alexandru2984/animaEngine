#!/usr/bin/env bash
# Build a Flatpak bundle of animaEngine using flatpak-builder.
#
# Usage:
#   scripts/build-flatpak.sh
#
# Outputs:
#   build/com.animaengine.Anima.flatpak     (single-file bundle, sharable)
#
# Prereqs (install once per machine):
#   sudo apt install flatpak flatpak-builder
#   flatpak remote-add --if-not-exists flathub \
#     https://flathub.org/repo/flathub.flatpakrepo
#   flatpak install -y flathub \
#     org.freedesktop.Sdk//24.08 \
#     org.freedesktop.Platform//24.08 \
#     org.freedesktop.Sdk.Extension.rust-stable//24.08

set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO"

APP_ID="com.animaengine.Anima"
MANIFEST="flatpak/${APP_ID}.yml"
BUILD_DIR="$REPO/build"
STAGE_DIR="$BUILD_DIR/flatpak-build"
STATE_DIR="$BUILD_DIR/flatpak-state"
REPO_DIR="$BUILD_DIR/flatpak-repo"
BUNDLE="$BUILD_DIR/${APP_ID}.flatpak"

log() { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

command -v flatpak-builder >/dev/null 2>&1 \
    || die "flatpak-builder not found. Install with: sudo apt install flatpak-builder"

mkdir -p "$BUILD_DIR"

# ── 1) Build into a local OSTree repo. --force-clean wipes prior state. ──
log "Building $APP_ID (this can take a few minutes the first time)…"
flatpak-builder \
    --force-clean \
    --state-dir="$STATE_DIR" \
    --repo="$REPO_DIR" \
    --install-deps-from=flathub \
    "$STAGE_DIR" \
    "$MANIFEST"

# ── 2) Pack the OSTree branch into a single .flatpak file ───────────────
log "Bundling to $BUNDLE…"
flatpak build-bundle "$REPO_DIR" "$BUNDLE" "$APP_ID"

log "Built $BUNDLE"
ls -lh "$BUNDLE"

cat <<EOF

Install + run locally:
  flatpak install --user -y $BUNDLE
  flatpak run $APP_ID

Uninstall:
  flatpak uninstall --user -y $APP_ID
EOF
