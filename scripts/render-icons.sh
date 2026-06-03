#!/usr/bin/env bash
# Rasterize data/anima-engine.svg into the discrete PNG sizes that
# desktop environments actually look up in the hicolor theme tree.
# Yaru / Adwaita / Breeze all walk the same sizes before falling back
# to scalable/, so we ship 7 PNGs alongside the SVG.
#
# Output: build/icons/<size>/anima-engine.png
#
# Tooling priority: rsvg-convert (cleanest) → inkscape → convert (ImageMagick).
# Any of the three is fine; whichever ships in your distro wins.

set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$REPO/data/anima-engine.svg"
OUT_BASE="$REPO/build/icons"
SIZES=(16 24 32 48 64 128 256)

if [[ ! -f "$SRC" ]]; then
    echo "error: source SVG missing at $SRC" >&2
    exit 1
fi

# Pick the first available rasterizer. rsvg-convert handles our SVG's
# gradients / filters faithfully; ImageMagick is the universal
# fallback every Linux desktop ships.
render() {
    local size="$1" out="$2"
    if command -v rsvg-convert >/dev/null 2>&1; then
        rsvg-convert -w "$size" -h "$size" -o "$out" "$SRC"
    elif command -v inkscape >/dev/null 2>&1; then
        inkscape --export-type=png --export-width="$size" --export-filename="$out" "$SRC" >/dev/null 2>&1
    elif command -v convert >/dev/null 2>&1; then
        convert -background none -resize "${size}x${size}" "$SRC" "$out"
    else
        echo "error: no SVG rasterizer found (need rsvg-convert, inkscape, or imagemagick)" >&2
        exit 1
    fi
}

echo "Rendering icons from $SRC"
for size in "${SIZES[@]}"; do
    out_dir="$OUT_BASE/$size"
    mkdir -p "$out_dir"
    out="$out_dir/anima-engine.png"
    render "$size" "$out"
    printf "  %dx%d → %s (%s)\n" "$size" "$size" "$out" "$(stat -c '%s bytes' "$out")"
done
echo "Done. Icons live at $OUT_BASE/<size>/anima-engine.png"
