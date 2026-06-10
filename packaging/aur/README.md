# AUR publishing workflow

The `PKGBUILD` here is the source of truth; the AUR git repo mirrors
it on every release. AUR can't be automated from CI without storing
SSH keys, so the publish step is manual and takes ~3 minutes.

## One-time setup

1. Create an account on <https://aur.archlinux.org> and add your SSH
   public key under *My Account*.
2. Clone the (initially empty) package repo — first push creates it:

```bash
git clone ssh://aur@aur.archlinux.org/anima-engine.git aur-anima-engine
```

## Per release

```bash
cd aur-anima-engine
cp /path/to/animaEngine/packaging/aur/PKGBUILD .

# Regenerate the metadata file the AUR indexes (REQUIRED — the AUR
# rejects pushes whose .SRCINFO doesn't match the PKGBUILD):
makepkg --printsrcinfo > .SRCINFO

git add PKGBUILD .SRCINFO
git commit -m "0.5.5"
git push
```

On a version bump remember to update in `PKGBUILD`: `pkgver`,
`sha256sums` (get it with
`curl -sL <tarball-url> | sha256sum`), and reset `pkgrel=1`.

## Testing the build (needs an Arch box or container)

```bash
podman run --rm -it -v "$PWD:/pkg" -w /pkg archlinux:latest bash -c '
  pacman -Syu --noconfirm base-devel rustup git &&
  rustup default stable &&
  useradd -m builder && chown -R builder /pkg &&
  su builder -c "makepkg -s --noconfirm"
'
```

`namcap PKGBUILD` and `namcap *.pkg.tar.zst` catch most reviewer
complaints (missing depends, misplaced files) before users do.

## Notes

- `depends` are runtime-linked libs only; wgpu loads Vulkan/GL at
  runtime through `vulkan-icd-loader`/`libglvnd`.
- `openh264` builds from vendored C source via the `cc` crate — no
  system codec dependency, but that's why `cmake` sits in
  `makedepends`.
- The `check()` runs the lib test suite (~250 tests, <1 min); drop it
  only if a future flaky test blocks users from installing.
