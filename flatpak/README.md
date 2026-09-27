# Flatpak

`com.animaengine.Anima.yml` is the manifest that drives `flatpak-builder`.
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
flatpak install --user -y build/com.animaengine.Anima.flatpak
flatpak run com.animaengine.Anima
```

## Flathub submission

The local manifest (`com.animaengine.Anima.yml`) **lets cargo fetch
crates over the network at build time**, which Flathub forbids. The
submission manifest is `com.animaengine.Anima.flathub.yml`: it builds
offline from `cargo-sources.json`. What is done and what is still open —
the domain decision and a local build test — is tracked in
[FLATHUB.md](FLATHUB.md).

The app-id stays `com.animaengine.Anima`. Flatpak lets an app own only
the D-Bus name equal to its id, and the
[stability policy](../docs/stability-policy.md) keeps that bus name for
the whole 1.x series, so a rename would break every compositor keybind
that calls it.

## Permissions explained

The `finish-args` block in the manifest is the security surface a user
sees in Flatseal / GNOME Software:

| Arg | Why we need it |
|-----|----------------|
| `--socket=x11` + `--socket=wayland` + `--socket=fallback-x11` | Two display servers; binary picks at runtime. |
| `--share=ipc` | Required for X11 shared memory. |
| `--device=dri` | wgpu needs GPU access (Vulkan / OpenGL). |
| `--talk-name=org.kde.StatusNotifierWatcher` | System tray icon. |
| `--talk-name=org.freedesktop.Notifications` | Future: toast → desktop notification on minimize. |
| `--own-name=com.animaengine.Anima` | Single-instance D-Bus name (Faza 6.3). |
| `--filesystem=home:ro` | Read assets the user drops on the overlay from anywhere in their home. Read-only so we can never delete or overwrite the user's files. |
| `--filesystem=xdg-config/animaEngine` | Persistent config at `~/.config/animaEngine/config.toml`. |
| `--filesystem=xdg-cache/animaEngine` | On-disk decoded-frame cache (Faza 2.4). |

What we explicitly **don't** request:

- `--share=network` — we never make network calls.
- `--filesystem=home` (writable) — we only read user files.
- `--system-talk-name` — no system-bus access.
- `--device=all` — `dri` is enough; no microphone, camera, USB.
