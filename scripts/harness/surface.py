"""Checks the native overlay's timing log: every buffer committed at the size last configured
(a shrink's Clear goes on at the old, still-acked size), none before the first configure, and
each showing round's first commit at full size, with no 1x1 commit while showing.

Run: PADWIGHT_HARNESS_RELEASE=1 PADWIGHT_OVERLAY=native scripts/overlay-harness.sh
scripts/harness/pacing.sh <out>, then python3 -I scripts/harness/surface.py <out>/overlay.log"""
import sys

configured = None
target = "Corner"
bad = []
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
print("violations:", bad or "none")
