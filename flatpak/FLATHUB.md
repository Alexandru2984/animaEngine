# Flathub submission checklist

Everything needed to get animaEngine on Flathub. The build-side work
is done; the items marked ☐ need a human decision or an asset that
doesn't exist yet.

## Ready ✅

- `com.animaengine.Anima.flathub.yml` — offline manifest building the
  `v1.2.0` tag, crates pinned in `cargo-sources.json` (1045 entries,
  regenerate on every release: see header comment in the manifest).
- Metainfo passes `appstreamcli validate` with release entries up to
  1.2.0, OARS rating, launchable, provides, URLs.
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
  wallpaper) and in the metainfo. Their URLs point at the **`v1.2.1`**
  tag, so they resolve once that tag is pushed — the next release has to
  carry that name, or the URLs change with it. To retake them, stage the
  demo scene on the rig (onboarding dismissed, the star moved out from
  under the panel) and keep the same file names.

## Blockers ☐

### 1. App-id / domain decision

The id `com.animaengine.Anima` implies control of `animaengine.com`.
Flathub's verification rules:

- **Own the domain** (or buy it, ~10 €/yr): put a token at
  `https://animaengine.com/.well-known/org.flathub.VerifiedApps.txt`
  and the listing gets the *verified* checkmark. No code changes.
- **Don't own it**: Flathub may still accept the submission, but it
  can never be verified, and a future rename to
  `io.github.alexandru2984.animaEngine` is painful (desktop file,
  D-Bus name, icon names, existing users' config paths). Decide
  *before* submitting.

## Submission steps

1. Fork <https://github.com/flathub/flathub>, branch from `new-pr`.
2. Copy in `com.animaengine.Anima.flathub.yml` (renamed to
   `com.animaengine.Anima.yml`) and `cargo-sources.json`.
3. Open a PR against the `new-pr` branch; CI builds it; a reviewer
   looks at finish-args and metainfo.
4. Typical review asks: justify the filesystem grants (answer: the
   app's own asset library `xdg-data/animaEngine:create`, plus
   `xdg-pictures:ro` + `xdg-download:ro` as the common drag-drop /
   config-path asset sources — deliberately *not* `home`, to keep
   credentials and browser profiles out of reach; a file-chooser portal
   for arbitrary locations is post-1.0), justify `--talk-name`s
   (StatusNotifierItem tray + notifications).
5. After merge, the app builds on Flathub's infra; install counts
   appear on the dashboard at <https://flathub.org/apps>.

## Per-release maintenance

On every new tag: bump `tag:`/`commit:` in the manifest, regenerate
`cargo-sources.json` from the new `Cargo.lock`, add the release entry
to the metainfo (already part of the release ceremony), push to the
flathub repo. Flathub bots open the update PR automatically if you
enable them (recommended: `flathubbot` checker).
