#!/usr/bin/env python3
"""Frame pacing of the overlay, from an overlay-harness run of scripts/harness/pacing.sh: how far
apart frames are shown against the display's refresh, how many refreshes were missed, how long a
frame from the daemon waits for its first `present`, and how evenly a fade steps.

A `present` line is stamped as each buffer is committed (src/overlay/wayland/window.rs); what the
compositor then shows is not in the log, so frames dropped at the display are inferred.
Frames shown twice are refreshes in which nothing was presented (an interval of two or more
refreshes), and presents with no new build since the one before (the same content again).

Usage: python3 -I scripts/harness/pacing.py [overlay.log] [--hz 60]
"""

import re
import statistics
import sys

args = [a for a in sys.argv[1:]]
hz = 60.0
if "--hz" in args:
    i = args.index("--hz")
    hz = float(args[i + 1])
    del args[i : i + 2]
path = args[0] if args else "screenshots/harness/overlay.log"
refresh = 1000.0 / hz
BREAK = 100.0  # ms: a longer gap is the overlay going idle, not a missed frame
MENU = re.compile(r"opacity=([\d.]+) lit=([\d.]+) flash=([\d.]+) since=open:([\d.]+),closed:(\S+?),moved")

events = []  # (ms since first line, kind, phase, payload)
phase = None
t0 = None
for line in open(path):
    parts = line.split()
    if len(parts) < 2:
        continue
    kind, ns = parts[0], int(parts[1])
    if t0 is None:
        t0 = ns
    ms = (ns - t0) / 1e6
    if kind == "mark":
        phase = parts[-1] if len(parts) > 3 else " ".join(parts[2:])
        continue
    if kind == "menu":
        m = MENU.search(line)
        if m:
            events.append((ms, "menu", phase, (float(m[1]), m[5] != "None")))
    elif kind in ("recv", "built", "present"):
        events.append((ms, kind, phase, None))


def pct(values, p):
    values = sorted(values)
    return values[min(len(values) - 1, int(len(values) * p / 100))] if values else float("nan")


def summary(label, ivs):
    if not ivs:
        print(f"  {label}: no frames")
        return
    missed = sum(max(0, round(v / refresh) - 1) for v in ivs)
    print(f"  {label}: {len(ivs) + 1} frames, interval median {statistics.median(ivs):.1f} p95 {pct(ivs, 95):.1f} max {max(ivs):.1f} ms; {missed} missed refreshes")


presents = [e for e in events if e[1] == "present"]
print(f"== frame intervals (refresh {refresh:.1f} ms; gaps over {BREAK:.0f} ms, or ended by a daemon frame, are idle and left out)")
by_phase = {}
all_ivs = []
recvs = [e[0] for e in events if e[1] == "recv"]
for a, b in zip(presents, presents[1:]):
    iv = b[0] - a[0]
    # A frame from the daemon after the overlay went still wakes it: that gap is idle time.
    woken = any(a[0] < r <= b[0] for r in recvs) and iv > 1.5 * refresh
    if iv < BREAK and not woken:
        all_ivs.append(iv)
        by_phase.setdefault(b[2], []).append(iv)
summary("all", all_ivs)
for name in ("open", "move", "hold", "close", "idle"):
    ivs = by_phase.get(name, [])
    n = sum(1 for p in presents if p[2] == name)
    if name in ("hold", "idle"):
        print(f"  {name}: {n} frames presented")
    else:
        summary(name, ivs)
edges = [0, 8, 14, 18, 25, 33, 50, BREAK]
counts = [sum(1 for v in all_ivs if lo <= v < hi) for lo, hi in zip(edges, edges[1:])]
print("  histogram (ms): " + "  ".join(f"{lo}-{hi:g}:{c}" for (lo, hi), c in zip(zip(edges, edges[1:]), counts)))
doubled = sum(1 for v in all_ivs if v >= 1.5 * refresh)
print(f"  shown twice or more (interval >= 1.5 refreshes): {doubled} of {len(all_ivs)} intervals")

if all_ivs:
    med = statistics.median(all_ivs)
    print(
        f"  timer drift: median interval is {med / refresh:.3f} refreshes, so a display locked to {hz:g} Hz would "
        f"show {max(0.0, 1000 / med - hz):.1f} frames/s fewer than are presented (inferred, not seen in the log)"
    )

# Presents with no build since the previous present repeat the same content.
same = 0
built_since = True
total = 0
for e in events:
    if e[1] == "built":
        built_since = True
    elif e[1] == "present":
        total += 1
        if not built_since:
            same += 1
        built_since = False
print(f"  presents with no build since the last present: {same} of {total}")

print("== recv to first present")
lat = []
pt = [p[0] for p in presents]
for e in events:
    if e[1] != "recv":
        continue
    nxt = next((t for t in pt if t >= e[0]), None)
    if nxt is not None and nxt - e[0] < 1000:
        lat.append(nxt - e[0])
if lat:
    print(f"  {len(lat)} frames: median {statistics.median(lat):.1f} p95 {pct(lat, 95):.1f} max {max(lat):.1f} ms")
else:
    print("  no recv followed by a present")

print("== fade steps (menu opacity at each presented frame)")
series = []  # (phase, closing, opacity) per present
menu, fresh = None, False
for e in events:
    if e[1] == "menu":
        menu, fresh = e[3], True
    elif e[1] == "present":
        # A present with no new menu line draws the same picture again (or no menu): not a step.
        if menu is not None and fresh:
            series.append((e[0], e[2], menu))
        fresh = False


def runs(closing):
    run = []
    for t, ph, (o, c) in series:
        if c == closing and 0.0 < o < 1.0:
            run.append((t, o))
        elif run:
            yield run
            run = []
    if run:
        yield run


for name, closing in (("open", False), ("close", True)):
    steps, frames, lens = [], [], []
    for run in runs(closing):
        if len(run) < 3:
            continue
        frames.append(len(run))
        lens.append(run[-1][0] - run[0][0])
        steps += [abs(b[1] - a[1]) for a, b in zip(run, run[1:])]
    if not steps:
        print(f"  {name}: no fades seen")
        continue
    print(
        f"  {name}: {len(frames)} fades, {statistics.median(frames):.0f} partly faded frames each over {statistics.median(lens):.0f} ms; "
        f"step median {statistics.median(steps):.3f} min {min(steps):.3f} max {max(steps):.3f} "
        f"(max/median {max(steps) / statistics.median(steps):.2f}, stdev {statistics.pstdev(steps):.3f})"
    )
