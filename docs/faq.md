# Questions and known problems

The questions people ask most, and the problems we know about and why
they happen. Something not here? [Open an issue](https://github.com/Alexandru2984/animaEngine/issues/new/choose)
with the diagnostic report (Appearance → About → **Copy diagnostic
report**).

## Using it

**I click a character and nothing happens.** Clicks go through the
characters to whatever is underneath: that is what lets them live on
your desktop. To change them, open edit mode — the ⚙ button in the
top-right corner of the screen, or `Ctrl+Shift+A`. To poke them without
it, turn on Appearance → **Characters can be tapped**.

**How do I add a character?** In edit mode, drop a PNG, GIF, WebP, JPEG
or MP4 on the screen, or a folder of PNG frames; or Scene → **Add file…**;
or paste a picture copied anywhere with `Ctrl+V`. The Library tab lists
what is in your asset library, and the presets in the command palette
(`Ctrl+K`) add ready-made scenes.

**A picture is refused.** It is too large (more than 4096 pixels a side,
or more than 256 MB once decoded), in another format, or an MP4 that is
not H.264. The message says which. A video is kept to its first 600
frames, about 20 seconds.

**Where are my settings?**

| Installed as | Settings | Asset library |
|---|---|---|
| a package, an AppImage, from source | `~/.config/animaengine/config.toml` | `~/.local/share/animaengine/assets/` |
| Flatpak | `~/.var/app/com.animaengine.Anima/config/animaengine/` | `~/.var/app/com.animaengine.Anima/data/animaengine/assets/` |
| Windows | `%APPDATA%\animaEngine\config\config.toml` | `%APPDATA%\animaEngine\data\assets\` |

The file can be edited while the app runs; it picks the change up within
two seconds. [config.md](config.md) has every setting.

**How do I start over?** Quit the app and delete `config.toml`: the next
start makes a new one. After a crash, `anima-engine --recover` puts the
scene saved just before it back in place (your config kept beside it as
`config.toml.bak`) and exits: start the app again.

**Does it send anything anywhere?** No. It makes no network connection
at all. The diagnostic report is copied to your clipboard and goes
wherever you paste it, nowhere else.

## Shortcuts

**`Ctrl+Shift+A` does nothing on GNOME or KDE under Wayland.** Global
shortcuts there go through the desktop's shortcuts portal, which asks
you, the first time, whether to allow them. Answer the dialog, and they
work everywhere. If it was dismissed, the desktop asks again at the
next start. Your desktop's settings are where to change or remove them
afterwards. The Keybindings tab says how the shortcuts are delivered.

**On sway, Hyprland or river**, the app has no way to take a key for
itself: bind one in the compositor to the app's D-Bus methods.
[wayland.md](wayland.md) has the lines to paste.

## Performance

**It uses my GPU.** While characters move, the overlay — the size of the
screen — is drawn about sixty times a second. Appearance → **Frames a
second while they move** draws thirty instead, always or only on
battery, at the same speed: on a laptop's integrated GPU that halved the
time drawing took. When nothing moves, it wakes a few times a second at
most. **Pause when I’m away** holds everything still after the time you
pick, and **Pause on battery** while unplugged.

**My laptop has an NVIDIA GPU too.** The app draws on the integrated GPU
and leaves the NVIDIA one asleep, which saves battery and seconds at
start and quit. `prime-run anima-engine`, or the desktop's "Launch using
Discrete Graphics Card", has it draw on the NVIDIA GPU instead.

## Known problems

- **Following the cursor, on native Wayland** (`ANIMA_USE_WAYLAND_NATIVE=1`),
  works in edit mode only: outside it the app is not told where the
  cursor is, by design of the protocol. On X11 and XWayland — the
  default — it works everywhere.
- **Pausing when away, under KDE on Wayland** with the default X11 path,
  is unavailable: no source there tells the app how long you have been
  away. The setting says so. Under GNOME, on a real X server, and on
  wlroots compositors with idle notifications it works.
- **Stepping aside for full-screen apps, under GNOME or KDE on Wayland**,
  sees only apps that run through XWayland: the desktop does not say
  which native Wayland window is full screen.
- **Landing on window edges** (physics) sees X11 windows only: Wayland
  does not say where other windows are, so under a Wayland session it
  finds the apps that run through XWayland.
- **On Windows, SmartScreen warns** before the installer or the `.exe`
  runs: they are not signed yet. "More info" → "Run anyway". The release's
  `SHA256SUMS` file lets you check the download first.
- **Windows support is new** and has been run under Wine so far, not on
  a real Windows. Reports are very welcome.
- **macOS** is not supported.

## Removing it

- `.deb`: `sudo apt remove anima-engine`.
- Flatpak: `flatpak uninstall com.animaengine.Anima` — add `--delete-data`
  to remove its settings too.
- AppImage or a build from source: delete the file; `sudo make uninstall`
  if you used `make install`.
- Windows: Settings → Apps → animaEngine → Uninstall. It takes "Start at
  login" with it.

Except for the Flatpak's `--delete-data`, your settings and characters
stay where the table above says, for a later install. Delete those
folders to remove them too.
