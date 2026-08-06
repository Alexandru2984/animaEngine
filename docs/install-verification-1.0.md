# 1.0 install verification

Contract point #5 — *installable from GitHub Releases, Flathub and AUR* —
made checkable. Each distribution channel is installed **from scratch on
a clean machine** (a throwaway VM or container, no dev toolchain, no
prior animaEngine state) and the result recorded below. This file is the
protocol *and* the log: the method is pre-registered so a run is just
execute-and-record, not invent-as-you-go.

Run these against the **RC build** (X.0), and re-run any channel whose
artifact changes before the final tag.

## Method (all channels)

1. Boot a clean VM/container of the target image. No `~/.config`,
   `~/.cache`, or `~/.local/share` for animaEngine; no Rust toolchain.
2. Install **only** via the channel's own published path — no manual
   dependency fixups. If a dependency is missing, that's a finding.
3. Launch, then walk the **smoke checklist** below.
4. Record the image, the exact commands, and the result in the log.

### Smoke checklist (per channel, after install)

- [ ] App launches; the ⚙ toggle button is visible top-right.
- [ ] Enter edit mode; the settings panel opens (Inspector / Scene /
      Library / Appearance tabs render).
- [ ] Drop a PNG/GIF onto the overlay → it becomes an animated entity
      (from an allowed source dir — see the Flatpak note).
- [ ] Tray icon appears; quit via the tray works.
- [ ] `~/.config/animaEngine/config.toml` is written.
- [ ] No missing-library / missing-portal error in
      `RUST_LOG=anima_engine=info` output.

## Turnkey commands — GitHub channels

The two GitHub-Releases artifacts (`.deb`, AppImage) are the only
channels runnable against an RC: Flathub and AUR install from their own
published manifests, which are version-bumped **at the final 1.0 tag**
(see below), so they can't be exercised against an rc build. Run these
on each clean target VM once the release is published:

```bash
TAG=v1.0.0-rc3   # or the final v1.0.0

# Fetch every asset for the tag (works on the published release).
gh release download "$TAG" --dir "$TAG" && cd "$TAG"
# …no gh on the VM? explicit URLs instead:
#   BASE=https://github.com/Alexandru2984/animaEngine/releases/download/$TAG
#   wget -q "$BASE/SHA256SUMS-$TAG.txt" \
#          "$BASE/anima-engine_1.0.0.rc3-1_amd64.deb" \
#          "$BASE/animaEngine-1.0.0-rc3-x86_64.AppImage"

sha256sum -c "SHA256SUMS-$TAG.txt"        # must print OK for both files

# .deb — clean Ubuntu 22.04 AND 24.04
sudo apt install ./anima-engine_*_amd64.deb
RUST_LOG=anima_engine=info anima-engine   # walk the smoke checklist; quit via tray

# AppImage — clean Ubuntu 22.04 (proves the glibc 2.35 floor the rc3
# build fix restored) AND one non-Debian distro (Fedora)
chmod +x animaEngine-*-x86_64.AppImage
RUST_LOG=anima_engine=info ./animaEngine-*-x86_64.AppImage
```

The checksum step is the load-bearing one: it proves the tilde→dot
rename holds (`sha256sum -c` fails otherwise) before you trust the rest.

## Channels

### GitHub Releases — `.deb`

- **Image:** clean Ubuntu 24.04 (and one older still-supported LTS).
- **Install:** `sudo apt install ./anima-engine_<ver>-1_amd64.deb`
- **Pass:** installs with its declared deps, launches, smoke checklist
  green. Verify the published `SHA256SUMS` matches the downloaded file.

### GitHub Releases — AppImage

- **Image:** clean Ubuntu 24.04 + one non-Debian distro (e.g. Fedora).
- **Install:** `chmod +x animaEngine-<ver>-x86_64.AppImage && ./…`
- **Pass:** runs with no system animaEngine install and no manual deps;
  smoke checklist green on both distros.

### Flathub

> **Deferred until the final 1.0 tag.** The Flathub manifest is still
> pinned to `v0.9.0`; it is bumped to `v1.0.0` and re-verified as part of
> cutting the final release, so there is nothing to test on an rc build.

- **Image:** clean GNOME and clean KDE session (the two that matter for
  the sandbox + tray).
- **Install:** `flatpak install flathub com.animaengine.Anima`
- **Pass:** installs from the live listing, launches **in the sandbox**,
  smoke checklist green. Specifically confirm: tray icon appears,
  drag-drop of an asset **from `~/Pictures` or `~/Downloads`** works
  (the scoped filesystem grant — assets elsewhere are out of scope until
  the file-chooser portal lands), and global shortcuts register via the
  GlobalShortcuts portal where the compositor offers it.

### AUR

> **Deferred until the final 1.0 tag.** The PKGBUILD `pkgver` is still
> `0.5.5`; it is bumped to `1.0.0` and re-verified as part of cutting the
> final release. Not runnable on an rc build.

- **Image:** clean Arch container (`archlinux:latest`).
- **Build:** `makepkg -si` from the PKGBUILD on a non-root build user;
  then `namcap` the PKGBUILD and the built package.
- **Pass:** builds and installs cleanly, `namcap` reports no errors
  (warnings triaged in the log), smoke checklist green.

## Log

Fill one row per run. Keep failures in the table with a link to the
issue — the point is a dated record, not a clean slate.

| Date | Channel | Image | Version | Result | Notes / issue |
|------|---------|-------|---------|--------|---------------|
|      | .deb | Ubuntu 24.04 | | | |
|      | .deb | Ubuntu 22.04 | | | |
|      | AppImage | Ubuntu 22.04 | | | glibc 2.35 floor — the rc3 build fix |
|      | AppImage | Ubuntu 24.04 | | | |
|      | AppImage | Fedora 40 | | | |
|      | Flathub | GNOME (Wayland) | | | *(final tag only)* |
|      | Flathub | KDE (Wayland) | | | *(final tag only)* |
|      | AUR | archlinux:latest | | | *(final tag only)* |

## Pre-RC from-source smoke runs (dev)

Separate from the channel protocol above: ad-hoc `cargo build --release`
+ run on clean VMs while hardening toward 1.0, to catch
build-dependency and display-server gaps the GitHub CI runners hide (CI
ships `pkg-config` preinstalled and renders at scale 1.0). These are
**from source**, not channel installs, and ran in Proxmox VMs **without
GPU passthrough** (software-rendered via llvmpipe) — so they verify
build + correctness, not performance.

Three distinct session types exercised so far — X11, GNOME Wayland
(XWayland fallback), and wlroots (native layer-shell):

| Date | Distro / host | Session | Result | Found & fixed |
|------|---------------|---------|--------|---------------|
| 2026-06-28 | Kali (XFCE) | X11 native | ✅ build + overlay works | docs missing `pkg-config` / `libxkbcommon-x11-dev`; overlay window oversized on fractional scale (⚙ button off-screen) |
| 2026-06-28 | Fedora | GNOME Wayland → XWayland | ✅ build + overlay works | Mutter dropped always-on-top → re-assert `_NET_WM_STATE_ABOVE` on focus/occlusion; click-through + stay-on-top both confirmed |
| 2026-06-28 | headless `sway` (dev host) | wlroots, native layer-shell | ✅ logic only (no visible output) | multi-monitor PerMonitor surface create / hotplug / drop-order crashes — see docs/wayland.md |
| 2026-06-29 | Arch (GNOME) | GNOME Wayland → XWayland | ✅ build + overlay works | a focus-change re-assert clobbered the XShape click-through (overlay swallowed clicks) → shape-last + 500ms self-heal; also surfaced that XShape click-through can't reach **native-Wayland** windows under the overlay (XWayland boundary — see docs, not fixable on this path) |
| 2026-06-29 | Alpine (XFCE) | X11 native | ⚙️ build ✅, run blocked | musl build needs `RUSTFLAGS="-C target-feature=-crt-static"` (Alpine ships no static `libxkbcommon.a`) — now in CONTRIBUTING; runtime then refused (no transparent alpha mode on the VM's software GL surface) — correct defensive behavior, needs a real GPU/WSI or transparent-capable compositor |

The maintainer's own Ubuntu GNOME-Wayland box additionally runs it on a
real GPU (Vulkan/RADV). Build portability now covered across glibc +
musl and X11 + Wayland; the remaining gaps are runtime-environment
(software VMs lack a transparent surface) and the post-1.0 native-path
items, not build breakage.

## Definition of done

Every channel has at least one **green** row against the version that
becomes 1.0.0. A channel that can't go green by the tag date is a
release blocker, not a footnote (see the v1.0 contract's failure mode).
