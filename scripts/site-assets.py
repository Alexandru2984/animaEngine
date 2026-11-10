#!/usr/bin/env python3
"""Pictures for animaengine.eu, made from the app's own drawing code and
the site's strings, into build/site-assets/ (not kept in git; run again to
make them):

  characters/  each built-in character standing at 512 px and asleep;
               256 px animated WebP of it standing, walking, falling and
               held (10 frames a second, looping); the same frames in a
               row as a sprite sheet for CSS `steps(12)`; and the ten, and
               the five of the first run, side by side
  icons/       favicon.ico and .svg, PNGs 16–1024, the Apple touch icon,
               a maskable icon and a site.webmanifest to start from
  social/      a 1200×630 preview for each of the site's ten languages
  press/       the logo and a wordmark for light and dark backgrounds, the
               site's colours, each language's texts, a README; and the
               same as press-kit.zip

Every file is checked to be under 25 MB, a Cloudflare Pages file's limit.

    scripts/site-assets.py [out-dir]
"""
import html
import io
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
import zipfile
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

REPO = Path(__file__).resolve().parent.parent
OUT = Path(sys.argv[1]) if len(sys.argv) > 1 else REPO / "build" / "site-assets"
CHARACTERS = ["ghost", "slime", "heart", "star", "cat", "bunny", "frog", "robot", "cloud", "mushroom"]
FIRST_RUN = ["ghost", "slime", "heart", "star", "cat"]
LOOPS = ["idle", "walk", "fall", "drag"]
FPS = 10
LIMIT = 25 * 1000 * 1000
LANGS = ["en", "ro", "de", "fr", "es", "it", "pt-BR", "nl", "pl", "ja"]
FONT_DIR = Path("/usr/share/fonts")
LATIN_BOLD = FONT_DIR / "truetype/noto/NotoSans-Bold.ttf"
LATIN = FONT_DIR / "truetype/noto/NotoSans-Regular.ttf"
CJK_BOLD = FONT_DIR / "opentype/noto/NotoSansCJK-Bold.ttc"
CJK = FONT_DIR / "opentype/noto/NotoSansCJK-Regular.ttc"


def run(*cmd, **kw):
    subprocess.run(cmd, check=True, **kw)


def frames(root, name, state):
    folder = root / name / state
    return [Image.open(p).convert("RGBA") for p in sorted(folder.glob("*.png"))]


def save_png(image, path):
    path.parent.mkdir(parents=True, exist_ok=True)
    image.save(path, optimize=True)


# ── characters ──────────────────────────────────────────────────────────

def characters(big, small):
    out = OUT / "characters"
    for name in CHARACTERS:
        save_png(frames(big, name, "idle")[0], out / f"{name}.png")
        save_png(frames(big, name, "sleep")[0], out / f"{name}-sleep.png")
        for state in LOOPS:
            loop = frames(small, name, state)
            loop[0].save(out / f"{name}-{state}.webp", save_all=True, append_images=loop[1:],
                         duration=1000 // FPS, loop=0, lossless=True, method=4)
            w, h = loop[0].size
            sheet = Image.new("RGBA", (w * len(loop), h))
            for i, frame in enumerate(loop):
                sheet.alpha_composite(frame, (i * w, 0))
            save_png(sheet, out / f"{name}-{state}-sheet.png")
    for label, names in (("all", CHARACTERS), ("first-run", FIRST_RUN)):
        stills = [frames(big, n, "idle")[0] for n in names]
        w, h = stills[0].size
        group = Image.new("RGBA", (w * len(stills), h))
        for i, still in enumerate(stills):
            group.alpha_composite(still, (i * w, 0))
        save_png(group, out / f"group-{label}.png")


# ── icons ───────────────────────────────────────────────────────────────

ICON_SVG = REPO / "data" / "anima-engine.svg"


def icon_variant(full_bleed, ghost_scale):
    """The icon's SVG with its badge's corners square and what is drawn on
    it scaled about the centre: for a maskable icon, and for iOS, which
    rounds the corners itself and fills what is transparent with black."""
    svg = ICON_SVG.read_text()
    if full_bleed:
        svg = svg.replace('rx="56" ry="56"', 'rx="0" ry="0"')
    marks = [m.end() for m in re.finditer(r'<rect width="256" height="256"[^>]*/>', svg)]
    if ghost_scale != 1 and marks:
        cut, end = marks[-1], svg.rindex("</svg>")
        svg = (svg[:cut] + f'<g transform="translate(128 128) scale({ghost_scale}) translate(-128 -128)">'
               + svg[cut:end] + "</g>" + svg[end:])
    return svg


def render_svg(svg_text, size):
    with tempfile.NamedTemporaryFile("w", suffix=".svg", delete=False) as f:
        f.write(svg_text)
    try:
        png = subprocess.run(["rsvg-convert", "-w", str(size), "-h", str(size), f.name],
                             check=True, capture_output=True).stdout
    finally:
        os.unlink(f.name)
    return Image.open(io.BytesIO(png)).convert("RGBA")


def icons():
    out = OUT / "icons"
    out.mkdir(parents=True, exist_ok=True)
    shutil.copy(ICON_SVG, out / "icon.svg")
    plain = ICON_SVG.read_text()
    for size in (16, 32, 48, 64, 96, 128, 192, 256, 512, 1024):
        save_png(render_svg(plain, size), out / f"icon-{size}.png")
    render_svg(plain, 256).save(out / "favicon.ico", sizes=[(16, 16), (32, 32), (48, 48)])
    save_png(render_svg(icon_variant(True, 1), 180), out / "apple-touch-icon.png")
    # Maskable: the ghost inside the middle 80 % a launcher may crop to.
    save_png(render_svg(icon_variant(True, 0.72), 512), out / "icon-maskable-512.png")
    manifest = {
        "name": "animaEngine",
        "short_name": "animaEngine",
        "icons": [
            {"src": "/icons/icon-192.png", "sizes": "192x192", "type": "image/png"},
            {"src": "/icons/icon-512.png", "sizes": "512x512", "type": "image/png"},
            {"src": "/icons/icon-maskable-512.png", "sizes": "512x512", "type": "image/png",
             "purpose": "maskable"},
        ],
        "theme_color": "#6b4fd8",
        "background_color": "#15131d",
        "display": "browser",
    }
    (out / "site.webmanifest").write_text(json.dumps(manifest, indent=2) + "\n")


# ── strings and colours ─────────────────────────────────────────────────

def strings(lang):
    return tomllib.loads((REPO / "site-src" / "strings" / f"{lang}.toml").read_text())


def plain(text):
    return html.unescape(re.sub(r"<[^>]+>", "", text)).strip()


def colours():
    css = (REPO / "site" / "style.css").read_text()
    def tokens(block):
        return dict(re.findall(r"--([a-z-]+):\s*(#[0-9a-fA-F]{3,8})", block))
    light = tokens(css.split("@media")[0])
    dark_block = re.search(r"prefers-color-scheme:\s*dark\)\s*\{\s*:root\s*\{([^}]*)\}", css)
    return {"light": light, "dark": tokens(dark_block.group(1)) if dark_block else {}}


# ── social previews ─────────────────────────────────────────────────────

def font(lang, bold, size):
    if lang == "ja":
        return ImageFont.truetype(str(CJK_BOLD if bold else CJK), size, index=0)
    return ImageFont.truetype(str(LATIN_BOLD if bold else LATIN), size)


def units_of(text, lang):
    """What a line may break between: words; in Japanese, which has no
    spaces, runs that end where hiragana gives way to another script —
    デスクトップに|住む|アニメーションキャラクター — so a word is not cut."""
    if lang != "ja":
        return text.split()
    def hiragana(c):
        return "\u3040" <= c <= "\u309f"
    units, run = [], ""
    for c in text:
        if run and hiragana(run[-1]) and not hiragana(c):
            units.append(run)
            run = ""
        run += c
    return units + ([run] if run else [])


def wrap(draw, text, face, width, lang):
    """Lines of `text` that fit `width`, broken between `units_of` it."""
    joiner = "" if lang == "ja" else " "
    lines, line = [], ""
    for unit in units_of(text, lang):
        trial = (line + joiner + unit) if line else unit
        if draw.textlength(trial, font=face) <= width or not line:
            line = trial
        else:
            lines.append(line)
            line = unit
    if line:
        lines.append(line)
    return lines


def fitted(draw, text, lang, width, size, max_lines):
    """The bold face and lines that set `text` in `width`, in at most
    `max_lines`: `size`, or smaller until every line fits."""
    while True:
        face = font(lang, True, size)
        lines = wrap(draw, text, face, width, lang)
        if size <= 24 or (len(lines) <= max_lines
                          and all(draw.textlength(l, font=face) <= width for l in lines)):
            return face, lines
        size -= 2


def social(big, palette):
    out = OUT / "social"
    out.mkdir(parents=True, exist_ok=True)
    dark = palette["dark"]
    W, H = 1200, 630
    base = Image.new("RGBA", (W, H), dark.get("bg", "#15131d"))
    glow = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    ImageDraw.Draw(glow).ellipse((600, 120, 1260, 760), fill=(154, 131, 255, 90))
    base.alpha_composite(glow.filter(ImageFilter.GaussianBlur(90)))
    # The five of the first run: those that stand on one ground line, the
    # cat in front; the heart and the star float above them.
    ground = H - 40
    cast = [("heart", 170, 790, 150), ("star", 170, 1080, 120), ("ghost", 220, 760, None),
            ("slime", 190, 1100, None), ("cat", 290, 935, None)]
    for name, size, cx, top in cast:
        still = frames(big, name, "idle")[0].resize((size, size), Image.LANCZOS)
        feet = still.getbbox()[3]
        base.alpha_composite(still, (cx - size // 2, top if top is not None else ground - feet))
    icon = render_svg(ICON_SVG.read_text(), 84)
    for lang in LANGS:
        s = strings(lang)
        im = base.copy()
        draw = ImageDraw.Draw(im)
        im.alpha_composite(icon, (64, 64))
        draw.text((166, 70), "animaEngine", font=font("en", True, 54), fill=dark.get("text", "#ece9f6"))
        face, lines = fitted(draw, plain(s["h1"]), lang, 560, 52, 4)
        y = 210
        for line in lines:
            draw.text((64, y), line, font=face, fill="#ffffff")
            y += int(face.size * 1.22)
        draw.text((64, H - 92), "animaengine.eu", font=font("en", True, 32), fill=dark.get("accent", "#9a83ff"))
        save_png(im.convert("RGB"), out / f"og-{lang}.png")
    shutil.copy(out / "og-en.png", out / "og.png")


# ── press kit ───────────────────────────────────────────────────────────

def wordmark(text_colour, h=400):
    """The icon `h` tall and the name beside it, as wide as they are."""
    face = font("en", True, int(h * 0.5))
    box = ImageDraw.Draw(Image.new("RGBA", (1, 1))).textbbox((0, 0), "animaEngine", font=face)
    gap = h // 6
    im = Image.new("RGBA", (h + gap + box[2] + h // 10, h), (0, 0, 0, 0))
    im.alpha_composite(render_svg(ICON_SVG.read_text(), h), (0, 0))
    ImageDraw.Draw(im).text((h + gap, (h - (box[3] - box[1])) // 2 - box[1]), "animaEngine",
                            font=face, fill=text_colour)
    return im.crop(im.getbbox())


def press(palette):
    out = OUT / "press"
    (out / "logo").mkdir(parents=True, exist_ok=True)
    shutil.copy(ICON_SVG, out / "logo" / "icon.svg")
    save_png(render_svg(ICON_SVG.read_text(), 1024), out / "logo" / "icon-1024.png")
    save_png(wordmark(palette["light"].get("text", "#1d1a29")), out / "logo" / "wordmark-on-light.png")
    save_png(wordmark(palette["dark"].get("text", "#ece9f6")), out / "logo" / "wordmark-on-dark.png")
    (out / "colours.json").write_text(json.dumps(palette, indent=2) + "\n")
    (out / "texts").mkdir(exist_ok=True)
    for lang in LANGS:
        s = strings(lang)
        lines = [f"# animaEngine — {plain(s['h1'])}", "", plain(s["description"]), "", plain(s["lead"]), "",
                 f"## {plain(s['features'])}", ""]
        for i in range(1, 9):
            if f"card{i}_h" in s:
                lines.append(f"- **{plain(s[f'card{i}_h'])}** — {plain(s[f'card{i}'])}")
        (out / "texts" / f"{lang}.md").write_text("\n".join(lines) + "\n")
    (out / "README.md").write_text("""# animaEngine press kit

- `logo/icon.svg`, `logo/icon-1024.png` — the app's icon.
- `logo/wordmark-on-light.png`, `logo/wordmark-on-dark.png` — the icon and
  the name, for light and for dark backgrounds.
- `colours.json` — the website's colours, light and dark.
- `texts/<language>.md` — the one-line description, the short one and the
  features, in the website's ten languages.
- `../characters/` — the built-in characters, standing and asleep, and
  animated.
- `../social/` — previews for links, 1200×630, one for each language.

Website: https://animaengine.eu — source: https://github.com/Alexandru2984/animaEngine
""")
    with zipfile.ZipFile(OUT / "press-kit.zip", "w", zipfile.ZIP_DEFLATED) as z:
        for path in sorted(out.rglob("*")):
            if path.is_file():
                z.write(path, Path("press") / path.relative_to(out))
        for name in CHARACTERS:
            for suffix in ("", "-sleep"):
                p = OUT / "characters" / f"{name}{suffix}.png"
                z.write(p, Path("characters") / p.name)
        for p in sorted((OUT / "social").glob("og-*.png")):
            z.write(p, Path("social") / p.name)


# ── index and the size limit ────────────────────────────────────────────

def index():
    rows, too_big, total = [], [], 0
    for path in sorted(OUT.rglob("*")):
        if path.is_file() and path.name not in ("INDEX.md", "README.md"):
            size = path.stat().st_size
            total += size
            rows.append(f"| `{path.relative_to(OUT)}` | {size / 1000:,.1f} kB |")
            if size >= LIMIT:
                too_big.append(path)
    (OUT / "INDEX.md").write_text("# Site assets\n\nMade by `scripts/site-assets.py`.\n\n"
                                  "| File | Size |\n|---|---|\n" + "\n".join(rows)
                                  + f"\n\n{len(rows)} files, {total / 1e6:,.1f} MB in all.\n")
    return len(rows), total, too_big


README = """# animaEngine — site assets

Made by `scripts/site-assets.py` (characters, icons, social, press) and
recorded on the test rig (clips, screenshots). Every file is under 25 MB.
`INDEX.md` lists them with their sizes.

## Clips

WebM first, MP4 for the browsers that want it; the poster shows until it
plays. No sound, so they may autoplay:

```html
<video autoplay muted loop playsinline poster="/clips/tap-poster.jpg" width="1280" height="720">
  <source src="/clips/tap-720p.webm" type="video/webm">
  <source src="/clips/tap.mp4" type="video/mp4">
</video>
```

`desktop` — they walk and float; `tap` — the cat and the heart tapped;
`pickup` — the cat held in edit mode and let fall; `sleep` — a minute away,
they doze, and wake.

## A character walking across the page

`characters/<name>-walk-sheet.png` is the walk's 12 frames in a row, 256 px
each; shown at 128 px for a sharp HiDPI picture:

```html
<div class="walker" aria-hidden="true"></div>
```
```css
.walker {
  width: 128px; height: 128px;
  background: url(/characters/cat-walk-sheet.png) 0 0 / 1536px 128px;
  animation: walk-frames 1.2s steps(12) infinite, walk-across 18s linear infinite;
  position: absolute; bottom: 0;
}
@keyframes walk-frames { to { background-position: -1536px 0; } }
@keyframes walk-across { from { left: -128px; } to { left: 100%; } }
@media (prefers-reduced-motion: reduce) { .walker { animation: none; } }
```

Or, simpler, `<img src="/characters/cat-walk.webp" width="128" height="128" alt="">`
— the animated WebP plays by itself.

## Previews and icons

```html
<meta property="og:image" content="https://animaengine.eu/social/og-en.png">
<meta property="og:image:width" content="1200">
<meta property="og:image:height" content="630">
<meta name="twitter:card" content="summary_large_image">
<link rel="icon" href="/icons/favicon.ico" sizes="48x48">
<link rel="icon" href="/icons/icon.svg" type="image/svg+xml">
<link rel="apple-touch-icon" href="/icons/apple-touch-icon.png">
<link rel="manifest" href="/icons/site.webmanifest">
```

`social/og-<lang>.png` for each language's pages (`og-pt-BR.png` and the
rest), `og.png` the English one.
"""


def main():
    # What this script makes, made again; the clips and screenshots, which
    # are recorded, are kept.
    OUT.mkdir(parents=True, exist_ok=True)
    for part in ("characters", "icons", "social", "press"):
        shutil.rmtree(OUT / part, ignore_errors=True)
    (OUT / "press-kit.zip").unlink(missing_ok=True)
    (OUT / "README.md").write_text(README)
    run("cargo", "build", "--release", "--locked", "--example", "site_pictures", cwd=REPO)
    example = REPO / "target" / "release" / "examples" / "site_pictures"
    with tempfile.TemporaryDirectory() as tmp:
        big, small = Path(tmp) / "512", Path(tmp) / "256"
        run(str(example), str(big), "4")
        run(str(example), str(small), "2")
        palette = colours()
        characters(big, small)
        icons()
        social(big, palette)
        press(palette)
    count, total, too_big = index()
    print(f"{count} files, {total / 1e6:.1f} MB in {OUT}")
    if too_big:
        sys.exit("over 25 MB: " + ", ".join(str(p) for p in too_big))


if __name__ == "__main__":
    main()
