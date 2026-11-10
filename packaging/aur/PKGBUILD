# AUR package for animaEngine. See packaging/aur/README.md for the
# publish workflow (this file lives in the app repo as the source of
# truth; the AUR repo mirrors it per release).

pkgname=animaengine
pkgver=1.5.0
pkgrel=1
pkgdesc="Linux-first animated desktop overlay engine — render animated characters over your desktop"
arch=('x86_64' 'aarch64')
url="https://github.com/Alexandru2984/animaEngine"
license=('MIT')
# alsa-lib: the default `audio` feature links libasound (script sounds).
depends=('libxkbcommon-x11' 'vulkan-icd-loader' 'libglvnd' 'wayland' 'libxrandr' 'alsa-lib')
makedepends=('cargo' 'librsvg')
# makepkg builds C and C++ with GCC's LTO by default; the H.264 decoder
# (openh264, built from source) then carries GCC bytecode that rustc's
# linker cannot read: "undefined symbol: WelsCreateDecoder".
options=('!lto')
source=("$pkgname-$pkgver.tar.gz::$url/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('8e41abbc64bafda3947f8bf2ee0bb0d88e6609bad94a0f2edc5c435039b8b261')

_srcdir="animaEngine-$pkgver"

prepare() {
  cd "$_srcdir"
  export RUSTUP_TOOLCHAIN=stable
  cargo fetch --locked --target "$(rustc -vV | sed -n 's/host: //p')"
}

build() {
  cd "$_srcdir"
  export RUSTUP_TOOLCHAIN=stable
  export CARGO_TARGET_DIR=target
  cargo build --frozen --release
  # Rasterize the hicolor PNG set (16…256 px) from the scalable icon.
  scripts/render-icons.sh
}

check() {
  cd "$_srcdir"
  export RUSTUP_TOOLCHAIN=stable
  cargo test --frozen --release --lib
}

package() {
  cd "$_srcdir"
  install -Dm0755 target/release/anima_engine "$pkgdir/usr/bin/anima-engine"
  install -Dm0644 data/eu.animaengine.Anima.desktop \
    "$pkgdir/usr/share/applications/eu.animaengine.Anima.desktop"
  install -Dm0644 data/anima-engine.svg \
    "$pkgdir/usr/share/icons/hicolor/scalable/apps/anima-engine.svg"
  for size in 16 24 32 48 64 128 256; do
    install -Dm0644 "build/icons/$size/anima-engine.png" \
      "$pkgdir/usr/share/icons/hicolor/${size}x${size}/apps/anima-engine.png"
  done
  install -Dm0644 data/eu.animaengine.Anima.metainfo.xml \
    "$pkgdir/usr/share/metainfo/eu.animaengine.Anima.metainfo.xml"
  install -Dm0644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
  # Every crate's licence, from 1.5 on (scripts/third-party-licenses.py).
  # The manual page, from 1.5.
  if [ -f data/anima-engine.1 ]; then
    install -Dm0644 data/anima-engine.1 "$pkgdir/usr/share/man/man1/anima-engine.1"
  fi
  if [ -f data/THIRD-PARTY-LICENSES.txt ]; then
    install -Dm0644 data/THIRD-PARTY-LICENSES.txt \
      "$pkgdir/usr/share/licenses/$pkgname/THIRD-PARTY-LICENSES.txt"
  fi
  install -Dm0644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
}
