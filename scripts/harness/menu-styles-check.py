#!/usr/bin/env python3
"""Judges an overlay-harness run of scripts/harness/menu-styles.sh (OVL-3): pass or fail, with
numbers. For every open-close round of every menu style:

  stale      a menu line (the overlay drawing a menu) under a different menu's name: a stale frame
  flicker    the menu fading in lost opacity, or the menu fading out gained it, by more than 0.05
  unfinished the fade-out had not nearly finished (its last drawn frame over --last-opacity, one
             frame's step from nothing) when the round went idle
  lingering  the overlay drew a menu after its round went idle (a frame that should be gone)
  slow       the first frame of a menu took more than --open-ms to be built after it was asked for

and for the minute-long hold: how many frames were built and presented while the menu held still
(the overlay must go quiet), the overlay's CPU while held still and once hidden (cpu.txt), and
that the daemon's last frame shows nothing.

Usage: python3 -I scripts/harness/menu-styles-check.py <out dir> [--open-ms 150] [--still-frames 5]
           [--last-opacity 0.2] [--still-cpu 3.0] [--hidden-cpu 1.0]
The CPU limits are shares of one core; they suit the optimized build (PADWIGHT_HARNESS_RELEASE=1),
and a debug build needs larger ones. Exit status 1 when any check fails.
"""

import json
import os
import re
import sys

args = sys.argv[1:]


def option(name, default):
    if name in args:
        i = args.index(name)
        value = float(args[i + 1])
        del args[i : i + 2]
        return value
    return default


OPEN_MS = option("--open-ms", 150.0)
LAST_OPACITY = option("--last-opacity", 0.2)
STILL_FRAMES = option("--still-frames", 5)
STILL_CPU = option("--still-cpu", 3.0)
HIDDEN_CPU = option("--hidden-cpu", 1.0)
folder = args[0] if args else "screenshots/harness"

MENU = re.compile(r'^menu (\d+) "([^"]*)" .*?opacity=([\d.]+).*?closed:(\S+?),')
STYLES = ["Directional", "Radial", "List", "Buttons", "Carousel", "Grid"]

events = []  # (ms, kind, payload)
t0 = None
for line in open(os.path.join(folder, "overlay.log")):
    parts = line.split()
    if len(parts) < 2 or not parts[1].isdigit():
        continue
    ns = int(parts[1])
    t0 = ns if t0 is None else t0
    ms = (ns - t0) / 1e6
    kind = parts[0]
    if kind == "mark":
        events.append((ms, "mark", " ".join(parts[2:])))
    elif kind == "menu":
        m = MENU.match(line)
        if m:
            events.append((ms, "menu", (m[2], float(m[3]), m[4] != "None")))
    elif kind in ("recv", "built", "present"):
        events.append((ms, kind, " ".join(parts[2:])))

marks = [(ms, text) for ms, kind, text in events if kind == "mark"]
problems = []
totals = {s: dict(rounds=0, stale=0, flicker=0, unfinished=0, lingering=0, slow=0) for s in STYLES}


def between(a, b):
    return [e for e in events if a <= e[0] < b]


for i, (start, text) in enumerate(marks):
    words = text.split()
    if len(words) != 3 or words[0] not in STYLES or words[2] != "open":
        continue
    style, n = words[0], words[1]
    idle = next((m for m in marks[i + 1 :] if m[1] == f"{style} {n} idle"), None)
    after = next((m for m in marks[i + 1 :] if m[1] != f"{style} {n} idle" and m[1].split()[-1] == "open" or m[1] == "hold start"), None)
    if not idle:
        problems.append(f"{style} {n}: the round never reached its idle mark")
        continue
    t = totals[style]
    t["rounds"] += 1
    block = between(start, idle[0])
    menus = [(ms, p) for ms, k, p in block if k == "menu"]
    t["stale"] += sum(1 for _, (title, _, _) in menus if title != style)
    # Fading in: opacity must not fall. Fading out: it must not rise.
    top, last_out = 0.0, None
    for _, (title, opacity, closing) in menus:
        if not closing:
            if opacity < top - 0.05:
                t["flicker"] += 1
            top = max(top, opacity)
        else:
            if last_out is not None and opacity > last_out + 0.05:
                t["flicker"] += 1
            last_out = opacity
    final = menus[-1][1] if menus else None
    if final is None or final[1] > LAST_OPACITY:
        t["unfinished"] += 1
    # Nothing may be drawn after the round went idle, up to the next round.
    end = after[0] if after else idle[0] + 300
    t["lingering"] += sum(1 for ms, k, p in between(idle[0], end) if k == "menu")
    # How long the overlay took to build its first frame of the menu after the daemon sent it.
    asked = next((ms for ms, k, p in block if k == "recv" and p.startswith("menu")), None)
    drawn = next((ms for ms, k, p in block if k == "menu"), None)
    if asked is not None and drawn is not None and drawn - asked > OPEN_MS:
        t["slow"] += 1

print("== open-close rounds per menu style")
print(f"  {'style':12} rounds stale flicker unfinished lingering slow")
failed = False
for style in STYLES:
    t = totals[style]
    bad = t["rounds"] == 0 or any(t[k] for k in ("stale", "flicker", "unfinished", "lingering", "slow"))
    failed |= bad
    print(f"  {style:12} {t['rounds']:6} {t['stale']:5} {t['flicker']:7} {t['unfinished']:10} {t['lingering']:9} {t['slow']:4}  {'FAIL' if bad else 'ok'}")

print("== held still")
start = next((ms for ms, text in marks if text == "hold still"), None)
end = next((ms for ms, text in marks if text == "hold end"), None)
if start is None or end is None:
    problems.append("the run has no hold marks")
else:
    window = between(start, end)
    began = next((ms for ms, text in marks if text == "hold start"), start)
    up = [p for ms, k, p in between(began, start) if k == "menu"]
    gone = [ms for ms, k, p in window if k == "recv" and p.startswith("none")]
    bad = bool(not up or up[-1][0] != "Radial" or up[-1][1] < 0.99 or gone)
    failed |= bad
    print(f"  the radial menu is fully shown by the start of the hold and stays up: {'FAIL' if bad else 'ok'}")
    built = sum(1 for _, k, _ in window if k == "built")
    shown = sum(1 for _, k, _ in window if k == "present")
    bad = built > STILL_FRAMES or shown > STILL_FRAMES
    failed |= bad
    print(f"  {(end - start) / 1000:.0f} s held: {built} frames built, {shown} presented (limit {STILL_FRAMES:g})  {'FAIL' if bad else 'ok'}")
cpu = {}
try:
    for line in open(os.path.join(folder, "cpu.txt")):
        name, value = line.split()
        cpu[name] = float(value)
except OSError:
    problems.append("no cpu.txt")
for name, limit in (("still", STILL_CPU), ("hidden", HIDDEN_CPU)):
    if name in cpu:
        bad = cpu[name] > limit
        failed |= bad
        print(f"  overlay CPU while {name}: {cpu[name]:.2f}% of one core (limit {limit:g}%)  {'FAIL' if bad else 'ok'}")
    else:
        problems.append(f"no {name} CPU reading")

print("== last frame from the daemon")
frames = [json.loads(l.split(" ", 1)[1]) for l in open(os.path.join(folder, "frames.log")) if " " in l]
if frames and frames[-1].get("active") is None:
    print("  shows nothing  ok")
else:
    failed = True
    print(f"  still shows {str(frames[-1].get('active'))[:60] if frames else 'no frame at all'}  FAIL")

for p in problems:
    failed = True
    print(f"  problem: {p}")
print("FAIL" if failed else "PASS")
sys.exit(1 if failed else 0)
