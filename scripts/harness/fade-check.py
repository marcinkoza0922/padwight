#!/usr/bin/env python3
"""Checks the pictures `fade_frames` draws (src/overlay/fade_frames.rs): whether a menu fades in
as one picture. Faded as one, every pixel at opacity `o` is the background plus `o` of the way to
the same pixel fully shown. Parts faded one by one stack up where they overlap, and a part that
ignores the fade shows at full strength; either way, pixels land away from that line.

For each scene and background it prints, per opacity, how many pixels are more than 12 levels
(of 255) off, the worst, and the box they fall in. Blending may happen in linear light or in
sRGB; the one that fits the plain panel (a single layer, which fades exactly either way) is used.

Usage: PADWIGHT_FADE_FRAMES=screenshots/fade cargo test fade_frames -- --ignored
       scripts/harness/fade-check.py [dir]          (default: screenshots/fade)
"""

import glob
import os
import re
import sys

from PIL import Image

folder = sys.argv[1] if len(sys.argv) > 1 else "screenshots/fade"
STEP, OFF = 2, 12


def to_linear(v):
    v /= 255
    return v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4


def to_srgb(v):
    v = max(0.0, min(1.0, v))
    return 255 * (v * 12.92 if v <= 0.0031308 else 1.055 * v ** (1 / 2.4) - 0.055)


def expected(bg, full, o, linear):
    if linear:
        return tuple(to_srgb(to_linear(b) + o * (to_linear(f) - to_linear(b))) for b, f in zip(bg, full))
    return tuple(b + o * (f - b) for b, f in zip(bg, full))


frames = {}
for path in glob.glob(os.path.join(folder, "*.png")):
    m = re.match(r"(.+)-(black|light)-(\d{3})(-[\w-]+)?\.png$", os.path.basename(path))
    if m:
        frames.setdefault((m[1], m[2]), {})[int(m[3]) / 100] = path

worst_all = 0
for (scene, bg_name), shots in sorted(frames.items()):
    if 1.0 not in shots:
        continue
    full = Image.open(shots[1.0]).convert("RGB")
    w, h = full.size
    bg = full.getpixel((2, 2))
    drawn = [(x, y) for y in range(0, h, STEP) for x in range(0, w, STEP) if full.getpixel((x, y)) != bg]
    # Which blending fits a pixel of plain panel: the most common drawn color fully shown.
    counts = {}
    for p in drawn:
        counts[full.getpixel(p)] = counts.get(full.getpixel(p), 0) + 1
    panel_color = max(counts, key=counts.get)
    panel = next(p for p in drawn if full.getpixel(p) == panel_color)
    print(f"== {scene} on {bg_name}")
    for o, path in sorted(shots.items()):
        if o == 1.0:
            continue
        im = Image.open(path).convert("RGB")
        linear = min(
            (False, True),
            key=lambda lin: sum(abs(a - b) for a, b in zip(im.getpixel(panel), expected(bg, panel_color, o, lin))),
        )
        off = []
        for p in drawn:
            want = expected(bg, full.getpixel(p), o, linear)
            d = max(abs(a - b) for a, b in zip(im.getpixel(p), want))
            if d > OFF:
                off.append((d, p))
        worst = max((d for d, _ in off), default=0)
        worst_all = max(worst_all, worst)
        box = ""
        if off:
            xs, ys = [p[0] for _, p in off], [p[1] for _, p in off]
            box = f" in {min(xs)},{min(ys)}..{max(xs)},{max(ys)}"
        share = 100 * len(off) / max(len(drawn), 1)
        print(f"  {o:.2f} ({'linear' if linear else 'srgb'}): {share:5.1f}% of pixels off, worst {worst:.0f}{box}")
print(f"worst anywhere: {worst_all:.0f}")
