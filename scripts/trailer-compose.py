#!/usr/bin/env python3
"""The store trailer, put together: a title card, four recorded scenes
with a caption each, an end card, cross-faded, no sound.

    cargo run --release --example store_pictures -- /tmp/pictures
    scripts/trailer-compose.py /tmp/scenes /tmp/pictures [out-folder]

The scenes are screen recordings at 1920x1080, 30 fps, named as in
SCENES below (a-desktop.mkv and the rest); docs/store-listing.md says
how they were made. The cards are drawn here, from the same sky and
titles as scripts/store-art.py, with the characters from
examples/store_pictures.rs. Writes:

    trailer-1080p.webm  VP9, for itch.io and the site's downloads
    trailer-1080p.mp4   H.264, for stores that ask for MP4 (Steam)
    trailer-720p.webm   VP9, small, for the product page
    trailer-poster.png  the title card, 1280x720

Needs Pillow and ffmpeg with libvpx-vp9 and libx264.
"""
import importlib.util
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image, ImageDraw

SCENES_DIR = Path(sys.argv[1])
PICTURES = Path(sys.argv[2])
OUT = Path(sys.argv[3]) if len(sys.argv) > 3 else Path("build/trailer")

# The scenes: file, the part of it used (seconds), its caption.
SCENES = [
    ("a-desktop.mkv", 0.7, 6.7, "They live on your desktop"),
    ("b-tap.mkv", 0.6, 7.8, "Tap one, and it answers"),
    ("c-welcome.mkv", 1.0, 10.3, "Choose who lives there"),
    ("d-pick-up.mkv", 0.6, 7.0, "Pick them up. Let them fall."),
]
TITLE_SECONDS = 3.0
END_SECONDS = 4.0
FADE = 0.5
W, H, FPS = 1920, 1080, 30

# store-art.py's sky, titles and characters; it reads its folders from
# the command line as it loads.
sys.argv = [sys.argv[0], str(PICTURES), str(OUT)]
spec = importlib.util.spec_from_file_location("store_art", Path(__file__).with_name("store-art.py"))
art = importlib.util.module_from_spec(spec)
spec.loader.exec_module(art)


def title_card(path):
    img = art.sky(W, H, seed=11)
    art.row(img, art.TEN, 230, H * 0.965, W * 0.09, W * 0.91)
    art.title(img, art.TITLE, 170, W / 2, H * 0.33)
    art.title(img, art.TAGLINE, 52, W / 2, H * 0.47, colour=(236, 230, 255, 255))
    img.convert("RGB").save(path)


def end_card(path):
    img = art.sky(W, H, seed=5)
    art.row(img, art.FIVE, 250, H * 0.965, W * 0.2, W * 0.8)
    art.title(img, art.TITLE, 150, W / 2, H * 0.3)
    art.title(img, art.TAGLINE, 50, W / 2, H * 0.43, colour=(236, 230, 255, 255))
    art.title(img, "For Linux and Windows", 40, W / 2, H * 0.52, colour=(206, 198, 245, 255))
    img.convert("RGB").save(path)


def caption(text, path):
    """A pill in the top left corner, clear of the characters."""
    img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    font = art.font(46)
    d = ImageDraw.Draw(img)
    box = d.textbbox((0, 0), text, font=font)
    tw, th = box[2] - box[0], box[3] - box[1]
    x, y, px, py = 64, 56, 30, 18
    d.rounded_rectangle([x, y, x + tw + 2 * px, y + th + 2 * py], radius=(th + 2 * py) // 2,
                        fill=(22, 14, 52, 196))
    d.text((x + px - box[0], y + py - box[1]), text, font=font, fill=(255, 255, 255, 255))
    img.save(path)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    tmp = Path(tempfile.mkdtemp(prefix="anima-trailer-"))
    title_card(tmp / "title.png")
    end_card(tmp / "end.png")
    inputs = ["-loop", "1", "-framerate", str(FPS), "-t", str(TITLE_SECONDS), "-i", str(tmp / "title.png")]
    for name, *_ in SCENES:
        inputs += ["-i", str(SCENES_DIR / name)]
    inputs += ["-loop", "1", "-framerate", str(FPS), "-t", str(END_SECONDS), "-i", str(tmp / "end.png")]
    for i, (*_, text) in enumerate(SCENES):
        caption(text, tmp / f"caption-{i}.png")
        inputs += ["-loop", "1", "-framerate", str(FPS), "-i", str(tmp / f"caption-{i}.png")]

    norm = f"fps={FPS},format=yuv420p,settb=AVTB"
    graph = [f"[0:v]{norm},fade=t=in:st=0:d=0.6[v0]"]
    durations = [TITLE_SECONDS]
    end_input = len(SCENES) + 1
    for i, (_, start, end, _) in enumerate(SCENES, start=1):
        d = end - start
        durations.append(d)
        cap = end_input + i
        graph.append(f"[{i}:v]trim=start={start}:end={end},setpts=PTS-STARTPTS,fps={FPS},format=rgba[s{i}]")
        graph.append(f"[{cap}:v]trim=duration={d},format=rgba,"
                     f"fade=t=in:st=0.35:d=0.4:alpha=1,fade=t=out:st={d - 0.9:.2f}:d=0.4:alpha=1[c{i}]")
        graph.append(f"[s{i}][c{i}]overlay=0:0,{norm}[v{i}]")
    graph.append(f"[{end_input}:v]{norm},fade=t=out:st={END_SECONDS - 0.8}:d=0.8[v{end_input}]")
    durations.append(END_SECONDS)
    last, offset = "v0", 0.0
    for i in range(1, end_input + 1):
        offset += durations[i - 1] - FADE
        out = f"x{i}" if i < end_input else "out"
        graph.append(f"[{last}][v{i}]xfade=transition=fade:duration={FADE}:offset={offset:.3f}[{out}]")
        last = out
    master = tmp / "master.mkv"
    subprocess.run(["ffmpeg", "-v", "error", "-y", *inputs, "-filter_complex", ";".join(graph),
                    "-map", "[out]", "-c:v", "libx264", "-preset", "veryslow", "-crf", "8",
                    "-pix_fmt", "yuv420p", str(master)], check=True)
    vp9 = ["-c:v", "libvpx-vp9", "-b:v", "0", "-row-mt", "1", "-deadline", "good", "-cpu-used", "2",
           "-pix_fmt", "yuv420p", "-an"]
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-i", str(master), *vp9, "-crf", "30",
                    str(OUT / "trailer-1080p.webm")], check=True)
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-i", str(master), "-c:v", "libx264", "-preset", "slow",
                    "-crf", "18", "-profile:v", "high", "-pix_fmt", "yuv420p", "-movflags", "+faststart",
                    "-an", str(OUT / "trailer-1080p.mp4")], check=True)
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-i", str(master), "-vf", "scale=1280:720:flags=lanczos",
                    *vp9, "-crf", "36", str(OUT / "trailer-720p.webm")], check=True)
    Image.open(tmp / "title.png").resize((1280, 720), Image.LANCZOS).save(OUT / "trailer-poster.png", optimize=True)
    for f in sorted(OUT.glob("trailer*")):
        print(f"{f}  {f.stat().st_size // 1024} KB")
    print(f"{sum(durations) - FADE * (len(durations) - 1):.1f} s")
    for f in tmp.iterdir():
        f.unlink()
    tmp.rmdir()


if __name__ == "__main__":
    main()
