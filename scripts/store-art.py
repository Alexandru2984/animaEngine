#!/usr/bin/env python3
"""Store and site art: the capsules, banners and covers a store page asks
for, composed from the built-in characters drawn large.

    cargo run --release --example store_pictures -- /tmp/pictures
    scripts/store-art.py /tmp/pictures [site/media/store]

The characters come from examples/store_pictures.rs (512 px each); the
background — a dusk sky, soft lights, two hills — and the titles are drawn
here. The title's font is Ubuntu Bold where it is installed, else DejaVu
Sans Bold. Needs Pillow. Steam's sizes are those of its graphical-asset
guidelines in 2026; check them before uploading.
"""
import math
import random
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

PICTURES = Path(sys.argv[1])
OUT = Path(sys.argv[2]) if len(sys.argv) > 2 else Path("site/media/store")
FONTS = [
    "/usr/share/fonts/truetype/ubuntu/Ubuntu-B.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
]
TITLE = "animaEngine"
TAGLINE = "Animated characters that live on your desktop"


def font(size):
    for f in FONTS:
        if Path(f).exists():
            return ImageFont.truetype(f, size)
    sys.exit("no title font: install fonts-ubuntu or fonts-dejavu")


def sky(w, h, seed=7):
    """A dusk gradient with soft lights and two hills."""
    top, bottom = (36, 26, 78), (112, 82, 196)
    img = Image.new("RGB", (w, h))
    d = ImageDraw.Draw(img)
    for y in range(h):
        t = y / max(h - 1, 1)
        d.line([(0, y), (w, y)], fill=tuple(int(a + (b - a) * t) for a, b in zip(top, bottom)))
    lights = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    ld = ImageDraw.Draw(lights)
    rnd = random.Random(seed)
    for _ in range(max(8, w * h // 60000)):
        r = rnd.uniform(0.04, 0.12) * min(w, h) + 10
        x, y = rnd.uniform(0, w), rnd.uniform(0, h * 0.75)
        colour = rnd.choice([(255, 180, 220), (170, 160, 255), (255, 220, 170), (150, 200, 255)])
        ld.ellipse([x - r, y - r, x + r, y + r], fill=colour + (rnd.randint(28, 60),))
    lights = lights.filter(ImageFilter.GaussianBlur(min(w, h) / 30))
    img = img.convert("RGBA")
    img.alpha_composite(lights)
    hills = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    hd = ImageDraw.Draw(hills)
    for k, (colour, base, amp) in enumerate([((60, 42, 120, 255), 0.80, 0.07), ((40, 28, 86, 255), 0.88, 0.05)]):
        pts = [(0, h)]
        for i in range(0, w + 21, 20):
            y = h * base - h * amp * math.sin(i / w * math.pi * (1.6 + k) + k)
            pts.append((i, y))
        pts.append((w, h))
        hd.polygon(pts, fill=colour)
    img.alpha_composite(hills)
    return img


def character(name, frame, size):
    pic = Image.open(PICTURES / f"{name}-{frame}.png").convert("RGBA")
    return pic.resize((size, size), Image.LANCZOS)


def put(img, name, frame, size, cx, ground):
    """A character `size` across, standing on `ground` at `cx`."""
    pic = character(name, frame, size)
    img.alpha_composite(pic, (int(cx - size / 2), int(ground - size * 0.92)))


def title(img, text, size, cx, cy, shadow=True, colour=(255, 255, 255, 255)):
    f = font(size)
    d = ImageDraw.Draw(img)
    box = d.textbbox((0, 0), text, font=f)
    x, y = cx - (box[2] - box[0]) / 2 - box[0], cy - (box[3] - box[1]) / 2 - box[1]
    if shadow:
        glow = Image.new("RGBA", img.size, (0, 0, 0, 0))
        ImageDraw.Draw(glow).text((x, y + size * 0.05), text, font=f, fill=(20, 10, 50, 170))
        img.alpha_composite(glow.filter(ImageFilter.GaussianBlur(size / 14)))
    d.text((x, y), text, font=f, fill=colour)


def row(img, names, size, y_ground, left, right):
    step = (right - left) / max(len(names) - 1, 1)
    for i, (name, frame) in enumerate(names):
        put(img, name, frame, size, left + i * step, y_ground)


FIVE = [("ghost", 3), ("slime", 0), ("cat", 0), ("heart", 6), ("star", 0)]
TEN = FIVE + [("bunny", 0), ("frog", 6), ("robot", 3), ("cloud", 0), ("mushroom", 3)]


def header(w, h, with_title=True, names=FIVE, scale=0.42):
    img = sky(w, h)
    size = int(h * scale)
    row(img, names, size, h * 0.93, w * 0.12, w * 0.88)
    if with_title:
        title(img, TITLE, int(h * 0.2), w / 2, h * 0.22)
    return img


def save(img, name):
    OUT.mkdir(parents=True, exist_ok=True)
    img.convert("RGB").save(OUT / name, optimize=True)
    print(f"{OUT / name}  {img.size[0]}x{img.size[1]}")


def main():
    save(header(920, 430), "steam-header-920x430.png")
    small = sky(462, 174)
    put(small, "ghost", 3, 120, 70, 168)
    put(small, "cat", 0, 120, 392, 168)
    title(small, TITLE, 46, 231, 80)
    save(small, "steam-small-462x174.png")
    main_cap = header(1232, 706, names=TEN, scale=0.26)
    title(main_cap, TAGLINE, 34, 616, 260, colour=(236, 230, 255, 255))
    save(main_cap, "steam-main-1232x706.png")
    vertical = sky(748, 896)
    title(vertical, TITLE, 92, 374, 150)
    for i, (name, frame) in enumerate(FIVE):
        # Three on the first row, two between them on the second.
        put(vertical, name, frame, 210, 150 + (i % 3) * 225 + (i // 3) * 112, 520 + (i // 3) * 270)
    save(vertical, "steam-vertical-748x896.png")
    library = sky(600, 900)
    title(library, TITLE, 76, 300, 140)
    for i, (name, frame) in enumerate(FIVE[:4]):
        put(library, name, frame, 220, 170 + (i % 2) * 260, 520 + (i // 2) * 260)
    save(library, "steam-library-600x900.png")
    hero = sky(3840, 1240)
    row(hero, TEN, 300, 1180, 520, 3320)
    save(hero, "steam-library-hero-3840x1240.png")
    logo = Image.new("RGBA", (1280, 720), (0, 0, 0, 0))
    put(logo, "ghost", 3, 360, 640, 420)
    title(logo, TITLE, 150, 640, 560)
    OUT.mkdir(parents=True, exist_ok=True)
    logo.save(OUT / "steam-logo-1280x720.png", optimize=True)
    print(f"{OUT / 'steam-logo-1280x720.png'}  1280x720 (transparent)")
    cover = sky(630, 500)
    title(cover, TITLE, 74, 315, 90)
    row(cover, [("ghost", 3), ("cat", 0), ("bunny", 0)], 190, 470, 130, 500)
    put(cover, "heart", 6, 120, 500, 250)
    put(cover, "star", 0, 110, 120, 250)
    save(cover, "itch-cover-630x500.png")
    banner = header(1600, 500, names=TEN, scale=0.36)
    save(banner, "banner-1600x500.png")


if __name__ == "__main__":
    main()
