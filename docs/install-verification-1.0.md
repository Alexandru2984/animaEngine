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
|      | AppImage | Ubuntu 24.04 | | | |
|      | AppImage | Fedora 40 | | | |
|      | Flathub | GNOME (Wayland) | | | |
|      | Flathub | KDE (Wayland) | | | |
|      | AUR | archlinux:latest | | | |

## Definition of done

Every channel has at least one **green** row against the version that
becomes 1.0.0. A channel that can't go green by the tag date is a
release blocker, not a footnote (see the v1.0 contract's failure mode).
