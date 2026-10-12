# 1.5 on a real GNOME session — checklist

Everything in 1.5 has been run in the headless test rig (sway, software
rendering, a private D-Bus and PipeWire, stand-ins for Mutter's idle
monitor, the portals and the notification server). What a rig cannot
show is how the real pieces behave: GNOME's portals and their dialogs,
Mutter, the notification daemon, Orca, a sound card, a GPU, Nautilus,
the AppIndicator tray. This is the list for that, in an order that
needs no restarts beyond those it asks for. Tick as you go; note
anything off under **Log** at the end.

On GNOME the overlay runs the X11 path, through XWayland (GNOME has no
layer-shell), so "X11" below is what you are testing.

## Before you start

```bash
cp -a ~/.config/animaengine ~/animaengine-config-before-1.5   # the tests change settings
cargo build --release
RUST_LOG=anima_engine=info ./target/release/anima_engine 2>&1 | tee /tmp/anima-1.5.log
```

Keep that terminal: several checks read the log. `Ctrl+Shift+A` opens
edit mode from anywhere.

## 1. Start

- [ ] **What's new, once.** Edit mode shows "What's new in 1.5" at the
      top of the panel: six rows (characters, your day, editing,
      scenes, sounds, the keyboard). Dismiss it with ✕, quit, start again: it does not come
      back.
- [ ] **The GPU, Vulkan only.** The log has `GPU adapter: … (RADV …)`
      and `Backend: Vulkan`, and no `trying GL as well`.
      `ps -o nlwp= -p $(pidof anima_engine)` says about 15–25 threads
      (1.4 had about 60 on this machine: 32 decoder threads that never
      went away, and GL set up for nothing).
- [ ] **Your 1.4 config, upgraded.** The first start reads your own
      `config.toml` from 1.4: your characters, groups and settings are
      all there, no banner says something did not read, and a shortcut
      you had changed in 1.4 still does what it did. *Cycle character*
      is on `N` now (its `Tab` was the panel's); one you had set by hand
      is kept.

## 2. Characters

- [ ] **Taps keep your keyboard.** Appearance → *Characters can be
      tapped*. Type in a text editor, tap a character, keep typing: it
      hops, says a line, a few hearts rise — and the letters still go
      to the editor. A click on empty desktop still reaches the window
      under it. *(The rig cannot show this: sway focuses XWayland
      windows on any click.)*
- [ ] **A sound on a tap, from where it stands.** Inspector → *Says* →
      a sound from the library; tap it on the left of the screen, then
      on the right: heard from that side.
- [ ] **Volume and mute.** Appearance → *Volume* at 50%: clearly
      quieter; tray → *Mute sounds* in the middle of a long sound: it
      stops at once, and taps stay silent; *Unmute sounds* brings them
      back. Quit and start: the mute (or not) is as you left it.
- [ ] **Sound survives a sound-server restart.** With a character
      playing often (a tap a second will do):
      `systemctl --user restart pipewire pipewire-pulse wireplumber`.
      Within ~10 s its sounds are back; the log says once
      `Audio output stopped taking sound …`. *(Rig: fixed and checked
      with a private PipeWire; this is the real one.)*
- [ ] **Behind your windows.** Appearance → *Behind your windows*: the
      characters go under every window and stay there when one is
      clicked; edit mode brings them to the front while it lasts.
      *(The rig's sway floats X11 windows; GNOME is the real test.)*

## 3. Your day

- [ ] **Desktop notifications.** Scene tab → *Also as desktop
      notifications*; start the focus timer (Scene tab, or
      `Ctrl+K` → *Start the focus timer*): a GNOME banner from
      *animaEngine*, its icon, the text the character says. With a
      reminder that names no character and contains `{name}`, the banner
      carries the speaker's name, not "animaEngine".
- [ ] **Focus holds reminders — notifications too.** A reminder every
      minute, notifications on, focus timer running: no banner and no
      bubble until the break; then it comes.
- [ ] **Snoozed, it goes to the banner at once.** Tray → *Snooze 15
      minutes*: the characters leave; a reminder due now is a banner,
      not a wait. Tray → *Wake up now* brings them back.
- [ ] **A sound with reminders.** Scene tab, Focus section: the
      speaker's own sound, then a file: each plays on a turn of the
      timer, on the speakers.
- [ ] **Away, via Mutter.** Appearance → *Pause when I'm away* → 1
      minute. The log said at start `Idle time from: Mutter's
      IdleMonitor (D-Bus)`. Leave mouse and keyboard for two minutes:
      Zzz…, still. Move the mouse: they wake, and the one in front says
      hello (*Say hello when I come back* is on by default).
- [ ] **On battery.** Appearance → *Pause on battery*; unplug the
      charger: they hold still; plug it back: they go on. (UPower, on
      the system bus.)
- [ ] **Bedtime and moods.** Bedtime from five minutes from now, Moods
      on: within those five minutes each yawns once; at bedtime they
      doze.

## 4. Full screen, and apps you choose

- [ ] **Full screen.** An X11 app full screen — `xterm -fullscreen`,
      or Firefox started with `MOZ_ENABLE_WAYLAND=0` and F11: the
      characters hide, and come back when it leaves full screen.
      Native Wayland apps (most of GNOME's) are not seen on GNOME:
      that is expected, and the setting's hint says so.
- [ ] **Apps you choose.** Appearance → *Also hide for these apps* →
      *Add an open app…*: an X11 app from the list; with it in front,
      full screen or not, they hide.

## 5. Shortcuts

- [ ] **Global shortcuts through the portal.** Keybindings: give *Start
      or stop the focus timer* and *Snooze 15 minutes or wake up* a
      key each. GNOME asks once to allow them. From another app, each
      key works, and GNOME's own settings list them for the app (where
      depends on the GNOME version).

- [ ] **Which mechanism.** The log says `Hotkey strategy: …`. With
      `X11 XGrabKey`: `Ctrl+Shift+A` from an X11 app in front (`xterm`)
      opens edit mode; switch the keyboard layout (`Super+Space`) and it
      still does; held down, it toggles once.
- [ ] **Their names, in your language.** With the app in Romanian,
      GNOME's list of the app's shortcuts (where item 1 of this section
      found it) names them in Romanian, after a restart of the app.
- [ ] **Without a mouse.** `Ctrl+Shift+A`; `Tab` to a character's name
      in the Scene tab, `Enter`; `Esc` (still in edit mode);
      `Shift+F10` opens the right-click menu with its first item
      focused — `Tab` to *Save as GIF…*, `Enter`: GNOME's save dialog.
      `Ctrl+A`, `Shift+F10`, *Align top edges*: they line up. With Orca
      on, each menu item is read as the focus reaches it.

## 6. Files, through GNOME's own dialogs

- [ ] **Save as GIF.** Right-click a character → *Save as GIF…*: GNOME's
      save dialog; the file opens in an image viewer and loops.
- [ ] **Share a scene, a character.** Scene tab → a saved scene's
      Share button; right-click → *Share character…*: each writes one
      file where you choose.
- [ ] **Import, and Add file….** Scene tab → *Import…* opens the
      `.animascene`; *Add file…* adds an image as a character.
- [ ] **Drag from Nautilus.** A PNG, an `.animascene` and an
      `.animachar` dragged from Nautilus onto the overlay in edit mode:
      a character, a saved scene, a character joining the scene.
- [ ] **Back up and restore.** Appearance → *Back up…*, then change a
      setting and *Restore…* the file: the setting comes back at once,
      the scenes are there, and `config.toml.before-restore` sits next
      to `config.toml`.
- [ ] **Paste a picture.** Copy an image in Firefox or Image Viewer;
      `Ctrl+V` in edit mode: it becomes a character where the pointer
      is.

- [ ] **A config from a newer version.** With the app running, add to
      `~/.config/animaengine/config.toml` a character a later version
      might write, and a setting it does not know:

      ```toml
      [[characters]]
      id = "holo"
      name = "Holo"
      asset_type = "hologram"
      asset_path = "h.holo"
      x = 0.0
      y = 0.0
      ```

      and `future_glow = 1` under `[global]`. Saved, the edit applies:
      the other characters stay, edit mode shows a banner that part of
      the file could not be read. Press `S`: both are still in the file.
      Take them out again.
- [ ] **In another language.** Appearance → Romanian: the presets in
      the Scene tab, the tray icon's tooltip, and the toast for a `.txt`
      dragged onto the overlay all read in Romanian.

## 7. The tray, login, Orca

- [ ] **The tray** (Ubuntu's AppIndicator extension): *Snooze 15
      minutes / 1 hour / until morning*, then *Snoozing until …* and
      *Wake up now*; *Start the focus timer* / *Stop the focus timer*,
      with *Focusing until …* while it runs; *Next scene* goes to the
      next saved scene; *Mute sounds* / *Unmute sounds*. All of it in
      the app's language, and in another a few seconds after Appearance
      switches it.
- [ ] **Start at login.** Appearance → *Start at login*: the entry is
      in `~/.config/autostart/com.animaengine.Anima.desktop` and in
      GNOME's startup list; log out and in: it starts. Turn it off
      again if you don't want it.
- [ ] **Orca hears them.** `Super+Alt+S` starts Orca. A tap, a
      reminder, a greeting, the focus timer: each line is read out,
      after what Orca was saying.

## 8. A working day

- [ ] **Still, it rests.** Hide the overlay (`Ctrl+Shift+H`), then count
      its wake-ups over ten seconds:

      ```bash
      pid=$(pidof anima_engine); n() { cat /proc/$pid/task/*/status | awk '/ctxt_switches/ {s+=$2} END {print s}'; }
      a=$(n); sleep 10; echo $(( ($(n) - a) / 10 )) wake-ups a second
      ```

      Low — a few dozen at most, most of them the GPU driver's — and
      `top -p $pid` near 0%. Show it again: the characters carry on.
      In the test rig a hidden overlay drew as often as a shown one
      before 1.5 (R58 in docs/runtime-findings.md).
- [ ] Leave it running for a day with your own scene. In the evening:
      `ps -o rss=,nlwp= -p $(pidof anima_engine)` — about what it was
      after the first hour. The rig's soak (docs/soak-testing.md) was
      flat; this is the real desktop's turn.

When you are done: `cp -a ~/animaengine-config-before-1.5/. ~/.config/animaengine/`
puts your settings back, if the tests left them somewhere you don't
want.

## Log

| Date | Item | Result | Note |
|---|---|---|---|
| | | | |
