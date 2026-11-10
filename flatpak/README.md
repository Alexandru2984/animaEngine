# Flatpak

`eu.animaengine.Anima.yml` is the manifest that drives `flatpak-builder`.
The local build path (`scripts/build-flatpak.sh`, invoked by
`make flatpak`) produces a single `.flatpak` bundle you can install with
`flatpak install --user`.

## Local build

Once-per-machine setup:

```bash
sudo apt install flatpak flatpak-builder
flatpak remote-add --if-not-exists flathub \
    https://flathub.org/repo/flathub.flatpakrepo
flatpak install -y flathub \
    org.freedesktop.Sdk//24.08 \
    org.freedesktop.Platform//24.08 \
    org.freedesktop.Sdk.Extension.rust-stable//24.08
```

Then from the repo root:

```bash
make flatpak
flatpak install --user -y build/eu.animaengine.Anima.flatpak
flatpak run eu.animaengine.Anima
```

## Flathub submission

The local manifest (`eu.animaengine.Anima.yml`) **lets cargo fetch
crates over the network at build time**, which Flathub forbids. The
submission manifest is `eu.animaengine.Anima.flathub.yml`: it builds
offline from `cargo-sources.json`. What is done and what is still open —
the domain decision and a local build test — is tracked in
[FLATHUB.md](FLATHUB.md).

The app-id is `eu.animaengine.Anima` from 1.5 (it was
`com.animaengine.Anima`, of a domain the project never held). Flatpak
lets an app own only the D-Bus name equal to its id, so inside the
Flatpak only the new name answers; the native packages answer to the
old one too, as the [stability policy](../docs/stability-policy.md)
keeps it for the rest of 1.x. No Flatpak was published under the old
id.

## Permissions explained

The `finish-args` block in the manifest is the security surface a user
sees in Flatseal / GNOME Software:

| Arg | Why we need it |
|-----|----------------|
| `--socket=x11` | The overlay draws through X11 — XWayland on a Wayland session — for XShape click-through and always-on-top. X11 only: adding `wayland` + `fallback-x11` withdraws X11 on every Wayland session, and the app could not start on GNOME or KDE. The opt-in native Wayland backend is therefore not available in the Flatpak. |
| `--share=ipc` | Required for X11 shared memory. |
| `--device=dri` | wgpu needs GPU access (Vulkan / OpenGL). |
| `--socket=pulseaudio` | Sound for scripts' `play()`. Flatpak has no playback-only socket; no capture code is linked (docs/threat-model.md). |
| `--talk-name=org.kde.StatusNotifierWatcher` | System tray icon. Inside Flatpak the item registers under its connection's unique name, since the sandbox cannot own the usual `org.kde.StatusNotifierItem-PID-N`. |
| `--talk-name=org.freedesktop.Notifications` | Future: toast → desktop notification on minimize. |
| `--own-name=eu.animaengine.Anima` | Single-instance D-Bus name, and the D-Bus methods compositor keybinds call. |
| `--filesystem=xdg-pictures:ro`, `--filesystem=xdg-download:ro` | Files dropped on the overlay usually come from these. Read-only. |

Config, cache and the asset library need no grant: inside the sandbox
they live under `~/.var/app/eu.animaengine.Anima/`, which the app owns.

**Drops from anywhere else do nothing.** A file dragged in from outside
Pictures and Downloads is not visible inside the sandbox, and the
windowing toolkit discards such a drop before the app sees it, so no
message appears either. Use **Add file…** in the Scene tab instead: it
goes through the desktop's file chooser portal, which grants the sandbox
exactly the files picked, wherever they are.

What we explicitly **don't** request:

- `--share=network` — we never make network calls.
- `--filesystem=home` (writable) — we only read user files.
- `--system-talk-name` — no system-bus access.
- `--device=all` — `dri` is enough; no microphone, camera, USB.
