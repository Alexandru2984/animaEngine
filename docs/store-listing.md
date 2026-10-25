# Store listing

Text and images for a store page — itch.io, Steam, a page of your own —
ready to paste. The images are in [`site/media/`](../site/media/). What
is still to decide before listing anywhere is in
[product-readiness.md](product-readiness.md): the price, the H.264
question, and signing for Windows.

## Name and lines

- **Name:** animaEngine
- **Tagline** (≤ 60 characters): Animated characters that live on your desktop
- **Short description** (Steam's limit is 300 characters):

  > Little animated characters that live on your desktop — over your
  > windows or behind them. They walk, bounce, follow each other and
  > play tag, say hello when tapped, and remind you to take a break.
  > Clicks go straight through to your work. Bring your own GIFs and
  > pictures. No network, no tracking.

## Description

animaEngine puts animated characters on your desktop and lets them live
there. They walk, wander, bounce, follow your cursor or each other, and
play tag. Tap one and it hops, says one of
its lines and plays its sound. Two who meet say hello.

**Make anything a character.** Drop a PNG, GIF, WebP or short MP4 on the
screen — or paste a picture you copied anywhere — and it comes alive.
Shimeji packs import as they are. A character can also move by a small
script you write.

**A companion for your day.** Characters remind you to stand up or drink
water, keep a focus timer, doze when you are away and greet you when you
come back. They go to sleep at bedtime, and can be livelier in the
morning.

**Out of your way.** Clicks pass straight through them to your windows.
They step aside for full-screen games, films and presentations, can live
behind your windows instead of over them, and hold still on battery if
you like. When nothing moves they cost next to nothing.

**Yours to arrange.** Edit mode gives every character a panel: size,
opacity, speed, behaviour, what it says and when it is on screen. Select
several, group them, line them up, undo anything. Save scenes, switch
between them by the time of day, and share a scene or a character as a
single file.

**For everyone.** Ten languages, screen readers, everything from the
keyboard, high-contrast themes, reduced motion.

**Private.** animaEngine makes no network connection at all. Nothing
about you or your desktop leaves your computer.

Free and open source under the MIT licence.

## Features, as a list

- Characters from PNG, GIF, WebP, JPEG, PNG sequences, sprite sheets and
  short MP4 (H.264) videos; Shimeji packs imported
- Behaviours: walk, wander, bounce, follow the cursor, follow another
  character, play tag, gravity and throwing, scripted (Rhai)
- Tap a character: a hop, a line of its own, a sound
- Speech bubbles, reminders at intervals or set times, a focus timer,
  bedtime, snooze, a welcome back
- Click-through overlay, over or behind your windows, on every monitor
- Steps aside for full-screen apps and apps you choose; pauses when you
  are away or on battery; 30 or 60 frames a second
- Named scenes with pictures, scheduled by the time of day; share scenes
  and characters as single files; back up everything in one file
- Undo and redo, multiple selection, groups, align and distribute,
  snapping
- Ten languages, screen readers, full keyboard control, high-contrast
  themes
- No network connection, no telemetry

## Images

| File | Size | Use |
|---|---|---|
| `site/media/overlay.png` | 1920×1080 | Screenshot: characters on the desktop, the cat speaking |
| `site/media/edit-mode.png` | 1920×1080 | Screenshot: edit mode, a character selected |
| `site/media/command-palette.png` | 1920×1080 | Screenshot: the command palette and presets |
| `site/media/appearance.png` | 1920×1080 | Screenshot: the Appearance settings |
| `site/media/cover-630x500.png` | 630×500 | itch.io cover |
| `site/media/demo.gif` | 800×500 | Animated preview, a store's GIF slot or the page |

Steam also asks for capsule, header and library art, in the sizes its
current graphical-asset guidelines give. They are art, not screenshots,
and are not made yet: they would come with the characters a store build
ships.

## System requirements

| | Minimum |
|---|---|
| Linux | x86_64, glibc 2.35 or newer (Ubuntu 22.04, Debian 12, Fedora, Arch and later); X11, or Wayland through XWayland (native Wayland on wlroots compositors) |
| Windows | Windows 10 or 11, 64-bit — new in 1.5, run under Wine so far |
| Graphics | Vulkan, or OpenGL 3.3 |
| Memory | about 100 MB with a few characters |
| Disk | about 25 MB |

## Store details

- **Category / tags:** Utilities, Desktop customisation, Desktop pet,
  Animation, Casual, Relaxing
- **Content:** no violence, no user-to-user communication, no
  purchases inside the app
- **Privacy policy:** `site/privacy.html`, once the site is published
  (the Website workflow; see [`site/`](../site/))
- **Support:** [SUPPORT.md](../SUPPORT.md), the [FAQ](faq.md)
- **Price:** your decision
