# Flathub submission checklist

Everything needed to get animaEngine on Flathub. The build-side work
is done; the items marked ☐ need a human decision or an asset that
doesn't exist yet.

## Ready ✅

- `eu.animaengine.Anima.flathub.yml` — offline manifest building the
  `v1.5.0` tag (its commit pinned), crates pinned in `cargo-sources.json` (991 entries,
  written from `Cargo.lock` by `scripts/cargo-sources.py`; CI fails
  while the two disagree).
- Metainfo passes `appstreamcli validate` with release entries up to
  1.4.0, OARS rating, launchable, provides, URLs.
- Desktop file + scalable icon installed under the app-id name (the
  manifest rewrites `Icon=` accordingly).
- **Built and run locally** (2026-09-27, from the commit after 1.2.0,
  runtime 24.08): starts on the X11 path, draws with transparency, tray
  registered and its menu readable, a PNG dropped from `~/Downloads`
  added, config kept across a restart, click-through in place. The run
  found and fixed two blockers — no X11 on any Wayland session, and no
  tray (R49 in `docs/runtime-findings.md`) — and confirmed that the
  `xdg-config`/`xdg-cache`/`xdg-data` grants reached nothing, so they
  are gone. **Not checked:** sound inside the sandbox (the test setup
  has no PulseAudio server). To rebuild the same way, point a copy of
  the manifest's git source at a local commit (`url: file://…`,
  `commit: <sha>`) and run `flatpak-builder --repo=… <build-dir> <copy>`.
- Three screenshots in `screenshots/` (pass-through, edit mode, command
  palette; 1600×1000, taken on the headless rig over a generated
  wallpaper) and in the metainfo. Their URLs point at the **`v1.5.0`**
  tag, so they resolve once that tag is pushed — the next release has to
  carry that name, or the URLs change with it. To retake them, stage the
  demo scene on the rig (onboarding dismissed, the star moved out from
  under the panel) and keep the same file names.

## Blockers ☐

### 1. Verify the app id ✅ decided

The id is `eu.animaengine.Anima` from 1.5, after **animaengine.eu**,
the project's domain. It was `com.animaengine.Anima` before, which named
animaengine.com, someone else's (registered since 2025-01-22), and so
could never be verified. The metainfo says the new id `<replaces>` the
old one; the old D-Bus name still answers outside Flatpak
(docs/stability-policy.md).

☐ To get the *verified* checkmark, put the token Flathub gives at
`https://animaengine.eu/.well-known/org.flathub.VerifiedApps.txt`.

## Submission steps

1. Fork <https://github.com/flathub/flathub>, branch from `new-pr`.
2. Copy in `eu.animaengine.Anima.flathub.yml` (renamed to
   `eu.animaengine.Anima.yml`) and `cargo-sources.json`.
3. Open a PR against the `new-pr` branch; CI builds it; a reviewer
   looks at finish-args and metainfo.
4. Typical review asks: justify the filesystem grants (answer: only
   `xdg-pictures:ro` + `xdg-download:ro`, the common drag-drop and
   config-path asset sources; the app's own config, cache and asset
   library live in `~/.var/app/eu.animaengine.Anima/` and need none;
   deliberately *not* `home`, to keep credentials and browser profiles
   out of reach; anything elsewhere comes through the file-chooser
   portal, Scene → Add file…), justify the D-Bus names
   (StatusNotifierItem tray, notifications, GNOME's idle monitor
   read-only, UPower for the battery) and `--socket=pulseaudio`
   (script sounds; no capture code is linked).
5. After merge, the app builds on Flathub's infra; install counts
   appear on the dashboard at <https://flathub.org/apps>.

## Per-release maintenance

On every new tag: bump `tag:`/`commit:` in the manifest, regenerate
`cargo-sources.json` from the new `Cargo.lock`
(`scripts/cargo-sources.py`), add the release entry to the metainfo
(already part of the release ceremony), push to the flathub repo.
Flathub bots open the update PR automatically if you enable them
(recommended: `flathubbot` checker).
