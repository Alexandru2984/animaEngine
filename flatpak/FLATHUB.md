# Flathub submission checklist

Everything needed to get animaEngine on Flathub. The build-side work
is done; the items marked ☐ need a human decision or an asset that
doesn't exist yet.

## Ready ✅

- `com.animaengine.Anima.flathub.yml` — offline manifest building the
  `v0.5.5` tag, crates pinned in `cargo-sources.json` (887 entries,
  regenerate on every release: see header comment in the manifest).
- Metainfo passes `appstreamcli validate` with release entries up to
  0.5.5, OARS rating, launchable, provides, URLs.
- Desktop file + scalable icon installed under the app-id name (the
  manifest rewrites `Icon=` accordingly).

## Blockers ☐

### 1. Screenshots in the metainfo (hard requirement)

Flathub rejects metainfo without at least one `<screenshot>`. After
recording the demo (see `docs/demo-recording.md`), capture 2–3 stills:

- overlay with characters over a desktop (pass-through),
- edit mode with the settings panel open,
- the command palette.

Host them at a stable URL — the conventional choice is raw URLs from a
`screenshots/` directory in this repo at a tagged ref — then add to
`data/com.animaengine.Anima.metainfo.xml`:

```xml
<screenshots>
  <screenshot type="default">
    <image>https://raw.githubusercontent.com/Alexandru2984/animaEngine/v0.5.5/screenshots/overlay.png</image>
    <caption>Animated characters over the desktop, click-through</caption>
  </screenshot>
</screenshots>
```

### 2. App-id / domain decision

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

### 3. Local build test (needs flatpak-builder)

```bash
sudo apt install flatpak-builder
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install -y flathub org.freedesktop.{Platform,Sdk}//24.08 \
    org.freedesktop.Sdk.Extension.rust-stable//24.08

flatpak-builder --force-clean --install-deps-from=flathub \
    --repo=/tmp/anima-repo /tmp/anima-build \
    flatpak/com.animaengine.Anima.flathub.yml
flatpak build-bundle /tmp/anima-repo /tmp/anima.flatpak com.animaengine.Anima
flatpak install --user -y /tmp/anima.flatpak
flatpak run com.animaengine.Anima
```

Things to verify inside the sandbox: tray icon appears, drag-drop
works from ~/Downloads, config persists across restarts, the overlay
is click-through.

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
