#!/usr/bin/env python3
"""Sums up an overlay-harness run, one block per `mark`: the frames the overlay received, how
long it went between builds (the longest gap, and how long after a menu opened it first drew it
partly faded in), the window size it laid out at, and, for a menu closed by a pick, how many
frames it held its pick fully shown and how strongly it flashed.

Usage: scripts/harness/analyze.py [overlay.log]   (default: screenshots/harness/overlay.log)
       add -v to list every build under its mark
"""

import re
import sys

args = [a for a in sys.argv[1:] if a != "-v"]
verbose = "-v" in sys.argv[1:]
path = args[0] if args else "screenshots/harness/overlay.log"

MENU = re.compile(r"opacity=([\d.]+) lit=([\d.]+) flash=([\d.]+) since=open:([\d.]+),closed:(\S+?),moved")


class Block:
    def __init__(self, name, start):
        self.name, self.start = name, start
        self.recv, self.sizes, self.gaps = [], set(), []
        self.first_partial = None
        self.held = 0
        self.flash = 0.0
        self.lines = []


blocks, block, last_built = [], None, None
for line in open(path):
    parts = line.split()
    if not parts:
        continue
    kind, ns = parts[0], int(parts[1])
    if kind == "mark":
        block = Block(" ".join(parts[2:]), ns)
        blocks.append(block)
        continue
    if block is None:
        continue
    ms = (ns - block.start) / 1e6
    if kind == "recv":
        block.recv.append(f"{ms:.0f}ms " + " ".join(parts[2:]))
    elif kind == "built":
        if last_built is not None:
            block.gaps.append(((ns - last_built) / 1e6, ms))
        last_built = ns
        if len(parts) > 3:
            block.sizes.add(parts[3])
        if verbose:
            block.lines.append(f"  {ms:7.1f}ms built {' '.join(parts[2:])}")
    elif kind == "menu":
        m = MENU.search(line)
        if not m:
            continue
        opacity, lit, flash, opened, closed = m.groups()
        opacity, lit, flash, opened = float(opacity), float(lit), float(flash), float(opened)
        if closed == "None" and 0.0 < opacity < 1.0 and block.first_partial is None:
            block.first_partial = opened * 1000
        if closed != "None" and lit >= 1.0 and opacity >= 1.0:
            block.held += 1
        block.flash = max(block.flash, flash)
        if verbose:
            block.lines.append(f"  {ms:7.1f}ms menu opacity={opacity:.2f} lit={lit:.2f} flash={flash:.2f}")

for b in blocks:
    print(f"== {b.name}")
    for r in b.recv:
        print(f"  recv {r}")
    longest = max(b.gaps, default=(0, 0))
    print(f"  longest gap between builds {longest[0]:.0f}ms (ending at {longest[1]:.0f}ms); laid out at {', '.join(sorted(b.sizes)) or '-'}")
    if b.first_partial is not None:
        print(f"  first partly faded-in frame {b.first_partial:.0f}ms after opening")
    if b.held or b.flash:
        print(f"  pick held fully shown for {b.held} frames, flash up to {b.flash:.2f}")
    for line in b.lines:
        print(line)
