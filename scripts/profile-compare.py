#!/usr/bin/env python3
"""Compares two results folders of the profiling suite (scripts/profile.sh, PADWIGHT_PROFILE_OUT)
and flags each measurement that got worse by more than its noise.

Usage: python3 -I scripts/profile-compare.py [--json] [--all] old/ new/
       python3 -I scripts/profile-compare.py --budget new/      (FEEL-5: an absolute latency budget)
       python3 -I scripts/profile-compare.py --self-test

Both folders hold the JSON files tests/profile writes (input_latency.json, resources.json, ...).
A measurement is flagged when new - old (in the bad direction) is larger than both its limit
and its noise:

  limit  latency (p50/p90/p99/mean, startup medians, draw times)  20% of the old value
         CPU                                                      0.5 points of one core
         memory (RSS)                                             10%, at least 2 MB
         threads, open files, missed samples, dropped events      any rise
         daemon binary size                                       5%
  noise  the spread (max - min) of the same measurement across the repeat runs the suite makes:
         per_session in input_latency.json, all_ms in the start-up files, rounds in the overlay
         styles file. Without repeats the noise is 0.

Idle CPU over 1% of a core is flagged whatever the old value was. Folders from different machines,
kernels or build kinds are not comparable; the script says so and stops. Latency on shared CI runners
is too noisy to compare: run this on the same machine each week.

--budget judges one folder against fixed numbers instead of an older run, so "input latency feels
like a wired pad" has a pass/fail: from the press written to the pad to the daemon's virtual pad
reporting it (input_latency.json), in each condition (idle, a 1 kHz stick flood, every CPU busy):

  p50        at most 500 us    a wired pad's own USB poll is 1 to 8 ms; the daemon adds well under that
  p99        at most 2 ms      still a fraction of a poll interval
  max        at most 16.6 ms   never a whole 60 Hz frame late (a rare scheduler stall is the limit)
  missed presses and dropped output events: none

The measured figures (a 12th-gen i7, Docker, 900 presses per condition) were p50 25 to 55 us and
p99 56 to 91 us, so the budget has ten times of room and fails on a real regression, not on a noisy
neighbour. This measures the daemon, not the player: whether it feels right in a game stays manual.

Exit status: 0 nothing worse, 1 something flagged, 2 the folders can't be compared.
"""
import json
import sys
from pathlib import Path

LATENCY_KEYS = {"p50_us", "p90_us", "p99_us", "mean_us", "build_p50_us"}
CPU_BUDGET = 1.0  # percent of one core, idle
ENV_KEYS = ("cpu", "cores", "kernel", "container", "release_build")
SKIP = {"environment", "per_session", "samples_us", "all_ms", "runs", "rounds", "what", "history"}


class Rule:
    def __init__(self, kind, rel=0.0, floor=0.0, unit=""):
        self.kind, self.rel, self.floor, self.unit = kind, rel, floor, unit


def rule_for(key):
    """How to judge a number with this key, or None to ignore it."""
    if key in LATENCY_KEYS:
        return Rule("latency", 0.20, 0, "us")
    if key == "median_ms":
        return Rule("startup", 0.20, 5, "ms")
    if key == "cpu_percent" or key.endswith("_cpu_percent"):
        return Rule("cpu", 0, 0.5, "%")
    if key in ("rss_end_kb", "rss_peak_kb"):
        return Rule("memory", 0.10, 2048, "kB")
    if key in ("threads_peak", "fds_peak", "missed", "dropped_output_events"):
        return Rule("count", 0, 0, "")
    if key == "daemon_binary_bytes":
        return Rule("size", 0.05, 0, "B")
    return None


def walk(node, path=()):
    """Yields (path, value) for each judged number under node."""
    if not isinstance(node, dict):
        return
    for key, value in node.items():
        if key in SKIP or isinstance(value, bool):
            continue
        if isinstance(value, (int, float)):
            if rule_for(key):
                yield path + (key,), value
        else:
            yield from walk(value, path + (key,))


def noise_samples(doc, path):
    """The same measurement in each repeat run of doc, if the suite made repeats."""
    key = path[-1]
    node = doc
    for step in path[:-1]:
        node = node.get(step, {}) if isinstance(node, dict) else {}
    if not isinstance(node, dict):
        return []
    if key == "median_ms" and isinstance(node.get("all_ms"), list):
        return node["all_ms"]
    if isinstance(node.get("rounds"), list):
        return [r[key] for r in node["rounds"] if isinstance(r, dict) and isinstance(r.get(key), (int, float))]
    if len(path) >= 3 and path[-2] == "latency" and isinstance(doc.get("per_session"), list):
        out = [s.get(path[-3], {}).get(key) for s in doc["per_session"]]
        return [v for v in out if isinstance(v, (int, float))]
    return []


def spread(values):
    return max(values) - min(values) if len(values) > 1 else 0.0


def load(folder):
    docs = {}
    for file in sorted(Path(folder).glob("*.json")):
        try:
            docs[file.stem] = json.loads(file.read_text())
        except (OSError, ValueError) as err:
            print(f"warning: {file}: {err}", file=sys.stderr)
    return docs


def environment_mismatch(old, new):
    for name in sorted(old.keys() & new.keys()):
        a, b = old[name].get("environment", {}), new[name].get("environment", {})
        notes = [f"{key}: {a[key]!r} -> {b[key]!r}" for key in ENV_KEYS if key in a and key in b and a[key] != b[key]]
        if notes:
            return notes
    return []


def compare(old, new):
    rows = []
    for name in sorted(old.keys() & new.keys()):
        if "skipped" in old[name] or "skipped" in new[name]:
            continue
        before, after = dict(walk(old[name])), dict(walk(new[name]))
        for path in sorted(before.keys() & after.keys()):
            rule = rule_for(path[-1])
            a, b = before[path], after[path]
            noise = max(spread(noise_samples(old[name], path)), spread(noise_samples(new[name], path)))
            limit = max(rule.rel * abs(a), rule.floor)
            delta = b - a
            over = rule.kind == "cpu" and "idle" in path and b > CPU_BUDGET
            if delta > limit and delta > noise:
                status = "WORSE"
            elif over:
                status = "over 1% budget"
            elif -delta > limit and -delta > noise:
                status = "better"
            else:
                status = "same"
            rows.append((name + "." + ".".join(path), rule, a, b, noise, status))
    return rows


def fmt(value, rule):
    if rule.unit == "kB":
        return f"{value / 1024:.1f} MB"
    if rule.unit == "B":
        return f"{value / 1e6:.2f} MB"
    if rule.unit == "%":
        return f"{value:.2f}%"
    return f"{value:.1f} {rule.unit}".strip()


# FEEL-5: the absolute input-latency budget, in microseconds.
BUDGET_US = {"p50_us": 500, "p99_us": 2000, "max_us": 16600}
CONDITIONS = ("idle", "flood_1khz", "cpu_load")


def over_budget(doc):
    """The (condition, measurement, value, limit) that break the latency budget in an input_latency.json."""
    found = []
    for condition in CONDITIONS:
        node = doc.get(condition)
        if not isinstance(node, dict):
            found.append((condition, "present", 0, 1))
            continue
        for key, limit in BUDGET_US.items():
            value = node.get("latency", {}).get(key)
            if not isinstance(value, (int, float)) or value > limit:
                found.append((condition, key, value, limit))
        for key in ("missed", "dropped_output_events"):
            if node.get(key, 0) != 0:
                found.append((condition, key, node.get(key), 0))
    return found


def budget(folder):
    doc = load(folder).get("input_latency")
    if doc is None or "skipped" in doc:
        print(f"no input_latency.json in {folder} (run scripts/profile.sh profile_input_latency)", file=sys.stderr)
        return 2
    found = over_budget(doc)
    for condition, key, value, limit in found:
        print(f"OVER BUDGET {condition}.{key}: {value} (limit {limit})")
    if not found:
        idle = doc["idle"]["latency"]
        print(f"within the latency budget (idle p50 {idle['p50_us']:.0f} us, p99 {idle['p99_us']:.0f} us)")
    return 1 if found else 0


def self_test():
    good = {c: {"latency": {"p50_us": 55, "p99_us": 91, "max_us": 304}, "missed": 0, "dropped_output_events": 0} for c in CONDITIONS}
    assert over_budget(good) == [], over_budget(good)
    slow = json.loads(json.dumps(good))
    slow["cpu_load"]["latency"]["p99_us"] = 2500
    slow["idle"]["missed"] = 1
    del slow["flood_1khz"]
    names = {(c, k) for c, k, _, _ in over_budget(slow)}
    assert names == {("cpu_load", "p99_us"), ("idle", "missed"), ("flood_1khz", "present")}, names
    stall = json.loads(json.dumps(good))
    stall["idle"]["latency"]["max_us"] = 20000
    assert [(c, k) for c, k, _, _ in over_budget(stall)] == [("idle", "max_us")]
    print("profile-compare budget self-test passed")
    return 0


def main(argv):
    if "--self-test" in argv:
        return self_test()
    if "--budget" in argv:
        folders = [a for a in argv if not a.startswith("--")]
        if len(folders) != 1:
            print(__doc__, file=sys.stderr)
            return 2
        return budget(folders[0])
    args = [a for a in argv if not a.startswith("--")]
    flags = {a for a in argv if a.startswith("--")}
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    old, new = load(args[0]), load(args[1])
    if not (old.keys() & new.keys()):
        print("no result file in both folders", file=sys.stderr)
        return 2
    mismatch = environment_mismatch(old, new)
    if mismatch:
        print("The runs are not comparable (" + "; ".join(mismatch) + "). Compare runs from the same machine.", file=sys.stderr)
        return 2
    rows = compare(old, new)
    flagged = [r for r in rows if r[5] in ("WORSE", "over 1% budget")]
    if "--json" in flags:
        print(json.dumps([{"metric": r[0], "old": r[2], "new": r[3], "noise": r[4], "status": r[5]} for r in rows], indent=2))
    else:
        for metric, rule, a, b, noise, status in rows:
            if "--all" in flags or status != "same":
                print(f"{status:<15} {metric}: {fmt(a, rule)} -> {fmt(b, rule)} (noise {fmt(noise, rule)})")
        better = sum(1 for r in rows if r[5] == "better")
        print(f"{len(rows)} measurements, {len(flagged)} worse, {better} better")
    return 1 if flagged else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
