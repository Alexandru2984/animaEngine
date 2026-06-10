# Recording the README demo GIF

One-time guide for producing `docs/media/demo.gif` — the 10–15 s clip
at the top of the README. GitHub renders GIFs inline and autoplays
them; videos require a click, so GIF wins for the README slot even at
a worse size/quality ratio.

## What to record (10–15 s, in this order)

1. Desktop visible, overlay running in pass-through with one or two
   characters already animating.
2. Drag a GIF from Files onto the overlay → character appears where
   dropped.
3. Click ⚙ → edit mode; click the new character, drag it somewhere,
   pull the scale slider.
4. Set behavior to *Walk around* — let it walk ~2 s.
5. `Ctrl+K` → command palette → apply a preset (Append).
6. Click ⚙ again → pass-through; characters keep animating over the
   desktop.

Keep the window count low and the wallpaper calm — the overlay is the
star, not the desktop.

## Recording on Ubuntu GNOME (Wayland)

Built-in recorder: `Ctrl+Shift+Alt+R` (records the whole screen to
`~/Videos/Screencasts/*.webm`). For a region-only capture install
Kooha (`flatpak install flathub io.github.seadve.Kooha`) and pick the
monitor/region there.

Record at the monitor's native resolution; downscale happens in the
conversion step.

## Converting to an optimized GIF

```bash
# 1. Trim to the good part (adjust -ss/-t):
ffmpeg -i input.webm -ss 2 -t 14 -c copy trimmed.webm

# 2. Two-pass palette GIF — 800 px wide, 18 fps, dithered:
ffmpeg -i trimmed.webm -vf "fps=18,scale=800:-1:flags=lanczos,palettegen" palette.png
ffmpeg -i trimmed.webm -i palette.png \
  -lavfi "fps=18,scale=800:-1:flags=lanczos[x];[x][1:v]paletteuse=dither=bayer:bayer_scale=4" \
  docs/media/demo.gif
```

Target: **under 10 MB** (GitHub truncates README images above ~10 MB
on slow connections; under 5 MB is ideal). Knobs, in order of
effectiveness: shorter clip, lower fps (15 is still smooth), narrower
scale (720 px).

## Wiring it into the README

Once `docs/media/demo.gif` exists, add directly under the README
title block:

```markdown
![animaEngine demo — drag-drop a GIF, drive behaviors, command palette](docs/media/demo.gif)
```

Then commit both together:

```
git add docs/media/demo.gif README.md
git commit -m "docs(readme): add demo gif"
```
