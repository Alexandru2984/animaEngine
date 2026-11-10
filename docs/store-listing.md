# Store listing

Text and images for a store page — itch.io, Steam, a page of your own —
ready to paste. The images are in [`site/media/`](../site/media/). What
is still to decide before listing anywhere is in
[product-readiness.md](product-readiness.md): the price, the H.264
question, and signing for Windows.

The product page in `site/` is in the app's ten languages. Its words
are in `site-src/strings/<language>.toml`, its pages in `site-src/`;
change those and run `scripts/site-pages.py`, never the pages in
`site/` themselves (CI checks they match). Like the app's, nine of the
ten were translated without a native speaker.

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
Shimeji packs can be imported. A character can also move by a small
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
| `site/media/demo.gif` | 800×500 | Animated preview, a store's GIF slot or the page |
| `site/media/store/itch-cover-630x500.png` | 630×500 | itch.io cover |
| `site/media/store/steam-header-920x430.png` | 920×430 | Steam header capsule |
| `site/media/store/steam-small-462x174.png` | 462×174 | Steam small capsule |
| `site/media/store/steam-main-1232x706.png` | 1232×706 | Steam main capsule |
| `site/media/store/steam-vertical-748x896.png` | 748×896 | Steam vertical capsule |
| `site/media/store/steam-library-600x900.png` | 600×900 | Steam library capsule |
| `site/media/store/steam-library-hero-3840x1240.png` | 3840×1240 | Steam library hero (no text, as it asks) |
| `site/media/store/steam-logo-1280x720.png` | 1280×720 | Steam library logo, transparent |
| `site/media/store/banner-1600x500.png` | 1600×500 | A banner: the README, social posts |

The capsules and banners are drawn by `scripts/store-art.py` from the
built-in characters at four times their size
(`cargo run --release --example store_pictures -- <folder>`), with the
title in Ubuntu Bold. Steam's sizes are those of its guidelines in 2026:
check them before uploading, and run the script again for others.

## Trailer

| File | Size | Use |
|---|---|---|
| `build/trailer/trailer-1080p.mp4` | 1920×1080, 33 s, H.264 | Steam's trailer upload |
| `build/trailer/trailer-1080p.webm` | 1920×1080, 33 s, VP9 | itch.io, a download |
| `site/media/trailer-720p.webm` | 1280×720, 33 s, VP9, 1 MB | The product page |
| `site/media/trailer-poster.jpg` | 1280×720 | Its poster: the title card |

No sound: add music if a store asks for an audio track. A title card,
four scenes with a caption each, an end card, cross-faded:

1. "They live on your desktop" — walkers on the floor, the ghost
   floating, the heart and the star idle.
2. "Tap one, and it answers" — a tap on the cat (it hops, says "Hi
   there!"), then on the heart ("Hello!").
3. "Choose who lives there" — the gear clicked, the welcome; the bunny,
   the frog, the mushroom and the cloud put on; Continue.
4. "Pick them up. Let them fall." — the cat picked up and held (it
   swings and laughs), let go: it falls and lands.

The scenes are recorded on the test rig (a headless sway at 1920×1080,
native Wayland, the app's debug build) with `wf-recorder -r 30`, the
pointer moved by a virtual pointer; the app runs from a folder whose
`assets/demo` holds the built-in characters drawn at 192 px, so they
are sharp at that size. `scripts/trailer-compose.py <scenes> <pictures>`
makes the cards and captions and cuts the files. `build/` is not
committed: the 1080p files are made again with the script.

## Site assets

For animaengine.eu: `scripts/site-assets.py` writes `build/site-assets/` —
each built-in character standing and asleep at 512 px, animated WebP and
CSS sprite sheets of it standing, walking, falling and held (from
`examples/site_pictures.rs`, the app's own drawing at any size), the
favicons, Apple touch and maskable icons with a `site.webmanifest`, a
1200×630 link preview in each of the ten languages, and a press kit (logo,
wordmarks, the site's colours, each language's texts) zipped as well. Its
`README.md` has the HTML and CSS to use them. Every file is under 25 MB, a
Cloudflare Pages file's limit; the script says so or stops.

The clips (`desktop`, `tap`, `pickup`, `sleep` — WebM at 1080p and 720p,
MP4, a poster) and the screenshots beside them are recorded on the test
rig like the trailer's scenes, at 1920×1080 over the store art's sky, with
no captions and no words in the bubbles ("Mrrp!", "♥", "Zzz…"), as the
site has ten languages. They are kept when the script runs again.

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
