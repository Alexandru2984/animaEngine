# Flathub submission checklist

Everything needed to get animaEngine on Flathub. The build-side work
is done; the items marked ☐ need a human decision or an asset that
doesn't exist yet.

## Ready ✅

- `eu.animaengine.Anima.flathub.yml` — offline manifest building the
  `v1.5.2` tag (its commit pinned) on runtime **26.08** (24.08 went
  end-of-life on 2026-10-21), crates pinned in `cargo-sources.json` (991
  entries, written from `Cargo.lock` by `scripts/cargo-sources.py`; CI
  fails while the two disagree). `flatpak-builder-lint manifest` passes
  on it under the name it goes in as, `eu.animaengine.Anima.yml`.
- Metainfo passes `appstreamcli validate` and `flatpak-builder-lint
  appstream`, with release entries up to 1.5.2, OARS rating, launchable,
  provides, homepage (animaengine.eu), vcs-browser and bug tracker.
- Desktop file + scalable icon installed under the app-id name (the
  manifest rewrites `Icon=` accordingly).
- **Built on 26.08** (2026-11-14, v1.5.2, offline from
  `cargo-sources.json`, 2 min 23 s). Run in the test rig, the app owns
  its D-Bus name with no grant and registers its tray item from the
  sandbox. It could not draw there: the rig's XWayland has no DRI3,
  which Mesa 26 needs to present, and the only adapter left, NVIDIA's
  Vulkan, offers an opaque X11 surface only. A real desktop has DRI3; the
  run on one is the open item below.
- **Built and run locally before** (2026-09-27, from the commit after 1.2.0,
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
  wallpaper) and in the metainfo. Their URLs point at the `v1.5.0` tag,
  pushed; they resolve (checked 2026-11-14). To retake them, stage the
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

### 2. A run of the 26.08 build on a real desktop ☐

The rig cannot show the X11 path drawing (above). On a desktop session:
`flatpak install --user build/flathub-pr/animaEngine-1.5.2-runtime-26.08-test.flatpak`
(made by `flatpak build-bundle` from the local build; not in git), then
`flatpak run eu.animaengine.Anima`: the characters over the desktop with
it showing through, the tray, a reminder as a notification (through the
portal), a sound, pausing when away. `flatpak uninstall --user
eu.animaengine.Anima` afterwards.

### 3. NVIDIA alone on X11 ☐

In the rig, NVIDIA's Vulkan driver gave the X11 surface `[Opaque]` only,
and wgpu's GL backend reports `Opaque` for every surface (wgpu-hal 24,
`gles/adapter.rs`, a TODO). If that holds on a real X server, a machine
whose only GPU is NVIDIA cannot start the X11 path — the `.deb` and the
AppImage as much as the Flatpak — and says so (`pick_alpha_mode`). To
check on a hybrid laptop: `__NV_PRIME_RENDER_OFFLOAD=1
__VK_LAYER_NV_optimus=NVIDIA_only anima-engine` and look for
"Available alpha modes" in the log.

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
   (the StatusNotifierWatcher for the tray, GNOME's idle monitor
   read-only, UPower on the system bus for the battery, read-only; the
   single-instance name is the app id, which needs no grant, and
   notifications go through the portal, which needs none either) and `--socket=pulseaudio`
   (script sounds; no capture code is linked).
5. After merge, the app builds on Flathub's infra; install counts
   appear on the dashboard at <https://flathub.org/apps>.

## Per-release maintenance

Once a year or so: a freedesktop runtime lives two years, and Flathub
refuses an end-of-life one. `flatpak remote-info flathub
org.freedesktop.Platform//<version>` says when it is; then move
`runtime-version` in both manifests, `flatpak/README.md` and
`scripts/build-flatpak.sh`, build and run it.

On every new tag: bump `tag:`/`commit:` in the manifest, regenerate
`cargo-sources.json` from the new `Cargo.lock`
(`scripts/cargo-sources.py`), add the release entry to the metainfo
(already part of the release ceremony), push to the flathub repo.
Flathub bots open the update PR automatically if you enable them
(recommended: `flathubbot` checker).
