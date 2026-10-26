#!/usr/bin/env bash
# Build animaEngine for Windows, from Linux: a portable ZIP and a per-user
# installer, in build/.
#
#   build/animaEngine-<version>-windows-x86_64.zip
#   build/animaEngine-<version>-windows-x86_64-setup.exe
#
# Cross-compiled for x86_64-pc-windows-gnu with mingw-w64. libstdc++ — the
# C++ runtime of the H.264 decoder — is linked in statically, from the
# win32 thread model's, so the .exe needs only DLLs every Windows has; the
# script fails if it would need another. The icon and version details
# come from build.rs. The installer is NSIS's
# (packaging/windows/installer.nsi). The ZIP is reproducible: its entries
# carry SOURCE_DATE_EPOCH, and the .exe is built as build-deb.sh builds.
#
# Needs: rustup target add x86_64-pc-windows-gnu
#        apt install gcc-mingw-w64-x86-64 g++-mingw-w64-x86-64 nsis
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

log() { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

TARGET=x86_64-pc-windows-gnu
command -v x86_64-w64-mingw32-g++ >/dev/null || die "no x86_64-w64-mingw32-g++: apt install g++-mingw-w64-x86-64"
command -v makensis >/dev/null || die "no makensis: apt install nsis"
# 22.04's mingw-w64 (GCC 10) cannot link the .exe: libgcc's gthr-win32
# wants __mingwthr_key_dtor, which its runtime does not give. 24.04's
# (GCC 13) can. Said here rather than as a page of linker errors.
# `-dumpversion` says `13-win32` on Debian and Ubuntu: the leading digits.
MINGW_GCC_MAJOR="$(x86_64-w64-mingw32-gcc -dumpversion)"
MINGW_GCC_MAJOR="${MINGW_GCC_MAJOR%%[!0-9]*}"
[[ "$MINGW_GCC_MAJOR" -gt 10 ]] || die "mingw-w64 GCC $MINGW_GCC_MAJOR cannot link it — build on Ubuntu 24.04 or newer (GCC 13)"
rustup target list --installed | grep -qx "$TARGET" || die "rustup target add $TARGET"

VERSION="$(cargo metadata --no-deps --format-version 1 \
    | python3 -c 'import json, sys; print(json.load(sys.stdin)["packages"][0]["version"])')"
# Windows' version resource takes numbers only: 1.5.0 of 1.5.0-rc1.
VERSION_NUM="${VERSION%%[-+]*}"
NAME="animaEngine-${VERSION}-windows-x86_64"
STAGE="build/windows/$NAME"

# libstdc++ statically: the cc crate links what CXXSTDLIB names, and the
# win32 thread model's library needs no winpthread DLL.
STDCXX="$(x86_64-w64-mingw32-g++-win32 -print-file-name=libstdc++.a 2>/dev/null \
    || x86_64-w64-mingw32-g++ -print-file-name=libstdc++.a)"
[[ -f "$STDCXX" ]] || die "no libstdc++.a for mingw-w64"
export CXXSTDLIB_x86_64_pc_windows_gnu="static=stdc++"
STDCXX_DIR="$(dirname "$STDCXX")"
export RUSTFLAGS="${RUSTFLAGS:-} -L native=$STDCXX_DIR"
# shellcheck source=scripts/reproducible-env.sh
source scripts/reproducible-env.sh

log "Building $NAME…"
if command -v cargo-auditable >/dev/null 2>&1; then
    cargo auditable build --release --locked --target "$TARGET"
else
    log "cargo-auditable not installed — building without embedded SBOM"
    cargo build --release --locked --target "$TARGET"
fi
EXE="target/$TARGET/release/anima_engine.exe"
[[ -f "$EXE" ]] || die "no $EXE"
if x86_64-w64-mingw32-objdump -p "$EXE" | grep -qiE "DLL Name: (libstdc|libgcc|libwinpthread)"; then
    die "$EXE needs a mingw runtime DLL Windows does not have"
fi

log "Staging $STAGE…"
rm -rf "$STAGE"
mkdir -p "$STAGE"
cp "$EXE" "$STAGE/anima-engine.exe"
python3 - "$STAGE" "$VERSION" <<'PY'
import sys
from pathlib import Path
stage, version = Path(sys.argv[1]), sys.argv[2]
def crlf(text):
    return text.replace("\r\n", "\n").replace("\n", "\r\n")
(stage / "LICENSE.txt").write_text(crlf(Path("LICENSE").read_text()), encoding="utf-8")
(stage / "THIRD-PARTY-LICENSES.txt").write_text(
    crlf(Path("data/THIRD-PARTY-LICENSES.txt").read_text()), encoding="utf-8")
(stage / "README.txt").write_text(crlf(f"""animaEngine {version} for Windows
{'=' * (len(version) + 24)}

Animated characters on your desktop.

Run anima-engine.exe. The characters appear over your windows, and clicks
go through them to whatever is underneath. The gear button in the
top-right corner of the screen opens edit mode: the settings panel, where
characters are added, changed and arranged. Ctrl+Shift+A does the same
from anywhere. The tray icon has the rest.

Drop a PNG, GIF, WebP or MP4 on the screen in edit mode to make it a
character.

Settings are kept in %APPDATA%\\animaEngine. Appearance -> "Start at
login" starts it with Windows.

The app makes no network connection. It is under the MIT licence
(LICENSE.txt); the open-source components it is built with are listed,
with their licences, in THIRD-PARTY-LICENSES.txt.

https://github.com/Alexandru2984/animaEngine
"""), encoding="utf-8")
PY

log "Packing the ZIP…"
python3 - "$STAGE" "build/$NAME.zip" "$SOURCE_DATE_EPOCH" <<'PY'
import sys, time, zipfile
from pathlib import Path
stage, out, epoch = Path(sys.argv[1]), sys.argv[2], int(sys.argv[3])
stamp = time.gmtime(max(epoch, 315532800))[:6]
with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as z:
    for f in sorted(stage.iterdir()):
        info = zipfile.ZipInfo(f"{stage.name}/{f.name}", date_time=stamp)
        info.compress_type = zipfile.ZIP_DEFLATED
        info.external_attr = 0o644 << 16
        z.writestr(info, f.read_bytes())
PY

log "Building the installer…"
# Absolute paths: makensis reads relative ones from the script's folder.
makensis -V2 -DVERSION="$VERSION" -DVERSION_NUM="$VERSION_NUM" -DSTAGE="$PWD/$STAGE" -DICON="$PWD/data/anima-engine.ico" \
    -DOUTFILE="$PWD/build/$NAME-setup.exe" packaging/windows/installer.nsi

log "Built:"
ls -lh "build/$NAME.zip" "build/$NAME-setup.exe"
