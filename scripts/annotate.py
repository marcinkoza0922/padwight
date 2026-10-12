#!/usr/bin/env python3
"""Annotates screenshots for the manual.

usage: scripts/annotate.py <spec.json>...
       scripts/annotate.py grid <screenshot.png>... [x y w h]

A spec is a JSON list of images:
  {"src": "screenshots/manual/x.png",   # a raw screenshot (see `padwight debug gui`)
   "out": "docs/images/manual/x.png",
   "crop": [x, y, w, h],                # optional, in the source's pixels
   "marks": [ ... ]}                    # coordinates are in the source's pixels too

Marks:
  {"box":   [x, y, w, h], "n": 1}       a rectangle, with a numbered badge on its corner when "n" is set
  {"arrow": [x0, y0, x1, y1]}           an arrow, head at (x1, y1)
  {"note":  [x, y], "text": "..."}      a label (a badge-colored pill) with its top left at (x, y)
  {"dim":   [x, y, w, h]}               dims everything outside the rectangle (a spotlight)
  {"blur":  [x, y, w, h]}               hides a region

Spec keys besides "marks": "crop" [x, y, w, h]; "autocrop" <margin> (the part that isn't plain background, for overlay
scenes; applied before "crop"); "scale" <factor> (applied last).

`grid` writes each screenshot with a labeled 50-pixel grid to screenshots/manual/grid/, for reading coordinates.
"""
import json
import math
import sys
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter, ImageFont

ACCENT = (232, 89, 12, 255)
INK = (255, 255, 255, 255)
FONT = Path(__file__).resolve().parent.parent / "assets/fonts/Inter-Regular.ttf"


def font(size):
    return ImageFont.truetype(str(FONT), size)


def badge(draw, cx, cy, n, r=11):
    draw.ellipse([cx - r, cy - r, cx + r, cy + r], fill=ACCENT, outline=INK, width=2)
    draw.text((cx, cy), str(n), fill=INK, font=font(14), anchor="mm")


def arrow(draw, x0, y0, x1, y1, width=3):
    draw.line([x0, y0, x1, y1], fill=ACCENT, width=width)
    angle = math.atan2(y1 - y0, x1 - x0)
    head = 13
    pts = [(x1, y1)] + [
        (x1 - head * math.cos(angle + s * 0.45), y1 - head * math.sin(angle + s * 0.45)) for s in (-1, 1)
    ]
    draw.polygon(pts, fill=ACCENT)


def note(draw, x, y, text):
    f = font(15)
    l, t, r, b = draw.textbbox((0, 0), text, font=f)
    draw.rounded_rectangle([x, y, x + r - l + 16, y + b - t + 10], radius=6, fill=ACCENT)
    draw.text((x + 8, y + 5 - t), text, fill=INK, font=f)


def grid(path, crop=None):
    g = Image.open(path).convert("RGBA")
    gd = ImageDraw.Draw(g)
    for x in range(0, g.width, 50):
        gd.line([x, 0, x, g.height], fill=(255, 0, 255, 90 if x % 100 else 160))
        if x % 100 == 0:
            gd.text((x + 2, 2), str(x), fill=(200, 0, 200, 255), font=font(11))
    for y in range(0, g.height, 50):
        gd.line([0, y, g.width, y], fill=(255, 0, 255, 90 if y % 100 else 160))
        if y % 100 == 0:
            gd.text((2, y + 2), str(y), fill=(200, 0, 200, 255), font=font(11))
    if crop:
        x, y, w, h = crop
        g = g.crop((x, y, x + w, y + h))
    out = Path("screenshots/manual/grid") / Path(path).name
    out.parent.mkdir(parents=True, exist_ok=True)
    g.convert("RGB").save(out)
    print(out)


def render(spec):
    img = Image.open(spec["src"]).convert("RGBA")
    marks = spec.get("marks", [])
    for m in marks:
        if "blur" in m:
            x, y, w, h = m["blur"]
            region = img.crop((x, y, x + w, y + h)).filter(ImageFilter.GaussianBlur(8))
            img.paste(region, (x, y))
    for m in marks:
        if "dim" in m:
            x, y, w, h = m["dim"]
            shade = Image.new("RGBA", img.size, (0, 0, 0, 110))
            ImageDraw.Draw(shade).rectangle([x, y, x + w, y + h], fill=(0, 0, 0, 0))
            img = Image.alpha_composite(img, shade)
    draw = ImageDraw.Draw(img)
    for m in marks:
        if "box" in m:
            x, y, w, h = m["box"]
            draw.rounded_rectangle([x, y, x + w, y + h], radius=5, outline=ACCENT, width=3)
            if "n" in m:
                badge(draw, x, y, m["n"])
        elif "arrow" in m:
            arrow(draw, *m["arrow"])
        elif "note" in m:
            note(draw, *m["note"], m["text"])
    if spec.get("autocrop"):
        # The part that isn't the plain background (an overlay on the scene's backdrop), with a margin.
        flat = Image.new("RGBA", img.size, img.getpixel((0, 0)))
        box = ImageChops.difference(img, flat).convert("L").point(lambda v: 255 if v > 24 else 0).getbbox()
        if box:
            m = spec["autocrop"]
            img = img.crop((max(box[0] - m, 0), max(box[1] - m, 0), min(box[2] + m, img.width), min(box[3] + m, img.height)))
    if "crop" in spec:
        x, y, w, h = spec["crop"]
        img = img.crop((x, y, x + w, y + h))
    if "scale" in spec:
        img = img.resize((round(img.width * spec["scale"]), round(img.height * spec["scale"])), Image.LANCZOS)
    out = Path(spec["out"])
    out.parent.mkdir(parents=True, exist_ok=True)
    # An adaptive palette keeps the pictures small; they are mostly flat colors and text.
    img.convert("RGB").quantize(colors=128, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE).save(out, optimize=True)
    print(out)


def main(args):
    if args[:1] == ["grid"]:
        # `grid <png>... [x y w h]`: a region only, still labeled with the screenshot's own coordinates.
        crop = [int(a) for a in args[-4:]] if len(args) > 4 and args[-1].isdigit() else None
        for path in args[1:len(args) - 4 if crop else None]:
            grid(path, crop)
        return
    for path in args:
        for spec in json.loads(Path(path).read_text()):
            render(spec)


if __name__ == "__main__":
    main(sys.argv[1:])
