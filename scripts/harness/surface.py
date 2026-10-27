"""Checks the native overlay's timing log: every buffer committed at the size last configured
(a shrink's Clear goes on at the old, still-acked size), none before the first configure, and
each showing round's first commit at full size, with no 1x1 commit while showing.

Run: PADWIGHT_HARNESS_RELEASE=1 scripts/overlay-harness.sh
scripts/harness/pacing.sh <out>, then python3 -I scripts/harness/surface.py <out>/overlay.log [scale]
(scripts/harness/scale.sh with SCALE=1.5 sets the output's scale with kscreen-doctor, as KWin's --scale doesn't
change the surface's scale; pass the scale here too)"""
import sys

configured = None
target = "Corner"
bad = []
devices = set()
scale = float(sys.argv[2]) if len(sys.argv) > 2 else 1.0
rounds = []
for p in (l.split() for l in open(sys.argv[1])):
    k = p[0]
    if k == "surface":
        target = p[3]
        if target == "Screen":
            rounds.append({"first": None, "commits": 0, "small": 0, "before_configure": 0, "configure": None})
    elif k == "configure":
        configured = p[2]
        if target == "Screen" and rounds and rounds[-1]["configure"] is None:
            rounds[-1]["configure"] = configured
    elif k == "commit":
        size, why = p[2], p[3]
        # `buffer WxH <mode>` at the end: the device size, which must follow the scale.
        if "buffer" in p:
            dev = p[p.index("buffer") + 1]
            lw, lh = map(int, size.split("x"))
            dw, dh = map(int, dev.split("x"))
            if "Viewport" in p[-1]:
                ok = abs(dw - lw * scale) <= 1 and abs(dh - lh * scale) <= 1
            else:
                n = int(p[-1].strip("Buffer()"))
                ok = (dw, dh) == (lw * n, lh * n)
            if not ok:
                bad.append(f"buffer {dev} is not {size} at the scale: " + " ".join(p))
            devices.add(dev)
        if configured is None:
            bad.append("commit before any configure: " + " ".join(p))
        elif size != configured:
            bad.append("size differs from the configure acked: " + " ".join(p))
        if target == "Screen" and rounds:
            r = rounds[-1]
            r["commits"] += 1
            if r["configure"] is None:
                r["before_configure"] += 1
            if r["first"] is None:
                r["first"] = size
            if size == "1x1":
                r["small"] += 1
for i, r in enumerate(rounds, 1):
    print(f"round {i}: configure {r['configure']}, first commit {r['first']}, {r['commits']} commits while showing, "
          f"{r['small']} at 1x1, {r['before_configure']} before the configure")
print("buffer sizes:", sorted(devices))
print("violations:", bad or "none")
